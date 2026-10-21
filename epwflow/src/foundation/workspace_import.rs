//! 有界的一次性旧工作区导入；原绑定和原生文件只读。
use super::{
    config::Config,
    layout,
    snapshot::RunBinding,
    stages::{self, Completion, Step},
    store::{self, Identity},
};
use crate::qe_data::{manifest::QeManifest, save};
use anyhow::{ensure, Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(serde::Serialize, serde::Deserialize)]
struct Transaction {
    source: Identity,
    partial: PathBuf,
    manifest: QeManifest,
    files: Vec<save::TreeFile>,
    source_files: Vec<save::TreeFile>,
    calculation_dir: PathBuf,
}

fn publish(cfg: &Config, t: &Transaction, report: &Path) -> Result<()> {
    t.source.verify()?;
    ensure!(
        t.source_files == save::files(&t.calculation_dir)?,
        "input_conflict: 导入来源文件集合改变"
    );
    let heads = t
        .files
        .iter()
        .map(|f| {
            f.relative_path
                .components()
                .next()
                .unwrap()
                .as_os_str()
                .to_owned()
        })
        .collect::<std::collections::BTreeSet<_>>();
    for head in heads {
        let from = t.partial.join(&head);
        let to = cfg.work_dir.join(&head);
        ensure!(
            !from.exists() || !to.exists(),
            "input_conflict: 导入来源与目标同时存在，保留现场"
        );
        let root = if from.exists() {
            &t.partial
        } else {
            &cfg.work_dir
        };
        for f in t
            .files
            .iter()
            .filter(|f| f.relative_path.starts_with(&head))
        {
            let p = root.join(&f.relative_path);
            ensure!(
                fs::metadata(&p)?.len() == f.byte_size && store::hash_file(&p)? == f.sha256,
                "input_conflict: 导入暂存数据改变 {}",
                p.display()
            );
        }
        if from.exists() {
            fs::rename(from, to)?;
        }
    }
    t.manifest.verify()?;
    store::write(
        &layout::path(&cfg.work_dir, "qe-data/manifest.json"),
        &t.manifest,
    )?;
    store::write(
        &Step::Phonon.directory(cfg).join("save/manifest.json"),
        &t.manifest,
    )?;
    for step in [Step::Scf, Step::Nscf, Step::Phonon] {
        let c = Completion {
            schema_version: 1,
            step,
            scientific_key: stages::scientific_key(cfg, step)?,
            upstream: stages::upstream(cfg, step)?,
            outputs: super::workspace_science::evidence(cfg, step)?,
            provenance: Some(t.source.clone()),
        };
        c.verify()?;
        store::write(&stages::receipt(&cfg.work_dir, step), &c)?;
    }
    t.source.verify()?;
    ensure!(
        t.source_files == save::files(&t.calculation_dir)?,
        "input_conflict: 原来源在发布期间改变"
    );
    store::write(
        report,
        &serde_json::json!({"schema_version":1,"source_sha256":t.source.sha256,"source_files":t.source_files,"calculation_dir":t.calculation_dir,"new_manifest":Identity::capture(&layout::path(&cfg.work_dir,"qe-data/manifest.json"))?}),
    )?;
    println!("import_status: complete\nqe_recalculated: false");
    Ok(())
}

pub fn import(config: &Path, source: &Path) -> Result<()> {
    let cfg = Config::load(config)?;
    let source = source.canonicalize()?;
    ensure!(
        !cfg.work_dir.starts_with(&source) && !source.starts_with(&cfg.work_dir.join("epw-state")),
        "input_conflict: 导入来源与目标重叠"
    );
    let mut binding = RunBinding::load(&source)?;
    // 历史绑定原字节保留；已删除的抽样程序不再是主科学数据导入依赖。
    binding.programs.remove("dynmat");
    binding.verify()?;
    let original = Config::load_imported(&source.join("input-init.toml"))?;
    let state = if source.join("epw-state/workflow.json").is_file() { "epw-state" } else { ".epwflow" };
    let record: serde_json::Value = store::read(&source.join(state).join("workflow.json"))?;
    if let Some(job) = record.get("job_id").and_then(|v| v.as_u64()) {
        ensure!(
            super::slurm::accounting(job)?.terminal,
            "scheduler_unknown: 旧任务仍活动或未知"
        );
    }
    let calc = PathBuf::from(
        record
            .get("calculation_dir")
            .and_then(|v| v.as_str())
            .context("data_invalid: 旧计算目录缺失")?,
    )
    .canonicalize()?;
    ensure!(calc.starts_with(&source), "input_conflict: 旧计算目录越界");
    let old_path = calc.join("qe-data/manifest.json");
    let old = QeManifest::load(&old_path)?;
    old.verify()?;
    let provenance = Identity::capture(&old_path)?;
    let _guard = store::Lock::acquire(&cfg.work_dir.join("epw-state/control"))?;
    ensure!(
        !cfg.work_dir.join("epw-state/active.json").exists(),
        "input_conflict: 先核对已有工作区尝试，再导入旧现场"
    );
    let report = cfg.work_dir.join("epw-state/legacy-import.json");
    if report.exists() {
        let prior: serde_json::Value = store::read(&report)?;
        ensure!(
            prior["source_sha256"].as_str() == Some(&provenance.sha256),
            "input_conflict: 工作区已导入不同来源"
        );
        for step in [Step::Scf, Step::Nscf, Step::Phonon] {
            let c: Completion = store::read(&stages::receipt(&cfg.work_dir, step))?;
            c.verify()?;
        }
        println!("import_status: reused");
        return Ok(());
    }
    for step in [Step::Scf, Step::Nscf, Step::Phonon] {
        ensure!(
            stages::scientific_key(&cfg, step)? == stages::scientific_key(&original, step)?,
            "input_conflict: 旧 {} 科学参数与当前配置不相容",
            step.name()
        );
    }
    layout::initialize(&cfg)?;
    let journal = cfg.work_dir.join("epw-state/importing.json");
    if journal.exists() {
        let t: Transaction = store::read(&journal)?;
        ensure!(
            t.source.sha256 == provenance.sha256,
            "input_conflict: 未完成导入属于其他来源"
        );
        return publish(&cfg, &t, &report);
    }
    for step in [Step::Scf, Step::Nscf, Step::Phonon] {
        ensure!(
            !step.directory(&cfg).exists(),
            "input_conflict: 导入目标 {} 已存在",
            step.directory(&cfg).display()
        );
    }
    let before = save::files(&calc)?;
    let partial = cfg
        .work_dir
        .join("epw-state")
        .join(format!("import-{}", store::token()));
    fs::create_dir_all(&partial)?;
    let mut manifest = old.clone();
    manifest.artifact_root = Some(cfg.work_dir.canonicalize()?);
    fn relocate(cfg: &Config, old: &Path, calc: &Path) -> Result<PathBuf> {
        let relative = old
            .strip_prefix(calc)
            .context("data_invalid: 原清单文件越界")?;
        let text = relative.to_str().context("data_invalid: 非 UTF-8 路径")?;
        Ok(layout::path(&cfg.work_dir, text))
    }
    for a in &mut manifest.artifacts {
        let from = calc.join(&a.relative_path);
        let mut to = relocate(&cfg, &from, &calc)?;
        if a.role == "qeflow_save/manifest.json" {
            to = Step::Phonon
                .directory(&cfg)
                .join("save/source-qeflow-manifest.json");
        }
        let rel = to.strip_prefix(&cfg.work_dir)?;
        let temp = partial.join(rel);
        fs::create_dir_all(temp.parent().unwrap())?;
        fs::copy(&from, &temp)?;
        ensure!(
            store::hash_file(&temp)? == a.sha256,
            "data_invalid: 导入复制哈希不符"
        );
        a.relative_path = rel.into();
    }
    // 严格清单只包含必要角色；另复制完整 NSCF 和 SCF 快照以保留所有可重读文件。
    for (from, rel) in [
        (
            old.scf
                .as_ref()
                .context("data_invalid: 缺 SCF")?
                .snapshot_dir
                .clone(),
            PathBuf::from("1.qe-scf/snapshot"),
        ),
        (
            old.states
                .as_ref()
                .context("data_invalid: 缺 NSCF")?
                .wavefunctions
                .parent()
                .unwrap()
                .into(),
            PathBuf::from("2.qe-nscf/out"),
        ),
    ] {
        for f in save::files(&from)? {
            let to = partial.join(&rel).join(&f.relative_path);
            fs::create_dir_all(to.parent().unwrap())?;
            if to.exists() {
                ensure!(
                    store::hash_file(&to)? == f.sha256,
                    "input_conflict: 导入角色冲突"
                );
            } else {
                fs::copy(from.join(&f.relative_path), &to)?;
            }
        }
    }
    // 旧共享链把 SCF 输入/日志放在声子目录；新布局归到第 1 步。
    for name in ["scf.in", "scf.out"] {
        let from = calc.join("qe-phonon").join(name);
        if from.exists() {
            fs::copy(from, partial.join("1.qe-scf").join(name))?;
        }
    }
    // 布局迁移同时物化已有后处理，避免为了 q2r/matdyn 产物重新启动 PH。
    let ph = calc.join("qe-phonon");
    if ph.is_dir() {
        for entry in fs::read_dir(&ph)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if entry.file_type()?.is_file()
                && (name.ends_with(".out")
                    || name.ends_with(".xml")
                    || name.ends_with(".freq")
                    || name.ends_with(".gp")
                    || name.ends_with(".dat")
                    || name.ends_with(".jpg")
                    || name == "KPATH.in")
            {
                let dest = partial
                    .join(layout::phonon_name(cfg.phonon.mode))
                    .join(name.as_ref());
                fs::create_dir_all(dest.parent().unwrap())?;
                fs::copy(entry.path(), dest)?;
            }
        }
    }
    let qe = crate::qe_data::qe_config::QeSettings::from_config(&cfg)?;
    fs::write(partial.join("1.qe-scf/scf.in"), qe.scf_input("./out")?)?;
    fs::write(partial.join("2.qe-nscf/nscf.in"), qe.nscf_input("./out")?)?;
    if let Some(s) = &mut manifest.scf {
        s.snapshot_dir = relocate(&cfg, &s.snapshot_dir, &calc)?;
    }
    if let Some(s) = &mut manifest.states {
        s.wavefunctions = relocate(&cfg, &s.wavefunctions, &calc)?;
    }
    for q in &mut manifest.qpoints {
        let target = Step::Phonon
            .directory(&cfg)
            .join(format!("{}.dyn0", cfg.qe.prefix));
        let temp = partial.join(target.strip_prefix(&cfg.work_dir)?);
        fs::create_dir_all(temp.parent().unwrap())?;
        fs::copy(&q.source_dyn0, &temp)?;
        q.source_dyn0 = target;
    }
    for r in &mut manifest.phonon_files {
        r.dyn_file = relocate(&cfg, &r.dyn_file, &calc)?;
        r.patterns = relocate(&cfg, &r.patterns, &calc)?;
        r.control_metadata = relocate(&cfg, &r.control_metadata, &calc)?;
        if let Some(p) = &mut r.dvscf {
            *p = relocate(&cfg, p, &calc)?;
        }
    }
    ensure!(
        before == save::files(&calc)?,
        "input_conflict: 原来源在导入期间改变"
    );
    let transaction = Transaction {
        source: provenance,
        files: save::files(&partial)?,
        partial,
        manifest,
        source_files: before,
        calculation_dir: calc,
    };
    store::write(&journal, &transaction)?;
    publish(&cfg, &transaction, &report)
}
