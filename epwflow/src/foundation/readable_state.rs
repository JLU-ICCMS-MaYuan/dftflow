//! 八阶段唯一可读状态；日志记录当前事实，任务号就近保存。
use super::{config::Config, stages::Step, store};
use anyhow::{ensure, Context, Result};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    os::fd::AsRawFd,
    path::{Path, PathBuf},
};

pub type Fields = BTreeMap<String, String>;
/// 旧嵌套配置仅限已显式绑定外部来源的工作区继续读取，公开配置仍严格校验。
pub(crate) fn load_config(path: &Path) -> Result<Config> {
    match Config::load(path) {
        Ok(cfg) => Ok(cfg),
        Err(original) => match Config::load_nested(path) {
            Ok(cfg) if super::phonon_import::bound(&cfg) => Ok(cfg),
            _ => Err(original),
        },
    }
}
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub sections: BTreeMap<String, Fields>,
}

/// 锁定稳定的父目录 inode，不创建会残留的锁文件；进程退出即释放。
pub struct DirectoryGuard {
    file: Option<File>,
    path: PathBuf,
}
thread_local! {static HELD:std::cell::RefCell<std::collections::HashSet<PathBuf>>=std::cell::RefCell::new(std::collections::HashSet::new());}
thread_local! {static EXECUTION:std::cell::RefCell<Option<(PathBuf,PathBuf)>>=const {std::cell::RefCell::new(None)};}
/// 分配内读取原文版本；其他 CLI 修改当前输入不会改变已排队任务。
pub struct ExecutionConfig(Option<(PathBuf, PathBuf)>);
impl ExecutionConfig {
    pub fn enter(root: &Path, version: &Path) -> Self {
        Self(EXECUTION.with(|slot| slot.replace(Some((root.into(), version.into())))))
    }
}
impl Drop for ExecutionConfig {
    fn drop(&mut self) {
        EXECUTION.with(|slot| slot.replace(self.0.take()));
    }
}
impl DirectoryGuard {
    pub fn acquire(path: &Path) -> Result<Self> {
        fs::create_dir_all(path)?;
        let path = fs::canonicalize(path)?;
        if HELD.with(|held| held.borrow().contains(&path)) {
            return Ok(Self { file: None, path });
        }
        let file = File::open(&path)?;
        loop {
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } == 0 {
                break;
            }
            let e = std::io::Error::last_os_error();
            if e.kind() != std::io::ErrorKind::Interrupted {
                return Err(e.into());
            }
        }
        HELD.with(|held| held.borrow_mut().insert(path.clone()));
        Ok(Self {
            file: Some(file),
            path,
        })
    }
}
impl Drop for DirectoryGuard {
    fn drop(&mut self) {
        if let Some(file) = &self.file {
            unsafe {
                libc::flock(file.as_raw_fd(), libc::LOCK_UN);
            }
            HELD.with(|held| held.borrow_mut().remove(&self.path));
        }
    }
}

pub fn stage_name(step: Step, mode: &str) -> String {
    match step {
        Step::Scf => "1.qe-scf".into(),
        Step::Nscf => "2.qe-nscf".into(),
        Step::Electronic => "3.qe-electronic".into(),
        Step::Phonon => format!("4.qe-phonon-{mode}"),
        Step::Wannier => "5.epw-wannier".into(),
        Step::Coarse => "6.epw-coarse".into(),
        Step::Fine => "7.epw-fine".into(),
        Step::Superconductivity => "8.epw-superconductivity".into(),
    }
}
pub fn mode(cfg: &Config) -> &'static str {
    use super::config::PhMode;
    match cfg.phonon.mode {
        PhMode::Nosplit => "nosplit",
        PhMode::Coords => "coords",
        PhMode::Index => "index",
    }
}
pub fn mode_in(root: &Path) -> Result<String> {
    for mode in ["nosplit", "coords", "index"] {
        if root
            .join("epw-state")
            .join(format!("4.qe-phonon-{mode}.log"))
            .is_file()
        {
            return Ok(mode.into());
        }
    }
    let legacy = root.join("epw-state/layout.json");
    if legacy.is_file() {
        let value: serde_json::Value = store::read(&legacy)?;
        if let Some(mode) = value["phonon"]
            .as_str()
            .filter(|m| matches!(*m, "nosplit" | "coords" | "index"))
        {
            return Ok(mode.into());
        }
    }
    let found: Vec<_> = ["nosplit", "coords", "index"]
        .into_iter()
        .filter(|mode| root.join(format!("4.qe-phonon-{mode}")).is_dir())
        .collect();
    ensure!(
        found.len() <= 1,
        "input_conflict: 多个声子目录尚未确定当前模式；请通过当前配置初始化工作区"
    );
    Ok(found.first().copied().unwrap_or("index").into())
}
pub fn path(root: &Path, step: Step) -> Result<PathBuf> {
    Ok(root
        .join("epw-state")
        .join(format!("{}.log", stage_name(step, &mode_in(root)?))))
}
fn title(step: Step) -> &'static str {
    match step {
        Step::Scf => "第 1 步 · QE 自洽",
        Step::Nscf => "第 2 步 · QE 非自洽",
        Step::Electronic => "第 3 步 · QE 电子参考",
        Step::Phonon => "第 4 步 · QE 声子",
        Step::Wannier => "第 5 步 · Wannier 拟合",
        Step::Coarse => "第 6 步 · EPW 粗网格",
        Step::Fine => "第 7 步 · EPW 细网格",
        Step::Superconductivity => "第 8 步 · 超导求解",
    }
}
impl Document {
    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.sections.get(section)?.get(key).map(String::as_str)
    }
    pub fn set(&mut self, section: &str, key: &str, value: impl ToString) {
        self.sections
            .entry(section.into())
            .or_default()
            .insert(key.into(), value.to_string());
    }
    pub fn remove(&mut self, section: &str) {
        self.sections.remove(section);
    }
    fn parse(text: &str, file: &Path) -> Result<Self> {
        ensure!(
            text.ends_with("记录结束\n"),
            "data_invalid: 阶段日志未写完整 {}；请核对实际任务和输出后恢复",
            file.display()
        );
        let mut doc = Self::default();
        let mut section = None;
        for (n, line) in text.lines().enumerate() {
            if let Some(name) = line.strip_prefix("【").and_then(|s| s.strip_suffix('】')) {
                ensure!(
                    !doc.sections.contains_key(name),
                    "data_invalid: {}:{} 重复小节",
                    file.display(),
                    n + 1
                );
                doc.sections.insert(name.into(), Fields::new());
                section = Some(name.to_owned());
            } else if let Some((key, value)) =
                line.strip_prefix("  ").and_then(|s| s.split_once('：'))
            {
                let section = section.as_ref().context("data_invalid: 日志字段缺少小节")?;
                ensure!(
                    doc.sections
                        .get_mut(section)
                        .unwrap()
                        .insert(key.trim().into(), value.trim().into())
                        .is_none(),
                    "data_invalid: {}:{} 重复字段",
                    file.display(),
                    n + 1
                );
            }
        }
        Ok(doc)
    }
    fn render(&self, step: Step) -> String {
        let mut out = format!("{}\n{}\n", title(step), "=".repeat(64));
        let order = ["工作区", "阶段", "问题", "下一步"];
        for name in order
            .into_iter()
            .filter(|n| self.sections.contains_key(*n))
            .chain(
                self.sections
                    .keys()
                    .map(String::as_str)
                    .filter(|n| !order.contains(n)),
            )
        {
            out.push_str(&format!("\n【{name}】\n"));
            for (key, value) in &self.sections[name] {
                let value = value.replace(['\n', '\r'], " / ");
                let width = key
                    .chars()
                    .map(|c| if c.is_ascii() { 1 } else { 2 })
                    .sum::<usize>();
                out.push_str(&format!(
                    "  {key}{}：{value}\n",
                    " ".repeat(18usize.saturating_sub(width))
                ));
            }
        }
        out.push_str(&format!("\n{}\n记录结束\n", "=".repeat(64)));
        out
    }
}
pub fn read(root: &Path, step: Step) -> Result<Document> {
    let p = path(root, step)?;
    match fs::read_to_string(&p) {
        Ok(text) => Document::parse(&text, &p),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Document::default()),
        Err(e) => Err(e).with_context(|| format!("data_invalid: 无法读取阶段日志 {}", p.display())),
    }
}
pub fn update(
    root: &Path,
    step: Step,
    change: impl FnOnce(&mut Document) -> Result<()>,
) -> Result<()> {
    let state = root.join("epw-state");
    let _guard = DirectoryGuard::acquire(&state)?;
    let mut doc = read(root, step)?;
    let previous = doc.clone();
    change(&mut doc)?;
    if doc == previous {
        return Ok(());
    }
    doc.set("工作区", "更新时间", timestamp());
    store::atomic_bytes(&path(root, step)?, doc.render(step).as_bytes())
}
fn timestamp() -> String {
    // 固定标注时区，不依赖集群进程继承的 TZ。
    let now = unsafe { libc::time(std::ptr::null_mut()) } + 8 * 3600;
    let mut tm = std::mem::MaybeUninit::<libc::tm>::uninit();
    unsafe {
        libc::gmtime_r(&now, tm.as_mut_ptr());
    }
    let tm = unsafe { tm.assume_init() };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} +08:00",
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min,
        tm.tm_sec
    )
}
pub fn initialize(cfg: &Config) -> Result<()> {
    let root = &cfg.work_dir;
    let state = root.join("epw-state");
    ensure!(
        !fs::symlink_metadata(&state).is_ok_and(|m| m.file_type().is_symlink()),
        "input_conflict: 状态目录不能是链接 {}",
        state.display()
    );
    let _guard = DirectoryGuard::acquire(&state)?;
    // 同一目录锁排除了仍在写入的进程；仅撤下原子写入中断留下的临时文件。
    super::legacy_state::retire_locks(&state)?;
    for entry in fs::read_dir(&state)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(".epw-") && name.ends_with(".tmp") && entry.file_type()?.is_file() {
            fs::remove_file(entry.path())?;
        }
    }
    for other in ["nosplit", "coords", "index"]
        .into_iter()
        .filter(|m| *m != mode(cfg))
    {
        let old = state.join(format!("4.qe-phonon-{other}.log"));
        if old.exists() {
            let doc = Document::parse(&fs::read_to_string(&old)?, &old)?;
            let jobs = jobs(root, Step::Phonon)?;
            for (name, section) in &doc.sections {
                if section.get("状态").is_some_and(|v| {
                    matches!(v.as_str(), "运行中" | "提交中" | "提交未知" | "已提交")
                }) {
                    let job = jobs
                        .get(name)
                        .context("scheduler_unknown: 旧声子模式提交缺少可确认的作业号")?;
                    ensure!(
                        super::slurm::accounting(*job)?.terminal,
                        "scheduler_unknown: 旧声子模式 {} 作业 {} 仍活动",
                        other,
                        job
                    );
                }
            }
            fs::remove_file(old)?;
        }
    }
    for step in Step::ALL {
        let file = state.join(format!("{}.log", stage_name(step, mode(cfg))));
        let mut doc = if file.exists() {
            Document::parse(&fs::read_to_string(&file)?, &file)?
        } else {
            Document::default()
        };
        let previous = doc.clone();
        doc.set("工作区", "布局", "八阶段编号目录");
        doc.set("工作区", "声子模式", mode(cfg));
        doc.set(
            "工作区",
            "计算目录",
            format!("{}/", stage_name(step, mode(cfg))),
        );
        if doc.get("阶段", "状态").is_none() {
            doc.set("阶段", "状态", "未开始");
        }
        if doc.get("阶段", "科学状态").is_none() {
            doc.set("阶段", "科学状态", "尚未检查");
        }
        if doc.get("下一步", "操作").is_none() {
            doc.set("下一步", "操作", "使用当前配置执行本阶段公开命令");
        }
        if doc != previous {
            doc.set("工作区", "更新时间", timestamp());
            store::atomic_bytes(&file, doc.render(step).as_bytes())?;
        }
    }
    Ok(())
}

pub fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}
pub fn resolve(root: &Path, value: &str) -> PathBuf {
    root.join(value)
}
pub fn jobs_path(root: &Path, step: Step) -> Result<PathBuf> {
    let name = stage_name(step, &mode_in(root)?);
    Ok(root.join(&name).join(format!("{name}.jobs")))
}
pub fn jobs(root: &Path, step: Step) -> Result<BTreeMap<String, u64>> {
    let p = jobs_path(root, step)?;
    let text = match fs::read_to_string(&p) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(e) => return Err(e.into()),
    };
    ensure!(
        text.starts_with("子任务\tSlurm 作业号\n"),
        "data_invalid: 任务号表头不完整 {}",
        p.display()
    );
    let mut values = BTreeMap::new();
    for line in text.lines().skip(1).filter(|l| !l.trim().is_empty()) {
        let (name, job) = line
            .split_once('\t')
            .context("data_invalid: 任务号表应为子任务和 Slurm 作业号两列")?;
        let job = job
            .parse::<u64>()
            .with_context(|| format!("data_invalid: 任务号无效 {}", p.display()))?;
        ensure!(
            job > 0 && values.insert(name.into(), job).is_none(),
            "data_invalid: 任务号表重复或无效"
        );
    }
    Ok(values)
}
pub fn set_job(root: &Path, step: Step, name: &str, job: Option<u64>) -> Result<()> {
    ensure!(
        !name.contains(['\t', '\n', '\r']),
        "configuration: 子任务名称含换行"
    );
    let _guard = DirectoryGuard::acquire(&root.join("epw-state"))?;
    let mut values = jobs(root, step)?;
    if let Some(job) = job {
        ensure!(job > 0, "data_invalid: 作业号为零");
        values.insert(name.into(), job);
    } else {
        values.remove(name);
    }
    let mut text = String::from("子任务\tSlurm 作业号\n");
    for (name, job) in values {
        text.push_str(&format!("{name}\t{job}\n"));
    }
    store::atomic_bytes(&jobs_path(root, step)?, text.as_bytes())
}

pub fn backup_config(input: &Path) -> Result<PathBuf> {
    let input = fs::canonicalize(input)
        .with_context(|| format!("configuration: 无法读取 {}", input.display()))?;
    let parent = input.parent().context("configuration: 输入缺父目录")?;
    let _guard = DirectoryGuard::acquire(parent)?;
    let bytes = fs::read(&input)?;
    let mut versions = Vec::new();
    for entry in fs::read_dir(parent)? {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if let Some(number) = name
            .strip_suffix("-config.toml")
            .filter(|s| s.len() >= 4 && s.bytes().all(|c| c.is_ascii_digit()))
        {
            versions.push((number.parse::<u64>()?, path));
        }
    }
    versions.sort_by_key(|(n, _)| *n);
    if let Some((_, path)) = versions.last() {
        if fs::read(path)? == bytes {
            return Ok(path.clone());
        }
    }
    let number = versions
        .last()
        .map(|(n, _)| n.checked_add(1).context("data_invalid: 配置版本编号溢出"))
        .transpose()?
        .unwrap_or(0);
    let output = parent.join(format!("{number:04}-config.toml"));
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    f.write_all(&bytes)?;
    f.sync_all()?;
    File::open(parent)?.sync_all()?;
    Ok(output)
}

pub fn register_config(cfg: &Config, input: &Path, version: &Path) -> Result<()> {
    let conversion = super::legacy_state::prepare(cfg)?;
    initialize(cfg)?;
    let input = fs::canonicalize(input)?;
    for step in Step::ALL {
        update(&cfg.work_dir, step, |doc| {
            doc.set("工作区", "当前输入", relative(&cfg.work_dir, &input));
            doc.set("工作区", "当前输入版本", relative(&cfg.work_dir, version));
            Ok(())
        })?;
        let command = next_command(cfg, step, false)?;
        update(&cfg.work_dir, step, |doc| {
            doc.set("下一步", "命令", &command);
            Ok(())
        })?;
    }
    if let Some(conversion) = conversion {
        super::legacy_state::finish(cfg, version, conversion)?;
    }
    Ok(())
}
pub fn current_config(root: &Path) -> Result<PathBuf> {
    if let Some(version) = EXECUTION.with(|slot| {
        slot.borrow()
            .as_ref()
            .filter(|(work, _)| work == root)
            .map(|(_, version)| version.clone())
    }) {
        return Ok(version);
    }
    let doc = read(root, Step::Scf)?;
    let value = doc
        .get("工作区", "当前输入版本")
        .context("configuration: 尚未记录当前配置；请使用 --config 指定输入")?;
    Ok(resolve(root, value))
}
pub fn computation_config(root: &Path, step: Step) -> Result<PathBuf> {
    let doc = read(root, step)?;
    match doc.get("阶段", "计算配置") {
        Some(p) => Ok(resolve(root, p)),
        None => current_config(root),
    }
}

/// 记录一个已通过调用方科学检查的子步骤；不复制清单或请求指纹。
pub fn complete_files(cfg: &Config, step: Step, label: &str, outputs: &[PathBuf]) -> Result<()> {
    let config = current_config(&cfg.work_dir)?;
    update(&cfg.work_dir, step, |d| {
        let batch = d.get("阶段", "批次").unwrap_or("existing").to_owned();
        d.remove(label);
        d.set(label, "状态", "已完成");
        d.set(label, "所属批次", batch);
        d.set(label, "使用配置", relative(&cfg.work_dir, &config));
        for (i, path) in outputs.iter().enumerate() {
            d.set(
                label,
                &format!("产物{:03}", i + 1),
                relative(&cfg.work_dir, path),
            );
        }
        Ok(())
    })
}
pub fn completed_files(
    cfg: &Config,
    step: Step,
    label: &str,
) -> Result<Option<Vec<store::Identity>>> {
    let doc = read(&cfg.work_dir, step)?;
    if doc.get(label, "状态") != Some("已完成")
        || doc.get(label, "所属批次") != Some(doc.get("阶段", "批次").unwrap_or("existing"))
    {
        return Ok(None);
    }
    let mut ids = Vec::new();
    if let Some(fields) = doc.sections.get(label) {
        for (key, value) in fields {
            if key.starts_with("产物") {
                let p = resolve(&cfg.work_dir, value);
                if !p.is_file() {
                    return Ok(None);
                }
                ids.push(store::Identity::capture(&p)?);
            }
        }
    }
    Ok(Some(ids))
}
pub fn checkpoint_config(cfg: &Config, step: Step, label: &str) -> Result<Option<Config>> {
    read(&cfg.work_dir, step)?
        .get(label, "使用配置")
        .map(|p| load_config(&resolve(&cfg.work_dir, p)))
        .transpose()
}

pub fn next_command(cfg: &Config, step: Step, resume: bool) -> Result<String> {
    let program = match step {
        Step::Scf | Step::Nscf | Step::Electronic | Step::Phonon => "epw_phonon",
        Step::Wannier | Step::Coarse => "epw_wannier",
        _ => "epw_superconductivity",
    };
    let doc = read(&cfg.work_dir, step)?;
    let input = doc
        .get("工作区", "当前输入")
        .map(|s| resolve(&cfg.work_dir, s))
        .unwrap_or(current_config(&cfg.work_dir)?);
    Ok(format!(
        "{program} -c {} --stage {}{} -s",
        shell_words::quote(&input.to_string_lossy()),
        step.name(),
        if resume { " --resume" } else { "" }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn root() -> PathBuf {
        let p = std::env::temp_dir().join(format!("readable-state-{}", store::token()));
        fs::create_dir_all(&p).unwrap();
        p
    }
    fn cfg(root: &Path) -> Config {
        let mut c: Config =
            toml::from_str(include_str!("../../tests/fixtures/input.toml")).unwrap();
        c.work_dir = root.into();
        c
    }
    #[test]
    fn eight_human_logs_and_jobs_survive_independent_updates() {
        let root = root();
        let c = cfg(&root);
        initialize(&c).unwrap();
        assert_eq!(fs::read_dir(root.join("epw-state")).unwrap().count(), 8);
        std::thread::scope(|scope| {
            for method in ["iso", "aniso"] {
                let root = &root;
                scope.spawn(move || {
                    update(root, Step::Superconductivity, |d| {
                        d.set(&format!("任务 · {method}"), "状态", "已完成");
                        Ok(())
                    })
                    .unwrap();
                });
            }
        });
        let d = read(&root, Step::Superconductivity).unwrap();
        for method in ["iso", "aniso"] {
            assert_eq!(d.get(&format!("任务 · {method}"), "状态"), Some("已完成"));
        }
        set_job(&root, Step::Superconductivity, "iso", Some(123)).unwrap();
        assert_eq!(jobs(&root, Step::Superconductivity).unwrap()["iso"], 123);
        let text = fs::read_to_string(path(&root, Step::Superconductivity).unwrap()).unwrap();
        assert!(text.contains("第 8 步 · 超导求解") && !text.contains("{\n"));
        assert_eq!(fs::read_dir(root.join("epw-state")).unwrap().count(), 8);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn raw_config_versions_capture_a_a_b_a_without_expanding_toml() {
        let root = root();
        let input = root.join("config.toml");
        for (raw, expected) in [
            ("# A\nx = 1\n", "0000-config.toml"),
            ("# A\nx = 1\n", "0000-config.toml"),
            ("# B\nx = 2\n", "0001-config.toml"),
            ("# A\nx = 1\n", "0002-config.toml"),
        ] {
            fs::write(&input, raw).unwrap();
            let version = backup_config(&input).unwrap();
            assert_eq!(version.file_name().unwrap(), expected);
            assert_eq!(fs::read_to_string(version).unwrap(), raw);
        }
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn truncated_log_and_unknown_submission_are_never_read_as_unstarted() {
        let root = root();
        let c = cfg(&root);
        initialize(&c).unwrap();
        update(&root, Step::Scf, |d| {
            d.set("任务 · scf", "状态", "提交未知");
            Ok(())
        })
        .unwrap();
        assert_eq!(
            read(&root, Step::Scf).unwrap().get("任务 · scf", "状态"),
            Some("提交未知")
        );
        fs::write(
            path(&root, Step::Scf).unwrap(),
            "第 1 步\n【阶段】\n  状态：已完成\n",
        )
        .unwrap();
        assert!(read(&root, Step::Scf).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
