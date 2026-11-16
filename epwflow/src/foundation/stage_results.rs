//! 分阶段派生交付；复用发布 journal，索引只是导航，不登记科学完成。
use super::{
    config::Config,
    stages::Step,
    store::{self, Identity},
};
use crate::{delivery::publication, interpolation::comparison_data::Artifact};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const MANIFEST: &str = "stage-manifest.json";
const INDEX_UPDATE: &str = ".index-update.json";
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub stage: String,
    pub unit: String,
    pub request_key: String,
    pub parameters: Value,
    pub sources: Vec<Identity>,
    pub files: Vec<Artifact>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub stage: String,
    pub unit: String,
    pub manifest: Artifact,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Index {
    pub schema_version: u32,
    pub entries: Vec<Entry>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IndexUpdate {
    entry: Entry,
    staging: String,
}

fn read_or_rebuild_index(results: &Path) -> Result<Index> {
    if results.join("index.json").exists() {
        return store::read(&results.join("index.json"));
    }
    let mut index = Index {
        schema_version: 1,
        entries: vec![],
    };
    for f in crate::qe_data::save::files(results)? {
        if f.relative_path.file_name().is_none_or(|n| n != MANIFEST)
            || f.relative_path
                .components()
                .any(|p| p.as_os_str().to_string_lossy().starts_with('.'))
        {
            continue;
        }
        let path = results.join(&f.relative_path);
        let dir = path.parent().unwrap();
        if dir.join(INDEX_UPDATE).exists() || publication::ensure_finished(dir, MANIFEST).is_err() {
            continue;
        }
        let m: Manifest = store::read(&path)?;
        ensure!(
            m.schema_version == 1
                && !m.files.is_empty()
                && results.join(&m.stage).join(&m.unit) == dir,
            "data_invalid: Index rediscovery manifest location conflict"
        );
        for artifact in &m.files {
            artifact.verify(dir)?;
        }
        index.entries.push(Entry {
            stage: m.stage,
            unit: m.unit,
            manifest: Artifact::capture(results, &f.relative_path.to_string_lossy())?,
        });
    }
    Ok(index)
}

/// 索引是导航，但其更新也必须能从已验证的发布事务接续。
fn finish_index(results: &Path, destination: &Path) -> Result<()> {
    let journal = destination.join(INDEX_UPDATE);
    if !journal.exists() {
        return Ok(());
    }
    publication::recover(destination, MANIFEST)?;
    let update: IndexUpdate = store::read(&journal)?;
    let entry = update.entry;
    if entry.manifest.verify(results).is_err() {
        ensure!(
            Path::new(&update.staging).components().count() == 1
                && update.staging.starts_with(".native-"),
            "data_invalid: The index temporary path is not valid"
        );
        let temp = destination.join(update.staging);
        ensure!(
            store::hash_file(&temp.join(MANIFEST))? == entry.manifest.sha256,
            "input_conflict: Index restore suspense manifest modified"
        );
        let next: Manifest = store::read(&temp.join(MANIFEST))?;
        let old = if destination.join(MANIFEST).exists() {
            store::read::<Manifest>(&destination.join(MANIFEST))?.files
        } else {
            vec![]
        };
        publication::publish(destination, &temp, MANIFEST, old, next.files.clone(), &next)?;
    }
    let path = entry.manifest.verify(results)?;
    ensure!(
        path == destination.join(MANIFEST),
        "data_invalid: Index recovery targets crossed the border"
    );
    let m: Manifest = store::read(&path)?;
    ensure!(
        m.schema_version == 1 && m.stage == entry.stage && m.unit == entry.unit,
        "data_invalid: The index restores an identity that does not match"
    );
    for f in &m.files {
        f.verify(destination)?;
    }
    let _lock = store::Lock::acquire(&results.join("index"))?;
    let index_path = results.join("index.json");
    let mut index = read_or_rebuild_index(results)?;
    ensure!(index.schema_version == 1, "data_invalid: Stage Index Version");
    index
        .entries
        .retain(|e| !(e.stage == entry.stage && e.unit == entry.unit));
    index.entries.push(entry);
    index
        .entries
        .sort_by(|a, b| (&a.stage, &a.unit).cmp(&(&b.stage, &b.unit)));
    store::write(&index_path, &index)?;
    fs::remove_file(journal)?;
    Ok(())
}

pub fn root(cfg: &Config) -> PathBuf {
    cfg.work_dir.join("results")
}
fn stage_name(cfg: &Config, step: Step) -> Result<String> {
    Ok(step
        .directory(cfg)
        .file_name()
        .context("data_invalid: Phase Name")?
        .to_str()
        .context("Stage name is not UTF-8")?
        .into())
}
pub fn check(results: &Path) -> Result<Index> {
    let index: Index = store::read(&results.join("index.json"))?;
    ensure!(index.schema_version == 1, "data_invalid: Stage Index Version");
    ensure!(!index.entries.is_empty(), "data_invalid: Results index is empty");
    for entry in &index.entries {
        let path = entry.manifest.verify(results)?;
        ensure!(
            !path.parent().unwrap().join(INDEX_UPDATE).exists(),
            "data_invalid: The index update has not yet been completed, please release it during recovery"
        );
        publication::ensure_finished(path.parent().unwrap(), MANIFEST)?;
        let m: Manifest = store::read(&path)?;
        ensure!(
            m.schema_version == 1
                && m.stage == entry.stage
                && m.unit == entry.unit
                && !m.files.is_empty(),
            "data_invalid: Stage manifest Identity or File Collection"
        );
        for f in &m.files {
            f.verify(path.parent().unwrap())?;
        }
    }
    Ok(index)
}
/// 每组文件在同一目录内发布；已存在文件只允许来自前一份已登记清单。
pub fn publish(
    cfg: &Config,
    step: Step,
    unit: &str,
    source: &Path,
    names: &[String],
    parameters: Value,
    sources: Vec<Identity>,
) -> Result<()> {
    let stage = stage_name(cfg, step)?;
    ensure!(
        Path::new(unit)
            .components()
            .all(|p| matches!(p, std::path::Component::Normal(_))),
        "data_invalid: The result unit crosses the border"
    );
    let results = root(cfg);
    let destination = results.join(&stage).join(unit);
    fs::create_dir_all(&destination)?;
    let _lock = store::Lock::acquire(&destination.join("stage-publication"))?;
    publication::recover(&destination, MANIFEST)?;
    finish_index(&results, &destination)?;
    let old = if destination.join(MANIFEST).exists() {
        let m: Manifest = store::read(&destination.join(MANIFEST))?;
        ensure!(
            m.stage == stage && m.unit == unit && m.schema_version == 1,
            "input_conflict: Phase delivery identity"
        );
        m.files
    } else {
        vec![]
    };
    let temp = destination.join(format!(".native-{}", store::token()));
    fs::create_dir(&temp)?;
    let mut files = vec![];
    for name in names {
        ensure!(
            Path::new(name).components().count() == 1 && name != MANIFEST,
            "data_invalid: Phase file name crossed the border"
        );
        let id = Identity::capture(&source.join(name))?;
        fs::copy(&id.path, temp.join(name))?;
        id.verify()?;
        files.push(Artifact::capture(&temp, name)?);
    }
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    ensure!(!files.is_empty(), "data_invalid: No stage results to publish");
    let manifest = Manifest {
        schema_version: 1,
        stage: stage.clone(),
        unit: unit.into(),
        request_key: super::stages::scientific_key(cfg, step)?,
        parameters,
        sources,
        files: files.clone(),
    };
    store::write(&temp.join(MANIFEST), &manifest)?;
    let mut artifact = Artifact::capture(&temp, MANIFEST)?;
    artifact.relative_path = destination.join(MANIFEST).strip_prefix(&results)?.into();
    store::write(
        &destination.join(INDEX_UPDATE),
        &IndexUpdate {
            entry: Entry {
                stage,
                unit: unit.into(),
                manifest: artifact,
            },
            staging: temp.file_name().unwrap().to_str().unwrap().into(),
        },
    )?;
    publication::publish(&destination, &temp, MANIFEST, old, files, &manifest)?;
    // 每个单元先完整落盘，再串行更新导航；中断后重发同一单元即可补索引。
    finish_index(&results, &destination)?;
    Ok(())
}
pub fn tree(cfg: &Config, step: Step, unit: &str, source: &Path, parameters: Value) -> Result<()> {
    let mut groups: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();
    for f in crate::qe_data::save::files(source)? {
        let path = f.relative_path;
        if path.components().any(|p| {
            let n = p.as_os_str().to_string_lossy();
            n.starts_with('.') || n.ends_with(".lock")
        }) || path.file_name().is_some_and(|n| n == MANIFEST)
        {
            continue;
        }
        groups
            .entry(path.parent().unwrap().into())
            .or_default()
            .push(path.file_name().unwrap().to_string_lossy().into_owned());
    }
    let sources = if let Some(ids) = parameters.get("sources") {
        serde_json::from_value(ids.clone())?
    } else {
        groups
            .iter()
            .flat_map(|(parent, names)| names.iter().map(move |n| source.join(parent).join(n)))
            .map(|p| Identity::capture(&p))
            .collect::<Result<Vec<_>>>()?
    };
    for (parent, names) in groups {
        let sub = Path::new(unit).join(&parent);
        publish(
            cfg,
            step,
            &sub.to_string_lossy(),
            &source.join(parent),
            &names,
            parameters.clone(),
            sources.clone(),
        )?;
    }
    Ok(())
}
pub fn pending(cfg: &Config, step: Step) -> Result<bool> {
    let index_path = root(cfg).join("index.json");
    if !index_path.exists() {
        return Ok(true);
    }
    let index: Index = store::read(&index_path)?;
    let stage = stage_name(cfg, step)?;
    let entries = index
        .entries
        .iter()
        .filter(|e| e.stage == stage)
        .collect::<Vec<_>>();
    if entries.is_empty() {
        return Ok(true);
    }
    if step == Step::Coarse && !entries.iter().any(|e| e.unit == "decay") {
        return Ok(true);
    }
    if step == Step::Electronic
        && ["qe_eband", "qe_eledos"]
            .iter()
            .any(|u| !entries.iter().any(|e| e.unit == *u))
    {
        return Ok(true);
    }
    if step == Step::Phonon
        && ["phband", "phdos", ""]
            .iter()
            .any(|u| !entries.iter().any(|e| e.unit == *u))
    {
        return Ok(true);
    }
    for e in entries {
        let dir = root(cfg).join(&e.stage).join(&e.unit);
        if dir.join(INDEX_UPDATE).exists() || dir.join(format!(".{MANIFEST}.publish.json")).exists()
        {
            return Ok(true);
        }
        if !root(cfg).join(&e.manifest.relative_path).exists() {
            return Ok(true);
        }
        let path = e.manifest.verify(&root(cfg))?;
        let m: Manifest = store::read(&path)?;
        if m.request_key != super::stages::scientific_key(cfg, step)? {
            return Ok(true);
        }
        for f in m.files {
            if !path.parent().unwrap().join(&f.relative_path).exists() {
                return Ok(true);
            }
            f.verify(path.parent().unwrap())?;
        }
    }
    Ok(false)
}

/// 重绘只使用包内数值及参数；来源绝对路径只用于审计，不作运行依赖。
pub fn redraw(results: &Path) -> Result<PathBuf> {
    let mut index: Index = store::read(&results.join("index.json"))?;
    ensure!(index.schema_version == 1, "data_invalid: Stage Index Version");
    let mut stages = std::collections::BTreeSet::new();
    for entry in &index.entries {
        let file = entry.manifest.verify(results)?;
        let m: Manifest = store::read(&file)?;
        for f in &m.files {
            let image = f
                .relative_path
                .extension()
                .is_some_and(|e| matches!(e.to_str(), Some("png" | "jpg" | "svg" | "pdf")));
            if !image || file.parent().unwrap().join(&f.relative_path).exists() {
                f.verify(file.parent().unwrap())?;
            }
        }
        stages.insert(entry.stage.clone());
    }
    for stage in stages {
        let dir = results.join(&stage);
        if dir.join("decay/decay-manifest.json").exists() {
            crate::delivery::decay::redraw(&dir.join("decay"))?;
        }
        if dir.join("comparison/manifest.json").exists() {
            crate::delivery::comparison_plot::plot(
                &dir,
                None,
                &["png".into(), "svg".into(), "pdf".into()],
            )?;
        }
        if dir.join("manifest.json").exists() {
            crate::delivery::native_plot::plot(
                &dir,
                None,
                &["png".into(), "svg".into(), "pdf".into()],
            )?;
        }
        for f in crate::qe_data::save::files(&dir)? {
            if f.relative_path.to_string_lossy().ends_with(".plot.json") {
                super::workspace_plot::redraw(&dir.join(f.relative_path))?;
            } else if f.relative_path.to_string_lossy().ends_with("-band-ranges.json") {
                super::band_ranges::redraw(&dir.join(f.relative_path))?;
            }
        }
        if stage.starts_with("4.qe-phonon-") {
            let m: Manifest = store::read(&dir.join(MANIFEST))?;
            let prefix = m.parameters["prefix"]
                .as_str()
                .context("data_invalid: phonon prefix")?;
            let elements: Vec<String> = serde_json::from_value(m.parameters["elements"].clone())?;
            let counts: Vec<usize> = serde_json::from_value(m.parameters["counts"].clone())?;
            let sigma = m.parameters["sigma_cm1"]
                .as_f64()
                .context("data_invalid: Unspeakable extension")?;
            let work = dir.to_str().context("Result path is not UTF-8")?;
            qf_phonon::plot::plot_phonon_band(work, prefix)?;
            qf_phonon::plot::plot_phonon_dos(work, prefix, &elements, &counts, sigma)?;
            qf_phonon::plot::plot_phonon_combined(work, prefix, &elements, &counts, sigma)?;
        }
    }
    // 派生图片清单可变；数值表和来源在重绘前均已严格校验。
    for entry in &mut index.entries {
        let file = results.join(&entry.manifest.relative_path);
        let mut m: Manifest = store::read(&file)?;
        for f in &mut m.files {
            *f = Artifact::capture(file.parent().unwrap(), &f.relative_path.to_string_lossy())?;
        }
        store::write(&file, &m)?;
        entry.manifest =
            Artifact::capture(results, &entry.manifest.relative_path.to_string_lossy())?;
    }
    store::write(&results.join("index.json"), &index)?;
    check(results)?;
    Ok(results.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_update_recovers_before_and_after_file_publication() {
        for already_published in [false, true] {
            let results = std::env::temp_dir().join(format!("stage-index-{}", store::token()));
            let dir = results.join("5.epw-wannier");
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("data.csv"), "old").unwrap();
            let mut m = Manifest {
                schema_version: 1,
                stage: "5.epw-wannier".into(),
                unit: "".into(),
                request_key: "science".into(),
                parameters: Value::Null,
                sources: vec![],
                files: vec![Artifact::capture(&dir, "data.csv").unwrap()],
            };
            store::write(&dir.join(MANIFEST), &m).unwrap();
            store::write(
                &results.join("index.json"),
                &Index {
                    schema_version: 1,
                    entries: vec![Entry {
                        stage: m.stage.clone(),
                        unit: "".into(),
                        manifest: Artifact::capture(&results, "5.epw-wannier/stage-manifest.json")
                            .unwrap(),
                    }],
                },
            )
            .unwrap();
            let temp = dir.join(".native-fixture");
            fs::create_dir(&temp).unwrap();
            fs::write(temp.join("data.csv"), "new").unwrap();
            let old = m.files.clone();
            m.files = vec![Artifact::capture(&temp, "data.csv").unwrap()];
            store::write(&temp.join(MANIFEST), &m).unwrap();
            let mut a = Artifact::capture(&temp, MANIFEST).unwrap();
            a.relative_path = "5.epw-wannier/stage-manifest.json".into();
            store::write(
                &dir.join(INDEX_UPDATE),
                &IndexUpdate {
                    entry: Entry {
                        stage: m.stage.clone(),
                        unit: "".into(),
                        manifest: a,
                    },
                    staging: ".native-fixture".into(),
                },
            )
            .unwrap();
            if already_published {
                publication::publish(&dir, &temp, MANIFEST, old, m.files.clone(), &m).unwrap();
            }
            assert!(check(&results).is_err());
            finish_index(&results, &dir).unwrap();
            check(&results).unwrap();
            assert_eq!(fs::read_to_string(dir.join("data.csv")).unwrap(), "new");
            assert!(!dir.join(INDEX_UPDATE).exists());
            fs::remove_file(results.join("index.json")).unwrap();
            let rebuilt = read_or_rebuild_index(&results).unwrap();
            assert_eq!(rebuilt.entries.len(), 1);
            store::write(&results.join("index.json"), &rebuilt).unwrap();
            check(&results).unwrap();
            fs::remove_dir_all(&results).unwrap();
        }
    }
}
