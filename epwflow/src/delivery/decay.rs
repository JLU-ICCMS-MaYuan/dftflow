//! 粗网格实空间衰减诊断；只消费已绑定原生文件，不执行科学程序。
use super::publication;
use crate::{
    foundation::{
        config::Config,
        stage_results,
        stages::{self, Completion, Step},
        store::{self, Identity},
    },
    interpolation::comparison_data::Artifact,
};
use anyhow::{ensure, Context, Result};
use plotters::{coord::Shift, prelude::*};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const NATIVE: [&str; 4] = ["decay.H", "decay.dynmat", "decay.epmate", "decay.epmatp"];
const FIGURES: [&str; 5] = [
    "decay-H",
    "decay-dynmat",
    "decay-epmate",
    "decay-epmatp",
    "decay-overview",
];
const FORMATS: [&str; 3] = ["png", "svg", "pdf"];
const DATA: &str = "decay-data.json";
const MANIFEST: &str = "decay-manifest.json";
const PURPLE: RGBColor = RGBColor(170, 0, 204);

pub fn is_native(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| NATIVE.contains(&n))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Parameters {
    distance_unit: String,
    amplitude_unit: String,
    log_base: u32,
    color: [u8; 3],
}
impl Default for Parameters {
    fn default() -> Self {
        Self {
            distance_unit: "angstrom".into(),
            amplitude_unit: "Ry".into(),
            log_base: 10,
            color: [170, 0, 204],
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Data {
    schema_version: u32,
    scientific_key: String,
    completion: Identity,
    sources: Vec<Identity>,
    parameters: Parameters,
    panels: Vec<Vec<[f64; 2]>>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    files: Vec<Artifact>,
}

fn image(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| FORMATS.contains(&s))
}

fn names() -> Vec<String> {
    NATIVE
        .iter()
        .map(|s| s.to_string())
        .chain(std::iter::once(DATA.into()))
        .chain(
            FIGURES
                .iter()
                .flat_map(|name| FORMATS.iter().map(move |format| format!("{name}.{format}"))),
        )
        .collect()
}

fn parse(text: &str, name: &str) -> Result<Vec<[f64; 2]>> {
    let mut samples = vec![];
    for (line, row) in text.lines().enumerate() {
        let row = row.trim();
        if row.is_empty() || row.starts_with('#') {
            continue;
        }
        let columns: Vec<_> = row.split_whitespace().take(2).collect();
        let location = || format!("data_invalid: {name}:{} requires two finite nonnegative numbers", line + 1);
        ensure!(columns.len() == 2, "{}", location());
        let mut pair = [0.; 2];
        for i in 0..2 {
            pair[i] = columns[i]
                .replace(['D', 'd'], "E")
                .parse::<f64>()
                .with_context(location)?;
            ensure!(pair[i].is_finite() && pair[i] >= 0., "{}", location());
        }
        samples.push(pair);
    }
    ensure!(!samples.is_empty(), "data_invalid: {name} has no data rows");
    Ok(samples)
}

/// 原路径只在收集时验证；可移动包的读取只依赖包内副本和清单。
fn load(dir: &Path, require_images: bool) -> Result<(Data, Manifest)> {
    publication::ensure_finished(dir, MANIFEST)?;
    let manifest: Manifest = store::read(&dir.join(MANIFEST))?;
    let expected: std::collections::BTreeSet<_> = names().into_iter().map(PathBuf::from).collect();
    let actual: std::collections::BTreeSet<_> = manifest
        .files
        .iter()
        .map(|f| f.relative_path.clone())
        .collect();
    ensure!(
        manifest.schema_version == 1 && expected == actual && actual.len() == manifest.files.len(),
        "data_invalid: Incomplete version or file collection of the decay package"
    );
    for file in &manifest.files {
        if require_images || !image(&file.relative_path) || dir.join(&file.relative_path).exists() {
            file.verify(dir)?;
        }
    }
    let data: Data = store::read(&dir.join(DATA))?;
    ensure!(
        data.schema_version == 1
            && data.parameters == Parameters::default()
            && data.panels.len() == 4
            && data.sources.len() == 4
            && !data.scientific_key.is_empty(),
        "data_invalid: The decaychart data or drawing parameters are not supported"
    );
    for (i, name) in NATIVE.iter().enumerate() {
        let artifact = manifest
            .files
            .iter()
            .find(|f| f.relative_path == Path::new(name))
            .unwrap();
        ensure!(
            data.sources[i].path.file_name() == Some(std::ffi::OsStr::new(name))
                && data.sources[i].sha256 == artifact.sha256
                && data.sources[i].bytes == artifact.byte_size
                && parse(&fs::read_to_string(dir.join(name))?, name)? == data.panels[i],
            "input_conflict: {name} copy, source and plot data are inconsistent"
        );
    }
    Ok((data, manifest))
}

fn panel<B: DrawingBackend>(area: &DrawingArea<B, Shift>, rows: &[[f64; 2]], i: usize) -> Result<()>
where
    B::ErrorType: 'static,
{
    area.fill(&WHITE)?;
    let max_x = rows.iter().map(|r| r[0]).fold(0., f64::max);
    let hi_x = if max_x == 0. {
        1.
    } else {
        (max_x * 1.05).min(f64::MAX)
    };
    let positives: Vec<_> = rows.iter().filter(|r| r[1] > 0.).collect();
    let (lo_y, mut hi_y) = if positives.is_empty() {
        (0.1, 1.)
    } else {
        let lo = positives.iter().map(|r| r[1]).fold(f64::INFINITY, f64::min);
        let hi = positives.iter().map(|r| r[1]).fold(0., f64::max);
        ((lo * 0.8).max(f64::from_bits(1)), (hi * 1.2).min(f64::MAX))
    };
    if hi_y <= lo_y {
        hi_y = lo_y * 2.;
    }
    let mut chart = ChartBuilder::on(area)
        .caption(NATIVE[i], ("sans-serif", 24))
        .margin(22)
        .x_label_area_size(48)
        .y_label_area_size(88)
        .build_cartesian_2d(0.0..hi_x, (lo_y..hi_y).log_scale())?;
    chart
        .configure_mesh()
        .disable_mesh()
        .y_labels(6)
        .x_desc(if i == 0 || i == 2 {
            "|Rₑ| (Å)"
        } else {
            "|Rₚ| (Å)"
        })
        .y_desc(match i {
            0 => "max |H_nm| (Ry)",
            1 => "max |D_mu,nu| (Ry)",
            _ => "max |g_mn,nu| (Ry)",
        })
        .label_style(("sans-serif", 17))
        .axis_desc_style(("sans-serif", 19))
        .y_label_formatter(&|y| format!("{y:.0e}"))
        .draw()?;
    chart.draw_series(
        positives
            .iter()
            .map(|r| Circle::new((r[0], r[1]), 5, PURPLE.filled())),
    )?;
    let zeros = rows.len() - positives.len();
    if zeros > 0 {
        let note = if positives.is_empty() {
            format!("No positive values; {zeros} zero values omitted")
        } else {
            format!("Zero values omitted: {zeros}")
        };
        area.draw(&Text::new(note, (100, 57), ("sans-serif", 16).into_font()))?;
    }
    Ok(())
}

fn render<B: DrawingBackend>(area: DrawingArea<B, Shift>, data: &Data, i: usize) -> Result<()>
where
    B::ErrorType: 'static,
{
    area.fill(&WHITE)?;
    if i == 4 {
        for (j, sub) in area.split_evenly((2, 2)).iter().enumerate() {
            panel(sub, &data.panels[j], j)?;
        }
    } else {
        panel(&area, &data.panels[i], i)?;
    }
    area.present()?;
    Ok(())
}

fn render_all(dir: &Path, data: &Data) -> Result<()> {
    let mut options = svg2pdf::usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    for (i, name) in FIGURES.iter().enumerate() {
        let size = if i == 4 { (1600, 1100) } else { (1000, 700) };
        let svg = dir.join(format!("{name}.svg"));
        render(SVGBackend::new(&svg, size).into_drawing_area(), data, i)?;
        render(
            BitMapBackend::new(&dir.join(format!("{name}.png")), size).into_drawing_area(),
            data,
            i,
        )?;
        let tree = svg2pdf::usvg::Tree::from_str(&fs::read_to_string(&svg)?, &options)?;
        let pdf = svg2pdf::to_pdf(&tree, Default::default(), Default::default())
            .map_err(|e| anyhow::anyhow!("PDF conversion failed: {e}"))?;
        fs::write(dir.join(format!("{name}.pdf")), pdf)?;
    }
    Ok(())
}

fn publish(dir: &Path, stage: &Path, old: Vec<Artifact>) -> Result<()> {
    let files = names()
        .iter()
        .map(|name| Artifact::capture(stage, name))
        .collect::<Result<Vec<_>>>()?;
    let manifest = Manifest {
        schema_version: 1,
        files: files.clone(),
    };
    publication::publish(dir, stage, MANIFEST, old, files, &manifest)
}

fn bound_sources(
    receipt: &Completion,
    source_dir: &Path,
) -> Result<(Vec<Identity>, Vec<&'static str>)> {
    let mut sources = vec![];
    let mut missing = vec![];
    for name in NATIVE {
        let path = source_dir.join(name);
        if let Some(id) = receipt.outputs.iter().find(|id| id.path == path) {
            if store::identity_available(id)? {
                sources.push(id.clone());
            } else {
                missing.push(name);
            }
        } else {
            missing.push(name);
        }
    }
    Ok((sources, missing))
}

/// 只收集指定粗网格回执绑定的文件；也供只读原生数据出图验证使用。
pub fn collect(completion_path: &Path, output: &Path) -> Result<()> {
    if let Some(root)=crate::qe_data::direct::workspace(completion_path) {
        let cfg=crate::qe_data::direct::current(&root)?;
        let source=Step::Coarse.directory(&cfg);
        let sources=NATIVE.iter().map(|n|Identity::reference(&source.join(n))).collect::<Vec<_>>();
        let panels=sources.iter().zip(NATIVE).map(|(id,name)|parse(&fs::read_to_string(&id.path).with_context(||format!("data_invalid: missing {}",id.path.display()))?,name)).collect::<Result<Vec<_>>>()?;
        let data=Data{schema_version:1,scientific_key:String::new(),completion:Identity::reference(completion_path),sources,parameters:Parameters::default(),panels};
        fs::create_dir_all(output)?;
        let paths=data.sources.iter().map(|id|crate::foundation::workspace_data::relative(output,&id.path)).collect::<Result<Vec<_>>>()?;
        store::write(&output.join("decay.plot.json"),&serde_json::json!({"kind":"Decay","sources":paths}))?;
        return render_all(output,&data).map_err(|e|anyhow::anyhow!("plot_failed: decay: {e:#}"));
    }

    let completion = Identity::capture(completion_path)?;
    let receipt: Completion = if let Some(root) = crate::qe_data::direct::workspace(completion_path)
    {
        stages::completion(&crate::qe_data::direct::current(&root)?, Step::Coarse)?
    } else {
        store::read(completion_path)?
    };
    ensure!(
        receipt.step == Step::Coarse,
        "input_conflict: decay data must come from a coarse-grid receipt"
    );
    receipt.verify()?;
    let source_dir = completion
        .target
        .parent()
        .context("data_invalid: coarse-grid receipt has no parent directory")?;
    let (sources, missing) = bound_sources(&receipt, source_dir)?;
    ensure!(
        missing.is_empty(),
        "data_invalid: coarse-grid decay file missing or unbound: {}; EPW is not rerun automatically",
        missing.join(", ")
    );
    let panels = sources
        .iter()
        .zip(NATIVE)
        .map(|(id, name)| parse(&fs::read_to_string(&id.path)?, name))
        .collect::<Result<Vec<_>>>()?;
    let data = Data {
        schema_version: 1,
        scientific_key: receipt.scientific_key,
        completion,
        sources,
        parameters: Parameters::default(),
        panels,
    };
    if let Some(root)=crate::qe_data::direct::workspace(completion_path) {
        let cfg=crate::qe_data::direct::current(&root)?;
        if crate::foundation::layout::enabled(&root) {
            let dir=stage_results::root(&cfg).join("6.epw-coarse");fs::create_dir_all(&dir)?;
            // 只保存绘图所需的相对原始文件引用；数值仍唯一保存在第 6 步。
            let sources=data.sources.iter().map(|id| crate::foundation::workspace_data::relative(&dir,&id.path)).collect::<Result<Vec<_>>>()?;
            store::write(&dir.join("decay.plot.json"),&serde_json::json!({"kind":"Decay","sources":sources}))?;
            render_all(&dir,&data).map_err(|e|anyhow::anyhow!("plot_failed: decay: {e:#}"))?;
            return Ok(());
        }
    }
    fs::create_dir_all(output)?;
    let _lock = store::Lock::acquire(&output.join("decay-publication"))?;
    publication::recover(output, MANIFEST)?;
    if output.join(MANIFEST).exists() {
        let (old, manifest) = load(output, false)?;
        let same = old.completion.sha256 == data.completion.sha256
            && old.scientific_key == data.scientific_key
            && old
                .sources
                .iter()
                .zip(&data.sources)
                .all(|(a, b)| a.sha256 == b.sha256 && a.target == b.target);
        if same {
            if manifest
                .files
                .iter()
                .all(|f| output.join(&f.relative_path).exists())
            {
                return Ok(());
            }
            return redraw_locked(output, &old, manifest.files);
        }
        ensure!(
            crate::qe_data::direct::workspace(completion_path).is_some(),
            "input_conflict: The source of the decay map has changed and the package has been retained"
        );
    }

    let stage = output.join(format!(".native-{}", store::token()));
    fs::create_dir(&stage)?;
    for (source, name) in data.sources.iter().zip(NATIVE) {
        fs::copy(&source.path, stage.join(name))?;
    }
    store::write(&stage.join(DATA), &data)?;
    render_all(&stage, &data)?;
    for source in &data.sources {
        source.verify()?;
    }
    data.completion.verify()?;
    publish(output, &stage, vec![])?;
    load(output, true)?;
    Ok(())
}

fn redraw_locked(dir: &Path, data: &Data, old: Vec<Artifact>) -> Result<()> {
    let stage = dir.join(format!(".native-{}", store::token()));
    fs::create_dir(&stage)?;
    for name in NATIVE.into_iter().chain([DATA]) {
        fs::copy(dir.join(name), stage.join(name))?;
    }
    render_all(&stage, data)?;
    publish(dir, &stage, old)?;
    load(dir, true)?;
    Ok(())
}

pub fn redraw(dir: &Path) -> Result<()> {
    if dir.join("decay.plot.json").is_file() {
        let saved:serde_json::Value=store::read(&dir.join("decay.plot.json"))?;
        let paths:Vec<PathBuf>=serde_json::from_value(saved["sources"].clone())?;
        ensure!(paths.len()==NATIVE.len(),"data_invalid: decay source count");
        let panels=paths.iter().zip(NATIVE).map(|(path,name)|parse(&fs::read_to_string(dir.join(path))?,name)).collect::<Result<Vec<_>>>()?;
        let data=Data {schema_version:1,scientific_key:String::new(),completion:Identity::capture(&dir.join("decay.plot.json"))?,sources:vec![],parameters:Parameters::default(),panels};
        return render_all(dir,&data).map_err(|e|anyhow::anyhow!("plot_failed: decay: {e:#}"));
    }
    let _lock = store::Lock::acquire(&dir.join("decay-publication"))?;
    publication::recover(dir, MANIFEST)?;
    let (data, manifest) = load(dir, false)?;
    redraw_locked(dir, &data, manifest.files)
}

/// 仅由已有尝试的终态核验路径调用，不清理未知所有者的锁。
pub fn recover_finished(cfg: &Config, job: u64) -> Result<()> {
    for target in [
        Step::Coarse.directory(cfg).join("decay/decay-publication"),
        stage_results::root(cfg).join("6.epw-coarse/decay/stage-publication"),
        stage_results::root(cfg).join("index"),
    ] {
        store::recover_finished_lock(&target, job)?;
    }
    Ok(())
}

/// 新日志协议下从当前任务号表确认遗留发布锁的归属，仍拒绝未知所有者。
pub fn recover_pending(cfg: &Config) -> Result<()> {
    let targets = [
        Step::Coarse.directory(cfg).join("decay/decay-publication"),
        stage_results::root(cfg).join("6.epw-coarse/decay/stage-publication"),
        stage_results::root(cfg).join("index"),
    ];
    let mut jobs = std::collections::BTreeSet::new();
    for step in Step::ALL {
        jobs.extend(crate::foundation::readable_state::jobs(&cfg.work_dir, step)?.into_values());
    }
    for target in targets {
        let owner_path = target.with_extension("lock").join("owner.json");
        if !owner_path.exists() {
            continue;
        }
        let owner: serde_json::Value = store::read(&owner_path)?;
        let job = owner["job_id"]
            .as_u64()
            .context("scheduler_unknown: Release lock missing task number, keep original lock")?;
        ensure!(
            jobs.contains(&job),
            "scheduler_unknown: Release lock-on jobs {job} Not on current task list, keep the original lock"
        );
        store::recover_finished_lock(&target, job)?;
    }
    Ok(())
}

pub fn pending(cfg: &Config) -> Result<bool> {
    if crate::foundation::layout::enabled(&cfg.work_dir) {
        let dir=stage_results::root(cfg).join("6.epw-coarse");
        return Ok(!dir.join("decay.plot.json").is_file() || FIGURES.iter().any(|n|FORMATS.iter().any(|e|!dir.join(format!("{n}.{e}")).is_file())));
    }

    let receipt = stages::completion(cfg, Step::Coarse)?;
    let source = Step::Coarse.directory(cfg);
    if !bound_sources(&receipt, &source)?.1.is_empty() {
        return Ok(true);
    }
    if crate::foundation::layout::enabled(&cfg.work_dir) {
        let dir=stage_results::root(cfg).join("6.epw-coarse");
        if !dir.join("decay.plot.json").is_file() {return Ok(true);}
        let outputs=crate::foundation::readable_state::completed_files(cfg,Step::Coarse,"Published plots · decay")?;
        return Ok(outputs.is_none() || FIGURES.iter().any(|name| FORMATS.iter().any(|ext|!dir.join(format!("{name}.{ext}")).is_file())));
    }
    let dir = source.join("decay");
    if !dir.join(MANIFEST).exists() {
        return Ok(true);
    }
    let (data, manifest) = load(&dir, false)?;
    let (sources, _) = bound_sources(&receipt, &source)?;
    Ok(data.scientific_key != receipt.scientific_key
        || data
            .sources
            .iter()
            .zip(&sources)
            .any(|(a, b)| a.sha256 != b.sha256)
        || manifest
            .files
            .iter()
            .any(|f| !dir.join(&f.relative_path).exists()))
}

pub fn refresh(cfg: &Config) -> Result<()> {
    if crate::foundation::layout::enabled(&cfg.work_dir) {
        if !pending(cfg)? {return Ok(());}
        let receipt=Step::Coarse.directory(cfg).join("epw_coarse.in");
        let output=stage_results::root(cfg).join("6.epw-coarse");
        collect(&receipt,&output)?;
        let mut files=Vec::new();
        for name in FIGURES { for ext in ["png","svg","pdf"] { files.push(output.join(format!("{name}.{ext}"))); } }
        crate::foundation::readable_state::complete_files(cfg,Step::Coarse,"Published plots · decay",&files)?;
        return Ok(());
    }

    let receipt = Step::Coarse.directory(cfg).join("epw_coarse.in");
    let c = stages::completion(cfg, Step::Coarse)?;
    ensure!(
        c.scientific_key == stages::scientific_key(cfg, Step::Coarse)?,
        "input_conflict: Change source of coarse grid configuration"
    );
    let dir = Step::Coarse.directory(cfg).join("decay");
    collect(&receipt, &dir)?;
    let (data, _) = load(&dir, true)?;
    let mut files = names();
    files.push(MANIFEST.into());
    stage_results::publish(
        cfg,
        Step::Coarse,
        "decay",
        &dir,
        &files,
        serde_json::json!({"kind":"real_space_decay","parameters":data.parameters}),
        std::iter::once(data.completion)
            .chain(data.sources)
            .collect(),
    )
}

#[cfg(test)]
mod tests;
