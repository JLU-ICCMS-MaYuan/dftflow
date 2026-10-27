//! 阶段交付契约：第 5～8 步由公开命令逐项推进，科学程序为隔离替身。
use super::*;
impl Case {
    pub(super) fn stage_setup(&self) {
        self.direct_setup();
        if let Ok(attempt) = store::read::<Value>(&self.root.join("run/epw-state/active.json")) {
            if matches!(attempt["state"].as_str(), Some("succeeded" | "failed")) {
                if let Some(job) = attempt["job"].as_u64() {
                    self.put(&format!("job-{job}.state"), if attempt["state"] == "succeeded" { "COMPLETED" } else { "FAILED" });
                }
            }
        }
        self.script(
            "scontrol",
            "#!/bin/bash\nprintf 'JobId=%s JobState=RUNNING NumNodes=1\\n' \"${@: -1}\"\n",
        );
    }
    pub(super) fn stage_execute(&self) -> Output {
        if let Ok(plan) = store::read::<Value>(&self.root.join("run/native-plan.json")) {
            if matches!(plan["target"].as_str(), Some("wannier" | "coarse" | "fine" | "superconductivity")) {
                return self.direct_stage_execute(plan["target"].as_str().unwrap());
            }
        }
        use std::{io::Write, os::unix::fs::MetadataExt};
        let a: Value = store::read(&self.root.join("run/epw-state/active.json")).unwrap();
        let script = PathBuf::from(a["script"]["path"].as_str().unwrap());
        let script_body = fs::read(&script).unwrap();
        let log_path = script.with_file_name(format!("slurm-{}.log", script.file_stem().unwrap().to_str().unwrap()));
        let mut open_log = fs::File::create(&log_path).unwrap();
        open_log.write_all(b"before stage initialization\n").unwrap();
        let receipt: Value = store::read(
            &PathBuf::from(a["config"]["path"].as_str().unwrap())
                .parent()
                .unwrap()
                .join("submission.json"),
        )
        .unwrap();
        let job = receipt["job"].as_u64().unwrap();
        let mut cmd = self.scheduled_command(&script, script.parent().unwrap(), job);
        cmd.env("SLURM_NTASKS", "1");
        let out = cmd.output().unwrap();
        open_log.write_all(b"after stage initialization\n").unwrap();
        assert_eq!(fs::read(&script).unwrap(), script_body, "当前阶段脚本不能随科学目录归档后消失");
        assert_eq!(open_log.metadata().unwrap().ino(), fs::metadata(&log_path).unwrap().ino(), "日志入口必须仍指向 Slurm 持有的 inode");
        assert_eq!(fs::read_to_string(log_path).unwrap(), "before stage initialization\nafter stage initialization\n");
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
        let bin = if matches!(stage, "wannier" | "coarse") { 1 } else { 2 };
        for _ in 0..128 {
            let task = self.direct_task();
            if task["state"] == "submitted" {
                assert!(task["job"].is_u64());
                self.execute_record(task);
            }
            let out = self.numbered_cli(bin, &["--stage", stage, "-s"], None, "native-stage-advance");
            self.put("stage-worker.stdout", &String::from_utf8_lossy(&out.stdout));
            self.put("stage-worker.stderr", &String::from_utf8_lossy(&out.stderr));
            if !out.status.success() || String::from_utf8_lossy(&out.stdout).contains("workflow_complete: true") {
                return out;
            }
        }
        panic!("原生阶段 {stage} 未在有限次公开推进中结束");
    }
    pub(super) fn stage_submit(&self, stage: &str, resume: bool) -> Output {
        let mut args = vec!["--stage", stage, "-s"];
        if resume {
            args.push("--resume");
        }
        self.numbered_cli(
            if stage == "electronic" { 0 } else { 1 },
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
        "electronic-bands.png",
        "electronic-dos.png",
        "electronic-bands.csv",
        "electronic-dos.csv",
        "sampling.json",
        "metrics.json",
        "stage-manifest.json",
    ] {
        assert!(result.join(name).is_file(), "缺少 {name}");
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
    fs::remove_file(result.join("electronic-bands.png")).unwrap();
    fs::remove_file(
        c.root
            .join("run/5.epw-wannier/comparison-native/electron/electronic-dos.png"),
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
    fs::remove_file(moved.join("5.epw-wannier/electronic-bands.png")).unwrap();
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
    let model = c.root.join("run/5.epw-wannier/completion.json");
    let before = fs::read(&model).unwrap();
    let calls = c.calls();
    assert!(!c.stage_submit("wannier", false).status.success());
    fs::remove_file(c.root.join("post-failure")).unwrap();
    fs::remove_file(
        c.root
            .join("run/5.epw-wannier/comparison-native/electron/scf.werr"),
    )
    .unwrap();
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
            .join("run/5.epw-wannier/comparison-native/electron/band.eig"),
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
                .join("run/5.epw-wannier/comparison-native/electron/band.eig")
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
        .contains("5.epw-wannier/comparison-native/electron"));
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
    let figures = results.join("6.epw-coarse/figures");
    assert!(figures.join("phonon_dispersion_comparison.png").is_file());
    assert!(figures.join("phonon_dos_comparison.png").is_file());
    let calls = c.calls();
    fs::remove_file(figures.join("phonon_dispersion_comparison.png")).unwrap();
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
        "电子模型变化不能重复 PW/PH/q2r/matdyn"
    );
    let phonon = c.root.join("run/6.epw-coarse/comparison-native/phonon");
    for unit in ["force-constants", "frequencies"] {
        let reuse: Value = store::read(&phonon.join(unit).join("reuse-outcome.json")).unwrap();
        assert_eq!(reuse["decision"], "verified_copy");
        assert_eq!(reuse["executed_programs"], json!([]));
    }
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
    assert!(c.root.join("run/6.epw-coarse/completion.json").is_file());
    let force = c
        .root
        .join("run/6.epw-coarse/comparison-native/phonon/force-constants/reference.fc.xml");
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
        reason["required_programs_if_no_verified_cache"],
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
    use epwflow::foundation::stages::{self, Step};
    let c = Case::numbered();
    c.stage_setup();
    success(c.stage_submit("wannier", false));
    success(c.stage_execute());
    let cfg = config::Config::load(&c.root.join("input.toml")).unwrap();
    let sampling = epwflow::interpolation::electronic_sampling::plan(&cfg).unwrap();
    let fit = c.root.join("run/5.epw-wannier");
    let post = fit.join("comparison-native/electron");
    let old = c.root.join("run/6.epw-coarse/comparison-native");
    fs::create_dir_all(old.join("epw")).unwrap();
    let band = old.join("epw/band.eig");
    // 旧协议必须包含路径及网格；新版仅路径输出不能冒充旧采样。
    let native = epwflow::interpolation::native::parse_bands(
        &fs::read_to_string(post.join("band.eig")).unwrap()).unwrap();
    let energies = native.values[0].iter().map(|v|v.to_string()).collect::<Vec<_>>().join(" ");
    let mut legacy = format!(" &plot nbnd= {}, nks= {} /\n", native.values[0].len(),
        sampling.path.len()+sampling.mesh.len());
    for point in sampling.path.iter().map(|p|p.crystal).chain(sampling.mesh.iter().copied()) {
        let cart = epwflow::interpolation::electronic_sampling::cart(point,&sampling.reciprocal);
        legacy.push_str(&format!("{} {} {}\n{energies}\n",cart[0],cart[1],cart[2]));
    }
    fs::write(&band,legacy).unwrap();
    let source = |p: PathBuf| json!({"sha256":store::hash_file(&p).unwrap(),"path":p});
    let xml = cfg
        .work_dir
        .join("2.qe-nscf/out/scf.save/data-file-schema.xml");
    let old_sources = json!({"schema_version":2,"run_id":"legacy","ph_mode":"nosplit",
        "fermi_reference":source(xml.clone()),"qe_mesh_xml":source(xml),
        "qe_bands_xml":source(cfg.work_dir.join("3.qe-electronic/qe_eband/out/scf.save/data-file-schema.xml")),
        "epw_bands":source(band.clone()),"qe_phonon_bands":source(band.clone()),
        "epw_phonon_bands":source(band.clone()),"epw_log":source(fit.join("epw.out")),
        "provenance":[source(fit.join("scf.chk"))],"path":sampling.path,"ticks":sampling.ticks,
        "mesh":sampling.mesh,"mesh_shape":cfg.qe.nscf.k_grid,"reciprocal":sampling.reciprocal,
        "window_ev":[-1.,1.],"wannier_wout":source(fit.join("scf.wout")),
        "electronic_sigma_ev":0.1,"phonon_sigma_cm1":10.});
    store::write(&old.join("sources.json"), &old_sources).unwrap();
    // 构造旧 Completion 版本：模型已绑定，但没有新版内嵌绘图回执。
    let receipt = stages::receipt(&cfg.work_dir, Step::Wannier);
    let mut science: stages::Completion = store::read(&receipt).unwrap();
    science
        .outputs
        .retain(|id| id.path != fit.join("embedded-bands.json"));
    store::write(&receipt, &science).unwrap();
    fs::remove_file(fit.join("embedded-bands.json")).unwrap();
    fs::remove_file(fit.join("post-completion.json")).unwrap();
    let before = fs::read(&receipt).unwrap();
    let calls = c.calls();
    let dos = c.post_calls();
    let wout = store::hash_file(&fit.join("scf.wout")).unwrap();
    let old_hash = store::hash_file(&old.join("sources.json")).unwrap();
    success(c.stage_submit("wannier", false));
    success(c.stage_execute());
    assert_eq!(before, fs::read(&receipt).unwrap());
    assert_eq!(calls, c.calls());
    assert_eq!(dos, c.post_calls());
    assert_eq!(wout, store::hash_file(&fit.join("scf.wout")).unwrap());
    assert_eq!(
        old_hash,
        store::hash_file(&old.join("sources.json")).unwrap()
    );
    assert_eq!(
        store::hash_file(&band).unwrap(),
        store::hash_file(&post.join("band.eig")).unwrap()
    );
    let origin: Value = store::read(&post.join("band-origin.json")).unwrap();
    assert_eq!(origin["driver"], "legacy_epw_readonly");
}

#[test]
fn sparse_high_symmetry_path_preserves_unequal_native_sampling_and_dos() {
    let c=Case::numbered(); c.stage_setup();
    c.put("KPATH.in","0 0 0 G\n0.5 0 0 X\n0.5 0 0 X\n0.5 0.25 0 M\n0 0 0.5 A\n0.25 0 0.5 L\n");
    success(c.stage_submit("wannier",false)); success(c.stage_execute());
    let dir=c.root.join("run/5.epw-wannier");
    let input=fs::read_to_string(dir.join("epw_wannier.in")).unwrap();
    assert!(input.contains("G 0 0 0 X 0.5 0 0"));
    assert!(input.contains("X 0.5 0 0 M 0.5 0.25 0"));
    assert!(input.contains("A 0 0 0.5 L 0.25 0 0.5"));
    assert!(!input.contains("P0 ") && !input.contains("Q0 "));
    let result=c.root.join("run/results/5.epw-wannier");
    let csv=fs::read_to_string(result.join("electronic-bands.csv")).unwrap();
    let qe=csv.lines().filter(|l|l.starts_with("qe,")).count();
    let epw=csv.lines().filter(|l|l.starts_with("epw,")).count();
    assert!(qe>epw && epw>0,"两侧应保留不同数量的原生点");
    let metrics:Value=store::read(&result.join("metrics.json")).unwrap();
    assert_eq!(metrics["independent_electronic_sampling"],true);
    assert!(metrics["electronic_path"]["paired_states"].as_u64().unwrap()>0);
    assert_eq!(metrics["electronic_mesh"]["points"],0);
    assert!(!fs::read_to_string(result.join("electronic-bands.plot.json")).unwrap().contains("coverage"));
    assert!(result.join("electronic-dos.png").exists());
    assert_eq!(c.post_calls(),"postw90\n");
    let bands_hash = store::hash_file(&result.join("electronic-bands.csv")).unwrap();
    success(c.stage_submit("coarse",false));
    success(c.stage_execute());
    let comparison = c.root.join("run/6.epw-coarse/comparison");
    assert_eq!(store::hash_file(&comparison.join("electronic_bands.csv")).unwrap(), bands_hash);
    let metadata:Value = store::read(&comparison.join("metadata.json")).unwrap();
    assert_eq!(metadata["electronic_independent_sampling"], true);
    assert_eq!(c.post_calls(),"postw90\n");
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
    let electronic = c.root.join("run/3.qe-electronic/completion.json");
    let electronic_hash = store::hash_file(&electronic).unwrap();
    c.numbered_change(|cfg| cfg.wannier.localization.max_iterations = 500);
    success(c.stage_submit("wannier", true));
    success(c.stage_execute());
    assert_eq!(c.calls().0, references, "恢复拟合不得重算 SCF/NSCF/PH/电子参考");
    assert!(!references.lines().any(|line| line == "ph"));
    assert_eq!(store::hash_file(&path).unwrap(), path_hash);
    assert_eq!(store::hash_file(&electronic).unwrap(), electronic_hash);
    assert_eq!(c.calls().1.lines().filter(|line| line.starts_with("wannier ")).count(), 2);
    assert!(fs::read_to_string(c.root.join("run/5.epw-wannier/epw_wannier.in")).unwrap().contains("num_iter = 500"));
    assert!(c.root.join("run/5.epw-wannier/completion.json").is_file());
    assert!(c.root.join("run/results/5.epw-wannier/stage-manifest.json").is_file());
}
