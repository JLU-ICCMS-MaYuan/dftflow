use anyhow::{bail, ensure, Result};
use std::collections::BTreeMap;
use super::compare;
use super::manifest::QualityReport;

/// 固定的插值质量阈值（FR-009）：频率最大绝对误差 ≤ 5 cm⁻¹，RMS ≤ 2 cm⁻¹。
pub fn thresholds() -> BTreeMap<String, f64> {
    let mut m = BTreeMap::new();
    m.insert("frequency_max_error_cm1".to_string(), 5.0);
    m.insert("frequency_rms_error_cm1".to_string(), 2.0);
    m
}

/// 独立插值质量检查：参考频率 vs 插值频率（FR-006/009）。
/// 覆盖不足（参考点数为 0）或缺参考即无效，不靠单一命中放行。
pub fn evaluate(
    reference: &[f64],
    interpolated: &[f64],
    energy_reference: &str,
) -> Result<QualityReport> {
    ensure!(!reference.is_empty(), "data_invalid: 缺参考频率，质量检查无效");
    ensure!(
        reference.len() == interpolated.len(),
        "data_invalid: 参考与插值频率点数不一致 {} vs {}",
        reference.len(),
        interpolated.len()
    );
    let (max_err, rms) = compare::error_stats(reference, interpolated);
    let th = thresholds();
    let mut report = QualityReport {
        points: reference.len(),
        energy_reference: energy_reference.to_string(),
        max_error: max_err,
        rms_error: rms,
        coverage: 1.0,
        thresholds: th.clone(),
        status: "valid".to_string(),
    };
    report.status = if max_err <= th["frequency_max_error_cm1"] && rms <= th["frequency_rms_error_cm1"] {
        "valid"
    } else {
        "invalid"
    }
    .to_string();
    if report.status == "invalid" {
        bail!(
            "data_invalid: 插值质量超差：max={} cm⁻¹, rms={} cm⁻¹（阈值 5/2）",
            max_err,
            rms
        );
    }
    Ok(report)
}

/// 覆盖度检查：质量点必须覆盖全部验证模式（FR-006）。
pub fn coverage_ok(report: &QualityReport, required_modes: usize) -> bool {
    report.points >= required_modes && report.coverage > 0.0
}
