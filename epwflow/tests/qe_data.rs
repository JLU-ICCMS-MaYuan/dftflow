//! F2 qe_data 行为测试：FR-001~FR-012。
//! 使用隔离临时目录与可执行 fixture，不触碰真实队列与 docs/local 之外数据。

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Once;

use epwflow::foundation::config;
use epwflow::foundation::snapshot;
use epwflow::foundation::stage::Backend;
use epwflow::foundation::store;
use epwflow::qe_data::cli::QeDataCommand;
use epwflow::qe_data::collect_ph;
use epwflow::qe_data::compare;
use epwflow::qe_data::manifest::{Artifact, ElectronicStates, QeManifest, Validation};
use epwflow::qe_data::qe_config::QeSettings;
use epwflow::qe_data::reuse;
use epwflow::qe_data::scheduler;
use epwflow::qe_data::{nscf, qmap, scf};

static INIT: Once = Once::new();

/// 隔离状态根，避免与机器或其它测试共享 campaign 注册表（FR-010）。
fn setup_state_root() {
    INIT.call_once(|| {
        let dir =
            std::env::temp_dir().join(format!("epwflow-qe-data-state-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::env::set_var("EPWFLOW_LOCAL_STATE", &dir);
    });
}

fn write_script(path: &PathBuf, body: &str) {
    std::fs::write(path, body).expect("写 fixture 失败");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .expect("chmod fixture 失败");
}

fn make_script(name: &str, dir: &PathBuf, body: &str) -> PathBuf {
    let p = dir.join(name);
    write_script(&p, body);
    p
}

fn poscar() -> &'static str {
    "Si\n\
     1.0\n\
     0.0 0.5 0.5\n\
     0.5 0.0 0.5\n\
     0.5 0.5 0.0\n\
     Si\n\
     2\n\
     Direct\n\
     0.0 0.0 0.0\n\
     0.25 0.25 0.25\n"
}

fn base_config(
    work: &PathBuf,
    out: &PathBuf,
    structure: &PathBuf,
    pw: &PathBuf,
    ph: &PathBuf,
    epw: &PathBuf,
    mode: &str,
    ledger: &PathBuf,
) -> String {
    format!(
        r#"schema_version = 1
work_dir = "{work}"
outdir = "{out}"
structure = "{structure}"

[programs]
launcher = []

[programs.pw]
path = "{pw}"
args = []

[programs.ph]
path = "{ph}"
args = []

[programs.epw]
path = "{epw}"
args = []

[pseudos]
Si = "{structure}"

[resources]
partition = "local"
nodes = 1
mpi = 1
wall_seconds = 60
storage_bytes = 1048576

[budget]
campaign_id = "epic-0007"
ledger = "{ledger}"

[qe]
prefix = "actual_prefix"
ecutwfc_ry = 30.0
ecutrho_ry = 240.0
occupations = "fixed"

[qe.scf]
k_grid = [2, 2, 2]
k_shift = [0, 0, 0]
conv_thr_ry = 1.0e-8

[qe.nscf]
k_grid = [2, 2, 2]
nbnd = 4
conv_thr_ry = 1.0e-8

[phonon]
mode = "{mode}"
q_grid = [2, 2, 2]
tr2_ph = 1.0e-12

[wannier]
num_wann = 2
projections = ["Si:sp3"]
exclude_bands = []
energy_reference = "qe_zero"
localization = {{ max_iterations = 100, tolerance_angstrom2 = 1.0e-6, window_iterations = 10 }}

[interpolation]
k_grid = [4, 4, 4]
q_grid = [2, 2, 2]
electron_smearing = "gaussian"
degaussw_ev = 0.01
phonon_smearings_mev = [1.0]
spectral_points = 4
fsthick_ev = 1.0
eps_acoustic_cm1 = 0.1
fermi_source = "f2"
quality = {{ k_points_crystal = [[0.0, 0.0, 0.0]], q_points_crystal = [[0.0, 0.0, 0.0]], energy_window_ev = [-2.0, 2.0], electron_max_ev = 0.01, electron_rms_ev = 0.001, phonon_max_cm1 = 10.0, phonon_rms_cm1 = 1.0 }}

[superconductivity]
mode = "iso"
mu_star = 0.1
frequency_cutoff_ev = 0.2
max_iterations = 50
convergence_relative = 1.0e-4
mixing_beta = 0.7
mixing_history = 4
initial_gaps_mev = [0.5, 1.5]
initial_shape = 1.0
gap_temperatures_k = [2.0, 5.0]
spectrum_index = 0
iso = {{ min_k = 1.0, max_k = 10.0, precision_k = 0.5, max_solves = 20, eigen_margin = 1.0e-6, normal_gap_mev = 0.1, super_gap_mev = 2.0 }}
"#,
        work = work.display(),
        out = out.display(),
        structure = structure.display(),
        pw = pw.display(),
        ph = ph.display(),
        epw = epw.display(),
        mode = mode,
        ledger = ledger.display(),
    )
}

/// 构造一个可运行的 F2 Case：临时目录、fake pw/ph/epw、POSCAR、配置。
struct Case {
    root: PathBuf,
    config_path: PathBuf,
}

fn make_case(mode: &str) -> Case {
    setup_state_root();
    let root = std::env::temp_dir().join(format!(
        "epwflow-qe-data-{}-{}-{}",
        mode,
        std::process::id(),
        store::token()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let structure = root.join("POSCAR");
    std::fs::write(&structure, poscar()).unwrap();

    std::fs::write(
        root.join("qe-native.py"),
        include_str!("fixtures/qe_native_io.py"),
    )
    .unwrap();
    let wrapper = |kind: &str| {
        format!(
            "#!/bin/sh\nexec /usr/bin/python3 '{}' {} \"$@\"\n",
            root.join("qe-native.py").display(),
            kind
        )
    };
    let pw = make_script("pw.x", &root, &wrapper("pw"));
    let ph = make_script("ph.x", &root, &wrapper("ph"));

    let _epw = make_script("epw.x", &root, "#!/bin/bash\nexit 0\n");

    let work = root.join("work");
    let out = root.join("out");
    let ledger = root.join("ledger.json");
    let config_path = root.join("config.toml");
    let cfg_text = base_config(&work, &out, &structure, &pw, &ph, &_epw, mode, &ledger);
    std::fs::write(&config_path, cfg_text).unwrap();

    Case { root, config_path }
}

fn try_pipeline(case: &Case) -> anyhow::Result<QeManifest> {
    // 序列化预算相关运行，并为每次运行隔离 state root，避免共享 campaign 注册表冲突。
    static BUDGET_SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = BUDGET_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let state_dir = std::env::temp_dir().join(format!("epwflow-qe-state-{}", store::token()));
    let _ = std::fs::create_dir_all(&state_dir);
    std::env::set_var("EPWFLOW_LOCAL_STATE", &state_dir);

    let cfg = config::Config::load(&case.config_path).expect("配置加载失败");
    let binding = snapshot::prepare(&cfg, &case.config_path).expect("prepare 失败");
    let qe = QeSettings::from_config(&cfg).expect("QeSettings 失败");
    scheduler::run(&binding, &cfg, &qe, Backend::Local, &[], &cfg.work_dir)
}

fn run_pipeline(case: &Case) -> QeManifest {
    try_pipeline(case).expect("scheduler 失败")
}

#[test]
fn native_qe_rejects_incomplete_and_mismatched_outputs() {
    for failure in [
        "energy",
        "missing_wfc",
        "duplicate_k",
        "wrong_q",
        "truncated_dvscf",
    ] {
        let case = make_case("index");
        std::fs::write(case.root.join("fixture-failure"), failure).unwrap();
        assert!(try_pipeline(&case).is_err(), "{failure} 应拒绝");
        assert!(
            !case.root.join("work/qe-data/manifest.json").exists(),
            "{failure} 不能发布成功清单"
        );
    }
}

#[test]
fn scf_snapshot_and_ph_copies_are_independent() {
    use std::os::unix::fs::MetadataExt;
    let case = make_case("coords");
    let m = run_pipeline(&case);
    let work = case.root.join("work");
    epwflow::qe_data::save::verify_snapshot(&work).unwrap();
    assert!(work.join("scf/snapshot/outside-save.wfc").exists());
    let name = "actual_prefix.save/charge-density.dat";
    let paths = [
        work.join("scf/out").join(name),
        work.join("scf/snapshot").join(name),
        work.join("nscf/out").join(name),
        work.join("ph/01/out").join(name),
        work.join("ph/02/out").join(name),
    ];
    let inodes = paths
        .iter()
        .map(|p| std::fs::metadata(p).unwrap().ino())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(inodes.len(), paths.len(), "每阶段必须独立物理复制");
    assert!(!work
        .join("scf/snapshot/actual_prefix.save/stage-local-marker")
        .exists());
    assert!((m.qpoints[1].crystal[0] - 0.5).abs() < 1e-9);
    assert!(
        m.qpoints[1]
            .crystal
            .iter()
            .zip(m.qpoints[1].cartesian)
            .any(|(a, b)| (a - b).abs() > 1e-4),
        "非正交转换不能使用单位矩阵"
    );
    let pattern = std::fs::read_to_string(&m.phonon_files[1].patterns).unwrap();
    assert!(pattern.contains("<QPOINT_NUMBER>2</QPOINT_NUMBER>"));
    let original =
        std::fs::read_to_string(work.join("ph/02/out/_ph0/actual_prefix.phsave/patterns.1.xml"))
            .unwrap();
    assert!(original.contains("<QPOINT_NUMBER>1</QPOINT_NUMBER>"));
    m.verify().unwrap();
    let victim = &m.states.as_ref().unwrap().wavefunctions.join("wfc1.dat");
    std::fs::write(victim, "tampered").unwrap();
    assert!(m.verify().is_err());
    assert!(reuse::import_from_run(&work, &case.root.join("bad-import")).is_err());
}

// ---------- FR-001/003~010：三种 PH 模式完整 qe-data 闭环 ----------

#[test]
fn qe_data_index_mode_full_pipeline() {
    let case = make_case("index");
    let m = run_pipeline(&case);
    assert!(
        m.validation.all_passed(),
        "index 模式校验未通过: {:?}",
        m.validation.status
    );
    assert_eq!(m.qpoints.len(), 2);
    assert_eq!(m.phonon_files.len(), 2);
    assert!(m.scf.is_some());
    assert!(m.states.is_some());
    let manifest_path = case.root.join("work").join("qe-data").join("manifest.json");
    assert!(manifest_path.exists(), "清单未发布");
}

#[test]
fn qe_data_coords_mode_full_pipeline() {
    let case = make_case("coords");
    let m = run_pipeline(&case);
    assert!(
        m.validation.all_passed(),
        "coords 模式校验未通过: {:?}",
        m.validation.status
    );
    assert_eq!(m.qpoints.len(), 2);
    assert_eq!(m.phonon_files.len(), 2);
}

#[test]
fn qe_data_nosplit_mode_full_pipeline() {
    let case = make_case("nosplit");
    let m = run_pipeline(&case);
    assert!(
        m.validation.all_passed(),
        "nosplit 模式校验未通过: {:?}",
        m.validation.status
    );
    assert_eq!(m.qpoints.len(), 2);
    assert_eq!(m.phonon_files.len(), 2);
}

// ---------- FR-002：输入渲染 ----------

#[test]
fn qe_config_renders_expected_namelists() {
    let case = make_case("index");
    let cfg = config::Config::load(&case.config_path).unwrap();
    let qe = QeSettings::from_config(&cfg).unwrap();
    let out = case.root.join("out");
    let scf_in = qe.scf_input(out.to_str().unwrap()).unwrap();
    assert!(scf_in.contains("&CONTROL") && scf_in.contains("calculation = 'scf'"));
    assert!(scf_in.contains("&SYSTEM") && scf_in.contains("ibrav = 0"));
    assert!(scf_in.contains("ATOMIC_POSITIONS"));
    let nscf_in = qe.nscf_input(out.to_str().unwrap()).unwrap();
    assert!(nscf_in.contains("calculation = 'nscf'") && nscf_in.contains("nbnd = 4"));
    let init_in = qe.phonon_init_input(out.to_str().unwrap());
    assert!(init_in.contains("only_init = .true.") && init_in.contains("ldisp = .true."));
    let q_in = qe
        .phonon_q_input(out.to_str().unwrap(), &config::PhMode::Index, None, 1)
        .unwrap();
    assert!(q_in.contains("start_q = 1") && q_in.contains("last_q = 1"));
}

// ---------- FR-003：SCF 能量解析 ----------

#[test]
fn scf_energy_parsing() {
    assert!(
        scf::parse_scf_energy("!    total energy              =    -100.12345678 Ry")
            .unwrap()
            .abs()
            - 100.12345678
            < 1e-9
    );
    assert!(scf::parse_scf_energy("no energy here").is_err());
}

// ---------- FR-004：NSCF XML 解析 ----------

#[test]
fn nscf_xml_parsing() {
    let xml = r#"<espresso><output><band_structure>
<nbnd>2</nbnd><nks>2</nks><fermi_energy>10.5</fermi_energy>
<ks_energies><k_point weight="0.5">0.0 0.0 0.0</k_point><eigenvalues size="2">1.0 2.0</eigenvalues></ks_energies>
<ks_energies><k_point weight="0.5">0.25 0.0 0.0</k_point><eigenvalues size="2">1.1 2.1</eigenvalues></ks_energies>
</band_structure></output></espresso>"#;
    let dir = std::env::temp_dir().join(format!("epwflow-nscf-{}", store::token()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("data-file-schema.xml");
    std::fs::write(&p, xml).unwrap();
    let st = nscf::parse_xml(&p).unwrap();
    assert_eq!(st.nbnd, 2);
    assert_eq!(st.weights.len(), 2);
    assert_eq!(st.kpoints.len(), 2);
    assert_eq!(st.kpoints[0], [0.0, 0.0, 0.0]);
    assert!((st.fermi - 10.5 * 27.211386245988).abs() < 1e-9);
    assert!(nscf::parse_xml(&dir.join("missing.xml")).is_err());
}

// ---------- FR-006：q 点映射 ----------

#[test]
fn qmap_parsing_and_periodic() {
    let dir = std::env::temp_dir().join(format!("epwflow-qmap-{}", store::token()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("ph.dyn0");
    std::fs::write(&p, "2 2 2\n2\n0 0 0\n0.5 0 0\n").unwrap();
    let geometry = epwflow::qe_data::xml::Geometry {
        alat: 1.0,
        lattice: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        reciprocal: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        atoms: vec![],
        fft: [2; 3],
    };
    let qs = qmap::parse_dyn0(&p, &geometry, [2; 3], false).unwrap();
    assert_eq!(qs.len(), 2);
    assert_eq!(qs[0].global_index, 1);
    assert_eq!(qs[1].global_index, 2);
    assert!(qmap::directory_of(7) == "07");
    assert!(qmap::directory_of(123) == "123");
    assert!(qmap::same_q_modulo_periodic(
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0]
    ));
    assert!(!qmap::same_q_modulo_periodic(
        [0.0, 0.0, 0.0],
        [0.5, 0.0, 0.0]
    ));
    assert!(qmap::parse_dyn0(&dir.join("empty.dyn0"), &geometry, [2; 3], false).is_err());
}

// ---------- FR-007：响应归集（成功与缺失） ----------

#[test]
fn collect_ph_success_and_missing() {
    let case = make_case("coords");
    let m = run_pipeline(&case);
    assert_eq!(m.qpoints[1].local_index, 1);
    let work = case.root.join("work");
    let geometry = epwflow::qe_data::xml::Geometry::read(
        &work.join("scf/snapshot/actual_prefix.save/data-file-schema.xml"),
    )
    .unwrap();
    let mut bad = m.phonon_files.clone();
    bad[0].dyn_file = case.root.join("missing.xml");
    assert!(collect_ph::collect(&work, &m.qpoints, &bad, "actual_prefix", &geometry).is_err());
}

// ---------- FR-008：清单加载与版本 ----------

#[test]
fn manifest_load_version_check() {
    let dir = std::env::temp_dir().join(format!("epwflow-manifest-{}", store::token()));
    std::fs::create_dir_all(&dir).unwrap();
    let m = QeManifest {
        schema_version: 1,
        artifact_root: None,
        artifacts: vec![],
        run_id: "r1".into(),
        fingerprints: epwflow::qe_data::manifest::Fingerprints {
            structure_sha256: "s".into(),
            pseudo_sha256: Default::default(),
            program_sha256: Default::default(),
        },
        scf: None,
        states: None,
        qpoints: vec![],
        phonon_files: vec![],
        validation: Validation {
            status: "valid".into(),
            checks: vec![],
            units: Default::default(),
            thresholds: Default::default(),
            evidence: vec![],
        },
    };
    let p = dir.join("manifest.json");
    store::write(&p, &m).unwrap();
    assert!(QeManifest::load(&p).is_ok());
}

// ---------- FR-011：数值对照标准 ----------

#[test]
fn compare_tolerances() {
    assert!(compare::compare_scalar(
        1.0,
        1.0 + 1e-8,
        compare::TOL_SCF_ENERGY_RY
    ));
    assert!(!compare::compare_scalar(
        1.0,
        1.0 + 1e-6,
        compare::TOL_SCF_ENERGY_RY
    ));
    assert!(compare::coordinate_match_periodic(
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        compare::TOL_COORD
    ));
    assert!(!compare::coordinate_match_periodic(
        [0.0, 0.0, 0.0],
        [0.5, 0.0, 0.0],
        compare::TOL_COORD
    ));
    assert!(compare::compare_weights(&[1.0, 1.0], &[2.0, 2.0]));
    assert!(!compare::compare_weights(&[1.0, 1.0], &[1.0, 2.0]));
    assert!(compare::compare_eigenvalues(
        &[vec![1.0, 2.0]],
        &[vec![1.0 + 1e-6, 2.0]],
        compare::TOL_NSCF_EIGEN_EV
    ));
    assert!(!compare::compare_eigenvalues(
        &[vec![1.0, 2.0]],
        &[vec![1.0 + 1e-3, 2.0]],
        compare::TOL_NSCF_EIGEN_EV
    ));
    assert!(compare::compare_frequencies(
        &[-1.0, 2.0],
        &[-1.0, 2.0],
        compare::TOL_FREQ_CM1
    ));
    assert!(!compare::compare_frequencies(
        &[-1.0],
        &[1.0],
        compare::TOL_FREQ_CM1
    )); // 虚频符号保留
}

// ---------- FR-009：导入复用（来源不可变、哈希重验） ----------

#[test]
fn reuse_import_from_run() {
    let case = make_case("index");
    let original = run_pipeline(&case);
    let source = case.root.join("work");
    let target = case.root.join("target");
    let report = reuse::import_from_run(&source, &target).unwrap();
    assert_eq!(report.status, "imported");
    let imported = QeManifest::load(&target.join("imported/qe-data/manifest.json")).unwrap();
    assert_eq!(original.artifacts.len(), imported.artifacts.len());
    original.verify().unwrap();
    imported.verify().unwrap();
    assert_ne!(
        original.scf.unwrap().snapshot_dir,
        imported.scf.unwrap().snapshot_dir
    );
}

// ---------- FR-001/012：qe-data check 命令（缺清单返回 no_data） ----------

#[test]
fn qe_data_check_no_data() {
    let dir = std::env::temp_dir().join(format!("epwflow-check-{}", store::token()));
    std::fs::create_dir_all(&dir).unwrap();
    // 不应因缺清单而崩溃；返回 no_data。
    let r = epwflow::qe_data::cli::run(QeDataCommand::Check {
        work: dir.clone(),
        json: true,
    });
    assert!(r.is_ok());
}

// 避免未使用告警。
#[allow(dead_code)]
fn _use_artifact(_a: Artifact) {}
#[allow(dead_code)]
fn _use_states(_s: ElectronicStates) {}

/// 显式只读检查机器上的真实历史来源；不提交作业，不修改科学文件。
#[test]
#[ignore = "需要 EPWFLOW_REAL_QE_ROOT 指向已有 LDA 证据，显式执行只读核验"]
fn actual_lda_native_layout() {
    use epwflow::qe_data::{manifest::PhResponse, native_ph, xml};
    let root = PathBuf::from(std::env::var("EPWFLOW_REAL_QE_ROOT").expect("缺少实际证据根目录"));
    let scf = root.join("014-lda-fsr-scf-nscf-phonon/phonon/tmp/mgb2.save/data-file-schema.xml");
    let nscf =
        root.join("014-lda-fsr-scf-nscf-phonon/electrons/tmp/mgb2.save/data-file-schema.xml");
    let geo = xml::Geometry::read(&scf).unwrap();
    let states = epwflow::qe_data::nscf::parse_xml(&nscf).unwrap();
    epwflow::qe_data::nscf::validate_grid(&states, [6; 3], 16, &geo).unwrap();
    let ph = root.join("019-lda-q6-phonon-wannier/phonon");
    let qs = qmap::parse_dyn0(&ph.join("mgb2.dyn0"), &geo, [6; 3], false).unwrap();
    let control = ph.join("tmp/_ph0/mgb2.phsave/control_ph.xml");
    native_ph::control(&control, &qs).unwrap();
    let mut paths = vec![scf, nscf, control.clone(), ph.join("mgb2.dyn0")];
    let mut responses = vec![];
    for q in &qs {
        let base = ph.join("tmp/_ph0");
        let dv = if q.global_index == 1 {
            base.join("mgb2.dvscf1")
        } else {
            base.join(format!("mgb2.q_{}/mgb2.dvscf1", q.global_index))
        };
        let r = PhResponse {
            q_id: q.global_index,
            dyn_file: ph.join(format!("mgb2.dyn{}.xml", q.global_index)),
            patterns: base.join(format!("mgb2.phsave/patterns.{}.xml", q.global_index)),
            dvscf: Some(dv),
            control_metadata: control.clone(),
            format: native_ph::FORMAT.into(),
            perturbations: 9,
        };
        paths.extend([
            r.dyn_file.clone(),
            r.patterns.clone(),
            r.dvscf.clone().unwrap(),
        ]);
        responses.push(r);
    }
    let before = paths
        .iter()
        .map(|p| store::hash_file(p).unwrap())
        .collect::<Vec<_>>();
    for (q, r) in qs.iter().zip(&responses) {
        native_ph::verify(r, q, &geo).unwrap();
    }
    let after = paths
        .iter()
        .map(|p| store::hash_file(p).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(before, after, "只读核验不能改变来源");
    let report = serde_json::json!({"qpoints":qs.len(),"modes_per_q":9,"nks":states.kpoints.len(),"bands":states.nbnd,"native_response_validation":true,"source_unchanged":true,"sources":paths.iter().zip(before).map(|(p,h)|serde_json::json!({"path":p,"sha256":h})).collect::<Vec<_>>()});
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    if let Ok(path) = std::env::var("EPWFLOW_REAL_CHECK_REPORT") {
        store::write(&PathBuf::from(path), &report).unwrap();
    }
}
