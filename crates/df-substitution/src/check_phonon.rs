//! 声子稳定性批量检查（复刻 `substitutionflow/check_phonon_stability.py`）。
//! 判定：`phonon_dos_elements.csv` 的 `frequency_cm-1` 最小值 /33.35641 → THz，
//! `< -1.0` 为 unstable；读取失败为 unfinished。

use anyhow::Result;
use std::path::Path;

use crate::plot_eledos::{
    discover_case_entries, discover_pressures, resolve_target_xa_dirs, CaseEntry,
};
use crate::status::get_case_phonon_workdir;

pub const CM_INV_PER_THZ: f64 = 33.35641;

pub struct StabilityRecord {
    pub status: String,
    pub min_frequency_thz: Option<f64>,
}

fn read_min_frequency_cm(path: &str) -> Result<f64> {
    let text = std::fs::read_to_string(path)?;
    let mut lines = text.lines();
    let header: Vec<String> = lines
        .next()
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    let Some(col) = header.iter().position(|h| h == "frequency_cm-1") else {
        anyhow::bail!("{path} 缺少 frequency_cm-1 列。");
    };
    let mut values: Vec<f64> = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() <= col {
            continue;
        }
        let value = fields[col].trim();
        if value.is_empty() {
            continue;
        }
        if let Ok(v) = value.parse::<f64>() {
            values.push(v);
        }
    }
    if values.is_empty() {
        anyhow::bail!("{path} 不包含有效 frequency_cm-1 数据。");
    }
    Ok(values.iter().cloned().fold(f64::INFINITY, f64::min))
}

fn phonon_dos_elements_path(case_dir: &str, pressure: &str) -> String {
    let workdir = get_case_phonon_workdir(case_dir).unwrap_or_default();
    format!("{case_dir}/{pressure}/{workdir}/phdos/phonon_dos_elements.csv")
}

pub fn load_case_stability_record(entry: &CaseEntry, pressure: &str) -> StabilityRecord {
    let csv_path = phonon_dos_elements_path(&entry.case_dir, pressure);
    let min_frequency_cm = match read_min_frequency_cm(&csv_path) {
        Ok(v) => v,
        Err(_) => {
            return StabilityRecord {
                status: "unfinished".to_string(),
                min_frequency_thz: None,
            }
        }
    };
    let min_frequency_thz = min_frequency_cm / CM_INV_PER_THZ;
    StabilityRecord {
        status: if min_frequency_thz < -1.0 { "unstable".into() } else { "stable".into() },
        min_frequency_thz: Some(min_frequency_thz),
    }
}

pub struct PressureStabilityData {
    pub xa_dir: String,
    pub pressure: String,
    pub rows: Vec<String>,
    pub cells: std::collections::BTreeMap<(String, usize), StabilityCell>,
}

pub struct StabilityCell {
    pub xb_symbol: String,
    pub status: String,
    pub min_frequency_thz: Option<f64>,
}

pub fn collect_pressure_stability_data(xa_dir: &str, pressure: &str) -> Result<PressureStabilityData> {
    let entries = discover_case_entries(xa_dir)?;
    let mut row_labels: Vec<String> = Vec::new();
    let mut cells = std::collections::BTreeMap::new();
    for entry in &entries {
        let record = load_case_stability_record(entry, pressure);
        for (row_label, column) in &entry.positions {
            if !row_labels.contains(row_label) {
                row_labels.push(row_label.clone());
            }
            cells.insert(
                (row_label.clone(), *column),
                StabilityCell {
                    xb_symbol: entry.xb_symbol.clone(),
                    status: record.status.clone(),
                    min_frequency_thz: record.min_frequency_thz,
                },
            );
        }
    }
    let rows: Vec<String> = crate::plot_eledos::ROW_DISPLAY_ORDER
        .iter()
        .filter(|r| row_labels.contains(&r.to_string()))
        .map(|r| r.to_string())
        .collect();
    Ok(PressureStabilityData {
        xa_dir: std::fs::canonicalize(xa_dir)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| xa_dir.to_string()),
        pressure: pressure.to_string(),
        rows,
        cells,
    })
}

fn format_min_frequency(value: Option<f64>) -> String {
    match value {
        None => "min = n/a".to_string(),
        Some(v) => format!("min = {v:.2} THz"),
    }
}

/// 周期表网格稳定性 PNG（plotters 重绘，非像素级等价）。
pub fn plot_pressure_stability(plot_data: &PressureStabilityData) -> Result<String> {
    use plotters::prelude::*;
    let rows = &plot_data.rows;
    if rows.is_empty() {
        anyhow::bail!(
            "{} 在 pressure={} 下没有可绘制行。",
            plot_data.xa_dir,
            plot_data.pressure
        );
    }
    let width = 2400u32;
    let height = (300 * rows.len() as u32).max(500);
    let output_path = format!(
        "{}/phonon_stability_summary_{}.png",
        plot_data.xa_dir, plot_data.pressure
    );
    let root = BitMapBackend::new(&output_path, (width, height)).into_drawing_area();
    root.fill(&WHITE).map_err(|e| anyhow::anyhow!("{e}"))?;
    let (legend_area, cells_area) = root.split_vertically(50);
    let grid = cells_area.split_evenly((rows.len(), 18));
    let mut grid_iter = grid.into_iter();
    let color_of = |status: &str| match status {
        "stable" => RGBColor(0x4C, 0x9F, 0x70),
        "unstable" => RGBColor(0xD9, 0x5F, 0x59),
        _ => RGBColor(0xEC, 0xEC, 0xEC),
    };
    let text_color_of = |status: &str| match status {
        "unfinished" => RGBColor(0x55, 0x55, 0x55),
        _ => WHITE,
    };
    for row_label in rows.iter() {
        for column_index in 0..18usize {
            let cell_area = grid_iter.next().unwrap();
            let payload = plot_data.cells.get(&(row_label.clone(), column_index + 1));
            let Some(payload) = payload else {
                let _ = cell_area.fill(&WHITE);
                continue;
            };
            let _ = cell_area.fill(&color_of(&payload.status));
            let tc = text_color_of(&payload.status);
            let _ = cell_area.draw_text(
                &payload.xb_symbol,
                &TextStyle::from(("sans-serif", 13).into_font()).color(&tc),
                (10, 8),
            );
            let _ = cell_area.draw_text(
                &payload.status,
                &TextStyle::from(("sans-serif", 10).into_font()).color(&tc),
                (10, 30),
            );
            let _ = cell_area.draw_text(
                &format_min_frequency(payload.min_frequency_thz),
                &TextStyle::from(("sans-serif", 9).into_font()).color(&tc),
                (10, 50),
            );
        }
    }
    let legend = [("stable", RGBColor(0x4C, 0x9F, 0x70)), ("unstable", RGBColor(0xD9, 0x5F, 0x59)), ("unfinished", RGBColor(0xEC, 0xEC, 0xEC))];
    let mut x = 20i32;
    for (label, color) in &legend {
        let _ = legend_area.fill(&WHITE);
        let _ = legend_area.draw(&Rectangle::new([(x, 8), (x + 14, 22)], color.filled()));
        let _ = legend_area.draw_text(
            label,
            &TextStyle::from(("sans-serif", 14).into_font()),
            (x + 20, 8),
        );
        x += 110;
    }
    root.present().map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(output_path.clone())
}

/// `substitution_check_phonon_stability` 主流程。
pub fn run(directory_args: &[String], _jobs: Option<usize>) -> Result<Vec<String>> {
    let mut outputs = Vec::new();
    let xa_dirs = resolve_target_xa_dirs(directory_args)?;
    for xa_dir in &xa_dirs {
        let entries = discover_case_entries(xa_dir)?;
        let pressures = discover_pressures(&entries);
        for pressure in &pressures {
            let plot_data = collect_pressure_stability_data(xa_dir, pressure)?;
            outputs.push(plot_pressure_stability(&plot_data)?);
        }
    }
    Ok(outputs)
}

/// 供测试：检查单个 case×pressure 的状态。
pub fn check_one(case_dir: &str, pressure: &str) -> StabilityRecord {
    let entry = CaseEntry {
        case_dir: case_dir.to_string(),
        formula: String::new(),
        xa_symbol: String::new(),
        xb_symbol: String::new(),
        pressures: Vec::new(),
        positions: Vec::new(),
    };
    load_case_stability_record(&entry, pressure)
}

/// 供外部调用：判定 path 是否存在（测试辅助）。
pub fn path_exists(path: &str) -> bool {
    Path::new(path).exists()
}
