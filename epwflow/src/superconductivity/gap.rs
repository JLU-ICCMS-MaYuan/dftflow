//! EPW 5.9 FSR 虚频轴完整分布；不从 Tc 或平均能隙构造数据。
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const KB_EV_K: f64 = 1.380649e-23 / 1.602176634e-19;

pub use crate::interpolation::native_matrices::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GapData {
    pub temperature_centikelvin: i64,
    pub frequencies_mev: Vec<f64>,
    /// aniso 顺序来自 egnv 的 ikfs、压缩带映射；iso 无状态索引。
    pub states: Vec<State>,
    /// 频率优先、状态次之，保留符号。
    pub delta_mev: Vec<f64>,
    pub z: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statistics {
    pub signed_mean_mev: f64,
    pub absolute_mean_mev: f64,
    pub rms_mev: f64,
    pub min_absolute_mev: f64,
    pub max_absolute_mev: f64,
    pub weight_sum: f64,
}

impl GapData {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.temperature_centikelvin > 0, "data_invalid: invalid temperature");
        let ns = self.states.len().max(1);
        let nf = self.frequencies_mev.len();
        ensure!(
            nf > 0
                && nf.checked_mul(ns) == Some(self.delta_mev.len())
                && self.z.len() == self.delta_mev.len(),
            "data_invalid: Delta/Z records missing or lengths inconsistent"
        );
        ensure!(
            self.delta_mev.iter().chain(&self.z).all(|x| x.is_finite()),
            "data_invalid: nonfinite Delta/Z"
        );
        ensure!(
            self.z.iter().all(|x| *x > 0.0),
            "data_invalid: FSR Z must be positive"
        );
        for (i, w) in self.frequencies_mev.iter().enumerate() {
            let expected = (2 * i + 1) as f64
                * std::f64::consts::PI
                * KB_EV_K
                * self.temperature_centikelvin as f64
                * 10.0;
            ensure!(
                w.is_finite() && (*w - expected).abs() <= 1e-8 + expected.abs() * 1e-9,
                "data_invalid: Matsubara frequency conflicts with actual temperature/index"
            );
        }
        let mut seen = BTreeSet::new();
        for s in &self.states {
            ensure!(
                s.kpoint > 0 && s.band > 0 && seen.insert((s.kpoint, s.band)),
                "data_invalid: duplicate or invalid k/band mapping"
            );
            ensure!(
                s.energy_ev.is_finite() && s.weight.is_finite() && s.weight >= 0.0,
                "data_invalid: invalid state energy/weight"
            );
            ensure!(
                s.k_crystal.is_none_or(|k| k.iter().all(|x| x.is_finite())),
                "data_invalid: nonfinite k coordinates"
            );
        }
        let weight_sum = self.states.iter().map(|s| s.weight).sum::<f64>();
        ensure!(
            self.states.is_empty() || (weight_sum.is_finite() && weight_sum > 0.0),
            "data_invalid: Fermi-surface weights are all zero or overflow"
        );
        Ok(())
    }

    pub fn d_inf(&self) -> f64 {
        self.delta_mev.iter().map(|x| x.abs()).fold(0.0, f64::max)
    }

    pub fn d0_inf(&self) -> f64 {
        self.delta_mev[..self.states.len().max(1)]
            .iter()
            .map(|x| x.abs())
            .fold(0.0, f64::max)
    }

    pub fn band_statistics(&self) -> Result<BTreeMap<usize, Statistics>> {
        self.validate()?;
        let mut grouped: BTreeMap<usize, Vec<(f64, f64)>> = BTreeMap::new();
        if self.states.is_empty() {
            grouped.insert(0, vec![(self.delta_mev[0], 1.0)]);
        } else {
            for (s, d) in self.states.iter().zip(&self.delta_mev) {
                grouped.entry(s.band).or_default().push((*d, s.weight));
            }
        }
        grouped
            .into_iter()
            .map(|(band, xs)| {
                let w = xs.iter().map(|(_, w)| w).sum::<f64>();
                ensure!(w > 0.0, "data_invalid: a band has zero Fermi-surface weight; averages cannot be published");
                Ok((
                    band,
                    Statistics {
                        signed_mean_mev: xs.iter().map(|(d, w)| d * w).sum::<f64>() / w,
                        absolute_mean_mev: xs.iter().map(|(d, w)| d.abs() * w).sum::<f64>() / w,
                        rms_mev: (xs.iter().map(|(d, w)| d * d * w).sum::<f64>() / w).sqrt(),
                        min_absolute_mev: xs
                            .iter()
                            .map(|(d, _)| d.abs())
                            .fold(f64::INFINITY, f64::min),
                        max_absolute_mev: xs.iter().map(|(d, _)| d.abs()).fold(0.0, f64::max),
                        weight_sum: w,
                    },
                ))
            })
            .collect()
    }
}

/// 文件没有 k/band 列，必须由同源 egnv 恢复顺序，逐行复核能量和频率。
pub fn parse_native(
    text: &str,
    temperature: i64,
    nfreq: usize,
    states: Vec<State>,
) -> Result<GapData> {
    let mut lines = text.lines();
    let header = lines
        .next()
        .unwrap_or("")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let aniso = !states.is_empty();
    let expected_header = if aniso {
        "# w [eV] Enk-Ef [eV] znorm(w) delta(w) [eV]"
    } else {
        "w [eV] znorm(w) delta(w) [eV]"
    };
    ensure!(
        header == expected_header,
        "data_invalid: columns/units do not match EPW 5.9 FSR imaginary-axis output"
    );
    let mut result = GapData {
        temperature_centikelvin: temperature,
        frequencies_mev: vec![],
        states,
        delta_mev: vec![],
        z: vec![],
    };
    let ns = result.states.len().max(1);
    let expected = nfreq
        .checked_mul(ns)
        .ok_or_else(|| anyhow::anyhow!("data_invalid: array overflow"))?;
    ensure!(expected > 0, "data_invalid: empty frequencies or states");
    for (row, line) in lines.filter(|l| !l.trim().is_empty()).enumerate() {
        ensure!(row < expected, "data_invalid: native gap output contains extra records");
        let xs = super::parse::numbers(line)?;
        ensure!(
            xs.len() == if aniso { 4 } else { 3 },
            "data_invalid: native Delta/Z column count mismatch"
        );
        let iw = row / ns;
        if row % ns == 0 {
            result.frequencies_mev.push(xs[0] * 1000.0);
        }
        ensure!(
            (xs[0] * 1000.0 - result.frequencies_mev[iw]).abs() <= 1e-8,
            "data_invalid: frequency-block alignment mismatch"
        );
        let offset = usize::from(aniso);
        if aniso {
            ensure!(
                (xs[1] - result.states[row % ns].energy_ev).abs() <= 1e-9,
                "data_invalid: egnv differs from gap-state energies"
            );
        }
        result.z.push(xs[1 + offset]);
        result.delta_mev.push(xs[2 + offset] * 1000.0);
    }
    ensure!(
        result.delta_mev.len() == expected,
        "data_invalid: truncated native gap output"
    );
    result.validate()?;
    Ok(result)
}
