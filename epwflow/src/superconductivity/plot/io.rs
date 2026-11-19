//! EPW 文本数据的流式解析；所有图中能量统一为 meV，保留能隙符号。
use anyhow::{bail, ensure, Context, Result};
use regex::Regex;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct A2fData {
    pub omega_mev: Vec<f64>,
    pub a2f: Vec<f64>,
    pub lambda_cumulative: Vec<f64>,
}
#[derive(Debug)]
pub struct GapData {
    pub temperature_k: f64,
    pub frequencies_mev: Vec<f64>,
    pub gaps_mev: Vec<Vec<f64>>,
}
#[derive(Debug)]
pub struct DosData {
    pub temperature_k: f64,
    pub omega_mev: Vec<f64>,
    pub dos_raw: Vec<f64>,
    /// EPW 原生 qdos 文件的第二列已经是 N_S/N_F；契约样本仍是未归一化 DOS。
    pub normalized: bool,
}
/// 原生 EPW 分布已经分箱，不得再对横坐标做无权直方图。
#[derive(Debug)]
pub enum Distribution {
    Samples { values: Vec<f64>, weights: Vec<f64> },
    Density { x: Vec<f64>, y: Vec<f64> },
}

fn rows(path: &Path, mut visit: impl FnMut(usize, &[f64]) -> Result<()>) -> Result<()> {
    let reader = BufReader::new(File::open(path).with_context(|| path.display().to_string())?);
    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        let text = line.split(['#', '!']).next().unwrap_or("").trim();
        if text.is_empty() {
            continue;
        }
        // 原生 EPW 的列头可能没有注释符；只跳过明确的列名行，不吞掉未知坏行。
        if index == 0
            && text.starts_with('w')
            && (text.contains("[eV]") || text.contains("N_S/N_F") || text.contains("znorm(w)"))
        {
            continue;
        }
        let values = text
            .split_whitespace()
            .map(|s| s.replace(['D', 'd'], "E").parse::<f64>())
            .collect::<std::result::Result<Vec<_>, _>>()
            .with_context(|| format!("{}:{}: invalid numeric row", path.display(), index + 1))?;
        ensure!(
            values.iter().all(|v| v.is_finite()),
            "{}:{}: non-finite value",
            path.display(),
            index + 1
        );
        visit(index + 1, &values).with_context(|| format!("{}:{}", path.display(), index + 1))?;
    }
    Ok(())
}
fn columns(row: &[f64], accepted: &[usize]) -> Result<()> {
    ensure!(
        accepted.contains(&row.len()),
        "column mismatch: expected {accepted:?}, got {}",
        row.len()
    );
    Ok(())
}
fn increasing(values: &[f64]) -> Result<()> {
    ensure!(
        values.windows(2).all(|p| p[1] > p[0]),
        "frequency/index must increase strictly"
    );
    Ok(())
}
/// 文件名最后一个下划线后的数值为温度（K）；拒绝非有限或非正温度。
pub fn temperature(path: &Path) -> Result<f64> {
    let name = path
        .file_name()
        .context("filename missing")?
        .to_string_lossy();
    let t: f64 = name
        .rsplit('_')
        .next()
        .context("temperature missing")?
        .parse()?;
    ensure!(t.is_finite() && t > 0., "invalid filename temperature");
    Ok(t)
}
/// 契约 1.1：一列 λ 或两列 λ/权重；空文件返回空数组，损坏数据报错。
pub fn parse_lambda_pairs(path: &Path) -> Result<Vec<f64>> {
    let mut values = vec![];
    rows(path, |_, row| {
        columns(row, &[1, 2])?;
        ensure!(row.iter().all(|v| *v >= 0.), "negative coupling/weight");
        values.push(row[0]);
        Ok(())
    })?;
    Ok(values)
}
/// 契约 1.2：两列索引/λ，按索引排序；拒绝重复、非整数索引及负 λ。
pub fn parse_lambda_k_pairs(path: &Path) -> Result<Vec<f64>> {
    let mut pairs = vec![];
    rows(path, |_, r| {
        columns(r, &[2])?;
        ensure!(
            r[0] >= 0. && r[0].fract() == 0. && r[1] >= 0.,
            "invalid index/coupling"
        );
        pairs.push((r[0], r[1]));
        Ok(())
    })?;
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
    increasing(&pairs.iter().map(|p| p.0).collect::<Vec<_>>())?;
    Ok(pairs.into_iter().map(|p| p.1).collect())
}
/// 兼容带 distribution 头的原生三列密度；契约两列样本保留权重。
pub fn parse_distribution(path: &Path, indexed: bool) -> Result<Distribution> {
    let header: String = BufReader::new(File::open(path)?)
        .lines()
        .take(2)
        .collect::<std::io::Result<Vec<_>>>()?
        .join("\n");
    if header.contains("distribution") {
        let (mut x, mut y) = (vec![], vec![]);
        rows(path, |_, r| {
            columns(r, &[3])?;
            ensure!(r.iter().all(|v| *v >= 0.), "negative density");
            x.push(r[0]);
            y.push(r[1]);
            Ok(())
        })?;
        increasing(&x)?;
        return Ok(Distribution::Density { x, y });
    }
    let values = if indexed {
        parse_lambda_k_pairs(path)?
    } else {
        parse_lambda_pairs(path)?
    };
    let mut weights = vec![];
    if indexed {
        weights.resize(values.len(), 1.);
    } else {
        rows(path, |_, r| {
            weights.push(*r.get(1).unwrap_or(&1.));
            Ok(())
        })?;
    }
    Ok(Distribution::Samples { values, weights })
}
/// 契约 1.3：第 1/2 列为 ω/α²F，第 12 列为累计 λ（不足时取最后一列）。
/// 只识别 EPW 的原生首行及固定尾注，不吞掉损坏的数值行。
pub fn parse_a2f(path: &Path) -> Result<A2fData> {
    let mut result = A2fData {
        omega_mev: vec![],
        a2f: vec![],
        lambda_cumulative: vec![],
    };
    let mut width = None;
    let mut footer = false;
    for (index, line) in BufReader::new(File::open(path)?).lines().enumerate() {
        let line = line?;
        let text = line.trim();
        if text.is_empty() || text.starts_with(['#', '!']) {
            continue;
        }
        if text.starts_with("w[meV]") {
            continue;
        }
        if text.starts_with("Integrated el-ph coupling") {
            footer = true;
            continue;
        }
        if footer {
            if [
                "Phonon smearing",
                "Electron smearing",
                "Fermi window",
                "Summed el-ph coupling",
            ]
            .iter()
            .any(|s| text.starts_with(s))
            {
                continue;
            }
            bail!("{}:{}: unexpected a2f footer", path.display(), index + 1);
        }
        let r = text
            .split_whitespace()
            .map(|s| s.replace(['D', 'd'], "E").parse::<f64>())
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ensure!(
            r.len() >= 3 && r.iter().all(|v| v.is_finite() && *v >= 0.),
            "invalid a2f row {}",
            index + 1
        );
        ensure!(width.is_none_or(|w| w == r.len()), "a2f column mismatch");
        width = Some(r.len());
        let lambda = r[(r.len() - 1).min(11)];
        // 原生输出会四舍五入，允许最后一位数值噪声。
        ensure!(
            result
                .lambda_cumulative
                .last()
                .is_none_or(|last| lambda + 1e-7 >= *last),
            "decreasing cumulative lambda"
        );
        result.omega_mev.push(r[0]);
        result.a2f.push(r[1]);
        result.lambda_cumulative.push(lambda);
    }
    ensure!(!result.omega_mev.is_empty(), "empty a2f");
    increasing(&result.omega_mev)?;
    Ok(result)
}
fn parse_imag(path: &Path, iso: bool) -> Result<GapData> {
    let mut result = GapData {
        temperature_k: temperature(path)?,
        frequencies_mev: vec![],
        gaps_mev: vec![],
    };
    rows(path, |_, r| {
        columns(r, if iso { &[3] } else { &[4] })?;
        ensure!(r[0] > 0., "non-positive Matsubara frequency");
        let w = r[0] * 1000.;
        if result.frequencies_mev.last() != Some(&w) {
            result.frequencies_mev.push(w);
            result.gaps_mev.push(vec![]);
        }
        result
            .gaps_mev
            .last_mut()
            .unwrap()
            .push(r[r.len() - 1] * 1000.);
        Ok(())
    })?;
    ensure!(!result.frequencies_mev.is_empty(), "empty gap data");
    increasing(&result.frequencies_mev)?;
    let expected = std::f64::consts::PI * 8.617333262e-2 * result.temperature_k;
    ensure!(
        (result.frequencies_mev[0] - expected).abs() <= expected * 1e-5,
        "temperature mismatch: filename {} K, first Matsubara {} meV",
        result.temperature_k,
        result.frequencies_mev[0]
    );
    let states = result.gaps_mev[0].len();
    ensure!(
        result.gaps_mev.iter().all(|r| r.len() == states),
        "inconsistent states per frequency"
    );
    Ok(result)
}
/// 契约 1.4：四列 iω/E/Z/Δ（eV），按频率分组，保留状态顺序和能隙符号。
pub fn parse_imag_aniso(path: &Path) -> Result<GapData> {
    parse_imag(path, false)
}
/// 原生 iso 为三列 iω/Z/Δ（eV），不将 Z 误当能隙。
pub fn parse_imag_iso(path: &Path) -> Result<GapData> {
    parse_imag(path, true)
}
/// 契约 1.5：五列 ω/ReZ/ImZ/ReΔ/ImΔ；原生六列额外包含第二列 E。
/// 超过 10 MB 时先流式计数，再按固定步长抽样；所有行仍检查列数和有限值。
/// 上限 10000 点，避免行宽估计偏差导致超额；不在内存中缓存整个文件。
fn parse_pade(path: &Path, accepted_columns: &[usize]) -> Result<GapData> {
    let mut step = 1;
    if path.metadata()?.len() > 10 * 1024 * 1024 {
        let mut count = 0usize;
        for line in BufReader::new(File::open(path)?).lines() {
            let line = line?;
            let text = line.trim();
            if !text.is_empty() && !text.starts_with(['#', '!']) {
                count += 1;
            }
        }
        step = (count / 10000).max(1);
        if count.div_ceil(step) > 10000 {
            step += 1;
        }
    }
    let mut result = GapData {
        temperature_k: temperature(path)?,
        frequencies_mev: vec![],
        gaps_mev: vec![],
    };
    let mut index = 0;
    let mut width = None;
    rows(path, |_, r| {
        columns(r, accepted_columns)?;
        ensure!(width.is_none_or(|w| w == r.len()), "pade column mismatch");
        width = Some(r.len());
        if index % step == 0 {
            result.frequencies_mev.push(r[0] * 1000.);
            result.gaps_mev.push(vec![r[r.len() - 2] * 1000.]);
        }
        index += 1;
        Ok(())
    })?;
    ensure!(!result.gaps_mev.is_empty(), "empty pade data");
    Ok(result)
}
/// 各向异性 Padé：原生 EPW 六列，取第 5 列实部能隙。
pub fn parse_pade_aniso(path: &Path) -> Result<GapData> {
    parse_pade(path, &[5, 6])
}
/// 各向同性 Padé：原生 EPW 五列，取第 4 列实部能隙。
pub fn parse_pade_iso(path: &Path) -> Result<GapData> {
    parse_pade(path, &[5])
}
/// 契约 1.6：四列虚频数据取首频率全部状态，按温度排序；分布 gap0 不伪装成状态。
pub fn extract_leading_gaps(paths: &[(f64, PathBuf)]) -> Result<Vec<(f64, Vec<f64>)>> {
    let mut result = vec![];
    for (t, path) in paths {
        let gap = parse_imag_aniso(path)?;
        ensure!(
            (*t - gap.temperature_k).abs() < 1e-6,
            "temperature mismatch in requested series"
        );
        result.push((*t, gap.gaps_mev.into_iter().next().unwrap()));
    }
    result.sort_by(|a, b| a.0.total_cmp(&b.0));
    increasing(&result.iter().map(|p| p.0).collect::<Vec<_>>())?;
    Ok(result)
}
/// 原生 `imag_aniso_gap0_*` 五列分布：第 1 列是温度加分布横向偏移，第 2 列是 Δ_nk(meV)。
/// 这正是 Mori Fig. 8 的竖向分布曲线；不能将其转换成少量 k 点状态。
pub fn parse_gap0_distribution(path: &Path) -> Result<Vec<(f64, f64)>> {
    let t = temperature(path)?;
    let mut points = Vec::new();
    rows(path, |_, r| {
        columns(r, &[5])?;
        ensure!(
            (r[2] - t).abs() < 1e-6,
            "gap0 filename temperature mismatch"
        );
        ensure!(
            r[3] >= 0. && r[3] <= 1. + 1e-8 && r[4] >= 0.,
            "invalid gap0 density"
        );
        ensure!(
            (r[0] - r[2] - r[3]).abs() < 1e-6,
            "invalid gap0 temperature offset"
        );
        points.push((r[0], r[1]));
        Ok(())
    })?;
    ensure!(!points.is_empty(), "empty gap0 distribution");
    Ok(points)
}
/// 契约 1.7：第 1 列 meV、第 2 列未归一化 DOS；带明确 [eV] 头时转换单位。
pub fn parse_qdos(path: &Path) -> Result<DosData> {
    let mut result = DosData {
        temperature_k: temperature(path)?,
        omega_mev: vec![],
        dos_raw: vec![],
        normalized: false,
    };
    let header = BufReader::new(File::open(path)?)
        .lines()
        .next()
        .transpose()?
        .unwrap_or_default();
    let factor = if header.contains("[eV]") || header.contains("(eV)") {
        1000.
    } else {
        1.
    };
    result.normalized = header.contains("N_S/N_F");
    rows(path, |_, r| {
        ensure!(r.len() >= 2 && r[1] >= 0., "invalid DOS columns/value");
        result.omega_mev.push(r[0] * factor);
        result.dos_raw.push(r[1]);
        Ok(())
    })?;
    ensure!(!result.omega_mev.is_empty(), "empty DOS");
    increasing(&result.omega_mev)?;
    Ok(result)
}
/// 契约 1.8：识别两种 DOS 输出，接受 Fortran 指数；失败由调用者警告并回退到 1。
pub fn extract_fermi_dos(path: &Path) -> Result<f64> {
    let re = Regex::new(r"(?i)DOS(?:\(states/spin/eV/Unit\s+Cell\))?\s*=\s*([+\-0-9.eEdD]+)")?;
    for line in BufReader::new(File::open(path)?).lines() {
        let line = line?;
        if let Some(c) = re.captures(&line) {
            let value: f64 = c[1].replace(['D', 'd'], "E").parse()?;
            ensure!(value.is_finite() && value > 0., "invalid Fermi DOS");
            return Ok(value);
        }
    }
    bail!("Fermi DOS missing in {}", path.display())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufWriter, Write};
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Fixture(PathBuf);
    impl Fixture {
        fn new(name: &str, text: &str) -> Self {
            let dir = std::env::current_dir()
                .unwrap()
                .join("bin/.build/plot-tests")
                .join(format!(
                    "{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
            std::fs::create_dir_all(&dir).unwrap();
            let p = dir.join(name);
            std::fs::write(&p, text).unwrap();
            Self(p)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(self.0.parent().unwrap());
        }
    }
    #[test]
    fn lambda_values_weights_and_empty() {
        let f = Fixture::new("pairs", "# header\n0.1\n2D0 3\n");
        assert_eq!(parse_lambda_pairs(&f.0).unwrap(), [0.1, 2.]);
        let d = parse_distribution(&f.0, false).unwrap();
        assert!(matches!(d,Distribution::Samples{weights,..} if weights==[1.,3.]));
        let f = Fixture::new("empty", "");
        assert!(parse_lambda_pairs(&f.0).unwrap().is_empty());
    }
    #[test]
    fn lambda_rejects_invalid() {
        for text in ["-1", "1 -2", "1 2 3", "NaN", "Inf", "bad"] {
            let f = Fixture::new("p", text);
            assert!(parse_lambda_pairs(&f.0).is_err(), "{text}");
        }
        assert!(parse_lambda_pairs(Path::new("missing-file-76")).is_err());
    }
    #[test]
    fn indexed_lambda_sorts_and_rejects_duplicates() {
        let f = Fixture::new("p", "2 0.5\n1 0.2\n");
        assert_eq!(parse_lambda_k_pairs(&f.0).unwrap(), [0.2, 0.5]);
        assert!(
            matches!(parse_distribution(&f.0,true).unwrap(),Distribution::Samples{weights,..} if weights==[1.,1.])
        );
        for text in ["1 0.1\n1 0.2", "0.5 1", "1 -1", "1"] {
            let f = Fixture::new("p", text);
            assert!(parse_lambda_k_pairs(&f.0).is_err());
        }
    }
    #[test]
    fn native_distribution_preserves_density() {
        let f = Fixture::new(
            "p",
            "# distribution = rho(lambda)\n# scaled\n0 0.2 20\n1 0.7 70\n",
        );
        assert!(
            matches!(parse_distribution(&f.0,true).unwrap(),Distribution::Density{x,y} if x==[0.,1.] && y==[0.2,0.7])
        );
        let f = Fixture::new("p", "# distribution\n0 -1 0");
        assert!(parse_distribution(&f.0, false).is_err());
    }
    #[test]
    fn a2f_uses_contract_column_twelve_and_native_footer() {
        let f = Fixture::new(
            "a2f",
            "# ω a2f\n0 1 2 3 4 5 6 7 8 9 10 11 12 13\n1 1 2 3 4 5 6 7 8 9 10 12 14 15\n",
        );
        assert_eq!(parse_a2f(&f.0).unwrap().lambda_cumulative, [11., 12.]);
        let f=Fixture::new("a2f","w[meV] a2f and integrated 2*a2f/w\n1 0.2 0.3\n2 0.4 0.7\nIntegrated el-ph coupling\n# 0.7\nPhonon smearing (meV)\n# 0.5\nElectron smearing (eV) 0.1\nFermi window (eV) 0.4\nSummed el-ph coupling 0.7\n");
        let d = parse_a2f(&f.0).unwrap();
        assert_eq!(d.a2f, [0.2, 0.4]);
        assert_eq!(d.lambda_cumulative, [0.3, 0.7]);
    }
    #[test]
    fn a2f_rejects_bad_columns_values_order_and_footer() {
        for text in [
            "",
            "1 2",
            "1 2 3\n2 3 4 5",
            "1 NaN 3",
            "1 -1 2",
            "2 1 2\n1 1 3",
            "1 1 3\n2 1 2",
            "1 2 bad",
            "1 2 3\nIntegrated el-ph coupling\nwrong",
        ] {
            let f = Fixture::new("a2f", text);
            assert!(parse_a2f(&f.0).is_err(), "{text}");
        }
    }
    fn gap_text(iso: bool, t: f64, g: f64) -> String {
        let w = std::f64::consts::PI * 8.617333262e-5 * t;
        if iso {
            format!("{w} 2 {g}\n{} 2 {}\n", 3. * w, g / 2.)
        } else {
            format!(
                "{w} 0 2 {g}\n{w} 1 2 {}\n{} 0 2 {}\n{} 1 2 {}\n",
                -g,
                3. * w,
                g / 2.,
                3. * w,
                -g / 2.
            )
        }
    }
    #[test]
    fn imaginary_groups_converts_and_keeps_signs() {
        let f = Fixture::new("MgB2.imag_aniso_020.00", &gap_text(false, 20., 0.01));
        let g = parse_imag_aniso(&f.0).unwrap();
        assert_eq!(g.gaps_mev, [vec![10., -10.], vec![5., -5.]]);
        assert!((g.frequencies_mev[0] - 5.41443).abs() < 1e-5);
        let f = Fixture::new(
            "MgB2.imag_iso_020.00",
            &format!(
                "w [eV] znorm(w) delta(w) [eV]\n{}",
                gap_text(true, 20., 0.008)
            ),
        );
        assert_eq!(parse_imag_iso(&f.0).unwrap().gaps_mev[0], [8.]);
    }
    #[test]
    fn imaginary_rejects_mismatches() {
        for text in [
            "",
            "0 0 1 0.01",
            "0.001 0 1 0.01",
            "0.0054144301739 0 1 NaN",
            "0.0054144301739 0 1",
            "0.0054144301739 0 1 0.01\n0.0054144301739 0 1 0.02\n0.016243290522 0 1 0.01",
            "0.0054144301739 0 1 0.01\n0.016243290522 0 1 0.01\n0.0054144301739 0 1 0.01",
        ] {
            let f = Fixture::new("gap_020.00", text);
            assert!(parse_imag_aniso(&f.0).is_err(), "{text}");
        }
        for name in ["bad", "gap_NaN", "gap_-2", "gap_0"] {
            let f = Fixture::new(name, "");
            assert!(temperature(&f.0).is_err());
        }
    }
    #[test]
    fn leading_gaps_sorts_and_validates() {
        let f = Fixture::new("gap_020.00", &gap_text(false, 20., 0.01));
        let g = Fixture::new("gap_030.00", &gap_text(false, 30., 0.005));
        let r = extract_leading_gaps(&[(30., g.0.clone()), (20., f.0.clone())]).unwrap();
        assert_eq!(r[0], (20., vec![10., -10.]));
        assert!(extract_leading_gaps(&[(21., f.0.clone())]).is_err());
        assert!(extract_leading_gaps(&[(20., f.0.clone()), (20., f.0.clone())]).is_err());
        assert!(extract_leading_gaps(&[]).unwrap().is_empty());
    }
    #[test]
    fn pade_contract_and_native_real_gap_columns() {
        for (text, expected) in [("0 1 0 -0.01 0.02", -10.), ("0 -0.2 1 0 0.03 0.04", 30.)] {
            let f = Fixture::new("pade_010.00", text);
            let d = parse_pade_aniso(&f.0).unwrap();
            assert_eq!(d.gaps_mev[0], [expected]);
        }
        for text in ["", "0 1 2 3", "0 1 0 NaN 0", "0 1 0 1 0\n0 1 0 1 0 1"] {
            let f = Fixture::new("pade_010.00", text);
            assert!(parse_pade_aniso(&f.0).is_err());
        }
    }
    #[test]
    fn pade_seventy_mb_is_bounded_and_checks_unsampled_rows() {
        let f = Fixture::new("pade_010.00", "");
        let mut writer = BufWriter::new(File::create(&f.0).unwrap());
        let row = format!("{:<199}\n", "0.001 1.0 0.0 0.01 0.0");
        for _ in 0..350_000 {
            writer.write_all(row.as_bytes()).unwrap();
        }
        writer.flush().unwrap();
        assert_eq!(f.0.metadata().unwrap().len(), 70_000_000);
        let d = parse_pade_aniso(&f.0).unwrap();
        assert_eq!(d.frequencies_mev.len(), 10_000);
        assert_eq!(d.gaps_mev[0], [10.]);
        let mut append = std::fs::OpenOptions::new().append(true).open(&f.0).unwrap();
        writeln!(append, "0 1 0 NaN 0").unwrap();
        assert!(parse_pade_aniso(&f.0).is_err());
    }
    #[test]
    fn qdos_units_columns_and_errors() {
        let f = Fixture::new("qdos_010.00", "# meV\n-1 2\n0 3\n1 4\n");
        let d = parse_qdos(&f.0).unwrap();
        assert_eq!(d.omega_mev, [-1., 0., 1.]);
        assert_eq!(d.dos_raw, [2., 3., 4.]);
        let f = Fixture::new("qdos_010.00", "# w [eV]\n-0.01 2\n0.01 3\n");
        assert_eq!(parse_qdos(&f.0).unwrap().omega_mev, [-10., 10.]);
        for text in ["", "1", "1 -2", "1 NaN", "1 2\n0 2"] {
            let f = Fixture::new("qdos_010.00", text);
            assert!(parse_qdos(&f.0).is_err());
        }
    }
    #[test]
    fn fermi_dos_multiple_formats_and_invalid() {
        for text in [
            "DOS(states/spin/eV/Unit Cell) = 2.5",
            "Fermi level (eV) = 5.1 DOS = 2.5D+00",
        ] {
            let f = Fixture::new("epw1.out", text);
            assert_eq!(extract_fermi_dos(&f.0).unwrap(), 2.5);
        }
        for text in ["", "DOS = 0", "DOS = -1", "DOS = 1e400"] {
            let f = Fixture::new("epw1.out", text);
            assert!(extract_fermi_dos(&f.0).is_err());
        }
    }
}
