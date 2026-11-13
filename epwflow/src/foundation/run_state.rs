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
        recheck_converted_electronic(cfg)?;
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
        for relative in crate::qe_data::save::all_files(&imports)? {
            let to = target.join(&relative);
            if to.exists() {
                if !qf_io::qe_payload::unchecked(&relative) {
                    ensure!(super::store::hash_file(&to)? == super::store::hash_file(&imports.join(&relative))?,
                        "input_conflict: 导入来源记录冲突");
                }
            } else {
                fs::create_dir_all(to.parent().unwrap())?;
                fs::copy(imports.join(&relative), &to)?;
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

/// 显式重试迁移时核验曾被拒绝的电子结果；不准备输入、不调用科学程序。
fn recheck_converted_electronic(cfg: &Config) -> Result<()> {
    use super::stages::{self, Step};
    let doc = log::read(&cfg.work_dir, Step::Electronic)?;
    if doc.get("阶段", "状态") != Some("旧结果需要检查") {
        return Ok(());
    }
    super::workspace::check_active(cfg, None)?;
    super::native_jobs::check_groups(cfg)?;
    super::native_jobs::reconcile(cfg, false)?;
    let result = (|| {
        let previous =
            log::load_config(&log::computation_config(&cfg.work_dir, Step::Electronic)?)?;
        ensure!(
            stages::parameters(cfg, Step::Electronic)?
                == stages::parameters(&previous, Step::Electronic)?,
            "input_conflict: 已转换电子阶段的科学参数已变化，不能重新登记旧结果"
        );
        for &dep in Step::Electronic.dependencies() {
            ensure!(
                stages::reusable(cfg, dep)?,
                "input_conflict: 电子阶段上游尚未通过复用核验"
            );
            let upstream = log::read(&cfg.work_dir, dep)?;
            ensure!(
                doc.get("依赖", &format!("{} 批次", dep.name())) == upstream.get("阶段", "批次"),
                "input_conflict: 已转换电子阶段的依赖批次不符"
            );
        }
        stages::finish(cfg, Step::Electronic)
    })();
    if let Err(error) = result {
        log::update(&cfg.work_dir, Step::Electronic, |d| {
            d.set("问题", "原因", format!("{error:#}"));
            Ok(())
        })?;
        return Err(error);
    }
    log::update(&cfg.work_dir, Step::Electronic, |d| {
        d.set(
            "下一步",
            "操作",
            "旧电子结果已重新核验并登记完成；可以推进 Wannier 阶段",
        );
        Ok(())
    })?;
    println!("第 3 步旧电子结果已重新核验；科学文件未改写，未提交计算");
    Ok(())
}
