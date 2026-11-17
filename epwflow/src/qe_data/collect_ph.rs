//! 原生 PH 交付到 EPW save，局部扰动模式与其响应势始终成对。
use super::super::foundation::store;
use super::{
    manifest::{Artifact, PhResponse, QPoint},
    native_ph,
    xml::Geometry,
};
use anyhow::{ensure, Context, Result};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

#[derive(Debug, Clone)]
pub struct CollectedPh {
    pub responses: Vec<PhResponse>,
    pub total_q: usize,
    pub artifacts: Vec<Artifact>,
}

pub fn parse_dyn_mode_count(path: &Path) -> Result<usize> {
    native_ph::dyn_modes(path)
}

pub fn collect(
    work: &Path,
    qpoints: &[QPoint],
    responses: &[PhResponse],
    prefix: &str,
    geometry: &Geometry,
) -> Result<CollectedPh> {
    ensure!(
        !qpoints.is_empty() && qpoints.len() == responses.len(),
        "data_invalid: PH Response and q The points don't match"
    );
    let mut by_q = BTreeMap::new();
    for r in responses {
        ensure!(
            by_q.insert(r.q_id, r).is_none(),
            "data_invalid: PH Repeat q Response"
        );
    }
    let mut source_checks = BTreeMap::new();
    for q in qpoints {
        let r = by_q
            .get(&q.global_index)
            .context("data_invalid: Lack of physics q Response")?;
        native_ph::verify(r, q, geometry)?;
        native_ph::control(&r.control_metadata, qpoints)?;
        for p in [
            &r.dyn_file,
            &r.patterns,
            &r.control_metadata,
            r.dvscf.as_ref().unwrap(),
        ] {
            if !crate::foundation::layout::enabled(work) {source_checks.insert(p.clone(), store::hash_file(p)?);}
        }
    }
    let dest = crate::foundation::layout::path(&work, "qe-data/save");
    let importing = work.join("epw-state/imports/phonon-source.json").is_file();
    ensure!(
        !dest.exists() || importing || crate::foundation::layout::enabled(work),
        "input_conflict: PH The delivery directory already exists, keep the original site"
    );
    let phsave = dest.join(format!("{prefix}.phsave"));
    fs::create_dir_all(&phsave)?;
    let mut paths = BTreeMap::new();
    let mut output = vec![];
    let dyn0 = dest.join(format!("{prefix}.dyn0.xml"));
    independent_copy(&qpoints[0].source_dyn0, &dyn0)?;
    paths.insert("dyn0".to_string(), dyn0);
    let control = phsave.join("control_ph.xml");
    independent_copy(&responses[0].control_metadata, &control)?;
    paths.insert("phsave_control".into(), control.clone());
    for q in qpoints {
        let r = by_q[&q.global_index];
        let dyn_file = dest.join(format!("{prefix}.dyn_q{}.xml", q.global_index));
        let dvscf = dest.join(format!("{prefix}.dvscf_q{}", q.global_index));
        independent_copy(&r.dyn_file, &dyn_file)?;
        independent_copy(r.dvscf.as_ref().unwrap(), &dvscf)?;
        let patterns = phsave.join(format!("patterns.{}.xml", q.global_index));
        let original = fs::read_to_string(&r.patterns)?;
        let doc = roxmltree::Document::parse(&original)?;
        let q_node = doc
            .descendants()
            .find(|n| n.has_tag_name("QPOINT_NUMBER"))
            .context("data_invalid: patterns Missing Internal q")?;
        let range = q_node.range();
        // 只更改内部整数标签，保留每个局部计算的完整复扰动基底。
        let normalized = format!(
            "{}<QPOINT_NUMBER>{}</QPOINT_NUMBER>{}",
            &original[..range.start],
            q.global_index,
            &original[range.end..]
        );
        if patterns.exists() {
            ensure!(
                fs::read_to_string(&patterns)? == normalized,
                "input_conflict: Recursive patterns Content Change"
            );
        } else {
            store::atomic_bytes(&patterns, normalized.as_bytes())?;
        }
        native_ph::patterns(&patterns, q.global_index, geometry.atoms.len() * 3)?;
        for (role, p) in [
            ("dyn", &dyn_file),
            ("patterns", &patterns),
            ("dvscf", &dvscf),
        ] {
            paths.insert(format!("{role}_q{}", q.global_index), p.clone());
        }
        output.push(PhResponse {
            q_id: q.global_index,
            dyn_file,
            patterns,
            dvscf: Some(dvscf),
            control_metadata: control.clone(),
            format: native_ph::FORMAT.into(),
            perturbations: geometry.atoms.len() * 3,
        });
    }
    if crate::foundation::layout::enabled(work) {return Ok(CollectedPh{responses:output,total_q:qpoints.len(),artifacts:Vec::new()});}
    let mut artifacts = vec![];
    let mut unique = BTreeSet::new();
    for (role, p) in paths {
        ensure!(unique.insert(p.clone()), "data_invalid: PH Role Path Conflict");
        artifacts.push(Artifact {
            role,
            relative_path: p.strip_prefix(work)?.into(),
            byte_size: fs::metadata(&p)?.len(),
            sha256: store::hash_file(&p)?,
            producer_stage: "phonon".into(),
            input_digest: String::new(),
            format_version: native_ph::FORMAT.into(),
        });
    }
    for (p, sha) in source_checks {
        ensure!(
            store::hash_file(&p)? == sha,
            "input_conflict: PH Source changes during clustering"
        );
    }
    Ok(CollectedPh {
        responses: output,
        total_q: qpoints.len(),
        artifacts,
    })
}

/// 部分导入可恢复；存在的目标必须逐字相同，不覆盖损坏或冲突证据。
pub(crate) fn independent_copy(source: &Path, target: &Path) -> Result<()> {
    if target.ancestors().any(crate::foundation::layout::enabled) {
        ensure!(!fs::symlink_metadata(target).is_ok_and(|m|m.file_type().is_symlink()),"input_conflict: PH destination must be independent, not a link {}",target.display());
        if source!=target {fs::copy(source,target).with_context(||format!("execution_failed: copy {}",source.display()))?;}
        return Ok(());
    }
    if target.exists() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            ensure!(
                fs::metadata(target)?.nlink() == 1,
                "input_conflict: The goal of the independent cluster cannot be a hard link {}",
                target.display()
            );
        }
        ensure!(
            !fs::symlink_metadata(target)?.file_type().is_symlink()
                && fs::metadata(source)?.len() == fs::metadata(target)?.len()
                && store::hash_file(source)? == store::hash_file(target)?,
            "input_conflict: Separated targets damaged or in conflict {}",
            target.display()
        );
        return Ok(());
    }
    let temporary = target.with_extension(format!("copy-{}", store::token()));
    fs::copy(source, &temporary)?;
    ensure!(
        store::hash_file(source)? == store::hash_file(&temporary)?,
        "input_conflict: Source change when copying"
    );
    fs::rename(temporary, target)?;
    Ok(())
}

/// 把 q 点映射落盘为可定位 JSON（FR-008 的 q_index_map.json）。
pub fn write_q_index_map(work: &Path, qpoints: &[QPoint]) -> Result<()> {
    let path = crate::foundation::layout::path(&work, "qe-data").join("q_index_map.json");
    let map: BTreeMap<usize, serde_json::Value> = qpoints
        .iter()
        .map(|q| {
            (
                q.global_index,
                serde_json::json!({
                    "local_index": q.local_index,
                    "directory": q.directory,
                    "crystal": q.crystal,
                    "cartesian": q.cartesian,
                }),
            )
        })
        .collect();
    store::write(&path, &map)?;
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn independent_import_copy_rejects_links_and_conflicting_partial_files() {
        let root = std::env::temp_dir().join(format!("ph-copy-{}", store::token()));
        fs::create_dir(&root).unwrap();
        let source = root.join("source");
        let target = root.join("target");
        fs::write(&source, b"verified response").unwrap();
        independent_copy(&source, &target).unwrap();
        independent_copy(&source, &target).unwrap();
        fs::write(&target, b"damaged").unwrap();
        assert!(independent_copy(&source, &target).is_err());
        fs::remove_file(&target).unwrap();
        fs::hard_link(&source, &target).unwrap();
        assert!(independent_copy(&source, &target).is_err());
        fs::remove_file(&target).unwrap();
        std::os::unix::fs::symlink(&source, &target).unwrap();
        assert!(independent_copy(&source, &target).is_err());
        assert_eq!(fs::read(&source).unwrap(), b"verified response");
        fs::remove_dir_all(root).unwrap();
    }
}
