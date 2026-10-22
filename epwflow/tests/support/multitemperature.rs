//! 0049：一次公开命令双作业、原生多温度、目录及部分提交／恢复。
use super::*;

impl Case {
    fn gap_batch(mode: config::Mode, temperatures: Vec<f64>, mu: Option<f64>) -> Self {
        let c = Self::numbered();
        c.numbered_change(|cfg| {
            let sc = &mut cfg.superconductivity;
            sc.task = config::SuperconductivityTask::Gap;
            sc.mode = mode;
            sc.gap_temperatures_k = temperatures;
            sc.iso = None;
            sc.aniso = None;
            sc.mu_star = mu.unwrap_or(0.13);
        });
        if mu.is_none() {
            let path = c.root.join("input.toml");
            let text = fs::read_to_string(&path).unwrap();
            fs::write(
                path,
                text.lines()
                    .filter(|l| !l.starts_with("mu_star ="))
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
            .unwrap();
        }
        c.accounting(Some(201));
        success(c.numbered_cli(2, &["--stage", "fine", "-r"], Some(201), "batch-upstream"));
        c.direct_setup();
        c.put("job-201.state", "COMPLETED");
        c.put("submissions", "allocated preparation\n");
        c
    }
}
const ARGS: [&str; 3] = ["--stage", "superconductivity", "-s"];

#[test]
fn batch_prepared_script_changes_are_rejected_before_submission() {
    let c = Case::gap_batch(config::Mode::Both, vec![20., 30.], None);
    success(c.direct_advance(2, &["--stage", "superconductivity"], "batch-prepare-only"));
    let iso = c.method_task("iso");
    assert_eq!(iso["state"], "prepared");
    let script = PathBuf::from(iso["script"]["path"].as_str().unwrap());
    let text = fs::read_to_string(&script).unwrap();
    fs::write(script, format!("{text}\n# independent debug edit\n")).unwrap();
    let result = c.direct_advance(2, &ARGS, "batch-edited-prepare-submit");
    assert!(!result.status.success());
    assert_eq!(c.method_task("iso")["state"], "prepared");
    // 一个方法的身份冲突不妨碍另一个方法提交。
    assert!(c.method_task("aniso")["job"].is_u64());
    assert_eq!(
        fs::read_to_string(c.root.join("submissions"))
            .unwrap()
            .lines()
            .count(),
        2
    );
}

#[test]
fn batch_default_mu_and_explicit_value_use_method_range_directories() {
    for (mode, temperatures, mu, expected) in [
        (
            config::Mode::Iso,
            vec![20., 25.5, 40.],
            None,
            "iso-mu0.13-20-40",
        ),
        (
            config::Mode::Aniso,
            vec![20.25],
            Some(0.1),
            "aniso-mu0.1-20.25-20.25",
        ),
    ] {
        let c = Case::gap_batch(mode, temperatures.clone(), mu);
        success(c.direct_advance(2, &ARGS, "batch-prepare-submit"));
        let name = if mode == config::Mode::Iso {
            "iso"
        } else {
            "aniso"
        };
        let task = c.method_task(name);
        let dir = c.root.join("run/8.epw-superconductivity").join(expected);
        assert_eq!(PathBuf::from(task["directory"].as_str().unwrap()), dir);
        assert_eq!(
            fs::read_to_string(c.root.join("submissions"))
                .unwrap()
                .lines()
                .count(),
            2
        );
        let input = fs::read_to_string(dir.join("epw.in")).unwrap();
        assert!(input.contains(&format!("nstemp = {}", temperatures.len())));
        let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
        assert_eq!(cfg.superconductivity.mu_star, mu.unwrap_or(0.13));
        assert_eq!(c.execute_record(task), 0);
        success(c.direct_advance(2, &ARGS, "batch-collect"));
        let run = epwflow::superconductivity::points::Run::load(
            &c.root.join("run/8.epw-superconductivity/manifest.json"),
        )
        .unwrap();
        assert!(run.execution_complete && run.data_complete);
        assert!(run.attempts.iter().all(|a| a.directory == dir));
        assert!(fs::read_dir(dir)
            .unwrap()
            .all(|e| !e.unwrap().path().is_dir()));
    }
}

#[test]
fn batch_second_submission_unknown_keeps_first_job_and_does_not_resubmit() {
    let c = Case::gap_batch(config::Mode::Both, vec![20., 30., 40.], None);
    c.script("sbatch",&format!("#!/bin/bash\nprintf '%s\\n' \"$*\" >> {p}\nn=$(wc -l < {p})\nif [[ $n == 3 ]]; then printf 'unknown receipt\\n'; exit 1; fi\nprintf '%s\\n' \"$((200+n))\"\n",p=shell_quote(c.root.join("submissions").to_str().unwrap())));
    let out = c.direct_advance(2, &ARGS, "batch-second-unknown");
    assert!(!out.status.success());
    let iso = c.method_task("iso");
    let aniso = c.method_task("aniso");
    assert!(iso["job"].is_u64());
    assert_eq!(aniso["state"], "unknown");
    assert_eq!(c.execute_record(iso), 0);
    let count = fs::read_to_string(c.root.join("submissions")).unwrap();
    let out = c.direct_advance(
        2,
        &["--stage", "superconductivity", "--resume", "-s"],
        "batch-unknown-resume",
    );
    assert!(!out.status.success());
    assert_eq!(
        count,
        fs::read_to_string(c.root.join("submissions")).unwrap()
    );
    let run = epwflow::superconductivity::points::Run::load(
        &c.root.join("run/8.epw-superconductivity/manifest.json"),
    )
    .unwrap();
    assert_eq!(
        run.attempts.iter().filter(|a| a.status == "valid").count(),
        3
    );
    let status = c.direct_advance(2, &["--status", "--json"], "batch-two-status");
    success(status.clone());
    let status: Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(status["native_methods"]["iso"]["state"], "validated");
    assert_eq!(status["native_methods"]["aniso"]["state"], "unknown");
}

#[test]
fn batch_native_early_stop_requires_explicit_resume_for_missing_temperatures() {
    let c = Case::gap_batch(config::Mode::Iso, vec![20., 30., 40.], None);
    c.put("gap-fault", "unconverged");
    success(c.direct_advance(2, &ARGS, "batch-early-submit"));
    assert_eq!(c.execute_record(c.method_task("iso")), 0);
    assert!(!c
        .direct_advance(2, &ARGS, "batch-early-collect")
        .status
        .success());
    let root = c.root.join("run/8.epw-superconductivity");
    let before =
        epwflow::superconductivity::points::Run::load(&root.join("manifest.json")).unwrap();
    assert!(!before.execution_complete);
    assert_eq!(before.attempts[0].status, "completed");
    assert!(before.attempts[1..].iter().all(|a| a.status == "failed"));
    fs::remove_file(c.root.join("gap-fault")).unwrap();
    success(c.direct_advance(
        2,
        &["--stage", "superconductivity", "--resume", "-s"],
        "batch-early-resume",
    ));
    let task = c.method_task("iso");
    let text =
        fs::read_to_string(PathBuf::from(task["directory"].as_str().unwrap()).join("epw.in"))
            .unwrap();
    assert!(text.contains("nstemp = 2"));
    assert!(!text.contains("temps(1) = 20.00"));
    assert_eq!(c.execute_record(task), 0);
    success(c.direct_advance(2, &ARGS, "batch-early-complete"));
    let after = epwflow::superconductivity::points::Run::load(&root.join("manifest.json")).unwrap();
    assert!(after.execution_complete);
    assert!(!after.data_complete);
    for entry in &before.attempts {
        entry.record.as_ref().unwrap().verify().unwrap();
    }
}

#[test]
fn batch_invalid_gap_data_and_missing_log_ending_remain_distinct() {
    for (fault, execution) in [("missing", true), ("nan", true), ("no-ending", false)] {
        let c = Case::gap_batch(config::Mode::Iso, vec![20.], None);
        c.put("gap-fault", fault);
        success(c.direct_advance(2, &ARGS, "batch-invalid-submit"));
        assert_eq!(c.execute_record(c.method_task("iso")), 0);
        let result = c.direct_advance(2, &ARGS, "batch-invalid-collect");
        assert_eq!(result.status.success(), execution);
        let run = epwflow::superconductivity::points::Run::load(
            &c.root.join("run/8.epw-superconductivity/manifest.json"),
        )
        .unwrap();
        assert_eq!(run.execution_complete, execution);
        assert!(!run.data_complete);
    }
}

#[test]
fn batch_wrong_temperature_log_and_active_upstream_change_are_rejected() {
    let c = Case::gap_batch(config::Mode::Both, vec![20., 30.], None);
    success(c.direct_advance(2, &ARGS, "batch-two-start"));
    // 第一方法仍在读细网格时，合法的新细网格请求也不能改写来源。
    c.numbered_change(|cfg| cfg.interpolation.spectral_points += 1);
    assert!(!c
        .direct_advance(2, &["--stage", "fine", "-s"], "batch-upstream-block")
        .status
        .success());
    c.numbered_change(|cfg| cfg.interpolation.spectral_points -= 1);
    assert_eq!(c.execute_record(c.method_task("iso")), 0);
    assert_eq!(c.execute_record(c.method_task("aniso")), 0);
    let iso = c.method_task("iso");
    let log = PathBuf::from(iso["directory"].as_str().unwrap()).join("epw.out");
    let text = fs::read_to_string(&log).unwrap();
    fs::write(
        log,
        text.replace("temp(2) = 30.00000 K", "temp(2) = 31.00000 K"),
    )
    .unwrap();
    assert!(!c
        .direct_advance(2, &ARGS, "batch-wrong-temperature")
        .status
        .success());
    let run = epwflow::superconductivity::points::Run::load(
        &c.root.join("run/8.epw-superconductivity/manifest.json"),
    )
    .unwrap();
    assert!(!run.execution_complete);
    assert!(run
        .attempts
        .iter()
        .filter(|a| a.request.mode == config::Mode::Aniso)
        .all(|a| a.status == "valid"));
}
