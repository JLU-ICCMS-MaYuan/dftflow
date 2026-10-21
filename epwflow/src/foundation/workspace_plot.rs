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
    // 非磁标量路径两侧均由原生程序计入两重自旋，不能再次乘二。
    let wpout = dir.join(format!("{}.wpout", cfg.qe.prefix));
    let electrons = native_dos_electrons(&fs::read_to_string(&wpout)?)?;
    let wan = table(wannier, fermi)?;
    let mut csv = String::from("source,energy_minus_fermi_ev,states_per_ev_cell\n");
    for (name, rows) in [("QE", &qe), ("Wannier", &wan)] {
        for (e, d) in rows {
            csv.push_str(&format!("{name},{e:.12},{d:.12}\n"));
        }
    }
    fs::write(dir.join("electronic-dos.csv"), csv)?;
    super::store::write(
        &dir.join("electronic-dos-normalization.json"),
        &serde_json::json!({"qe_spin_degeneracy":2,"postw90_num_elec_per_state":electrons,"postw90_multiplier":1,"electronic_sigma_ev":cfg.comparison.clone().unwrap_or_default().electronic_sigma_ev,"raw_native_files_modified":false,"postw90_log":super::store::Identity::capture(&wpout)?,"native_dos":super::store::Identity::capture(wannier)?}),
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
        for range in path_sections(&x) {
            chart.draw_series(LineSeries::new(
                x[range.clone()]
                    .iter()
                    .copied()
                    .zip(y[range].iter().copied()),
                BLACK.stroke_width(2),
            ))?;
        }
    }
    for (x, y) in wan {
        for range in path_sections(&x) {
            chart.draw_series(DashedLineSeries::new(
                x[range.clone()]
                    .iter()
                    .map(|v| v * scale)
                    .zip(y[range].iter().copied()),
                6,
                4,
                RED.stroke_width(2),
            ))?;
        }
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
    for (x, name) in path_labels(&label)? {
        let (px, py) = chart.backend_coord(&(x * scale, lo));
        area.draw(&Text::new(
            name,
            (px, py + 12),
            TextStyle::from(("sans-serif", 18).into_font()).pos(Pos::new(HPos::Center, VPos::Top)),
        ))?;
    }
    area.present()?;
    Ok(())
}

fn native_dos_electrons(log: &str) -> Result<u32> {
    let re = regex::Regex::new(r"Number of electrons per state\s*:\s*(\d+)\b")?;
    let found = re
        .captures_iter(log)
        .last()
        .ok_or_else(|| anyhow::anyhow!("data_invalid: postw90 缺少每态电子数的原生证据"))?;
    let value: u32 = found[1].parse()?;
    ensure!(
        value == 2,
        "data_invalid: 当前非磁标量 DOS 要求原生每态电子数为 2，实际 {value}"
    );
    Ok(value)
}

fn path_sections(x: &[f64]) -> Vec<std::ops::Range<usize>> {
    let mut ranges = vec![];
    let mut start = 0;
    for i in 1..x.len() {
        if (x[i] - x[i - 1]).abs() <= 1e-9 {
            if i - start > 1 {
                ranges.push(start..i);
            }
            start = i;
        }
    }
    if x.len() - start > 1 {
        ranges.push(start..x.len());
    }
    ranges
}

fn path_labels(text: &str) -> Result<Vec<(f64, String)>> {
    let mut groups: Vec<(f64, Vec<String>)> = vec![];
    for line in text
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
    {
        let fields: Vec<_> = line.split_whitespace().collect();
        ensure!(fields.len() >= 3, "data_invalid: 高对称点标签格式不完整");
        let x: f64 = fields[2].parse()?;
        ensure!(x.is_finite(), "data_invalid: 高对称点位置非有限");
        let name = if matches!(fields[0], "G" | "GAMMA") {
            "Γ"
        } else {
            fields[0]
        }
        .to_string();
        if let Some((last, names)) = groups.last_mut() {
            ensure!(x + 1e-9 >= *last, "data_invalid: 高对称点位置倒序");
            if (x - *last).abs() <= 1e-9 {
                if !names.contains(&name) {
                    names.push(name);
                }
                continue;
            }
        }
        groups.push((x, vec![name]));
    }
    Ok(groups
        .into_iter()
        .map(|(x, names)| (x, names.join(" | ")))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_dos_spin_evidence_rejects_missing_or_incompatible_values() {
        assert_eq!(
            native_dos_electrons("| Number of electrons per state : 2 |").unwrap(),
            2
        );
        assert!(native_dos_electrons("").is_err());
        assert!(native_dos_electrons("Number of electrons per state : 1").is_err());
    }
    #[test]
    fn disconnected_paths_are_not_joined_and_shared_labels_are_combined() {
        assert_eq!(
            path_sections(&[0., 1., 1., 2., 3., 3., 4.]),
            vec![0..2, 2..5, 5..7]
        );
        assert_eq!(
            path_labels("GAMMA 1 0\nA 2 1\nL 3 1\nL 4 1\nM 5 2\nH 6 2\n").unwrap(),
            vec![(0., "Γ".into()), (1., "A | L".into()), (2., "M | H".into())]
        );
        assert!(path_labels("G 1 NaN").is_err());
        assert!(path_labels("G 1 2\nM 2 1").is_err());
    }
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
