//! 单 SCF 后的独立 PH 保存树与目标 QE 原生输出映射。
use super::super::foundation::{config::PhMode, stage::Backend};
use super::{
    manifest::{PhResponse, QPoint},
    native_ph,
    qe_config::QeSettings,
    qmap, save, xml,
};
use anyhow::{ensure, Result};
use std::path::{Path, PathBuf};

pub fn geometry(qe: &QeSettings, work: &Path) -> Result<xml::Geometry> {
    save::verify_snapshot(work)?;
    xml::Geometry::read(
        &crate::foundation::layout::path(&work, "scf/snapshot")
            .join(format!("{}.save/data-file-schema.xml", qe.config.qe.prefix)),
    )
}

fn execute(
    qe: &QeSettings,
    work: &Path,
    dir: &Path,
    input: &str,
    name: &str,
    launcher: &[String],
    backend: Backend,
) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    save::stage_scf(work, &dir.join("out"), &qe.config.qe.prefix)?;
    let file = dir.join(format!("{name}.in"));
    std::fs::write(&file, input)?;
    super::execution::run(
        &qe.config.programs.ph,
        &file,
        dir,
        launcher,
        backend,
        name,
        false,
    )?;
    save::verify_snapshot(work)
}

fn dyn0(qe: &QeSettings, dir: &Path, geometry: &xml::Geometry) -> Result<Vec<QPoint>> {
    qmap::parse_dyn0(
        &dir.join(format!("{}.dyn0", qe.config.qe.prefix)),
        geometry,
        qe.config.phonon.q_grid,
        matches!(qe.config.phonon.mode, PhMode::Coords),
    )
}

fn control(qe: &QeSettings, dir: &Path) -> PathBuf {
    dir.join("out/_ph0")
        .join(format!("{}.phsave/control_ph.xml", qe.config.qe.prefix))
}

pub(crate) fn response(
    qe: &QeSettings,
    dir: &Path,
    q: &QPoint,
    control: &Path,
    geometry: &xml::Geometry,
) -> Result<PhResponse> {
    let prefix = &qe.config.qe.prefix;
    let root = dir.join("out/_ph0");
    let gamma = q.crystal.iter().all(|x| x.abs() < 1e-9);
    // QE 对 ldisp=false 且非 qplot 的单 q 强制 lqdir=false。
    let dv_dir = if gamma || matches!(qe.config.phonon.mode, PhMode::Coords) {
        root.clone()
    } else {
        root.join(format!("{prefix}.q_{}", q.local_index))
    };
    let candidates = [
        dv_dir.join(format!("{prefix}.dvscf")),
        dv_dir.join(format!("{prefix}.dvscf1")),
    ];
    let files = candidates
        .iter()
        .filter(|p| p.is_file())
        .collect::<Vec<_>>();
    ensure!(
        files.len() == 1,
        "data_invalid: 缺少或存在歧义的原生 dvscf q={}",
        q.global_index
    );
    // coords 的 fildyn 为不带全局编号的单 q 文件，内部 q 固定为 1。
    let dyn_name = if matches!(qe.config.phonon.mode, PhMode::Coords) {
        format!("{prefix}.dyn.xml")
    } else {
        format!("{prefix}.dyn{}.xml", q.global_index)
    };
    let result = PhResponse {
        q_id: q.global_index,
        dyn_file: dir.join(dyn_name),
        patterns: root.join(format!("{prefix}.phsave/patterns.{}.xml", q.local_index)),
        dvscf: Some(files[0].to_path_buf()),
        control_metadata: control.into(),
        format: native_ph::FORMAT.into(),
        perturbations: geometry.atoms.len() * 3,
    };
    native_ph::verify(&result, q, geometry)?;
    Ok(result)
}

pub fn run_init(
    qe: &QeSettings,
    work: &Path,
    launcher: &[String],
    backend: Backend,
) -> Result<Vec<QPoint>> {
    let dir = crate::foundation::layout::path(&work, "ph_init");
    let input = qe.phonon_init_input(dir.join("out").to_str().unwrap());
    execute(qe, work, &dir, &input, "ph_init", launcher, backend)?;
    let geometry = geometry(qe, work)?;
    let points = dyn0(qe, &dir, &geometry)?;
    native_ph::control(&control(qe, &dir), &points)?;
    Ok(points)
}

pub fn run_q(
    qe: &QeSettings,
    work: &Path,
    launcher: &[String],
    backend: Backend,
    q: &QPoint,
    q_index: usize,
) -> Result<PhResponse> {
    ensure!(q_index == q.global_index, "data_invalid: 分 q 全局索引不符");
    let dir = crate::foundation::layout::path(&work, "ph").join(&q.directory);
    let input = qe.phonon_q_input(
        dir.join("out").to_str().unwrap(),
        &qe.config.phonon.mode,
        Some(q),
        q_index,
    )?;
    execute(qe, work, &dir, &input, "ph", launcher, backend)?;
    response(
        qe,
        &dir,
        q,
        &control(qe, &crate::foundation::layout::path(&work, "ph_init")),
        &geometry(qe, work)?,
    )
}

/// nosplit 自己产生完整 dyn0 与 control，不增加一遍 only_init 计算。
pub fn run_nosplit(
    qe: &QeSettings,
    work: &Path,
    launcher: &[String],
    backend: Backend,
) -> Result<(Vec<QPoint>, Vec<PhResponse>)> {
    let dir = crate::foundation::layout::path(&work, "ph_nosplit");
    let input = qe.phonon_nosplit_input(dir.join("out").to_str().unwrap());
    execute(qe, work, &dir, &input, "ph", launcher, backend)?;
    let geometry = geometry(qe, work)?;
    let points = dyn0(qe, &dir, &geometry)?;
    let meta = control(qe, &dir);
    native_ph::control(&meta, &points)?;
    let responses = points
        .iter()
        .map(|q| response(qe, &dir, q, &meta, &geometry))
        .collect::<Result<Vec<_>>>()?;
    Ok((points, responses))
}
