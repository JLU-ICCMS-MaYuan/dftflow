//! 第二阶段的固定路径和总量 DOS；科学执行仍在原分配内，绘图复用 F5。
use super::{
    attempt, comparison, comparison_data,
    epw_config::{EpwConfig, NativeStage},
    native,
    native_quality::SourceFile,
    reference_asr, restart,
};
use crate::{
    foundation::{
        config::{Config, Program},
        stage::Backend,
        store::{self, Identity},
    },
    qe_data::{self, manifest::QeManifest, qe_config::QeSettings, save, xml::Geometry},
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: u32,
    digest: String,
    sources: Vec<Identity>,
}

fn resume_request(cfg: &Config, current: &[Identity], previous: &Request, digest: &str) -> Result<String> {
    ensure!(previous.schema_version == 1, "data_invalid: 比较请求版本错误");
    if !crate::foundation::layout::enabled(&cfg.work_dir) {
        ensure!(previous.digest == digest, "input_conflict: 比较固定请求已变化");
        for id in &previous.sources { id.verify()?; }
        return Ok(digest.into());
    }
    ensure!(previous.sources.len() == current.len() && current.len() >= 9,
        "input_conflict: 比较请求来源布局变化");
    ensure!(previous.digest == crate::foundation::stages::request_digest(
        cfg, crate::foundation::stages::Step::Coarse, &previous.sources, "comparison")?,
        "input_conflict: 比较科学参数变化");
    for (index, (old, new)) in previous.sources.iter().zip(current).enumerate() {
        if index == 6 { continue; } // 原编排器由旧请求追溯，当前版本由本次冻结提交追溯。
        // 每次提交会物化新的 KPATH 快照；相同角色允许路径变化，但字节身份必须一致。
        ensure!(old.sha256 == new.sha256 && old.bytes == new.bytes,
            "input_conflict: 比较科学来源变化：{}", old.path.display());
        old.verify()?;
        new.verify()?;
    }
    Ok(previous.digest.clone())
}

fn file(path: &Path) -> Result<SourceFile> {
    Ok(SourceFile {
        path: path.canonicalize()?,
        sha256: store::hash_file(path)?,
    })
}
fn evidence(dir: &Path) -> Result<Vec<Identity>> {
    save::files(dir)?
        .iter()
        .map(|f| Identity::capture(&dir.join(&f.relative_path)))
        .collect()
}
fn cart(point: [f64; 3], reciprocal: &[[f64; 3]; 3]) -> [f64; 3] {
    std::array::from_fn(|j| (0..3).map(|i| point[i] * reciprocal[i][j]).sum())
}

fn path_points(
    path: &Path,
    reciprocal: &[[f64; 3]; 3],
    intervals: usize,
) -> Result<(Vec<comparison::PathPoint>, Vec<comparison_data::Tick>)> {
    ensure!(
        intervals > 0 && reciprocal.iter().flatten().all(|x| x.is_finite()),
        "configuration: 比较路径参数无效"
    );
    let segments =
        qf_input::electronic::parse_kpath_segments(path.to_str().context("路径不是 UTF-8")?)?;
    let mut points: Vec<comparison::PathPoint> = vec![];
    let mut ticks = vec![];
    let mut distance = 0.0;
    for (segment_id, segment) in segments.iter().enumerate() {
        for (edge, pair) in segment.windows(2).enumerate() {
            let (a, b) = (&pair[0], &pair[1]);
            let delta = cart(std::array::from_fn(|j| b.0[j] - a.0[j]), reciprocal);
            let length = delta.iter().map(|x| x * x).sum::<f64>().sqrt();
            ensure!(
                length.is_finite() && length > 1e-12,
                "data_invalid: 路径含零长度或无效边"
            );
            if edge == 0 {
                // 断点保留独立段；留出可见空隙，但绝不跨段连曲线。
                if !points.is_empty() {
                    distance += length / intervals as f64;
                }
                points.push(comparison::PathPoint {
                    segment_id,
                    point_id: points.len(),
                    distance,
                    crystal: a.0,
                });
                ticks.push(comparison_data::Tick {
                    distance,
                    label: a.1.clone(),
                });
            }
            let start = distance;
            for i in 1..=intervals {
                distance = start + length * i as f64 / intervals as f64;
                points.push(comparison::PathPoint {
                    segment_id,
                    point_id: points.len(),
                    distance,
                    crystal: std::array::from_fn(|j| {
                        a.0[j] + (b.0[j] - a.0[j]) * i as f64 / intervals as f64
                    }),
                });
            }
            ticks.push(comparison_data::Tick {
                distance,
                label: b.1.clone(),
            });
        }
    }
    ensure!(points.len() >= 2, "data_invalid: 比较路径为空");
    Ok((points, ticks))
}

fn check_points(
    values: &native::NativeBands,
    points: &[[f64; 3]],
    geometry: &Geometry,
    modes: usize,
) -> Result<()> {
    ensure!(
        values.values.len() == points.len() && values.values.iter().all(|v| v.len() == modes),
        "data_invalid: 比较点／带／模式数不同"
    );
    for (p, q) in points.iter().zip(&values.points_cartesian) {
        ensure!(
            cart(*p, &geometry.reciprocal)
                .iter()
                .zip(q)
                .all(|(a, b)| (a - b).abs() < 1e-6),
            "input_conflict: 比较原生输出坐标或点序不同"
        );
    }
    Ok(())
}

/// KPATH 在公开第一阶段已冻结。旧无路径内部调用不改变粗网格接口。
pub(crate) fn run(
    cfg: &Config,
    epw: &EpwConfig,
    backend: Backend,
    launcher: &[String],
    work: &Path,
    f2_work: &Path,
    retry: bool,
) -> Result<()> {
    let Some(kpath) = &cfg.kpath else {
        return Ok(());
    };
    ensure!(
        backend == Backend::Local,
        "allocation: 自动比较必须在已登记分配内同步执行"
    );
    let f2_path = crate::foundation::layout::path(&f2_work, "qe-data/manifest.json");
    let f2 = QeManifest::load(&f2_path)?;
    f2.verify()?;
    save::verify_snapshot(f2_work)?;
    let qe = QeSettings::from_config(cfg)?;
    let geometry = qe_data::phonon::geometry(&qe, f2_work)?;
    geometry.validate(&qe)?;
    let options = cfg.comparison.clone().unwrap_or_default();
    let (path, ticks) = path_points(
        kpath,
        &geometry.reciprocal,
        options.points_per_edge as usize,
    )?;
    let states = f2.states.as_ref().context("data_invalid: 缺 F2 均匀网格")?;
    let mesh: Vec<_> = states
        .kpoints
        .iter()
        .map(|p| geometry.crystal(*p))
        .collect();
    // 完整网格的原生浮点误差规范到已核验的整数网格键，不能改变点序。
    let mesh: Vec<[f64; 3]> = mesh
        .iter()
        .map(|p| {
            std::array::from_fn(|i| {
                (p[i] * states.k_grid[i] as f64).round() / states.k_grid[i] as f64
            })
        })
        .collect();
    let path_crystal: Vec<_> = path.iter().map(|p| p.crystal).collect();
    let all: Vec<_> = path_crystal.iter().chain(&mesh).copied().collect();
    let post_dir = cfg
        .programs
        .ph
        .path
        .parent()
        .context("configuration: PH 缺父目录")?;
    let q2r = Program {
        path: post_dir.join("q2r.x"),
        args: vec![],
    };
    let matdyn = Program {
        path: post_dir.join("matdyn.x"),
        args: vec![],
    };
    for (role, program) in [("q2r", &q2r), ("matdyn", &matdyn)] {
        ensure!(
            f2.fingerprints.program_sha256.get(role) == Some(&store::hash_file(&program.path)?),
            "input_conflict: 自动比较的 {role}.x 未在 F1 冻结或已改变"
        );
    }
    let coarse_dir = crate::foundation::layout::path(&work, "coarse");
    let modes = geometry.atoms.len() * 3;
    let nwann = cfg.wannier.num_wann as usize;
    let mut identities = vec![
        Identity::capture(&f2_path)?,
        Identity::capture(kpath)?,
        Identity::capture(&q2r.path)?,
        Identity::capture(&matdyn.path)?,
        Identity::capture(&cfg.programs.pw.path)?,
        Identity::capture(&cfg.programs.epw.path)?,
        Identity::capture(&std::env::current_exe()?)?,
        Identity::capture(&crate::foundation::layout::path(&work, "quality-reference.json"))?,
        Identity::capture(&coarse_dir.join(format!("{}.wout", cfg.qe.prefix)))?,
    ];
    for (_, p) in restart::files(
        &coarse_dir,
        &coarse_dir.join("out"),
        &cfg.qe.prefix,
        nwann,
        modes,
    )? {
        identities.push(Identity::capture(&p)?);
    }
    let (identity_cfg, identity_launcher) = crate::foundation::parallelism::request_identity(cfg, launcher)?;
    let mut digest = if crate::foundation::layout::enabled(work) {
        crate::foundation::stages::request_digest(cfg,crate::foundation::stages::Step::Coarse,&identities,"comparison")?
    }else{store::hash_bytes(&serde_json::to_vec(&(&identity_cfg, &identity_launcher, &identities))?)};
    let root = crate::foundation::layout::path(&work, "comparison-native");
    let request_path = root.join("request.json");
    if request_path.exists() {
        let previous: Request = store::read(&request_path)?;
        ensure!(retry, "execution_failed: 比较已有现场，请显式 --resume");
        digest = resume_request(cfg, &identities, &previous, &digest)?;
    } else {
        ensure!(
            fs::symlink_metadata(&root).is_err(),
            "input_conflict: 比较目录缺请求，保留未知现场"
        );
        store::write(
            &request_path,
            &Request {
                schema_version: 1,
                digest: digest.clone(),
                sources: identities,
            },
        )?;
    }
    let electron = root.join("electron");
    let electron_input = electron.join("reference.in");
    let electron_xml = electron.join(format!("out/{}.save/data-file-schema.xml", cfg.qe.prefix));
    let inputs = [
        electron_input.clone(),
        crate::foundation::layout::path(&f2_work, "scf/snapshot-files.json"),
    ];
    attempt::run_with_inputs(
        work,
        "comparison-electron",
        &electron,
        store::hash_bytes(format!("{digest}:electron").as_bytes()),
        retry,
        "reference.exitcode",
        Some(&inputs),
        || {
            fs::create_dir_all(&electron)?;
            save::stage_scf(f2_work, &electron.join("out"), &cfg.qe.prefix)?;
            fs::write(
                &electron_input,
                qe.reference_input(
                    electron.join("out").to_str().context("路径不是 UTF-8")?,
                    &path_crystal,
                )?,
            )?;
            Ok(())
        },
        || {
            qe_data::execution::run(
                &cfg.programs.pw,
                &electron_input,
                &electron,
                launcher,
                backend,
                "reference",
                false,
            )
        },
        || {
            qe_data::xml::validate_save(&electron.join("out"), &cfg.qe.prefix, &qe, None)?;
            let data = qe_data::nscf::parse_xml(&electron_xml)?;
            ensure!(
                data.nbnd == states.nbnd && data.kpoints.len() == path.len(),
                "data_invalid: 比较 NSCF 点／带数不同"
            );
            for (expected, actual) in path_crystal.iter().zip(&data.kpoints) {
                ensure!(
                    cart(*expected, &geometry.reciprocal)
                        .iter()
                        .zip(actual)
                        .all(|(a, b)| (a - b).abs() < 1e-7),
                    "input_conflict: 比较 NSCF 坐标或顺序不同"
                );
            }
            Ok(())
        },
        |_| evidence(&electron),
    )?;

    let phonon = root.join("phonon");
    let gamma_q = f2
        .qpoints
        .iter()
        .find(|q| q.crystal.iter().all(|x| x.abs() < 1e-9))
        .context("data_invalid: 缺 Γ")?;
    let gamma = &f2
        .phonon_files
        .iter()
        .find(|p| p.q_id == gamma_q.global_index)
        .context("data_invalid: 缺 Γ 映射")?
        .dyn_file;
    attempt::run_with_inputs(
        work,
        "comparison-phonon",
        &phonon,
        store::hash_bytes(format!("{digest}:phonon").as_bytes()),
        retry,
        "matdyn.exitcode",
        None,
        || {
            fs::create_dir_all(&phonon)?;
            fs::copy(
                &gamma_q.source_dyn0,
                phonon.join(format!("{}.dyn0", cfg.qe.prefix)),
            )?;
            let gamma_text = fs::read_to_string(gamma)?;
            for p in &f2.phonon_files {
                fs::write(
                    phonon.join(format!("{}.dyn{}.xml", cfg.qe.prefix, p.q_id)),
                    reference_asr::qe_xml_records(&reference_asr::corrected_stars(
                        &fs::read_to_string(&p.dyn_file)?,
                        &gamma_text,
                        geometry.atoms.len(),
                    )?)?,
                )?;
            }
            fs::write(
                phonon.join("q2r.in"),
                format!(
                    "&input\n fildyn='{}.dyn.xml'\n flfrc='reference.fc'\n zasr='no'\n/\n",
                    cfg.qe.prefix
                ),
            )?;
            let mut input = String::from("&input\n flfrc='reference.fc.xml'\n flfrq='reference.freq'\n asr='no'\n q_in_cryst_coord=.false.\n q_in_band_form=.false.\n");
            for (i, el) in qe.cell.elements.iter().enumerate() {
                let mass =
                    qf_input::species::get_atomic_mass(el, cfg.atomic_masses.get(el).copied());
                input.push_str(&format!(" amass({})={mass:.17e}\n", i + 1));
            }
            input.push_str(&format!("/\n{}\n", all.len()));
            for p in &all {
                let p = cart(*p, &geometry.reciprocal);
                input.push_str(&format!("{:.17e} {:.17e} {:.17e}\n", p[0], p[1], p[2]));
            }
            fs::write(phonon.join("matdyn.in"), input)?;
            Ok(())
        },
        || {
            qe_data::execution::run(
                &q2r,
                &phonon.join("q2r.in"),
                &phonon,
                launcher,
                backend,
                "q2r",
                false,
            )?;
            // QE write_dyn_mat_header 总在 flfrc 基名后追加 .xml。
            save::nonempty(&phonon.join("reference.fc.xml"))?;
            qe_data::execution::run(
                &matdyn,
                &phonon.join("matdyn.in"),
                &phonon,
                launcher,
                backend,
                "matdyn",
                false,
            )
        },
        || {
            check_points(
                &native::parse_matdyn_path(&fs::read_to_string(phonon.join("reference.freq"))?)?,
                &all,
                &geometry,
                modes,
            )
        },
        |_| evidence(&phonon),
    )?;

    let epw_dir = root.join("epw");
    attempt::run(
        work,
        "comparison-epw",
        &epw_dir,
        store::hash_bytes(format!("{digest}:epw").as_bytes()),
        retry,
        || {
            restart::stage(
                &coarse_dir,
                &coarse_dir.join("out"),
                &cfg.qe.prefix,
                nwann,
                modes,
                &epw_dir,
            )?;
            EpwConfig::write_points(&epw_dir, &all, &all)?;
            fs::write(
                epw_dir.join("epw.in"),
                epw.native_input(
                    NativeStage::RestartQuality,
                    "./out",
                    "./save",
                    cfg.interpolation.phonon_smearings_mev[0],
                )?,
            )?;
            Ok(())
        },
        || super::coarse::run_epw(epw, &epw_dir, "epw.in", launcher, backend),
        || {
            native::checked_log(&fs::read_to_string(epw_dir.join("epw.out"))?, 0)?;
            check_points(
                &native::parse_bands(&fs::read_to_string(epw_dir.join("band.eig"))?)?,
                &all,
                &geometry,
                nwann,
            )?;
            check_points(
                &native::parse_bands(&fs::read_to_string(epw_dir.join("phband.freq"))?)?,
                &all,
                &geometry,
                modes,
            )
        },
        |_| evidence(&epw_dir),
    )?;
    f2.verify()?;
    let request: Request = store::read(&request_path)?;
    for (index, id) in request.sources.iter().enumerate() {
        if index == 6 && crate::foundation::layout::enabled(work) { continue; }
        id.verify()?;
    }
    let mut provenance = vec![file(&request_path)?];
    for dir in [&electron, &phonon, &epw_dir] {
        provenance.extend(
            evidence(dir)?
                .iter()
                .map(|id| file(&id.path))
                .collect::<Result<Vec<_>>>()?,
        );
    }
    let sources = comparison::Sources {
        schema_version: 1,
        run_id: f2.run_id,
        ph_mode: format!("{:?}", cfg.phonon.mode).to_lowercase(),
        quality_reference: file(&crate::foundation::layout::path(&work, "quality-reference.json"))?,
        fermi_reference: file(&states.wavefunctions.join("data-file-schema.xml"))?,
        qe_bands_xml: file(&electron_xml)?,
        qe_mesh_xml: file(&states.wavefunctions.join("data-file-schema.xml"))?,
        epw_bands: file(&epw_dir.join("band.eig"))?,
        qe_phonon_bands: file(&phonon.join("reference.freq"))?,
        epw_phonon_bands: file(&epw_dir.join("phband.freq"))?,
        epw_log: file(&epw_dir.join("epw.out"))?,
        provenance,
        path,
        ticks,
        mesh,
        mesh_shape: states.k_grid.map(|v| v as usize),
        reciprocal: geometry.reciprocal,
        window_ev: cfg.interpolation.quality.energy_window_ev,
        wannier_wout: Some(file(&coarse_dir.join(format!("{}.wout", cfg.qe.prefix)))?),
        electronic_sigma_ev: options.electronic_sigma_ev,
        phonon_sigma_cm1: options.phonon_sigma_cm1,
    };
    let source_path = root.join("sources.json");
    if source_path.exists() {
        let previous: comparison::Sources = store::read(&source_path)?;
        ensure!(
            serde_json::to_value(&previous)? == serde_json::to_value(&sources)?,
            "input_conflict: 比较来源清单变化"
        );
    } else {
        store::write(&source_path, &sources)?;
    }
    let output = crate::foundation::layout::path(&work, "interpolation/comparison");
    if output.exists() {
        let data = comparison_data::load(&output)?;
        ensure!(
            data.manifest.upstream_digest == store::hash_file(&source_path)?,
            "input_conflict: 比较包绑定其他来源"
        );
    } else {
        fs::create_dir_all(output.parent().unwrap())?;
        comparison::export(&source_path, &output)?;
    }
    let results = crate::delivery::comparison::collect(work, None)?;
    let pending = results
        .join("figures/.comparison-figures.json.publish.json")
        .exists();
    if pending
        || !crate::delivery::comparison_plot::check_figure_formats(
            &results,
            &["png", "svg", "pdf"],
        )?
    {
        crate::delivery::comparison_plot::plot(
            &results,
            None,
            &["png".into(), "svg".into(), "pdf".into()],
        )?;
    }
    ensure!(
        crate::delivery::comparison_plot::check_figure_formats(&results, &["png", "svg", "pdf"])?,
        "data_invalid: 比较图表未完整发布"
    );
    Ok(())
}

pub(super) fn artifacts(work: &Path) -> Result<Vec<super::manifest::Artifact>> {
    let mut paths = vec![
        crate::foundation::layout::path(&work, "comparison-native/request.json"),
        crate::foundation::layout::path(&work, "comparison-native/sources.json"),
    ];
    for dir in [
        "interpolation/comparison",
        "results/comparison",
        "results/figures",
    ] {
        paths.extend(
            save::files(&crate::foundation::layout::path(work,dir))?
                .iter()
                .map(|f| crate::foundation::layout::path(work,dir).join(&f.relative_path)),
        );
    }
    let digest = store::hash_file(&crate::foundation::layout::path(&work, "comparison-native/sources.json"))?;
    paths
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            Ok(super::manifest::Artifact {
                role: format!("coarse_comparison_{i}"),
                relative_path: p.strip_prefix(work)?.into(),
                byte_size: fs::metadata(&p)?.len(),
                sha256: store::hash_file(&p)?,
                producer_stage: "comparison".into(),
                input_digest: digest.clone(),
                format_version: "1".into(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nonorthogonal_path_keeps_disconnected_segments_and_metric() {
        let dir = std::env::temp_dir().join(format!("epw-path-{}", store::token()));
        fs::create_dir(&dir).unwrap();
        let path = dir.join("KPATH.in");
        fs::write(
            &path,
            "0 0 0 G\n1 1 0 M\n1 1 0 M\n1 1 1 A\n0 0 1 L\n0 1 1 H\n",
        )
        .unwrap();
        let reciprocal = [[1., 0., 0.], [0.5, 1., 0.], [0., 0., 2.]];
        let (points, ticks) = path_points(&path, &reciprocal, 2).unwrap();
        assert_eq!(points.len(), 8);
        assert_eq!(
            points.iter().map(|p| p.segment_id).collect::<Vec<_>>(),
            [0, 0, 0, 0, 0, 1, 1, 1]
        );
        assert!((points[2].distance - 3.25f64.sqrt()).abs() < 1e-12);
        assert!((points[4].distance - points[2].distance - 2.).abs() < 1e-12);
        assert!(points.windows(2).all(|p| p[0].distance < p[1].distance));
        assert_eq!(
            ticks.iter().map(|t| t.label.as_str()).collect::<Vec<_>>(),
            ["G", "M", "A", "L", "H"]
        );
        fs::write(&path, "0 0 0 G\n0 0 0 G\n").unwrap();
        assert!(path_points(&path, &reciprocal, 2).is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}
