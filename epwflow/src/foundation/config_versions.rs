//! 原文配置集中在 epw-state；解析目录随版本保存，迁移不改变科学输入。
use super::{paths, readable_state as log, stages::Step, store};
use anyhow::{ensure, Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn number(path: &Path) -> Option<u64> {
    let n = path.file_name()?.to_str()?.strip_suffix("-config.toml")?;
    (n.len() >= 4 && n.bytes().all(|c| c.is_ascii_digit()))
        .then(|| n.parse().ok())
        .flatten()
}
fn section(path: &Path) -> String {
    format!("配置快照 · {}", path.file_name().unwrap().to_string_lossy())
}

/// 原文未展开；仅登记过的状态目录编号文件采用原输入目录。
pub(crate) fn base(path: &Path) -> Result<PathBuf> {
    let parent = path.parent().context("configuration: 配置缺少父目录")?;
    if parent.file_name().is_none_or(|n| n != "epw-state") || number(path).is_none() {
        return Ok(parent.into());
    }
    ensure!(
        !fs::symlink_metadata(path)?.file_type().is_symlink(),
        "input_conflict: 编号配置不能是链接"
    );
    let root = parent
        .parent()
        .context("configuration: 状态目录缺少工作区")?;
    let doc = log::read(root, Step::Scf)?;
    let label = section(path);
    let origin = PathBuf::from(
        doc.get(&label, "原始目录")
            .context("data_invalid: 编号配置缺原始目录记录")?,
    );
    ensure!(
        origin.is_absolute(),
        "data_invalid: 配置原始目录必须为绝对路径"
    );
    ensure!(
        doc.get(&label, "原文 SHA-256") == Some(store::hash_file(path)?.as_str()),
        "input_conflict: 编号配置内容与来源记录不符 {}",
        path.display()
    );
    Ok(origin)
}

fn versions(dir: &Path) -> Result<Vec<(u64, PathBuf)>> {
    let mut result = vec![];
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if let Some(n) = number(&entry.path()) {
            ensure!(
                entry.file_type()?.is_file(),
                "input_conflict: 编号配置必须是独立文件 {}",
                entry.path().display()
            );
            result.push((n, entry.path()));
        }
    }
    result.sort_by_key(|(n, _)| *n);
    ensure!(
        result.windows(2).all(|w| w[0].0 != w[1].0),
        "input_conflict: 配置版本编号重复"
    );
    Ok(result)
}

/// 仅入口预存的原文与来源记录不构成已初始化的旧式运行目录。
pub(crate) fn only_versions(root: &Path) -> Result<bool> {
    if !root.is_dir() {
        return Ok(false);
    }
    for entry in fs::read_dir(root)? {
        if entry?.file_name() != "epw-state" {
            return Ok(false);
        }
    }
    let state = root.join("epw-state");
    if !state.is_dir() {
        return Ok(false);
    }
    for entry in fs::read_dir(&state)? {
        let entry = entry?;
        if entry.file_name() != "1.qe-scf.log" && number(&entry.path()).is_none() {
            return Ok(false);
        }
    }
    let doc = log::read(root, Step::Scf)?;
    if doc.sections.iter().any(|(name, fields)| {
        !name.starts_with("配置快照 · ")
            && !(name == "工作区" && fields.keys().all(|k| k == "更新时间"))
    }) {
        return Ok(false);
    }
    let versions = versions(&state)?;
    for (_, path) in &versions {
        ensure!(
            work_dir(&fs::read(path)?, &base(path)?)? == root,
            "input_conflict: 预存配置的工作区归属不符"
        );
    }
    Ok(!versions.is_empty())
}
fn work_dir(raw: &[u8], origin: &Path) -> Result<PathBuf> {
    let value: toml::Value = toml::from_str(std::str::from_utf8(raw)?)?;
    let work = value
        .get("work_dir")
        .map(|v| {
            v.as_str()
                .context("configuration: work_dir 必须是路径字符串")
        })
        .transpose()?;
    paths::destination(&paths::absolute(origin, Path::new(work.unwrap_or(".")))?)
}
fn record(root: &Path, output: &Path, origin: &Path, raw: &[u8]) -> Result<()> {
    log::update(root, Step::Scf, |doc| {
        doc.set(&section(output), "原始目录", origin.display());
        doc.set(&section(output), "原文 SHA-256", store::hash_bytes(raw));
        Ok(())
    })
}
fn publish(root: &Path, output: &Path, origin: &Path, raw: &[u8]) -> Result<()> {
    // 先登记后原子落盘；中断不会留下缺少解析目录的已发布版本。
    record(root, output, origin, raw)?;
    store::atomic_bytes(output, raw)
}
fn same_existing(output: &Path, origin: &Path, raw: &[u8]) -> Result<()> {
    ensure!(
        fs::read(output)? == raw,
        "input_conflict: 配置迁移目标内容冲突 {}",
        output.display()
    );
    ensure!(
        base(output)? == origin,
        "input_conflict: 配置来源目录冲突 {}",
        output.display()
    );
    Ok(())
}

fn migration_idle(root: &Path) -> Result<()> {
    for step in Step::ALL {
        let doc = log::read(root, step)?;
        let jobs = log::jobs(root, step)?;
        for (name, fields) in &doc.sections {
            if fields.get("状态").is_some_and(|s| {
                matches!(
                    s.as_str(),
                    "提交中"
                        | "提交未知"
                        | "已提交"
                        | "运行中"
                        | "submitted"
                        | "running"
                        | "submitting"
                        | "submission_unknown"
                )
            }) {
                ensure!(
                    jobs.contains_key(name),
                    "scheduler_unknown: 旧配置仍被 {name} 使用且缺少作业号，保留原位置"
                );
            }
        }
        for (_, job) in jobs {
            ensure!(
                super::slurm::accounting(job)?.terminal,
                "scheduler_unknown: 旧配置所属作业 {job} 仍活动，完成后再迁入 epw-state"
            );
        }
    }
    Ok(())
}

fn recover_only_current(
    root: &Path,
    input: &Path,
    version: &Path,
    origin: &Path,
    raw: &[u8],
) -> Result<()> {
    // 日志整体丢失时，只能用其他日志对唯一版本及原输入的明确绑定恢复；不推测历史来源。
    let mut found = false;
    for step in Step::ALL.into_iter().filter(|s| *s != Step::Scf) {
        let doc = log::read(root, step)?;
        if doc.get("工作区", "当前输入版本") == Some(log::relative(root, version).as_str())
        {
            let source = doc
                .get("工作区", "当前输入")
                .context("data_invalid: 现存日志缺少配置来源，不能恢复配置记录")?;
            ensure!(
                log::resolve(root, source) == input,
                "input_conflict: 现存日志的配置来源不同，不能恢复配置记录"
            );
            found = true;
        }
    }
    ensure!(found, "data_invalid: 配置来源记录缺失且没有其他日志证据");
    migration_idle(root)?;
    record(root, version, origin, raw)
}

fn migrate(root: &Path, origin: &Path, input: &Path) -> Result<()> {
    let state = root.join("epw-state");
    if origin == state {
        return Ok(());
    }
    let mut old = vec![];
    for (_, path) in versions(origin)? {
        let raw = fs::read(&path)?;
        if work_dir(&raw, origin)? == root {
            ensure!(
                path != input,
                "input_conflict: 迁移旧编号配置时请通过原始 --config 输入文件启动"
            );
            let output = state.join(path.file_name().unwrap());
            if output.exists() {
                same_existing(&output, origin, &raw)?;
            }
            old.push((path, output, raw));
        }
    }
    if old.is_empty() {
        return Ok(());
    }
    migration_idle(root)?;
    // 旧 JSON 布局仍需确认其中的活动／未知作业，不能只检查新任务号表。
    if root.join("epw-state/layout.json").exists() || root.join(".epwflow").exists() {
        let cfg = super::config::Config::load(input)
            .or_else(|_| super::config::Config::load_nested(input))?;
        let _ = super::legacy_state::prepare(&cfg)?;
        if root.join(".epwflow").exists() {
            let _ = super::legacy_state::prepare_at(&cfg, &root.join(".epwflow"))?;
        }
    }
    // 全部内容及日志先检查，再写目标；失败时原文件仍保留。
    for step in Step::ALL {
        log::read(root, step)?;
    }
    for (_, output, raw) in &old {
        if !output.exists() {
            publish(root, output, origin, raw)?;
        }
    }
    for step in Step::ALL {
        log::update(root, step, |doc| {
            for fields in doc.sections.values_mut() {
                for value in fields.values_mut() {
                    for (source, output, _) in &old {
                        if *value == log::relative(root, source)
                            || *value == source.to_string_lossy()
                        {
                            *value = log::relative(root, output);
                            break;
                        }
                    }
                }
            }
            Ok(())
        })?;
    }
    for (source, _, raw) in &old {
        ensure!(
            fs::read(source)? == *raw,
            "input_conflict: 迁移期间旧配置发生变化，保留原文件"
        );
    }
    for (source, _, _) in old {
        fs::remove_file(source)?;
    }
    fs::File::open(origin)?.sync_all()?;
    Ok(())
}

pub(crate) fn backup(input: &Path) -> Result<PathBuf> {
    let input = paths::absolute(&std::env::current_dir()?, input)?;
    let origin = base(&input)?;
    let raw =
        fs::read(&input).with_context(|| format!("configuration: 无法读取 {}", input.display()))?;
    let root = work_dir(&raw, &origin)?;
    let state = root.join("epw-state");
    ensure!(
        !fs::symlink_metadata(&state).is_ok_and(|m| m.file_type().is_symlink()),
        "input_conflict: 状态目录不能是链接 {}",
        state.display()
    );
    let _guard = log::DirectoryGuard::acquire(&state)?;
    migrate(&root, &origin, &input)?;
    let versions = versions(&state)?;
    if let Some((_, last)) = versions.last() {
        if fs::read(last)? == raw {
            if versions.len() == 1 && !log::path(&root, Step::Scf)?.exists() {
                recover_only_current(&root, &input, last, &origin, &raw)?;
            }
            same_existing(last, &origin, &raw)?;
            return Ok(last.clone());
        }
        base(last)?;
    }
    let next = versions
        .last()
        .map(|(n, _)| n.checked_add(1).context("data_invalid: 配置版本编号溢出"))
        .transpose()?
        .unwrap_or(0);
    let output = state.join(format!("{next:04}-config.toml"));
    publish(&root, &output, &origin, &raw)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn root() -> PathBuf {
        let p = std::env::temp_dir().join(format!("config-versions-{}", store::token()));
        fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn snapshots_keep_origin_with_external_config_directory_and_spaces() {
        let root = root();
        let source = root.join("input space");
        fs::create_dir_all(&source).unwrap();
        let input = source.join("custom.toml");
        fs::write(&input, "work_dir = '../work'\nstructure='POSCAR'\n").unwrap();
        let path = backup(&input).unwrap();
        assert_eq!(path, root.join("work/epw-state/0000-config.toml"));
        assert_eq!(base(&path).unwrap(), source);
        assert_eq!(fs::read(&path).unwrap(), fs::read(&input).unwrap());
        assert_eq!(backup(&input).unwrap(), path);
        assert_eq!(backup(&path).unwrap(), path);
        assert!(only_versions(&root.join("work")).unwrap());
        assert!(!super::super::layout::enabled(&root.join("work")));
        fs::write(root.join("work/unknown"), "keep").unwrap();
        assert!(!only_versions(&root.join("work")).unwrap());
        fs::remove_file(root.join("work/unknown")).unwrap();
        fs::write(&path, "work_dir = '../work'\nstructure='different'\n").unwrap();
        assert!(base(&path).is_err());
        assert!(backup(&input).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn concurrent_identical_observations_publish_one_complete_version() {
        let root = root();
        let input = root.join("input.toml");
        fs::write(&input, "# original\nx=1\n").unwrap();
        std::thread::scope(|scope| {
            let joins: Vec<_> = (0..8)
                .map(|_| scope.spawn(|| backup(&input).unwrap()))
                .collect();
            for j in joins {
                assert_eq!(j.join().unwrap(), root.join("epw-state/0000-config.toml"));
            }
        });
        assert_eq!(versions(&root.join("epw-state")).unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn legacy_versions_migrate_all_references_and_recover_after_copy_interruption() {
        let root = root();
        let input = root.join("input.toml");
        let old = root.join("0000-config.toml");
        fs::write(&input, "x=2\n").unwrap();
        fs::write(&old, "x=1\n").unwrap();
        for step in Step::ALL {
            log::update(&root, step, |d| {
                d.set("阶段", "计算配置", "0000-config.toml");
                d.set("子任务", "使用配置", old.display());
                d.set("阶段", "批次", "same-science");
                Ok(())
            })
            .unwrap();
        }
        let new = root.join("epw-state/0000-config.toml");
        // 模拟已经复制、尚未完成引用转换时中断。
        publish(&root, &new, &root, b"x=1\n").unwrap();
        assert_eq!(
            backup(&input).unwrap(),
            root.join("epw-state/0001-config.toml")
        );
        assert!(!old.exists());
        assert_eq!(fs::read(&new).unwrap(), b"x=1\n");
        for step in Step::ALL {
            let d = log::read(&root, step).unwrap();
            assert_eq!(
                d.get("阶段", "计算配置"),
                Some("epw-state/0000-config.toml")
            );
            assert_eq!(
                d.get("子任务", "使用配置"),
                Some("epw-state/0000-config.toml")
            );
            assert_eq!(d.get("阶段", "批次"), Some("same-science"));
        }
        assert_eq!(
            backup(&input).unwrap(),
            root.join("epw-state/0001-config.toml")
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn legacy_conflict_or_unknown_job_keeps_original_versions() {
        let root = root();
        let input = root.join("input.toml");
        let old = root.join("0000-config.toml");
        fs::write(&input, "x=1\n").unwrap();
        fs::write(&old, "x=1\n").unwrap();
        log::update(&root, Step::Scf, |d| {
            d.set("任务 · scf", "状态", "提交未知");
            Ok(())
        })
        .unwrap();
        assert!(format!("{:#}", backup(&input).unwrap_err()).contains("scheduler_unknown"));
        assert!(old.is_file());
        assert!(!root.join("epw-state/0000-config.toml").exists());
        log::update(&root, Step::Scf, |d| {
            d.set("任务 · scf", "状态", "失败");
            Ok(())
        })
        .unwrap();
        publish(
            &root,
            &root.join("epw-state/0000-config.toml"),
            &root,
            b"x=9\n",
        )
        .unwrap();
        assert!(format!("{:#}", backup(&input).unwrap_err()).contains("内容冲突"));
        assert_eq!(fs::read(old).unwrap(), b"x=1\n");
        fs::remove_dir_all(root).unwrap();
    }
}
