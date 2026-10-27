//! 旧隐藏状态只允许显式转换；当前工作流不再读取旧树或创建迁移历史。
use super::{config::Config, readable_state as log};
use anyhow::{ensure, Context, Result};
use std::{fs, path::Path};
pub fn ready(root: &Path) -> Result<()> {
    ensure!(!root.join(".epwflow").exists(),"input_conflict: 存在旧隐藏状态 .epwflow/；请使用当前配置执行 --migrate-state，程序会先检查旧作业是否已退出");
    Ok(())
}
pub fn migrate(cfg: &Config, input: &Path) -> Result<()> {
    let _operation = log::DirectoryGuard::acquire(&cfg.work_dir)?;
    ensure!(
        cfg.layout.as_deref() == Some("numbered"),
        "configuration: --migrate-state 需要编号工作区"
    );
    let old = cfg.work_dir.join(".epwflow");
    if !old.exists() {
        println!("状态已采用八阶段日志，无需转换");
        return Ok(());
    }
    let conversion = super::legacy_state::prepare_at(cfg, &old)?
        .context("unsupported_version: 未识别旧隐藏布局；请用 --import-work 导入新的编号工作区")?;
    super::legacy_state::retire_locks(&old)?;
    let version = log::backup_config(input)?;
    log::register_config(cfg, input, &version)?;
    let imports = old.join("imports");
    if imports.is_dir() {
        let target = cfg.work_dir.join("epw-state/imports");
        fs::create_dir_all(&target)?;
        for file in crate::qe_data::save::files(&imports)? {
            let to = target.join(&file.relative_path);
            if to.exists() {
                ensure!(
                    super::store::hash_file(&to)? == file.sha256,
                    "input_conflict: 导入来源记录冲突"
                );
            } else {
                fs::create_dir_all(to.parent().unwrap())?;
                fs::copy(imports.join(&file.relative_path), &to)?;
            }
        }
    }
    super::legacy_state::finish(cfg, &version, conversion)?;
    if imports.exists() {
        fs::remove_dir_all(imports)?;
    }
    fs::remove_dir(&old)?;
    println!("旧隐藏状态已转换为八份阶段日志；科学数据未重新计算");
    Ok(())
}
