//! 阶段交付契约：第 5、6 步在分配内自动完成交付，第 7、8 步逐项推进；科学程序为隔离替身。
use super::*;
impl Case {
    pub(super) fn stage_setup(&self) {
        self.direct_setup();
        use epwflow::foundation::{readable_state as logs,stages::Step};
        let root=self.root.join("run");
        for step in Step::ALL {
            if let Ok(doc)=logs::read(&root,step){
                if matches!(doc.get("Stage allocation","Status"),Some("completed"|"failed")){
                    if let Some(job)=logs::jobs(&root,step).unwrap().get("Stage allocation"){
                        self.put(&format!("job-{job}.state"),if doc.get("Stage allocation","Status")==Some("completed"){"COMPLETED"}else{"FAILED"});
                    }
                }
            }
        }
        self.script(
            "scontrol",
            "#!/bin/bash\nprintf 'JobId=%s JobState=RUNNING NumNodes=1\\n' \"${@: -1}\"\n",
        );
    }
    pub(super) fn stage_execute(&self) -> Output {
        use epwflow::foundation::{readable_state as logs, stages::Step};
        let cfg = config::Config::load(&self.root.join("input.toml")).or_else(|_| logs::current_config(&self.root.join("run")).and_then(|p| config::Config::load(&p))).unwrap();
        let allocation = Step::ALL
            .into_iter()
            .filter_map(|step| {
                let doc = logs::read(&cfg.work_dir, step).unwrap();
                (doc.get("Stage allocation", "Status") == Some("submitted")).then_some((step, doc))
            })
            .next();
        if allocation.is_none() {
            if let Ok(stage) = fs::read_to_string(self.root.join("last-stage-target")) {
                return self.direct_stage_execute(stage.trim());
            }
            let task = self.direct_task();
            let dir = PathBuf::from(task["directory"].as_str().unwrap());
            let step = Step::ALL
                .into_iter()
                .find(|step| dir.starts_with(step.directory(&cfg)))
                .unwrap();
            return self.direct_stage_execute(step.name());
        }
        use std::{io::Write, os::unix::fs::MetadataExt};
        let (step, doc) = allocation.unwrap();
        let script = logs::resolve(&cfg.work_dir, doc.get("Stage allocation", "Script").unwrap());
        let script_body = fs::read(&script).unwrap();
        let log_path = script.with_file_name(format!("slurm-{}.log", step.name()));
        let mut open_log = fs::File::create(&log_path).unwrap();
        open_log
            .write_all(b"before stage initialization\n")
            .unwrap();
        let job = logs::jobs(&cfg.work_dir, step).unwrap()["Stage allocation"];
        let mut cmd = self.scheduled_command(&script, script.parent().unwrap(), job);
        cmd.env("SLURM_NTASKS", "1");
        let out = cmd.output().unwrap();
        open_log.write_all(b"after stage initialization\n").unwrap();
        assert_eq!(
            fs::read(&script).unwrap(),
            script_body,
            "Scripts at the current stage cannot disappear after the scientific catalogue is archived"
        );
        assert_eq!(
            open_log.metadata().unwrap().ino(),
            fs::metadata(&log_path).unwrap().ino(),
            "The log entrance must remain pointed Slurm Holdings inode"
        );
        assert_eq!(
            fs::read_to_string(log_path).unwrap(),
            "before stage initialization\nafter stage initialization\n"
        );
        self.put("stage-worker.stdout", &String::from_utf8_lossy(&out.stdout));
        self.put("stage-worker.stderr", &String::from_utf8_lossy(&out.stderr));
        self.put(
            &format!("job-{job}.state"),
            if out.status.success() {
                "COMPLETED"
            } else {
                "FAILED"
            },
        );
        out
    }
    fn direct_stage_execute(&self, stage: &str) -> Output {
        let bin = match stage { "scf" | "nscf" | "electronic" | "phonon" => 0, "wannier" | "coarse" => 1, _ => 2 };
        for _ in 0..128 {
            let task = self.direct_task();
            if task["state"] == "submitted" {
                assert!(task["job"].is_u64());
                self.execute_record(task);
            }
            let out =
                self.numbered_cli(bin, &["--stage", stage, "-s"], None, "native-stage-advance");
            self.put("stage-worker.stdout", &String::from_utf8_lossy(&out.stdout));
            self.put("stage-worker.stderr", &String::from_utf8_lossy(&out.stderr));
            let cfg=config::Config::load(&self.root.join("input.toml")).unwrap();
            let step=epwflow::foundation::stages::Step::ALL.into_iter().find(|s|s.name()==stage).unwrap();
            let pending=epwflow::foundation::native_jobs::tasks(&cfg).unwrap().iter().any(|task| matches!(task.state.as_str(),"submitted"|"running"|"submitting"|"submission_unknown"));
            if !out.status.success()
                || (!pending && epwflow::foundation::stages::reusable(&cfg,step).unwrap_or(false))
            {
                return out;
            }
        }
        panic!("native {stage} Not completed in finite public advances");
    }
    pub(super) fn stage_submit(&self, stage: &str, resume: bool) -> Output {
        let mut args = vec!["--stage", stage, "-s"];
        if resume {
            args.push("--resume");
        }
        self.numbered_cli(
            if matches!(stage, "scf" | "nscf" | "electronic" | "phonon") { 0 } else { 1 },
            &args,
            None,
            "stage-submit",
        )
    }
    pub(super) fn post_calls(&self) -> String {
        fs::read_to_string(self.root.join("post-calls")).unwrap_or_default()
    }
}
#[test]
fn wannier_allocation_publishes_without_phonons_and_repairs_only_derivatives() {
    let c = Case::numbered();
    c.stage_setup();
    success(c.stage_submit("wannier", false));
    success(c.stage_execute());
    let result = c.root.join("run/results/5.epw-wannier");
    for name in [
        "plot/electronic.png",
        "plot/electronic-from-lowest-to-highest.png",
        "electronic-bands.csv",
        "electronic-dos.csv",
        "sampling.json",
        "metrics.json",
        "stage-manifest.json",
    ] {
        assert!(result.join(name).is_file(), "Missing {name}");
    }
    assert!(!c.root.join("run/4.qe-phonon-nosplit/ph.out").exists());
    assert!(!c.root.join("run/6.epw-coarse").exists());
    assert_eq!(c.post_calls(), "postw90\n");
    let manifest: Value = store::read(&result.join("stage-manifest.json")).unwrap();
    assert!(manifest["parameters"]["fermi_ev"]
        .as_f64()
        .unwrap()
        .is_finite());
    assert_eq!(
        manifest["parameters"]["outer_window_ev"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        manifest["parameters"]["frozen_window_ev"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        c.calls()
            .1
            .lines()
            .filter(|l| l.starts_with("wannier "))
            .count(),
        1
    );
    let scientific = c.calls();
    let post = c.post_calls();
    fs::remove_file(result.join("plot/electronic.png")).unwrap();
    fs::remove_file(
        c.root
            .join("run/5.epw-wannier/plot/electronic-from-lowest-to-highest.png"),
    )
    .unwrap();
    success(c.stage_submit("wannier", false));
    success(c.stage_execute());
    assert_eq!(scientific, c.calls());
    assert_eq!(post, c.post_calls());
    epwflow::foundation::stage_results::check(&c.root.join("run/results")).unwrap();
    let submissions = fs::read(c.root.join("submissions")).unwrap();
    success(c.stage_submit("wannier", false));
    assert_eq!(submissions, fs::read(c.root.join("submissions")).unwrap());
    let moved = c.root.join("portable-results");
    copy_results(&c.root.join("run/results"), &moved);
    fs::remove_file(moved.join("5.epw-wannier/plot/electronic.png")).unwrap();
    let mut cmd = Command::new(BINS[1]);
    cmd.args(["--plot", "--results", moved.to_str().unwrap()])
        .env("PATH", "");
    let out = cmd.output().unwrap();
    success(out);
    epwflow::foundation::stage_results::check(&moved).unwrap();
    assert_eq!(scientific, c.calls());
    assert_eq!(post, c.post_calls());
}
#[test]
fn wannier_post_failure_keeps_model_and_recovery_never_calls_wannier90() {
    let c = Case::numbered();
    c.stage_setup();
    c.put("post-failure", "postw90 STOP");
    success(c.stage_submit("wannier", false));
    assert!(!c.stage_execute().status.success());
    let model = c.root.join("run/5.epw-wannier/scf.ukk");
    let before = fs::read(&model).unwrap();
    let calls = c.calls();
    assert!(!c.stage_submit("wannier", false).status.success());
    fs::remove_file(c.root.join("post-failure")).unwrap();
    success(c.stage_submit("wannier", true));
    success(c.stage_execute());
    assert_eq!(before, fs::read(&model).unwrap());
    assert_eq!(calls, c.calls());
    assert!(c.post_calls().lines().all(|l| l == "postw90"));
}
#[test]
fn coarse_publishes_phonons_using_stage_five_electrons_without_reference_pw() {
    let c = Case::numbered();
    c.stage_setup();
    success(c.stage_submit("wannier", false));
    success(c.stage_execute());
    let before = c.calls();
    let post = c.post_calls();
    let electron = store::hash_file(
        &c.root
            .join("run/5.epw-wannier/comparison-native/band.eig"),
    )
    .unwrap();
    success(c.stage_submit("coarse", false));
    success(c.stage_execute());
    assert_eq!(
        before.0.lines().filter(|l| *l == "pw").count(),
        c.calls().0.lines().filter(|l| *l == "pw").count()
    );
    assert_eq!(post, c.post_calls());
    assert_eq!(
        electron,
        store::hash_file(
            &c.root
                .join("run/5.epw-wannier/comparison-native/band.eig")
        )
        .unwrap()
    );
    let source: Value = store::read(
        &c.root
            .join("run/6.epw-coarse/comparison-native/sources.json"),
    )
    .unwrap();
    assert!(source["epw_bands"]["path"]
        .as_str()
        .unwrap()
        .contains("5.epw-wannier/comparison-native"));
    assert!(source["qe_bands_xml"]["path"]
        .as_str()
        .unwrap()
        .contains("3.qe-electronic/qe_eband"));
    assert!(!c
        .root
        .join("run/6.epw-coarse/comparison-native/electron")
        .exists());
    let results = c.root.join("run/results");
    epwflow::foundation::stage_results::check(&results).unwrap();
    let figures = results.join("6.epw-coarse/plot");
    assert!(figures.join("phonon.png").is_file());
    assert!(figures.join("phonon.png").is_file());
    let calls = c.calls();
    fs::remove_file(figures.join("phonon.png")).unwrap();
    success(c.stage_submit("coarse", false));
    success(c.stage_execute());
    assert_eq!(calls, c.calls());
    assert_eq!(post, c.post_calls());
    let moved = c.root.join("portable-all-stages");
    copy_results(&results, &moved);
    let mut cmd = Command::new(BINS[1]);
    cmd.args(["--plot", "--results", moved.to_str().unwrap()])
        .env("PATH", "");
    success(cmd.output().unwrap());
    epwflow::foundation::stage_results::check(&moved).unwrap();
    assert_eq!(calls, c.calls());
    assert_eq!(post, c.post_calls());
    // 新电子拟合改变第 6 步身份，但同一 PH、ASR 和 q 点的后处理仍可复用。
    c.numbered_change(|cfg| cfg.wannier.outer_window_ev.as_mut().unwrap()[0] = 1.4);
    success(c.stage_submit("coarse", false));
    success(c.stage_execute());
    assert_eq!(
        calls.0,
        c.calls().0,
        "Electronic model changes cannot be repeated PW/PH/q2r/matdyn"
    );
    let phonon = c.root.join("run/6.epw-coarse/comparison-native");
    assert!(phonon.join("force-constants/reference.fc.xml").is_file());
    assert!(phonon.join("frequencies/reference.freq").is_file());
    // 显式重建上游 SCF 时，原 PH 来源即失效，不能跨来源保留比较检查点。
    success(c.stage_submit("scf", true));
    let coarse = epwflow::foundation::readable_state::read(
        &c.root.join("run"), epwflow::foundation::stages::Step::Coarse,
    ).unwrap();
    assert!(coarse.get("Substep · comparison-q2r", "Status").is_none());
    assert!(coarse.get("Substep · comparison-matdyn", "Status").is_none());
}

#[test]
fn matdyn_recovery_reuses_successful_force_constants_and_native_coarse() {
    let c = Case::numbered();
    c.accounting(Some(201));
    success(c.numbered_cli(
        0,
        &["--stage", "phonon", "-r"],
        Some(201),
        "phonon-reference",
    ));
    c.stage_setup();
    c.put("job-201.state", "COMPLETED");
    c.put("submissions", "allocated preparation\n");
    success(c.stage_submit("wannier", false));
    success(c.stage_execute());
    c.put("comparison-failure", "matdyn");
    success(c.stage_submit("coarse", false));
    assert!(!c.stage_execute().status.success());
    let before = c.calls();
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    assert!(epwflow::foundation::stages::reusable(&cfg, epwflow::foundation::stages::Step::Coarse).unwrap());
    let force = c
        .root
        .join("run/6.epw-coarse/comparison-native/force-constants/reference.fc.xml");
    let hash = store::hash_file(&force).unwrap();
    fs::remove_file(c.root.join("comparison-failure")).unwrap();
    success(c.stage_submit("coarse", true));
    success(c.stage_execute());
    assert_eq!(hash, store::hash_file(&force).unwrap());
    assert_eq!(
        before.0.lines().filter(|s| *s == "q2r").count(),
        c.calls().0.lines().filter(|s| *s == "q2r").count()
    );
    assert_eq!(
        before
            .1
            .lines()
            .filter(|s| s.starts_with("coarse "))
            .count(),
        c.calls()
            .1
            .lines()
            .filter(|s| s.starts_with("coarse "))
            .count()
    );
    assert_eq!(
        before.0.lines().filter(|s| *s == "pw").count(),
        c.calls().0.lines().filter(|s| *s == "pw").count()
    );
    let reason: Value = store::read(&force.parent().unwrap().join("reuse-decision.json")).unwrap();
    assert_eq!(
        reason["required_programs"],
        json!(["q2r.x"])
    );
    assert!(reason["reason"].as_str().unwrap().contains("ASR"));
}

#[test]
fn corrupted_embedded_coordinates_are_rejected_without_refitting() {
    let c = Case::numbered();
    c.stage_setup();
    success(c.stage_submit("wannier", false));
    success(c.stage_execute());
    let before = c.calls();
    c.put("run/5.epw-wannier/scf_band.kpt", "broken coordinates");
    assert!(!c.stage_submit("wannier", false).status.success());
    assert_eq!(before, c.calls());
}

#[test]
fn registered_legacy_electronic_samples_migrate_without_refitting_or_changing_science() {
    let c = Case::numbered();
    c.stage_setup();
    success(c.stage_submit("wannier", false));
    success(c.stage_execute());
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    let sampling = epwflow::interpolation::electronic_sampling::plan(&cfg).unwrap();
    let fit = c.root.join("run/5.epw-wannier");
    let post = fit.join("comparison-native");
    let old = c.root.join("run/6.epw-coarse/comparison-native");
    fs::create_dir_all(old.join("epw")).unwrap();
    let band = old.join("epw/band.eig");
    // 旧协议必须包含路径及网格；新版仅路径输出不能冒充旧采样。
    let native = epwflow::interpolation::native::parse_bands(
        &fs::read_to_string(post.join("band.eig")).unwrap(),
    )
    .unwrap();
    let energies = native.values[0]
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let mut legacy = format!(
        " &plot nbnd= {}, nks= {} /\n",
        native.values[0].len(),
        sampling.path.len() + sampling.mesh.len()
    );
    for point in sampling
        .path
        .iter()
        .map(|p| p.crystal)
        .chain(sampling.mesh.iter().copied())
    {
        let cart = epwflow::interpolation::electronic_sampling::cart(point, &sampling.reciprocal);
        legacy.push_str(&format!(
            "{} {} {}\n{energies}\n",
            cart[0], cart[1], cart[2]
        ));
    }
    fs::write(&band, legacy).unwrap();
    let source = |p: PathBuf| json!({"sha256":store::hash_file(&p).unwrap(),"path":p});
    let xml = cfg
        .work_dir
        .join("2.qe-nscf/out/scf.save/data-file-schema.xml");
    let old_sources = json!({"schema_version":2,"run_id":"legacy","ph_mode":"nosplit",
        "fermi_reference":source(xml.clone()),"qe_mesh_xml":source(xml),
        "qe_bands_xml":source(cfg.work_dir.join("3.qe-electronic/qe_eband/out/scf.save/data-file-schema.xml")),
        "epw_bands":source(band.clone()),"qe_phonon_bands":source(band.clone()),
        "epw_phonon_bands":source(band.clone()),"epw_log":source(fit.join("epw_wannier.out")),
        "provenance":[source(fit.join("scf.chk"))],"path":sampling.path,"ticks":sampling.ticks,
        "mesh":sampling.mesh,"mesh_shape":cfg.qe.nscf.k_grid,"reciprocal":sampling.reciprocal,
        "window_ev":[-1.,1.],"wannier_wout":source(fit.join("scf.wout")),
        "electronic_sigma_ev":0.1,"phonon_sigma_cm1":10.});
    store::write(&old.join("sources.json"), &old_sources).unwrap();
    fs::remove_file(fit.join("embedded-bands.json")).unwrap();
    let model = fit.join("scf.ukk");
    let before = fs::read(&model).unwrap();
    let calls = c.calls();
    let dos = c.post_calls();
    let wout = store::hash_file(&fit.join("scf.wout")).unwrap();
    let old_hash = store::hash_file(&old.join("sources.json")).unwrap();
    success(c.stage_submit("wannier", false));
    success(c.stage_execute());
    assert_eq!(before, fs::read(&model).unwrap());
    assert_eq!(calls, c.calls());
    assert_eq!(dos, c.post_calls());
    assert_eq!(wout, store::hash_file(&fit.join("scf.wout")).unwrap());
    assert_eq!(
        old_hash,
        store::hash_file(&old.join("sources.json")).unwrap()
    );
    let origin: Value = store::read(&post.join("band-origin.json")).unwrap();
    assert_ne!(origin["driver"], "legacy_epw_readonly", "Effective current raw output takes precedence over old, more cache");

}

#[test]
fn sparse_high_symmetry_path_preserves_unequal_native_sampling_and_dos() {
    let c = Case::numbered();
    c.stage_setup();
    c.put(
        "KPATH.in",
        "0 0 0 G\n0.5 0 0 X\n0.5 0 0 X\n0.5 0.25 0 M\n0 0 0.5 A\n0.25 0 0.5 L\n",
    );
    success(c.stage_submit("wannier", false));
    success(c.stage_execute());
    let dir = c.root.join("run/5.epw-wannier");
    let input = fs::read_to_string(dir.join("epw_wannier.in")).unwrap();
    assert!(input.contains("G 0 0 0 X 0.5 0 0"));
    assert!(input.contains("X 0.5 0 0 M 0.5 0.25 0"));
    assert!(input.contains("A 0 0 0.5 L 0.25 0 0.5"));
    assert!(!input.contains("P0 ") && !input.contains("Q0 "));
    let result = c.root.join("run/results/5.epw-wannier");
    let csv = fs::read_to_string(result.join("electronic-bands.csv")).unwrap();
    let qe = csv.lines().filter(|l| l.starts_with("qe,")).count();
    let epw = csv.lines().filter(|l| l.starts_with("epw,")).count();
    assert!(qe > epw && epw > 0, "Different numbers of native sites should be retained on both sides");
    let metrics: Value = store::read(&result.join("metrics.json")).unwrap();
    assert_eq!(metrics["independent_electronic_sampling"], true);
    assert!(
        metrics["electronic_path"]["paired_states"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert_eq!(metrics["electronic_mesh"]["points"], 0);
    assert!(
        !fs::read_to_string(result.join("electronic-bands.json"))
            .unwrap()
            .contains("coverage")
    );
    assert!(result.join("plot/electronic.png").exists());
    assert_eq!(c.post_calls(), "postw90\n");
    let bands_hash = store::hash_file(&result.join("electronic-bands.csv")).unwrap();
    success(c.stage_submit("coarse", false));
    success(c.stage_execute());
    let comparison = c.root.join("run/6.epw-coarse/comparison");
    assert_eq!(
        store::hash_file(&comparison.join("electronic_bands.csv")).unwrap(),
        bands_hash
    );
    let metadata: Value = store::read(&comparison.join("metadata.json")).unwrap();
    assert_eq!(metadata["electronic_independent_sampling"], true);
    assert_eq!(c.post_calls(), "postw90\n");
    epwflow::foundation::stage_results::check(&c.root.join("run/results")).unwrap();
}

#[test]
fn sip2_titled_path_survives_wannier_resume_without_recomputing_references() {
    let c = Case::numbered();
    c.stage_setup();
    c.put("KPATH.in", include_str!("../fixtures/sip2-kpath.txt"));
    let path = c.root.join("KPATH.in");
    let path_hash = store::hash_file(&path).unwrap();
    success(c.stage_submit("wannier", false));
    success(c.stage_execute());
    let input = fs::read_to_string(c.root.join("run/5.epw-wannier/epw_wannier.in")).unwrap();
    assert!(input.contains("Y 0 0.5 0 T 0 0.5 0.5"));
    assert!(input.contains("U 0.5 0 0.5 X 0.5 0 0"));
    assert!(input.contains("S 0.5 0.5 0 R 0.5 0.5 0.5"));
    let references = c.calls().0;
    let electronic = c.root.join("run/3.qe-electronic/qe_eband/eband.out");
    let electronic_hash = store::hash_file(&electronic).unwrap();
    c.numbered_change(|cfg| cfg.wannier.localization.max_iterations = 500);
    success(c.stage_submit("wannier", true));
    success(c.stage_execute());
    assert_eq!(
        c.calls().0,
        references,
        "Recovery is not recalculated SCF/NSCF/PH/Electronic reference"
    );
    assert!(!references.lines().any(|line| line == "ph"));
    assert_eq!(store::hash_file(&path).unwrap(), path_hash);
    assert_eq!(store::hash_file(&electronic).unwrap(), electronic_hash);
    assert_eq!(
        c.calls()
            .1
            .lines()
            .filter(|line| line.starts_with("wannier "))
            .count(),
        2
    );
    assert!(
        fs::read_to_string(c.root.join("run/5.epw-wannier/epw_wannier.in"))
            .unwrap()
            .contains("num_iter = 500")
    );
    assert!(c.root.join("run/5.epw-wannier/scf.ukk").is_file());
    assert!(c
        .root
        .join("run/results/5.epw-wannier/stage-manifest.json")
        .is_file());
}

#[test]
fn failed_wannier_generation_error_recovers_without_archived_inputs() {
    let c = Case::numbered();
    c.stage_setup();
    c.put(
        "KPATH.in",
        "custom title\n20\nLine-Mode\nReciprocal\n0 0 0 G\n0.5 0 0 X\n0.5 0 0 X\n0.5 0.1 0 M\n",
    );
    c.numbered_change(|cfg| {
        cfg.set_native(
            &["5.epw-wannier", "wannier90", "bands_num_points"],
            json!(100),
        )
        .unwrap()
    });
    let solver = fs::read_to_string(c.root.join("epw_native_io.py")).unwrap();
    c.put(
        "epw_native_io.py",
        &solver.replace(
            "<<< Wannierisation convergence criteria satisfied >>>",
            "simulated iteration limit",
        ),
    );
    success(c.stage_submit("wannier", false));
    let failure = c.stage_execute();
    assert!(!failure.status.success());
    assert!(String::from_utf8_lossy(&failure.stderr).contains("Evidence of localization"));
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    let old = serde_json::to_value(
        epwflow::foundation::native_jobs::tasks(&cfg).unwrap().into_iter()
            .find(|task| task.directory == cfg.work_dir.join("5.epw-wannier") && task.stem == "epw_wannier")
            .unwrap()
    ).unwrap();
    assert_eq!(old["state"], "failed");
    let scientific = c.calls().0;
    let path_hash = store::hash_file(&c.root.join("KPATH.in")).unwrap();
    c.numbered_change(|cfg| {
        cfg.wannier.localization.max_iterations = 500;
        cfg.set_native(
            &["5.epw-wannier", "wannier90", "bands_num_points"],
            json!(1),
        )
        .unwrap();
    });
    success(c.stage_submit("wannier", true));
    let failure = c.stage_execute();
    assert!(!failure.status.success());
    assert!(String::from_utf8_lossy(&failure.stderr).contains("zero interval"));
    assert!(!c.root.join("run/5.epw-wannier/epw_wannier.in").exists());
    let pointer = c.root.join("run/epw-state/native-active.json");
    store::write(&pointer, &old).unwrap();
    assert!(!c.root.join("run/epw-state/history").exists());
    c.put("epw_native_io.py", &solver);
    c.numbered_change(|cfg| {
        cfg.set_native(
            &["5.epw-wannier", "wannier90", "bands_num_points"],
            json!(100),
        )
        .unwrap()
    });
    success(c.stage_submit("wannier", true));
    success(c.stage_execute());
    assert!(!pointer.exists());
    assert!(!c.root.join("run/epw-state/history").exists());
    assert_eq!(c.calls().0, scientific);
    assert_eq!(
        store::hash_file(&c.root.join("KPATH.in")).unwrap(),
        path_hash
    );
    assert_eq!(
        c.calls()
            .1
            .lines()
            .filter(|line| line.starts_with("wannier "))
            .count(),
        2
    );
    assert!(c.root.join("run/5.epw-wannier/scf.ukk").is_file());
}

#[test]
fn band_ranges_are_drawn_inside_each_stage_and_repaired_without_scientific_calls() {
    let c = Case::numbered();
    c.stage_setup();
    for (stage, directory) in [
        ("scf", "1.qe-scf/plots"),
        ("nscf", "2.qe-nscf/plots"),
        ("electronic", "3.qe-electronic/qe_eband/plots"),
    ] {
        success(c.stage_submit(stage, false));
        success(c.stage_execute());
        let image = c.root.join("run").join(directory).join(format!("{stage}-band-ranges.png"));
        assert!(image.is_file(), "The end of the phase must have a graph:{}", image.display());
        let mut reader = png::Decoder::new(fs::File::open(&image).unwrap()).read_info().unwrap();
        let info = reader.info();
        assert_eq!(info.pixel_dims.unwrap().xppu, 11812);
        assert!(info.width >= 3000 && info.height >= 2000);
        let mut pixels = vec![0;reader.output_buffer_size()];
        reader.next_frame(&mut pixels).unwrap();
        let report:Value=store::read(&image.with_extension("json")).unwrap();
        assert_eq!(report["stage"],stage);
        if stage == "scf" { assert!(!c.root.join("run/2.qe-nscf/out").exists(),"SCF I can't wait NSCF"); }
        if stage == "nscf" { assert!(!c.root.join("run/3.qe-electronic/qe_eband/out").exists(),"NSCF I can't wait electronic"); }
    }
    let before = c.calls();
    let post = c.post_calls();
    let image = c.root.join("run/2.qe-nscf/plots/nscf-band-ranges.png");
    fs::remove_file(&image).unwrap();
    success(c.stage_submit("nscf", false));
    success(c.stage_execute());
    assert!(image.is_file());
    assert_eq!(before,c.calls()); assert_eq!(post,c.post_calls());
    let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
    assert!(!epwflow::foundation::workspace_science::post_pending(&cfg,epwflow::foundation::stages::Step::Electronic).unwrap());
    let moved=c.root.join("moved-results");
    copy_results(&c.root.join("run/results"),&moved);
    let copied=moved.join("2.qe-nscf/band-ranges/nscf-band-ranges.png");
    fs::remove_file(&copied).unwrap();
    let out=Command::new(BINS[1]).args(["--plot","--results",moved.to_str().unwrap()]).env("PATH","").output().unwrap();
    success(out); assert!(copied.is_file());
    assert_eq!(before,c.calls()); assert_eq!(post,c.post_calls());
}

#[test]
fn electronic_band_ranges_precede_dos_failure_and_invalid_windows_keep_diagnostics() {
    let c = Case::numbered();
    c.stage_setup();
    c.numbered_change(|cfg| {
        cfg.wannier.selection_mode=qf_wannier::SelectionMode::EnergyOnly;
        cfg.wannier.exclude_bands.clear();
        cfg.wannier.num_wann=5;
        cfg.wannier.outer_window_ev=Some([-1.,6.]);
        cfg.wannier.frozen_window_ev=Some([-1.,4.]);
        cfg.wannier.frozen_max_ev=None;
        cfg.wannier.energy_reference=config::EnergyReference::QeZero;
    });
    c.script("dos.x","#!/bin/sh\nexit 17\n");
    success(c.stage_submit("electronic",false));
    assert!(!c.stage_execute().status.success());
    let root=c.root.join("run");
    for path in ["1.qe-scf/plots/scf-band-ranges.json","2.qe-nscf/plots/nscf-band-ranges.json","3.qe-electronic/qe_eband/plots/electronic-band-ranges.json"] {
        let r:Value=store::read(&root.join(path)).unwrap();
        assert_eq!(r["sample_capacity_ok"],false,"{path}");
        assert!(root.join(path).with_extension("png").is_file());
    }
    assert!(!root.join("3.qe-electronic/qe_eledos/dos.exitcode").is_file()
        || fs::read_to_string(root.join("3.qe-electronic/qe_eledos/dos.exitcode")).unwrap().trim()!="0");
    let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
    let states=epwflow::qe_data::nscf::parse_xml(&root.join(format!("2.qe-nscf/out/{}.save/data-file-schema.xml",cfg.qe.prefix))).unwrap();
    let error=epwflow::interpolation::epw_config::EpwConfig::from_config(&cfg).unwrap().with_fermi(states.fermi).unwrap().with_bands(&states.eigenvalues).unwrap().window_selection().unwrap_err();
    assert!(error.to_string().contains("Do Not Autocrop"));
}

#[test]
fn wannier_and_coarse_single_allocation_freezes_config_and_automatically_plots() {
    use epwflow::foundation::{readable_state as logs, stages::Step};
    let c = Case::numbered();
    c.stage_setup();
    for (step, stage, folder) in [
        (Step::Wannier, "wannier", "5.epw-wannier"),
        (Step::Coarse, "coarse", "6.epw-coarse"),
    ] {
        let original = fs::read(c.root.join("input.toml")).unwrap();
        let count = fs::read_to_string(c.root.join("submissions")).unwrap_or_default().lines().count();
        success(c.stage_submit(stage, false));
        let doc = logs::read(&c.root.join("run"), step).unwrap();
        let script = logs::resolve(&c.root.join("run"), doc.get("Stage allocation", "Script").unwrap());
        let body = fs::read_to_string(script).unwrap();
        assert!(body.contains(&format!("--allocated-stage {stage}")));
        assert!(body.contains("epw_wannier"));
        assert!(!c.stage_submit(stage, false).status.success());
        c.put("input.toml", "invalid pending edit");
        // 真实执行一次捕获的 Slurm 脚本；不使用作业外反复推进辅助器。
        success(c.stage_execute());
        fs::write(c.root.join("input.toml"), original).unwrap();
        assert_eq!(fs::read_to_string(c.root.join("submissions")).unwrap().lines().count(), count + 1,
            "No embedded submission at the stage");
        assert_eq!(logs::read(&c.root.join("run"), step).unwrap().get("Stage allocation", "Status"), Some("completed"));
        let result = c.root.join("run/results").join(folder);
        let comparison = c.root.join("run").join(folder).join("comparison-native");
        assert!(!comparison.join("electron").exists());
        assert!(!comparison.join("phonon").exists());
        if step == Step::Wannier {
            assert!(comparison.join("postw90.out").is_file());
            assert!(comparison.join("band.eig").is_file());
        } else {
            assert!(comparison.join("force-constants/reference.fc.xml").is_file());
            assert!(comparison.join("frequencies/reference.freq").is_file());
        }
        assert!(result.join("stage-manifest.json").is_file());
        let files = epwflow::qe_data::save::files(&result).unwrap();
        assert!(files.iter().any(|f| f.relative_path.extension().is_some_and(|ext| ext == "png")));
        let before = (c.calls(), c.post_calls());
        success(c.stage_submit(stage, false));
        assert_eq!(before, (c.calls(), c.post_calls()));
        assert_eq!(fs::read_to_string(c.root.join("submissions")).unwrap().lines().count(), count + 1);
    }
}

#[test]
fn current_plot_overwrites_original_images_without_scientific_execution() {
    let c = Case::numbered();
    c.stage_setup();
    success(c.stage_submit("wannier", false));
    success(c.stage_execute());
    let comparison = c.root.join("run/5.epw-wannier/plot");
    let scientific = c.calls();
    let post = c.post_calls();
    let submissions = fs::read(c.root.join("submissions")).unwrap();
    let source = store::Identity::capture(&c.root.join("run/5.epw-wannier/comparison-native/band.eig")).unwrap();
    let model = store::Identity::capture(&c.root.join("run/5.epw-wannier/scf.chk")).unwrap();
    let input = store::Identity::capture(&comparison.join("electronic.plot.json")).unwrap();
    let result_image = store::Identity::capture(&c.root.join("run/results/5.epw-wannier/plot/electronic.png")).unwrap();
    for name in ["electronic", "electronic-from-lowest-to-highest"] {
        fs::remove_file(comparison.join(format!("{name}.png"))).unwrap();
    }
    let out = c.numbered_cli(1, &["--plot"], None, "current-replot");
    assert!(String::from_utf8_lossy(&out.stdout).contains("plots:"));
    success(out);
    for name in ["electronic", "electronic-from-lowest-to-highest"] {
        assert!(comparison.join(format!("{name}.png")).is_file());
    }
    assert!(c.root.join("run/5.epw-wannier/plot/electronic.png").is_file());
    assert_eq!(scientific, c.calls());
    assert_eq!(post, c.post_calls());
    assert_eq!(submissions, fs::read(c.root.join("submissions")).unwrap());
    for id in [source, model, input, result_image] { id.verify().unwrap(); }
}

#[test]
fn completed_matdyn_keeps_215_native_points_when_current_qe_plan_is_935() {
    use epwflow::foundation::{readable_state as log,stages::Step};
    let c=Case::numbered();c.stage_setup();
    c.put("KPATH.in",include_str!("../../../examples/MgB2/KPATH.in"));
    c.numbered_change(|cfg| {
        cfg.qe.scf.k_grid=[4,4,2];cfg.qe.nscf.k_grid=[4,4,2];
        cfg.comparison.as_mut().unwrap().points_per_edge=20;
        cfg.set_native(&[&cfg.phonon_stage(),"workflow","points_per_edge"],json!(100)).unwrap();
        cfg.set_native(&[epwflow::foundation::native_config::WANNIER,"workflow","points_per_edge"],json!(20)).unwrap();
        cfg.set_native(&[epwflow::foundation::native_config::COARSE,"workflow","points_per_edge"],json!(20)).unwrap();
    });
    success(c.stage_submit("coarse",false));
    success(c.stage_execute());
    let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
    let comparison=c.root.join("run/6.epw-coarse/comparison");
    let xml=cfg.work_dir.join("2.qe-nscf/out").join(format!("{}.save/data-file-schema.xml",cfg.qe.prefix));
    let modes=epwflow::qe_data::xml::Geometry::read(&xml).unwrap().atoms.len()*3;
    let csv=fs::read_to_string(comparison.join("phonon_bands.csv")).unwrap();
    assert_eq!(csv.lines().filter(|l|l.starts_with("qe,")).count(),903*modes);
    assert_eq!(csv.lines().filter(|l|l.starts_with("epw,")).count(),183*modes);
    let directory=c.root.join("run/6.epw-coarse/comparison-native/frequencies");
    let input=fs::read_to_string(directory.join("matdyn.in")).unwrap();
    let (header,body)=input.split_once("/\n").unwrap();
    let rows=body.lines().filter(|l|!l.trim().is_empty()).skip(1).collect::<Vec<_>>();
    assert_eq!(rows.len(),935);
    let values=epwflow::interpolation::native::parse_matdyn_path(
        &fs::read_to_string(directory.join("reference.freq")).unwrap()).unwrap();
    let sampling=epwflow::interpolation::electronic_sampling::plan(&cfg).unwrap();
    let mut indices=Vec::new();let mut cursor=0;
    for point in &sampling.path {
        let cart=epwflow::interpolation::electronic_sampling::cart(point.crystal,&sampling.reciprocal);
        while cursor<903 {
            let row=rows[cursor].split_whitespace().map(|x|x.parse::<f64>().unwrap()).collect::<Vec<_>>();
            if cart.iter().zip(&row).all(|(a,b)|(a-b).abs()<1e-9) { break; }
            cursor+=1;
        }
        assert!(cursor<903);indices.push(cursor);cursor+=1;
    }
    indices.extend(903..935);assert_eq!(indices.len(),215);
    let mut old_input=format!("{header}/\n215\n");
    let mut old_output=format!(" &plot nbnd= {modes}, nks= 215 /\n");
    for &i in &indices {
        old_input.push_str(rows[i]);old_input.push('\n');
        let p=values.points_cartesian[i];
        old_output.push_str(&format!(" {:.17} {:.17} {:.17} 0\n",p[0],p[1],p[2]));
        old_output.push_str(&values.values[i].iter().map(|v|format!("{v:.17}")).collect::<Vec<_>>().join(" "));
        old_output.push('\n');
    }
    fs::write(directory.join("matdyn.in"),old_input).unwrap();
    fs::write(directory.join("reference.freq"),old_output).unwrap();
    // 模拟旧程序已登记成功、但新程序按不同密度重新归集的实际场景。
    fs::remove_file(c.root.join("run/6.epw-coarse/comparison-native/sources.json")).unwrap();
    fs::remove_dir_all(&comparison).unwrap();
    log::update(&cfg.work_dir,Step::Coarse,|d| {d.remove("Derived delivery");Ok(())}).unwrap();
    let before=c.calls();let post=c.post_calls();
    let frequency=store::hash_file(&directory.join("reference.freq")).unwrap();
    let request=store::hash_file(&directory.join("matdyn.in")).unwrap();
    success(c.stage_submit("coarse",false));
    success(c.stage_execute());
    assert_eq!(before,c.calls(),"已完成的主科学数据和比较 EPW 不得重跑");
    assert_eq!(post,c.post_calls(),"不得为新推导点数补跑 matdyn 或其他后处理");
    assert_eq!(frequency,store::hash_file(&directory.join("reference.freq")).unwrap());
    assert_eq!(request,store::hash_file(&directory.join("matdyn.in")).unwrap());
    let csv=fs::read_to_string(comparison.join("phonon_bands.csv")).unwrap();
    assert_eq!(csv.lines().filter(|l|l.starts_with("qe,")).count(),183*modes);
    assert_eq!(csv.lines().filter(|l|l.starts_with("epw,")).count(),183*modes);
    // 下游只准备，不执行新科学作业；不能再被旧缓存点数阻断。
    success(c.numbered_cli(2,&["--stage","fine"],None,"fine-after-own-sampling"));
    assert_eq!(before,c.calls());
    // 修改 EPW 自己的出图密度，只更新 EPW 重读；QE 旧点和粗 EPC 模型保持。
    let coarse_model=store::hash_file(&c.root.join("run/6.epw-coarse/out/scf.epmatwp")).unwrap();
    let coarse_log=store::hash_file(&c.root.join("run/6.epw-coarse/epw_coarse.out")).unwrap();
    let mut changed=config::Config::load(&c.root.join("input.toml")).unwrap();
    changed.set_native(&[epwflow::foundation::native_config::COARSE,"workflow","points_per_edge"],json!(40)).unwrap();
    fs::write(c.root.join("input.toml"),toml::to_string_pretty(&changed).unwrap()).unwrap();
    success(c.stage_submit("coarse",false));success(c.stage_execute());
    assert_eq!(before.0,c.calls().0);
    assert_eq!(post,c.post_calls());
    assert_eq!(before.1.lines().filter(|l|*l=="coarse" || *l=="wannier").count(),
        c.calls().1.lines().filter(|l|*l=="coarse" || *l=="wannier").count());
    assert_eq!(coarse_model,store::hash_file(&c.root.join("run/6.epw-coarse/out/scf.epmatwp")).unwrap());
    assert_eq!(coarse_log,store::hash_file(&c.root.join("run/6.epw-coarse/epw_coarse.out")).unwrap());
    assert_eq!(frequency,store::hash_file(&directory.join("reference.freq")).unwrap());
    let csv=fs::read_to_string(comparison.join("phonon_bands.csv")).unwrap();
    assert_eq!(csv.lines().filter(|l|l.starts_with("qe,")).count(),183*modes);
    assert_eq!(csv.lines().filter(|l|l.starts_with("epw,")).count(),363*modes);
}

#[test]
fn legacy_electronic_native_points_are_not_replaced_by_current_density() {
    use epwflow::foundation::{readable_state as log,stages::Step};
    let c=Case::numbered();c.stage_setup();
    success(c.stage_submit("wannier",false));success(c.stage_execute());
    let cfg=config::Config::load(&c.root.join("input.toml")).unwrap();
    let sampling=epwflow::interpolation::electronic_sampling::plan(&cfg).unwrap();
    let fit=c.root.join("run/5.epw-wannier");let post=fit.join("comparison-native");
    let old=c.root.join("run/6.epw-coarse/comparison-native");
    fs::create_dir_all(old.join("epw")).unwrap();
    let raw=epwflow::interpolation::native::parse_bands(&fs::read_to_string(post.join("band.eig")).unwrap()).unwrap();
    let energies=raw.values[0].iter().map(|x|x.to_string()).collect::<Vec<_>>().join(" ");
    let endpoints=[sampling.path.first().unwrap().crystal,sampling.path.last().unwrap().crystal];
    let mut band=format!(" &plot nbnd={}, nks={} /\n",raw.values[0].len(),endpoints.len()+sampling.mesh.len());
    for p in endpoints.iter().chain(&sampling.mesh) {
        let p=epwflow::interpolation::electronic_sampling::cart(*p,&sampling.reciprocal);
        band.push_str(&format!("{} {} {}\n{energies}\n",p[0],p[1],p[2]));
    }
    let native=old.join("epw/band.eig");fs::write(&native,band).unwrap();
    let source=|p:PathBuf|json!({"path":p,"sha256":store::hash_file(&p).unwrap()});
    let xml=cfg.work_dir.join("2.qe-nscf/out/scf.save/data-file-schema.xml");
    let sources=json!({"schema_version":2,"run_id":"legacy","ph_mode":"nosplit",
        "fermi_reference":source(xml.clone()),"qe_mesh_xml":source(xml),
        "qe_bands_xml":source(cfg.work_dir.join("3.qe-electronic/qe_eband/out/scf.save/data-file-schema.xml")),
        "epw_bands":source(native.clone()),"qe_phonon_bands":source(native.clone()),"epw_phonon_bands":source(native.clone()),
        "epw_log":source(fit.join("epw_wannier.out")),"provenance":[source(fit.join("scf.chk"))],
        "path":sampling.path,"ticks":sampling.ticks,"mesh":sampling.mesh,"mesh_shape":cfg.qe.nscf.k_grid,
        "reciprocal":sampling.reciprocal,"window_ev":[-2.,2.],"wannier_wout":source(fit.join("scf.wout")),
        "electronic_sigma_ev":0.1,"phonon_sigma_cm1":10.});
    store::write(&old.join("sources.json"),&sources).unwrap();
    fs::remove_file(fit.join("embedded-bands.json")).unwrap();
    let before=c.calls();let post_calls=c.post_calls();
    let model=store::hash_file(&fit.join("scf.chk")).unwrap();
    let original=store::hash_file(&native).unwrap();
    log::update(&cfg.work_dir,Step::Wannier,|d|{d.remove("Derived delivery");Ok(())}).unwrap();
    success(c.stage_submit("wannier",false));success(c.stage_execute());
    assert_eq!(before,c.calls());assert_eq!(post_calls,c.post_calls());
    assert_eq!(model,store::hash_file(&fit.join("scf.chk")).unwrap());
    assert_eq!(original,store::hash_file(&native).unwrap());
    let csv=fs::read_to_string(post.join("electronic-bands.csv")).unwrap();
    let qe=csv.lines().filter(|l|l.starts_with("qe,")).count();
    let epw=csv.lines().filter(|l|l.starts_with("epw,")).count();
    assert!(qe>epw && epw>0,"旧原生稀疏点保留，不凑成 QE 密度");
    assert_eq!(store::read::<Value>(&post.join("band-origin.json")).unwrap()["driver"],"legacy_epw_readonly");
}
