//! eband/eledos 后处理黄金对照：Rust 输出 vs Python 参考实现产物（逐字节/容差）。

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture_dir(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
}

fn workspace_tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "qeflow_golden_{}_{}_{}",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let target = dst.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn process_eband_matches_python_reference() {
    let golden = fixture_dir("golden/eband");
    let work = workspace_tmp("eband");
    copy_dir(&golden, &work);

    let output = Command::new(env!("CARGO_BIN_EXE_qeflow"))
        .args([
            "qe-process-eband",
            "-i",
            "Mg1B2_band.projwfc_up",
            "-b",
            "Mg1B2_band",
            "--cell",
            "eband.in",
        ])
        .current_dir(&work)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "qe-process-eband 失败: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    for name in [
        "Mg1B2_eleband_proj.dat",
        "Mg1B2_eleband_proj_sum.dat",
        "Mg1B2_band.dat",
    ] {
        let report = qf_compare::compare_text_exact(&golden.join(name), &work.join(name)).unwrap();
        report.expect_ok(name).unwrap();
    }
    let _ = std::fs::remove_dir_all(&work);
}

#[test]
fn process_eledos_matches_python_reference() {
    let golden = fixture_dir("golden/eledos");
    let work = workspace_tmp("eledos");
    copy_dir(&golden, &work);

    let output = Command::new(env!("CARGO_BIN_EXE_qeflow"))
        .args(["qe-process-el-edos", "-i", "."])
        .current_dir(&work)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "qe-process-el-edos 失败: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    for name in [
        "Mg1B2_dos_proj.dat",
        "Mg1B2_dos_proj_sum.dat",
        "Mg1B2_idos_proj.dat",
        "Mg1B2_idos_proj_sum.dat",
    ] {
        let report = qf_compare::compare_text_exact(&golden.join(name), &work.join(name)).unwrap();
        report.expect_ok(name).unwrap();
    }
    let _ = std::fs::remove_dir_all(&work);
}

#[test]
fn process_eledos_rejects_mismatched_grid() {
    // 真实归档（test_auto/MgB2/MgB2_calc/qe_eledos）的 pdos 与 tdos 网格不一致，
    // Python 参考同样抛错；这里锁定"网格不一致必须报错"的边界。
    let golden = fixture_dir("golden/eledos");
    let work = workspace_tmp("eledos_grid");
    copy_dir(&golden, &work);
    // 构造行数不同的 pdos 网格
    let pdos = work.join("Mg1B2.pdos_atm#1(Mg)_wfc#1(s)");
    let text = std::fs::read_to_string(&pdos).unwrap();
    let truncated: Vec<&str> = text.lines().take(10).collect();
    std::fs::write(&pdos, truncated.join("\n")).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_qeflow"))
        .args(["qe-process-el-edos", "-i", "."])
        .current_dir(&work)
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "能量网格不一致时应失败（对齐 Python）"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("能量网格与 TDOS 不一致"),
        "错误信息应指向网格不一致"
    );
    let _ = std::fs::remove_dir_all(&work);
}
