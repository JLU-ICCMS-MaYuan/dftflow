//! 八阶段唯一可读状态；日志记录当前事实，任务号就近保存。
use super::{config::Config, stages::Step, store};
use anyhow::{ensure, Context, Result};
use std::{
    collections::BTreeMap,
    fs::{self, File},
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
    legacy_language: bool,
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
/// 复用提示统一使用实际编号阶段名，子任务与目录层级一致。
pub fn reuse_message(cfg: &Config, step: Step, task: Option<&str>) -> String {
    let stage = stage_name(step, mode(cfg));
    match task {
        Some(task) => format!("reuse: {stage} /{task}"),
        None => format!("reuse: {stage}"),
    }
}
pub fn report_reuse(cfg: &Config, step: Step, task: Option<&str>) {
    println!("stage_complete: {}{}",stage_name(step,mode(cfg)),task.map(|t|format!(" /{t}")).unwrap_or_default());
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
        "input_conflict: multiple phonon directories exist without a current mode; initialize the workspace with the current configuration"
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
        Step::Scf => "Step 1 · QE SCF",
        Step::Nscf => "Step 2 · QE NSCF",
        Step::Electronic => "Step 3 · QE electronic reference",
        Step::Phonon => "Step 4 · QE phonons",
        Step::Wannier => "Step 5 · Wannier fitting",
        Step::Coarse => "Step 6 · EPW coarse grid",
        Step::Fine => "Step 7 · EPW fine grid",
        Step::Superconductivity => "Step 8 · Superconductivity solver",
    }
}
fn split_field(line: &str) -> Option<(&str, &str)> {
    let index = line.find([':', '：'])?;
    let delimiter = line[index..].chars().next()?;
    Some((&line[..index], &line[index + delimiter.len_utf8()..]))
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
            text.ends_with("End of record\n") || text.ends_with(super::legacy_language::FOOTER),
            "data_invalid: stage log incomplete {}; verify actual jobs and output before recovery",
            file.display()
        );
        let mut doc = Self { legacy_language: text.ends_with(super::legacy_language::FOOTER), ..Self::default() };
        let mut section = None;
        for (n, line) in text.lines().enumerate() {
            if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']'))
                .or_else(|| line.strip_prefix('【').and_then(|s| s.strip_suffix('】'))) {
                let name = super::legacy_language::translate(name).into_owned();
                ensure!(
                    !doc.sections.contains_key(&name),
                    "data_invalid: {}:{} duplicate section",
                    file.display(),
                    n + 1
                );
                doc.sections.insert(name.clone(), Fields::new());
                section = Some(name.to_owned());
            } else if let Some((key, value)) =
                line.strip_prefix("  ").and_then(split_field)
            {
                let section = section.as_ref().context("data_invalid: log field has no section")?;
                let key = super::legacy_language::translate(key.trim()).into_owned();
                let value = super::legacy_language::value(&key, value.trim()).into_owned();
                ensure!(
                    doc.sections
                        .get_mut(section)
                        .unwrap()
                        .insert(key, value)
                        .is_none(),
                    "data_invalid: {}:{} duplicate field",
                    file.display(),
                    n + 1
                );
            }
        }
        Ok(doc)
    }
    fn render(&self, step: Step) -> String {
        let mut out = format!("{}\n{}\n", title(step), "=".repeat(64));
        let order = ["Workspace", "Stage", "Problem", "Next step"];
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
            out.push_str(&format!("\n[{name}]\n"));
            for (key, value) in &self.sections[name] {
                let value = value.replace(['\n', '\r'], " / ");
                let width = key
                    .chars()
                    .map(|c| if c.is_ascii() { 1 } else { 2 })
                    .sum::<usize>();
                out.push_str(&format!(
                    "  {key}{}: {value}\n",
                    " ".repeat(18usize.saturating_sub(width))
                ));
            }
        }
        out.push_str(&format!("\n{}\nEnd of record\n", "=".repeat(64)));
        out
    }
}
pub fn read(root: &Path, step: Step) -> Result<Document> {
    let p = path(root, step)?;
    match fs::read_to_string(&p) {
        Ok(text) => Document::parse(&text, &p),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Document::default()),
        Err(e) => Err(e).with_context(|| format!("data_invalid: cannot read stage log {}", p.display())),
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
    if doc == previous && !doc.legacy_language {
        return Ok(());
    }
    doc.set("Workspace", "Updated at", timestamp());
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
        "input_conflict: state directory cannot be a symlink {}",
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
                if section.get("Status").is_some_and(|v| {
                    matches!(v.as_str(), "running" | "submitting" | "submission unknown" | "submitted")
                }) {
                    let job = jobs
                        .get(name)
                        .context("scheduler_unknown: old phonon-mode submission has no confirmed job ID")?;
                    ensure!(
                        super::slurm::accounting(*job)?.terminal,
                        "scheduler_unknown: old phonon-mode {} job {} remains active",
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
        doc.set("Workspace", "Layout", "Eight numbered stages");
        doc.set("Workspace", "Phonon mode", mode(cfg));
        doc.set(
            "Workspace",
            "Calculation directory",
            format!("{}/", stage_name(step, mode(cfg))),
        );
        if doc.get("Stage", "Status").is_none() {
            doc.set("Stage", "Status", "not started");
        }
        if doc.get("Stage", "Scientific status").is_none() {
            doc.set("Stage", "Scientific status", "not checked");
        }
        if doc.get("Next step", "Action").is_none() {
            doc.set("Next step", "Action", "Run this stage public command with the current configuration");
        }
        if doc != previous {
            doc.set("Workspace", "Updated at", timestamp());
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
        text.starts_with("Task\tSlurm job ID\n") || text.starts_with(super::legacy_language::JOB_HEADER),
        "data_invalid: incomplete job-table header {}",
        p.display()
    );
    let mut values = BTreeMap::new();
    for line in text.lines().skip(1).filter(|l| !l.trim().is_empty()) {
        let (name, job) = line
            .split_once('\t')
            .context("data_invalid: job table must contain task and Slurm job ID columns")?;
        let job = job
            .parse::<u64>()
            .with_context(|| format!("data_invalid: invalid job ID {}", p.display()))?;
        ensure!(
            job > 0 && values.insert(super::legacy_language::translate(name).into_owned(), job).is_none(),
            "data_invalid: duplicate or invalid job-table entry"
        );
    }
    Ok(values)
}
pub fn set_job(root: &Path, step: Step, name: &str, job: Option<u64>) -> Result<()> {
    ensure!(
        !name.contains(['\t', '\n', '\r']),
        "configuration: subtask name contains a newline"
    );
    let _guard = DirectoryGuard::acquire(&root.join("epw-state"))?;
    let mut values = jobs(root, step)?;
    if let Some(job) = job {
        ensure!(job > 0, "data_invalid: job ID is zero");
        values.insert(name.into(), job);
    } else {
        values.remove(name);
    }
    let mut text = String::from("Task\tSlurm job ID\n");
    for (name, job) in values {
        text.push_str(&format!("{name}\t{job}\n"));
    }
    store::atomic_bytes(&jobs_path(root, step)?, text.as_bytes())
}

pub fn backup_config(input: &Path) -> Result<PathBuf> {
    super::config_versions::backup(input)
}

pub fn register_config(cfg: &Config, input: &Path, version: &Path) -> Result<()> {
    let conversion = super::legacy_state::prepare(cfg)?;
    initialize(cfg)?;
    let input = fs::canonicalize(input)?;
    for step in Step::ALL {
        update(&cfg.work_dir, step, |doc| {
            doc.set("Workspace", "Current input", relative(&cfg.work_dir, &input));
            doc.set("Workspace", "Current input version", relative(&cfg.work_dir, version));
            Ok(())
        })?;
        let command = next_command(cfg, step, false)?;
        update(&cfg.work_dir, step, |doc| {
            doc.set("Next step", "Command", &command);
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
        .get("Workspace", "Current input version")
        .context("configuration: The current configuration has not been recorded; please use --config Specify Inputs")?;
    Ok(resolve(root, value))
}
pub fn computation_config(root: &Path, step: Step) -> Result<PathBuf> {
    let doc = read(root, step)?;
    match doc.get("Stage", "Calculation configuration") {
        Some(p) => Ok(resolve(root, p)),
        None => current_config(root),
    }
}

/// 记录一个已通过调用方科学检查的子步骤；不复制清单或请求指纹。
pub fn complete_files(cfg: &Config, step: Step, label: &str, outputs: &[PathBuf]) -> Result<()> {
    let config = current_config(&cfg.work_dir)?;
    update(&cfg.work_dir, step, |d| {
        let batch = d.get("Stage", "Batch").unwrap_or("existing").to_owned();
        d.remove(label);
        d.set(label, "Status", "completed");
        d.set(label, "Owning batch", batch);
        d.set(label, "Configuration used", relative(&cfg.work_dir, &config));
        let _ = outputs;
        Ok(())
    })
}
pub fn completed_files(
    cfg: &Config,
    step: Step,
    label: &str,
) -> Result<Option<Vec<store::Identity>>> {
    let doc = read(&cfg.work_dir, step)?;
    if doc.get(label, "Status") != Some("completed")
        || doc.get(label, "Owning batch") != Some(doc.get("Stage", "Batch").unwrap_or("existing"))
    {
        return Ok(None);
    }
    Ok(Some(Vec::new()))
}
pub fn checkpoint_config(cfg: &Config, step: Step, label: &str) -> Result<Option<Config>> {
    read(&cfg.work_dir, step)?
        .get(label, "Configuration used")
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
        .get("Workspace", "Current input")
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
    #[test]
    fn legacy_language_preserves_identity_paths_and_unknown_submissions() {
        let root = root();
        let c = cfg(&root);
        initialize(&c).unwrap();
        let path = path(&root, Step::Scf).unwrap();
        let legacy = include_str!("../../tests/fixtures/legacy-chinese-state.log");
        fs::write(&path, legacy).unwrap();
        let doc = read(&root, Step::Scf).unwrap();
        assert_eq!(doc.get("Stage", "Status"), Some("running"));
        assert_eq!(doc.get("Stage", "Batch"), Some("retained-batch"));
        assert_eq!(doc.get("Stage allocation", "Status"), Some("submission unknown"));
        assert_eq!(doc.get("Stage allocation", "Submission ID"), Some("retained-allocation"));
        assert_eq!(doc.get("Task · scf", "Status"), Some("completed"));
        let origin = doc.get("Configuration snapshot · 0000-config.toml", "Original directory").unwrap();
        assert!(legacy.contains(origin));
        assert_eq!(doc.get("Problem", "Reason"), Some("data_invalid: stage log incomplete broken.log; verify actual jobs and output before recovery"));
        assert_eq!(doc.get("Next step", "Action"), Some("After the job finishes, run: epw_phonon -c input.toml --stage scf -s"));
        assert_eq!(fs::read_to_string(&path).unwrap(), legacy, "read must be nonmutating");
        update(&root, Step::Scf, |_| Ok(())).unwrap();
        let rewritten = fs::read_to_string(&path).unwrap();
        assert!(rewritten.contains("[Stage allocation]") && rewritten.ends_with("End of record\n"));
        // 用户提供的路径原样保留；其余字段和值均采用英文。
        assert!(!rewritten.replace(origin, "preserved-user-path").chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c)));
        let mut after = read(&root, Step::Scf).unwrap();
        after.sections.get_mut("Workspace").unwrap().remove("Updated at");
        assert_eq!(after.sections, doc.sections);
        let mixed = legacy.replace(super::super::legacy_language::FOOTER, "[Stage]\n  Status: completed\nEnd of record\n");
        fs::write(&path, mixed).unwrap();
        assert!(read(&root, Step::Scf).unwrap_err().to_string().contains("duplicate section"));
        fs::write(&path, legacy.trim_end_matches(super::super::legacy_language::FOOTER)).unwrap();
        assert!(read(&root, Step::Scf).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn legacy_job_header_preserves_allocation_job_identity_and_rejects_alias_duplicates() {
        let root = root();
        let c = cfg(&root);
        initialize(&c).unwrap();
        let p = jobs_path(&root, Step::Scf).unwrap();
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        let legacy = include_str!("../../tests/fixtures/legacy-chinese-jobs.txt").to_owned();
        fs::write(&p, &legacy).unwrap();
        assert_eq!(jobs(&root, Step::Scf).unwrap().get("Stage allocation"), Some(&201));
        assert_eq!(fs::read_to_string(&p).unwrap(), legacy);
        set_job(&root, Step::Scf, "scf", Some(202)).unwrap();
        let en = fs::read_to_string(&p).unwrap();
        assert_eq!(en, "Task\tSlurm job ID\nStage allocation\t201\nscf\t202\n");
        fs::write(&p, format!("{legacy}Stage allocation\t203\n")).unwrap();
        assert!(jobs(&root, Step::Scf).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reuse_messages_identify_every_numbered_stage() {
        let root = root();
        let cfg = cfg(&root);
        for step in Step::ALL {
            assert_eq!(
                reuse_message(&cfg, step, None),
                format!("reuse: {}", stage_name(step, mode(&cfg)))
            );
        }
        assert_eq!(
            reuse_message(&cfg, Step::Electronic, Some("projwfc")),
            "reuse: 3.qe-electronic /projwfc"
        );
        fs::remove_dir_all(root).unwrap();
    }
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
                        d.set(&format!("Task · {method}"), "Status", "completed");
                        Ok(())
                    })
                    .unwrap();
                });
            }
        });
        let d = read(&root, Step::Superconductivity).unwrap();
        for method in ["iso", "aniso"] {
            assert_eq!(d.get(&format!("Task · {method}"), "Status"), Some("completed"));
        }
        set_job(&root, Step::Superconductivity, "iso", Some(123)).unwrap();
        assert_eq!(jobs(&root, Step::Superconductivity).unwrap()["iso"], 123);
        let text = fs::read_to_string(path(&root, Step::Superconductivity).unwrap()).unwrap();
        assert!(text.contains("Step 8 · Superconductivity solver") && !text.contains("{\n"));
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
            d.set("Task · scf", "Status", "submission unknown");
            Ok(())
        })
        .unwrap();
        assert_eq!(
            read(&root, Step::Scf).unwrap().get("Task · scf", "Status"),
            Some("submission unknown")
        );
        fs::write(
            path(&root, Step::Scf).unwrap(),
            "Step 1\n[Stage]\n  Status:completed\n",
        )
        .unwrap();
        assert!(read(&root, Step::Scf).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
