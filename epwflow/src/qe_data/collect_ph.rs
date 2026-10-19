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
        "data_invalid: PH 响应与 q 点数不符"
    );
    let mut by_q = BTreeMap::new();
    for r in responses {
        ensure!(
            by_q.insert(r.q_id, r).is_none(),
            "data_invalid: PH 重复 q 响应"
        );
    }
    let mut source_checks = BTreeMap::new();
    for q in qpoints {
        let r = by_q
            .get(&q.global_index)
            .context("data_invalid: 缺少物理 q 响应")?;
        native_ph::verify(r, q, geometry)?;
        native_ph::control(&r.control_metadata, qpoints)?;
        for p in [
            &r.dyn_file,
            &r.patterns,
            &r.control_metadata,
            r.dvscf.as_ref().unwrap(),
        ] {
            source_checks.insert(p.clone(), store::hash_file(p)?);
        }
    }
    let dest = work.join("qe-data/save");
    ensure!(
        !dest.exists(),
        "input_conflict: PH 交付目录已存在，保留原现场"
    );
    let phsave = dest.join(format!("{prefix}.phsave"));
    fs::create_dir_all(&phsave)?;
    let mut paths = BTreeMap::new();
    let mut output = vec![];
    let dyn0 = dest.join(format!("{prefix}.dyn0.xml"));
    fs::copy(&qpoints[0].source_dyn0, &dyn0)?;
    paths.insert("dyn0".to_string(), dyn0);
    let control = phsave.join("control_ph.xml");
    fs::copy(&responses[0].control_metadata, &control)?;
    paths.insert("phsave_control".into(), control.clone());
    for q in qpoints {
        let r = by_q[&q.global_index];
        let dyn_file = dest.join(format!("{prefix}.dyn_q{}.xml", q.global_index));
        let dvscf = dest.join(format!("{prefix}.dvscf_q{}", q.global_index));
        fs::copy(&r.dyn_file, &dyn_file)?;
        fs::copy(r.dvscf.as_ref().unwrap(), &dvscf)?;
        let patterns = phsave.join(format!("patterns.{}.xml", q.global_index));
        let original = fs::read_to_string(&r.patterns)?;
        let doc = roxmltree::Document::parse(&original)?;
        let q_node = doc
            .descendants()
            .find(|n| n.has_tag_name("QPOINT_NUMBER"))
            .context("data_invalid: patterns 缺内部 q")?;
        let range = q_node.range();
        // 只更改内部整数标签，保留每个局部计算的完整复扰动基底。
        let normalized = format!(
            "{}<QPOINT_NUMBER>{}</QPOINT_NUMBER>{}",
            &original[..range.start],
            q.global_index,
            &original[range.end..]
        );
        fs::write(&patterns, normalized)?;
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
    let mut artifacts = vec![];
    let mut unique = BTreeSet::new();
    for (role, p) in paths {
        ensure!(unique.insert(p.clone()), "data_invalid: PH 角色路径冲突");
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
            "input_conflict: PH 来源在归集期间变化"
        );
    }
    Ok(CollectedPh {
        responses: output,
        total_q: qpoints.len(),
        artifacts,
    })
}

/// 把 q 点映射落盘为可定位 JSON（FR-008 的 q_index_map.json）。
pub fn write_q_index_map(work: &Path, qpoints: &[QPoint]) -> Result<()> {
    let path = work.join("qe-data").join("q_index_map.json");
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
