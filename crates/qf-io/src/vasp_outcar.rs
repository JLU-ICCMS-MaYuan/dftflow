//! VASP `OUTCAR` 解析（对齐 `vaspflow` 后处理读取 Fermi 能/总能/晶格）。
//!
//! 关注字段：E-fermi、energy without entropy、direct lattice vectors、volume。

use anyhow::Result;
use regex::Regex;

#[derive(Debug, Clone, Default)]
pub struct Outcar {
    pub efermi: Option<f64>,
    pub total_energy: Option<f64>,
    pub volume: Option<f64>,
    pub lattice: Option<[[f64; 3]; 3]>,
}

fn first_float(text: &str, pat: &str) -> Option<f64> {
    let re = Regex::new(pat).ok()?;
    re.captures(text)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

/// 解析 `OUTCAR` 文本（纯函数）。
pub fn parse_outcar(text: &str) -> Result<Outcar> {
    let mut out = Outcar::default();
    let mut lattice: Option<[[f64; 3]; 3]> = None;
    let lines: Vec<&str> = text.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if out.efermi.is_none() {
            if let Some(v) = first_float(line, r"E-fermi\s*:\s*([-\d.Ee+]+)") {
                out.efermi = Some(v);
            }
        }
        if out.total_energy.is_none() {
            if let Some(v) = first_float(line, r"energy\s+without entropy=\s*([-\d.Ee+]+)") {
                out.total_energy = Some(v);
            }
        }
        if out.volume.is_none() {
            if let Some(v) = first_float(line, r"(?i)volume of cell\s*:\s*([-\d.Ee+]+)") {
                out.volume = Some(v);
            }
        }
        if lattice.is_none() && line.to_uppercase().contains("DIRECT LATTICE VECTORS") {
            let mut rows = [[0.0f64; 3]; 3];
            let mut ok = true;
            for off in 0..3 {
                match lines.get(i + 1 + off) {
                    Some(l) => {
                        let p: Vec<f64> =
                            l.split_whitespace().filter_map(|s| s.parse().ok()).collect();
                        if p.len() < 3 {
                            ok = false;
                            break;
                        }
                        rows[off] = [p[0], p[1], p[2]];
                    }
                    None => {
                        ok = false;
                        break;
                    }
                }
            }
            if ok {
                lattice = Some(rows);
            }
        }
    }
    out.lattice = lattice;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_outcar_fields() {
        let txt = "\
 E-fermi :  3.4567
 some line
 FREE ENERGIE OF THE ION-ELECTRON SYSTEM (eV)
  energy  without entropy=    -128.3456 energy(sigma->0) =   -128.3456
  volume of cell :   123.4
  direct lattice vectors
    5.0  0.0  0.0
    0.0  5.0  0.0
    0.0  0.0  5.0
";
        let o = parse_outcar(txt).unwrap();
        assert_eq!(o.efermi, Some(3.4567));
        assert_eq!(o.total_energy, Some(-128.3456));
        assert_eq!(o.volume, Some(123.4));
        let lat = o.lattice.unwrap();
        assert_eq!(lat[0], [5.0, 0.0, 0.0]);
        assert_eq!(lat[2], [0.0, 0.0, 5.0]);
    }
}
