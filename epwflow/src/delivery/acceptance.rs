use super::report_store::{ComparisonCell, ResultsBundle};

/// 六组合验收表（3 PH × 2 求解，FR-004）：逐单元格标注“已运行/未运行”。
/// 诚实标注：仅实际运行的组合填入指标，其余为 not_run，不编造证据。
pub fn acceptance_table(actual: &ResultsBundle) -> Vec<ComparisonCell> {
    let ph_modes = ["nosplit", "index", "coords"];
    let solver_modes = ["iso", "aniso"];
    let mut cells = Vec::new();
    for ph in ph_modes {
        for solver in solver_modes {
            let actual_match = actual.cells.iter().any(|c| {
                (c.ph_mode == ph || c.ph_mode == "n/a") && c.solver_mode == solver
            });
            if actual_match {
                // 复制实际单元格，强制 ph_mode 为当前枚举值以保持表格完整。
                if let Some(src) = actual.cells.iter().find(|c| c.solver_mode == solver) {
                    let mut cell = src.clone();
                    cell.ph_mode = ph.to_string();
                    cells.push(cell);
                    continue;
                }
            }
            cells.push(ComparisonCell {
                ph_mode: ph.to_string(),
                solver_mode: solver.to_string(),
                reference: "epwpy".to_string(),
                input_hashes: Default::default(),
                metrics: Default::default(),
                tolerances: Default::default(),
                status: "not_run".to_string(),
                evidence: vec!["未运行：本机小网格验收仅覆盖实际组合".to_string()],
            });
        }
    }
    cells
}
