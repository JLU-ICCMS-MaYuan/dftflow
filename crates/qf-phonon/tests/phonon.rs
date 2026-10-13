//! qf-phonon 集成测试：锁定 split 生成、merge 文件归集、archive、表格的关键行为。

use std::fs;
use std::path::Path;

fn temp_workspace(tag: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!("qf_phonon_test_{}_{}", tag, std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    base
}

fn write_input_toml(dir: &Path) -> std::path::PathBuf {
    // 伪赝势目录：generate_split_inputs 写 scf.in 需要 pp_dir 与元素匹配文件。
    let pp = dir.join("pp");
    fs::create_dir_all(&pp).unwrap();
    fs::write(pp.join("Si.UPF"), "dummy").unwrap();
    let pp_abs = pp.to_string_lossy().into_owned();
    let p = dir.join("input.toml");
    fs::write(
        &p,
        format!(
            "pp_dir = \"{pp_abs}\"\n[qe_phonon]\nwork_dir = \".\"\n[qe_phonon.scf]\nprefix = \"Si2\"\n[qe_phonon.ph]\nqpoints = [2, 2, 1]\n"
        ),
    )
    .unwrap();
    p
}

#[test]
fn split_generates_q_dirs_and_index_map() {
    let dir = temp_workspace("split");
    let toml = write_input_toml(&dir);
    let cfg = qf_config::load(Some(toml.to_str().unwrap()), Some(toml.to_str().unwrap())).unwrap();
    let poscar = dir.join("POSCAR");
    fs::write(
        &poscar,
        "Si2\n1.0\n4.0 0.0 0.0\n0.0 4.0 0.0\n0.0 0.0 4.0\nSi\n2\nDirect\n0.0 0.0 0.0\n0.5 0.5 0.5\n",
    )
    .unwrap();
    let structure = qf_crystal::parse_poscar(poscar.to_str().unwrap()).unwrap();
    let entries =
        qf_phonon::generate_split_inputs(dir.to_str().unwrap(), &cfg, "Si2", "split_assign_index", &structure)
            .unwrap();
    assert_eq!(entries.len(), 4, "2x2x1 网格应生成 4 个 q 点");
    for entry in &entries {
        let q_dir = dir.join(&entry.directory);
        assert!(q_dir.join("split_ph.in").exists(), "缺少 {}/split_ph.in", entry.directory);
        assert!(q_dir.join("scf.in").exists(), "缺少 {}/scf.in", entry.directory);
        assert!(q_dir.join("run_split.sh").exists(), "缺少 {}/run_split.sh", entry.directory);
    }
    assert!(dir.join("q_index_map.json").exists());
}

#[test]
fn dos_tables_sum_elements() {
    let dir = temp_workspace("dos");
    let phdos = dir.join("phdos");
    fs::create_dir_all(&phdos).unwrap();
    fs::write(
        phdos.join("Si2.phdos.dat"),
        "# EFermi = 0.0 eV\n-5.0 0.1 0.05 0.05\n0.0 0.5 0.25 0.25\n5.0 0.2 0.1 0.1\n10.0 0.0 0.0 0.0\n",
    )
    .unwrap();
    let elements = vec!["Si".to_string()];
    let counts = vec![2usize];
    let (atoms_rows, elements_rows, element_names) =
        qf_phonon::build_phonon_dos_tables(dir.to_str().unwrap(), "Si2", &elements, &counts).unwrap();
    assert_eq!(element_names, vec!["Si".to_string()]);
    // 表头 + 4 数据行
    assert_eq!(atoms_rows.len(), 5);
    // 第一行元素分波列应等于两个 Si 原子之和 0.10
    let si_val: f64 = elements_rows[1][2].parse().unwrap();
    assert!((si_val - 0.10).abs() < 1e-9, "元素 Si 分波应累加两个原子: {si_val}");
}

#[test]
fn band_tables_match_label_points() {
    let dir = temp_workspace("band");
    let phband = dir.join("phband");
    fs::create_dir_all(&phband).unwrap();
    fs::write(
        phband.join("Si2_band.labelinfo.dat"),
        "GAMMA 1 0.0 0.0 0.0 0.0\nX 2 0.5 0.5 0.0 0.0\nW 3 0.75 0.25 0.0 0.0\nL 4 1.0 0.0 0.0 0.0\n",
    )
    .unwrap();
    fs::write(
        phband.join("Si2.phband.freq"),
        "1.0 2.0 3.0 4.0 5.0 6.0\n1.1 2.1 3.1 4.1 5.1 6.1\n1.2 2.2 3.2 4.2 5.2 6.2\n1.3 2.3 3.3 4.3 5.3 6.3\n",
    )
    .unwrap();
    let elements = vec!["Si".to_string()];
    let counts = vec![2usize];
    let (band_rows, _labels) =
        qf_phonon::build_phonon_band_tables(dir.to_str().unwrap(), "Si2", &elements, &counts).unwrap();
    // 表头 + 4 个路径点
    assert_eq!(band_rows.len(), 5);
    assert_eq!(band_rows[0].len(), 5 + 6, "应含 5 坐标列 + 6 模式列");
}
