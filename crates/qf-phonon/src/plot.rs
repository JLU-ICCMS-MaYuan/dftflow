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
) -> Result<String> {
    let data = load_dos(workspace, prefix, elements, counts)?;
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
) -> Result<String> {
    let band = BandData::load(workspace, prefix)?;
    let dos = load_dos(workspace, prefix, elements, counts)?;
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
