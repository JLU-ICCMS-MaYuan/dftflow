use anyhow::{bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use super::{config, store, status};

/// 运行绑定：prepare 唯一建立；input-init.toml 不可修改；新输入新绑定（数据模型 v1）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunBinding {
    pub schema_version: u32,
    pub run_id: String,
    pub config_sha256: String,
    pub structure: store::Identity,
    pub pseudos: BTreeMap<String, store::Identity>,
    pub programs: BTreeMap<String, store::Identity>,
    pub environment_status: String,
    pub work_dir: PathBuf,
    pub outdir_base: PathBuf,
    pub budget_id: String,
}

impl RunBinding {
    pub fn load(work: &Path) -> Result<Self> {
        let p = work.join("run-binding.json");
        ensure!(p.exists(), "input_conflict: 运行目录缺少 run-binding.json");
        let b: Self = store::read(&p)?;
        ensure!(
            b.schema_version == 1,
            "unsupported_version: 运行绑定 schema_version 必须为 1"
        );
        Ok(b)
    }
    /// 核对绑定来源未被篡改；复用前必须重算指纹，不能把历史 check 当作永久有效。
    pub fn verify(&self) -> Result<()> {
        self.structure.verify().context("input_conflict: 结构快照被修改")?;
        for (el, id) in &self.pseudos {
            id.verify()
                .with_context(|| format!("input_conflict: 赝势 {el} 被修改"))?;
        }
        for (n, id) in &self.programs {
            id.verify()
                .with_context(|| format!("input_conflict: 程序 {n} 被修改"))?;
        }
        Ok(())
    }
}

fn input_init_path(work: &Path) -> PathBuf {
    work.join("input-init.toml")
}

fn config_sha_of_init(work: &Path) -> Result<String> {
    store::hash_file(&input_init_path(work))
}

/// 固定输入并准备独立运行；相同输入重复准备幂等，冲突/篡改/已存在不明文件拒绝覆盖。
pub fn prepare(config: &config::Config, config_path: &Path) -> Result<RunBinding> {
    let work = config.work_dir.clone();
    if work.exists() {
        let bind_path = work.join("run-binding.json");
        if bind_path.exists() {
            let existing = store::read::<RunBinding>(&bind_path)?;
            ensure!(
                existing.schema_version == 1,
                "unsupported_version: 既有运行绑定 schema_version 必须为 1"
            );
            let init_hash = config_sha_of_init(&work)?;
            if existing.config_sha256 == init_hash {
                // 幂等：重新写入同一 input-init.toml，不改变既有快照。
                fs::write(&input_init_path(&work), fs::read(config_path)?)
                    .with_context(|| format!("data_invalid: 无法写回 {}", input_init_path(&work).display()))?;
                return Ok(existing);
            }
            bail!("input_conflict: 运行目录已绑定不同配置，拒绝覆盖");
        }
        bail!("input_conflict: 工作目录已存在且不是有效的既有运行，拒绝覆盖");
    }
    fs::create_dir_all(&work)
        .with_context(|| format!("data_invalid: 无法创建工作目录 {}", work.display()))?;

    // 原结构加 -init 的副本，固定结构身份。
    let stem = config
        .structure
        .file_name()
        .context("data_invalid: 结构文件缺少文件名")?
        .to_string_lossy()
        .into_owned();
    let init_name = format!("{stem}-init");
    let init_path = work.join(&init_name);
    fs::copy(&config.structure, &init_path)
        .with_context(|| format!("data_invalid: 无法复制结构到 {}", init_path.display()))?;
    let structure = store::Identity::capture(&init_path)?;

    let mut pseudos = BTreeMap::new();
    for (el, p) in &config.pseudos {
        pseudos.insert(el.clone(), store::Identity::capture(p)?);
    }
    let mut programs = BTreeMap::new();
    for (n, p) in [
        ("pw", &config.programs.pw),
        ("ph", &config.programs.ph),
        ("epw", &config.programs.epw),
    ] {
        programs.insert(n.to_string(), store::Identity::capture(&p.path)?);
    }

    let config_sha256 = store::hash_file(config_path)?;
    let run_id = run_id_of(&config_sha256, &structure, &pseudos, &programs);
    let binding = RunBinding {
        schema_version: 1,
        run_id,
        config_sha256,
        structure,
        pseudos,
        programs,
        environment_status: "unverified".to_string(),
        work_dir: work.clone(),
        outdir_base: config.outdir.clone(),
        budget_id: config.budget.campaign_id.clone(),
    };
    // input-init.toml 以原始 TOML 字节固定，下游只读此绑定。
    fs::write(&input_init_path(&work), fs::read(config_path)?)
        .with_context(|| format!("data_invalid: 无法写 {}", input_init_path(&work).display()))?;
    store::write(&work.join("run-binding.json"), &binding)?;
    Ok(binding)
}

fn run_id_of(
    config_sha: &str,
    structure: &store::Identity,
    pseudos: &BTreeMap<String, store::Identity>,
    programs: &BTreeMap<String, store::Identity>,
) -> String {
    let mut s = String::new();
    s.push_str(config_sha);
    s.push(':');
    s.push_str(&structure.sha256);
    for (el, id) in pseudos {
        s.push(':');
        s.push_str(el);
        s.push(':');
        s.push_str(&id.sha256);
    }
    for (n, id) in programs {
        s.push(':');
        s.push_str(n);
        s.push(':');
        s.push_str(&id.sha256);
    }
    store::hash_bytes(s.as_bytes())[..16].to_string()
}

/// 状态查询：已绑定但未完成科学链；只读可以成功返回“失败/未知”，但必须显式显示。
pub fn status(work: &Path) -> Result<serde_json::Value> {
    let binding = RunBinding::load(work)?;
    Ok(status::envelope(
        "ok",
        "in_progress",
        serde_json::json!({
            "run_id": binding.run_id,
            "workflow_complete": false,
            "stages": {}
        }),
    ))
}

/// 续算：先核对固定快照，再按选择续算；F1 科学阶段未接入，一律返回明确不可用。
pub fn resume(work: &Path, submit: bool, require_allocation: bool) -> Result<()> {
    let binding = RunBinding::load(work)?;
    binding.verify()?;
    if submit && require_allocation {
        bail!("cannot: resume 的 -s 与 -r 必须互斥");
    }
    if !submit && !require_allocation {
        bail!("required: resume 必须明确选择 -s 或 -r");
    }
    if require_allocation {
        bail!("not_implemented: 科学阶段尚未接入，resume -r 需要分配记录");
    }
    bail!("not_implemented: 科学阶段尚未接入，resume 不可用");
}
