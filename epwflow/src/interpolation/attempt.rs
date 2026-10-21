//! 原生粗／细尝试的持久检查点；只在显式恢复时归档失败并按原参数重试。
use crate::foundation::store::{self, Identity};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema_version: u32,
    id: String,
    digest: String,
    directory: PathBuf,
    state: State,
    inputs: Vec<Identity>,
    outputs: Vec<Identity>,
    archive: Option<PathBuf>,
    archive_had_directory: bool,
    #[serde(default = "epw_exitcode")]
    exit_code: String,
}

#[derive(Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum State {
    Preparing,
    Running,
    Complete,
    Archiving,
}

fn epw_exitcode() -> String { "epw.exitcode".into() }

fn exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn inputs(dir: &Path, result: &mut Vec<Identity>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let meta = fs::symlink_metadata(&path)?;
        ensure!(
            !meta.file_type().is_symlink(),
            "input_conflict: 物化输入不得包含链接"
        );
        if meta.is_file() && (crate::foundation::native_jobs::is_log(&path) || crate::foundation::workspace::slurm_log(&path)) { continue; }
        if meta.is_dir() {
            inputs(&path, result)?;
        } else {
            result.push(Identity::capture(&path)?);
        }
    }
    Ok(())
}

fn verify(ids: &[Identity]) -> Result<()> {
    for id in ids {
        id.verify()?;
    }
    Ok(())
}

fn verify_inputs(ids: &[Identity], dir: &Path) -> Result<()> {
    let mut prefix = None;
    if let Some(input) = ids.iter().find(|id| id.path == dir.join("epw_coarse.in")) {
        input.verify()?;
        let text = fs::read_to_string(&input.path)?;
        let out = regex::Regex::new(r"(?m)^\s*outdir\s*=\s*'\./out'\s*$")?;
        let name = regex::Regex::new(r"(?m)^\s*prefix\s*=\s*'([A-Za-z0-9_-]+)'\s*$")?;
        if out.is_match(&text) {
            prefix = name.captures(&text).map(|v| format!("{}.wfc", &v[1]));
        }
    }
    let out = dir.join("out");
    for id in ids {
        let scratch = id.path.parent() == Some(out.as_path()) && prefix.as_ref().is_some_and(|prefix| {
            id.path.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_prefix(prefix))
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()) && n.parse::<usize>().is_ok_and(|n| n > 0))
        });
        if !scratch && !crate::foundation::workspace::slurm_log(&id.path) { id.verify()?; }
    }
    Ok(())
}

fn checked_collect<T>(dir: &Path, exit_code: &str, collect: &impl Fn() -> Result<T>) -> Result<T> {
    ensure!(
        fs::read_to_string(dir.join(exit_code))?.trim() == "0",
        "execution_failed: 原生尝试没有成功退出证据"
    );
    collect()
}

fn complete<T>(
    receipt: &mut Receipt,
    target: &Path,
    result: &T,
    evidence: &impl Fn(&T) -> Result<Vec<Identity>>,
) -> Result<()> {
    receipt.outputs = evidence(result)?;
    ensure!(
        !receipt.outputs.is_empty(),
        "data_invalid: 原生成功证据为空"
    );
    receipt
        .outputs
        .push(Identity::capture(&receipt.directory.join(&receipt.exit_code))?);
    verify_inputs(&receipt.inputs, &receipt.directory)?;
    verify(&receipt.outputs)?;
    receipt.state = State::Complete;
    store::write(target, receipt)
}

/// 先记录归档目的，再移动；崩溃后可继续同一次归档，绝不覆盖未知目录。
fn archive(receipt: &mut Receipt, target: &Path, root: &Path, label: &str) -> Result<()> {
    if receipt.state != State::Archiving {
        let destination = root.join("history").join(format!("{label}-{}", receipt.id));
        ensure!(
            !exists(&destination),
            "input_conflict: 失败归档目标已有未知现场"
        );
        receipt.archive_had_directory = exists(&receipt.directory);
        receipt.state = State::Archiving;
        receipt.archive = Some(destination);
        store::write(target, receipt)?;
    }
    let destination = receipt
        .archive
        .as_ref()
        .context("data_invalid: 归档缺少目标")?;
    ensure!(
        destination.parent() == Some(root.join("history").as_path())
            && destination
                .file_name()
                .is_some_and(|n| n == format!("{label}-{}", receipt.id).as_str()),
        "input_conflict: 归档路径不属于当前尝试"
    );
    fs::create_dir_all(root.join("history"))?;
    if exists(&receipt.directory) {
        if crate::foundation::layout::enabled(receipt.directory.parent().context("data_invalid: 粗网格缺父目录")?) && label=="coarse" {
            fs::create_dir_all(destination)?;
            for entry in fs::read_dir(&receipt.directory)? {
                let entry=entry?;
                if ["quality-sources","quality-reference.json","quality-binding.json","quality-report.json"].iter().any(|n|entry.file_name()==*n) {continue}
                fs::rename(entry.path(),destination.join(entry.file_name()))?;
            }
            crate::foundation::workspace::preserve_active_logs(destination, &receipt.directory)?;
        }else{
            ensure!(!exists(destination),"input_conflict: 原现场与归档同时存在，保留现场");
            fs::rename(&receipt.directory, destination)?;
        }
    }
    ensure!(
        !receipt.archive_had_directory || exists(destination),
        "input_conflict: 原现场与已登记归档均缺失"
    );
    // 准备阶段可能尚未创建目录；仍保存检查点，避免丢失失败尝试。
    let history = destination.with_extension("json");
    if history.exists() {
        ensure!(
            serde_json::to_value(store::read::<Receipt>(&history)?)? == serde_json::to_value(&receipt)?,
            "input_conflict: 归档检查点发生变化"
        );
    } else {
        store::write(&history, receipt)?;
    }
    Ok(())
}

/// 每次显式恢复最多重新执行一次；完整输出可在清单发布中断后直接重验归集。
pub fn run<T>(
    work: &Path,
    label: &str,
    dir: &Path,
    digest: String,
    retry: bool,
    prepare: impl FnOnce() -> Result<()>,
    execute: impl FnOnce() -> Result<()>,
    collect: impl Fn() -> Result<T>,
    evidence: impl Fn(&T) -> Result<Vec<Identity>>,
) -> Result<T> {
    run_with_inputs(work, label, dir, digest, retry, "epw.exitcode", None,
        prepare, execute, collect, evidence)
}

/// QE 会改写工作保存树；只冻结显式只读输入，执行产物由归集器登记。
pub(super) fn run_with_inputs<T>(
    work: &Path, label: &str, dir: &Path, digest: String, retry: bool,
    exit_code: &str, selected_inputs: Option<&[PathBuf]>,
    prepare: impl FnOnce() -> Result<()>, execute: impl FnOnce() -> Result<()>,
    collect: impl Fn() -> Result<T>, evidence: impl Fn(&T) -> Result<Vec<Identity>>,
) -> Result<T> {
    ensure!(Path::new(exit_code).components().count() == 1
        && !exit_code.starts_with('.') && !exit_code.contains('/'),
        "input_conflict: 退出证据必须是普通文件名");
    ensure!(
        dir.starts_with(work)
            && !label.is_empty()
            && crate::foundation::paths::destination(dir)?
                == work.canonicalize()?.join(dir.strip_prefix(work)?)
            && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
        "input_conflict: 原生尝试路径无效"
    );
    let root = crate::foundation::layout::path(&work, "native-attempts");
    let target = root.join(format!("{label}.json"));
    if target.exists() {
        let mut previous: Receipt = store::read(&target)?;
        ensure!(
            previous.schema_version == 1
                && previous.exit_code == exit_code
                && previous.digest == digest
                && previous.directory == dir
                && !previous.id.is_empty()
                && previous
                    .id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-'),
            "input_conflict: 原生尝试的固定来源或参数变化"
        );
        ensure!(retry, "execution_failed: 原生尝试已有现场，请显式 --resume");
        if matches!(previous.state, State::Running | State::Complete) {
            ensure!(
                !previous.inputs.is_empty(),
                "data_invalid: 原生检查点缺少物化输入身份"
            );
        }
        if previous.state == State::Complete {
            ensure!(
                !previous.outputs.is_empty(),
                "data_invalid: 原生检查点缺少成功身份"
            );
        }
        if previous.state == State::Complete {
            verify_inputs(&previous.inputs, &previous.directory)?;
            verify(&previous.outputs)?;
            return checked_collect(dir, exit_code, &collect);
        }
        if previous.state == State::Running {
            verify_inputs(&previous.inputs, &previous.directory)?;
            if crate::foundation::native_jobs::enabled() && !crate::foundation::native_jobs::failed(dir)? {
                // 一个科学检查点可能包含 q2r→matdyn；逐个返回已登记报告，
                // 在下一个尚未执行的原生调用处暂停，不重新准备或归档现场。
                execute()?;
                let result=checked_collect(dir,exit_code,&collect)?;
                complete(&mut previous,&target,&result,&evidence)?;
                return Ok(result);
            }
            match checked_collect(dir, exit_code, &collect) {
                Ok(result) => {
                    complete(&mut previous, &target, &result, &evidence)?;
                    return Ok(result);
                }
                Err(e) if crate::foundation::native_jobs::enabled() && !crate::foundation::native_jobs::retry_enabled() => return Err(e),
                Err(_) => (),
            }
        }
        archive(&mut previous, &target, &root, label)?;
        if crate::foundation::native_jobs::enabled() { crate::foundation::native_jobs::retire(dir)?; }
    } else {
        ensure!(
            !exists(dir) || (crate::foundation::layout::enabled(work) && label=="coarse" && !dir.join("epw.out").exists() && !dir.join("out").exists()),
            "input_conflict: 原生目录缺少持久请求，保留未知现场"
        );
    }
    let mut receipt = Receipt {
        schema_version: 1,
        id: store::token(),
        digest,
        directory: dir.into(),
        state: State::Preparing,
        inputs: vec![],
        outputs: vec![],
        archive: None,
        archive_had_directory: false,
        exit_code: exit_code.into(),
    };
    store::write(&target, &receipt)?;
    prepare()?;
    if let Some(paths) = selected_inputs {
        receipt.inputs = paths.iter().map(|p| Identity::capture(p)).collect::<Result<_>>()?;
    } else { inputs(dir, &mut receipt.inputs)?; }
    ensure!(!receipt.inputs.is_empty(), "data_invalid: 原生物化输入为空");
    receipt.state = State::Running;
    store::write(&target, &receipt)?;
    execute()?;
    let result = checked_collect(dir, exit_code, &collect)?;
    complete(&mut receipt, &target, &result, &evidence)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coarse_scratch_is_mutable_but_portable_sources_and_other_files_are_not() {
        let dir = std::env::temp_dir().join(format!("coarse-scratch-{}", store::token()));
        fs::create_dir_all(dir.join("out/seed.save")).unwrap();
        fs::create_dir_all(dir.join("save")).unwrap();
        let input = dir.join("epw_coarse.in");
        let raw = dir.join("out/seed.wfc1");
        let portable = dir.join("out/seed.save/wfc1.dat");
        let ph = dir.join("save/seed.dvscf_q1");
        fs::write(&input, "&inputepw\n prefix = 'seed'\n outdir = './out'\n/\n").unwrap();
        for p in [&raw, &portable, &ph] { fs::write(p, "original science").unwrap(); }
        let ids = [&input, &raw, &portable, &ph].into_iter().map(|p| Identity::capture(p).unwrap()).collect::<Vec<_>>();
        fs::write(&raw, "native working copy changed").unwrap();
        verify_inputs(&ids, &dir).unwrap();
        fs::write(&portable, "portable corruption").unwrap();
        assert!(verify_inputs(&ids, &dir).is_err());
        fs::write(&portable, "original science").unwrap();
        fs::write(&ph, "response corruption").unwrap();
        assert!(verify_inputs(&ids, &dir).is_err());
        fs::write(&ph, "original science").unwrap();
        for path in [dir.join("out/other.wfc1"), dir.join("seed.wfc1")] {
            fs::write(&path, "before").unwrap();
            let extra = Identity::capture(&path).unwrap(); fs::write(&path, "changed").unwrap();
            let mut added = ids.clone(); added.push(extra);
            assert!(verify_inputs(&added, &dir).is_err());
        }
        fs::write(&input, "outdir = './other'\n prefix = 'seed'\n").unwrap();
        assert!(verify_inputs(&ids, &dir).is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn interrupted_archive_resumes_and_missing_evidence_is_rejected() {
        for missing in [false, true] {
            let work = std::env::temp_dir().join(format!("epw-archive-{}", store::token()));
            let root = crate::foundation::layout::path(&work, "native-attempts");
            let destination = root.join("history/coarse-test");
            fs::create_dir_all(&destination).unwrap();
            fs::write(destination.join("evidence"), "failed output").unwrap();
            let mut receipt = Receipt {
                schema_version: 1,
                id: "test".into(),
                digest: "fixed".into(),
                directory: crate::foundation::layout::path(&work, "coarse"),
                state: State::Archiving,
                inputs: vec![],
                outputs: vec![],
                archive: Some(destination.clone()),
                archive_had_directory: true,
                exit_code: epw_exitcode(),
            };
            if missing {
                fs::remove_dir_all(&destination).unwrap();
            }
            let result = archive(&mut receipt, &root.join("coarse.json"), &root, "coarse");
            if missing {
                assert!(result.is_err());
            } else {
                result.unwrap();
                archive(&mut receipt, &root.join("coarse.json"), &root, "coarse").unwrap();
                assert_eq!(
                    fs::read_to_string(destination.join("evidence")).unwrap(),
                    "failed output"
                );
                assert!(destination.with_extension("json").exists());
                // 旧 EPW 检查点没有 exit_code；默认值兼容不改写原归档。
                let history = destination.with_extension("json");
                let mut old: serde_json::Value = store::read(&history).unwrap();
                old.as_object_mut().unwrap().remove("exit_code");
                store::write(&history, &old).unwrap();
                let original = fs::read(&history).unwrap();
                archive(&mut receipt, &root.join("coarse.json"), &root, "coarse").unwrap();
                assert_eq!(fs::read(&history).unwrap(), original);
                old["digest"] = "changed".into();
                store::write(&history, &old).unwrap();
                assert!(archive(&mut receipt, &root.join("coarse.json"), &root, "coarse").is_err());
            }
            fs::remove_dir_all(work).unwrap();
        }
    }
}
