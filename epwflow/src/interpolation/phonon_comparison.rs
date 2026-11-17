//! 第六步只消费声子参考和本阶段模型；不读取电子曲线或来源清单。
use super::{
    epw_config::{EpwConfig, NativeStage},
    native,
};
use crate::foundation::{
    combined_plot::{self, Curve, Figure},
    config::Config,
    native_config::COARSE,
    stages::Step,
};
use anyhow::{ensure, Context, Result};
use std::fs;

fn numbers(line: &str) -> Result<Vec<f64>> {
    line.split_whitespace()
        .map(|s| {
            let value = s.replace(['D', 'd'], "e").parse::<f64>()?;
            ensure!(
                value.is_finite(),
                "data_invalid: non-finite phonon comparison value"
            );
            Ok(value)
        })
        .collect()
}
fn curve(name: &str, points: Vec<(f64, f64)>) -> Curve {
    Curve {
        name: name.into(),
        points,
        color: if name == "QE" {
            [0, 0, 0]
        } else {
            [0, 102, 204]
        },
        dashed: name == "EPW",
    }
}

pub fn run(cfg: &Config) -> Result<()> {
    let Some(kpath) = &cfg.kpath else {
        return Ok(());
    };
    let dir = Step::Coarse.directory(cfg);
    let ph = Step::Phonon.directory(cfg);
    let qe_band = ph.join(format!("{}.phband.freq", cfg.qe.prefix));
    let qe_dos = ph.join(format!("{}.phdos.dat", cfg.qe.prefix));
    let reference = native::parse_matdyn_path(
        &fs::read_to_string(&qe_band)
            .with_context(|| format!("data_invalid: missing {}", qe_band.display()))?,
    )?;
    let density = fs::read_to_string(&qe_dos)
        .with_context(|| format!("data_invalid: missing {}", qe_dos.display()))?;
    crate::foundation::reuse_files::report(&cfg.work_dir, [qe_band, qe_dos]);
    // crystal.fmt 第五条记录为 Fortran 列主序的倒格矩阵，坐标单位与原生 phband.freq 一致。
    let crystal = fs::read_to_string(dir.join("crystal.fmt"))?;
    let bg = numbers(
        crystal
            .lines()
            .nth(4)
            .context("data_invalid: crystal.fmt reciprocal lattice missing")?,
    )?;
    ensure!(
        bg.len() == 9,
        "data_invalid: crystal.fmt reciprocal lattice requires 9 values"
    );
    let reciprocal = std::array::from_fn(|i| std::array::from_fn(|j| bg[3 * i + j]));
    let (path, ticks) = super::automatic_comparison::path_points(
        kpath,
        &reciprocal,
        cfg.native_value::<u32>(&[COARSE, "workflow", "points_per_edge"])? as usize,
    )?;
    let path_points = path.iter().map(|p| p.crystal).collect::<Vec<_>>();
    let grid = cfg.phonon.q_grid.map(|n| n * 4);
    let mut q = path_points.clone();
    for a in 0..grid[0] {
        for b in 0..grid[1] {
            for c in 0..grid[2] {
                q.push([
                    a as f64 / grid[0] as f64,
                    b as f64 / grid[1] as f64,
                    c as f64 / grid[2] as f64,
                ]);
            }
        }
    }
    let structure = crate::qe_data::qe_config::QeSettings::from_config(cfg)?;
    let epw = EpwConfig::from_config(cfg)?.with_species(&structure.cell.elements)?;
    let outputs = || -> Result<native::NativeBands> {
        native::checked_log(&fs::read_to_string(dir.join("epw_compare.out"))?, 0)?;
        let mut bands = native::parse_bands(&fs::read_to_string(dir.join("phband.freq"))?)?;
        ensure!(
            bands.values.len() == q.len(),
            "data_invalid: phonon interpolation point count mismatch"
        );
        let requested = q
            .iter()
            .map(|p| std::array::from_fn(|i| (0..3).map(|j| p[j] * reciprocal[j][i]).sum()))
            .collect::<Vec<_>>();
        super::native_sampling::output(&bands, &requested, structure.cell.positions.len() * 3)?;
        // 原生 EPW phband.freq 使用 meV；QE matdyn 与图表使用 cm^-1。
        for value in bands.values.iter_mut().flatten() {
            *value *= super::differences::MEV_CM1;
        }
        Ok(bands)
    };
    let bands = super::attempt::run(
        &cfg.work_dir,
        "comparison-epw",
        &dir,
        String::new(),
        true,
        || {
            EpwConfig::write_points(&dir, &[[0., 0., 0.]], &q)?;
            fs::write(
                dir.join("epw_compare.in"),
                epw.native_input(
                    NativeStage::Comparison,
                    "./out",
                    "./save",
                    cfg.interpolation.phonon_smearings_mev[0],
                )?,
            )?;
            Ok(())
        },
        || {
            super::coarse::run_epw(
                &epw,
                &dir,
                "epw_compare.in",
                &cfg.programs.launcher,
                crate::foundation::stage::Backend::Local,
            )
        },
        outputs,
        |_| Ok(vec![]),
    )?;
    let qe_path = super::comparison::native_path(&path, &reference.points_cartesian, &reciprocal)?;
    let mut curves = Vec::new();
    for (name, positions, values) in [
        ("QE", qe_path.as_slice(), reference.values.as_slice()),
        ("EPW", path.as_slice(), &bands.values[..path_points.len()]),
    ] {
        for mode in 0..structure.cell.positions.len() * 3 {
            ensure!(
                values.iter().all(|r| r.len() > mode),
                "data_invalid: incomplete phonon modes"
            );
            let mut start = 0;
            for end in 1..=positions.len() {
                if end == positions.len()
                    || positions[end].segment_id != positions[start].segment_id
                {
                    curves.push(curve(
                        name,
                        (start..end)
                            .map(|i| (positions[i].distance, values[i][mode]))
                            .collect(),
                    ));
                    start = end;
                }
            }
        }
    }
    let qe_density = density
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with(['#', '!']))
        .map(|l| {
            let row = numbers(l)?;
            ensure!(
                row.len() >= 2,
                "data_invalid: phonon DOS needs frequency and density"
            );
            ensure!(row[1] >= 0., "data_invalid: negative phonon DOS");
            Ok((row[0], row[1]))
        })
        .collect::<Result<Vec<_>>>()?;
    ensure!(!qe_density.is_empty(), "data_invalid: empty QE phonon DOS");
    let maximum = bands
        .values
        .iter()
        .flatten()
        .copied()
        .chain(qe_density.iter().map(|p| p.0))
        .fold(0., f64::max);
    let minimum = bands
        .values
        .iter()
        .flatten()
        .copied()
        .chain(qe_density.iter().map(|p| p.0))
        .fold(0., f64::min);
    let sigma = cfg.comparison.clone().unwrap_or_default().phonon_sigma_cm1;
    ensure!(
        sigma > 0. && maximum > minimum,
        "configuration: invalid phonon DOS bounds or smearing"
    );
    let frequencies = bands.values[path_points.len()..]
        .iter()
        .flatten()
        .copied()
        .collect::<Vec<_>>();
    let nmesh = (q.len() - path.len()) as f64;
    let epw_density = (0..501)
        .map(|i| {
            let x = minimum - 4. * sigma + (maximum - minimum + 8. * sigma) * i as f64 / 500.;
            let y = frequencies
                .iter()
                .map(|w| (-((x - w) / sigma).powi(2)).exp())
                .sum::<f64>()
                / (nmesh * sigma * std::f64::consts::PI.sqrt());
            (x, y)
        })
        .collect();
    let output = crate::foundation::stage_results::root(cfg).join("6.epw-coarse");
    fs::create_dir_all(&output)?;
    combined_plot::write(
        &output.join("phonon.png"),
        &Figure {
            kind: "Combined".into(),
            renderer_version: combined_plot::VERSION,
            title: "QE / EPW phonon dispersion and DOS".into(),
            bounds: [minimum, maximum],
            bands: curves,
            dos: vec![curve("QE", qe_density), curve("EPW", epw_density)],
            labels: ticks.into_iter().map(|t| (t.distance, t.label)).collect(),
            window: vec![],
            notes: vec![format!(
                "QE native stage-4 DOS; EPW Gaussian sigma={sigma} cm-1; native sampling retained"
            )],
        },
    )?;
    Ok(())
}
