//! 从编号计算目录读取 QE 产物；清单只在内存中组织，不另存状态目录。
use super::{manifest::*, native_ph, nscf, qe_config::QeSettings, qmap, save, scf};
use crate::foundation::{config::Config, layout, stages::Step, store};
use anyhow::{Context, Result};
use std::collections::BTreeMap;

pub fn load(cfg: &Config) -> Result<QeManifest> {
    if crate::foundation::phonon_import::bound(cfg) {
        anyhow::ensure!(
            crate::foundation::phonon_import::binding(cfg).is_file(),
            "data_invalid: external phonon source record missing; specify --import-phonon again; PH will not be rerun"
        );
    }
    let root = &cfg.work_dir;
    let qe = QeSettings::from_config(cfg)?;
    let geometry = super::phonon::geometry(&qe, root)?;
    geometry.validate(&qe)?;
    let dir = Step::Phonon.directory(cfg).join("save");
    let prefix = &cfg.qe.prefix;
    let dyn0 = dir.join(format!("{prefix}.dyn0.xml"));
    let qpoints = qmap::parse_dyn0(&dyn0, &geometry, cfg.phonon.q_grid, false)?;
    let control = dir.join(format!("{prefix}.phsave/control_ph.xml"));
    native_ph::control(&control, &qpoints)?;
    let mut artifacts = Vec::new();
    let mut add = |role: String, path: std::path::PathBuf, stage: &str| -> Result<()> {
        artifacts.push(Artifact {
            role,
            relative_path: path.strip_prefix(root)?.into(),
            byte_size: std::fs::metadata(&path)?.len(),
            sha256: store::hash_file(&path)?,
            producer_stage: stage.into(),
            input_digest: String::new(),
            format_version: native_ph::FORMAT.into(),
        });
        Ok(())
    };
    add("dyn0".into(), dyn0, "phonon")?;
    add("phsave_control".into(), control.clone(), "phonon")?;
    let mut responses = Vec::new();
    for q in &qpoints {
        let r = PhResponse {
            q_id: q.global_index,
            dyn_file: dir.join(format!("{prefix}.dyn_q{}.xml", q.global_index)),
            patterns: dir.join(format!("{prefix}.phsave/patterns.{}.xml", q.global_index)),
            dvscf: Some(dir.join(format!("{prefix}.dvscf_q{}", q.global_index))),
            control_metadata: control.clone(),
            format: native_ph::FORMAT.into(),
            perturbations: geometry.atoms.len() * 3,
        };
        native_ph::verify(&r, q, &geometry)?;
        for (role, path) in [
            ("dyn", &r.dyn_file),
            ("patterns", &r.patterns),
            ("dvscf", r.dvscf.as_ref().unwrap()),
        ] {
            add(
                format!("{role}_q{}", q.global_index),
                path.clone(),
                "phonon",
            )?;
        }
        responses.push(r);
    }
    let snapshot = layout::path(root, "scf/snapshot");
    for (stage, dir) in [
        ("scf", snapshot.clone()),
        ("nscf", layout::path(root, "nscf/out")),
    ] {
        for file in save::files(&dir)? {
            add(
                format!("{stage}_{}", file.relative_path.display()),
                dir.join(file.relative_path),
                stage,
            )?;
        }
    }
    add(
        "scf_snapshot_identity".into(),
        if crate::foundation::layout::enabled(root) { crate::foundation::layout::path(root,"scf/snapshot").join(format!("{}.save/data-file-schema.xml",cfg.qe.prefix)) } else { layout::path(root, "scf/snapshot-files.json") },
        "scf",
    )?;
    artifacts.push(nscf_input_artifact(root)?);
    let mut programs = BTreeMap::new();
    for (name, p) in [
        ("pw", &cfg.programs.pw.path),
        ("ph", &cfg.programs.ph.path),
        ("epw", &cfg.programs.epw.path),
    ] {
        programs.insert(name.into(), store::hash_file(p)?);
    }
    for name in ["q2r", "matdyn"] {
        let p = &cfg.programs.get(name)?.path;
        programs.insert(name.into(), store::hash_file(&p)?);
    }
    let result = QeManifest {
        schema_version: 2,
        artifact_root: Some(root.canonicalize()?),
        artifacts,
        run_id: crate::foundation::readable_state::read(root, Step::Phonon)?
            .get("Stage", "Batch")
            .unwrap_or("Existing phonon results")
            .into(),
        fingerprints: Fingerprints {
            structure_sha256: store::hash_file(&cfg.structure)?,
            pseudo_sha256: cfg
                .pseudos
                .iter()
                .map(|(e, p)| Ok((e.clone(), store::hash_file(p)?)))
                .collect::<Result<_>>()?,
            program_sha256: programs,
        },
        scf: Some(ScfRecord {
            total_energy_ry: scf::parse_scf_energy(&std::fs::read_to_string(layout::path(
                root,
                "scf/scf.out",
            ))?)?,
            attempt_count: 1,
            success_count: 1,
            snapshot_dir: snapshot,
        }),
        states: Some(nscf::verified_states(cfg, root)?),
        qpoints,
        phonon_files: responses,
        validation: Validation {
            status: "valid".into(),
            checks: vec![Check {
                name: "Direct validation of calculation directory".into(),
                passed: true,
                detail: "Validate structure, NSCF grid, PH response potentials and perturbation modes individually".into(),
            }],
            units: BTreeMap::new(),
            thresholds: BTreeMap::new(),
            evidence: vec![],
        },
    };
    result.verify()?;
    Ok(result)
}

pub fn workspace(path: &std::path::Path) -> Option<std::path::PathBuf> {
    path.ancestors()
        .find(|p| layout::enabled(p))
        .map(std::path::Path::to_path_buf)
}
pub fn current(root: &std::path::Path) -> Result<Config> {
    crate::foundation::readable_state::load_config(
        &crate::foundation::readable_state::current_config(root)
            .context("configuration: Please use the current configuration to initialize phase 8 logs")?,
    )
}

/// 仅声子本身的原生交付，不读取 NSCF、电子图或文件身份清单。
pub struct PhononData {pub phonon_files:Vec<PhResponse>}
pub fn phonon(cfg:&Config)->Result<PhononData> {
    let qe=QeSettings::from_config(cfg)?;
    let geometry=super::phonon::geometry(&qe,&cfg.work_dir)?;geometry.validate(&qe)?;
    let dir=Step::Phonon.directory(cfg).join("save");let prefix=&cfg.qe.prefix;
    let points=qmap::parse_dyn0(&dir.join(format!("{prefix}.dyn0.xml")),&geometry,cfg.phonon.q_grid,false)?;
    let control=dir.join(format!("{prefix}.phsave/control_ph.xml"));native_ph::control(&control,&points)?;
    let mut phonon_files=Vec::new();
    for q in &points {
        let response=PhResponse{q_id:q.global_index,dyn_file:dir.join(format!("{prefix}.dyn_q{}.xml",q.global_index)),
            patterns:dir.join(format!("{prefix}.phsave/patterns.{}.xml",q.global_index)),
            dvscf:Some(dir.join(format!("{prefix}.dvscf_q{}",q.global_index))),control_metadata:control.clone(),
            format:native_ph::FORMAT.into(),perturbations:geometry.atoms.len()*3};
        native_ph::verify(&response,q,&geometry)?;phonon_files.push(response);
    }
    Ok(PhononData{phonon_files})
}
