//! F2 qe_data 行为测试：FR-001~FR-012。
//! 使用隔离临时目录与可执行 fixture，不触碰真实队列与 docs/local 之外数据。

#[path="support/native_fixture.rs"]
mod native_fixture;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

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


fn write_script(path: &PathBuf, body: &str) {
    std::fs::write(path, body).expect("Write fixture Failed");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .expect("chmod fixture Failed");
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
) -> String {
    let text=format!(
        r#"work_dir = "{work}"
structure = "{structure}"
schema_version = 2
outdir = "{out}"

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
prefix = "actual_prefix"

ecutwfc = 30.0
ecutrho = 240.0
occupations = "fixed"

conv_thr = 1e-8

[stages."2.qe-nscf"]
K_POINTS = [2, 2, 2]
ATOMIC_SPECIES = "Si 28.0855 {structure}"

calculation = "nscf"
prefix = "actual_prefix"

ecutwfc = 30.0
ecutrho = 240.0
occupations = "fixed"
nbnd = 4
nosym = false
noinv = false

conv_thr = 1e-8

[workflow."3.qe-bands"]
points_per_edge = 20

[workflow."4.qe-phonon-{mode}"]
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
prefix = "actual_prefix"

ecutwfc = 30.0
ecutrho = 240.0
occupations = "fixed"
nbnd = 4
nosym = true
noinv = true

conv_thr = 1e-8

[stages."4.qe-phonon-{mode}"]
prefix = "actual_prefix"
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
prefix = "actual_prefix"
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
# dos_energy_step、dos_smr_fixed_en_width：eV。
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
prefix = "actual_prefix"
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
prefix = "actual_prefix"
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
precision_k = 0.5
max_solves = 20
eigen_margin = 1e-6
normal_gap_mev = 0.1
super_gap_mev = 2.0

[stages."3.qe-bands-post"]
prefix = "actual_prefix"
lsym = false

[stages."3.qe-dos"]
# degauss：Ry；Emin/Emax/DeltaE：eV。
prefix = "actual_prefix"
ngauss = 0
degauss = 0.007349864435130998
Emin = -100.0
Emax = 100.0
DeltaE = 0.01

[stages."3.qe-pdos"]
# degauss：Ry；Emin/Emax/DeltaE：eV。
prefix = "actual_prefix"
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

[stages."4.qe-phonon-{mode}-q"]
prefix = "actual_prefix"
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
prefix = "actual_prefix"
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
prefix = "actual_prefix"
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
nsiter = 50
conv_thr_iaxis = 0.0001
broyden_beta = 0.7
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
prefix = "actual_prefix"
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
nsiter = 50
conv_thr_iaxis = 0.0001
broyden_beta = 0.7
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
        out = out.display(),
        structure = structure.display(),
        pw = pw.display(),
        ph = ph.display(),
        epw = epw.display(),
        mode = mode,
    );
    let mut document: toml::Value = toml::from_str(&text).unwrap();
    if mode == "nosplit" {
        document["stages"].as_table_mut().unwrap().remove("4.qe-phonon-nosplit-q");
    }
    let mut cfg: config::Config = document.try_into().unwrap();
    native_fixture::phonon(&mut cfg,serde_json::from_value(serde_json::json!(mode)).unwrap());
    toml::to_string_pretty(&cfg).unwrap()
}

/// 构造一个可运行的 F2 Case：临时目录、fake pw/ph/epw、POSCAR、配置。
struct Case {
    root: PathBuf,
    config_path: PathBuf,
}

fn make_case(mode: &str) -> Case {
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
    let config_path = root.join("config.toml");
    let cfg_text = base_config(&work, &out, &structure, &pw, &ph, &_epw, mode);
    std::fs::write(&config_path, cfg_text).unwrap();

    Case { root, config_path }
}

fn try_pipeline(case: &Case) -> anyhow::Result<QeManifest> {
    let cfg = config::Config::load(&case.config_path).expect("Configure Load Failed");
    let binding = snapshot::prepare(&cfg, &case.config_path).expect("prepare Failed");
    let qe = QeSettings::from_config(&cfg).expect("QeSettings Failed");
    scheduler::run(&binding, &cfg, &qe, Backend::Local, &[], &cfg.work_dir)
}

fn run_pipeline(case: &Case) -> QeManifest {
    try_pipeline(case).expect("scheduler Failed")
}

#[test]
fn native_qe_rejects_incomplete_and_mismatched_outputs() {
    for failure in [
        "energy",
        "duplicate_k",
        "wrong_q",
        "truncated_dvscf",
    ] {
        let case = make_case("index");
        std::fs::write(case.root.join("fixture-failure"), failure).unwrap();
        assert!(try_pipeline(&case).is_err(), "{failure} It should be rejected");
        assert!(
            !case.root.join("work/qe-data/manifest.json").exists(),
            "{failure} Can not publish successful manifest"
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
    assert_eq!(inodes.len(), paths.len(), "Each stage must be independently physically copied");
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
        "The unit matrix cannot be used for non-positive conversion"
    );
    let pattern = std::fs::read_to_string(&m.phonon_files[1].patterns).unwrap();
    assert!(pattern.contains("<QPOINT_NUMBER>2</QPOINT_NUMBER>"));
    let original =
        std::fs::read_to_string(work.join("ph/02/out/_ph0/actual_prefix.phsave/patterns.1.xml"))
            .unwrap();
    assert!(original.contains("<QPOINT_NUMBER>1</QPOINT_NUMBER>"));
    m.verify().unwrap();
    let mut unsafe_manifest = m.clone();
    let mut invalid = m.artifacts[0].clone();
    invalid.role = "unchecked-outside".into();
    invalid.relative_path = "../wfc1.dat".into();
    unsafe_manifest.artifacts.push(invalid);
    assert!(unsafe_manifest.verify().is_err(), "Excludes data validation and does not cancel the common path boundary");
    let victim = &m.states.as_ref().unwrap().wavefunctions.join("wfc1.dat");
    std::fs::write(victim, "tampered").unwrap();
    m.verify().unwrap();
    reuse::import_from_run(&work, &case.root.join("unchecked-import")).unwrap();
    assert_eq!(std::fs::read(case.root.join("unchecked-import/imported/qe-data/nscf/out/actual_prefix.save/wfc1.dat")).unwrap(), b"tampered");
}

// ---------- FR-001/003~010：三种 PH 模式完整 qe-data 闭环 ----------

#[test]
fn qe_data_index_mode_full_pipeline() {
    let case = make_case("index");
    let m = run_pipeline(&case);
    assert!(
        m.validation.all_passed(),
        "index Mode validation not passed: {:?}",
        m.validation.status
    );
    assert_eq!(m.qpoints.len(), 2);
    assert_eq!(m.phonon_files.len(), 2);
    assert!(m.scf.is_some());
    assert!(m.states.is_some());
    let manifest_path = case.root.join("work").join("qe-data").join("manifest.json");
    assert!(manifest_path.exists(), "manifest not published");
}

#[test]
fn qe_data_coords_mode_full_pipeline() {
    let case = make_case("coords");
    let m = run_pipeline(&case);
    assert!(
        m.validation.all_passed(),
        "coords Mode validation not passed: {:?}",
        m.validation.status
    );
    assert_eq!(m.qpoints.len(), 2);
    assert_eq!(m.phonon_files.len(), 2);
    let root = case.root.join("work/ph/02/out/_ph0");
    assert!(root.join("actual_prefix.dvscf1").is_file());
    assert!(!root.join("actual_prefix.q_1").exists());
}

#[test]
fn qe_data_nosplit_mode_full_pipeline() {
    let case = make_case("nosplit");
    let m = run_pipeline(&case);
    assert!(
        m.validation.all_passed(),
        "nosplit Mode validation not passed: {:?}",
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
    let reference_in = qe.path_input(out.to_str().unwrap(), &[[0., 0., 0.]]).unwrap();
    assert!(reference_in.contains("calculation = 'bands'"));
    for deck in [&scf_in, &nscf_in, &reference_in] {
        assert!(deck.starts_with("&CONTROL\n"), "{deck}");
        assert!(!deck.contains("Parameter Source"), "{deck}");
        for pseudo in cfg.pseudos.values() {
            assert!(deck.contains(&format!("'{}'", pseudo.parent().unwrap().display())));
            assert!(deck.contains(pseudo.file_name().unwrap().to_str().unwrap()));
        }
    }
    let init_in = qe.phonon_init_input(out.to_str().unwrap()).unwrap();
    assert!(init_in.contains("only_init = .true.") && init_in.contains("ldisp = .true."));
    let q_in = qe
        .phonon_q_input(out.to_str().unwrap(), &config::PhMode::Index, None, 1)
        .unwrap();
    assert!(q_in.contains("start_q = 1") && q_in.contains("last_q = 1"));
    for deck in [&init_in, &q_in] {
        assert!(!deck.contains("Parameter Source"), "{deck}");
    }
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
#[ignore = "Yes EPWFLOW_REAL_QE_ROOT Pointed to already LDA Proof. Visible performance of read-only tests"]
fn actual_lda_native_layout() {
    use epwflow::qe_data::{manifest::PhResponse, native_ph, xml};
    let root = PathBuf::from(std::env::var("EPWFLOW_REAL_QE_ROOT").expect("Lack of physical evidence base directory"));
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
    assert_eq!(before, after, "Read-and-certify can't change the source");
    let report = serde_json::json!({"qpoints":qs.len(),"modes_per_q":9,"nks":states.kpoints.len(),"bands":states.nbnd,"native_response_validation":true,"source_unchanged":true,"sources":paths.iter().zip(before).map(|(p,h)|serde_json::json!({"path":p,"sha256":h})).collect::<Vec<_>>()});
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    if let Ok(path) = std::env::var("EPWFLOW_REAL_CHECK_REPORT") {
        store::write(&PathBuf::from(path), &report).unwrap();
    }
}

fn shared_case() -> (Case, config::Config, snapshot::RunBinding) {
    shared_case_with_args(false)
}

fn shared_case_with_args(explicit_args: bool) -> (Case, config::Config, snapshot::RunBinding) {
    let case = make_case("nosplit");
    std::fs::write(case.root.join("KPATH.in"), "0 0 0 G\n0.5 0 0 X\n").unwrap();
    for name in ["q2r", "matdyn"] {
        make_script(&format!("{name}.x"), &case.root, &format!("#!/bin/sh\nexec /usr/bin/python3 '{}' {name} \"$@\"\n", case.root.join("qe-native.py").display()));
    }
    let mut cfg = config::Config::load(&case.config_path).unwrap();
    cfg.programs.q2r = Some(config::Program { path: case.root.join("q2r.x"), args: vec![] });
    cfg.programs.matdyn = Some(config::Program { path: case.root.join("matdyn.x"), args: vec![] });
    if explicit_args {
        for name in ["q2r", "matdyn"] {
            let args = vec![format!("--{name}-only"), "value with spaces".into()];
            make_script(&format!("{name}.x"), &case.root, &format!("#!/bin/sh\n[ \"$1\" = '--{name}-only' ] && [ \"$2\" = 'value with spaces' ] || exit 81\nprintf '%s\\n' '{name}' >> '{}'\nshift 2\nexec /usr/bin/python3 '{}' {name} \"$@\"\n", case.root.join("explicit-post-args").display(), case.root.join("qe-native.py").display()));
            if name == "q2r" { cfg.programs.q2r.as_mut().unwrap().args = args; }
            else { cfg.programs.matdyn.as_mut().unwrap().args = args; }
        }
    }
    cfg.atomic_masses.insert("Si".into(), 29.1234);
    cfg.qe.scf.k_shift = [0; 3];
    native_fixture::matrix(&mut cfg);
    let binding = snapshot::prepare(&cfg, &case.config_path).unwrap();
    (case, cfg, binding)
}

#[test]
fn shared_qeflow_nosplit_preserves_mass_zero_shift_checkpoint_and_complete_delivery() {
    let (case, cfg, binding) = shared_case();
    let qe = QeSettings::from_config(&cfg).unwrap();
    let m = scheduler::run_qeflow_registered(&binding, &cfg, &qe, Backend::Local, &[], &cfg.work_dir).unwrap();
    m.verify().unwrap();
    assert!(m.validation.all_passed());
    assert_eq!(m.states.unwrap().kpoints.len(), 8);
    let calls = std::fs::read_to_string(case.root.join("scientific-calls")).unwrap();
    assert_eq!(calls, "pw\nph\nq2r\nmatdyn\nmatdyn\npw\n");
    let shared = cfg.work_dir.join("qe-phonon");
    let input = std::fs::read_to_string(shared.join("scf.in")).unwrap();
    assert!(input.contains("29.1234"));
    assert!(input.contains("2 2 2 0 0 0"));
    assert!(!cfg.work_dir.join("scf/snapshot/actual_prefix.save/stage-local-marker").exists());
    assert!(shared.join("out/actual_prefix.save/stage-local-marker").is_file());
    assert!(shared.join("save/manifest.json").is_file());
    assert!(shared.join("phonon_combined.svg").is_file());
    assert!(m.artifacts.iter().any(|a| a.role == "qeflow_shared-source-identities.json"));
    std::fs::remove_file(case.root.join("KPATH.in")).unwrap();
    binding.verify().unwrap();
    let fixed = config::Config::load(&cfg.work_dir.join("input-init.toml")).unwrap();
    assert_eq!(fixed.kpath, Some(cfg.work_dir.join("KPATH-init")));
}

#[test]
fn shared_qeflow_failures_stop_before_nscf_and_never_publish() {
    for failure in ["energy", "truncated_dvscf"] {
        let (case, cfg, binding) = shared_case();
        std::fs::write(case.root.join("fixture-failure"), failure).unwrap();
        let qe = QeSettings::from_config(&cfg).unwrap();
        assert!(scheduler::run_qeflow_registered(&binding, &cfg, &qe, Backend::Local, &[], &cfg.work_dir).is_err());
        assert!(!cfg.work_dir.join("nscf").exists());
        assert!(!cfg.work_dir.join("qe-data/manifest.json").exists());
        let calls = std::fs::read_to_string(case.root.join("scientific-calls")).unwrap();
        assert_eq!(calls.lines().filter(|s| *s == "pw").count(), 1);
    }
}

#[test]
fn shared_qeflow_resume_reuses_scf_ph_and_rejects_unregistered_changes() {
    let (case, cfg, binding) = shared_case();
    let qe = QeSettings::from_config(&cfg).unwrap();
    std::fs::write(case.root.join("post-failure"), "fail once").unwrap();
    assert!(scheduler::run_qeflow_registered(&binding, &cfg, &qe, Backend::Local, &[], &cfg.work_dir).is_err());
    let checkpoint = store::Identity::capture(&cfg.work_dir.join("qe-phonon/scf-checkpoint.json")).unwrap();
    let calls = std::fs::read_to_string(case.root.join("scientific-calls")).unwrap();
    assert_eq!(calls, "pw\nph\nq2r\n");
    std::fs::remove_file(case.root.join("post-failure")).unwrap();
    scheduler::run_qeflow_registered(&binding, &cfg, &qe, Backend::Local, &[], &cfg.work_dir).unwrap();
    checkpoint.verify().unwrap();
    let calls = std::fs::read_to_string(case.root.join("scientific-calls")).unwrap();
    assert_eq!(calls, "pw\nph\nq2r\nq2r\nmatdyn\nmatdyn\npw\n");

    let (case, cfg, binding) = shared_case();
    let qe = QeSettings::from_config(&cfg).unwrap();
    std::fs::write(case.root.join("post-failure"), "fail").unwrap();
    assert!(scheduler::run_qeflow_registered(&binding, &cfg, &qe, Backend::Local, &[], &cfg.work_dir).is_err());
    let calls = std::fs::read_to_string(case.root.join("scientific-calls")).unwrap();
    std::fs::write(cfg.work_dir.join("qe-phonon/bin/ph.x"), "tampered").unwrap();
    assert!(scheduler::run_qeflow_registered(&binding, &cfg, &qe, Backend::Local, &[], &cfg.work_dir).is_err());
    assert_eq!(calls, std::fs::read_to_string(case.root.join("scientific-calls")).unwrap());
}

#[test]
fn shared_qeflow_reuses_xml_q2r_after_matdyn_failure() {
    let (case, cfg, binding) = shared_case();
    let qe = QeSettings::from_config(&cfg).unwrap();
    let failure = case.root.join("comparison-failure");
    std::fs::write(&failure, "matdyn").unwrap();
    assert!(scheduler::run_qeflow_registered(&binding, &cfg, &qe, Backend::Local, &[], &cfg.work_dir).is_err());
    let output = store::Identity::capture(&cfg.work_dir.join("qe-phonon/q2r.out")).unwrap();
    let force = store::Identity::capture(&cfg.work_dir.join("qe-phonon/actual_prefix.fc.xml")).unwrap();
    std::fs::remove_file(failure).unwrap();
    scheduler::run_qeflow_registered(&binding, &cfg, &qe, Backend::Local, &[], &cfg.work_dir).unwrap();
    output.verify().unwrap();
    force.verify().unwrap();
    let calls = std::fs::read_to_string(case.root.join("scientific-calls")).unwrap();
    assert_eq!(calls.lines().filter(|l| *l == "pw").count(), 2); // SCF 与完整 NSCF。
    for stage in ["ph", "q2r"] {
        assert_eq!(calls.lines().filter(|l| *l == stage).count(), 1);
    }
}


#[test]
fn missing_nscf_wavefunction_does_not_trigger_integrity_rejection() {
    let case = make_case("index");
    std::fs::write(case.root.join("fixture-failure"), "missing_wfc").unwrap();
    let manifest = run_pipeline(&case);
    manifest.verify().unwrap();
}

#[test]
fn shared_qeflow_passes_each_explicit_postprocessor_argument_list() {
    let (case, cfg, binding) = shared_case_with_args(true);
    let qe = QeSettings::from_config(&cfg).unwrap();
    scheduler::run_qeflow_registered(&binding, &cfg, &qe, Backend::Local, &[], &cfg.work_dir).unwrap().verify().unwrap();
    assert_eq!(std::fs::read_to_string(case.root.join("explicit-post-args")).unwrap(), "q2r\nmatdyn\nmatdyn\n");
}
