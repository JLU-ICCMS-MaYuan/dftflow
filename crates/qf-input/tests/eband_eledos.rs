//! eband / eledos 输入卡写出集成测试：锁定与 Python `input_deck/bands.py`、`dos.py` 对齐的关键产物。
//! 用临时工作目录构造 POSCAR + 伪势文件 + input.toml，断言生成的输入卡含预期关键字。

use std::fs;
use std::path::Path;

const POSCAR: &str = "Si
5.43
0.0 0.5 0.5
0.5 0.0 0.5
0.5 0.5 0.0
Si
2
direct
0.0 0.0 0.0
0.25 0.25 0.25
";

const KPATH: &str = "0.0 0.0 0.0 G
0.5 0.0 0.5 X
0.5 0.0 0.5 X
0.5 0.25 0.75 W
";

fn temp_workspace() -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!("qf_input_eband_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    base
}

#[test]
fn eband_inputs_match_python_format() {
    let dir = temp_workspace();
    let poscar = dir.join("POSCAR");
    fs::write(&poscar, POSCAR).unwrap();
    let pp = dir.join("pp");
    fs::create_dir_all(&pp).unwrap();
    fs::write(pp.join("Si.UPF"), "dummy Si pseudo\n").unwrap();
    let kpath = dir.join("KPATH.in");
    fs::write(&kpath, KPATH).unwrap();

    let input_toml = dir.join("input.toml");
    fs::write(
        &input_toml,
        format!("pp_dir = \"{}\"\n\n[qeledos_noop]\n", pp.display()),
    )
    .unwrap();

    let cfg = qf_config::load(Some(input_toml.to_str().unwrap()), Some(input_toml.to_str().unwrap()))
        .expect("load config");
    let structure = qf_crystal::parse_poscar(poscar.to_str().unwrap()).expect("parse poscar");
    let prefix = structure.formula.clone();

    let work = dir.join("qe_eband");
    fs::create_dir_all(&work).unwrap();
    // KPATH.in 需要在 work 目录内（与 CLI 行为一致）
    fs::write(work.join("KPATH.in"), KPATH).unwrap();

    let written = qf_input::electronic::write_eband_inputs(work.to_str().unwrap(), &cfg, &structure, &prefix)
        .expect("write eband inputs");
    assert!(written.len() == 5, "eband 应写出 5 个文件，实际 {written:?}");

    let eband = fs::read_to_string(work.join("eband.in")).unwrap();
    assert!(eband.contains("calculation = 'bands'"), "eband.in 缺 calculation=bands");
    assert!(eband.contains("K_POINTS crystal_b"), "eband.in 缺 K_POINTS crystal_b");
    assert!(eband.contains("occupations = 'tetrahedra'"), "eband.in 缺 occupations");

    let banddata = fs::read_to_string(work.join("elebanddata.in")).unwrap();
    assert!(banddata.contains("&BANDS"), "elebanddata.in 缺 &BANDS");
    assert!(banddata.contains("lp = .true."), "elebanddata.in 缺 lp");

    let projdata = fs::read_to_string(work.join("elebandprojdata.in")).unwrap();
    assert!(projdata.contains("&projwfc"), "elebandprojdata.in 缺 &projwfc");

    assert!(
        work.join(format!("{prefix}_band.labelinfo.dat")).exists(),
        "缺少 band.labelinfo.dat"
    );
    assert!(
        work.join(format!("{prefix}_gun_band.labelinfo.dat")).exists(),
        "缺少 gun_band.labelinfo.dat"
    );
}

#[test]
fn eledos_inputs_match_python_format() {
    let dir = temp_workspace();
    let poscar = dir.join("POSCAR");
    fs::write(&poscar, POSCAR).unwrap();
    let pp = dir.join("pp");
    fs::create_dir_all(&pp).unwrap();
    fs::write(pp.join("Si.UPF"), "dummy Si pseudo\n").unwrap();

    let input_toml = dir.join("input.toml");
    fs::write(
        &input_toml,
        format!("pp_dir = \"{}\"\n\n[qeledos_noop]\n", pp.display()),
    )
    .unwrap();

    let cfg = qf_config::load(Some(input_toml.to_str().unwrap()), Some(input_toml.to_str().unwrap()))
        .expect("load config");
    let structure = qf_crystal::parse_poscar(poscar.to_str().unwrap()).expect("parse poscar");
    let prefix = structure.formula.clone();

    let work = dir.join("qe_eledos");
    let written = qf_input::electronic::write_eledos_inputs(work.to_str().unwrap(), &cfg, &prefix)
        .expect("write eledos inputs");
    assert!(written.len() == 2, "eledos 应写出 2 个文件，实际 {written:?}");

    let eletdos = fs::read_to_string(work.join("eletdos.in")).unwrap();
    assert!(eletdos.contains("&dos"), "eletdos.in 缺 &dos");
    assert!(eletdos.contains("fildos"), "eletdos.in 缺 fildos");
    assert!(eletdos.contains("DeltaE = 0.01"), "eletdos.in 缺 DeltaE 默认");

    let elepdos = fs::read_to_string(work.join("elepdos.in")).unwrap();
    assert!(elepdos.contains("&projwfc"), "elepdos.in 缺 &projwfc");
    assert!(elepdos.contains("filpdos"), "elepdos.in 缺 filpdos");
    let _ = Path::new(&poscar);
}
