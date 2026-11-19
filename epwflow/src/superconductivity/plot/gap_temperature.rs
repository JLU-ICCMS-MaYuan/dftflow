//! 逐状态温度曲线；不排序能隙值、不平滑或强制单调。
use super::{io, PlotTask};
use anyhow::Result;
use plotters::style::Color;
use qf_plot::superconductivity::{Curve, Mark, Panel, ScColors};
fn temperature_panels(task: &PlotTask, iso: bool) -> Result<Option<Vec<Panel>>> {
    let mut panels = vec![];
    let mut any = false;
    {
        let dir = &task.work_dir;
        let data = &task.data;
        let mut panel = Panel::new(
            super::parameter_title(dir),
            "Temperature (K)",
            if iso { "Δ₀ (meV)" } else { "Δ_{nk} (meV)" },
        );
        let mut series = vec![];
        if iso {
            for path in &data.imag_iso {
                if let Some(g) = task.load(path, io::parse_imag_iso) {
                    series.push((g.temperature_k, g.gaps_mev[0].clone()));
                }
            }
        } else {
            // 原生 gap0 是五列分布，不含逐状态身份；不能把分箱伪装成 k 点。
            // 温度曲线只从同一方法目录的完整四列 imag_aniso 首 Matsubara 数据读取。
            for path in &data.imag_aniso {
                let Ok(t) = io::temperature(path) else {
                    task.warn(format!("{}: invalid temperature", path.display()));
                    continue;
                };
                if series.iter().any(|(old, _)| (*old - t).abs() < 1e-6) {
                    continue;
                }
                if let Some(g) = task.load(path, io::parse_imag_aniso) {
                    series.push((g.temperature_k, g.gaps_mev[0].clone()));
                }
            }
        }
        series.sort_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((_, first)) = series.first() {
            let states = first.len();
            series.retain(|(t, g)| {
                if g.len() != states {
                    task.warn(format!("T={t}: state count changed; skipped temperature"));
                    false
                } else {
                    true
                }
            });
            for state in 0..states {
                let points = series.iter().map(|(t, g)| (*t, g[state])).collect();
                let mut curve = Curve::new(
                    points,
                    ScColors::TEMPERATURE.to_rgba(),
                    if iso { Mark::Dashed } else { Mark::PointsLine },
                    "",
                );
                curve.radius = if iso { 4 } else { 3 };
                panel.curves.push(curve);
            }
            any = true;
        }
        panels.push(panel);
    }
    if !any {
        task.warn(format!(
            "fig{}: missing valid temperature series; skipped",
            if iso { 4 } else { 3 }
        ));
        return Ok(None);
    }
    while panels.len() < 2 {
        panels.push(Panel::new(
            "No second parameter set",
            "Temperature (K)",
            if iso { "Δ₀ (meV)" } else { "Δ_{nk} (meV)" },
        ));
    }
    Ok(Some(panels))
}
/// 各向异性逐状态曲线，不将五列分布直方图误认为状态。
pub fn plot_aniso_gap_vs_t(task: &PlotTask) -> Result<Option<Vec<Panel>>> {
    temperature_panels(task, false)
}
/// 各向同性首 Matsubara 能隙，以蓝色虚线和圆点绘制。
pub fn plot_iso_gap_vs_t(task: &PlotTask) -> Result<Option<Vec<Panel>>> {
    temperature_panels(task, true)
}
