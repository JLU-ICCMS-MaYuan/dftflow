//! 从已核验的电子采样绘制比较图；此模块不执行科学程序。
use super::{config::Config, electronic_style as style};
use anyhow::{ensure, Context, Result};
use plotters::{
    element::DashedPathElement,
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
    let mesh =
        crate::qe_data::nscf::parse_xml(&crate::interpolation::electronic_sampling::xml(cfg))?;
    let wan = table(wannier, fermi)?;
    let mut axis = wan.iter().map(|(e, _)| *e).collect::<Vec<_>>();
    // QE 已有全带本征值；在原 postw90 步长上向下延伸 QE 参考，不替插值补零。
    let minimum = mesh
        .eigenvalues
        .iter()
        .flatten()
        .copied()
        .fold(f64::INFINITY, f64::min)
        - fermi;
    let step = axis[1] - axis[0];
    ensure!(
        step > 0. && axis.windows(2).all(|p| ((p[1] - p[0]) - step).abs() < 1e-5),
        "data_invalid: postw90 DOS 能网格非均匀"
    );
    let first = axis[0];
    let extra = ((first - minimum).max(0.) / step).ceil() as usize;
    ensure!(extra < 1000000, "data_invalid: DOS 扩展点数异常");
    let mut extended = (1..=extra)
        .rev()
        .map(|i| first - i as f64 * step)
        .collect::<Vec<_>>();
    extended.extend(axis);
    axis = extended;

    let states = mesh
        .eigenvalues
        .iter()
        .map(|r| r.iter().map(|e| e - fermi).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let sigma = cfg
        .comparison
        .clone()
        .unwrap_or_default()
        .electronic_sigma_ev;
    let qe = axis
        .iter()
        .copied()
        .zip(crate::interpolation::comparison::gaussian_dos(
            &axis,
            &states,
            &vec![2.0 / mesh.kpoints.len() as f64; mesh.kpoints.len()],
            sigma,
        )?)
        .collect::<Vec<_>>();
    // 非磁标量路径两侧均由原生程序计入两重自旋，不能再次乘二。
    let wpout = dir.join(format!("{}.wpout", cfg.qe.prefix));
    let electrons = native_dos_electrons(&fs::read_to_string(&wpout)?)?;
    let mut csv = String::from("source,energy_minus_fermi_ev,states_per_ev_cell\n");
    for (name, rows) in [("QE", &qe), ("EPW", &wan)] {
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
    let outer = outer_bounds(&window)?;
    dos_figure(
        &dir.join("electronic-dos.png"),
        &qe,
        Some(&wan),
        &window,
        &metrics_note(dir, "electronic_mesh")?,
        Some(outer),
        &[],
    )?;
    dos_figure(
        &dir.join("electronic-dos-valence.png"),
        &qe,
        Some(&wan),
        &window,
        &metrics_note(dir, "electronic_mesh")?,
        Some(display_bounds(cfg, dir, fermi, Some(outer))?),
        &[],
    )
}

/// 窗口值已相对 E_F；必须按名字查找，不能混淆绝对窗或 frozen 边界。
fn outer_bounds(window: &[(f64, &str, RGBColor)]) -> Result<[f64; 2]> {
    let boundary = |name| -> Result<f64> {
        let values = window.iter().filter(|v| v.1 == name).map(|v| v.0).collect::<Vec<_>>();
        ensure!(values.len() == 1 && values[0].is_finite(),
            "data_invalid: 默认插值图需要唯一且有限的实际 {name}");
        Ok(values[0])
    };
    let bounds = [boundary("dis_win_min")?, boundary("dis_win_max")?];
    ensure!(bounds[0] < bounds[1], "data_invalid: 实际 Wannier 外窗顺序错误");
    Ok(bounds)
}
fn is_primary(file: &Path) -> bool {
    matches!(file.file_name().and_then(|s| s.to_str()),
        Some("electronic-bands.png" | "electronic-dos.png"))
}
fn primary_bounds(file: &Path, native: bool, window: &[(f64, &str, RGBColor)],
                  previous: Option<[f64; 2]>) -> Result<Option<[f64; 2]>> {
    if native && is_primary(file) { Ok(Some(outer_bounds(window)?)) } else { Ok(previous) }
}
fn sync_default_bounds(file: &Path, native: bool, bounds: [f64; 2]) -> Result<()> {
    if native && is_primary(file) {
        let metadata = file.parent().unwrap().join("electronic-display.json");
        if metadata.exists() {
            let mut value: serde_json::Value = super::store::read(&metadata)?;
            let object = value.as_object_mut().context("data_invalid: 电子显示参数必须为对象")?;
            object.insert("default_ev".into(), serde_json::json!(bounds));
            object.insert("default_preset".into(), serde_json::json!("wannier_outer"));
            super::store::write(&metadata, &value)?;
        }
    }
    Ok(())
}

fn dos_figure(
    file: &Path,
    qe: &[(f64, f64)],
    wan: Option<&[(f64, f64)]>,
    window: &[(f64, &str, RGBColor)],
    note: &str,
    bounds: Option<[f64; 2]>,
    elements: &[(String, Vec<(f64, f64)>)],
) -> Result<()> {
    let bounds = primary_bounds(file, wan.is_some(), window, bounds)?;
    super::store::write(
        &plot_file(file),
        &PlotData::Dos {
            qe: qe.to_vec(),
            wan: wan.map(|w| w.to_vec()),
            window: stored_window(window),
            note: note.into(),
            bounds,
            elements: elements.to_vec(),
        },
    )?;
    let [lo, hi] = bounds.unwrap_or([-5., 5.]);
    ensure!(lo < hi, "data_invalid: DOS 显示范围错误");
    let ymax = qe
        .iter()
        .chain(wan.unwrap_or(&[]))
        .chain(elements.iter().flat_map(|(_, r)| r.iter()))
        .filter(|(e, _)| *e >= lo && *e <= hi)
        .map(|(_, d)| *d)
        .fold(0.0, f64::max);
    ensure!(ymax > 0.0, "data_invalid: DOS 为空");
    let mut svg = String::new();
    {
        let backend = SVGBackend::with_string(&mut svg, (1000, 700));
        let area = backend.into_drawing_area();
        area.fill(&WHITE)?;
        let mut chart = ChartBuilder::on(&area)
            .caption(
                if wan.is_some() {
                    "QE / EPW electronic DOS"
                } else {
                    "QE electronic DOS"
                },
                ("sans-serif", 22),
            )
            .margin(20)
            .margin_right(240)
            .x_label_area_size(45)
            .y_label_area_size(55)
            .build_cartesian_2d(lo..hi, 0.0..ymax * 1.05)?;
        chart
            .configure_mesh()
            .x_desc("E - E_F (eV)")
            .y_desc("DOS (states/eV/cell)")
            .max_light_lines(0)
            .bold_line_style(RGBColor(235, 235, 235))
            .draw()?;
        chart.draw_series(LineSeries::new(
            dos_points(qe, lo, hi),
            style::QE.stroke_width(if wan.is_some() || !elements.is_empty() {
                style::REFERENCE_WIDTH
            } else { 2 }),
        ))?;
        for (i, (_, rows)) in elements.iter().enumerate() {
            let color = style::ELEMENTS[i % style::ELEMENTS.len()];
            let stroke = style::element_stroke(i);
            chart.draw_series(DashedLineSeries::new(
                dos_points(rows, lo, hi), stroke.dash, stroke.gap,
                color.stroke_width(stroke.width),
            ))?;
        }
        if let Some(wan) = wan {
            chart.draw_series(DashedLineSeries::new(
                dos_points(wan, lo, hi),
                style::INTERPOLATED_STROKE.dash,
                style::INTERPOLATED_STROKE.gap,
                style::INTERPOLATED.stroke_width(style::INTERPOLATED_STROKE.width),
            ))?;
        }
        for (e, _, color) in window.iter().copied() {
            chart.draw_series(DashedLineSeries::new(
                if e >= lo && e <= hi {
                    vec![(e, 0.), (e, ymax * 1.05)]
                } else {
                    vec![]
                },
                4,
                4,
                color.stroke_width(1),
            ))?;
        }
        chart.draw_series(std::iter::once(PathElement::new(
            [(0., 0.), (0., ymax * 1.05)],
            style::FERMI.stroke_width(2),
        )))?;
        outside_legend(
            &area,
            wan.is_some(),
            window,
            &elements.iter().map(|e| e.0.clone()).collect::<Vec<_>>(),
        )?;
        let coverage = if let Some(wan) = wan {
            format!("DOS support: QE [{:.2}, {:.2}], EPW [{:.2}, {:.2}] eV; missing ranges are not filled.",qe[0].0,qe.last().unwrap().0,wan[0].0,wan.last().unwrap().0)
        } else {
            format!(
                "QE DOS support [{:.2}, {:.2}] eV",
                qe[0].0,
                qe.last().unwrap().0
            )
        };
        area.draw(&Text::new(
            coverage,
            (75, 665),
            ("sans-serif", 11).into_font(),
        ))?;
        if !note.is_empty() {
            area.draw(&Text::new(
                note.to_owned(),
                (75, 685),
                ("sans-serif", 11).into_font(),
            ))?;
        }
        area.present()?;
    }
    style::png(&svg, file, 3000, 2100)?;
    sync_default_bounds(file, wan.is_some(), [lo, hi])
}

pub fn bands(cfg: &Config, dir: &Path, wannier: &Path, fermi: f64) -> Result<()> {
    let reference_sampling = crate::interpolation::electronic_sampling::plan(cfg)?;
    let independent = crate::interpolation::electronic_sampling::independent_sampling(dir)?;
    let sampling: crate::interpolation::electronic_sampling::Sampling = if independent {
        super::store::read(&dir.join("sampling.json"))?
    } else {
        crate::interpolation::electronic_sampling::plan(cfg)?
    };
    let qe_xml = cfg
        .work_dir
        .join("3.qe-electronic/qe_eband/out")
        .join(format!("{}.save/data-file-schema.xml", cfg.qe.prefix));
    let states = crate::qe_data::nscf::parse_xml(&qe_xml)?;
    let geometry = crate::qe_data::xml::Geometry::read(&qe_xml)?;
    geometry.validate(&crate::qe_data::qe_config::QeSettings::from_config(cfg)?)?;
    let native = crate::interpolation::native::parse_bands(&fs::read_to_string(wannier)?)?;
    let n = sampling.path.len();
    ensure!(
        states.kpoints.len() == reference_sampling.path.len()
            && native.values.len()
                == if independent {
                    n
                } else {
                    n + sampling.mesh.len()
                },
        "data_invalid: 电子比较路径或网格点数不符"
    );
    for (path, coords) in [
        (&reference_sampling.path, &states.kpoints),
        (&sampling.path, &native.points_cartesian),
    ] {
        for (i, p) in path.iter().enumerate() {
            let expected =
                crate::interpolation::electronic_sampling::cart(p.crystal, &sampling.reciprocal);
            ensure!(
                expected
                    .iter()
                    .zip(&coords[i])
                    .all(|(a, b)| (a - b).abs() < 1e-7),
                "input_conflict: 电子原生坐标与路径不同，点 {i}"
            );
        }
    }
    let to_bands = |path: &[crate::interpolation::comparison::PathPoint],
                    rows: &[Vec<f64>]|
     -> Result<Vec<Band>> {
        ensure!(
            !rows.is_empty() && rows.iter().all(|r| r.len() == rows[0].len()),
            "data_invalid: 电子带数变化"
        );
        let mut curves = vec![];
        for segment in path
            .iter()
            .map(|p| p.segment_id)
            .collect::<std::collections::BTreeSet<_>>()
        {
            let indices = path
                .iter()
                .enumerate()
                .filter(|(_, p)| p.segment_id == segment)
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            for b in 0..rows[0].len() {
                curves.push((
                    indices.iter().map(|&i| path[i].distance).collect(),
                    indices.iter().map(|&i| rows[i][b] - fermi).collect(),
                ));
            }
        }
        Ok(curves)
    };
    let qe = to_bands(&reference_sampling.path, &states.eigenvalues)?;
    let wan = to_bands(&sampling.path, &native.values[..n])?;
    let window = windows(cfg, dir, fermi)?;
    let bounds = [window[0].0, window[1].0];
    let labels = path_labels(
        &sampling
            .ticks
            .iter()
            .enumerate()
            .map(|(i, t)| format!("{} {i} {:.17}", t.label, t.distance))
            .collect::<Vec<_>>()
            .join("\n"),
    )?;
    let mut csv = String::from("method,segment_id,point_id,distance,band,energy_ev\n");
    for (method, path, rows) in [
        ("qe", &reference_sampling.path, &states.eigenvalues[..]),
        ("epw", &sampling.path, &native.values[..n]),
    ] {
        for (p, row) in path.iter().zip(rows) {
            for (b, e) in row.iter().enumerate() {
                csv.push_str(&format!(
                    "{method},{},{},{:.15},{},{:.15}\n",
                    p.segment_id,
                    p.point_id,
                    p.distance,
                    b + 1,
                    e - fermi
                ));
            }
        }
    }
    fs::write(dir.join("electronic-bands.csv"), csv)?;
    let path = crate::interpolation::electronic_sampling::common_difference(
        &reference_sampling.path,
        &states.eigenvalues,
        &sampling.path,
        &native.values[..n],
        fermi,
        bounds,
    )?;
    let mesh_stats = if independent {
        crate::interpolation::differences::electronic(&[], None, &[], &[], fermi, bounds)?
    } else {
        let mesh =
            crate::qe_data::nscf::parse_xml(&crate::interpolation::electronic_sampling::xml(cfg))?;
        crate::interpolation::differences::electronic(
            &sampling.mesh,
            None,
            &mesh.eigenvalues,
            &native.values[n..],
            fermi,
            bounds,
        )?
    };
    super::store::write(
        &dir.join("metrics.json"),
        &serde_json::json!({"schema_version":2,
        "independent_electronic_sampling":independent,
        "electronic_path":path,"electronic_mesh":mesh_stats,"matching":"ordered_minimum_squared_energy_pairs",
        "note":"Energy pairs do not establish eigenstate identity"}),
    )?;
    bands_figure(
        &dir.join("electronic-bands.png"),
        &qe,
        Some(&wan),
        bounds,
        &window,
        &labels,
        &metrics_note(dir, "electronic_path")?,
    )?;
    bands_figure(
        &dir.join("electronic-bands-valence.png"),
        &qe,
        Some(&wan),
        display_bounds(cfg, dir, fermi, Some(bounds))?,
        &window,
        &labels,
        &metrics_note(dir, "electronic_path")?,
    )
}

type Band = (Vec<f64>, Vec<f64>);
pub(crate) fn bands_figure(
    file: &Path,
    qe: &[Band],
    wan: Option<&[Band]>,
    bounds: [f64; 2],
    window: &[(f64, &str, RGBColor)],
    labels: &[(f64, String)],
    note: &str,
) -> Result<()> {
    let [lo, hi] = primary_bounds(file, wan.is_some(), window, Some(bounds))?
        .context("data_invalid: 能带缺少显示范围")?;
    super::store::write(
        &plot_file(file),
        &PlotData::Bands {
            qe: qe.to_vec(),
            wan: wan.map(|w| w.to_vec()),
            bounds: [lo, hi],
            window: stored_window(window),
            labels: labels.to_vec(),
            note: note.into(),
        },
    )?;
    let xmax = qe
        .iter()
        .flat_map(|(x, _)| x.iter().copied())
        .fold(0.0, f64::max);
    let mut svg = String::new();
    {
        let area = SVGBackend::with_string(&mut svg, (1000, 700)).into_drawing_area();
        area.fill(&WHITE)?;
        let mut chart = ChartBuilder::on(&area)
            .caption(
                if note.starts_with("UNCONVERGED") {
                    "UNCONVERGED Wannier fit / QE"
                } else if wan.is_some() {
                    "QE / EPW electronic bands"
                } else {
                    "QE electronic bands"
                },
                ("sans-serif", 22),
            )
            .margin(20)
            .margin_right(240)
            .x_label_area_size(40)
            .y_label_area_size(55)
            .build_cartesian_2d(0.0..xmax, lo..hi)?;
        chart
            .configure_mesh()
            .x_labels(0)
            .y_desc("E - E_F (eV)")
            .max_light_lines(0)
            .bold_line_style(RGBColor(235, 235, 235))
            .draw()?;
        for (x, y) in qe {
            for range in path_sections(&x) {
                let points = x[range.clone()]
                    .iter()
                    .copied()
                    .zip(y[range].iter().copied())
                    .collect::<Vec<_>>();
                for points in crate::delivery::comparison_plot::clip(&points, lo, hi) {
                    chart.draw_series(LineSeries::new(
                        points,
                        style::QE.stroke_width(if wan.is_some() { style::REFERENCE_WIDTH } else { 2 }),
                    ))?;
                }
            }
        }
        if let Some(wan) = wan {
            for (x, y) in wan {
                for range in path_sections(&x) {
                    let points = x[range.clone()]
                        .iter()
                        .copied()
                        .zip(y[range].iter().copied())
                        .collect::<Vec<_>>();
                    for points in crate::delivery::comparison_plot::clip(&points, lo, hi) {
                        chart.draw_series(DashedLineSeries::new(
                            points,
                            style::INTERPOLATED_STROKE.dash,
                            style::INTERPOLATED_STROKE.gap,
                            style::INTERPOLATED.stroke_width(style::INTERPOLATED_STROKE.width),
                        ))?;
                    }
                }
            }
        }
        for (e, _, color) in window.iter().copied() {
            chart.draw_series(DashedLineSeries::new(
                if e >= lo && e <= hi {
                    vec![(0., e), (xmax, e)]
                } else {
                    vec![]
                },
                4,
                4,
                color.stroke_width(1),
            ))?;
        }
        chart.draw_series(std::iter::once(PathElement::new(
            [(0., 0.), (xmax, 0.)],
            style::FERMI.stroke_width(2),
        )))?;
        outside_legend(&area, wan.is_some(), window, &[])?;
        for (x, name) in separated_labels(qe, labels) {
            let (px, py) = chart.backend_coord(&(x, lo));
            area.draw(&Text::new(
                name.clone(),
                (px, py + 12),
                TextStyle::from(("sans-serif", 18).into_font())
                    .pos(Pos::new(HPos::Center, VPos::Top)),
            ))?;
        }
        if !note.is_empty() {
            area.draw(&Text::new(
                note.to_owned(),
                (75, 685),
                ("sans-serif", 11).into_font(),
            ))?;
        }
        area.present()?;
    }
    style::png(&svg, file, 3000, 2100)?;
    sync_default_bounds(file, wan.is_some(), [lo, hi])
}

/// 图例放在绘图区右侧的独立留白，避免遮挡窄峰或切断费米线。
fn outside_legend(
    area: &DrawingArea<SVGBackend<'_>, plotters::coord::Shift>,
    native: bool,
    window: &[(f64, &str, RGBColor)],
    elements: &[String],
) -> Result<()> {
    let mut entries = vec![(
        "QE".to_string(),
        style::QE,
        None,
    )];
    if native {
        entries.push((
            "EPW".into(),
            style::INTERPOLATED,
            Some(style::INTERPOLATED_STROKE),
        ));
    }
    for (i, name) in elements.iter().enumerate() {
        entries.push((
            format!("{name} PDOS"),
            style::ELEMENTS[i % style::ELEMENTS.len()],
            Some(style::element_stroke(i)),
        ));
    }
    for (e, name, color) in window {
        entries.push((
            format!("{name}: {e:.3} eV"),
            *color,
            Some(style::ComparisonStroke {
                dash: 4,
                gap: 4,
                width: 1,
            }),
        ));
    }
    entries.push(("E_F = 0 eV".into(), style::FERMI, None));
    for (i, (label, color, stroke)) in entries.iter().enumerate() {
        let y = 80 + i as i32 * 25;
        if let Some(stroke) = stroke {
            area.draw(&DashedPathElement::new(
                [(775, y), (805, y)],
                stroke.dash,
                stroke.gap,
                color.stroke_width(stroke.width),
            ))?;
        } else {
            area.draw(&PathElement::new(
                [(775, y), (805, y)],
                color.stroke_width(if *color == style::QE && (native || !elements.is_empty()) { style::REFERENCE_WIDTH } else { 2 }),
            ))?;
        }
        area.draw(&Text::new(
            label.clone(),
            (812, y),
            ("sans-serif", 12).into_font(),
        ))?;
    }
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
    let rows = if !super::workspace_pdos::files(cfg)?.is_empty() {
        super::workspace_pdos::projected_total(cfg, fermi)?
    } else {
        table(
            &dir.parent()
                .unwrap()
                .join(format!("{}.tdos", cfg.qe.prefix)),
            fermi,
        )?
    };
    let mut csv = String::from("energy_minus_fermi_ev,states_per_ev_cell\n");
    for (e, d) in &rows {
        csv.push_str(&format!("{e:.12},{d:.12}\n"));
    }
    fs::write(dir.join("electronic-dos.csv"), csv)?;
    let elements = super::workspace_pdos::elements(cfg, fermi, &rows)?;
    let note = if elements.is_empty() {
        "Element PDOS missing; TDOS only (incomplete)"
    } else {
        ""
    };
    dos_figure(
        &dir.join("electronic-dos.png"),
        &rows,
        None,
        &[],
        note,
        Some([-5., 5.]),
        &elements,
    )?;
    dos_figure(
        &dir.join("electronic-dos-valence.png"),
        &rows,
        None,
        &[],
        note,
        Some(display_bounds(cfg, dir, fermi, None)?),
        &elements,
    )
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
    fs::write(dir.join("electronic-bands.csv"), csv)?;
    bands_figure(
        &dir.join("electronic-bands.png"),
        &bands,
        None,
        [-5., 5.],
        &[],
        &path_labels(&label_text)?,
        "",
    )?;
    bands_figure(
        &dir.join("electronic-bands-valence.png"),
        &bands,
        None,
        display_bounds(cfg, dir, fermi, None)?,
        &[],
        &path_labels(&label_text)?,
        "",
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

/// 很短的人工段间空隙会使 A／L 等标签互相覆盖。只合并已确认段尾／段首的文字，
/// 标签居中于空隙；真实点位、曲线横坐标与 CSV 均保持原样。
fn separated_labels(bands: &[Band], labels: &[(f64, String)]) -> Vec<(f64, String)> {
    let span = bands
        .iter()
        .flat_map(|b| b.0.iter())
        .copied()
        .fold(0., f64::max);
    let mut out: Vec<(f64, String)> = Vec::new();
    for (x, name) in labels {
        if let Some((previous, text)) = out.last_mut() {
            let tail = bands
                .iter()
                .any(|b| b.0.last().is_some_and(|v| (v - *previous).abs() < 1e-9));
            let head = bands
                .iter()
                .any(|b| b.0.first().is_some_and(|v| (v - x).abs() < 1e-9));
            if tail && head && *x > *previous && x - *previous < span * 0.025 {
                *previous = (*previous + x) / 2.;
                *text = format!("{text} | {name}");
                continue;
            }
        }
        out.push((*x, name.clone()));
    }
    out
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
        let curves = vec![(vec![0., 1.], vec![0., 1.]), (vec![1.01, 2.], vec![2., 3.])];
        assert_eq!(
            separated_labels(
                &curves,
                &[
                    (0., "Γ".into()),
                    (1., "A".into()),
                    (1.01, "L".into()),
                    (2., "M".into())
                ]
            ),
            vec![(0., "Γ".into()), (1.005, "A | L".into()), (2., "M".into())]
        );
        assert_eq!(curves[1].0[0], 1.01, "标签布局不改变科学点位或曲线坐标");
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
        (outer[0] - fermi, "dis_win_min", style::WINDOWS[0]),
        (outer[1] - fermi, "dis_win_max", style::WINDOWS[1]),
    ];
    if let Some(inner) = crate::interpolation::comparison::parse_frozen_window(&log)? {
        values.extend([
            (inner[0] - fermi, "dis_froz_min", style::WINDOWS[2]),
            (inner[1] - fermi, "dis_froz_max", style::WINDOWS[3]),
        ]);
    }
    Ok(values)
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind")]
enum PlotData {
    Bands {
        qe: Vec<Band>,
        wan: Option<Vec<Band>>,
        bounds: [f64; 2],
        window: Vec<(f64, String, [u8; 3])>,
        labels: Vec<(f64, String)>,
        #[serde(default)]
        note: String,
    },
    Dos {
        qe: Vec<(f64, f64)>,
        wan: Option<Vec<(f64, f64)>>,
        window: Vec<(f64, String, [u8; 3])>,
        #[serde(default)]
        note: String,
        #[serde(default)]
        bounds: Option<[f64; 2]>,
        #[serde(default)]
        elements: Vec<(String, Vec<(f64, f64)>)>,
    },
}
fn stored_window(window: &[(f64, &str, RGBColor)]) -> Vec<(f64, String, [u8; 3])> {
    window
        .iter()
        .map(|(e, n, c)| (*e, n.to_string(), [c.0, c.1, c.2]))
        .collect()
}
fn plot_file(file: &Path) -> std::path::PathBuf {
    file.with_extension("plot.json")
}
/// 只使用现场保存的绘图参数覆盖原图，不运行科学程序或改变结果包。
pub fn redraw_current(cfg: &Config) -> Result<Option<std::path::PathBuf>> {
    let stage = super::stages::Step::Wannier.directory(cfg);
    let dir = super::layout::path(&cfg.work_dir, "comparison-native/electron");
    let names = [
        "electronic-bands", "electronic-dos",
        "electronic-bands-valence", "electronic-dos-valence",
    ];
    let inputs: Vec<_> = names.iter()
        .map(|name| dir.join(format!("{name}.plot.json")))
        .filter(|path| path.is_file())
        .collect();
    if inputs.is_empty() {
        return Ok(None);
    }
    for input in inputs {
        redraw(&input)?;
    }
    for name in ["electronic-bands.png", "electronic-dos.png"] {
        let source = dir.join(name);
        if source.is_file() {
            fs::copy(&source, stage.join(name))?;
        }
    }
    Ok(Some(dir))
}

pub fn redraw(file: &Path) -> Result<()> {
    let data: PlotData = super::store::read(file)?;
    let target = file.with_file_name(
        file.file_name()
            .unwrap()
            .to_string_lossy()
            .replace(".plot.json", ".png"),
    );
    let window_values = match &data {
        PlotData::Bands { window, .. } | PlotData::Dos { window, .. } => window,
    };
    let window = window_values
        .iter()
        .enumerate()
        .map(|(i, (e, n, _))| (*e, n.as_str(), style::WINDOWS[i % 4]))
        .collect::<Vec<_>>();
    match &data {
        PlotData::Bands {
            qe,
            wan,
            bounds,
            labels,
            note,
            ..
        } => bands_figure(&target, qe, wan.as_deref(), *bounds, &window, labels, note),
        PlotData::Dos {
            qe,
            wan,
            note,
            bounds,
            elements,
            ..
        } => dos_figure(
            &target,
            qe,
            wan.as_deref(),
            &window,
            if wan.is_none() && elements.is_empty() {
                "Element PDOS missing; TDOS only (incomplete)"
            } else {
                note
            },
            *bounds,
            elements,
        ),
    }
}

fn metrics_note(dir: &Path, kind: &str) -> Result<String> {
    let data: serde_json::Value = super::store::read(&dir.join("metrics.json"))?;
    let stats: crate::interpolation::differences::Statistics =
        serde_json::from_value(data[kind].clone())?;
    if data["independent_electronic_sampling"] == true {
        return Ok(stats.maximum.map_or_else(String::new, |m| {
            format!("Max |dE| = {:.6} eV", m.absolute_difference)
        }));
    }
    Ok(format!(
        "{}; paired {}; unmatched QE {}, EPW {}. Energy pairing only.",
        stats.maximum.map_or_else(
            || "No paired states".into(),
            |m| format!("Max |dE| = {:.6} eV", m.absolute_difference)
        ),
        stats.paired_states,
        stats.unmatched_qe_states,
        stats.unmatched_epw_states
    ))
}

/// 显示下界只取共同 NSCF 有限采样中的最低占据态，不影响科学比较窗。
fn display_bounds(cfg: &Config, dir: &Path, fermi: f64, default: Option<[f64; 2]>) -> Result<[f64; 2]> {
    let xml = crate::interpolation::electronic_sampling::xml(cfg);
    let states = crate::qe_data::nscf::parse_xml(&xml)?;
    let minimum = states
        .eigenvalues
        .iter()
        .flatten()
        .copied()
        .filter(|e| *e <= fermi)
        .fold(f64::INFINITY, f64::min);
    ensure!(
        minimum.is_finite(),
        "data_invalid: NSCF 缺占据态，无法确定最低价带"
    );
    super::store::write(
        &dir.join("electronic-display.json"),
        &serde_json::json!({
        "schema_version":1,"near_ev":[-5.,5.],"valence_ev":[minimum-fermi,5.],
        "default_ev":default.unwrap_or([-5.,5.]),
        "default_preset":if default.is_some() {"wannier_outer"} else {"fermi_pm5"},
        "minimum_occupied_ev":minimum,"fermi_ev":fermi,"reference":super::store::Identity::capture(&xml)?,
        "minimum_scope":"finite QE NSCF sampling, explicit pseudopotential valence states",
        "pixels":[3000,2100],"dpi":300.0248,"fermi_color":[220,0,0],"interpolated_color":[0,70,210],
        "display_window_is_not_metric_window":true}),
    )?;
    Ok([minimum - fermi, 5.])
}

fn dos_points(rows: &[(f64, f64)], lo: f64, hi: f64) -> Vec<(f64, f64)> {
    let swapped = rows.iter().map(|&(e, d)| (d, e)).collect::<Vec<_>>();
    crate::delivery::comparison_plot::clip(&swapped, lo, hi)
        .into_iter()
        .flatten()
        .map(|(d, e)| (e, d))
        .collect()
}

#[cfg(test)]
mod electronic_delivery_tests {
    use super::*;
    #[test]
    fn legacy_redraw_reserves_red_for_fermi_and_blue_for_interpolation_at_300_dpi() {
        let root = std::env::temp_dir().join(format!(
            "electronic-colors-{}",
            super::super::store::token()
        ));
        fs::create_dir_all(&root).unwrap();
        // 历史包把红蓝给窗口，重绘也必须纠正；其他输入无红蓝。
        let file = root.join("dense.plot.json");
        super::super::store::write(&file,&serde_json::json!({"kind":"Dos","qe":[[-5.,0.],[0.,1.],[5.,0.]],"wan":[[-5.,0.],[0.,0.5],[5.,0.]],"window":[[-2.,"dis_win_min",[0,0,255]],[2.,"dis_win_max",[255,0,0]]],"note":"legacy"})).unwrap();
        let mut legacy: serde_json::Value = super::super::store::read(&file).unwrap();
        legacy["qe"] = serde_json::json!((0..20001)
            .map(|i| {
                let e = -5. + i as f64 * 0.0005;
                (e, (-((e + 2.37) / 0.05).powi(2)).exp())
            })
            .collect::<Vec<_>>());
        super::super::store::write(&file, &legacy).unwrap();
        redraw(&file).unwrap();
        let decoder = png::Decoder::new(fs::File::open(root.join("dense.png")).unwrap());
        let mut reader = decoder.read_info().unwrap();
        assert_eq!((reader.info().width, reader.info().height), (3000, 2100));
        assert_eq!(reader.info().pixel_dims.unwrap().xppu, 11812);
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let pixels = &buf[..info.buffer_size()];
        let red = pixels
            .chunks_exact(4)
            .enumerate()
            .filter(|(_, p)| p[0] > 200 && p[1] < 30 && p[2] < 30)
            .map(|(i, _)| i % 3000)
            .collect::<Vec<_>>();
        let blue = pixels
            .chunks_exact(4)
            .filter(|p| p[0] < 30 && p[1] < 100 && p[2] > 180)
            .count();
        assert!(red.len() > 1000 && blue > 1000);
        // 对比图允许设计内的虚线间隔；密集折线碎片另用 QE 单侧实线检查。
        legacy["wan"] = serde_json::Value::Null;
        super::super::store::write(&file, &legacy).unwrap();
        redraw(&file).unwrap();
        let mut continuous = png::Decoder::new(fs::File::open(root.join("dense.png")).unwrap())
            .read_info()
            .unwrap();
        let mut data = vec![0; continuous.output_buffer_size()];
        continuous.next_frame(&mut data).unwrap();
        let pixels = &data;
        for row in 300..1800 {
            assert!(
                (750..1250).any(|col| {
                    let p = &pixels[(row * 3000 + col) * 4..][..4];
                    p[0] < 80 && p[1] < 80 && p[2] < 80
                }),
                "密集尖峰不应出现脱离主体的断裂，像素行 {row}"
            );
        }

        // 竖直费米线集中于零点列；旧红窗口位于另一列，应消失（图例除外）。
        let body = red
            .iter()
            .filter(|&&x| x < 2200)
            .copied()
            .collect::<Vec<_>>();
        assert!(body.len() > 1000);
        let saved: serde_json::Value = super::super::store::read(&file).unwrap();
        assert_eq!(saved["bounds"], serde_json::Value::Null);
        assert_eq!(saved["window"][0][2], serde_json::json!([0, 115, 60]));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn wannier_primary_replot_uses_actual_outer_window_without_changing_data_or_valence() {
        let root = std::env::temp_dir().join(format!("outer-display-{}", super::super::store::token()));
        fs::create_dir_all(&root).unwrap();
        super::super::store::write(&root.join("electronic-display.json"),
            &serde_json::json!({"fermi_ev":7., "near_ev":[-5.,5.], "valence_ev":[-20.,5.]})).unwrap();
        for (kind, name, curve) in [
            ("Bands", "electronic-bands", serde_json::json!([[[0.,1.],[-9.,13.]]])),
            ("Dos", "electronic-dos", serde_json::json!([[-10.,1.],[0.,2.],[14.,1.]])),
        ] {
            for valence in [false, true] {
                let file=root.join(format!("{name}{}.plot.json", if valence {"-valence"} else {""}));
                let before=serde_json::json!({"kind":kind,"qe":curve,"wan":curve,
                    "bounds":if valence {[-20.,5.]} else {[-5.,5.]},"labels":[],
                    "window":[[-8.25,"dis_win_min",[0,115,60]], [12.75,"dis_win_max",[125,85,35]]]});
                super::super::store::write(&file,&before).unwrap();
                redraw(&file).unwrap();
                let after:serde_json::Value=super::super::store::read(&file).unwrap();
                assert_eq!(after["bounds"], serde_json::json!(if valence {[-20.,5.]} else {[-8.25,12.75]}));
                assert_eq!(after["qe"],before["qe"]);
                assert_eq!(after["wan"],before["wan"]);
                // 第 3 步不读取 Wannier 窗，沿用独立的显示预设。
                let mut qe=before;qe["wan"]=serde_json::Value::Null;
                super::super::store::write(&file,&qe).unwrap();
                redraw(&file).unwrap();
                let saved:serde_json::Value=super::super::store::read(&file).unwrap();
                assert_eq!(saved["bounds"],qe["bounds"]);
            }
        }
        let metadata:serde_json::Value=super::super::store::read(&root.join("electronic-display.json")).unwrap();
        assert_eq!(metadata["default_ev"],serde_json::json!([-8.25,12.75]));
        assert_eq!(metadata["default_preset"],"wannier_outer");
        assert_eq!(metadata["fermi_ev"],7.,"已相对费米能的外窗不能二次减去 E_F");
        assert!(outer_bounds(&[]).is_err());
        assert!(outer_bounds(&[(2.,"dis_win_min",style::QE),(1.,"dis_win_max",style::QE)]).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn coincident_bands_and_dos_keep_qe_solid_and_all_colors_visible() {
        let root = std::env::temp_dir().join(format!(
            "electronic-overlap-{}",
            super::super::store::token()
        ));
        fs::create_dir_all(&root).unwrap();
        for case in ["Dos", "Bands", "Elements"] {
            let kind = if case == "Elements" { "Dos" } else { case };
            let file = root.join(format!("{case}.plot.json"));
            // 两侧逐点完全相同，避开费米线；不借助图例或不重合部分证明可见。
            let curve = if kind == "Dos" {
                serde_json::json!([[-5., 1.], [5., 1.]])
            } else {
                serde_json::json!([[[0., 1.], [1., 1.]]])
            };
            super::super::store::write(
                &file,
                &serde_json::json!({
                    "kind": kind, "qe": curve,
                    "wan": if case == "Elements" { serde_json::Value::Null } else { curve.clone() },
                    "elements": if case == "Elements" { serde_json::json!([["B", curve], ["Mg", curve]]) } else { serde_json::json!([]) },
                    "window": [], "note": "identical data", "labels": [], "bounds": [-5., 5.]
                }),
            )
            .unwrap();
            redraw(&file).unwrap();
            let mut reader = png::Decoder::new(
                fs::File::open(root.join(format!("{case}.png")))
                .unwrap(),
            )
            .read_info()
            .unwrap();
            let mut pixels = vec![0; reader.output_buffer_size()];
            reader.next_frame(&mut pixels).unwrap();
            let same_row_visible = (180..1800).any(|row| {
                let colors: &[[u8; 3]] = if case == "Elements" {
                    &[[40,40,40], [0,115,60], [220,135,0]]
                } else { &[[40,40,40], [0,70,210]] };
                colors.iter().all(|color| (300..2100).filter(|&col| {
                    &pixels[(row * 3000 + col) * 4..][..3] == color
                }).count() > 100) && (300..2100).all(|col| {
                    let rgb = &pixels[(row * 3000 + col) * 4..][..3];
                    rgb.iter().any(|&v| v < 100)
                })
            });
            assert!(
                same_row_visible,
                "{case}: 完全重合时必须保留所有颜色，且 QE 参考线不得出现虚线间隔"
            );
        }
        fs::remove_dir_all(root).unwrap();
    }
}
