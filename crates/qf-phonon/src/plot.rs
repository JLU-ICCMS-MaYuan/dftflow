//! 声子色散与 DOS：共享已验证路径，联合图共享频率轴。
use crate::{
    archive::{PHBAND_DIR, PHDOS_DIR},
    band::{archived_file, frequency_range, BandData},
};
use anyhow::{ensure, Result};
use plotters::{
    coord::Shift,
    prelude::*,
    style::text_anchor::{HPos, Pos, VPos},
};

const INK: RGBColor = RGBColor(35, 42, 52);
const BAND: RGBColor = RGBColor(31, 119, 180);
const COLORS: [RGBColor; 6] = [
    RGBColor(213, 94, 0),
    RGBColor(0, 158, 115),
    RGBColor(204, 121, 167),
    RGBColor(86, 180, 233),
    RGBColor(230, 159, 0),
    RGBColor(100, 100, 100),
];

fn draw_band<DB: DrawingBackend>(
    area: &DrawingArea<DB, Shift>,
    data: &BandData,
    range: (f64, f64),
) -> Result<()>
where
    DB::ErrorType: 'static,
{
    let xmax = data.points.last().unwrap().distance;
    let mut chart = ChartBuilder::on(area)
        .caption("Phonon dispersion", ("sans-serif", 26))
        .margin_top(24)
        .margin_right(28)
        .margin_bottom(16)
        .margin_left(18)
        .x_label_area_size(64)
        .y_label_area_size(82)
        .build_cartesian_2d(0.0..xmax, range.0..range.1)?;
    chart
        .configure_mesh()
        .disable_x_mesh()
        .x_labels(0)
        .y_labels(6)
        .max_light_lines(0)
        .light_line_style(WHITE)
        .bold_line_style(RGBColor(232, 235, 239))
        .axis_style(INK)
        .label_style(("sans-serif", 20))
        .y_desc("Frequency (cm⁻¹)")
        .axis_desc_style(("sans-serif", 22))
        .draw()?;
    for (distance, _) in &data.ticks {
        if *distance > 0. && *distance < xmax {
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(*distance, range.0), (*distance, range.1)],
                RGBColor(205, 210, 216).stroke_width(1),
            )))?;
        }
    }
    chart.draw_series(std::iter::once(PathElement::new(
        vec![(0., 0.), (xmax, 0.)],
        RGBColor(145, 150, 155).stroke_width(1),
    )))?;
    for mode in 0..data.frequencies[0].len() {
        for segment in &data.segments {
            chart.draw_series(LineSeries::new(
                segment
                    .clone()
                    .map(|i| (data.points[i].distance, data.frequencies[i][mode])),
                BAND.stroke_width(2),
            ))?;
        }
    }
    // 将标签放在横轴下方，位置由真实路径距离确定，禁止数值横轴刻度。
    let (xp, yp) = chart.plotting_area().get_pixel_range();
    let (origin_x, origin_y) = area.get_pixel_range();
    drop(chart);
    let style = TextStyle::from(("sans-serif", 20).into_font())
        .color(&INK)
        .pos(Pos::new(HPos::Center, VPos::Top));
    for (distance, label) in &data.ticks {
        let x = xp.start + ((*distance / xmax) * f64::from(xp.end - xp.start)).round() as i32
            - origin_x.start;
        let y = yp.end - origin_y.start;
        area.draw(&PathElement::new(vec![(x, y), (x, y + 6)], INK))?;
        area.draw(&Text::new(label.clone(), (x, y + 12), style.clone()))?;
    }
    Ok(())
}

/// 以 cm⁻¹ 为单位；0 显式关闭。仅用于绘图，CSV 保持原值。
pub fn dos_plot_sigma(config: &qf_config::Config) -> Result<f64> {
    let sigma = match config.qe_phonon.phdos.get("plot_sigma_cm1") {
        Some(value) => value.as_f64().ok_or_else(|| anyhow::anyhow!("plot_sigma_cm1 必须是非负数"))?,
        None => 10.0,
    };
    ensure!(sigma.is_finite() && sigma >= 0.0, "plot_sigma_cm1 必须是有限非负数");
    Ok(sigma)
}

/// 同一线性卷积核作用于全部 DOS 列。每个源点的核按输出梯形权重归一，
/// 保留积分及投影可加性，包括有限频率区间的端点。
fn gaussian_dos(rows: Vec<Vec<f64>>, sigma: f64) -> Result<Vec<Vec<f64>>> {
    ensure!(sigma.is_finite() && sigma >= 0.0, "高斯展宽必须是有限非负数");
    if sigma == 0.0 { return Ok(rows); }
    ensure!(rows.len() >= 2 && rows.windows(2).all(|w| w[1][0] > w[0][0]), "高斯平滑要求至少两个严格递增的频率点");
    let lo = rows[0][0];
    let hi = rows.last().unwrap()[0];
    let n = (((hi-lo)/(sigma/4.0)).ceil() as usize).max(rows.len()).min(20000).max(2);
    let dx = (hi-lo)/(n-1) as f64;
    let mut result: Vec<Vec<f64>> = (0..n).map(|i| { let mut r=vec![0.0; rows[0].len()]; r[0]=lo+i as f64*dx; r }).collect();
    for (i, source) in rows.iter().enumerate() {
        let source_weight = if i == 0 { (rows[1][0]-lo)/2.0 } else if i+1 == rows.len() { (hi-rows[i-1][0])/2.0 } else { (rows[i+1][0]-rows[i-1][0])/2.0 };
        // 减去最近点指数，避免极窄核在所有输出点下溢。
        let nearest = ((source[0]-lo)/dx).round().clamp(0.0,(n-1) as f64) as usize;
        let minimum = ((result[nearest][0]-source[0])/sigma).powi(2);
        let kernel: Vec<f64> = result.iter().map(|r| (-0.5*(((r[0]-source[0])/sigma).powi(2)-minimum)).exp()).collect();
        let norm: f64 = kernel.iter().enumerate().map(|(j,k)| k*dx*if j==0 || j+1==n {0.5} else {1.0}).sum();
        ensure!(norm.is_finite() && norm > 0.0, "高斯展宽过小，请增大 plot_sigma_cm1");
        for (r,k) in result.iter_mut().zip(kernel) {
            for col in 1..source.len() { r[col] += source[col]*source_weight*k/norm; }
        }
    }
    Ok(result)
}

struct DosData {
    rows: Vec<Vec<f64>>,
    names: Vec<String>,
}
fn load_dos(
    workspace: &str,
    prefix: &str,
    elements: &[String],
    counts: &[usize],
) -> Result<DosData> {
    let text = std::fs::read_to_string(archived_file(
        workspace,
        PHDOS_DIR,
        &qf_input::filenames::derive_filenames(prefix).fldos,
    ))?;
    ensure!(
        !regex::Regex::new(r"(?i)\b(?:nan|inf|infinity)\b")?.is_match(&text),
        "声子 DOS 包含非有限数值"
    );
    let rows = qf_io::read_numeric_rows(&text);
    ensure!(
        !rows.is_empty() && rows[0].len() >= 2,
        "声子 DOS 至少需要频率与 TDOS 两列"
    );
    let width = rows[0].len();
    ensure!(
        rows.iter()
            .all(|r| r.len() == width && r.iter().all(|v| v.is_finite())),
        "声子 DOS 列数不一致或包含非有限数值"
    );
    // 与表格层一致：投影列是逐原子值，按结构的原子顺序聚合为元素。
    if width > 2 {
        let (_, table, names) =
            crate::tables::build_phonon_dos_tables(workspace, prefix, elements, counts)?;
        let values = table
            .iter()
            .skip(1)
            .map(|r| {
                r.iter()
                    .map(|v| v.parse::<f64>())
                    .collect::<std::result::Result<Vec<_>, _>>()
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        return Ok(DosData {
            rows: values,
            names: std::iter::once("TDOS".to_owned()).chain(names).collect(),
        });
    }
    let _ = elements;
    Ok(DosData {
        rows,
        names: vec!["TDOS".into()],
    })
}
fn draw_dos<DB: DrawingBackend>(
    area: &DrawingArea<DB, Shift>,
    data: &DosData,
    range: (f64, f64),
    show_y: bool,
) -> Result<()>
where
    DB::ErrorType: 'static,
{
    let xmax = data
        .rows
        .iter()
        .flat_map(|r| r.iter().skip(1))
        .copied()
        .fold(0., f64::max)
        .max(1e-12)
        * 1.08;
    let mut chart = ChartBuilder::on(area)
        .caption("Phonon DOS", ("sans-serif", 26))
        .margin_top(24)
        .margin_right(28)
        .margin_bottom(16)
        .margin_left(if show_y { 18 } else { 0 })
        .x_label_area_size(64)
        .y_label_area_size(if show_y { 82 } else { 12 })
        .build_cartesian_2d(0.0..xmax, range.0..range.1)?;
    chart
        .configure_mesh()
        .disable_x_mesh()
        .max_light_lines(0)
        .x_labels(4)
        .y_labels(if show_y { 6 } else { 0 })
        .bold_line_style(RGBColor(232, 235, 239))
        .axis_style(INK)
        .label_style(("sans-serif", 18))
        .x_desc("DOS")
        .y_desc(if show_y { "Frequency (cm⁻¹)" } else { "" })
        .axis_desc_style(("sans-serif", 22))
        .draw()?;
    chart.draw_series(std::iter::once(PathElement::new(
        vec![(0., 0.), (xmax, 0.)],
        RGBColor(145, 150, 155),
    )))?;
    for (i, name) in data.names.iter().enumerate() {
        let color = if i == 0 {
            INK
        } else {
            COLORS[(i - 1) % COLORS.len()]
        };
        chart
            .draw_series(LineSeries::new(
                data.rows.iter().map(|r| (r[i + 1], r[0])),
                color.stroke_width(2),
            ))?
            .label(name)
            .legend(move |(x, y)| {
                PathElement::new(vec![(x, y), (x + 22, y)], color.stroke_width(2))
            });
    }
    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::UpperRight)
        .background_style(WHITE.mix(0.9))
        .label_font(("sans-serif", 17))
        .border_style(WHITE)
        .draw()?;
    Ok(())
}

pub fn plot_phonon_band(workspace: &str, prefix: &str) -> Result<String> {
    let data = BandData::load(workspace, prefix)?;
    let range = frequency_range(data.frequencies.iter().flatten().copied())?;
    std::fs::create_dir_all(format!("{workspace}/{PHBAND_DIR}"))?;
    let out = format!("{workspace}/{PHBAND_DIR}/phonon_band.png");
    {
        let area = BitMapBackend::new(&out, (1200, 760)).into_drawing_area();
        area.fill(&WHITE)?;
        draw_band(&area, &data, range)?;
        area.present()?;
    }
    Ok(out)
}
pub fn plot_phonon_dos(
    workspace: &str,
    prefix: &str,
    elements: &[String],
    counts: &[usize],
    sigma: f64,
) -> Result<String> {
    let mut data = load_dos(workspace, prefix, elements, counts)?;
    data.rows = gaussian_dos(data.rows, sigma)?;
    let range = frequency_range(data.rows.iter().map(|r| r[0]))?;
    std::fs::create_dir_all(format!("{workspace}/{PHDOS_DIR}"))?;
    let out = format!("{workspace}/{PHDOS_DIR}/phonon_dos.png");
    {
        let area = BitMapBackend::new(&out, (720, 760)).into_drawing_area();
        area.fill(&WHITE)?;
        draw_dos(&area, &data, range, true)?;
        area.present()?;
    }
    Ok(out)
}
pub fn plot_phonon_combined(
    workspace: &str,
    prefix: &str,
    elements: &[String],
    counts: &[usize],
    sigma: f64,
) -> Result<String> {
    let band = BandData::load(workspace, prefix)?;
    let mut dos = load_dos(workspace, prefix, elements, counts)?;
    dos.rows = gaussian_dos(dos.rows, sigma)?;
    let range = frequency_range(
        band.frequencies
            .iter()
            .flatten()
            .copied()
            .chain(dos.rows.iter().map(|r| r[0])),
    )?;
    let out = format!("{workspace}/phonon_combined.jpg");
    {
        let area = BitMapBackend::new(&out, (1600, 760)).into_drawing_area();
        area.fill(&WHITE)?;
        let (left, right) = area.split_horizontally(1200);
        draw_band(&left, &band, range)?;
        draw_dos(&right, &dos, range, false)?;
        area.present()?;
    }
    Ok(out)
}

#[cfg(test)]
mod smoothing_tests {
    use super::*;
    fn integral(rows: &[Vec<f64>], col: usize) -> f64 {
        rows.windows(2).map(|w| (w[1][0]-w[0][0])*(w[1][col]+w[0][col])/2.0).sum()
    }
    #[test]
    fn gaussian_preserves_weight_and_projection_sum_and_reduces_peak() {
        let rows: Vec<Vec<f64>> = (0..101).map(|i| {
            let a = if i == 50 {1.0} else {0.0};
            let b = if i == 30 {0.5} else {0.0};
            vec![i as f64, a+b, a, b]
        }).collect();
        assert_eq!(gaussian_dos(rows.clone(),0.0).unwrap(), rows);
        let narrow=gaussian_dos(rows.clone(),5.0).unwrap();
        let broad=gaussian_dos(rows.clone(),10.0).unwrap();
        for col in 1..4 { assert!((integral(&rows,col)-integral(&broad,col)).abs()<1e-10); }
        assert!(broad.iter().all(|r| (r[1]-r[2]-r[3]).abs()<1e-12));
        assert!(broad.iter().map(|r|r[2]).fold(0.0,f64::max) < narrow.iter().map(|r|r[2]).fold(0.0,f64::max));
        assert!(gaussian_dos(rows.clone(),-1.0).is_err());
        assert!(gaussian_dos(rows,f64::NAN).is_err());
        assert!(gaussian_dos(vec![vec![0.0,1.0],vec![0.0,2.0]],10.0).is_err());
    }
}
