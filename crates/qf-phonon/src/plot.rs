//! 声子谱 / 态密度绘图（对齐 `qeflow/phonon/phonon_plot.py` 与 Issue 0003）。
//! 使用 `plotters` 重绘：左声子谱、右 DOS/元素分波，共享频率轴；产物等价（非像素级）。

use std::path::Path;

use anyhow::Result;
use plotters::prelude::*;

use qf_io::{read_band_frequencies, read_labelinfo, read_numeric_rows};

use crate::archive::{PHBAND_DIR, PHDOS_DIR};

const BG: RGBColor = WHITE;

/// 绘制声子谱 PNG：`phband/phonon_band.png`。
pub fn plot_phonon_band(workspace: &str, prefix: &str) -> Result<String> {
    let names = qf_input::filenames::derive_filenames(prefix);
    let label_path = format!("{workspace}/{PHBAND_DIR}/{prefix}_band.labelinfo.dat");
    let label_path = if Path::new(&label_path).exists() {
        label_path
    } else {
        format!("{workspace}/{prefix}_band.labelinfo.dat")
    };
    let freq_path = format!("{workspace}/{PHBAND_DIR}/{}", names.flfrq);
    let freq_path = if Path::new(&freq_path).exists() {
        freq_path
    } else {
        format!("{workspace}/{}", names.flfrq)
    };
    let labels = read_labelinfo(&std::fs::read_to_string(&label_path)?);
    let freq_text = std::fs::read_to_string(&freq_path)?;
    let modes = crate::tables::detect_mode_count(&freq_text);
    let frequencies = read_band_frequencies(&freq_text, modes)?;

    let out = format!("{workspace}/{PHBAND_DIR}/phonon_band.png");
    let root = BitMapBackend::new(&out, (900, 600)).into_drawing_area();
    root.fill(&BG)?;

    let x_max = labels.last().map(|l| l.distance).unwrap_or(1.0).max(1e-6);
    let mut y_max = 0.0f64;
    for row in &frequencies {
        for v in row {
            y_max = y_max.max(*v);
        }
    }
    y_max = y_max.max(1.0) * 1.05;

    let mut chart = ChartBuilder::on(&root)
        .caption("Phonon Band Structure", ("sans-serif", 24))
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(0.0..x_max, 0.0..y_max)?;
    chart
        .configure_mesh()
        .x_desc("Wave vector")
        .y_desc("Frequency (cm-1)")
        .x_labels(10)
        .draw()?;

    // 高对称点的竖直分隔线
    for l in &labels {
        chart
            .draw_series(std::iter::once(PathElement::new(
                vec![(l.distance, 0.0), (l.distance, y_max)],
                &BLACK.mix(0.4),
            )))?;
        chart.draw_series(std::iter::once(Text::new(
            l.label.clone(),
            (l.distance, 0.0),
            ("sans-serif", 16),
        )))?;
    }

    for m in 0..modes {
        let points: Vec<(f64, f64)> = frequencies
            .iter()
            .enumerate()
            .map(|(qi, row)| {
                let x = labels.get(qi).map(|l| l.distance).unwrap_or(0.0);
                (x, row[m])
            })
            .collect();
        chart.draw_series(LineSeries::new(points, &BLUE))?;
    }
    root.present()?;
    Ok(out.clone())
}

/// 绘制声子 DOS PNG：`phdos/phonon_dos.png`（TDOS + 元素分波）。
pub fn plot_phonon_dos(workspace: &str, prefix: &str, elements: &[String]) -> Result<String> {
    let names = qf_input::filenames::derive_filenames(prefix);
    let dos_path = format!("{workspace}/{PHDOS_DIR}/{}", names.fldos);
    let dos_path = if Path::new(&dos_path).exists() {
        dos_path
    } else {
        format!("{workspace}/{}", names.fldos)
    };
    let rows = read_numeric_rows(&std::fs::read_to_string(&dos_path)?);
    if rows.is_empty() || rows[0].len() < 2 {
        anyhow::bail!("声子 DOS 文件至少需要 frequency 与 TDOS 两列。");
    }
    let x_max = rows.last().map(|r| r[0]).unwrap_or(1.0);
    let mut y_max = 0.0f64;
    for r in &rows {
        y_max = y_max.max(r[1]);
    }
    let n_extra = rows[0].len() - 2;
    let element_colors = [RED, GREEN, MAGENTA, CYAN, YELLOW, BLACK];

    let out = format!("{workspace}/{PHDOS_DIR}/phonon_dos.png");
    let root = BitMapBackend::new(&out, (900, 600)).into_drawing_area();
    root.fill(&BG)?;

    let mut chart = ChartBuilder::on(&root)
        .caption("Phonon DOS", ("sans-serif", 24))
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(0.0..x_max, 0.0..y_max * 1.05)?;
    chart
        .configure_mesh()
        .x_desc("Frequency (cm-1)")
        .y_desc("DOS")
        .draw()?;

    chart.draw_series(LineSeries::new(
        rows.iter().map(|r| (r[0], r[1])),
        &BLACK,
    ))?;
    for e in 0..n_extra {
        let color = element_colors[e % element_colors.len()];
        chart.draw_series(LineSeries::new(
            rows.iter().map(|r| (r[0], r[2 + e])),
            &color,
        ))?;
    }
    let _ = elements;
    root.present()?;
    Ok(out.clone())
}

/// 绘制联合 JPG：`phonon_combined.jpg`（左声子谱、右 DOS，共享频率轴），对齐 Issue 0003。
pub fn plot_phonon_combined(workspace: &str, prefix: &str, elements: &[String]) -> Result<String> {
    // 复用单图数据解析
    let names = qf_input::filenames::derive_filenames(prefix);
    let label_path = format!("{workspace}/{PHBAND_DIR}/{prefix}_band.labelinfo.dat");
    let label_path = if Path::new(&label_path).exists() {
        label_path
    } else {
        format!("{workspace}/{prefix}_band.labelinfo.dat")
    };
    let freq_path = format!("{workspace}/{PHBAND_DIR}/{}", names.flfrq);
    let freq_path = if Path::new(&freq_path).exists() {
        freq_path
    } else {
        format!("{workspace}/{}", names.flfrq)
    };
    let labels = read_labelinfo(&std::fs::read_to_string(&label_path)?);
    let freq_text = std::fs::read_to_string(&freq_path)?;
    let modes = crate::tables::detect_mode_count(&freq_text);
    let frequencies = read_band_frequencies(&freq_text, modes)?;

    let dos_path = format!("{workspace}/{PHDOS_DIR}/{}", names.fldos);
    let dos_path = if Path::new(&dos_path).exists() {
        dos_path
    } else {
        format!("{workspace}/{}", names.fldos)
    };
    let dos_rows = read_numeric_rows(&std::fs::read_to_string(&dos_path)?);

    let out = format!("{workspace}/phonon_combined.jpg");
    let root = BitMapBackend::new(&out, (1600, 600)).into_drawing_area();
    root.fill(&BG)?;
    let (left, right) = root.split_horizontally(800);

    let x_max = labels.last().map(|l| l.distance).unwrap_or(1.0).max(1e-6);
    let mut y_max = 0.0f64;
    for row in &frequencies {
        for v in row {
            y_max = y_max.max(*v);
        }
    }
    if let Some(last) = dos_rows.last() {
        y_max = y_max.max(last[0].abs());
    }
    y_max = y_max.max(1.0) * 1.05;

    // 左：声子谱
    {
        let mut chart = ChartBuilder::on(&left)
            .caption("Phonon Band", ("sans-serif", 20))
            .margin(15)
            .x_label_area_size(35)
            .y_label_area_size(55)
            .build_cartesian_2d(0.0..x_max, 0.0..y_max)?;
        chart
            .configure_mesh()
            .x_desc("Wave vector")
            .y_desc("Frequency (cm-1)")
            .draw()?;
        for l in &labels {
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(l.distance, 0.0), (l.distance, y_max)],
                &BLACK.mix(0.4),
            )))?;
        }
        for m in 0..modes {
            let points: Vec<(f64, f64)> = frequencies
                .iter()
                .enumerate()
                .map(|(qi, row)| {
                    let x = labels.get(qi).map(|l| l.distance).unwrap_or(0.0);
                    (x, row[m])
                })
                .collect();
            chart.draw_series(LineSeries::new(points, &BLUE))?;
        }
    }

    // 右：DOS（共享频率轴）
    if !dos_rows.is_empty() && dos_rows[0].len() >= 2 {
        let dos_x_max = dos_rows.last().map(|r| r[0]).unwrap_or(1.0);
        let mut dos_y_max = 0.0f64;
        for r in &dos_rows {
            dos_y_max = dos_y_max.max(r[1]);
        }
        let n_extra = dos_rows[0].len() - 2;
        let element_colors = [RED, GREEN, MAGENTA, CYAN, YELLOW, BLACK];
        let mut chart = ChartBuilder::on(&right)
            .caption("Phonon DOS", ("sans-serif", 20))
            .margin(15)
            .x_label_area_size(35)
            .y_label_area_size(55)
            .build_cartesian_2d(0.0..dos_x_max, 0.0..y_max)?;
        chart
            .configure_mesh()
            .x_desc("Frequency (cm-1)")
            .y_desc("DOS")
            .draw()?;
        chart.draw_series(LineSeries::new(dos_rows.iter().map(|r| (r[0], r[1])), &BLACK))?;
        for e in 0..n_extra {
            let color = element_colors[e % element_colors.len()];
            chart.draw_series(LineSeries::new(dos_rows.iter().map(|r| (r[0], r[2 + e])), &color))?;
        }
    }
    let _ = elements;
    root.present()?;
    Ok(out.clone())
}
