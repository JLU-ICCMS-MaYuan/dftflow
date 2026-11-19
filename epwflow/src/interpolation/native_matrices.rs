//! EPW 5.9 INTEGER(4)/REAL(8) little-endian 原生细网格适配。
use anyhow::{ensure, Context, Result};
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct State {
    /// 同源细网格的不可约 k 索引；band 在原生绑定时加上日志 ibndmin 偏移。
    pub kpoint: usize,
    pub band: usize,
    pub energy_ev: f64,
    pub weight: f64,
    #[serde(default)]
    pub k_crystal: Option<[f64; 3]>,
}

pub const FORMAT: &str = "epw5.9-le-i4-r8";
pub const RY_EV: f64 = 4.3597447222071e-18 / 1.602176634e-19 / 2.0;

pub fn band_offset(log: &str, bands: usize) -> Result<usize> {
    let bound = |name: &str| -> Result<usize> {
        let re = regex::Regex::new(&format!(
            r"(?m)^\s*{name}\s*=\s*(\d+)\s+ebnd(?:min|max)\s*="
        ))?;
        let values = re
            .captures_iter(log)
            .map(|c| c[1].parse::<usize>())
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ensure!(
            !values.is_empty() && values[0] > 0 && values.iter().all(|x| *x == values[0]),
            "data_invalid: Missing or conflicting native belt borders"
        );
        Ok(values[0])
    };
    let lo = bound("ibndmin")?;
    let hi = bound("ibndmax")?;
    ensure!(
        hi >= lo && hi - lo + 1 == bands,
        "data_invalid: egnv The number does not match the native tape window"
    );
    Ok(lo - 1)
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> Reader<'a> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N]> {
        let end = self
            .pos
            .checked_add(N)
            .context("data_invalid: Binary offset overflow")?;
        let raw = self
            .bytes
            .get(self.pos..end)
            .context("data_invalid: Interrupted native binary")?;
        self.pos = end;
        Ok(raw.try_into().unwrap())
    }
    fn int(&mut self) -> Result<usize> {
        let v = i32::from_le_bytes(self.take()?);
        ensure!(v >= 0, "data_invalid: Negative or not supported bytes for native integers");
        Ok(v as usize)
    }
    fn real(&mut self) -> Result<f64> {
        let v = f64::from_le_bytes(self.take()?);
        ensure!(v.is_finite(), "data_invalid: Originals are not finite");
        Ok(v)
    }
}

pub struct Eigenvalues {
    pub grid: [u32; 3],
    pub kpoints: usize,
    pub bands: usize,
    pub energies_ev: Vec<f64>,
    pub states: Vec<State>,
    pub fsthick_ev: f64,
    pub degaussw_ev: f64,
}

pub fn eigenvalues(bytes: &[u8]) -> Result<Eigenvalues> {
    let mut r = Reader { bytes, pos: 0 };
    let nk = r.int()?;
    let grid = [r.int()?, r.int()?, r.int()?];
    let nkfs = r.int()?;
    let nb = r.int()?;
    ensure!(
        nk > 0
            && nb > 0
            && nkfs > 0
            && nkfs <= nk
            && grid.iter().all(|g| *g > 0 && *g <= u32::MAX as usize),
        "data_invalid: egnv Grid/State Number Invalid"
    );
    let _ef = r.real()?;
    let ef0 = r.real()? * RY_EV;
    let dos = r.real()? / RY_EV;
    let smear = r.real()? * RY_EV;
    let shell = r.real()? * RY_EV;
    ensure!(
        dos > 0.0 && smear > 0.0 && shell > 0.0,
        "data_invalid: egnv Femi-noodle parameter invalid"
    );
    let stride = 40usize
        .checked_add(nb.checked_mul(8).context("data_invalid: egnv Size Spill")?)
        .context("data_invalid: egnv Size Spill")?;
    ensure!(
        nk.checked_mul(stride).and_then(|n| n.checked_add(r.pos)) == Some(bytes.len()),
        "data_invalid: egnv Length/INTEGER/REAL Layout Conflict"
    );
    let mut by_compressed = std::collections::BTreeMap::new();
    let mut all_energies = Vec::new();
    for ik in 1..=nk {
        let wk = r.real()?;
        let k_crystal = [r.real()?, r.real()?, r.real()?];
        ensure!(
            wk >= 0.0 && r.int()? == ik,
            "data_invalid: egnv k Index/weight conflict"
        );
        let ix = r.int()?;
        ensure!(ix <= nkfs, "data_invalid: egnv ikfs Crossing the border");
        let mut states = vec![];
        for ib in 1..=nb {
            let energy = r.real()? * RY_EV - ef0;
            all_energies.push(energy);
            if energy.abs() < shell {
                ensure!(ix != 0, "data_invalid: In the face of Ferami k Unmapped");
                let w = 0.5 * wk * (-(energy / smear).powi(2)).exp()
                    / (std::f64::consts::PI.sqrt() * smear * dos);
                states.push(State {
                    kpoint: ik,
                    band: ib,
                    energy_ev: energy,
                    weight: w,
                    k_crystal: Some(k_crystal),
                });
            }
        }
        if ix > 0 {
            ensure!(
                !states.is_empty() && by_compressed.insert(ix, states).is_none(),
                "data_invalid: Repeat / Empty ikfs Map"
            );
        }
    }
    ensure!(
        by_compressed.len() == nkfs && by_compressed.keys().copied().eq(1..=nkfs),
        "data_invalid: Missing ikfs Map"
    );
    Ok(Eigenvalues {
        grid: grid.map(|x| x as u32),
        kpoints: nk,
        bands: nb,
        energies_ev: all_energies,
        states: by_compressed.into_values().flatten().collect(),
        fsthick_ev: shell,
        degaussw_ev: smear,
    })
}

pub fn frequencies(bytes: &[u8], expected_grid: [u32; 3]) -> Result<Vec<f64>> {
    let mut r = Reader { bytes, pos: 0 };
    let nq = r.int()?;
    let grid = [r.int()?, r.int()?, r.int()?];
    let nm = r.int()?;
    let product = expected_grid
        .iter()
        .try_fold(1usize, |n, x| n.checked_mul(*x as usize))
        .context("data_invalid: q Grid overflowing")?;
    ensure!(
        grid == expected_grid.map(|x| x as usize) && nq == product && nm > 0,
        "data_invalid: freq Grid/Model Conflict"
    );
    let expected = nq
        .checked_mul(nm.checked_add(3).context("data_invalid: freq Spill")?)
        .and_then(|n| n.checked_mul(8))
        .and_then(|n| n.checked_add(r.pos));
    ensure!(
        expected == Some(bytes.len()),
        "data_invalid: freq Interdiction/redundant data/format mismatch"
    );
    let mut ws = vec![];
    for _ in 0..nq {
        for _ in 0..3 {
            r.real()?;
        }
        for _ in 0..nm {
            ws.push(r.real()? * RY_EV * 1000.0);
        }
    }
    Ok(ws)
}
