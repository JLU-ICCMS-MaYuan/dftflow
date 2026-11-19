//! 第 8 步独立绘图；图表失败只记录警告，不改变科学结果或完成状态。
pub mod coupling;
pub mod dos;
pub mod gap_frequency;
pub mod gap_temperature;
pub mod io;
use crate::foundation::config::Config;
use anyhow::{ensure, Context, Result};
pub use qf_config::PlotFormat;
use qf_plot::superconductivity::Panel;
use rayon::prelude::*;
use std::{
    cell::Cell,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

thread_local! { static SKIP: Cell<bool> = const { Cell::new(false) }; }
/// CLI 当前调用内禁用自动绘图，不修改配置或后续调用。
pub fn set_skip(skip: bool) {
    SKIP.set(skip);
}

#[derive(Debug, Default)]
pub struct DataFiles {
    pub lambda_pairs: Option<PathBuf>,
    pub lambda_k_pairs: Option<PathBuf>,
    pub a2f: Option<PathBuf>,
    pub imag_aniso: Vec<PathBuf>,
    pub pade_aniso: Vec<PathBuf>,
    pub aniso_gap0: Vec<PathBuf>,
    pub imag_iso: Vec<PathBuf>,
    pub qdos: Vec<PathBuf>,
    pub epw_output: Option<PathBuf>,
}
impl DataFiles {
    /// 只扫描当前方法目录，不按 MgB2 硬编码前缀；source.a2f 优先级低于原生谱。
    pub fn scan(dir: &Path) -> Result<Self> {
        let mut data = Self::default();
        let mut entries = fs::read_dir(dir)?
            .map(|e| e.map(|e| e.path()))
            .collect::<std::io::Result<Vec<_>>>()?;
        entries.sort();
        for path in entries {
            if !path.is_file() {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy();
            if name.ends_with(".lambda_pairs") {
                data.lambda_pairs = Some(path);
            } else if name.ends_with(".lambda_k_pairs") {
                data.lambda_k_pairs = Some(path);
            } else if name.ends_with(".a2f") {
                if name != "source.a2f" || data.a2f.is_none() {
                    data.a2f = Some(path);
                }
            } else if name.contains(".imag_aniso_gap0_") && io::temperature(&path).is_ok() {
                data.aniso_gap0.push(path);
            } else if name.contains(".imag_aniso_")
                && !name.contains(".imag_aniso_gap_FS_")
                && io::temperature(&path).is_ok()
            {
                data.imag_aniso.push(path);
            } else if name.contains(".pade_aniso_")
                && !name.contains(".pade_aniso_gap0_")
                && io::temperature(&path).is_ok()
            {
                data.pade_aniso.push(path);
            } else if name.contains(".imag_iso_") && io::temperature(&path).is_ok() {
                data.imag_iso.push(path);
            } else if name.contains(".qdos_") && io::temperature(&path).is_ok() {
                data.qdos.push(path);
            } else if name == "epw1.out" || (name == "epw.out" && data.epw_output.is_none()) {
                data.epw_output = Some(path);
            }
        }
        // source.a2f 链接已经明确选择了谱来源，只读取同前缀伴随分布，不搜索其他网格。
        if let Some(spectrum) = &data.a2f {
            if fs::symlink_metadata(spectrum)?.file_type().is_symlink() {
                let source = fs::canonicalize(spectrum)?;
                for (file, extension) in [
                    (&mut data.lambda_pairs, "lambda_pairs"),
                    (&mut data.lambda_k_pairs, "lambda_k_pairs"),
                ] {
                    let candidate = source.with_extension(extension);
                    if file.is_none() && candidate.is_file() {
                        *file = Some(candidate);
                    }
                }
            }
        }
        for files in [
            &mut data.imag_aniso,
            &mut data.pade_aniso,
            &mut data.aniso_gap0,
            &mut data.imag_iso,
            &mut data.qdos,
        ] {
            files.sort_by(|a, b| {
                io::temperature(a)
                    .unwrap_or(f64::INFINITY)
                    .total_cmp(&io::temperature(b).unwrap_or(f64::INFINITY))
            });
        }
        Ok(data)
    }
    fn has_data(&self) -> bool {
        self.a2f.is_some()
            || !self.imag_aniso.is_empty()
            || !self.aniso_gap0.is_empty()
            || !self.imag_iso.is_empty()
            || !self.qdos.is_empty()
    }
}
#[derive(Debug, Default)]
pub struct PlotReport {
    pub files: Vec<PathBuf>,
    pub warnings: Vec<String>,
}

pub struct PlotTask {
    pub work_dir: PathBuf,
    pub result_dir: PathBuf,
    pub data: DataFiles,
    pub formats: Vec<PlotFormat>,
    messages: Mutex<Vec<String>>,
}
impl PlotTask {
    /// result_dir 为当前方法的最终输出目录；默认仅写 SVG。
    pub fn from_work_dir(work_dir: &Path, result_dir: PathBuf) -> Result<Self> {
        Ok(Self {
            work_dir: work_dir.into(),
            result_dir,
            data: DataFiles::scan(work_dir)?,
            formats: vec![PlotFormat::Svg],
            messages: Mutex::new(vec![]),
        })
    }
    /// 保留第 8 步之后的完整相对层级；拒绝不属于该阶段的输入路径。
    pub fn mirror_to_results(work_dir: &Path, results: &Path) -> Result<Self> {
        let stage = work_dir
            .ancestors()
            .find(|p| {
                p.file_name()
                    .is_some_and(|n| n == "8.epw-superconductivity")
            })
            .context("plot input must be inside 8.epw-superconductivity")?;
        let relative = work_dir.strip_prefix(stage)?;
        Self::from_work_dir(
            work_dir,
            results.join("8.epw-superconductivity").join(relative),
        )
    }
    pub(super) fn warn(&self, message: impl Into<String>) {
        let message = message.into();
        eprintln!("Warning: superconductivity plot: {message}");
        self.messages
            .lock()
            .unwrap()
            .push(format!("Warning: {message}"));
    }
    pub(super) fn load<T>(
        &self,
        path: &Path,
        parser: impl FnOnce(&Path) -> Result<T>,
    ) -> Option<T> {
        match parser(path) {
            Ok(data) => {
                self.messages
                    .lock()
                    .unwrap()
                    .push(format!("read: {}", path.display()));
                Some(data)
            }
            Err(error) => {
                self.warn(format!("{}: {error:#}", path.display()));
                None
            }
        }
    }
    /// 同一细网格下对比同模式的不同参数目录；最多两组，当前组始终先绘。
    pub(super) fn groups(&self, mode: &str) -> Result<Vec<(PathBuf, DataFiles)>> {
        let mut dirs = vec![];
        let current = self
            .work_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        if current.starts_with(mode)
            || (mode == "iso-" && !self.data.imag_iso.is_empty())
            || (mode == "aniso-"
                && (!self.data.imag_aniso.is_empty() || !self.data.aniso_gap0.is_empty()))
            || (!current.starts_with("iso-") && !current.starts_with("aniso-"))
        {
            dirs.push(self.work_dir.clone());
        }
        if let Some(parent) = self.work_dir.parent() {
            let mut siblings = fs::read_dir(parent)?
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
                .map(|e| e.path())
                .filter(|p| {
                    p != &self.work_dir
                        && p.file_name()
                            .is_some_and(|n| n.to_string_lossy().starts_with(mode))
                })
                .collect::<Vec<_>>();
            siblings.sort();
            dirs.extend(siblings);
        }
        dirs.truncate(2);
        dirs.into_iter()
            .map(|dir| {
                let data = DataFiles::scan(&dir)?;
                Ok((dir, data))
            })
            .collect()
    }
    /// 五类图独立并行；日志在全部线程结束后一次写入，缺失或损坏数据不传播至科学流程。
    pub fn execute(&self) -> Result<PlotReport> {
        ensure!(!self.formats.is_empty(), "plot formats cannot be empty");
        fs::create_dir_all(&self.result_dir)?;
        type Draw = fn(&PlotTask) -> Result<Option<Vec<Panel>>>;
        let jobs: [(&str, Draw); 5] = [
            ("fig1_coupling_strength", coupling::plot_coupling_triple),
            (
                "fig2_gap_imaginary_real",
                gap_frequency::plot_gap_imaginary_real,
            ),
            ("fig3_aniso_gap_vs_T", gap_temperature::plot_aniso_gap_vs_t),
            ("fig4_iso_gap_vs_T", gap_temperature::plot_iso_gap_vs_t),
            ("fig5_superconducting_dos", dos::plot_superconducting_dos),
        ];
        // 独立线程池最多五路，防止宿主线程数放大 PDF 转换内存。
        let pool = rayon::ThreadPoolBuilder::new().num_threads(5).build()?;
        let files = pool.install(|| {
            jobs.par_iter()
                .flat_map(|(name, draw)| {
                    let result = (|| {
                        let Some(panels) = draw(self)? else {
                            return Ok(vec![]);
                        };
                        let base = self.result_dir.join(name);
                        let saved = qf_plot::superconductivity::save(
                            base.to_str().context("non-UTF8 output path")?,
                            &self.formats,
                            &panels,
                        )?;
                        for format in &self.formats {
                            if !saved
                                .iter()
                                .any(|p| p.ends_with(&format!(".{}", format.extension())))
                            {
                                self.warn(format!("{name}: {} output failed", format.extension()));
                            }
                        }
                        Ok::<_, anyhow::Error>(
                            saved.into_iter().map(PathBuf::from).collect::<Vec<_>>(),
                        )
                    })();
                    match result {
                        Ok(paths) => paths,
                        Err(e) => {
                            self.warn(format!("{name}: {e:#}"));
                            vec![]
                        }
                    }
                })
                .collect::<Vec<_>>()
        });
        let mut messages = self.messages.lock().unwrap().clone();
        messages.sort();
        messages.dedup();
        let warnings = messages
            .iter()
            .filter(|s| s.starts_with("Warning:"))
            .cloned()
            .collect();
        messages.extend(files.iter().map(|p| format!("saved: {}", p.display())));
        fs::write(self.result_dir.join("plot.log"), messages.join("\n") + "\n")?;
        Ok(PlotReport { files, warnings })
    }
}
/// 用方法和 μc 标注参数组，避免完整目录名遮挡相邻子图。
pub(super) fn parameter_title(dir: &Path) -> String {
    let name = dir.file_name().unwrap_or_default().to_string_lossy();
    let label = name.split("-Tc").next().unwrap_or(&name);
    label.replace("-mu", ", μc = ")
}
/// 扫描第 8 步，手动模式不要求清单完成，也不加载或验证科学程序、赝势。
pub fn plot_existing(
    work_root: &Path,
    results: &Path,
    formats: &[PlotFormat],
) -> Result<PlotReport> {
    let stage = work_root.join("8.epw-superconductivity");
    ensure!(
        stage.is_dir(),
        "missing plot data directory: {}",
        stage.display()
    );
    let mut dirs = vec![stage];
    let mut report = PlotReport::default();
    while let Some(dir) = dirs.pop() {
        let data = DataFiles::scan(&dir)?;
        if data.has_data() {
            let mut task = PlotTask::mirror_to_results(&dir, results)?;
            task.formats = formats.into();
            match task.execute() {
                Ok(r) => {
                    report.files.extend(r.files);
                    report.warnings.extend(r.warnings);
                }
                Err(e) => {
                    let message = format!("{}: {e:#}", dir.display());
                    eprintln!("Warning: superconductivity plot: {message}");
                    report.warnings.push(message);
                }
            }
        }
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir()
                && entry.path().strip_prefix(work_root)?.components().count() < 5
            {
                dirs.push(entry.path());
            }
        }
    }
    Ok(report)
}
/// 自动绘图由第 8 步完成边界调用；所有出图错误仅作为警告返回。
pub fn auto_plot(cfg: &Config) {
    if SKIP.get() {
        return;
    }
    let formats = match cfg.plot.formats() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Warning: superconductivity plot: {e:#}");
            return;
        }
    };
    match plot_existing(
        &cfg.work_dir,
        &cfg.work_dir
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("results"),
        &formats,
    ) {
        Ok(r) => println!("superconductivity_plots: {}", r.files.len()),
        Err(e) => eprintln!("Warning: superconductivity plot: {e:#}"),
    }
}
/// --plot-only 只读取根级路径及绘图格式，避免引发配置备份、程序检查或任务提交。
pub fn plot_config(path: &Path) -> Result<PlotReport> {
    let text = fs::read_to_string(path)?;
    let doc: toml::Value = toml::from_str(&text)?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let work = doc
        .get("work_dir")
        .and_then(|v| v.as_str())
        .unwrap_or("run");
    let work = parent.join(work);
    let section: qf_config::PlotSection = doc
        .get("plot")
        .map(|p| p.clone().try_into())
        .transpose()?
        .unwrap_or_default();
    let formats = section.validate_formats()?;
    plot_existing(
        &work,
        &work
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("results"),
        &formats,
    )
}
