//! scflow 全链路黄金对照（Python 参考 `python-reference-0004` 产物 vs Rust）：
//! 分展宽 CSV 逐字节一致；result.json 数值按容差（Eliashberg 求解器差异另记）；
//! CIF/POSCAR/PDF 文件形态与存在性。
//!
//! fixture：`scripts/gen_scflow_fixture.py` 生成合成 `*.dyn*.elph*` + `ph.out`
//! （六方 MgB2 型，2 q 点 × 6 模式 × 3 展宽，resolution=60），
//! Python 黄金产物由参考工作树（`git worktree add ../dftflow-py python-reference-0004`
//! 后 `PYTHONPATH=.. python3 -m scflow.main`）生成后检入 `golden/`。

use std::path::{Path, PathBuf};

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/scflow_parity")
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

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    current.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                current.push(c);
            }
        } else if c == '"' {
            in_quotes = true;
        } else if c == ',' {
            fields.push(current.clone());
            current.clear();
        } else {
            current.push(c);
        }
    }
    fields.push(current);
    fields
}

#[test]
fn full_pipeline_matches_python_reference() {
    let golden = fixture_dir().join("golden");
    let work = std::env::temp_dir().join(format!(
        "scflow_parity_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ));
    copy_dir(&fixture_dir().join("work"), &work.join("work"));
    std::fs::copy(fixture_dir().join("input.toml"), work.join("input.toml")).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_scflow"))
        .args(["-c", "input.toml"])
        .current_dir(&work)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "scflow 运行失败: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let results = work.join("results");
    // 1) 文件清单一致（PDF 存在即可，非像素级对照）
    for rel in [
        "MgB2.vasp",
        "MgB2_35.cif",
        "summary_MgB2_s3_mu0,1.csv",
        "0.01/direct_s3_b0,01.csv",
        "0.01/a2f_s3_r60_g5_b0,01.csv",
        "0.01/result.json",

        "0.02/direct_s3_b0,02.csv",
        "0.02/a2f_s3_r60_g5_b0,02.csv",
        "0.02/result.json",
        "0.04/direct_s3_b0,04.csv",
        "0.04/a2f_s3_r60_g5_b0,04.csv",
        "0.04/result.json",
    ] {
        assert!(results.join(rel).is_file(), "缺少产物 {rel}");
        assert!(golden.join(rel).is_file(), "golden 缺少 {rel}");
    }

    // 2) 分展宽 CSV 逐字节一致；summary CSV 豁免 Eliashberg 级联列
    for rel in [
        "0.01/direct_s3_b0,01.csv",
        "0.01/a2f_s3_r60_g5_b0,01.csv",
        "0.02/direct_s3_b0,02.csv",
        "0.02/a2f_s3_r60_g5_b0,02.csv",
        "0.04/direct_s3_b0,04.csv",
        "0.04/a2f_s3_r60_g5_b0,04.csv",
    ] {
        let reference = std::fs::read_to_string(golden.join(rel)).unwrap();
        let new = std::fs::read_to_string(results.join(rel)).unwrap();
        assert_eq!(reference, new, "CSV 不一致: {rel}");
    }
    {
        // summary：Tc E 及其派生列（DeltaC/Tc、Delta、Hc2）由 Eliashberg 求解器
        // 差异决定（Python float32+LAPACK vs Rust f64），其余列逐字节一致。
        let rel = "summary_MgB2_s3_mu0,1.csv";
        let reference = parse_csv_line_first(&std::fs::read_to_string(golden.join(rel)).unwrap());
        let new = parse_csv_line_first(&std::fs::read_to_string(results.join(rel)).unwrap());
        let exempt = [11usize, 14, 15, 16]; // Tc E, DeltaC/Tc, Delta, Hc2
        for (row, (ra, rb)) in reference.iter().zip(&new).enumerate() {
            for (col, (x, y)) in ra.iter().zip(rb).enumerate() {
                if exempt.contains(&col) {
                    continue;
                }
                assert_eq!(x, y, "summary[{row}][{col}] 不一致");
            }
        }
    }

    // 3) POSCAR 逐字节一致
    let reference = std::fs::read_to_string(golden.join("MgB2.vasp")).unwrap();
    let new = std::fs::read_to_string(results.join("MgB2.vasp")).unwrap();
    assert_eq!(reference, new, "MgB2.vasp 不一致");

    // 4) result.json：除 Eliashberg 求解器差分外逐字段对照
    for broadening in ["0.01", "0.02", "0.04"] {
        let reference: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(golden.join(broadening).join("result.json")).unwrap(),
        )
        .unwrap();
        let new: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(results.join(broadening).join("result.json")).unwrap(),
        )
        .unwrap();
        let mut failures = Vec::new();
        compare(&reference, &new, String::new(), &mut failures);
        // 路径类字段（zero_frequency_handling.modes[N].file）与运行目录相关，忽略
        // Eliashberg Tc 及其派生量：Python 求解器为 float32 矩阵 + LAPACK，
        // Rust 为 f64（交付目标为静态 OpenBLAS，musl 验证待 Linux），
        // 在该合成数据（非物理 λ≈12）上相对差 ~2%；真实数据（λ~0.7）差异远小。
        let tc_e_py = reference["Tc, K"]["Eliashberg"].as_f64().unwrap();
        let tc_e_rs = new["Tc, K"]["Eliashberg"].as_f64().unwrap();
        assert!(
            (tc_e_py - tc_e_rs).abs() <= 0.05 * tc_e_py.max(1.0),
            "Eliashberg Tc 相对差超 5%: py={tc_e_py} rs={tc_e_rs}"
        );
        let allowed_prefix = [
            ".Tc, K.Eliashberg",
            ".DeltaC/Tc",
            ".Delta",
            ".Hc2, T",
            ".zero_frequency_handling.modes",
        ];
        let real_failures: Vec<String> = failures
            .iter()
            .filter(|f| !allowed_prefix.iter().any(|p| f.starts_with(p)))
            .cloned()
            .collect();
        assert!(
            real_failures.is_empty(),
            "result.json 存在非 Eliashberg 差异: {real_failures:?}"
        );
    }

    // 5) CIF：空间群编号/符号与晶胞参数等价（非逐字节）
    let cif = std::fs::read_to_string(results.join("MgB2_35.cif")).unwrap();
    assert!(cif.contains("_symmetry_Int_Tables_number 35"), "空间群编号缺失");
    assert!(cif.contains("Cmm2"), "空间群符号缺失");
    let _ = std::fs::remove_dir_all(&work);
}

fn compare(reference: &serde_json::Value, new: &serde_json::Value, path: String, failures: &mut Vec<String>) {
    match (reference, new) {
        (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
            for (k, v) in a {
                if !b.contains_key(k) {
                    failures.push(format!("{path}.{k} missing"));
                    continue;
                }
                compare(v, &b[k], format!("{path}.{k}"), failures);
            }
        }
        (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
            if a.len() != b.len() {
                failures.push(format!("{path} len {} vs {}", a.len(), b.len()));
                return;
            }
            for (i, (u, v)) in a.iter().zip(b).enumerate() {
                compare(u, v, format!("{path}[{i}]"), failures);
            }
        }
        (serde_json::Value::Number(a), serde_json::Value::Number(b)) => {
            let (Some(x), Some(y)) = (a.as_f64(), b.as_f64()) else {
                return;
            };
            if (x - y).abs() > 1e-6 * x.abs().max(1.0) {
                failures.push(format!("{path}: {x} vs {y}"));
            }
        }
        _ => {
            if reference != new {
                failures.push(format!("{path}: {reference} vs {new}"));
            }
        }
    }
}

use std::process::Command;

/// 按行拆分 CSV（首行 + 数据行），保留引号字段。
fn parse_csv_line_first(text: &str) -> Vec<Vec<String>> {
    text.lines().map(|l| parse_csv_line(l)).collect()
}
