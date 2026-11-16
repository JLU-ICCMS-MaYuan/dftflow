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
    ensure!(rows.next().is_some(), "data_invalid: bvec Short head");
    let dims = rows.next().context("data_invalid: bvec Short Size")?
        .split_whitespace().map(str::parse::<usize>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(dims.len() == 2 && dims[0] == points && dims[1] > 0 && bands > 0,
        "data_invalid: bvec k Point or Near Neighbourhood Number and Current NSCF It's incompatible");
    let count = points.checked_mul(dims[1]).context("data_invalid: bvec Size Spill")?;
    for _ in 0..count {
        ensure!(numbers(rows.next().context("data_invalid: bvec Vector Cut")?)?.len() == 4,
            "data_invalid: bvec Error vector/weight dimension");
    }
    ensure!(rows.all(|s| s.trim().is_empty()), "data_invalid: bvec Excess records");
    let count = count.checked_mul(bands).and_then(|n| n.checked_mul(bands))
        .context("data_invalid: mmn Size Spill")?;
    let text = fs::read_to_string(dir.join(format!("{prefix}.mmn")))?;
    let mut rows = text.lines();
    for _ in 0..count {
        let row = rows.next().context("data_invalid: EPW mmn Overlap matrix cut")?.trim();
        ensure!(row.starts_with('(') && row.ends_with(')') && numbers(row)?.len() == 2,
            "data_invalid: EPW mmn Bulk Layout Error");
    }
    ensure!(rows.all(|s| s.trim().is_empty()), "data_invalid: EPW mmn Excess records");
    Ok(())
}

fn numbers(text: &str) -> Result<Vec<f64>> {
    text.replace(['(', ')', ','], " ")
        .split_whitespace()
        .map(|s| {
            let n: f64 = s.replace(['d', 'D'], "e").parse()?;
            ensure!(n.is_finite(), "data_invalid: coarse grid data is not finite");
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
    let header = rows.next().context("data_invalid: Missing ukk Head")?.split_whitespace()
        .map(str::parse::<usize>).collect::<std::result::Result<Vec<_>,_>>()?;
    ensure!(header.len() == 2 && header[0] >= nwann && nwann > 0, "data_invalid: ukk Error with number");
    let bands = header[0];
    let kept = (0..bands).map(|_| rows.next().context("data_invalid: ukk Map Break")?
        .parse::<usize>().map_err(Into::into)).collect::<Result<Vec<_>>>()?;
    ensure!(kept[0] > 0 && kept.windows(2).all(|w| w[0] < w[1]), "data_invalid: ukk Map invalid");
    let mut rotations = 0;
    while rows.peek().is_some_and(|s| s.starts_with('(')) {
        ensure!(numbers(rows.next().unwrap())?.len() == 2, "data_invalid: ukk Bulk Layout Error");
        rotations += 1;
    }
    ensure!(rotations > 0 && rotations % (bands * nwann) == 0, "data_invalid: ukk Rotation Matrix Break");
    let points = rotations / (bands * nwann);
    let mut flags = vec![];
    while rows.peek().is_some_and(|s| matches!(*s, "T" | "F")) {
        flags.push(rows.next().unwrap() == "T");
    }
    let window_size = points * bands;
    ensure!(flags.len() > window_size, "data_invalid: ukk Window/Exclusion Belt Cut");
    let excluded = &flags[window_size..];
    ensure!(header[1] <= excluded.len()
        && excluded.iter().enumerate().filter(|(_, x)| !**x).map(|(i, _)| i+1).eq(kept.iter().copied())
        && flags[..window_size].chunks_exact(bands).all(|w| w.iter().filter(|v| **v).count() >= nwann),
        "data_invalid: ukk Fluid/exclusion band incompatibility");
    for _ in 0..nwann {
        ensure!(numbers(rows.next().context("data_invalid: ukk Center cut")?)?.len() == 3, "data_invalid: ukk Centre dimension error");
    }
    ensure!(rows.next().is_none(), "data_invalid: ukk Excess records");
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
    let ef = numbers(lines.next().context("data_invalid: Missing coarse grid Fermi Yes")?)?;
    ensure!(ef.len() == 1, "data_invalid: coarse Grid Fermi Dimension Error");
    let header = lines
        .next()
        .context("data_invalid: Short of coarse grid size")?
        .split_whitespace()
        .map(str::parse::<usize>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(
        header.len() == 5
            && header.iter().all(|n| *n > 0)
            && header[0] == nwann
            && header[2] == modes,
        "input_conflict: coarse Grid Wannier /Mode dimensions do not match"
    );
    let (nk, nq, ng) = (header[1], header[3], header[4]);
    let rest = numbers(&lines.collect::<Vec<_>>().join("\n"))?;
    let expected = (modes / 3) * 9 + 9 + 2 * nwann * nwann * nk + 2 * modes * modes * nq;
    ensure!(
        rest.len() == expected,
        "data_invalid: epwdata.fmt Intercept or layout does not match"
    );
    let crystal = fs::read_to_string(dir.join("crystal.fmt"))?;
    let dims = crystal
        .lines()
        .take(2)
        .map(|s| s.trim().parse::<usize>())
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(
        dims == [modes / 3, modes],
        "data_invalid: crystal.fmt Atoms/Models do not match"
    );
    let wigner = fs::read_to_string(dir.join("wigner.fmt"))?;
    let mut rows = wigner.lines();
    let dims = rows
        .next()
        .context("data_invalid: Missing Wigner Dimensions")?
        .split_whitespace()
        .map(str::parse::<usize>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(
        dims.len() == 5
            && dims[..3] == [nk, nq, ng]
            && ((dims[3] == 1 && dims[4] == 1) || (dims[3] == nwann && dims[4] == modes / 3)),
        "data_invalid: Wigner The size doesn't match"
    );
    for (count, rows_per, cols) in [
        (nk, dims[3], dims[3]),
        (nq, dims[4], dims[4]),
        (ng, dims[3], dims[4]),
    ] {
        for _ in 0..count {
            let row = numbers(rows.next().context("data_invalid: Wigner Vector Cut")?)?;
            ensure!(
                row.len() == 4 && row[..3].iter().all(|x| x.fract() == 0.0) && row[3] >= 0.0,
                "data_invalid: Wigner Vector invalid"
            );
            for _ in 0..rows_per {
                let weights = rows
                    .next()
                    .context("data_invalid: Wigner Simplistic and cut")?
                    .split_whitespace()
                    .map(str::parse::<u32>)
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                ensure!(weights.len() == cols, "data_invalid: Wigner Simplicity does not match");
            }
        }
    }
    ensure!(
        rows.all(|s| s.trim().is_empty()),
        "data_invalid: Wigner Extra data"
    );
    for (name, factor) in [("vmedata.fmt", 12), ("dmedata.fmt", 6)] {
        ensure!(
            numbers(&fs::read_to_string(dir.join(name))?)?.len() == factor * nwann * nwann * nk,
            "data_invalid: {name} Intersection or dimension mismatch"
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
    .context("data_invalid: coarse grid size spills")?;
    ensure!(
        fs::metadata(&matrix)?.len() == size,
        "data_invalid: epmatwp Interrupted or Non-Supported Layout"
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
            "data_invalid: epmatwp Unlimited or Interrupted"
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
        crate::foundation::native_jobs::empty_execution_directory(target)?,
        "input_conflict: A fine grid target exists. Stay on site"
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
            "input_conflict: coarse Grid Copy Identity Change"
        );
    }
    save::nonempty(&target.join("epwdata.fmt"))?;
    Ok(())
}
