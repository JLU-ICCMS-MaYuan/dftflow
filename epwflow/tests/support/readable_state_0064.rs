//! 0064：真实公开进程与受控科学替身验证八日志协议，不启动真实 QE/EPW。
use super::*;
use epwflow::foundation::stages::Step;

fn task(c: &Case, mode: Option<&str>) -> Value {
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    let t = if let Some(mode) = mode {
        epwflow::foundation::native_jobs::group_task(&cfg, mode).unwrap()
    } else {
        epwflow::foundation::native_jobs::active_task(&cfg).unwrap()
    }
    .unwrap();
    serde_json::to_value(t).unwrap()
}
fn eight(c: &Case) {
    let root = c.root.join("run/epw-state");
    let names = fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(names.len(), 8, "{names:?}");
    for name in names {
        assert!(name.ends_with(".log"));
        let text = fs::read_to_string(root.join(name)).unwrap();
        assert!(
            text.contains("【阶段】")
                && text.contains("【下一步】")
                && !text.contains("schema_version")
        );
    }
}
#[test]
fn readable_0064_versions_preserve_raw_text_and_old_missing_input_never_blocks_prepare() {
    let c = Case::numbered();
    c.direct_setup();
    let p = c.root.join("input.toml");
    let original = fs::read(&p).unwrap();
    success(c.numbered_cli(0, &["--check"], None, "readable-check-a"));
    success(c.numbered_cli(0, &["--check"], None, "readable-check-aa"));
    let mut changed = original.clone();
    changed.extend_from_slice(b"\n# user annotation\n");
    fs::write(&p, &changed).unwrap();
    success(c.numbered_cli(0, &["--check"], None, "readable-check-b"));
    fs::write(&p, &original).unwrap();
    success(c.numbered_cli(0, &["--check"], None, "readable-check-a2"));
    assert_eq!(fs::read(c.root.join("0000-config.toml")).unwrap(), original);
    assert_eq!(fs::read(c.root.join("0001-config.toml")).unwrap(), changed);
    assert_eq!(fs::read(c.root.join("0002-config.toml")).unwrap(), original);
    assert!(!c.root.join("0003-config.toml").exists());
    eight(&c);
    let root = c.root.join("run");
    store::write(
        &root.join("epw-state/layout.json"),
        &json!({"schema_version":1,"phonon":"nosplit"}),
    )
    .unwrap();
    store::write(
        &root.join("epw-state/native-active.json"),
        &json!({"state":"failed","inputs":[{"path":"/missing/archived/scf.in"}]}),
    )
    .unwrap();
    success(c.numbered_cli(
        0,
        &["--stage", "scf", "--resume", "-s"],
        None,
        "readable-missing-input",
    ));
    eight(&c);
    assert!(!root.join("epw-state/native-active.json").exists());
    let t = task(&c, None);
    assert_eq!(t["state"], "submitted");
    assert!(root.join("1.qe-scf/1.qe-scf.jobs").is_file());
}
#[test]
fn readable_0064_phonon_chain_and_fine_use_only_logs_jobs_and_scientific_outputs() {
    let c = Case::numbered();
    c.direct_setup();
    c.numbered_change(|cfg| {
        cfg.interpolation.phonon_smearings_mev = vec![0.5, 1.];
        cfg.superconductivity.task = config::SuperconductivityTask::Gap;
        cfg.superconductivity.mode = config::Mode::Both;
        cfg.superconductivity.gap_temperatures_k = vec![20., 30., 40.];
        cfg.superconductivity.iso = None;
        cfg.superconductivity.aniso = None;
    });
    success(c.numbered_cli(
        0,
        &["--stage", "phonon", "-r"],
        Some(201),
        "readable-phonon",
    ));
    eight(&c);
    c.put("job-201.state", "COMPLETED");
    success(c.numbered_cli(
        1,
        &["--stage", "wannier", "-r"],
        Some(202),
        "readable-wannier",
    ));
    c.put("job-202.state", "COMPLETED");
    success(c.numbered_cli(
        1,
        &["--stage", "coarse", "-r"],
        Some(203),
        "readable-coarse",
    ));
    c.put("job-203.state", "COMPLETED");
    c.put("epw-failure", "fine-second-exit");
    assert!(!c
        .numbered_cli(
            2,
            &["--stage", "fine", "-r"],
            Some(204),
            "readable-second-smearing-failed"
        )
        .status
        .success());
    c.put("job-204.state", "FAILED");
    let first =
        store::Identity::capture(&c.root.join("run/7.epw-fine/smearing-0001/scf.a2f")).unwrap();
    fs::remove_file(c.root.join("epw-failure")).unwrap();
    success(c.numbered_cli(
        2,
        &["--stage", "fine", "--resume", "-r"],
        Some(205),
        "readable-fine",
    ));
    first.verify().unwrap();
    c.put("job-205.state", "COMPLETED");
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    for step in [
        Step::Scf,
        Step::Nscf,
        Step::Electronic,
        Step::Phonon,
        Step::Wannier,
        Step::Coarse,
        Step::Fine,
    ] {
        assert!(
            epwflow::foundation::stages::reusable(&cfg, step).unwrap(),
            "{}",
            step.name()
        );
    }
    let before = c.calls();
    success(c.numbered_cli(2, &["--stage", "fine", "-r"], Some(206), "readable-reuse"));
    assert_eq!(before, c.calls());
    c.put("job-206.state", "COMPLETED");
    eight(&c);
    c.put(
        "submissions",
        "allocation\nallocation\nallocation\nallocation\nallocation\nallocation\n",
    );
    c.put(
        "gap-fail30",
        "test transient stop after first valid temperature",
    );
    success(c.numbered_cli(
        2,
        &["--stage", "superconductivity", "-s"],
        None,
        "readable-both-submit",
    ));
    let submissions = fs::read(c.root.join("submissions")).unwrap();
    success(c.numbered_cli(
        2,
        &["--stage", "superconductivity", "-s"],
        None,
        "readable-both-active",
    ));
    assert_eq!(submissions, fs::read(c.root.join("submissions")).unwrap());
    assert_eq!(c.execute_record(task(&c, Some("iso"))), 17);
    assert_eq!(c.execute_record(task(&c, Some("aniso"))), 0);
    assert!(!c
        .numbered_cli(
            2,
            &["--stage", "superconductivity", "-s"],
            None,
            "readable-partial-gap"
        )
        .status
        .success());
    let iso_dir = PathBuf::from(task(&c, Some("iso"))["directory"].as_str().unwrap());
    let valid = fs::read(iso_dir.join("point-20.json")).unwrap();
    fs::remove_file(c.root.join("gap-fail30")).unwrap();
    success(c.numbered_cli(
        2,
        &["--stage", "superconductivity", "--resume", "-s"],
        None,
        "readable-partial-resume",
    ));
    let input = fs::read_to_string(iso_dir.join("epw.in")).unwrap();
    assert!(input.contains("nstemp = 2") && !input.contains("temps(1) = 20"));
    assert_eq!(c.execute_record(task(&c, Some("iso"))), 0);
    success(c.numbered_cli(
        2,
        &["--stage", "superconductivity", "-s"],
        None,
        "readable-gap-finished",
    ));
    assert_eq!(valid, fs::read(iso_dir.join("point-20.json")).unwrap());
    assert!(epwflow::foundation::stages::reusable(&cfg, Step::Superconductivity).unwrap());
    assert_eq!(before.0, c.calls().0);
    assert_eq!(before.1, c.calls().1);
    eight(&c);

    assert!(!c
        .root
        .join("run/4.qe-phonon-nosplit/save/manifest.json")
        .exists());
    assert!(!c.root.join("run/6.epw-coarse/manifest.json").exists());
    assert!(!c.root.join("run/7.epw-fine/manifest.json").exists());
}

#[test]
fn readable_0064_failed_phonon_post_resumes_without_repeating_ph_or_electronic() {
    let c = Case::numbered();
    c.direct_setup();
    c.put("q2r-failure", "temporary failure");
    let program = c.root.join("q2r.x");
    let original = fs::read_to_string(&program).unwrap();
    fs::rename(&program, c.root.join("q2r-implementation")).unwrap();
    c.script(
        "q2r.x",
        &format!(
            "#!/bin/bash\nif [[ -e {} ]]; then exit 17; fi\n{} \"$@\"\n",
            shell_quote(c.root.join("q2r-failure").to_str().unwrap()),
            shell_quote(c.root.join("q2r-implementation").to_str().unwrap())
        ),
    );
    assert!(!c
        .numbered_cli(
            0,
            &["--stage", "phonon", "-r"],
            Some(201),
            "readable-q2r-failed"
        )
        .status
        .success());
    let before = c.calls().0;
    assert_eq!(before.lines().filter(|s| *s == "ph").count(), 1);
    c.put("job-201.state", "FAILED");
    fs::remove_file(c.root.join("q2r-failure")).unwrap();
    success(c.numbered_cli(
        0,
        &["--stage", "phonon", "--resume", "-r"],
        Some(202),
        "readable-q2r-resume",
    ));
    let after = c.calls().0;
    for kind in ["pw", "ph"] {
        assert_eq!(
            before.lines().filter(|s| *s == kind).count(),
            after.lines().filter(|s| *s == kind).count(),
            "{kind} repeated"
        );
    }
    assert_eq!(after.lines().filter(|s| *s == "q2r").count(), 1);
    eight(&c);
    let _ = original;
}
#[test]
fn readable_0064_live_and_unknown_submissions_cannot_be_overwritten() {
    let c = Case::numbered();
    c.direct_setup();
    success(c.numbered_cli(0, &["--stage", "scf", "-s"], None, "readable-active-submit"));
    let input = fs::read(c.root.join("run/1.qe-scf/scf.in")).unwrap();
    for args in [
        vec!["--stage", "scf", "-s"],
        vec!["--stage", "scf", "--resume", "-s"],
    ] {
        assert!(!c
            .numbered_cli(0, &args, None, "readable-active-repeat")
            .status
            .success());
    }
    assert_eq!(
        fs::read(c.root.join("submissions"))
            .unwrap()
            .split(|b| *b == b'\n')
            .filter(|s| !s.is_empty())
            .count(),
        1
    );
    assert_eq!(input, fs::read(c.root.join("run/1.qe-scf/scf.in")).unwrap());
    use epwflow::foundation::readable_state as logs;
    logs::set_job(
        &c.root.join("run"),
        Step::Scf,
        "任务 · 1.qe-scf / scf",
        None,
    )
    .unwrap();
    logs::update(&c.root.join("run"), Step::Scf, |d| {
        d.set("任务 · 1.qe-scf / scf", "状态", "提交未知");
        Ok(())
    })
    .unwrap();
    assert!(!c
        .numbered_cli(
            0,
            &["--stage", "scf", "--resume", "-s"],
            None,
            "readable-unknown-repeat"
        )
        .status
        .success());
    eight(&c);
}

#[test]
fn readable_0064_missing_stage_log_still_checks_jobs_before_resubmission() {
    use epwflow::foundation::readable_state as logs;
    for (stage, step) in [("scf", Step::Scf), ("electronic", Step::Electronic)] {
        let c = Case::numbered();
        c.direct_setup();
        success(c.numbered_cli(0, &["--stage", stage, "-s"], None, "readable-orphan-submit"));
        let root = c.root.join("run");
        let jobs = logs::jobs(&root, step).unwrap();
        assert_eq!(jobs.values().copied().collect::<Vec<_>>(), vec![201]);
        let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
        let sentinel = step.directory(&cfg).join("still-running");
        fs::write(&sentinel, "active work").unwrap();
        fs::remove_file(logs::path(&root, step).unwrap()).unwrap();
        let submissions = fs::read(c.root.join("submissions")).unwrap();
        for unknown in [false, true] {
            if unknown {
                c.script("sacct", "#!/bin/bash\nexit 1\n");
            }
            let result = c.numbered_cli(
                0,
                &["--stage", stage, "--resume", "-s"],
                None,
                "readable-orphan-repeat",
            );
            assert!(!result.status.success());
            assert!(String::from_utf8_lossy(&result.stderr).contains("scheduler_unknown"));
            assert_eq!(submissions, fs::read(c.root.join("submissions")).unwrap());
            assert_eq!(fs::read_to_string(&sentinel).unwrap(), "active work");
            assert_eq!(logs::jobs(&root, step).unwrap(), jobs);
        }
        c.direct_setup();
        c.put("job-201.state", "COMPLETED");
        success(c.numbered_cli(
            0,
            &["--stage", stage, "--resume", "-s"],
            None,
            "readable-orphan-terminal",
        ));
        assert_eq!(
            logs::jobs(&root, step)
                .unwrap()
                .values()
                .copied()
                .collect::<Vec<_>>(),
            vec![202]
        );
        eight(&c);
    }
}

#[test]
fn readable_0064_external_phonon_import_keeps_only_imports_and_never_runs_pw_or_ph() {
    let (c, source) = super::phonon_import_tests::prepared();
    c.stage_setup();
    c.put("submissions", "manual allocation\nmanual allocation\n");
    c.put("job-201.state", "COMPLETED");
    c.put("job-202.state", "COMPLETED");
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    let files = epwflow::qe_data::save::files(&source).unwrap();
    let before = c.calls().0;
    success(c.numbered_cli(
        0,
        &["--import-phonon", source.to_str().unwrap(), "-s"],
        None,
        "readable-import-submit",
    ));
    success(c.stage_execute());
    let after = c.calls().0;
    for kind in ["pw", "ph"] {
        assert_eq!(
            before.lines().filter(|s| *s == kind).count(),
            after.lines().filter(|s| *s == kind).count()
        );
    }
    assert_eq!(files, epwflow::qe_data::save::files(&source).unwrap());
    assert!(epwflow::foundation::stages::reusable(&cfg, Step::Phonon).unwrap());
    let state = cfg.work_dir.join("epw-state");
    assert!(state.join("imports/phonon-source.json").is_file());
    assert_eq!(fs::read_dir(state).unwrap().count(), 9);
}

#[test]
fn readable_0064_queued_allocation_keeps_its_raw_version_after_user_edits() {
    let c = Case::numbered();
    c.stage_setup();
    success(c.numbered_cli(
        0,
        &["--stage", "electronic", "-s"],
        None,
        "readable-pinned-submit",
    ));
    let original = fs::read(c.root.join("0000-config.toml")).unwrap();
    c.numbered_change(|cfg| cfg.comparison.as_mut().unwrap().points_per_edge = 3);
    success(c.numbered_cli(0, &["--check"], None, "readable-pinned-edit"));
    let updated = fs::read(c.root.join("0001-config.toml")).unwrap();
    assert_ne!(original, updated);
    success(c.stage_execute());
    assert!(!c.root.join("0002-config.toml").exists());
    use epwflow::foundation::readable_state as logs;
    let doc = logs::read(&c.root.join("run"), Step::Electronic).unwrap();
    assert!(doc
        .get("工作区", "当前输入版本")
        .unwrap()
        .ends_with("0001-config.toml"));
    assert!(doc
        .get("阶段", "计算配置")
        .unwrap()
        .ends_with("0000-config.toml"));
    let points: Vec<[f64; 3]> =
        store::read(&c.root.join("run/3.qe-electronic/qe_eband/path-points.json")).unwrap();
    assert_eq!(points.len(), 3);
    eight(&c);
}

#[test]
fn readable_0064_missing_matdyn_output_repairs_only_that_numeric_task() {
    let c = Case::numbered();
    c.direct_setup();
    success(c.numbered_cli(
        0,
        &["--stage", "phonon", "-r"],
        Some(201),
        "missing-matdyn-initial",
    ));
    c.put("job-201.state", "COMPLETED");
    let calls = c.calls().0;
    fs::remove_file(c.root.join("run/4.qe-phonon-nosplit/scf.phdos.dat")).unwrap();
    success(c.numbered_cli(
        0,
        &["--stage", "phonon", "--resume", "-r"],
        Some(202),
        "missing-matdyn-repair",
    ));
    let after = c.calls().0;
    for kind in ["pw", "ph", "q2r"] {
        assert_eq!(
            calls.lines().filter(|s| *s == kind).count(),
            after.lines().filter(|s| *s == kind).count(),
            "{kind} 不应重算"
        );
    }
    assert_eq!(
        after.lines().filter(|s| *s == "matdyn").count(),
        calls.lines().filter(|s| *s == "matdyn").count() + 1
    );
    eight(&c);
}
