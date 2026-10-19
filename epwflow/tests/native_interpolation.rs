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
}
impl Drop for Case {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
fn executable(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}
fn case(failure: &str) -> Case {
    let root = std::env::temp_dir().join(format!("epwflow-native-interp-{}", store::token()));
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
    for name in ["pw", "ph"] {
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
        } else {
            cfg.programs.ph = prog
        };
    }
    let ep = root.join("epw.x");
    executable(&ep, include_str!("fixtures/epw_native_io.py"));
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
    cfg.interpolation.k_grid = [4; 3];
    cfg.interpolation.q_grid = [2; 3];
    cfg.interpolation.phonon_smearings_mev = vec![0.5, 1.];
    cfg.interpolation.spectral_points = 4;
    cfg.superconductivity.spectrum_index = 0;
    cfg.interpolation.quality.energy_window_ev = [-0.5, 0.5];
    cfg.interpolation.quality.reference_manifest = Some(root.join("reference.json"));
    cfg.interpolation.quality.q_points_crystal =
        vec![[0., 0., 0.], [0.25, 0., 0.], [0.5, 0., 0.], [0.75, 0., 0.]];
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
    store::write(
        cfg.interpolation
            .quality
            .reference_manifest
            .as_ref()
            .unwrap(),
        &sources,
    )
    .unwrap();
    fs::write(root.join("epw-failure"), failure).unwrap();
    Case { root, cfg }
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
    let f3 = run(&c).unwrap();
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
