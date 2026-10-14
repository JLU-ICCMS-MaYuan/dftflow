//! 外部计算二进制调用、Slurm 两段提交脚本生成、并行读取。
//! 仅通过 `std::process::Command` spawn 外部 QE/VASP，不内嵌 Fortran。

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use qf_state::{TaskState, WorkflowStatus};

pub const PREFIX: &str = "[qeflow]";

/// 由 `[qe].executable_path`（"启动命令 + QE bin 目录"，如 "mpirun -np 8 /qe/bin"）
/// 解析出某个 QE 程序的完整调用串，例如 "mpirun -np 8 /qe/bin/pw.x"。
/// 复刻现状 `resolve_qe_command`。
pub fn resolve_qe_command(exec_path: &str, bin: &str) -> String {
    let tokens = shell_words::split(exec_path).unwrap_or_else(|_| vec![exec_path.into()]);
    if tokens.is_empty() {
        return bin.into();
    }
    let variable =
        regex::Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}|\$([A-Za-z_][A-Za-z0-9_]*)").unwrap();
    let mut tokens: Vec<String> = tokens
        .into_iter()
        .map(|s| {
            let s = variable
                .replace_all(&s, |caps: &regex::Captures| {
                    std::env::var(caps.get(1).or_else(|| caps.get(2)).unwrap().as_str())
                        .unwrap_or_else(|_| caps[0].to_string())
                })
                .into_owned();
            if let Some(tail) = s.strip_prefix("~/") {
                format!("{}/{tail}", std::env::var("HOME").unwrap_or_default())
            } else {
                s
            }
        })
        .collect();
    let target = tokens.last_mut().unwrap();
    if Path::new(target).file_name().and_then(|s| s.to_str()) != Some(bin) {
        *target = format!("{}/{bin}", target.trim_end_matches('/'));
    }
    tokens
        .iter()
        .map(|t| shell_words::quote(t).into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

/// spawn 外部命令，将 stdin/stdout/stderr 重定向到文件（复刻现状 shell 重定向语义）。
pub fn spawn_redirect(
    cmd: &str,
    stdin_file: &str,
    stdout_file: &str,
) -> Result<std::process::ExitStatus> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        anyhow::bail!("empty command");
    }
    let mut command = Command::new(parts[0]);
    for p in &parts[1..] {
        command.arg(p);
    }
    let status = command
        .stdin(std::fs::File::open(stdin_file).with_context(|| format!("open {stdin_file}"))?)
        .stdout(
            std::fs::File::create(stdout_file).with_context(|| format!("create {stdout_file}"))?,
        )
        .stderr(
            std::fs::File::create(format!("{stdout_file}.err"))
                .with_context(|| format!("create {stdout_file}.err"))?,
        )
        .status()
        .with_context(|| format!("spawn {cmd}"))?;
    Ok(status)
}

/// 在 `cwd` 用 bash 直接运行一条 shell 命令（对应现状 `-r` 本地执行）。
pub fn run_shell(command: &str, cwd: &Path) -> Result<()> {
    let status = Command::new("bash")
        .args(["-c", command])
        .current_dir(cwd)
        .status()
        .with_context(|| format!("run shell in {}", cwd.display()))?;
    if !status.success() {
        anyhow::bail!("command failed ({status}) in {}: {command}", cwd.display());
    }
    Ok(())
}

#[cfg(test)]
fn parse_job_id(text: &str) -> Option<String> {
    let marker = "Submitted batch job";
    let idx = text.find(marker)?;
    let rest = text[idx + marker.len()..].trim_start();
    let id = rest.split_whitespace().next()?;
    if id.is_empty() {
        None
    } else {
        Some(id.to_string())
    }
}

/// 通过原生 `sbatch --parsable` 提交 Slurm 作业，解析返回的作业号。
/// 对齐 `qeflow/phonon/console.submit_script`。
pub fn submit_via_sbatch(work_dir: &Path, script_name: &str) -> Result<Option<String>> {
    submit_slurm(work_dir, script_name, &[]).map(Some)
}

/// 根据 run_calc / submit_job 决定本地运行还是提交 Slurm。
/// 对齐 `qeflow/phonon/runner.submit_or_run`。
pub fn submit_or_run(
    run_calc: bool,
    submit_job: bool,
    cwd: &Path,
    script_name: &str,
) -> Result<bool> {
    if run_calc {
        run_shell(&format!("bash {script_name}"), cwd)?;
        Ok(true)
    } else if submit_job {
        submit_via_sbatch(cwd, script_name)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// 单引号包裹并转义（复刻 Python `shlex.quote` 的最简子集）。
pub fn shlex_quote(s: &str) -> String {
    let escaped = s.replace('\'', "'\\''");
    format!("'{escaped}'")
}

pub fn echo_qeflow(message: &str) -> String {
    format!("echo {}", shlex_quote(&format!("{PREFIX} {message}")))
}

/// 生成形如 `echo '[qeflow] 1/11 label: command'` 的单步回显行。
pub fn echo_command_step(step_no: usize, total: usize, label: &str, command: &str) -> String {
    echo_qeflow(&format!("{step_no}/{total} {label}: {command}"))
}

pub struct LabeledContext<'a> {
    pub workflow_name: &'a str,
    pub prefix: &'a str,
    pub work_dir: &'a str,
    pub stage: Option<&'a str>,
}

/// 生成带上下文的逐步命令序列（对齐 `script_plan.build_labeled_commands` 的 stage 分支）。
pub fn build_labeled_commands(
    steps: &[(String, String)],
    start_index: usize,
    total: usize,
    ctx: Option<&LabeledContext>,
) -> Vec<String> {
    let total = if total == 0 { steps.len() } else { total };
    let mut cmds = Vec::new();
    for (i, (label, command)) in steps.iter().enumerate() {
        let step_no = start_index + i;
        let step_label = format!("{step_no}/{total} {label}");
        match ctx {
            Some(c) => {
                cmds.push(echo_qeflow(&"-".repeat(60)));
                cmds.push(echo_qeflow(&format!("workflow: {}", c.workflow_name)));
                cmds.push(echo_qeflow(&format!("prefix: {}", c.prefix)));
                if let Some(stage) = c.stage {
                    cmds.push(echo_qeflow(&format!("stage: {stage}")));
                }
                cmds.push(echo_qeflow(&format!("step: {step_label}")));
                cmds.push(echo_qeflow(&format!("work_dir: {}", c.work_dir)));
                cmds.push(echo_qeflow(&format!("command: {command}")));
                cmds.push(format!("if ! cd \"{}\"; then", c.work_dir));
                cmds.push(echo_qeflow(&format!("failed: {step_label}")));
                cmds.push("  exit 1".to_string());
                cmds.push("fi".to_string());
                // 用子 shell 包裹整个命令：bash 的 `!` 只取反一个 pipeline，
                // 复合命令（含 `&&`/`;`）不加 `( )` 时成功分支会被短路（对齐
                // Python `subprocess.run(check=True)` 的整体成功/失败语义）。
                cmds.push(format!("if ! ( {command} ); then"));
                cmds.push(echo_qeflow(&format!("failed: {step_label}")));
                cmds.push("  exit 1".to_string());
                cmds.push("fi".to_string());
                cmds.push(echo_qeflow(&format!("done: {step_label}")));
            }
            None => {
                cmds.push(echo_command_step(step_no, total, label, command));
                cmds.push(command.clone());
            }
        }
    }
    cmds
}

/// 脚本生成上下文：工作目录、前缀、Slurm 头部。
pub struct ScriptContext<'a> {
    pub workspace: &'a Path,
    pub prefix: &'a str,
    pub slurm_header: &'a str,
}

pub fn write_job_script(
    workspace: &Path,
    script_name: &str,
    slurm_header: &str,
    commands: &[String],
) -> Result<PathBuf> {
    let mut lines: Vec<String> = vec!["#!/bin/bash".to_string()];
    let header = slurm_header.trim();
    if !header.is_empty() {
        for l in header.lines() {
            lines.push(l.to_string());
        }
    }
    lines.extend(commands.iter().cloned());
    let path = workspace.join(script_name);
    let text = format!("{}\n", lines.join("\n"));
    std::fs::write(&path, text).with_context(|| format!("write {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms)?;
    }
    Ok(path)
}

/// 生成带 step 标签的聚合脚本（对齐 `script_plan.write_labeled_script`）。
pub fn write_labeled_script(
    ctx: &ScriptContext,
    script_name: &str,
    steps: &[(String, String)],
    start_index: usize,
    total: usize,
    stage: Option<&str>,
    completion: Option<&str>,
) -> Result<PathBuf> {
    let workflow_name = ctx
        .workspace
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let labeled = LabeledContext {
        workflow_name: &workflow_name,
        prefix: ctx.prefix,
        work_dir: ctx.workspace.to_str().unwrap_or(""),
        stage,
    };
    let mut cmds = vec!["set -e".to_string()];
    cmds.extend(build_labeled_commands(
        steps,
        start_index,
        total,
        Some(&labeled),
    ));
    if let Some(m) = completion {
        cmds.push(echo_qeflow(&format!("完成: {m}")));
    }
    write_job_script(ctx.workspace, script_name, ctx.slurm_header, &cmds)
}

/// 生成单个 q 点 split 运行脚本 `run_split.sh`（对齐 `input_deck/scripts.write_split_run_script`）。
pub fn write_split_run_script(
    q_dir: &Path,
    slurm_header: &str,
    pw_cmd: &str,
    ph_cmd: &str,
) -> Result<PathBuf> {
    let commands = vec![
        format!("{pw_cmd} < scf.in > scf.out"),
        format!("{ph_cmd} < split_ph.in > split_ph.out"),
    ];
    write_job_script(q_dir, "run_split.sh", slurm_header, &commands)
}

/// 生成 pre + split 提交阶段脚本 `submit_pre_and_split_stage.sh`。
/// 对齐 `split_workflow._write_pre_and_split_stage_script`。
pub fn write_pre_and_split_stage_script(
    ctx: &ScriptContext,
    config_file: &str,
    mode: &str,
    status: &WorkflowStatus,
) -> Result<PathBuf> {
    let mut steps: Vec<(String, String)> = Vec::new();
    if status.scf != TaskState::Finished {
        steps.push(("运行 SCF".to_string(), "bash run_scf.sh".to_string()));
    }
    if status.ph.dyn0 != TaskState::Finished {
        steps.push((
            "运行 dyn0".to_string(),
            format!("qeflow qe-ph-nosplit -c {config_file} --dyn0 -r"),
        ));
    }
    steps.push((
        "准备并提交 split q 点任务".to_string(),
        format!("qeflow qe-split-submit -c {config_file} --mode {mode}"),
    ));
    write_labeled_script(
        ctx,
        "submit_pre_and_split_stage.sh",
        &steps,
        1,
        11,
        Some("pre_and_split"),
        Some("1/11-3/11 pre + split 提交阶段"),
    )
}

/// 生成 merge + 后处理阶段脚本 `submit_merge_and_process_phonon.sh`。
/// 对齐 `split_workflow._write_merge_and_process_stage_script`。
pub fn write_merge_and_process_stage_script(
    ctx: &ScriptContext,
    config_file: &str,
) -> Result<PathBuf> {
    let steps: Vec<(String, String)> = vec![
        (
            "merge".to_string(),
            format!("qeflow qe-merge -c {config_file}"),
        ),
        ("q2r".to_string(), "bash run_q2r.sh".to_string()),
        (
            "matdyn_phband".to_string(),
            "bash run_matdyn_phband.sh".to_string(),
        ),
        (
            "collect phband".to_string(),
            format!("qeflow qe-process-phonon -c {config_file} --collect"),
        ),
        (
            "matdyn_phdos".to_string(),
            "bash run_matdyn_phdos.sh".to_string(),
        ),
        (
            "collect phdos".to_string(),
            format!("qeflow qe-process-phonon -c {config_file} --collect"),
        ),
        (
            "plot".to_string(),
            format!("qeflow qe-process-phonon -c {config_file} --plot"),
        ),
        ("done".to_string(), "true".to_string()),
    ];
    write_labeled_script(
        ctx,
        "submit_merge_and_process_phonon.sh",
        &steps,
        4,
        11,
        Some("merge_and_process"),
        Some("4/11-11/11 merge + phonon 后处理阶段"),
    )
}

/// 一键生成两段提交脚本，返回两个脚本路径 `(pre, post)`。
/// 对齐 `split_workflow` 的 pre/post 两段提交约定（文件系统后端，无 daemon）。
pub fn write_slurm_two_stage(
    ctx: &ScriptContext,
    config_file: &str,
    mode: &str,
    status: &WorkflowStatus,
) -> Result<(PathBuf, PathBuf)> {
    let pre = write_pre_and_split_stage_script(ctx, config_file, mode, status)?;
    let post = write_merge_and_process_stage_script(ctx, config_file)?;
    Ok((pre, post))
}

/// 生成 `submit_split_stage.sh`，仅包含未 finished 的 q 点 `run_split.sh` 调用。
/// 对齐 `script_plan.write_split_stage_script`。
pub fn write_split_stage_script(
    ctx: &ScriptContext,
    tasks: &HashMap<String, TaskState>,
) -> Result<PathBuf> {
    let mut cmds: Vec<String> = vec!["set -e".to_string()];
    let mut names: Vec<&String> = tasks.keys().collect();
    names.sort();
    for q_name in names {
        if tasks.get(q_name) != Some(&TaskState::Finished) {
            cmds.push(format!("(cd {q_name} && bash run_split.sh)"));
        }
    }
    write_job_script(
        ctx.workspace,
        "submit_split_stage.sh",
        ctx.slurm_header,
        &cmds,
    )
}

/// 逐 q 点提交 `run_split.sh`（跳过 finished），返回成功提交数量。
/// 对齐 `script_plan.submit_split_stage_tasks`。
pub fn submit_split_stage_tasks(
    workspace: &Path,
    tasks: &HashMap<String, TaskState>,
) -> Result<usize> {
    let mut submitted = 0usize;
    let mut names: Vec<&String> = tasks.keys().collect();
    names.sort();
    for q_name in names {
        if tasks.get(q_name) == Some(&TaskState::Finished) {
            continue;
        }
        submit_via_sbatch(&workspace.join(q_name), "run_split.sh")?;
        submitted += 1;
    }
    Ok(submitted)
}

/// 并行读取多个路径的产物（对齐 `runner.parallel_read`，使用 rayon ThreadPool）。
pub fn parallel_read<P, F, T>(paths: &[P], f: F) -> Vec<T>
where
    P: AsRef<Path> + Sync,
    F: Fn(&P) -> T + Sync + Send,
    T: Send,
{
    use rayon::prelude::*;
    paths.par_iter().map(f).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_command_with_launcher() {
        assert_eq!(
            resolve_qe_command("mpirun -np 8 /qe/bin", "pw.x"),
            "mpirun -np 8 /qe/bin/pw.x"
        );
    }

    #[test]
    fn resolves_explicit_binary_and_quoted_directory() {
        assert_eq!(
            resolve_qe_command("mpirun -np 2 '/opt/QE suite/pw.x'", "pw.x"),
            "mpirun -np 2 '/opt/QE suite/pw.x'"
        );
        assert_eq!(
            resolve_qe_command("'/opt/QE suite'", "pw.x"),
            "'/opt/QE suite/pw.x'"
        );
    }

    #[test]
    fn resolves_command_empty_prefix() {
        assert_eq!(resolve_qe_command("", "pw.x"), "pw.x");
    }

    #[test]
    fn shlex_quotes_and_escapes() {
        assert_eq!(shlex_quote("a b"), "'a b'");
        assert_eq!(shlex_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn parses_sbatch_job_id() {
        assert_eq!(
            parse_job_id("Submitted batch job 12345"),
            Some("12345".to_string())
        );
        assert_eq!(parse_job_id("no job here"), None);
    }

    #[test]
    fn labeled_commands_with_context_emit_cd_blocks() {
        let ctx = LabeledContext {
            workflow_name: "wf",
            prefix: "P",
            work_dir: "/w",
            stage: Some("s"),
        };
        let cmds = build_labeled_commands(
            &[("label".to_string(), "echo hi".to_string())],
            1,
            2,
            Some(&ctx),
        );
        let joined = cmds.join("\n");
        assert!(joined.contains("step: 1/2 label"));
        assert!(joined.contains("cd \"/w\""));
        assert!(joined.contains("echo hi"));
    }

    #[test]
    fn writes_two_stage_scripts() {
        let dir = std::env::temp_dir().join(format!("qf_runner_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let ctx = ScriptContext {
            workspace: dir.as_path(),
            prefix: "P",
            slurm_header: "#SBATCH --job-name=t",
        };
        let status = WorkflowStatus::default();
        let (pre, post) = write_slurm_two_stage(&ctx, "input.toml", "index", &status).unwrap();
        let pre_text = std::fs::read_to_string(&pre).unwrap();
        assert!(pre_text.contains("#SBATCH --job-name=t"));
        assert!(pre_text.contains("bash run_scf.sh"));
        assert!(pre_text.contains("qeflow qe-split-submit -c input.toml --mode index"));
        let post_text = std::fs::read_to_string(&post).unwrap();
        assert!(post_text.contains("qeflow qe-merge -c input.toml"));
        assert!(post_text.contains("qeflow qe-process-phonon -c input.toml --plot"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// 原生 Slurm 提交；作业号和退出状态均验证，不无限重试。
pub fn submit_slurm(work_dir: &Path, script: &str, dependencies: &[String]) -> Result<String> {
    let mut command = Command::new("sbatch");
    command.arg("--parsable").arg("--kill-on-invalid-dep=yes");
    if !dependencies.is_empty() {
        anyhow::ensure!(dependencies.iter().all(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())), "无效依赖作业号");
        command.arg(format!("--dependency=afterok:{}", dependencies.join(":")));
    }
    let output = command.arg(script).current_dir(work_dir).output()?;
    anyhow::ensure!(output.status.success(), "Slurm 提交 {script} 失败: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_parsable_job_id(stdout.trim())
}

pub fn parse_parsable_job_id(text: &str) -> Result<String> {
    let id = text.split(';').next().unwrap_or("");
    anyhow::ensure!(!id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) && text.lines().count() == 1, "无法解析 Slurm 作业号: {text:?}");
    Ok(id.to_string())
}

/// 查询失败不能当作作业已经结束，避免重复提交。
pub fn slurm_job_state(id: &str) -> Result<String> {
    anyhow::ensure!(id.chars().all(|c| c.is_ascii_digit()) && !id.is_empty(), "无效作业号");
    let out = Command::new("squeue").args(["-h", "-j", id, "-o", "%T|%r"]).output()?;
    anyhow::ensure!(out.status.success(), "squeue 查询失败: {}", String::from_utf8_lossy(&out.stderr));
    let state = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !state.is_empty() {
        if state.contains("DependencyNeverSatisfied") { return Ok("INVALID_DEPENDENCY".into()); }
        return Ok(state.split('|').next().unwrap().to_string());
    }
    let out = Command::new("sacct").args(["-n", "-X", "-j", id, "--format=State", "--parsable2"]).output()?;
    anyhow::ensure!(out.status.success(), "sacct 查询失败");
    let text=String::from_utf8_lossy(&out.stdout);
    let state=text.lines().next().unwrap_or("").trim().trim_end_matches('|').split_whitespace().next().unwrap_or("");
    anyhow::ensure!(!state.is_empty(), "作业 {id} 不在队列或 accounting 中，拒绝重复提交");
    Ok(state.to_string())
}

#[cfg(test)]
mod slurm_tests {
    #[test]
    fn parsable_requires_an_actual_id() {
        assert_eq!(super::parse_parsable_job_id("123;cluster").unwrap(), "123");
        for s in ["", "Submitted batch job 12", "12\n13", "warning\n12", "12x"] {
            assert!(super::parse_parsable_job_id(s).is_err(), "{s}");
        }
    }
}
