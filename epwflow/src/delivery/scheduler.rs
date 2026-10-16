use anyhow::Result;
use std::path::Path;
use super::acceptance;
use super::collect;
use super::plot;
use super::report_store::ResultsBundle;
use super::super::foundation::store;

/// 交付编排（F5）：收集 → 验收表 → 绘图 → 写入 results/manifest.json。
pub fn run(work: &Path) -> Result<ResultsBundle> {
    let bundle = collect::collect(work)?;
    let cells = acceptance::acceptance_table(&bundle);
    let out = work.join("results");
    std::fs::create_dir_all(&out)?;

    // 仅当存在插值谱时绘 α²F 图（FR-001）。
    let f3_path = work.join("interpolation").join("manifest.json");
    if f3_path.exists() {
        if let Ok(f3) = super::super::interpolation::manifest::InterpolationManifest::load(&f3_path) {
            if let (Some(fine), Some(spec)) = (f3.fine.as_ref(), f3.spectra.first()) {
                let cm1_to_mev = 0.123984_f64;
                let freq_mev: Vec<f64> = fine.frequencies.iter().map(|w| *w * cm1_to_mev).collect();
                let _ = plot::plot_alpha2f(&freq_mev, &spec.a2f, &out.join("alpha2f.svg"));
            }
        }
    }
    let _ = plot::export_csv(&bundle, &out.join("results.csv"));

    let final_bundle = ResultsBundle {
        schema_version: 1,
        run_id: bundle.run_id.clone(),
        cells,
        tc: bundle.tc,
        gaps_mev: bundle.gaps_mev,
        lambda_eff: bundle.lambda_eff,
        omega_log_k: bundle.omega_log_k,
        validation: bundle.validation.clone(),
    };
    store::write(&out.join("manifest.json"), &final_bundle)?;
    Ok(final_bundle)
}
