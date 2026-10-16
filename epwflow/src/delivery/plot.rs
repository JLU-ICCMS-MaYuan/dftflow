//! 轻量 SVG 绘图（FR-001/003）：不引入重型依赖，直接生成可缩放矢量图。
//! 真实验收表与重绘入口仅依赖本模块产出的 SVG/CSV，可在无网络环境查看。

use anyhow::Result;
use std::path::Path;

/// 生成 α²F(ω) 折线图 SVG（频率 meV，a²F meV）。
pub fn plot_alpha2f(frequencies_mev: &[f64], a2f: &[f64], out: &Path) -> Result<()> {
    let svg = line_chart("alpha2F (meV)", "frequency (meV)", "alpha2F (meV)", frequencies_mev, a2f);
    std::fs::write(out, svg)?;
    Ok(())
}

/// 生成能隙随温度变化 SVG（温度 K，能隙 meV）。
pub fn plot_gap_vs_t(temperatures_k: &[f64], gaps_mev: &[f64], out: &Path) -> Result<()> {
    let svg = line_chart("gap vs T", "temperature (K)", "gap (meV)", temperatures_k, gaps_mev);
    std::fs::write(out, svg)?;
    Ok(())
}

/// 通用折线 SVG（最小实现，含坐标轴与折线，便于浏览器/编辑器直接打开）。
fn line_chart(title: &str, xlabel: &str, ylabel: &str, xs: &[f64], ys: &[f64]) -> String {
    let w: i32 = 640;
    let h: i32 = 360;
    let ml: i32 = 60;
    let mb: i32 = 50;
    let plot_w = (w - ml - 20) as f64;
    let plot_h = (h - mb - 20) as f64;
    let (xmin, xmax) = bounds(xs);
    let (ymin, ymax) = bounds(ys);
    let sx = |x: f64| ml as f64 + (if xmax - xmin == 0.0 { 0.0 } else { (x - xmin) / (xmax - xmin) }) * plot_w;
    let sy = |y: f64| (h - mb) as f64 - (if ymax - ymin == 0.0 { 0.0 } else { (y - ymin) / (ymax - ymin) }) * plot_h;
    let mut path = String::new();
    for (i, (x, y)) in xs.iter().zip(ys.iter()).enumerate() {
        let cmd = if i == 0 { "M" } else { "L" };
        path.push_str(&format!(" {} {:.1} {:.1}", cmd, sx(*x), sy(*y)));
    }
    let y0 = (h - mb).to_string();
    let x1 = (w - 20).to_string();
    let yt = (h - 10).to_string();
    let yl = (h / 2).to_string();
    let mut s = String::new();
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n",
        w, h, w, h
    ));
    s.push_str("  <rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n");
    s.push_str(&format!(
        "  <text x=\"{}\" y=\"20\" font-family=\"sans-serif\" font-size=\"14\">{}</text>\n",
        ml, title
    ));
    s.push_str(&format!(
        "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#888\"/>\n",
        ml, y0, x1, y0
    ));
    s.push_str(&format!(
        "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"20\" stroke=\"#888\"/>\n",
        ml, y0, ml
    ));
    s.push_str(&format!(
        "  <text x=\"{}\" y=\"{}\" font-family=\"sans-serif\" font-size=\"12\">{}</text>\n",
        ml, yt, xlabel
    ));
    s.push_str(&format!(
        "  <text x=\"10\" y=\"{}\" font-family=\"sans-serif\" font-size=\"12\" transform=\"rotate(-90 10 {})\">{}</text>\n",
        yl, yl, ylabel
    ));
    s.push_str(&format!(
        "  <path d=\"{}\" fill=\"none\" stroke=\"#1f6eb\" stroke-width=\"2\"/>\n",
        path
    ));
    s.push_str("</svg>\n");
    s
}

fn bounds(v: &[f64]) -> (f64, f64) {
    if v.is_empty() {
        return (0.0, 1.0);
    }
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for x in v {
        lo = lo.min(*x);
        hi = hi.max(*x);
    }
    if hi - lo < 1e-9 {
        hi = lo + 1.0;
    }
    (lo, hi)
}

/// 将结果包关键量导出为 CSV（FR-003），便于可移植重绘。
pub fn export_csv(bundle: &super::report_store::ResultsBundle, out: &Path) -> Result<()> {
    let mut s = String::from("key,value\n");
    if let Some(tc) = bundle.tc {
        s.push_str(&format!("tc_k,{}\n", tc));
    }
    s.push_str(&format!("lambda_eff,{}\n", bundle.lambda_eff));
    for (i, g) in bundle.gaps_mev.iter().enumerate() {
        s.push_str(&format!("gap_mev_{},{}\n", i, g));
    }
    std::fs::write(out, s)?;
    Ok(())
}
