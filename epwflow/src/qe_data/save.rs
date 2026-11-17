//! QE 保存树复制与受保护元数据核验；电荷密度和波函数不参与核对。
use super::super::foundation::store;
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeFile {
    pub relative_path: PathBuf,
    pub byte_size: u64,
    pub sha256: String,
}

/// 仅列出需要核验的文件，不读取被排除的原始数据。
pub fn files(root: &Path) -> Result<Vec<TreeFile>> {
    fn visit(root: &Path, dir: &Path, result: &mut Vec<TreeFile>) -> Result<()> {
        for e in fs::read_dir(dir)? {
            let e = e?;
            let path = e.path();
            if qf_io::qe_payload::unchecked(&path) || path.file_name().is_some_and(|n|n=="atomic_proj.xml" || n.to_string_lossy().starts_with("AUX")) { continue; }
            let kind = e.file_type()?;
            if kind.is_symlink() {
                if path.is_dir() { continue; }
                ensure!(path.is_file(),"data_invalid: broken shared file {}",path.display());
            }
            if kind.is_dir() {
                visit(root, &path, result)?;
            } else {
                ensure!(kind.is_file() || kind.is_symlink(), "data_invalid: Save an unknown file type for tree contents");
                result.push(TreeFile {
                    relative_path: path.strip_prefix(root)?.into(),
                    byte_size: fs::metadata(&path)?.len(),
                    sha256: store::hash_file(&path)?,
                });
            }
        }
        Ok(())
    }
    ensure!(
        root.is_dir(),
        "data_invalid: Save Tree does not exist {}",
        root.display()
    );
    let mut result = vec![];
    visit(root, root, &mut result)?;
    result.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(result)
}

/// 真实复制所需的文件遍历；不计算数据哈希，不将此列表作为 payload 核验依据。
pub fn all_files(root: &Path) -> Result<Vec<PathBuf>> {
    fn visit(root: &Path, dir: &Path, result: &mut Vec<PathBuf>) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if qf_io::qe_payload::unchecked(&path) {
                result.push(path.strip_prefix(root)?.into());
                continue;
            }
            let kind = entry.file_type()?;
            ensure!(!kind.is_symlink(), "data_invalid: Save tree does not contain links {}", path.display());
            if kind.is_dir() { visit(root, &path, result)?; }
            else {
                ensure!(kind.is_file(), "data_invalid: Save the unknown type of tree contents {}", path.display());
                result.push(path.strip_prefix(root)?.into());
            }
        }
        Ok(())
    }
    let mut result = Vec::new();
    visit(root, root, &mut result)?;
    result.sort();
    Ok(result)
}

/// 兼容旧记录：仅在内存中过滤，不改写历史清单。
pub fn checked_entries(entries: &[TreeFile]) -> Vec<TreeFile> {
    entries.iter().filter(|f| !qf_io::qe_payload::unchecked(&f.relative_path)).cloned().collect()
}

/// 复制全部文件到空目录；只对受保护文件核对复制结果。
pub fn copy_tree(source: &Path, target: &Path) -> Result<Vec<TreeFile>> {
    let before = files(source)?;
    ensure!(
        !target.exists() || (target.is_dir() && fs::read_dir(target)?.next().is_none()),
        "input_conflict: Save tree target is not empty {}",
        target.display()
    );
    fs::create_dir_all(target)?;
    for relative in all_files(source)? {
        let dest = target.join(&relative);
        fs::create_dir_all(dest.parent().unwrap())?;
        fs::copy(source.join(&relative), &dest)?;
        // 快照可能不可写；下游副本需要独立写权限，绝不修改来源权限。
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&dest)?.permissions();
            permissions.set_mode(permissions.mode() | 0o200);
            fs::set_permissions(&dest, permissions)?;
        }
    }
    ensure!(
        before == files(source)? && before == files(target)?,
        "input_conflict: Save tree change or copy incomplete during copying"
    );
    Ok(before)
}

pub fn snapshot(outdir: &Path, work: &Path) -> Result<PathBuf> {
    let target = crate::foundation::layout::path(&work, "scf/snapshot");
    if crate::foundation::layout::enabled(work) { return Ok(outdir.to_owned()); }

    let entries = copy_tree(outdir, &target)?;
    store::write(&crate::foundation::layout::path(&work, "scf/snapshot-files.json"), &entries)?;
    for relative in all_files(&target)? {
        let p = target.join(relative);
        let mut permissions = fs::metadata(&p)?.permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&p, permissions)?;
    }
    Ok(target)
}

pub fn verify_snapshot(work: &Path) -> Result<()> {
    if crate::foundation::layout::enabled(work) { return Ok(()); }

    let expected: Vec<TreeFile> = store::read(&crate::foundation::layout::path(&work, "scf/snapshot-files.json"))
        .context("data_invalid: Missing completeness SCF Take an ID")?;
    ensure!(
        checked_entries(&expected) == files(&crate::foundation::layout::path(&work, "scf/snapshot"))?,
        "input_conflict: SCF Snapshot modified"
    );
    Ok(())
}

/// PH 需要完整的收敛 SCF 电子态；此副本只用于可写工作区，归集后清理。
pub fn stage_scf(work:&Path,outdir:&Path,prefix:&str)->Result<()> {
    verify_snapshot(work)?;
    copy_tree(&crate::foundation::layout::path(work,"scf/snapshot"),outdir)?;
    nonempty(&outdir.join(format!("{prefix}.save/data-file-schema.xml")))?;
    verify_snapshot(work)
}

/// NSCF／路径能带重新生成电子态，不复制旧波函数。
pub fn stage_ground_state(work:&Path,outdir:&Path,prefix:&str)->Result<()> {
    verify_snapshot(work)?;
    let source=crate::foundation::layout::path(work,"scf/snapshot");
    if crate::foundation::layout::enabled(work) {crate::foundation::workspace_data::stage_ground_state(&source,outdir,prefix)?;}
    else {copy_tree(&source,outdir)?;}
    nonempty(&outdir.join(format!("{prefix}.save/data-file-schema.xml")))?;
    verify_snapshot(work)
}

pub fn nonempty(path: &Path) -> Result<()> {
    if qf_io::qe_payload::unchecked(path) { return Ok(()); }
    ensure!(
        path.is_file() && fs::metadata(path)?.len() > 0,
        "data_invalid: Missing or empty files must be saved {}",
        path.display()
    );
    Ok(())
}
