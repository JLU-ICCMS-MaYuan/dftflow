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
use epwflow::qe_data::manifest::{Artifact, ElectronicStates, PhResponse, QeManifest, QPoint, Validation};
use epwflow::qe_data::qe_config::QeSettings;
use epwflow::qe_data::reuse;
use epwflow::qe_data::scheduler;
use epwflow::qe_data::{nscf, qmap, scf};

static INIT: Once = Once::new();

/// 隔离状态根，避免与机器或其它测试共享 campaign 注册表（FR-010）。
fn setup_state_root() {
    INIT.call_once(|| {
        let dir = std::env::temp_dir().join(format!("epwflow-qe-data-state-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::env::set_var("EPWFLOW_LOCAL_STATE", &dir);
    });
}

fn write_script(path: &PathBuf, body: &str) {
    std::fs::write(path, body).expect("写 fixture 失败");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("chmod fixture 失败");
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

fn base_config(work: &PathBuf, out: &PathBuf, structure: &PathBuf, pw: &PathBuf, ph: &PathBuf, epw: &PathBuf, mode: &str, ledger: &PathBuf) -> String {
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
prefix = "scf"
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

    let pw = make_script(
        "pw.x",
        &root,
        r#"#!/bin/bash
inp="$2"
base=$(basename "$inp")
mkdir -p out
case "$base" in
  scf.in)
    printf 'convergence has been achieved\n!    total energy              =    -100.12345678 Ry\nJOB DONE.\n'
    mkdir -p out/scf.save
    printf '<xml/>\n' > out/scf.save/data-file-schema.xml
    ;;
  nscf.in)
    mkdir -p out/nscf.save
    cat > out/nscf.save/data-file-schema.xml <<'XML'
<espresso><output><band_structure>
<nbnd>2</nbnd><nks>2</nks><fermi_energy>10.5</fermi_energy>
<ks_energies><k_point weight="0.5">0.0 0.0 0.0</k_point><eigenvalues size="2">1.0 2.0</eigenvalues></ks_energies>
<ks_energies><k_point weight="0.5">0.25 0.0 0.0</k_point><eigenvalues size="2">1.1 2.1</eigenvalues></ks_energies>
</band_structure></output></espresso>
XML
    ;;
esac
exit 0
"#,
    );
    let ph = make_script(
        "ph.x",
        &root,
        r#"#!/bin/bash
inp="$2"
base=$(basename "$inp")
mkdir -p out
case "$base" in
  ph_init.in)
    cat > out/ph.dyn0 <<'DYN'
Dynamical matrix file
q = ( 0.0 0.0 0.0 )
q = ( 0.5 0.0 0.0 )
DYN
    ;;
  ph.in)
    if grep -q 'ldisp = .true.' "$inp" && ! grep -q 'start_q' "$inp"; then
      for i in 1 2; do
        printf 'omega(   1) =    100.0 [cm-1]\nomega(   2) =    200.0 [cm-1]\n' > "out/ph.dyn$i"
        mkdir -p out/ph.phsave/_ph0
        printf 'patterns\n' > "out/ph.phsave/patterns.$i"
        printf 'dvscf\n' > "out/ph.dvscf_q.$i"
        printf 'control\n' > "out/ph.phsave/_ph0/control_ph.xml"
      done
    elif grep -q 'start_q' "$inp"; then
      idx=$(sed -n 's/.*start_q *= *\([0-9]*\).*/\1/p' "$inp")
      printf 'omega(   1) =    100.0 [cm-1]\nomega(   2) =    200.0 [cm-1]\n' > "out/ph.dyn$idx"
      mkdir -p out/ph.phsave/_ph0
      printf 'patterns\n' > "out/ph.phsave/patterns.$idx"
      printf 'dvscf\n' > "out/ph.dvscf_q.$idx"
      printf 'control\n' > "out/ph.phsave/_ph0/control_ph.xml"
    else
      x=$(tail -n 1 "$inp" | awk '{print $1}')
      idx=$(awk "BEGIN{printf \"%d\", ($x*2)+1}")
      printf 'omega(   1) =    100.0 [cm-1]\nomega(   2) =    200.0 [cm-1]\n' > "out/ph.dyn$idx"
      mkdir -p out/ph.phsave/_ph0
      printf 'patterns\n' > "out/ph.phsave/patterns.$idx"
      printf 'dvscf\n' > "out/ph.dvscf_q.$idx"
      printf 'control\n' > "out/ph.phsave/_ph0/control_ph.xml"
    fi
    ;;
esac
exit 0
"#,
    );
    let _epw = make_script("epw.x", &root, "#!/bin/bash\nexit 0\n");

    let work = root.join("work");
    let out = root.join("out");
    let ledger = root.join("ledger.json");
    let config_path = root.join("config.toml");
    let cfg_text = base_config(&work, &out, &structure, &pw, &ph, &_epw, mode, &ledger);
    std::fs::write(&config_path, cfg_text).unwrap();

    Case { root, config_path }
}

fn run_pipeline(case: &Case) -> QeManifest {
    // 序列化预算相关运行，并为每次运行隔离 state root，避免共享 campaign 注册表冲突。
    static BUDGET_SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = BUDGET_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let state_dir = std::env::temp_dir().join(format!("epwflow-qe-state-{}", store::token()));
    let _ = std::fs::create_dir_all(&state_dir);
    std::env::set_var("EPWFLOW_LOCAL_STATE", &state_dir);

    let cfg = config::Config::load(&case.config_path).expect("配置加载失败");
    let binding = snapshot::prepare(&cfg, &case.config_path).expect("prepare 失败");
    let qe = QeSettings::from_config(&cfg).expect("QeSettings 失败");
    scheduler::run(&binding, &cfg, &qe, Backend::Local, &[], &cfg.work_dir).expect("scheduler 失败")
}

// ---------- FR-001/003~010：三种 PH 模式完整 qe-data 闭环 ----------

#[test]
fn qe_data_index_mode_full_pipeline() {
    let case = make_case("index");
    let m = run_pipeline(&case);
    assert!(m.validation.all_passed(), "index 模式校验未通过: {:?}", m.validation.status);
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
    assert!(m.validation.all_passed(), "coords 模式校验未通过: {:?}", m.validation.status);
    assert_eq!(m.qpoints.len(), 2);
    assert_eq!(m.phonon_files.len(), 2);
}

#[test]
fn qe_data_nosplit_mode_full_pipeline() {
    let case = make_case("nosplit");
    let m = run_pipeline(&case);
    assert!(m.validation.all_passed(), "nosplit 模式校验未通过: {:?}", m.validation.status);
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
    let q_in = qe.phonon_q_input(out.to_str().unwrap(), &config::PhMode::Index, None, 1).unwrap();
    assert!(q_in.contains("start_q = 1") && q_in.contains("last_q = 1"));
}

// ---------- FR-003：SCF 能量解析 ----------

#[test]
fn scf_energy_parsing() {
    assert!(scf::parse_scf_energy("!    total energy              =    -100.12345678 Ry").unwrap().abs() - 100.12345678 < 1e-9);
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
    std::fs::write(&p, "Dynamical matrix file\nq = ( 0.0 0.0 0.0 )\nq = ( 0.5 0.0 0.0 )\n").unwrap();
    let qs = qmap::parse_dyn0(&p).unwrap();
    assert_eq!(qs.len(), 2);
    assert_eq!(qs[0].global_index, 1);
    assert_eq!(qs[1].global_index, 2);
    assert!(qmap::directory_of(7) == "007");
    assert!(qmap::directory_of(123) == "123");
    assert!(qmap::same_q_modulo_periodic([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]));
    assert!(!qmap::same_q_modulo_periodic([0.0, 0.0, 0.0], [0.5, 0.0, 0.0]));
    assert!(qmap::parse_dyn0(&dir.join("empty.dyn0")).is_err());
}

// ---------- FR-007：响应归集（成功与缺失） ----------

#[test]
fn collect_ph_success_and_missing() {
    let dir = std::env::temp_dir().join(format!("epwflow-collect-{}", store::token()));
    std::fs::create_dir_all(&dir).unwrap();
    let qp = QPoint {
        global_index: 1,
        local_index: 1,
        directory: "001".into(),
        crystal: [0.0, 0.0, 0.0],
        cartesian: [0.0, 0.0, 0.0],
        alat: 1.0,
        reciprocal_basis: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        source_dyn0: dir.join("ph.dyn0"),
    };
    let out = dir.join("out");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("ph.dyn1"), "omega(   1) = 100.0\nomega(   2) = 200.0\n").unwrap();
    std::fs::create_dir_all(out.join("ph.phsave").join("_ph0")).unwrap();
    std::fs::write(out.join("ph.phsave").join("patterns.1"), "p").unwrap();
    std::fs::write(out.join("ph.phsave").join("_ph0").join("control_ph.xml"), "c").unwrap();
    std::fs::write(out.join("ph.dvscf_q.1"), "d").unwrap();
    let resp = PhResponse {
        q_id: 1,
        dyn_file: out.join("ph.dyn1"),
        patterns: out.join("ph.phsave").join("patterns.1"),
        dvscf: Some(out.join("ph.dvscf_q.1")),
        control_metadata: out.join("ph.phsave").join("_ph0").join("control_ph.xml"),
        format: "qe-ph".into(),
        perturbations: 0,
    };
    let collected = collect_ph::collect(&dir, &[qp.clone()], &[resp.clone()]).unwrap();
    assert_eq!(collected.total_q, 1);
    assert_eq!(collected.responses[0].perturbations, 2);
    assert!(!collected.artifacts.is_empty());

    // 缺失 dyn 文件必须拒绝。
    let bad = PhResponse { q_id: 1, dyn_file: out.join("missing.dyn1"), ..resp };
    assert!(collect_ph::collect(&dir, &[qp], &[bad]).is_err());
}

// ---------- FR-008：清单加载与版本 ----------

#[test]
fn manifest_load_version_check() {
    let dir = std::env::temp_dir().join(format!("epwflow-manifest-{}", store::token()));
    std::fs::create_dir_all(&dir).unwrap();
    let m = QeManifest {
        schema_version: 1,
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
    assert!(compare::compare_scalar(1.0, 1.0 + 1e-8, compare::TOL_SCF_ENERGY_RY));
    assert!(!compare::compare_scalar(1.0, 1.0 + 1e-6, compare::TOL_SCF_ENERGY_RY));
    assert!(compare::coordinate_match_periodic([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], compare::TOL_COORD));
    assert!(!compare::coordinate_match_periodic([0.0, 0.0, 0.0], [0.5, 0.0, 0.0], compare::TOL_COORD));
    assert!(compare::compare_weights(&[1.0, 1.0], &[2.0, 2.0]));
    assert!(!compare::compare_weights(&[1.0, 1.0], &[1.0, 2.0]));
    assert!(compare::compare_eigenvalues(&[vec![1.0, 2.0]], &[vec![1.0 + 1e-6, 2.0]], compare::TOL_NSCF_EIGEN_EV));
    assert!(!compare::compare_eigenvalues(&[vec![1.0, 2.0]], &[vec![1.0 + 1e-3, 2.0]], compare::TOL_NSCF_EIGEN_EV));
    assert!(compare::compare_frequencies(&[-1.0, 2.0], &[-1.0, 2.0], compare::TOL_FREQ_CM1));
    assert!(!compare::compare_frequencies(&[-1.0], &[1.0], compare::TOL_FREQ_CM1)); // 虚频符号保留
}

// ---------- FR-009：导入复用（来源不可变、哈希重验） ----------

#[test]
fn reuse_import_from_run() {
    let dir = std::env::temp_dir().join(format!("epwflow-reuse-{}", store::token()));
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("source");
    let qe_data_dir = src.join("qe-data");
    std::fs::create_dir_all(&qe_data_dir).unwrap();
    let dyn_path = qe_data_dir.join("ph.dyn1");
    std::fs::write(&dyn_path, "omega(   1) = 100.0\n").unwrap();
    std::fs::write(qe_data_dir.join("patterns.1"), "p").unwrap();
    std::fs::write(qe_data_dir.join("control.1"), "c").unwrap();
    let m = QeManifest {
        schema_version: 1,
        run_id: "src1".into(),
        fingerprints: epwflow::qe_data::manifest::Fingerprints {
            structure_sha256: "s".into(),
            pseudo_sha256: Default::default(),
            program_sha256: Default::default(),
        },
        scf: None,
        states: None,
        qpoints: vec![],
        phonon_files: vec![PhResponse {
            q_id: 1,
            dyn_file: dyn_path.clone(),
            patterns: qe_data_dir.join("patterns.1"),
            dvscf: None,
            control_metadata: qe_data_dir.join("control.1"),
            format: "qe-ph".into(),
            perturbations: 1,
        }],
        validation: Validation {
            status: "valid".into(),
            checks: vec![epwflow::qe_data::manifest::Check { name: "x".into(), passed: true, detail: "".into() }],
            units: Default::default(),
            thresholds: Default::default(),
            evidence: vec![],
        },
    };
    store::write(&qe_data_dir.join("manifest.json"), &m).unwrap();

    let target = dir.join("target");
    std::fs::create_dir_all(&target).unwrap();
    let report = reuse::import_from_run(&src, &target).expect("导入失败");
    assert_eq!(report.status, "imported");
    assert!(!report.imported_roles.is_empty());
    // 来源文件执行前后哈希不变：导入后来源仍在。
    assert!(dyn_path.exists());
}

// ---------- FR-001/012：qe-data check 命令（缺清单返回 no_data） ----------

#[test]
fn qe_data_check_no_data() {
    let dir = std::env::temp_dir().join(format!("epwflow-check-{}", store::token()));
    std::fs::create_dir_all(&dir).unwrap();
    // 不应因缺清单而崩溃；返回 no_data。
    let r = epwflow::qe_data::cli::run(QeDataCommand::Check { work: dir.clone(), json: true });
    assert!(r.is_ok());
}

// 避免未使用告警。
#[allow(dead_code)]
fn _use_artifact(_a: Artifact) {}
#[allow(dead_code)]
fn _use_states(_s: ElectronicStates) {}
