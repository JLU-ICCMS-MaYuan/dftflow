//! 只读取拟合现场出诊断图；不要求收敛，不创建科学完成回执或执行外部程序。
use super::{config::Config, electronic_style as style, stages::Step, store, workspace_plot};
use crate::{
    interpolation::electronic_sampling::{self, Sampling},
    qe_data::nscf,
};
use anyhow::{ensure, Context, Result};
use plotters::prelude::*;
use regex::Regex;
use serde_json::json;
use std::{fs, path::PathBuf};

fn windows(text: &str, fermi: f64) -> Result<Vec<(f64, &'static str, RGBColor)>> {
    let mut result = vec![];
    for (index, name) in ["dis_win_min", "dis_win_max", "dis_froz_min", "dis_froz_max"]
        .into_iter()
        .enumerate()
    {
        let pattern = Regex::new(&format!(r"(?mi)^\s*{name}\s*=\s*([^!\s]+)"))?;
        if let Some(value) = pattern.captures(text) {
            let value: f64 = value[1].replace(['D', 'd'], "e").parse()?;
            ensure!(value.is_finite(), "data_invalid: The diagnostic window is not finite");
            result.push((value - fermi, name, style::WINDOWS[index]));
        }
    }
    Ok(result)
}

fn spread_history(text: &str) -> Result<Vec<(f64, f64)>> {
    let cycle = Regex::new(r"^\s*Cycle:\s*(\d+)")?;
    let spread = Regex::new(r"Sum of centres and spreads\s*\([^)]*\)\s*(\S+)")?;
    let mut iteration = 0.;
    let mut rows = vec![];
    for line in text.lines() {
        if let Some(value) = cycle.captures(line) {
            iteration = value[1].parse()?;
        }
        if let Some(value) = spread.captures(line) {
            let value: f64 = value[1].replace(['D', 'd'], "e").parse()?;
            ensure!(
                value.is_finite() && value >= 0.,
                "data_invalid: Diagnosis spread Unlimited or negative"
            );
            if rows.last().is_none_or(|(i, _)| *i != iteration) {
                rows.push((iteration, value));
            }
        }
    }
    Ok(rows)
}

/// 使用现场写出的原始数据；修改源配置不会将旧模型标成新参数的结果。
pub fn generate(cfg: &Config) -> Result<PathBuf> {
    let source = Step::Wannier.directory(cfg);
    ensure!(
        source.is_dir(),
        "required: Wannier It doesn't exist {}",
        source.display()
    );
    let destination = source.join("diagnostics");
    fs::create_dir_all(&destination)?;
    for name in [
        "wannier-fit.png",
        "wannier-fit.svg",
        "wannier-fit.plot.json",
        "wannier-fit.csv",
        "wannier-fit-near-fermi.png",
        "wannier-fit-near-fermi.svg",
        "wannier-fit-near-fermi.plot.json",
        "wannier-spread.png",
        "wannier-spread.svg",
        "wannier-spread.csv",
    ] {
        let previous = destination.join(name);
        if previous.is_file() {
            fs::remove_file(previous)?;
        }
    }
    let prefix = &cfg.qe.prefix;
    let mut identities = vec![];
    for suffix in [
        ".win",
        ".wout",
        "_band.dat",
        "_band.kpt",
        "_band.labelinfo.dat",
    ] {
        let name = format!("{prefix}{suffix}");
        if source.join(&name).is_file() {
            // 活动日志可继续增长；只绑定本次读取的快照，不要求原文件停止写入。
            let original = source.join(&name);
            let bytes = fs::read(&original)?;
            store::atomic_bytes(&destination.join(&name), &bytes)?;
            identities.push(json!({"source_path":original,
                "snapshot":store::Identity::capture(&destination.join(&name))?}));
        } else if destination.join(&name).is_file() {
            fs::remove_file(destination.join(&name))?;
        }
    }
    let log = fs::read_to_string(destination.join(format!("{prefix}.wout"))).unwrap_or_default();
    let input = fs::read_to_string(destination.join(format!("{prefix}.win"))).unwrap_or_default();
    let converged = crate::interpolation::wannier::check_native(
        &destination.join(format!("{prefix}.wout")),
        cfg.wannier.num_wann as usize,
    )
    .is_ok();
    let state = if converged {
        "CONVERGED MODEL: diagnostic only"
    } else {
        "UNCONVERGED / INCOMPLETE MODEL: diagnostic only"
    };
    let mut report = json!({"schema_version":1,"diagnostic_only":true,"converged":converged,
        "scientific_completion_published":false,"model_input":input,"sources":identities,"plots":[],"errors":[]});
    let history = spread_history(&log)?;
    if !history.is_empty() {
        let mut svg = String::new();
        {
            let area = SVGBackend::with_string(&mut svg, (1000, 700)).into_drawing_area();
            area.fill(&WHITE)?;
            let lo = history.iter().map(|r| r.1).fold(f64::INFINITY, f64::min);
            let hi = history.iter().map(|r| r.1).fold(0., f64::max);
            let mut chart = ChartBuilder::on(&area)
                .caption(state, ("sans-serif", 20))
                .margin(25)
                .x_label_area_size(45)
                .y_label_area_size(75)
                .build_cartesian_2d(
                    0.0..history.last().unwrap().0.max(1.),
                    (lo * 0.95)..(hi * 1.05).max(lo + 1.),
                )?;
            chart
                .configure_mesh()
                .x_desc("Wannier iteration")
                .y_desc("Total spread (Angstrom^2)")
                .draw()?;
            chart.draw_series(LineSeries::new(
                history.iter().copied(),
                BLUE.stroke_width(2),
            ))?;
            area.present()?;
        }
        fs::write(destination.join("wannier-spread.svg"), &svg)?;
        style::png(&svg, &destination.join("wannier-spread.png"), 3000, 2100)?;
        let mut csv = String::from("iteration,total_spread_angstrom2\n");
        for (i, value) in &history {
            csv.push_str(&format!("{i},{value:.12}\n"));
        }
        fs::write(destination.join("wannier-spread.csv"), csv)?;
        report["plots"]
            .as_array_mut()
            .unwrap()
            .push(json!("wannier-spread.png"));
    }
    let bands = (|| -> Result<()> {
        let decision: serde_json::Value =
            store::read(&source.join("embedded-sampling-decision.json"))?;
        let sampling: Sampling = serde_json::from_value(decision["sampling"].clone())?;
        let reference = cfg.work_dir.join(format!(
            "3.qe-electronic/qe_eband/out/{prefix}.save/data-file-schema.xml"
        ));
        let qe = nscf::parse_xml(&reference)?;
        let mesh = nscf::parse_xml(&electronic_sampling::xml(cfg))?;
        let fermi = mesh.fermi;
        ensure!(
            qe.eigenvalues.len() == sampling.path.len(),
            "data_invalid: The number of diagnostic reference path points differs from the original sample"
        );
        for (actual, point) in qe.kpoints.iter().zip(&sampling.path) {
            let expected = electronic_sampling::cart(point.crystal, &sampling.reciprocal);
            ensure!(
                actual
                    .iter()
                    .zip(expected)
                    .all(|(a, b)| (a - b).abs() <= 2.0e-6),
                "data_invalid: The diagnostic reference path coordinates differ from the original sample"
            );
        }
        let kpt = fs::read_to_string(destination.join(format!("{prefix}_band.kpt")))?;
        let coords = kpt
            .lines()
            .skip(1)
            .filter(|l| !l.trim().is_empty())
            .map(|line| {
                let v = crate::qe_data::xml::numbers(line)?;
                ensure!(v.len() == 4, "data_invalid: Native kpt Line invalid");
                Ok([v[0], v[1], v[2]])
            })
            .collect::<Result<Vec<_>>>()?;
        ensure!(!coords.is_empty(), "required: It's not written yet");
        let raw = fs::read_to_string(destination.join(format!("{prefix}_band.dat")))?;
        let values = raw
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|line| {
                let v = crate::qe_data::xml::numbers(line)?;
                ensure!(
                    v.len() == 2 && v.iter().all(|v| v.is_finite()),
                    "data_invalid: Numerical energy is invalid"
                );
                Ok(v[1])
            })
            .collect::<Result<Vec<_>>>()?;
        ensure!(
            values.len() % coords.len() == 0,
            "data_invalid: It's not finished yet"
        );
        let count = values.len() / coords.len();
        let model_count = Regex::new(r"(?mi)^\s*num_wann\s*=\s*(\d+)")?
            .captures(&input)
            .context("data_invalid: Actual win Missing num_wann")?[1]
            .parse::<usize>()?;
        ensure!(count == model_count, "data_invalid: The number of diagnostic models does not match");
        let native_path = if decision["path_format"] == "high_symmetry_endpoints" {
            // 从实际模型输入恢复端点，避免源配置后来修改 KPATH 后错配旧模型。
            let mut kpath = String::new();
            let mut inside = false;
            for line in input.lines().map(str::trim) {
                if line.eq_ignore_ascii_case("begin kpoint_path") {
                    inside = true;
                    continue;
                }
                if line.eq_ignore_ascii_case("end kpoint_path") {
                    break;
                }
                if !inside || line.is_empty() {
                    continue;
                }
                let cells = line.split_whitespace().collect::<Vec<_>>();
                ensure!(cells.len() == 8, "data_invalid: Actual win Path peer format invalid");
                kpath.push_str(&format!(
                    "{} {} {} {}\n{} {} {} {}\n",
                    cells[1], cells[2], cells[3], cells[0], cells[5], cells[6], cells[7], cells[4]
                ));
            }
            let path = destination.join("model-kpath.in");
            store::atomic_bytes(&path, kpath.as_bytes())?;
            let edges = electronic_sampling::high_symmetry_edges(&path)?;
            let first_end = sampling
                .ticks
                .get(1)
                .context("data_invalid: Missing original reference path endpoint")?
                .distance;
            let gap_intervals = sampling
                .path
                .iter()
                .take_while(|p| p.distance <= first_end + 1e-9)
                .count()
                .saturating_sub(1)
                .try_into()?;
            let labels =
                fs::read_to_string(destination.join(format!("{prefix}_band.labelinfo.dat")))?;
            report["independent_native_sampling"] = json!(true);
            Some(
                electronic_sampling::native_path_points(
                    &edges,
                    &sampling.reciprocal,
                    &labels,
                    &coords,
                    gap_intervals,
                )?
                .0,
            )
        } else {
            None
        };
        let fit_path = native_path.as_ref().unwrap_or(&sampling.path);
        let indices = fit_path
            .iter()
            .map(|point| {
                coords
                    .iter()
                    .position(|p| {
                        p.iter()
                            .zip(point.crystal)
                            .all(|(a, b)| (a - b).abs() <= 5.1e-7)
                    })
                    .context("required: Native output does not yet cover the full diagnostic path")
            })
            .collect::<Result<Vec<_>>>()?;
        let mut qe_curves = vec![];
        let mut fit_curves = vec![];
        let mut csv = String::from("source,segment,point,band,distance,energy_minus_fermi_ev\n");
        for (method, path, nbands) in [
            ("QE", &sampling.path, qe.eigenvalues[0].len()),
            ("Wannier", fit_path, count),
        ] {
            for segment in path
                .iter()
                .map(|p| p.segment_id)
                .collect::<std::collections::BTreeSet<_>>()
            {
                let points = path
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.segment_id == segment)
                    .collect::<Vec<_>>();
                for band in 0..nbands {
                    let mut x = vec![];
                    let mut y = vec![];
                    for (i, p) in &points {
                        let energy = if method == "QE" {
                            qe.eigenvalues[*i][band]
                        } else {
                            values[band * coords.len() + indices[*i]]
                        } - fermi;
                        x.push(p.distance);
                        y.push(energy);
                        csv.push_str(&format!(
                            "{method},{segment},{i},{band},{:.12},{energy:.12}\n",
                            p.distance
                        ));
                    }
                    if method == "QE" {
                        qe_curves.push((x, y));
                    } else {
                        fit_curves.push((x, y));
                    }
                }
            }
        }
        fs::write(destination.join("wannier-fit.csv"), csv)?;
        let window = windows(&input, fermi)?;
        let minimum = qe
            .eigenvalues
            .iter()
            .flatten()
            .copied()
            .fold(f64::INFINITY, f64::min)
            - fermi;
        let maximum = qe
            .eigenvalues
            .iter()
            .flatten()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
            - fermi;
        let boundary = |name| window.iter().find(|r| r.1 == name).map(|r| r.0);
        let bounds = [
            boundary("dis_win_min").unwrap_or(minimum),
            boundary("dis_win_max").unwrap_or(maximum),
        ];
        let labels = sampling
            .ticks
            .iter()
            .map(|t| (t.distance, t.label.replace('G', "Γ")))
            .collect::<Vec<_>>();
        let note = format!("{state}; window markers from actual .win; E_F={fermi:.6} eV");
        workspace_plot::bands_figure(
            &destination.join("wannier-fit.png"),
            &qe_curves,
            Some(&fit_curves),
            bounds,
            &window,
            &labels,
            &note,
        )?;
        workspace_plot::bands_figure(
            &destination.join("wannier-fit-near-fermi.png"),
            &qe_curves,
            Some(&fit_curves),
            [-5., 5.],
            &window,
            &labels,
            &note,
        )?;
        report["plots"].as_array_mut().unwrap().extend([
            json!("wannier-fit.png"),
            json!("wannier-fit-near-fermi.png"),
        ]);
        report["fermi_ev"] = json!(fermi);
        report["window_markers_source"] = json!("actual_model_win_input");
        report["reference_xml"] = serde_json::to_value(store::Identity::capture(&reference)?)?;
        Ok(())
    })();
    if let Err(error) = bands {
        report["errors"]
            .as_array_mut()
            .unwrap()
            .push(json!(format!("{error:#}")));
    }
    store::write(&destination.join("diagnostic.json"), &report)?;
    ensure!(
        !report["plots"].as_array().unwrap().is_empty(),
        "required: There is no comparable data available at the site {}",
        destination.join("diagnostic.json").display()
    );
    Ok(destination)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incomplete_log_keeps_spread_history_and_input_windows() {
        let rows=spread_history("Cycle: 1\nSum of centres and spreads (0,0,0) 12.5\nCycle: 2\nSum of centres and spreads (0,0,0) 11.2\n").unwrap();
        assert_eq!(rows, vec![(1., 12.5), (2., 11.2)]);
        let values = windows(
            "dis_win_min = -2\ndis_win_max = 8\ndis_froz_min = 0\ndis_froz_max = 4\n",
            1.,
        )
        .unwrap();
        assert_eq!(
            values.iter().map(|v| v.0).collect::<Vec<_>>(),
            [-3., 7., -1., 3.]
        );
    }
}
