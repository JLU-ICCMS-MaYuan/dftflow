//! 读取两侧原生总 DOS，沿用共享绘图库，不生成替代科学数据。
use super::config::Config;
use anyhow::{ensure, Context, Result};
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
    dos_figure(
        &dir.join("electronic-dos.png"),
        &qe,
        Some(&wan),
        &windows(cfg, dir, fermi)?,
    )
}

fn dos_figure(
    file: &Path,
    qe: &[(f64, f64)],
    wan: Option<&[(f64, f64)]>,
    window: &[(f64, &str, RGBColor)],
) -> Result<()> {
    let lo = window
        .first()
        .map_or(qe[0].0, |w| w.0)
        .max(qe[0].0)
        .max(wan.map_or(qe[0].0, |w| w[0].0));
    let hi = window
        .get(1)
        .map_or(qe.last().unwrap().0, |w| w.0)
        .min(qe.last().unwrap().0)
        .min(wan.map_or(qe.last().unwrap().0, |w| w.last().unwrap().0));
    ensure!(lo < hi, "data_invalid: 两侧 DOS 没有共同能窗");
    let ymax = qe
        .iter()
        .chain(wan.unwrap_or(&[]))
        .filter(|(e, _)| *e >= lo && *e <= hi)
        .map(|(_, d)| *d)
        .fold(0.0, f64::max);
    ensure!(ymax > 0.0, "data_invalid: DOS 为空");
    let backend = BitMapBackend::new(file, (1000, 700));
    let area = backend.into_drawing_area();
    area.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&area)
        .caption(
            if wan.is_some() {
                "QE / Wannier electronic DOS"
            } else {
                "QE electronic DOS"
            },
            ("sans-serif", 22),
        )
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
    if let Some(wan) = wan {
        chart
            .draw_series(DashedLineSeries::new(
                wan.iter().copied(),
                6,
                4,
                RED.stroke_width(2),
            ))?
            .label("Wannier")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));
    }
    for (e, name, color) in window.iter().copied() {
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
    let label = fs::read_to_string(dir.join(format!("{}_band.labelinfo.dat", cfg.qe.prefix)))?;
    let labels = path_labels(&label)?
        .into_iter()
        .map(|(x, name)| (x * scale, name))
        .collect::<Vec<_>>();
    bands_figure(
        &dir.join("electronic-bands.png"),
        &qe,
        Some(&wan),
        scale,
        [lo, hi],
        &window,
        &labels,
    )
}

type Band = (Vec<f64>, Vec<f64>);
fn bands_figure(
    file: &Path,
    qe: &[Band],
    wan: Option<&[Band]>,
    scale: f64,
    [lo, hi]: [f64; 2],
    window: &[(f64, &str, RGBColor)],
    labels: &[(f64, String)],
) -> Result<()> {
    let xmax = qe
        .iter()
        .flat_map(|(x, _)| x.iter().copied())
        .fold(0.0, f64::max);
    let area = BitMapBackend::new(file, (1000, 700)).into_drawing_area();
    area.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&area)
        .caption(
            if wan.is_some() {
                "QE / Wannier electronic bands"
            } else {
                "QE electronic bands"
            },
            ("sans-serif", 22),
        )
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
    if let Some(wan) = wan {
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
    }
    for (e, name, color) in window.iter().copied() {
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
    for (x, name) in labels {
        let (px, py) = chart.backend_coord(&(*x, lo));
        area.draw(&Text::new(
            name.clone(),
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

/// 单侧图直接读取电子任务产物，不依赖 PH 或实际 Wannier 能窗。
pub fn qe_dos(cfg: &Config, dir: &Path, fermi: f64) -> Result<()> {
    let rows = table(
        &dir.parent()
            .unwrap()
            .join(format!("{}.tdos", cfg.qe.prefix)),
        fermi,
    )?;
    let mut csv = String::from("energy_minus_fermi_ev,states_per_ev_cell\n");
    for (e, d) in &rows {
        csv.push_str(&format!("{e:.12},{d:.12}\n"));
    }
    fs::write(dir.join("electronic-dos.csv"), csv)?;
    dos_figure(&dir.join("electronic-dos.png"), &rows, None, &[])
}

pub fn qe_bands(cfg: &Config, dir: &Path, fermi: f64) -> Result<()> {
    let leaf = dir.parent().unwrap();
    let xml = leaf.join(format!("out/{}.save/data-file-schema.xml", cfg.qe.prefix));
    let states = crate::qe_data::nscf::parse_xml(&xml)?;
    let geometry = crate::qe_data::xml::Geometry::read(&xml)?;
    let points: Vec<[f64; 3]> = super::store::read(&leaf.join("path-points.json"))?;
    let segments = qf_input::electronic::parse_kpath_segments(
        cfg.kpath
            .as_ref()
            .context("required: 电子能带图需要 KPATH")?
            .to_str()
            .context("KPATH 非 UTF-8")?,
    )?;
    let intervals = cfg.comparison.clone().unwrap_or_default().points_per_edge as usize;
    ensure!(
        intervals > 0 && states.kpoints.len() == points.len(),
        "data_invalid: 电子路径点数不匹配"
    );
    let mut x = Vec::new();
    let mut label_text = String::new();
    let mut index = 0;
    let mut distance = 0.0;
    for segment in segments {
        let count = (segment.len() - 1) * intervals + 1;
        ensure!(
            index + count <= points.len(),
            "data_invalid: 电子路径索引越界"
        );
        for j in 0..count {
            let at = index + j;
            let native = states.kpoints[at];
            let expected = std::array::from_fn::<_, 3, _>(|a| {
                (0..3)
                    .map(|b| points[at][b] * geometry.reciprocal[b][a])
                    .sum::<f64>()
            });
            ensure!(
                native
                    .iter()
                    .zip(expected)
                    .all(|(a, b)| (a - b).abs() < 1e-7),
                "data_invalid: 电子路径坐标与原生结果不符"
            );
            if j > 0 {
                distance += (0..3)
                    .map(|a| (native[a] - states.kpoints[at - 1][a]).powi(2))
                    .sum::<f64>()
                    .sqrt();
            }
            x.push(distance);
        }
        for (j, (_, label)) in segment.iter().enumerate() {
            label_text.push_str(&format!(
                "{label} {} {}\n",
                index + j * intervals + 1,
                x[index + j * intervals]
            ));
        }
        index += count;
    }
    ensure!(
        index == points.len() && distance > 0.0,
        "data_invalid: 电子路径覆盖不完整"
    );
    let mut csv = String::from("point_index,distance,band,energy_minus_fermi_ev\n");
    let nband = states.nbnd;
    ensure!(
        states.eigenvalues.len() == x.len() && nband > 0,
        "data_invalid: 电子本征值缺失"
    );
    let mut bands = Vec::new();
    for band in 0..nband {
        let mut energies = Vec::new();
        for (i, row) in states.eigenvalues.iter().enumerate() {
            ensure!(row.len() == nband, "data_invalid: 电子能带数不一致");
            let energy = row[band] - fermi;
            csv.push_str(&format!(
                "{},{:.12},{},{:.12}\n",
                i + 1,
                x[i],
                band + 1,
                energy
            ));
            energies.push(energy);
        }
        bands.push((x.clone(), energies));
    }
    let lo = bands
        .iter()
        .flat_map(|(_, y)| y)
        .copied()
        .fold(f64::INFINITY, f64::min);
    let hi = bands
        .iter()
        .flat_map(|(_, y)| y)
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    ensure!(
        lo.is_finite() && hi.is_finite(),
        "data_invalid: 电子能量非有限"
    );
    let padding = ((hi - lo) * 0.05).max(0.1);
    fs::write(dir.join("electronic-bands.csv"), csv)?;
    bands_figure(
        &dir.join("electronic-bands.png"),
        &bands,
        None,
        1.0,
        [lo - padding, hi + padding],
        &[],
        &path_labels(&label_text)?,
    )
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
