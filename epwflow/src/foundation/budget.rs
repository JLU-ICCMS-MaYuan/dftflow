use super::store;
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

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
    /// 核对当前分配已经登记且覆盖阶段请求，不创建第二份预留。
    pub fn require_allocation(&self, job: u64, resources: &super::config::Resources) -> Result<String> {
        let allocation = self
            .allocations
            .iter()
            .find(|a| a.job_id == Some(job))
            .context("allocation: 当前 Slurm 分配尚未登记；请先 budget reserve/attach")?;
        ensure!(
            allocation.node_seconds >= resources.nodes as u64 * resources.wall_seconds
                && allocation.storage_bytes >= resources.storage_bytes,
            "budget_denied: 已登记分配不足以覆盖请求时间或存储"
        );
        Ok(allocation.reservation_id.clone())
    }

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
            let registry = campaigns_registry()?;
            if registry.exists() {
                let entries: BTreeMap<String, String> = store::read(&registry)?;
                ensure!(
                    !entries.values().any(|id| id == campaign_id),
                    "input_conflict: 已登记活动的账本丢失或换路径，拒绝重置预算"
                );
            }
            register_campaign(path, campaign_id)?;
            let l = Self::init(campaign_id);
            Ok(l)
        }
    }
    fn reserved_node_seconds(&self) -> u64 {
        self.reservations
            .iter()
            .map(|r| r.node_seconds)
            .chain(self.allocations.iter().map(|a| a.node_seconds))
            .sum()
    }
    fn reserved_storage(&self) -> u64 {
        self.reservations
            .iter()
            .map(|r| r.storage_bytes)
            .chain(self.allocations.iter().map(|a| a.storage_bytes))
            .sum()
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
    fs::create_dir_all(
        reg_path
            .parent()
            .context("data_invalid: 注册表缺少父目录")?,
    )?;
    store::write(&reg_path, &map)?;
    Ok(())
}

/// 在持久账本上执行带锁的原子操作；重启后读取已持久化身份，恢复已提交任务与预留。
pub fn with_ledger<F, T>(ledger_path: &Path, campaign_id: &str, f: F) -> Result<T>
where
    F: FnOnce(&mut BudgetLedger) -> Result<T>,
{
    let normalized = super::paths::destination(&super::paths::absolute(
        &std::env::current_dir()?,
        ledger_path,
    )?)?;
    let ledger_path = normalized.as_path();
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
    mpi: u32,
    wall_seconds: u64,
    storage_bytes: u64,
) -> Result<String> {
    ensure!(
        nodes == 1 && (1..=64).contains(&mpi) && (1..=7200).contains(&wall_seconds),
        "budget_denied: 每作业只允许1节点、1～64 MPI、1～7200秒"
    );
    ensure!(storage_bytes > 0, "budget_denied: 必须预留存储");
    let node_seconds = (nodes as u64) * wall_seconds;
    ensure!(
        ledger.spent_node_seconds + ledger.reserved_node_seconds() + node_seconds
            <= ledger.limits.node_seconds,
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
    ensure!(
        job_id.is_some_and(|id| id > 0),
        "budget_denied: 需要有效作业号"
    );
    ensure!(
        !ledger.allocations.iter().any(|a| a.job_id == job_id),
        "budget_denied: 分配已登记，不可重复计费"
    );
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

/// 自动入口保守限制所有未结算预留；排队或未知任务也占名额。
pub fn reserve_workflow(ledger: &mut BudgetLedger, label: &str, r: &super::config::Resources) -> Result<String> {
    ensure!(ledger.reservations.len() + ledger.allocations.len() < ledger.limits.max_lanes as usize,
        "budget_denied: 已有两个未结算任务；先核对终态并结算");
    reserve(ledger, label, r.nodes, r.mpi, r.wall_seconds, r.storage_bytes)
}

/// 提交回执与计算节点启动可能并发；仅同一 token/作业允许幂等绑定。
pub fn attach_once(ledger: &mut BudgetLedger, token: &str, job: u64) -> Result<()> {
    if let Some(a) = ledger.allocations.iter().find(|a| a.reservation_id == token) {
        ensure!(a.job_id == Some(job), "input_conflict: 预留已绑定其他作业");
        return Ok(());
    }
    attach(ledger, token, Some(job))
}

/// 资源内运行必须复用已登记分配，不按阶段重复预留。
pub fn registered_allocation(ledger: &BudgetLedger, job: u64, r: &super::config::Resources) -> Result<String> {
    ledger.require_allocation(job, r)
}

/// 仅生成脚本的计算节点入口可自动登记；已有分配仍复用原预留并检查额度。
pub fn register_generated(ledger: &mut BudgetLedger, job: u64, r: &super::config::Resources) -> Result<String> {
    if ledger.allocations.iter().any(|a| a.job_id == Some(job)) {
        return registered_allocation(ledger, job, r);
    }
    let token = reserve_workflow(ledger, &format!("generated:{job}"), r)?;
    attach_once(ledger, &token, job)?;
    Ok(token)
}

/// 依据调度证据结算终态；同一 allocation 仅结算一次；未知作业保留预留不退款。
pub fn reconcile(ledger: &mut BudgetLedger) -> Result<u64> {
    // 先取得全部证据，查询失败时不做部分修改；with_ledger 随后原子发布。
    let observations = ledger
        .allocations
        .iter()
        .map(|a| super::slurm::accounting(a.job_id.context("scheduler_unknown: 分配缺作业号")?))
        .collect::<Result<Vec<_>>>()?;
    let mut settled = 0u64;
    let mut retained = Vec::new();
    for (a, evidence) in ledger.allocations.iter().zip(observations) {
        if evidence.terminal {
            settled = settled
                .checked_add(evidence.node_seconds)
                .context("budget_denied: 计费溢出")?;
        } else {
            retained.push(a.clone());
        }
    }
    ledger.spent_node_seconds = ledger
        .spent_node_seconds
        .checked_add(settled)
        .context("budget_denied: 计费溢出")?;
    ledger.allocations = retained;
    ledger.lanes = ledger.allocations.len() as u32;
    Ok(settled)
}

/// 只读账本状态，用于 budget status。
pub fn status_json(ledger_path: &Path, campaign_id: &str) -> Result<serde_json::Value> {
    let normalized = super::paths::destination(&super::paths::absolute(
        &std::env::current_dir()?,
        ledger_path,
    )?)?;
    let ledger_path = normalized.as_path();
    ensure!(
        ledger_path.is_file(),
        "data_invalid: 账本尚未创建；status 不创建预算身份"
    );
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
