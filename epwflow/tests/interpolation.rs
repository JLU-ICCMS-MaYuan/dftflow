//! F3 行为测试：FR-001~FR-012。隔离临时目录与可执行 fixture。

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use epwflow::foundation::config;
use epwflow::foundation::stage::Backend;
use epwflow::foundation::store;
use epwflow::interpolation::adapter;
use epwflow::interpolation::cli::InterpolationCommand;
use epwflow::interpolation::coarse;
use epwflow::interpolation::epw_config::EpwConfig;
use epwflow::interpolation::fine;
use epwflow::interpolation::manifest::{InterpolationManifest, Validation};
use epwflow::interpolation::reuse;
use epwflow::interpolation::scheduler;
use epwflow::interpolation::spectral;
use epwflow::interpolation::wannier;
use epwflow::qe_data::manifest as qe_manifest;

fn write_script(path: &PathBuf, body: &str) {
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn poscar() -> &'static str {
    "Si\n1.0\n0.0 0.5 0.5\n0.5 0.0 0.5\n0.5 0.5 0.0\nSi\n2\nDirect\n0.0 0.0 0.0\n0.25 0.25 0.25\n"
}

fn base_config(
    work: &PathBuf,
    structure: &PathBuf,
    pw: &PathBuf,
    ph: &PathBuf,
    epw: &PathBuf,
    wan: &PathBuf,
    pw2: &PathBuf,
    ledger: &PathBuf,
) -> String {
    format!(
        r#"schema_version = 1
work_dir = "{work}"
outdir = "{work}/out"
structure = "{structure}"

[programs.pw]
path = "{pw}"
args = []

[programs.ph]
path = "{ph}"
args = []

[programs.epw]
path = "{epw}"
args = []

[programs.wannier90]
path = "{wan}"
args = []

[programs.pw2wannier90]
path = "{pw2}"
args = []

[programs]
launcher = []

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
mode = "index"
q_grid = [2, 2, 2]
tr2_ph = 1.0e-12

[wannier]
num_wann = 2
projections = ["Si:sp3"]
exclude_bands = []
energy_reference = "qe_zero"

[wannier.localization]
max_iterations = 100
tolerance_angstrom2 = 1.0e-6
window_iterations = 10

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
        structure = structure.display(),
        pw = pw.display(),
        ph = ph.display(),
        epw = epw.display(),
        wan = wan.display(),
        pw2 = pw2.display(),
        ledger = ledger.display(),
    )
}

struct Case {
    root: PathBuf,
    config_path: PathBuf,
    work: PathBuf,
}

/// 构造 F3 Case：临时目录、全部 fake 程序、POSCAR、配置，并植入有效的 F2 清单与参考频率。
fn make_case() -> Case {
    let root = std::env::temp_dir().join(format!("epwflow-interp-{}", store::token()));
    std::fs::create_dir_all(&root).unwrap();
    let structure = root.join("POSCAR");
    std::fs::write(&structure, poscar()).unwrap();
    let pw = root.join("pw.x");
    write_script(&pw, "#!/bin/bash\nexit 0\n");
    let ph = root.join("ph.x");
    write_script(&ph, "#!/bin/bash\nexit 0\n");
    let epw = root.join("epw.x");
    write_script(
        &epw,
        r#"#!/bin/bash
inp="$2"; base=$(basename "$inp"); mkdir -p out
case "$base" in
  epw_coarse.in) printf 'coarse ok\n' > epw.out; printf 'ephmat\n' > coarse.ephmat ;;
  epw_fine.in) printf 'fine ok\n' > epw.out; printf '100.0 200.0 300.0\n' > fine_frequencies.dat; printf '1.0 2.0 3.0\n' > fine_eigenvalues.dat; printf 'kq\n' > fine_kqmap.dat; printf 'shard\n' > fine_ephmat_shard.dat ;;
esac
exit 0
"#,
    );
    let wan = root.join("wannier90.x");
    write_script(
        &wan,
        r#"#!/bin/bash
seed="$1"; printf 'Convergence reached\nSpread = 1.5\n' > "$seed.wout"
exit 0
"#,
    );
    let pw2 = root.join("pw2wannier90.x");
    write_script(&pw2, "#!/bin/bash\nexit 0\n");

    let work = root.join("work");
    let ledger = root.join("ledger.json");
    let config_path = root.join("config.toml");
    std::fs::write(
        &config_path,
        base_config(&work, &structure, &pw, &ph, &epw, &wan, &pw2, &ledger),
    )
    .unwrap();

    // 植入有效的 F2 清单与参考频率（F3 仅依赖其状态，不重跑 F2）。
    let f2_dir = work.join("qe-data");
    std::fs::create_dir_all(&f2_dir).unwrap();
    let f2 = qe_manifest::QeManifest {
        schema_version: 1,
        artifact_root: None,
        artifacts: vec![],
        run_id: "f2".into(),
        fingerprints: qe_manifest::Fingerprints {
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
            checks: vec![qe_manifest::Check {
                name: "x".into(),
                passed: true,
                detail: "".into(),
            }],
            units: Default::default(),
            thresholds: Default::default(),
            evidence: vec![],
        },
    };
    store::write(&f2_dir.join("manifest.json"), &f2).unwrap();
    std::fs::write(
        f2_dir.join("reference_frequencies.dat"),
        "100.0 200.0 300.0\n",
    )
    .unwrap();

    Case {
        root,
        config_path,
        work,
    }
}

// 旧夹具只写自述 valid 与简化文件，不能作为完整科学链的正向验收。
#[test]
fn incomplete_fixture_cannot_publish_interpolation() {
    let case = make_case();
    let cfg = config::Config::load(&case.config_path).unwrap();
    let result = scheduler::run(&cfg, Backend::Local, &[], &case.work, &case.work);
    assert!(result.is_err());
    assert!(!case.work.join("interpolation/manifest.json").exists());
    assert!(
        !case.root.join("ledger.json").exists(),
        "输入不完整时不能先预留资源"
    );
}

#[test]
fn interpolation_check_no_data() {
    let dir = std::env::temp_dir().join(format!("epwflow-interp-check-{}", store::token()));
    std::fs::create_dir_all(&dir).unwrap();
    assert!(
        epwflow::interpolation::cli::run(InterpolationCommand::Check {
            work: dir.clone(),
            json: true
        })
        .is_ok()
    );
}

// ---------- FR-002：配置与能量参考 ----------

#[test]
fn epw_config_renders_and_requires_energy_reference() {
    let case = make_case();
    let cfg = config::Config::load(&case.config_path).unwrap();
    let ec = EpwConfig::from_config(&cfg)
        .unwrap()
        .with_fermi(5.0)
        .unwrap()
        .with_bands(&[vec![0., 4., 6., 10.]])
        .unwrap();
    let win = ec.wannier90_input().unwrap();
    assert!(win.contains("num_wann = 2") && !win.contains("energy_reference"));
    let coarse_in = ec.epw_coarse_input("out").unwrap();
    assert!(coarse_in.contains("epwread = .false.") && coarse_in.contains("band_plot = .false."));
    let fine_in = ec.epw_fine_input("out", 1.0).unwrap();
    assert!(fine_in.contains("epwread = .true."));
}

// ---------- FR-003：绑定 F2 ----------

#[test]
fn adapter_rejects_empty_and_invalid_f2() {
    let case = make_case();
    let cfg = config::Config::load(&case.config_path).unwrap();
    assert!(
        adapter::bind_f2(&case.work.join("qe-data").join("manifest.json"), &cfg).is_err(),
        "空 F2 不能凭自述 valid 放行"
    );

    // 篡改 F2 校验状态后必须拒绝。
    let f2_dir = case.work.join("qe-data");
    let mut f2: qe_manifest::QeManifest = store::read(&f2_dir.join("manifest.json")).unwrap();
    f2.validation.status = "invalid".into();
    f2.validation.checks[0].passed = false;
    store::write(&f2_dir.join("manifest.json"), &f2).unwrap();
    assert!(adapter::bind_f2(&f2_dir.join("manifest.json"), &cfg).is_err());
}

// ---------- FR-004：Wannier 数值成功 ----------

#[test]
fn wannier_convergence_parsing() {
    let dir = std::env::temp_dir().join(format!("epwflow-wan-{}", store::token()));
    std::fs::create_dir_all(&dir).unwrap();
    let wout = dir.join("scf.wout");
    let native = "Using band disentanglement : T\n<<< Disentanglement convergence criteria satisfied >>>\nCycle: 14\n<<< Wannierisation convergence criteria satisfied >>>\nFinal State\nWF centre and spread 1 ( 0 0 0 ) 1.234\nWF centre and spread 2 ( 0 0 0 ) 1.1\n";
    std::fs::write(&wout, native).unwrap();
    let r = wannier::parse_wannier_log(&wout).unwrap();
    assert!(r.converged);
    assert!((r.max_spread_angstrom2 - 1.234).abs() < 1e-9);
    assert_eq!(r.num_iter, 14);
    wannier::check_native(&wout, 2).unwrap();
    assert!(wannier::check_native(&wout, 3).is_err());
    for broken in [
        native.replace("<<< Disentanglement convergence criteria satisfied >>>", ""),
        native.replace("1.234", "NaN"),
        native.replace("spread 2", "spread 3"),
    ] {
        std::fs::write(&wout, broken).unwrap();
        assert!(wannier::parse_wannier_log(&wout).is_err());
    }
    // 未收敛必须拒绝。
    std::fs::write(&wout, "Spread = 1.0\n").unwrap();
    assert!(wannier::parse_wannier_log(&wout).is_err());
}

// ---------- FR-008：α²F / λ 与低频规则 ----------

#[test]
fn spectral_low_frequency_and_lambda() {
    let w = spectral::apply_low_frequency_rule(&[0.0, -0.05, 5.0], 0.1);
    assert_eq!(w, vec![0.0, -0.1, 5.0]);
    let a2f = vec![0.5, 0.5, 0.5];
    let spec =
        spectral::compute_spectrum(&[100.0, 200.0, 300.0], &a2f, "sm0001", 0.2, 0.1).unwrap();
    assert!(spec.lambda_integral.is_finite());
    assert!(spec.lambda_sum > 0.0);
}

// ---------- FR-012：清单加载与版本 ----------

#[test]
fn interpolation_manifest_load_version() {
    let dir = std::env::temp_dir().join(format!("epwflow-im-{}", store::token()));
    std::fs::create_dir_all(&dir).unwrap();
    let m = InterpolationManifest {
        schema_version: 2,
        run_id: "f3".into(),
        f2_digest: "x".into(),
        wannier_digest: "y".into(),
        coarse: None,
        fine: None,
        spectra: vec![],
        mode_readiness: Default::default(),
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
    assert!(InterpolationManifest::load(&p).is_ok());
}

// ---------- FR-010：导入复用 ----------

#[test]
fn interpolation_reuse_rejects_unvalidated_manifest() {
    let dir = std::env::temp_dir().join(format!("epwflow-ir-{}", store::token()));
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("source").join("interpolation");
    std::fs::create_dir_all(&src).unwrap();
    let coarse_file = src.join("coarse.ephmat");
    std::fs::write(&coarse_file, "ephmat").unwrap();
    let m = InterpolationManifest {
        schema_version: 2,
        run_id: "src".into(),
        f2_digest: "x".into(),
        wannier_digest: "y".into(),
        coarse: Some(epwflow::interpolation::manifest::CoarseData {
            kpoints: 1,
            qpoints: 1,
            nwann: 2,
            ephmat_role: "coarse_ephmat".into(),
            artifacts: vec![epwflow::qe_data::manifest::Artifact {
                role: "coarse_ephmat".into(),
                relative_path: coarse_file.clone(),
                byte_size: 6,
                sha256: "z".into(),
                producer_stage: "coarse".into(),
                input_digest: "".into(),
                format_version: "1".into(),
            }],
        }),
        fine: None,
        spectra: vec![],
        mode_readiness: Default::default(),
        validation: Validation {
            status: "valid".into(),
            checks: vec![],
            units: Default::default(),
            thresholds: Default::default(),
            evidence: vec![],
        },
    };
    store::write(&src.join("manifest.json"), &m).unwrap();
    let target = dir.join("target");
    std::fs::create_dir_all(&target).unwrap();
    assert!(
        reuse::import_from_run(&dir.join("source"), &target).is_err(),
        "没有校验项不能仅凭 valid 导入"
    );
}

// 避免未使用告警。
#[allow(dead_code)]
#[allow(dead_code)]
fn _use_coarse(_c: coarse::CoarseResult) {}
#[allow(dead_code)]
fn _use_fine(_f: fine::FineResult) {}
