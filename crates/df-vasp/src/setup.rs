//! VASP setup 编排：把合并后的参数段写出 INCAR/KPOINTS/POTCAR，并拷贝 POSCAR 到工作目录。
//! 运行脚本与 run/submit 由 `vaspflow` 二进制负责（保持 df-vasp 为纯库）。

use anyhow::{Context, Result};
use qf_config::{merge_recursive, SectionMap};
use serde_json::{Map, Value};

use crate::incar;
use crate::kpoints;
use crate::potcar;

/// 合并默认模板与用户段，返回最终 INCAR 参数（内部 `vasp_*` 子表）。
pub fn merged_params(name: &str, user_section: &SectionMap) -> SectionMap {
    let defaults = crate::templates::default_params(name);
    let mut merged_value = if let Some(Value::Object(d)) = defaults.get(name) {
        Value::Object(d.clone())
    } else {
        Value::Object(Default::default())
    };
    let user_value = Value::Object(
        user_section
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Map<String, Value>>(),
    );
    merge_recursive(&mut merged_value, &user_value);
    match merged_value {
        Value::Object(map) => map.into_iter().collect(),
        _ => SectionMap::new(),
    }
}

/// 写出 VASP 输入三件套（INCAR/KPOINTS/POTCAR）并拷贝 POSCAR。
/// 返回 (incar_path, kpoints_path, potcar_path)。
///
/// 生成顺序对齐 Python `BaseVaspCohpSetup.setup()`：
/// POSCAR → INCAR → POTCAR → KPOINTS（vaspkit 读工作目录中的 POSCAR）。
pub fn build_vasp_inputs(
    name: &str,
    user_section: &SectionMap,
    poscar_path: &str,
    potcar_dir: &str,
    out_dir: &str,
) -> Result<(String, String, String)> {
    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("create {out_dir}"))?;
    let params = merged_params(name, user_section);

    // 拷贝 POSCAR（先行：vaspkit 需读取工作目录中的 POSCAR）
    let src = std::fs::read(poscar_path).with_context(|| format!("read {poscar_path}"))?;
    std::fs::write(format!("{out_dir}/POSCAR"), src)
        .with_context(|| format!("copy POSCAR to {out_dir}"))?;

    // INCAR（vaspkit 不会覆盖已存在的 INCAR，已实测验证）
    let incar_path = incar::write_incar(&params, &format!("{out_dir}/INCAR"))?;

    // POTCAR
    let structure =
        qf_crystal::parse_poscar(poscar_path).map_err(|e| anyhow::anyhow!("解析结构失败: {e}"))?;
    let elements = potcar::distinct_elements(&structure);
    let potcar_path = if !potcar_dir.is_empty() {
        potcar::concat_potcar(potcar_dir, &elements, &format!("{out_dir}/POTCAR"))?
    } else {
        String::new()
    };

    // KPOINTS：kmesh 驱动 automatic mesh；fatband 用 kpath_kmesh 的 line（此处仍走 mesh）。
    // 主路径调用 vaspkit（与 Python 链路 by construction 一致），不可用时回退纯 Rust 公式。
    let kmesh = params
        .get("KMESH")
        .and_then(|v| v.as_f64())
        .or_else(|| params.get("kmesh").and_then(|v| v.as_f64()))
        .unwrap_or(0.025);
    let kpoints_path = if kpoints::write_kpoints_via_vaspkit(out_dir, kmesh) {
        println!("[{name}] KPOINTS 由 vaspkit 生成（与 Python 链路一致）");
        format!("{out_dir}/KPOINTS")
    } else {
        println!(
            "[{name}] vaspkit 不可用或运行失败，回退纯 Rust 公式生成 KPOINTS（六方面内网格可能与 vaspkit 有偏差）"
        );
        kpoints::write_kpoints_from_kmesh(
            &format!("{out_dir}/KPOINTS"),
            poscar_path,
            kmesh,
            "Gamma",
        )?
    };

    let _ = elements;
    Ok((incar_path, kpoints_path, potcar_path))
}
