//! 电声耦合分布及 Eliashberg 双轴图。
use super::{
    io::{self, Distribution},
    PlotTask,
};
use anyhow::Result;
use plotters::style::Color;
use qf_plot::superconductivity::{histogram, Curve, Mark, Panel, ScColors};
/// 缺少谱跳过整图，缺少 λ 数据只留空对应子图。
pub fn plot_coupling_triple(task: &PlotTask) -> Result<Option<Vec<Panel>>> {
    let Some(path) = &task.data.a2f else {
        task.warn("fig1: missing a2f; skipped");
        return Ok(None);
    };
    let Some(data) = task.load(path, io::parse_a2f) else {
        return Ok(None);
    };
    let mut panels = vec![
        Panel::new("Pair coupling", "λ_{nk,mk+q}", "ρ(λ_{nk,mk+q})"),
        Panel::new("Fermi-surface coupling", "λ_{nk}", "ρ(λ_{nk})"),
    ];
    for (index, path) in [&task.data.lambda_pairs, &task.data.lambda_k_pairs]
        .iter()
        .enumerate()
    {
        let Some(path) = path else {
            task.warn(format!(
                "fig1 panel {}: missing lambda distribution",
                index + 1
            ));
            continue;
        };
        if let Some(d) = task.load(path, |p| io::parse_distribution(p, index == 1)) {
            let (points, mark) = match d {
                Distribution::Samples { values, weights } => match histogram(&values, &weights) {
                    Ok(points) => (points, Mark::Bars),
                    Err(error) => {
                        task.warn(format!("{}: {error:#}", path.display()));
                        continue;
                    }
                },
                Distribution::Density { x, y } => (x.into_iter().zip(y).collect(), Mark::Line),
            };
            if points.is_empty() {
                task.warn(format!("{}: empty lambda distribution", path.display()));
            }
            panels[index]
                .curves
                .push(Curve::new(points, ScColors::BLACK.to_rgba(), mark, ""));
        }
    }
    let mut spectrum = Panel::new("Eliashberg spectrum", "ω (meV)", "α²F(ω)");
    spectrum.curves.push(Curve::new(
        data.omega_mev.iter().copied().zip(data.a2f).collect(),
        ScColors::BLACK.to_rgba(),
        Mark::Line,
        "α²F(ω)",
    ));
    spectrum.secondary = Some(Curve::new(
        data.omega_mev
            .into_iter()
            .zip(data.lambda_cumulative)
            .collect(),
        ScColors::LAMBDA.to_rgba(),
        Mark::Line,
        "λ(ω)",
    ));
    panels.push(spectrum);
    Ok(Some(panels))
}
