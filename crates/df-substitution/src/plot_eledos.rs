//! 批量 DOS 结果周期表总览（复刻 `substitutionflow/plot_eledos.py`）。
//! 数据文件（`.dat`）与 Python 逐字段一致；PNG 用 plotters 重绘（布局等价、非像素级）。

use anyhow::{bail, Context, Result};
use regex::Regex;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

use crate::selection::{PERIOD_TABLE_LAYOUT, SERIES_TABLE_LAYOUT};
use crate::{pressure_key};

pub const XA_DIR_PATTERN: &str = r"^\d+\.XA_[A-Z][a-z]?$";
pub const ROW_DISPLAY_ORDER: [&str; 9] = [
    "period_1",
    "period_2",
    "period_3",
    "period_4",
    "period_5",
    "period_6",
    "period_7",
    "lanthanides",
    "actinides",
];
pub const BAR_LABELS: [&str; 4] = ["XA", "XB", "H", "TDOS"];

fn is_xa_dir(path: &str) -> bool {
    let re = Regex::new(XA_DIR_PATTERN).unwrap();
    let name = Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    Path::new(path).is_dir() && re.is_match(&name)
}

pub fn discover_xa_dirs(root_dir: &str) -> Result<Vec<String>> {
    let root = std::fs::canonicalize(root_dir)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| root_dir.to_string());
    let mut result = Vec::new();
    for entry in crate::list_sorted(&root)? {
        let path = format!("{root}/{entry}");
        if is_xa_dir(&path) {
            result.push(path);
        }
    }
    if result.is_empty() {
        bail!("在 {root} 下未找到任何 [Z].XA_[元素] 目录。");
    }
    Ok(result)
}

pub fn resolve_target_xa_dirs(directory_args: &[String]) -> Result<Vec<String>> {
    if directory_args.is_empty() {
        let cwd = std::env::current_dir()?.to_string_lossy().into_owned();
        return discover_xa_dirs(&cwd);
    }
    let abs_paths: Vec<String> = directory_args
        .iter()
        .map(|p| {
            std::fs::canonicalize(p)
                .map(|q| q.to_string_lossy().into_owned())
                .unwrap_or_else(|_| p.clone())
        })
        .collect();
    let xa_flags: Vec<bool> = abs_paths.iter().map(|p| is_xa_dir(p)).collect();
    if abs_paths.len() == 1 && !xa_flags[0] {
        return discover_xa_dirs(&abs_paths[0]);
    }
    if xa_flags.iter().all(|f| *f) {
        return Ok(abs_paths);
    }
    bail!("`-d/--directory` 只能传一个批量根目录，或一个/多个 [Z].XA_[元素] 目录，不能混传。")
}

pub struct CaseEntry {
    pub case_dir: String,
    pub formula: String,
    pub xa_symbol: String,
    pub xb_symbol: String,
    pub pressures: Vec<String>,
    pub positions: Vec<(String, usize)>,
}

fn matching_positions(symbol: &str) -> Result<Vec<(String, usize)>> {
    let mut matches = Vec::new();
    for (row_label, layout) in PERIOD_TABLE_LAYOUT {
        for (column, current) in layout.iter() {
            if *current == symbol {
                matches.push((row_label.to_string(), *column));
            }
        }
    }
    for (row_label, layout) in SERIES_TABLE_LAYOUT {
        for (column, current) in layout.iter() {
            if *current == symbol {
                matches.push((row_label.to_string(), *column));
            }
        }
    }
    if matches.is_empty() {
        bail!("无法在周期表布局中定位元素 {symbol}。");
    }
    Ok(matches)
}

pub fn discover_case_entries(xa_dir: &str) -> Result<Vec<CaseEntry>> {
    let xa_abs = std::fs::canonicalize(xa_dir)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| xa_dir.to_string());
    let mut entries = Vec::new();
    let mut manifest_paths: Vec<String> = Vec::new();
    for period in crate::list_sorted(&xa_abs)? {
        let candidate = format!("{xa_abs}/{period}/workflow_manifest.json");
        if Path::new(&candidate).is_file() {
            manifest_paths.push(candidate);
        }
    }
    manifest_paths.sort();
    for manifest_path in &manifest_paths {
        let manifest: Value = crate::load_config_file(manifest_path)?;
        let period_dir = Path::new(manifest_path).parent().unwrap();
        let Some(cases) = manifest.get("cases").and_then(|v| v.as_array()) else {
            continue;
        };
        for case_entry in cases {
            let case_dirname = case_entry.get("case_dir").and_then(|v| v.as_str()).unwrap_or_default();
            let case_dir = period_dir.join(case_dirname).to_string_lossy().into_owned();
            let xb_symbol = case_entry.get("xb_symbol").and_then(|v| v.as_str()).unwrap_or_default();
            let pressures: Vec<String> = manifest
                .get("pressures")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().map(|p| pressure_key(p)).collect())
                .unwrap_or_default();
            entries.push(CaseEntry {
                case_dir,
                formula: case_entry.get("formula").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                xa_symbol: manifest.get("xa_symbol").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                xb_symbol: xb_symbol.to_string(),
                pressures,
                positions: matching_positions(xb_symbol)?,
            });
        }
    }
    if entries.is_empty() {
        bail!("在 {xa_dir} 下未找到任何 workflow_manifest.json。");
    }
    Ok(entries)
}

pub fn discover_pressures(case_entries: &[CaseEntry]) -> Vec<String> {
    let mut pressures: Vec<String> = Vec::new();
    for entry in case_entries {
        for pressure in &entry.pressures {
            if !pressures.contains(pressure) {
                pressures.push(pressure.clone());
            }
        }
    }
    pressures
}

fn load_csv_row(path: &str) -> Result<BTreeMap<String, f64>> {
    let text = std::fs::read_to_string(path)?;
    let mut lines = text.lines();
    let header: Vec<String> = lines
        .next()
        .context("CSV 为空")?
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    let data_line = lines.next().context(format!("{path} 不包含数据行。"))?;
    let mut row = BTreeMap::new();
    for (key, value) in header.iter().zip(data_line.split(',')) {
        if key.is_empty() {
            continue;
        }
        row.insert(key.clone(), value.trim().parse().context("CSV 数值解析失败")?);
    }
    Ok(row)
}

fn read_case_step_status(case_dir: &str, pressure: &str) -> Option<String> {
    let path = format!("{case_dir}/case_status.json");
    let payload: Value = crate::load_config_file(&path).ok()?;
    payload
        .get("pressures")?
        .get(pressure)?
        .get("vasp_process_eledos")?
        .as_str()
        .map(|s| s.to_string())
}

fn find_at_ef_csv(case_dir: &str, pressure: &str) -> Result<Option<String>> {
    let dir = format!("{case_dir}/{pressure}/vasp_eledos");
    let mut candidates: Vec<String> = Vec::new();
    if let Ok(names) = crate::list_sorted(&dir) {
        for name in names {
            if name.ends_with("_at_ef.csv") {
                let path = format!("{dir}/{name}");
                if crate::is_nonempty_file(&path) {
                    candidates.push(path);
                }
            }
        }
    }
    match candidates.len() {
        0 => Ok(None),
        1 => Ok(Some(candidates.remove(0))),
        _ => bail!("{dir} 下找到多个 *_at_ef.csv，无法确定使用哪个。"),
    }
}

pub struct PlotRecord {
    pub status: String,
    /// XA/XB/H/TDOS
    pub values: Option<Vec<f64>>,
}

pub fn load_case_plot_record(entry: &CaseEntry, pressure: &str) -> PlotRecord {
    let _ = read_case_step_status(&entry.case_dir, pressure);
    let csv_path = match find_at_ef_csv(&entry.case_dir, pressure) {
        Ok(Some(p)) => p,
        _ => {
            return PlotRecord {
                status: "error".to_string(),
                values: None,
            }
        }
    };
    let Ok(row) = load_csv_row(&csv_path) else {
        return PlotRecord {
            status: "error".to_string(),
            values: None,
        };
    };
    let required = [&entry.xa_symbol, &entry.xb_symbol, &"H".to_string(), &"TDOS".to_string()];
    if required.iter().any(|name| !row.contains_key(*name)) {
        return PlotRecord {
            status: "error".to_string(),
            values: None,
        };
    }
    PlotRecord {
        status: "finished".to_string(),
        values: Some(vec![
            row[&entry.xa_symbol],
            row[&entry.xb_symbol],
            row["H"],
            row["TDOS"],
        ]),
    }
}

pub struct PressurePlotData {
    pub xa_dir: String,
    pub pressure: String,
    pub rows: Vec<String>,
    pub cells: BTreeMap<(String, usize), CellPayload>,
    pub summary_rows: Vec<(String, String, Vec<f64>)>,
    pub ymax: f64,
}

pub struct CellPayload {
    pub xb_symbol: String,
    pub formula: String,
    pub status: String,
    pub values: Option<Vec<f64>>,
}

pub fn collect_pressure_plot_data(xa_dir: &str, pressure: &str) -> Result<PressurePlotData> {
    let entries = discover_case_entries(xa_dir)?;
    let mut row_labels: Vec<String> = Vec::new();
    let mut cells: BTreeMap<(String, usize), CellPayload> = BTreeMap::new();
    let mut summary_rows: Vec<(String, String, Vec<f64>)> = Vec::new();
    let empty: Vec<f64> = vec![f64::NAN; 4];

    for entry in &entries {
        let record = load_case_plot_record(entry, pressure);
        summary_rows.push((
            entry.formula.clone(),
            record.status.clone(),
            record.values.clone().unwrap_or_else(|| empty.clone()),
        ));
        for (row_label, column) in &entry.positions {
            if !row_labels.contains(row_label) {
                row_labels.push(row_label.clone());
            }
            cells.insert(
                (row_label.clone(), *column),
                CellPayload {
                    xb_symbol: entry.xb_symbol.clone(),
                    formula: entry.formula.clone(),
                    status: record.status.clone(),
                    values: record.values.clone(),
                },
            );
        }
    }
    let rows: Vec<String> = ROW_DISPLAY_ORDER
        .iter()
        .filter(|r| row_labels.contains(&r.to_string()))
        .map(|r| r.to_string())
        .collect();
    let mut ymax = 1.0_f64;
    for payload in cells.values() {
        if let Some(values) = &payload.values {
            ymax = ymax.max(values.iter().cloned().fold(f64::NEG_INFINITY, f64::max) * 1.1);
        }
    }
    Ok(PressurePlotData {
        xa_dir: std::fs::canonicalize(xa_dir)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| xa_dir.to_string()),
        pressure: pressure.to_string(),
        rows,
        cells,
        summary_rows,
        ymax,
    })
}

pub fn write_pressure_summary_dat(plot_data: &PressurePlotData) -> Result<String> {
    let output_path = format!("{}/eledos_summary_{}.dat", plot_data.xa_dir, plot_data.pressure);
    let headers = [
        "体系化学配比名称",
        "元素XA的费米能级处DOS",
        "元素XB的费米能级处DOS",
        "H的费米能级处DOS",
        "TDOS",
        "status",
    ];
    let mut text = String::new();
    text.push_str(headers.join(" ").as_str());
    text.push('\n');
    for (formula, status, values) in &plot_data.summary_rows {
        text.push_str(&format!(
            "{} {} {} {} {} {}\n",
            formula,
            crate::plot_eledos::fmt_py_float(values[0]),
            crate::plot_eledos::fmt_py_float(values[1]),
            crate::plot_eledos::fmt_py_float(values[2]),
            crate::plot_eledos::fmt_py_float(values[3]),
            status
        ));
    }
    std::fs::write(&output_path, text)?;
    Ok(output_path)
}

/// Python `str(float)` 风格：整数带 `.0`，NaN 输出 `nan`。
pub fn fmt_py_float(v: f64) -> String {
    if v.is_nan() {
        return "nan".to_string();
    }
    if v.is_infinite() {
        return if v > 0.0 { "inf".into() } else { "-inf".into() };
    }
    if v.fract() == 0.0 && v.abs() < 1e16 {
        return format!("{v:.1}");
    }
    format!("{v}")
}

/// 周期表网格总览 PNG（plotters 重绘，非像素级等价）。
pub fn plot_pressure_summary(plot_data: &PressurePlotData) -> Result<String> {
    use plotters::prelude::*;
    let rows = &plot_data.rows;
    if rows.is_empty() {
        bail!(
            "{} 在 pressure={} 下没有可绘制行。",
            plot_data.xa_dir,
            plot_data.pressure
        );
    }
    let width = 2400u32;
    let height = (300 * rows.len() as u32).max(500);
    let output_path = format!("{}/eledos_summary_{}.png", plot_data.xa_dir, plot_data.pressure);
    let root = BitMapBackend::new(&output_path, (width, height)).into_drawing_area();
    root.fill(&WHITE).map_err(|e| anyhow::anyhow!("{e}"))?;

    let legend_area = root.split_vertically(50);
    // 图例
    let legend_colors = [RGBColor(0x4C, 0x78, 0xA8), RGBColor(0xF5, 0x85, 0x18), RGBColor(0x54, 0xA2, 0x4B), RGBColor(0xE4, 0x57, 0x56)];
    let mut legend_labels: Vec<(String, RGBColor)> = Vec::new();
    for (label, color) in BAR_LABELS.iter().zip(&legend_colors) {
        legend_labels.push((label.to_string(), *color));
    }
    let cells_area = legend_area.1;
    let grid = cells_area.split_evenly((rows.len(), 18));
    let mut grid_iter = grid.into_iter();
    for (row_index, row_label) in rows.iter().enumerate() {
        for column_index in 0..18usize {
            let cell_area = grid_iter.next().context("网格不足")?;
            let payload = plot_data.cells.get(&(row_label.clone(), column_index + 1));
            let Some(payload) = payload else {
                let _ = cell_area.fill(&WHITE);
                continue;
            };
            let _ = cell_area.fill(&WHITE);
            let _ = cell_area.draw_text(
                &payload.xb_symbol,
                &TextStyle::from(("sans-serif", 12).into_font()),
                (10, 4),
            );
            match &payload.values {
                None => {
                    let _ = cell_area.fill(&RGBColor(0xEC, 0xEC, 0xEC));
                    let _ = cell_area.draw_text(
                        "unfinished",
                        &TextStyle::from(("sans-serif", 11).into_font()),
                        (40, 40),
                    );
                }
                Some(values) => {
                    // 四柱状图：在 cell 内再分 1x1 绘制。
                    let mut chart = ChartBuilder::on(&cell_area)
                        .x_label_area_size(0)
                        .y_label_area_size(0)
                        .margin(6)
                        .build_cartesian_2d(-0.6f64..3.6f64, 0.0f64..plot_data.ymax)
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                    chart.plotting_area().fill(&WHITE).ok();
                    for (i, v) in values.iter().enumerate() {
                        let x0 = i as f64 - 0.36;
                        let x1 = i as f64 + 0.36;
                        chart
                            .draw_series(std::iter::once(Rectangle::new(
                                [(x0, 0.0), (x1, *v)],
                                legend_colors[i].filled(),
                            )))
                            .ok();
                    }
                }
            }
        }
        let _ = row_index;
    }

    // 图例区文字
    let mut x = 20i32;
    for (label, color) in &legend_labels {
        let _ = legend_area.0.draw(&Rectangle::new([(x, 8), (x + 14, 22)], color.filled()));
        let _ = legend_area.0.draw_text(
            label,
            &TextStyle::from(("sans-serif", 14).into_font()),
            (x + 20, 8),
        );
        x += 100;
    }
    root.present().map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(output_path.clone())
}

/// `substitution_plot_eledos` 主流程。返回输出文件列表。
pub fn run(directory_args: &[String]) -> Result<Vec<String>> {
    let mut outputs = Vec::new();
    let xa_dirs = resolve_target_xa_dirs(directory_args)?;
    for xa_dir in &xa_dirs {
        let entries = discover_case_entries(xa_dir)?;
        let pressures = discover_pressures(&entries);
        for pressure in &pressures {
            let plot_data = collect_pressure_plot_data(xa_dir, pressure)?;
            outputs.push(plot_pressure_summary(&plot_data)?);
            outputs.push(write_pressure_summary_dat(&plot_data)?);
        }
    }
    Ok(outputs)
}
