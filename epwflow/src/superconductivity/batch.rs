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
    let settings = cfg.solver_for(mode).expect("Configuration validated at public entry");
    let ts = &settings.gap_temperatures_k;

    let degaussq = smearing(cfg, mode).expect("Configuration validated at public entry");

    format!(
        "{}-mu{}-Tc{}-{}-degaussq{}meV",
        method(mode),
        settings.mu_star,
        ts[0],
        ts[ts.len() - 1],
        degaussq
    )
}

/// 目录与原生输入共用同一展宽，显式非法值不能静默回退第七步。
pub(crate) fn smearing(cfg: &Config, mode: Mode) -> Result<f64> {
    let settings = cfg.solver_for(mode)?;
    let input: serde_json::Value = cfg.native_value(&[
        crate::foundation::native_config::SUPER, method(mode), "epw", "INPUTEPW",
    ])?;
    let value = if let Some(value) = input.get("degaussq") {
        value.as_f64().context("configuration: superconductivity degaussq must be a number in meV")?
    } else {
        *cfg.interpolation.phonon_smearings_mev.get(settings.spectrum_index as usize)
            .context("configuration: superconductivity spectrum_index is out of range")?
    };
    ensure!(value.is_finite() && value > 0.,
        "configuration: superconductivity degaussq must be finite and positive (meV)");
    Ok(value)
}

pub fn render(requests: &[Request], source: &NativeInputs) -> Result<String> {
    ensure!(
        !requests.is_empty() && requests.len() <= 50,
        "configuration: native batch requires 1-50 temperatures"
    );
    let mut text = attempt::render_base(&requests[0], source)?;
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
            "configuration: batch mixes solver settings within the same method"
        );
        ensure!(
            !requests[..i]
                .iter()
                .any(|q| q.temperature_centikelvin == request.temperature_centikelvin),
            "configuration: duplicate batch temperature"
        );
        temperatures.push_str(&format!(
            " temps({}) = {:.2}\n",
            i + 1,
            request.temperature_centikelvin as f64 / 100.0
        ));
    }
    ensure!(text.contains(&single), "data_invalid: native temperature-input template missing");
    text = text.replacen(&single, &temperatures, 1);
    attempt::configured(
        &requests[0],
        source,
        &text.replacen("single-temperature solve", "multi-temperature solve", 1),
    )
}

/// 真实 itemp／温度先与固定批次核对；随后只把索引归一化交给既有单段解析器。
fn segment(log: &str, temperatures: &[i64], request: &Request) -> Result<Option<String>> {
    let header = regex::Regex::new(r"(?m)^\s*temp\(\s*(\d+)\)\s*=\s*([0-9.]+)\s+K\s*$")?;
    let blocks: Vec<_> = header.captures_iter(log).collect();
    ensure!(
        blocks.len() <= temperatures.len(),
        "data_invalid: batch contains extra temperatures"
    );
    let opposite = if request.mode == Mode::Iso {
        "anisotropic"
    } else {
        "isotropic"
    };
    ensure!(
        !log.contains(&format!("Solve {opposite} Eliashberg equations"))
            && !log.contains("linearized Eliashberg"),
        "data_invalid: incorrect batch mode"
    );
    for (i, block) in blocks.iter().enumerate() {
        ensure!(
            block[1].parse::<usize>()? == i + 1
                && (block[2].parse::<f64>()? * 100.0 - temperatures[i] as f64).abs() < 1e-5,
            "data_invalid: actual batch temperature or index mismatch"
        );
    }
    let i = temperatures
        .iter()
        .position(|&t| t == request.temperature_centikelvin)
        .context("data_invalid: point does not belong to the batch")?;
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
            "data_invalid: batch frequency-index mismatch"
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
    let batch = record.batch.as_ref().context("data_invalid: batch evidence missing")?;
    batch.log.verify()?;
    let log = fs::read_to_string(&batch.log.path)?;
    crate::interpolation::native::runtime_version(&log)?;
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
        status.reason = Some("Native batch stopped before reaching this temperature".into());
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
        status.reason = Some("Execution incomplete or native normal-termination evidence missing at this temperature".into());
        return Ok((status, None, None));
    }
    if status.converged != Some(true) {
        status.data_status = "not_converged".into();
        status.reason = Some("No explicit native convergence evidence obtained".into());
        return Ok((status, None, None));
    }
    let outcome = (|| -> Result<(SolveEvidence, GapData)> {
        let evidence = parse::solve_segment(&part, &record.request)?;
        let file = batch
            .gap_file
            .as_ref()
            .context("data_invalid: complete native Delta/Z missing")?;
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
        "configuration: EPW arguments must not override batch input"
    );
    let dir = Step::Superconductivity
        .directory(cfg)
        .join(directory_name(cfg, mode));
    fs::create_dir_all(&dir)?;
    // 仅覆盖本轮仍需计算温度的数据；成功温度保留当前科学结果。
    for request in &requests {
        let old = dir.join(format!(
            "{}.imag_{}_{:06.2}",
            source.prefix,
            method(mode),
            request.temperature_centikelvin as f64 / 100.0
        ));
        if old.is_file() {
            fs::remove_file(old)?;
        }
    }
    let input = dir.join("epw.in");
    store::atomic_bytes(&input, render(&requests, source)?.as_bytes())?;
    source.stage_matrix_links(&dir,mode)?;
    if source.interpolation_digest!="native-spectrum" {
        crate::foundation::workspace_data::link(&source.crystal.path,&dir.join("crystal.fmt"))?;
    }
    if source.reuse_spectrum(mode) && source.a2f.path.is_file() {crate::foundation::workspace_data::link(&source.a2f.path,&dir.join("source.a2f"))?;}
    if mode == Mode::Aniso || !source.reuse_spectrum(mode) {
        crate::foundation::workspace_data::link(
            &source
                .selecq
                .as_ref()
                .context("data_invalid: selecq missing")?
                .path,
            &dir.join("selecq.fmt"),
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

fn collect(cfg: &Config, run: &mut Run, index: usize, source: &NativeInputs) -> Result<()> {
    let batch = run.batches[index].clone();
    let dir = &batch.task.directory;
    let raw = dir.join("epw.out");
    for request in &batch.requests {
        let temperature = decimal(request.temperature_centikelvin);
        let point_log = dir.join(format!("point-{temperature}.out"));
        if raw.is_file() {
            fs::copy(&raw, &point_log)?;
        }
        let canonical = dir.join(format!(
            "{}.imag_{}_{:06.2}",
            source.prefix,
            method(batch.mode),
            request.temperature_centikelvin as f64 / 100.0
        ));
        let gap_file = if canonical.is_file() {
            Some(Identity::capture(&canonical)?)
        } else {
            None
        };
        let log = Identity::capture(&point_log)?;
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
            artifacts: vec![log],
            evidence: None,
            gap: None,
            gap_status: None,
        };
        record.artifacts.extend(gap_file);
        match inspect(&record) {
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
            Err(e) => record.reason = Some(format!("{e:#}")),
        }
        let path = dir.join(format!("point-{temperature}.json"));
        store::write(&path, &record)?;
        let entry = Entry {
            request: request.clone(),
            directory: dir.clone(),
            record: Some(Identity::capture(&path)?),
            status: record.status.clone(),
            reason: record.reason.clone(),
            job_id: None,
        };
        run.attempts.retain(|e| {
            !(e.request.mode == request.mode
                && e.request.temperature_centikelvin == request.temperature_centikelvin)
        });
        run.attempts.push(entry);
        crate::foundation::readable_state::update(&cfg.work_dir, Step::Superconductivity, |doc| {
            let label = format!("Temperature · {} · {temperature} K", method(batch.mode));
            doc.set(
                &label,
                "Status",
                match record.status.as_str() {
                    "valid" => "completed; gap data valid",
                    "completed" => "Solver finished; gap data invalid",
                    "failed" => "failed",
                    _ => "validation pending",
                },
            );
            doc.set(
                &label,
                "Scientific result",
                crate::foundation::readable_state::relative(&cfg.work_dir, &path),
            );
            if let Some(status) = &record.gap_status {
                doc.set(
                    &label,
                    "Converged",
                    match status.converged {
                        Some(true) => "yes",
                        Some(false) => "no",
                        None => "unconfirmed",
                    },
                );
                doc.set(&label, "Data status", &status.data_status);
            }
            if let Some(reason) = &record.reason {
                doc.set(&label, "Reason", reason);
            }
            Ok(())
        })?;
    }
    run.batches[index].collected = true;
    run.refresh();
    Ok(())
}

/// 工作流事实只从第八步日志读取，结果文件仅用于重新检查科学数值。
pub(crate) fn current(cfg: &Config, source: &NativeInputs, run_id: &str) -> Result<Run> {
    let mut run = Run {
        schema_version: 4,
        task: "gap".into(),
        run_id: run_id.into(),
        interpolation_digest: source.interpolation_digest.clone(),
        binding_digest: super::points::binding(cfg, source, &cfg.programs.launcher)?,
        plan: super::points::plan(cfg)?,
        attempts: vec![],
        batches: vec![],
        execution_complete: false,
        data_complete: false,
    };
    let doc = crate::foundation::readable_state::read(&cfg.work_dir, Step::Superconductivity)?;
    for request in &run.plan {
        let label = format!(
            "Temperature · {} · {} K",
            method(request.mode),
            decimal(request.temperature_centikelvin)
        );
        let Some(path) = doc.get(&label, "Scientific result") else {
            continue;
        };
        let path = crate::foundation::readable_state::resolve(&cfg.work_dir, path);
        if !path.is_file() {
            continue;
        }
        let record = super::verify::record(&path)?;
        ensure!(
            serde_json::to_value(&record.request)? == serde_json::to_value(request)?,
            "input_conflict: temperature-result parameters differ from the current plan"
        );
        run.attempts.push(Entry {
            request: request.clone(),
            directory: path.parent().unwrap().into(),
            record: Some(Identity::capture(&path)?),
            status: record.status,
            reason: record.reason,
            job_id: None,
        });
    }
    run.refresh();
    Ok(run)
}
fn publish_science(cfg: &Config, run: &Run) -> Result<()> {
    let mut science = run.clone();
    science.batches.clear();
    store::write(
        &Step::Superconductivity.directory(cfg).join("manifest.json"),
        &science,
    )
}

pub fn run(cfg: &Config, generation: &str, submit: bool, resume: bool) -> Result<()> {
    run_mode(cfg, generation, submit, resume, false)
}
pub(crate) fn run_mode(
    cfg: &Config,
    generation: &str,
    submit: bool,
    resume: bool,
    allocated: bool,
) -> Result<()> {
    use crate::foundation::readable_state as log;
    let root = Step::Superconductivity.directory(cfg);
    fs::create_dir_all(&root)?;
    let _guard = log::DirectoryGuard::acquire(&root)?;
    let source = super::source::selected(cfg)?;
    let mut run = current(cfg, &source, "native")?;
    let mut pending = false;
    let mut errors = vec![];
    for mode in [Mode::Iso, Mode::Aniso] {
        if !run.plan.iter().any(|q| q.mode == mode) {
            continue;
        }
        let source=source.for_mode(cfg,mode)?;
        let result = (|| -> Result<()> {
            let label = format!("Method · {}", method(mode));
            if let Some(mut task) = native_jobs::group_task(cfg, method(mode))? {
                let doc = log::read(&cfg.work_dir, Step::Superconductivity)?;
                let already = doc.get(&label, "Committed submissions collected") == Some(task.id.as_str());
                if !already {
                    let temperatures = doc
                        .get(&label, "Current temperatures")
                        .context("data_invalid: method log has no current temperature set")?;
                    let temperatures = temperatures
                        .split(',')
                        .map(|s| {
                            crate::foundation::config::temperature(
                                s.trim().trim_end_matches('K').trim().parse()?,
                                "batch temperature",
                            )
                        })
                        .collect::<Result<Vec<_>>>()?;
                    let requests = temperatures
                        .iter()
                        .map(|t| {
                            run.plan
                                .iter()
                                .find(|q| q.mode == mode && q.temperature_centikelvin == *t)
                                .cloned()
                                .context("input_conflict: batch temperature is outside the current plan")
                        })
                        .collect::<Result<Vec<_>>>()?;
                    let state = native_jobs::task_status(&task)?;
                    ensure!(
                        !matches!(
                            state["state"].as_str(),
                            Some("unknown" | "submission_pending")
                        ),
                        "scheduler_unknown: method {} submission status unknown",
                        method(mode)
                    );
                    if state["state"] == "running" {
                        pending = true;
                        return Ok(());
                    }
                    if task.state == "prepared" && allocated {
                        native_jobs::execute_group(cfg, &mut task)?;
                    } else if task.state == "prepared" {
                        if submit {
                            native_jobs::submit_task(cfg, &mut task)?;
                        }
                        pending = true;
                        return Ok(());
                    }
                    if !allocated {
                        native_jobs::reconcile_task(cfg, true, &mut task)?;
                    }
                    run.batches.push(Batch {
                        mode,
                        requests,
                        executable: Identity::capture(&cfg.programs.epw.path)?,
                        task,
                        collected: false,
                    });
                    let index = run.batches.len() - 1;
                    if run.batches[index].task.directory.join("epw.out").is_file() {
                        collect(cfg, &mut run, index, &source)?;
                    }
                    let complete = run.batches[index].requests.iter().all(|q| {
                        run.latest(q)
                            .is_some_and(|a| matches!(a.status.as_str(), "valid" | "completed"))
                    });
                    native_jobs::finish_group(cfg, &mut run.batches[index].task, complete)?;
                    let id = run.batches[index].task.id.clone();
                    log::update(&cfg.work_dir, Step::Superconductivity, |d| {
                        d.set(&label, "Committed submissions collected", id);
                        Ok(())
                    })?;
                }
            }
            let requests = run
                .plan
                .iter()
                .filter(|q| {
                    q.mode == mode
                        && !run
                            .latest(q)
                            .is_some_and(|a| matches!(a.status.as_str(), "valid" | "completed"))
                })
                .cloned()
                .collect::<Vec<_>>();
            if requests.is_empty() {
                return Ok(());
            }
            ensure!(
                resume || requests.iter().all(|q| run.latest(q).is_none()),
                "execution_failed: {} has failed temperatures; fix the cause and use --resume -s",
                method(mode)
            );
            let mut batch = prepare(cfg, generation, &source, requests, run.attempts.len() + 1)?;
            log::update(&cfg.work_dir, Step::Superconductivity, |d| {
                d.set(
                    &label,
                    "Current temperatures",
                    batch
                        .requests
                        .iter()
                        .map(|q| format!("{} K", decimal(q.temperature_centikelvin)))
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                Ok(())
            })?;
            if allocated {
                native_jobs::execute_group(cfg, &mut batch.task)?;
                run.batches.push(batch);
                let index = run.batches.len() - 1;
                collect(cfg, &mut run, index, &source)?;
                let complete = run.batches[index].requests.iter().all(|q| {
                    run.latest(q)
                        .is_some_and(|a| matches!(a.status.as_str(), "valid" | "completed"))
                });
                native_jobs::finish_group(cfg, &mut run.batches[index].task, complete)?;
                log::update(&cfg.work_dir, Step::Superconductivity, |d| {
                    d.set(&label, "Committed submissions collected", &run.batches[index].task.id);
                    Ok(())
                })?;
                ensure!(
                    complete,
                    "execution_failed: {} has incomplete temperatures; fix the cause and use --resume -r",
                    method(mode)
                );
            } else if submit {
                native_jobs::submit_task(cfg, &mut batch.task)?;
            }
            pending |= !allocated;
            Ok(())
        })();
        if let Err(e) = result {
            errors.push(format!("{}：{e:#}", method(mode)));
        }
    }
    run.refresh();
    publish_science(cfg, &run)?;
    if !errors.is_empty() {
        anyhow::bail!("execution_failed: {}", errors.join("；"));
    }
    if pending {
        return Err(native_jobs::Paused.into());
    }
    ensure!(run.execution_complete, "partial: specified temperatures are not all complete");
    let plotted=crate::delivery::finalize_to(&cfg.work_dir, Some(&root.join("results")))?;
    crate::foundation::stage_results::tree(
        cfg,
        Step::Superconductivity,
        "",
        &plotted,
        serde_json::json!({"kind":"native_superconductivity"}),
    )?;
    Ok(())
}

pub fn allocated_finish(_config: &std::path::Path, _group: &str, _exit: i32) -> Result<()> {
    anyhow::bail!("unsupported_version: legacy finalization scripts disabled; collect method results with the current public command")
}

#[cfg(test)]
mod directory_tests {
    use super::*;
    fn config() -> Config {
        let mut cfg: Config = toml::from_str(include_str!("../../tests/fixtures/input.toml")).unwrap();
        cfg.superconductivity.mode = Mode::Iso;
        cfg.superconductivity.mu_star = 0.13;
        cfg.superconductivity.gap_temperatures_k = vec![20., 25.5, 40.];
        cfg
    }
    #[test]
    fn method_directory_uses_explicit_smearing_and_preserves_plan_endpoints() {
        let mut cfg = config();
        assert_eq!(directory_name(&cfg, Mode::Iso), "iso-mu0.13-Tc20-40-degaussq0.5meV");
        cfg.set_native(&[crate::foundation::native_config::SUPER,"iso","epw","INPUTEPW","degaussq"], serde_json::json!(2.0)).unwrap();
        assert_eq!(directory_name(&cfg, Mode::Iso), "iso-mu0.13-Tc20-40-degaussq2meV");
        cfg.superconductivity.mode = Mode::Aniso;
        cfg.superconductivity.mu_star = 0.1;
        cfg.superconductivity.gap_temperatures_k = vec![20.25];
        assert_eq!(directory_name(&cfg, Mode::Aniso), "aniso-mu0.1-Tc20.25-20.25-degaussq0.5meV");
    }
    #[test]
    fn missing_smearing_uses_selected_index_and_invalid_explicit_value_is_rejected() {
        let mut cfg = config();
        cfg.interpolation.phonon_smearings_mev = vec![0.5, 2.];
        cfg.superconductivity.spectrum_index = 1;
        assert_eq!(smearing(&cfg, Mode::Iso).unwrap(), 2.);
        cfg.superconductivity.spectrum_index = 2;
        assert!(smearing(&cfg, Mode::Iso).is_err());
    }
}
