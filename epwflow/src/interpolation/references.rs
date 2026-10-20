//! 固定独立 QE/PH 质量参考；复用 SCF 副本和原生尝试恢复，不新增执行入口。
use super::{
    attempt,
    native_quality::{QualitySources, SourceFile},
    quality_binding, reference_asr,
};
use crate::{
    foundation::{
        config::{Config, PhMode, Program},
        stage::Backend,
        store::{self, Identity},
    },
    qe_data::{
        self,
        manifest::{QPoint, QeManifest},
        qe_config::QeSettings,
        save,
        xml::Geometry,
    },
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: u32,
    digest: String,
    sources: Vec<Identity>,
}

/// 显式恢复允许修复编排程序；配置和五项科学来源仍必须完全一致。
fn resume_request(cfg: &Config, launcher: &[String], current: &[Identity], previous: &Request, retry: bool) -> Result<String> {
    ensure!(retry, "execution_failed: 独立参考已有现场，请显式 --resume");
    if crate::foundation::layout::enabled(&cfg.work_dir) {
        ensure!(retry && previous.schema_version==1 && previous.sources.len()==6 && current.len()==6,
            "input_conflict: 独立参考请求无效");
        let digest=crate::foundation::stages::request_digest(cfg,crate::foundation::stages::Step::Coarse,&previous.sources,"references")?;
        ensure!(digest==previous.digest && serde_json::to_value(&previous.sources[..5])?==serde_json::to_value(&current[..5])?,"input_conflict: 独立参考科学来源变化");
        for id in &previous.sources{id.verify()?;}
        return Ok(digest);
    }
    let (cfg, launcher) = crate::foundation::parallelism::request_identity(cfg, launcher)?;
    ensure!(previous.schema_version == 1 && previous.sources.len() == 6 && current.len() == 6
        && previous.digest == store::hash_bytes(&serde_json::to_vec(&(&cfg, &launcher, &previous.sources))?)
        && serde_json::to_value(&previous.sources[..5])? == serde_json::to_value(&current[..5])?,
        "input_conflict: 独立参考请求的固定输入或科学程序变化");
    for id in &previous.sources { id.verify()?; }
    Ok(previous.digest.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orchestrator_upgrade_requires_explicit_resume_and_unchanged_scientific_identity() {
        let root = std::env::temp_dir().join(format!("epw-reference-upgrade-{}", store::token()));
        fs::create_dir(&root).unwrap();
        let mut cfg: Config = toml::from_str(include_str!("../../tests/fixtures/input.toml")).unwrap();
        let mut old = Vec::new();
        for i in 0..6 {
            let path = root.join(i.to_string()); fs::write(&path, i.to_string()).unwrap();
            old.push(Identity::capture(&path).unwrap());
        }
        let digest = store::hash_bytes(&serde_json::to_vec(&(&cfg, &[] as &[String], &old)).unwrap());
        let previous = Request {schema_version: 1, digest: digest.clone(), sources: old.clone()};
        let replacement = root.join("new-driver"); fs::write(&replacement, "fixed renderer").unwrap();
        let mut current = old.clone(); current[5] = Identity::capture(&replacement).unwrap();
        assert_eq!(resume_request(&cfg, &[], &current, &previous, true).unwrap(), digest);
        assert!(resume_request(&cfg, &[], &current, &previous, false).is_err());
        cfg.wannier.num_wann += 1;
        assert!(resume_request(&cfg, &[], &current, &previous, true).is_err());
        cfg.wannier.num_wann -= 1;
        current[0] = current[5].clone();
        assert!(resume_request(&cfg, &[], &current, &previous, true).is_err());
        current[0] = old[0].clone();
        assert!(resume_request(&cfg, &["changed-launcher".into()], &current, &previous, true).is_err());
        fs::write(&old[5].path, "changed old driver").unwrap();
        assert!(resume_request(&cfg, &[], &current, &previous, true).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}

fn file(path: &Path) -> Result<SourceFile> {
    Ok(SourceFile {
        path: path.canonicalize()?,
        sha256: store::hash_file(path)?,
    })
}
fn cartesian(crystal: [f64; 3], geometry: &Geometry) -> [f64; 3] {
    std::array::from_fn(|j| (0..3).map(|i| crystal[i] * geometry.reciprocal[i][j]).sum())
}
fn evidence(dir: &Path) -> Result<Vec<Identity>> {
    save::files(dir)?
        .iter()
        .map(|f| Identity::capture(&dir.join(&f.relative_path)))
        .collect()
}

/// 已显式提供参考时不执行任何参考计算。自动来源必须在同一固定分配中同步执行。
pub(super) fn ensure_sources(
    cfg: &Config,
    backend: Backend,
    launcher: &[String],
    work: &Path,
    f2_work: &Path,
    retry: bool,
) -> Result<PathBuf> {
    if let Some(path) = &cfg.interpolation.quality.reference_manifest {
        return Ok(path.clone());
    }
    if let Some(path)=crate::foundation::workspace_import::reference(cfg)? {return Ok(path)}
    ensure!(
        matches!(backend, Backend::Local),
        "allocation: 自动参考必须在已登记分配内执行"
    );
    let f2_path = crate::foundation::layout::path(&f2_work, "qe-data/manifest.json");
    let f2 = QeManifest::load(&f2_path)?;
    f2.verify()?;
    save::verify_snapshot(f2_work)?;
    let dynmat = Program {
        path: cfg
            .programs
            .ph
            .path
            .parent()
            .context("configuration: PH 缺父目录")?
            .join("dynmat.x"),
        args: vec![],
    };
    ensure!(
        dynmat.path.is_file(),
        "data_invalid: 自动独立质量参考需要与 ph.x 同目录的 dynmat.x"
    );
    let mut sources = vec![
        Identity::capture(&f2_path)?,
        Identity::capture(&crate::foundation::layout::path(&f2_work, "scf/snapshot-files.json"))?,
        Identity::capture(&cfg.programs.pw.path)?,
        Identity::capture(&cfg.programs.ph.path)?,
        Identity::capture(&dynmat.path)?,
        Identity::capture(&std::env::current_exe()?)?,
    ];
    ensure!(
        f2.fingerprints.program_sha256.get("dynmat") == Some(&sources[4].sha256),
        "input_conflict: dynmat.x 未在初始运行中冻结；请使用新运行目录或显式独立参考"
    );
    let (identity_cfg, identity_launcher) = crate::foundation::parallelism::request_identity(cfg, launcher)?;
    let mut digest = if crate::foundation::layout::enabled(work) {
        crate::foundation::stages::request_digest(cfg,crate::foundation::stages::Step::Coarse,&sources,"references")?
    }else{store::hash_bytes(&serde_json::to_vec(&(&identity_cfg, &identity_launcher, &sources))?)};
    let root = crate::foundation::layout::path(&work, "quality-sources");
    let request_path = root.join("request.json");
    if request_path.exists() {
        let previous: Request = store::read(&request_path)?;
        digest = resume_request(cfg, launcher, &sources, &previous, retry)?;
        // 原请求保持不变；参考发布同时绑定原编排程序和当前执行程序。
        sources.push(previous.sources[5].clone());
    } else {
        ensure!(
            fs::symlink_metadata(&root).is_err(),
            "input_conflict: 独立参考目录缺请求，保留未知现场"
        );
        store::write(
            &request_path,
            &Request {
                schema_version: 1,
                digest: digest.clone(),
                sources: sources.clone(),
            },
        )?;
    }
    let qe = QeSettings::from_config(cfg)?;
    let geometry = qe_data::phonon::geometry(&qe, f2_work)?;
    geometry.validate(&qe)?;
    let qcfg = &cfg.interpolation.quality;
    ensure!(
        qcfg.k_points_crystal.len() >= 6 && qcfg.q_points_crystal.len() >= 4,
        "configuration: 独立质量参考至少需要六个 k 点和四个 q 点"
    );
    let electron_dir = root.join("electron");
    let input = electron_dir.join("reference.in");
    let xml = electron_dir.join(format!("out/{}.save/data-file-schema.xml", cfg.qe.prefix));
    let immutable = [input.clone(), crate::foundation::layout::path(&f2_work, "scf/snapshot-files.json")];
    let electron = attempt::run_with_inputs(
        work,
        "reference-electron",
        &electron_dir,
        store::hash_bytes(format!("{digest}:electron").as_bytes()),
        retry,
        "reference.exitcode",
        Some(&immutable),
        || {
            fs::create_dir_all(&electron_dir)?;
            save::stage_scf(f2_work, &electron_dir.join("out"), &cfg.qe.prefix)?;
            fs::write(
                &input,
                qe.reference_input(
                    electron_dir
                        .join("out")
                        .to_str()
                        .context("目录不是 UTF-8")?,
                    &qcfg.k_points_crystal,
                )?,
            )?;
            Ok(())
        },
        || {
            qe_data::execution::run(
                &cfg.programs.pw,
                &input,
                &electron_dir,
                launcher,
                backend,
                "reference",
                false,
            )
        },
        || {
            qf_io::qe_completion::checked_log(&electron_dir.join("reference.out"), false)?;
            save::verify_snapshot(f2_work)?;
            let actual_geo =
                qe_data::xml::validate_save(&electron_dir.join("out"), &cfg.qe.prefix, &qe, None)?;
            let states = qe_data::nscf::parse_xml(&xml)?;
            ensure!(
                states.nbnd == cfg.qe.nscf.nbnd as usize
                    && states.kpoints.len() == qcfg.k_points_crystal.len(),
                "data_invalid: 独立 NSCF 点数或带数不符"
            );
            for (p, expected) in states.kpoints.iter().zip(&qcfg.k_points_crystal) {
                ensure!(
                    actual_geo
                        .crystal(*p)
                        .iter()
                        .zip(expected)
                        .all(|(a, b)| (a - b).abs() < 1e-7),
                    "input_conflict: 独立 NSCF 坐标或顺序不符"
                );
            }
            Ok(xml.clone())
        },
        |_| evidence(&electron_dir),
    )?;
    sources.extend(evidence(&electron_dir)?);
    let gamma_index = f2
        .qpoints
        .iter()
        .position(|q| q.crystal.iter().all(|v| v.abs() < 1e-9))
        .context("data_invalid: F2 缺少同源 Γ 动力学矩阵")?;
    let gamma_id = f2.qpoints[gamma_index].global_index;
    let gamma = &f2
        .phonon_files
        .iter()
        .find(|r| r.q_id == gamma_id)
        .context("data_invalid: Γ 响应映射缺失")?
        .dyn_file;
    let gamma_text = fs::read_to_string(gamma)?;
    sources.push(Identity::capture(gamma)?);
    // PH 响应复用既有严格原生验证；局部编号固定为 1。
    let mut reference_cfg = cfg.clone();
    reference_cfg.phonon.mode = PhMode::Coords;
    let reference_qe = QeSettings::from_config(&reference_cfg)?;
    let mut frequency_text = format!(
        "&plot nbnd= {}, nks= {} /\n",
        geometry.atoms.len() * 3,
        qcfg.q_points_crystal.len()
    );
    for (index, crystal) in qcfg.q_points_crystal.iter().enumerate() {
        let q = QPoint {
            global_index: index + 1,
            local_index: 1,
            directory: format!("reference-q-{:04}", index + 1),
            crystal: *crystal,
            cartesian: cartesian(*crystal, &geometry),
            alat: geometry.alat,
            reciprocal_basis: geometry.reciprocal,
            source_dyn0: f2.qpoints[0].source_dyn0.clone(),
        };
        let original = f2.qpoints.iter().position(|p| {
            p.crystal
                .iter()
                .zip(crystal)
                .all(|(a, b)| (a - b).abs() < 1e-9)
        });
        let raw = if let Some(i) = original {
            f2.phonon_files
                .iter()
                .find(|r| r.q_id == f2.qpoints[i].global_index)
                .context("data_invalid: 固定 q 响应映射缺失")?
                .dyn_file
                .clone()
        } else {
            let label = format!("reference-ph-{:04}", index + 1);
            let dir = root.join(format!("ph-{:04}", index + 1));
            let input = dir.join("reference.in");
            let immutable = [input.clone(), crate::foundation::layout::path(&f2_work, "scf/snapshot-files.json")];
            let response = attempt::run_with_inputs(
                work,
                &label,
                &dir,
                store::hash_bytes(format!("{digest}:{label}").as_bytes()),
                retry,
                "reference.exitcode",
                Some(&immutable),
                || {
                    fs::create_dir_all(&dir)?;
                    save::stage_scf(f2_work, &dir.join("out"), &cfg.qe.prefix)?;
                    fs::write(
                        &input,
                        reference_qe.phonon_q_input(
                            dir.join("out").to_str().context("目录不是 UTF-8")?,
                            &PhMode::Coords,
                            Some(&q),
                            q.global_index,
                        )?,
                    )?;
                    Ok(())
                },
                || {
                    qe_data::execution::run(
                        &cfg.programs.ph,
                        &input,
                        &dir,
                        launcher,
                        backend,
                        "reference",
                        false,
                    )
                },
                || {
                    qf_io::qe_completion::checked_log(&dir.join("reference.out"), false)?;
                    save::verify_snapshot(f2_work)?;
                    qe_data::phonon::response(
                        &reference_qe,
                        &dir,
                        &q,
                        &dir.join(format!("out/_ph0/{}.phsave/control_ph.xml", cfg.qe.prefix)),
                        &geometry,
                    )
                },
                |_| evidence(&dir),
            )?;
            sources.extend(evidence(&dir)?);
            response.dyn_file
        };
        sources.push(Identity::capture(&raw)?);
        let corrected = reference_asr::corrected_xml(
            &fs::read_to_string(&raw)?,
            &gamma_text,
            geometry.atoms.len(),
        )?;
        let native_xml = reference_asr::qe_xml_records(&corrected)?;
        let label = format!("reference-dynmat-{:04}", index + 1);
        let dir = root.join(format!("dynmat-{:04}", index + 1));
        let input = dir.join("reference.in");
        let dyn_xml = dir.join("corrected.xml");
        let immutable = [input.clone(), dyn_xml.clone()];
        let frequencies = attempt::run_with_inputs(
            work,
            &label,
            &dir,
            store::hash_bytes(
                format!(
                    "{digest}:{label}:{}",
                    store::hash_bytes(corrected.as_bytes())
                )
                .as_bytes(),
            ),
            retry,
            "reference.exitcode",
            Some(&immutable),
            || {
                fs::create_dir_all(&dir)?;
                fs::write(&dyn_xml, &native_xml)?;
                let mut text =
                    String::from("&input\n fildyn='corrected.xml', asr='no', filout='modes.out'\n");
                for (i, el) in qe.cell.elements.iter().enumerate() {
                    text.push_str(&format!(
                        " amass({})={:.17e}\n",
                        i + 1,
                        qf_input::species::get_atomic_mass(el, cfg.atomic_masses.get(el).copied())
                    ));
                }
                text.push_str("/\n");
                fs::write(&input, text)?;
                Ok(())
            },
            || {
                qe_data::execution::run(
                    &dynmat,
                    &input,
                    &dir,
                    launcher,
                    backend,
                    "reference",
                    false,
                )
            },
            || {
                qf_io::qe_completion::checked_log(&dir.join("reference.out"), false)?;
                reference_asr::frequencies(
                    &fs::read_to_string(dir.join("modes.out"))?,
                    q.cartesian,
                    geometry.atoms.len() * 3,
                )
            },
            |_| evidence(&dir),
        )?;
        sources.extend(evidence(&dir)?);
        frequency_text.push_str(&format!(
            "{:.17e} {:.17e} {:.17e}\n{}\n",
            q.cartesian[0],
            q.cartesian[1],
            q.cartesian[2],
            frequencies
                .iter()
                .map(|v| format!("{v:.17e}"))
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    f2.verify()?;
    save::verify_snapshot(f2_work)?;
    for source in &sources {
        source.verify()?;
    }
    let phonon = root.join("reference.freq");
    // 派生表发布可中断恢复，未知内容不能覆盖。
    if let Ok(meta) = fs::symlink_metadata(&phonon) {
        ensure!(
            meta.is_file() && !meta.file_type().is_symlink(),
            "input_conflict: 独立声子参考表必须为普通文件"
        );
        ensure!(
            fs::read_to_string(&phonon)? == frequency_text,
            "input_conflict: 独立声子参考表变化"
        );
    } else {
        store::atomic_bytes(&phonon, frequency_text.as_bytes())?;
    }
    let fermi_xml = f2
        .states
        .as_ref()
        .context("data_invalid: F2 缺电子态")?
        .wavefunctions
        .join("data-file-schema.xml");
    let fermi = qe_data::nscf::parse_xml(&fermi_xml)?.fermi;
    let placeholder = || SourceFile {
        path: "unpaired".into(),
        sha256: String::new(),
    };
    sources.push(Identity::capture(&request_path)?);
    sources.sort_by(|a, b| a.path.cmp(&b.path));
    sources.dedup_by(|a, b| a.path == b.path && a.sha256 == b.sha256);
    let reference = QualitySources {
        schema_version: 1,
        asr: "epw_simple_gamma_onsite".into(),
        window_ev: qcfg.energy_window_ev.map(|v| v + fermi),
        fermi_reference: file(&fermi_xml)?,
        electron_reference: file(&electron)?,
        phonon_reference_cm1: file(&phonon)?,
        electron_interpolated: placeholder(),
        phonon_interpolated_mev: placeholder(),
        provenance: sources
            .iter()
            .map(|id| SourceFile {
                path: id.path.clone(),
                sha256: id.sha256.clone(),
            })
            .collect(),
    };
    let path = root.join("reference.json");
    quality_binding::write_matching(&path, &reference)?;
    // 生成器和外部清单使用同一严格坐标／共同 Fermi／ASR 校验。
    quality_binding::freeze_at(cfg, fermi, &path)?.verify()?;
    Ok(path)
}
