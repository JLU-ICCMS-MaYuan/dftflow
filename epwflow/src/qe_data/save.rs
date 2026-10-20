//! QE 保存树的完整复制与身份核验；SCF 来源不作为下游可写目录。
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

pub fn files(root: &Path) -> Result<Vec<TreeFile>> {
    fn visit(root: &Path, dir: &Path, result: &mut Vec<TreeFile>) -> Result<()> {
        for e in fs::read_dir(dir)? {
            let e = e?;
            let path = e.path();
            let kind = e.file_type()?;
            ensure!(
                !kind.is_symlink(),
                "data_invalid: 保存树不能含链接 {}",
                path.display()
            );
            if kind.is_dir() {
                visit(root, &path, result)?;
            } else {
                ensure!(kind.is_file(), "data_invalid: 保存树含未知文件类型");
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
        "data_invalid: 保存树不存在 {}",
        root.display()
    );
    let mut result = vec![];
    visit(root, root, &mut result)?;
    result.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    ensure!(!result.is_empty(), "data_invalid: 保存树为空");
    Ok(result)
}

/// 只复制到空目录；复制前后核对完整集合，避免合并不同尝试或共享可写链接。
pub fn copy_tree(source: &Path, target: &Path) -> Result<Vec<TreeFile>> {
    let before = files(source)?;
    ensure!(
        !target.exists() || (target.is_dir() && fs::read_dir(target)?.next().is_none()),
        "input_conflict: 保存树目标非空 {}",
        target.display()
    );
    fs::create_dir_all(target)?;
    for file in &before {
        let dest = target.join(&file.relative_path);
        fs::create_dir_all(dest.parent().unwrap())?;
        fs::copy(source.join(&file.relative_path), &dest)?;
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
        "input_conflict: 保存树在复制期间变化或复制不完整"
    );
    Ok(before)
}

pub fn snapshot(outdir: &Path, work: &Path) -> Result<PathBuf> {
    let target = crate::foundation::layout::path(&work, "scf/snapshot");
    let entries = copy_tree(outdir, &target)?;
    store::write(&crate::foundation::layout::path(&work, "scf/snapshot-files.json"), &entries)?;
    for e in &entries {
        let p = target.join(&e.relative_path);
        let mut permissions = fs::metadata(&p)?.permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&p, permissions)?;
    }
    Ok(target)
}

pub fn verify_snapshot(work: &Path) -> Result<()> {
    let expected: Vec<TreeFile> = store::read(&crate::foundation::layout::path(&work, "scf/snapshot-files.json"))
        .context("data_invalid: 缺少完整 SCF 快照身份")?;
    ensure!(
        expected == files(&crate::foundation::layout::path(&work, "scf/snapshot"))?,
        "input_conflict: SCF 快照被修改"
    );
    Ok(())
}

pub fn stage_scf(work: &Path, outdir: &Path, prefix: &str) -> Result<()> {
    verify_snapshot(work)?;
    copy_tree(&crate::foundation::layout::path(&work, "scf/snapshot"), outdir)?;
    nonempty(&outdir.join(format!("{prefix}.save/data-file-schema.xml")))?;
    verify_snapshot(work)
}

pub fn nonempty(path: &Path) -> Result<()> {
    ensure!(
        path.is_file() && fs::metadata(path)?.len() > 0,
        "data_invalid: 必需保存文件缺失或空 {}",
        path.display()
    );
    Ok(())
}
