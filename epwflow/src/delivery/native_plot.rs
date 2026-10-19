//! 原生结果的六类图表；只读取包内数据，不访问求解器和调度器。
use super::{
    comparison::Artifact,
    native::{self, Data},
};
use crate::foundation::store;
use anyhow::{ensure, Result};
use plotters::{coord::Shift, prelude::*};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
pub const KINDS: [&str; 6] = [
    "a2f_coupling",
    "gap_temperature",
    "gap_aniso_distribution",
    "gap_aniso_k",
    "tc_interval",
    "summary",
];

pub const GAP_BIN_MEV: f64 = 0.05;

/// 同一初值、已通过双初值分类的最低正 Matsubara 分布。每个温度的箱质量和为 1。
/// 不连接温度，不平滑能隙，不将带编号命名为未经投影验证的 sigma/pi。
pub fn gap_distributions(data: &Data) -> Result<BTreeMap<i64, Vec<(f64, f64)>>> {
    use crate::superconductivity::search::Side;
    let Some(mode) = data.modes.get("aniso") else {
        return Ok(BTreeMap::new());
    };
    let eligible = |t: i64| {
        matches!(
            mode.search.points.get(&t),
            Some(Side::Superconducting | Side::Normal)
        ) && !mode.failed_temperatures_centikelvin.contains(&t)
    };
    let seed = data
        .states
        .iter()
        .filter(|s| s.mode == "aniso" && eligible(s.temperature_centikelvin))
        .map(|s| s.seed_mev)
        .min_by(f64::total_cmp);
    let mut masses: BTreeMap<i64, BTreeMap<u32, f64>> = BTreeMap::new();
    let mut identities = BTreeSet::new();
    for s in data.states.iter().filter(|s| {
        s.mode == "aniso" && Some(s.seed_mev) == seed && eligible(s.temperature_centikelvin)
    }) {
        ensure!(
            s.weight.is_finite()
                && s.weight >= 0.0
                && s.delta_mev.is_finite()
                && s.delta_mev.abs() / GAP_BIN_MEV < u32::MAX as f64,
            "data_invalid: 分布权重或能隙无效"
        );
        ensure!(
            identities.insert((s.temperature_centikelvin, s.kpoint, s.band)),
            "data_invalid: 分布含重复物理态"
        );
        let bin = (s.delta_mev.abs() / GAP_BIN_MEV).floor() as u32;
        *masses
            .entry(s.temperature_centikelvin)
            .or_default()
            .entry(bin)
            .or_default() += s.weight;
    }
    masses
        .into_iter()
        .map(|(t, bins)| {
            let total: f64 = bins.values().sum();
            ensure!(
                total.is_finite() && total > 0.0,
                "data_invalid: 分布总权重无效"
            );
            Ok((
                t,
                bins.into_iter()
                    .filter(|(_, w)| *w > 0.0)
                    .map(|(b, w)| ((b as f64 + 0.5) * GAP_BIN_MEV, w / total))
                    .collect(),
            ))
        })
        .collect()
}

fn distribution_panel<DB: DrawingBackend>(area: &DrawingArea<DB, Shift>, data: &Data) -> Result<()>
where
    DB::ErrorType: 'static,
{
    let rows = gap_distributions(data)?;
    if rows.is_empty() {
        area.draw(&Text::new(
            "Anisotropic gap distribution: no classified dual-seed data",
            (20, 45),
            ("sans-serif", 16),
        ))?;
        return Ok(());
    }
    let times: Vec<_> = rows.keys().map(|&t| t as f64 / 100.0).collect();
    let width = times.windows(2).map(|w| w[1] - w[0]).fold(3.0, f64::min) * 0.75;
    let maximum = rows.values().flatten().map(|(_, w)| *w).fold(0.0, f64::max);
    let ymax = rows
        .values()
        .flatten()
        .map(|(d, _)| d + GAP_BIN_MEV)
        .fold(0.5, f64::max)
        * 1.08;
    let mu = data
        .solve_parameters
        .get("aniso")
        .map(|r| format!("; mu*={}", r.mu_star))
        .unwrap_or_default();
    let mut chart = ChartBuilder::on(area)
        .caption(
            format!("Lowest positive Matsubara Fermi-surface gap{mu}"),
            ("sans-serif", 17),
        )
        .margin(15)
        .x_label_area_size(45)
        .y_label_area_size(65)
        .build_cartesian_2d(0.0..(times.last().unwrap() + width + 2.0), 0.0..ymax)?;
    chart
        .configure_mesh()
        .x_desc("Temperature (K)")
        .y_desc("|Delta_nk(iw0)| (meV)")
        .draw()?;
    for (t, bins) in rows {
        let t = t as f64 / 100.0;
        let hi = bins.last().unwrap().0 + GAP_BIN_MEV / 2.0;
        chart.draw_series(std::iter::once(PathElement::new(
            [(t, 0.0), (t, hi)],
            BLUE.mix(0.35),
        )))?;
        chart.draw_series(bins.into_iter().map(|(delta, mass)| {
            Rectangle::new(
                [
                    (t, (delta - GAP_BIN_MEV / 2.0).max(0.0)),
                    (t + width * mass / maximum, delta + GAP_BIN_MEV / 2.0),
                ],
                BLUE.mix(0.7).filled(),
            )
        }))?;
    }
    area.draw(&Text::new(
        "Width: normalized Fermi-surface weight; gap bin 0.05 meV; no temperature interpolation",
        (75, 58),
        ("sans-serif", 11),
    ))?;
    Ok(())
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Figures {
    schema_version: u32,
    data_digest: String,
    files: Vec<Artifact>,
}
fn range(xs: impl Iterator<Item = f64>) -> (f64, f64) {
    let values: Vec<_> = xs.collect();
    if values.is_empty() {
        return (0.0, 1.0);
    }
    let lo = values.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let pad = ((hi - lo) * 0.08).max(0.01);
    (lo - pad, hi + pad)
}
fn panel<DB: DrawingBackend>(area: &DrawingArea<DB, Shift>, data: &Data, kind: usize) -> Result<()>
where
    DB::ErrorType: 'static,
{
    if kind == 2 {
        return distribution_panel(area, data);
    }
    if kind == 0 {
        let parts = area.split_evenly((2, 1));
        for (i, part) in parts.iter().enumerate() {
            let xr = range(
                data.spectra
                    .iter()
                    .flat_map(|s| s.energy_mev.iter().copied()),
            );
            let mut curves = vec![];
            for s in &data.spectra {
                let values = native::cumulative(s);
                curves.push((
                    s.smearing_id.clone(),
                    s.energy_mev
                        .iter()
                        .copied()
                        .zip(if i == 0 { s.a2f.clone() } else { values })
                        .collect::<Vec<_>>(),
                ));
            }
            let yr = range(curves.iter().flat_map(|(_, xs)| xs.iter().map(|x| x.1)));
            let mut chart = ChartBuilder::on(part)
                .caption(
                    if i == 0 {
                        "Native alpha^2 F"
                    } else {
                        "Cumulative lambda"
                    },
                    ("sans-serif", 17),
                )
                .margin(12)
                .x_label_area_size(38)
                .y_label_area_size(65)
                .build_cartesian_2d(xr.0..xr.1, yr.0..yr.1)?;
            chart
                .configure_mesh()
                .x_desc("Energy (meV)")
                .y_desc(if i == 0 { "alpha^2 F" } else { "lambda" })
                .draw()?;
            for (n, (name, xs)) in curves.into_iter().enumerate() {
                let color = Palette99::pick(n).to_rgba();
                chart
                    .draw_series(LineSeries::new(xs, color.stroke_width(2)))?
                    .label(name)
                    .legend(move |(x, y)| {
                        PathElement::new([(x, y), (x + 20, y)], color.stroke_width(2))
                    });
            }
            chart
                .configure_series_labels()
                .background_style(WHITE.mix(0.85))
                .draw()?;
        }
        return Ok(());
    }
    if kind == 4 {
        let mut y = 28;
        area.draw(&Text::new(
            "Tc intervals (K); midpoint is an interval estimate",
            (20, y),
            ("sans-serif", 19),
        ))?;
        for (mode, r) in &data.modes {
            y += 44;
            area.draw(&Text::new(
                format!("{mode}: {}; {} attempts", r.search.status, r.attempt_count),
                (20, y),
                ("sans-serif", 17),
            ))?;
            y += 28;
            let label = if r.search.status == "located" {
                let [a, b] = r.search.interval_k().unwrap();
                format!(
                    "[{a:.2}, {b:.2}] K; midpoint {:.2}, half-width {:.2}",
                    r.search.estimate_k().unwrap(),
                    r.search.half_width_k().unwrap()
                )
            } else {
                format!("Known bracket: {:?}; no Tc estimate", r.search.interval_k())
            };
            area.draw(&Text::new(label, (20, y), ("sans-serif", 15)))?;
            y += 25;
            area.draw(&Text::new(r.method.clone(), (20, y), ("sans-serif", 13)))?;
            y += 25;
            area.draw(&Text::new(
                format!(
                    "rho margin {}; normal/super gap = {}/{} meV",
                    r.thresholds.eigen_margin,
                    r.thresholds.normal_gap_mev,
                    r.thresholds.super_gap_mev
                ),
                (20, y),
                ("sans-serif", 12),
            ))?;
        }
        return Ok(());
    }
    let title = match kind {
        1 => "Lowest positive Matsubara gap: band mean and range",
        _ => "Anisotropic gap by actual k index; not a symmetry path",
    };
    let mut curves: Vec<(String, Vec<(f64, f64)>)> = vec![];
    if kind == 1 {
        let keys: BTreeSet<_> = data.gaps.iter().map(|g| (g.mode.clone(), g.band)).collect();
        for (mode, band) in keys {
            let seed = data.gaps.iter().find(|g| g.mode == mode).unwrap().seed_mev;
            let mut rows: Vec<_> = data
                .gaps
                .iter()
                .filter(|g| g.mode == mode && g.band == band && g.seed_mev == seed)
                .collect();
            rows.sort_by_key(|g| g.temperature_centikelvin);
            // 只连接连续有效采样段；在已有失败尝试温度处断开。
            let failed: Vec<_> = data
                .modes
                .get(&mode)
                .map(|m| {
                    m.search
                        .points
                        .iter()
                        .filter(|(_, s)| **s == crate::superconductivity::search::Side::Uncertain)
                        .map(|(&t, _)| t)
                        .chain(m.failed_temperatures_centikelvin.iter().copied())
                        .collect()
                })
                .unwrap_or_default();
            let mut segment = vec![];
            let mut previous = None;
            for g in rows {
                if previous.is_some_and(|t| {
                    failed
                        .iter()
                        .any(|&f| f > t && f < g.temperature_centikelvin)
                }) && !segment.is_empty()
                {
                    curves.push((format!("{mode} band {band}"), std::mem::take(&mut segment)));
                }
                segment.push((
                    g.temperature_centikelvin as f64 / 100.0,
                    g.statistics.absolute_mean_mev,
                ));
                previous = Some(g.temperature_centikelvin);
            }
            if !segment.is_empty() {
                curves.push((format!("{mode} band {band}"), segment));
            }
        }
    } else {
        let keys: BTreeSet<_> = data
            .states
            .iter()
            .filter(|s| s.mode == "aniso")
            .map(|s| (s.temperature_centikelvin, s.band))
            .collect();
        let seed = data
            .states
            .iter()
            .find(|s| s.mode == "aniso")
            .map(|s| s.seed_mev);
        for (t, band) in keys {
            let xs = data
                .states
                .iter()
                .filter(|s| {
                    s.mode == "aniso"
                        && s.temperature_centikelvin == t
                        && s.band == band
                        && Some(s.seed_mev) == seed
                })
                .map(|s| (s.kpoint as f64, s.delta_mev))
                .collect();
            curves.push((format!("{} K, band {band}", t as f64 / 100.0), xs));
        }
    }
    if curves.is_empty() {
        area.draw(&Text::new(
            format!("{title}: no valid data"),
            (25, 45),
            ("sans-serif", 18),
        ))?;
        return Ok(());
    }
    let xr = range(curves.iter().flat_map(|(_, xs)| xs.iter().map(|x| x.0)));
    let yr = if kind == 1 {
        range(
            data.gaps
                .iter()
                .flat_map(|g| [g.statistics.min_absolute_mev, g.statistics.max_absolute_mev]),
        )
    } else {
        range(curves.iter().flat_map(|(_, xs)| xs.iter().map(|x| x.1)))
    };
    let mut chart = ChartBuilder::on(area)
        .caption(title, ("sans-serif", 17))
        .margin(15)
        .x_label_area_size(45)
        .y_label_area_size(65)
        .build_cartesian_2d(xr.0..xr.1, yr.0..yr.1)?;
    chart
        .configure_mesh()
        .x_desc(match kind {
            1 => "Temperature (K)",
            _ => "Original k-point index (see gap_states.csv)",
        })
        .y_desc("Gap (meV)")
        .draw()?;
    for (n, (name, xs)) in curves.into_iter().enumerate() {
        let color = Palette99::pick(n).to_rgba();
        if kind == 1 {
            chart.draw_series(LineSeries::new(xs.clone(), color.stroke_width(2)))?;
        }
        chart
            .draw_series(xs.into_iter().map(|xy| Circle::new(xy, 3, color.filled())))?
            .label(name)
            .legend(move |(x, y)| Circle::new((x + 10, y), 4, color.filled()));
    }
    if kind == 1 {
        for g in &data.gaps {
            let seed = data
                .gaps
                .iter()
                .find(|r| r.mode == g.mode)
                .unwrap()
                .seed_mev;
            if g.seed_mev != seed {
                continue;
            }
            let t = g.temperature_centikelvin as f64 / 100.0;
            chart.draw_series(std::iter::once(PathElement::new(
                [
                    (t, g.statistics.min_absolute_mev),
                    (t, g.statistics.max_absolute_mev),
                ],
                BLACK.mix(0.4),
            )))?;
        }
    }
    chart
        .configure_series_labels()
        .background_style(WHITE.mix(0.85))
        .label_font(("sans-serif", 11))
        .draw()?;
    Ok(())
}
fn render<DB: DrawingBackend>(area: DrawingArea<DB, Shift>, data: &Data, kind: usize) -> Result<()>
where
    DB::ErrorType: 'static,
{
    area.fill(&WHITE)?;
    let area = area.titled(
        &format!(
            "{} | {} | imaginary-axis FSR",
            data.run_id,
            if data.complete {
                "selected modes complete"
            } else {
                "PARTIAL RESULTS"
            }
        ),
        ("sans-serif", 20),
    )?;
    if kind == 5 {
        for (i, a) in area.split_evenly((3, 2)).iter().enumerate().take(5) {
            panel(a, data, i)?;
        }
    } else {
        panel(&area, data, kind)?;
    }
    area.present()?;
    Ok(())
}
pub fn plot(results: &Path, output: Option<&Path>, formats: &[String]) -> Result<PathBuf> {
    ensure!(
        !formats.is_empty()
            && formats
                .iter()
                .all(|s| matches!(s.as_str(), "png" | "svg" | "pdf")),
        "data_invalid: 格式只能为 png,svg,pdf"
    );
    let (_, data) = native::load(results)?;
    let output = output
        .map(PathBuf::from)
        .unwrap_or_else(|| results.join("figures"));
    fs::create_dir_all(&output)?;
    ensure!(
        !output.canonicalize()?.starts_with(
            results
                .join("gaps")
                .canonicalize()
                .unwrap_or_else(|_| results.join("gaps"))
        ),
        "input_conflict: 不得覆盖科学数据"
    );
    let _lock = store::Lock::acquire(&output.join("native-figures"))?;
    super::publication::recover(&output, "native-figures.json")?;
    let path = output.join("native-figures.json");
    let old = if path.exists() {
        Some(store::read::<Figures>(&path)?)
    } else {
        None
    };
    let mut owned = BTreeSet::new();
    let previous_files = old.as_ref().map(|o| o.files.clone()).unwrap_or_default();
    if let Some(old) = old {
        ensure!(old.schema_version == 1, "unsupported_version: 图像版本");
        for f in old.files {
            f.verify(&output)?;
            owned.insert(f.relative_path);
        }
    }
    for name in KINDS {
        for f in formats {
            let n = PathBuf::from(format!("{name}.{f}"));
            ensure!(
                !output.join(&n).exists() || owned.contains(&n),
                "input_conflict: 未知图像冲突 {}",
                n.display()
            );
        }
    }
    let stage = output.join(format!(".native-{}", store::token()));
    fs::create_dir(&stage)?;
    let mut files = vec![];
    for (i, name) in KINDS.iter().enumerate() {
        let size = if i == 5 { (1800, 1800) } else { (1300, 900) };
        if formats.iter().any(|f| f == "svg" || f == "pdf") {
            let svg = stage.join(format!("{name}.svg"));
            render(SVGBackend::new(&svg, size).into_drawing_area(), &data, i)?;
            if formats.iter().any(|f| f == "pdf") {
                let mut options = svg2pdf::usvg::Options::default();
                options.fontdb_mut().load_system_fonts();
                let tree = svg2pdf::usvg::Tree::from_str(&fs::read_to_string(&svg)?, &options)?;
                let bytes = svg2pdf::to_pdf(&tree, Default::default(), Default::default())
                    .map_err(|e| anyhow::anyhow!("PDF 转换失败: {e}"))?;
                fs::write(stage.join(format!("{name}.pdf")), bytes)?;
            }
        }
        if formats.iter().any(|f| f == "png") {
            render(
                BitMapBackend::new(&stage.join(format!("{name}.png")), size).into_drawing_area(),
                &data,
                i,
            )?;
        }
        for f in formats {
            files.push(Artifact::capture(&stage, &format!("{name}.{f}"))?);
        }
        let svg = stage.join(format!("{name}.svg"));
        if !formats.iter().any(|f| f == "svg") && svg.exists() {
            fs::remove_file(svg)?;
        }
    }
    let figures = Figures {
        schema_version: 1,
        data_digest: store::hash_file(&results.join("manifest.json"))?,
        files,
    };
    super::publication::publish(
        &output,
        &stage,
        "native-figures.json",
        previous_files,
        figures.files.clone(),
        &figures,
    )?;
    Ok(output)
}
pub fn check(results: &Path) -> Result<bool> {
    super::publication::ensure_finished(&results.join("figures"), "native-figures.json")?;
    let path = results.join("figures/native-figures.json");
    if !path.exists() {
        return Ok(false);
    }
    let f: Figures = store::read(&path)?;
    ensure!(
        f.schema_version == 1 && f.data_digest == store::hash_file(&results.join("manifest.json"))?,
        "data_invalid: 原生图表来源改变"
    );
    for a in &f.files {
        a.verify(&results.join("figures"))?;
    }
    Ok(KINDS.iter().all(|name| {
        ["png", "svg", "pdf"].iter().all(|ext| {
            f.files
                .iter()
                .filter(|a| a.relative_path == PathBuf::from(format!("{name}.{ext}")))
                .count()
                == 1
        })
    }))
}
