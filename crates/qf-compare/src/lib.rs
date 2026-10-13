//! 产物对照器（T011）：文件名/状态/退出码精确比较 + 浮点按契约固定容差比较。
//!
//! 契约（docs/specs/0004-dftflow-rust/contracts/config-and-output.md §4）：
//! `|new-ref| <= A + 1e-6 × |ref|`；有确定小数位的字段 `A = max(1e-8, 0.5 × 末位单位)`，
//! 未格式化字段 `A = 1e-8`。失败只报差异，禁止放宽。

use anyhow::Result;
use serde_json::Value;
use std::path::Path;

pub const REL_TOLERANCE: f64 = 1e-6;
pub const DEFAULT_ABS_TOLERANCE: f64 = 1e-8;

/// 由小数位数推导绝对容差：`A = max(1e-8, 0.5 × 10^-decimals)`。
pub fn abs_tolerance_for_decimals(decimals: u32) -> f64 {
    let half_unit = 0.5 * 10f64.powi(-(decimals as i32));
    half_unit.max(DEFAULT_ABS_TOLERANCE)
}

#[derive(Debug, Default)]
pub struct CompareReport {
    pub failures: Vec<String>,
    pub checked: usize,
}

impl CompareReport {
    pub fn is_ok(&self) -> bool {
        self.failures.is_empty()
    }

    pub fn expect_ok(&self, label: &str) -> Result<()> {
        if self.is_ok() {
            Ok(())
        } else {
            anyhow::bail!(
                "对照失败 [{label}]（{} 项中 {} 项不符）:\n  {}",
                self.checked,
                self.failures.len(),
                self.failures.join("\n  ")
            )
        }
    }
}

fn float_within(new: f64, reference: f64, abs_tol: f64) -> bool {
    (new - reference).abs() <= abs_tol + REL_TOLERANCE * reference.abs()
}

/// 逐行数值表比较：两侧行数一致，浮点按给定绝对容差 + 全局相对容差。
/// `decimals`：参考产物的固定小数位（用于 A 的推导）；None → DEFAULT_ABS_TOLERANCE。
pub fn compare_numeric_tables(
    reference_path: &Path,
    new_path: &Path,
    decimals: Option<u32>,
) -> Result<CompareReport> {
    let read_rows = |path: &Path| -> Result<Vec<Vec<f64>>> {
        let text = std::fs::read_to_string(path)?;
        Ok(text
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
            .map(|l| {
                l.split_whitespace()
                    .filter_map(|t| t.parse::<f64>().ok())
                    .collect()
            })
            .collect())
    };
    let reference = read_rows(reference_path)?;
    let new = read_rows(new_path)?;
    let abs_tol = match decimals {
        Some(d) => abs_tolerance_for_decimals(d),
        None => DEFAULT_ABS_TOLERANCE,
    };
    let mut report = CompareReport::default();
    if reference.len() != new.len() {
        report
            .failures
            .push(format!("行数不一致: ref={} new={}", reference.len(), new.len()));
        return Ok(report);
    }
    for (r, (ref_row, new_row)) in reference.iter().zip(&new).enumerate() {
        if ref_row.len() != new_row.len() {
            report.failures.push(format!("第 {} 行列数不一致", r + 1));
            continue;
        }
        for (c, (ref_val, new_val)) in ref_row.iter().zip(new_row).enumerate() {
            report.checked += 1;
            if !float_within(*new_val, *ref_val, abs_tol) {
                report.failures.push(format!(
                    "[{},{}] ref={ref_val} new={new_val} 误差={:.3e} 容差 A={abs_tol}",
                    r + 1,
                    c + 1,
                    (new_val - ref_val).abs()
                ));
            }
        }
    }
    Ok(report)
}

/// 文本精确比较（忽略每行尾部空白）。
pub fn compare_text_exact(reference_path: &Path, new_path: &Path) -> Result<CompareReport> {
    let reference = std::fs::read_to_string(reference_path)?;
    let new = std::fs::read_to_string(new_path)?;
    let mut report = CompareReport::default();
    let ref_lines: Vec<&str> = reference.lines().map(str::trim_end).collect();
    let new_lines: Vec<&str> = new.lines().map(str::trim_end).collect();
    if ref_lines.len() != new_lines.len() {
        report
            .failures
            .push(format!("行数不一致: ref={} new={}", ref_lines.len(), new_lines.len()));
        return Ok(report);
    }
    for (i, (r, n)) in ref_lines.iter().zip(&new_lines).enumerate() {
        report.checked += 1;
        if r != n {
            report.failures.push(format!("第 {} 行不一致: ref={r:?} new={n:?}", i + 1));
        }
    }
    Ok(report)
}

/// 状态 JSON 比较：状态枚举精确；数值字段按浮点容差。
pub fn compare_status_json(reference_path: &Path, new_path: &Path) -> Result<CompareReport> {
    let reference: Value = serde_json::from_str(&std::fs::read_to_string(reference_path)?)?;
    let new: Value = serde_json::from_str(&std::fs::read_to_string(new_path)?)?;
    let mut report = CompareReport::default();
    compare_json_value(&reference, &new, &mut Vec::new(), &mut report);
    Ok(report)
}

fn compare_json_value(reference: &Value, new: &Value, path: &mut Vec<String>, report: &mut CompareReport) {
    report.checked += 1;
    match (reference, new) {
        (Value::Object(ref_obj), Value::Object(new_obj)) => {
            for (key, ref_val) in ref_obj {
                if !new_obj.contains_key(key) {
                    report.failures.push(format!("缺少键: {}", path_join(path, key)));
                    continue;
                }
                path.push(key.clone());
                compare_json_value(ref_val, &new_obj[key], path, report);
                path.pop();
            }
        }
        (Value::Array(ref_arr), Value::Array(new_arr)) => {
            if ref_arr.len() != new_arr.len() {
                report
                    .failures
                    .push(format!("数组长度不一致: {}", path.join("/")));
                return;
            }
            for (i, (r, n)) in ref_arr.iter().zip(new_arr).enumerate() {
                path.push(i.to_string());
                compare_json_value(r, n, path, report);
                path.pop();
            }
        }
        (Value::Number(ref_num), Value::Number(new_num)) => {
            let (Some(r), Some(n)) = (ref_num.as_f64(), new_num.as_f64()) else {
                if ref_num != new_num {
                    report.failures.push(format!(
                        "数值不一致: {} ref={ref_num} new={new_num}",
                        path.join("/")
                    ));
                }
                return;
            };
            if !float_within(n, r, DEFAULT_ABS_TOLERANCE) {
                report.failures.push(format!(
                    "数值超差: {} ref={r} new={n}",
                    path.join("/")
                ));
            }
        }
        _ => {
            if reference != new {
                report.failures.push(format!(
                    "值不一致: {} ref={reference} new={new}",
                    path.join("/")
                ));
            }
        }
    }
}

fn path_join(path: &[String], key: &str) -> String {
    if path.is_empty() {
        key.to_string()
    } else {
        format!("{}/{}", path.join("/"), key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_file(name: &str, content: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("qf_compare_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn abs_tolerance_from_decimals() {
        assert!((abs_tolerance_for_decimals(6) - 5e-7).abs() < 1e-15);
        assert!((abs_tolerance_for_decimals(2) - 0.005).abs() < 1e-15);
        assert_eq!(abs_tolerance_for_decimals(12), 1e-8);
    }

    #[test]
    fn numeric_table_pass_and_fail() {
        let reference = temp_file("ref.dat", "1.000000 2.000000\n3.000000 4.000000\n");
        let pass = temp_file("pass.dat", "1.000000 2.000000\n3.000000 4.000000\n");
        let fail = temp_file("fail.dat", "1.000000 2.000000\n3.000000 4.100000\n");
        assert!(compare_numeric_tables(&reference, &pass, Some(6)).unwrap().is_ok());
        assert!(!compare_numeric_tables(&reference, &fail, Some(6)).unwrap().is_ok());
    }

    #[test]
    fn status_json_compares_states_exactly() {
        let reference = temp_file("ref.json", r#"{"scf": "finished", "n": 3}"#);
        let pass = temp_file("pass.json", r#"{"scf": "finished", "n": 3}"#);
        let fail = temp_file("fail.json", r#"{"scf": "unstarted", "n": 3}"#);
        assert!(compare_status_json(&reference, &pass).unwrap().is_ok());
        assert!(!compare_status_json(&reference, &fail).unwrap().is_ok());
    }
}
