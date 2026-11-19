//! 超导分析图共用样式及直方图、双轴、散点和温度曲线原语。
use anyhow::{ensure, Result};
use plotters::prelude::*;

pub struct ScColors;
impl ScColors {
    pub const BLACK: RGBColor = RGBColor(0, 0, 0);
    pub const LAMBDA: RGBColor = RGBColor(220, 20, 60);
    pub const TEMPERATURE: RGBColor = RGBColor(30, 144, 255);
    pub const REAL: RGBAColor = RGBAColor(128, 128, 128, 0.3);
}
pub struct ScStyle;
impl ScStyle {
    pub const PANEL: (u32, u32) = (600, 600);
    pub const LINE_WIDTH: u32 = 1;
    pub const DPI: u32 = 300;
}
/// 复用共享导出器；PNG 写入 300 DPI 的物理分辨率元数据。
pub fn save(
    base: &str,
    formats: &[qf_config::PlotFormat],
    panels: &[Panel],
) -> Result<Vec<String>> {
    let files = crate::save_plot(base, formats, |svg| draw(svg, panels))?;
    for path in &files {
        if path.ends_with(".png") {
            let mut reader = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path)?))
                .read_info()?;
            let mut pixels = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut pixels)?;
            let mut encoded = Vec::new();
            {
                let mut encoder = png::Encoder::new(&mut encoded, info.width, info.height);
                encoder.set_color(info.color_type);
                encoder.set_depth(info.bit_depth);
                let pixels_per_meter = (ScStyle::DPI as f64 / 0.0254).round() as u32;
                encoder.set_pixel_dims(Some(png::PixelDimensions {
                    xppu: pixels_per_meter,
                    yppu: pixels_per_meter,
                    unit: png::Unit::Meter,
                }));
                encoder
                    .write_header()?
                    .write_image_data(&pixels[..info.buffer_size()])?;
            }
            std::fs::write(path, encoded)?;
        }
    }
    Ok(files)
}
#[derive(Clone, Copy)]
pub enum Mark {
    Line,
    Scatter,
    PointsLine,
    Dashed,
    Bars,
}
pub struct Curve {
    pub points: Vec<(f64, f64)>,
    pub color: RGBAColor,
    pub mark: Mark,
    pub radius: i32,
    pub label: String,
}
impl Curve {
    pub fn new(
        points: Vec<(f64, f64)>,
        color: RGBAColor,
        mark: Mark,
        label: impl Into<String>,
    ) -> Self {
        Self {
            points,
            color,
            mark,
            radius: 3,
            label: label.into(),
        }
    }
}
pub struct Panel {
    pub title: String,
    pub x_label: String,
    pub y_label: String,
    pub curves: Vec<Curve>,
    pub secondary: Option<Curve>,
}
impl Panel {
    pub fn new(title: impl Into<String>, x: &str, y: &str) -> Self {
        Self {
            title: title.into(),
            x_label: x.into(),
            y_label: y.into(),
            curves: vec![],
            secondary: None,
        }
    }
}
/// Freedman–Diaconis 分箱；权重参与密度归一化，常数样本仍有有限箱宽。
pub fn histogram(values: &[f64], weights: &[f64]) -> Result<Vec<(f64, f64)>> {
    ensure!(
        values.len() == weights.len(),
        "histogram weights length mismatch"
    );
    if values.is_empty() {
        return Ok(vec![]);
    }
    ensure!(
        values.iter().all(|v| v.is_finite()) && weights.iter().all(|w| w.is_finite() && *w >= 0.),
        "invalid histogram data"
    );
    let total: f64 = weights.iter().sum();
    ensure!(total > 0., "histogram has zero total weight");
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    let lo = sorted[0];
    let hi = sorted[n - 1];
    let iqr = sorted[3 * (n - 1) / 4] - sorted[(n - 1) / 4];
    let fd = 2. * iqr / (n as f64).cbrt();
    let bins = if hi == lo {
        1
    } else if fd > 0. {
        ((hi - lo) / fd).ceil() as usize
    } else {
        (n as f64).sqrt().ceil() as usize
    }
    .clamp(1, 512);
    let span = if hi > lo {
        hi - lo
    } else {
        lo.abs().max(1.) * 0.1
    };
    let start = if hi > lo { lo } else { lo - span / 2. };
    let width = span / bins as f64;
    let mut counts = vec![0.; bins];
    for (&v, &w) in values.iter().zip(weights) {
        let i = (((v - start) / width).floor().max(0.) as usize).min(bins - 1);
        counts[i] += w;
    }
    Ok(counts
        .iter()
        .enumerate()
        .map(|(i, c)| (start + (i as f64 + 0.5) * width, c / total / width))
        .collect())
}
fn range(values: impl Iterator<Item = f64>, zero: bool) -> std::ops::Range<f64> {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for v in values {
        lo = lo.min(v);
        hi = hi.max(v);
    }
    if !lo.is_finite() {
        return 0. ..1.;
    }
    if zero {
        lo = lo.min(0.);
        hi = hi.max(0.);
    }
    let pad = (hi - lo).abs().max(hi.abs() * 0.01).max(1e-6) * 0.06;
    (lo - pad)..(hi + pad)
}
fn bar_half_width(curve: &Curve) -> f64 {
    let dx = curve
        .points
        .windows(2)
        .map(|p| (p[1].0 - p[0].0).abs())
        .fold(f64::INFINITY, f64::min);
    if dx.is_finite() {
        dx * 0.5
    } else {
        curve
            .points
            .first()
            .filter(|p| p.1 > 0.)
            .map(|p| 0.5 / p.1)
            .unwrap_or(0.05)
    }
}
/// 一次输出 SVG，再由共享 save_plot 转换所需格式；缺失子图保留说明。
pub fn draw(svg: &mut String, panels: &[Panel]) -> Result<()> {
    ensure!(!panels.is_empty(), "empty superconductivity plot");
    svg.push_str(&format!(
        "<svg width=\"{}\" height=\"600\" xmlns=\"http://www.w3.org/2000/svg\">\n",
        ScStyle::PANEL.0 * panels.len() as u32
    ));
    for (index, panel) in panels.iter().enumerate() {
        let mut part = String::new();
        draw_panels(&mut part, std::slice::from_ref(panel))?;
        let start = part
            .find('>')
            .ok_or_else(|| anyhow::anyhow!("invalid SVG header"))?
            + 1;
        let end = part
            .rfind("</svg>")
            .ok_or_else(|| anyhow::anyhow!("invalid SVG end"))?;
        svg.push_str(&format!(
            "<g id=\"panel-{index}\" transform=\"translate({},0)\">",
            index as u32 * ScStyle::PANEL.0
        ));
        svg.push_str(&part[start..end]);
        svg.push_str("</g>\n");
    }
    svg.push_str("</svg>\n");
    Ok(())
}
fn draw_panels(svg: &mut String, panels: &[Panel]) -> Result<()> {
    ensure!(!panels.is_empty(), "empty superconductivity plot");
    let root = SVGBackend::with_string(
        svg,
        (ScStyle::PANEL.0 * panels.len() as u32, ScStyle::PANEL.1),
    )
    .into_drawing_area();
    root.fill(&WHITE)?;
    for (area, panel) in root.split_evenly((1, panels.len())).into_iter().zip(panels) {
        if panel.curves.iter().all(|c| c.points.is_empty()) {
            area.titled(&panel.title, ("sans-serif", 20))?;
            area.draw(&Text::new(
                "Data unavailable",
                (200, 300),
                ("sans-serif", 18)
                    .into_font()
                    .color(&RGBColor(128, 128, 128)),
            ))?;
            continue;
        }
        let xr = range(
            panel.curves.iter().flat_map(|c| {
                let half = if matches!(c.mark, Mark::Bars) {
                    bar_half_width(c)
                } else {
                    0.
                };
                c.points.iter().flat_map(move |p| [p.0 - half, p.0 + half])
            }),
            false,
        );
        let yr = range(
            panel
                .curves
                .iter()
                .flat_map(|c| c.points.iter().map(|p| p.1)),
            true,
        );
        let secondary = range(
            panel
                .secondary
                .iter()
                .flat_map(|c| c.points.iter().map(|p| p.1)),
            true,
        );
        let mut chart = ChartBuilder::on(&area)
            .caption(&panel.title, ("sans-serif", 19))
            .margin(16)
            .x_label_area_size(58)
            .y_label_area_size(65)
            .right_y_label_area_size(if panel.secondary.is_some() { 55 } else { 8 })
            .build_cartesian_2d(xr.clone(), yr)?
            .set_secondary_coord(xr, secondary);
        chart
            .configure_mesh()
            .disable_mesh()
            .x_desc(&panel.x_label)
            .y_desc(&panel.y_label)
            .axis_desc_style(("sans-serif", 20))
            .label_style(("sans-serif", 16))
            .draw()?;
        for curve in &panel.curves {
            let style = curve.color.stroke_width(ScStyle::LINE_WIDTH);
            match curve.mark {
                Mark::Line | Mark::PointsLine => {
                    chart.draw_series(LineSeries::new(curve.points.iter().copied(), style))?;
                }
                Mark::Dashed => {
                    chart.draw_series(DashedLineSeries::new(
                        curve.points.iter().copied(),
                        5,
                        4,
                        style,
                    ))?;
                }
                Mark::Bars => {
                    let half = bar_half_width(curve);
                    chart.draw_series(curve.points.iter().map(|&(x, y)| {
                        Rectangle::new([(x - half, 0.), (x + half, y)], curve.color.filled())
                    }))?;
                }
                Mark::Scatter => {}
            }
            if matches!(curve.mark, Mark::Scatter | Mark::PointsLine | Mark::Dashed) {
                chart.draw_series(
                    curve
                        .points
                        .iter()
                        .map(|p| Circle::new(*p, curve.radius, curve.color.filled())),
                )?;
            }
            if !curve.label.is_empty() {
                let color = curve.color;
                chart
                    .draw_series(std::iter::empty::<PathElement<(f64, f64)>>())?
                    .label(&curve.label)
                    .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 18, y)], color));
            }
        }
        if let Some(c) = &panel.secondary {
            chart
                .configure_secondary_axes()
                .y_desc("λ")
                .label_style(("sans-serif", 16))
                .axis_desc_style(("sans-serif", 20))
                .draw()?;
            chart
                .draw_secondary_series(LineSeries::new(
                    c.points.iter().copied(),
                    c.color.stroke_width(1),
                ))?
                .label(&c.label)
                .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 18, y)], ScColors::LAMBDA));
        }
        if panel.curves.iter().any(|c| !c.label.is_empty()) || panel.secondary.is_some() {
            chart
                .configure_series_labels()
                .position(SeriesLabelPosition::UpperRight)
                .background_style(WHITE.mix(0.85))
                .border_style(BLACK)
                .label_font(("sans-serif", 14))
                .draw()?;
        }
    }
    root.present()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weighted_histogram_and_degenerate_range() {
        let h = histogram(&[0., 0., 1., 1.], &[1., 1., 3., 3.]).unwrap();
        let width = if h.len() > 1 { h[1].0 - h[0].0 } else { 1. };
        assert!((h.iter().map(|p| p.1 * width).sum::<f64>() - 1.).abs() < 1e-10);
        assert_eq!(histogram(&[2.; 5], &[1.; 5]).unwrap().len(), 1);
        assert!(histogram(&[1.], &[0.]).is_err());
        assert!(histogram(&[1.], &[]).is_err());
        assert!(histogram(&[], &[]).unwrap().is_empty());
    }
    #[test]
    fn blank_panels_have_valid_svg() {
        let mut svg = String::new();
        draw(&mut svg, &[Panel::new("Missing", "x", "y")]).unwrap();
        assert!(svg.contains("Data unavailable"));
        assert!(svg.contains("width=\"600\""));
    }
}
