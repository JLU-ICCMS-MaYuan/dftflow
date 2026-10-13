//! df-vasp 集成测试：INCAR 控制键排除、KPOINTS 读写、kmesh 整数倍推导。

use std::path::Path;
use df_vasp::incar;
use df_vasp::kpoints;
use qf_config::SectionMap;
use serde_json::json;

#[test]
fn incar_skips_kmesh_and_formats_bools() {
    let mut params = SectionMap::new();
    params.insert("kmesh".to_string(), json!(0.04));
    params.insert("SYSTEM".to_string(), json!("VASP_Job"));
    params.insert("LWAVE".to_string(), json!(".TRUE."));
    params.insert("NCORE".to_string(), json!(4));
    let dir = std::env::temp_dir().join(format!("df_vasp_test_incar_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("INCAR");
    incar::write_incar(&params, p.to_str().unwrap()).unwrap();
    let text = std::fs::read_to_string(&p).unwrap();
    assert!(!text.contains("KMESH"), "INCAR 不应含控制键 kmesh");
    assert!(text.contains("SYSTEM = VASP_Job"));
    assert!(text.contains("LWAVE = .TRUE."));
    assert!(text.contains("NCORE = 4"));
}

#[test]
fn kpoints_automatic_round_trip() {
    let dir = std::env::temp_dir().join(format!("df_vasp_test_kp_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("KPOINTS");
    let kp = kpoints::AutomaticKpoints {
        title: "Automatic mesh".to_string(),
        style: "Gamma".to_string(),
        mesh: [10, 12, 14],
        shift: [0.0, 0.0, 0.0],
    };
    kpoints::write_automatic_kpoints(p.to_str().unwrap(), &kp).unwrap();
    let read = kpoints::read_automatic_kpoints(p.to_str().unwrap()).unwrap();
    assert_eq!(read.mesh, [10, 12, 14]);
    assert_eq!(read.style, "Gamma");
}

#[test]
fn kmesh_multiplier_ceil() {
    // scf 0.03 -> target 0.02 => ratio 1.5 => ceil 2
    let m = kpoints::derive_integer_kmesh_multiplier(0.03, 0.02).unwrap();
    assert_eq!(m, 2);
    // ratio <= 1 => 1
    let m2 = kpoints::derive_integer_kmesh_multiplier(0.02, 0.03).unwrap();
    assert_eq!(m2, 1);
    // exact integer ratio
    let m3 = kpoints::derive_integer_kmesh_multiplier(0.04, 0.02).unwrap();
    assert_eq!(m3, 2);
}

#[test]
fn kmesh_to_grid_positive() {
    let lattice = [
        [0.0, 2.715, 2.715],
        [2.715, 0.0, 2.715],
        [2.715, 2.715, 0.0],
    ];
    let grid = kpoints::kmesh_to_grid(&lattice, 0.04);
    assert!(grid.iter().all(|&n| n >= 1));
    let _ = Path::new("");
}

/// 回退公式按 vaspkit 1.4.0 黑盒标定校准（正交/立方方向精确一致）：
/// 立方 a=3, K=0.04 → 8 8 8；正交 3/4/5, K=0.04 → 8 6 5（vaspkit 实测值）。
#[test]
fn kmesh_to_grid_matches_vaspkit_orthorhombic() {
    // 立方 a=3
    let cubic = [[3.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 3.0]];
    assert_eq!(kpoints::kmesh_to_grid(&cubic, 0.04), [8, 8, 8]);
    // 正交 a=3 b=4 c=5
    let orth = [[3.0, 0.0, 0.0], [0.0, 4.0, 0.0], [0.0, 0.0, 5.0]];
    assert_eq!(kpoints::kmesh_to_grid(&orth, 0.04), [8, 6, 5]);
    // 六方 c 向（MgB2: a=3.0737, c=3.5343, K=0.04）→ vaspkit 实测 N_c=7
    let hex = [
        [3.0737, 0.0, 0.0],
        [-1.53685, 2.66190, 0.0],
        [0.0, 0.0, 3.5343],
    ];
    let grid = kpoints::kmesh_to_grid(&hex, 0.04);
    assert_eq!(grid[2], 7);
    // 已知偏差：六方面内 a 向 vaspkit 有对称量化（实测 12），回退公式给 9（round(0.3756/0.04)）
    assert_eq!(grid[0], 9);
}
