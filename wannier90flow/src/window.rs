//! wannier_window：能量窗口解析（复刻 `wannier90flow/wannier_window.py`）。
//!
//! 支持 VASP `EIGENVAL` 与 QE/FLEUR 的 XML 输出（Har2eV = 13.60569253×2）。

use anyhow::{Context, Result};

const HAR2EV: f64 = 13.60569253 * 2.0;

/// `get_energies`：EIGENVAL 或 XML → 每个 k 点的能带本征值列表。
pub fn get_energies(xml_name: &str) -> Result<Vec<Vec<f64>>> {
    if xml_name == "EIGENVAL" {
        let text =
            std::fs::read_to_string("EIGENVAL").context("读取 EIGENVAL 失败")?;
        let raw: Vec<&str> = text.lines().collect();
        if raw.len() <= 7 {
            anyhow::bail!("EIGENVAL 内容过短");
        }
        let content = &raw[7..];
        // 4 列行 = k 点块头
        let entries: Vec<usize> = content
            .iter()
            .enumerate()
            .filter(|(_, line)| line.split_whitespace().count() == 4)
            .map(|(i, _)| i)
            .collect();
        let mut eng_full: Vec<Vec<f64>> = Vec::with_capacity(entries.len());
        for &i in &entries {
            let mut eng_ki: Vec<f64> = Vec::new();
            for line in content.iter().skip(i + 1) {
                if line.split_whitespace().count() == 3 {
                    if let Some(v) = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()) {
                        eng_ki.push(v);
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            eng_full.push(eng_ki);
        }
        Ok(eng_full)
    } else {
        // Quantum ESPRESSO / FLEUR XML
        let text =
            std::fs::read_to_string(xml_name).with_context(|| format!("读取 {xml_name} 失败"))?;
        let doc = roxmltree::Document::parse(&text)
            .map_err(|e| anyhow::anyhow!("解析 XML {xml_name} 失败: {e}"))?;
        let root = doc.root_element();
        let mut eng_full: Vec<Vec<f64>> = Vec::new();
        if root.tag_name().name() == "fleurOutput" {
            // 取最后一个 eigenvalues 节点下的 eigenvaluesAt 列表
            let eigen_nodes: Vec<_> = doc
                .descendants()
                .filter(|n| n.tag_name().name() == "eigenvalues")
                .collect();
            let Some(eigenvalues) = eigen_nodes.last() else {
                anyhow::bail!("XML 中未找到 eigenvalues");
            };
            for ek in eigenvalues
                .descendants()
                .filter(|n| n.tag_name().name() == "eigenvaluesAt")
            {
                let data = ek
                    .children()
                    .find(|c| c.node_type() == roxmltree::NodeType::Text)
                    .and_then(|t| t.text())
                    .unwrap_or("");
                eng_full.push(
                    data.split_whitespace()
                        .filter_map(|f| f.parse::<f64>().ok())
                        .map(|f| f * HAR2EV)
                        .collect(),
                );
            }
        } else if root.tag_name().name() == "espresso"
            || root.has_tag_name(("qes", "espresso"))
            || root.tag_name().name() == "output"
        {
            let eigenvalues: Vec<_> = doc
                .descendants()
                .filter(|n| n.tag_name().name() == "eigenvalues")
                .collect();
            for ek in &eigenvalues {
                let data = ek
                    .children()
                    .find(|c| c.node_type() == roxmltree::NodeType::Text)
                    .and_then(|t| t.text())
                    .unwrap_or("");
                eng_full.push(
                    data.split_whitespace()
                        .filter_map(|f| f.parse::<f64>().ok())
                        .map(|f| f * HAR2EV)
                        .collect(),
                );
            }
        } else {
            anyhow::bail!("Unknown xml output");
        }
        Ok(eng_full)
    }
}

fn band_min_max(eng_full: &[Vec<f64>], band_index: usize) -> (f64, f64) {
    let mut emin = f64::INFINITY;
    let mut emax = f64::NEG_INFINITY;
    for eng in eng_full {
        if let Some(&e) = eng.get(band_index - 1) {
            emin = emin.min(e);
            emax = emax.max(e);
        }
    }
    (emin, emax)
}

fn count_in_window(ek: &[f64], emin: f64, emax: f64) -> usize {
    ek.iter().filter(|&&e| emin <= e && e <= emax).count()
}

/// `--auto` 模式：自适应估计能窗。
fn run_auto(eng_full: &[Vec<f64>], nbnd1: usize, num_wann: usize, nbnd3: usize, delta: f64) -> Result<()> {
    let nbnd_total = eng_full[0].len();
    if nbnd1 < 1 || num_wann < 1 || nbnd3 == usize::MAX {
        anyhow::bail!("--auto 参数必须满足：nbnd1>=1, num_wann>=1, nbnd3>=0");
    }
    if nbnd1 > nbnd_total {
        anyhow::bail!("nbnd1 超出能带数量");
    }

    let (min1, _) = band_min_max(eng_full, nbnd1);
    let dis_froz_min = min1 - delta;
    let dis_win_min = dis_froz_min;

    let mut nbnd2 = nbnd1 + num_wann;
    if nbnd2 > nbnd_total {
        anyhow::bail!("nbnd1 + num_wann 超出能带数量");
    }

    let (nbnd2, dis_froz_max) = loop {
        if nbnd2 < nbnd1 {
            anyhow::bail!("自适应失败：nbnd2 已小于 nbnd1");
        }
        let (_, max2) = band_min_max(eng_full, nbnd2);
        let dfm = max2 + delta;
        let max_count = eng_full
            .iter()
            .map(|ek| count_in_window(ek, dis_froz_min, dfm))
            .max()
            .unwrap_or(0);
        println!(
            "auto: nbnd2={nbnd2}, dis_froz_max={dfm:.6}, max_nbnd={max_count}"
        );
        if max_count <= num_wann {
            break (nbnd2, dfm);
        }
        nbnd2 -= 1;
    };

    let nbnd_win = nbnd2 + nbnd3;
    if nbnd_win > nbnd_total {
        anyhow::bail!("nbnd2 + nbnd3 超出能带数量");
    }
    let (_, max_win) = band_min_max(eng_full, nbnd_win);
    let dis_win_max = max_win + delta;

    println!("dis_froz_min = {dis_froz_min:.6}");
    println!("dis_froz_max = {dis_froz_max:.6}");
    println!("dis_win_min = {dis_win_min:.6}");
    println!("dis_win_max = {dis_win_max:.6}");
    let plot_end = (nbnd_win + 8).min(nbnd_total);
    plot_band_ranges(
        eng_full,
        1,
        plot_end,
        dis_froz_min,
        dis_froz_max,
        dis_win_min,
        dis_win_max,
        "band_ranges.png",
    )?;
    Ok(())
}

/// `plot_band_ranges`：每条带的红柱 + 4 条水平参考线（plotters 内容等价重绘）。
fn plot_band_ranges(
    eng_full: &[Vec<f64>],
    start_band: usize,
    end_band: usize,
    dis_froz_min: f64,
    dis_froz_max: f64,
    dis_win_min: f64,
    dis_win_max: f64,
    output_path: &str,
) -> Result<()> {
    use plotters::prelude::*;
    let band_indices: Vec<usize> = (start_band..=end_band).collect();
    let mut y_min = dis_froz_min.min(dis_win_min);
    let mut y_max = dis_froz_max.max(dis_win_max);
    for &i in &band_indices {
        let (emin, emax) = band_min_max(eng_full, i);
        y_min = y_min.min(emin);
        y_max = y_max.max(emax);
    }
    let pad = (y_max - y_min).abs() * 0.05 + 1e-9;
    let (y_min, y_max) = (y_min - pad, y_max + pad);
    let x_max = *band_indices.iter().max().unwrap_or(&1) as f64 + 1.0;

    let root = BitMapBackend::new(output_path, (1200, 900)).into_drawing_area();
    root.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&root)
        .caption("Energy spectrum of bands", ("sans-serif", 24))
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(0.0..x_max, y_min..y_max)?;
    chart
        .configure_mesh()
        .x_desc("Band Index")
        .y_desc("Energy")
        .x_label_formatter(&|v| {
            if v.fract() == 0.0 && *v >= 1.0 {
                format!("{}", *v as i64)
            } else {
                String::new()
            }
        })
        .draw()?;
    // 每条带的能量范围红柱
    for &i in &band_indices {
        let (emin, emax) = band_min_max(eng_full, i);
        if emax > emin {
            chart.draw_series(std::iter::once(Rectangle::new(
                [(i as f64 - 0.3, emin), (i as f64 + 0.3, emax)],
                RED.mix(0.3).filled(),
            )))?;
        }
    }
    // 4 条参考线
    for (y, color) in [
        (dis_froz_min, RED),
        (dis_froz_max, RED),
        (dis_win_min, BLACK),
        (dis_win_max, BLACK),
    ] {
        chart.draw_series(std::iter::once(PathElement::new(
            vec![(0.0, y), (x_max, y)],
            color.stroke_width(2),
        )))?;
    }
    root.present()?;
    Ok(())
}

/// wannier_window 主流程。
pub enum WindowMode {
    Bands(Vec<usize>),
    Energies(Vec<f64>),
    Auto {
        nbnd1: usize,
        num_wann: usize,
        nbnd3: usize,
        delta: f64,
    },
}

pub fn run(xml: &str, mode: WindowMode) -> Result<()> {
    let eng_full = get_energies(xml)?;
    match mode {
        WindowMode::Bands(n) => {
            if n.is_empty() || n.len() > 2 {
                anyhow::bail!("-n 需要 1 或 2 个整数参数");
            }
            if n.iter().any(|&i| i < 1) {
                anyhow::bail!("band_index 必须从 1 开始");
            }
            if n.len() == 1 {
                let (emin, emax) = band_min_max(&eng_full, n[0]);
                println!("emin = {emin:.6}");
                println!("emax = {emax:.6}");
            } else {
                let (emin, _) = band_min_max(&eng_full, n[0]);
                let (_, emax) = band_min_max(&eng_full, n[1]);
                println!("emin = {emin:.6}");
                println!("emax = {emax:.6}");
            }
            Ok(())
        }
        WindowMode::Energies(e) => {
            if e.is_empty() || e.len() > 2 {
                anyhow::bail!("-e 需要 1 或 2 个数值参数");
            }
            if e.len() == 1 {
                let band_index = e[0] as usize;
                if band_index < 1 {
                    anyhow::bail!("band_index 必须从 1 开始");
                }
                let (emin, emax) = band_min_max(&eng_full, band_index);
                println!("emin = {emin:.6}");
                println!("emax = {emax:.6}");
            } else {
                let (emin, emax) = (e[0], e[1]);
                let mut nbnd: Vec<usize> = Vec::with_capacity(eng_full.len());
                for (ik, ek) in eng_full.iter().enumerate() {
                    let num = count_in_window(ek, emin, emax);
                    println!("ik = {}, nbnd = {num}", ik + 1);
                    nbnd.push(num);
                }
                println!("{} {}", nbnd.iter().min().unwrap_or(&0), nbnd.iter().max().unwrap_or(&0));
            }
            Ok(())
        }
        WindowMode::Auto {
            nbnd1,
            num_wann,
            nbnd3,
            delta,
        } => run_auto(&eng_full, nbnd1, num_wann, nbnd3, delta),
    }
}
