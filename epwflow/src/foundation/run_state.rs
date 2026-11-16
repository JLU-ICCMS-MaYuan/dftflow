//! 旧隐藏状态只允许显式转换；当前工作流不再读取旧树或创建迁移历史。
use super::{config::Config, readable_state as log};
use anyhow::{ensure, Context, Result};
use std::{fs, path::Path};
pub fn ready(root: &Path) -> Result<()> {
    ensure!(!root.join(".epwflow").exists(),"input_conflict: An old hidden state exists .epwflow/;Please use the current configuration to execute --migrate-state ,The program will first check if the old job is out");
    Ok(())
}
pub fn migrate(cfg: &Config, input: &Path) -> Result<()> {
    let _operation = log::DirectoryGuard::acquire(&cfg.work_dir)?;
    ensure!(
        cfg.layout.as_deref() == Some("numbered"),
        "configuration: --migrate-state _Other Organiser"
    );
    let old = cfg.work_dir.join(".epwflow");
    if !old.exists() {
        recheck_converted_electronic(cfg)?;
        println!("State already uses eight stage logs; no migration needed");
        return Ok(());
    }
    let conversion = super::legacy_state::prepare_at(cfg, &old)?
        .context("unsupported_version: old hidden layout not recognized; please use --import-work Import a new numbered workspace")?;
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
                        "input_conflict: Import Source Record Conflict");
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
    println!("Legacy state migrated to eight stage logs; scientific calculations were not rerun");
    Ok(())
}

/// 显式重试迁移时核验曾被拒绝的电子结果；不准备输入、不调用科学程序。
fn recheck_converted_electronic(cfg: &Config) -> Result<()> {
    use super::stages::{self, Step};
    let doc = log::read(&cfg.work_dir, Step::Electronic)?;
    if doc.get("Stage", "Status") != Some("Legacy results require validation") {
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
            "input_conflict: The scientific parameters of the converted electronic phase have been changed and the old results cannot be re-registered"
        );
        for &dep in Step::Electronic.dependencies() {
            ensure!(
                stages::reusable(cfg, dep)?,
                "input_conflict: The e-phase upstream has not yet been re-applicated"
            );
            let upstream = log::read(&cfg.work_dir, dep)?;
            ensure!(
                doc.get("Dependencies", &format!("{} batch", dep.name())) == upstream.get("Stage", "Batch"),
                "input_conflict: Reliance batch not consistent with converted electronic phase"
            );
        }
        stages::finish(cfg, Step::Electronic)
    })();
    if let Err(error) = result {
        log::update(&cfg.work_dir, Step::Electronic, |d| {
            d.set("Problem", "Reason", format!("{error:#}"));
            Ok(())
        })?;
        return Err(error);
    }
    log::update(&cfg.work_dir, Step::Electronic, |d| {
        d.set(
            "Next step",
            "Action",
            "Legacy electronic results revalidated and registered complete; advance to the Wannier stage",
        );
        Ok(())
    })?;
    println!("Stage 3 legacy electronic results revalidated; scientific files unchanged; no calculations submitted");
    Ok(())
}
