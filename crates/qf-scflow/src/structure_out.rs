//! ph.out 解析出的结构（轻量版，用于体积/密度/化学式与 POSCAR/CIF 写出）。

use anyhow::Result;

#[derive(Debug, Clone)]
pub struct StructureOut {
    pub lattice: [[f64; 3]; 3],
    pub species: Vec<String>,
    /// 笛卡尔坐标（Å）。
    pub coords: Vec<[f64; 3]>,
}

fn symbol_to_z(symbol: &str) -> Option<usize> {
    // 数字后缀（如 Mg2）剥离。
    let base: String = symbol.chars().take_while(|c| c.is_alphabetic()).collect();
    let base = if base.is_empty() { symbol } else { base.as_str() };
    qf_crystal::elements::symbol_to_atomic_number(base)
}

impl StructureOut {
    /// 元素首次出现顺序与计数。
    pub fn element_counts(&self) -> Vec<(String, usize)> {
        let mut counts: Vec<(String, usize)> = Vec::new();
        for s in &self.species {
            let base: String = s.chars().take_while(|c| c.is_alphabetic()).collect();
            let base = if base.is_empty() { s.clone() } else { base };
            match counts.iter_mut().find(|(e, _)| *e == base) {
                Some((_, c)) => *c += 1,
                None => counts.push((base, 1)),
            }
        }
        counts
    }

    pub fn gcd(counts: &[usize]) -> usize {
        let mut g = 0usize;
        for c in counts {
            g = if g == 0 { *c } else { gcd(g, *c) };
        }
        g.max(1)
    }

    /// Python `parse_formula(structure)`：先 `structure.formula`（首现顺序 + 数量），
    /// 再按 gcd 缩减重建（数量 1 省略）。
    pub fn formula(&self, get_gcd: bool) -> (String, usize) {
        let element_counts = self.element_counts();
        let counts: Vec<usize> = element_counts.iter().map(|(_, c)| *c).collect();
        let fu = Self::gcd(&counts);
        let mut result = String::new();
        for (i, (element, count)) in element_counts.iter().enumerate() {
            result.push_str(element);
            if i != counts.len() {
                let reduced = count / fu;
                if reduced != 1 {
                    result.push_str(&reduced.to_string());
                }
            }
        }
        let _ = get_gcd;
        (result, fu)
    }

    /// 体积（Å³）。
    pub fn volume(&self) -> f64 {
        let l = &self.lattice;
        l[0][0] * (l[1][1] * l[2][2] - l[1][2] * l[2][1])
            - l[0][1] * (l[1][0] * l[2][2] - l[1][2] * l[2][0])
            + l[0][2] * (l[1][0] * l[2][1] - l[1][1] * l[2][0])
    }

    /// 密度（g/cm³）。
    pub fn density(&self) -> Result<f64> {
        let mut mass = 0.0_f64;
        for (element, count) in self.element_counts() {
            mass += qf_crystal::atomic_mass(&element)
                .ok_or_else(|| anyhow::anyhow!("未知元素质量: {element}"))?
                * count as f64;
        }
        // pint/pymatgen 的 amu→g 换算常数（CODATA：1 amu = 1.66053906892e-24 g）
        Ok(mass / self.volume() * 1.66053906892e-24 * 1e24)
    }

    /// pymatgen `Poscar(struct).write_file` 等价写出：
    /// 注释 = `structure.formula`（未约化、首现顺序）、晶格/坐标 `{:21.16f}` 右对齐、
    /// "direct" 小写、坐标行尾附物种符号。
    pub fn write_vasp(&self, path: &std::path::Path) -> Result<()> {
        let element_counts = self.element_counts();
        let elements: Vec<String> = element_counts.iter().map(|(e, _)| e.clone()).collect();
        let counts: Vec<usize> = element_counts.iter().map(|(_, c)| *c).collect();
        // 分数坐标
        let det = self.lattice[0][0]
            * (self.lattice[1][1] * self.lattice[2][2] - self.lattice[1][2] * self.lattice[2][1])
            - self.lattice[0][1]
                * (self.lattice[1][0] * self.lattice[2][2] - self.lattice[1][2] * self.lattice[2][0])
            + self.lattice[0][2]
                * (self.lattice[1][0] * self.lattice[2][1] - self.lattice[1][1] * self.lattice[2][0]);
        let mut fract = Vec::with_capacity(self.coords.len());
        if det.abs() > 1e-12 {
            let inv = [
                [
                    (self.lattice[1][1] * self.lattice[2][2] - self.lattice[1][2] * self.lattice[2][1]) / det,
                    (self.lattice[0][2] * self.lattice[2][1] - self.lattice[0][1] * self.lattice[2][2]) / det,
                    (self.lattice[0][1] * self.lattice[1][2] - self.lattice[0][2] * self.lattice[1][1]) / det,
                ],
                [
                    (self.lattice[1][2] * self.lattice[2][0] - self.lattice[1][0] * self.lattice[2][2]) / det,
                    (self.lattice[0][0] * self.lattice[2][2] - self.lattice[0][2] * self.lattice[2][0]) / det,
                    (self.lattice[0][2] * self.lattice[1][0] - self.lattice[0][0] * self.lattice[1][2]) / det,
                ],
                [
                    (self.lattice[1][0] * self.lattice[2][1] - self.lattice[1][1] * self.lattice[2][0]) / det,
                    (self.lattice[0][1] * self.lattice[2][0] - self.lattice[0][0] * self.lattice[2][1]) / det,
                    (self.lattice[0][0] * self.lattice[1][1] - self.lattice[0][1] * self.lattice[1][0]) / det,
                ],
            ];
            for p in &self.coords {
                // 行向量约定：fract = cart · M⁻¹
                fract.push([
                    inv[0][0] * p[0] + inv[1][0] * p[1] + inv[2][0] * p[2],
                    inv[0][1] * p[0] + inv[1][1] * p[1] + inv[2][1] * p[2],
                    inv[0][2] * p[0] + inv[1][2] * p[1] + inv[2][2] * p[2],
                ]);
            }
        }
        // pymatgen comment：structure.formula（未约化，数量 1 也写出）
        let mut comment = String::new();
        for (element, count) in &element_counts {
            comment.push_str(element);
            comment.push_str(&count.to_string());
            comment.push(' ');
        }
        let comment = comment.trim_end().to_string();
        // 逐原子物种（坐标行尾标注）
        let mut site_species = Vec::new();
        for (element, count) in &element_counts {
            for _ in 0..*count {
                site_species.push(element.clone());
            }
        }
        let mut text = String::new();
        text.push_str(&format!("{comment}\n1.0\n"));
        for row in &self.lattice {
            let fields: Vec<String> = row.iter().map(|v| format!("{v:>21.16}")).collect();
            text.push_str(&fields.join(" "));
            text.push('\n');
        }
        text.push_str(&elements.join(" "));
        text.push('\n');
        text.push_str(&counts.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(" "));
        text.push('\n');
        text.push_str("direct\n");
        for (p, species) in fract.iter().zip(&site_species) {
            let fields: Vec<String> = p.iter().map(|v| format!("{v:>21.16}")).collect();
            text.push_str(&format!("{} {species}\n", fields.join(" ")));
        }
        std::fs::write(path, text)?;
        Ok(())
    }

    /// 交给 qf-crystal 做对称化分析。
    pub fn to_structure(&self) -> qf_crystal::Structure {
        let element_counts = self.element_counts();
        let elements: Vec<String> = element_counts.iter().map(|(e, _)| e.clone()).collect();
        let counts: Vec<usize> = element_counts.iter().map(|(_, c)| *c).collect();
        let (formula, _) = self.formula(false);
        qf_crystal::Structure {
            formula,
            elements,
            counts,
            lattice: self.lattice,
            positions: self.coords.clone(),
            coord_type: "angstrom".to_string(),
        }
    }
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 { a } else { gcd(b, a % b) }
}

