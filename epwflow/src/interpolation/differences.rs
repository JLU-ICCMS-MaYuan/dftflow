//! 常规出图数据的最大差值；不含阈值、评级或执行决策。
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

pub const MEV_CM1: f64 = 8.065543937;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Maximum {
    /// 原始点顺序，零起始；带和排序模式编号一律一起始。
    pub point_index: usize,
    pub crystal: [f64; 3],
    pub path_distance: Option<f64>,
    pub qe_index: usize,
    pub epw_index: usize,
    pub qe_value: f64,
    pub epw_value: f64,
    pub absolute_difference: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Statistics {
    pub unit: String,
    pub points: usize,
    pub paired_states: usize,
    pub unmatched_qe_states: usize,
    pub unmatched_epw_states: usize,
    pub maximum: Option<Maximum>,
}

impl Statistics {
    fn new(unit: &str, points: usize) -> Self {
        Self {
            unit: unit.into(),
            points,
            paired_states: 0,
            unmatched_qe_states: 0,
            unmatched_epw_states: 0,
            maximum: None,
        }
    }

    fn record(
        &mut self,
        point_index: usize,
        crystal: [f64; 3],
        path_distance: Option<f64>,
        qe: (usize, f64),
        epw: (usize, f64),
    ) -> Result<()> {
        let difference = (qe.1 - epw.1).abs();
        ensure!(difference.is_finite(), "data_invalid: 差值计算溢出");
        self.paired_states += 1;
        if self
            .maximum
            .as_ref()
            .is_none_or(|m| difference > m.absolute_difference)
        {
            self.maximum = Some(Maximum {
                point_index,
                crystal,
                path_distance,
                qe_index: qe.0,
                epw_index: epw.0,
                qe_value: qe.1,
                epw_value: epw.1,
                absolute_difference: difference,
            });
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            (self.paired_states == 0) == self.maximum.is_none(),
            "data_invalid: 最大差值与配对数量不符"
        );
        if let Some(m) = &self.maximum {
            ensure!(
                m.point_index < self.points
                    && m.qe_index > 0
                    && m.epw_index > 0
                    && m.crystal.iter().all(|v| v.is_finite())
                    && m.path_distance.is_none_or(f64::is_finite)
                    && [m.qe_value, m.epw_value, m.absolute_difference]
                        .iter()
                        .all(|v| v.is_finite())
                    && m.absolute_difference >= 0.0
                    && ((m.qe_value - m.epw_value).abs() - m.absolute_difference).abs() < 1e-8,
                "data_invalid: 最大差值或位置无效"
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Differences {
    pub electronic_path: Statistics,
    pub electronic_mesh: Statistics,
    pub phonon_path: Statistics,
    pub phonon_mesh: Statistics,
}

impl Differences {
    pub fn entries(&self) -> [(&str, &Statistics); 4] {
        [
            ("electronic_path", &self.electronic_path),
            ("electronic_mesh", &self.electronic_mesh),
            ("phonon_path", &self.phonon_path),
            ("phonon_mesh", &self.phonon_mesh),
        ]
    }

    pub fn validate(&self) -> Result<()> {
        for (name, stats) in self.entries() {
            ensure!(
                stats.unit
                    == if name.starts_with("electronic") {
                        "eV"
                    } else {
                        "cm-1"
                    },
                "data_invalid: 差值单位无效"
            );
            stats.validate()?;
        }
        Ok(())
    }

    pub fn print(&self) {
        for (name, s) in self.entries() {
            if let Some(m) = &s.maximum {
                println!("{name}: max_abs={:.9} {} crystal={:?} QE[{}]={:.9} EPW[{}]={:.9} paired={} unmatched_qe={} unmatched_epw={}",
                    m.absolute_difference, s.unit, m.crystal, m.qe_index, m.qe_value,
                    m.epw_index, m.epw_value, s.paired_states, s.unmatched_qe_states, s.unmatched_epw_states);
            } else {
                println!(
                    "{name}: 无可配对数据; paired=0 unmatched_qe={} unmatched_epw={}",
                    s.unmatched_qe_states, s.unmatched_epw_states
                );
            }
        }
    }
}

/// 较少一侧全部配对较多一侧的有序子集；不复用同一态，不拟合零点。
fn pairs(qe: &[(usize, f64)], epw: &[(usize, f64)]) -> Result<Vec<(usize, usize)>> {
    let swapped = qe.len() > epw.len();
    let (small, large) = if swapped { (epw, qe) } else { (qe, epw) };
    let mut cost = vec![vec![f64::INFINITY; large.len() + 1]; small.len() + 1];
    let mut take = vec![vec![false; large.len() + 1]; small.len() + 1];
    cost[0].fill(0.0);
    for a in 1..=small.len() {
        for b in 1..=large.len() {
            // 最小化平方和等价于最小化其平方根；hypot 避免有限大差值平方溢出。
            let matched = cost[a - 1][b - 1].hypot(small[a - 1].1 - large[b - 1].1);
            if matched < cost[a][b - 1] {
                cost[a][b] = matched;
                take[a][b] = true;
            } else {
                cost[a][b] = cost[a][b - 1];
            }
        }
    }
    ensure!(
        cost[small.len()][large.len()].is_finite(),
        "data_invalid: 能量配对计算溢出"
    );
    let (mut a, mut b) = (small.len(), large.len());
    let mut out = Vec::with_capacity(a);
    while a > 0 {
        ensure!(b > 0, "data_invalid: 能量配对不完整");
        if take[a][b] {
            out.push(if swapped {
                (b - 1, a - 1)
            } else {
                (a - 1, b - 1)
            });
            a -= 1;
        }
        b -= 1;
    }
    out.reverse();
    Ok(out)
}

fn check_points(
    points: &[[f64; 3]],
    distance: Option<&[f64]>,
    qe: &[Vec<f64>],
    epw: &[Vec<f64>],
) -> Result<()> {
    ensure!(
        points.len() == qe.len()
            && qe.len() == epw.len()
            && distance.is_none_or(|d| d.len() == points.len() && d.iter().all(|v| v.is_finite()))
            && points.iter().flatten().all(|v| v.is_finite())
            && qe.iter().chain(epw).flatten().all(|v| v.is_finite()),
        "data_invalid: 比较点数、坐标或数值无效"
    );
    Ok(())
}

pub fn electronic(
    points: &[[f64; 3]],
    distance: Option<&[f64]>,
    qe: &[Vec<f64>],
    epw: &[Vec<f64>],
    fermi: f64,
    window: [f64; 2],
) -> Result<Statistics> {
    check_points(points, distance, qe, epw)?;
    ensure!(
        fermi.is_finite() && window.iter().all(|v| v.is_finite()) && window[0] < window[1],
        "data_invalid: 比较能窗或能量零点无效"
    );
    let mut stats = Statistics::new("eV", points.len());
    for i in 0..points.len() {
        let select = |row: &[f64]| -> Result<Vec<(usize, f64)>> {
            let mut selected = Vec::new();
            for (index, e) in row.iter().enumerate() {
                let relative = e - fermi;
                ensure!(relative.is_finite(), "data_invalid: 能量零点换算溢出");
                if window[0] <= relative && relative <= window[1] {
                    selected.push((index + 1, relative));
                }
            }
            selected.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            Ok(selected)
        };
        let (r, c) = (select(&qe[i])?, select(&epw[i])?);
        let matched = pairs(&r, &c)?;
        stats.unmatched_qe_states += r.len() - matched.len();
        stats.unmatched_epw_states += c.len() - matched.len();
        for (a, b) in matched {
            stats.record(i, points[i], distance.map(|d| d[i]), r[a], c[b])?;
        }
    }
    Ok(stats)
}

/// 输入 QE 为 cm⁻¹，EPW 为 meV；比较排序后的频率，不声称本征矢身份一致。
pub fn phonon(
    points: &[[f64; 3]],
    distance: Option<&[f64]>,
    qe: &[Vec<f64>],
    epw_mev: &[Vec<f64>],
    modes: usize,
) -> Result<Statistics> {
    check_points(points, distance, qe, epw_mev)?;
    ensure!(modes > 0 && modes % 3 == 0, "data_invalid: 声子模式数无效");
    let mut stats = Statistics::new("cm-1", points.len());
    for i in 0..points.len() {
        ensure!(
            qe[i].len() == modes && epw_mev[i].len() == modes,
            "data_invalid: 缺少 3N 声子模式"
        );
        let mut r = qe[i].clone();
        let mut c: Vec<_> = epw_mev[i].iter().map(|v| v * MEV_CM1).collect();
        ensure!(
            c.iter().all(|v| v.is_finite()),
            "data_invalid: 声子单位换算溢出"
        );
        r.sort_by(f64::total_cmp);
        c.sort_by(f64::total_cmp);
        for m in 0..modes {
            stats.record(
                i,
                points[i],
                distance.map(|d| d[i]),
                (m + 1, r[m]),
                (m + 1, c[m]),
            )?;
        }
    }
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn energy_matching_is_injective_and_keeps_original_indices_without_thresholds() {
        let q = vec![vec![20., -10., 0., 4.]];
        let e = vec![vec![3., 1.]];
        let r = electronic(&[[0.; 3]], Some(&[0.]), &q, &e, 0., [-30., 30.]).unwrap();
        assert_eq!(r.paired_states, 2);
        assert_eq!(r.unmatched_qe_states, 2);
        let m = r.maximum.unwrap();
        assert_eq!(m.absolute_difference, 1.);
        assert_eq!((m.qe_index, m.epw_index), (3, 2));
        let r = electronic(&[[0.; 3]], None, &e, &q, 0., [-30., 30.]).unwrap();
        assert_eq!(r.paired_states, 2);
        assert_eq!(r.unmatched_epw_states, 2);
    }
    #[test]
    fn energy_pairs_minimize_total_square_and_use_actual_window_common_fermi() {
        // 最近邻贪心会复用或选错状态；枚举两个有序位置得到相同全局最优值。
        let qe = vec![vec![10., 14.]];
        let epw = vec![vec![7., 11., 15.]];
        let r = electronic(&[[0.25, 0., 0.]], None, &qe, &epw, 10., [-4., 6.]).unwrap();
        assert_eq!(r.paired_states, 2);
        assert_eq!(r.unmatched_epw_states, 1);
        let m = r.maximum.unwrap();
        assert_eq!((m.qe_value, m.epw_value), (0., 1.));
        assert_eq!((m.qe_index, m.epw_index), (1, 2));
        // 窗外态不参与任何配对或未配对计数；窗口不必覆盖 ±0.5 eV。
        let r = electronic(&[[0.; 3]], None, &qe, &epw, 10., [3., 6.]).unwrap();
        assert_eq!(r.paired_states, 1);
        assert_eq!(r.unmatched_epw_states, 0);
        let m = r.maximum.unwrap();
        assert_eq!((m.qe_value, m.epw_value), (4., 5.));
    }
    #[test]
    fn large_finite_differences_do_not_overflow_squared_matching_costs() {
        let r = electronic(
            &[[0.; 3]],
            None,
            &[vec![-1e200, 2e200]],
            &[vec![0.]],
            0.,
            [-3e200, 3e200],
        )
        .unwrap();
        r.validate().unwrap();
        assert_eq!(r.maximum.unwrap().absolute_difference, 1e200);
    }
    #[test]
    fn empty_window_has_no_fabricated_zero() {
        let r = electronic(&[[0.; 3]], None, &[vec![1.]], &[vec![2.]], 0., [-0.5, 0.5]).unwrap();
        assert_eq!(r.paired_states, 0);
        assert!(r.maximum.is_none());
        let r = electronic(&[[0.; 3]], None, &[vec![1.]], &[vec![]], 0., [0., 3.]).unwrap();
        assert_eq!(r.unmatched_qe_states, 1);
        assert!(r.maximum.is_none());
    }
    #[test]
    fn phonons_keep_signed_frequencies_degeneracy_units_and_location() {
        let r = phonon(
            &[[0.; 3], [0.25, 0., 0.]],
            Some(&[0., 1.]),
            &[vec![10., -2., 10.], vec![10., 10., -5.]],
            &[
                vec![10. / MEV_CM1, 10. / MEV_CM1, -2. / MEV_CM1],
                vec![20. / MEV_CM1, -8. / MEV_CM1, 10. / MEV_CM1],
            ],
            3,
        )
        .unwrap();
        let m = r.maximum.unwrap();
        assert!((m.absolute_difference - 10.).abs() < 1e-10);
        assert_eq!(m.point_index, 1);
        assert_eq!(m.crystal, [0.25, 0., 0.]);
        assert_eq!(m.path_distance, Some(1.));
        assert_eq!(m.qe_index, 3);
    }
    #[test]
    fn invalid_shapes_nonfinite_values_and_missing_modes_remain_errors() {
        assert!(electronic(&[[0.; 3]], None, &[], &[], 0., [-1., 1.]).is_err());
        assert!(electronic(
            &[[0.; 3]],
            None,
            &[vec![f64::NAN]],
            &[vec![0.]],
            0.,
            [-1., 1.]
        )
        .is_err());
        assert!(phonon(&[[0.; 3]], None, &[vec![0.; 2]], &[vec![0.; 2]], 3).is_err());
    }
}
