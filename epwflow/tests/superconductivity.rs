//! F4 行为测试：FR-001~FR-013。隔离临时目录与可执行 fixture。

#[path = "support/cli.rs"]
mod stage_cli;

#[path="support/native_fixture.rs"]
mod native_fixture;
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
    let p = root.join("config.toml");
    std::fs::write(
        &p,
        format!(
            r#"work_dir = "{work}"
structure = "{structure}"
schema_version = 2
outdir = "{work}/out"

[pseudos]
Si = "{structure}"

[programs]
launcher = []
pw_x = "{pw}"
pw_args = []
ph_x = "{ph}"
ph_args = []
epw_x = "{epw}"
epw_args = []
wannier90_x = "{wan}"
wannier90_args = []
pw2wannier90_x = "{pw2}"
pw2wannier90_args = []
bands_x = "{pw}"
bands_args = []
dos_x = "{pw}"
dos_args = []
projwfc_x = "{pw}"
projwfc_args = []
q2r_x = "{pw}"
q2r_args = []
matdyn_x = "{pw}"
matdyn_args = []
postw90_x = "{pw}"
postw90_args = []

[slurm]
header = """
#SBATCH --partition=local
#SBATCH --nodes=1
#SBATCH --ntasks=1
"""

[stages."1.qe-scf"]
K_POINTS = "automatic\n2 2 2 0 0 0"
ATOMIC_SPECIES = "Si 28.0855 {structure}"

calculation = "scf"
prefix = "scf"

ecutwfc = 30.0
ecutrho = 240.0
occupations = "fixed"

conv_thr = 1e-8

[stages."2.qe-nscf"]
K_POINTS = [2, 2, 2]
ATOMIC_SPECIES = "Si 28.0855 {structure}"

calculation = "nscf"
prefix = "scf"

ecutwfc = 30.0
ecutrho = 240.0
occupations = "fixed"
nbnd = 4
nosym = false
noinv = false

conv_thr = 1e-8

[workflow."3.qe-bands"]
points_per_edge = 20

[workflow."4.qe-phonon-index"]
phonon_sigma_cm1 = 10.0
points_per_edge = 100

[workflow."5.epw-wannier"]
points_per_edge = 20
fermi_source = "f2"
selection_mode = "fixed_plus_projectability"

[stages."5.wannier90"]
bands_plot = true
bands_num_points = 1
dis_proj_min = 0.01
dis_proj_max = 0.95
dis_froz_proj = true
dis_num_iter = 100
dis_conv_tol = 1e-6
dis_conv_window = 10
num_wann = 2
projections = [ "Si:sp3",]
exclude_bands = []
num_iter = 100
conv_tol = 1e-6
conv_window = 10

[workflow."6.epw-coarse"]
points_per_edge = 20
fermi_source = "f2"
onsite_asr = "gamma_hermitian"

[workflow."7.epw-fine"]
fermi_source = "f2"

[workflow."8.epw-superconductivity"]
task = "tc"
mode = "iso"
spectrum_index = 0
initial_gaps = [ 0.0005, 0.0015,]

[stages."3.qe-bands"]
ATOMIC_SPECIES = "Si 28.0855 {structure}"

calculation = "bands"
prefix = "scf"

ecutwfc = 30.0
ecutrho = 240.0
occupations = "fixed"
nbnd = 4
nosym = true
noinv = true

conv_thr = 1e-8

[stages."4.qe-phonon-index"]
prefix = "scf"
tr2_ph = 1e-12
ldisp = true
only_init = true
lqdir = true
recover = false
only_wfc = false
fildvscf = "dvscf"
nq1 = 2
nq2 = 2
nq3 = 2

[stages."4.qe-q2r"]
zasr = "simple"

[stages."5.epw-wannier"]
prefix = "scf"
ep_coupling = true
elph = false
etf_mem = 0
nbndsub = 2
iverbosity = 2
amass = [ 28.0855,]
fsthick = 1.0
degaussw = 0.01
degaussq = 1.0
nqstep = 4
nqsmear = 1
eps_acoustic = 0.1
elecselfen = false
phonselfen = false
a2f = false
ngaussw = 0
efermi_read = true
nk1 = 2
nk2 = 2
nk3 = 2
nq1 = 2
nq2 = 2
nq3 = 2
epwread = false
epwwrite = true
wannierize = true
ephwrite = false
eliashberg = false
band_plot = false
num_iter = 100

[stages."5.postw90-dos"]
write_hr = false
write_bvec = false
bands_plot = false
dos = true
num_elec_per_state = 2
dos_kmesh = [ 2, 2, 2,]
dos_energy_step = 0.01
dos_adpt_smr = false
dos_smr_fixed_en_width = 0.1
dos_smr_type = "gauss"

[stages."6.epw-coarse"]
prefix = "scf"
ep_coupling = true
elph = true
etf_mem = 0
nbndsub = 2
iverbosity = 2
amass = [ 28.0855,]
fsthick = 1.0
degaussw = 0.01
degaussq = 1.0
nqstep = 4
nqsmear = 1
eps_acoustic = 0.1
elecselfen = false
phonselfen = false
a2f = false
ngaussw = 0
efermi_read = true
nk1 = 2
nk2 = 2
nk3 = 2
nq1 = 2
nq2 = 2
nq3 = 2
epwread = false
epwwrite = true
wannierize = false
ephwrite = false
eliashberg = false
band_plot = false
nkf1 = 2
nkf2 = 2
nkf3 = 2
nqf1 = 2
nqf2 = 2
nqf3 = 2

[stages."7.epw-fine"]
prefix = "scf"
ep_coupling = true
elph = true
etf_mem = 0
nbndsub = 2
iverbosity = 2
amass = [ 28.0855,]
fsthick = 1.0
degaussw = 0.01
degaussq = 1.0
nqstep = 4
nqsmear = 1
eps_acoustic = 0.1
elecselfen = false
phonselfen = false
a2f = false
ngaussw = 0
efermi_read = true
nk1 = 2
nk2 = 2
nk3 = 2
nq1 = 2
nq2 = 2
nq3 = 2
epwread = true
epwwrite = false
wannierize = false
ephwrite = true
eliashberg = true
liso = false
laniso = false
mp_mesh_k = true
nstemp = 1
temps = [ 2.0,]
nkf1 = 4
nkf2 = 4
nkf3 = 4
nqf1 = 2
nqf2 = 2
nqf3 = 2

[workflow."8.epw-superconductivity".iso]
min_k = 1.0
max_k = 10.0
precision_k = 0.1
max_solves = 20
eigen_margin = 1e-6
normal_gap_mev = 0.1
super_gap_mev = 2.0

[stages."3.qe-bands-post"]
prefix = "scf"
lsym = false

[stages."3.qe-dos"]
prefix = "scf"
ngauss = 0
degauss = 0.007349864435130998
Emin = -100.0
Emax = 100.0
DeltaE = 0.01

[stages."3.qe-pdos"]
prefix = "scf"
ngauss = 0
degauss = 0.007349864435130998
Emin = -100.0
Emax = 100.0
DeltaE = 0.01

[stages."4.qe-dispersion"]
asr = "simple"
q_in_cryst_coord = true
q_in_band_form = true

[stages."4.qe-phdos"]
asr = "simple"
dos = true
ndos = 200
nk1 = 8
nk2 = 8
nk3 = 8

[stages."4.qe-phonon-index-q"]
prefix = "scf"
tr2_ph = 1e-12
ldisp = true
only_init = false
lqdir = true
recover = false
only_wfc = false
fildvscf = "dvscf"
nq1 = 2
nq2 = 2
nq3 = 2

[stages."6.qe-q2r"]
zasr = "no"

[stages."6.qe-matdyn"]
asr = "no"
q_in_cryst_coord = false
q_in_band_form = false
amass = [ 28.0855,]

[stages."6.epw-comparison"]
prefix = "scf"
ep_coupling = true
elph = true
etf_mem = 0
nbndsub = 2
iverbosity = 2
amass = [ 28.0855,]
fsthick = 1.0
degaussw = 0.01
degaussq = 1.0
nqstep = 4
nqsmear = 1
eps_acoustic = 0.1
elecselfen = false
phonselfen = false
a2f = false
ngaussw = 0
efermi_read = true
nk1 = 2
nk2 = 2
nk3 = 2
nq1 = 2
nq2 = 2
nq3 = 2
epwread = true
epwwrite = false
wannierize = false
ephwrite = false
eliashberg = false
band_plot = true
epbread = false
selecqread = true
filkf = "kpoints.dat"
filqf = "qpoints.dat"

[stages."8.epw-iso"]
prefix = "scf"
epwread = true
epbread = false
epwwrite = false
wannierize = false
ep_coupling = false
elph = false
eliashberg = true
liso = true
laniso = false
tc_linear_solver = "lapack"
limag = true
lreal = false
lpade = false
lacon = false
imag_read = false
fbw = false
positive_matsu = true
gridsamp = 0
nswi = 0
nstemp = 2
temps = [ 2.0, 5.0,]
muc = 0.1
wscut = 0.2
nsiter = 600
conv_thr_iaxis = 1e-5
broyden_beta = 0.2
broyden_ndim = 4
a_gap0 = 1.0
nqstep = 4
iverbosity = 2
eps_acoustic = 0.1
mp_mesh_k = true
nkf1 = 4
nkf2 = 4
nkf3 = 4
nqf1 = 2
nqf2 = 2
nqf3 = 2

[stages."8.epw-aniso"]
prefix = "scf"
epwread = true
epbread = false
epwwrite = false
wannierize = false
ep_coupling = false
elph = false
eliashberg = true
liso = false
laniso = true
tc_linear_solver = "lapack"
limag = true
lreal = false
lpade = false
lacon = false
imag_read = false
fbw = false
positive_matsu = true
gridsamp = 0
nswi = 0
nstemp = 2
temps = [ 2.0, 5.0,]
muc = 0.1
wscut = 0.2
nsiter = 600
conv_thr_iaxis = 1e-5
broyden_beta = 0.2
broyden_ndim = 4
a_gap0 = 1.0
nqstep = 4
iverbosity = 2
eps_acoustic = 0.1
mp_mesh_k = true
nkf1 = 4
nkf2 = 4
nkf3 = 4
nqf1 = 2
nqf2 = 2
nqf3 = 2
"#,
            work = work.display(),
            structure = structure.display(),
            pw = pw.display(),
            ph = ph.display(),
            epw = epw.display(),
            wan = wan.display(),
            pw2 = pw2.display(),
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
        configured_stages: None,
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
        assert!(parse::numbers(bad).is_err(), "I must refuse {bad}");
    }
    let r = request(config::Mode::Aniso, attempt::SolveKind::Nonlinear);
    let text = raw_gap(&r, &states(), 1.7366215176e-99);
    let omitted = text.replace("1.7366215176e-102", "1.7366215176-102");
    assert_ne!(text, omitted, "You must overwrite the true omission format in the full-capable gap file");
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
    assert!(result.is_err(), "You can't release a superconductor without proof of native solvency");
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
fn gap_task_config_has_no_tc_or_dual_seed_requirement() {
    let root = std::env::temp_dir().join(format!("gap-config-{}", store::token()));
    std::fs::create_dir_all(&root).unwrap();
    let path = make_config(&root);
    let mut value: toml::Value = std::fs::read_to_string(&path).unwrap().parse().unwrap();
    let sc=value["workflow"]["8.epw-superconductivity"].as_table_mut().unwrap();
    sc.insert("task".into(),"gap".into());sc.insert("mode".into(),"both".into());
    for name in ["iso","aniso","initial_gaps"]{sc.remove(name);}
    for method in ["iso","aniso"] {
        let epw=value["stages"][format!("8.epw-{method}")].as_table_mut().unwrap();
        epw.insert("temps".into(),toml::Value::Array(vec![20.into(),30.into(),40.into()]));
        epw.insert("nstemp".into(),3.into());epw.insert("broyden_beta".into(),(-0.3).into());
        epw.insert("gap_edge".into(),0.01.into());epw.insert("tc_linear".into(),false.into());
    }
    std::fs::write(&path,toml::to_string(&value).unwrap()).unwrap();
    let cfg=config::Config::load(&path).unwrap();
    assert_eq!(cfg.superconductivity.task,config::SuperconductivityTask::Gap);
    assert_eq!(cfg.superconductivity.initial_gap(),10.0);
    for bad in [vec![],vec![0.0],vec![20.001],vec![20.0,20.0],vec![1000.0]] {
        let mut bad_cfg=cfg.clone();bad_cfg.superconductivity.gap_temperatures_k=bad;
        assert!(bad_cfg.validate().is_err());
    }
    let mut arbitrary=cfg.clone();
    arbitrary.superconductivity.gap_temperatures_k=vec![12.34,26.5];
    arbitrary.superconductivity.initial_gap_mev=Some(2.5);
    arbitrary.set_native(&["8.epw-superconductivity","iso","epw","INPUTEPW","nstemp"],serde_json::json!(2)).unwrap();
    arbitrary.validate().unwrap();
    for beta in [0.0,1.01,-1.01,f64::NAN] {let mut bad=cfg.clone();bad.superconductivity.mixing_beta=beta;assert!(bad.validate().is_err());}
    for seed in [0.0,-1.0,f64::NAN] {let mut bad=cfg.clone();bad.superconductivity.initial_gap_mev=Some(seed);assert!(bad.validate().is_err());}
    let old: config::Config = toml::from_str(include_str!("fixtures/input.toml")).unwrap();
    let serialized=serde_json::to_value(&old.superconductivity).unwrap();
    assert!(serialized.get("task").is_none());
    assert!(serialized.get("initial_gap_mev").is_none());
    let mut bad=cfg;bad.superconductivity.frequency_cutoff_ev=1e-8;
    assert!(bad.validate().is_err());
    std::fs::remove_dir_all(root).unwrap();
}
