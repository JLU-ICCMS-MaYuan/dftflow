use super::{config, status, store};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

/// 运行绑定：prepare 唯一建立；input-init.toml 不可修改；新输入新绑定（数据模型 v1）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunBinding {
    pub schema_version: u32,
    pub run_id: String,
    pub config_sha256: String,
    pub structure: store::Identity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kpath: Option<store::Identity>,
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
        ensure!(
            config_sha_of_init(&self.work_dir)? == self.config_sha256,
            "input_conflict: 固定配置被修改"
        );
        self.structure
            .verify()
            .context("input_conflict: 结构快照被修改")?;
        if let Some(path) = &self.kpath { path.verify().context("input_conflict: 路径快照被修改")?; }
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
pub fn prepare(config: &config::Config, _config_path: &Path) -> Result<RunBinding> {
    let work = config.work_dir.clone();
    super::run_state::ready(&work)?;
    let _lock = store::Lock::acquire(&work)?;
    let stem = config
        .structure
        .file_name()
        .context("data_invalid: 结构缺少文件名")?
        .to_string_lossy();
    let init_name = if stem.ends_with("-init") {
        stem.to_string()
    } else {
        format!("{stem}-init")
    };
    let mut fixed = config.clone();
    fixed.structure = work.join(&init_name);
    fixed.kpath = config.kpath.as_ref().map(|_| work.join("KPATH-init"));
    let bytes = toml::to_string_pretty(&fixed)?.into_bytes();
    if work.exists() {
        let existing = RunBinding::load(&work)?;
        existing.verify()?;
        ensure!(
            store::hash_bytes(&bytes) == existing.config_sha256,
            "input_conflict: 工作目录已绑定不同配置，拒绝覆盖"
        );
        ensure!(
            store::hash_file(&config.structure)? == existing.structure.sha256,
            "input_conflict: 原结构与快照不一致"
        );
        if let Some(path) = &config.kpath {
            ensure!(existing.kpath.as_ref().is_some_and(|id| store::hash_file(path).ok().as_ref() == Some(&id.sha256)), "input_conflict: 原路径与快照不一致");
        }
        return Ok(existing);
    }
    let staging = work.with_file_name(format!("epw-prepare-{}", store::token()));
    fs::create_dir(&staging)?;
    let result = (|| -> Result<RunBinding> {
        fs::copy(&config.structure, staging.join(&init_name))?;
        let mut structure = store::Identity::capture(&staging.join(&init_name))?;
        structure.path = fixed.structure.clone();
        structure.target = fixed.structure.clone();
        let kpath = if let Some(path) = &config.kpath {
            fs::copy(path, staging.join("KPATH-init"))?;
            let mut id = store::Identity::capture(&staging.join("KPATH-init"))?;
            id.path = fixed.kpath.clone().unwrap();
            id.target = id.path.clone();
            Some(id)
        } else { None };
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
        if config.phonon.mode == config::PhMode::Nosplit {
            if let Some(dir) = config.programs.ph.path.parent() {
                for name in ["q2r", "matdyn"] {
                    let path = dir.join(format!("{name}.x"));
                    if path.is_file() { programs.insert(name.into(), store::Identity::capture(&path)?); }
                }
            }
        }
        if config.interpolation.quality.reference_manifest.is_none() {
            if let Some(dir) = config.programs.ph.path.parent() {
                let path = dir.join("dynmat.x");
                if path.is_file() { programs.insert("dynmat".into(), store::Identity::capture(&path)?); }
            }
        }
        for (n, p) in [
            ("wannier90", &config.programs.wannier90),
            ("pw2wannier90", &config.programs.pw2wannier90),
        ] {
            if let Some(p) = p {
                programs.insert(n.to_owned(), store::Identity::capture(&p.path)?);
            }
        }
        if let Some(p) = &config.programs.environment_script {
            programs.insert("environment_script".into(), store::Identity::capture(p)?);
        }
        let config_sha256 = store::hash_bytes(&bytes);
        let binding = RunBinding {
            schema_version: 1,
            run_id: run_id_of(&config_sha256, &structure, &pseudos, &programs, kpath.as_ref()),
            config_sha256,
            structure,
            kpath,
            pseudos,
            programs,
            environment_status: "unverified".into(),
            work_dir: work.clone(),
            outdir_base: config.outdir.clone(),
            budget_id: config.budget.campaign_id.clone(),
        };
        store::atomic_bytes(&staging.join("input-init.toml"), &bytes)?;
        store::write(&staging.join("run-binding.json"), &binding)?;
        fs::rename(&staging, &work)?;
        Ok(binding)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn run_id_of(
    config_sha: &str,
    structure: &store::Identity,
    pseudos: &BTreeMap<String, store::Identity>,
    programs: &BTreeMap<String, store::Identity>,
    kpath: Option<&store::Identity>,
) -> String {
    let mut s = String::new();
    s.push_str(config_sha);
    s.push(':');
    s.push_str(&structure.sha256);
    if let Some(id) = kpath { s.push_str(":kpath:"); s.push_str(&id.sha256); }
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

/// 续算沿用统一入口的固定快照、执行边界与分配登记。
pub fn resume(work: &Path, submit: bool, require_allocation: bool) -> Result<()> {
    super::workflow::resume(work, None, submit, require_allocation)
}
