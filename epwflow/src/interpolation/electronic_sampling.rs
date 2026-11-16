//! EPW 内嵌 Wannier90 的高对称路径声明及原生输出读取。
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
        cfg.native_value::<u32>(&[
            crate::foundation::native_config::ELECTRONIC,
            "workflow",
            "points_per_edge",
        ])? as usize,
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
/// 只传递真实高对称端点；内部采样和 DOS 网格不写入路径声明。
pub fn wdata(
    path: &Path,
    bands_plot: bool,
    bands_num_points: u32,
    reciprocal: &[[f64; 3]; 3],
) -> Result<Vec<String>> {
    ensure!(
        bands_num_points > 0,
        "configuration: bands_num_points 必须为正整数"
    );
    let edges = high_symmetry_edges(path)?;
    let first_length = length(edges[0].start, edges[0].end, reciprocal);
    ensure!(
        first_length.is_finite() && first_length > 1e-12,
        "data_invalid: {} 首段长度无效",
        path.display()
    );
    for edge in &edges {
        let intervals = (bands_num_points as f64 * length(edge.start, edge.end, reciprocal)
            / first_length)
            .round();
        ensure!(intervals.is_finite() && intervals >= 1.,
            "configuration: {}:{} 在 bands_num_points={} 下被 Wannier90 舍入为零个间隔，请提高 bands_num_points",
            path.display(), edge.line, bands_num_points);
    }
    let mut data = vec![
        format!("bands_plot = {bands_plot}"),
        format!("bands_num_points = {bands_num_points}"),
        "bands_plot_format = gnuplot".into(),
        "begin kpoint_path".into(),
    ];
    let point = |p: [f64; 3]| {
        p.iter()
            .map(|v| {
                let value = format!("{v:.10}");
                let value = value.trim_end_matches('0').trim_end_matches('.');
                if value == "-0" {
                    "0".into()
                } else {
                    value.to_owned()
                }
            })
            .collect::<Vec<String>>()
            .join(" ")
    };
    for edge in edges {
        let row = format!(
            "{} {} {} {}",
            edge.start_label,
            point(edge.start),
            edge.end_label,
            point(edge.end)
        );
        ensure!(
            row.len() <= 79,
            "configuration: {}:{} 的高对称路径行超过部署 EPW 的 79 字节限制，不能截断标签或坐标",
            path.display(),
            edge.line
        );
        data.push(row);
    }
    data.push("end kpoint_path".into());
    Ok(data)
}

/// 输入仍为现有 KPATH.in 端点对格式；不把插值点反推成高对称端点。
#[derive(Clone, Debug)]
pub(crate) struct PathEdge {
    pub start: [f64; 3],
    pub end: [f64; 3],
    pub start_label: String,
    pub end_label: String,
    pub segment_id: usize,
    pub line: usize,
}
pub(crate) fn high_symmetry_edges(path: &Path) -> Result<Vec<PathEdge>> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("required: 无法读取 {}", path.display()))?;
    let lines: Vec<_> = text
        .lines()
        .enumerate()
        .filter_map(|(i, raw)| {
            let line = raw.split('#').next().unwrap_or("").trim();
            (!line.is_empty()).then_some((i, line))
        })
        .collect();
    // 标题可以自定义；完整四行结构才是文件头，不能静默跳过正文错误。
    let header_len = if lines.len() >= 4
        && lines[1].1.parse::<u32>().is_ok_and(|n| n > 0)
        && lines[2].1.eq_ignore_ascii_case("Line-Mode")
        && lines[3].1.eq_ignore_ascii_case("Reciprocal")
    {
        4
    } else {
        0
    };
    let mut endpoints = Vec::new();
    for (i, line) in lines.into_iter().skip(header_len) {
        let cells: Vec<_> = line.split_whitespace().collect();
        ensure!(
            cells.len() == 4,
            "data_invalid: {}:{} 应为 kx ky kz label",
            path.display(),
            i + 1
        );
        let coords = (0..3)
            .map(|j| cells[j].parse::<f64>())
            .collect::<std::result::Result<Vec<_>, _>>()
            .with_context(|| format!("data_invalid: {}:{} 端点坐标无效", path.display(), i + 1))?;
        ensure!(
            coords.iter().all(|x| x.is_finite()),
            "data_invalid: {}:{} 端点坐标非有限",
            path.display(),
            i + 1
        );
        ensure!(
            cells[3].len() <= 20
                && cells[3].is_ascii()
                && cells[3]
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_'),
            "data_invalid: {}:{} 标签应为最多 20 字节的 ASCII 字母、数字或下划线",
            path.display(),
            i + 1
        );
        endpoints.push((
            [coords[0], coords[1], coords[2]],
            cells[3].to_owned(),
            i + 1,
        ));
    }
    ensure!(
        !endpoints.is_empty() && endpoints.len() % 2 == 0,
        "data_invalid: {} 高对称端点为空或不成对",
        path.display()
    );
    let mut edges: Vec<PathEdge> = Vec::new();
    let mut segment_id = 0;
    for pair in endpoints.chunks_exact(2) {
        ensure!(
            pair[0]
                .0
                .iter()
                .zip(pair[1].0)
                .any(|(a, b)| (a - b).abs() > 1e-12),
            "data_invalid: {}:{} 路径包含零长度边",
            path.display(),
            pair[0].2
        );
        if let Some(previous) = edges.last() {
            if previous.end_label != pair[0].1
                || previous
                    .end
                    .iter()
                    .zip(pair[0].0)
                    .any(|(a, b)| (a - b).abs() > 1e-10)
            {
                segment_id += 1;
            }
        }
        edges.push(PathEdge {
            start: pair[0].0,
            end: pair[1].0,
            start_label: pair[0].1.clone(),
            end_label: pair[1].1.clone(),
            segment_id,
            line: pair[0].2,
        });
    }
    Ok(edges)
}
/// 原生 labelinfo 给出真实端点的行号；逐段核对 kpt，不依赖两侧采样点数一致。
pub(crate) fn native_path_points(
    edges: &[PathEdge],
    reciprocal: &[[f64; 3]; 3],
    labelinfo: &str,
    coords: &[[f64; 3]],
    gap_intervals: u32,
) -> Result<(Vec<PathPoint>, Vec<Tick>)> {
    ensure!(
        gap_intervals > 0 && reciprocal.iter().flatten().all(|x| x.is_finite()),
        "data_invalid: 原生路径度量无效"
    );
    let mut labels = Vec::new();
    for line in labelinfo.lines().filter(|line| !line.trim().is_empty()) {
        let cells: Vec<_> = line.split_whitespace().collect();
        ensure!(
            cells.len() == 6,
            "data_invalid: 原生 labelinfo 应含标签、点序、距离及三维坐标"
        );
        let index = cells[1]
            .parse::<usize>()?
            .checked_sub(1)
            .context("data_invalid: 原生端点序号应从 1 开始")?;
        let values = crate::qe_data::xml::numbers(&cells[2..].join(" "))?;
        ensure!(
            values.len() == 4 && values.iter().all(|x| x.is_finite()) && index < coords.len(),
            "data_invalid: 原生 labelinfo 点序或坐标无效"
        );
        labels.push((
            cells[0].to_owned(),
            index,
            [values[1], values[2], values[3]],
        ));
    }
    let mut cursor = 0;
    let mut end_index = 0;
    let mut distance = 0.;
    let mut points: Vec<PathPoint> = Vec::new();
    let mut ticks = Vec::new();
    for (i, edge) in edges.iter().enumerate() {
        let new_segment = i == 0 || edge.segment_id != edges[i - 1].segment_id;
        let start_index = if new_segment {
            let record = labels
                .get(cursor)
                .context("data_invalid: 原生 labelinfo 缺少起点")?;
            ensure!(
                record.0.eq_ignore_ascii_case(&edge.start_label) && same(&record.2, &edge.start),
                "input_conflict: 原生起点标签或坐标不同：{}",
                edge.start_label
            );
            cursor += 1;
            record.1
        } else {
            end_index
        };
        let end = labels
            .get(cursor)
            .context("data_invalid: 原生 labelinfo 缺少终点")?;
        ensure!(
            end.0.eq_ignore_ascii_case(&edge.end_label) && same(&end.2, &edge.end),
            "input_conflict: 原生终点标签或坐标不同：{}",
            edge.end_label
        );
        cursor += 1;
        ensure!(
            end.1 > start_index
                && start_index
                    == if i == 0 {
                        0
                    } else if new_segment {
                        end_index + 1
                    } else {
                        end_index
                    },
            "data_invalid: 原生路径端点序号不连续或为空"
        );
        end_index = end.1;
        let intervals = end_index - start_index;
        let size = length(edge.start, edge.end, reciprocal);
        ensure!(
            size.is_finite() && size > 1e-12,
            "data_invalid: 原生路径零长度"
        );
        if new_segment {
            if !points.is_empty() {
                distance += size / gap_intervals as f64;
            }
            ticks.push(Tick {
                distance,
                label: edge.start_label.clone(),
            });
        }
        let start_distance = distance;
        for index in start_index..=end_index {
            let t = (index - start_index) as f64 / intervals as f64;
            let expected =
                std::array::from_fn(|j| edge.start[j] + (edge.end[j] - edge.start[j]) * t);
            ensure!(
                same(&expected, &coords[index]),
                "input_conflict: 原生 kpt 第 {} 点偏离 {}–{} 声明路径",
                index + 1,
                edge.start_label,
                edge.end_label
            );
            if index == start_index && !new_segment {
                continue;
            }
            points.push(PathPoint {
                segment_id: edge.segment_id,
                point_id: points.len(),
                distance: start_distance + size * t,
                crystal: expected,
            });
        }
        distance += size;
        ticks.push(Tick {
            distance,
            label: edge.end_label.clone(),
        });
    }
    ensure!(
        !points.is_empty() && points.len() == coords.len() && cursor == labels.len(),
        "data_invalid: 原生路径包含多余或缺失的采样／端点"
    );
    Ok((points, ticks))
}
fn same(a: &[f64; 3], b: &[f64; 3]) -> bool {
    a.iter().zip(b).all(|(a, b)| (a - b).abs() <= 5.1e-7)
}
/// W90 kpt 保留六位小数；拒绝不能唯一分辨的不同目标位置。
pub fn extract(cfg: &Config, dir: &Path) -> Result<()> {
    let mut sampling = plan(cfg)?;
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
    let decision = crate::foundation::stages::Step::Wannier
        .directory(cfg)
        .join("embedded-sampling-decision.json");
    let sparse = decision.is_file()
        && store::read::<serde_json::Value>(&decision)?["path_format"] == "high_symmetry_endpoints";
    let targets: Vec<[f64; 3]> = if sparse {
        let edges = high_symmetry_edges(cfg.kpath.as_ref().context("required: 缺少 KPATH.in")?)?;
        let (path, ticks) = native_path_points(
            &edges,
            &sampling.reciprocal,
            &fs::read_to_string(dir.join(format!("{}_band.labelinfo.dat", cfg.qe.prefix)))?,
            &coords,
            cfg.native_value(&[
                crate::foundation::native_config::ELECTRONIC,
                "workflow",
                "points_per_edge",
            ])?,
        )?;
        sampling.path = path;
        sampling.ticks = ticks;
        sampling.path.iter().map(|p| p.crystal).collect()
    } else {
        sampling
            .path
            .iter()
            .map(|p| p.crystal)
            .chain(sampling.mesh.iter().copied())
            .collect()
    };
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
        &serde_json::json!({"driver":"epw.x/wdata","path_format":if sparse {"high_symmetry_endpoints"} else {"legacy_samples"},"coordinates":store::Identity::capture(&dir.join(format!("{}_band.kpt",cfg.qe.prefix)))?,"energies":store::Identity::capture(&band)?,"labels":store::Identity::capture(&dir.join(format!("{}_band.labelinfo.dat",cfg.qe.prefix)))?}),
    )?;
    Ok(())
}

pub(crate) fn independent_sampling(dir: &Path) -> Result<bool> {
    let path = dir.join("band-origin.json");
    Ok(path.exists()
        && store::read::<serde_json::Value>(&path)?["path_format"] == "high_symmetry_endpoints")
}

/// 只比较双方实际相同坐标和同一路径位置的原生能量；不补点，也不显示覆盖率。
pub(crate) fn common_difference(
    qe_path: &[PathPoint],
    qe: &[Vec<f64>],
    epw_path: &[PathPoint],
    epw: &[Vec<f64>],
    fermi: f64,
    window: [f64; 2],
) -> Result<super::differences::Statistics> {
    ensure!(
        qe_path.len() == qe.len() && epw_path.len() == epw.len(),
        "data_invalid: 原生电子点数与能量不符"
    );
    let mut points = Vec::new();
    let mut distances = Vec::new();
    let mut reference = Vec::new();
    let mut values = Vec::new();
    for (i, p) in qe_path.iter().enumerate() {
        if let Some((j, _)) = epw_path.iter().enumerate().find(|(_, q)| {
            p.segment_id == q.segment_id
                && same(&p.crystal, &q.crystal)
                && (p.distance - q.distance).abs() < 1e-7
        }) {
            points.push(p.crystal);
            distances.push(p.distance);
            reference.push(qe[i].clone());
            values.push(epw[j].clone());
        }
    }
    super::differences::electronic(
        &points,
        Some(&distances),
        &reference,
        &values,
        fermi,
        window,
    )
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
    const RECIPROCAL: [[f64; 3]; 3] = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    fn path_file(text: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("epw-high-symmetry-{}", store::token()));
        fs::create_dir(&dir).unwrap();
        let path = dir.join("KPATH.in");
        fs::write(&path, text).unwrap();
        path
    }
    #[test]
    fn sparse_path_keeps_real_labels_edges_breaks_and_density_independence() {
        let path = path_file("K-Path Generated by VASPKIT.\n20\nLine-Mode\nReciprocal\n0 0 0 G\n0.5 0 0 M\n0.5 0 0 M\n0.3333333333 0.3333333333 0 K\n0 0 0.5 A\n0.5 0 0.5 L\n");
        let first = wdata(&path, true, 20, &RECIPROCAL).unwrap();
        let second = wdata(&path, true, 100, &RECIPROCAL).unwrap();
        assert_eq!(&first[2..], &second[2..]);
        assert_eq!(first.len(), 8);
        assert_eq!(first[4], "G 0 0 0 M 0.5 0 0");
        assert_eq!(first[6], "A 0 0 0.5 L 0.5 0 0.5");
        assert!(!first
            .iter()
            .any(|line| line.contains("P0") || line.contains("Q0")));
        let edges = high_symmetry_edges(&path).unwrap();
        assert_eq!(
            edges.iter().map(|e| e.segment_id).collect::<Vec<_>>(),
            [0, 0, 1]
        );
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    #[test]
    fn custom_titled_header_preserves_bare_path_and_source_lines() {
        let bare = path_file("0 0 0 G\n0.5 0 0 X\n");
        let expected = wdata(&bare, true, 20, &RECIPROCAL).unwrap();
        for title in ["SiP2 Pbam; historical Wannier path", "自定义路径标题", "0 0 0 G"] {
            let text = format!("# preamble\n\n{title} # title\n 20\n line-mode\n reciprocal # coordinates\n\n0 0 0 G\n0.5 0 0 X\n");
            let path = path_file(&text);
            assert_eq!(wdata(&path, true, 20, &RECIPROCAL).unwrap(), expected);
            assert_eq!(high_symmetry_edges(&path).unwrap()[0].line, 8);
            assert_eq!(fs::read_to_string(&path).unwrap(), text);
            fs::remove_dir_all(path.parent().unwrap()).unwrap();
        }
        fs::remove_dir_all(bare.parent().unwrap()).unwrap();
    }
    #[test]
    fn sip2_original_path_generates_twelve_real_edges() {
        let text = include_str!("../../tests/fixtures/sip2-kpath.txt");
        let path = path_file(text);
        let data = wdata(&path, true, 1, &RECIPROCAL).unwrap();
        assert_eq!(data.len(), 17);
        assert_eq!(data[4], "G 0 0 0 X 0.5 0 0");
        assert_eq!(data[13], "Y 0 0.5 0 T 0 0.5 0.5");
        assert_eq!(data[14], "U 0.5 0 0.5 X 0.5 0 0");
        assert_eq!(data[15], "S 0.5 0.5 0 R 0.5 0.5 0.5");
        assert_eq!(fs::read_to_string(&path).unwrap(), text);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    #[test]
    fn malformed_headers_and_body_lines_are_never_silently_skipped() {
        for (text, line) in [
            ("custom title\n0 0 0 G\n0.5 0 0 X\n", 1),
            ("custom title\n20\nReciprocal\n0 0 0 G\n0.5 0 0 X\n", 1),
            ("custom title\n20\nLine-Mode\nCartesian\n0 0 0 G\n0.5 0 0 X\n", 1),
            ("custom title\n0\nLine-Mode\nReciprocal\n0 0 0 G\n0.5 0 0 X\n", 1),
            ("custom title\n20\nLine-Mode\nReciprocal\n0 0 0 G\n20\n0.5 0 0 X\n", 6),
            ("custom title\n20\nLine-Mode\nReciprocal\n0 0 0 G\nLine-Mode\n0.5 0 0 X\n", 6),
            ("custom title\n20\nLine-Mode\nReciprocal\nK-Path bogus\n0 0 0 G\n0.5 0 0 X\n", 5),
            ("custom title\n20\nLine-Mode\nReciprocal\n0 0 0 G\nNaN 0 0 X\n", 6),
            ("custom title\n20\nLine-Mode\nReciprocal\n0 0 0 G\n0 bad 0 X\n", 6),
            ("custom title\n20\nLine-Mode\nReciprocal\n0 0 0 G\n0.5 0 X\n", 6),
        ] {
            let path = path_file(text);
            let error = format!("{:#}", wdata(&path, true, 20, &RECIPROCAL).unwrap_err());
            assert!(error.contains(&format!("{}:{line} ", path.display())), "{error}");
            fs::remove_dir_all(path.parent().unwrap()).unwrap();
        }
    }
    #[test]
    fn malformed_paths_fail_with_file_and_location() {
        for text in [
            "",
            "0 0 0 G\n",
            "0 0 0 G\n0 0 0 G\n",
            "NaN 0 0 G\n1 0 0 X\n",
            "0 0 G\n1 0 0 X\n",
            "0 bad 0 G\n1 0 0 X\n",
            "0 0 0 WAYTOOLONGLABELTHATEXCEEDS20\n1 0 0 X\n",
        ] {
            let path = path_file(text);
            let error = wdata(&path, true, 20, &RECIPROCAL).unwrap_err();
            assert!(format!("{error:#}").contains(path.to_str().unwrap()));
            fs::remove_dir_all(path.parent().unwrap()).unwrap();
        }
    }
    #[test]
    fn insufficient_density_rejects_zero_interval_native_edges() {
        let path = path_file("0 0 0 G\n0.5 0 0 X\n0.5 0 0 X\n0.5 0.01 0 M\n");
        let error = wdata(&path, true, 1, &RECIPROCAL).unwrap_err();
        assert!(error.to_string().contains("bands_num_points=1"));
        assert!(wdata(&path, true, 100, &RECIPROCAL).is_ok());
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn mgb2_native_input_has_exactly_nine_high_symmetry_edges() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_auto/MgB2/rust-reference/KPATH.in");
        let data = wdata(&path, true, 100, &RECIPROCAL).unwrap();
        assert_eq!(data.len(), 14);
        assert!(data[11].starts_with("L ") && data[12].starts_with("H "));
    }

    #[test]
    fn native_path_uses_actual_endpoint_indices_and_rejects_wrong_or_extra_points() {
        let path =
            path_file("0 0 0 G\n0.5 0 0 X\n0.5 0 0 X\n0.5 0.25 0 M\n0 0 0.5 A\n0.25 0 0.5 L\n");
        let edges = high_symmetry_edges(&path).unwrap();
        let reciprocal = [[1., 0., 0.], [0.3, 1.2, 0.], [0., 0., 0.7]];
        let coords = vec![
            [0., 0., 0.],
            [0.25, 0., 0.],
            [0.5, 0., 0.],
            [0.5, 0.25, 0.],
            [0., 0., 0.5],
            [0.25, 0., 0.5],
        ];
        let labels = "G 1 0 0 0 0\nX 3 0.5 0.5 0 0\nM 4 0.8 0.5 0.25 0\nA 5 0.8 0 0 0.5\nL 6 1.05 0.25 0 0.5\n";
        let (points, ticks) = native_path_points(&edges, &reciprocal, labels, &coords, 20).unwrap();
        assert_eq!(points.len(), 6);
        assert_eq!(
            points.iter().map(|p| p.segment_id).collect::<Vec<_>>(),
            [0, 0, 0, 0, 1, 1]
        );
        assert_eq!(ticks.len(), 5);
        assert!(
            (points[3].distance - 0.5 - (0.075f64.powi(2) + 0.3f64.powi(2)).sqrt()).abs() < 1e-12
        );
        let mut bad = coords.clone();
        bad[1][1] = 0.01;
        assert!(native_path_points(&edges, &reciprocal, labels, &bad, 20).is_err());
        bad = coords.clone();
        bad.push([0., 0., 0.]);
        assert!(native_path_points(&edges, &reciprocal, labels, &bad, 20).is_err());
        assert!(native_path_points(
            &edges,
            &reciprocal,
            &labels.replace("M 4", "M 3"),
            &coords,
            20
        )
        .is_err());
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn only_common_native_coordinates_contribute_energy_differences() {
        let point = |i, x| PathPoint {
            segment_id: 0,
            point_id: i,
            distance: x,
            crystal: [x, 0., 0.],
        };
        let qe = vec![point(0, 0.), point(1, 0.25), point(2, 0.5)];
        let epw = vec![point(0, 0.), point(1, 0.5)];
        let stats = common_difference(
            &qe,
            &[vec![0.], vec![999.], vec![2.]],
            &epw,
            &[vec![1.], vec![3.]],
            0.,
            [-10., 1000.],
        )
        .unwrap();
        assert_eq!(stats.points, 2);
        assert_eq!(stats.maximum.unwrap().absolute_difference, 1.);
        let none = common_difference(
            &qe,
            &[vec![0.], vec![999.], vec![2.]],
            &[point(0, 0.123)],
            &[vec![4.]],
            0.,
            [-10., 1000.],
        )
        .unwrap();
        assert_eq!(none.points, 0);
        assert!(none.maximum.is_none());
    }

    #[test]
    #[ignore = "显式提供既有 MgB2 配置，只读检查 wdata，不运行科学程序"]
    fn real_workspace_wdata_readonly() {
        let path = std::env::var_os("DFTFLOW_0050_REAL_CONFIG").expect("显式配置");
        let cfg = Config::load(Path::new(&path)).unwrap();
        let sampling = plan(&cfg).unwrap();
        let data = wdata(cfg.kpath.as_ref().unwrap(), true, 100, &sampling.reciprocal).unwrap();
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
