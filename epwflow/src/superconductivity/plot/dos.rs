//! 超导态密度按各方法自己的 N_F 归一化，图例标注温度。
use super::{io, PlotTask};
use anyhow::Result;
use plotters::style::Color;
use qf_plot::superconductivity::{Curve, Mark, Panel, ScColors};
/// 同模式不同参数两联图；优先 10 K，否则明确标注最低可用温度。
pub fn plot_superconducting_dos(task: &PlotTask) -> Result<Option<Vec<Panel>>> {
    let mode = if task
        .work_dir
        .file_name()
        .is_some_and(|n| n.to_string_lossy().starts_with("iso-"))
    {
        "iso-"
    } else {
        "aniso-"
    };
    let mut panels = vec![];
    let mut any = false;
    for (dir, data) in task.groups(mode)? {
        let mut panel = Panel::new(super::parameter_title(&dir), "ω (meV)", "N_S(ω)/N_F");
        let mut paths = data.qdos;
        paths.sort_by(|a, b| {
            let key = |p: &std::path::PathBuf| {
                let t = io::temperature(p).unwrap_or(f64::INFINITY);
                if (t - 10.).abs() < 1e-6 {
                    0.
                } else {
                    t + 1.
                }
            };
            key(a).total_cmp(&key(b))
        });
        if let Some(dos) = paths.iter().find_map(|p| task.load(p, io::parse_qdos)) {
            let normalized = dos.normalized;
            let nf = if normalized {
                1.
            } else {
                data.epw_output
                    .as_ref()
                    .and_then(|p| task.load(p, io::extract_fermi_dos))
                    .unwrap_or_else(|| {
                        task.warn(format!(
                            "{}: Fermi DOS unavailable; using N_F=1.0",
                            dir.display()
                        ));
                        1.
                    })
            };
            let points = dos
                .omega_mev
                .into_iter()
                .zip(
                    dos.dos_raw
                        .into_iter()
                        .map(|v| if normalized { v } else { v / nf }),
                )
                .collect();
            panel.curves.push(Curve::new(
                points,
                ScColors::BLACK.to_rgba(),
                Mark::Line,
                format!("T = {} K; N_F = {nf}", dos.temperature_k),
            ));
            any = true;
        }
        panels.push(panel);
    }
    if !any {
        task.warn("fig5: missing qdos; skipped");
        return Ok(None);
    }
    while panels.len() < 2 {
        panels.push(Panel::new(
            "No second parameter set",
            "ω (meV)",
            "N_S(ω)/N_F",
        ));
    }
    Ok(Some(panels))
}
