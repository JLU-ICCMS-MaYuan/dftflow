//! 单模式、单温度原生 EPW 尝试。调用者负责 F3 质量门和 F1 资源登记。
use super::{
    gap::{self, GapData, State},
    parse::{self, SolveEvidence},
};
use crate::foundation::{
    config::{Config, Mode},
    process,
    store::{self, Identity},
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SolveKind {
    Linear,
    Nonlinear,
    /// 指定温度的单初值非线性能隙，执行与数值有效性分别记录。
    Gap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub mode: Mode,
    pub kind: SolveKind,
    pub temperature_centikelvin: i64,
    pub seed_mev: Option<f64>,
    pub mu_star: f64,
    pub cutoff_ev: f64,
    pub max_iterations: u32,
    pub convergence_relative: f64,
    pub mixing_beta: f64,
    pub mixing_history: u32,
    pub initial_shape: f64,
}

impl Request {
    pub fn from_config(
        cfg: &Config,
        mode: Mode,
        kind: SolveKind,
        temperature: i64,
        seed_mev: Option<f64>,
    ) -> Result<Self> {
        let sc = &cfg.superconductivity;
        let r = Self {
            mode,
            kind,
            temperature_centikelvin: temperature,
            seed_mev,
            mu_star: sc.mu_star,
            cutoff_ev: sc.frequency_cutoff_ev,
            max_iterations: sc.max_iterations,
            convergence_relative: sc.convergence_relative,
            mixing_beta: sc.mixing_beta,
            mixing_history: sc.mixing_history,
            initial_shape: sc.initial_shape,
        };
        r.validate()?;
        ensure!(
            mode != Mode::Both && (sc.mode == Mode::Both || sc.mode == mode),
            "configuration: 尝试模式与固定配置不符"
        );
        if sc.task == crate::foundation::config::SuperconductivityTask::Gap {
            ensure!(
                kind == SolveKind::Gap
                    && seed_mev == Some(sc.initial_gap())
                    && sc
                        .gap_temperatures_k
                        .iter()
                        .any(|t| (*t * 100.0).round() as i64 == temperature),
                "configuration: 指定温度请求与固定计划不符"
            );
            return Ok(r);
        }
        ensure!(
            kind != SolveKind::Gap,
            "configuration: Tc 任务不能使用单点请求"
        );
        let tc = match mode {
            Mode::Iso => sc.iso.as_ref(),
            _ => sc.aniso.as_ref(),
        }
        .context("configuration: 缺少模式搜索参数")?;
        ensure!(
            temperature >= crate::foundation::config::temperature(tc.min_k, "min_k")?
                && temperature <= crate::foundation::config::temperature(tc.max_k, "max_k")?,
            "configuration: 尝试温度超出固定范围"
        );
        if let Some(seed) = seed_mev {
            ensure!(
                sc.initial_gaps_mev.contains(&seed),
                "configuration: 非预固定初值"
            );
        }
        Ok(r)
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.mode != Mode::Both && (1..=99999).contains(&self.temperature_centikelvin),
            "configuration: EPW 5.9 尝试温度须在 0.01..999.99 K，且单模式"
        );
        ensure!(
            self.kind != SolveKind::Linear || (self.mode == Mode::Iso && self.seed_mev.is_none()),
            "configuration: 仅 iso 支持线性求解，线性无非线性初值"
        );
        ensure!(
            self.kind == SolveKind::Linear
                || self.seed_mev.is_some_and(|s| s.is_finite() && s > 0.0),
            "configuration: 非线性必须固定非零初值"
        );
        ensure!(
            self.mu_star.is_finite()
                && self.mu_star >= 0.0
                && self.cutoff_ev.is_finite()
                && self.cutoff_ev > 0.0,
            "configuration: μ*/截断无效"
        );
        ensure!(
            self.max_iterations > 0
                && self.convergence_relative.is_finite()
                && self.convergence_relative > 0.0
                && self.mixing_beta.is_finite()
                && self.mixing_beta.abs() > 0.0
                && self.mixing_beta.abs() <= 1.0
                && (1..=8).contains(&self.mixing_history),
            "configuration: 非线性迭代设置无效"
        );
        ensure!(
            self.initial_shape.is_finite() && self.initial_shape >= 0.0,
            "configuration: 首版仅支持固定非负 a_gap0"
        );
        self.nfreq()?;
        Ok(())
    }

    pub fn nfreq(&self) -> Result<usize> {
        let x = 0.5
            * (self.cutoff_ev
                / (std::f64::consts::PI * gap::KB_EV_K * self.temperature_centikelvin as f64
                    / 100.0)
                - 1.0);
        ensure!(
            x.is_finite() && x >= 0.0 && x < usize::MAX as f64,
            "configuration: 截断低于首个 Matsubara 频率或网格溢出"
        );
        Ok(x.trunc() as usize + 1)
    }
}

/// 原生源身份随尝试保存；aniso 状态表必须由同一 egnv 生成。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeInputs {
    pub interpolation_digest: String,
    pub prefix: String,
    pub k_grid: [u32; 3],
    pub q_grid: [u32; 3],
    pub spectral_points: usize,
    pub eps_acoustic_cm1: f64,
    pub a2f: Identity,
    pub crystal: Identity,
    pub selecq: Option<Identity>,
    pub matrix_outdir: PathBuf,
    pub matrix_files: Vec<Identity>,
    pub states: Vec<State>,
}

impl NativeInputs {
    pub fn verify(&self, mode: Mode) -> Result<()> {
        ensure!(
            self.prefix
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "_-".contains(c))
                && !self.prefix.is_empty(),
            "data_invalid: 原生 prefix 非法"
        );
        ensure!(
            self.spectral_points > 1 && !self.interpolation_digest.is_empty(),
            "data_invalid: 缺少谱/F3 身份"
        );
        ensure!(
            self.eps_acoustic_cm1.is_finite() && self.eps_acoustic_cm1 >= 0.0,
            "data_invalid: 低频规则无效"
        );
        self.a2f.verify()?;
        self.crystal.verify()?;
        if mode == Mode::Aniso {
            self.selecq
                .as_ref()
                .context("data_invalid: 缺少原生 selecq.fmt")?
                .verify()?;
            ensure!(
                !self.states.is_empty()
                    && self.matrix_files.len() >= 4
                    && self.matrix_outdir.is_absolute(),
                "data_invalid: 缺少原生矩阵元/状态映射"
            );
            for x in &self.matrix_files {
                x.verify()?;
            }
        }
        ensure!(
            self.k_grid
                .iter()
                .zip(self.q_grid)
                .all(|(k, q)| q > 0 && *k > 0 && *k % q == 0),
            "data_invalid: 原生细网格不可公度"
        );
        Ok(())
    }
}

fn quoted(path: &Path) -> Result<String> {
    let s = path.to_str().context("configuration: 非 UTF-8 原生路径")?;
    ensure!(
        !s.contains(['\n', '\r', '\0']),
        "configuration: 原生路径含控制字符"
    );
    Ok(format!("'{}'", s.replace('\'', "''")))
}

pub fn render(request: &Request, input: &NativeInputs) -> Result<String> {
    request.validate()?;
    let iso = request.mode == Mode::Iso;
    let mut s = format!("DFTFlow EPW 5.9 single-temperature solve\n&inputepw\n prefix = '{}'\n outdir = {}\n epwread = .true.\n epbread = .false.\n epwwrite = .false.\n wannierize = .false.\n ep_coupling = .false.\n elph = .false.\n eliashberg = .true.\n liso = {}\n laniso = {}\n tc_linear = {}\n tc_linear_solver = 'lapack'\n limag = .true.\n lreal = .false.\n lpade = .false.\n lacon = .false.\n imag_read = .false.\n fbw = .false.\n positive_matsu = .true.\n gridsamp = 0\n nswi = 0\n nstemp = 1\n temps(1) = {:.2}\n muc = {:.16e}\n wscut = {:.16e}\n nsiter = {}\n conv_thr_iaxis = {:.16e}\n broyden_beta = {:.16e}\n broyden_ndim = {}\n a_gap0 = {:.16e}\n nqstep = {}\n iverbosity = 2\n",
        input.prefix, quoted(&input.matrix_outdir)?, if iso { ".true." } else { ".false." },
        if iso { ".false." } else { ".true." }, if request.kind == SolveKind::Linear { ".true." } else { ".false." },
        request.temperature_centikelvin as f64 / 100.0, request.mu_star, request.cutoff_ev,
        request.max_iterations, request.convergence_relative, request.mixing_beta,
        request.mixing_history, request.initial_shape, input.spectral_points);
    // 目标版本 fila2f 只有 80 字符；副本属于本次尝试，避免绝对路径被静默截断。
    s.push_str(" fila2f = 'source.a2f'\n");
    s.push_str(&format!(
        " eps_acoustic = {:.16e}\n",
        input.eps_acoustic_cm1
    ));
    s.push_str(" mp_mesh_k = .true.\n");
    if let Some(seed) = request.seed_mev {
        s.push_str(&format!(" gap_edge = {:.16e}\n", seed / 1000.0));
    }
    for (i, (k, q)) in input.k_grid.iter().zip(input.q_grid).enumerate() {
        s.push_str(&format!(" nkf{} = {}\n nqf{} = {}\n", i + 1, k, i + 1, q));
    }
    s.push_str(" /\n");
    Ok(s)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttemptResult {
    pub schema_version: u32,
    pub request: Request,
    pub source: NativeInputs,
    pub executable: Identity,
    pub command: Vec<String>,
    pub status: String,
    pub reason: Option<String>,
    pub exit_status: Option<i32>,
    pub duration_secs: Option<f64>,
    pub artifacts: Vec<Identity>,
    pub evidence: Option<SolveEvidence>,
    pub gap: Option<GapData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap_status: Option<GapStatus>,
}

/// 必须使用新目录。先落盘 running，再启动；已有目录一律不重提，失败现场也保留。
pub fn run_attempt(
    request: &Request,
    inputs: &NativeInputs,
    program: &crate::foundation::config::Program,
    launcher: &[String],
    dir: &Path,
) -> Result<AttemptResult> {
    request.validate()?;
    inputs.verify(request.mode)?;
    ensure!(
        !program
            .args
            .iter()
            .any(|a| matches!(a.as_str(), "-in" | "-inp" | "-input")),
        "configuration: programs.epw.args 不得覆盖单温度输入"
    );
    if crate::foundation::native_jobs::continuing(dir, "epw")? {
        let record: AttemptResult = store::read(&dir.join("attempt.json"))?;
        ensure!(
            serde_json::to_vec(&record.request)? == serde_json::to_vec(request)?
                && serde_json::to_vec(&record.source)? == serde_json::to_vec(inputs)?,
            "input_conflict: 原生尝试固定来源变化"
        );
        return collect(record, dir);
    }
    let executable = Identity::capture(&program.path)?;
    ensure!(dir.is_absolute(), "configuration: 尝试目录须为绝对路径");
    let text = render(request, inputs)?;
    fs::create_dir(dir).with_context(|| {
        format!(
            "scheduler_unknown: 尝试目录已存在或不可创建 {}，拒绝自动重提",
            dir.display()
        )
    })?;
    let input_path = dir.join("epw.in");
    store::atomic_bytes(&input_path, text.as_bytes())?;
    crate::foundation::workspace::native_script(
        program,
        &input_path,
        dir,
        launcher,
        "epw",
        false,
        false,
    )?;
    fs::copy(&inputs.crystal.path, dir.join("crystal.fmt"))?;
    fs::copy(&inputs.a2f.path, dir.join("source.a2f"))?;
    ensure!(
        store::hash_file(&dir.join("source.a2f"))? == inputs.a2f.sha256,
        "input_conflict: 原生谱副本与来源不符"
    );
    if request.mode == Mode::Aniso {
        fs::copy(
            &inputs
                .selecq
                .as_ref()
                .context("data_invalid: 缺少 selecq")?
                .path,
            dir.join("selecq.fmt"),
        )?;
    }
    let mut command = launcher.to_vec();
    command.push(program.path.to_string_lossy().into_owned());
    command.extend(program.args.clone());
    command.extend(["-inp".into(), input_path.to_string_lossy().into_owned()]);
    let mut record = AttemptResult {
        schema_version: 1,
        request: request.clone(),
        source: inputs.clone(),
        executable,
        command: command.clone(),
        status: "running".into(),
        reason: None,
        exit_status: None,
        duration_secs: None,
        artifacts: vec![
            Identity::capture(&input_path)?,
            Identity::capture(&dir.join("crystal.fmt"))?,
            Identity::capture(&dir.join("source.a2f"))?,
        ],
        evidence: None,
        gap: None,
        gap_status: None,
    };
    if request.mode == Mode::Aniso {
        record
            .artifacts
            .push(Identity::capture(&dir.join("selecq.fmt"))?);
    }
    store::write(&dir.join("attempt.json"), &record)?;
    collect(record, dir)
}

fn collect(mut record: AttemptResult, dir: &Path) -> Result<AttemptResult> {
    let request = record.request.clone();
    let inputs = record.source.clone();
    let command = record.command.clone();
    let outcome = (|| -> Result<()> {
        let report = process::run_report(&command, dir)?;
        record.exit_status = report.exit_status;
        record.duration_secs = Some(report.duration_secs);
        for (path, name) in [
            (&report.stdout_path, "epw.out"),
            (&report.stderr_path, "epw.err"),
        ] {
            fs::copy(path, dir.join(name))?;
            record.artifacts.push(Identity::capture(&dir.join(name))?);
        }
        ensure!(
            report.exit_status == Some(0),
            "execution_failed: EPW 退出 {:?}",
            report.exit_status
        );
        let log = fs::read_to_string(dir.join("epw.out"))?;
        if request.kind == SolveKind::Gap {
            crate::interpolation::native::checked_log(&log, 0)?;
            let (status, evidence, data) = inspect_gap(&record, dir)?;
            let path = gap_path(&record, dir);
            if path.is_file() {
                record.artifacts.push(Identity::capture(&path)?);
            }
            record.gap_status = Some(status);
            record.evidence = evidence;
            record.gap = data;
            inputs.verify(request.mode)?;
            record.executable.verify()?;
            for a in &record.artifacts {
                a.verify()?;
            }
            return Ok(());
        }
        let evidence = parse::solve_log(&log, &request)?;
        if request.kind == SolveKind::Nonlinear {
            let filename = format!(
                "{}.imag_{}_{:06.2}",
                inputs.prefix,
                if request.mode == Mode::Iso {
                    "iso"
                } else {
                    "aniso"
                },
                request.temperature_centikelvin as f64 / 100.0
            );
            let path = dir.join(filename);
            let data = gap::parse_native(
                &fs::read_to_string(&path).context("data_invalid: 缺少原生完整 Δ/Z")?,
                request.temperature_centikelvin,
                evidence.nfreq,
                if request.mode == Mode::Iso {
                    vec![]
                } else {
                    inputs.states.clone()
                },
            )?;
            record.artifacts.push(Identity::capture(&path)?);
            record.gap = Some(data);
        }
        inputs.verify(request.mode)?;
        record.executable.verify()?;
        for a in &record.artifacts {
            a.verify()?;
        }
        record.evidence = Some(evidence);
        Ok(())
    })();
    match outcome {
        Ok(()) => {
            record.status = if request.kind == SolveKind::Gap && record.gap.is_none() {
                "completed"
            } else {
                "valid"
            }
            .into()
        }
        Err(e) if crate::foundation::native_jobs::paused(&e) => return Err(e),
        Err(e) => {
            record.status = "failed".into();
            record.reason = Some(format!("{e:#}"));
        }
    }
    store::write(&dir.join("attempt.json"), &record)?;
    Ok(record)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GapStatus {
    pub execution_complete: bool,
    pub converged: Option<bool>,
    pub data_status: String,
    pub reason: Option<String>,
}

pub fn gap_path(record: &AttemptResult, dir: &Path) -> PathBuf {
    dir.join(format!(
        "{}.imag_{}_{:06.2}",
        record.source.prefix,
        if record.request.mode == Mode::Iso {
            "iso"
        } else {
            "aniso"
        },
        record.request.temperature_centikelvin as f64 / 100.0
    ))
}

/// 只读重建正常结束与数值状态；不把退出 0 当成收敛，不伪造缺失能隙。
pub fn inspect_gap(
    record: &AttemptResult,
    dir: &Path,
) -> Result<(GapStatus, Option<SolveEvidence>, Option<GapData>)> {
    ensure!(
        record.request.kind == SolveKind::Gap,
        "data_invalid: 非指定温度请求"
    );
    ensure!(
        record.exit_status == Some(0),
        "execution_failed: EPW 未零退出"
    );
    let log = fs::read_to_string(dir.join("epw.out"))?;
    crate::interpolation::native::checked_log(&log, 0)?;
    let reached = log.contains("Convergence was reached in nsiter =");
    let not_reached = log.contains("Convergence was not reached");
    let mut status = GapStatus {
        execution_complete: true,
        converged: match (reached, not_reached) {
            (true, false) => Some(true),
            (false, true) => Some(false),
            _ => None,
        },
        data_status: "invalid".into(),
        reason: None,
    };
    if status.converged != Some(true) {
        status.data_status = "not_converged".into();
        status.reason = Some("未获得明确的原生收敛证据".into());
        return Ok((status, None, None));
    }
    let outcome = (|| -> Result<(SolveEvidence, GapData)> {
        let evidence = parse::solve_log(&log, &record.request)?;
        let path = gap_path(record, dir);
        if !path.is_file() {
            status.data_status = "missing".into();
            anyhow::bail!("data_invalid: 缺少原生完整 Δ/Z");
        }
        let data = gap::parse_native(
            &fs::read_to_string(path)?,
            record.request.temperature_centikelvin,
            evidence.nfreq,
            if record.request.mode == Mode::Iso {
                vec![]
            } else {
                record.source.states.clone()
            },
        )?;
        Ok((evidence, data))
    })();
    match outcome {
        Ok((e, g)) => {
            status.data_status = "valid".into();
            Ok((status, Some(e), Some(g)))
        }
        Err(e) => {
            status.reason = Some(format!("{e:#}"));
            Ok((status, None, None))
        }
    }
}
