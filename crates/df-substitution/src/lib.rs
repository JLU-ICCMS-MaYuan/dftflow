//! df-substitution：`substitutionflow` 的 Rust 移植（selection / structure /
//! status / workflow / prepare / plot_eledos / check_phonon）。

pub mod check_phonon;
pub mod plot_eledos;
pub mod prepare;
pub mod selection;
pub mod status;
pub mod structure;
pub mod workflow;

use anyhow::{Context, Result};
use serde_json::Value;

/// `public.config_templates.load_config_file`：.json 用 json，其余按 TOML 解析。
pub fn load_config_file(path: &str) -> Result<Value> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read config {path}"))?;
    if path.ends_with(".json") {
        return serde_json::from_str(&text).with_context(|| format!("parse JSON {path}"));
    }
    let value: toml::Value = toml::from_str(&text).with_context(|| format!("parse TOML {path}"))?;
    serde_json::to_value(value).context("convert TOML to JSON value")
}

/// `public.job.write_job_script`（announce=False 由调用方打印）。
pub fn write_job_script(script_path: &str, header: &str, commands: &[String]) -> Result<()> {
    let mut text = String::new();
    let header = header.trim();
    if !header.is_empty() {
        text.push_str(header);
        text.push_str("\n\n");
    } else {
        text.push_str("#!/bin/bash\n");
    }
    for command in commands {
        text.push_str(command.trim_end_matches('\n'));
        text.push('\n');
    }
    std::fs::write(script_path, text).with_context(|| format!("write {script_path}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(script_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(script_path, perms)?;
    }
    Ok(())
}

/// 压强键：整数不带小数点（Python `f"{p:g}"`），其余走最短浮点表示。
pub fn pressure_key(pressure: &Value) -> String {
    if let Some(i) = pressure.as_i64() {
        return format!("{i}");
    }
    if let Some(f) = pressure.as_f64() {
        if f.fract() == 0.0 && f.abs() < 1e15 {
            return format!("{}", f as i64);
        }
        return format!("{f}");
    }
    match pressure {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// `shlex.quote` 最小子集。
pub fn shlex_quote(s: &str) -> String {
    if s.is_empty() {
        return "''".to_string();
    }
    if s.chars().all(|c| {
        c.is_ascii_alphanumeric() || "_%+=:,./-@".contains(c)
    }) {
        return s.to_string();
    }
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// `shlex.join`。
pub fn shlex_join(items: &[String]) -> String {
    items.iter().map(|s| shlex_quote(s)).collect::<Vec<_>>().join(" ")
}

/// 读取目录下按名称排序的条目。
pub fn list_sorted(dir: &str) -> std::io::Result<Vec<String>> {
    let mut names: Vec<String> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    Ok(names)
}

pub fn is_nonempty_file(path: &str) -> bool {
    std::fs::metadata(path).map(|m| m.len() > 0).unwrap_or(false)
}

pub fn directory_has_any_file(path: &str) -> bool {
    fn walk(dir: &std::path::Path) -> bool {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for entry in rd.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    if walk(&p) {
                        return true;
                    }
                } else {
                    return true;
                }
            }
        }
        false
    }
    let p = std::path::Path::new(path);
    p.is_dir() && walk(p)
}
