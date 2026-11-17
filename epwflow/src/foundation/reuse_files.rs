//! 按实际交接输出来源；不持久登记文件身份，不递归检查来源阶段。
use anyhow::{Context, Result};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

thread_local! {static SEEN:std::cell::RefCell<Option<BTreeSet<PathBuf>>>=const {std::cell::RefCell::new(None)};}
pub struct Scope(bool);
impl Scope {
    pub fn enter() -> Self {
        Self(SEEN.with(|seen| {
            let mut seen = seen.borrow_mut();
            if seen.is_some() {
                false
            } else {
                *seen = Some(BTreeSet::new());
                true
            }
        }))
    }
}
impl Drop for Scope {
    fn drop(&mut self) {
        if self.0 {
            SEEN.with(|seen| *seen.borrow_mut() = None);
        }
    }
}

pub fn report(root: &Path, paths: impl IntoIterator<Item = PathBuf>) {
    let mut groups: BTreeMap<String, BTreeSet<PathBuf>> = BTreeMap::new();
    for path in paths {
        let new = SEEN.with(|seen| {
            seen.borrow_mut()
                .as_mut()
                .is_none_or(|set| set.insert(path.clone()))
        });
        if !new {
            continue;
        }
        if let Ok(relative) = path.strip_prefix(root) {
            if let Some(stage) = relative.components().next() {
                let stage = stage.as_os_str().to_string_lossy().into_owned();
                if stage.as_bytes().first().is_some_and(u8::is_ascii_digit) {
                    groups.entry(stage).or_default().insert(relative.to_owned());
                }
            }
        }
    }
    for (stage, paths) in groups {
        println!("reuse: {stage}");
        for path in paths {
            println!("       {}", path.display());
        }
    }
}

/// 数据文件只链接，原始密度／波函数不做存在性、大小或哈希预检。
pub fn link(source: &Path, target: &Path) -> Result<()> {
    let parent = target
        .parent()
        .context("configuration: link parent missing")?;
    fs::create_dir_all(parent)?;
    let source = if source.is_absolute() {
        source.to_owned()
    } else {
        std::env::current_dir()?.join(source)
    };
    let parent = fs::canonicalize(parent)?;
    let a = parent.components().collect::<Vec<_>>();
    let b = source.components().collect::<Vec<_>>();
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    anyhow::ensure!(common > 0, "configuration: link paths have no common root");
    let mut relative = PathBuf::new();
    for _ in common..a.len() {
        relative.push("..");
    }
    for part in &b[common..] {
        relative.push(part.as_os_str());
    }
    if let Ok(metadata) = fs::symlink_metadata(target) {
        anyhow::ensure!(
            metadata.file_type().is_symlink() && fs::read_link(target)? == relative,
            "input_conflict: retain existing target {}",
            target.display()
        );
        return Ok(());
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(relative, target)?;
    #[cfg(not(unix))]
    anyhow::bail!("unsupported: symbolic links require Unix");
    Ok(())
}

/// 独立元数据副本防止原生程序回写来源；大载荷只读链接，赝势来自用户目录。
pub fn electronic(
    cfg: &super::config::Config,
    source: &Path,
    target: &Path,
    waves: bool,
) -> Result<()> {
    fs::create_dir_all(target)?;
    let mut used = vec![];
    for relative in crate::qe_data::save::all_files(source).with_context(|| {
        format!(
            "data_invalid: missing source directory {}",
            source.display()
        )
    })? {
        let name = relative.file_name().unwrap().to_string_lossy();
        if name.to_ascii_lowercase().ends_with(".upf")
            || cfg
                .pseudos
                .values()
                .any(|p| p.file_name() == relative.file_name())
        {
            continue;
        }
        let raw = qf_io::qe_payload::unchecked(&relative);
        if raw && !waves && !name.starts_with("charge-density.") {
            continue;
        }
        if !relative.starts_with(format!("{}.save", cfg.qe.prefix)) && !raw {
            continue;
        }
        if !raw
            && name != "data-file-schema.xml"
            && name != "paw.txt"
            && !name.starts_with("occup")
            && !name.starts_with("kinetic-density.")
            && !name.starts_with("tau.")
        {
            continue;
        }
        let src = source.join(&relative);
        let dst = target.join(&relative);
        fs::create_dir_all(dst.parent().unwrap())?;
        if raw && waves {
            link(&src, &dst)?;
        } else {
            fs::copy(&src, &dst)
                .with_context(|| format!("execution_failed: copy {}", src.display()))?;
        }
        used.push(src);
    }
    let save = target.join(format!("{}.save", cfg.qe.prefix));
    fs::create_dir_all(&save)?;
    for pseudo in cfg.pseudos.values() {
        let dst = save.join(
            pseudo
                .file_name()
                .context("configuration: pseudo filename missing")?,
        );
        // 原生程序会写自己的副本；原子替换目录项，不能通过旧链接写到用户原件。
        anyhow::ensure!(
            pseudo != &dst,
            "configuration: pseudopotential source overlaps working output {}",
            dst.display()
        );
        let temporary = save.join(format!(".pseudo-copy-{}", super::store::token()));
        fs::copy(pseudo, &temporary).with_context(|| {
            format!(
                "execution_failed: copy configured pseudopotential {}",
                pseudo.display()
            )
        })?;
        fs::rename(&temporary, &dst)?;
    }
    report(&cfg.work_dir, used);
    Ok(())
}

/// 粗网格真正需要的 PH 原生文件；q 点来自 dyn0，不扫描科学清单。
pub fn phonon_inputs(cfg: &super::config::Config) -> Result<Vec<PathBuf>> {
    let save = super::stages::Step::Phonon.directory(cfg).join("save");
    let prefix = &cfg.qe.prefix;
    let dyn0 = save.join(format!("{prefix}.dyn0.xml"));
    crate::qe_data::save::nonempty(&dyn0)?;
    let xml = super::layout::path(&cfg.work_dir, "nscf/out")
        .join(format!("{prefix}.save/data-file-schema.xml"));
    let geo = crate::qe_data::xml::Geometry::read(&xml)?;
    geo.validate(&crate::qe_data::qe_config::QeSettings::from_config(cfg)?)?;
    let points = crate::qe_data::qmap::parse_dyn0(&dyn0, &geo, cfg.phonon.q_grid, false)?;
    let control = save.join(format!("{prefix}.phsave/control_ph.xml"));
    crate::qe_data::native_ph::control(&control, &points)?;
    let mut paths = vec![dyn0, control];
    for point in points {
        paths.push(save.join(format!("{prefix}.dyn_q{}.xml", point.global_index)));
        paths.push(save.join(format!("{prefix}.dvscf_q{}", point.global_index)));
        paths.push(save.join(format!(
            "{prefix}.phsave/patterns.{}.xml",
            point.global_index
        )));
    }
    for path in &paths {
        crate::qe_data::save::nonempty(path)?;
    }
    Ok(paths)
}

/// 分配提交前只检查直接元数据，不检查原始密度／波函数，不补算。
pub fn check_inputs(cfg: &super::config::Config, step: super::stages::Step) -> Result<()> {
    use super::stages::Step;
    let scf = || {
        super::layout::path(&cfg.work_dir, "scf/snapshot")
            .join(format!("{}.save/data-file-schema.xml", cfg.qe.prefix))
    };
    let nscf = || {
        super::layout::path(&cfg.work_dir, "nscf/out")
            .join(format!("{}.save/data-file-schema.xml", cfg.qe.prefix))
    };
    let paths = match step {
        Step::Scf => Vec::new(),
        Step::Nscf | Step::Phonon => vec![scf()],
        Step::Electronic => vec![scf(), nscf()],
        Step::Wannier => vec![nscf()],
        Step::Coarse => {
            let mut files = vec![nscf()];
            for suffix in ["ukk", "mmn", "bvec"] {
                files.push(
                    Step::Wannier
                        .directory(cfg)
                        .join(format!("{}.{suffix}", cfg.qe.prefix)),
                );
            }
            for path in &files {
                crate::qe_data::save::nonempty(path)?;
            }
            files.extend(phonon_inputs(cfg)?);
            files
        }
        Step::Fine | Step::Superconductivity => Vec::new(),
    };
    for path in paths {
        crate::qe_data::save::nonempty(&path)?;
    }
    Ok(())
}
