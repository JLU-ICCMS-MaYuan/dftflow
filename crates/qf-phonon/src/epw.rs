//! nosplit 原生 PH 的只读归档；科学模式／扰动基底由 EPW 消费端进一步校验。
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct File {
    pub role: String,
    pub source: PathBuf,
    pub relative_path: PathBuf,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub prefix: String,
    pub qpoints: usize,
    pub files: Vec<File>,
}

fn digest(path: &Path) -> Result<(u64, String)> {
    let meta = fs::symlink_metadata(path)?;
    ensure!(
        meta.is_file() && meta.len() > 0,
        "EPW 数据缺失、为空或不是普通文件: {}",
        path.display()
    );
    let mut input = fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = input.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok((meta.len(), format!("{:x}", hash.finalize())))
}

fn add(files: &mut Vec<File>, role: String, source: PathBuf, relative_path: PathBuf) -> Result<()> {
    let (bytes, sha256) = digest(&source)?;
    files.push(File {
        role,
        source: source.canonicalize()?,
        relative_path,
        bytes,
        sha256,
    });
    Ok(())
}

fn phsave(files: &mut Vec<File>, root: &Path, relative: &Path, prefix: &str) -> Result<()> {
    let mut entries = fs::read_dir(root.join(relative))?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let rel = relative.join(entry.file_name());
        let kind = entry.file_type()?;
        ensure!(
            !kind.is_symlink(),
            "EPW phsave 不允许符号链接: {}",
            entry.path().display()
        );
        if kind.is_dir() {
            phsave(files, root, &rel, prefix)?;
        } else {
            add(
                files,
                format!("phsave:{}", rel.display()),
                entry.path(),
                PathBuf::from(format!("{prefix}.phsave")).join(rel),
            )?;
        }
    }
    Ok(())
}

struct Lock(PathBuf);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_dir(&self.0);
    }
}

/// 输入必须来自 ldisp=true、lqdir=true、fildvscf=dvscf 的完整 XML nosplit 计算。
/// 不修改原 scratch，不从文本 dyn 或缺失的 q 响应推测数据；相同来源可幂等复核。
pub fn export_nosplit(work: &Path, outdir: &Path, prefix: &str) -> Result<PathBuf> {
    ensure!(
        !prefix.is_empty()
            && prefix
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "_-".contains(c)),
        "EPW prefix 无效"
    );
    let lock = work.join(".epw-save.lock");
    fs::create_dir(&lock).context("EPW save 正在归档，或存在需核对的残留锁")?;
    let _lock = Lock(lock);
    qf_io::qe_completion::checked_log(&work.join("ph.out"), false)?;
    let dyn0 = work.join(format!("{prefix}.dyn0"));
    let raw = fs::read_to_string(&dyn0)?;
    let count: usize = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .nth(1)
        .context("dyn0 缺 q 数")?
        .trim()
        .parse()?;
    let points = qf_io::parse_dyn0_qpoints(&raw)?;
    ensure!(
        count > 0 && points.len() == count,
        "EPW dyn0 q 点列表不完整"
    );
    ensure!(
        points[0].q_crystal.iter().all(|v| v.abs() < 1e-9),
        "EPW 第一个 q 必须为 Gamma"
    );
    let root = outdir.join("_ph0");
    let metadata = root.join(format!("{prefix}.phsave"));
    let mut files = vec![];
    add(
        &mut files,
        "dyn0".into(),
        dyn0,
        format!("{prefix}.dyn0.xml").into(),
    )?;
    digest(&metadata.join("control_ph.xml"))?;
    for (i, point) in points.iter().enumerate() {
        ensure!(
            point.q_crystal.iter().all(|v| v.is_finite()),
            "EPW q 坐标非有限"
        );
        for prior in &points[..i] {
            ensure!(
                prior
                    .q_crystal
                    .iter()
                    .zip(point.q_crystal)
                    .any(|(a, b)| (a - b).abs() > 1e-9),
                "EPW dyn0 存在重复 q"
            );
        }
        let q = i + 1;
        let dir = if point.q_crystal.iter().all(|v| v.abs() < 1e-9) {
            root.clone()
        } else {
            root.join(format!("{prefix}.q_{q}"))
        };
        let candidates = [
            dir.join(format!("{prefix}.dvscf")),
            dir.join(format!("{prefix}.dvscf1")),
        ];
        let candidates = candidates.iter().filter(|p| p.exists()).collect::<Vec<_>>();
        ensure!(candidates.len() == 1, "EPW q={q} 缺少逐 q dvscf 或存在歧义");
        digest(&metadata.join(format!("patterns.{q}.xml")))?;
        add(
            &mut files,
            format!("dyn_q{q}"),
            work.join(format!("{prefix}.dyn{q}.xml")),
            format!("{prefix}.dyn_q{q}.xml").into(),
        )?;
        add(
            &mut files,
            format!("dvscf_q{q}"),
            candidates[0].clone(),
            format!("{prefix}.dvscf_q{q}").into(),
        )?;
    }
    phsave(&mut files, &metadata, Path::new(""), prefix)?;
    let manifest = Manifest {
        schema_version: 1,
        prefix: prefix.into(),
        qpoints: count,
        files,
    };
    let destination = work.join("save");
    if destination.exists() {
        let old: Manifest = serde_json::from_slice(
            &fs::read(destination.join("manifest.json"))
                .context("EPW save 已存在但缺归档清单，禁止覆盖")?,
        )?;
        ensure!(old == manifest, "EPW save 已绑定不同来源，禁止覆盖");
        for file in &manifest.files {
            ensure!(
                digest(&destination.join(&file.relative_path))?
                    == (file.bytes, file.sha256.clone()),
                "EPW save 归档数据已改变"
            );
        }
        return Ok(destination);
    }
    let partial = work.join(format!("save.partial-{}", std::process::id()));
    fs::create_dir(&partial).context("EPW save 临时交付已存在，保留现场")?;
    for file in &manifest.files {
        let target = partial.join(&file.relative_path);
        fs::create_dir_all(target.parent().context("EPW 目标缺父目录")?)?;
        fs::copy(&file.source, &target)?;
        ensure!(
            digest(&target)? == (file.bytes, file.sha256.clone())
                && digest(&file.source)? == (file.bytes, file.sha256.clone()),
            "EPW 来源在归档期间改变"
        );
    }
    for file in &manifest.files {
        ensure!(
            digest(&file.source)? == (file.bytes, file.sha256.clone()),
            "EPW 来源在发布前改变"
        );
    }
    fs::write(
        partial.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    fs::rename(partial, &destination)?;
    Ok(destination)
}
