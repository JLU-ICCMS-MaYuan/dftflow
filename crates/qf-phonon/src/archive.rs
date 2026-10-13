//! 声子后处理归档（对齐 `qeflow/phonon/archive.py`）。
//! 把 matdyn 产物拷入 `phband/` 与 `phdos/` 子目录，并生成归档命令供运行脚本复用。

use anyhow::Result;
use std::path::Path;

pub const PHBAND_DIR: &str = "phband";
pub const PHDOS_DIR: &str = "phdos";

fn copy_required(workspace: &str, target_dir_name: &str, filenames: &[&str]) -> Result<()> {
    let target = format!("{workspace}/{target_dir_name}");
    std::fs::create_dir_all(&target)?;
    for filename in filenames {
        if !Path::new(&format!("{workspace}/{filename}")).exists() {
            anyhow::bail!("缺少待归档文件: {filename}");
        }
        std::fs::copy(
            format!("{workspace}/{filename}"),
            format!("{target}/{filename}"),
        )?;
    }
    Ok(())
}

/// 归档声子谱（`.phband.freq` 与 `_band.labelinfo.dat`）。
pub fn collect_phband_outputs(workspace: &str, prefix: &str) -> Result<()> {
    let names = qf_input::filenames::derive_filenames(prefix);
    copy_required(
        workspace,
        PHBAND_DIR,
        &[names.flfrq.as_str(), &format!("{prefix}_band.labelinfo.dat")],
    )
}

/// 归档声子 DOS（`.phdos.dat`）。
pub fn collect_phdos_outputs(workspace: &str, prefix: &str) -> Result<()> {
    let names = qf_input::filenames::derive_filenames(prefix);
    copy_required(workspace, PHDOS_DIR, &[names.fldos.as_str()])
}

/// 生成归档声子谱所需的 shell 命令（供运行脚本写入）。
pub fn build_collect_phband_commands(prefix: &str) -> Vec<String> {
    let names = qf_input::filenames::derive_filenames(prefix);
    vec![
        format!("mkdir -p {PHBAND_DIR}"),
        format!("cp -f {} {PHBAND_DIR}/", names.flfrq),
        format!("cp -f {prefix}_band.labelinfo.dat {PHBAND_DIR}/"),
    ]
}

/// 生成归档声子 DOS 所需的 shell 命令。
pub fn build_collect_phdos_commands(prefix: &str) -> Vec<String> {
    let names = qf_input::filenames::derive_filenames(prefix);
    vec![
        format!("mkdir -p {PHDOS_DIR}"),
        format!("cp -f {} {PHDOS_DIR}/", names.fldos),
    ]
}

/// 归档产物文件清单（用于状态判定与清洗）。
pub fn phonon_plot_files() -> Vec<String> {
    vec![
        format!("{PHBAND_DIR}/phonon_band.csv"),
        format!("{PHBAND_DIR}/phonon_band_labels.csv"),
        format!("{PHBAND_DIR}/phonon_band.png"),
        format!("{PHDOS_DIR}/phonon_dos_atoms.csv"),
        format!("{PHDOS_DIR}/phonon_dos_elements.csv"),
        format!("{PHDOS_DIR}/phonon_dos.png"),
    ]
}
