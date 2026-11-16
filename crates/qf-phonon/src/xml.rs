//! 读取 QE SCF XML 的倒格矢，供晶体/笛卡尔坐标转换使用。
use anyhow::Context;
use regex::Regex;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct QeGridInfo {
    /// `bg[vector][component]`，即 `b1/b2/b3` 三个倒格矢，每个含 x/y/z 分量。
    pub bg: [[f64; 3]; 3],
}

fn with_s(_text: &str, pattern: &str) -> anyhow::Result<Regex> {
    Regex::new(&format!("(?s){}", pattern)).with_context(|| format!("Rectification failed: {pattern}"))
}

fn parse_three_floats(body: &str) -> anyhow::Result<[f64; 3]> {
    let mut out = [0.0_f64; 3];
    let mut it = body.split_whitespace();
    for slot in out.iter_mut() {
        let tok = it.next().context("Less than three vectors")?;
        *slot = tok
            .parse::<f64>()
            .with_context(|| format!("Could not parse floats: {tok}"))?;
    }
    Ok(out)
}

pub fn parse_qe_save_xml(path: &Path) -> anyhow::Result<QeGridInfo> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Unable to read XML: {}", path.display()))?;

    let b1 = with_s(&text, r"<b1>(.*?)</b1>")?
        .captures(&text)
        .context("Not found <b1>")?;
    let b2 = with_s(&text, r"<b2>(.*?)</b2>")?
        .captures(&text)
        .context("Not found <b2>")?;
    let b3 = with_s(&text, r"<b3>(.*?)</b3>")?
        .captures(&text)
        .context("Not found <b3>")?;
    let bg = [
        parse_three_floats(b1.get(1).unwrap().as_str())?,
        parse_three_floats(b2.get(1).unwrap().as_str())?,
        parse_three_floats(b3.get(1).unwrap().as_str())?,
    ];

    Ok(QeGridInfo { bg })
}
