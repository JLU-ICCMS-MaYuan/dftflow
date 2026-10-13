//! 解析层黄金基线集成测试（CI 可跑，不依赖 QE/VASP 二进制）。
//!
//! 对齐 Python `qeflow/io_reader` 的解析契约：用提交的样例 QE 输出 fixture 断言关键字段，
//! 作为后续逐 flow 对照 `test_auto/MgB2/` pytest 的轻量前置门禁。

use qf_io::{parse_dyn0_qpoints, parse_pw_output, parse_tdos};
use std::path::Path;

fn fixture(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(p).expect("fixture missing")
}

#[test]
fn scf_golden_fields() {
    let r = parse_pw_output(&fixture("sample_scf.out"));
    assert!(r.finished);
    assert!(!r.has_error);
    assert_eq!(r.total_energy_ry, Some(-123.456));
    assert_eq!(r.number_of_atoms, Some(2));
    assert_eq!(r.number_of_kpoints, Some(100));
    assert_eq!(r.fermi_energy_ev, Some(12.34));
    let cell = r.cell_angstrom.expect("cell");
    assert_eq!(cell[2], [0.0, 0.0, 3.5]);
}

#[test]
fn dyn0_golden_qpoints() {
    let qs = parse_dyn0_qpoints(&fixture("sample_dyn0.txt")).unwrap();
    assert_eq!(qs.len(), 3);
    assert_eq!(qs[0].q_crystal, [0.0, 0.0, 0.0]);
    assert_eq!(qs[1].index, 2);
}

#[test]
fn tdos_golden() {
    let t = parse_tdos(&fixture("sample_tdos.dat")).unwrap();
    assert_eq!(t.fermi, Some(5.0));
    assert_eq!(t.energies, vec![-1.0, 0.0, 1.0]);
    assert_eq!(t.tdos, vec![0.1, 0.5, 0.2]);
}
