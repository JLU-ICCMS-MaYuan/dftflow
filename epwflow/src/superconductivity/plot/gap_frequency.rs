//! 同温度虚频/实频能隙；实频缺失时不借用其他温度的数据。
use super::{io, PlotTask};
use anyhow::Result;
use plotters::style::Color;
use qf_plot::superconductivity::{Curve, Mark, Panel, ScColors};
/// 选最低可解析温度，Padé 必须匹配该温度。
pub fn plot_gap_imaginary_real(task: &PlotTask) -> Result<Option<Vec<Panel>>> {
    let gap = task
        .data
        .imag_aniso
        .iter()
        .find_map(|p| task.load(p, io::parse_imag_aniso));
    let Some(gap) = gap else {
        // 各向同性方法目录本来就没有 imag_aniso；图 2 对应 Mori 的各向异性图，
        // 该方法留空不属于数据损坏，也不应污染自动绘图警告。
        if !task.data.imag_aniso.is_empty() {
            task.warn("fig2: missing valid imaginary-axis anisotropic gaps; skipped");
        }
        return Ok(None);
    };
    let t = gap.temperature_k;
    let mut left = Panel::new(
        format!("Imaginary axis, T = {t} K"),
        "iω (meV)",
        "Δ_{nk}(iω) (meV)",
    );
    let points = gap
        .frequencies_mev
        .into_iter()
        .zip(gap.gaps_mev)
        .flat_map(|(w, g)| g.into_iter().map(move |v| (w, v)))
        .collect();
    let mut curve = Curve::new(points, ScColors::BLACK.to_rgba(), Mark::Scatter, "");
    curve.radius = 2;
    left.curves.push(curve);
    let mut right = Panel::new(
        format!("Real axis, T = {t} K"),
        "ω (meV)",
        "Re Δ_{nk}(ω) (meV)",
    );
    let pade = task
        .data
        .pade_aniso
        .iter()
        .filter(|p| io::temperature(p).is_ok_and(|v| (v - t).abs() < 1e-6))
        .find_map(|p| task.load(p, io::parse_pade_aniso));
    if let Some(pade) = pade {
        let points = pade
            .frequencies_mev
            .into_iter()
            .zip(pade.gaps_mev)
            .flat_map(|(w, g)| g.into_iter().map(move |v| (w, v)))
            .collect();
        let mut curve = Curve::new(points, ScColors::REAL, Mark::Scatter, "");
        curve.radius = 1;
        right.curves.push(curve);
    } else {
        task.warn(format!(
            "fig2: missing Padé data at {t} K; right panel skipped"
        ));
    }
    Ok(Some(vec![left, right]))
}
