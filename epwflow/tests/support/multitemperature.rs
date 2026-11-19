//! 0049：一次公开命令双作业、原生多温度、目录及部分提交／恢复。
use super::*;

impl Case {
    pub(super) fn gap_batch(mode: config::Mode, temperatures: Vec<f64>, mu: Option<f64>) -> Self {
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
        c.stage_setup();
        // 当前独立阶段不隐式补上游；本夹具显式生成完整细网格来源。
        for stage in ["scf", "nscf", "electronic", "phonon", "wannier", "coarse"] {
            success(c.stage_submit(stage, false));
            success(c.stage_execute());
        }
        c.accounting(Some(999));
        success(c.numbered_cli(2, &["--stage", "fine", "-r"], Some(999), "batch-upstream"));
        // 真实细网格交付包含 DOS；旧 IO 替身尚未写出此矩阵路线配套文件。
        let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
        let fine = epwflow::interpolation::fine::directory(&cfg).unwrap();
        fs::write(fine.join("out").join(format!("{}.dos", cfg.qe.prefix)), "0 1\n1 1\n").unwrap();
        c.direct_setup();
        // 后续提交计数从 1 开始，不能继承上游夹具中相同作业号的终态。
        for entry in fs::read_dir(&c.root).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("job-") && name.ends_with(".state") {
                fs::remove_file(entry.path()).unwrap();
            }
        }
        c.put("job-999.state", "COMPLETED");
        c.put("submissions", "allocated preparation\n");
        c
    }
}
const ARGS: [&str; 3] = ["--stage", "superconductivity", "-s"];

#[test]
fn batch_independent_degaussq_keeps_old_results_and_reuses_fine_matrices() {
    use epwflow::foundation::native_config::SUPER;
    let c = Case::gap_batch(config::Mode::Iso, vec![20.], None);
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    let fine = epwflow::interpolation::fine::directory(&cfg).unwrap();
    let fine_log = store::hash_file(&fine.join("epw_fine.out")).unwrap();
    let scientific_calls = c.calls();
    let mut previous: Option<(PathBuf, String)> = None;
    for width in [0.5, 2.] {
        let mut cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
        cfg.set_native(&[SUPER,"iso","epw","INPUTEPW","degaussq"], json!(width)).unwrap();
        c.put("input.toml", &toml::to_string_pretty(&cfg).unwrap());
        success(c.direct_advance(2, &ARGS, "independent-width-submit"));
        let task = c.method_task("iso");
        let dir = PathBuf::from(task["directory"].as_str().unwrap());
        assert!(dir.ends_with(format!("iso-mu0.13-Tc20-20-degaussq{width}meV")));
        let input = fs::read_to_string(dir.join("epw.in")).unwrap();
        let actual = regex::Regex::new(r"(?m)^\s*degaussq\s*=\s*([^\s,]+)").unwrap()
            .captures(&input).unwrap()[1].parse::<f64>().unwrap();
        assert_eq!(actual, width);
        assert!(!input.contains("source.a2f"), "Independent smearing must recompute the spectrum from matrices");
        assert!(dir.join("out/scf.ephmat/egnv").is_file());
        assert_eq!(c.execute_record(task), 0);
        success(c.direct_advance(2, &ARGS, "independent-width-collect"));
        if let Some((old, hash)) = &previous {
            assert_eq!(store::hash_file(old).unwrap(), *hash, "Changing width must preserve old scientific output");
        }
        let output = dir.join("epw.out");
        previous = Some((output.clone(), store::hash_file(&output).unwrap()));
        assert_eq!(c.calls().0, scientific_calls.0, "No upstream QE rerun");
        assert_eq!(c.calls().1.matches("fine ").count(), scientific_calls.1.matches("fine ").count());
        assert_eq!(store::hash_file(&fine.join("epw_fine.out")).unwrap(), fine_log);
    }
}

#[test]
fn batch_prepared_script_comments_do_not_require_fingerprints() {
    let c = Case::gap_batch(config::Mode::Both, vec![20., 30.], None);
    success(c.direct_advance(2, &["--stage", "superconductivity"], "batch-prepare-only"));
    let iso = c.method_task("iso");
    assert_eq!(iso["state"], "prepared");
    let script = PathBuf::from(iso["script"]["path"].as_str().unwrap());
    let text = fs::read_to_string(&script).unwrap();
    fs::write(script, format!("{text}\n# independent debug edit\n")).unwrap();
    let result = c.direct_advance(2, &ARGS, "batch-edited-prepare-submit");
    success(result);
    assert_eq!(c.method_task("iso")["state"], "submitted");
    // 一个方法的身份冲突不妨碍另一个方法提交。
    assert!(c.method_task("aniso")["job"].is_u64());
    assert_eq!(
        fs::read_to_string(c.root.join("submissions"))
            .unwrap()
            .lines()
            .count(),
        3
    );
}

#[test]
fn batch_explicit_native_mu_values_use_method_range_directories() {
    for (mode, temperatures, mu, expected) in [
        (
            config::Mode::Iso,
            vec![20., 25.5, 40.],
            None,
            "iso-mu0.13-Tc20-40-degaussq0.5meV",
        ),
        (
            config::Mode::Aniso,
            vec![20.25],
            Some(0.1),
            "aniso-mu0.1-Tc20.25-20.25-degaussq0.5meV",
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
        assert!(!dir.join("epw.err").exists());
        let recorded = c.method_task(name);
        assert_eq!(recorded["schema_version"], 3);
        assert_eq!(recorded["stdout"], recorded["stderr"]);
        success(c.direct_advance(2, &ARGS, "batch-collect"));
        let run = epwflow::superconductivity::points::Run::load(
            &c.root.join("run/8.epw-superconductivity/manifest.json"),
        )
        .unwrap();
        assert!(run.execution_complete && run.data_complete);
        assert!(run.attempts.iter().all(|a| a.directory == dir));
        assert!(fs::read_dir(dir).unwrap().all(|e| {
            let e=e.unwrap(); !e.path().is_dir() || e.file_name()=="out"
        }));
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
    // 纯原生脚本保留求解器退出码；提前结束由作业外核验拒绝。
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
    before.attempts[0].record.as_ref().unwrap().verify().unwrap();
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
    success(c.direct_advance(2, &ARGS, "batch-valid-before-log-edit"));
    let valid = epwflow::superconductivity::points::Run::load(&c.root.join("run/8.epw-superconductivity/manifest.json")).unwrap();
    let iso = c.method_task("iso");
    let log = PathBuf::from(iso["directory"].as_str().unwrap()).join("point-30.out");
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
    // 作业外归集已冻结有效结果；后来修改原日志必须拒绝复用，不能追认或重算。
    assert!(epwflow::superconductivity::points::Run::load(&c.root.join("run/8.epw-superconductivity/manifest.json")).is_err());
    assert!(valid.execution_complete);
    assert!(valid
        .attempts
        .iter()
        .filter(|a| a.request.mode == config::Mode::Aniso)
        .all(|a| a.status == "valid"));
    for entry in valid.attempts.iter().filter(|a|a.request.mode==config::Mode::Aniso) {
        epwflow::superconductivity::verify::record(&entry.record.as_ref().unwrap().path).unwrap();
    }
}
