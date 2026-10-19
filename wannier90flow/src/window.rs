//! wannier_window：能量窗口解析与上游默认组合选窗。
//!
//! 支持 VASP `EIGENVAL` 与 QE/FLEUR 的 XML 输出（Har2eV = 13.60569253×2）。

use anyhow::{Context, Result};

const HAR2EV: f64 = 13.60569253 * 2.0;

/// `get_energies`：EIGENVAL 或 XML → 每个 k 点的能带本征值列表。
pub fn get_energies(xml_name: &str) -> Result<Vec<Vec<f64>>> {
    if std::path::Path::new(xml_name).file_name().is_some_and(|n| n == "EIGENVAL") {
        let text =
            std::fs::read_to_string(xml_name).context("读取 EIGENVAL 失败")?;
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

/// wannier_window 主流程。
pub enum WindowMode {
    Bands(Vec<usize>),
    Energies(Vec<f64>),
    Default {
        num_wann: usize,
        fermi: Option<f64>,
        exclude_bands: Vec<usize>,
    },
}

pub fn run(xml: &str, mode: WindowMode) -> Result<()> {
    if let WindowMode::Default { num_wann, fermi, exclude_bands } = mode {
        let (bands, actual_fermi) = if xml.ends_with(".xml") {
            let (bands, actual) = qf_wannier::read_qe_bands(std::path::Path::new(xml))?;
            (bands, fermi.unwrap_or(actual))
        } else {
            (get_energies(xml)?, fermi.context("非 QE XML 的自动选窗必须指定 --fermi（绝对 eV）")?)
        };
        let selected = qf_wannier::select_default(&bands, actual_fermi, num_wann, &exclude_bands,
            &qf_wannier::Bounds::default(), qf_wannier::Reference::UpstreamRelative)?;
        println!("{}", serde_json::to_string_pretty(&selected)?);
        return Ok(());
    }
    let eng_full = get_energies(xml)?;
    let nbnd = qf_wannier::validate_bands(&eng_full)?;
    match mode {
        WindowMode::Bands(n) => {
            if n.is_empty() || n.len() > 2 {
                anyhow::bail!("-n 需要 1 或 2 个整数参数");
            }
            if n.iter().any(|&i| i < 1 || i > nbnd) {
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
                if !e[0].is_finite() || e[0].fract() != 0.0 || band_index < 1 || band_index > nbnd {
                    anyhow::bail!("band_index 必须从 1 开始");
                }
                let (emin, emax) = band_min_max(&eng_full, band_index);
                println!("emin = {emin:.6}");
                println!("emax = {emax:.6}");
            } else {
                let (emin, emax) = (e[0], e[1]);
                anyhow::ensure!(emin.is_finite() && emax.is_finite() && emin <= emax, "能量范围必须有限且递增");
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
        WindowMode::Default { .. } => unreachable!(),
    }
}
