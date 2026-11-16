//! 第二阶段的固定路径和总量 DOS；科学执行仍在原分配内，绘图复用 F5。
use super::{
    attempt, comparison,
    comparison::SourceFile,
    comparison_asr, comparison_data,
    epw_config::{EpwConfig, NativeStage},
    native, restart,
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
use std::{fs, path::Path};

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

#[cfg(test)]
fn same_sampling(
    sampling: &super::electronic_sampling::Sampling,
    path: &[comparison::PathPoint],
    mesh: &[[f64; 3]],
    reciprocal: &[[f64; 3]; 3],
) -> bool {
    // SCF 与 NSCF XML 的倒格矢末位可能不同，不能要求派生距离的 JSON 完全相等。
    let close = |a: f64, b: f64| (a - b).abs() < 1e-10;
    sampling.path.len() == path.len()
        && sampling.mesh.len() == mesh.len()
        && sampling
            .reciprocal
            .iter()
            .flatten()
            .zip(reciprocal.iter().flatten())
            .all(|(a, b)| close(*a, *b))
        && sampling.path.iter().zip(path).all(|(a, b)| {
            a.segment_id == b.segment_id
                && a.point_id == b.point_id
                && close(a.distance, b.distance)
                && a.crystal.iter().zip(b.crystal).all(|(a, b)| close(*a, b))
        })
        && sampling
            .mesh
            .iter()
            .zip(mesh)
            .all(|(a, b)| a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-7))
}

pub(crate) fn path_points(
    path: &Path,
    reciprocal: &[[f64; 3]; 3],
    intervals: usize,
) -> Result<(Vec<comparison::PathPoint>, Vec<comparison_data::Tick>)> {
    ensure!(
        intervals > 0 && reciprocal.iter().flatten().all(|x| x.is_finite()),
        "configuration: Invalid path parameter for comparison"
    );
    let segments =
        qf_input::electronic::parse_kpath_segments(path.to_str().context("The path isn't UTF-8")?)?;
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
                "data_invalid: Path with zero length or invalid edge"
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
    ensure!(points.len() >= 2, "data_invalid: Compare Path to Empty");
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
        "data_invalid: Number of points/bands/modes"
    );
    for (p, q) in points.iter().zip(&values.points_cartesian) {
        ensure!(
            cart(*p, &geometry.reciprocal)
                .iter()
                .zip(q)
                .all(|(a, b)| (a - b).abs() < 1e-6),
            "input_conflict: Compare native output coordinates or order of points"
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
        "allocation: Autocomparison must be synchronized within the registered allocation"
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
        cfg.native_value::<u32>(&[
            crate::foundation::native_config::COARSE,
            "workflow",
            "points_per_edge",
        ])? as usize,
    )?;
    let states = f2.states.as_ref().context("data_invalid: Missing F2 Flat Grid")?;
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
    // QE 声子密度由自身阶段设置，不为差值统计服从 EPW 的路径点。
    let (qe_path, _) = path_points(kpath, &geometry.reciprocal,
        cfg.native_value::<u32>(&[&cfg.phonon_stage(), "workflow", "points_per_edge"])? as usize)?;
    let qe_all: Vec<_> = qe_path.iter().map(|p| p.crystal).chain(mesh.iter().copied()).collect();
    let post_dir = cfg
        .programs
        .ph
        .path
        .parent()
        .context("configuration: PH Missing Parent Directory")?;
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
            "input_conflict: Autocomparison {role}.x Not here F1 Freezing or changed"
        );
    }
    let coarse_dir = crate::foundation::layout::path(&work, "coarse");
    let modes = geometry.atoms.len() * 3;
    let nwann = cfg.wannier.num_wann as usize;
    let mut identities = vec![
        if crate::foundation::layout::enabled(work) {
            Identity::capture(&crate::foundation::layout::path(work, "nscf/nscf.in"))?
        } else {
            Identity::capture(&f2_path)?
        },
        Identity::capture(kpath)?,
        Identity::capture(&q2r.path)?,
        Identity::capture(&matdyn.path)?,
        Identity::capture(&cfg.programs.pw.path)?,
        Identity::capture(&cfg.programs.epw.path)?,
        Identity::capture(&std::env::current_exe()?)?,
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
    let (identity_cfg, identity_launcher) =
        crate::foundation::parallelism::request_identity(cfg, launcher)?;
    let digest = if crate::foundation::layout::enabled(work) {
        crate::foundation::stages::request_digest(
            cfg,
            crate::foundation::stages::Step::Coarse,
            &identities,
            "comparison",
        )?
    } else {
        store::hash_bytes(&serde_json::to_vec(&(
            &identity_cfg,
            &identity_launcher,
            &identities,
        ))?)
    };
    let root = crate::foundation::layout::path(&work, "comparison-native");
    fs::create_dir_all(&root)?;
    let numbered = crate::foundation::layout::enabled(work);
    let electron = if numbered {
        crate::foundation::layout::path(work, "comparison-native/electron")
    } else {
        root.join("electron")
    };
    let electron_input = electron.join("reference.in");
    let electron_xml = if numbered {
        crate::foundation::stages::Step::Electronic
            .directory(cfg)
            .join(format!(
                "qe_eband/out/{}.save/data-file-schema.xml",
                cfg.qe.prefix
            ))
    } else {
        electron.join(format!("out/{}.save/data-file-schema.xml", cfg.qe.prefix))
    };
    let inputs = [
        electron_input.clone(),
        crate::foundation::layout::path(&f2_work, "scf/snapshot-files.json"),
    ];
    if numbered {
        let sampling = super::electronic_sampling::plan(cfg)?;
        ensure!(
            sampling.mesh.len() == mesh.len()
                && sampling
                    .mesh
                    .iter()
                    .zip(&mesh)
                    .all(|(a, b)| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-7)),
            "input_conflict: I don't think so 5/6 The step reference grid is different"
        );
        crate::foundation::workspace_science::verify_electronic_comparison(cfg)?;
    } else {
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
                    qe.path_input(
                        electron.join("out").to_str().context("The path isn't UTF-8")?,
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
                    "data_invalid: Comparison NSCF Different points/bands"
                );
                for (expected, actual) in path_crystal.iter().zip(&data.kpoints) {
                    ensure!(
                        cart(*expected, &geometry.reciprocal)
                            .iter()
                            .zip(actual)
                            .all(|(a, b)| (a - b).abs() < 1e-7),
                        "input_conflict: Comparison NSCF Different coordinates or sequences"
                    );
                }
                Ok(())
            },
            |_| evidence(&electron),
        )?;
    }
    let phonon = crate::foundation::layout::path(work, "comparison-native/phonon");
    let gamma_q = f2
        .qpoints
        .iter()
        .find(|q| q.crystal.iter().all(|x| x.abs() < 1e-9))
        .context("data_invalid: Missing Γ")?;
    let gamma = &f2
        .phonon_files
        .iter()
        .find(|p| p.q_id == gamma_q.global_index)
        .context("data_invalid: Deficiencies and mapping")?
        .dyn_file;
    let force = phonon.join("force-constants");
    let force_file = force.join("reference.fc.xml");
    let onsite: String = cfg.native_value(&[
        crate::foundation::native_config::COARSE,
        "workflow",
        "onsite_asr",
    ])?;
    ensure!(
        onsite == "gamma_hermitian",
        "configuration: stages.6.epw-coarse.workflow.onsite_asr: Current only support gamma_hermitian"
    );
    attempt::run_with_inputs(
        work,
        "comparison-q2r",
        &force,
        store::hash_bytes(format!("{digest}:gamma-onsite-force").as_bytes()),
        retry,
        "q2r.exitcode",
        None,
        || {
            fs::create_dir_all(&force)?;
            store::write(
                &force.join("reuse-decision.json"),
                &serde_json::json!({
                    "required_programs":["q2r.x"],
                    "reference_mode":cfg.phonon.mode,
                    "reason":"I don't think so 4 Step q2r/matdyn Use simple ASR;EPW More so Γ onsite Add Hermitian Amended、q2r/matdyn No more impositions ASR .The current power constant does not meet the caliber; only the verified power matrix is converted and completed q2r ,Do Not Run PW/PH.",
                    "sources":f2.phonon_files
                }),
            )?;
            fs::copy(
                &gamma_q.source_dyn0,
                force.join(format!("{}.dyn0", cfg.qe.prefix)),
            )?;
            let gamma_text = fs::read_to_string(gamma)?;
            for p in &f2.phonon_files {
                fs::write(
                    force.join(format!("{}.dyn{}.xml", cfg.qe.prefix, p.q_id)),
                    comparison_asr::qe_xml_records(&comparison_asr::corrected_stars(
                        &fs::read_to_string(&p.dyn_file)?,
                        &gamma_text,
                        geometry.atoms.len(),
                    )?)?,
                )?;
            }
            fs::write(
                force.join("q2r.in"),
                format!(
                    "&input\n fildyn='{}.dyn.xml'\n flfrc='reference.fc'\n zasr='no'\n/\n",
                    cfg.qe.prefix
                ),
            )?;
            cfg.configure_file(
                &[
                    crate::foundation::native_config::COARSE,
                    "comparison",
                    "q2r",
                ],
                &force.join("q2r.in"),
            )?;
            Ok(())
        },
        || {
            qe_data::execution::run(
                &q2r,
                &force.join("q2r.in"),
                &force,
                launcher,
                backend,
                "q2r",
                false,
            )
        },
        || save::nonempty(&force_file),
        |_| evidence(&force),
    )?;
    let frequency = phonon.join("frequencies");
    let force_identity = Identity::capture(&force_file)?;
    attempt::run_with_inputs(
        work,
        "comparison-matdyn",
        &frequency,
        store::hash_bytes(format!("{digest}:frequencies:{}", force_identity.sha256).as_bytes()),
        retry,
        "matdyn.exitcode",
        None,
        || {
            fs::create_dir_all(&frequency)?;
            fs::copy(&force_file, frequency.join("reference.fc.xml"))?;
            force_identity.verify()?;
            store::write(
                &frequency.join("reuse-decision.json"),
                &serde_json::json!({
                    "required_programs":["matdyn.x"],
                    "reason":"I don't think so 4 We're using different steps ASR ,and does not contain the complete path plus DOS grids;read completed comparative strength constants, as required q Dot the frequency. Do not run again q2r/PW/PH.",
                    "force_constants":force_identity,"qpoints":qe_all
                }),
            )?;
            let mut input = String::from("&input\n flfrc='reference.fc.xml'\n flfrq='reference.freq'\n asr='no'\n q_in_cryst_coord=.false.\n q_in_band_form=.false.\n");
            for (i, el) in qe.cell.elements.iter().enumerate() {
                let mass =
                    qf_input::species::get_atomic_mass(el, cfg.atomic_masses.get(el).copied());
                input.push_str(&format!(" amass({})={mass:.17e}\n", i + 1));
            }
            input.push_str(&format!("/\n{}\n", qe_all.len()));
            for p in &qe_all {
                let p = cart(*p, &geometry.reciprocal);
                input.push_str(&format!("{:.17e} {:.17e} {:.17e}\n", p[0], p[1], p[2]));
            }
            fs::write(frequency.join("matdyn.in"), input)?;
            cfg.configure_file(
                &[
                    crate::foundation::native_config::COARSE,
                    "comparison",
                    "matdyn",
                ],
                &frequency.join("matdyn.in"),
            )?;
            Ok(())
        },
        || {
            qe_data::execution::run(
                &matdyn,
                &frequency.join("matdyn.in"),
                &frequency,
                launcher,
                backend,
                "matdyn",
                false,
            )
        },
        || {
            check_points(
                &native::parse_matdyn_path(&fs::read_to_string(frequency.join("reference.freq"))?)?,
                &qe_all,
                &geometry,
                modes,
            )
        },
        |_| evidence(&frequency),
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
            store::write(
                &epw_dir.join("reuse-decision.json"),
                &serde_json::json!({"required_programs":["epw.x"],"reason":"There are coarse grid real-space data that do not contain paths and DOS q interpolation frequency of points;using epwread Reads the model, only for acoustics. native band_plot At the same time, it requires electronics k Table, give a place-holder point, the output is not used for electronic comparison; the electronic comparison is reused 5 Step.","qpoints":all}),
            )?;
            EpwConfig::write_points(&epw_dir, if numbered { &all[..1] } else { &all }, &all)?;
            fs::write(
                epw_dir.join("epw.in"),
                epw.native_input(
                    NativeStage::Comparison,
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
                if numbered { &all[..1] } else { &all },
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
    for id in &identities {
        id.verify()?;
    }
    let mut provenance = vec![];
    let provenance_dirs = if numbered {
        vec![&phonon, &epw_dir]
    } else {
        vec![&electron, &phonon, &epw_dir]
    };
    if numbered {
        for n in ["band.eig", "sampling.json", "metrics.json"] {
            provenance.push(file(&electron.join(n))?);
        }
    }
    for dir in provenance_dirs {
        provenance.extend(
            evidence(dir)?
                .iter()
                .map(|id| file(&id.path))
                .collect::<Result<Vec<_>>>()?,
        );
    }
    let sources = comparison::Sources {
        schema_version: 2,
        run_id: f2.run_id,
        ph_mode: format!("{:?}", cfg.phonon.mode).to_lowercase(),
        fermi_reference: file(&states.wavefunctions.join("data-file-schema.xml"))?,
        qe_bands_xml: file(&electron_xml)?,
        qe_mesh_xml: file(&states.wavefunctions.join("data-file-schema.xml"))?,
        epw_bands: file(&if numbered {
            electron.join("band.eig")
        } else {
            epw_dir.join("band.eig")
        })?,
        electronic_stage: if numbered {
            Some(comparison::ElectronicStage::capture(&electron)?)
        } else {
            None
        },
        qe_phonon_bands: file(&frequency.join("reference.freq"))?,
        epw_phonon_bands: file(&epw_dir.join("phband.freq"))?,
        epw_log: file(&epw_dir.join("epw.out"))?,
        provenance,
        path,
        ticks,
        mesh,
        mesh_shape: states.k_grid.map(|v| v as usize),
        reciprocal: geometry.reciprocal,
        window_ev: comparison::parse_outer_window(&fs::read_to_string(
            coarse_dir.join(format!("{}.wout", cfg.qe.prefix)),
        )?)?
        .map(|v| v - states.fermi),
        wannier_wout: Some(file(&coarse_dir.join(format!("{}.wout", cfg.qe.prefix)))?),
        electronic_sigma_ev: options.electronic_sigma_ev,
        phonon_sigma_cm1: options.phonon_sigma_cm1,
    };
    let source_path = root.join("sources.json");
    if source_path.exists() {
        let previous: comparison::Sources = store::read(&source_path)?;
        ensure!(
            serde_json::to_value(&previous)? == serde_json::to_value(&sources)?,
            "input_conflict: Comparison of changes in the manifest of sources"
        );
    } else {
        store::write(&source_path, &sources)?;
    }
    let output = crate::foundation::layout::path(&work, "interpolation/comparison");
    if output.exists() {
        let data = comparison_data::load(&output)?;
        ensure!(
            data.manifest.upstream_digest == store::hash_file(&source_path)?,
            "input_conflict: Compared to other sources"
        );
    } else {
        fs::create_dir_all(output.parent().unwrap())?;
        comparison::export(&source_path, &output)?;
    }
    let results = crate::delivery::comparison::collect(work, None)?;
    if crate::foundation::layout::enabled(work) {
        crate::foundation::combined_plot::coarse(cfg, &results)?;
    } else {
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
    }
    Ok(())
}

pub(super) fn artifacts(work: &Path) -> Result<Vec<super::manifest::Artifact>> {
    let mut paths = vec![crate::foundation::layout::path(
        &work,
        "comparison-native/sources.json",
    )];
    for dir in [
        "interpolation/comparison",
        "results/comparison",
        "results/figures",
    ] {
        paths.extend(
            save::files(&crate::foundation::layout::path(work, dir))?
                .iter()
                .map(|f| crate::foundation::layout::path(work, dir).join(&f.relative_path)),
        );
    }
    let digest = store::hash_file(&crate::foundation::layout::path(
        &work,
        "comparison-native/sources.json",
    ))?;
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
    fn sampling_accepts_xml_roundoff_but_rejects_changed_coordinates_order_and_count() {
        let point = |id, x| comparison::PathPoint {
            segment_id: 0,
            point_id: id,
            distance: x,
            crystal: [x, 0., 0.],
        };
        let reciprocal = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
        let mesh = [[0., 0., 0.], [0.5, 0., 0.]];
        let sampling = super::super::electronic_sampling::Sampling {
            path: vec![point(0, 0.), point(1, 0.5)],
            ticks: vec![],
            mesh: mesh.to_vec(),
            reciprocal,
        };
        let mut path = vec![point(0, 0.), point(1, 0.5)];
        path[1].distance += 1e-15;
        let mut rounded = reciprocal;
        rounded[0][0] += 1e-15;
        assert!(same_sampling(&sampling, &path, &mesh, &rounded));
        assert!(!same_sampling(&sampling, &path[..1], &mesh, &rounded));
        assert!(!same_sampling(&sampling, &path, &mesh[..1], &rounded));
        path[1].crystal[0] += 1e-4;
        assert!(!same_sampling(&sampling, &path, &mesh, &rounded));
        path[1] = point(1, 0.5);
        path[1].segment_id = 1;
        assert!(!same_sampling(&sampling, &path, &mesh, &rounded));
        path[1] = point(1, 0.5);
        path.swap(0, 1);
        assert!(!same_sampling(&sampling, &path, &mesh, &rounded));
    }
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
