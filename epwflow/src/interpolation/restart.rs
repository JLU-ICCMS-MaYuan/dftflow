//! EPW 5.9 的粗网格重启角色；按目标源码 io.f90 的记录布局核验。
use crate::{foundation::store, qe_data::save};
use anyhow::{ensure, Context, Result};
use std::{
    fs,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

pub const FORMAT: &str = "epw5.9-coarse-le-c16";

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

pub fn files(
    dir: &Path,
    out: &Path,
    prefix: &str,
    nwann: usize,
    modes: usize,
) -> Result<Vec<(String, PathBuf)>> {
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
    let mut result = vec![("coarse_epmatwp".into(), matrix)];
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
