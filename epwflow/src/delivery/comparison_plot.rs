//! 四类叠图共享同一绘图模型；QE 先画实线，EPW 后画虚线。
use super::comparison::{self, Artifact, Curve, Dataset, KINDS};
use crate::foundation::store;
use anyhow::{ensure, Result};
use plotters::{coord::Shift, element::DashedPathElement, prelude::*};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

const ERROR_PURPLE: RGBColor = RGBColor(138, 43, 226);

fn coordinate(value: f64) -> String {
    if value.abs() < 1e-7 {
        return "0".into();
    }
    for denominator in 1..=48 {
        let numerator = (value * denominator as f64).round();
        if (value - numerator / denominator as f64).abs() < 1e-7 {
            return if denominator == 1 {
                format!("{numerator:.0}")
            } else {
                format!("{numerator:.0}/{denominator}")
            };
        }
    }
    format!("{value:.6}")
}

/// 路径外最大误差单独展示，不投影到无关的高对称点或覆盖连续色散。
fn phonon_panel<DB: DrawingBackend>(area: &DrawingArea<DB, Shift>, data: &Dataset) -> Result<()>
where
    DB::ErrorType: 'static,
{
    let markers = &data.metadata.phonon_error_markers;
    if markers.is_empty() {
        return chart(area, &data.phonon, data, 2, "total");
    }
    let (_, height) = area.dim_in_pixel();
    let (main, footer) = area.split_vertically(height.saturating_sub(155));
    chart(&main, &data.phonon, data, 2, "total")?;
    for (card, marker) in footer.split_evenly((1, markers.len())).iter().zip(markers) {
        let (width, _) = card.dim_in_pixel();
        let title = if marker.scope == "sampled_maximum" {
            if marker.path_distance.is_some() {
                "Sampled maximum (on path)"
            } else {
                "Sampled maximum (OFF PATH)"
            }
        } else {
            "On-path sampled maximum"
        };
        let label = marker
            .path_label
            .as_deref()
            .unwrap_or("Not a point on this high-symmetry path");
        let q = marker
            .q_crystal
            .iter()
            .map(|v| coordinate(*v))
            .collect::<Vec<_>>()
            .join(", ");
        for (y, text, size) in [
            (20, format!("{title}: {:.2} cm^-1", marker.error_cm1), 16),
            (43, format!("q(crystal) = ({q})"), 14),
            (
                65,
                format!("mode {} (frequency sorted); {label}", marker.mode_sorted),
                13,
            ),
            (90, format!("QE ph.x: {:.6} cm^-1", marker.qe_cm1), 14),
            (113, format!("EPW: {:.6} cm^-1", marker.epw_cm1), 14),
            (
                137,
                "Maximum over verified samples; not the entire Brillouin zone".into(),
                12,
            ),
        ] {
            card.draw(&Text::new(
                text,
                (12, y),
                ("sans-serif", size).into_font().color(&ERROR_PURPLE),
            ))?;
        }
        // 两个紫点分别对应参考和插值频率；连线长度不代表主图的路径距离。
        let x = width as i32 - 42;
        let (qe_y, epw_y) = if marker.qe_cm1 <= marker.epw_cm1 {
            (111, 87)
        } else {
            (87, 111)
        };
        card.draw(&PathElement::new(
            vec![(x, qe_y), (x, epw_y)],
            ERROR_PURPLE.stroke_width(2),
        ))?;
        for y in [qe_y, epw_y] {
            card.draw(&Circle::new((x, y), 5, ERROR_PURPLE.filled()))?;
        }
    }
    Ok(())
}

/// 显式截取线段，窗外数据不贴着边界生成虚假的水平能带。
fn clip(points: &[(f64, f64)], lo: f64, hi: f64) -> Vec<Vec<(f64, f64)>> {
    let mut paths: Vec<Vec<(f64, f64)>> = Vec::new();
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if (a.1 < lo && b.1 < lo) || (a.1 > hi && b.1 > hi) {
            continue;
        }
        let cut = |p: (f64, f64), other: (f64, f64)| {
            if p.1 >= lo && p.1 <= hi {
                p
            } else {
                let y = p.1.clamp(lo, hi);
                (p.0 + (other.0 - p.0) * (y - p.1) / (other.1 - p.1), y)
            }
        };
        let (a, b) = (cut(a, b), cut(b, a));
        if let Some(last) = paths.last_mut().filter(|p| p.last() == Some(&a)) {
            last.push(b);
        } else {
            paths.push(vec![a, b]);
        }
    }
    paths
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Figures {
    schema_version: u32,
    comparison_digest: String,
    files: Vec<Artifact>,
}

pub fn check_figures(results: &Path) -> Result<bool> {
    let output = results.join("figures");
    let path = output.join("comparison-figures.json");
    if !path.exists() {
        return Ok(false);
    }
    let record: Figures = store::read(&path)?;
    ensure!(
        record.schema_version == 1
            && record.comparison_digest
                == store::hash_file(&results.join("comparison/manifest.json"))?,
        "data_invalid: 图像清单来源不符"
    );
    for file in record.files {
        file.verify(&output)?;
    }
    Ok(true)
}

fn missing<DB: DrawingBackend>(
    area: &DrawingArea<DB, Shift>,
    title: &str,
    reason: &str,
) -> Result<()>
where
    DB::ErrorType: 'static,
{
    area.fill(&WHITE)?;
    area.draw(&Text::new(title, (40, 45), ("sans-serif", 23).into_font()))?;
    area.draw(&Text::new(
        reason,
        (40, 90),
        ("sans-serif", 16).into_font().color(&RGBColor(160, 60, 30)),
    ))?;
    Ok(())
}

fn chart<DB: DrawingBackend>(
    area: &DrawingArea<DB, Shift>,
    curves: &[Curve],
    data: &Dataset,
    kind: usize,
    projection: &str,
) -> Result<()>
where
    DB::ErrorType: 'static,
{
    let title = match kind {
        0 => "Electronic bands",
        1 => "Electronic DOS",
        2 => "Phonon dispersion",
        _ => "Phonon DOS",
    };
    let selected: Vec<_> = curves
        .iter()
        .filter(|c| c.projection == projection)
        .collect();
    if selected.is_empty() {
        return missing(
            area,
            &format!("{title} | {projection}"),
            "Missing paired data / verified projection",
        );
    }
    let xmin = if kind == 1 {
        data.metadata.window_ev[0]
    } else {
        selected
            .iter()
            .flat_map(|c| &c.points)
            .map(|p| p.0)
            .fold(f64::INFINITY, f64::min)
    };
    let xmax = if kind == 1 {
        data.metadata.window_ev[1]
    } else {
        selected
            .iter()
            .flat_map(|c| &c.points)
            .map(|p| p.0)
            .fold(f64::NEG_INFINITY, f64::max)
    };
    let (ymin, ymax) = if kind == 0 {
        (data.metadata.window_ev[0], data.metadata.window_ev[1])
    } else {
        let lo = selected
            .iter()
            .flat_map(|c| &c.points)
            .map(|p| p.1)
            .fold(0.0, f64::min);
        let hi = selected
            .iter()
            .flat_map(|c| &c.points)
            .map(|p| p.1)
            .fold(0.0, f64::max);
        (
            lo - if kind == 2 { 15.0 } else { 0.0 },
            hi + (hi - lo).abs().max(1e-8) * 0.08,
        )
    };
    let (xlabel, ylabel) = match kind {
        0 => ("k path (2pi/alat)", "E - EF (eV)"),
        1 => ("E - EF (eV)", "states / (eV cell)"),
        2 => ("q path (2pi/alat)", "Frequency (cm^-1)"),
        _ => ("Frequency (cm^-1)", "states / (cm^-1 cell)"),
    };
    let mut ctx = ChartBuilder::on(area)
        .caption(format!("{title} | {projection}"), ("sans-serif", 22))
        .margin(18)
        .x_label_area_size(46)
        .y_label_area_size(72)
        .build_cartesian_2d(xmin..xmax, ymin..ymax)?;
    ctx.configure_mesh()
        .x_desc(xlabel)
        .y_desc(ylabel)
        .label_style(("sans-serif", 16))
        .axis_desc_style(("sans-serif", 17))
        .max_light_lines(0)
        .bold_line_style(RGBColor(235, 235, 235))
        .draw()?;
    let ticks = if kind == 0 {
        &data.metadata.electronic_ticks
    } else {
        &data.metadata.phonon_ticks
    };
    if kind == 0 || kind == 2 {
        for tick in ticks {
            ctx.draw_series(std::iter::once(PathElement::new(
                vec![(tick.distance, ymin), (tick.distance, ymax)],
                RGBColor(200, 200, 200),
            )))?;
            // 路径标签放在绘图区上沿，避免与数值刻度混淆。
            ctx.draw_series(std::iter::once(Text::new(
                tick.label.clone(),
                (tick.distance, ymax - (ymax - ymin) * 0.035),
                ("sans-serif", 15).into_font(),
            )))?;
        }
    }
    if kind == 0 {
        ctx.draw_series(std::iter::once(PathElement::new(
            vec![(xmin, 0.0), (xmax, 0.0)],
            RGBColor(190, 190, 190),
        )))?;
    }
    // 全部 QE 图层先完成，再画全部 EPW 图层；不按分支交错覆盖。
    for method in ["qe", "epw"] {
        let color = match (projection.split(':').next().unwrap_or("total"), method) {
            ("Mg", "qe") => RGBColor(30, 69, 135),
            ("Mg", _) => RGBColor(75, 125, 210),
            ("B", "qe") => RGBColor(23, 98, 70),
            ("B", _) => RGBColor(42, 160, 112),
            (_, "qe") => RGBColor(45, 45, 55),
            _ => RGBColor(211, 65, 49),
        };
        let mut legend = false;
        for curve in &selected {
            if curve.method != method {
                continue;
            }
            // DOS 按能量横轴截取，能带按能量纵轴截取；保留真实边界交点。
            let paths = if kind == 1 {
                let swapped: Vec<_> = curve.points.iter().map(|p| (p.1, p.0)).collect();
                clip(&swapped, xmin, xmax)
                    .into_iter()
                    .map(|path| path.into_iter().map(|p| (p.1, p.0)).collect())
                    .collect()
            } else {
                clip(&curve.points, ymin, ymax)
            };
            for points in paths {
                if method == "qe" {
                    let series = ctx.draw_series(LineSeries::new(points, color.stroke_width(2)))?;
                    if !legend {
                        series
                            .label(if kind == 2 {
                                "QE q2r/matdyn (solid)"
                            } else {
                                "QE (solid)"
                            })
                            .legend(move |(x, y)| {
                                PathElement::new(vec![(x, y), (x + 30, y)], color.stroke_width(2))
                            });
                    }
                } else {
                    let series = ctx.draw_series(DashedLineSeries::new(
                        points,
                        9,
                        5,
                        color.stroke_width(2),
                    ))?;
                    if !legend {
                        series
                            .label("EPW / Wannier (dashed)")
                            .legend(move |(x, y)| {
                                DashedPathElement::new(
                                    vec![(x, y), (x + 30, y)],
                                    9,
                                    5,
                                    color.stroke_width(2),
                                )
                            });
                    }
                }
                legend = true;
            }
        }
    }
    if kind == 2 && !data.direct.is_empty() {
        ctx.draw_series(
            data.direct
                .iter()
                .copied()
                .filter(|p| p.0 >= xmin && p.0 <= xmax)
                .map(|p| Circle::new(p, 4, RGBColor(25, 105, 190).filled())),
        )?
        .label("QE ph.x (direct points)")
        .legend(|(x, y)| Circle::new((x + 15, y), 4, RGBColor(25, 105, 190).filled()));
    }
    if kind == 2 {
        if let Some(marker) = data
            .metadata
            .phonon_error_markers
            .iter()
            .find(|m| m.scope == "path_maximum")
        {
            if let Some(x) = marker.path_distance {
                ctx.draw_series(std::iter::once(PathElement::new(
                    vec![(x, marker.qe_cm1), (x, marker.epw_cm1)],
                    ERROR_PURPLE.stroke_width(3),
                )))?;
                ctx.draw_series(
                    [marker.qe_cm1, marker.epw_cm1]
                        .into_iter()
                        .map(|y| Circle::new((x, y), 6, ERROR_PURPLE.filled())),
                )?
                .label("Purple: max direct PH / EPW error on path")
                .legend(|(x, y)| Circle::new((x + 15, y), 5, ERROR_PURPLE.filled()));
                ctx.draw_series(std::iter::once(
                    EmptyElement::at((x, marker.qe_cm1.max(marker.epw_cm1)))
                        + Text::new(
                            format!("mode {}: {:.2} cm^-1", marker.mode_sorted, marker.error_cm1),
                            (10, -13),
                            ("sans-serif", 14).into_font().color(&ERROR_PURPLE),
                        ),
                ))?;
            }
        }
    }
    ctx.configure_series_labels()
        .background_style(WHITE.mix(0.9))
        .border_style(RGBColor(180, 180, 180))
        .label_font(("sans-serif", 14))
        .position(if kind == 0 || kind == 2 {
            SeriesLabelPosition::LowerRight
        } else {
            SeriesLabelPosition::UpperMiddle
        })
        .draw()?;
    Ok(())
}

fn render<DB: DrawingBackend>(
    area: DrawingArea<DB, Shift>,
    data: &Dataset,
    kind: usize,
) -> Result<()>
where
    DB::ErrorType: 'static,
{
    area.fill(&WHITE)?;
    let banner = format!(
        "{} | {} | quality: {} | {}",
        data.metadata.title,
        data.manifest.ph_mode,
        data.manifest.quality_status,
        if data.manifest.missing.is_empty() {
            "diagnostic"
        } else {
            "PARTIAL RESULTS"
        }
    );
    let body = area.titled(&banner, ("sans-serif", 21))?;
    let body = body.titled(
        &format!(
            "Common EF = {:.6} eV; electronic window = [{:.3}, {:.3}] eV relative to EF",
            data.metadata.fermi_ev, data.metadata.window_ev[0], data.metadata.window_ev[1]
        ),
        ("sans-serif", 15),
    )?;
    let body = if let Some(window) = data.metadata.disentanglement_window_ev {
        body.titled(
            &format!(
                "Wannier outer window (absolute eV): dis_win_min = {:.5}; dis_win_max = {:.5}",
                window[0], window[1]
            ),
            ("sans-serif", 15),
        )?
    } else {
        body
    };
    let body = if let Some(note) = data.metadata.notes.first() {
        body.titled(note, ("sans-serif", 15))?
    } else {
        body
    };
    if kind == 4 {
        let panels = body.split_evenly((2, 2));
        for (i, curves) in [
            &data.electronic,
            &data.electronic_dos,
            &data.phonon,
            &data.phonon_dos,
        ]
        .iter()
        .enumerate()
        {
            if i == 2 {
                phonon_panel(&panels[i], data)?;
            } else {
                chart(&panels[i], curves, data, i, "total")?;
            }
        }
    } else if kind == 1 || kind == 3 {
        let mut projections = vec!["total".to_string()];
        projections.extend(data.metadata.elements.clone());
        if kind == 1 {
            projections.extend(data.metadata.orbitals.clone());
        }
        let panels = body.split_evenly((projections.len().div_ceil(2), 2));
        let curves = if kind == 1 {
            &data.electronic_dos
        } else {
            &data.phonon_dos
        };
        for (panel, p) in panels.iter().zip(projections) {
            chart(panel, curves, data, kind, &p)?;
        }
    } else if kind == 2 {
        phonon_panel(&body, data)?;
    } else {
        chart(
            &body,
            if kind == 0 {
                &data.electronic
            } else {
                &data.phonon
            },
            data,
            kind,
            "total",
        )?;
    }
    area.present()?;
    Ok(())
}

pub fn plot(results: &Path, output: Option<&Path>, formats: &[String]) -> Result<PathBuf> {
    ensure!(
        !formats.is_empty()
            && formats
                .iter()
                .all(|f| matches!(f.as_str(), "png" | "svg" | "pdf")),
        "data_invalid: 格式只能是 png,svg,pdf"
    );
    let root = results.join("comparison");
    let data = comparison::load(&root)?;
    let output = output
        .map(PathBuf::from)
        .unwrap_or_else(|| results.join("figures"));
    fs::create_dir_all(&output)?;
    let _lock = store::Lock::acquire(&output.join("comparison-figures"))?;
    ensure!(
        !output.canonicalize()?.starts_with(root.canonicalize()?),
        "input_conflict: 图像输出不能覆盖比较数据"
    );
    let previous = output.join("comparison-figures.json");
    let owned = if previous.exists() {
        let old: Figures = store::read(&previous)?;
        ensure!(old.schema_version == 1, "data_invalid: 图像清单版本未知");
        for f in &old.files {
            f.verify(&output)?;
        }
        old.files
            .into_iter()
            .map(|f| f.relative_path)
            .collect::<std::collections::BTreeSet<_>>()
    } else {
        Default::default()
    };
    for name in KINDS {
        for format in formats {
            let file = PathBuf::from(format!("{name}.{format}"));
            ensure!(
                !output.join(&file).exists() || owned.contains(&file),
                "input_conflict: 未知图像冲突 {}",
                file.display()
            );
        }
    }
    let stage = output.join(format!(".comparison-{}", store::token()));
    fs::create_dir(&stage)?;
    let mut files = Vec::new();
    for (i, name) in KINDS.iter().enumerate() {
        let size = if i == 4 {
            (1800, 1300)
        } else if i == 1 {
            (
                1500,
                ((data.metadata.elements.len() + data.metadata.orbitals.len() + 1).div_ceil(2)
                    as u32)
                    * 420
                    + 50,
            )
        } else if i == 3 {
            (1500, 900)
        } else {
            (1300, 800)
        };
        if formats.iter().any(|f| f == "svg" || f == "pdf") {
            let svg = stage.join(format!("{name}.svg"));
            render(SVGBackend::new(&svg, size).into_drawing_area(), &data, i)?;
            if formats.iter().any(|f| f == "pdf") {
                let mut options = svg2pdf::usvg::Options::default();
                options.fontdb_mut().load_system_fonts();
                let tree = svg2pdf::usvg::Tree::from_str(&fs::read_to_string(&svg)?, &options)?;
                let pdf = svg2pdf::to_pdf(&tree, Default::default(), Default::default())
                    .map_err(|e| anyhow::anyhow!("PDF 转换失败: {e}"))?;
                fs::write(stage.join(format!("{name}.pdf")), pdf)?;
            }
        }
        if formats.iter().any(|f| f == "png") {
            render(
                BitMapBackend::new(&stage.join(format!("{name}.png")), size).into_drawing_area(),
                &data,
                i,
            )?;
        }
        for format in formats {
            files.push(Artifact::capture(&stage, &format!("{name}.{format}"))?);
        }
    }
    let record = Figures {
        schema_version: 1,
        comparison_digest: store::hash_file(&root.join("manifest.json"))?,
        files,
    };
    for f in &record.files {
        fs::rename(stage.join(&f.relative_path), output.join(&f.relative_path))?;
    }
    store::write(&previous, &record)?;
    // PDF-only 的中间 SVG 仅属于本次 staging，可定向移除。
    for name in KINDS {
        let svg = stage.join(format!("{name}.svg"));
        if svg.exists() {
            fs::remove_file(svg)?;
        }
    }
    fs::remove_dir(stage)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&comparison::summary(&data))?
    );
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::clip;
    #[test]
    fn window_clipping_does_not_invent_horizontal_boundary_bands() {
        assert!(clip(&[(0., 2.), (1., 3.)], -0.5, 0.5).is_empty());
        let clipped = clip(&[(0., -1.), (1., 1.)], -0.5, 0.5);
        assert_eq!(clipped, vec![vec![(0.25, -0.5), (0.75, 0.5)]]);
        let separated = clip(&[(0., 0.), (1., 2.), (2., 2.), (3., 0.)], -0.5, 0.5);
        assert_eq!(separated.len(), 2);
    }
}
