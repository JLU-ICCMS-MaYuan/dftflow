//! 能带与 DOS 的共同纵轴图；图参自包含，重绘不访问科学程序。
use super::{
    config::Config, electronic_style as style, stages::Step, store, workspace_plot::PlotData,
};
use anyhow::{ensure, Context, Result};
use plotters::{
    prelude::*,
    style::text_anchor::{HPos, Pos, VPos},
};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

pub const VERSION: u32 = 1;
pub const ELECTRONIC_FILES: &[&str] = &[
    "electronic.png",
    "electronic.plot.json",
    "electronic-from-lowest-to-highest.png",
    "electronic-from-lowest-to-highest.plot.json",
];
#[derive(Clone, Serialize, Deserialize)]
pub struct Curve {
    pub name: String,
    pub points: Vec<(f64, f64)>,
    pub color: [u8; 3],
    pub dashed: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Figure {
    pub kind: String,
    pub renderer_version: u32,
    pub title: String,
    pub bounds: [f64; 2],
    pub bands: Vec<Curve>,
    /// 保留 (能量或频率, DOS) 原始列序；渲染时转置。
    pub dos: Vec<Curve>,
    pub labels: Vec<(f64, String)>,
    pub window: Vec<(f64, String, [u8; 3])>,
    pub notes: Vec<String>,
}
fn rgb(c: RGBColor) -> [u8; 3] {
    [c.0, c.1, c.2]
}
fn dashed_stroke(name: &str, color: RGBColor) -> style::ComparisonStroke {
    if name == "EPW" {
        return style::INTERPOLATED_STROKE;
    }
    if name.starts_with("dis_") {
        return style::ComparisonStroke {
            dash: 4,
            gap: 4,
            width: 1,
        };
    }
    style::ELEMENTS
        .iter()
        .position(|c| *c == color)
        .map(style::element_stroke)
        .unwrap_or(style::INTERPOLATED_STROKE)
}
fn curve(name: &str, points: Vec<(f64, f64)>, color: RGBColor, dashed: bool) -> Curve {
    Curve {
        name: name.into(),
        points,
        color: rgb(color),
        dashed,
    }
}
/// 当前现场重绘只消费已保存数值；旧分离图参定向迁移，不执行科学后处理。
pub fn redraw_stage(cfg: &Config, step: Step) -> Result<Option<std::path::PathBuf>> {
    ensure!(
        matches!(step, Step::Electronic | Step::Wannier),
        "configuration: electronic redraw supports only stages 3 and 5"
    );
    let stage = step.directory(cfg);
    let output = super::stage_results::root(cfg).join(step.directory(cfg).file_name().unwrap());
    let near = output.join("electronic.plot.json");
    if near.is_file() {
        let mut data: Figure = store::read(&near)?;
        data.bounds = cfg.plot.bounds();
        write(&output.join("electronic.png"), &data)?;
        redraw(&output.join("electronic-from-lowest-to-highest.plot.json"))?;
        let metadata = output.join("electronic-display.json");
        if metadata.is_file() {
            let mut data: serde_json::Value = store::read(&metadata)?;
            data["near_ev"] = serde_json::json!(cfg.plot.bounds());
            store::write(&metadata, &data)?;
        }
        super::readable_state::update(&cfg.work_dir,step,|doc| {doc.remove("Plot failure");Ok(())})?;
        return Ok(Some(output));
    }
    let sources = if step == Step::Electronic {
        [stage.join("qe_eband/plots"), stage.join("qe_eledos/plots")]
    } else {
        let source = super::layout::path(&cfg.work_dir, "comparison-native/electron");
        [source.clone(), source]
    };
    let mut inputs = Vec::new();
    for (name, source) in ["electronic-bands", "electronic-dos"].iter().zip(&sources) {
        let candidates = [
            output.join(format!("{name}.json")),
            source.join(format!("{name}.json")),
            source.join(format!("{name}.plot.json")),
        ];
        let Some(path) = candidates.into_iter().find(|p| p.is_file()) else {
            return Ok(None);
        };
        inputs.push((store::Identity::capture(&path)?, format!("{name}.json")));
    }
    fs::create_dir_all(&output)?;
    for (id, name) in &inputs {
        let target = output.join(name);
        if id.path != target {
            fs::copy(&id.path, &target)?;
        }
        id.verify()?;
    }
    electronic(cfg, &output, &output)?;
    for source in sources {
        cleanup(&source)?;
    }
    Ok(Some(output))
}
pub fn electronic(cfg: &Config, source: &Path, output: &Path) -> Result<()> {
    let PlotData::Bands {
        qe,
        wan,
        window,
        labels,
        note,
        ..
    } = store::read(&source.join("electronic-bands.json"))?
    else {
        anyhow::bail!("data_invalid: electronic bands are missing");
    };
    let PlotData::Dos {
        qe: density,
        wan: interpolated_density,
        elements,
        note: dos_note,
        ..
    } = store::read(&source.join("electronic-dos.json"))?
    else {
        anyhow::bail!("data_invalid: electronic DOS is missing");
    };
    let lo = qe
        .iter()
        .flat_map(|b| &b.1)
        .copied()
        .fold(f64::INFINITY, f64::min);
    let hi = qe
        .iter()
        .flat_map(|b| &b.1)
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    ensure!(
        lo.is_finite() && hi.is_finite() && lo < hi,
        "data_invalid: invalid QE path energy extrema"
    );
    let comparison = wan.is_some();
    let mut bands = vec![];
    for (name, curves, color, dashed) in [
        ("QE", Some(qe), style::QE, false),
        ("EPW", wan, style::INTERPOLATED, true),
    ] {
        for (x, y) in curves.unwrap_or_default() {
            let points = x.into_iter().zip(y).collect::<Vec<_>>();
            // 同一横坐标的段尾／段首不跨段连接。
            let mut start = 0;
            for i in 1..=points.len() {
                if i == points.len() || points[i].0 <= points[i - 1].0 {
                    if i - start >= 2 {
                        bands.push(curve(name, points[start..i].to_vec(), color, dashed));
                    }
                    start = i;
                }
            }
        }
    }
    let mut dos = vec![curve("QE", density, style::QE, false)];
    for (i, (name, rows)) in elements.into_iter().enumerate() {
        dos.push(curve(
            &format!("{name} PDOS"),
            rows,
            style::ELEMENTS[i % style::ELEMENTS.len()],
            true,
        ));
    }
    if let Some(rows) = interpolated_density {
        dos.push(curve("EPW", rows, style::INTERPOLATED, true));
    }
    let mut notes = vec![note, dos_note]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    for c in &dos {
        if let (Some(a), Some(b)) = (c.points.first(), c.points.last()) {
            notes.push(format!(
                "{} DOS support [{:.3}, {:.3}] eV; uncovered ranges remain empty",
                c.name, a.0, b.0
            ));
        }
    }
    let mut fig = Figure {
        kind: "Combined".into(),
        renderer_version: VERSION,
        title: if comparison {
            "QE / EPW electronic bands and DOS"
        } else {
            "QE electronic bands and DOS"
        }
        .into(),
        bounds: cfg.plot.bounds(),
        bands,
        dos,
        labels,
        window,
        notes,
    };
    fs::create_dir_all(output)?;
    let mut full=fig.clone(); full.bounds=[lo,hi];
    store::write(&output.join("electronic-from-lowest-to-highest.plot.json"),&full)?;
    write(&output.join("electronic.png"), &fig)?;
    fig.bounds = [lo, hi];
    write(&output.join("electronic-from-lowest-to-highest.png"), &fig)?;
    cleanup(output)?;
    Ok(())
}
pub fn write(file: &Path, data: &Figure) -> Result<()> {
    // 正常绘制只有一次；异常由所属阶段报告，不自行降级或重试。

    store::write(&file.with_extension("plot.json"), data).context("plot_failed: plot data output failed")?;
    let svg = svg(data).context("plot_failed: combined figure rendering failed")?;
    style::png(&svg, file, 3600, 2400).context("plot_failed: PNG output failed")
}
pub fn redraw(file: &Path) -> Result<()> {
    let data: Figure = store::read(file)?;
    ensure!(
        data.kind == "Combined",
        "data_invalid: invalid combined figure kind"
    );
    write(&file.with_extension("").with_extension("png"), &data)
}

pub fn svg(data: &Figure) -> Result<String> {
    let [lo, hi] = data.bounds;
    ensure!(
        lo.is_finite() && hi.is_finite() && lo < hi,
        "data_invalid: invalid combined figure bounds"
    );
    let xmax = data
        .bands
        .iter()
        .flat_map(|c| &c.points)
        .map(|p| p.0)
        .fold(0., f64::max);
    ensure!(xmax > 0., "data_invalid: combined figure path is empty");
    ensure!(
        data.bands
            .iter()
            .chain(&data.dos)
            .all(|c| c.points.iter().all(|p| p.0.is_finite() && p.1.is_finite())),
        "data_invalid: non-finite combined figure data"
    );
    let dmax = data
        .dos
        .iter()
        .flat_map(|c| clipped_dos(&c.points, lo, hi))
        .map(|p| p.0)
        .fold(0., f64::max)
        .max(1e-6)
        * 1.06;
    let mut svg = String::new();
    {
        let area = SVGBackend::with_string(&mut svg, (1200, 800)).into_drawing_area();
        area.fill(&WHITE)?;
        area.draw(&Text::new(
            data.title.clone(),
            (70, 30),
            ("sans-serif", 24).into_font(),
        ))?;
        let (body, footer) = area.split_vertically(620);
        let (panels, legend) = body.split_horizontally(960);
        let (left, right) = panels.split_horizontally(650);
        // 两侧完全一致的顶／底留白，保证实际绘图区逐像素对齐。
        let mut bands = ChartBuilder::on(&left)
            .margin_top(60)
            .margin_bottom(18)
            .margin_left(14)
            .margin_right(10)
            .x_label_area_size(44)
            .y_label_area_size(65)
            .build_cartesian_2d(0.0..xmax, lo..hi)?;
        let mut dos = ChartBuilder::on(&right)
            .margin_top(60)
            .margin_bottom(18)
            .margin_left(16)
            .margin_right(16)
            .x_label_area_size(44)
            .y_label_area_size(0)
            .build_cartesian_2d(0.0..dmax, lo..hi)?;
        let phonon = data.title.contains("phonon");
        bands
            .configure_mesh()
            .x_labels(0)
            .max_light_lines(0)
            .disable_x_mesh()
            .y_desc(if phonon {
                "Frequency (cm⁻¹)"
            } else {
                "E - E_F (eV)"
            })
            .bold_line_style(RGBColor(235, 235, 235))
            .draw()?;
        dos.configure_mesh()
            .y_labels(0)
            .disable_y_axis()
            .max_light_lines(0)
            .disable_x_mesh()
            .x_desc(if phonon {
                "DOS (states/cm⁻¹/cell)"
            } else {
                "DOS (states/eV/cell)"
            })
            .bold_line_style(RGBColor(235, 235, 235))
            .draw()?;
        for (x, _) in &data.labels {
            if *x >= 0. && *x <= xmax {
                bands.draw_series(std::iter::once(PathElement::new(
                    [(*x, lo), (*x, hi)],
                    RGBColor(220, 220, 220),
                )))?;
            }
        }
        for c in &data.bands {
            let color = RGBColor(c.color[0], c.color[1], c.color[2]);
            for points in crate::delivery::comparison_plot::clip(&c.points, lo, hi) {
                if c.dashed {
                    bands.draw_series(DashedLineSeries::new(
                        points,
                        style::INTERPOLATED_STROKE.dash,
                        style::INTERPOLATED_STROKE.gap,
                        color.stroke_width(style::INTERPOLATED_STROKE.width),
                    ))?;
                } else {
                    bands.draw_series(LineSeries::new(
                        points,
                        color.stroke_width(style::REFERENCE_WIDTH),
                    ))?;
                }
            }
        }
        for c in &data.dos {
            let color = RGBColor(c.color[0], c.color[1], c.color[2]);
            let points = clipped_dos(&c.points, lo, hi);
            if c.dashed {
                let stroke = dashed_stroke(&c.name, color);
                dos.draw_series(DashedLineSeries::new(
                    points,
                    stroke.dash,
                    stroke.gap,
                    color.stroke_width(stroke.width),
                ))?;
            } else {
                dos.draw_series(LineSeries::new(
                    points,
                    color.stroke_width(style::REFERENCE_WIDTH),
                ))?;
            }
        }
        for (e, _, color) in &data.window {
            if *e < lo || *e > hi {
                continue;
            }
            let color = RGBColor(color[0], color[1], color[2]);
            bands.draw_series(DashedLineSeries::new(
                [(0., *e), (xmax, *e)],
                4,
                4,
                color.stroke_width(1),
            ))?;
            dos.draw_series(DashedLineSeries::new(
                [(0., *e), (dmax, *e)],
                4,
                4,
                color.stroke_width(1),
            ))?;
        }
        if !phonon && lo <= 0. && hi >= 0. {
            bands.draw_series(std::iter::once(PathElement::new(
                [(0., 0.), (xmax, 0.)],
                style::FERMI,
            )))?;
            dos.draw_series(std::iter::once(PathElement::new(
                [(0., 0.), (dmax, 0.)],
                style::FERMI,
            )))?;
        }
        let mut labels: Vec<(i32, i32, String)> = vec![];
        for (x, label) in &data.labels {
            let (px, py) = bands.backend_coord(&(*x, lo));
            let label = if matches!(label.as_str(), "G" | "GAMMA") {
                "Γ"
            } else {
                label
            }
            .to_string();
            if let Some((last_x, _, last_label)) = labels.last_mut() {
                if px - *last_x < 26 {
                    *last_x = (*last_x + px) / 2;
                    if !last_label.split(" | ").any(|s| s == label) {
                        *last_label += &format!(" | {label}");
                    }
                    continue;
                }
            }
            labels.push((px, py, label));
        }
        for (px, py, label) in labels {
            left.draw(&Text::new(
                label,
                (px, py + 12),
                TextStyle::from(("sans-serif", 17).into_font())
                    .pos(Pos::new(HPos::Center, VPos::Top)),
            ))?;
        }
        // 图例独占右侧留白，曲线与图例的线型一致。
        let mut entries: Vec<(String, [u8; 3], bool)> = vec![];
        for c in data.bands.iter().chain(&data.dos) {
            if !entries.iter().any(|e| e.0 == c.name) {
                entries.push((c.name.clone(), c.color, c.dashed));
            }
        }
        entries.extend(
            data.window
                .iter()
                .map(|(e, n, c)| (format!("{n}: {e:.3} eV"), *c, true)),
        );
        if !phonon {
            entries.push(("E_F = 0 eV".into(), rgb(style::FERMI), false));
        }
        for (i, (name, c, dashed)) in entries.into_iter().enumerate() {
            let y = 80 + i as i32 * 27;
            let color = RGBColor(c[0], c[1], c[2]);
            if dashed {
                let stroke = dashed_stroke(&name, color);
                legend.draw(&plotters::element::DashedPathElement::new(
                    [(5, y), (30, y)],
                    stroke.dash,
                    stroke.gap,
                    color.stroke_width(stroke.width),
                ))?;
            } else {
                legend.draw(&PathElement::new(
                    [(5, y), (30, y)],
                    color.stroke_width(if name == "QE" {
                        style::REFERENCE_WIDTH
                    } else {
                        1
                    }),
                ))?;
            }
            legend.draw(&Text::new(name, (37, y), ("sans-serif", 12).into_font()))?;
        }
        for (i, note) in data.notes.iter().enumerate() {
            footer.draw(&Text::new(
                note.clone(),
                (70, 22 + i as i32 * 20),
                ("sans-serif", 12).into_font(),
            ))?;
        }
        area.present()?;
    }
    Ok(svg)
}
fn clipped_dos(points: &[(f64, f64)], lo: f64, hi: f64) -> Vec<(f64, f64)> {
    let swapped = points.iter().map(|(e, d)| (*d, *e)).collect::<Vec<_>>();
    crate::delivery::comparison_plot::clip(&swapped, lo, hi)
        .into_iter()
        .flatten()
        .collect()
}

/// 只清理由旧生成端明确拥有的图像及其图参。
pub fn cleanup(dir: &Path) -> Result<()> {
    for name in [
        "electronic-bands",
        "electronic-dos",
        "electronic-bands-valence",
        "electronic-dos-valence",
    ] {
        for ext in ["png", "plot.json"] {
            let path = dir.join(format!("{name}.{ext}"));
            if path.is_file() {
                fs::remove_file(path)?;
            }
        }
    }
    Ok(())
}

pub fn coarse(cfg: &Config, _results: &Path) -> Result<()> {
    super::stage_results::retire_units(cfg, Step::Coarse, &["figures"])?;
    retire_comparison_figures(&super::stage_results::root(cfg).join("6.epw-coarse/figures"))?;
    let data = crate::interpolation::comparison_data::load(&super::layout::path(&cfg.work_dir,"interpolation/comparison"))?;
    let mut bands = vec![];
    let mut dos = vec![];
    for (target, curves) in [(&mut bands, &data.phonon), (&mut dos, &data.phonon_dos)] {
        for method in ["QE", "EPW"] {
            for c in curves
                .iter()
                .filter(|c| c.method.eq_ignore_ascii_case(method) && c.projection == "total")
            {
                target.push(curve(
                    method,
                    c.points.clone(),
                    if method == "QE" {
                        style::QE
                    } else {
                        style::INTERPOLATED
                    },
                    method == "EPW",
                ));
            }
        }
    }
    let maximum = bands
        .iter()
        .flat_map(|c| &c.points)
        .map(|p| p.1)
        .fold(0., f64::max);
    ensure!(maximum > 0., "data_invalid: no positive phonon frequency");
    let mut notes =
        vec!["QE q2r/matdyn / EPW; negative frequencies retained in data and validation".into()];
    let treatment = data.metadata.notes.iter().filter(|s|s.starts_with("QE stage 4 ")).cloned().collect::<Vec<_>>();
    if !treatment.is_empty() {
        notes.push(treatment.join("; "));
        notes.push(format!("QE native stage-4 DOS; EPW Gaussian sigma={} cm-1. Differences include treatment/sampling effects.",
            cfg.comparison.clone().unwrap_or_default().phonon_sigma_cm1));
    }
    if let Some(d) = data.differences {
        if let Some(m) = d.phonon_path.maximum {
            notes.push(format!(
                "Path max |QE - EPW| = {:.6} cm⁻¹",
                m.absolute_difference
            ));
        }
    }
    let fig = Figure {
        kind: "Combined".into(),
        renderer_version: VERSION,
        title: "QE / EPW phonon dispersion and DOS".into(),
        bounds: [0., maximum],
        bands,
        dos,
        labels: data
            .metadata
            .phonon_ticks
            .into_iter()
            .map(|t| (t.distance, t.label))
            .collect(),
        window: vec![],
        notes,
    };
    let dir = super::stage_results::root(cfg).join("6.epw-coarse");
    fs::create_dir_all(&dir)?;
    write(&dir.join("phonon.png"), &fig)?;
    super::stage_results::publish(
        cfg,
        Step::Coarse,
        "plot",
        &dir,
        &["phonon.png".into(), "phonon.plot.json".into()],
        serde_json::json!({"kind":"phonon_combined","renderer_version":VERSION}),
        vec![],
    )?;
    Ok(())
}
pub fn qe_phonon(
    cfg: &Config,
    dir: &Path,
    elements: &[String],
    counts: &[usize],
    sigma: f64,
) -> Result<()> {
    let band =
        qf_phonon::band::BandData::load(dir.to_str().context("non-UTF-8 path")?, &cfg.qe.prefix)?;
    let mut bands = vec![];
    for mode in 0..band.frequencies[0].len() {
        for range in &band.segments {
            bands.push(curve(
                "QE",
                range
                    .clone()
                    .map(|i| (band.points[i].distance, band.frequencies[i][mode]))
                    .collect(),
                style::QE,
                false,
            ));
        }
    }
    let (rows, names) = qf_phonon::plot::combined_dos_data(
        dir.to_str().unwrap(),
        &cfg.qe.prefix,
        elements,
        counts,
        sigma,
    )?;
    let dos = names
        .into_iter()
        .enumerate()
        .map(|(i, n)| {
            curve(
                if i == 0 { "QE" } else { &n },
                rows.iter().map(|r| (r[0], r[i + 1])).collect(),
                if i == 0 {
                    style::QE
                } else {
                    style::ELEMENTS[(i - 1) % style::ELEMENTS.len()]
                },
                i > 0,
            )
        })
        .collect();
    let maximum = band
        .frequencies
        .iter()
        .flatten()
        .copied()
        .fold(0., f64::max);
    ensure!(maximum > 0., "data_invalid: no positive phonon frequency");
    write(
        &dir.join("phonon.png"),
        &Figure {
            kind: "Combined".into(),
            renderer_version: VERSION,
            title: "QE phonon dispersion and DOS".into(),
            bounds: [0., maximum],
            bands,
            dos,
            labels: band.ticks,
            window: vec![],
            notes: vec!["Negative frequencies retained in data and validation".into()],
        },
    )
}

fn retire_comparison_figures(dir: &Path) -> Result<()> {
    let manifest = dir.join("comparison-figures.json");
    if !manifest.is_file() {
        return Ok(());
    }
    crate::delivery::publication::ensure_finished(dir, "comparison-figures.json")?;
    let record: serde_json::Value = store::read(&manifest)?;
    let files: Vec<crate::interpolation::comparison_data::Artifact> =
        serde_json::from_value(record["files"].clone())?;
    for f in files {
        let path = f.verify(dir)?;
        fs::remove_file(path)?;
    }
    fs::remove_file(manifest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_axis_and_transposed_dos_keep_bounds_and_uncovered_ranges_empty() {
        let data = Figure {
            kind: "Combined".into(),
            renderer_version: VERSION,
            title: "QE bands and DOS".into(),
            bounds: [-3., 3.],
            bands: vec![curve("QE", vec![(0., -8.), (1., 8.)], style::QE, false)],
            dos: vec![curve(
                "QE",
                vec![(-2., 1.), (0., 2.), (2., 1.)],
                style::QE,
                false,
            )],
            labels: vec![(0., "G".into()), (1., "X".into())],
            window: vec![],
            notes: vec![],
        };
        let text = svg(&data).unwrap();
        assert!(text.contains("E - E_F (eV)") && text.contains("DOS (states/eV/cell)"));
        let red: Vec<_> = text
            .lines()
            .filter(|l| l.contains("#DC0000") && l.contains("polyline"))
            .take(2)
            .collect();
        assert_eq!(red.len(), 2);
        let ys = red
            .iter()
            .map(|l| {
                l.split("points=\"")
                    .nth(1)
                    .unwrap()
                    .split(',')
                    .nth(1)
                    .unwrap()
                    .split_whitespace()
                    .next()
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(ys[0], ys[1]);
        assert_eq!(
            clipped_dos(&data.dos[0].points, -3., 3.),
            vec![(1., -2.), (2., 0.), (1., 2.)]
        );
        let cut = clipped_dos(&[(-5., 1.), (5., 3.)], -3., 3.);
        assert!((cut[0].0 - 1.4).abs() < 1e-12 && (cut[1].0 - 2.6).abs() < 1e-12);
        assert_eq!((cut[0].1, cut[1].1), (-3., 3.));
    }
    #[test]
    fn plot_config_defaults_and_independent_limits_are_validated() {
        let default: super::super::config::PlotConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(default.bounds(), [-3., 3.]);
        let custom: super::super::config::PlotConfig = serde_json::from_str(
            r#"{"electronic_energy_min_ev":-1.5,"electronic_energy_max_ev":4.0}"#,
        )
        .unwrap();
        custom.validate().unwrap();
        let partial: super::super::config::PlotConfig =
            serde_json::from_str(r#"{"electronic_energy_max_ev":4.0}"#).unwrap();
        assert_eq!(partial.bounds(), [-3., 4.]);
        for bounds in [[3., -3.], [1., 1.], [f64::NAN, 3.], [0., f64::INFINITY]] {
            assert!(super::super::config::PlotConfig {
                electronic_energy_min_ev: bounds[0],
                electronic_energy_max_ev: bounds[1]
            }
            .validate()
            .is_err());
        }
    }
}
