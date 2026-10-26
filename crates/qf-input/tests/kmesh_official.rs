//! 直接执行仓库内官方脚本，禁止把预期重新实现成另一份生成器。
use std::{path::Path, process::Command};

#[test]
fn weighted_mesh_is_byte_identical_to_official_perl() {
    let script =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../external/wannier90/utility/kmesh.pl");
    for grid in [
        [1, 1, 1],
        [2, 3, 6],
        [6, 6, 6],
        [4, 4, 2],
        [2, 3, 7],
        [11, 3, 5],
    ] {
        let output = Command::new("perl")
            .arg(&script)
            .args(grid.map(|n| n.to_string()))
            .output()
            .expect("需要 Perl 执行官方对照");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            qf_input::pw::uniform_kpoints_text(grid, true)
                .unwrap()
                .as_bytes(),
            output.stdout,
            "{grid:?}"
        );
    }
}

#[test]
fn mesh_rejects_invalid_sizes_before_generating_rows() {
    for grid in [[0, 1, 1], [-1, 1, 1], [i64::MAX, 2, 1], [65536, 65536, 1]] {
        assert!(
            qf_input::pw::uniform_kpoints_text(grid, true).is_err(),
            "{grid:?}"
        );
    }
}
