//! EPW 内置 Wannier 化与粗网格，消费已逐文件核验的 F2 原生数据。
use super::super::foundation::{stage::Backend, store, scientific_logs};
use super::{
    epw_config::EpwConfig,
    manifest::{Artifact, CoarseData},
    restart,
};
use crate::qe_data::{manifest::QeManifest, save};
use anyhow::{ensure, Context, Result};
use std::{fs, path::Path};

#[derive(Debug, Clone)]
pub struct CoarseResult {
    pub kpoints: usize,
    pub qpoints: usize,
    pub nwann: usize,
    pub artifacts: Vec<Artifact>,
}

pub fn stage_f2(f2: &QeManifest, work: &Path) -> Result<()> {
    f2.verify()?;
    let dir = crate::foundation::layout::path(&work, "coarse");
    ensure!(
        crate::foundation::native_jobs::empty_execution_directory(&dir)?
            || (crate::foundation::layout::enabled(work)
                && !dir.join("out").exists()
                && !scientific_logs::output(&dir, "epw_coarse").exists()),
        "input_conflict: The coarse grid target is in place. Keep it on site"
    );
    let states = f2.states.as_ref().context("data_invalid: Missing F2 NSCF")?;
    save::copy_tree(
        states
            .wavefunctions
            .parent()
            .context("data_invalid: NSCF Missing Parent Directory")?,
        &dir.join("out"),
    )?;
    let ph_dir = f2
        .phonon_files
        .first()
        .context("data_invalid: Missing F2 PH")?
        .dyn_file
        .parent()
        .context("data_invalid: PH Missing Parent Directory")?;
    save::copy_tree(ph_dir, &dir.join("save"))?;
    if crate::foundation::layout::enabled(work) {
        let source = work.join("5.epw-wannier");
        let completion = crate::foundation::stages::completion(
            &crate::qe_data::direct::current(work)?,
            crate::foundation::stages::Step::Wannier,
        )?;
        completion.verify()?;
        let prefix = states
            .wavefunctions
            .file_stem()
            .context("data_invalid: NSCF No prefix")?
            .to_string_lossy();
        for suffix in ["ukk", "wout", "bvec", "mmn"] {
            let name = format!("{prefix}.{suffix}");
            let identity = completion
                .outputs
                .iter()
                .find(|id| id.path == source.join(&name))
                .with_context(|| {
                    format!("input_conflict: Wannier The receipt was not tied {name},It has to be rebuilt by a public order 5 Step")
                })?;
            identity.verify()?;
            save::nonempty(&source.join(&name))?;
            fs::copy(source.join(&name), dir.join(&name))?;
            ensure!(
                store::hash_file(&dir.join(&name))? == identity.sha256,
                "data_invalid: Wannier The identity does not match after copying the supporting file:{name}"
            );
        }
    }
    f2.verify()
}

pub fn run_epw(
    epw_cfg: &EpwConfig,
    dir: &Path,
    input_name: &str,
    launcher: &[String],
    backend: Backend,
) -> Result<()> {
    ensure!(
        backend == Backend::Local,
        "allocation: EPW To be in F1 Synchronize in registered distribution"
    );
    let program = &epw_cfg.cfg.programs.epw;
    let stem = Path::new(input_name).file_stem().and_then(|s| s.to_str())
        .context("configuration: EPW Enter missing master name")?;
    crate::foundation::workspace::native_script(
        program,
        &dir.join(input_name),
        dir,
        launcher,
        stem,
        false,
        false,
    )?;
    let mut cmd = launcher.to_vec();
    cmd.push(program.path.to_string_lossy().into_owned());
    cmd.extend(program.args.clone());
    cmd.extend([
        "-inp".into(),
        dir.join(input_name).to_string_lossy().into_owned(),
    ]);
    let report = super::super::foundation::process::run_report_named(&cmd, dir, stem)?;
    if !crate::foundation::native_jobs::enabled() {
        report.copy_logs(dir, stem)?;
    }
    let output = if crate::foundation::native_jobs::enabled() {
        report.stdout_path.clone()
    } else { dir.join(format!("{stem}.out")) };
    fs::write(
        output.with_extension("exitcode"),
        report
            .exit_status
            .map(|x| x.to_string())
            .unwrap_or_else(|| "signal".into()),
    )?;
    ensure!(
        report.exit_status == Some(0),
        "execution_failed: EPW Quit Status {:?}",
        report.exit_status
    );
    super::native::checked_log(&fs::read_to_string(output)?, 0)
}

pub fn run_coarse(
    epw_cfg: &EpwConfig,
    work: &Path,
    launcher: &[String],
    backend: Backend,
) -> Result<CoarseResult> {
    prepare_coarse(epw_cfg, work)?;
    let dir = crate::foundation::layout::path(work, "coarse");
    if !crate::foundation::layout::enabled(work) {
        run_epw(epw_cfg, &dir, "epw_wannier.in", launcher, backend)?;
    }
    run_epw(
        epw_cfg,
        &crate::foundation::layout::path(&work, "coarse"),
        "epw_coarse.in",
        launcher,
        backend,
    )?;
    collect_coarse(work, epw_cfg)
}

pub(super) fn prepare_coarse(epw_cfg: &EpwConfig, work: &Path) -> Result<()> {
    let dir = crate::foundation::layout::path(&work, "coarse");
    ensure!(
        dir.join("out").is_dir() && dir.join("save").is_dir(),
        "data_invalid: coarse Grid Missing native F2 Save Data"
    );
    ensure!(
        !dir.join("epw_coarse.in").exists() && !scientific_logs::output(&dir, "epw_coarse").exists(),
        "input_conflict: The coarse grid is on site"
    );
    qf_wannier::write_selection(
        &dir.join("window-selection.json"),
        &epw_cfg.window_selection()?,
    )?;
    epw_cfg.require_window_support()?;
    if !crate::foundation::layout::enabled(work) {
        fs::write(
            dir.join("epw_wannier.in"),
            epw_cfg.native_input(
                super::epw_config::NativeStage::WannierOnly,
                "./out",
                "./save",
                epw_cfg.cfg.interpolation.phonon_smearings_mev[0],
            )?,
        )?;
    }
    fs::write(
        dir.join("epw_coarse.in"),
        epw_cfg.native_input(
            super::epw_config::NativeStage::ReuseWannier,
            "./out",
            "./save",
            epw_cfg.cfg.interpolation.phonon_smearings_mev[0],
        )?,
    )?;
    Ok(())
}

pub fn collect_coarse(work: &Path, epw_cfg: &EpwConfig) -> Result<CoarseResult> {
    let dir = crate::foundation::layout::path(&work, "coarse");
    let expected = epw_cfg.native_input(super::epw_config::NativeStage::ReuseWannier, "./out", "./save", epw_cfg.cfg.interpolation.phonon_smearings_mev[0])?;
    let normalize = |text: &str| text.lines().map(|line| line.split('!').next().unwrap_or("")).flat_map(str::split_whitespace).collect::<Vec<_>>().join(" ");
    ensure!(normalize(&fs::read_to_string(dir.join("epw_coarse.in"))?) == normalize(&expected),
        "input_conflict: The coarse grid actually entered does not match the current parameter");
    super::native::checked_log(&fs::read_to_string(scientific_logs::output(&dir, "epw_coarse"))?, 0)?;
    let cfg = epw_cfg.cfg;
    let structure = crate::qe_data::qe_config::QeSettings::from_config(cfg)?;
    let modes = structure.cell.positions.len() * 3;
    let mut paths = restart::files(
        &dir,
        &dir.join("out"),
        &cfg.qe.prefix,
        cfg.wannier.num_wann as usize,
        modes,
    )?;
    paths.extend([
        ("coarse_log".into(), scientific_logs::output(&dir, "epw_coarse")),
        ("coarse_input".into(), dir.join("epw_coarse.in")),
        (
            "coarse_wannier".into(),
            dir.join(format!("{}.wout", cfg.qe.prefix)),
        ),
    ]);
    super::wannier::check_native(
        &dir.join(format!("{}.wout", cfg.qe.prefix)),
        cfg.wannier.num_wann as usize,
    )?;
    epw_cfg.check_native_windows(&fs::read_to_string(
        dir.join(format!("{}.wout", cfg.qe.prefix)),
    )?)?;
    let digest = store::hash_file(&dir.join("epw_coarse.in"))?;
    let artifacts = paths
        .into_iter()
        .map(|(role, p)| {
            save::nonempty(&p)?;
            Ok(Artifact {
                role,
                relative_path: p.strip_prefix(work)?.into(),
                byte_size: fs::metadata(&p)?.len(),
                sha256: store::hash_file(&p)?,
                producer_stage: "coarse".into(),
                input_digest: digest.clone(),
                format_version: restart::FORMAT.into(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(CoarseResult {
        kpoints: cfg.qe.nscf.k_grid.iter().map(|x| *x as usize).product(),
        qpoints: cfg.phonon.q_grid.iter().map(|x| *x as usize).product(),
        nwann: cfg.wannier.num_wann as usize,
        artifacts,
    })
}

pub fn into_coarse_data(r: &CoarseResult) -> CoarseData {
    CoarseData {
        kpoints: r.kpoints,
        qpoints: r.qpoints,
        nwann: r.nwann,
        ephmat_role: "coarse_epmatwp".into(),
        artifacts: r.artifacts.clone(),
    }
}
