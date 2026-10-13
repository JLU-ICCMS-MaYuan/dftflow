//! VASP KPOINTS 生成（复刻 `vaspflow/kpoints.py` 与 vaspkit 网格派发逻辑）。
//!
//! Python 链路的 `kmesh → 整数网格` 映射由 `vaspkit` task 102 计算（stdin 序列
//! `102 → 2(Gamma) → kmesh`，kmesh 语义为 *"Kmesh-Resolved Value in Units of 2π/Å"*）。
//! 主路径见 [`write_kpoints_via_vaspkit`]：与 Python 完全相同地调用 vaspkit，by construction 对齐。
//!
//! 纯 Rust 回退公式（vaspkit 不可用时）：
//! `N_i = max(1, round(|b_i| / (2π·kmesh)))`，其中 `|b_i|` 为含 2π 的倒格矢长度。
//! 该公式已按 vaspkit 1.4.0 黑盒标定验证：
//! - 立方/正交/六方 **c 向**与 vaspkit 完全一致（a=3,K=0.04 → 8；正交 3/4/5 → 8 6 5）；
//! - 六方**面内 a 向**存在 vaspkit 特有的对称量化（实测 15/12/9，均为 3 的倍数，
//!   且台阶边界无法用闭式公式复刻），此时回退公式与 vaspkit 可能相差一档；
//!   需要文件级精确对齐的场景必须保证 vaspkit 可用。

use anyhow::{Context, Result};
use std::path::Path;

/// 由实空间晶格（3×3，单位 Å，每行为一个基矢）计算 reciprocal lattice（含 2π 因子）。
pub fn reciprocal_lattice(a: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let cross = |u: [f64; 3], v: [f64; 3]| -> [f64; 3] {
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]
    };
    let volume = a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
        - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
        + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]);
    let fac = 2.0 * std::f64::consts::PI / volume;
    let mut b = [[0.0f64; 3]; 3];
    b[0] = cross(a[1], a[2]);
    b[1] = cross(a[2], a[0]);
    b[2] = cross(a[0], a[1]);
    for row in b.iter_mut() {
        for x in row.iter_mut() {
            *x *= fac;
        }
    }
    b
}

/// kmesh → 整数网格：`N_i = max(1, round(|b_i| / (2π·kmesh)))`。
///
/// `kmesh` 语义与 vaspkit task 102 一致（单位 2π/Å 的 Kmesh-Resolved Value），
/// 等价于 `round(1/(a_i·kmesh))`（a_i 为该方向实空间周期，正交情形）。
pub fn kmesh_to_grid(lattice: &[[f64; 3]; 3], kmesh: f64) -> [i32; 3] {
    let b = reciprocal_lattice(lattice);
    let len = |v: [f64; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    let f = |x: f64| {
        if x <= 0.0 {
            1
        } else {
            let n = (x / (2.0 * std::f64::consts::PI * kmesh)).round() as i64;
            (if n < 1 { 1 } else { n }) as i32
        }
    };
    [f(len(b[0])), f(len(b[1])), f(len(b[2]))]
}

/// 尝试用 vaspkit 在 `work_dir` 内生成 KPOINTS（task 102 → Gamma → Kmesh-Resolved）。
///
/// 与 Python `run_vaspkit(work_dir, "102\n2\n{kmesh}\n")` 完全一致：同 cwd
/// （vaspkit 读取工作目录中的 `POSCAR`）、同 stdin 序列；stdout/stderr 丢弃。
/// 返回 `true` 表示 vaspkit 已写出 `KPOINTS`；`false` 表示不可用或失败，
/// 调用方应回退 [`write_kpoints_from_kmesh`]。
pub fn write_kpoints_via_vaspkit(work_dir: &str, kmesh: f64) -> bool {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let Ok(mut child) = Command::new("vaspkit")
        .current_dir(work_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(format!("102\n2\n{}\n", kmesh).as_bytes());
        // stdin 在此 drop，vaspkit 读到 EOF
    }
    let _ = child.wait();
    Path::new(work_dir).join("KPOINTS").is_file()
}

/// 按 kmesh 密度比推导用于网格点数的整数倍（复刻 `derive_integer_kmesh_multiplier`）。
pub fn derive_integer_kmesh_multiplier(scf_kmesh: f64, target_kmesh: f64) -> Result<i32> {
    if scf_kmesh <= 0.0 || target_kmesh <= 0.0 {
        anyhow::bail!("kmesh 必须为正数");
    }
    let ratio = scf_kmesh / target_kmesh;
    if ratio <= 1.0 {
        return Ok(1);
    }
    let nearest = ratio.round();
    if (ratio - nearest).abs() < 1e-8 {
        Ok(nearest as i32)
    } else {
        Ok(ratio.ceil() as i32)
    }
}

/// 给定网格与各向同性整数倍，推导放大后的网格（复刻 `expected_scaled_mesh`）。
pub fn expected_scaled_mesh(mesh: [i32; 3], multiplier: i32) -> [i32; 3] {
    [
        mesh[0] * multiplier,
        mesh[1] * multiplier,
        mesh[2] * multiplier,
    ]
}

#[derive(Debug, Clone, PartialEq)]
pub struct AutomaticKpoints {
    pub title: String,
    pub style: String,
    pub mesh: [i32; 3],
    pub shift: [f64; 3],
}

/// 写出 automatic mesh KPOINTS（复刻 `write_automatic_kpoints`）。
pub fn write_automatic_kpoints(
    path: &str,
    kpoints: &AutomaticKpoints,
) -> Result<String> {
    let shift = format!(
        "{} {} {}",
        format_number(kpoints.shift[0]),
        format_number(kpoints.shift[1]),
        format_number(kpoints.shift[2])
    );
    let content = format!(
        "{}\n0\n{}\n{} {} {}\n{}\n",
        kpoints.title,
        if kpoints.style.is_empty() {
            "Gamma".to_string()
        } else {
            kpoints.style.clone()
        },
        kpoints.mesh[0],
        kpoints.mesh[1],
        kpoints.mesh[2],
        shift
    );
    std::fs::write(path, content).with_context(|| format!("write KPOINTS {path}"))?;
    Ok(path.to_string())
}

/// 读取 automatic mesh KPOINTS（复刻 `read_automatic_kpoints`）。
pub fn read_automatic_kpoints(path: &str) -> Result<AutomaticKpoints> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read KPOINTS {path}"))?;
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() < 4 {
        anyhow::bail!("{path} 不是完整的 automatic mesh KPOINTS");
    }
    let generation_mode: i32 = lines[1]
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow::anyhow!("{path} 第二行不是 automatic mesh 标记"))?
        .parse()
        .map_err(|_| anyhow::anyhow!("{path} 第二行不是 automatic mesh 标记"))?;
    if generation_mode != 0 {
        anyhow::bail!("{path} 不是 automatic mesh KPOINTS");
    }
    let style = lines[2].trim().to_string();
    let style_lower = style.to_lowercase();
    if !(style_lower.starts_with('g') || style_lower.starts_with('m')) {
        anyhow::bail!("{path} 第三行不是 Gamma 或 Monkhorst-Pack");
    }
    let mesh_parts: Vec<i32> = lines[3]
        .split_whitespace()
        .take(3)
        .filter_map(|s| s.parse().ok())
        .collect();
    if mesh_parts.len() != 3 || mesh_parts.iter().any(|&v| v <= 0) {
        anyhow::bail!("{path} 第四行不是有效的三维正整数网格");
    }
    let mut shift = [0.0f64, 0.0, 0.0];
    if lines.len() >= 5 {
        let sp: Vec<f64> = lines[4]
            .split_whitespace()
            .take(3)
            .filter_map(|s| s.parse().ok())
            .collect();
        if sp.len() == 3 {
            shift = [sp[0], sp[1], sp[2]];
        }
    }
    Ok(AutomaticKpoints {
        title: lines[0].trim().to_string(),
        style,
        mesh: [mesh_parts[0], mesh_parts[1], mesh_parts[2]],
        shift,
    })
}

/// 写出 band path（Line-mode）KPOINTS。
pub fn write_line_kpoints(
    path: &str,
    label_points: &[(String, [f64; 3])],
    ndiv: i32,
) -> Result<String> {
    let mut content = String::new();
    content.push_str("k-points along high symmetry lines\n");
    content.push_str("20\n"); // generation mode = line
    content.push_str("Line-mode\n");
    content.push_str("reciprocal\n");
    for pair in label_points.chunks(2) {
        if pair.len() == 2 {
            for (label, p) in pair {
                content.push_str(&format!(
                    " {:.8} {:.8} {:.8} ! {}\n",
                    p[0], p[1], p[2], label
                ));
            }
            content.push('\n');
        }
    }
    let _ = ndiv;
    std::fs::write(path, content).with_context(|| format!("write KPOINTS {path}"))?;
    Ok(path.to_string())
}

fn format_number(value: f64) -> String {
    if value.fract() == 0.0 {
        (value as i64).to_string()
    } else {
        format!("{value}")
    }
}

/// 辅助：从 POSCAR 解析实空间晶格（供 kmesh_to_grid 调用）。
pub fn lattice_from_poscar(poscar_path: &str) -> Result<[[f64; 3]; 3]> {
    let structure =
        qf_crystal::parse_poscar(poscar_path).map_err(|e| anyhow::anyhow!("解析结构失败: {e}"))?;
    Ok(structure.lattice)
}

/// 便捷函数：从结构与 kmesh 直接生成 automatic KPOINTS 文件。
pub fn write_kpoints_from_kmesh(
    path: &str,
    poscar_path: &str,
    kmesh: f64,
    style: &str,
) -> Result<String> {
    let lattice = lattice_from_poscar(poscar_path)?;
    let mesh = kmesh_to_grid(&lattice, kmesh);
    let kp = AutomaticKpoints {
        title: "Automatic mesh".to_string(),
        style: style.to_string(),
        mesh,
        shift: [0.0, 0.0, 0.0],
    };
    write_automatic_kpoints(path, &kp)
}

#[allow(dead_code)]
fn _path_unused(_p: &Path) {}
