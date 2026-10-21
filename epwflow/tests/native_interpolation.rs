//! F2→F3 原生格式的独立进程交接；数值只用于接口故障注入。
use epwflow::{
    foundation::{
        config::{self, EnergyReference, PhMode, Program},
        snapshot,
        stage::Backend,
        store,
    },
    interpolation,
    qe_data::{self, qe_config::QeSettings},
};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

struct Case {
    root: PathBuf,
    cfg: config::Config,
    preserve: bool,
}
impl Drop for Case {
    fn drop(&mut self) {
        if !self.preserve && !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
fn executable(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}
fn case(failure: &str) -> Case { case_with_auto(failure, false) }
fn case_with_auto(failure: &str, automatic: bool) -> Case { case_options(failure, automatic, false) }
fn case_options(failure: &str, automatic: bool, compare: bool) -> Case {
    let retained = compare.then(|| std::env::var_os("DFTFLOW_NATIVE_COMPARISON_TEST_ROOT")).flatten();
    let preserve = retained.is_some();
    let parent = retained.map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    fs::create_dir_all(&parent).unwrap();
    let root = parent.join(format!("epwflow-native-interp-{}", store::token()));
    fs::create_dir(&root).unwrap();
    let mut cfg: config::Config = toml::from_str(include_str!("fixtures/input.toml")).unwrap();
    cfg.work_dir = root.join("run");
    cfg.outdir = root.join("out");
    cfg.budget.ledger = root.join("fixture-ledger.json");
    cfg.structure = root.join("POSCAR");
    fs::write(
        &cfg.structure,
        "Si\n1\n1 0 0\n0 1 0\n0 0 1\nSi\n2\nDirect\n0 0 0\n0.25 0.25 0.25\n",
    )
    .unwrap();
    cfg.pseudos = [("Si".into(), cfg.structure.clone())].into();
    cfg.atomic_masses.clear();
    cfg.qe.prefix = "scf".into();
    fs::write(
        root.join("qe_native_io.py"),
        include_str!("fixtures/qe_native_io.py"),
    )
    .unwrap();
    for name in ["pw", "ph", "q2r", "matdyn"] {
        let p = root.join(format!("{name}.x"));
        executable(
            &p,
            &format!(
                "#!/bin/bash\nexec /usr/bin/python3 \"{}/qe_native_io.py\" {name} \"$@\"\n",
                root.display()
            ),
        );
        let prog = Program {
            path: p,
            args: vec![],
        };
        if name == "pw" {
            cfg.programs.pw = prog
        } else if name == "ph" {
            cfg.programs.ph = prog
        };
    }
    let ep = root.join("epw.x");
    let epw_script = include_str!("fixtures/epw_native_io.py");
    let epw_script = if automatic {
        epw_script.lines().filter(|s| !s.contains("dis_proj_min dis_proj_max dis_froz_proj"))
            .collect::<Vec<_>>().join("\n") + "\n"
    } else { epw_script.to_string() };
    executable(&ep, &epw_script);
    cfg.programs.epw = Program {
        path: ep,
        args: vec!["--io-contract".into()],
    };
    cfg.programs.launcher.clear();
    cfg.programs.wannier90 = None;
    cfg.programs.pw2wannier90 = None;
    cfg.qe.scf.k_grid = [2; 3];
    cfg.qe.nscf.k_grid = [2; 3];
    cfg.phonon.q_grid = [2; 3];
    cfg.phonon.mode = PhMode::Nosplit;
    cfg.qe.nscf.nbnd = 16;
    cfg.wannier.num_wann = 5;
    cfg.wannier.energy_reference = EnergyReference::QeZero;
    cfg.wannier.exclude_bands = (1..=7).collect();
    cfg.wannier.outer_window_ev = Some([1.5, 5.]);
    cfg.wannier.frozen_window_ev = Some([2.22, 3.2]);
    cfg.wannier.frozen_max_ev = None;
    if automatic {
        cfg.wannier.selection_mode = qf_wannier::SelectionMode::EnergyOnly;
        cfg.wannier.frozen_window_ev = Some([2.2, 3.3]);
    }
    cfg.interpolation.k_grid = [4; 3];
    cfg.interpolation.q_grid = [2; 3];
    cfg.interpolation.phonon_smearings_mev = vec![0.5, 1.];
    cfg.interpolation.spectral_points = 4;
    cfg.superconductivity.spectrum_index = 0;
    if compare {
        let kpath = root.join("KPATH.in");
        fs::write(&kpath,"0 0 0 G\n0.5 0 0 X\n0 0.5 0 Y\n0.5 0.5 0 M\n").unwrap();
        cfg.kpath = Some(kpath);
        cfg.comparison = Some(config::Comparison { points_per_edge: 2, electronic_sigma_ev: 0.1, phonon_sigma_cm1: 10. });
    }
    let binding = snapshot::prepare(&cfg, &root.join("input.toml")).unwrap();
    qe_data::scheduler::run_registered(
        &binding,
        &cfg,
        &QeSettings::from_config(&cfg).unwrap(),
        Backend::Local,
        &[],
        &cfg.work_dir,
    )
    .unwrap();
    fs::write(root.join("epw-failure"), failure).unwrap();
    Case { root, cfg, preserve }
}
fn run(c: &Case) -> anyhow::Result<interpolation::manifest::InterpolationManifest> {
    interpolation::scheduler::run_registered(
        &c.cfg,
        Backend::Local,
        &[],
        &c.cfg.work_dir,
        &c.cfg.work_dir,
    )
}

#[test]
fn native_pipeline_pairs_new_coarse_and_isolates_smearings() {
    let c = case("");
    let work = &c.cfg.work_dir;
    let before = store::hash_file(&work.join("nscf/out/scf.save/data-file-schema.xml")).unwrap();
    let coarse = interpolation::scheduler::run_coarse_registered(
        &c.cfg, Backend::Local, &[], work, work,
    ).unwrap();
    assert!(coarse.validation.all_passed());
    assert!(coarse.fine.is_none() && coarse.spectra.is_empty());
    assert!(!work.join("fine").exists());
    assert!(!work.join("interpolation/manifest.json").exists());
    assert_eq!(fs::read_to_string(c.root.join("epw-calls")).unwrap().lines().count(), 1);
    assert!(epwflow::superconductivity::source::bind(work, &c.cfg, &coarse).is_err());
    let identity = store::Identity::capture(&work.join("wannier/manifest.json")).unwrap();
    let f3 = interpolation::scheduler::run_fine_registered(
        &c.cfg, Backend::Local, &[], work,
    ).unwrap();
    identity.verify().unwrap();
    assert!(f3.validation.all_passed());
    assert_eq!(f3.spectra.len(), 2);
    assert_ne!(f3.spectra[0].a2f, f3.spectra[1].a2f);
    assert!(f3
        .fine
        .as_ref()
        .unwrap()
        .artifacts
        .iter()
        .filter(|a| !a.role.starts_with("smearing_"))
        .all(|a| a.relative_path.starts_with("fine/smearing-0001")));
    let input = epwflow::superconductivity::source::bind(work, &c.cfg, &f3).unwrap();
    input.verify(config::Mode::Aniso).unwrap();
    assert_eq!(input.matrix_outdir, work.join("fine/smearing-0001/out"));
    assert!(work.join("fine/smearing-0002/scf.a2f").exists());
    assert_eq!(
        store::hash_file(&work.join("nscf/out/scf.save/data-file-schema.xml")).unwrap(),
        before
    );
    let calls = fs::read_to_string(c.root.join("epw-calls")).unwrap();
    assert_eq!(calls.lines().count(), 3);
    assert_eq!(calls.matches("coarse ").count(), 1);
    assert!(!c.cfg.budget.ledger.exists());
    // 在首个展宽有效时也不能复用未知的粗网格现场。
    assert!(run(&c).is_err());
    assert_eq!(fs::read_to_string(c.root.join("epw-calls")).unwrap(), calls);
    fs::write(work.join("coarse/epwdata.fmt"), "tampered").unwrap();
}

#[test]
fn native_pipeline_failure_preserves_evidence_and_blocks_downstream() {
    for failure in ["coarse-truncated", "coarse-ukk", "fine-exit", "fine-truncated", "fine-selection"] {
        let c = case(failure);
        let err = run(&c).unwrap_err().to_string();
        assert!(
            !c.cfg.work_dir.join("interpolation/manifest.json").exists(),
            "{failure}: {err}"
        );
        let calls = fs::read_to_string(c.root.join("epw-calls")).unwrap();
        assert_eq!(
            calls.lines().count(),
            if failure.starts_with("fine-") { 2 } else { 1 },
            "{failure}: {err}"
        );
        if failure == "fine-exit" {
            assert!(
                fs::read_to_string(c.cfg.work_dir.join("fine/smearing-0001/epw.out"))
                    .unwrap()
                    .contains("fine failed evidence")
            );
        }
    }
}

#[test]
fn energy_window_mismatch_blocks_publication_even_without_comparison_plots() {
    for (failure, expected) in [("coarse-window", "原生 dis_win_min"), ("coarse-window-missing", "缺少 Inner")] {
        let c = case_with_auto(failure, true);
        assert!(c.cfg.kpath.is_none());
        let error = run(&c).unwrap_err().to_string();
        assert!(error.contains(expected), "{failure}: {error}");
        assert!(!c.cfg.work_dir.join("wannier/manifest.json").exists());
        assert!(!c.cfg.work_dir.join("interpolation/manifest.json").exists());
        assert!(!c.cfg.work_dir.join("fine").exists());
        assert!(c.cfg.work_dir.join("coarse/scf.wout").is_file());
        assert_eq!(fs::read_to_string(c.root.join("epw-calls")).unwrap().lines().count(), 1);
    }
}

#[test]
fn fine_rechecks_actual_windows_in_hash_consistent_legacy_coarse_manifest() {
    let c = case_with_auto("", true);
    let work = &c.cfg.work_dir;
    let mut manifest = interpolation::scheduler::run_coarse_registered(
        &c.cfg, Backend::Local, &[], work, work,
    ).unwrap();
    // 模拟旧发布者的错误窗口与自洽哈希；其余原生数据保持。
    let wout = work.join("coarse/scf.wout");
    let text = fs::read_to_string(&wout).unwrap();
    let line = text.lines().find(|line| line.starts_with("| Outer:")).unwrap();
    fs::write(&wout, text.replace(line, "| Outer: 1.52 to 5.0 (eV)")).unwrap();
    let identity = store::Identity::capture(&wout).unwrap();
    manifest.wannier_digest = identity.sha256.clone();
    let artifact = manifest.coarse.as_mut().unwrap().artifacts.iter_mut()
        .find(|artifact| artifact.role == "coarse_wannier").unwrap();
    artifact.sha256 = identity.sha256;
    artifact.byte_size = fs::metadata(&wout).unwrap().len();
    store::write(&work.join("wannier/manifest.json"), &manifest).unwrap();
    interpolation::scheduler::verify_coarse(&c.cfg, work).unwrap();
    let error = interpolation::scheduler::run_fine_registered(
        &c.cfg, Backend::Local, &[], work,
    ).unwrap_err().to_string();
    assert!(error.contains("原生 dis_win_min"), "{error}");
    assert!(!work.join("fine").exists());
    assert_eq!(fs::read_to_string(c.root.join("epw-calls")).unwrap().lines().count(), 1);
}

#[test]
#[ignore = "依赖本机忽略的真实 LDA 粗网格，只读核验须显式指定"]
fn actual_lda_coarse_restart_and_wannier() {
    let root = PathBuf::from(std::env::var_os("EPWFLOW_REAL_QE_ROOT").expect("指定本机来源根目录"));
    let dir = root.join("019-lda-q6-phonon-wannier/coarse");
    let r = interpolation::wannier::check_native(&dir.join("mgb2.wout"), 5).unwrap();
    assert_eq!(r.num_iter, 14);
    assert!((r.max_spread_angstrom2 - 1.98180498).abs() < 1e-8);
    let files = interpolation::restart::files(&dir, &dir.join("tmp"), "mgb2", 5, 9).unwrap();
    let before: Vec<_> = files
        .iter()
        .map(|(role, p)| {
            (
                role.clone(),
                p.clone(),
                store::hash_file(p).unwrap(),
                fs::metadata(p).unwrap().len(),
            )
        })
        .collect();
    let target = std::env::temp_dir().join(format!("epwflow-real-restart-{}", store::token()));
    interpolation::restart::stage(&dir, &dir.join("tmp"), "mgb2", 5, 9, &target).unwrap();
    for (_, p, hash, _) in &before {
        assert_eq!(&store::hash_file(p).unwrap(), hash);
        let dest = if p.parent() == Some(dir.join("tmp").as_path()) {
            target.join("out").join(p.file_name().unwrap())
        } else {
            target.join(p.file_name().unwrap())
        };
        assert_eq!(&store::hash_file(&dest).unwrap(), hash);
    }
    if let Some(p) = std::env::var_os("EPWFLOW_REAL_CHECK_REPORT") {
        store::write(Path::new(&p),&serde_json::json!({"schema_version":1,"status":"read_only_passed","native_programs_executed":0,"wannier_iterations":r.num_iter,"max_spread_angstrom2":r.max_spread_angstrom2,"files":before,"source_hashes_unchanged":true})).unwrap();
    }
    fs::remove_dir_all(target).unwrap();
}

#[test]
fn fine_stage_requires_immutable_coarse_handoff_before_any_native_call() {
    let c = case("");
    let work = &c.cfg.work_dir;
    assert!(interpolation::scheduler::run_fine_registered(&c.cfg, Backend::Local, &[], work).is_err());
    assert!(!c.root.join("epw-calls").exists());
    interpolation::scheduler::run_coarse_registered(&c.cfg, Backend::Local, &[], work, work).unwrap();
    let calls = fs::read_to_string(c.root.join("epw-calls")).unwrap();
    fs::write(work.join("coarse/epwdata.fmt"), "tampered").unwrap();
    assert!(interpolation::scheduler::run_fine_registered(&c.cfg, Backend::Local, &[], work).is_err());
    assert_eq!(calls, fs::read_to_string(c.root.join("epw-calls")).unwrap());
    assert!(!work.join("fine").exists());
}

fn retry_coarse(c: &Case) -> anyhow::Result<interpolation::manifest::InterpolationManifest> {
    interpolation::scheduler::run_coarse_with_retry(&c.cfg, Backend::Local, &[], &c.cfg.work_dir, &c.cfg.work_dir, true)
}
fn retry_fine(c: &Case) -> anyhow::Result<interpolation::manifest::InterpolationManifest> {
    interpolation::scheduler::run_fine_with_retry(&c.cfg, Backend::Local, &[], &c.cfg.work_dir, true)
}
fn calls(c: &Case) -> String { fs::read_to_string(c.root.join("epw-calls")).unwrap() }

#[test]
fn automatic_comparison_delivers_stage_two_figures_from_native_path_and_mesh() {
    let c = case_options("", true, true);
    let work = &c.cfg.work_dir;
    let f2 = qe_data::manifest::QeManifest::load(&work.join("qe-data/manifest.json")).unwrap();
    let before: Vec<_> = f2.artifacts.iter().map(|a| a.sha256.clone()).collect();
    let m = interpolation::scheduler::run_coarse_registered(&c.cfg,Backend::Local,&[],work,work).unwrap();
    assert!(m.fine.is_none());
    assert!(!work.join("fine").exists());
    assert!(m.coarse.unwrap().artifacts.iter().any(|a| a.producer_stage == "comparison"));
    interpolation::scheduler::verify_coarse(&c.cfg,work).unwrap();
    let dataset = interpolation::comparison_data::load(&work.join("interpolation/comparison")).unwrap();
    assert_eq!(dataset.metadata.elements,["Si"]);
    assert!(dataset.manifest.quality_status.is_none());
    assert!(!dataset.manifest.missing.is_empty()); // 暂缓投影不冒充完整结果。
    let figures: serde_json::Value = store::read(&work.join("results/figures/comparison-figures.json")).unwrap();
    assert_eq!(figures["files"].as_array().unwrap().len(),15);
    let source: interpolation::comparison::Sources = store::read(&work.join("comparison-native/sources.json")).unwrap();
    assert_eq!(source.path.len(),6);
    assert_eq!(source.mesh.len(),8);
    assert!(work.join("comparison-native/phonon/reference.fc.xml").is_file());
    let matdyn_input=fs::read_to_string(work.join("comparison-native/phonon/matdyn.in")).unwrap();
    assert!(matdyn_input.contains("flfrc='reference.fc.xml'"));
    assert!(matdyn_input.contains("asr='no'") && matdyn_input.contains("q_in_band_form=.false."));
    let epw = interpolation::native::parse_bands(&fs::read_to_string(&source.epw_bands.path).unwrap()).unwrap();
    assert_eq!(epw.values.len(),14);
    assert_eq!(calls(&c).lines().filter(|l|l.starts_with("coarse ")).count(),1);
    assert_eq!(calls(&c).lines().filter(|l|l.starts_with("reread ")).count(),1);
    let qe_calls=fs::read_to_string(c.root.join("scientific-calls")).unwrap();
    assert_eq!(qe_calls.lines().filter(|l|*l=="q2r").count(),1);
    assert_eq!(qe_calls.lines().filter(|l|*l=="matdyn").count(),1);
    let path_input=fs::read_to_string(work.join("comparison-native/electron/reference.in")).unwrap();
    assert!(path_input.contains("'bands'"));
    assert!(!path_input.contains("'nscf'"));
    f2.verify().unwrap();
    assert_eq!(before,f2.artifacts.iter().map(|a|a.sha256.clone()).collect::<Vec<_>>());
    // 发布成功标记前中断；已有仅 SVG 重绘不能替代自动交付的三格式。
    fs::remove_file(work.join("wannier/manifest.json")).unwrap();
    epwflow::delivery::comparison_plot::plot(&work.join("results"),None,&["svg".into()]).unwrap();
    assert!(epwflow::delivery::comparison_plot::check_figure_formats(&work.join("results"), &["png","svg","pdf"]).unwrap());
    // 模拟开始自动交付前只有 SVG 的独立比较结果，补齐必须复用成功科学任务。
    let figure_manifest=work.join("results/figures/comparison-figures.json");
    let mut partial:serde_json::Value=store::read(&figure_manifest).unwrap();
    partial["files"].as_array_mut().unwrap().retain(|f| {
        let name=f["relative_path"].as_str().unwrap();
        if name.ends_with(".svg") {true} else {
            fs::remove_file(work.join("results/figures").join(name)).unwrap(); false
        }
    });
    store::write(&figure_manifest,&partial).unwrap();
    assert!(!epwflow::delivery::comparison_plot::check_figure_formats(&work.join("results"), &["png","svg","pdf"]).unwrap());
    interpolation::scheduler::run_coarse_with_retry(&c.cfg,Backend::Local,&[],work,work,true).unwrap();
    interpolation::scheduler::verify_coarse(&c.cfg,work).unwrap();
    assert!(epwflow::delivery::comparison_plot::check_figure_formats(&work.join("results"), &["png","svg","pdf"]).unwrap());
    // 移动后的数据包只调用绘图，不访问原生来源或创建任务。
    let moved=c.root.join("moved");
    fs::rename(work.join("results"),&moved).unwrap();
    epwflow::delivery::comparison_plot::plot(&moved,None,&["svg".into()]).unwrap();
    assert_eq!(qe_calls,fs::read_to_string(c.root.join("scientific-calls")).unwrap());
    assert_eq!(calls(&c).lines().count(),2);
}

#[test]
fn automatic_comparison_resume_only_retries_failed_native_task_and_rejects_tamper() {
    let c=case_options("",true,true);
    let work=&c.cfg.work_dir;
    fs::write(c.root.join("comparison-failure"),"matdyn").unwrap();
    assert!(interpolation::scheduler::run_coarse_registered(&c.cfg,Backend::Local,&[],work,work).is_err());
    assert!(!work.join("wannier/manifest.json").exists());
    let qe_before=fs::read_to_string(c.root.join("scientific-calls")).unwrap();
    fs::remove_file(c.root.join("comparison-failure")).unwrap();
    interpolation::scheduler::run_coarse_with_retry(&c.cfg,Backend::Local,&[],work,work,true).unwrap();
    let qe_after=fs::read_to_string(c.root.join("scientific-calls")).unwrap();
    assert_eq!(qe_before.lines().filter(|l|*l=="pw"||l.starts_with("ph")).collect::<Vec<_>>(),
        qe_after.lines().filter(|l|*l=="pw"||l.starts_with("ph")).collect::<Vec<_>>());
    assert_eq!(calls(&c).lines().count(),2);
    assert!(work.join("native-attempts/history").read_dir().unwrap().any(|e|
        e.unwrap().file_name().to_string_lossy().starts_with("comparison-phonon-")));
    fs::write(work.join("comparison-native/electron/reference.out"),"tampered").unwrap();
    assert!(interpolation::scheduler::run_coarse_with_retry(&c.cfg,Backend::Local,&[],work,work,true).is_err());
    assert_eq!(qe_after,fs::read_to_string(c.root.join("scientific-calls")).unwrap());
}

#[test]
fn automatic_comparison_large_differences_allow_fine_without_quality_decisions() {
    let c=case_options("large-difference",true,true);
    let work=&c.cfg.work_dir;
    let coarse=interpolation::scheduler::run_coarse_registered(&c.cfg,Backend::Local,&[],work,work).unwrap();
    assert!(coarse.validation.all_passed());
    assert!(work.join("wannier/manifest.json").exists());
    assert!(!work.join("interpolation/manifest.json").exists());
    assert!(!work.join("fine").exists());
    let data=interpolation::comparison_data::load(&work.join("results/comparison")).unwrap();
    assert!(data.manifest.quality_status.is_none());
    assert!(data.differences.as_ref().unwrap().phonon_path.maximum.as_ref().unwrap().absolute_difference > 50.);
    assert!(epwflow::delivery::comparison_plot::check_figures(&work.join("results")).unwrap());
    let fine=interpolation::scheduler::run_fine_registered(&c.cfg,Backend::Local,&[],work).unwrap();
    assert!(fine.validation.all_passed());
    assert!(!fine.spectra.is_empty());
    assert_eq!(calls(&c).lines().count(),4);
}

#[test]
fn failed_coarse_requires_explicit_retry_and_archives_evidence() {
    let c = case("coarse-truncated");
    assert!(run(&c).is_err());
    let log = calls(&c);
    let qe_calls = fs::read_to_string(c.root.join("scientific-calls")).unwrap();
    let source = store::Identity::capture(&c.cfg.work_dir.join("nscf/out/scf.save/data-file-schema.xml")).unwrap();
    assert!(run(&c).is_err());
    assert_eq!(calls(&c), log);
    fs::write(c.root.join("epw-failure"), "").unwrap();
    retry_coarse(&c).unwrap();
    source.verify().unwrap();
    assert_eq!(fs::read_to_string(c.root.join("scientific-calls")).unwrap(), qe_calls);
    assert_eq!(calls(&c).lines().count(), 2);
    let history = c.cfg.work_dir.join("native-attempts/history");
    let archives: Vec<_> = fs::read_dir(history).unwrap().map(|e| e.unwrap().path()).filter(|p| p.is_dir()).collect();
    assert_eq!(archives.len(), 1);
    assert_eq!(fs::read(archives[0].join("out/scf.epmatwp")).unwrap(), b"broken");
    assert!(!c.cfg.work_dir.join("fine").exists());
    assert!(!c.cfg.budget.ledger.exists());
}

#[test]
fn second_smearing_retry_keeps_first_and_coarse_unchanged() {
    let c = case("fine-second-exit");
    assert!(run(&c).is_err());
    assert_eq!(calls(&c).lines().count(), 3);
    let coarse = store::Identity::capture(&c.cfg.work_dir.join("wannier/manifest.json")).unwrap();
    let first = store::Identity::capture(&c.cfg.work_dir.join("fine/smearing-0001/scf.a2f")).unwrap();
    let checkpoint = store::Identity::capture(&c.cfg.work_dir.join("native-attempts/fine-smearing-0001.json")).unwrap();
    fs::write(c.root.join("epw-failure"), "").unwrap();
    let m = retry_fine(&c).unwrap();
    coarse.verify().unwrap(); first.verify().unwrap(); checkpoint.verify().unwrap();
    assert_eq!(calls(&c).lines().count(), 4);
    assert_eq!(calls(&c).matches("smearing-0001").count(), 1);
    epwflow::superconductivity::source::bind(&c.cfg.work_dir, &c.cfg, &m).unwrap().verify(config::Mode::Aniso).unwrap();
    let archives: Vec<_> = fs::read_dir(c.cfg.work_dir.join("native-attempts/history")).unwrap().map(|e| e.unwrap().path()).filter(|p| p.is_dir()).collect();
    assert_eq!(archives.len(), 1);
    assert!(fs::read_to_string(archives[0].join("epw.out")).unwrap().contains("fine failed evidence"));
}

#[test]
fn completed_native_attempt_recovers_publication_interruption_without_execution() {
    let c = case("");
    run(&c).unwrap();
    let log = calls(&c);
    // 模拟已写完原生成功输出，但尚未写检查点与清单发布完成的崩溃窗口。
    fs::remove_file(c.cfg.work_dir.join("wannier/manifest.json")).unwrap();
    fs::remove_file(c.cfg.work_dir.join("interpolation/manifest.json")).unwrap();
    let path = c.cfg.work_dir.join("native-attempts/coarse.json");
    let mut checkpoint: serde_json::Value = store::read(&path).unwrap();
    checkpoint["state"] = "running".into(); checkpoint["outputs"] = serde_json::json!([]);
    store::write(&path, &checkpoint).unwrap();
    retry_coarse(&c).unwrap();
    retry_fine(&c).unwrap();
    assert_eq!(calls(&c), log);
    assert!(!c.cfg.work_dir.join("native-attempts/history").exists());
}

#[test]
fn retry_rejects_tampered_complete_or_running_inputs_and_unknown_directories() {
    for kind in ["input", "output", "running-input", "request", "unknown", "empty-receipt"] {
        let mut c = case(if kind == "running-input" { "coarse-truncated" } else { "" });
        if kind == "unknown" {
            fs::create_dir(c.cfg.work_dir.join("coarse")).unwrap();
            fs::write(c.cfg.work_dir.join("coarse/keep"), "unknown").unwrap();
            assert!(retry_coarse(&c).is_err());
            assert!(!c.root.join("epw-calls").exists());
            assert_eq!(fs::read_to_string(c.cfg.work_dir.join("coarse/keep")).unwrap(), "unknown");
            continue;
        }
        let _ = run(&c);
        let log = calls(&c);
        match kind {
            "input" | "running-input" => fs::write(c.cfg.work_dir.join("coarse/epw_coarse.in"), "changed").unwrap(),
            "output" => fs::write(c.cfg.work_dir.join("coarse/epwdata.fmt"), "changed").unwrap(),
            "request" => c.cfg.wannier.outer_window_ev = Some([1.4, 5.]),
            "empty-receipt" => {
                let path = c.cfg.work_dir.join("native-attempts/coarse.json");
                let mut receipt: serde_json::Value = store::read(&path).unwrap();
                receipt["inputs"] = serde_json::json!([]);
                store::write(&path, &receipt).unwrap();
            },
            _ => unreachable!(),
        }
        assert!(retry_coarse(&c).is_err(), "{kind}");
        assert_eq!(calls(&c), log, "{kind}");
        assert!(!c.cfg.work_dir.join("native-attempts/history").exists());
    }
}


#[test]
fn no_sampling_computations_or_files_are_created() {
    let c=case_with_auto("",true);
    let before=fs::read_to_string(c.root.join("scientific-calls")).unwrap();
    let manifest=run(&c).unwrap();
    assert!(manifest.validation.all_passed());
    assert_eq!(before,fs::read_to_string(c.root.join("scientific-calls")).unwrap());
    for name in ["quality-sources","quality-reference.json","quality-binding.json","quality-report.json"] {
        assert!(!c.cfg.work_dir.join(name).exists(),"{name}");
    }
    for name in ["kpoints.dat","qpoints.dat","band.eig","phband.freq"] {
        assert!(!c.cfg.work_dir.join("coarse").join(name).exists(),"{name}");
    }
    let input=fs::read_to_string(c.cfg.work_dir.join("coarse/epw_coarse.in")).unwrap();
    assert!(input.contains("band_plot = .false."));
    assert!(!input.contains("filkf"));
}
