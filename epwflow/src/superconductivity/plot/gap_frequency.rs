//! 同温度虚频/实频能隙；各向异性读取 aniso 数据，各向同性读取 iso 数据。
use super::{io, PlotTask};
use anyhow::Result;
use plotters::style::Color;
use qf_plot::superconductivity::{Curve, Mark, Panel, ScColors};
/// 选最低可解析温度，Padé 必须匹配该温度。
pub fn plot_gap_imaginary_real(task: &PlotTask) -> Result<Option<Vec<Panel>>> {
    // 优先尝试 aniso 数据（四列格式）
    let gap = task
        .data
        .imag_aniso
        .iter()
        .find_map(|p| task.load(p, io::parse_imag_aniso));

    // 如果没有 aniso，尝试 iso 数据（三列格式）
    let (gap, is_iso) = if let Some(gap) = gap {
        (gap, false)
    } else if let Some(gap) = task
        .data
        .imag_iso
        .iter()
        .find_map(|p| task.load(p, io::parse_imag_iso))
    {
        (gap, true)
    } else {
        // 两种数据都没有才跳过，只在有文件但解析失败时才警告
        if !task.data.imag_aniso.is_empty() || !task.data.imag_iso.is_empty() {
            task.warn("fig2: missing valid imaginary-axis gap data; skipped");
        }
        return Ok(None);
    };

    let t = gap.temperature_k;
    let axis_label = if is_iso { "Δ(iω) (meV)" } else { "Δ_{nk}(iω) (meV)" };
    let mut left = Panel::new(
        format!("Imaginary axis, T = {t} K"),
        "iω (meV)",
        axis_label,
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

    // 实频轴严格使用当前方法的 Padé 文件，不跨目录借用另一方法。
    let re_label = if is_iso { "Re Δ(ω) (meV)" } else { "Re Δ_{nk}(ω) (meV)" };
    let mut right = Panel::new(
        format!("Real axis, T = {t} K"),
        "ω (meV)",
        re_label,
    );

    let pade = if is_iso {
        task.data
            .pade_iso
            .iter()
            .filter(|p| io::temperature(p).is_ok_and(|v| (v - t).abs() < 1e-6))
            .find_map(|p| task.load(p, io::parse_pade_iso))
    } else {
        task.data
            .pade_aniso
            .iter()
            .filter(|p| io::temperature(p).is_ok_and(|v| (v - t).abs() < 1e-6))
            .find_map(|p| task.load(p, io::parse_pade_aniso))
    };
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
        task.warn(format!("fig2: missing Padé data at {t} K; right panel skipped"));
    }

    Ok(Some(vec![left, right]))
}
