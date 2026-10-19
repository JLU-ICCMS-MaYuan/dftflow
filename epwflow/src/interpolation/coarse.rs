//! EPW 内置 Wannier 化与粗网格，消费已逐文件核验的 F2 原生数据。
use super::super::foundation::{stage::Backend, store};
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
    let dir = work.join("coarse");
    ensure!(!dir.exists(), "input_conflict: 粗网格目标已存在，保留现场");
    let states = f2.states.as_ref().context("data_invalid: 缺少 F2 NSCF")?;
    save::copy_tree(
        states
            .wavefunctions
            .parent()
            .context("data_invalid: NSCF 缺父目录")?,
        &dir.join("out"),
    )?;
    let ph_dir = f2
        .phonon_files
        .first()
        .context("data_invalid: 缺少 F2 PH")?
        .dyn_file
        .parent()
        .context("data_invalid: PH 缺父目录")?;
    save::copy_tree(ph_dir, &dir.join("save"))?;
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
        "allocation: EPW 须在 F1 已登记分配内同步运行"
    );
    let program = &epw_cfg.cfg.programs.epw;
    let mut cmd = launcher.to_vec();
    cmd.push(program.path.to_string_lossy().into_owned());
    cmd.extend(program.args.clone());
    cmd.extend([
        "-inp".into(),
        dir.join(input_name).to_string_lossy().into_owned(),
    ]);
    let report = super::super::foundation::process::run_report(&cmd, dir)?;
    fs::copy(report.stdout_path, dir.join("epw.out"))?;
    fs::copy(report.stderr_path, dir.join("epw.err"))?;
    fs::write(
        dir.join("epw.exitcode"),
        report
            .exit_status
            .map(|x| x.to_string())
            .unwrap_or_else(|| "signal".into()),
    )?;
    ensure!(
        report.exit_status == Some(0),
        "execution_failed: EPW 退出状态 {:?}",
        report.exit_status
    );
    super::native::checked_log(&fs::read_to_string(dir.join("epw.out"))?, 0)
}

pub fn run_coarse(
    epw_cfg: &EpwConfig,
    work: &Path,
    launcher: &[String],
    backend: Backend,
) -> Result<CoarseResult> {
    prepare_coarse(epw_cfg, work)?;
    run_epw(epw_cfg, &work.join("coarse"), "epw_coarse.in", launcher, backend)?;
    collect_coarse(work, epw_cfg)
}

pub(super) fn prepare_coarse(epw_cfg: &EpwConfig, work: &Path) -> Result<()> {
    let dir = work.join("coarse");
    ensure!(
        dir.join("out").is_dir() && dir.join("save").is_dir(),
        "data_invalid: 粗网格缺少原生 F2 保存数据"
    );
    ensure!(
        !dir.join("epw_coarse.in").exists() && !dir.join("epw.out").exists(),
        "input_conflict: 粗网格已有执行现场"
    );
    qf_wannier::write_selection(
        &dir.join("window-selection.json"),
        &epw_cfg.window_selection()?,
    )?;
    epw_cfg.require_window_support()?;
    epw_cfg.write_quality_points(&dir)?;
    fs::write(
        dir.join("epw_coarse.in"),
        epw_cfg.epw_coarse_input("./out")?,
    )?;
    Ok(())
}

/// 隔离重读已验证的原生粗网格；只生成质量点输出，保留原 Wannier 结果。
/// 调用方仍须核对来源的物理参数和规范，不能据此替代 F2 导入绑定。
pub fn reread_quality(
    epw_cfg: &EpwConfig,
    source: &Path,
    source_out: &Path,
    work: &Path,
    launcher: &[String],
    backend: Backend,
) -> Result<CoarseResult> {
    let cfg = epw_cfg.cfg;
    let structure = crate::qe_data::qe_config::QeSettings::from_config(cfg)?;
    let wout = source.join(format!("{}.wout", cfg.qe.prefix));
    super::wannier::check_native(&wout, cfg.wannier.num_wann as usize)?;
    let original = store::Identity::capture(&wout)?;
    let dir = work.join("coarse");
    restart::stage(source, source_out, &cfg.qe.prefix,
        cfg.wannier.num_wann as usize, structure.cell.positions.len() * 3, &dir)?;
    fs::copy(&wout, dir.join(wout.file_name().unwrap()))?;
    original.verify()?;
    epw_cfg.write_quality_points(&dir)?;
    fs::write(dir.join("epw_coarse.in"), epw_cfg.native_input(
        super::epw_config::NativeStage::RestartQuality, "./out", "./save",
        cfg.interpolation.phonon_smearings_mev[0])?)?;
    run_epw(epw_cfg, &dir, "epw_coarse.in", launcher, backend)?;
    collect_coarse(work, epw_cfg)
}

pub fn collect_coarse(work: &Path, epw_cfg: &EpwConfig) -> Result<CoarseResult> {
    let dir = work.join("coarse");
    super::native::checked_log(&fs::read_to_string(dir.join("epw.out"))?, 0)?;
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
        ("coarse_log".into(), dir.join("epw.out")),
        ("coarse_input".into(), dir.join("epw_coarse.in")),
        (
            "coarse_wannier".into(),
            dir.join(format!("{}.wout", cfg.qe.prefix)),
        ),
        ("coarse_electron_quality".into(), dir.join("band.eig")),
        ("coarse_phonon_quality".into(), dir.join("phband.freq")),
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
