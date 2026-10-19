//! wannier90flow 配置模板与合并（复刻 `wannier90flow/templates.py` +
//! `public.config_templates.build_cli_config`）。
//!
//! Python 语义：把用户配置顶层键过滤到 allowed_keys 白名单后，递归深合并进模板
//! （`None` 不覆盖，双方均为表才递归，见 `qf_config::merge_recursive`）。

use anyhow::Result;
use serde_json::Value;

/// 共享白名单（对齐 `WANNIER_SHARED_ALLOWED_KEYS`）。
pub const SHARED_ALLOWED_KEYS: [&str; 4] = ["slurm", "wannier90", "pw2wannier90", "postw90"];

/// `_base_wannier_template()`。
pub fn base_template() -> Value {
    serde_json::json!({
        "slurm": { "header": "" },
        "wannier90": { "executable_path": "wannier90.x" },
        "pw2wannier90": {
            "executable_path": "pw2wannier90.x",
            "input_file": null,
            "prefix": null,
            "outdir": "./",
            "seedname": null,
            "write_amn": true,
            "write_mmn": true,
            "write_unk": null,
            "spin_component": null,
            "wan_mode": null
        },
        "postw90": { "executable_path": "postw90.x" }
    })
}

/// `get_wannier_init_parameters_template()`。
pub fn wannier_init_template() -> Value {
    let mut t = base_template();
    t["wannier_init"] = serde_json::json!({
        "system_name": null,
        "num_wann": null,
        "num_bands": null,
        "bands_file": null,
        "exclude_bands": null,
        "iprint": null,
        "num_iter": null,
        "dis_num_iter": null,
        "dis_froz_min": null,
        "dis_froz_max": null,
        "dis_win_min": null,
        "dis_win_max": null,
        "write_hr": null,
        "write_bvec": null,
        "bands_plot": false,
        "bands_plot_format": null,
        "wvfn_formatted": null,
        "wannier_plot": null,
        "wannier_plot_format": null,
        "fermi_energy": null,
        "fermi_surface_plot": null,
        "dos": null,
        "dos_kmesh": null,
        "dos_project": null,
        "projections": { "list": [] },
        "k_points": { "mp_grid": [4, 4, 4], "kpoints": null, "kpath_file": "KPATH.in" }
    });
    t
}

/// `get_wannier_post_parameters_template()`。
pub fn wannier_post_template() -> Value {
    let mut t = base_template();
    t["wannier_post"] = serde_json::json!({
        "dos": true,
        "kmesh": null,
        "project": null,
        "dos_project": null
    });
    t
}

/// `build_cli_config`：白名单过滤 + 递归深合并（None 不覆盖）。
pub fn apply_config_overrides(
    template: &mut Value,
    input_config: &Value,
    allowed_keys: &[&str],
) -> Result<Value> {
    let Some(obj) = input_config.as_object() else {
        anyhow::bail!("配置文件顶层必须是表");
    };
    let mut filtered = serde_json::Map::new();
    for (k, v) in obj {
        if allowed_keys.contains(&k.as_str()) {
            filtered.insert(k.clone(), v.clone());
        }
    }
    qf_config::merge_recursive(template, &Value::Object(filtered));
    Ok(template.clone())
}

/// `wannier_init` 的配置加载（模板 + 白名单 + 合并）。
pub fn load_init_config(path: &str) -> Result<Value> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("读取配置失败 {path}: {e}"))?;
    let input: Value = parse_toml_or_json(&text, path)?;
    let mut template = wannier_init_template();
    let mut keys = SHARED_ALLOWED_KEYS.to_vec();
    keys.push("wannier_init");
    let mut cfg = apply_config_overrides(&mut template, &input, &keys)?;
    if let Some(file) = cfg["wannier_init"]["bands_file"].as_str() {
        let file = std::path::Path::new(file);
        if file.is_relative() {
            let base = std::path::Path::new(path).parent().unwrap_or(std::path::Path::new("."));
            cfg["wannier_init"]["bands_file"] = Value::String(base.join(file).to_string_lossy().into_owned());
        }
    }
    Ok(cfg)
}

/// TOML 优先、失败时尝试 JSON 的宽容解析（对齐 `load_config_file` 按扩展名/内容尝试）。
pub fn parse_toml_or_json(text: &str, path: &str) -> Result<Value> {
    if path.ends_with(".json") {
        let v: Value = serde_json::from_str(text)
            .map_err(|e| anyhow::anyhow!("解析 JSON {path} 失败: {e}"))?;
        return Ok(v);
    }
    match toml::from_str::<toml::Value>(text) {
        Ok(v) => {
            let json_v: Value =
                serde_json::to_value(v).map_err(|e| anyhow::anyhow!("TOML 转 JSON 失败: {e}"))?;
            Ok(json_v)
        }
        Err(toml_err) => {
            // 尝试 JSON（对齐 Python load_config_file 的 TOML→JSON 回退）
            match serde_json::from_str::<Value>(text) {
                Ok(v) => Ok(v),
                Err(_) => anyhow::bail!("解析配置 {path} 失败: {toml_err}"),
            }
        }
    }
}
