//! F2→F3 原生格式的独立进程交接；数值只用于接口故障注入。
use epwflow::{
    foundation::{
        config::{self, EnergyReference, PhMode, Program},
        snapshot,
        stage::Backend,
        store,
    },
    interpolation::{
        self,
        native_quality::{QualitySources, SourceFile},
    },
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
    for name in ["pw", "ph", "dynmat", "q2r", "matdyn"] {
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
    cfg.interpolation.quality.energy_window_ev = [-0.5, 0.5];
    cfg.interpolation.quality.reference_manifest = if automatic { None } else { Some(root.join("reference.json")) };
    cfg.interpolation.quality.q_points_crystal =
        vec![[0., 0., 0.], [0.25, 0., 0.], [0.5, 0., 0.], [0.75, 0., 0.]];
    if compare {
        let kpath = root.join("KPATH.in");
        fs::write(&kpath,"0 0 0 G\n0.5 0 0 X\n0 0.5 0 Y\n0.5 0.5 0 M\n").unwrap();
        cfg.kpath = Some(kpath);
        cfg.comparison = Some(config::Comparison { points_per_edge: 2, electronic_sigma_ev: 0.1, phonon_sigma_cm1: 10. });
    }
    let binding = snapshot::prepare(&cfg, &root.join("input.toml")).unwrap();
    let f2 = qe_data::scheduler::run_registered(
        &binding,
        &cfg,
        &QeSettings::from_config(&cfg).unwrap(),
        Backend::Local,
        &[],
        &cfg.work_dir,
    )
    .unwrap();
    let xml = f2
        .states
        .unwrap()
        .wavefunctions
        .join("data-file-schema.xml");
    let geometry = qe_data::xml::Geometry::read(&xml).unwrap();
    cfg.interpolation.quality.k_points_crystal = qe_data::nscf::parse_xml(&xml)
        .unwrap()
        .kpoints
        .iter()
        .map(|p| geometry.crystal(*p))
        .collect();
    // 四个固定参考点来自单独文件；旧插值输出故意不存在，检验执行顺序。
    let freq = root.join("reference.freq");
    fs::write(
        &freq,
        format!(
            "&plot nbnd= 6, nks= 4 /\n{}",
            cfg.interpolation
                .quality
                .q_points_crystal
                .iter()
                .map(|p| {
                    let cart: [f64; 3] = std::array::from_fn(|j| {
                        (0..3).map(|i| p[i] * geometry.reciprocal[i][j]).sum()
                    });
                    format!(
                        "{} {} {}\n100 100 100 100 100 100\n",
                        cart[0], cart[1], cart[2]
                    )
                })
                .collect::<String>()
        ),
    )
    .unwrap();
    let file = |p: &Path| SourceFile {
        path: p.into(),
        sha256: store::hash_file(p).unwrap(),
    };
    let missing = || SourceFile {
        path: "no-old-interpolated-output".into(),
        sha256: "0".repeat(64),
    };
    let fermi = qe_data::nscf::parse_xml(&xml).unwrap().fermi;
    let sources = QualitySources {
        schema_version: 1,
        asr: "epw_simple_gamma_onsite".into(),
        window_ev: [fermi - 0.5, fermi + 0.5],
        fermi_reference: file(&xml),
        electron_reference: file(&xml),
        phonon_reference_cm1: file(&freq),
        electron_interpolated: missing(),
        phonon_interpolated_mev: missing(),
        provenance: vec![file(&cfg.structure)],
    };
    if !automatic {
    store::write(
        cfg.interpolation
            .quality
            .reference_manifest
            .as_ref()
            .unwrap(),
        &sources,
    )
    .unwrap();
    }
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
    assert!(
        interpolation::quality_binding::check(&c.cfg, work)
            .unwrap()
            .passed
    );
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
    fs::write(work.join("coarse/band.eig"), "tampered").unwrap();
    assert!(interpolation::quality_binding::check(&c.cfg, work).is_err());
}

#[test]
fn native_pipeline_failure_preserves_evidence_and_blocks_downstream() {
    for failure in ["quality", "coarse-truncated", "coarse-ukk", "fine-exit", "fine-truncated", "fine-selection"] {
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
        if failure == "quality" {
            let report: serde_json::Value =
                store::read(&c.cfg.work_dir.join("quality-report.json")).unwrap();
            assert_eq!(report["passed"], false);
        }
    }
}

#[test]
fn reference_changes_and_wrong_points_stop_before_native_calls() {
    for kind in ["hash", "point"] {
        let mut c = case("");
        if kind == "hash" {
            fs::write(c.root.join("reference.freq"), "changed").unwrap();
        } else {
            c.cfg.interpolation.quality.q_points_crystal[2][1] = 0.1;
        }
        assert!(run(&c).is_err());
        assert!(!c.root.join("epw-calls").exists());
        assert!(!c.cfg.work_dir.join("coarse").exists());
    }
}

#[test]
fn reread_quality_keeps_source_and_does_not_select_a_new_window() {
    let c = case("");
    run(&c).unwrap();
    let source = c.cfg.work_dir.join("coarse");
    let before = store::hash_file(&source.join("out/scf.epmatwp")).unwrap();
    let target = c.root.join("reread");
    let ep = interpolation::epw_config::EpwConfig::from_config(&c.cfg).unwrap()
        .with_fermi(qe_data::nscf::parse_xml(&c.cfg.work_dir.join("nscf/out/scf.save/data-file-schema.xml")).unwrap().fermi).unwrap();
    let r = interpolation::coarse::reread_quality(&ep, &source, &source.join("out"), &target, &[], Backend::Local).unwrap();
    assert_eq!(r.artifacts.len(), 12);
    assert!(interpolation::quality_binding::freeze(&c.cfg,
        qe_data::nscf::parse_xml(&c.cfg.work_dir.join("nscf/out/scf.save/data-file-schema.xml")).unwrap().fermi).unwrap().pair(&target).unwrap().passed);
    assert_eq!(store::hash_file(&source.join("out/scf.epmatwp")).unwrap(), before);
    let input = fs::read_to_string(target.join("coarse/epw_coarse.in")).unwrap();
    for keyword in ["dis_froz", "dis_proj", "proj(", "wdata("] { assert!(!input.contains(keyword)); }
    assert!(interpolation::coarse::reread_quality(&ep, &source, &source.join("out"), &target, &[], Backend::Local).is_err());
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
    fs::write(work.join("coarse/band.eig"), "tampered").unwrap();
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
    assert_eq!(dataset.manifest.quality_status,"passed");
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
fn automatic_comparison_quality_failure_only_publishes_diagnostics() {
    let c=case_options("quality",true,true);
    let work=&c.cfg.work_dir;
    let error=interpolation::scheduler::run_coarse_registered(&c.cfg,Backend::Local,&[],work,work).unwrap_err();
    assert!(format!("{error:#}").contains("质量未通过"),"{error:#}");
    assert!(!work.join("wannier/manifest.json").exists());
    assert!(!work.join("interpolation/manifest.json").exists());
    assert!(!work.join("fine").exists());
    let data=interpolation::comparison_data::load(&work.join("results/comparison")).unwrap();
    assert_eq!(data.manifest.quality_status,"failed");
    assert!(epwflow::delivery::comparison_plot::check_figures(&work.join("results")).unwrap());
    assert!(interpolation::scheduler::run_fine_registered(&c.cfg,Backend::Local,&[],work).is_err());
    assert_eq!(calls(&c).lines().count(),2);
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
fn quality_failure_reuses_native_output_and_still_blocks_fine() {
    let c = case("quality");
    assert!(run(&c).is_err());
    let log = calls(&c);
    let before = store::Identity::capture(&c.cfg.work_dir.join("quality-report.json")).unwrap();
    // 改变进程故障开关不能诱发重新求解以绕过质量门。
    fs::write(c.root.join("epw-failure"), "").unwrap();
    assert!(retry_coarse(&c).is_err());
    before.verify().unwrap();
    assert_eq!(calls(&c), log);
    assert!(!c.cfg.work_dir.join("native-attempts/history").exists());
    assert!(!c.cfg.work_dir.join("fine").exists());
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
    // 模拟已写完原生成功输出，但尚未写检查点与质量发布完成的崩溃窗口。
    fs::remove_file(c.cfg.work_dir.join("wannier/manifest.json")).unwrap();
    fs::remove_file(c.cfg.work_dir.join("interpolation/manifest.json")).unwrap();
    fs::remove_file(c.cfg.work_dir.join("quality-binding.json")).unwrap();
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
    for kind in ["input", "output", "running-input", "request", "unknown", "quality", "quality-link", "empty-receipt"] {
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
            "output" => fs::write(c.cfg.work_dir.join("coarse/band.eig"), "changed").unwrap(),
            "request" => c.cfg.wannier.outer_window_ev = Some([1.4, 5.]),
            "quality" => fs::write(c.cfg.work_dir.join("quality-report.json"), "changed").unwrap(),
            "quality-link" => {
                let path = c.cfg.work_dir.join("quality-report.json");
                fs::rename(&path, c.root.join("report.json")).unwrap();
                std::os::unix::fs::symlink(c.root.join("report.json"), path).unwrap();
            },
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
fn automatic_quality_sources_use_scf_copies_and_fixed_independent_points() {
    let c = case_with_auto("", true);
    let work = &c.cfg.work_dir;
    let before = fs::read_to_string(c.root.join("scientific-calls")).unwrap();
    let original = store::Identity::capture(&work.join("nscf/out/scf.save/data-file-schema.xml")).unwrap();
    let source = qe_data::save::files(&work.join("scf/snapshot")).unwrap();
    // 合法清单不依赖响应数组顺序，必须按物理 q 编号关联。
    let mut f2 = qe_data::manifest::QeManifest::load(&work.join("qe-data/manifest.json")).unwrap();
    f2.phonon_files.reverse();
    store::write(&work.join("qe-data/manifest.json"), &f2).unwrap();
    let coarse = interpolation::scheduler::run_coarse_registered(&c.cfg,Backend::Local,&[],work,work).unwrap();
    assert!(coarse.validation.all_passed());
    original.verify().unwrap();
    assert_eq!(qe_data::save::files(&work.join("scf/snapshot")).unwrap(),source);
    let calls = fs::read_to_string(c.root.join("scientific-calls")).unwrap();
    let added = calls.strip_prefix(&before).unwrap();
    assert_eq!(added.lines().filter(|l| *l == "pw").count(),1);
    assert_eq!(added.lines().filter(|l| *l == "ph").count(),2);
    assert_eq!(added.lines().filter(|l| l.starts_with("dynmat ")).count(),4);
    assert_eq!(calls.matches("q2r\n").count(), before.matches("q2r\n").count());
    assert!(fs::read_to_string(work.join("quality-sources/electron/reference.in")).unwrap().contains("'nscf'"));
    assert!(!work.join("fine").exists());
    let sources: QualitySources = store::read(&work.join("quality-sources/reference.json")).unwrap();
    assert!(sources.provenance.iter().any(|f| f.path.ends_with("corrected.xml")));
    assert!(sources.provenance.iter().any(|f| f.path.ends_with("scf.dyn.xml")));
    assert!(sources.provenance.iter().any(|f| f.path.ends_with("scf.dyn_q1.xml")));
    assert!(interpolation::quality_binding::check(&c.cfg,work).unwrap().passed);
    let mut resumed = c.cfg.clone();
    // 固定清单变化不能悄悄补算其他参考点。
    resumed.interpolation.quality.q_points_crystal[1][1] = 0.125;
    assert!(interpolation::scheduler::run_coarse_with_retry(&resumed,Backend::Local,&[],work,work,true).is_err());
    assert_eq!(fs::read_to_string(c.root.join("scientific-calls")).unwrap(),calls);
    interpolation::scheduler::run_fine_registered(&c.cfg,Backend::Local,&[],work).unwrap();
}

#[test]
fn failed_reference_resumes_only_failed_native_task_and_rejects_tampering() {
    for tamper in [false,true] {
        let c = case_with_auto("",true);
        let work = &c.cfg.work_dir;
        fs::write(c.root.join("reference-failure"),"dynmat-0002").unwrap();
        assert!(interpolation::scheduler::run_coarse_registered(&c.cfg,Backend::Local,&[],work,work).is_err());
        assert!(!c.root.join("epw-calls").exists());
        assert!(!work.join("quality-sources/reference.json").exists());
        let log = fs::read_to_string(c.root.join("scientific-calls")).unwrap();
        assert!(interpolation::scheduler::run_coarse_registered(&c.cfg,Backend::Local,&[],work,work).is_err());
        assert_eq!(fs::read_to_string(c.root.join("scientific-calls")).unwrap(),log);
        fs::remove_file(c.root.join("reference-failure")).unwrap();
        let complete = store::Identity::capture(&work.join("quality-sources/dynmat-0001/modes.out")).unwrap();
        if tamper { fs::write(&complete.path,"changed").unwrap(); }
        let result = retry_coarse(&c);
        if tamper {
            assert!(result.is_err());
            assert_eq!(fs::read_to_string(c.root.join("scientific-calls")).unwrap(),log);
            assert!(!c.root.join("epw-calls").exists());
        } else {
            result.unwrap(); complete.verify().unwrap();
            let calls = fs::read_to_string(c.root.join("scientific-calls")).unwrap();
            assert_eq!(calls.lines().filter(|l| *l == "pw").count(),2+1); // F2 SCF/NSCF + 独立 NSCF。
            assert_eq!(calls.matches("dynmat dynmat-0001\n").count(),1);
            assert_eq!(calls.matches("dynmat dynmat-0002\n").count(),2);
            assert_eq!(calls.matches("dynmat dynmat-0003\n").count(),1);
            let history = work.join("native-attempts/history");
            assert!(fs::read_dir(history).unwrap().any(|e| e.unwrap().path().join("reference.exitcode").is_file()));
        }
    }
}

#[test]
fn auto_reference_preflight_and_native_validation_never_bypass_quality() {
    for kind in ["program","unknown","wrong-q","duplicate-k","quality"] {
        let c = case_with_auto(if kind == "quality" { "quality" } else { "" },true);
        let work = &c.cfg.work_dir;
        let before = fs::read_to_string(c.root.join("scientific-calls")).unwrap();
        match kind {
            "program" => fs::write(c.root.join("dynmat.x"),"changed").unwrap(),
            "unknown" => {
                fs::create_dir(work.join("quality-sources")).unwrap();
                fs::write(work.join("quality-sources/keep"),"unknown").unwrap();
            },
            "wrong-q" => fs::write(c.root.join("fixture-failure"),"wrong_q").unwrap(),
            "duplicate-k" => fs::write(c.root.join("fixture-failure"),"duplicate_k").unwrap(),
            _ => {},
        }
        assert!(interpolation::scheduler::run_coarse_registered(&c.cfg,Backend::Local,&[],work,work).is_err(),"{kind}");
        assert!(!work.join("wannier/manifest.json").exists());
        assert!(!work.join("fine").exists());
        if kind != "quality" { assert!(!c.root.join("epw-calls").exists()); }
        if kind == "program" || kind == "unknown" {
            assert_eq!(fs::read_to_string(c.root.join("scientific-calls")).unwrap(),before);
        }
        if kind == "quality" {
            let scientific = fs::read_to_string(c.root.join("scientific-calls")).unwrap();
            let native = calls(&c);
            assert!(!interpolation::quality_binding::check(&c.cfg,work).unwrap().passed);
            assert!(retry_coarse(&c).is_err());
            assert_eq!(fs::read_to_string(c.root.join("scientific-calls")).unwrap(),scientific);
            assert_eq!(calls(&c),native);
        }
    }
}
