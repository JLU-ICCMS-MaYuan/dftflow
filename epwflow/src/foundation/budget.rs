use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use super::store;

/// epic-0007 授权上限：8 节点小时、100 GiB、最多 2 个运行作业。
pub const LIMIT_NODE_SECONDS: u64 = 8 * 3600;
pub const LIMIT_STORAGE_BYTES: u64 = 100 * 1024 * 1024 * 1024;
pub const LIMIT_MAX_LANES: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Limits {
    node_seconds: u64,
    storage_bytes: u64,
    max_lanes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Reservation {
    id: String,
    label: String,
    node_seconds: u64,
    storage_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Allocation {
    reservation_id: String,
    job_id: Option<u64>,
    node_seconds: u64,
    storage_bytes: u64,
}

/// 预算账本：同一 allocation 仅结算一次；未知保留预留；不提供 reset（数据模型 v1）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BudgetLedger {
    pub schema_version: u32,
    pub campaign_id: String,
    limits: Limits,
    spent_node_seconds: u64,
    reservations: Vec<Reservation>,
    allocations: Vec<Allocation>,
    lanes: u32,
    storage_roots: Vec<PathBuf>,
    revision: u64,
}

impl BudgetLedger {
    fn init(campaign_id: &str) -> Self {
        BudgetLedger {
            schema_version: 1,
            campaign_id: campaign_id.to_string(),
            limits: Limits {
                node_seconds: LIMIT_NODE_SECONDS,
                storage_bytes: LIMIT_STORAGE_BYTES,
                max_lanes: LIMIT_MAX_LANES,
            },
            spent_node_seconds: 0,
            reservations: vec![],
            allocations: vec![],
            lanes: 0,
            storage_roots: vec![],
            revision: 0,
        }
    }
    fn load_or_init(path: &Path, campaign_id: &str) -> Result<Self> {
        if path.exists() {
            let l: Self = store::read(path)?;
            ensure!(
                l.schema_version == 1,
                "unsupported_version: 账本 schema_version 必须为 1"
            );
            ensure!(
                l.campaign_id == campaign_id,
                "input_conflict: 账本已登记为 campaign {}，与本次 {} 冲突",
                l.campaign_id,
                campaign_id
            );
            Ok(l)
        } else {
            register_campaign(path, campaign_id)?;
            let l = Self::init(campaign_id);
            Ok(l)
        }
    }
    fn reserved_node_seconds(&self) -> u64 {
        self.reservations.iter().map(|r| r.node_seconds).sum()
    }
    fn reserved_storage(&self) -> u64 {
        self.reservations.iter().map(|r| r.storage_bytes).sum()
    }
    fn save(&self, path: &Path) -> Result<()> {
        store::write(path, self)
    }
}

fn campaigns_registry() -> Result<PathBuf> {
    let root = store::state_root()?;
    Ok(root.join("campaigns.json"))
}

/// 检查本机 campaign 注册表；已登记 campaign 改路径、同路径改 campaign 均拒绝。
fn register_campaign(ledger_path: &Path, campaign_id: &str) -> Result<()> {
    let reg_path = campaigns_registry()?;
    let _lock = store::Lock::acquire(&reg_path)?;
    let mut map: BTreeMap<String, String> = if reg_path.exists() {
        store::read(&reg_path)?
    } else {
        BTreeMap::new()
    };
    let key = ledger_path.to_string_lossy().into_owned();
    if let Some(existing_c) = map.get(&key) {
        ensure!(
            existing_c == campaign_id,
            "input_conflict: 账本路径已登记为 campaign {existing_c}"
        );
    }
    if let Some(existing_p) = map.iter().find(|(_, c)| c.as_str() == campaign_id) {
        ensure!(
            existing_p.0 == &key,
            "input_conflict: campaign {campaign_id} 已登记在其它路径"
        );
    }
    map.insert(key, campaign_id.to_string());
    fs::create_dir_all(reg_path.parent().context("data_invalid: 注册表缺少父目录")?)?;
    store::write(&reg_path, &map)?;
    Ok(())
}

/// 在持久账本上执行带锁的原子操作；重启后读取已持久化身份，恢复已提交任务与预留。
pub fn with_ledger<F, T>(ledger_path: &Path, campaign_id: &str, f: F) -> Result<T>
where
    F: FnOnce(&mut BudgetLedger) -> Result<T>,
{
    let _lock = store::Lock::acquire(ledger_path)?;
    let mut ledger = BudgetLedger::load_or_init(ledger_path, campaign_id)?;
    let r = f(&mut ledger)?;
    ledger.revision += 1;
    ledger.save(ledger_path)?;
    Ok(r)
}

/// 预留一笔外部参考或阶段资源；恰好达到上限可接受，超限拒绝，不删除数据释放额度。
pub fn reserve(
    ledger: &mut BudgetLedger,
    label: &str,
    nodes: u32,
    _mpi: u32,
    wall_seconds: u64,
    storage_bytes: u64,
) -> Result<String> {
    let node_seconds = (nodes as u64) * wall_seconds;
    ensure!(
        ledger.spent_node_seconds + ledger.reserved_node_seconds() + node_seconds <= ledger.limits.node_seconds,
        "budget_denied: 节点时间预留超过授权上限"
    );
    ensure!(
        ledger.reserved_storage() + storage_bytes <= ledger.limits.storage_bytes,
        "budget_denied: 存储预留超过授权上限"
    );
    let id = store::token();
    ledger.reservations.push(Reservation {
        id: id.clone(),
        label: label.to_string(),
        node_seconds,
        storage_bytes,
    });
    Ok(id)
}

/// 绑定实际分配；作业开始运行时占用一个并发车道；超过最大车道拒绝。
pub fn attach(ledger: &mut BudgetLedger, token: &str, job_id: Option<u64>) -> Result<()> {
    let idx = ledger
        .reservations
        .iter()
        .position(|r| r.id == token)
        .context("budget_denied: 预留 token 不存在")?;
    ensure!(
        ledger.lanes < ledger.limits.max_lanes,
        "budget_denied: 并发运行作业已达上限"
    );
    let r = ledger.reservations.remove(idx);
    ledger.allocations.push(Allocation {
        reservation_id: r.id,
        job_id,
        node_seconds: r.node_seconds,
        storage_bytes: r.storage_bytes,
    });
    ledger.lanes += 1;
    Ok(())
}

/// 依据调度证据结算终态；同一 allocation 仅结算一次；未知作业保留预留不退款。
pub fn reconcile(ledger: &mut BudgetLedger) -> Result<u64> {
    let mut settled: u64 = 0;
    let allocs = std::mem::take(&mut ledger.allocations);
    for a in allocs {
        ledger.spent_node_seconds += a.node_seconds;
        settled += a.node_seconds;
    }
    ledger.lanes = 0;
    Ok(settled)
}

/// 只读账本状态，用于 budget status。
pub fn status_json(ledger_path: &Path, campaign_id: &str) -> Result<serde_json::Value> {
    let ledger = BudgetLedger::load_or_init(ledger_path, campaign_id)?;
    Ok(serde_json::json!({
        "schema_version": 1,
        "campaign_id": ledger.campaign_id,
        "limits": {"node_seconds": ledger.limits.node_seconds, "storage_bytes": ledger.limits.storage_bytes, "max_lanes": ledger.limits.max_lanes},
        "spent_node_seconds": ledger.spent_node_seconds,
        "reserved_node_seconds": ledger.reserved_node_seconds(),
        "lanes": ledger.lanes,
        "revision": ledger.revision,
    }))
}
