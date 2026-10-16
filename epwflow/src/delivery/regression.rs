use anyhow::Result;
use serde::Serialize;
use super::compare;
use super::report_store::{ComparisonCell, ResultsBundle};

/// 回归比对（FR-005/007）：将当前结果包与基线（先前结果包）按容差逐项比较。
#[derive(Debug, Clone, Serialize)]
pub struct RegressionReport {
    pub tc_regressed: bool,
    pub lambda_regressed: bool,
    pub all_close: bool,
    pub notes: Vec<String>,
}

/// 与基线结果包比较；无基线时视为首次运行，全部通过。
pub fn against_baseline(current: &ResultsBundle, baseline: Option<&ResultsBundle>) -> Result<RegressionReport> {
    let mut notes = Vec::new();
    let (mut tc_regressed, mut lambda_regressed) = (false, false);
    if let Some(base) = baseline {
        if !compare::tc_close(current.tc, base.tc) {
            tc_regressed = true;
            notes.push(format!("Tc 偏离基线: {:?} vs {:?}", current.tc, base.tc));
        }
        if !compare::lambda_close(current.lambda_eff, base.lambda_eff) {
            lambda_regressed = true;
            notes.push(format!("λ 偏离基线: {} vs {}", current.lambda_eff, base.lambda_eff));
        }
    } else {
        notes.push("无基线，跳过回归比对".to_string());
    }
    Ok(RegressionReport {
        tc_regressed,
        lambda_regressed,
        all_close: !tc_regressed && !lambda_regressed,
        notes,
    })
}

/// 校验六组合覆盖表中所有单元格均非“未运行”才算整体成功（FR-004）。
pub fn all_combos_run(cells: &[ComparisonCell]) -> bool {
    cells.iter().all(|c| c.status != "not_run")
}
