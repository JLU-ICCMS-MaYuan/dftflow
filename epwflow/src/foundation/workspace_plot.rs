//! 读取两侧原生总 DOS，沿用共享绘图库，不生成替代科学数据。
use super::config::Config;
use anyhow::{ensure, Result};
use plotters::{
    prelude::*,
    style::text_anchor::{HPos, Pos, VPos},
};
use std::{fs, path::Path};
fn table(path: &Path, fermi: f64) -> Result<Vec<(f64, f64)>> {
    let mut rows = vec![];
    for line in fs::read_to_string(path)?.lines() {
        let v: Vec<f64> = line
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if v.len() >= 2 {
            ensure!(
                v[..2].iter().all(|x| x.is_finite()),
                "data_invalid: DOS 非有限"
            );
            rows.push((v[0] - fermi, v[1]));
        }
    }
    ensure!(
        rows.len() > 1 && rows.windows(2).all(|w| w[0].0 < w[1].0),
        "data_invalid: DOS 能量轴不递增"
    );
    Ok(rows)
}
pub fn dos(cfg: &Config, dir: &Path, wannier: &Path, fermi: f64) -> Result<()> {
    let qe = table(
        &cfg.work_dir
            .join("3.qe-electronic/qe_eledos")
            .join(format!("{}.tdos", cfg.qe.prefix)),
        fermi,
    )?;
    // 当前生产适配只支持非磁标量 QE；dos.x 含两重自旋，postw90 每 Wannier 态计一次。
    let wan = table(wannier, fermi)?
        .into_iter()
        .map(|(e, d)| (e, d * 2.0))
        .collect::<Vec<_>>();
    let mut csv = String::from("source,energy_minus_fermi_ev,states_per_ev_cell\n");
    for (name, rows) in [("QE", &qe), ("Wannier", &wan)] {
        for (e, d) in rows {
            csv.push_str(&format!("{name},{e:.12},{d:.12}\n"));
        }
    }
    fs::write(dir.join("electronic-dos.csv"), csv)?;
    super::store::write(
        &dir.join("electronic-dos-normalization.json"),
        &serde_json::json!({"qe_spin_degeneracy":2,"postw90_multiplier":2,"electronic_sigma_ev":cfg.comparison.clone().unwrap_or_default().electronic_sigma_ev,"raw_native_files_modified":false}),
    )?;
    let window = windows(cfg, dir, fermi)?;
    let lo = window[0].0.max(qe[0].0).max(wan[0].0);
    let hi = window[1]
        .0
        .min(qe.last().unwrap().0)
        .min(wan.last().unwrap().0);
    ensure!(lo < hi, "data_invalid: 两侧 DOS 没有共同能窗");
    let ymax = qe
        .iter()
        .chain(&wan)
        .filter(|(e, _)| *e >= lo && *e <= hi)
        .map(|(_, d)| *d)
        .fold(0.0, f64::max);
    ensure!(ymax > 0.0, "data_invalid: DOS 为空");
    let path = dir.join("electronic-dos.png");
    let backend = BitMapBackend::new(&path, (1000, 700));
    let area = backend.into_drawing_area();
    area.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&area)
        .margin(20)
        .x_label_area_size(45)
        .y_label_area_size(55)
        .build_cartesian_2d(lo..hi, 0.0..ymax * 1.05)?;
    chart
        .configure_mesh()
        .x_desc("E - E_F (eV)")
        .y_desc("DOS (states/eV/cell)")
        .draw()?;
    chart
        .draw_series(LineSeries::new(qe.iter().copied(), BLACK.stroke_width(2)))?
        .label("QE")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLACK));
    chart
        .draw_series(DashedLineSeries::new(
            wan.iter().copied(),
            6,
            4,
            RED.stroke_width(2),
        ))?
        .label("Wannier")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));
    for (e, name, color) in window {
        chart
            .draw_series(DashedLineSeries::new(
                [(e, 0.0), (e, ymax * 1.05)],
                4,
                4,
                color.stroke_width(1),
            ))?
            .label(name);
    }
    chart
        .configure_series_labels()
        .background_style(WHITE)
        .draw()?;
    area.present()?;
    Ok(())
}

pub fn bands(cfg: &Config, dir: &Path, wannier: &Path, fermi: f64) -> Result<()> {
    let qe = wannier90flow::plot::parse_segment_file(
        &cfg.work_dir
            .join("3.qe-electronic/qe_eband")
            .join(format!("{}_band.gnu", cfg.qe.prefix)),
        fermi,
    )?;
    let wan = wannier90flow::plot::parse_segment_file(wannier, fermi)?;
    ensure!(
        !qe.is_empty() && !wan.is_empty(),
        "data_invalid: 能带对比数据缺失"
    );
    let xmax = qe
        .iter()
        .flat_map(|(x, _)| x.iter().copied())
        .fold(0.0, f64::max);
    let window = windows(cfg, dir, fermi)?;
    let lo = window[0].0;
    let hi = window[1].0;
    let wx = wan
        .iter()
        .flat_map(|(x, _)| x.iter().copied())
        .fold(0.0, f64::max);
    ensure!(wx > 0.0, "data_invalid: Wannier 路径长度无效");
    let scale = xmax / wx;
    ensure!(xmax > 0.0 && lo < hi, "data_invalid: 能带范围为空");
    let file = dir.join("electronic-bands.png");
    let area = BitMapBackend::new(&file, (1000, 700)).into_drawing_area();
    area.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&area)
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(55)
        .build_cartesian_2d(0.0..xmax, lo..hi)?;
    chart
        .configure_mesh()
        .x_labels(0)
        .y_desc("E - E_F (eV)")
        .draw()?;
    for (x, y) in qe {
        chart.draw_series(LineSeries::new(x.into_iter().zip(y), BLACK.stroke_width(2)))?;
    }
    for (x, y) in wan {
        chart.draw_series(DashedLineSeries::new(
            x.into_iter().map(|v| v * scale).zip(y),
            6,
            4,
            RED.stroke_width(2),
        ))?;
    }
    for (e, name, color) in window {
        chart
            .draw_series(DashedLineSeries::new(
                [(0.0, e), (xmax, e)],
                4,
                4,
                color.stroke_width(1),
            ))?
            .label(name);
    }
    chart
        .configure_series_labels()
        .background_style(WHITE)
        .draw()?;
    let label = fs::read_to_string(dir.join(format!("{}_band.labelinfo.dat", cfg.qe.prefix)))?;
    for line in label.lines() {
        let v: Vec<_> = line.split_whitespace().collect();
        if v.len() < 3 {
            continue;
        }
        if let Ok(x) = v[2].parse::<f64>() {
            let name = if matches!(v[0], "G" | "GAMMA") {
                "Γ"
            } else {
                v[0]
            };
            let (px, py) = chart.backend_coord(&(x * scale, lo));
            area.draw(&Text::new(
                name,
                (px, py + 12),
                TextStyle::from(("sans-serif", 18).into_font())
                    .pos(Pos::new(HPos::Center, VPos::Top)),
            ))?;
        }
    }
    area.present()?;
    Ok(())
}

fn windows(cfg: &Config, dir: &Path, fermi: f64) -> Result<Vec<(f64, &'static str, RGBColor)>> {
    let log = fs::read_to_string(dir.join(format!("{}.wout", cfg.qe.prefix)))?;
    let outer = crate::interpolation::comparison::parse_outer_window(&log)?;
    let mut values = vec![
        (outer[0] - fermi, "dis_win_min", BLUE),
        (outer[1] - fermi, "dis_win_max", CYAN),
    ];
    if let Some(inner) = crate::interpolation::comparison::parse_frozen_window(&log)? {
        values.extend([
            (inner[0] - fermi, "dis_froz_min", RGBColor(255, 165, 0)),
            (inner[1] - fermi, "dis_froz_max", MAGENTA),
        ]);
    }
    Ok(values)
}
