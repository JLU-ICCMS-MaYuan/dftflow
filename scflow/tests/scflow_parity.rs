//! scflow 全链路黄金对照（Python 参考 `python-reference-0004` 产物 vs Rust）：
//! 分展宽 CSV 逐字节一致；result.json 全部数值按固定容差；
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

    // 2) 分展宽及 summary CSV（含 Tc 与所有派生量）逐字段一致
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
        // OpenBLAS 恢复全部列的严格对照，包括 Eliashberg 派生量。
        let rel = "summary_MgB2_s3_mu0,1.csv";
        let reference = parse_csv_line_first(&std::fs::read_to_string(golden.join(rel)).unwrap());
        let new = parse_csv_line_first(&std::fs::read_to_string(results.join(rel)).unwrap());
        assert_eq!(reference.len(), new.len());
        for (row, (ra, rb)) in reference.iter().zip(&new).enumerate() {
            assert_eq!(ra.len(), rb.len());
            for (col, (x, y)) in ra.iter().zip(rb).enumerate() {
                assert_eq!(x, y, "summary[{row}][{col}] 不一致");
            }
        }
    }

    // 3) POSCAR 逐字节一致
    let reference = std::fs::read_to_string(golden.join("MgB2.vasp")).unwrap();
    let new = std::fs::read_to_string(results.join("MgB2.vasp")).unwrap();
    assert_eq!(reference, new, "MgB2.vasp 不一致");

    // 4) result.json：全部数值与状态对照，仅归一化临时输入目录
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
        assert!(failures.is_empty(), "result.json 存在差异: {failures:?}");
    }

    // 5) CIF：空间群编号/符号与晶胞参数等价（非逐字节）
    let cif = std::fs::read_to_string(results.join("MgB2_35.cif")).unwrap();
    assert!(
        cif.contains("_symmetry_Int_Tables_number 35"),
        "空间群编号缺失"
    );
    assert!(cif.contains("Cmm2"), "空间群符号缺失");
    let mut pdfs = Vec::new();
    collect_pdfs(&results, &mut pdfs);
    assert_eq!(pdfs.len(), 7, "每个展宽两份 PDF，加一份 summary");
    for pdf in pdfs {
        let bytes = std::fs::read(&pdf).unwrap();
        assert!(
            bytes.starts_with(b"%PDF-"),
            "PDF 类型错误: {}",
            pdf.display()
        );
        assert!(bytes.windows(5).any(|s| s == b"%%EOF"), "PDF 截断");
    }
    let _ = std::fs::remove_dir_all(&work);
}

fn compare(
    reference: &serde_json::Value,
    new: &serde_json::Value,
    path: String,
    failures: &mut Vec<String>,
) {
    match (reference, new) {
        (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
            assert_eq!(a.len(), b.len(), "JSON 字段数量不同: {path}");
            for (k, v) in a {
                if path.starts_with(".zero_frequency_handling.modes[") && k == "file" {
                    assert_eq!(
                        std::path::Path::new(v.as_str().unwrap()).file_name(),
                        std::path::Path::new(b[k].as_str().unwrap()).file_name()
                    );
                    continue;
                }
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
            if (x - y).abs() > 1e-8 + 1e-6 * x.abs() {
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

fn collect_pdfs(dir: &Path, paths: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let p = entry.unwrap().path();
        if p.is_dir() {
            collect_pdfs(&p, paths);
        } else if p.extension().and_then(|e| e.to_str()) == Some("pdf") {
            paths.push(p);
        }
    }
}
