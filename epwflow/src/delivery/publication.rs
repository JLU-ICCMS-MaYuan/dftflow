//! 派生文件发布日志：中断后只接续已登记的新旧文件，不覆盖未知内容。
use super::comparison::Artifact;
use crate::foundation::store;
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Component, Path},
};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    schema_version: u32,
    stage: String,
    manifest: String,
    previous_digest: Option<String>,
    payload: Vec<u8>,
    files: Vec<Artifact>,
    previous_files: Vec<Artifact>,
}
fn single(name: &str) -> bool {
    let mut c = Path::new(name).components();
    matches!(c.next(), Some(Component::Normal(_))) && c.next().is_none()
}
fn journal_name(manifest: &str) -> String {
    format!(".{manifest}.publish.json")
}
pub fn ensure_finished(root: &Path, manifest: &str) -> Result<()> {
    ensure!(
        !root.join(journal_name(manifest)).exists(),
        "data_invalid: 存在半写发布；请显式 collect/plot 恢复"
    );
    Ok(())
}
/// 同源部分重绘继续登记未请求的有效格式，发布器仍校验全部新旧哈希。
pub fn retain_unmodified(
    root: &Path,
    stage: &Path,
    previous: &[Artifact],
    files: &mut Vec<Artifact>,
) -> Result<()> {
    for old in previous {
        if !files.iter().any(|f| f.relative_path == old.relative_path) {
            old.verify(root)?;
            fs::copy(
                root.join(&old.relative_path),
                stage.join(&old.relative_path),
            )?;
            files.push(old.clone());
        }
    }
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(())
}
fn prepare<T: Serialize>(
    root: &Path,
    stage: &Path,
    manifest: &str,
    old: Vec<Artifact>,
    files: Vec<Artifact>,
    record: &T,
) -> Result<()> {
    ensure!(
        single(manifest) && stage.parent() == Some(root),
        "data_invalid: 发布路径越界"
    );
    let path = root.join(journal_name(manifest));
    ensure!(!path.exists(), "input_conflict: 已有发布日志");
    let mut payload = serde_json::to_vec_pretty(record)?;
    payload.push(b'\n');
    let journal = Journal {
        schema_version: 1,
        stage: stage.file_name().unwrap().to_str().unwrap().into(),
        manifest: manifest.into(),
        previous_digest: if root.join(manifest).exists() {
            Some(store::hash_file(&root.join(manifest))?)
        } else {
            None
        },
        payload,
        files,
        previous_files: old,
    };
    store::write(&path, &journal)
}
pub fn publish<T: Serialize>(
    root: &Path,
    stage: &Path,
    manifest: &str,
    old: Vec<Artifact>,
    files: Vec<Artifact>,
    record: &T,
) -> Result<()> {
    prepare(root, stage, manifest, old, files, record)?;
    recover(root, manifest)
}
pub fn recover(root: &Path, manifest: &str) -> Result<()> {
    ensure!(single(manifest), "data_invalid: 发布清单名非法");
    let path = root.join(journal_name(manifest));
    if !path.exists() {
        return Ok(());
    }
    let j: Journal = store::read(&path)?;
    ensure!(
        j.schema_version == 1
            && j.manifest == manifest
            && single(&j.stage)
            && j.stage.starts_with(".native-"),
        "data_invalid: 发布日志版本/路径非法"
    );
    let stage = root.join(&j.stage);
    let next_digest = store::hash_bytes(&j.payload);
    if root.join(manifest).exists() {
        let actual = store::hash_file(&root.join(manifest))?;
        ensure!(
            actual == next_digest || j.previous_digest.as_ref() == Some(&actual),
            "input_conflict: 发布清单被外部修改"
        );
    } else {
        ensure!(
            j.previous_digest.is_none(),
            "input_conflict: 旧发布清单丢失"
        );
    }
    ensure!(!j.files.is_empty(), "data_invalid: 发布文件集合为空");
    let mut seen = std::collections::BTreeSet::new();
    for f in &j.files {
        ensure!(
            f.relative_path.components().count() == 1
                && f.relative_path != Path::new(manifest)
                && seen.insert(f.relative_path.clone()),
            "data_invalid: 发布文件越界或重复"
        );
        if root.join(&f.relative_path).exists() {
            ensure!(
                f.verify(root).is_ok()
                    || j.previous_files
                        .iter()
                        .any(|a| a.relative_path == f.relative_path && a.verify(root).is_ok()),
                "input_conflict: 发布目标包含未知内容"
            );
        }
        if stage.join(&f.relative_path).exists() {
            f.verify(&stage)?;
        } else {
            f.verify(root)?;
        }
    }
    if stage.exists() {
        ensure!(
            stage.canonicalize()?.parent() == Some(root.canonicalize()?.as_path()),
            "data_invalid: staging 链接越界"
        );
        for e in fs::read_dir(&stage)? {
            let e = e?;
            let name = e.file_name();
            ensure!(
                seen.contains(Path::new(&name)) || name == manifest,
                "input_conflict: staging 包含未知文件"
            );
            if name == manifest {
                ensure!(
                    store::hash_file(&e.path())? == next_digest,
                    "input_conflict: staging 清单冲突"
                );
            }
        }
    }
    // 所有文件已验证后才开始接续发布；主清单最后原子替换。
    for f in &j.files {
        let from = stage.join(&f.relative_path);
        if from.exists() {
            fs::rename(from, root.join(&f.relative_path))?;
        }
    }
    store::atomic_bytes(&root.join(manifest), &j.payload)?;
    if stage.join(manifest).exists() {
        fs::remove_file(stage.join(manifest))?;
    }
    if stage.exists() {
        fs::remove_dir(&stage)?;
    }
    fs::remove_file(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interrupted_publish_recovers_but_unknown_target_is_never_overwritten() {
        let root = std::env::temp_dir().join(format!("native-publish-{}", store::token()));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("a.csv"), "old a").unwrap();
        fs::write(root.join("b.csv"), "old b").unwrap();
        let old = vec![
            Artifact::capture(&root, "a.csv").unwrap(),
            Artifact::capture(&root, "b.csv").unwrap(),
        ];
        store::write(
            &root.join("manifest.json"),
            &serde_json::json!({"revision":1}),
        )
        .unwrap();
        let stage = root.join(".native-fixture");
        fs::create_dir(&stage).unwrap();
        fs::write(stage.join("a.csv"), "new a").unwrap();
        fs::write(stage.join("b.csv"), "new b").unwrap();
        let files = vec![
            Artifact::capture(&stage, "a.csv").unwrap(),
            Artifact::capture(&stage, "b.csv").unwrap(),
        ];
        prepare(
            &root,
            &stage,
            "manifest.json",
            old,
            files,
            &serde_json::json!({"revision":2}),
        )
        .unwrap();
        fs::rename(stage.join("a.csv"), root.join("a.csv")).unwrap();
        assert!(ensure_finished(&root, "manifest.json").is_err());
        fs::write(root.join("b.csv"), "user data").unwrap();
        assert!(recover(&root, "manifest.json").is_err());
        assert_eq!(fs::read_to_string(root.join("b.csv")).unwrap(), "user data");
        assert_eq!(
            store::read::<serde_json::Value>(&root.join("manifest.json")).unwrap()["revision"],
            1
        );
        fs::write(root.join("b.csv"), "old b").unwrap();
        recover(&root, "manifest.json").unwrap();
        assert_eq!(fs::read_to_string(root.join("b.csv")).unwrap(), "new b");
        assert_eq!(
            store::read::<serde_json::Value>(&root.join("manifest.json")).unwrap()["revision"],
            2
        );
        recover(&root, "manifest.json").unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}
