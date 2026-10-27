//! EPW 内嵌 Wannier90 的采样输入；只整理已有 QE 点位，不计算本征值。
use crate::{
    foundation::{config::Config, layout, store},
    interpolation::{automatic_comparison, comparison::PathPoint, comparison_data::Tick},
    qe_data::{nscf, xml::Geometry},
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
pub struct Sampling {
    pub path: Vec<PathPoint>,
    pub ticks: Vec<Tick>,
    pub mesh: Vec<[f64; 3]>,
    pub reciprocal: [[f64; 3]; 3],
}
pub fn xml(cfg: &Config) -> PathBuf {
    layout::path(&cfg.work_dir, "nscf/out")
        .join(format!("{}.save/data-file-schema.xml", cfg.qe.prefix))
}
pub fn plan(cfg: &Config) -> Result<Sampling> {
    let file = xml(cfg);
    let geo = Geometry::read(&file)?;
    geo.validate(&crate::qe_data::qe_config::QeSettings::from_config(cfg)?)?;
    let mut states = nscf::parse_xml(&file)?;
    states.k_grid = cfg.qe.nscf.k_grid;
    nscf::validate_grid(&states, states.k_grid, cfg.qe.nscf.nbnd as usize, &geo)?;
    let (path, ticks) = automatic_comparison::path_points(
        cfg.kpath.as_ref().context("required: 电子比较需要 KPATH")?,
        &geo.reciprocal,
        cfg.native_value::<u32>(&[crate::foundation::native_config::WANNIER,"workflow","points_per_edge"])? as usize,
    )?;
    let mesh = states.kpoints.iter().map(|p| geo.crystal(*p)).collect();
    Ok(Sampling {
        path,
        ticks,
        mesh,
        reciprocal: geo.reciprocal,
    })
}
pub fn cart(p: [f64; 3], reciprocal: &[[f64; 3]; 3]) -> [f64; 3] {
    std::array::from_fn(|j| (0..3).map(|i| p[i] * reciprocal[i][j]).sum())
}
fn length(a: [f64; 3], b: [f64; 3], reciprocal: &[[f64; 3]; 3]) -> f64 {
    cart(std::array::from_fn(|i| b[i] - a[i]), reciprocal)
        .iter()
        .map(|v| v * v)
        .sum::<f64>()
        .sqrt()
}
/// 每个 QE 样本均为某段端点；允许 W90 额外采样，随后按真实坐标提取。
/// 首段以最短 QE 间隔校准，避免 W90 nint 把短段取成零点。
pub fn wdata(s: &Sampling, bands_plot: bool, bands_num_points: u32) -> Result<Vec<String>> {
    let edges: Vec<_> = s
        .path
        .windows(2)
        .filter(|p| p[0].segment_id == p[1].segment_id)
        .collect();
    let first = edges
        .iter()
        .min_by(|a, b| {
            length(a[0].crystal, a[1].crystal, &s.reciprocal).total_cmp(&length(
                b[0].crystal,
                b[1].crystal,
                &s.reciprocal,
            ))
        })
        .context("data_invalid: 空电子路径")?;
    let anchor = first[0].crystal;
    let mut calibration = (anchor, first[1].crystal);
    for p in &s.mesh {
        let d = length(anchor, *p, &s.reciprocal);
        if d > 1e-10 && d < length(calibration.0, calibration.1, &s.reciprocal) {
            calibration = (anchor, *p);
        }
    }
    let reference = length(calibration.0, calibration.1, &s.reciprocal);
    ensure!(
        reference.is_finite() && reference > 1e-10,
        "data_invalid: 零长度电子路径"
    );
    let mut rows = vec![calibration];
    let mut i = 0;
    while i + 1 < s.path.len() {
        if s.path[i].segment_id != s.path[i + 1].segment_id {
            i += 1;
            continue;
        }
        let mut end = i + 1;
        // 只合并同一直线且 W90 的等分确实覆盖每个原始点的连续间隔。
        for j in i + 2..s.path.len() {
            if s.path[j].segment_id != s.path[i].segment_id {
                break;
            }
            let count = j - i;
            let straight = (1..count).all(|k| {
                (0..3).all(|a| {
                    (s.path[i + k].crystal[a]
                        - (s.path[i].crystal[a]
                            + (s.path[j].crystal[a] - s.path[i].crystal[a]) * k as f64
                                / count as f64))
                        .abs()
                        < 1e-10
                })
            });
            if !straight {
                break;
            }
            let n = (length(s.path[i].crystal, s.path[j].crystal, &s.reciprocal) / reference)
                .round() as usize;
            if n >= count && n % count == 0 {
                end = j;
            }
        }
        rows.push((s.path[i].crystal, s.path[end].crystal));
        i = end;
    }
    for p in &s.mesh {
        if length(anchor, *p, &s.reciprocal) > 1e-10 {
            rows.push((anchor, *p));
        }
    }
    // 不同晶格方向的长度比可能使等分段无法合并。此时用不连续的端点对
    // 覆盖所有目标；W90 原生输出每段两端，extract 仍按真实坐标恢复原路径。
    // 这些行只是模型采样任务，不能把它们的人工横轴当作科学路径。
    let mut targets: Vec<[f64; 3]> = Vec::new();
    for point in s
        .path
        .iter()
        .map(|p| p.crystal)
        .chain(s.mesh.iter().copied())
    {
        if !targets
            .iter()
            .any(|p| p.iter().zip(point).all(|(a, b)| (a - b).abs() < 1e-10))
        {
            targets.push(point);
        }
    }
    let mut pairs = Vec::new();
    for pair in targets.chunks(2) {
        pairs.push((pair[0], *pair.get(1).unwrap_or(&targets[0])));
    }
    if pairs.len() + 1 < rows.len() {
        let shortest = *pairs
            .iter()
            .min_by(|a, b| {
                length(a.0, a.1, &s.reciprocal).total_cmp(&length(b.0, b.1, &s.reciprocal))
            })
            .context("data_invalid: 空电子采样")?;
        ensure!(
            length(shortest.0, shortest.1, &s.reciprocal) > 1e-10,
            "data_invalid: 零长度电子采样段"
        );
        rows = vec![shortest];
        rows.extend(pairs);
    }
    let mut data = vec![
        format!("bands_plot = {bands_plot}"),
        format!("bands_num_points = {bands_num_points}"),
        "begin kpoint_path".into(),
    ];
    for (i, (a, b)) in rows.iter().enumerate() {
        // 实测部署程序生成的 win 每行只有 80 字节（含首空格），与源码容量不同。
        // 紧凑十位小数远高于 kpt 六位精度；不截掉必要坐标或改科学路径。
        let point = |p: &[f64; 3]| p.iter().map(|v| {
            let value = format!("{v:.10}");
            let value = value.trim_end_matches('0').trim_end_matches('.');
            if value == "-0" { "0".into() } else { value.to_owned() }
        }).collect::<Vec<String>>().join(" ");
        data.push(format!(
            "P{i} {} Q{i} {}", point(a), point(b)
        ));
    }
    data.push("end kpoint_path".into());
    ensure!(data.iter().all(|line| line.len() <= 79),
        "configuration: 部署 EPW 的路径行超过 79 字节，不能截断坐标");
    Ok(data)
}
fn same(a: &[f64; 3], b: &[f64; 3]) -> bool {
    a.iter().zip(b).all(|(a, b)| (a - b).abs() <= 5.1e-7)
}
/// W90 kpt 保留六位小数；拒绝不能唯一分辨的不同目标位置。
pub fn extract(cfg: &Config, dir: &Path) -> Result<()> {
    let sampling = plan(cfg)?;
    let band = dir.join(format!("{}_band.dat", cfg.qe.prefix));
    let kpt = fs::read_to_string(dir.join(format!("{}_band.kpt", cfg.qe.prefix)))?;
    let mut lines = kpt.lines();
    let count: usize = lines
        .next()
        .context("data_invalid: 空 W90 kpt")?
        .trim()
        .parse()?;
    let coords = lines
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let r = crate::qe_data::xml::numbers(l)?;
            ensure!(
                r.len() == 4 && (r[3] - 1.0).abs() < 1e-8,
                "data_invalid: W90 kpt 格式"
            );
            Ok([r[0], r[1], r[2]])
        })
        .collect::<Result<Vec<_>>>()?;
    ensure!(coords.len() == count, "data_invalid: W90 点表截断");
    let bands = wannier90flow::plot::parse_segment_file(&band, 0.0)?;
    ensure!(
        bands.len() == cfg.wannier.num_wann as usize
            && bands
                .iter()
                .all(|(x, y)| x.len() == count && y.len() == count),
        "data_invalid: W90 能带和真实坐标数量不同"
    );
    let targets: Vec<_> = sampling
        .path
        .iter()
        .map(|p| p.crystal)
        .chain(sampling.mesh.iter().copied())
        .collect();
    for (i, p) in targets.iter().enumerate() {
        for q in &targets[..i] {
            ensure!(
                !same(p, q) || p.iter().zip(q).all(|(a, b)| (a - b).abs() < 1e-10),
                "data_invalid: kpt 六位小数不足以分辨目标点"
            );
        }
    }
    let mut output = format!(" &plot nbnd= {}, nks= {} /\n", bands.len(), targets.len());
    for p in targets {
        let indices = coords
            .iter()
            .enumerate()
            .filter(|(_, q)| same(&p, q))
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        let index = *indices.first().with_context(|| {
            format!("input_conflict: 内嵌 W90 未输出 QE 目标点 {p:?}；不能缩放横轴或重算 PW")
        })?;
        for b in &bands {
            ensure!(
                indices.iter().all(|&j| (b.1[j] - b.1[index]).abs() < 1e-5),
                "data_invalid: 重复位置的 W90 本征值不一致"
            );
        }
        let c = cart(p, &sampling.reciprocal);
        output.push_str(&format!("{:.15} {:.15} {:.15}\n", c[0], c[1], c[2]));
        output.push_str(
            &bands
                .iter()
                .map(|b| format!("{:.15}", b.1[index]))
                .collect::<Vec<_>>()
                .join(" "),
        );
        output.push('\n');
    }
    fs::write(dir.join("band.eig"), output)?;
    store::write(&dir.join("sampling.json"), &sampling)?;
    store::write(
        &dir.join("band-origin.json"),
        &serde_json::json!({"driver":"epw.x/wdata","coordinates":store::Identity::capture(&dir.join(format!("{}_band.kpt",cfg.qe.prefix)))?,"energies":store::Identity::capture(&band)?}),
    )?;
    Ok(())
}

/// 仅接收已登记的旧四类比较来源；绝不因缺少新版 wdata 输出重做拟合。
pub fn legacy(cfg: &Config, dir: &Path) -> Result<()> {
    let file = cfg
        .work_dir
        .join("6.epw-coarse/comparison-native/sources.json");
    let source: super::comparison::Sources = store::read(&file).context(
        "required: 旧模型缺少内嵌能带回执，也没有可核验的旧比较采样；保留模型，不自动重做拟合",
    )?;
    ensure!(
        source.schema_version == 2,
        "unsupported_version: 旧电子比较来源版本"
    );
    let base = file.parent().unwrap();
    for f in [
        &source.fermi_reference,
        &source.qe_bands_xml,
        &source.qe_mesh_xml,
        &source.epw_bands,
        &source.qe_phonon_bands,
        &source.epw_phonon_bands,
        &source.epw_log,
    ]
    .into_iter()
    .chain(&source.provenance)
    {
        f.checked_path(base)?;
    }
    let sampling = plan(cfg)?;
    ensure!(
        serde_json::to_value(&sampling.path)? == serde_json::to_value(&source.path)?
            && sampling.mesh.len() == source.mesh.len()
            && sampling
                .mesh
                .iter()
                .zip(&source.mesh)
                .all(|(a, b)| same(a, b)),
        "input_conflict: 旧采样不覆盖当前路径或网格"
    );
    ensure!(
        store::hash_file(&xml(cfg))? == source.qe_mesh_xml.sha256,
        "input_conflict: 旧电子网格来源不同"
    );
    let log = source
        .wannier_wout
        .as_ref()
        .context("required: 旧比较没有实际拟合窗口来源")?;
    log.checked_path(base)?;
    ensure!(
        store::hash_file(
            &cfg.work_dir
                .join("5.epw-wannier")
                .join(format!("{}.wout", cfg.qe.prefix))
        )? == log.sha256,
        "input_conflict: 旧采样不属于当前拟合模型"
    );
    let mut bound = false;
    for ext in ["ukk", "chk"] {
        let name = format!("{}.{ext}", cfg.qe.prefix);
        let current = cfg.work_dir.join("5.epw-wannier").join(&name);
        let digest = store::hash_file(&current)?;
        bound |= source.provenance.iter().any(|id| {
            id.path.file_name().is_some_and(|n| n == name.as_str()) && id.sha256 == digest
        });
    }
    ensure!(
        bound,
        "input_conflict: 旧采样没有绑定当前 ukk/chk，不能仅凭相同窗口日志认作同一模型"
    );
    let native = source.epw_bands.checked_path(base)?;
    fs::copy(&native, dir.join("band.eig"))?;
    store::write(&dir.join("sampling.json"), &sampling)?;
    store::write(
        &dir.join("band-origin.json"),
        &serde_json::json!({"driver":"legacy_epw_readonly","source":store::Identity::capture(&file)?,"values":source.epw_bands,"reason":"有界读取已核验旧 EPW 采样；保留旧科学文件、输入脚本和历史回执，不重算"}),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unequal_edges_retain_all_qe_samples_and_capacity_is_explicit() {
        let path = (0..=20)
            .map(|i| PathPoint {
                segment_id: 0,
                point_id: i,
                distance: i as f64,
                crystal: [i as f64 / 20.0, 0.0, 0.0],
            })
            .collect();
        let s = Sampling {
            path,
            ticks: vec![],
            mesh: vec![[0.25, 0.25, 0.0]],
            reciprocal: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        };
        let data = wdata(&s, true, 1).unwrap();
        assert_eq!(data.len(), 7); // 校准、合并路径、网格三段以及四个设置行。
        assert!(data.iter().all(|l| l.len() <= 79));
    }

    #[test]
    fn nonorthogonal_unequal_segments_cover_the_exact_reference_coordinates() {
        let reciprocal = [[1., 0., 0.], [0.3, 1.2, 0.], [0., 0., 0.7]];
        let mut path = vec![];
        for (segment, (a, b)) in [
            ([0., 0., 0.], [0.5, 0., 0.]),
            ([0., 0., 0.5], [0.5, 0.5, 0.5]),
        ]
        .into_iter()
        .enumerate()
        {
            for i in 0..=20 {
                path.push(PathPoint {
                    segment_id: segment,
                    point_id: path.len(),
                    distance: path.len() as f64,
                    crystal: std::array::from_fn(|j| a[j] + (b[j] - a[j]) * i as f64 / 20.),
                });
            }
        }
        let s = Sampling {
            path,
            ticks: vec![],
            mesh: vec![[0., 0., 0.], [0.25, 0.5, 0.75]],
            reciprocal,
        };
        let data = wdata(&s, true, 1).unwrap();
        let mut points = vec![];
        let mut reference = 0.;
        for row in &data[3..data.len() - 1] {
            let cells = row.split_whitespace().collect::<Vec<_>>();
            let a = std::array::from_fn(|j| cells[j + 1].parse::<f64>().unwrap());
            let b = std::array::from_fn(|j| cells[j + 5].parse::<f64>().unwrap());
            let distance = length(a, b, &s.reciprocal);
            if reference == 0. {
                reference = distance;
            }
            let n = (distance / reference).round() as usize;
            assert!(n > 0);
            for i in 0..=n {
                points.push(std::array::from_fn(|j| {
                    a[j] + (b[j] - a[j]) * i as f64 / n as f64
                }));
            }
        }
        for target in s.path.iter().map(|p| p.crystal).chain(s.mesh) {
            assert!(
                points
                    .iter()
                    .any(|p: &[f64; 3]| p.iter().zip(target).all(|(a, b)| (a - b).abs() < 1e-12)),
                "遗漏 {target:?}"
            );
        }
    }

    #[test]
    fn long_orthorhombic_path_packs_endpoints_without_losing_samples() {
        let mut path = Vec::new();
        for axis in 0..3 {
            for i in 0..=80 {
                let mut crystal = [0.; 3];
                crystal[axis] = i as f64 / 160.;
                path.push(PathPoint {
                    segment_id: axis,
                    point_id: path.len(),
                    distance: path.len() as f64,
                    crystal,
                });
            }
        }
        let mesh = (0..2)
            .flat_map(|a| {
                (0..3).flat_map(move |b| {
                    (0..6).map(move |c| [a as f64 / 2., b as f64 / 3., c as f64 / 6.])
                })
            })
            .collect();
        let s = Sampling {
            path,
            ticks: vec![],
            mesh,
            reciprocal: [
                [1., 0., 0.],
                [0., 1.66599609188825, 0.],
                [0., 0., 3.744448649530427],
            ],
        };
        let data = wdata(&s, true, 1).unwrap();
        assert!(data.len() < 150, "{} rows", data.len());
        let mut output = Vec::new();
        let mut reference = 0.;
        for row in &data[3..data.len() - 1] {
            let fields: Vec<_> = row.split_whitespace().collect();
            let a: [f64; 3] = std::array::from_fn(|j| fields[j + 1].parse().unwrap());
            let b: [f64; 3] = std::array::from_fn(|j| fields[j + 5].parse().unwrap());
            let d = length(a, b, &s.reciprocal);
            if reference == 0. {
                reference = d;
            }
            let n = (d / reference).round() as usize;
            assert!(n > 0);
            for i in 0..=n {
                output.push(std::array::from_fn(|j| {
                    a[j] + (b[j] - a[j]) * i as f64 / n as f64
                }));
            }
        }
        for target in s.path.iter().map(|p| p.crystal).chain(s.mesh) {
            assert!(
                output.iter().any(|p: &[f64; 3]| same(p, &target)),
                "遗漏 {target:?}"
            );
        }
    }

    #[test]
    #[ignore = "显式提供既有 MgB2 配置，只读检查 wdata，不运行科学程序"]
    fn real_workspace_wdata_readonly() {
        let path = std::env::var_os("DFTFLOW_0050_REAL_CONFIG").expect("显式配置");
        let cfg = Config::load(Path::new(&path)).unwrap();
        let sampling = plan(&cfg).unwrap();
        let data = wdata(&sampling, true, 1).unwrap();
        eprintln!(
            "path={} mesh={} sampling_wdata={}",
            sampling.path.len(),
            sampling.mesh.len(),
            data.len()
        );
        let epw = super::super::epw_config::EpwConfig::from_config(&cfg).unwrap();
        let qe = crate::qe_data::qe_config::QeSettings::from_config(&cfg).unwrap();
        let states = nscf::parse_xml(&xml(&cfg)).unwrap();
        let input = epw
            .with_fermi(states.fermi)
            .unwrap()
            .with_bands(&states.eigenvalues)
            .unwrap()
            .with_species(&qe.cell.elements)
            .unwrap()
            .native_input(
                super::super::epw_config::NativeStage::WannierOnly,
                "./out",
                "./save",
                cfg.interpolation.phonon_smearings_mev[0],
            )
            .unwrap();
        assert!(input.contains("bands_plot = true"));
    }
}
