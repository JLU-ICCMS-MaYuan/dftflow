//! QE dyn0 的实际网格和不可约 q 点，不以单位矩阵代替非正交倒格矢。
use super::{
    manifest::QPoint,
    xml::{self, Geometry},
};
use anyhow::{ensure, Context, Result};
use std::path::Path;

pub fn parse_dyn0(
    path: &Path,
    geometry: &Geometry,
    expected_grid: [u32; 3],
    coords: bool,
) -> Result<Vec<QPoint>> {
    let raw = std::fs::read_to_string(path)?;
    let mut lines = raw.lines().filter(|line| !line.trim().is_empty());
    let grid = lines
        .next()
        .context("data_invalid: dyn0 缺网格")?
        .split_whitespace()
        .map(str::parse::<u32>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(grid == expected_grid, "data_invalid: dyn0 网格与配置不符");
    let count: usize = lines
        .next()
        .context("data_invalid: dyn0 缺 q 数")?
        .trim()
        .parse()?;
    ensure!(count > 0, "data_invalid: dyn0 q 数为零");
    let values = lines.map(xml::vector).collect::<Result<Vec<_>>>()?;
    ensure!(
        values.len() == count,
        "data_invalid: dyn0 声明数量与坐标数不符"
    );
    let mut points: Vec<QPoint> = vec![];
    let width = count.to_string().len().max(2);
    for (n, cartesian) in values.into_iter().enumerate() {
        let crystal = geometry.crystal(cartesian);
        ensure!(
            crystal
                .iter()
                .zip(expected_grid)
                .all(|(x, g)| (x * g as f64 - (x * g as f64).round()).abs() < 1e-8),
            "data_invalid: dyn0 q 点不属于固定均匀网格"
        );
        ensure!(
            points
                .iter()
                .all(|q| !same_q_modulo_periodic(q.crystal, crystal)),
            "data_invalid: dyn0 重复物理 q"
        );
        points.push(QPoint {
            global_index: n + 1,
            local_index: if coords { 1 } else { n + 1 },
            directory: format!("{:0width$}", n + 1),
            crystal,
            cartesian,
            alat: geometry.alat,
            reciprocal_basis: geometry.reciprocal,
            source_dyn0: path.into(),
        });
    }
    ensure!(
        points[0].crystal.iter().all(|x| x.abs() < 1e-9),
        "data_invalid: 目标 EPW 网格第一 q 必须为 Γ"
    );
    Ok(points)
}

pub fn directory_of(global_index: usize) -> String {
    format!("{global_index:02}")
}

pub fn same_q_modulo_periodic(a: [f64; 3], b: [f64; 3]) -> bool {
    a.iter()
        .zip(b)
        .all(|(x, y)| (x - y - (x - y).round()).abs() < 1e-9)
}
