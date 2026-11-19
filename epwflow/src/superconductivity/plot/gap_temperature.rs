//! 原生各向异性能隙分布及各向同性首 Matsubara 温度曲线。
use super::{io, PlotTask};
use anyhow::Result;
use plotters::style::Color;
use qf_plot::superconductivity::{Curve, Mark, Panel, ScColors};
fn temperature_panels(task: &PlotTask, iso: bool) -> Result<Option<Vec<Panel>>> {
    if !iso {
        let mut panel = Panel::new(
            super::parameter_title(&task.work_dir),
            "Temperature (K)",
            "Δ_{nk} (meV)",
        );
        for path in &task.data.aniso_gap0 {
            if let Some(points) = task.load(path, io::parse_gap0_distribution) {
                panel.curves.push(Curve::new(
                    points,
                    ScColors::TEMPERATURE.to_rgba(),
                    Mark::Line,
                    "",
                ));
            }
        }
        if panel.curves.is_empty() {
            if !task.data.aniso_gap0.is_empty() {
                task.warn("fig3: no valid gap0 distribution; skipped");
            }
            return Ok(None);
        }
        return Ok(Some(vec![
            panel,
            Panel::new("No second parameter set", "Temperature (K)", "Δ_{nk} (meV)"),
        ]));
    }
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
        let has_candidates = if iso {
            !task.data.imag_iso.is_empty()
        } else {
            !task.data.imag_aniso.is_empty()
        };
        if has_candidates {
            task.warn(format!(
                "fig{}: available temperature files did not yield a valid series; skipped",
                if iso { 4 } else { 3 }
            ));
        }
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
/// Mori Fig. 8：直接绘制当前目录原生 gap0 的温度偏移分布。
pub fn plot_aniso_gap_vs_t(task: &PlotTask) -> Result<Option<Vec<Panel>>> {
    temperature_panels(task, false)
}
/// 各向同性首 Matsubara 能隙，以蓝色虚线和圆点绘制。
pub fn plot_iso_gap_vs_t(task: &PlotTask) -> Result<Option<Vec<Panel>>> {
    temperature_panels(task, true)
}
