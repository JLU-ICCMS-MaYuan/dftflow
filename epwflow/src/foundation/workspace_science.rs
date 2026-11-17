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
    fs,
    path::{Path, PathBuf},
};

fn states(cfg: &Config) -> Result<qe_data::manifest::ElectronicStates> {
    qe_data::nscf::verified_states(cfg, &cfg.work_dir)
}
pub(super) fn post_program(cfg: &Config, name: &str) -> Result<Program> {
    Ok(cfg.programs.get(name)?.clone())
}
pub(super) fn qe_run(
    cfg: &Config,
    dir: &Path,
    program: &Program,
    input: &str,
    stem: &str,
) -> Result<()> {
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
    Ok(serde_json::to_string(&json!({"stage":step.name(),"plot":cfg.plot,"comparison":cfg.comparison,
        "request": cfg.native_value::<serde_json::Value>(&[if step==Step::Wannier {super::native_config::WANNIER}else{super::native_config::COARSE}])?}))?)
}
fn read_post(cfg: &Config, step: Step) -> Result<Option<PostReceipt>> {
    let Some(outputs) = super::readable_state::completed_files(cfg, step, "Derived delivery")? else {
        return Ok(None);
    };
    let Some(old) = super::readable_state::checkpoint_config(cfg, step, "Derived delivery")? else {
        return Ok(None);
    };
    Ok(Some(PostReceipt {
        key: post_key(&old, step)?,
        outputs,
    }))
}
fn save_post(cfg: &Config, step: Step, outputs: &[Identity]) -> Result<()> {
    super::readable_state::complete_files(
        cfg,
        step,
        "Derived delivery",
        &outputs.iter().map(|id| id.path.clone()).collect::<Vec<_>>(),
    )
}

pub fn post_pending(cfg: &Config, target: Step) -> Result<bool> {
    if target==Step::Coarse && layout::enabled(&cfg.work_dir) {
        return Ok(crate::delivery::decay::pending(cfg)? || (cfg.kpath.is_some() && !super::stage_results::root(cfg).join("6.epw-coarse/phonon.png").is_file()));
    }

    if target == Step::Coarse
        && stages::reusable(cfg, Step::Coarse)?
        && crate::delivery::decay::pending(cfg)?
    {
        return Ok(true);
    }
    if super::workspace_qe_plot::pending(cfg, target)? {
        return Ok(true);
    }
    for step in stages::execution_steps(cfg, target) {
        if matches!(
            step,
            Step::Electronic
                | Step::Phonon
                | Step::Wannier
                | Step::Coarse
                | Step::Superconductivity
        ) && stages::reusable(cfg, step)?
            && super::stage_results::pending(cfg, step)?
        {
            return Ok(true);
        }
    }
    for step in [Step::Wannier, Step::Coarse] {
        if target != step || !stages::reusable(cfg, step)? {
            continue;
        }
        let Some(r) = read_post(cfg, step)? else {
            return Ok(true);
        };
        if r.key != post_key(cfg, step)? {
            return Ok(true);
        }
        for id in r.outputs {
            if !store::identity_available(&id)? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// 展宽及图像独立于拟合/EPC 的科学清单，不能改写下游已绑定的原生数据。
pub fn refresh_post(cfg: &Config,target:Step)->Result<()> {
    let doc=super::readable_state::read(&cfg.work_dir,target)?;
    if let Some(reason)=doc.get("Plot failure","Reason") {
        anyhow::bail!("plot_failed: {reason}; automatic retry disabled; request plotting explicitly");
    }
    let result=refresh_post_inner(cfg,target);
    if let Err(error)=&result {
        if format!("{error:#}").contains("plot_failed:") {
            super::readable_state::update(&cfg.work_dir,target,|d|{
                d.set("Plot failure","Reason",format!("{error:#}"));
                d.set("Stage","Status","plot failed; scientific results retained");Ok(())
            })?;
        }
    }
    result
}
fn refresh_post_inner(cfg: &Config, target: Step) -> Result<()> {
    if target==Step::Coarse && layout::enabled(&cfg.work_dir) {
        crate::delivery::decay::refresh(cfg)?;
        return interpolation::phonon_comparison::run(cfg);
    }

    // 原生粗网格回执先落盘；衰减交付不等待电子／声子比较收尾。
    if target == Step::Coarse && stages::reusable(cfg, Step::Coarse)? {
        crate::delivery::decay::refresh(cfg)?;
    }
    for step in stages::execution_steps(cfg, target) {
        if matches!(step, Step::Scf | Step::Nscf | Step::Electronic | Step::Phonon) && stages::reusable(cfg, step)? {
            if step == Step::Electronic {
                super::electronic_reuse::refresh(cfg, || super::workspace_qe_plot::refresh(cfg, step))?;
            } else {
                super::workspace_qe_plot::refresh(cfg, step)?;
            }
        }
    }
    for step in [Step::Wannier, Step::Coarse] {
        if target != step {
            continue;
        }
        let key = post_key(cfg, step)?;
        if let Some(r) = read_post(cfg, step)? {
            if r.key == key {
                let mut available = true;
                for id in r.outputs {
                    if !store::identity_available(&id)? {
                        available = false;
                    }
                }
                if available && !super::stage_results::pending(cfg, step)? {
                    continue;
                }
            }
        }
        let outputs = if step == Step::Wannier {
            let dir = step.directory(cfg);
            electronic_comparison(cfg, &dir, states(cfg)?.fermi)?;
            electronic_files()
                .iter()
                .map(|n| {
                    native_reference(
                        &layout::path(&cfg.work_dir, "comparison-native/electron").join(n),
                    )
                })
                .chain(super::combined_plot::ELECTRONIC_FILES.iter().map(|n| native_reference(&super::stage_results::root(cfg).join("5.epw-wannier").join(n))))
                .collect::<Result<Vec<_>>>()?
        } else {
            let previous = super::readable_state::checkpoint_config(cfg, step, "Comparison path")?;
            let changed = previous
                .as_ref()
                .map(|old| {
                    Ok::<_, anyhow::Error>(
                        old.native_value::<serde_json::Value>(&[
                            super::native_config::COARSE,
                            "comparison",
                        ])? != cfg.native_value::<serde_json::Value>(&[
                            super::native_config::COARSE,
                            "comparison",
                        ])? || old.kpath != cfg.kpath
                            || old.native_value::<u32>(&[super::native_config::COARSE, "workflow", "points_per_edge"])?
                                != cfg.native_value::<u32>(&[super::native_config::COARSE, "workflow", "points_per_edge"])?
                            || old.native_value::<u32>(&[&old.phonon_stage(), "workflow", "points_per_edge"])?
                                != cfg.native_value::<u32>(&[&cfg.phonon_stage(), "workflow", "points_per_edge"])?,
                    )
                })
                .transpose()?
                .unwrap_or(false);
            let source_file = layout::path(&cfg.work_dir, "comparison-native/sources.json");
            let old_qe_reference = source_file.is_file()
                && store::read::<interpolation::comparison::Sources>(&source_file)?.qe_phonon_dos.is_none();
            if changed || old_qe_reference {
                for relative in ["comparison-native/sources.json", "interpolation/comparison"] {
                    let p = layout::path(&cfg.work_dir, relative);
                    if p.is_dir() {
                        for name in ["electronic_bands.csv","electronic_dos.csv","phonon_bands.csv","phonon_dos.csv","metadata.json","metrics.json","manifest.json"] {
                            let file=p.join(name);if file.is_file(){fs::remove_file(file)?;}
                        }
                    } else if p.exists() { fs::remove_file(p)?; }
                }
            }
            if !layout::path(&cfg.work_dir, "comparison-native/sources.json").exists() {
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
            super::readable_state::complete_files(cfg, step, "Comparison path", &[])?;
            let old = layout::path(&cfg.work_dir, "comparison-native/sources.json");
            let mut sources: interpolation::comparison::Sources = store::read(&old)?;
            let options = cfg.comparison.clone().unwrap_or_default();
            let output = layout::path(&cfg.work_dir, "interpolation/comparison");
            let results = crate::delivery::comparison::result_dir(&cfg.work_dir);
            sources.epw_bands = interpolation::comparison::SourceFile {
                path: layout::path(&cfg.work_dir, "comparison-native/electron/band.eig"),
                sha256: store::hash_file(&layout::path(
                    &cfg.work_dir,
                    "comparison-native/electron/band.eig",
                ))?,
            };
            sources.electronic_stage = Some(interpolation::comparison::ElectronicStage::capture(
                &layout::path(&cfg.work_dir, "comparison-native/electron"),
            )?);
            let qe_bands = Step::Electronic.directory(cfg).join(format!(
                "qe_eband/out/{}.save/data-file-schema.xml",
                cfg.qe.prefix
            ));
            sources.qe_bands_xml = interpolation::comparison::SourceFile {
                sha256: store::hash_file(&qe_bands)?,
                path: qe_bands,
            };
            sources.electronic_sigma_ev = options.electronic_sigma_ev;
            sources.phonon_sigma_cm1 = options.phonon_sigma_cm1;
            let source = layout::path(&cfg.work_dir, "comparison-native/plot-sources.json");
            store::write(&source, &sources)?;
            let current = interpolation::comparison_data::load(&output)?;
            if current.manifest.upstream_digest != store::hash_file(&source)? {
                for p in [&output, &results] {
                    if p.is_dir() {
                        for name in ["electronic_bands.csv","electronic_dos.csv","phonon_bands.csv","phonon_dos.csv","metadata.json","metrics.json","manifest.json"] {
                            let file=p.join(name);if file.is_file(){fs::remove_file(file)?;}
                        }
                    }
                }
                interpolation::comparison::export(&source, &output)?;
                crate::delivery::comparison::collect(&cfg.work_dir, None)?;
            }
            crate::delivery::comparison::collect(&cfg.work_dir, None)?;
            super::combined_plot::coarse(cfg, &results)?;
            vec![
                native_reference(&output.join("manifest.json"))?,
                native_reference(&super::stage_results::root(cfg).join("6.epw-coarse/phonon.plot.json"))?,
            ]
        };
        save_post(cfg, step, &outputs)?;
    }
    if target == Step::Superconductivity && cfg.superconductivity.task==super::config::SuperconductivityTask::Gap && stages::reusable(cfg, Step::Superconductivity)? {
        let result = super::stage_results::root(cfg).join("8.epw-superconductivity");
        crate::delivery::finalize_to(&cfg.work_dir, Some(&result))?;
        super::stage_results::tree(
            cfg,
            Step::Superconductivity,
            "",
            &result,
            json!({"kind":"native_superconductivity"}),
        )?;
    }

    Ok(())
}

pub fn generate(cfg: &Config, target: Step) -> Result<()> {
    let qe = QeSettings::from_config(cfg)?;
    for step in stages::execution_steps(cfg, target) {
        if stages::reusable(cfg, step)? {
            continue;
        }
        let dir = step.directory(cfg);
        fs::create_dir_all(&dir)?;
        match step {
            Step::Scf => fs::write(dir.join("scf.in"), qe.scf_input("./out")?)?,
            Step::Nscf => fs::write(dir.join("nscf.in"), qe.nscf_input("./out")?)?,
            Step::Electronic => super::workspace_electronic::inputs(cfg, &qe)?,
            Step::Phonon => {
                if cfg.phonon.mode == PhMode::Nosplit {
                    fs::write(dir.join("ph.in"), qe.phonon_nosplit_input("./out")?)?;
                } else {
                    fs::create_dir_all(dir.join("init"))?;
                    let input = dir.join("init/ph_init.in");
                    fs::write(&input, qe.phonon_init_input("./out")?)?;
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
                            &if step==Step::Coarse {super::workspace_data::relative(&dir,&Step::Phonon.directory(cfg).join("save"))?.to_string_lossy().into_owned()} else {"./save".into()},
                            cfg.interpolation.phonon_smearings_mev[0],
                        )?,
                    )?;
                }
            }
            Step::Superconductivity => {}
        }
    }
    Ok(())
}

pub fn run(cfg: &Config, step: Step) -> Result<()> {
    super::electronic_reuse::invalidate(step);
    let root = &cfg.work_dir;
    let qe = QeSettings::from_config(cfg)?;
    match step {
        Step::Scf => {
            qe_data::scf::run_scf(&qe, root, &cfg.programs.launcher, Backend::Local)?;
            // 自动绘制 SCF 能带范围图
            if let Err(e) = super::workspace_qe_plot::refresh(cfg, step) {
                eprintln!("Warning: SCF plotting failed: {:#}", e);
                let _ = super::readable_state::update(root, step, |d| {
                    d.set("Plot · SCF", "Status", "failed");
                    d.set("Plot · SCF", "Failure", format!("{:#}", e));
                    Ok(())
                });
            }
        }
        Step::Nscf => {
            qe_data::nscf::run_nscf(&qe, root, &cfg.programs.launcher, Backend::Local)?;
            // 自动绘制 NSCF 能带范围图
            if let Err(e) = super::workspace_qe_plot::refresh(cfg, step) {
                eprintln!("Warning: NSCF plotting failed: {:#}", e);
                let _ = super::readable_state::update(root, step, |d| {
                    d.set("Plot · NSCF", "Status", "failed");
                    d.set("Plot · NSCF", "Failure", format!("{:#}", e));
                    Ok(())
                });
            }
        }
        Step::Electronic => {
            run_electronic_leaf(cfg, "qe_eband")?;
            run_electronic_leaf(cfg, "qe_eledos")?;
            // 自动绘制 Electronic 全部图（能带范围、能带、DOS、PDOS）
            if let Err(e) = super::workspace_qe_plot::refresh(cfg, step) {
                eprintln!("Warning: Electronic plotting failed: {:#}", e);
                let _ = super::readable_state::update(root, step, |d| {
                    d.set("Plot · Electronic", "Status", "failed");
                    d.set("Plot · Electronic", "Failure", format!("{:#}", e));
                    Ok(())
                });
            }
        }
        Step::Phonon => {
            run_phonon(cfg, &qe)?;
            // 自动绘制声子能带图 + 声子 DOS 图
            if let Err(e) = super::workspace_qe_plot::refresh(cfg, step) {
                eprintln!("Warning: Phonon plotting failed: {:#}", e);
                let _ = super::readable_state::update(root, step, |d| {
                    d.set("Plot · Phonon", "Status", "failed");
                    d.set("Plot · Phonon", "Failure", format!("{:#}", e));
                    Ok(())
                });
            }
        }
        Step::Wannier => {
            run_wannier(cfg, &qe)?;
            // 自动绘制四类对比图
            if let Err(e) = refresh_post(cfg, step) {
                eprintln!("Warning: Wannier plotting failed: {:#}", e);
                // refresh_post 内部已处理失败状态记录
            }
        }
        Step::Coarse => {
            interpolation::scheduler::run_coarse_with_retry(
                cfg,
                Backend::Local,
                &cfg.programs.launcher,
                root,
                root,
                true,
            )?;
            // 自动绘制四类对比图 + 实空间衰减图
            if let Err(e) = refresh_post(cfg, step) {
                eprintln!("Warning: Coarse plotting failed: {:#}", e);
                // refresh_post 内部已处理失败状态记录
            }
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
            crate::superconductivity::scheduler::run_configured(
                cfg,
                root,
                &cfg.programs.launcher,
                Backend::Local,
                true,
            )?;
            if cfg.superconductivity.task==super::config::SuperconductivityTask::Tc {return Ok(());}
            let result=crate::delivery::finalize_to(root,Some(&super::stage_results::root(cfg).join("8.epw-superconductivity")))?;
            super::stage_results::tree(
                cfg,
                Step::Superconductivity,
                "",
                &result,
                json!({"kind":"native_superconductivity"}),
            )?;
        }
    }
    Ok(())
}

pub(super) fn partial_phonon(cfg: &Config) -> Result<Option<qe_data::direct::PhononData>> {
    if !Step::Phonon.directory(cfg).join("save").is_dir() {
        return Ok(None);
    }
    qe_data::direct::phonon(cfg).map(Some)
}

fn collect_phonon(cfg: &Config, qe: &QeSettings) -> Result<qe_data::direct::PhononData> {
    if super::phonon_import::bound(cfg) {
        return super::phonon_import::collect(cfg, qe).map(|m| qe_data::direct::PhononData{phonon_files:m.phonon_files});
    }
    let root = &cfg.work_dir;
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
    let data=qe_data::collect_ph::collect(root,&points,&responses,&cfg.qe.prefix,&qe_data::phonon::geometry(qe,root)?)?;
    Ok(qe_data::direct::PhononData{phonon_files:data.responses})
}

fn run_phonon(cfg: &Config, qe: &QeSettings) -> Result<()> {
    let manifest = match partial_phonon(cfg)? {
        Some(m) => {
            super::readable_state::report_reuse(cfg, Step::Phonon, Some("PH responses and save manifest; continuing q2r/matdyn"));
            m
        }
        None => collect_phonon(cfg, qe)?,
    };
    let dir = Step::Phonon.directory(cfg);
    // q2r 消费严格归集的同一组 XML；coords 局部编号已在 save 中规范化。
    fs::create_dir_all(&dir)?;
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
    shared.qe_phonon.phband.insert(
        "kpath_points".into(),
        cfg.native_value::<serde_json::Value>(&[
            &cfg.phonon_stage(),
            "workflow",
            "points_per_edge",
        ])?,
    );
    qf_input::q2r::write_q2r_input(dir.to_str().unwrap(), &shared, &cfg.qe.prefix)?;
    qf_input::matdyn::write_matdyn_phband_input(
        dir.to_str().unwrap(),
        &shared,
        &cfg.qe.prefix,
        qe.cell.lattice,
    )?;
    qf_input::matdyn::write_matdyn_phdos_input(dir.to_str().unwrap(), &shared, &cfg.qe.prefix)?;
    let stage = cfg.phonon_stage();
    cfg.configure_file(&[&stage, "q2r"], &dir.join("q2r.in"))?;
    // 显式列表直接消费归集的 dyn_q XML；不存在该标识的 dyn0，q2r 从输入读取网格。
    let q2r=fs::read_to_string(dir.join("q2r.in"))?;
    let field=regex::Regex::new(r"(?im)^\s*fildyn\s*=.*$")?;
    let mut q2r=field.replace(&q2r," fildyn='save/explicit-dynamical-matrices.xml'").into_owned();
    q2r.push_str(&format!("{} {} {}\n{}\n",cfg.phonon.q_grid[0],cfg.phonon.q_grid[1],cfg.phonon.q_grid[2],manifest.phonon_files.len()));
    for response in &manifest.phonon_files {
        let name=super::workspace_data::relative(&dir,&response.dyn_file)?;
        q2r.push_str(&format!("{}\n",name.with_extension("").display()));
    }
    fs::write(dir.join("q2r.in"),q2r)?;

    cfg.configure_file(
        &[&stage, "dispersion", "matdyn"],
        &dir.join("matdyn_phband.in"),
    )?;
    cfg.configure_file(&[&stage, "dos", "matdyn"], &dir.join("matdyn_phdos.in"))?;
    let tasks = [
        ("q2r", "q2r.in", "q2r"),
        ("matdyn", "matdyn_phband.in", "matdyn_phband"),
        ("matdyn", "matdyn_phdos.in", "matdyn_phdos"),
    ];
    for (program, input, stem) in tasks {
        let output = match stem {
            "q2r" => dir.join(format!("{}.fc.xml", cfg.qe.prefix)),
            "matdyn_phband" => dir.join(format!("{}.phband.freq", cfg.qe.prefix)),
            _ => dir.join(format!("{}.phdos.dat", cfg.qe.prefix)),
        };
        super::native_jobs::require_outputs(cfg, &dir, stem, &[output])?;
        qe_run(cfg, &dir, &post_program(cfg, program)?, input, stem)?;
    }
    let outputs = [
        format!("{}.fc.xml", cfg.qe.prefix),
        format!("{}.phband.freq", cfg.qe.prefix),
        format!("{}_band.labelinfo.dat", cfg.qe.prefix),
        format!("{}.phdos.dat", cfg.qe.prefix),
    ]
    .iter()
    .map(|n| dir.join(n))
    .collect::<Vec<_>>();
    super::readable_state::complete_files(cfg, Step::Phonon, "Phonon postprocessing", &outputs)?;
    Ok(())
}

fn run_wannier(cfg: &Config, qe: &QeSettings) -> Result<()> {
    let dir = Step::Wannier.directory(cfg);
    let s = states(cfg)?;
    let epw = EpwConfig::from_config(cfg)?
        .with_fermi(s.fermi)?
        .with_bands(&s.eigenvalues)?
        .with_species(&qe.cell.elements)?;
    if !super::native_jobs::continuing(&dir, "epw_wannier")? {
        super::reuse_files::electronic(cfg,&layout::path(&cfg.work_dir,"nscf/out"),&dir.join("out"),true)?;
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
    let execution = interpolation::coarse::run_epw(
        &epw,
        &dir,
        "epw_wannier.in",
        &cfg.programs.launcher,
        Backend::Local,
    );
    if execution.as_ref().is_err_and(super::native_jobs::paused) {
        return execution;
    }
    let result = execution.and_then(|_| -> Result<()> {
        let prefix = &cfg.qe.prefix;
        interpolation::wannier::check_native(
            &dir.join(format!("{prefix}.wout")),
            cfg.wannier.num_wann as usize,
        )?;
        epw.check_native_windows(&fs::read_to_string(dir.join(format!("{prefix}.wout")))?)?;
        validate_ukk(&dir.join(format!("{prefix}.ukk")), cfg)?;
        interpolation::restart::velocity_files(
            &dir,
            prefix,
            s.kpoints.len(),
            cfg.qe.nscf.nbnd as usize - cfg.wannier.exclude_bands.len(),
        )?;

        super::workspace_data::release_electronic_work(cfg,&dir)?;
        Ok(())
    });
    if result.is_err() {
        if let Err(plot_error) = super::wannier_diagnostics::generate(cfg) {
            eprintln!("Wannier diagnostic_plots: {plot_error:#}");
        }
    }
    result
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
        "input_conflict: ukk k Points and Current NSCF It's incompatible"
    );
    let expected = (1..=cfg.qe.nscf.nbnd as usize)
        .filter(|n| !cfg.wannier.exclude_bands.contains(&(*n as u32)))
        .collect::<Vec<_>>();
    ensure!(
        kept == expected,
        "input_conflict: ukk Disclose Belt is incompatible with the current configuration"
    );
    let raw = fs::read_to_string(path)?;
    let first = raw
        .lines()
        .next()
        .context("data_invalid: Empty ukk")?
        .split_whitespace()
        .map(str::parse::<usize>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(
        first.len() == 2
            && first[0] > 0
            && first[0] <= cfg.qe.nscf.nbnd as usize
            && first[1] <= cfg.qe.nscf.nbnd as usize,
        "data_invalid: ukk It's not compatible with space"
    );
    ensure!(
        raw.lines().count() > cfg.wannier.num_wann as usize,
        "data_invalid: ukk Data incomplete"
    );
    Ok(())
}

/// EPW 库直接传入这些数据，独立 W90 则必须在 win 中读取；保留 NSCF 点序。
fn standalone_inputs(cfg: &Config, original: &str) -> Result<(String, String)> {
    let s = states(cfg)?;
    let xml = s.wavefunctions.join("data-file-schema.xml");
    let geometry = qe_data::xml::Geometry::read(&xml)?;
    geometry.validate(&QeSettings::from_config(cfg)?)?;
    let mut content = original.to_owned();
    for block in ["unit_cell_cart", "atoms_cart", "atoms_frac", "kpoints"] {
        content = regex::Regex::new(&format!(
            r"(?ims)^\s*begin\s+{block}\s*$.*?^\s*end\s+{block}\s*$"
        ))?
        .replace_all(&content, "")
        .into_owned();
    }
    // 旧生成器误把 W90 内部变量作为输入关键字；恢复自己的旧 win 时清除。
    content = regex::Regex::new(r"(?m)^\s*dos_smr_index\s*=[^\n]*\n?")?
        .replace_all(&content, "")
        .into_owned();
    content = wannier90flow::post::update_win_content(
        &content,
        &[
            (
                "num_bands".into(),
                json!(s.nbnd - cfg.wannier.exclude_bands.len()),
            ),
            (
                "mp_grid".into(),
                json!(format!("{} {} {}", s.k_grid[0], s.k_grid[1], s.k_grid[2])),
            ),
        ],
    );
    content.push_str("\nbegin unit_cell_cart\nAng\n");
    for row in geometry.lattice {
        let [x, y, z] = row.map(|v| v * qe_data::xml::BOHR_ANGSTROM);
        content.push_str(&format!("{x:.16e} {y:.16e} {z:.16e}\n"));
    }
    content.push_str("end unit_cell_cart\n\nbegin atoms_cart\nAng\n");
    for (name, position) in &geometry.atoms {
        let [x, y, z] = position.map(|v| v * qe_data::xml::BOHR_ANGSTROM);
        content.push_str(&format!("{name} {x:.16e} {y:.16e} {z:.16e}\n"));
    }
    content.push_str("end atoms_cart\n\nbegin kpoints\n");
    for point in s.kpoints {
        let [x, y, z] = geometry.crystal(point);
        content.push_str(&format!("{x:.16e} {y:.16e} {z:.16e}\n"));
    }
    content.push_str("end kpoints\n");
    // 独立程序读取排除后的紧凑带编号，能量沿用 NSCF 的绝对 eV。
    let mut eigenvalues = String::new();
    for (k, energies) in s.eigenvalues.iter().enumerate() {
        let kept = energies
            .iter()
            .enumerate()
            .filter(|(b, _)| !cfg.wannier.exclude_bands.contains(&((*b + 1) as u32)));
        for (b, (_, energy)) in kept.enumerate() {
            eigenvalues.push_str(&format!("{} {} {energy:.16e}\n", b + 1, k + 1));
        }
    }
    Ok((content, eigenvalues))
}

fn check_wannier_post(
    report: &super::process::ExecutionReport,
    dir: &Path,
    prefix: &str,
    label: &str,
) -> Result<()> {
    let stdout = fs::read_to_string(&report.stdout_path)?;
    let stderr = if report.stdout_path == report.stderr_path {
        String::new()
    } else {
        fs::read_to_string(&report.stderr_path)?
    };
    let error_file = dir.join(format!("{prefix}.werr"));
    let error = if error_file.exists() {
        fs::read_to_string(error_file)?
    } else {
        String::new()
    };
    let text = format!("{error}\n{stdout}\n{stderr}");
    let failed = regex::Regex::new(r"(?im)^\s*(?:error\b|exiting\.+)")?;
    let detail = if error.trim().is_empty() {
        text.lines()
            .find(|line| failed.is_match(line))
            .unwrap_or("")
            .trim()
    } else {
        error.trim()
    };
    ensure!(
        report.exit_status == Some(0) && error.is_empty() && detail.is_empty(),
        "execution_failed: {label} Quit Status {:?};{detail}(Inspection {prefix}.werr And the native birthday )",
        report.exit_status
    );
    Ok(())
}

fn electronic_files() -> [&'static str; 9] {
    ["electronic-bands.csv","electronic-dos.csv","electronic-bands.json","electronic-dos.json",
     "electronic-dos-normalization.json","sampling.json","metrics.json","band.eig","band-origin.json"]
}

pub(crate) fn verify_electronic_comparison(cfg: &Config) -> Result<()> {
    let r = read_post(cfg, Step::Wannier)?.context("data_invalid: Electronic comparison not yet completed")?;
    ensure!(
        r.key == post_key(cfg, Step::Wannier)?,
        "input_conflict: I don't think so 5 Step Electronic Comparison Parameter has changed"
    );
    for id in r.outputs {
        id.verify()?;
    }
    Ok(())
}

fn electronic_comparison(cfg: &Config, dir: &Path, fermi: f64) -> Result<()> {
    let post = layout::path(&cfg.work_dir, "comparison-native/electron");
    fs::create_dir_all(&post)?;
    // 科学模型只读；所有 postw90 输入和派生产物位于电子比较目录。
    for ext in ["win", "wout", "chk", "mmn", "amn", "nnkp"] {
        let name = format!("{}.{ext}", cfg.qe.prefix);
        if dir.join(&name).exists() {
            let source = native_reference(&dir.join(&name))?;
            if post!=dir && !post.join(&name).exists() {
                fs::copy(&source.path, post.join(&name))?;
                source.verify()?;
            }
        }
    }
    for suffix in ["_band.dat","_band.kpt","_band.labelinfo.dat"] {
        let source=dir.join(format!("{}{suffix}",cfg.qe.prefix));save::nonempty(&source)?;
        if post!=dir {fs::copy(&source,post.join(source.file_name().unwrap()))?;}
    }
    interpolation::electronic_sampling::extract(cfg,&post)?;
    electronic_comparison_inner(cfg, &post, fermi)?;
    for name in ["electronic-bands.csv","electronic-dos.csv","electronic-dos-normalization.json"] {
        if post.join(name)!=dir.join(name) {fs::copy(post.join(name),dir.join(name))?;}
    }
    let plot = super::stage_results::root(cfg).join("5.epw-wannier");
    super::combined_plot::electronic(cfg,&post,&plot)?;
    super::combined_plot::cleanup(&post)?;
    super::combined_plot::cleanup(dir)?;
    let provenance = [
        interpolation::electronic_sampling::xml(cfg),
        Step::Electronic.directory(cfg).join(format!(
            "qe_eband/out/{}.save/data-file-schema.xml",
            cfg.qe.prefix
        )),
        dir.join(format!("{}.chk", cfg.qe.prefix)),
        dir.join(format!("{}.wout", cfg.qe.prefix)),
    ]
    .iter()
    .map(|p| native_reference(p))
    .collect::<Result<Vec<_>>>()?;
    super::stage_results::publish(cfg,Step::Wannier,"plot",&plot,
        &super::combined_plot::ELECTRONIC_FILES.iter().map(|n|n.to_string()).collect::<Vec<_>>(),
        json!({"kind":"electronic_combined","plot":cfg.plot}),provenance)

}

fn electronic_comparison_inner(cfg: &Config, dir: &Path, fermi: f64) -> Result<()> {
    let prefix = &cfg.qe.prefix;
    let win = dir.join(format!("{prefix}.win"));
    let (original, eigenvalues) = standalone_inputs(cfg, &fs::read_to_string(&win)?)?;
    let options = cfg.comparison.clone().unwrap_or_default();
    let window = interpolation::comparison::parse_outer_window(&fs::read_to_string(
        dir.join(format!("{prefix}.wout")),
    )?)?;
    let mut parameters: serde_json::Map<String, serde_json::Value> =
        cfg.native_value(&[super::native_config::WANNIER, "electron_dos", "postw90"])?;
    for (key, value) in [
        ("dos_energy_min", window[0]),
        ("dos_energy_max", window[1]),
        ("fermi_energy", fermi),
    ] {
        ensure!(
            !parameters.contains_key(key),
            "configuration: stages.5.epw-wannier.electron_dos.postw90.{key}: Duplicate with actual model derived values"
        );
        parameters.insert(key.into(), json!(value));
    }
    let updated = wannier90flow::post::update_win_content(
        &original,
        &parameters.into_iter().collect::<Vec<_>>(),
    );
    let updated = regex::Regex::new(r"(?m)^\s*restart\s*=[^\n]*\n?")?
        .replace_all(&updated, "")
        .into_owned();
    let program = post_program(cfg, "postw90")?;
    let dos = PathBuf::from(wannier90flow::post::resolve_dos_output(
        dir.to_str().unwrap(),
        prefix,
    ));
    let reusable =
        super::readable_state::completed_files(cfg, Step::Wannier, "Electronic substep · postw90")?
            .is_some()
            && super::readable_state::checkpoint_config(
                cfg,
                Step::Wannier,
                "Electronic substep · postw90",
            )?
            .is_some_and(|old| {
                old.native_value::<serde_json::Value>(&[
                    super::native_config::WANNIER,
                    "electron_dos",
                    "postw90",
                ])
                .ok()
                    == cfg
                        .native_value::<serde_json::Value>(&[
                            super::native_config::WANNIER,
                            "electron_dos",
                            "postw90",
                        ])
                        .ok()
            });
    fs::write(&win, &updated)?;
    if !reusable {
        let reason = json!({"schema_version":1,"decision":"necessary_postprocessing",
            "reason":"Existing electronic models do not provide compatible postw90 DOS ;Only the existing flat grid total DOS ,Do Not Rewrite",
            "program":program.path,"model":native_reference(&dir.join(format!("{prefix}.chk")))?,
            "mesh":cfg.qe.nscf.k_grid,"sigma_ev":options.electronic_sigma_ev,"expected_output":dos});
        store::write(&dir.join("dos-decision.json"), &reason)?;
        if !super::native_jobs::continuing(dir, "postw90")? {
            let error = dir.join(format!("{prefix}.werr"));
            if error.exists() {
                fs::remove_file(&error)?;
            }
            fs::write(&win, &updated)?;
            fs::write(dir.join(format!("{prefix}.eig")), eigenvalues)?;
        }
        let mut argv = vec![program.path.to_string_lossy().into_owned()];
        argv.extend(program.args);
        argv.push(prefix.clone());
        let report = super::process::run_report_named(&argv, dir, "postw90")?;
        report.copy_logs(dir, "postw90")?;
        check_wannier_post(&report, dir, prefix, "postw90 DOS")?;
        let actual = PathBuf::from(wannier90flow::post::resolve_dos_output(
            dir.to_str().unwrap(),
            prefix,
        ));
        super::readable_state::complete_files(
            cfg,
            Step::Wannier,
            "Electronic substep · postw90",
            &[actual, dir.join(format!("{prefix}.wpout"))],
        )?;
    }
    let actual = PathBuf::from(wannier90flow::post::resolve_dos_output(
        dir.to_str().unwrap(),
        prefix,
    ));
    super::workspace_plot::bands(cfg, dir, &dir.join("band.eig"), fermi)?;
    super::workspace_plot::dos(cfg, dir, &actual, fermi)?;
    Ok(())
}

/// 科学完成与调度完成分开：每次复用均检查原生文件。
pub fn validate(cfg: &Config, step: Step) -> Result<()> {
    let dir = step.directory(cfg);
    match step {
        Step::Scf => {
            let output = dir.join("scf.out");
            qf_io::qe_completion::checked_log(&output, true)?;
            save::verify_snapshot(&cfg.work_dir)?;
            let energy = qe_data::scf::parse_scf_energy(&fs::read_to_string(output)?)?;
            qe_data::xml::validate_save(
                &layout::path(&cfg.work_dir,"scf/snapshot"),
                &cfg.qe.prefix,
                &QeSettings::from_config(cfg)?,
                Some(energy),
            )?;
        }
        Step::Nscf => {
            qf_io::qe_completion::checked_log(&dir.join("nscf.out"), false)?;
            states(cfg)?;
        }
        Step::Electronic => {
            ensure!(
                super::workspace_electronic::completed(cfg)?,
                "data_invalid: The electronic reference sub-task has not yet been fully checked"
            );
        }
        Step::Phonon => {
            qe_data::direct::phonon(cfg)?;
            for stem in ["q2r", "matdyn_phband", "matdyn_phdos"] {
                qf_io::qe_completion::checked_log(&dir.join(format!("{stem}.out")), false)?;
            }
            for name in [
                format!("{}.fc.xml", cfg.qe.prefix),
                format!("{}.phband.freq", cfg.qe.prefix),
                format!("{}.phdos.dat", cfg.qe.prefix),
            ] {
                save::nonempty(&dir.join(name))?;
            }
            let modes = QeSettings::from_config(cfg)?.cell.positions.len() * 3;
            let frequencies = qf_io::read_band_frequencies(
                &fs::read_to_string(dir.join(format!("{}.phband.freq", cfg.qe.prefix)))?,
                modes,
            )?;
            ensure!(!frequencies.is_empty(), "data_invalid: Sound frequency output is empty");
        }
        Step::Wannier => {
            let result = interpolation::wannier::check_native(
                &dir.join(format!("{}.wout", cfg.qe.prefix)),
                cfg.wannier.num_wann as usize,
            )?;
            ensure!(result.converged, "data_invalid: Wannier Unrecovered");
            validate_ukk(&dir.join(format!("{}.ukk", cfg.qe.prefix)), cfg)
                .context("data_invalid: Wannier UKK Format Invalid")?;
            save::nonempty(&dir.join(format!("{}.chk", cfg.qe.prefix)))?;
            let (points,_)=interpolation::restart::manifold_dimensions(&dir.join(format!("{}.ukk",cfg.qe.prefix)),cfg.wannier.num_wann as usize)?;
            interpolation::restart::velocity_files(
                &dir,
                &cfg.qe.prefix,
                points,
                cfg.qe.nscf.nbnd as usize - cfg.wannier.exclude_bands.len(),
            )?;
        }
        Step::Coarse => {
            interpolation::scheduler::verify_coarse(cfg, &cfg.work_dir)?;
        }
        Step::Fine => {
            let manifest = interpolation::manifest::InterpolationManifest::load(&layout::path(
                &cfg.work_dir,
                "interpolation/manifest.json",
            ))?;
            ensure!(
                manifest.fine.is_some() && manifest.validation.all_passed(),
                "data_invalid: The fine grid results are incomplete"
            );
        }
        Step::Superconductivity => {
            if cfg.superconductivity.task==super::config::SuperconductivityTask::Tc {
                let dir=step.directory(cfg).join(crate::superconductivity::batch::directory_name(cfg,super::config::Mode::Iso));
                crate::superconductivity::linear::validate(cfg,&fs::read_to_string(dir.join("epw.out"))?)?;
                return Ok(());
            }
            crate::superconductivity::verify::complete(&cfg.work_dir,cfg,"native")?;
            crate::delivery::native::check(&super::stage_results::root(cfg).join("8.epw-superconductivity"))?;
        }
    }
    Ok(())
}

pub fn evidence(cfg: &Config, step: Step) -> Result<Vec<Identity>> {
    if layout::enabled(&cfg.work_dir) {return Ok(Vec::new());}
    if step == Step::Nscf {
        states(cfg)?;
    }
    let dir = step.directory(cfg);
    let ids = save::files(&dir)?
        .iter()
        .filter(|f| {
            let name = f.relative_path.file_name().unwrap().to_string_lossy();
            let rel = f.relative_path.to_string_lossy();
            if f.relative_path
                .components()
                .any(|p| p.as_os_str() == "plots")
                || name == "qe-post-completion.json"
            {
                return false;
            }
            if step == Step::Wannier
                && !rel.starts_with("out/")
                && ![
                    format!("{}.ukk", cfg.qe.prefix),
                    format!("{}.bvec", cfg.qe.prefix),
                    format!("{}.mmn", cfg.qe.prefix),
                    format!("{}.chk", cfg.qe.prefix),
                    format!("{}.wout", cfg.qe.prefix),
                    format!("{}_hr.dat", cfg.qe.prefix),
                    "embedded-bands.json".into(),
                    "epw_wannier.in".into(),
                    "epw_wannier.out".into(),
                    "epw_wannier.err".into(),
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
            if step==Step::Coarse && (name.starts_with("epw_compare") || matches!(name.as_ref(),"phband.freq"|"band.eig"|"kpoints.dat"|"qpoints.dat"|"reuse-decision.json")) {return false;}
            if matches!(step, Step::Coarse | Step::Superconductivity)
                && (rel.starts_with("batch-finalization.lock/")
                    || rel.starts_with("comparison/")
                    || rel.starts_with("decay/")
                    || rel.starts_with("results/")
                    || rel.starts_with("comparison-native/"))
            {
                return false;
            }
            name != "completion.json"
                && name != "post-completion.json"
                && !rel.starts_with("receipts/")
                && !name.ends_with(".sh")
                && !name.ends_with(".jobs")
                && !name.starts_with("submit-")
                && !super::native_jobs::is_log(&f.relative_path)
                && !name.starts_with("slurm-")
                && !name.starts_with(".epw-")
                && !(step == Step::Phonon && name == "manifest.json")
        })
        .map(|f| native_reference(&dir.join(&f.relative_path)))
        .collect::<Result<Vec<_>>>()?;
    ensure!(!ids.is_empty(), "data_invalid: There's no scientific evidence at the stage");
    Ok(ids)
}

#[cfg(test)]
mod post_program_tests {
    use super::*;

    #[test]
    fn postprocessors_use_only_their_explicit_paths_and_arguments() {
        let mut cfg: Config =
            toml::from_str(include_str!("../../tests/fixtures/input.toml")).unwrap();
        cfg.programs.pw.path = PathBuf::from("/unrelated/pw.x");
        cfg.programs.pw.args = vec!["-nk".into(), "8".into()];
        cfg.programs.bands = Some(Program {
            path: PathBuf::from("/separate directory/bands.x"),
            args: vec!["-pd".into(), ".true.".into()],
        });
        let bands = post_program(&cfg, "bands").unwrap();
        assert_eq!(bands.path, PathBuf::from("/separate directory/bands.x"));
        assert_eq!(bands.args, ["-pd", ".true."]);
        assert!(post_program(&cfg, "dos").unwrap().args.is_empty());
        cfg.programs.matdyn = None;
        assert!(post_program(&cfg, "matdyn").unwrap_err().to_string().contains("programs.matdyn_x"));
    }
}

fn native_reference(path:&Path)->Result<Identity> {Ok(Identity::reference(path))}
