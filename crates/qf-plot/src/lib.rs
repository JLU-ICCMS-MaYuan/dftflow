//! 绘图封装（对齐 Python `plotters` 等价重绘，内容/布局等价，非像素级）。
//!
//! 当前提供折线图与能带/声子谱图的基础封装；后续 P2/P3 各 flow 的带结构、DOS、联合图、
//! fatband、COHP、LOCPOT、周期表、Wannier 能带均复用此 crate。

use anyhow::Result;
use plotters::prelude::*;
use std::path::Path;

/// 一条折线序列。
#[derive(Debug, Clone)]
pub struct Series {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub label: String,
}

impl Series {
    pub fn new(x: Vec<f64>, y: Vec<f64>, label: impl Into<String>) -> Self {
        Self {
            x,
            y,
            label: label.into(),
        }
    }
}

const PALETTE: [RGBColor; 6] = [RED, BLUE, GREEN, MAGENTA, CYAN, BLACK];

fn bounds(series: &[Series]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for s in series {
        for (x, y) in s.x.iter().zip(s.y.iter()) {
            min_x = min_x.min(*x);
            max_x = max_x.max(*x);
            min_y = min_y.min(*y);
            max_y = max_y.max(*y);
        }
    }
    if !min_x.is_finite() {
        (0.0, 1.0, 0.0, 1.0)
    } else {
        // 给纵轴留 5% 余量，避免贴边。
        let pad = (max_y - min_y).abs() * 0.05 + 1e-9;
        (min_x, max_x, min_y - pad, max_y + pad)
    }
}

/// 将多条折线写入 PNG（自动缩放坐标轴、带图例）。
pub fn line_plot_png<P: AsRef<Path>>(
    path: P,
    series: &[Series],
    x_label: &str,
    y_label: &str,
    title: &str,
) -> Result<()> {
    let root = BitMapBackend::new(path.as_ref(), (900, 600)).into_drawing_area();
    root.fill(&WHITE)?;
    let (min_x, max_x, min_y, max_y) = bounds(series);
    let mut chart = ChartBuilder::on(&root)
        .caption(title, ("sans-serif", 24))
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(min_x..max_x, min_y..max_y)?;
    chart
        .configure_mesh()
        .x_desc(x_label)
        .y_desc(y_label)
        .draw()?;
    for (i, s) in series.iter().enumerate() {
        let color = PALETTE[i % PALETTE.len()];
        chart
            .draw_series(LineSeries::new(
                s.x.iter().zip(s.y.iter()).map(|(x, y)| (*x, *y)),
                color,
            ))?
            .label(&s.label)
            .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], color));
    }
    chart
        .configure_series_labels()
        .background_style(WHITE.mix(0.8))
        .border_style(BLACK)
        .draw()?;
    root.present()?;
    Ok(())
}

/// 能带/声子谱图：给定每个 q 点的路径坐标 `distances` 与频率矩阵 `frequencies`（nq × nmodes）。
pub fn band_plot_png<P: AsRef<Path>>(
    path: P,
    distances: &[f64],
    frequencies: &[Vec<f64>],
    y_label: &str,
    title: &str,
) -> Result<()> {
    let nmodes = frequencies.first().map(|f| f.len()).unwrap_or(0);
    let mut series = Vec::with_capacity(nmodes);
    for m in 0..nmodes {
        let y: Vec<f64> = frequencies
            .iter()
            .filter_map(|row| row.get(m).copied())
            .collect();
        series.push(Series::new(distances.to_vec(), y, format!("mode {m}")));
    }
    line_plot_png(path, &series, "distance", y_label, title)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_line_plot_tmp() {
        let dir = std::env::temp_dir();
        let path = dir.join("qf_plot_test_line.png");
        let series = vec![Series::new(
            vec![0.0, 1.0, 2.0],
            vec![0.0, 1.0, 4.0],
            "demo",
        )];
        line_plot_png(&path, &series, "x", "y", "test").unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn writes_band_plot_tmp() {
        let dir = std::env::temp_dir();
        let path = dir.join("qf_plot_test_band.png");
        let dist = vec![0.0, 1.0, 2.0];
        let freqs = vec![
            vec![0.0, 100.0, 200.0],
            vec![10.0, 110.0, 210.0],
            vec![20.0, 120.0, 220.0],
        ];
        band_plot_png(&path, &dist, &freqs, "cm-1", "phonon").unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_file(&path);
    }
}
