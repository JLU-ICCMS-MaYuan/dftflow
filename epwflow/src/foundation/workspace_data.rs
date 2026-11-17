//! 正式数据的相对引用及最小基态输入；不把原始密度和波函数加入核验。
use anyhow::{ensure, Context, Result};
use std::{fs, path::{Component, Path, PathBuf}};

pub fn relative(from: &Path, target: &Path) -> Result<PathBuf> {
    ensure!(from.is_absolute() && target.is_absolute(), "configuration: shared data paths must be absolute");
    let a: Vec<_> = from.components().collect();
    let b: Vec<_> = target.components().collect();
    let common = a.iter().zip(&b).take_while(|(x,y)|x==y).count();
    ensure!(common > 0, "configuration: shared paths have no common root");
    let mut result = PathBuf::new();
    for _ in common..a.len() { result.push(".."); }
    for c in &b[common..] { result.push(c.as_os_str()); }
    Ok(result)
}
pub fn link(source: &Path, target: &Path) -> Result<()> {
    let source = fs::canonicalize(source).with_context(||format!("data_invalid: missing shared source {}",source.display()))?;
    let parent = target.parent().context("configuration: link needs parent")?;
    fs::create_dir_all(parent)?;
    if fs::symlink_metadata(target).is_ok() {
        ensure!(fs::symlink_metadata(target)?.file_type().is_symlink()
            && fs::canonicalize(target)? == source,
            "input_conflict: shared target differs; retain existing file {}",target.display());
        return Ok(());
    }
    let parent = fs::canonicalize(parent)?;
    let relative = relative(&parent,&source)?;
    #[cfg(unix)] std::os::unix::fs::symlink(relative,target)?;
    #[cfg(not(unix))] anyhow::bail!("unsupported: native shared data requires symbolic links");
    Ok(())
}
/// 复制原生非自洽计算真正需要的基态输入，不复制旧波函数或整个 out。
pub fn stage_ground_state(source: &Path, target: &Path, prefix: &str) -> Result<()> {
    let src=source.join(format!("{prefix}.save"));
    let dst=target.join(format!("{prefix}.save"));
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(&src)? {
        let entry=entry?;let name=entry.file_name();let name=name.to_string_lossy();
        let density=name.starts_with("charge-density.");
        let metadata=name=="data-file-schema.xml" || name.ends_with(".UPF") || name.ends_with(".upf")
            || name=="paw.txt" || name.starts_with("occup") || name.starts_with("kinetic-density.") || name.starts_with("tau.");
        if !density && !metadata { continue; }
        // 原始密度不做大小、存在性或身份比较；复制失败按实际 I/O 报错。
        fs::copy(entry.path(),dst.join(entry.file_name()))?;
    }
    Ok(())
}
/// 仅移除已明确属于该调用的工作目录；输入／日志和正式文件由调用方分开保留。
pub fn remove_work_tree(work: &Path, target: &Path) -> Result<()> {
    ensure!(target.starts_with(work) && target.components().all(|c|!matches!(c,Component::ParentDir)),
        "input_conflict: cleanup outside workspace");
    if fs::symlink_metadata(target).is_ok() {
        ensure!(!fs::symlink_metadata(target)?.file_type().is_symlink(),"input_conflict: cleanup target is a link");
        fs::remove_dir_all(target)?;
    }
    Ok(())
}

/// EPW 的必要电子态工作树：保存目录及其树外原生波函数，不携带其它计算目录。
pub fn stage_electronic_work(source: &Path, target: &Path, prefix: &str) -> Result<()> {
    fs::create_dir_all(target)?;
    for relative in crate::qe_data::save::all_files(source)? {
        let inside=relative.starts_with(format!("{prefix}.save"));
        if !inside && !qf_io::qe_payload::unchecked(&relative) { continue; }
        let dst=target.join(&relative);fs::create_dir_all(dst.parent().unwrap())?;
        fs::copy(source.join(relative),dst)?;
    }
    Ok(())
}
pub fn release_electronic_work(cfg: &super::config::Config, directory: &Path) -> Result<()> {
    let out=directory.join("out");
    let save=out.join(format!("{}.save",cfg.qe.prefix));
    super::native_jobs::forget_work_inputs(cfg,&save)?;
    remove_work_tree(&cfg.work_dir,&save)?;
    if out.is_dir() {
        for entry in fs::read_dir(&out)? {
            let p=entry?.path();
            if qf_io::qe_payload::unchecked(&p) { fs::remove_file(p)?; }
        }
        if fs::read_dir(&out)?.next().is_none() { fs::remove_dir(out)?; }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_models_are_relative_links_and_different_existing_files_are_retained() {
        let root=std::env::temp_dir().join(format!("epw-shared-{}",crate::foundation::store::token()));
        let source=root.join("5.epw-wannier/test.mmn");fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source,"model").unwrap();let target=root.join("6.epw-coarse/test.mmn");
        link(&source,&target).unwrap();link(&source,&target).unwrap();
        assert_eq!(fs::read_link(&target).unwrap(),Path::new("../5.epw-wannier/test.mmn"));
        assert_eq!(fs::read_to_string(&target).unwrap(),"model");
        fs::remove_file(&target).unwrap();fs::write(&target,"different").unwrap();
        assert!(link(&source,&target).is_err());assert_eq!(fs::read_to_string(target).unwrap(),"different");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn nscf_preparation_copies_ground_state_but_never_old_wavefunctions() {
        let root=std::env::temp_dir().join(format!("epw-ground-{}",crate::foundation::store::token()));
        let source=root.join("source");fs::create_dir_all(source.join("x.save")).unwrap();
        for (name,text) in [("data-file-schema.xml","xml"),("charge-density.dat","density"),("X.UPF","pseudo"),("wfc1.dat","not a required ground-state input")] {
            fs::write(source.join("x.save").join(name),text).unwrap();
        }
        let target=root.join("nscf");stage_ground_state(&source,&target,"x").unwrap();
        assert_eq!(fs::read_to_string(target.join("x.save/data-file-schema.xml")).unwrap(),"xml");
        assert_eq!(fs::read_to_string(target.join("x.save/X.UPF")).unwrap(),"pseudo");
        // 不把密度存在性或字节比较作为复用核验；这里只验证必要复制调用返回成功。
        assert!(!target.join("x.save/wfc1.dat").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
