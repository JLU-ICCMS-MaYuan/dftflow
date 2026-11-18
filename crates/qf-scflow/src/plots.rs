//! scflow 绘图（复刻 `scflow/plotters.py` 的三类 PDF）：
//! `plot_system`（3×2 面板）、`plot_article_view`（α²F/λ/wlog/Tc_AD）、`plot_summary`（Tc 收敛）。
//! plotters 绘制 SVG，`svg2pdf` 纯 Rust 转 PDF；内容/布局等价，非像素级。

use anyhow::Result;
use qf_config::PlotFormat;
use plotters::prelude::*;
use std::path::Path;

use crate::outputs;
use crate::system::OrderedDict;

fn get<'a>(dict: &'a OrderedDict, key: &str) -> &'a [f64] {
    dict.iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_slice())
        .unwrap_or(&[])
}

/// SVG 文本 → PDF 文件（svg2pdf，纯 Rust）。
pub fn svg_to_pdf(svg_text: &str, pdf_path: &Path) -> Result<()> {
    qf_plot::svg_to_pdf(svg_text, pdf_path)
}

fn plot_xy(
    area: &DrawingArea<SVGBackend<'_>, plotters::coord::Shift>,
    x: &[f64],
    series: &[(Vec<f64>, RGBColor, bool)],
    title: &str,
    y_label: &str,
) -> Result<()> {
    let xmax = x.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let ymax = series
        .iter()
        .flat_map(|(y, _, _)| y.iter().cloned())
        .fold(1e-12_f64, f64::max)
        * 1.05;
    let mut chart = ChartBuilder::on(area)
        .caption(title, ("sans-serif", 16).into_font())
        .margin(8)
        .x_label_area_size(28)
        .y_label_area_size(44)
        .build_cartesian_2d(0.0f64..xmax, 0.0f64..ymax)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    chart
        .configure_mesh()
        .x_desc("ω, THz")
        .y_desc(y_label)
        .disable_mesh()
        .draw()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    for (y, color, filled) in series {
        if *filled {
            let base = vec![0.0; y.len()];
            let mut polygon: Vec<(f64, f64)> =
                x.iter().zip(y.iter()).map(|(a, b)| (*a, *b)).collect();
            polygon
                .extend(x.iter().rev().zip(base.iter().rev()).map(|(a, b)| (*a, *b)));
            chart
                .draw_series(std::iter::once(Polygon::new(polygon, color.mix(0.35).filled())))
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        chart
            .draw_series(
                x.iter()
                    .zip(y.iter())
                    .filter(|(_, b)| **b > 0.0)
                    .map(|(a, b)| Circle::new((*a, *b), 2, color.filled())),
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    Ok(())
}

fn plot_line(
    area: &DrawingArea<SVGBackend<'_>, plotters::coord::Shift>,
    x: &[f64],
    y: &[f64],
    color: RGBColor,
    title: &str,
    y_label: &str,
) -> Result<()> {
    let xmax = x.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let ymax = y.iter().cloned().fold(f64::NEG_INFINITY, f64::max) * 1.05;
    let mut chart = ChartBuilder::on(area)
        .caption(title, ("sans-serif", 16).into_font())
        .margin(8)
        .x_label_area_size(28)
        .y_label_area_size(44)
        .build_cartesian_2d(0.0f64..xmax, 0.0f64..ymax.max(1e-12))
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    chart
        .configure_mesh()
        .x_desc("ω, THz")
        .y_desc(y_label)
        .disable_mesh()
        .draw()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    chart
        .draw_series(
            x.iter()
                .zip(y.iter())
                .filter(|(_, b)| **b > 0.0)
                .map(|(a, b)| PathElement::new(vec![(*a, *b)], color.stroke_width(2))),
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

/// `plot_system`：3×2 面板 → `plot_s{s}_r{r}_g{g}[_b{b}].pdf`。
#[allow(clippy::too_many_arguments)]
pub fn plot_system_with_formats(
    direct: &OrderedDict,
    a2f: &OrderedDict,
    _formula: &str,
    smoothing: f64,
    resolution: usize,
    sigma: f64,
    _mu: f64,
    path: &str,
    b: f64,
    formats: &[PlotFormat],
) -> Result<Vec<String>> {
    let freqs_a2f = get(a2f, "freqs, THz");
    let freqs_direct = get(direct, "freqs, THz");
    let file_name = if b > 0.0 {
        format!(
            "plot_s{}_r{}_g{}_b{}.pdf",
            outputs::format_smoothing_raw(smoothing),
            resolution,
            outputs::format_smoothing_raw(sigma),
            outputs::format_smoothing_raw(b)
        )
    } else {
        format!(
            "plot_s{}_r{}_g{}.pdf",
            outputs::format_smoothing_raw(smoothing),
            resolution,
            outputs::format_smoothing_raw(sigma)
        )
    };
    let out_path = Path::new(path).join(&file_name);
    let (width, height) = (1200u32, 1600u32);
    let base = out_path.with_extension("");
    let mut all_formats = vec![PlotFormat::Pdf];
    all_formats.extend_from_slice(formats);
    let saved = qf_plot::save_plot(&base.to_string_lossy(), &all_formats, |svg| {
        let root = SVGBackend::with_string(svg, (width, height)).into_drawing_area();
        root.fill(&WHITE).map_err(|e| anyhow::anyhow!("{e}"))?;
        let cells = root.split_evenly((3, 2));
        // (0,0) a2F：interp 填充 + direct 散点
        plot_xy(
            &cells[0],
            freqs_a2f,
            &[
                (get(a2f, "a2f (lambda), THz").to_vec(), RGBColor(0x11, 0x99, 0xDD), true),
                (get(a2f, "a2f (gamma), THz").to_vec(), RGBColor(0xFF, 0xA5, 0x00), true),
            ],
            "Eliashberg function α²f(ω)",
            "α²f",
        )?;
        plot_xy(
            &cells[0],
            freqs_direct,
            &[
                (get(direct, "a2f (lambda), THz").to_vec(), RGBColor(0x44, 0x44, 0x44), false),
                (get(direct, "a2f (gamma), THz").to_vec(), RGBColor(0x88, 0x88, 0x88), false),
            ],
            "Eliashberg function α²f(ω)",
            "α²f",
        )?;
        let panels = [
            (1usize, "EPC coefficient λ", "λ", "lambda (lambda)", "lambda (gamma)"),
            (2, "Logarithmic average frequency ω_log", "ω_log, K", "wlog (lambda), K", "wlog (gamma), K"),
            (3, "Mean square frequency ω²", "ω², K", "w2 (lambda), K", "w2 (gamma), K"),
            (4, "McMillan T_C", "T_C, K", "Tc_McM (lambda), K", "Tc_McM (gamma), K"),
            (5, "Allen-Dynes T_C", "T_C, K", "Tc_AD (lambda), K", "Tc_AD (gamma), K"),
        ];
        for (idx, title, y_label, key_l, key_g) in panels {
            plot_line(&cells[idx], freqs_a2f, get(a2f, key_l), RGBColor(0x22, 0x66, 0xAA), title, y_label)?;
            plot_line(&cells[idx], freqs_a2f, get(a2f, key_g), RGBColor(0xAA, 0x44, 0x22), title, y_label)?;
            plot_line(&cells[idx], freqs_direct, get(direct, key_l), RGBColor(0x00, 0x00, 0x00), title, y_label)?;
            plot_line(&cells[idx], freqs_direct, get(direct, key_g), RGBColor(0x66, 0x66, 0x66), title, y_label)?;
        }
        root.present().map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(())
    })?;
    anyhow::ensure!(saved.iter().any(|p| Path::new(p) == out_path), "Required scflow PDF was not saved: {}", out_path.display());
    Ok(saved)
}

/// `plot_article_view`：α²F 填充 + λ/wlog/Tc_AD 曲线 → `plot_article_{formula}.pdf`。
pub fn plot_article_view_with_formats(
    _direct: &OrderedDict,
    a2f: &OrderedDict,
    formula: &str,
    path: &str,
    formats: &[PlotFormat],
) -> Result<Vec<String>> {
    let freqs_a2f = get(a2f, "freqs, THz");
    let out_path = Path::new(path).join(format!("plot_article_{formula}.pdf"));
    let (width, height) = (1200u32, 600u32);
    let base = out_path.with_extension("");
    let mut all_formats = vec![PlotFormat::Pdf];
    all_formats.extend_from_slice(formats);
    let saved = qf_plot::save_plot(&base.to_string_lossy(), &all_formats, |svg| {
        let root = SVGBackend::with_string(svg, (width, height)).into_drawing_area();
        root.fill(&WHITE).map_err(|e| anyhow::anyhow!("{e}"))?;
        plot_xy(
            &root,
            freqs_a2f,
            &[(
                get(a2f, "a2f (gamma), THz").to_vec(),
                RGBColor(0x80, 0x80, 0x80),
                true,
            )],
            format!("{formula}: α²f, λ, ω_log, Allen-Dynes T_C").as_str(),
            "α²f",
        )?;
        // λ / wlog / Tc_AD 以各自尺度叠加（等价信息，非像素级）
        plot_line(&root, freqs_a2f, get(a2f, "lambda (gamma)"), RGBColor(0x46, 0x82, 0xB4), "λ", "λ")?;
        plot_line(&root, freqs_a2f, get(a2f, "wlog (gamma), K"), RGBColor(0xFF, 0x8C, 0x00), "ω_log", "ω_log, K")?;
        plot_line(&root, freqs_a2f, get(a2f, "Tc_AD (gamma), K"), RGBColor(0x22, 0x88, 0x22), "Allen-Dynes T_C", "T_C, K")?;
        root.present().map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(())
    })?;
    anyhow::ensure!(saved.iter().any(|p| Path::new(p) == out_path), "Required scflow PDF was not saved: {}", out_path.display());
    Ok(saved)
}

/// `plot_summary`：Tc–展宽收敛 → `summary_{formula}_s{s}_mu{mu}.pdf`。
pub fn plot_summary_with_formats(
    broadening: &[f64],
    summary: &std::collections::BTreeMap<String, Vec<f64>>,
    formula: &str,
    smoothing: f64,
    sigma: f64,
    resolution: usize,
    mu: f64,
    path: &str,
    formats: &[PlotFormat],
) -> Result<Vec<String>> {
    let out_path = Path::new(path).join(format!(
        "summary_{formula}_s{}_mu{}.pdf",
        outputs::format_smoothing_raw(smoothing),
        outputs::format_smoothing_raw(mu)
    ));
    let (width, height) = (1200u32, 600u32);
    let base = out_path.with_extension("");
    let mut all_formats = vec![PlotFormat::Pdf];
    all_formats.extend_from_slice(formats);
    let saved = qf_plot::save_plot(&base.to_string_lossy(), &all_formats, |svg| {
        let root = SVGBackend::with_string(svg, (width, height)).into_drawing_area();
        root.fill(&WHITE).map_err(|e| anyhow::anyhow!("{e}"))?;
        let mut keys: Vec<&String> = summary.keys().filter(|k| k.starts_with("Tc")).collect();
        keys.sort();
        let ymax = keys
            .iter()
            .flat_map(|k| summary[*k].iter().cloned())
            .fold(1e-12_f64, f64::max)
            * 1.1;
        let xmax = broadening.iter().cloned().fold(1e-9_f64, f64::max);
        let mut chart = ChartBuilder::on(&root)
            .caption(
                format!(
                    "Convergence of {formula}: Smoothing {smoothing} THz, resolution {resolution}, σ {sigma}, μ {mu}"
                ),
                ("sans-serif", 16).into_font(),
            )
            .margin(10)
            .x_label_area_size(34)
            .y_label_area_size(50)
            .build_cartesian_2d(0.0f64..xmax, 0.0f64..ymax)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        chart
            .configure_mesh()
            .x_desc("Broadening, mRy")
            .y_desc("T_C, K")
            .disable_mesh()
            .draw()
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let palette = [
            RGBColor(0x22, 0x66, 0xAA),
            RGBColor(0xAA, 0x44, 0x22),
            RGBColor(0x22, 0x88, 0x22),
        ];
        for (i, key) in keys.iter().enumerate() {
            let color = palette[i % 3];
            let y = &summary[*key];
            chart
                .draw_series(
                    broadening
                        .iter()
                        .zip(y.iter())
                        .map(|(a, b)| Circle::new((*a, *b), 3, color.filled())),
                )
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            chart
                .draw_series(broadening.iter().zip(y.iter()).map(|(a, b)| {
                    PathElement::new(vec![(*a, *b)], color.stroke_width(2))
                }))
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        root.present().map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(())
    })?;
    anyhow::ensure!(saved.iter().any(|p| Path::new(p) == out_path), "Required scflow PDF was not saved: {}", out_path.display());
    Ok(saved)
}

/// 保留仅返回主 PDF 的库接口；CLI 使用带格式参数的版本。
#[allow(clippy::too_many_arguments)]
pub fn plot_system(direct: &OrderedDict, a2f: &OrderedDict, formula: &str,
    smoothing: f64, resolution: usize, sigma: f64, mu: f64, path: &str, b: f64) -> Result<String> {
    Ok(plot_system_with_formats(direct, a2f, formula, smoothing, resolution, sigma, mu, path, b, &[])?[0].clone())
}

pub fn plot_article_view(direct: &OrderedDict, a2f: &OrderedDict, formula: &str, path: &str) -> Result<String> {
    Ok(plot_article_view_with_formats(direct, a2f, formula, path, &[])?[0].clone())
}

#[allow(clippy::too_many_arguments)]
pub fn plot_summary(broadening: &[f64], summary: &std::collections::BTreeMap<String, Vec<f64>>,
    formula: &str, smoothing: f64, sigma: f64, resolution: usize, mu: f64, path: &str) -> Result<String> {
    Ok(plot_summary_with_formats(broadening, summary, formula, smoothing, sigma, resolution, mu, path, &[])?[0].clone())
}

#[cfg(test)]
mod format_tests {
    use super::*;

    #[test]
    fn extra_format_failure_preserves_required_pdf_and_legacy_return() {
        let dir = std::env::temp_dir().join(format!("scflow-plot-extra-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("plot_article_MgB2.jpg")).unwrap();
        let data = vec![("freqs, THz".into(), vec![0.0, 1.0, 2.0])];
        let saved = plot_article_view_with_formats(&data, &data, "MgB2", dir.to_str().unwrap(), &[PlotFormat::Jpg, PlotFormat::Svg]).unwrap();
        assert_eq!(saved.len(), 2);
        assert!(saved[0].ends_with(".pdf") && saved[1].ends_with(".svg"));
        let pdf = plot_article_view(&data, &data, "MgB2", dir.to_str().unwrap()).unwrap();
        assert_eq!(pdf, saved[0]);
        assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF-"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn required_pdf_failure_is_reported_even_if_svg_succeeds() {
        let dir = std::env::temp_dir().join(format!("scflow-plot-required-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("plot_article_MgB2.pdf")).unwrap();
        let data = vec![("freqs, THz".into(), vec![0.0, 1.0, 2.0])];
        let error = plot_article_view_with_formats(&data, &data, "MgB2", dir.to_str().unwrap(), &[PlotFormat::Svg]).unwrap_err();
        assert!(error.to_string().contains("Required scflow PDF was not saved"));
        assert!(dir.join("plot_article_MgB2.svg").is_file());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
