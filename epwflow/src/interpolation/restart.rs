//! EPW 5.9 的粗网格重启角色；按目标源码 io.f90 的记录布局核验。
use crate::{foundation::store, qe_data::save};
use anyhow::{ensure, Context, Result};
use std::{
    fs,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

pub const FORMAT: &str = "epw5.9-coarse-le-c16";

/// vmebloch2wan 无条件读取这组原生近邻与重叠矩阵；布局来自 EPW 5.9。
pub fn velocity_files(dir: &Path, prefix: &str, points: usize, bands: usize) -> Result<()> {
    let text = fs::read_to_string(dir.join(format!("{prefix}.bvec")))?;
    let mut rows = text.lines();
    ensure!(rows.next().is_some(), "data_invalid: bvec 缺头");
    let dims = rows.next().context("data_invalid: bvec 缺尺寸")?
        .split_whitespace().map(str::parse::<usize>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(dims.len() == 2 && dims[0] == points && dims[1] > 0 && bands > 0,
        "data_invalid: bvec k 点或近邻数与当前 NSCF 不相容");
    let count = points.checked_mul(dims[1]).context("data_invalid: bvec 尺寸溢出")?;
    for _ in 0..count {
        ensure!(numbers(rows.next().context("data_invalid: bvec 向量截断")?)?.len() == 4,
            "data_invalid: bvec 向量／权重维度错误");
    }
    ensure!(rows.all(|s| s.trim().is_empty()), "data_invalid: bvec 多余记录");
    let count = count.checked_mul(bands).and_then(|n| n.checked_mul(bands))
        .context("data_invalid: mmn 尺寸溢出")?;
    let text = fs::read_to_string(dir.join(format!("{prefix}.mmn")))?;
    let mut rows = text.lines();
    for _ in 0..count {
        let row = rows.next().context("data_invalid: EPW mmn 重叠矩阵截断")?.trim();
        ensure!(row.starts_with('(') && row.ends_with(')') && numbers(row)?.len() == 2,
            "data_invalid: EPW mmn 复数布局错误");
    }
    ensure!(rows.all(|s| s.trim().is_empty()), "data_invalid: EPW mmn 多余记录");
    Ok(())
}

fn numbers(text: &str) -> Result<Vec<f64>> {
    text.replace(['(', ')', ','], " ")
        .split_whitespace()
        .map(|s| {
            let n: f64 = s.replace(['d', 'D'], "e").parse()?;
            ensure!(n.is_finite(), "data_invalid: 粗网格数据含非有限数");
            Ok(n)
        })
        .collect()
}

/// EPW 主程序即使 epwread=true 也先调用 loadbm；必须交付原 .ukk 能带流形。
pub fn band_manifold(path: &Path, nwann: usize) -> Result<()> {
    manifold_dimensions(path,nwann).map(|_| ())
}

pub fn manifold_dimensions(path: &Path, nwann: usize) -> Result<(usize,Vec<usize>)> {
    let text = fs::read_to_string(path)?;
    let mut rows = text.lines().map(str::trim).filter(|s| !s.is_empty()).peekable();
    let header = rows.next().context("data_invalid: 缺 ukk 头")?.split_whitespace()
        .map(str::parse::<usize>).collect::<std::result::Result<Vec<_>,_>>()?;
    ensure!(header.len() == 2 && header[0] >= nwann && nwann > 0, "data_invalid: ukk 带数错误");
    let bands = header[0];
    let kept = (0..bands).map(|_| rows.next().context("data_invalid: ukk 带映射截断")?
        .parse::<usize>().map_err(Into::into)).collect::<Result<Vec<_>>>()?;
    ensure!(kept[0] > 0 && kept.windows(2).all(|w| w[0] < w[1]), "data_invalid: ukk 带映射无效");
    let mut rotations = 0;
    while rows.peek().is_some_and(|s| s.starts_with('(')) {
        ensure!(numbers(rows.next().unwrap())?.len() == 2, "data_invalid: ukk 复数布局错误");
        rotations += 1;
    }
    ensure!(rotations > 0 && rotations % (bands * nwann) == 0, "data_invalid: ukk 旋转矩阵截断");
    let points = rotations / (bands * nwann);
    let mut flags = vec![];
    while rows.peek().is_some_and(|s| matches!(*s, "T" | "F")) {
        flags.push(rows.next().unwrap() == "T");
    }
    let window_size = points * bands;
    ensure!(flags.len() > window_size, "data_invalid: ukk 窗口／排除带截断");
    let excluded = &flags[window_size..];
    ensure!(header[1] <= excluded.len()
        && excluded.iter().enumerate().filter(|(_, x)| !**x).map(|(i, _)| i+1).eq(kept.iter().copied())
        && flags[..window_size].chunks_exact(bands).all(|w| w.iter().filter(|v| **v).count() >= nwann),
        "data_invalid: ukk 流形／排除带不相容");
    for _ in 0..nwann {
        ensure!(numbers(rows.next().context("data_invalid: ukk 中心截断")?)?.len() == 3, "data_invalid: ukk 中心维度错误");
    }
    ensure!(rows.next().is_none(), "data_invalid: ukk 多余记录");
    Ok((points, kept))
}

pub fn files(
    dir: &Path,
    out: &Path,
    prefix: &str,
    nwann: usize,
    modes: usize,
) -> Result<Vec<(String, PathBuf)>> {
    band_manifold(&dir.join(format!("{prefix}.ukk")), nwann)?;
    let text = fs::read_to_string(dir.join("epwdata.fmt"))?;
    let mut lines = text.lines();
    let ef = numbers(lines.next().context("data_invalid: 缺少粗网格 Fermi 能")?)?;
    ensure!(ef.len() == 1, "data_invalid: 粗网格 Fermi 维度错误");
    let header = lines
        .next()
        .context("data_invalid: 缺粗网格尺寸")?
        .split_whitespace()
        .map(str::parse::<usize>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(
        header.len() == 5
            && header.iter().all(|n| *n > 0)
            && header[0] == nwann
            && header[2] == modes,
        "input_conflict: 粗网格 Wannier／模式维度不符"
    );
    let (nk, nq, ng) = (header[1], header[3], header[4]);
    let rest = numbers(&lines.collect::<Vec<_>>().join("\n"))?;
    let expected = (modes / 3) * 9 + 9 + 2 * nwann * nwann * nk + 2 * modes * modes * nq;
    ensure!(
        rest.len() == expected,
        "data_invalid: epwdata.fmt 截断或布局不符"
    );
    let crystal = fs::read_to_string(dir.join("crystal.fmt"))?;
    let dims = crystal
        .lines()
        .take(2)
        .map(|s| s.trim().parse::<usize>())
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(
        dims == [modes / 3, modes],
        "data_invalid: crystal.fmt 原子／模式数不符"
    );
    let wigner = fs::read_to_string(dir.join("wigner.fmt"))?;
    let mut rows = wigner.lines();
    let dims = rows
        .next()
        .context("data_invalid: 缺 Wigner 尺寸")?
        .split_whitespace()
        .map(str::parse::<usize>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(
        dims.len() == 5
            && dims[..3] == [nk, nq, ng]
            && ((dims[3] == 1 && dims[4] == 1) || (dims[3] == nwann && dims[4] == modes / 3)),
        "data_invalid: Wigner 尺寸不符"
    );
    for (count, rows_per, cols) in [
        (nk, dims[3], dims[3]),
        (nq, dims[4], dims[4]),
        (ng, dims[3], dims[4]),
    ] {
        for _ in 0..count {
            let row = numbers(rows.next().context("data_invalid: Wigner 向量截断")?)?;
            ensure!(
                row.len() == 4 && row[..3].iter().all(|x| x.fract() == 0.0) && row[3] >= 0.0,
                "data_invalid: Wigner 向量无效"
            );
            for _ in 0..rows_per {
                let weights = rows
                    .next()
                    .context("data_invalid: Wigner 简并截断")?
                    .split_whitespace()
                    .map(str::parse::<u32>)
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                ensure!(weights.len() == cols, "data_invalid: Wigner 简并维度不符");
            }
        }
    }
    ensure!(
        rows.all(|s| s.trim().is_empty()),
        "data_invalid: Wigner 多余数据"
    );
    for (name, factor) in [("vmedata.fmt", 12), ("dmedata.fmt", 6)] {
        ensure!(
            numbers(&fs::read_to_string(dir.join(name))?)?.len() == factor * nwann * nwann * nk,
            "data_invalid: {name} 截断或维度不符"
        );
    }
    let matrix = out.join(format!("{prefix}.epmatwp"));
    let size = [
        16u64,
        nwann as u64,
        nwann as u64,
        nk as u64,
        modes as u64,
        ng as u64,
    ]
    .into_iter()
    .try_fold(1u64, |p, n| p.checked_mul(n))
    .context("data_invalid: 粗网格尺寸溢出")?;
    ensure!(
        fs::metadata(&matrix)?.len() == size,
        "data_invalid: epmatwp 截断或非支持布局"
    );
    let mut input = BufReader::new(fs::File::open(&matrix)?);
    let mut buffer = [0u8; 65536];
    let mut remaining = size;
    while remaining > 0 {
        let n = remaining.min(buffer.len() as u64) as usize;
        input.read_exact(&mut buffer[..n])?;
        remaining -= n as u64;
        ensure!(
            n % 8 == 0
                && buffer[..n]
                    .chunks_exact(8)
                    .all(|b| f64::from_le_bytes(b.try_into().unwrap()).is_finite()),
            "data_invalid: epmatwp 非有限或截断"
        );
    }
    let mut result = vec![("coarse_epmatwp".into(), matrix),
        ("coarse_ukk".into(), dir.join(format!("{prefix}.ukk")))];
    for name in [
        "epwdata.fmt",
        "crystal.fmt",
        "wigner.fmt",
        "vmedata.fmt",
        "dmedata.fmt",
    ] {
        result.push((
            format!("coarse_{}", name.trim_end_matches(".fmt")),
            dir.join(name),
        ));
    }
    Ok(result)
}

/// 每个展宽在独立目录重读同一规范；完整核验后复制，绝不共享可写数据。
pub fn stage(
    dir: &Path,
    out: &Path,
    prefix: &str,
    nwann: usize,
    modes: usize,
    target: &Path,
) -> Result<()> {
    let sources = files(dir, out, prefix, nwann, modes)?;
    ensure!(
        !target.exists(),
        "input_conflict: 细网格目标已存在，保留现场"
    );
    fs::create_dir_all(target.join("out"))?;
    for (_, src) in sources {
        let hash = store::hash_file(&src)?;
        let dst = if src.parent() == Some(out) {
            target.join("out").join(src.file_name().unwrap())
        } else {
            target.join(src.file_name().unwrap())
        };
        fs::copy(&src, &dst)?;
        ensure!(
            store::hash_file(&src)? == hash && store::hash_file(&dst)? == hash,
            "input_conflict: 粗网格复制身份变化"
        );
    }
    save::nonempty(&target.join("epwdata.fmt"))?;
    Ok(())
}
