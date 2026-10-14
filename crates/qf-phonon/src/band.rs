//! 声子表格与绘图共用的路径数据；端点标签与逐点频率分离。
use anyhow::{ensure, Result};
use qf_io::{read_band_frequencies, read_labelinfo, LabelInfo};
use std::{ops::Range, path::Path};

#[derive(Debug)]
pub struct BandData {
    pub labels: Vec<LabelInfo>,
    pub points: Vec<LabelInfo>,
    pub frequencies: Vec<Vec<f64>>,
    pub segments: Vec<Range<usize>>,
    pub ticks: Vec<(f64, String)>,
}

pub(crate) fn archived_file(workspace: &str, folder: &str, name: &str) -> String {
    let archived = format!("{workspace}/{folder}/{name}");
    if Path::new(&archived).exists() {
        archived
    } else {
        format!("{workspace}/{name}")
    }
}

impl BandData {
    pub fn load(workspace: &str, prefix: &str) -> Result<Self> {
        let labels = std::fs::read_to_string(archived_file(
            workspace,
            "phband",
            &format!("{prefix}_band.labelinfo.dat"),
        ))?;
        let text = std::fs::read_to_string(archived_file(
            workspace,
            "phband",
            &qf_input::filenames::derive_filenames(prefix).flfrq,
        ))?;
        let invalid = regex::Regex::new(r"(?i)\b(?:nan|inf|infinity)\b")?;
        ensure!(
            !invalid.is_match(&text) && !invalid.is_match(&labels),
            "声子数据包含非有限数值"
        );
        let modes = crate::tables::detect_mode_count(&text);
        let frequencies = read_band_frequencies(&text, modes)?;
        if let Some(c) = regex::Regex::new(r"(?i)\bnks\s*=\s*(\d+)")?.captures(&text) {
            ensure!(
                c[1].parse::<usize>()? == frequencies.len(),
                "频率点数与 nks 不一致"
            );
        }
        Self::new(read_labelinfo(&labels), frequencies)
    }

    pub fn new(labels: Vec<LabelInfo>, frequencies: Vec<Vec<f64>>) -> Result<Self> {
        let count = frequencies.len();
        ensure!(
            labels.len() >= 2 && count >= 2,
            "声子路径/频率数据为空或点数不足"
        );
        let modes = frequencies[0].len();
        ensure!(
            modes > 0
                && frequencies
                    .iter()
                    .all(|row| row.len() == modes && row.iter().all(|v| v.is_finite())),
            "声子频率模式数不一致或包含非有限数值"
        );
        ensure!(
            labels[0].point_index == 1 && labels.last().unwrap().point_index == count,
            "声子路径端点索引与频率点数不一致"
        );
        ensure!(
            labels
                .iter()
                .all(|l| [l.distance, l.kx, l.ky, l.kz].iter().all(|v| v.is_finite())),
            "声子路径包含非有限数值"
        );
        ensure!(
            labels[0].distance.abs() <= 1e-9 && labels.last().unwrap().distance > 0.,
            "声子路径累计距离无效"
        );
        let mut points = Vec::with_capacity(count);
        let mut segments = Vec::new();
        let mut start = 0;
        let mut ticks: Vec<(f64, String)> = Vec::new();
        for l in &labels {
            let label = l.label.replace("GAMMA", "Γ");
            if let Some(last) = ticks.last_mut() {
                if (last.0 - l.distance).abs() <= 1e-9 {
                    if last.1.split('|').all(|s| s != label) {
                        last.1.push('|');
                        last.1.push_str(&label);
                    }
                    continue;
                }
            }
            ticks.push((l.distance, label));
        }
        for pair in labels.windows(2) {
            let (left, right) = (&pair[0], &pair[1]);
            ensure!(
                right.point_index > left.point_index
                    && right.point_index <= count
                    && right.distance >= left.distance,
                "声子路径索引须严格递增，距离须非递减"
            );
            let jump = (right.distance - left.distance).abs() <= 1e-9;
            if jump {
                ensure!(
                    right.point_index == left.point_index + 1
                        && ((right.kx - left.kx).abs()
                            + (right.ky - left.ky).abs()
                            + (right.kz - left.kz).abs())
                            > 1e-9,
                    "零长度路径段不是有效跳段"
                );
                segments.push(start..left.point_index);
                start = right.point_index - 1;
            }
            for index in left.point_index..right.point_index {
                let f = (index - left.point_index) as f64
                    / (right.point_index - left.point_index) as f64;
                let lerp = |a: f64, b: f64| a + (b - a) * f;
                points.push(LabelInfo {
                    label: String::new(),
                    point_index: index,
                    distance: lerp(left.distance, right.distance),
                    kx: lerp(left.kx, right.kx),
                    ky: lerp(left.ky, right.ky),
                    kz: lerp(left.kz, right.kz),
                });
            }
        }
        points.push(labels.last().unwrap().clone());
        segments.push(start..count);
        ensure!(points.len() == count, "声子路径点数与频率数量不一致");
        Ok(Self {
            labels,
            points,
            frequencies,
            segments,
            ticks,
        })
    }
}

pub(crate) fn frequency_range(values: impl Iterator<Item = f64>) -> Result<(f64, f64)> {
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    for v in values {
        ensure!(v.is_finite(), "频率包含非有限数值");
        low = low.min(v);
        high = high.max(v);
    }
    ensure!(low.is_finite() && high.is_finite(), "频率数据为空");
    low = low.min(0.);
    high = high.max(0.);
    let margin = ((high - low) * 0.05).max(1.);
    Ok((low - margin, high + margin))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn label(name: &str, i: usize, d: f64, x: f64) -> LabelInfo {
        LabelInfo {
            label: name.into(),
            point_index: i,
            distance: d,
            kx: x,
            ky: 0.,
            kz: 0.,
        }
    }
    #[test]
    fn interpolates_every_sample_and_breaks_jumps() {
        let b = BandData::new(
            vec![
                label("GAMMA", 1, 0., 0.),
                label("A", 21, 2., 0.5),
                label("L", 22, 2., 1.),
                label("M", 42, 5., 2.),
            ],
            vec![vec![-3., 12.]; 42],
        )
        .unwrap();
        assert_eq!(b.points.len(), 42);
        assert!((b.points[12].distance - 1.2).abs() < 1e-12);
        assert_eq!(b.segments, vec![0..21, 21..42]);
        assert_eq!(
            b.ticks,
            vec![(0., "Γ".into()), (2., "A|L".into()), (5., "M".into())]
        );
        assert_eq!(
            frequency_range(b.frequencies.iter().flatten().copied()).unwrap(),
            (-4., 13.)
        );
    }
    #[test]
    fn rejects_bad_indices_counts_and_nonfinite_values() {
        for labels in [
            vec![],
            vec![label("G", 1, 0., 0.), label("X", 4, 1., 1.)],
            vec![
                label("G", 1, 0., 0.),
                label("X", 1, 1., 1.),
                label("Y", 3, 2., 2.),
            ],
            vec![
                label("G", 1, 0., 0.),
                label("X", 2, -1., 1.),
                label("Y", 3, 2., 2.),
            ],
            vec![
                label("G", 1, 0., 0.),
                label("X", 2, 0., 0.),
                label("Y", 3, 2., 2.),
            ],
        ] {
            assert!(BandData::new(labels, vec![vec![1.]; 3]).is_err());
        }
        assert!(BandData::new(
            vec![label("G", 1, 0., 0.), label("X", 3, 1., 1.)],
            vec![vec![f64::NAN]; 3]
        )
        .is_err());
    }
}
