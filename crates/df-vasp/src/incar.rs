//! VASP INCAR 写出（复刻 `vaspflow` 各 setup 将合并后的参数 dict 写成 INCAR 的行为）。
//!
//! 约定：INCAR 仅写入 VASP 识别的标签；`kmesh`/`scf_kmesh`/`kpath_kmesh`/`kmesh_scheme`
//! 等控制键不写入 INCAR（它们用于派发 KPOINTS）。`None` 值跳过。

use anyhow::{Context, Result};
use qf_config::SectionMap;
use serde_json::Value;

/// 不写入 INCAR 的控制键（属于 setup 调度逻辑，非 VASP 标签）。
pub const INCAR_CONTROL_KEYS: &[&str] = &[
    "kmesh",
    "scf_kmesh",
    "kpath_kmesh",
    "kmesh_scheme",
];

/// 将合并后的参数段写成 INCAR。返回写出路径。
pub fn write_incar(params: &SectionMap, path: &str) -> Result<String> {
    let mut lines = Vec::new();
    // SYSTEM 优先放在首行（VASP 习惯）。
    if let Some(Value::String(sys)) = params.get("SYSTEM").or_else(|| params.get("system")) {
        lines.push(format!("SYSTEM = {sys}"));
    }
    for (key, value) in params {
        let k = key.to_uppercase();
        if INCAR_CONTROL_KEYS.contains(&k.to_lowercase().as_str()) {
            continue;
        }
        if k == "SYSTEM" {
            continue; // 已在首行写出
        }
        match value {
            Value::Null => continue,
            Value::Bool(b) => lines.push(format!("{k} = .{}", if *b { "TRUE." } else { "FALSE." })),
            Value::String(s) => lines.push(format!("{k} = {s}")),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    lines.push(format!("{k} = {i}"));
                } else if let Some(f) = n.as_f64() {
                    lines.push(format!("{k} = {f}"));
                }
            }
            _ => continue,
        }
    }
    let content = lines.join("\n") + "\n";
    std::fs::write(path, content).with_context(|| format!("write INCAR {path}"))?;
    Ok(path.to_string())
}
