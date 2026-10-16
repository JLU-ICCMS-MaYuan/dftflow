use anyhow::{Context, Result};
use std::path::Path;
use super::report_store::{ComparisonCell, ResultsBundle};
use super::super::interpolation::manifest::InterpolationManifest;
use super::super::qe_data::manifest::QeManifest;
use super::super::superconductivity::manifest::SuperconductivityManifest;

/// 一次入口自动收集各阶段交付（FR-001）：读取 qe-data / interpolation / superconductivity 清单，
/// 聚合 Tc、能隙、λ 等，并构建六组合覆盖表的“实际运行”单元格。
pub fn collect(work: &Path) -> Result<ResultsBundle> {
    let f2_path = work.join("qe-data").join("manifest.json");
    let f3_path = work.join("interpolation").join("manifest.json");
    let f4_path = work.join("superconductivity").join("manifest.json");

    let _f2 = if f2_path.exists() {
        Some(QeManifest::load(&f2_path).context("data_invalid: 无法加载 qe-data 清单")?)
    } else {
        None
    };
    let f3 = if f3_path.exists() {
        Some(InterpolationManifest::load(&f3_path).context("data_invalid: 无法加载插值清单")?)
    } else {
        None
    };
    let f4 = if f4_path.exists() {
        Some(SuperconductivityManifest::load(&f4_path).context("data_invalid: 无法加载超导清单")?)
    } else {
        None
    };

    ensure_any_stage(&f3, &f4)?;

    let lambda_eff = f3
        .as_ref()
        .and_then(|m| m.spectra.first())
        .map(|s| s.lambda_integral)
        .unwrap_or(0.0);
    let omega_log_k = f4.as_ref().map(|_| 0.0).unwrap_or(0.0);

    let tc = f4.as_ref().and_then(|m| m.tc);
    let gaps_mev = f4.as_ref().map(|m| m.gaps_mev.clone()).unwrap_or_default();

    let mut cells: Vec<ComparisonCell> = Vec::new();
    if let (Some(f3m), Some(f4m)) = (f3.as_ref(), f4.as_ref()) {
        let ph_mode = if f3m.mode_readiness.contains_key("iso") {
            "index".to_string()
        } else {
            "n/a".to_string()
        };
        let mut metrics = std::collections::BTreeMap::new();
        if let Some(t) = tc {
            metrics.insert("tc_k".to_string(), t);
        }
        for (i, g) in gaps_mev.iter().enumerate() {
            metrics.insert(format!("gap_mev_{}", i), *g);
        }
        metrics.insert("lambda".to_string(), lambda_eff);
        let cell = ComparisonCell {
            ph_mode,
            solver_mode: f4m.mode.clone(),
            reference: "epwpy".to_string(),
            input_hashes: {
                let mut m = std::collections::BTreeMap::new();
                m.insert("interpolation".to_string(), f3m.run_id.clone());
                m
            },
            metrics,
            tolerances: std::collections::BTreeMap::new(),
            status: if f4m.validation.all_passed() { "ok" } else { "partial" }.to_string(),
            evidence: f4m.validation.evidence.clone(),
        };
        cells.push(cell);
    }

    let validation = f4
        .as_ref()
        .map(|m| m.validation.clone())
        .unwrap_or_else(|| crate::qe_data::manifest::Validation {
            status: "no_data".to_string(),
            checks: vec![],
            units: std::collections::BTreeMap::new(),
            thresholds: std::collections::BTreeMap::new(),
            evidence: vec![],
        });

    Ok(ResultsBundle {
        schema_version: 1,
        run_id: f4.as_ref().map(|m| m.run_id.clone()).unwrap_or_else(|| "delivery".to_string()),
        cells,
        tc,
        gaps_mev,
        lambda_eff,
        omega_log_k,
        validation,
    })
}

fn ensure_any_stage(
    f3: &Option<InterpolationManifest>,
    f4: &Option<SuperconductivityManifest>,
) -> Result<()> {
    anyhow::ensure!(
        f3.is_some() || f4.is_some(),
        "data_invalid: 交付收集需要至少一个上游阶段清单（interpolation 或 superconductivity）"
    );
    Ok(())
}
