//! 指定温度按方法提交：每个方法一个原生多温度作业，逐点证据沿用既有交付。
use super::{
    attempt::{self, AttemptResult, GapStatus, NativeInputs, Request},
    engine::Entry,
    gap::{self, GapData},
    parse::{self, SolveEvidence},
    points::Run,
};
use crate::foundation::{
    config::{Config, Mode},
    native_jobs::{self, Task},
    stages::Step,
    store::{self, Identity},
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Batch {
    pub mode: Mode,
    pub requests: Vec<Request>,
    pub task: Task,
    pub executable: Identity,
    pub collected: bool,
}

/// 输入和完整日志快照不可变；原生逐温度文件同时保留在方法目录。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub log: Identity,
    pub temperatures: Vec<i64>,
    pub gap_file: Option<Identity>,
}

fn method(mode: Mode) -> &'static str {
    if mode == Mode::Iso {
        "iso"
    } else {
        "aniso"
    }
}
fn decimal(t: i64) -> String {
    (t as f64 / 100.0).to_string()
}
pub fn directory_name(cfg: &Config, mode: Mode) -> String {
    let ts = &cfg.superconductivity.gap_temperatures_k;
    format!(
        "{}-mu{}-{}-{}",
        method(mode),
        cfg.superconductivity.mu_star,
        ts[0],
        ts[ts.len() - 1]
    )
}

pub fn render(requests: &[Request], source: &NativeInputs) -> Result<String> {
    ensure!(
        !requests.is_empty() && requests.len() <= 50,
        "configuration: 原生批次需要 1～50 个温度"
    );
    let mut text = attempt::render(&requests[0], source)?;
    let single = format!(
        " nstemp = 1\n temps(1) = {:.2}\n",
        requests[0].temperature_centikelvin as f64 / 100.0
    );
    let mut temperatures = format!(" nstemp = {}\n", requests.len());
    for (i, request) in requests.iter().enumerate() {
        request.validate()?;
        let mut comparable = request.clone();
        comparable.temperature_centikelvin = requests[0].temperature_centikelvin;
        ensure!(
            serde_json::to_value(comparable)? == serde_json::to_value(&requests[0])?,
            "configuration: 同方法批次混用求解设置"
        );
        ensure!(
            !requests[..i]
                .iter()
                .any(|q| q.temperature_centikelvin == request.temperature_centikelvin),
            "configuration: 批次温度重复"
        );
        temperatures.push_str(&format!(
            " temps({}) = {:.2}\n",
            i + 1,
            request.temperature_centikelvin as f64 / 100.0
        ));
    }
    ensure!(text.contains(&single), "data_invalid: 原生温度输入模板缺失");
    text = text.replacen(&single, &temperatures, 1);
    Ok(text.replacen("single-temperature solve", "multi-temperature solve", 1))
}

/// 真实 itemp／温度先与固定批次核对；随后只把索引归一化交给既有单段解析器。
fn segment(log: &str, temperatures: &[i64], request: &Request) -> Result<Option<String>> {
    let header = regex::Regex::new(r"(?m)^\s*temp\(\s*(\d+)\)\s*=\s*([0-9.]+)\s+K\s*$")?;
    let blocks: Vec<_> = header.captures_iter(log).collect();
    ensure!(
        blocks.len() <= temperatures.len(),
        "data_invalid: 批次含额外温度"
    );
    let opposite = if request.mode == Mode::Iso {
        "anisotropic"
    } else {
        "isotropic"
    };
    ensure!(
        !log.contains(&format!("Solve {opposite} Eliashberg equations"))
            && !log.contains("linearized Eliashberg"),
        "data_invalid: 批次模式错误"
    );
    for (i, block) in blocks.iter().enumerate() {
        ensure!(
            block[1].parse::<usize>()? == i + 1
                && (block[2].parse::<f64>()? * 100.0 - temperatures[i] as f64).abs() < 1e-5,
            "data_invalid: 批次实际温度或索引不符"
        );
    }
    let i = temperatures
        .iter()
        .position(|&t| t == request.temperature_centikelvin)
        .context("data_invalid: 点不属于批次")?;
    let Some(block) = blocks.get(i) else {
        return Ok(None);
    };
    let start = block.get(0).unwrap().start();
    let end = blocks
        .get(i + 1)
        .map_or(log.len(), |c| c.get(0).unwrap().start());
    let text = &log[start..end];
    let frequencies =
        regex::Regex::new(r"(?m)(Total number of frequency points nsiw\(\s*)(\d+)(\)\s*=)")?;
    for c in frequencies.captures_iter(text) {
        ensure!(
            c[2].parse::<usize>()? == i + 1,
            "data_invalid: 批次频率索引不符"
        );
    }
    let text = header.replace(
        text,
        format!(
            "temp(1) = {:.5} K",
            request.temperature_centikelvin as f64 / 100.0
        ),
    );
    Ok(Some(
        frequencies.replace_all(&text, "${1}1${3}").into_owned(),
    ))
}

/// 已收敛的前缀可在后续温度失败时保留；最后一点仍须核对整个原生结束。
pub fn inspect(
    record: &AttemptResult,
) -> Result<(GapStatus, Option<SolveEvidence>, Option<GapData>)> {
    let batch = record.batch.as_ref().context("data_invalid: 缺批次证据")?;
    batch.log.verify()?;
    let log = fs::read_to_string(&batch.log.path)?;
    ensure!(
        regex::Regex::new(r"Program EPW v\.5\.9\s")?
            .find_iter(&log)
            .count()
            == 1,
        "data_invalid: 批次缺少唯一 EPW 5.9 运行证据"
    );
    let normal =
        record.exit_status == Some(0) && crate::interpolation::native::checked_log(&log, 0).is_ok();
    let part = segment(&log, &batch.temperatures, &record.request)?;
    let mut status = GapStatus {
        execution_complete: false,
        converged: None,
        data_status: "unavailable".into(),
        reason: None,
    };
    let Some(part) = part else {
        status.reason = Some("原生批次在到达此温度前停止".into());
        return Ok((status, None, None));
    };
    let reached = part.contains("Convergence was reached in nsiter =");
    let not_reached = part.contains("Convergence was not reached");
    status.converged = match (reached, not_reached) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        _ => None,
    };
    let is_last = batch.temperatures.last() == Some(&record.request.temperature_centikelvin);
    status.execution_complete = record.exit_status.is_some()
        && ((normal && (reached || not_reached)) || (reached && !is_last));
    if !status.execution_complete {
        status.reason = Some("本温度执行不完整或缺原生正常结束证据".into());
        return Ok((status, None, None));
    }
    if status.converged != Some(true) {
        status.data_status = "not_converged".into();
        status.reason = Some("未获得明确的原生收敛证据".into());
        return Ok((status, None, None));
    }
    let outcome = (|| -> Result<(SolveEvidence, GapData)> {
        let evidence = parse::solve_segment(&part, &record.request)?;
        let file = batch
            .gap_file
            .as_ref()
            .context("data_invalid: 缺少原生完整 Δ/Z")?;
        file.verify()?;
        let gap = gap::parse_native(
            &fs::read_to_string(&file.path)?,
            record.request.temperature_centikelvin,
            evidence.nfreq,
            if record.request.mode == Mode::Iso {
                vec![]
            } else {
                record.source.states.clone()
            },
        )?;
        Ok((evidence, gap))
    })();
    match outcome {
        Ok((e, g)) => {
            status.data_status = "valid".into();
            Ok((status, Some(e), Some(g)))
        }
        Err(error) => {
            status.data_status = if batch.gap_file.is_none() {
                "missing"
            } else {
                "invalid"
            }
            .into();
            status.reason = Some(format!("{error:#}"));
            Ok((status, None, None))
        }
    }
}

fn prepare(
    cfg: &Config,
    generation: &str,
    source: &NativeInputs,
    requests: Vec<Request>,
    number: usize,
) -> Result<Batch> {
    let mode = requests[0].mode;
    source.verify(mode)?;
    ensure!(
        !cfg.programs
            .epw
            .args
            .iter()
            .any(|s| matches!(s.as_str(), "-in" | "-inp" | "-input")),
        "configuration: EPW 参数不得覆盖批次输入"
    );
    let dir = Step::Superconductivity
        .directory(cfg)
        .join(directory_name(cfg, mode));
    fs::create_dir_all(&dir)?;
    // 每次覆盖主输入之前保存旧文件；逐点记录只依赖已收集的不可变快照。
    for name in ["epw.in", "epw.out", "epw.err", "epw.exitcode"] {
        let path = dir.join(name);
        if path.is_file() {
            let archive = dir.join(format!("previous-{number:04}-{name}"));
            if !archive.exists() {
                fs::copy(&path, archive)?;
            }
        }
    }
    for request in &requests {
        let old = dir.join(format!(
            "{}.imag_{}_{:06.2}",
            source.prefix,
            method(mode),
            request.temperature_centikelvin as f64 / 100.0
        ));
        if old.is_file() {
            let archive = dir.join(format!(
                "previous-{number:04}-{}",
                old.file_name().unwrap().to_string_lossy()
            ));
            ensure!(!archive.exists(), "input_conflict: 失败温度归档已存在");
            fs::rename(&old, archive)?;
        }
    }
    let input = dir.join("epw.in");
    store::atomic_bytes(&input, render(&requests, source)?.as_bytes())?;
    fs::copy(&source.crystal.path, dir.join("crystal.fmt"))?;
    fs::copy(&source.a2f.path, dir.join("source.a2f"))?;
    if mode == Mode::Aniso {
        fs::copy(
            &source
                .selecq
                .as_ref()
                .context("data_invalid: 缺 selecq")?
                .path,
            dir.join("selecq.fmt"),
        )?;
    }
    let mut command = cfg.programs.launcher.clone();
    command.push(cfg.programs.epw.path.to_string_lossy().into_owned());
    command.extend(cfg.programs.epw.args.clone());
    command.extend(["-inp".into(), input.to_string_lossy().into_owned()]);
    let task = native_jobs::prepare_group(
        cfg,
        &format!("{generation}-{number}"),
        method(mode),
        &command,
        &dir,
    )?;
    Ok(Batch {
        mode,
        requests,
        task,
        executable: Identity::capture(&cfg.programs.epw.path)?,
        collected: false,
    })
}

fn collect(run: &mut Run, index: usize, source: &NativeInputs) -> Result<()> {
    let batch = run.batches[index].clone();
    let dir = &batch.task.directory;
    let number = index + 1;
    let mut artifacts = vec![];
    let mut log = None;
    for suffix in ["in", "out", "err"] {
        let path = dir.join(format!("epw.{suffix}"));
        if path.is_file() {
            let snapshot = dir.join(format!("epw-attempt-{number:04}.{suffix}"));
            if snapshot.exists() {
                ensure!(
                    store::hash_file(&snapshot)? == store::hash_file(&path)?,
                    "input_conflict: 批次快照已存在且不同"
                );
            } else {
                fs::copy(path, &snapshot)?;
            }
            let id = Identity::capture(&snapshot)?;
            if suffix == "out" {
                log = Some(id.clone());
            }
            artifacts.push(id);
        }
    }
    let log = log.context("data_invalid: 批次缺少原生日志")?;
    for request in &batch.requests {
        let canonical = dir.join(format!(
            "{}.imag_{}_{:06.2}",
            source.prefix,
            method(batch.mode),
            request.temperature_centikelvin as f64 / 100.0
        ));
        let gap_file = if canonical.is_file() {
            let snapshot = dir.join(format!(
                "gap-attempt-{number:04}-{}.dat",
                decimal(request.temperature_centikelvin)
            ));
            if !snapshot.exists() {
                fs::copy(&canonical, &snapshot)?;
            }
            Some(Identity::capture(&snapshot)?)
        } else {
            None
        };
        let mut record = AttemptResult {
            schema_version: 2,
            batch: Some(Evidence {
                log: log.clone(),
                temperatures: batch
                    .requests
                    .iter()
                    .map(|q| q.temperature_centikelvin)
                    .collect(),
                gap_file: gap_file.clone(),
            }),
            request: request.clone(),
            source: source.clone(),
            executable: batch.executable.clone(),
            command: batch.task.command.clone(),
            status: "failed".into(),
            reason: None,
            exit_status: batch.task.exit_status,
            duration_secs: Some(batch.task.duration_secs),
            artifacts: artifacts.clone(),
            evidence: None,
            gap: None,
            gap_status: None,
        };
        record.artifacts.extend(gap_file);
        let result = inspect(&record);
        match result {
            Ok((status, evidence, gap)) => {
                record.status = if !status.execution_complete {
                    "failed"
                } else if gap.is_some() {
                    "valid"
                } else {
                    "completed"
                }
                .into();
                record.reason = status.reason.clone();
                record.gap_status = Some(status);
                record.evidence = evidence;
                record.gap = gap;
            }
            Err(error) => {
                record.reason = Some(format!("{error:#}"));
            }
        }
        let path = dir.join(format!(
            "point-{}-attempt-{number:04}.json",
            decimal(request.temperature_centikelvin)
        ));
        store::write(&path, &record)?;
        let entry = run
            .attempts
            .iter_mut()
            .rev()
            .find(|a| {
                a.request.mode == request.mode
                    && a.request.temperature_centikelvin == request.temperature_centikelvin
                    && a.status == "running"
            })
            .context("data_invalid: 批次缺点记录")?;
        println!(
            "method: {}\ntemperature_k: {}\nexecution_status: {}\nreason: {}",
            method(request.mode),
            decimal(request.temperature_centikelvin),
            record.status,
            record.reason.as_deref().unwrap_or("无")
        );
        entry.status = record.status;
        entry.reason = record.reason;
        entry.record = Some(Identity::capture(&path)?);
    }
    run.batches[index].collected = true;
    run.refresh();
    Ok(())
}
pub fn run(cfg: &Config, generation: &str, submit: bool, resume: bool) -> Result<()> {
    let root = Step::Superconductivity.directory(cfg);
    fs::create_dir_all(&root)?;
    let path = root.join("manifest.json");
    let f3 = crate::interpolation::manifest::InterpolationManifest::load(
        &crate::foundation::layout::path(&cfg.work_dir, "interpolation/manifest.json"),
    )?;
    ensure!(f3.validation.all_passed(), "data_invalid: 细网格数据无效");
    let source = super::source::bind(&cfg.work_dir, cfg, &f3)?;
    let mut run = if path.is_file() {
        super::points::published(&cfg.work_dir, cfg, &f3.run_id)?
    } else {
        Run {
            schema_version: 4,
            task: "gap".into(),
            run_id: f3.run_id,
            interpolation_digest: source.interpolation_digest.clone(),
            binding_digest: super::points::binding(cfg, &source, &cfg.programs.launcher)?,
            plan: super::points::plan(cfg)?,
            attempts: vec![],
            batches: vec![],
            execution_complete: false,
            data_complete: false,
        }
    };
    let mut pending = false;
    let mut errors = vec![];
    for mode in [Mode::Iso, Mode::Aniso] {
        if !run.plan.iter().any(|q| q.mode == mode) {
            continue;
        }
        let result = (|| -> Result<()> {
            if let Some(index) = run
                .batches
                .iter()
                .rposition(|b| b.mode == mode && !b.collected)
            {
                if let Some(task) = native_jobs::group_task(cfg, method(mode))? {
                    ensure!(
                        task.key == run.batches[index].task.key,
                        "input_conflict: 方法批次作业指针冲突"
                    );
                    run.batches[index].task = task;
                }
                let task = &mut run.batches[index].task;
                let state = native_jobs::task_status(task)?;
                if state["state"] == "running" {
                    pending = true;
                    return Ok(());
                }
                if matches!(
                    state["state"].as_str(),
                    Some("unknown" | "submission_pending")
                ) {
                    anyhow::bail!("scheduler_unknown: {} 提交回执未知，禁止重提", method(mode));
                }
                if task.state == "prepared" {
                    if submit {
                        native_jobs::submit_task(cfg, task)?;
                    }
                    pending = true;
                    return Ok(());
                }
                native_jobs::reconcile_task(cfg, true, task)?;
                collect(&mut run, index, &source)?;
                let complete = run.batches[index].requests.iter().all(|q| {
                    run.latest(q)
                        .is_some_and(|a| matches!(a.status.as_str(), "valid" | "completed"))
                });
                native_jobs::finish_group(cfg, &mut run.batches[index].task, complete)?;
                store::write(&path, &run)?;
            }
            let requests: Vec<_> = run
                .plan
                .iter()
                .filter(|q| {
                    q.mode == mode
                        && !run
                            .latest(q)
                            .is_some_and(|a| matches!(a.status.as_str(), "valid" | "completed"))
                })
                .cloned()
                .collect();
            if requests.is_empty() {
                return Ok(());
            }
            ensure!(
                resume || requests.iter().all(|q| run.latest(q).is_none()),
                "execution_failed: {} 有未完成点；保留现场，使用 --resume -s",
                method(mode)
            );
            // 切换版本只影响新批次；已有历史点记录原样保留。
            run.schema_version = 4;
            let batch = prepare(cfg, generation, &source, requests, run.batches.len() + 1)?;
            for request in &batch.requests {
                run.attempts.push(Entry {
                    request: request.clone(),
                    directory: batch.task.directory.clone(),
                    record: None,
                    status: "running".into(),
                    reason: None,
                    job_id: None,
                });
            }
            run.batches.push(batch);
            run.refresh();
            store::write(&path, &run)?;
            let batch = run.batches.last_mut().unwrap();
            if submit {
                native_jobs::submit_task(cfg, &mut batch.task)?;
            }
            println!(
                "method: {}\ndirectory: {}\nscript: {}\nworkflow_status: {}",
                method(mode),
                batch.task.directory.display(),
                batch.task.script.path.display(),
                batch.task.state
            );
            pending = true;
            Ok(())
        })();
        if let Err(error) = result {
            errors.push(format!("{}: {error:#}", method(mode)));
        }
        run.refresh();
        store::write(&path, &run)?;
    }
    if !errors.is_empty() {
        anyhow::bail!("execution_failed: {}", errors.join("; "));
    }
    if pending {
        return Err(native_jobs::Paused.into());
    }
    ensure!(run.execution_complete, "partial: 指定温度尚未执行完成");
    crate::delivery::finalize_to(&cfg.work_dir, Some(&root.join("results")))?;
    Ok(())
}
