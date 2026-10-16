//! F4 行为测试：FR-001~FR-013。隔离临时目录与可执行 fixture。

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use epwflow::foundation::config;
use epwflow::foundation::stage::Backend;
use epwflow::foundation::store;
use epwflow::superconductivity::{
    aniso, attempt, gap, iso, manifest::SuperconductivityManifest, parse, scheduler, solver_config,
};

fn write_script(path: &PathBuf, body: &str) {
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn make_config(root: &PathBuf) -> PathBuf {
    let structure = root.join("POSCAR");
    std::fs::write(&structure, "Si\n1.0\n0.0 0.5 0.5\n0.5 0.0 0.5\n0.5 0.5 0.0\nSi\n2\nDirect\n0.0 0.0 0.0\n0.25 0.25 0.25\n").unwrap();
    let pw = root.join("pw.x");
    write_script(&pw, "#!/bin/bash\nexit 0\n");
    let ph = root.join("ph.x");
    write_script(&ph, "#!/bin/bash\nexit 0\n");
    let epw = root.join("epw.x");
    write_script(&epw, "#!/bin/bash\nexit 0\n");
    let wan = root.join("wannier90.x");
    write_script(&wan, "#!/bin/bash\nexit 0\n");
    let pw2 = root.join("pw2wannier90.x");
    write_script(&pw2, "#!/bin/bash\nexit 0\n");
    let work = root.join("work");
    std::fs::create_dir_all(&work).unwrap();
    let ledger = root.join("ledger.json");
    let p = root.join("config.toml");
    std::fs::write(
        &p,
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
quality = {{ k_points_crystal = [[0.0, 0.0, 0.0]], q_points_crystal = [[0.0, 0.0, 0.0]], energy_window_ev = [-2.0, 2.0], electron_max_ev = 0.01, electron_rms_ev = 0.001, phonon_max_cm1 = 10.0, phonon_rms_cm1 = 1.0 }}

[superconductivity]
mode = "iso"
mu_star = 0.1
frequency_cutoff_ev = 0.2
max_iterations = 600
convergence_relative = 1.0e-5
mixing_beta = 0.2
mixing_history = 4
initial_gaps_mev = [0.5, 1.5]
initial_shape = 1.0
gap_temperatures_k = [2.0, 5.0]
spectrum_index = 0
iso = {{ min_k = 1.0, max_k = 10.0, precision_k = 0.1, max_solves = 20, eigen_margin = 1.0e-6, normal_gap_mev = 0.1, super_gap_mev = 2.0 }}
"#,
            work = work.display(),
            structure = structure.display(),
            pw = pw.display(),
            ph = ph.display(),
            epw = epw.display(),
            wan = wan.display(),
            pw2 = pw2.display(),
            ledger = ledger.display(),
        ),
    )
    .unwrap();
    p
}

/// 构造一个会让 Tc 落在 [1,10] K 内的谱（FR-004/008 单元验证）。
fn strong_spectrum() -> parse::Spectrum {
    parse::compute_lambda(&[5.0, 10.0, 15.0, 20.0], &[0.192, 0.192, 0.192, 0.192], 200.0).unwrap()
}

#[test]
fn iso_tc_search_located() {
    let spec = strong_spectrum();
    let tc = iso::crit_temp_iso(&spec, 0.1, 200, 1.0, 10.0, 0.01, 50);
    assert!(tc.is_some(), "Tc 应在有界温度内被定位");
    let tc = tc.unwrap();
    assert!(tc > 1.0 && tc < 10.0, "Tc={tc} 应落在 (1,10) K");
}

#[test]
fn aniso_dual_initial_converged() {
    let spec = strong_spectrum();
    let root = std::env::temp_dir().join(format!("epwflow-sc-{}", store::token()));
    std::fs::create_dir_all(&root).unwrap();
    let cfg = config::Config::load(&make_config(&root)).unwrap();
    let sc = solver_config::SolverConfig::from_config(&cfg).unwrap();
    let r = aniso::solve(&spec, &sc).unwrap();
    assert!(r.located, "aniso Tc 应被定位");
    assert!(r.converged, "双初值应一致收敛");
    let tc = r.tc.unwrap();
    assert!(tc > 1.0 && tc < 10.0);
}

#[test]
fn gap_solve_finite_and_renormalized() {
    let spec = strong_spectrum();
    let omega = gap::matsubara(2.0, 200);
    let (shape, _lambda) = gap::power_iterate_gap(2.0, &spec, 0.1, 200, 0.5, 200, 1e-4);
    assert!(shape.iter().all(|x| x.is_finite()), "能隙形状应有限");
    let amax = shape.iter().map(|x| x.abs()).fold(0.0_f64, f64::max);
    assert!((amax - 1.0).abs() < 1e-6, "归一化形状最大值应为 1");
    let z = gap::renormalization_z(&shape, &omega, &spec, 0.1, 2.0);
    let min_z = z.iter().map(|x| *x).fold(f64::INFINITY, f64::min);
    assert!(min_z >= 1.0, "重整化函数应 ≥ 1");
    // BCS 能隙在 Tc 以下为正、以上为 0（FR-007）。
    assert!(gap::bcs_gap(2.0, 6.0) > 0.0);
    assert_eq!(gap::bcs_gap(8.0, 6.0), 0.0);
}

#[test]
fn single_attempt_convergence_separation() {
    let spec = strong_spectrum();
    let root = std::env::temp_dir().join(format!("epwflow-sc-{}", store::token()));
    std::fs::create_dir_all(&root).unwrap();
    let cfg = config::Config::load(&make_config(&root)).unwrap();
    let sc = solver_config::SolverConfig::from_config(&cfg).unwrap();
    let below = attempt::run_attempt(config::Mode::Iso, &spec, &sc, 2.0).unwrap();
    let above = attempt::run_attempt(config::Mode::Iso, &spec, &sc, 8.0).unwrap();
    assert!(below.converged, "低于 Tc 应收敛");
    assert!(!above.converged, "高于 Tc 应不收敛");
    assert!(below.gap_mev > 0.0);
}

#[test]
fn superconductivity_full_pipeline() {
    let root = std::env::temp_dir().join(format!("epwflow-f4-{}", store::token()));
    std::fs::create_dir_all(&root).unwrap();
    let cfg_path = make_config(&root);
    let cfg = config::Config::load(&cfg_path).unwrap();
    let work = root.join("work");

    // 直接构造合法插值清单（FR-003 数据源）。
    let f3 = work.join("interpolation");
    std::fs::create_dir_all(&f3).unwrap();
    let manifest = serde_json::json!({
        "schema_version": 1,
        "run_id": "f3",
        "f2_digest": "x",
        "wannier_digest": "y",
        "coarse": null,
        "fine": {
            "kpoints": 8, "qpoints": 8,
            "energies": [1.0, 2.0, 3.0, 4.0],
            "frequencies": [40.3, 80.6, 120.9, 161.2],
            "weights": [0.25, 0.25, 0.25, 0.25],
            "kqmap": 4, "layout": "epw", "ephmat_shards": 1, "artifacts": []
        },
        "spectra": [{
            "smearing_id": "sm0001",
            "energy_mev": [5.0, 10.0, 15.0, 20.0],
            "a2f": [0.192, 0.192, 0.192, 0.192],
            "lambda_integral": 0.8, "lambda_sum": 0.8, "cutoff_ev": 0.2
        }],
        "quality": null,
        "mode_readiness": {"iso": true},
        "validation": {
            "status": "valid",
            "checks": [{"name": "x", "passed": true, "detail": "ok"}],
            "units": {}, "thresholds": {}, "evidence": []
        }
    });
    std::fs::write(f3.join("manifest.json"), serde_json::to_string_pretty(&manifest).unwrap()).unwrap();

    let m = scheduler::run(&cfg, &work, &[], Backend::Local).unwrap();
    assert!(m.tc.is_some(), "全流程应定位 Tc");
    assert!(m.located);
    assert_eq!(m.mode, "iso");
    assert!(m.validation.all_passed(), "校验应全通过: {}", m.validation.status);

    let loaded = SuperconductivityManifest::load(&work.join("superconductivity").join("manifest.json")).unwrap();
    assert_eq!(loaded.run_id, "f4");
}
