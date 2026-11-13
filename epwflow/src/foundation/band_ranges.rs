//! 前置电子阶段的能带范围诊断；只读 XML，不执行科学程序或放宽选窗。
use super::{
    config::{Config, EnergyReference},
    electronic_style as style,
    stages::Step,
    store::{self, Identity},
};
use crate::qe_data::nscf;
use anyhow::{Context, Result};
use qf_wannier::{Bounds, Reference, SelectionMode};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};

const VERSION: u32 = 2;
const NAMES: [&str; 4] = ["dis_win_min", "dis_win_max", "dis_froz_min", "dis_froz_max"];
const COLORS: [&str; 8] = [
    "#008046", "#d88a00", "#91419b", "#7d5523", "#647014", "#008c95", "#b26060", "#555555",
];

#[derive(Debug, Serialize, Deserialize)]
pub struct BandRange {
    pub band: usize,
    pub min_ev: f64,
    pub max_ev: f64,
    pub crosses_fermi: bool,
    pub excluded: bool,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Report {
    pub schema_version: u32,
    pub renderer_version: u32,
    pub stage: String,
    pub prefix: String,
    pub sample_kind: String,
    pub nk: usize,
    pub nbnd: usize,
    pub fermi_ev: f64,
    pub fermi_source: String,
    pub num_wann: usize,
    pub windows_absolute_ev: [Option<f64>; 4],
    pub window_source: String,
    pub window_error: Option<String>,
    pub band_ranges: Vec<BandRange>,
    pub outer_counts: Option<Vec<usize>>,
    pub energy_frozen_counts: Option<Vec<usize>>,
    pub sample_capacity_ok: Option<bool>,
    pub sources: Vec<Identity>,
    pub request_key: String,
    pub output_sha256: BTreeMap<String, String>,
}

fn paths(cfg: &Config, step: Step) -> Result<(PathBuf, Vec<PathBuf>)> {
    let xml = format!("{}.save/data-file-schema.xml", cfg.qe.prefix);
    let dir = step.directory(cfg);
    Ok(match step {
        Step::Scf => (dir.join("plots"), vec![dir.join("snapshot").join(xml)]),
        Step::Nscf => (dir.join("plots"), vec![dir.join("out").join(xml)]),
        Step::Electronic => (
            dir.join("qe_eband/plots"),
            vec![
                dir.join("qe_eband/out").join(&xml),
                Step::Nscf.directory(cfg).join("out").join(xml),
            ],
        ),
        _ => anyhow::bail!("configuration: 能带范围图仅支持 SCF／NSCF／electronic"),
    })
}
fn request(cfg: &Config, step: Step) -> Result<(PathBuf, Vec<Identity>, String)> {
    let (dir, inputs) = paths(cfg, step)?;
    let sources = inputs
        .iter()
        .map(|p| Identity::capture(p))
        .collect::<Result<Vec<_>>>()?;
    let key = store::hash_bytes(&serde_json::to_vec(&(
        VERSION,
        step,
        &cfg.qe.prefix,
        &cfg.wannier,
        &sources,
    ))?);
    Ok((dir, sources, key))
}
fn current(dir: &Path, stem: &str, key: &str) -> bool {
    let Ok(old) = store::read::<Report>(&dir.join(format!("{stem}.json"))) else {
        return false;
    };
    old.request_key == key
        && old.output_sha256.len() == 3
        && ["png", "svg", "csv"].iter().all(|ext| {
            let name = format!("{stem}.{ext}");
            old.output_sha256
                .get(&name)
                .is_some_and(|hash| store::hash_file(&dir.join(&name)).ok().as_ref() == Some(hash))
        })
}
pub fn pending(cfg: &Config, step: Step) -> Result<bool> {
    let (dir, _, key) = request(cfg, step)?;
    Ok(
        !current(&dir, &format!("{}-band-ranges", step.name()), &key)
            || super::stage_results::pending(cfg, step)?,
    )
}

fn summarize(cfg: &Config, step: Step, sources: Vec<Identity>, key: String) -> Result<Report> {
    let states = nscf::parse_xml(&sources[0].path)?;
    let fermi = if step == Step::Electronic {
        nscf::parse_xml(&sources[1].path)?.fermi
    } else {
        states.fermi
    };
    let w = &cfg.wannier;
    let bands = &states.eigenvalues;
    let nbnd = qf_wannier::validate_bands(bands)?;
    let excluded = |band: usize| w.exclude_bands.contains(&(band as u32));
    let kept = bands
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .filter(|(i, _)| !excluded(i + 1))
                .map(|(_, e)| *e)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let bounds = Bounds {
        dis_win_min: w.outer_window_ev.map(|v| v[0]),
        dis_win_max: w.outer_window_ev.map(|v| v[1]),
        dis_froz_min: w.frozen_window_ev.map(|v| v[0]),
        dis_froz_max: w.frozen_window_ev.map(|v| v[1]).or(w.frozen_max_ev),
    };
    let reference = match w.energy_reference {
        EnergyReference::QeZero => Reference::Absolute,
        EnergyReference::F2Fermi => Reference::Fermi,
    };
    let offset = match w.energy_reference {
        EnergyReference::QeZero => 0.,
        EnergyReference::F2Fermi => fermi,
    };
    let mut windows = [
        bounds.dis_win_min,
        bounds.dis_win_max,
        bounds.dis_froz_min,
        bounds.dis_froz_max,
    ]
    .map(|v| v.map(|e| e + offset));
    let resolved = match w.selection_mode {
        SelectionMode::EnergyOnly => {
            qf_wannier::energy_only_bounds(&kept, fermi, &bounds, reference).map(|v| v.map(Some))
        }
        SelectionMode::FixedPlusProjectability => qf_wannier::select_default(
            bands,
            fermi,
            w.num_wann as usize,
            &w.exclude_bands
                .iter()
                .map(|v| *v as usize)
                .collect::<Vec<_>>(),
            &bounds,
            reference,
        )
        .map(|s| NAMES.map(|name| s.parameters.get(name).copied())),
    };
    let window_error = match resolved {
        Ok(v) => {
            windows = v;
            None
        }
        Err(e) => Some(format!("{e:#}")),
    };
    // 缺少边界时不以图轴范围或猜测值冒充实际窗。
    let counts = |low: Option<f64>, high: Option<f64>| {
        low.zip(high).map(|(lo, hi)| {
            kept.iter()
                .map(|row| row.iter().filter(|&&e| lo <= e && e <= hi).count())
                .collect::<Vec<_>>()
        })
    };
    let outer = counts(windows[0], windows[1]);
    let frozen = counts(windows[2].or(windows[0]), windows[3]);
    let capacity = outer.as_ref().zip(frozen.as_ref()).map(|(o, f)| {
        o.iter()
            .zip(f)
            .all(|(&o, &f)| f <= w.num_wann as usize && w.num_wann as usize <= o)
    });
    let ranges = (0..nbnd)
        .map(|i| {
            let min = bands.iter().map(|r| r[i]).fold(f64::INFINITY, f64::min);
            let max = bands.iter().map(|r| r[i]).fold(f64::NEG_INFINITY, f64::max);
            BandRange {
                band: i + 1,
                min_ev: min,
                max_ev: max,
                crosses_fermi: min <= fermi && fermi <= max,
                excluded: excluded(i + 1),
            }
        })
        .collect();
    Ok(Report {
        schema_version: 1,
        renderer_version: VERSION,
        stage: step.name().into(),
        prefix: cfg.qe.prefix.clone(),
        sample_kind: if step == Step::Electronic {
            "path"
        } else {
            "mesh"
        }
        .into(),
        nk: bands.len(),
        nbnd,
        fermi_ev: fermi,
        fermi_source: if step == Step::Electronic {
            "NSCF".into()
        } else {
            step.name().to_uppercase()
        },
        num_wann: w.num_wann as usize,
        windows_absolute_ev: windows,
        window_source: "preview from this stage sample; final Wannier selection uses NSCF".into(),
        window_error,
        band_ranges: ranges,
        outer_counts: outer,
        energy_frozen_counts: frozen,
        sample_capacity_ok: capacity,
        sources,
        request_key: key,
        output_sha256: BTreeMap::new(),
    })
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn text(svg: &mut String, x: f64, y: f64, size: u32, anchor: &str, color: &str, value: &str) {
    write!(svg, "<text x='{x:.2}' y='{y:.2}' font-family='sans-serif' font-size='{size}' text-anchor='{anchor}' fill='{color}'>{}</text>", escape(value)).unwrap();
}
fn span(values: &Option<Vec<usize>>) -> String {
    values
        .as_ref()
        .and_then(|v| v.iter().min().zip(v.iter().max()))
        .map(|(a, b)| format!("{a}-{b}"))
        .unwrap_or_else(|| "unresolved".into())
}
fn render(r: &Report) -> (String, u32, u32) {
    let width = 2200u32;
    let crossed = r
        .band_ranges
        .iter()
        .filter(|b| b.crosses_fermi)
        .collect::<Vec<_>>();
    let height = (crossed.len() as u32 * 30 + 780).max(1180);
    let (left, right, top, bottom) = (105., 1660., 80., height as f64 - 205.);
    let mut low = r
        .band_ranges
        .iter()
        .map(|b| b.min_ev - r.fermi_ev)
        .fold(0., f64::min);
    let mut high = r
        .band_ranges
        .iter()
        .map(|b| b.max_ev - r.fermi_ev)
        .fold(0., f64::max);
    for w in r.windows_absolute_ev.iter().flatten() {
        low = low.min(*w - r.fermi_ev);
        high = high.max(*w - r.fermi_ev);
    }
    let pad = ((high - low) * 0.045).max(0.2);
    low -= pad;
    high += pad;
    let x = |band: f64| left + (band - 0.4) / (r.nbnd as f64 + 0.2) * (right - left);
    let y = |e: f64| bottom - (e - low) / (high - low) * (bottom - top);
    let mut svg = format!("<svg xmlns='http://www.w3.org/2000/svg' width='{width}' height='{height}' viewBox='0 0 {width} {height}'><rect width='100%' height='100%' fill='white'/>");
    text(
        &mut svg,
        (left + right) / 2.,
        40.,
        26,
        "middle",
        "#222",
        &format!(
            "{}: {} band energy ranges / {} k points / {} bands ({})",
            r.prefix,
            r.stage.to_uppercase(),
            r.nk,
            r.nbnd,
            r.sample_kind
        ),
    );
    let tick = ((r.nbnd as f64 / 24. / 5.).ceil() as usize * 5).max(1);
    for b in 1..=r.nbnd {
        if b != 1 && b % tick != 0 {
            continue;
        }
        let bx = x(b as f64);
        write!(
            svg,
            "<path d='M {bx:.2} {top} V {bottom}' stroke='#dedede'/>"
        )
        .unwrap();
        text(
            &mut svg,
            bx,
            bottom + 30.,
            19,
            "middle",
            "#222",
            &b.to_string(),
        );
    }
    let raw_step = (high - low) / 9.;
    let decade = 10f64.powf(raw_step.log10().floor());
    let step = [1., 2., 5., 10.]
        .into_iter()
        .find(|v| *v * decade >= raw_step)
        .unwrap()
        * decade;
    let mut e = (low / step).ceil() * step;
    while e <= high {
        text(
            &mut svg,
            left - 15.,
            y(e) + 6.,
            19,
            "end",
            "#222",
            &format!("{e:.1}"),
        );
        e += step;
    }
    let fy = y(0.);
    write!(
        svg,
        "<path d='M {left} {fy:.2} H {right}' stroke='#dc0000' stroke-dasharray='3 4'/>"
    )
    .unwrap();
    let mut range_lines = String::new();
    for band in &r.band_ranges {
        let color = crossed
            .iter()
            .position(|b| b.band == band.band)
            .map(|i| COLORS[i % COLORS.len()])
            .unwrap_or("#999999");
        let bx = x(band.band as f64);
        let y1 = y(band.min_ev - r.fermi_ev);
        let y2 = y(band.max_ev - r.fermi_ev);
        // 零宽能量范围仍用端点短横线表示，避免单 k 点的 band 消失。
        write!(range_lines, "<g data-band='{}'><path d='M {bx:.2} {y1:.2} V {y2:.2}' stroke='{color}' stroke-width='5'/><path d='M {:.2} {y1:.2} h 6 M {:.2} {y2:.2} h 6' stroke='{color}' stroke-width='2'/></g>", band.band, bx-3., bx-3.).unwrap();
    }
    let mut groups: Vec<(f64, Vec<usize>)> = vec![];
    for (i, boundary) in r.windows_absolute_ev.iter().enumerate() {
        if let Some(v) = boundary {
            if let Some((_, members)) = groups.iter_mut().find(|(e, _)| (*e - *v).abs() < 1e-8) {
                members.push(i);
            } else {
                groups.push((*v, vec![i]));
            }
        }
    }
    for (boundary, members) in &groups {
        let yy = y(boundary - r.fermi_ev);
        for (phase, i) in members.iter().enumerate() {
            let color = COLORS[[0, 3, 2, 1][*i]];
            let gap = if members.len() > 1 {
                18 * (members.len() - 1)
            } else {
                12
            };
            write!(svg, "<path data-window='{}' d='M {left} {yy:.2} H {right}' stroke='{color}' stroke-width='2.5' stroke-dasharray='18 {gap}' stroke-dashoffset='{}'/>", NAMES[*i], -(phase as i32)*18).unwrap();
        }
        text(
            &mut svg,
            left + 16.,
            yy - 9.,
            18,
            "start",
            COLORS[[0, 3, 2, 1][members[0]]],
            &members
                .iter()
                .map(|i| NAMES[*i])
                .collect::<Vec<_>>()
                .join(" = "),
        );
    }
    // 深能级几乎无色散，端点可能与窗口线重合；范围条最后绘制以保留可见性。
    svg.push_str(&range_lines);
    write!(svg, "<rect x='{left}' y='{top}' width='{}' height='{}' fill='none' stroke='#333' stroke-width='1.5'/>", right-left, bottom-top).unwrap();
    text(
        &mut svg,
        (left + right) / 2.,
        bottom + 67.,
        22,
        "middle",
        "#222",
        &format!("{} band index (1-based)", r.stage.to_uppercase()),
    );
    write!(svg, "<text transform='translate(30 {}) rotate(-90)' font-family='sans-serif' font-size='23' text-anchor='middle'>E − E_F (eV)</text>", (top+bottom)/2.).unwrap();
    let lx = 1700.;
    let mut ly = 108.;
    text(
        &mut svg,
        lx,
        ly,
        21,
        "start",
        "#777",
        "Band min-max on this sample",
    );
    ly += 35.;
    for (i, b) in crossed.iter().enumerate() {
        text(
            &mut svg,
            lx,
            ly,
            20,
            "start",
            COLORS[i % COLORS.len()],
            &format!(
                "Band {}: crosses E_F{}",
                b.band,
                if b.excluded { " (excluded)" } else { "" }
            ),
        );
        ly += 30.;
    }
    if crossed.is_empty() {
        text(
            &mut svg,
            lx,
            ly,
            20,
            "start",
            "#555",
            "No sampled band crosses E_F",
        );
        ly += 30.;
    }
    ly += 20.;
    for (i, value) in r.windows_absolute_ev.iter().enumerate() {
        let color = COLORS[[0, 3, 2, 1][i]];
        text(&mut svg, lx, ly, 21, "start", color, NAMES[i]);
        ly += 26.;
        if let Some(v) = value {
            text(
                &mut svg,
                lx + 12.,
                ly,
                20,
                "start",
                "#222",
                &format!("{v:.8} eV (absolute)"),
            );
            ly += 25.;
            text(
                &mut svg,
                lx + 12.,
                ly,
                20,
                "start",
                "#222",
                &format!("E_F {:+.8} eV", v - r.fermi_ev),
            );
        } else {
            text(
                &mut svg,
                lx + 12.,
                ly,
                19,
                "start",
                "#555",
                "automatic / unresolved",
            );
            ly += 25.;
        }
        ly += 43.;
    }
    let capacity = match r.sample_capacity_ok {
        Some(true) => "satisfied on this sample",
        Some(false) => "VIOLATED on this sample",
        None => "unresolved",
    };
    text(
        &mut svg,
        left,
        bottom + 110.,
        20,
        "start",
        "#222",
        &format!(
            "E_F = {:.8} eV ({}); ranges cover this {} sample only. num_wann = {}.",
            r.fermi_ev, r.fermi_source, r.sample_kind, r.num_wann
        ),
    );
    text(
        &mut svg,
        left,
        bottom + 142.,
        20,
        "start",
        "#222",
        &format!(
            "Energy-frozen states per k: {}; outer states per k: {}; frozen <= N <= outer: {}.",
            span(&r.energy_frozen_counts),
            span(&r.outer_counts),
            capacity
        ),
    );
    text(&mut svg, left, bottom+174., 19, "start", "#555", "Window preview from this stage; final Wannier selection uses NSCF. Excluded bands omitted from counts; projectability freezing is not counted.");
    if r.window_error.is_some() {
        text(
            &mut svg,
            lx,
            ly + 10.,
            18,
            "start",
            "#555",
            "Window resolution failed; see JSON.",
        );
    }
    svg.push_str("</svg>");
    (svg, width, height)
}

/// 阶段完成／复用后生成诊断，不把不合法窗口当作有效科学结果。
pub fn generate(cfg: &Config, step: Step) -> Result<()> {
    let (dir, sources, key) = request(cfg, step)?;
    let stem = format!("{}-band-ranges", step.name());
    let files = ["png", "svg", "csv", "json"].map(|ext| format!("{stem}.{ext}"));
    if !current(&dir, &stem, &key) {
        fs::create_dir_all(&dir)?;
        let mut report = summarize(cfg, step, sources.clone(), key)?;
        let (svg, width, height) = render(&report);
        store::atomic_bytes(&dir.join(&files[1]), svg.as_bytes())?;
        let temporary = dir.join(format!(".band-ranges-{}.png", store::token()));
        let rendered = style::png(&svg, &temporary, width * 2, height * 2);
        if let Err(error) = rendered {
            let _ = fs::remove_file(&temporary);
            return Err(error).context("plot_failed: 能带范围图生成失败；科学结果保留");
        }
        fs::rename(temporary, dir.join(&files[0]))?;
        let mut csv =
            "band,min_ev,max_ev,min_minus_fermi_ev,max_minus_fermi_ev,crosses_fermi,excluded\n"
                .to_string();
        for b in &report.band_ranges {
            writeln!(
                csv,
                "{},{:.12},{:.12},{:.12},{:.12},{},{}",
                b.band,
                b.min_ev,
                b.max_ev,
                b.min_ev - report.fermi_ev,
                b.max_ev - report.fermi_ev,
                b.crosses_fermi,
                b.excluded
            )?;
        }
        store::atomic_bytes(&dir.join(&files[2]), csv.as_bytes())?;
        for name in &files[..3] {
            report
                .output_sha256
                .insert(name.clone(), store::hash_file(&dir.join(name))?);
        }
        store::write(&dir.join(&files[3]), &report)?;
        println!("能带范围图：{}", dir.join(&files[0]).display());
    }
    for source in &sources {
        source.verify()?;
    }
    super::stage_results::publish(
        cfg,
        step,
        "band-ranges",
        &dir,
        &files,
        serde_json::json!({"kind":"band_ranges","renderer_version":VERSION,"window_preview":true}),
        sources,
    )
}

/// 可移动结果包只使用已校验报告重绘，不重新读取来源绝对路径。
pub fn redraw(path: &Path) -> Result<()> {
    let report: Report = store::read(path)?;
    let (svg, width, height) = render(&report);
    let stem = path.with_extension("");
    store::atomic_bytes(&stem.with_extension("svg"), svg.as_bytes())?;
    style::png(&svg, &stem.with_extension("png"), width * 2, height * 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn energy_range_statistics_keep_band_identity_and_exclusions() {
        let mut cfg: Config =
            toml::from_str(include_str!("../../tests/fixtures/input.toml")).unwrap();
        cfg.wannier.selection_mode = SelectionMode::EnergyOnly;
        cfg.wannier.num_wann = 1;
        cfg.wannier.energy_reference = EnergyReference::F2Fermi;
        cfg.wannier.outer_window_ev = Some([-4., 4.]);
        cfg.wannier.frozen_window_ev = Some([-2., 1.]);
        cfg.wannier.frozen_max_ev = None;
        cfg.wannier.exclude_bands = vec![1];
        let root = std::env::temp_dir().join(format!("range-statistics-{}", store::token()));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("bands.xml");
        let ha = 27.211386245988;
        let mut xml = "<root><band_structure><nbnd>4</nbnd><nks>2</nks><fermi_energy>".to_string()
            + &(5. / ha).to_string()
            + "</fermi_energy>";
        for row in [[1., 4., 5., 8.], [2., 4.5, 5.5, 9.]] {
            xml += &format!("<ks_energies><k_point weight='1'>0 0 0</k_point><eigenvalues>{}</eigenvalues></ks_energies>",row.map(|e|(e/ha).to_string()).join(" "));
        }
        xml += "</band_structure></root>";
        fs::write(&source, xml).unwrap();
        let r = summarize(
            &cfg,
            Step::Scf,
            vec![Identity::capture(&source).unwrap()],
            "test".into(),
        )
        .unwrap();
        assert_eq!(r.band_ranges[0].band, 1);
        assert!(r.band_ranges[0].excluded);
        assert!((r.band_ranges[3].min_ev - 8.).abs() < 1e-12);
        assert!((r.band_ranges[3].max_ev - 9.).abs() < 1e-12);
        assert_eq!(r.energy_frozen_counts, Some(vec![2, 2]));
        assert_eq!(r.sample_capacity_ok, Some(false));
        assert_eq!(
            r.windows_absolute_ev,
            [Some(1.), Some(9.), Some(3.), Some(6.)]
        );
        let (svg, _, _) = render(&r);
        assert!(svg.contains("VIOLATED"));
        assert!(svg.contains("data-band='4'"));
        assert!(qf_wannier::select_energy_only(
            &[vec![4., 5., 8.], vec![4.5, 5.5, 9.]],
            5.,
            1,
            &[],
            &Bounds {
                dis_froz_min: Some(-2.),
                dis_froz_max: Some(1.),
                ..Default::default()
            },
            Reference::Fermi
        )
        .is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
