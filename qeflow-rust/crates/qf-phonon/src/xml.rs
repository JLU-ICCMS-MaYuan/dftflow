//! 解析 Quantum ESPRESSO 的 `data-file-schema.xml`（SCF save 目录里），提取生成 dyn0 所需的：
//! - 倒格矢 `b1/b2/b3`（笛卡尔，tpiba=2π/a₀ 单位）
//! - 全部晶体轴旋转矩阵 `s`（`<symmetry><rotation>`）
//! - `time_reversal`（缺省按非磁性体系视为 `.TRUE.`）

use anyhow::{bail, Context};
use regex::Regex;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct QeGridInfo {
    /// `bg[vector][component]`，即 `b1/b2/b3` 三个倒格矢，每个含 x/y/z 分量。
    pub bg: [[f64; 3]; 3],
    /// 晶体轴旋转矩阵 `s[ns][row][col]`，与 `kpoint_grid` 的 `s(i,j,ns)` 对齐。
    pub rotations: Vec<[[i32; 3]; 3]>,
    /// 时间反演对称性（非磁性默认 true）。
    pub time_reversal: bool,
}

fn with_s(_text: &str, pattern: &str) -> anyhow::Result<Regex> {
    Regex::new(&format!("(?s){}", pattern)).with_context(|| format!("正则编译失败: {pattern}"))
}

fn parse_three_floats(body: &str) -> anyhow::Result<[f64; 3]> {
    let mut out = [0.0_f64; 3];
    let mut it = body.split_whitespace();
    for slot in out.iter_mut() {
        let tok = it.next().context("向量分量不足三个")?;
        *slot = tok
            .parse::<f64>()
            .with_context(|| format!("无法解析浮点数: {tok}"))?;
    }
    Ok(out)
}

fn parse_nine_ints(body: &str) -> anyhow::Result<[[i32; 3]; 3]> {
    // QE 把晶体轴旋转矩阵以浮点（如 1.000000000000000E+000）写出，这里按浮点解析后四舍五入为整数。
    let mut vals = [0i32; 9];
    let mut it = body.split_whitespace();
    for slot in vals.iter_mut() {
        let tok = it.next().context("旋转矩阵分量不足九个")?;
        let f = tok
            .parse::<f64>()
            .with_context(|| format!("无法解析浮点数: {tok}"))?;
        *slot = f.round() as i32;
    }
    Ok([vals[0..3].try_into().unwrap(), vals[3..6].try_into().unwrap(), vals[6..9].try_into().unwrap()])
}

pub fn parse_qe_save_xml(path: &Path) -> anyhow::Result<QeGridInfo> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("无法读取 XML: {}", path.display()))?;

    let b1 = with_s(&text, r"<b1>(.*?)</b1>")?
        .captures(&text)
        .context("找不到 <b1>")?;
    let b2 = with_s(&text, r"<b2>(.*?)</b2>")?
        .captures(&text)
        .context("找不到 <b2>")?;
    let b3 = with_s(&text, r"<b3>(.*?)</b3>")?
        .captures(&text)
        .context("找不到 <b3>")?;
    let bg = [
        parse_three_floats(b1.get(1).unwrap().as_str())?,
        parse_three_floats(b2.get(1).unwrap().as_str())?,
        parse_three_floats(b3.get(1).unwrap().as_str())?,
    ];

    let rot_re = with_s(&text, r"<rotation[^>]*>(.*?)</rotation>")?;
    let mut rotations = Vec::new();
    for cap in rot_re.captures_iter(&text) {
        rotations.push(parse_nine_ints(cap.get(1).unwrap().as_str())?);
    }
    if rotations.is_empty() {
        bail!("XML 中未找到任何 <rotation> 对称矩阵");
    }

    // time_reversal / use_time_reversal：缺省按 .TRUE.（非磁性）。
    // QE 的 logical 写为 `.true.`/`.false.`（带点），需去除点号再判。
    let time_reversal = match with_s(&text, r"<(time_reversal|use_time_reversal)>(.*?)</")
        .ok()
        .and_then(|re| re.captures(&text))
    {
        Some(cap) => {
            let v = cap
                .get(2)
                .unwrap()
                .as_str()
                .trim()
                .replace('.', "")
                .to_lowercase();
            v == "true" || v == "t" || v == "1" || v == "yes"
        }
        None => true,
    };

    Ok(QeGridInfo { bg, rotations, time_reversal })
}
