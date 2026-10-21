//! 编号阶段适配；科学输入、数据校验及求解器仍复用原有生产组件。
use super::{
    config::{Config, PhMode, Program},
    layout,
    stage::Backend,
    stages::{self, Step},
    store::{self, Identity},
};
use crate::{
    interpolation::{
        self,
        epw_config::{EpwConfig, NativeStage},
    },
    qe_data::{self, qe_config::QeSettings, save},
};
use anyhow::{ensure, Context, Result};
use serde_json::json;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

fn states(cfg: &Config) -> Result<qe_data::manifest::ElectronicStates> {
    let path = layout::path(&cfg.work_dir, "nscf/out")
        .join(format!("{}.save/data-file-schema.xml", cfg.qe.prefix));
    let mut s = qe_data::nscf::parse_xml(&path)?;
    s.k_grid = cfg.qe.nscf.k_grid;
    qe_data::nscf::validate_grid(
        &s,
        cfg.qe.nscf.k_grid,
        cfg.qe.nscf.nbnd as usize,
        &qe_data::xml::Geometry::read(&path)?,
    )?;
    Ok(s)
}
pub(super) fn post_program(cfg: &Config, name: &str) -> Result<Program> {
    Ok(Program {
        path: cfg
            .programs
            .pw
            .path
            .parent()
            .context("configuration: QE 缺程序目录")?
            .join(format!("{name}.x")),
        args: if matches!(name, "bands" | "dos") {
            vec!["-pd".into(), ".true.".into()]
        } else {
            vec![]
        },
    })
}
pub(super) fn qe_run(cfg: &Config, dir: &Path, program: &Program, input: &str, stem: &str) -> Result<()> {
    qe_data::execution::run(
        program,
        &dir.join(input),
        dir,
        &cfg.programs.launcher,
        Backend::Local,
        stem,
        false,
    )
}

pub fn run_electronic_leaf(cfg: &Config, name: &str) -> Result<()> {
    super::workspace_electronic::run_leaf(cfg, name)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PostReceipt {
    key: String,
    outputs: Vec<Identity>,
}

fn post_key(cfg: &Config, step: Step) -> Result<String> {
    let dependencies = if step == Step::Wannier {
        vec![Step::Wannier, Step::Electronic]
    } else {
        vec![Step::Coarse]
    };
    let keys = dependencies
        .iter()
        .map(|s| {
            let c: stages::Completion = store::read(&stages::receipt(&cfg.work_dir, *s))?;
            c.verify()?;
            c.output_key()
        })
        .collect::<Result<Vec<_>>>()?;
    let mut programs = vec![];
    if step == Step::Wannier {
        for p in [
            cfg.programs
                .wannier90
                .clone()
                .unwrap_or(post_program(cfg, "wannier90")?),
            cfg.programs
                .postw90
                .clone()
                .unwrap_or(post_program(cfg, "postw90")?),
            post_program(cfg, "dos")?,
        ] {
            programs.push((store::hash_file(&p.path)?, p.args));
        }
    }
    Ok(store::hash_bytes(&serde_json::to_vec(&(
        1,
        keys,
        programs,
        cfg.comparison.clone().unwrap_or_default(),
        cfg.kpath
            .as_ref()
            .map(|p| store::hash_file(p))
            .transpose()?,
    ))?))
}
fn post_file(cfg: &Config, step: Step) -> PathBuf {
    step.directory(cfg).join("post-completion.json")
}

pub fn post_pending(cfg: &Config, target: Step) -> Result<bool> {
    for step in [Step::Wannier, Step::Coarse] {
        if target < step || !stages::reusable(cfg, step)? {
            continue;
        }
        let p = post_file(cfg, step);
        if !p.exists() {
            return Ok(true);
        }
        let r: PostReceipt = store::read(&p)?;
        if r.key != post_key(cfg, step)? {
            return Ok(true);
        }
        for id in r.outputs {
            if !store::identity_available(&id)? { return Ok(true); }
        }
    }
    Ok(false)
}

/// 展宽及图像独立于拟合/EPC 的科学清单，不能改写下游已绑定的原生数据。
pub fn refresh_post(cfg: &Config, target: Step) -> Result<()> {
    for step in [Step::Wannier, Step::Coarse] {
        if target < step {
            continue;
        }
        let file = post_file(cfg, step);
        let key = post_key(cfg, step)?;
        if file.exists() {
            let r: PostReceipt = store::read(&file)?;
            if r.key == key {
                let mut available = true;
                for id in r.outputs {
                    if !store::identity_available(&id)? { available = false; }
                }
                if available { continue; }
            }
        }
        let outputs = if step == Step::Wannier {
            run_electronic_leaf(cfg, "qe_eledos")?;
            let dir = step.directory(cfg);
            electronic_comparison(cfg, &dir, states(cfg)?.fermi)?;
            [
                "electronic-bands.png",
                "electronic-dos.png",
                "electronic-dos.csv",
                "electronic-dos-normalization.json",
            ]
            .into_iter()
            .map(|n| Identity::capture(&dir.join(n)))
            .collect::<Result<Vec<_>>>()?
        } else {
            let path_key = store::hash_bytes(&serde_json::to_vec(&(
                cfg.comparison.clone().unwrap_or_default().points_per_edge,
                cfg.kpath
                    .as_ref()
                    .map(|p| store::hash_file(p))
                    .transpose()?,
            ))?);
            let path_file = cfg.work_dir.join("epw-state/post/coarse-path.json");
            if path_file.exists() && store::read::<String>(&path_file)? != path_key {
                let history = cfg
                    .work_dir
                    .join("epw-state/history")
                    .join(format!("comparison-path-{}", store::token()));
                fs::create_dir_all(&history)?;
                for (name, relative) in [
                    ("native", "comparison-native"),
                    ("tables", "interpolation/comparison"),
                    ("results", "results"),
                ] {
                    let p = layout::path(&cfg.work_dir, relative);
                    if p.exists() {
                        fs::rename(p, history.join(name))?;
                    }
                }
                for label in ["comparison-electron", "comparison-phonon", "comparison-epw"] {
                    let p = layout::path(&cfg.work_dir, "native-attempts")
                        .join(format!("{label}.json"));
                    if p.exists() {
                        fs::rename(p, history.join(format!("{label}.json")))?;
                    }
                }
                let s = states(cfg)?;
                let qe = QeSettings::from_config(cfg)?;
                let epw = EpwConfig::from_config(cfg)?
                    .with_fermi(s.fermi)?
                    .with_bands(&s.eigenvalues)?
                    .with_species(&qe.cell.elements)?;
                interpolation::automatic_comparison::run(
                    cfg,
                    &epw,
                    Backend::Local,
                    &cfg.programs.launcher,
                    &cfg.work_dir,
                    &cfg.work_dir,
                    true,
                )?;
            }
            store::write(&path_file, &path_key)?;
            let old = layout::path(&cfg.work_dir, "comparison-native/sources.json");
            let mut sources: interpolation::comparison::Sources = store::read(&old)?;
            let options = cfg.comparison.clone().unwrap_or_default();
            let output = layout::path(&cfg.work_dir, "interpolation/comparison");
            let results = layout::path(&cfg.work_dir, "results");
            sources.electronic_sigma_ev = options.electronic_sigma_ev;
            sources.phonon_sigma_cm1 = options.phonon_sigma_cm1;
            let source = cfg.work_dir.join("epw-state/post/coarse-sources.json");
            store::write(&source, &sources)?;
            let current = interpolation::comparison_data::load(&output)?;
            if current.manifest.upstream_digest != store::hash_file(&source)? {
                let history = cfg
                    .work_dir
                    .join("epw-state/history")
                    .join(format!("plots-{}", store::token()));
                fs::create_dir_all(&history)?;
                for (name, p) in [("tables", &output), ("results", &results)] {
                    if p.exists() {
                        fs::rename(p, history.join(name))?;
                    }
                }
                interpolation::comparison::export(&source, &output)?;
                crate::delivery::comparison::collect(&cfg.work_dir, None)?;
                crate::delivery::comparison_plot::plot(
                    &results,
                    None,
                    &["png".into(), "svg".into(), "pdf".into()],
                )?;
            }
            vec![
                Identity::capture(&output.join("manifest.json"))?,
                Identity::capture(&results.join("comparison/manifest.json"))?,
            ]
        };
        store::write(&file, &PostReceipt { key, outputs })?;
    }
    Ok(())
}

pub fn generate(cfg: &Config, target: Step) -> Result<()> {
    let qe = QeSettings::from_config(cfg)?;
    for step in target.closure() {
        if stages::reusable(cfg, step)? {
            continue;
        }
        let dir = step.directory(cfg);
        fs::create_dir_all(&dir)?;
        let request = cfg
            .work_dir
            .join("epw-state/requests")
            .join(format!("{}.json", step.name()));
        if request.is_file()
            && store::read::<String>(&request)? == stages::scientific_key(cfg, step)?
            && dir.join(match step { Step::Scf => "scf.in", Step::Nscf => "nscf.in", Step::Wannier => "epw_wannier.in", _ => "submit.sh" }).is_file()
        {
            if step == Step::Electronic {
                super::workspace_electronic::inputs(cfg, &qe)?;
            }
            continue;
        }
        match step {
            Step::Scf => fs::write(dir.join("scf.in"), qe.scf_input("./out")?)?,
            Step::Nscf => fs::write(dir.join("nscf.in"), qe.nscf_input("./out")?)?,
            Step::Electronic => super::workspace_electronic::inputs(cfg, &qe)?,
            Step::Phonon => {
                if cfg.phonon.mode == PhMode::Nosplit {
                    fs::write(dir.join("ph.in"), qe.phonon_nosplit_input("./out"))?;
                } else {
                    fs::create_dir_all(dir.join("init"))?;
                    let input = dir.join("init/ph_init.in");
                    fs::write(&input, qe.phonon_init_input("./out"))?;
                    super::workspace::native_script(
                        &cfg.programs.ph,
                        &input,
                        &dir.join("init"),
                        &cfg.programs.launcher,
                        "ph_init",
                        true,
                        false,
                    )?;
                }
            }
            Step::Wannier | Step::Coarse | Step::Fine => {
                if let Ok(s) = states(cfg) {
                    let epw = EpwConfig::from_config(cfg)?
                        .with_fermi(s.fermi)?
                        .with_bands(&s.eigenvalues)?
                        .with_species(&qe.cell.elements)?;
                    let native = match step {
                        Step::Wannier => NativeStage::WannierOnly,
                        Step::Coarse => NativeStage::ReuseWannier,
                        _ => NativeStage::FineMatrices,
                    };
                    fs::write(
                        dir.join(match step {
                            Step::Wannier => "epw_wannier.in",
                            Step::Coarse => "epw_coarse.in",
                            _ => "epw-template.in",
                        }),
                        epw.native_input(
                            native,
                            "./out",
                            "./save",
                            cfg.interpolation.phonon_smearings_mev[0],
                        )?,
                    )?;
                }
            }
            Step::Superconductivity => {
                for mode in ["iso", "aniso"] {
                    fs::create_dir_all(dir.join(mode))?;
                }
            }
        }
    }
    Ok(())
}

pub fn run(cfg: &Config, step: Step) -> Result<()> {
    let root = &cfg.work_dir;
    let qe = QeSettings::from_config(cfg)?;
    match step {
        Step::Scf => {
            qe_data::scf::run_scf(&qe, root, &cfg.programs.launcher, Backend::Local)?;
        }
        Step::Nscf => {
            qe_data::nscf::run_nscf(&qe, root, &cfg.programs.launcher, Backend::Local)?;
        }
        Step::Electronic => {
            run_electronic_leaf(cfg, "qe_eband")?;
            run_electronic_leaf(cfg, "qe_eledos")?;
        }
        Step::Phonon => run_phonon(cfg, &qe)?,
        Step::Wannier => run_wannier(cfg, &qe)?,
        Step::Coarse => {
            refresh_qe_manifest(cfg)?;
            interpolation::scheduler::run_coarse_with_retry(
                cfg,
                Backend::Local,
                &cfg.programs.launcher,
                root,
                root,
                true,
            )?;
        }
        Step::Fine => {
            interpolation::scheduler::run_fine_with_retry(
                cfg,
                Backend::Local,
                &cfg.programs.launcher,
                root,
                true,
            )?;
        }
        Step::Superconductivity => {
            let result = crate::superconductivity::scheduler::run_with_retry(
                cfg,
                root,
                &cfg.programs.launcher,
                Backend::Local,
                true,
            )?;
            ensure!(
                result.validation.all_passed(),
                "execution_failed: 原生超导仍为 partial，请在修复后续算"
            );
            crate::delivery::finalize_to(
                root,
                Some(&Step::Superconductivity.directory(cfg).join("results")),
            )?;
        }
    }
    Ok(())
}

/// PH 与 NSCF 是分支；更换 NSCF 后只重建组合交付，不重复计算 PH。
fn refresh_qe_manifest(cfg: &Config) -> Result<()> {
    let root = &cfg.work_dir;
    let path = layout::path(root, "qe-data/manifest.json");
    let mut m: qe_data::manifest::QeManifest = store::read(&path)?;
    let ph: stages::Completion = store::read(&stages::receipt(root, Step::Phonon))?;
    ph.verify()?;
    m.states = Some(states(cfg)?);
    m.artifacts.retain(|a| {
        !matches!(a.producer_stage.as_str(), "scf" | "nscf") || a.role == "scf_snapshot_identity"
    });
    for (stage, dir) in [
        ("scf", layout::path(root, "scf/snapshot")),
        ("nscf", layout::path(root, "nscf/out")),
    ] {
        for f in save::files(&dir)? {
            m.artifacts.push(qe_data::manifest::Artifact {
                role: format!("{stage}_{}", f.relative_path.display()),
                relative_path: dir.join(&f.relative_path).strip_prefix(root)?.into(),
                byte_size: f.byte_size,
                sha256: f.sha256,
                producer_stage: stage.into(),
                input_digest: String::new(),
                format_version: "qe7.4.1-portable".into(),
            });
        }
    }
    m.verify()?;
    store::write(&path, &m)
}

fn run_phonon(cfg: &Config, qe: &QeSettings) -> Result<()> {
    let root = &cfg.work_dir;
    let cached = layout::path(root, "qe-data/manifest.json");
    let manifest = if super::native_jobs::enabled() && cached.is_file() {
        // 后处理的每次原生调用返回后重验既有交付，避免重新归集或覆盖保存树。
        let manifest: qe_data::manifest::QeManifest = store::read(&cached)?;
        ensure!(manifest.run_id == stages::scientific_key(cfg, Step::Phonon)?,
            "input_conflict: PH 检查点参数变化");
        manifest.verify()?;
        ensure!(manifest.artifact_root.as_ref() == Some(&cfg.work_dir.canonicalize()?),
            "input_conflict: PH 检查点不属于当前工作区");
        manifest
    } else {
        let (points, responses) = if cfg.phonon.mode == PhMode::Nosplit {
            qe_data::phonon::run_nosplit(qe, root, &cfg.programs.launcher, Backend::Local)?
        } else {
            let points = qe_data::phonon::run_init(qe, root, &cfg.programs.launcher, Backend::Local)?;
            let responses = points
                .iter()
                .enumerate()
                .map(|(i, q)| {
                    qe_data::phonon::run_q(qe, root, &cfg.programs.launcher, Backend::Local, q, i + 1)
                })
                .collect::<Result<Vec<_>>>()?;
            (points, responses)
        };
        let scf = qe_data::scf::ScfResult {
            energy_ry: qe_data::scf::parse_scf_energy(&fs::read_to_string(layout::path(
                root,
                "scf/scf.out",
            ))?)?,
            snapshot_dir: layout::path(root, "scf/snapshot"),
            attempt_count: 1,
            success_count: 1,
        };
        let mut programs = BTreeMap::new();
        for (name, p) in [
            ("pw", cfg.programs.pw.path.clone()),
            ("ph", cfg.programs.ph.path.clone()),
            ("epw", cfg.programs.epw.path.clone()),
            ("q2r", post_program(cfg, "q2r")?.path),
            ("matdyn", post_program(cfg, "matdyn")?.path),
            ("dynmat", post_program(cfg, "dynmat")?.path),
        ] {
            programs.insert(name.into(), store::hash_file(&p)?);
        }
        let fingerprints = qe_data::manifest::Fingerprints {
            structure_sha256: store::hash_file(&cfg.structure)?,
            pseudo_sha256: cfg
                .pseudos
                .iter()
                .map(|(e, p)| Ok((e.clone(), store::hash_file(p)?)))
                .collect::<Result<_>>()?,
            program_sha256: programs,
        };
        qe_data::scheduler::publish_data(
            &stages::scientific_key(cfg, Step::Phonon)?,
            fingerprints,
            qe,
            root,
            scf,
            states(cfg)?,
            points,
            responses,
        )?
    };
    let dir = Step::Phonon.directory(cfg);
    // q2r 消费严格归集的同一组 XML；coords 局部编号已在 save 中规范化。
    fs::create_dir_all(&dir)?;
    // 手动推进会重新归集 PH；不得把原 dyn0 复制到自身而截断。
    let dyn0_source = &manifest.qpoints[0].source_dyn0;
    let dyn0_target = dir.join(format!("{}.dyn0", cfg.qe.prefix));
    if !dyn0_target.exists() || dyn0_source.canonicalize()? != dyn0_target.canonicalize()? {
        fs::copy(dyn0_source, &dyn0_target)?;
    }
    for r in &manifest.phonon_files {
        fs::copy(
            &r.dyn_file,
            dir.join(format!("{}.dyn{}.xml", cfg.qe.prefix, r.q_id)),
        )?;
    }
    let mut shared = qf_config::Config::default();
    shared.qe_phonon.retain_epw_data = true;
    shared.qe_phonon.q2r.insert("zasr".into(), json!("simple"));
    // 声子后处理网格独立于下游 EPC nqf，避免修改 nqf 牵动 PH。
    shared
        .qe_phonon
        .phdos
        .insert("qpoints".into(), json!(cfg.phonon.q_grid.map(|n| n * 4)));
    shared
        .qe_phonon
        .phband
        .insert("kpath_file".into(), json!(cfg.kpath));
    qf_input::q2r::write_q2r_input(dir.to_str().unwrap(), &shared, &cfg.qe.prefix)?;
    qf_input::matdyn::write_matdyn_phband_input(
        dir.to_str().unwrap(),
        &shared,
        &cfg.qe.prefix,
        qe.cell.lattice,
    )?;
    qf_input::matdyn::write_matdyn_phdos_input(dir.to_str().unwrap(), &shared, &cfg.qe.prefix)?;
    for (program, input, stem) in [
        ("q2r", "q2r.in", "q2r"),
        ("matdyn", "matdyn_phband.in", "matdyn_phband"),
        ("matdyn", "matdyn_phdos.in", "matdyn_phdos"),
    ] {
        qe_run(cfg, &dir, &post_program(cfg, program)?, input, stem)?;
    }
    store::write(&dir.join("save/manifest.json"), &manifest)?;
    Ok(())
}

fn run_wannier(cfg: &Config, qe: &QeSettings) -> Result<()> {
    let dir = Step::Wannier.directory(cfg);
    let s = states(cfg)?;
    let epw = EpwConfig::from_config(cfg)?
        .with_fermi(s.fermi)?
        .with_bands(&s.eigenvalues)?
        .with_species(&qe.cell.elements)?;
    if !super::native_jobs::continuing(&dir, "epw")? {
    save::copy_tree(&layout::path(&cfg.work_dir, "nscf/out"), &dir.join("out"))?;
    fs::write(
        dir.join("epw_wannier.in"),
        epw.native_input(
            NativeStage::WannierOnly,
            "./out",
            "./save",
            cfg.interpolation.phonon_smearings_mev[0],
        )?,
    )?;
    }
    interpolation::coarse::run_epw(
        &epw,
        &dir,
        "epw_wannier.in",
        &cfg.programs.launcher,
        Backend::Local,
    )?;
    let prefix = &cfg.qe.prefix;
    interpolation::wannier::check_native(
        &dir.join(format!("{prefix}.wout")),
        cfg.wannier.num_wann as usize,
    )?;
    epw.check_native_windows(&fs::read_to_string(dir.join(format!("{prefix}.wout")))?)?;
    validate_ukk(&dir.join(format!("{prefix}.ukk")), cfg)?;

    Ok(())
}

pub fn validate_ukk(path: &Path, cfg: &Config) -> Result<()> {
    let (points, kept) =
        interpolation::restart::manifold_dimensions(path, cfg.wannier.num_wann as usize)?;
    ensure!(
        points
            == cfg
                .qe
                .nscf
                .k_grid
                .iter()
                .map(|&n| n as usize)
                .product::<usize>(),
        "input_conflict: ukk k 点数与当前 NSCF 不相容"
    );
    let expected = (1..=cfg.qe.nscf.nbnd as usize)
        .filter(|n| !cfg.wannier.exclude_bands.contains(&(*n as u32)))
        .collect::<Vec<_>>();
    ensure!(
        kept == expected,
        "input_conflict: ukk 排除带与当前配置不相容"
    );
    let raw = fs::read_to_string(path)?;
    let first = raw
        .lines()
        .next()
        .context("data_invalid: 空 ukk")?
        .split_whitespace()
        .map(str::parse::<usize>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(
        first.len() == 2
            && first[0] > 0
            && first[0] <= cfg.qe.nscf.nbnd as usize
            && first[1] <= cfg.qe.nscf.nbnd as usize,
        "data_invalid: ukk 带空间不相容"
    );
    ensure!(
        raw.lines().count() > cfg.wannier.num_wann as usize,
        "data_invalid: ukk 数据不完整"
    );
    Ok(())
}

fn electronic_comparison(cfg: &Config, dir: &Path, fermi: f64) -> Result<()> {
    if super::native_jobs::enabled() {
        let post = dir.join("native-post");
        if !post.exists() {
            fs::create_dir_all(&post)?;
            for ext in ["win", "wout", "chk", "mmn", "amn", "eig", "nnkp"] {
                let name = format!("{}.{ext}", cfg.qe.prefix);
                if dir.join(&name).exists() { fs::copy(dir.join(&name),post.join(name))?; }
            }
        }
        fs::copy(dir.join(format!("{}.wout",cfg.qe.prefix)),post.join(format!("{}.wout",cfg.qe.prefix)))?;
        electronic_comparison_inner(cfg,&post,fermi)?;
        for name in ["electronic-bands.png","electronic-dos.png","electronic-dos.csv","electronic-dos-normalization.json"] {
            fs::copy(post.join(name),dir.join(name))?;
        }
        return Ok(());
    }
    electronic_comparison_inner(cfg,dir,fermi)
}

fn electronic_comparison_inner(cfg: &Config, dir: &Path, fermi: f64) -> Result<()> {
    let prefix = &cfg.qe.prefix;
    let win = dir.join(format!("{prefix}.win"));
    let segments = qf_input::electronic::parse_kpath_segments(
        cfg.kpath
            .as_ref()
            .context("required: 电子对比需要 KPATH.in")?
            .to_str()
            .unwrap(),
    )?;
    let mut path = String::from("\nbegin kpoint_path\n");
    for segment in segments {
        for pair in segment.windows(2) {
            path.push_str(&format!(
                "{} {} {} {} {} {} {} {}\n",
                pair[0].1,
                pair[0].0[0],
                pair[0].0[1],
                pair[0].0[2],
                pair[1].1,
                pair[1].0[0],
                pair[1].0[1],
                pair[1].0[2]
            ));
        }
    }
    path.push_str("end kpoint_path\n");
    let original = fs::read_to_string(&win)?;
    let original =
        regex::Regex::new(r"(?ims)^\s*begin\s+kpoint_path\s*$.*?^\s*end\s+kpoint_path\s*$")?
            .replace_all(&original, "")
            .into_owned();
    let options = cfg.comparison.clone().unwrap_or_default();
    let window = interpolation::comparison::parse_outer_window(&fs::read_to_string(
        dir.join(format!("{prefix}.wout")),
    )?)?;
    let updated = wannier90flow::post::update_win_content(
        &original,
        &[
            ("write_hr".into(), json!(false)),
            ("bands_plot".into(), json!(true)),
            ("bands_num_points".into(), json!(options.points_per_edge)),
            ("dos".into(), json!(true)),
            ("dos_kmesh".into(), json!(cfg.qe.nscf.k_grid)),
            ("dos_energy_min".into(), json!(window[0])),
            ("dos_energy_max".into(), json!(window[1])),
            ("dos_energy_step".into(), json!(0.01)),
            ("dos_adpt_smr".into(), json!(false)),
            (
                "dos_smr_fixed_en_width".into(),
                json!(options.electronic_sigma_ev),
            ),
            ("dos_smr_index".into(), json!(0)),
            ("fermi_energy".into(), json!(fermi)),
        ],
    );
    let updated = wannier90flow::post::update_win_content(
        &(updated + &path),
        &[("restart".into(), json!("plot"))],
    );
    if !super::native_jobs::continuing(dir, "wannier-plot")? {
        fs::write(&win, updated)?;
    }
    // postw90 是原生计算；不以高斯拟合或总 DOS 伪装缺失投影。
    let original_wout = fs::read(dir.join(format!("{prefix}.wout")))?;
    let wannier = cfg
        .programs
        .wannier90
        .clone()
        .unwrap_or(post_program(cfg, "wannier90")?);
    save::nonempty(&dir.join(format!("{prefix}.chk")))?;
    let mut argv = vec![wannier.path.to_string_lossy().into_owned()];
    argv.extend(wannier.args);
    argv.push(prefix.clone());
    let report = super::process::run_report_named(&argv, dir, "wannier-plot")?;
    fs::copy(report.stdout_path, dir.join("wannier-plot.out"))?;
    fs::copy(report.stderr_path, dir.join("wannier-plot.err"))?;
    ensure!(
        report.exit_status == Some(0),
        "execution_failed: Wannier90 restart=plot 未成功"
    );
    fs::write(dir.join(format!("{prefix}.wout")), original_wout)?;
    let program = cfg
        .programs
        .postw90
        .clone()
        .unwrap_or(post_program(cfg, "postw90")?);
    save::nonempty(&program.path)?;
    let mut argv = vec![program.path.to_string_lossy().into_owned()];
    argv.extend(program.args);
    argv.push(prefix.clone());
    let report = super::process::run_report_named(&argv, dir, "postw90")?;
    fs::copy(report.stdout_path, dir.join("postw90.out"))?;
    fs::copy(report.stderr_path, dir.join("postw90.err"))?;
    ensure!(
        report.exit_status == Some(0),
        "execution_failed: postw90 未成功"
    );
    let band = dir.join(format!("{prefix}_band.dat"));
    let labels = dir.join(format!("{prefix}_band.labelinfo.dat"));
    let dos = PathBuf::from(wannier90flow::post::resolve_dos_output(
        dir.to_str().unwrap(),
        prefix,
    ));
    for p in [&band, &labels, &dos] {
        save::nonempty(p)?;
    }
    super::workspace_plot::bands(cfg, dir, &band, fermi)?;
    super::workspace_plot::dos(cfg, dir, &dos, fermi)?;
    save::nonempty(&dir.join("electronic-bands.png"))?;
    Ok(())
}

pub fn evidence(cfg: &Config, step: Step) -> Result<Vec<Identity>> {
    let dir = step.directory(cfg);
    let mut ids = save::files(&dir)?
        .iter()
        .filter(|f| {
            let name = f.relative_path.file_name().unwrap().to_string_lossy();
            let rel = f.relative_path.to_string_lossy();
            if step == Step::Wannier
                && !rel.starts_with("out/")
                && ![
                    format!("{}.ukk", cfg.qe.prefix),
                    format!("{}.chk", cfg.qe.prefix),
                    format!("{}.wout", cfg.qe.prefix),
                    format!("{}_hr.dat", cfg.qe.prefix),
                    "epw_wannier.in".into(),
                    "epw.out".into(),
                    "epw.err".into(),
                ]
                .contains(&rel.to_string())
            {
                return false;
            }
            if step == Step::Electronic
                && rel.starts_with("qe_eledos/")
                && !rel.starts_with("qe_eledos/out/")
            {
                return false;
            }
            if step == Step::Coarse
                && (rel.starts_with("comparison/")
                    || rel.starts_with("results/")
                    || rel.starts_with("comparison-native/"))
            {
                return false;
            }
            name != "completion.json"
                && name != "post-completion.json"
                && !rel.starts_with("receipts/")
                && name != "submit.sh"
                && !name.starts_with("submit-")
                && !super::native_jobs::is_log(&f.relative_path)
                && !name.starts_with("slurm-")
                && !name.starts_with(".epw-")
                && !(step == Step::Phonon && name == "manifest.json")
        })
        .map(|f| Identity::capture(&dir.join(&f.relative_path)))
        .collect::<Result<Vec<_>>>()?;
    if step == Step::Coarse {
        ids.push(Identity::capture(&layout::path(
            &cfg.work_dir,
            "wannier/manifest.json",
        ))?);
    }
    if step == Step::Fine {
        ids.push(Identity::capture(&layout::path(
            &cfg.work_dir,
            "interpolation/manifest.json",
        ))?);
    }
    ensure!(!ids.is_empty(), "data_invalid: 阶段没有科学证据");
    Ok(ids)
}

pub fn invalidate(cfg: &Config, step: Step, history: &Path) -> Result<()> {
    let root = &cfg.work_dir;
    let mut paths = vec![];
    match step {
        Step::Phonon => paths.push(layout::path(root, "qe-data")),
        Step::Coarse => {
            paths.push(layout::path(root, "wannier"));
            // 新科学请求重新建立质量绑定；资源变更复用通过阶段缓存，不进入这里。
            for p in [
                "quality-reference.json",
                "quality-binding.json",
                "quality-report.json",
                "quality-sources",
                "comparison-native",
            ] {
                paths.push(root.join(p));
            }
            paths.push(crate::foundation::layout::path(&root, "native-attempts"));
        }
        Step::Fine => {
            paths.push(layout::path(root, "interpolation"));
            if crate::foundation::layout::path(&root, "native-attempts").is_dir() {
                for e in fs::read_dir(crate::foundation::layout::path(&root, "native-attempts"))? {
                    let p = e?.path();
                    if p.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with("fine-")
                    {
                        paths.push(p);
                    }
                }
            }
        }
        _ => (),
    }
    for (n, p) in paths.iter().enumerate() {
        if p.exists() {
            fs::create_dir_all(history)?;
            fs::rename(p, history.join(format!("metadata-{n}")))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod post_program_tests {
    use super::*;

    #[test]
    fn pencil_is_limited_to_bands_and_dos_without_inherited_pools() {
        let mut cfg: Config = toml::from_str(include_str!("../../tests/fixtures/input.toml")).unwrap();
        cfg.programs.pw.path = PathBuf::from("/qe with spaces/bin/pw.x");
        cfg.programs.pw.args = vec!["-nk".into(), "8".into()];
        for name in ["bands", "dos", "q2r", "matdyn", "dynmat", "wannier90", "postw90"] {
            let p = post_program(&cfg, name).unwrap();
            assert_eq!(p.path, PathBuf::from(format!("/qe with spaces/bin/{name}.x")));
            let expected = if matches!(name, "bands" | "dos") {
                vec!["-pd".to_string(), ".true.".to_string()]
            } else {
                vec![]
            };
            assert_eq!(p.args, expected, "{name}");
        }
        assert_eq!(cfg.programs.pw.args, vec!["-nk", "8"]);
    }
}
