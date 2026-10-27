//! 旧可见状态的一次性转换。仅确认已退出的任务，不验证已归档的旧输入。
use super::{
    config::Config,
    readable_state as log,
    stages::{self, Step},
    store,
};
use anyhow::{ensure, Context, Result};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};
// layout 最后删除；转换中断后它仍是重试入口。
const OLD: &[&str] = &[
    "control.lock",
    "execution.lock",
    "steps",
    "electronic",
    "current-config.toml",
    "parameter-map.json",
    "active.json",
    "attempts",
    "native-active.json",
    "native-active-iso.json",
    "native-active-aniso.json",
    "native-jobs",
    "native-progress",
    "requests",
    "native-attempts",
    "qe-data",
    "coarse",
    "interpolation",
    "post",
    "history",
    "migration.json",
    "legacy-import.json",
    "layout.json",
];
/// 旧执行锁已不承担互斥。仅核对可识别的活动所有者，撤下已退出的固定旧锁。
pub(crate) fn retire_locks(state: &Path) -> Result<()> {
    for name in ["control.lock", "execution.lock"] {
        let path = state.join(name);
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.file_type().is_symlink() || metadata.is_file() {
            fs::remove_file(path)?;
            continue;
        }
        let owner: Option<Value> = fs::read(path.join("owner.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok());
        if let Some(job) = owner.as_ref().and_then(|v| v["job_id"].as_u64()) {
            ensure!(
                super::slurm::accounting(job)?.terminal,
                "scheduler_unknown: 旧锁所属作业 {job} 仍活动"
            );
        } else if let Some(owner) = &owner {
            if owner["host"].as_str() == Some(store::hostname().as_str()) {
                if let Some(pid) = owner["pid"].as_u64() {
                    let alive = pid > 0
                        && pid <= i32::MAX as u64
                        && (unsafe { libc::kill(pid as i32, 0) } == 0
                            || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM));
                    ensure!(!alive, "scheduler_unknown: 旧命令进程 {pid} 仍活动");
                }
            }
        }
        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            ensure!(
                entry.file_name() == "owner.json" && entry.file_type()?.is_file(),
                "input_conflict: 旧锁含未知内容 {}",
                entry.path().display()
            );
            fs::remove_file(entry.path())?;
        }
        fs::remove_dir(path)?;
    }
    Ok(())
}
pub struct Conversion {
    complete: Vec<Step>,
    leaves: Vec<(Step, String, Vec<PathBuf>)>,
    old: Vec<PathBuf>,
}
fn submission(value: &Value) -> Result<()> {
    let v = value.get("attempt").unwrap_or(value);
    if let Some(job) = v.get("job").and_then(Value::as_u64) {
        ensure!(
            super::slurm::accounting(job)?.terminal,
            "scheduler_unknown: 旧作业 {job} 仍活动，暂不转换或覆盖"
        );
    } else if let Some(state) = v.get("state").and_then(Value::as_str) {
        ensure!(
            !matches!(
                state,
                "submission_pending" | "unknown" | "running" | "submitted"
            ),
            "scheduler_unknown: 旧提交状态 {state} 尚无法确认，请核对 Slurm"
        );
    }
    Ok(())
}
pub fn prepare(cfg: &Config) -> Result<Option<Conversion>> {
    prepare_at(cfg, &cfg.work_dir.join("epw-state"))
}
pub fn prepare_at(cfg: &Config, state: &Path) -> Result<Option<Conversion>> {
    if !state.is_dir()
        || !OLD
            .iter()
            .filter(|n| !n.ends_with(".lock"))
            .any(|name| state.join(name).exists())
    {
        return Ok(None);
    }
    ensure!(
        !fs::symlink_metadata(state)?.file_type().is_symlink(),
        "input_conflict: 旧状态目录是链接 {}",
        state.display()
    );
    for entry in fs::read_dir(&state)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        ensure!(
            OLD.contains(&name.as_ref()) || name == "imports" || name.ends_with(".log"),
            "input_conflict: 旧状态存在尚未识别的内容 {}；保留现场，请先核对",
            entry.path().display()
        );
    }
    for name in [
        "active.json",
        "native-active.json",
        "native-active-iso.json",
        "native-active-aniso.json",
    ] {
        let path = state.join(name);
        if path.is_file() {
            submission(&store::read(&path)?)?;
        }
    }
    for name in ["attempts", "native-jobs"] {
        let root = state.join(name);
        if root.is_dir() {
            for f in crate::qe_data::save::files(&root)? {
                if f.relative_path.extension().is_some_and(|x| x == "json")
                    && !f.relative_path.starts_with("history")
                {
                    let value: Value = store::read(&root.join(f.relative_path))?;
                    submission(&value)?;
                }
            }
        }
    }
    let mut complete = vec![];
    let mut leaves = vec![];
    for step in Step::ALL {
        let native_receipt = stages::receipt(&cfg.work_dir, step);
        let central = state.join("steps").join(format!("{}.json", step.name()));
        let receipt = if native_receipt.is_file() {
            native_receipt
        } else {
            central
        };
        if receipt.is_file() {
            let value: Value = store::read(&receipt)?;
            if value["scientific_key"].as_str() == Some(stages::scientific_key(cfg, step)?.as_str())
            {
                complete.push(step);
            }
        }
        if step == Step::Electronic {
            let local = step.directory(cfg).join("receipts");
            let root = if local.is_dir() {
                local
            } else {
                state.join("electronic")
            };
            for task in ["pw-bands", "bands", "dos"] {
                let path = root.join(format!("{task}.json"));
                if path.is_file() {
                    let value: Value = store::read(&path)?;
                    let paths = value["outputs"]
                        .as_array()
                        .context("data_invalid: 旧电子子步骤缺产物")?
                        .iter()
                        .filter_map(|v| v["path"].as_str().map(PathBuf::from))
                        .filter(|p| p.is_file())
                        .collect::<Vec<_>>();
                    if !paths.is_empty() {
                        leaves.push((step, format!("电子子步骤 · {task}"), paths));
                    }
                }
            }
        }
    }
    Ok(Some(Conversion {
        complete,
        leaves,
        old: OLD
            .iter()
            .map(|name| state.join(name))
            .filter(|p| p.exists())
            .collect(),
    }))
}
pub fn finish(cfg: &Config, version: &Path, conversion: Conversion) -> Result<()> {
    for step in Step::ALL {
        log::update(&cfg.work_dir, step, |d| {
            if d.get("阶段", "批次").is_none() {
                d.set("阶段", "批次", format!("converted-{}", step.name()));
                d.set("阶段", "计算配置", log::relative(&cfg.work_dir, version));
            }
            d.set(
                "工作区",
                "旧状态转换",
                "已核对旧作业终态；旧输入路径不作为恢复前提",
            );
            Ok(())
        })?;
    }
    for (step, label, paths) in conversion.leaves {
        log::complete_files(cfg, step, &label, &paths)?;
    }
    for step in conversion.complete {
        match super::workspace_science::validate(cfg, step) {
            Ok(()) => {
                log::update(&cfg.work_dir, step, |d| {
                    d.set("阶段", "科学状态", "已完成");
                    d.set("阶段", "状态", "已完成");
                    Ok(())
                })?;
            }
            Err(e) => {
                log::update(&cfg.work_dir, step, |d| {
                    d.set("阶段", "状态", "旧结果需要检查");
                    d.set("问题", "原因", format!("{e:#}"));
                    Ok(())
                })?;
            }
        }
    }
    for step in Step::ALL {
        let deps = stages::upstream(cfg, step)?;
        log::update(&cfg.work_dir, step, |d| {
            for (dep, batch) in &deps {
                d.set("依赖", &format!("{dep} 批次"), batch);
            }
            Ok(())
        })?;
    }
    for step in Step::ALL {
        let receipt = stages::receipt(&cfg.work_dir, step);
        if receipt.exists() {
            fs::remove_file(receipt)?;
        }
        let root = step.directory(cfg).join("receipts");
        if root.is_dir() {
            fs::remove_dir_all(root)?;
        }
    }
    for path in conversion.old {
        let kind = fs::symlink_metadata(&path)?.file_type();
        if kind.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Config, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("legacy-0064-{}", store::token()));
        fs::create_dir_all(&root).unwrap();
        let mut cfg: Config =
            toml::from_str(include_str!("../../tests/fixtures/input.toml")).unwrap();
        cfg.work_dir = root.clone();
        log::initialize(&cfg).unwrap();
        let state = root.join("epw-state");
        store::write(
            &state.join("layout.json"),
            &serde_json::json!({"schema_version":1,"phonon":"index"}),
        )
        .unwrap();
        (cfg, root, state)
    }
    #[test]
    fn interrupted_cleanup_keeps_layout_until_retry_succeeds() {
        let (cfg, root, state) = fixture();
        let first = state.join("current-config.toml");
        fs::write(&first, "obsolete").unwrap();
        let version = root.join("0000-config.toml");
        fs::write(&version, "raw user input").unwrap();
        let conversion = Conversion {
            complete: vec![],
            leaves: vec![],
            old: vec![
                first.clone(),
                state.join("missing"),
                state.join("layout.json"),
            ],
        };
        assert!(finish(&cfg, &version, conversion).is_err());
        assert!(!first.exists());
        assert!(state.join("layout.json").exists());
        let retry = prepare(&cfg).unwrap().unwrap();
        finish(&cfg, &version, retry).unwrap();
        assert_eq!(fs::read_dir(&state).unwrap().count(), 8);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn missing_old_input_is_ignored_but_unknown_submission_is_preserved() {
        let (cfg, root, state) = fixture();
        let path = state.join("native-active.json");
        store::write(
            &path,
            &serde_json::json!({"state":"failed","inputs":[{"path":"/missing/archive/scf.in"}]}),
        )
        .unwrap();
        assert!(prepare(&cfg).unwrap().is_some());
        store::write(&path, &serde_json::json!({"state":"unknown"})).unwrap();
        assert!(prepare(&cfg).is_err());
        assert!(path.exists());
        fs::remove_dir_all(root).unwrap();
    }
}
