//! F4 行为测试：FR-001~FR-013。隔离临时目录与可执行 fixture。

#[path = "support/cli.rs"]
mod stage_cli;

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use epwflow::foundation::config;
use epwflow::foundation::stage::Backend;
use epwflow::foundation::store;
use epwflow::superconductivity::{aniso, attempt, gap, iso, parse, scheduler, search};

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

// 以下是原生格式/进程契约 fixture，不是 MgB₂ 科学结果或真实 EPW 验收。
fn request(mode: config::Mode, kind: attempt::SolveKind) -> attempt::Request {
    attempt::Request {
        mode,
        kind,
        temperature_centikelvin: 2000,
        seed_mev: if kind == attempt::SolveKind::Nonlinear {
            Some(1.0)
        } else {
            None
        },
        mu_star: 0.05,
        cutoff_ev: 0.03,
        max_iterations: 1000,
        convergence_relative: 1e-8,
        mixing_beta: 0.2,
        mixing_history: 4,
        initial_shape: 1.0,
    }
}

fn states() -> Vec<gap::State> {
    vec![
        gap::State {
            k_crystal: Some([0.0, 0.0, 0.0]),
            kpoint: 2,
            band: 1,
            energy_ev: -0.01,
            weight: 0.25,
        },
        gap::State {
            k_crystal: Some([0.5, 0.0, 0.0]),
            kpoint: 5,
            band: 3,
            energy_ev: 0.02,
            weight: 0.75,
        },
    ]
}

fn raw_gap(r: &attempt::Request, mapping: &[gap::State], delta: f64) -> String {
    let aniso = !mapping.is_empty();
    let mut s = if aniso {
        "# w [eV] Enk-Ef [eV] znorm(w) delta(w) [eV]\n"
    } else {
        "w [eV] znorm(w) delta(w) [eV]\n"
    }
    .to_string();
    for i in 0..r.nfreq().unwrap() {
        let w = (2 * i + 1) as f64
            * std::f64::consts::PI
            * gap::KB_EV_K
            * r.temperature_centikelvin as f64
            / 100.0;
        if aniso {
            for state in mapping {
                s.push_str(&format!(
                    "{w:.10e} {:.10e} 1.5 {:.10e}\n",
                    state.energy_ev,
                    delta / 1000.0
                ));
            }
        } else {
            s.push_str(&format!("{w:.10e} 1.5 {:.10e}\n", delta / 1000.0));
        }
    }
    s
}

fn solve_log(r: &attempt::Request) -> String {
    let nf = r.nfreq().unwrap();
    let body = if r.kind == attempt::SolveKind::Linear {
        let cut = (2 * nf + 1) as f64
            * std::f64::consts::PI
            * gap::KB_EV_K
            * r.temperature_centikelvin as f64
            / 100.0;
        format!("Start: Solving (isotropic) linearized Eliashberg equation with solver = lapack\nFor the first Temp. 20.00 K\nTotal number of frequency points nsiw(1) = {nf}\nTemp. Max. nsiw wscut Nr. of iters\n(K) eigenvalue (itemp) (eV) to Converge\n20.00 1.2000000 {nf} {cut:.4} 1\nFinish: Solving (isotropic) linearized Eliashberg equation\n")
    } else {
        format!("temp(1) = 20.00000 K\nSolve {} Eliashberg equations on imaginary-axis\nTotal number of frequency points nsiw(1) = {nf}\niter ethr znormi deltai [meV]\n1 1e-2 1.5 2.0\n2 1e-10 1.5 2.0\nConvergence was reached in nsiter = 2\n",if r.mode==config::Mode::Iso {"isotropic"} else {"anisotropic"})
    };
    format!("Program EPW v.5.9 starts\n{body}Total program execution\nEPW : 0.01s CPU 0.02s WALL\n")
}

fn native_inputs(root: &PathBuf) -> attempt::NativeInputs {
    use epwflow::foundation::store::Identity;
    let a = root.join("fixture.a2f");
    let c = root.join("crystal.fmt");
    std::fs::write(&a, "# fixture\n10 0.1\n20 0.2\n").unwrap();
    std::fs::write(&c, "fixture crystal\n").unwrap();
    let mut matrix_files = vec![];
    for name in ["egnv", "freq", "ikmap", "ephmat1"] {
        let p = root.join(name);
        std::fs::write(&p, "fixture only").unwrap();
        matrix_files.push(Identity::capture(&p).unwrap());
    }
    attempt::NativeInputs {
        interpolation_digest: "fixture-f3-digest".into(),
        prefix: "mgb2".into(),
        k_grid: [4; 3],
        q_grid: [2; 3],
        spectral_points: 2,
        eps_acoustic_cm1: 0.1,
        a2f: Identity::capture(&a).unwrap(),
        crystal: Identity::capture(&c).unwrap(),
        selecq: Some(Identity::capture(&c).unwrap()),
        matrix_outdir: root.clone(),
        matrix_files,
        states: states(),
    }
}

#[test]
fn fortran_omitted_e_exponents_preserve_values_and_reject_invalid_tokens() {
    let actual = parse::numbers("1.7366215176-102 -2.5+100 +.25-100 1.0D-3 2e+1").unwrap();
    assert_eq!(actual, vec![1.7366215176e-102, -2.5e100, 0.25e-100, 1e-3, 20.0]);
    for bad in ["1-102", "1.2-10", "1.2--102", "1.2E-102-103", "1.2-102x", "1.0+999", "***", "NaN", "inf", "1.0E999"] {
        assert!(parse::numbers(bad).is_err(), "必须拒绝 {bad}");
    }
    let r = request(config::Mode::Aniso, attempt::SolveKind::Nonlinear);
    let text = raw_gap(&r, &states(), 1.7366215176e-99);
    let omitted = text.replace("1.7366215176e-102", "1.7366215176-102");
    assert_ne!(text, omitted, "必须覆盖完整能隙文件中的真实省略格式");
    let gap = gap::parse_native(&omitted, 2000, r.nfreq().unwrap(), states()).unwrap();
    assert!((gap.delta_mev[0] / 1.7366215176e-99 - 1.0).abs() < 1e-12);
    let log = solve_log(&r).replace("2 1e-10", "2 1.7366215176-102");
    assert_eq!(parse::solve_log(&log, &r).unwrap().relative_residual, Some(1.7366215176e-102));
}

#[test]
fn full_native_gap_preserves_units_mapping_and_frequencies() {
    let r = request(config::Mode::Aniso, attempt::SolveKind::Nonlinear);
    let data = gap::parse_native(
        &raw_gap(&r, &states(), 2.0),
        2000,
        r.nfreq().unwrap(),
        states(),
    )
    .unwrap();
    assert_eq!(data.delta_mev.len(), 6);
    assert_eq!(data.states[1].band, 3);
    assert!((data.delta_mev[5] - 2.0).abs() < 1e-10);
    assert_eq!(data.band_statistics().unwrap()[&3].weight_sum, 0.75);
    assert!(gap::parse_native(
        &raw_gap(&r, &states(), 2.0),
        2001,
        r.nfreq().unwrap(),
        states()
    )
    .is_err());
    let truncated = raw_gap(&r, &states(), 2.0)
        .lines()
        .take(6)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(gap::parse_native(&truncated, 2000, r.nfreq().unwrap(), states()).is_err());
    let mut wrong = states();
    wrong[0].energy_ev += 0.001;
    assert!(gap::parse_native(
        &raw_gap(&r, &states(), 2.0),
        2000,
        r.nfreq().unwrap(),
        wrong
    )
    .is_err());
    assert!(gap::parse_native(
        &raw_gap(&r, &states(), 2.0).replace("[eV]", "[meV]"),
        2000,
        r.nfreq().unwrap(),
        states()
    )
    .is_err());
}

#[test]
fn native_log_rejects_unconverged_small_gap_and_wrong_identity() {
    let r = request(config::Mode::Iso, attempt::SolveKind::Nonlinear);
    let log = solve_log(&r);
    let e = parse::solve_log(&log, &r).unwrap();
    assert_eq!(e.relative_residual, Some(1e-10));
    assert_eq!(e.iterations, Some(2));
    for bad in [
        log.replace("Convergence was reached", "Convergence was not reached"),
        log.replace("1e-10", "1e-2"),
        log.replace("1e-10", "NaN"),
        log.replace("20.00000", "20.01000"),
        log.replace("isotropic", "anisotropic"),
        log.replace("2 1e-10", "3 1e-10"),
        format!("{log}{log}"),
    ] {
        assert!(parse::solve_log(&bad, &r).is_err(), "{bad}");
    }
}

#[test]
fn native_linear_has_no_invented_residual_and_classifies_rho() {
    let r = request(config::Mode::Iso, attempt::SolveKind::Linear);
    let e = parse::solve_log(&solve_log(&r), &r).unwrap();
    assert_eq!(e.relative_residual, None);
    assert_eq!(e.maximum_eigenvalue, Some(1.2));
    assert_eq!(
        iso::classify(1.2, 1e-6).unwrap(),
        search::Side::Superconducting
    );
    assert_eq!(iso::classify(0.9, 1e-6).unwrap(), search::Side::Normal);
    assert_eq!(iso::classify(1.0, 1e-6).unwrap(), search::Side::Uncertain);
    assert!(iso::classify(f64::NAN, 1e-6).is_err());
    assert!(parse::solve_log(
        &solve_log(&r).replace("solver = lapack", "solver = power"),
        &r
    )
    .is_err());
}

#[test]
fn dual_seed_uses_full_distribution_and_only_global_sign() {
    let r = request(config::Mode::Aniso, attempt::SolveKind::Nonlinear);
    let a = gap::parse_native(&raw_gap(&r, &states(), 2.0), 2000, 3, states()).unwrap();
    let mut b = a.clone();
    b.delta_mev.iter_mut().for_each(|d| *d = -*d);
    assert_eq!(
        aniso::classify(&a, &b, 1e-4, 1e-3).unwrap(),
        search::Side::Superconducting
    );
    b.delta_mev[0] = -b.delta_mev[0];
    assert!(aniso::classify(&a, &b, 1e-4, 1e-3).is_err());
    b = a.clone();
    b.z[4] += 0.1;
    assert!(aniso::classify(&a, &b, 1e-4, 1e-3).is_err());
    let mut cancellation = a.clone();
    cancellation.delta_mev[0] = -6.0;
    assert!(cancellation.band_statistics().unwrap()[&1].signed_mean_mev < 0.0);
    assert_eq!(
        aniso::classify(&cancellation, &cancellation, 1e-4, 1e-3).unwrap(),
        search::Side::Superconducting
    );
    b.delta_mev.fill(1e-5);
    assert_eq!(
        aniso::classify(&b, &b, 1e-4, 1e-3).unwrap(),
        search::Side::Normal
    );
    b.delta_mev[5] = 0.001;
    assert_eq!(
        aniso::classify(&b, &b, 1e-4, 1e-3).unwrap(),
        search::Side::Uncertain
    );
}

#[test]
fn native_attempt_runs_one_temperature_keeps_failure_and_refuses_repeat() {
    let root = std::env::temp_dir().join(format!("epwflow-native-{}", store::token()));
    std::fs::create_dir_all(&root).unwrap();
    let mut inputs = native_inputs(&root);
    let long_source = root.join("long-native-source-directory-for-the-80-character-fortran-field");
    std::fs::create_dir(&long_source).unwrap();
    std::fs::copy(&inputs.a2f.path, long_source.join("original.a2f")).unwrap();
    inputs.a2f = store::Identity::capture(&long_source.join("original.a2f")).unwrap();
    assert!(inputs.a2f.path.as_os_str().len() > 80);
    for mode in [config::Mode::Iso, config::Mode::Aniso] {
        let r = request(mode, attempt::SolveKind::Nonlinear);
        let mapping = if mode == config::Mode::Iso {
            vec![]
        } else {
            states()
        };
        std::fs::write(root.join("stdout.fixture"), solve_log(&r)).unwrap();
        std::fs::write(root.join("gap.fixture"), raw_gap(&r, &mapping, 2.0)).unwrap();
        let script = root.join("epw-fixture");
        let output_name = format!(
            "mgb2.imag_{}_020.00",
            if mode == config::Mode::Iso {
                "iso"
            } else {
                "aniso"
            }
        );
        // 路径通过程序 args 传递，不执行命令字符串插值。
        write_script(&script,&format!("#!/bin/bash\ntest \"$2\" = '-inp' || exit $?\ntest -f \"$3\" || exit $?\ncat \"$1/stdout.fixture\" || exit $?\ncp \"$1/gap.fixture\" '{output_name}'\n"));
        let program = config::Program {
            path: script,
            args: vec![root.to_string_lossy().into_owned()],
        };
        let dir = root.join(if mode == config::Mode::Iso {
            "iso"
        } else {
            "aniso"
        });
        let result = attempt::run_attempt(&r, &inputs, &program, &[], &dir).unwrap();
        assert_eq!(result.status, "valid", "{:?}", result.reason);
        assert!(result.gap.is_some());
        assert_eq!(result.exit_status, Some(0));
        let input = std::fs::read_to_string(dir.join("epw.in")).unwrap();
        assert!(input.contains("nstemp = 1"));
        assert!(input.contains("temps(1) = 20.00"));
        assert!(input.contains("gap_edge = 1.0000000000000000e-3"));
        assert!(input.contains("imag_read = .false."));
        assert!(input.contains("fila2f = 'source.a2f'"));
        assert_eq!(
            store::hash_file(&dir.join("source.a2f")).unwrap(),
            inputs.a2f.sha256
        );
        assert!(attempt::run_attempt(&r, &inputs, &program, &[], &dir).is_err());
    }
    let r = request(config::Mode::Iso, attempt::SolveKind::Linear);
    let script = root.join("fail");
    write_script(
        &script,
        "#!/bin/bash\nprintf 'original error' >&2\nexit 7\n",
    );
    let p = config::Program {
        path: script,
        args: vec![],
    };
    let result = attempt::run_attempt(&r, &inputs, &p, &[], &root.join("failed")).unwrap();
    assert_eq!(result.status, "failed");
    assert_eq!(result.exit_status, Some(7));
    assert!(std::fs::read_to_string(root.join("failed/epw.err"))
        .unwrap()
        .contains("original error"));
    std::fs::write(&inputs.a2f.path, "changed").unwrap();
    assert!(attempt::run_attempt(&r, &inputs, &p, &[], &root.join("changed")).is_err());
    assert!(!root.join("changed").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_broyden_history_rejects_unsupported_size_before_launch() {
    let root = std::env::temp_dir().join(format!("epwflow-history-{}", store::token()));
    std::fs::create_dir(&root).unwrap();
    let mut cfg = config::Config::load(&make_config(&root)).unwrap();
    let mut r = request(config::Mode::Iso, attempt::SolveKind::Nonlinear);
    for history in [0, 9, u32::MAX] {
        cfg.superconductivity.mixing_history = history;
        r.mixing_history = history;
        assert!(cfg.validate().is_err());
        assert!(r.validate().is_err());
    }
    for history in [1, 8] {
        cfg.superconductivity.mixing_history = history;
        r.mixing_history = history;
        cfg.validate().unwrap();
        r.validate().unwrap();
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn centikelvin_search_brackets_handles_uncertain_and_detects_inconsistent() {
    let mut s = search::Search::new(1000, 5500, 50).unwrap();
    let mut seen = std::collections::BTreeSet::new();
    while let Some(t) = s.next_temperature() {
        assert!(seen.insert(t));
        s.observe(
            t,
            if t < 3200 {
                search::Side::Superconducting
            } else {
                search::Side::Normal
            },
        )
        .unwrap();
    }
    assert_eq!(s.status, "located");
    let [a, b] = s.interval_centikelvin.unwrap();
    assert!(a < 3200 && b >= 3200 && b - a <= 50);
    let mut q = search::Search::new(100, 900, 1).unwrap();
    q.observe(100, search::Side::Superconducting).unwrap();
    q.observe(900, search::Side::Normal).unwrap();
    assert_eq!(q.next_temperature(), Some(500));
    q.observe(500, search::Side::Uncertain).unwrap();
    assert_eq!(q.next_temperature(), Some(300));
    q.observe(300, search::Side::Uncertain).unwrap();
    assert_eq!(q.next_temperature(), Some(700));
    q.observe(700, search::Side::Uncertain).unwrap();
    assert_eq!(q.next_temperature(), None);
    assert_eq!(q.status, "classification_uncertain");
    let mut broken = search::Search::new(100, 900, 10).unwrap();
    broken.observe(100, search::Side::Normal).unwrap();
    broken.observe(900, search::Side::Superconducting).unwrap();
    assert_eq!(broken.status, "inconsistent");
    assert!(broken.observe(900, search::Side::Normal).is_err());
}

#[test]
fn native_stage_requires_existing_sufficient_allocation() {
    use epwflow::foundation::budget::BudgetLedger;
    let root = std::env::temp_dir().join(format!("epwflow-f4-budget-{}", store::token()));
    std::fs::create_dir_all(&root).unwrap();
    let cfg = config::Config::load(&make_config(&root)).unwrap();
    let mut value = serde_json::json!({
        "schema_version":1,"campaign_id":"fixture",
        "limits":{"node_seconds":28800,"storage_bytes":107374182400_u64,"max_lanes":2},
        "spent_node_seconds":0,"reservations":[],"lanes":1,"storage_roots":[],"revision":0,
        "allocations":[{"reservation_id":"native","job_id":42,
            "node_seconds":cfg.resources.nodes as u64 * cfg.resources.wall_seconds,
            "storage_bytes":cfg.resources.storage_bytes}]
    });
    let ledger: BudgetLedger = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(
        ledger.require_allocation(42, &cfg.resources).unwrap(),
        "native"
    );
    assert!(ledger.require_allocation(43, &cfg.resources).is_err());
    value["allocations"][0]["node_seconds"] = 0.into();
    let ledger: BudgetLedger = serde_json::from_value(value.clone()).unwrap();
    assert!(ledger.require_allocation(42, &cfg.resources).is_err());
    value["allocations"][0]["node_seconds"] =
        (cfg.resources.nodes as u64 * cfg.resources.wall_seconds).into();
    value["allocations"][0]["storage_bytes"] = 0.into();
    let ledger: BudgetLedger = serde_json::from_value(value).unwrap();
    assert!(ledger.require_allocation(42, &cfg.resources).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn missing_native_solver_cannot_publish_superconductivity() {
    let root = std::env::temp_dir().join(format!("epwflow-f4-{}", store::token()));
    std::fs::create_dir_all(&root).unwrap();
    let cfg_path = make_config(&root);
    let cfg = config::Config::load(&cfg_path).unwrap();
    let work = root.join("work");

    // 自称 valid 的合成清单没有原生求解或质量证据，必须拒绝。
    let f3 = work.join("interpolation");
    std::fs::create_dir_all(&f3).unwrap();
    let manifest = serde_json::json!({
        "schema_version": 2,
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
        "mode_readiness": {"iso": true},
        "validation": {
            "status": "valid",
            "checks": [{"name": "x", "passed": true, "detail": "ok"}],
            "units": {}, "thresholds": {}, "evidence": []
        }
    });
    std::fs::write(
        f3.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let result = scheduler::run(&cfg, &work, &[], Backend::Local);
    assert!(result.is_err(), "无原生求解证据不得发布超导结果");
    assert!(!work.join("superconductivity/manifest.json").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn binary_egnv_recovers_compressed_order_original_bands_and_weights() {
    use epwflow::superconductivity::source::{self, RY_EV};
    let mut bytes = vec![];
    for n in [2i32, 4, 4, 4, 2, 3] {
        bytes.extend(n.to_le_bytes());
    }
    // ef、ef0、DOS、展宽、费米窗，原生单位 Ry。
    for x in [0.5f64, 0.5, 2.0, 0.01, 0.1] {
        bytes.extend(x.to_le_bytes());
    }
    for (ik, ix, energies) in [(1i32, 2i32, [0.49f64, 0.8, 0.52]), (2, 1, [0.48, 0.5, 0.9])] {
        for x in [0.5f64, 0.0, 0.0, 0.0] {
            bytes.extend(x.to_le_bytes());
        }
        bytes.extend(ik.to_le_bytes());
        bytes.extend(ix.to_le_bytes());
        for x in energies {
            bytes.extend(x.to_le_bytes());
        }
    }
    let e = source::eigenvalues(&bytes).unwrap();
    assert_eq!(
        e.states
            .iter()
            .map(|s| (s.kpoint, s.band))
            .collect::<Vec<_>>(),
        vec![(2, 1), (2, 2), (1, 1), (1, 3)]
    );
    assert!((e.states[0].energy_ev + 0.02 * RY_EV).abs() < 1e-12);
    assert!(e.states.iter().all(|s| s.weight > 0.0));
    let expected = 0.5 * 0.5 * (-4.0f64).exp() / (std::f64::consts::PI.sqrt() * 0.01 * 2.0);
    assert!((e.states[0].weight - expected).abs() < 1e-12);
    assert!(source::eigenvalues(&bytes[..bytes.len() - 1]).is_err());
    let mut corrupt = bytes.clone();
    corrupt[0..4].copy_from_slice(&2i32.to_be_bytes());
    assert!(source::eigenvalues(&corrupt).is_err());
}

#[test]
fn native_engine_persists_counts_reuses_valid_and_does_not_retry_failed() {
    use epwflow::superconductivity::engine::Engine;
    let root = std::env::temp_dir().join(format!("epwflow-engine-{}", store::token()));
    std::fs::create_dir_all(&root).unwrap();
    let mut cfg = config::Config::load(&make_config(&root)).unwrap();
    cfg.superconductivity.frequency_cutoff_ev = 0.03;
    cfg.superconductivity.initial_gaps_mev = [1.0, 10.0];
    cfg.superconductivity.iso.as_mut().unwrap().max_k = 55.0;
    cfg.superconductivity.iso.as_mut().unwrap().max_solves = 1;
    let inputs = native_inputs(&root);
    let req = request(config::Mode::Iso, attempt::SolveKind::Linear);
    std::fs::write(root.join("stdout.fixture"), solve_log(&req)).unwrap();
    let script = root.join("native-fixture");
    write_script(&script, "#!/bin/bash\ncat \"$1/stdout.fixture\"\n");
    cfg.programs.epw = config::Program {
        path: script,
        args: vec![root.to_string_lossy().into_owned()],
    };
    let dir = root.join("engine");
    let mut calls = 0;
    {
        let mut e = Engine::open(&cfg, &inputs, &[], &dir, config::Mode::Iso).unwrap();
        e.attempt(2000, attempt::SolveKind::Linear, None, &mut || {
            calls += 1;
            Ok(())
        })
        .unwrap();
    }
    let mut e = Engine::open(&cfg, &inputs, &[], &dir, config::Mode::Iso).unwrap();
    e.attempt(2000, attempt::SolveKind::Linear, None, &mut || {
        calls += 1;
        Ok(())
    })
    .unwrap();
    assert_eq!(calls, 1);
    assert_eq!(e.result.attempts.len(), 1);
    let err = e
        .attempt(2100, attempt::SolveKind::Linear, None, &mut || {
            calls += 1;
            Ok(())
        })
        .unwrap_err();
    assert!(err.to_string().contains("attempt_limit"));
    assert_eq!(calls, 1);
    // 模拟提交/进程启动后中断：未知状态不能自动重提。
    e.result.attempts[0].status = "running".into();
    e.result.attempts[0].record = None;
    e.save().unwrap();
    drop(e);
    let mut e = Engine::open(&cfg, &inputs, &[], &dir, config::Mode::Iso).unwrap();
    assert!(e
        .attempt(2000, attempt::SolveKind::Linear, None, &mut || {
            calls += 1;
            Ok(())
        })
        .unwrap_err()
        .to_string()
        .contains("scheduler_unknown"));
    assert_eq!(calls, 1);
    // 确认失败也保持占用次数，不因重新打开而归零。
    e.result.attempts[0].status = "failed".into();
    e.save().unwrap();
    drop(e);
    let mut e = Engine::open(&cfg, &inputs, &[], &dir, config::Mode::Iso).unwrap();
    assert!(e
        .attempt(2000, attempt::SolveKind::Linear, None, &mut || {
            calls += 1;
            Ok(())
        })
        .is_err());
    assert_eq!(calls, 1);
    cfg.superconductivity.mu_star = 0.15;
    assert!(Engine::open(&cfg, &inputs, &[], &dir, config::Mode::Iso).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_engine_both_modes_use_process_results_and_keep_independent_intervals() {
    use epwflow::superconductivity::engine::Engine;
    let root = std::env::temp_dir().join(format!("epwflow-engine-both-{}", store::token()));
    std::fs::create_dir_all(&root).unwrap();
    let mut cfg = config::Config::load(&make_config(&root)).unwrap();
    cfg.superconductivity.mode = config::Mode::Both;
    cfg.superconductivity.frequency_cutoff_ev = 0.03;
    cfg.superconductivity.initial_gaps_mev = [1.0, 10.0];
    cfg.superconductivity.gap_temperatures_k = vec![20.0, 25.0, 30.0];
    cfg.superconductivity.convergence_relative = 1e-8;
    let tc = cfg.superconductivity.iso.as_mut().unwrap();
    tc.min_k = 10.0;
    tc.max_k = 55.0;
    tc.precision_k = 5.0;
    tc.max_solves = 64;
    tc.normal_gap_mev = 1e-4;
    tc.super_gap_mev = 1e-3;
    cfg.superconductivity.aniso = cfg.superconductivity.iso.clone();
    let mut inputs = native_inputs(&root);
    let f3_path = root.join("interpolation/manifest.json");
    store::write(&f3_path,&serde_json::json!({"schema_version":2,"run_id":"native-fixture","f2_digest":"fixture","wannier_digest":"fixture",
        "coarse":null,"fine":null,"spectra":[{"smearing_id":"fixture","energy_mev":[0.0,10.0,20.0],"a2f":[0.0,0.2,0.1],
            "lambda_integral":0.35,"lambda_sum":0.35,"cutoff_ev":0.03}],"mode_readiness":{},
        "validation":{"status":"valid","checks":[{"name":"fixture","passed":true,"detail":"SYNTHETIC IO fixture"}],"units":{},"thresholds":{},"evidence":[]}})).unwrap();
    inputs.interpolation_digest = store::hash_file(&f3_path).unwrap();
    let mut native_modes = std::collections::BTreeMap::new();
    // 此进程只模拟目标版本的 IO/收敛契约。阈值用于验证独立搜索，不是物理模型。
    let script = root.join("epw-fixture.py");
    write_script(&script, include_str!("fixtures/epw_tc_io.py"));
    cfg.programs.epw = config::Program {
        path: script,
        args: vec!["--engine-fixture-states".into()],
    };
    let mut results = vec![];
    for mode in [config::Mode::Iso, config::Mode::Aniso] {
        let dir = root
            .join("superconductivity")
            .join(if mode == config::Mode::Iso {
                "iso"
            } else {
                "aniso"
            });
        let mut e = Engine::open(&cfg, &inputs, &[], &dir, mode).unwrap();
        let mut calls = 0;
        let mut probe = || {
            calls += 1;
            Ok(())
        };
        assert_eq!(
            e.probe_upper_endpoint(&mut probe).unwrap(),
            search::Side::Normal
        );
        assert_eq!(
            e.probe_upper_endpoint(&mut probe).unwrap(),
            search::Side::Normal
        );
        assert!(e
            .result
            .attempts
            .iter()
            .all(|a| a.request.temperature_centikelvin == 5500));
        assert_eq!(
            e.result.attempts.len(),
            if mode == config::Mode::Iso { 1 } else { 2 }
        );
        e.run(&mut || {
            calls += 1;
            Ok(())
        })
        .unwrap();
        assert!(e.result.complete, "{:?}", e.result.reason);
        assert_eq!(e.result.search.status, "located");
        assert_eq!(calls, e.result.attempts.len());
        let n = calls;
        drop(e);
        let mut resumed = Engine::open(&cfg, &inputs, &[], &dir, mode).unwrap();
        resumed
            .run(&mut || {
                calls += 1;
                Ok(())
            })
            .unwrap();
        assert!(resumed.result.complete);
        assert_eq!(calls, n);
        results.push(resumed.result.search.interval_centikelvin.unwrap());
        native_modes.insert(
            if mode == config::Mode::Iso {
                "iso".into()
            } else {
                "aniso".into()
            },
            resumed.result,
        );
    }
    assert_ne!(results[0], results[1]);
    // 固定 iso 非线性诊断可交付完整双初值，但不能冒充线性化 Tc 分类。
    let mut diagnostic = native_modes["iso"].clone();
    diagnostic.attempts.retain(|a| {
        a.request.kind == attempt::SolveKind::Nonlinear && a.request.temperature_centikelvin == 2000
    });
    assert_eq!(diagnostic.attempts.len(), 2);
    diagnostic.search = search::Search::new(1000, 5500, 500).unwrap();
    diagnostic.complete = false;
    diagnostic.reason = Some("固定非线性诊断，未执行线性 Tc 搜索".into());
    let diagnostic_root = root.join("iso-nonlinear-diagnostic");
    let partial = scheduler::publish(
        &diagnostic_root,
        "native-fixture".into(),
        inputs.interpolation_digest.clone(),
        config::Mode::Iso,
        std::collections::BTreeMap::from([("iso".into(), diagnostic.clone())]),
    )
    .unwrap();
    assert!(!partial.converged && !partial.located);
    let loaded = epwflow::superconductivity::manifest::SuperconductivityManifest::load(
        &diagnostic_root.join("manifest.json"),
    )
    .unwrap();
    assert_eq!(loaded.native_modes["iso"].attempts.len(), 2);
    diagnostic
        .search
        .observe(2000, search::Side::Superconducting)
        .unwrap();
    assert!(scheduler::publish(
        &root.join("rejected-fake-linear"),
        "native-fixture".into(),
        inputs.interpolation_digest.clone(),
        config::Mode::Iso,
        std::collections::BTreeMap::from([("iso".into(), diagnostic)]),
    )
    .is_err());
    let root_sc = root.join("superconductivity");
    let original = scheduler::publish(
        &root_sc,
        "native-fixture".into(),
        inputs.interpolation_digest.clone(),
        config::Mode::Both,
        native_modes,
    )
    .unwrap();
    epwflow::superconductivity::verify::complete(&root, &cfg, "native-fixture").unwrap();
    let mut tampered = original.clone();
    tampered
        .native_modes
        .get_mut("iso")
        .unwrap()
        .search
        .interval_centikelvin = Some([1000, 1001]);
    store::write(&root_sc.join("manifest.json"), &tampered).unwrap();
    assert!(
        epwflow::superconductivity::manifest::SuperconductivityManifest::load(
            &root_sc.join("manifest.json")
        )
        .is_err()
    );
    let mut missing_mode = original.clone();
    missing_mode.native_modes.remove("aniso");
    store::write(&root_sc.join("manifest.json"), &missing_mode).unwrap();
    assert!(
        epwflow::superconductivity::manifest::SuperconductivityManifest::load(
            &root_sc.join("manifest.json")
        )
        .is_err()
    );
    store::write(&root_sc.join("manifest.json"), &original).unwrap();
    let mut partial_modes = original.native_modes.clone();
    let aniso = partial_modes.get_mut("aniso").unwrap();
    aniso.complete = false;
    aniso.reason = Some("fixture: stopped after a solver failure".into());
    aniso
        .search
        .stop("solver_failed", "fixture: no Tc estimate");
    scheduler::publish(
        &root_sc,
        "native-fixture".into(),
        inputs.interpolation_digest.clone(),
        config::Mode::Both,
        partial_modes,
    )
    .unwrap();
    assert!(epwflow::superconductivity::verify::complete(&root, &cfg, "native-fixture").is_err());
    let partial_output = root.join("partial-results");
    epwflow::delivery::collect_results(&root, Some(&partial_output)).unwrap();
    let (_, partial) = epwflow::delivery::native::load(&partial_output).unwrap();
    assert!(!partial.complete);
    assert!(partial.modes["aniso"].search.estimate_k().is_none());
    assert_eq!(
        epwflow::delivery::check_results(&partial_output).unwrap()["workflow_complete"],
        false
    );
    scheduler::publish(
        &root_sc,
        "native-fixture".into(),
        inputs.interpolation_digest.clone(),
        config::Mode::Both,
        original.native_modes.clone(),
    )
    .unwrap();
    let output = epwflow::delivery::collect_results(&root, None).unwrap();
    epwflow::delivery::collect_results(&root, None).unwrap();
    for action in ["collect", "results"] {
        let cli = stage_cli::command(&["delivery", action, "-w", root.to_str().unwrap(), "--json"], &root)
            .env("PATH", "")
            .output()
            .unwrap();
        assert!(
            cli.status.success(),
            "{}",
            String::from_utf8_lossy(&cli.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&cli.stdout).unwrap();
        assert_eq!(report["native_complete"], true);
        assert_eq!(report["workflow_complete"], false, "尚未生成三格式图");
    }
    let (_, data) = epwflow::delivery::native::load(&output).unwrap();
    assert_eq!(data.modes.len(), 2);
    assert!(!data.states.is_empty());
    assert!(data
        .states
        .iter()
        .all(|s| s.k_crystal.is_some() && s.z > 0.0));
    let moved = std::env::temp_dir().join(format!("epwflow-moved-{}", store::token()));
    std::fs::rename(&output, &moved).unwrap();
    let hidden = root.with_extension("hidden");
    std::fs::rename(&root, &hidden).unwrap();
    assert_eq!(
        epwflow::delivery::native::check(&moved).unwrap()["source_verification"],
        "unavailable"
    );
    let command = std::process::Command::new(env!("CARGO_BIN_EXE_epw_superconductivity"))
        .args(["--plot", "--results", moved.to_str().unwrap()])
        .env("PATH", "")
        .output()
        .unwrap();
    assert!(
        command.status.success(),
        "{}",
        String::from_utf8_lossy(&command.stderr)
    );
    assert!(epwflow::delivery::native_plot::check(&moved).unwrap());
    let distribution =
        std::fs::read_to_string(moved.join("figures/gap_aniso_distribution.svg")).unwrap();
    for label in [
        "Temperature (K)",
        "|Delta_nk(iw0)| (meV)",
        "normalized Fermi-surface weight",
        "no temperature interpolation",
    ] {
        assert!(distribution.contains(label), "缺少分布说明 {label}");
    }
    if let Ok(dest) = std::env::var("EPWFLOW_FIXTURE_ARTIFACTS") {
        std::fs::create_dir_all(&dest).unwrap();
        for ext in ["png", "svg", "pdf"] {
            let name = format!("gap_aniso_distribution.{ext}");
            std::fs::copy(
                moved.join("figures").join(&name),
                PathBuf::from(&dest).join(&name),
            )
            .unwrap();
        }
    }
    assert_eq!(
        epwflow::delivery::check_results(&moved).unwrap()["workflow_complete"],
        true
    );
    let before = store::hash_file(&moved.join("gap_states.csv")).unwrap();
    epwflow::delivery::plot_results(&moved, None, &["svg".into()]).unwrap();
    assert_eq!(
        store::hash_file(&moved.join("gap_states.csv")).unwrap(),
        before
    );
    // 同源部分重绘保留其他有效格式的归属，三格式完成门仍通过。
    assert!(epwflow::delivery::native_plot::check(&moved).unwrap());
    epwflow::delivery::plot_results(
        &moved, None, &["png".into(), "svg".into(), "pdf".into()],
    ).unwrap();
    if let Ok(dest) = std::env::var("EPWFLOW_FIXTURE_ARTIFACTS") {
        std::fs::create_dir_all(&dest).unwrap();
        for name in ["summary.png", "summary.svg", "summary.pdf"] {
            std::fs::copy(
                moved.join("figures").join(name),
                PathBuf::from(&dest).join(name),
            )
            .unwrap();
        }
    }
    std::fs::write(moved.join("gap_states.csv"), "tampered").unwrap();
    assert!(epwflow::delivery::native::load(&moved).is_err());
    std::fs::remove_dir_all(moved).unwrap();
    std::fs::remove_dir_all(hidden).unwrap();
}

#[test]
fn native_fine_handoff_binds_binary_roles_band_offset_and_rejects_changes() {
    use epwflow::{
        interpolation::{epw_config::EpwConfig, fine, manifest::InterpolationManifest, native},
        superconductivity::source::{self, RY_EV},
    };
    let root = std::env::temp_dir().join(format!("epwflow-native-fine-{}", store::token()));
    std::fs::create_dir_all(&root).unwrap();
    let mut cfg = config::Config::load(&make_config(&root)).unwrap();
    cfg.wannier.num_wann = 4;
    cfg.interpolation.spectral_points = 3;
    let dir = root.join("fine");
    let matrix = dir.join("out/scf.ephmat");
    std::fs::create_dir_all(&matrix).unwrap();
    let mut bytes = vec![];
    for n in [2i32, 4, 4, 4, 2, 3] {
        bytes.extend(n.to_le_bytes());
    }
    for x in [
        0.5f64,
        0.5,
        2.0,
        cfg.interpolation.degaussw_ev / RY_EV,
        cfg.interpolation.fsthick_ev / RY_EV,
    ] {
        bytes.extend(x.to_le_bytes());
    }
    for (ik, ix, energies) in [(1i32, 2i32, [0.49f64, 0.8, 0.52]), (2, 1, [0.48, 0.5, 0.9])] {
        for x in [0.5f64, 0.0, 0.0, 0.0] {
            bytes.extend(x.to_le_bytes());
        }
        bytes.extend(ik.to_le_bytes());
        bytes.extend(ix.to_le_bytes());
        for x in energies {
            bytes.extend(x.to_le_bytes());
        }
    }
    std::fs::write(matrix.join("egnv"), &bytes).unwrap();
    let mut freq = vec![];
    for n in [8i32, 2, 2, 2, 3] {
        freq.extend(n.to_le_bytes());
    }
    for _ in 0..8 {
        for x in [0.0f64, 0.0, 0.0, 0.001, 0.002, 0.003] {
            freq.extend(x.to_le_bytes());
        }
    }
    std::fs::write(matrix.join("freq"), freq).unwrap();
    let mut ikmap = vec![];
    for _ in 0..2 {
        for i in 0..64 {
            ikmap.extend(((i % 2 + 1) as i32).to_le_bytes());
        }
    }
    std::fs::write(matrix.join("ikmap"), ikmap).unwrap();
    std::fs::write(matrix.join("npool"), 1i32.to_le_bytes()).unwrap();
    let shard = [1i32.to_le_bytes(), 2i32.to_le_bytes()].concat();
    std::fs::write(
        matrix.join("ephmat1"),
        [shard, 0.001f64.to_le_bytes().to_vec()].concat(),
    )
    .unwrap();
    std::fs::write(dir.join("restart.fmt"), "8\n0\n0\n1\n1\n0\n").unwrap();
    std::fs::write(dir.join("epw_fine.in"), "fixture input").unwrap();
    std::fs::write(dir.join("epw.out"),"Program EPW v.5.9 starts\nibndmin = 2 ebndmin = -1.0 eV\nibndmax = 4 ebndmax = 1.0 eV\nTotal program execution\nEPW : 1s CPU 2s WALL\n").unwrap();
    std::fs::write(dir.join("crystal.fmt"), "fixture crystal").unwrap();
    std::fs::write(dir.join("selecq.fmt"), "8\n8\n1 2 3 4 5 6 7 8\n").unwrap();
    let table="w[meV] a2f and integrated 2*a2f/w for 1 smearing values\n1 1 2\n2 2 4\n3 3 6\nIntegrated el-ph coupling\n # 6\nSummed el-ph coupling 6\n";
    std::fs::write(dir.join("scf.a2f"), table).unwrap();
    let epw = EpwConfig::from_config(&cfg).unwrap();
    let fr = fine::collect_fine(&root, &epw).unwrap();
    let fd = fine::into_fine_data(&fr);
    assert_eq!(fd.layout, source::FORMAT);
    let parsed = native::NativeSpectrum::parse(table, 3).unwrap();
    let manifest = serde_json::json!({
        "schema_version":2,"run_id":"fixture-native","f2_digest":"fixture","wannier_digest":"fixture",
        "coarse":null,"fine":fd,
        "spectra":[{"smearing_id":"fixture","energy_mev":parsed.energy_mev,"a2f":parsed.a2f[0],
            "lambda_integral":6.0,"lambda_sum":6.0,"cutoff_ev":0.2}],
        "mode_readiness":{},
        "validation":{"status":"valid","checks":[{"name":"fixture","passed":true,"detail":"IO only"}],"units":{},"thresholds":{},"evidence":[]}
    });
    store::write(&root.join("interpolation/manifest.json"), &manifest).unwrap();
    let m = InterpolationManifest::load(&root.join("interpolation/manifest.json")).unwrap();
    let bound = source::bind(&root, &cfg, &m).unwrap();
    assert_eq!(
        bound
            .states
            .iter()
            .map(|s| (s.kpoint, s.band))
            .collect::<Vec<_>>(),
        vec![(2, 2), (2, 3), (1, 2), (1, 4)]
    );
    bound.verify(config::Mode::Aniso).unwrap();
    let evidence_path = root.join("native-stage-report.json");
    std::fs::write(&evidence_path, "verified stage fixture").unwrap();
    let evidence = store::Identity::capture(&evidence_path).unwrap();
    let stage_bound =
        source::bind_native_roles(&root, &cfg, m.fine.as_ref().unwrap(), &m.spectra, &evidence)
            .unwrap();
    assert_eq!(stage_bound.interpolation_digest, evidence.sha256);
    assert_eq!(stage_bound.states.len(), bound.states.len());
    std::fs::write(&evidence_path, "changed").unwrap();
    assert!(source::bind_native_roles(
        &root,
        &cfg,
        m.fine.as_ref().unwrap(),
        &m.spectra,
        &evidence,
    )
    .is_err());
    std::fs::write(matrix.join("egnv"), &bytes[..bytes.len() - 1]).unwrap();
    assert!(source::bind(&root, &cfg, &m).is_err());
    assert!(fine::collect_fine(&root, &epw).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_explicit_resume_preserves_failed_evidence_and_counts_unknown_attempts() {
    use epwflow::superconductivity::engine::{Engine, Entry};
    let root = std::env::temp_dir().join(format!("epwflow-native-retry-{}", store::token()));
    std::fs::create_dir_all(&root).unwrap();
    let mut cfg = config::Config::load(&make_config(&root)).unwrap();
    cfg.superconductivity.frequency_cutoff_ev = 0.03;
    cfg.superconductivity.iso.as_mut().unwrap().max_k = 55.0;
    cfg.superconductivity.iso.as_mut().unwrap().max_solves = 3;
    let input = native_inputs(&root);
    std::fs::write(
        root.join("stdout.fixture"),
        solve_log(&request(config::Mode::Iso, attempt::SolveKind::Linear)),
    )
    .unwrap();
    let script = root.join("retry-fixture");
    write_script(
        &script,
        "#!/bin/sh\nif [ ! -f \"$1/allow\" ]; then exit 7; fi\ncat \"$1/stdout.fixture\"\n",
    );
    cfg.programs.epw = config::Program {
        path: script,
        args: vec![root.to_string_lossy().into_owned()],
    };
    let dir = root.join("search");
    let mut engine = Engine::open(&cfg, &input, &[], &dir, config::Mode::Iso).unwrap();
    assert!(engine
        .attempt(2000, attempt::SolveKind::Linear, None, &mut || Ok(()))
        .is_err());
    let old = store::Identity::capture(&dir.join("attempt-0001/attempt.json")).unwrap();
    drop(engine);
    let mut engine = Engine::open(&cfg, &input, &[], &dir, config::Mode::Iso).unwrap();
    assert!(engine
        .attempt(2000, attempt::SolveKind::Linear, None, &mut || Ok(()))
        .is_err());
    engine
        .prepare_resume(&mut |_| panic!("已结束进程不需猜测调度状态"))
        .unwrap();
    std::fs::write(root.join("allow"), "fixture").unwrap();
    engine
        .attempt(2000, attempt::SolveKind::Linear, None, &mut || Ok(()))
        .unwrap();
    old.verify().unwrap();
    assert_eq!(engine.result.attempts.len(), 2);
    let req = attempt::Request::from_config(
        &cfg,
        config::Mode::Iso,
        attempt::SolveKind::Linear,
        2100,
        None,
    )
    .unwrap();
    engine.result.attempts.push(Entry {
        request: req,
        directory: dir.join("attempt-0003"),
        record: None,
        status: "running".into(),
        reason: None,
        job_id: Some(99),
    });
    engine
        .result
        .search
        .stop("scheduler_unknown", "fixture interrupted");
    engine.save().unwrap();
    drop(engine);
    let mut engine = Engine::open(&cfg, &input, &[], &dir, config::Mode::Iso).unwrap();
    assert!(engine
        .prepare_resume(&mut |_| anyhow::bail!("scheduler_unknown: active"))
        .is_err());
    engine
        .prepare_resume(&mut |id| {
            assert_eq!(id, 99);
            Ok(())
        })
        .unwrap();
    assert_eq!(engine.result.attempts[2].status, "failed");
    assert_eq!(engine.result.search.status, "pending");
    let mut launches = 0;
    let error = engine
        .attempt(2100, attempt::SolveKind::Linear, None, &mut || {
            launches += 1;
            Ok(())
        })
        .unwrap_err();
    assert!(error.to_string().contains("attempt_limit"));
    assert_eq!(launches, 0);
    assert_eq!(engine.result.attempts.len(), 3);
    std::fs::remove_dir_all(root).unwrap();
}
