//! 显式清理三类历史；不加载科学配置，不改写现用记录和预算。
use super::{layout::Layout, slurm, status, store};
use anyhow::{bail, ensure, Context, Result};
use clap::Parser;
use serde_json::Value;
use std::{
    collections::BTreeSet,
    fs, io,
    path::{Component, Path, PathBuf},
    process::Command,
};

const HISTORIES: [&str; 3] = [
    "epw-state/history",
    "epw-state/native-attempts/history",
    "epw-state/native-jobs/history",
];

#[derive(Parser)]
#[command(about = "清理 EPW 三类历史，默认仅预览；--yes 实际删除")]
struct Options {
    /// 编号工作区根目录，不要求完整科学配置。
    #[arg(short, long, default_value = ".")]
    work: PathBuf,
    /// 仅列出清理清单，不修改工作区。
    #[arg(long, conflicts_with = "yes")]
    dry_run: bool,
    /// 实际删除三处 history 的全部内容，保留现用计算与记录。
    #[arg(long)]
    yes: bool,
}

pub fn run() {
    let options = Options::parse();
    if let Err(error) = clear(&options.work, options.yes) {
        status::fail(error);
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Count {
    directories: u64,
    files: u64,
    bytes: u64,
}

fn metadata(path: &Path) -> Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(meta) => Ok(Some(meta)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("无法读取 {}", path.display())),
    }
}

fn children(path: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = fs::read_dir(path)
        .with_context(|| format!("无法列出 {}", path.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<io::Result<Vec<_>>>()?;
    paths.sort();
    Ok(paths)
}

/// 逐级检查，不能让中间目录的链接把固定清理范围转向工作区外。
fn validate_roots(work: &Path) -> Result<Vec<PathBuf>> {
    let mut roots = Vec::new();
    for relative in HISTORIES {
        let mut path = work.to_path_buf();
        for part in Path::new(relative) {
            path.push(part);
            let Some(meta) = metadata(&path)? else { break };
            ensure!(
                meta.is_dir() && !meta.file_type().is_symlink(),
                "input_conflict: 清理路径必须是普通目录，拒绝链接或文件 {}",
                path.display()
            );
        }
        roots.push(work.join(relative));
    }
    Ok(roots)
}

fn count(path: &Path, result: &mut Count) -> Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.is_dir() {
        result.directories += 1;
        for child in children(path)? {
            count(&child, result)?;
        }
    } else {
        ensure!(
            meta.is_file() || meta.file_type().is_symlink(),
            "input_conflict: 历史含不支持的文件类型 {}",
            path.display()
        );
        result.files += 1;
        result.bytes += meta.len();
    }
    Ok(())
}

fn inventory(roots: &[PathBuf]) -> Result<Vec<Option<Count>>> {
    roots
        .iter()
        .map(|root| {
            if metadata(root)?.is_none() {
                return Ok(None);
            }
            let mut result = Count::default();
            for child in children(root)? {
                count(&child, &mut result)?;
            }
            Ok(Some(result))
        })
        .collect()
}

/// 不要求被引用文件仍存在，也能识别相对路径与 ..。
fn normalized(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => (),
            Component::ParentDir => {
                result.pop();
            }
            part => result.push(part.as_os_str()),
        }
    }
    result
}

fn reference(value: &str, source: &Path, work: &Path, roots: &[PathBuf]) -> Option<PathBuf> {
    // 完整绝对路径先匹配，支持脚本引号内带空格的路径。
    for root in roots {
        let text = root.to_str()?;
        if value.match_indices(text).any(|(index, _)| {
            value[index + text.len()..]
                .chars()
                .next()
                .is_none_or(|c| c == '/' || c.is_whitespace() || ['\'', '"', ';', ','].contains(&c))
        }) {
            return Some(root.clone());
        }
    }
    for token in std::iter::once(value)
        .chain(value.split(['\'', '"', '\n', ' ', '\t', '=', ';', ',', '(', ')']))
    {
        if token.is_empty() {
            continue;
        }
        let path = Path::new(token);
        for base in [source.parent().unwrap_or(work), work] {
            let path = normalized(&base.join(path));
            if roots.iter().any(|root| path.starts_with(root)) {
                return Some(path);
            }
        }
    }
    None
}

fn check_value(
    value: &Value,
    source: &Path,
    work: &Path,
    roots: &[PathBuf],
    jobs: &mut BTreeSet<u64>,
) -> Result<()> {
    match value {
        Value::Object(fields) => {
            // attempts 中是旧提交快照，作业状态由现用 active／native-jobs 记录负责。
            if fields.contains_key("job")
                && fields.contains_key("state")
                && !source.starts_with(work.join("epw-state/attempts"))
            {
                let state = fields
                    .get("state")
                    .and_then(Value::as_str)
                    .with_context(|| {
                        format!("data_invalid: 作业记录缺少状态 {}", source.display())
                    })?;
                match state {
                    "validated" | "succeeded" | "complete" | "completed" | "failed"
                    | "cancelled" | "stopped" => (),
                    "prepared" if fields["job"].is_null() => (),
                    "running" | "submitted" | "awaiting_validation" => {
                        let job =
                            fields["job"]
                                .as_u64()
                                .filter(|job| *job > 0)
                                .with_context(|| {
                                    format!(
                                        "scheduler_unknown: {} 状态为 {state}，缺少可核查作业号",
                                        source.display()
                                    )
                                })?;
                        jobs.insert(job);
                    }
                    _ => bail!(
                        "scheduler_unknown: {} 状态为 {state}，拒绝清理",
                        source.display()
                    ),
                }
            }
            for child in fields.values() {
                check_value(child, source, work, roots, jobs)?;
            }
        }
        Value::Array(values) => {
            for child in values {
                check_value(child, source, work, roots, jobs)?;
            }
        }
        Value::String(value) => {
            if let Some(target) = reference(value, source, work, roots) {
                bail!(
                    "input_conflict: 现用记录 {} 引用历史 {}",
                    source.display(),
                    target.display()
                );
            }
        }
        _ => (),
    }
    Ok(())
}

fn scan_live(
    path: &Path,
    work: &Path,
    roots: &[PathBuf],
    jobs: &mut BTreeSet<u64>,
    current_attempt: Option<&Path>,
) -> Result<()> {
    if roots.iter().any(|root| path == root) {
        return Ok(());
    }
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() {
        ensure!(
            !path
                .file_name()
                .is_some_and(|name| name == "active.json"
                    || name.to_string_lossy().starts_with("native-active"))
                && path.parent() != Some(work.join("epw-state/native-jobs").as_path()),
            "input_conflict: 现用作业记录不能是符号链接 {}",
            path.display()
        );
        let link = fs::read_link(path)?;
        let target = normalized(&path.parent().unwrap_or(work).join(link));
        let resolved = fs::canonicalize(path).unwrap_or(target.clone());
        ensure!(
            !roots
                .iter()
                .any(|root| target.starts_with(root) || resolved.starts_with(root)),
            "input_conflict: 现用链接 {} 引用历史 {}",
            path.display(),
            target.display()
        );
    } else if meta.is_dir() {
        for child in children(path)? {
            // 旧尝试仅供追溯；只有 active 正在绑定的快照参与现用依赖检查。
            if path == work.join("epw-state/attempts")
                && !current_attempt.is_some_and(|attempt| attempt.starts_with(&child))
            {
                continue;
            }
            scan_live(&child, work, roots, jobs, current_attempt)?;
        }
    } else if meta.is_file() {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("json") => {
                let mut value: Value = store::read(path)?;
                if path.file_name().is_some_and(|name| {
                    name == "active.json" || name.to_string_lossy().starts_with("native-active")
                }) || path.parent() == Some(work.join("epw-state/native-jobs").as_path())
                {
                    ensure!(
                        value.get("job").is_some() && value.get("state").is_some(),
                        "data_invalid: 活动记录不完整 {}",
                        path.display()
                    );
                }
                // 声子链提交端使用独立回执，不能把有明确作业号的 pending 快照误判为未知。
                if path == work.join("epw-state/active.json")
                    && value["phonon_chain"] == true
                    && value["job"].is_null()
                    && value["state"] == "submission_pending"
                {
                    let config = value["config"]["path"]
                        .as_str()
                        .context("data_invalid: 活动尝试缺少配置快照")?;
                    let receipt = Path::new(config).with_file_name("submission.json");
                    ensure!(
                        receipt.starts_with(work),
                        "input_conflict: 提交回执不属于工作区"
                    );
                    let submitted: Value = store::read(&receipt)?;
                    if submitted["job"].as_u64().is_some_and(|job| job > 0) {
                        value["job"] = submitted["job"].clone();
                        value["state"] = Value::String("submitted".into());
                    }
                }
                check_value(&value, path, work, roots, jobs)?;
            }
            Some("toml" | "sh") => {
                let text = fs::read_to_string(path)?;
                if let Some(target) = reference(&text, path, work, roots) {
                    bail!(
                        "input_conflict: 现用文件 {} 引用历史 {}",
                        path.display(),
                        target.display()
                    );
                }
            }
            _ => (),
        }
    }
    Ok(())
}

fn check_queue(work: &Path) -> Result<()> {
    let output = match Command::new("squeue")
        .args(["--noheader", "--me", "--format=%i|%Z"])
        .output()
    {
        Ok(output) => output,
        // 没有 Slurm 的机器仍可清理已终止的本地工作区；有未决作业时 accounting 会拒绝。
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error).context("scheduler_unknown: 无法查询 squeue"),
    };
    ensure!(
        output.status.success(),
        "scheduler_unknown: squeue 查询失败：{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for line in String::from_utf8(output.stdout)?
        .lines()
        .filter(|line| !line.trim().is_empty())
    {
        let (job, directory) = line
            .split_once('|')
            .context("scheduler_unknown: squeue 缺少作业工作目录")?;
        let directory = Path::new(directory.trim());
        let resolved = fs::canonicalize(directory).unwrap_or_else(|_| normalized(directory));
        ensure!(
            !resolved.starts_with(work),
            "scheduler_unknown: 作业 {} 正在使用工作区 {}",
            job.trim(),
            directory.display()
        );
    }
    Ok(())
}

fn preflight(work: &Path, roots: &[PathBuf]) -> Result<()> {
    validate_roots(work)?;
    let cwd = std::env::current_dir()?;
    ensure!(
        !roots.iter().any(|root| cwd.starts_with(root)),
        "input_conflict: 当前终端位于待清理历史内，请先进入工作区根目录"
    );
    let active = work.join("epw-state/active.json");
    let current_attempt = if let Some(meta) = metadata(&active)? {
        ensure!(
            meta.is_file() && !meta.file_type().is_symlink(),
            "input_conflict: 现用作业记录必须是普通文件 {}",
            active.display()
        );
        let value: Value = store::read(&active)?;
        value["config"]["path"]
            .as_str()
            .map(|path| normalized(&work.join(path)))
    } else {
        None
    };
    let mut jobs = BTreeSet::new();
    scan_live(work, work, roots, &mut jobs, current_attempt.as_deref())?;
    for job in jobs {
        ensure!(
            slurm::accounting(job)?.terminal,
            "scheduler_unknown: 作业 {job} 仍活动，拒绝清理"
        );
    }
    check_queue(work)
}

fn remove_contents(root: &Path) -> Vec<String> {
    let paths = match children(root) {
        Ok(paths) => paths,
        Err(error) => return vec![format!("{error:#}")],
    };
    let mut failures = Vec::new();
    for path in paths {
        let result = (|| -> io::Result<()> {
            if fs::symlink_metadata(&path)?.is_dir() {
                fs::remove_dir_all(&path)
            } else {
                fs::remove_file(&path)
            }
        })();
        if let Err(error) = result {
            failures.push(format!("{}: {error}", path.display()));
        }
    }
    failures
}

pub fn clear(path: &Path, delete: bool) -> Result<()> {
    let work = fs::canonicalize(path).context("configuration: 工作区不存在或不可读")?;
    if work.join("epw-state/1.qe-scf.log").is_file() && !work.join("epw-state/layout.json").exists()
    {
        println!(
            "{}: eight-stage log layout does not generate history directories; no cleanup needed",
            work.display()
        );
        return Ok(());
    }
    let roots = validate_roots(&work)?;
    let layout: Layout = store::read(&work.join("epw-state/layout.json"))?;
    ensure!(
        layout.schema_version == 1,
        "unsupported_version: 工作区布局版本"
    );
    let before = inventory(&roots)?;
    preflight(&work, &roots)?;
    println!(
        "{}: {}",
        if delete {
            "Cleanup"
        } else {
            "Read-only preview"
        },
        work.display()
    );
    for (root, count) in roots.iter().zip(&before) {
        match count {
            Some(count) => println!(
                "{}: {} directories, {} files/links, {} logical bytes",
                root.display(),
                count.directories,
                count.files,
                count.bytes
            ),
            None => println!("{}: absent; no cleanup needed", root.display()),
        }
    }
    if !delete {
        println!("No files changed; use --yes to clean the three history locations listed above");
        return Ok(());
    }
    // 预检可能耗时；删除前重验活动记录和目标，不能沿用过期的清单。
    preflight(&work, &roots)?;
    ensure!(
        inventory(&roots)? == before,
        "input_conflict: 预检期间历史发生变化，请重新执行"
    );
    let mut failures = Vec::new();
    for (root, old) in roots.iter().zip(&before) {
        if old.is_none() {
            continue;
        }
        failures.extend(remove_contents(root));
        match children(root) {
            Ok(remaining) if remaining.is_empty() => println!("Cleared {}", root.display()),
            Ok(remaining) => failures.extend(
                remaining
                    .iter()
                    .map(|path| format!("未删除 {}", path.display())),
            ),
            Err(error) => failures.push(format!("{error:#}")),
        }
    }
    ensure!(
        failures.is_empty(),
        "execution_failed: 历史清理未全部完成：\n{}",
        failures.join("\n")
    );
    println!("History cleanup complete; current calculations, results, task records and budget unchanged");
    Ok(())
}
