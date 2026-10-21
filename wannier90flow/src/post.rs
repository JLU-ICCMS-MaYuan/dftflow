//! wannier_post：postw90.x 流程（复刻 `wannier90flow/wannier_post.py`）。

use crate::config::{apply_config_overrides, wannier_post_template};
use crate::init::slurm_header;
use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;

/// `resolve_win_path`：返回 (win 路径, 前缀)。
pub fn resolve_win_path(input_path: &str) -> Result<(String, String)> {
    let mut win_path = input_path.to_string();
    if Path::new(&win_path).is_dir() {
        anyhow::bail!("输入应为 *.win 文件路径，而不是目录。");
    }
    if !win_path.ends_with(".win") {
        win_path = format!("{win_path}.win");
    }
    if !Path::new(&win_path).exists() {
        anyhow::bail!("未找到 {win_path}，请检查 -i 参数。");
    }
    let prefix = Path::new(&win_path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok((win_path, prefix))
}

/// `load_post_config`：文本级合并 [wannier_post] 中的 dos_project 行后解析 TOML。
///
/// 复刻 Python 逻辑：收集该 section 内所有 `dos_project` 行（删除原行），把值合并为
/// 单一 `dos_project = [...]`（数组字面量原样保留，单值加引号），插回 section 末尾。
pub fn load_post_config(path: &str) -> Result<Value> {
    if !Path::new(path).exists() {
        anyhow::bail!("找不到配置文件: {path}");
    }
    let text = std::fs::read_to_string(path)?;
    let raw_lines: Vec<&str> = text.lines().collect();

    let mut dos_projects: Vec<String> = Vec::new();
    let mut dos_projects_literal: Option<String> = None;
    let mut out_lines: Vec<String> = Vec::new();
    let mut current_section = String::new();
    let mut dos_section_end: Option<usize> = None;

    for line in raw_lines {
        let stripped = line.trim();
        if stripped.starts_with('[') && stripped.ends_with(']') {
            if current_section == "[wannier_post]" {
                dos_section_end = Some(out_lines.len());
            }
            current_section = stripped.to_string();
            out_lines.push(line.to_string());
            continue;
        }
        if current_section == "[wannier_post]" && stripped.starts_with("dos_project") {
            if let Some((_, value_part)) = stripped.split_once('=') {
                let value = value_part.split('#').next().unwrap_or("").trim();
                if !value.is_empty() {
                    if value.starts_with('[') || value.starts_with('{') {
                        dos_projects_literal = Some(value.to_string());
                    } else if (value.starts_with('\'') || value.starts_with('"'))
                        && value.ends_with(value.chars().next().unwrap())
                        && value.len() >= 2
                    {
                        dos_projects.push(value.to_string());
                    } else {
                        dos_projects.push(format!("\"{value}\""));
                    }
                }
            }
            continue;
        }
        out_lines.push(line.to_string());
    }
    if current_section == "[wannier_post]" {
        dos_section_end = Some(out_lines.len());
    }

    let insert_at = dos_section_end.unwrap_or(out_lines.len());
    if let Some(lit) = dos_projects_literal {
        out_lines.insert(insert_at, format!("dos_project = {lit}"));
    } else if !dos_projects.is_empty() {
        out_lines.insert(
            insert_at,
            format!("dos_project = [{}]", dos_projects.join(", ")),
        );
    }

    let merged_text = out_lines.join("\n") + "\n";
    let parsed: Value = toml::from_str::<toml::Value>(&merged_text)
        .map_err(|e| anyhow::anyhow!("解析合并后的配置失败: {e}"))
        .and_then(|v| serde_json::to_value(v).map_err(|e| anyhow::anyhow!("TOML 转 JSON: {e}")))?;
    let mut template = wannier_post_template();
    let mut keys = crate::config::SHARED_ALLOWED_KEYS.to_vec();
    keys.push("wannier_post");
    apply_config_overrides(&mut template, &parsed, &keys)
}

/// `format_win_value`。
pub fn format_win_value(key: &str, value: &Value) -> String {
    match value {
        Value::Bool(b) => {
            if *b {
                ".true.".into()
            } else {
                ".false.".into()
            }
        }
        Value::Number(_) => value.to_string(),
        Value::String(s) => s.clone(),
        other => {
            if let Some(arr) = other.as_array() {
                let items: Vec<String> = arr.iter().map(item_string).collect();
                if key == "dos_project" {
                    items.join(", ")
                } else if key == "dos_kmesh" {
                    items.join(" ")
                } else {
                    other.to_string()
                }
            } else if other.is_null() {
                "None".to_string()
            } else {
                other.to_string()
            }
        }
    }
}

fn item_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => {
            if *b {
                ".true.".into()
            } else {
                ".false.".into()
            }
        }
        other => other.to_string(),
    }
}

/// `update_win_content`：删除旧行后按序追加 `key = value`。
pub fn update_win_content(content: &str, updates: &[(String, Value)]) -> String {
    let keys: Vec<&str> = updates.iter().map(|(k, _)| k.as_str()).collect();
    let mut lines: Vec<String> = Vec::new();
    for line in content.lines() {
        let stripped = line.trim_start();
        let hit = keys
            .iter()
            .any(|k| stripped.starts_with(&format!("{k} ")) || stripped.starts_with(&format!("{k}=")));
        if !hit {
            lines.push(line.to_string());
        }
    }
    for (k, v) in updates {
        lines.push(format!("{k} = {}", format_win_value(k, v)));
    }
    lines.join("\n") + "\n"
}

/// `sanitize_project_tag`。
pub fn sanitize_project_tag(value: &str) -> String {
    let mut tag = value.trim().trim_matches('"').trim_matches('\'').to_string();
    for ch in [':', ',', ' '] {
        tag = tag.replace(ch, "_");
    }
    while tag.contains("__") {
        tag = tag.replace("__", "_");
    }
    tag.trim_matches('_').to_string()
}

/// `resolve_dos_output`：`{prefix}_dos.dat` 优先，其次 `{prefix}-dos.dat`。
pub fn resolve_dos_output(work_dir: &str, prefix: &str) -> String {
    let underscore = format!("{work_dir}/{prefix}_dos.dat");
    let hyphen = format!("{work_dir}/{prefix}-dos.dat");
    if Path::new(&underscore).exists() {
        underscore
    } else if Path::new(&hyphen).exists() {
        hyphen
    } else {
        String::new()
    }
}

/// `create_run_script`：run_postw90.sh。
pub fn create_run_script(work_dir: &str, prefix: &str, cfg: &Value) -> Result<()> {
    let post_exec_path = cfg["postw90"]["executable_path"]
        .as_str()
        .unwrap_or("postw90.x");
    let directory = std::fs::canonicalize(work_dir)?;
    let slurm_header = slurm_header(&directory, cfg);
    let script_path = format!("{work_dir}/run_postw90.sh");
    let content = format!(
        "{}\n\ncd -- {} || exit $?\n\necho \"Run postw90 at $(date)\"\n{post_exec_path} \"{prefix}\" > POSTW90.log 2>&1\n",
        slurm_header.trim(), qf_runner::shlex_quote(&directory.to_string_lossy())
    );
    std::fs::write(&script_path, content)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))?;
    }
    println!("已生成提交脚本: {script_path}");
    Ok(())
}

/// `run_postw90`。
pub fn run_postw90(work_dir: &str, prefix: &str, cfg: &Value) -> Result<()> {
    let post_exec_path = cfg["postw90"]["executable_path"]
        .as_str()
        .unwrap_or("postw90.x");
    let cmd_post = format!("{post_exec_path} \"{prefix}\" > POSTW90.log 2>&1");
    println!("执行命令: {cmd_post}");
    qf_runner::run_shell(&cmd_post, Path::new(work_dir))?;
    Ok(())
}

/// wannier_post 主流程（对齐 `main()`，run 模式含 dos_project 轮换与 .win 还原）。
pub fn run(config: &str, input: &str, run: bool, submit: bool) -> Result<()> {
    let cfg = load_post_config(config)?;
    let (win_path, prefix) = resolve_win_path(input)?;
    let work_dir = Path::new(&win_path)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| ".".to_string());

    create_run_script(&work_dir, &prefix, &cfg)?;
    if run {
        let original_content = std::fs::read_to_string(&win_path)?;
        let dos_cfg = &cfg["wannier_post"];
        let dos_projects = dos_cfg.get("dos_project");
        let projects: Vec<(String, Value)> = if let Some(obj) = dos_projects.and_then(|v| v.as_object())
        {
            obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        } else if let Some(arr) = dos_projects.and_then(|v| v.as_array()) {
            arr.iter()
                .map(|v| (sanitize_project_tag(&item_string(v)), v.clone()))
                .collect()
        } else if let Some(v) = dos_projects {
            vec![(sanitize_project_tag(&item_string(v)), v.clone())]
        } else {
            Vec::new()
        };

        let mut common_updates: Vec<(String, Value)> = Vec::new();
        if let Some(obj) = dos_cfg.as_object() {
            for (key, value) in obj {
                if key == "dos_project" {
                    continue;
                }
                let out_key = if key.starts_with("dos") {
                    key.clone()
                } else {
                    format!("dos_{key}")
                };
                common_updates.push((out_key, value.clone()));
            }
        }
        if !common_updates.iter().any(|(k, _)| k == "dos") {
            common_updates.push(("dos".into(), Value::Bool(true)));
        }

        let result: Result<()> = (|| {
            if !projects.is_empty() {
                for (tag, proj) in &projects {
                    let mut updates = common_updates.clone();
                    updates.push(("dos_project".into(), proj.clone()));
                    let content = update_win_content(&original_content, &updates);
                    std::fs::write(&win_path, content)?;
                    run_postw90(&work_dir, &prefix, &cfg)?;
                    let output_path = resolve_dos_output(&work_dir, &prefix);
                    if !output_path.is_empty() {
                        let target = format!("{work_dir}/{prefix}_dos_{tag}.dat");
                        std::fs::rename(&output_path, &target)?;
                        println!("已保存投影 DOS: {target}");
                    }
                }
            } else {
                let content = update_win_content(&original_content, &common_updates);
                std::fs::write(&win_path, content)?;
                run_postw90(&work_dir, &prefix, &cfg)?;
            }
            Ok(())
        })();
        // finally: 还原原始 .win
        let restore = std::fs::write(&win_path, &original_content);
        result?;
        restore.with_context(|| format!("还原 {win_path}"))?;
    } else if submit {
        qf_runner::submit_via_sbatch(Path::new(&work_dir), "run_postw90.sh")?;
    }
    Ok(())
}
