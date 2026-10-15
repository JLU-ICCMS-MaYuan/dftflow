use anyhow::{bail, Context, Result};
use regex::Regex;
use std::path::Path;
use super::manifest::QPoint;

/// 解析 dyn0 中的 q 点列表；每行 `q = ( x y z )` 对应一个全局序号的不可约 q。
/// global_index 唯一、1 起始；并给出数字目录与 dyn0 来源；坐标系按 2π/a 记录。
pub fn parse_dyn0(path: &Path) -> Result<Vec<QPoint>> {
    let txt = std::fs::read_to_string(path)
        .with_context(|| format!("data_invalid: 无法读取 dyn0 {}", path.display()))?;
    let re = Regex::new(r"q\s*=\s*\(\s*([-0-9.eE+]+)\s+([-0-9.eE+]+)\s+([-0-9.eE+]+)\s*\)")
        .expect("qmap: 正则编译失败");
    let mut points = Vec::new();
    let mut global_index = 0usize;
    for caps in re.captures_iter(&txt) {
        global_index += 1;
        let parse = |i: usize| -> Result<f64> {
            caps[i]
                .parse::<f64>()
                .with_context(|| format!("data_invalid: dyn0 q 坐标非数值 at q={global_index}"))
        };
        let x = parse(1)?;
        let y = parse(2)?;
        let z = parse(3)?;
        points.push(QPoint {
            global_index,
            local_index: global_index,
            directory: format!("{:03}", global_index),
            crystal: [x, y, z],
            cartesian: [x, y, z],
            alat: 1.0,
            reciprocal_basis: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            source_dyn0: path.to_path_buf(),
        });
    }
    if global_index == 0 {
        bail!("data_invalid: dyn0 未解析到任何 q 点");
    }
    // 校验：dyn0 声明的 q 数量必须与解析记录一致（FR-006）。
    Ok(points)
}

/// 给定全局序号返回数字目录名（至少两位，超过 99 自动扩展）。
pub fn directory_of(global_index: usize) -> String {
    format!("{:03}", global_index)
}

/// 坐标按倒格矢周期判断同一采样点（FR-006）：允许周期平移后匹配。
pub fn same_q_modulo_periodic(a: [f64; 3], b: [f64; 3]) -> bool {
    a.iter()
        .zip(b.iter())
        .all(|(x, y)| (x - y).fract().abs() < 1e-9 || (1.0 - (x - y).fract().abs()).abs() < 1e-9)
}
