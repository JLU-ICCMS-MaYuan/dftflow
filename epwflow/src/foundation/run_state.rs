//! 可见运行状态与有界迁移；正常运行不回读旧隐藏树。
use super::{
    config::Config,
    layout,
    stages::{self, Completion, Step},
    store::{self, Identity},
};
use crate::qe_data::save;
use anyhow::{ensure, Result};
use serde_json::{json, Value};
use std::{fs, path::Path};

pub fn ready(root: &Path) -> Result<()> {
    let state = root.join("epw-state");
    let report = state.join("migration.json");
    if report.exists() {
        let r: Value = store::read(&report)?;
        ensure!(
            r["schema_version"] == 1,
            "unsupported_version: 状态迁移记录版本"
        );
        ensure!(
            r["complete"] == true,
            "input_conflict: 状态迁移尚未完成；请重新执行 epw_wannier --migrate-state"
        );
    } else {
        ensure!(!root.join(".epwflow").try_exists()?,
            "input_conflict: 检测到旧隐藏状态 .epwflow/；先核对原作业，再执行 epw_wannier --migrate-state；不自动改写旧现场");
    }
    Ok(())
}

fn hidden_reference(value: &Value) -> bool {
    match value {
        Value::String(s) => Path::new(s)
            .components()
            .any(|c| c.as_os_str() == ".epwflow"),
        Value::Array(a) => a.iter().any(hidden_reference),
        Value::Object(m) => m.values().any(hidden_reference),
        _ => false,
    }
}

fn copy_records(source: &Path, dest: &Path) -> Result<()> {
    if source.is_dir() {
        for file in save::files(source)? {
            let from = source.join(&file.relative_path);
            // 仅原样接入不读取隐藏路径的清单，历史来源另存完整快照。
            if file.relative_path.extension().is_some_and(|e| e == "json")
                && hidden_reference(&store::read::<Value>(&from)?)
            {
                continue;
            }
            let to = dest.join(file.relative_path);
            if to.exists() {
                ensure!(
                    store::hash_file(&to)? == file.sha256,
                    "input_conflict: 迁移目标记录已改变 {}",
                    to.display()
                );
            } else {
                store::atomic_bytes(&to, &fs::read(from)?)?;
            }
        }
    }
    Ok(())
}

fn terminal_attempt(path: &Path) -> Result<()> {
    let record: Value = store::read(path)?;
    // 电子原生子任务的回执封装在 attempt 字段中，与主提交共用相同状态约束。
    let active = record.get("attempt").unwrap_or(&record);
    if let Some(job) = active["job"].as_u64() {
        ensure!(
            super::slurm::accounting(job)?.terminal,
            "scheduler_unknown: 原作业 {job} 仍活动或终态未知，禁止迁移或重复提交"
        );
    } else {
        ensure!(
            active["state"]
                .as_str()
                .is_some_and(|s| matches!(s, "prepared" | "failed" | "succeeded")),
            "scheduler_unknown: 原提交状态未知 {}，禁止迁移或重复提交",
            path.display()
        );
    }
    Ok(())
}

fn legacy_files(root: &Path) -> Result<Vec<save::TreeFile>> {
    // 迁移借用旧控制锁阻止旧版本并发写入；自身锁不属于历史来源。
    Ok(save::files(root)?
        .into_iter()
        .filter(|f| !f.relative_path.starts_with("control.lock"))
        .collect())
}

/// 显式迁移只接入与隐藏路径无关的编号上游；冻结脚本及后续交付只作历史保存。
pub fn migrate(cfg: &Config) -> Result<()> {
    ensure!(cfg.layout.as_deref() == Some("numbered"),
        "input_conflict: --migrate-state 仅支持编号工作区；旧非编号工作区请使用 --import-work 导入新编号目录");
    let root = &cfg.work_dir;
    let _guard = store::Lock::acquire(&root.join("epw-state-migration"))?;
    let old = root.join(".epwflow");
    let state = root.join("epw-state");
    let report = state.join("migration.json");
    if report.is_file() && store::read::<Value>(&report)?["complete"] == true {
        println!("migration_status: complete（已迁移，无需重复）");
        return Ok(());
    }
    ensure!(old.is_dir(), "data_invalid: 未发现旧隐藏状态 .epwflow/");
    ensure!(
        !old.join("control.lock").exists() && !old.join("execution.lock").exists(),
        "scheduler_unknown: 旧控制锁或执行锁仍存在；核对 owner 与原作业后再迁移，不清除未知锁"
    );
    let _old_guard = store::Lock::acquire(&old.join("control"))?;
    for name in ["importing.json", "submission-pending.json"] {
        ensure!(
            !old.join(name).exists(),
            "scheduler_unknown: 旧工作区仍有未完成事务 {name}"
        );
    }
    let marker: layout::Layout = store::read(&old.join("layout.json"))?;
    ensure!(
        marker.schema_version == 1 && marker.phonon == cfg.phonon.mode,
        "input_conflict: 旧布局版本或声子模式与配置不同"
    );
    if old.join("active.json").is_file() {
        terminal_attempt(&old.join("active.json"))?;
    }
    if old.join("attempts").is_dir() {
        for entry in fs::read_dir(old.join("attempts"))? {
            let attempt = entry?.path().join("attempt.json");
            if attempt.is_file() {
                terminal_attempt(&attempt)?;
            }
        }
    }
    let ledger = if cfg.budget.ledger.exists() {
        Some(Identity::capture(&cfg.budget.ledger)?)
    } else {
        None
    };
    let before = legacy_files(&old)?;
    if !state.exists() {
        let partial = root.join("epw-state-migrating");
        ensure!(
            !partial.exists(),
            "input_conflict: 迁移暂存目录已存在 {}；核对其内容后处理，不自动覆盖",
            partial.display()
        );
        for file in &before {
            let to = partial
                .join("history/legacy-state")
                .join(&file.relative_path);
            fs::create_dir_all(to.parent().unwrap())?;
            fs::copy(old.join(&file.relative_path), to)?;
        }
        ensure!(
            legacy_files(&old)? == before,
            "input_conflict: 复制期间原状态改变"
        );
        store::write(
            &partial.join("migration.json"),
            &json!({"schema_version":1,"complete":false,"source_files":before}),
        )?;
        fs::rename(partial, &state)?;
    } else {
        ensure!(
            report.is_file(),
            "input_conflict: 可见状态与旧隐藏状态并存且未登记迁移，拒绝合并"
        );
        let prior: Value = store::read(&report)?;
        ensure!(
            prior["source_files"] == serde_json::to_value(&before)?,
            "input_conflict: 迁移期间原状态已改变"
        );
    }
    ensure!(
        save::files(&state.join("history/legacy-state"))? == before,
        "input_conflict: 迁移历史快照与来源不一致"
    );
    // 所有记录仍原样保存；正常入口只读取以下明确接入的可见记录。
    layout::initialize(cfg)?;
    copy_records(&old.join("qe-data"), &state.join("qe-data"))?;
    if root.join("3.qe-electronic").is_dir() && !state.join("history/rebuild-electronic").exists() {
        copy_records(
            &old.join("electronic"),
            &root.join("3.qe-electronic/receipts"),
        )?;
    }
    let mut reused = vec![];
    let mut rebuild = vec![];
    for step in Step::ALL {
        let from = old.join("steps").join(format!("{}.json", step.name()));
        let mut accepted = false;
        if step < Step::Coarse && from.is_file() && step.directory(cfg).is_dir() {
            let c: Completion = store::read(&from)?;
            ensure!(c.step == step, "data_invalid: 旧阶段回执身份不符");
            if c.scientific_key == stages::scientific_key(cfg, step)?
                && c.available()?
                && !c
                    .outputs
                    .iter()
                    .any(|i| i.path.starts_with(&old) || i.target.starts_with(&old))
                && step
                    .dependencies()
                    .iter()
                    .all(|s| reused.contains(&s.name()))
                && (step != Step::Phonon || state.join("qe-data/manifest.json").is_file())
            {
                store::write(&stages::receipt(root, step), &c)?;
                accepted = stages::reusable(cfg, step)?;
            }
        }
        if accepted {
            reused.push(step.name());
        } else {
            rebuild.push(step.name());
            // 清单内有旧隐藏路径的科学交付只保留为历史；不改写身份冒充新交付。
            let dir = step.directory(cfg);
            let dest = state
                .join("history")
                .join(format!("rebuild-{}", step.name()));
            if dir.exists() {
                ensure!(
                    !dest.exists(),
                    "input_conflict: 迁移归档已存在 {}",
                    dest.display()
                );
                fs::rename(dir, dest)?;
            }
        }
    }
    ensure!(
        legacy_files(&old)? == before,
        "input_conflict: 迁移期间原状态改变"
    );
    if let Some(id) = ledger {
        id.verify()?;
    }
    store::write(
        &report,
        &json!({"schema_version":1,"complete":true,"source_files":before,"reused":reused,"rebuild":rebuild,
        "legacy_scripts":"history/legacy-state/attempts","budget_ledger":cfg.budget.ledger}),
    )?;
    println!("migration_status: complete\nreuse: {}\nrebuild: {}\n旧隐藏目录与预算账本保留；请用公开命令生成新脚本，不重提旧脚本。",
        reused.join(", "), rebuild.join(", "));
    Ok(())
}
