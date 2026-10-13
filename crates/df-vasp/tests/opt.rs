//! df-vasp vasp_opt rv4 四步生成与 lobsterin 生成测试。

use qf_config::SectionMap;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn poscar_path(dir: &std::path::Path) -> PathBuf {
    dir.join("POSCAR")
}

fn write_temp_poscar(dir: &std::path::Path) {
    let poscar = "\
Si bulk
   1.0
   2.0   0.0   0.0
   0.0   2.0   0.0
   0.0   0.0   2.0
Si
2
Direct
   0.0   0.0   0.0
   0.5   0.5   0.5
";
    std::fs::write(poscar_path(dir), poscar).unwrap();
}

fn read(path: &std::path::Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

fn base_vopt() -> SectionMap {
    let mut m = BTreeMap::new();
    m.insert("PSTRESS".to_string(), serde_json::json!(0.0));
    m.insert("kmesh".to_string(), serde_json::json!(0.025));
    m.into_iter().collect()
}

fn run_rv4(dir: &std::path::Path, work_sub: &str, pressure: Option<f64>) -> PathBuf {
    let work_dir = dir.join(work_sub);
    let args = df_vasp::opt::VaspOptArgs {
        work_dir: work_dir.to_string_lossy().into_owned(),
        struct_file: Some(poscar_path(dir).to_string_lossy().into_owned()),
        mode: "rv4".to_string(),
        pressure_gpa: pressure,
        vasp_execmd: "vasp_std".to_string(),
        potcar_dir: String::new(),
        slurm_header: String::new(),
    };
    let script = df_vasp::opt::run_vasp_opt(&base_vopt(), &SectionMap::new(), &args).unwrap();
    assert_eq!(script, "fouropt.sh");
    work_dir
}

#[test]
fn vasp_opt_rv4_generates_four_incars_and_script() {
    let dir = std::env::temp_dir().join(format!("df_vasp_opt_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    write_temp_poscar(&dir);

    let wd = run_rv4(&dir, "vasp_opt", None);

    let incar1 = read(&wd.join("INCAR_1"));
    // stage1: POTIM 应位于 ISIF 与 LWAVE 之间
    let pos_isif = incar1.find("ISIF").unwrap();
    let pos_potim = incar1.find("POTIM").unwrap();
    let pos_lwave = incar1.find("LWAVE").unwrap();
    assert!(pos_isif < pos_potim && pos_potim < pos_lwave);

    let incar4 = read(&wd.join("INCAR_4"));
    // 终态 INCAR: KSPACING 由 kmesh 推导；PSTRESS 来自配置；NCORE=1
    assert!(incar4.contains("KSPACING  = 0.15707963000000003"));
    assert!(incar4.contains("PSTRESS   = 0"));
    assert!(incar4.contains("NCORE     = 1"));

    let fouropt = read(&wd.join("fouropt.sh"));
    assert!(fouropt.contains("for i in {1..4}; do"));
    assert!(fouropt.contains("cp -f INCAR_$i INCAR"));
    assert!(fouropt.contains("vasp_std > vasp.log_$i 2>&1"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn vasp_opt_pressure_converts_gpa_to_kbar() {
    let dir = std::env::temp_dir().join(format!("df_vasp_opt_p_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    write_temp_poscar(&dir);

    let wd = run_rv4(&dir, "10/vasp_opt", Some(10.0));
    let incar4 = read(&wd.join("INCAR_4"));
    assert!(incar4.contains("PSTRESS   = 100"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn lobsterin_writes_input_and_script() {
    let dir = std::env::temp_dir().join(format!("df_vasp_lob_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // lobsterin 工作目录为相对 CWD，需在目标目录内执行
    std::env::set_current_dir(&dir).unwrap();

    let args = df_vasp::lobsterin::LobsterinArgs {
        cohp_start: -10.0,
        cohp_end: 5.0,
        species_custom: ["Si".to_string(), "O".to_string()],
        zvalances: vec!["Si:3s,3p".to_string(), "O:2s,2p".to_string()],
        d_limit: [1.0, 2.5],
        orbital_wise: false,
        k_space_cohp: false,
        mode: 5,
        custom_basis: vec![],
        lobster_execmd: String::new(),
        slurm_header: String::new(),
    };
    let wd = df_vasp::lobsterin::run_lobsterin(&args).unwrap();
    assert_eq!(wd, "Si_O_1.0_2.5");
    let lob = read(&std::path::Path::new("Si_O_1.0_2.5/lobsterin"));
    assert!(lob.contains("COHPstartEnergy  -10"));
    assert!(lob.contains("usebasisset pbeVaspFit2015"));
    assert!(lob.contains("basisfunctions Si 3s 3p"));
    assert!(lob.contains("basisfunctions O 2s 2p"));
    assert!(lob.contains("cohpGenerator from 1 to 2.5 type Si type O"));
    let run = read(&std::path::Path::new("Si_O_1.0_2.5/run_lobster.sh"));
    assert!(run.contains("echo 'Job started at' `date`"));
    std::fs::remove_dir_all(&dir).ok();
}
