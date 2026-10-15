use anyhow::{bail, ensure, Context, Result};
use std::path::Path;
use super::super::foundation::config;

/// 晶胞：晶格、元素、数量、坐标（直接或笛卡尔）与赝势名。
#[derive(Debug, Clone)]
pub struct Cell {
    pub lattice: [[f64; 3]; 3],
    pub species: Vec<String>,
    pub counts: Vec<usize>,
    pub positions: Vec<[f64; 3]>,
    pub direct: bool,
}

/// 解析 VASP POSCAR/CONTCAR 为 QE 所需的晶胞描述；非正交晶格保留完整基底。
pub fn parse_poscar(path: &Path) -> Result<Cell> {
    let txt = std::fs::read_to_string(path)
        .with_context(|| format!("input_conflict: 无法读取结构 {}", path.display()))?;
    let lines: Vec<&str> = txt.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    ensure!(lines.len() >= 8, "data_invalid: POSCAR 行数不足");
    let scale: f64 = lines[1]
        .split_whitespace()
        .next()
        .context("data_invalid: POSCAR 缺少缩放因子")?
        .parse()
        .context("data_invalid: POSCAR 缩放因子非数值")?;
    let mut lattice = [[0.0f64; 3]; 3];
    for i in 0..3 {
        let v: Vec<f64> = lines[2 + i]
            .split_whitespace()
            .map(|x| x.parse())
            .collect::<std::result::Result<_, _>>()
            .context("data_invalid: POSCAR 晶格非数值")?;
        ensure!(v.len() == 3, "data_invalid: POSCAR 晶格必须有 3 分量");
        for j in 0..3 {
            lattice[i][j] = v[j] * scale;
        }
    }
    let species: Vec<String> = lines[5].split_whitespace().map(|s| s.to_string()).collect();
    let counts: Vec<usize> = lines[6]
        .split_whitespace()
        .map(|x| x.parse())
        .collect::<std::result::Result<_, _>>()
        .context("data_invalid: POSCAR 数量非整数")?;
    ensure!(species.len() == counts.len(), "data_invalid: POSCAR 元素与数量不匹配");
    let direct = lines[7]
        .chars()
        .next()
        .map(|c| c == 'D' || c == 'd' || c == 'K' || c == 'k')
        .unwrap_or(true);
    let mut pos_lines = Vec::new();
    for l in lines.iter().skip(8) {
        let v: Vec<f64> = l
            .split_whitespace()
            .map(|x| x.parse())
            .collect::<std::result::Result<_, _>>()
            .context("data_invalid: POSCAR 坐标非数值")?;
        if v.len() >= 3 {
            pos_lines.push([v[0], v[1], v[2]]);
        }
    }
    let total: usize = counts.iter().sum();
    ensure!(pos_lines.len() == total, "data_invalid: POSCAR 坐标数 {} 与元素总数 {} 不符", pos_lines.len(), total);
    Ok(Cell { lattice, species, counts, positions: pos_lines, direct })
}

/// 由 foundation Config 与已解析晶胞生成 QE 输入；统一受控生成器，不接受任意透传。
pub struct QeSettings<'a> {
    pub config: &'a config::Config,
    pub cell: Cell,
}

impl<'a> QeSettings<'a> {
    pub fn from_config(config: &'a config::Config) -> Result<Self> {
        let cell = parse_poscar(&config.structure)?;
        Ok(QeSettings { config, cell })
    }

    fn control(&self, calculation: &str, prefix: &str, outdir: &str) -> String {
        format!(
            "&control\n  calculation = '{calculation}'\n  prefix = '{prefix}'\n  outdir = '{outdir}'\n/\n"
        )
    }

    fn system(&self, nbnd: Option<usize>) -> String {
        let c = &self.config.qe;
        let nat = self.cell.positions.len();
        let ntyp = self.cell.species.len();
        let occ = match c.occupations {
            config::Occupations::Fixed => "fixed",
            config::Occupations::Smearing => "smearing",
        };
        let mut s = format!(
            "&system\n  ibrav = 0\n  nat = {nat}\n  ntyp = {ntyp}\n  ecutwfc = {}\n  ecutrho = {}\n  occupations = '{occ}'\n",
            c.ecutwfc_ry, c.ecutrho_ry
        );
        if let Some(d) = c.degauss_ry {
            s.push_str(&format!("  degauss = {d}\n"));
        }
        if let Some(sm) = &c.smearing {
            let sms = match sm {
                config::Smearing::Gaussian => "gaussian",
                config::Smearing::Mp => "mp",
                config::Smearing::Mv => "mv",
                config::Smearing::Fd => "fd",
            };
            s.push_str(&format!("  smearing = '{sms}'\n"));
        }
        if let Some(n) = nbnd {
            s.push_str(&format!("  nbnd = {n}\n"));
        }
        s.push_str("/\n");
        s
    }

    fn electrons(&self) -> String {
        format!("&electrons\n  conv_thr = {}\n/\n", self.config.qe.scf.conv_thr_ry)
    }

    fn atomic_species(&self) -> String {
        let mut s = String::from("ATOMIC_SPECIES\n");
        for el in &self.cell.species {
            let pseudo = self
                .config
                .pseudos
                .get(el)
                .cloned()
                .unwrap_or_else(|| format!("{el}.upf").into());
            let name = std::path::Path::new(&pseudo)
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_else(|| pseudo.to_string_lossy().into_owned());
            s.push_str(&format!("{el} 1.0 {name}\n"));
        }
        s
    }

    fn cell_params(&self) -> String {
        let mut s = String::from("CELL_PARAMETERS angstrom\n");
        for v in self.cell.lattice {
            s.push_str(&format!("  {} {} {}\n", v[0], v[1], v[2]));
        }
        s
    }

    fn atomic_positions(&self) -> String {
        let kind = if self.cell.direct { "crystal" } else { "angstrom" };
        let mut s = format!("ATOMIC_POSITIONS {kind}\n");
        let mut idx = 0usize;
        for (el, n) in self.cell.species.iter().zip(self.cell.counts.iter()) {
            for _ in 0..*n {
                let p = self.cell.positions[idx];
                s.push_str(&format!("{el}  {} {} {}\n", p[0], p[1], p[2]));
                idx += 1;
            }
        }
        s
    }

    fn kpoints_automatic(&self, grid: [u32; 3], shift: [u32; 3]) -> String {
        format!(
            "K_POINTS automatic\n  {} {} {}  {} {} {}\n",
            grid[0], grid[1], grid[2], shift[0], shift[1], shift[2]
        )
    }

    /// 渲染 SCF 输入；含完整晶胞与自动 k 网格。
    pub fn scf_input(&self, outdir: &str) -> String {
        let mut s = String::new();
        s.push_str(&self.control("scf", "scf", outdir));
        s.push_str(&self.system(None));
        s.push_str(&self.electrons());
        s.push_str(&self.atomic_species());
        s.push_str(&self.cell_params());
        s.push_str(&self.atomic_positions());
        s.push_str(&self.kpoints_automatic(self.config.qe.scf.k_grid, self.config.qe.scf.k_shift));
        s
    }

    /// 渲染 NSCF 输入：显式完整均匀 k 网格与带数。
    pub fn nscf_input(&self, outdir: &str) -> String {
        let mut s = String::new();
        s.push_str(&self.control("nscf", "nscf", outdir));
        s.push_str(&self.system(Some(self.config.qe.nscf.nbnd as usize)));
        s.push_str(&self.electrons());
        s.push_str(&self.atomic_species());
        s.push_str(&self.cell_params());
        s.push_str(&self.atomic_positions());
        s.push_str(&self.kpoints_automatic(self.config.qe.nscf.k_grid, [0, 0,0]));
        s
    }

    /// 渲染 PH 初始化输入（only_init）。
    pub fn phonon_init_input(&self, outdir: &str) -> String {
        let p = &self.config.phonon;
        format!(
            "&inputph\n  tr2_ph = {}\n  ldisp = .true.\n  only_init = .true.\n  lqdir = .true.\n  prefix = 'ph'\n  outdir = '{outdir}'\n/\n  {} {} {}\n",
            p.tr2_ph, p.q_grid[0], p.q_grid[1], p.q_grid[2]
        )
    }

    /// 渲染 PH nosplit 输入：单个任务计算完整 q 网格（FR-005）。
    pub fn phonon_nosplit_input(&self, outdir: &str) -> String {
        let p = &self.config.phonon;
        format!(
            "&inputph\n  tr2_ph = {}\n  ldisp = .true.\n  last_q = 0\n  lqdir = .true.\n  prefix = 'ph'\n  outdir = '{outdir}'\n/\n  {} {} {}\n",
            p.tr2_ph, p.q_grid[0], p.q_grid[1], p.q_grid[2]
        )
    }

    /// 渲染 PH 分 q 输入：index 用 ldisp+start_q=last_q；coords 用显式笛卡尔 q。
    pub fn phonon_q_input(
        &self,
        outdir: &str,
        _mode: &config::PhMode,
        q: Option<&super::manifest::QPoint>,
        q_index: usize,
    ) -> Result<String> {
        let p = &self.config.phonon;
        let header = format!(
            "&inputph\n  tr2_ph = {}\n  prefix = 'ph'\n  outdir = '{outdir}'\n",
            p.tr2_ph
        );
        let body = match p.mode {
            config::PhMode::Nosplit => "  ldisp = .true.\n  last_q = 0\n".to_string(),
            config::PhMode::Index => format!(
                "  ldisp = .true.\n  start_q = {q_index}\n  last_q = {q_index}\n"
            ),
            config::PhMode::Coords => match q {
                Some(qp) => format!(
                    "  ldisp = .false.\n  xq(1) = {:.10}\n  xq(2) = {:.10}\n  xq(3) = {:.10}\n",
                    qp.cartesian[0], qp.cartesian[1], qp.cartesian[2]
                ),
                None => bail!("data_invalid: coords 模式需要显式 q 坐标"),
            },
        };
        Ok(format!("{header}{body}/\n"))
    }
}
