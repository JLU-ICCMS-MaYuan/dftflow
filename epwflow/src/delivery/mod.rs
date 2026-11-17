//! F5 交付模块族（spec 0014，FR-001~FR-010）。
pub mod acceptance;
pub mod budget;
pub mod cli;
pub mod collect;
pub mod compare;
pub mod decay;
pub mod comparison;
pub mod comparison_plot;
pub mod native;
pub mod native_plot;
pub mod plot;
pub(crate) mod publication;
pub mod regression;
pub mod report_store;
pub mod scheduler;

/// 统一收集入口；比较诊断和原生求解共享结果根目录，各自保留版本清单。
pub fn collect_results(
    work: &std::path::Path,
    output: Option<&std::path::Path>,
) -> anyhow::Result<std::path::PathBuf> {
    if output.is_none() && crate::foundation::layout::enabled(work) {
        let root=work.join("results");
        crate::foundation::stage_results::check(&root)?;
        return Ok(root);
    }
    let result = output
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| crate::foundation::layout::path(work,"results"));
    let mut found = false;
    if crate::foundation::layout::path(&work, "superconductivity/manifest.json").exists() {
        if crate::foundation::layout::enabled(work) {return native::collect(work,Some(&result));}
        native::collect(work, Some(&result))?;
        found = true;
    }
    if crate::foundation::layout::path(&work, "interpolation/comparison/manifest.json").exists()
        && (!crate::foundation::layout::enabled(work) || !found) {
        comparison::collect(work, Some(&result))?;
        found = true;
    }
    anyhow::ensure!(found, "data_invalid: No native or comparative results to collect");
    Ok(result)
}

pub fn plot_results(
    results: &std::path::Path,
    output: Option<&std::path::Path>,
    formats: &[String],
) -> anyhow::Result<std::path::PathBuf> {
    if results.join("native.plot.json").is_file() { return native_plot::plot(results,output,formats); }
    if results.join("index.json").exists() || crate::qe_data::save::files(results)?.iter().any(|f| f.relative_path.to_string_lossy().ends_with(".plot.json")) {
        let target=if let Some(output)=output {
            anyhow::ensure!(!output.exists(),"input_conflict: Repaint output already exists");
            crate::qe_data::save::copy_tree(results,output)?;
            output
        } else {results};
        return crate::foundation::stage_results::redraw(target);
    }
    let mut found = false;
    let target = output
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| results.join("figures"));
    if results.join("manifest.json").exists() {
        native_plot::plot(results, Some(&target), formats)?;
        found = true;
    }
    if results.join("comparison/manifest.json").exists() {
        comparison_plot::plot(results, Some(&target), formats)?;
        found = true;
    }
    anyhow::ensure!(found, "data_invalid: No Plotable Results Package");
    Ok(target)
}

pub fn check_results(results: &std::path::Path) -> anyhow::Result<serde_json::Value> {
    if results.join("native.plot.json").is_file() {
        let mut report=native::check(results)?;
        let figures=native_plot::check(results)?;
        report["workflow_complete"]=(report["native_complete"]==true && figures).into();return Ok(report);
    }

    if results.join("index.json").exists() || crate::qe_data::save::files(results)?.iter().any(|f| f.relative_path.to_string_lossy().ends_with(".plot.json")) {
        let index=crate::foundation::stage_results::check(results)?;
        return Ok(serde_json::json!({"schema_version":1,"operation_status":"ok","stage_results_verified":true,"entries":index.entries,"workflow_complete":false}));
    }
    let mut report =
        serde_json::json!({"schema_version":1,"operation_status":"ok","workflow_complete":false});
    let mut found = false;
    if results.join("manifest.json").exists() {
        report = native::check(results)?;
        let figures = native_plot::check(results)?;
        report["native_figures_verified"] = figures.into();
        report["workflow_complete"] = (report["native_complete"] == true && figures).into();
        found = true;
    }
    if results.join("comparison/manifest.json").exists() {
        let data = comparison::load(&results.join("comparison"))?;
        let summary = comparison::summary(&data);
        report["comparison_complete"] = summary["comparison_complete"].clone();
        report["differences"] = summary["differences"].clone();
        report["figures_verified"] = comparison_plot::check_figures(results)?.into();
        if report["figures_verified"] != true {
            report["workflow_complete"] = false.into();
        }
        found = true;
    }
    anyhow::ensure!(found, "data_invalid: The result package is missing");
    Ok(report)
}

pub fn finalize(work: &std::path::Path) -> anyhow::Result<std::path::PathBuf> {
    finalize_to(work, None)
}

pub fn finalize_to(
    work: &std::path::Path,
    output: Option<&std::path::Path>,
) -> anyhow::Result<std::path::PathBuf> {
    let results = collect_results(work, output)?;
    plot_results(&results, None, &["png".into(), "svg".into(), "pdf".into()])?;
    anyhow::ensure!(
        check_results(&results)?["workflow_complete"] == true,
        "partial: native delivery or plots are incomplete"
    );
    Ok(results)
}
