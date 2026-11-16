//! 原生输出只与它自身保存的请求核对；物理路径相同不要求内部采样相同。
use super::{comparison, native::NativeBands};
use crate::qe_data::xml::Geometry;
use anyhow::{ensure, Context, Result};

fn rows<'a>(text: &'a str) -> Vec<&'a str> {
    text.lines()
        .map(|line| line.split('!').next().unwrap().trim())
        .filter(|line| !line.is_empty())
        .collect()
}
fn points(lines: &[&str], count: usize, columns: usize) -> Result<Vec<[f64; 3]>> {
    ensure!(
        count > 0 && lines.len() == count,
        "data_invalid: native request declares {count} points but contains {} rows",
        lines.len()
    );
    lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let row = line.split_whitespace().collect::<Vec<_>>();
            ensure!(
                row.len() == columns || (columns == 3 && row.len() == 4),
                "data_invalid: native request point {} has invalid columns",
                i + 1
            );
            let mut p = [0.0_f64; 3];
            for j in 0..3 {
                p[j] = row[j].replace(['d', 'D'], "e").parse()?;
                ensure!(
                    p[j].is_finite(),
                    "data_invalid: non-finite native request coordinate"
                );
            }
            Ok(p)
        })
        .collect()
}
pub(super) fn matdyn(text: &str) -> Result<Vec<[f64; 3]>> {
    let lines = rows(text);
    for name in ["q_in_band_form", "q_in_cryst_coord"] {
        let re = regex::Regex::new(&format!(r"(?im)^\s*{name}\s*=\s*\.false\."))?;
        ensure!(
            re.is_match(text),
            "data_invalid: comparison matdyn requires explicit {name}=.false."
        );
    }
    let end = lines
        .iter()
        .position(|line| *line == "/")
        .context("data_invalid: missing matdyn input namelist terminator")?;
    let count: usize = lines
        .get(end + 1)
        .context("data_invalid: missing matdyn request count")?
        .parse()?;
    points(&lines[end + 2..], count, 3)
}
pub(super) fn epw(text: &str) -> Result<Vec<[f64; 3]>> {
    let lines = rows(text);
    let header = lines
        .first()
        .context("data_invalid: empty EPW point request")?
        .split_whitespace()
        .collect::<Vec<_>>();
    ensure!(
        header.len() == 2 && header[1].eq_ignore_ascii_case("crystal"),
        "data_invalid: EPW comparison request must declare crystal coordinates"
    );
    points(&lines[1..], header[0].parse()?, 4)
}
pub(crate) fn pw(text: &str) -> Result<Vec<[f64; 3]>> {
    let lines = rows(text);
    let start = lines
        .iter()
        .position(|line| {
            let normalized = line.replace(['{', '}', '(', ')'], " ").to_lowercase();
            normalized.split_whitespace().collect::<Vec<_>>() == ["k_points", "crystal"]
        })
        .context("data_invalid: comparison PW requires explicit K_POINTS crystal")?;
    let count = lines
        .get(start + 1)
        .context("data_invalid: missing PW point count")?
        .parse()?;
    points(&lines[start + 2..], count, 4)
}
pub(super) fn output(values: &NativeBands, requested: &[[f64; 3]], modes: usize) -> Result<()> {
    ensure!(values.points_cartesian.len() == requested.len()
        && values.values.len() == requested.len()
        && values.values.iter().all(|v| v.len() == modes),
        "data_invalid: native output has {} points, request has {}; expected {modes} bands/modes, got {:?}",
        values.values.len(), requested.len(), values.values.iter().map(Vec::len).collect::<std::collections::BTreeSet<_>>());
    for (i, (actual, expected)) in values.points_cartesian.iter().zip(requested).enumerate() {
        ensure!(
            actual
                .iter()
                .zip(expected)
                .all(|(a, b)| a.is_finite() && b.is_finite() && (a - b).abs() < 1e-6),
            "input_conflict: native output differs from its own input at point {}",
            i + 1
        );
    }
    Ok(())
}
pub(super) fn phonon(
    values: &NativeBands,
    requested: &[[f64; 3]],
    reference: &[comparison::PathPoint],
    mesh: &[[f64; 3]],
    geometry: &Geometry,
    modes: usize,
) -> Result<()> {
    output(values, requested, modes)?;
    let count = requested
        .len()
        .checked_sub(mesh.len())
        .context("data_invalid: native phonon request lacks its DOS grid")?;
    ensure!(
        count >= 2,
        "data_invalid: native phonon request lacks path endpoints"
    );
    comparison::native_path(
        reference,
        &values.points_cartesian[..count],
        &geometry.reciprocal,
    )?;
    for (i, point) in mesh.iter().enumerate() {
        let expected = super::electronic_sampling::cart(*point, &geometry.reciprocal);
        ensure!(
            expected
                .iter()
                .zip(requested[count + i])
                .all(|(a, b)| (a - b).abs() < 1e-7),
            "input_conflict: native phonon request DOS grid differs at point {i}"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::comparison::PathPoint;
    use super::*;
    fn geo() -> Geometry {
        Geometry {
            alat: 1.,
            lattice: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            reciprocal: [[1., 0., 0.], [0.5, 1., 0.], [0., 0., 2.]],
            atoms: vec![("X".into(), [0.; 3]); 3],
            fft: [1; 3],
        }
    }
    #[test]
    fn independent_nonuniform_requests_validate_only_their_own_output() {
        let g = geo();
        let p = |segment_id, point_id, distance, crystal| PathPoint {
            segment_id,
            point_id,
            distance,
            crystal,
        };
        let reference = vec![
            p(0, 0, 0., [0., 0., 0.]),
            p(0, 1, 1., [1., 0., 0.]),
            p(1, 2, 2., [0., 0., 0.5]),
            p(1, 3, 3., [1., 0., 0.5]),
        ];
        let mesh = [[0., 0., 0.], [0.5, 0.5, 0.]];
        for xs in [vec![0., 0.25, 0.8, 1.], vec![0., 0.1, 0.2, 0.3, 0.6, 1.]] {
            let points = xs
                .iter()
                .map(|&x| [x, 0., 0.])
                .chain(xs.iter().map(|&x| [x, 0., 0.5]))
                .chain(mesh.iter().copied())
                .map(|p| super::super::electronic_sampling::cart(p, &g.reciprocal))
                .collect::<Vec<_>>();
            let mut values = NativeBands {
                points_cartesian: points.clone(),
                values: vec![vec![1.; 9]; points.len()],
            };
            phonon(&values, &points, &reference, &mesh, &g, 9).unwrap();
            assert!(output(&values, &points[..points.len() - 1], 9).is_err());
            values.points_cartesian[1][1] += 0.1;
            assert!(phonon(&values, &points, &reference, &mesh, &g, 9).is_err());
            let bad_request = values.points_cartesian.clone();
            assert!(
                phonon(&values, &bad_request, &reference, &mesh, &g, 9).is_err(),
                "同输入输出仍需在物理路径上"
            );
            values.points_cartesian = points.clone();
            values.values[0].pop();
            assert!(phonon(&values, &points, &reference, &mesh, &g, 9).is_err());
        }
    }
    #[test]
    fn saved_requests_reject_missing_rows_wrong_coordinate_space_and_nan() {
        let input =
            "&input\n q_in_cryst_coord=.false.\n q_in_band_form=.false.\n/\n2\n0 0 0\n1D0 0 0\n";
        assert_eq!(matdyn(input).unwrap(), vec![[0., 0., 0.], [1., 0., 0.]]);
        assert!(matdyn(&input.replace("\n2\n", "\n3\n")).is_err());
        assert!(matdyn(&input.replace("1D0", "NaN")).is_err());
        assert!(matdyn(&input.replace("q_in_band_form=.false.", "q_in_band_form=.true.")).is_err());
        assert_eq!(epw("2 crystal\n0 0 0 0.5\n1 0 0 0.5\n").unwrap().len(), 2);
        assert!(epw("2 cartesian\n0 0 0 1\n1 0 0 1\n").is_err());
        assert!(epw("2 crystal\n0 0 0 1\n").is_err());
        assert_eq!(
            pw("&control\n/\nK_POINTS crystal\n2\n0 0 0 1\n1 0 0 1\n")
                .unwrap()
                .len(),
            2
        );
    }
    #[test]
    #[ignore = "requires DFTFLOW_REAL_COMPARISON_INPUT; reads existing metadata only"]
    fn actual_cached_matdyn_uses_its_own_request_instead_of_current_density() {
        let input =
            std::path::PathBuf::from(std::env::var("DFTFLOW_REAL_COMPARISON_INPUT").unwrap());
        let cfg = crate::foundation::config::Config::load(&input).unwrap();
        let root = cfg
            .work_dir
            .join("6.epw-coarse/comparison-native/frequencies");
        let own = matdyn(&std::fs::read_to_string(root.join("matdyn.in")).unwrap()).unwrap();
        let values = super::super::native::parse_matdyn_path(
            &std::fs::read_to_string(root.join("reference.freq")).unwrap(),
        )
        .unwrap();
        let xml = cfg
            .work_dir
            .join("2.qe-nscf/out")
            .join(format!("{}.save/data-file-schema.xml", cfg.qe.prefix));
        let g = Geometry::read(&xml).unwrap();
        let states = crate::qe_data::nscf::parse_xml(&xml).unwrap();
        let mesh = states
            .kpoints
            .iter()
            .map(|p| {
                let p = g.crystal(*p);
                std::array::from_fn(|i| {
                    (p[i] * cfg.qe.nscf.k_grid[i] as f64).round() / cfg.qe.nscf.k_grid[i] as f64
                })
            })
            .collect::<Vec<_>>();
        let (reference, _) = super::super::automatic_comparison::path_points(
            cfg.kpath.as_ref().unwrap(),
            &g.reciprocal,
            cfg.native_value::<u32>(&[&cfg.phonon_stage(), "workflow", "points_per_edge"])
                .unwrap() as usize,
        )
        .unwrap();
        assert_ne!(
            own.len(),
            reference.len() + mesh.len(),
            "本例须确实重现新计划与旧原生采样不同"
        );
        phonon(&values, &own, &reference, &mesh, &g, g.atoms.len() * 3).unwrap();
        println!(
            "existing_request={} current_plan={} modes={} accepted without recalculation",
            own.len(),
            reference.len() + mesh.len(),
            g.atoms.len() * 3
        );
    }
}
