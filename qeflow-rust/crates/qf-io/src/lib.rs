//! QE 输出集中读取（对齐 `qeflow/io_reader`，M2）。

/// `scf.out` 解析结果（M2）。
pub struct ScfOut;
/// `ph.out` 解析结果（M2）。
pub struct PhOut;
/// `*.dyn0` 解析结果（M2）。
pub struct Dyn0;
/// 声子 DOS 解析结果（M2）。
pub struct PhDos;

/// 读取并解析 `scf.out`（M2）。
pub fn read_scf_out(_path: &str) -> anyhow::Result<ScfOut> {
    anyhow::bail!("read_scf_out not yet implemented (M2)")
}

/// 读取并解析 `ph.out`（M2）。
pub fn read_ph_out(_path: &str) -> anyhow::Result<PhOut> {
    anyhow::bail!("read_ph_out not yet implemented (M2)")
}

/// 读取并解析 `*.dyn0`（M2）。
pub fn read_dyn0(_path: &str) -> anyhow::Result<Dyn0> {
    anyhow::bail!("read_dyn0 not yet implemented (M2)")
}

/// 读取并解析声子 DOS（M2）。
pub fn read_phdos(_path: &str) -> anyhow::Result<PhDos> {
    anyhow::bail!("read_phdos not yet implemented (M2)")
}
