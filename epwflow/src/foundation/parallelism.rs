//! 尝试级单 image 并行度；固定科学输入和历史请求身份保持不变。
use super::{config::Config, snapshot::RunBinding};
use anyhow::{ensure, Context, Result};

fn count_arg(args: &mut Vec<String>, flags: &[&str], count: u32, required: bool) -> Result<()> {
    ensure!(
        !args
            .iter()
            .any(|s| !flags.contains(&s.as_str()) && flags.iter().any(|flag| s.starts_with(flag))),
        "configuration: 并行数量请使用独立参数值，拒绝不支持的并行参数写法"
    );
    let indices: Vec<_> = args
        .iter()
        .enumerate()
        .filter_map(|(i, s)| flags.contains(&s.as_str()).then_some(i))
        .collect();
    ensure!(indices.len() <= 1, "configuration: 并行参数重复或有歧义");
    if let Some(&i) = indices.first() {
        let value = args
            .get_mut(i + 1)
            .context("configuration: 并行参数缺少数量")?;
        ensure!(
            value.parse::<u32>().is_ok_and(|n| n > 0),
            "configuration: 并行数量无效"
        );
        *value = count.to_string();
    } else {
        ensure!(
            !required,
            "configuration: MPI 启动器需要显式 -np/-n/--np/--ntasks 数量"
        );
        args.extend([flags[0].to_owned(), count.to_string()]);
    }
    Ok(())
}

pub fn apply(cfg: &mut Config, mpi: u32) -> Result<()> {
    ensure!((1..=64).contains(&mpi), "configuration: MPI 必须在 1..64");
    let nk: u64 = cfg.qe.nscf.k_grid.iter().map(|&n| u64::from(n)).product();
    ensure!(
        u64::from(mpi) <= nk,
        "configuration: pool 数不能超过完整粗 k 点数，禁止空 pool"
    );
    for args in [&cfg.programs.launcher, &cfg.programs.epw.args] {
        let image_flags = ["-ni", "-nimage", "--nimage"];
        ensure!(
            !args.iter().any(|s| !image_flags.contains(&s.as_str())
                && image_flags.iter().any(|flag| s.starts_with(flag))),
            "configuration: MPI 恢复仅支持显式单 image"
        );
        let images: Vec<_> = args
            .iter()
            .enumerate()
            .filter_map(|(i, s)| image_flags.contains(&s.as_str()).then_some(i))
            .collect();
        ensure!(images.len() <= 1, "configuration: image 参数重复");
        if let Some(&i) = images.first() {
            ensure!(
                args.get(i + 1).map(String::as_str) == Some("1"),
                "configuration: MPI 恢复仅支持单 image"
            );
        }
    }
    ensure!(
        !cfg.programs.launcher.is_empty(),
        "configuration: MPI 恢复需要显式启动器"
    );
    count_arg(
        &mut cfg.programs.launcher,
        &["-np", "-n", "--np", "--ntasks"],
        mpi,
        true,
    )?;
    count_arg(
        &mut cfg.programs.epw.args,
        &["-nk", "-npool", "--npool"],
        mpi,
        false,
    )?;
    cfg.resources.mpi = mpi;
    cfg.validate()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> Config {
        toml::from_str(include_str!("../../tests/fixtures/input.toml")).unwrap()
    }
    #[test]
    fn mpi_and_pool_follow_32_points_but_physics_is_preserved() {
        let mut cfg = config();
        cfg.resources.mpi = 64;
        cfg.programs.launcher[2] = "64".into();
        cfg.programs.epw.args = vec!["-nk".into(), "64".into()];
        let qe = serde_json::to_value(&cfg.qe).unwrap();
        apply(&mut cfg, 32).unwrap();
        assert_eq!(cfg.resources.mpi, 32);
        assert_eq!(cfg.programs.launcher[2], "32");
        assert_eq!(cfg.programs.epw.args, ["-nk", "32"]);
        assert_eq!(serde_json::to_value(&cfg.qe).unwrap(), qe);
        assert!(apply(&mut cfg, 64).is_err());
    }
    #[test]
    fn ambiguous_or_multi_image_layout_is_rejected() {
        for args in [
            vec!["-nk", "16", "-npool", "16"],
            vec!["-nk=16"],
            vec!["-ni", "2"],
            vec!["-ni=2"],
        ] {
            let mut cfg = config();
            cfg.programs.epw.args = args.into_iter().map(str::to_owned).collect();
            assert!(apply(&mut cfg, 16).is_err());
        }
    }
}

/// 仅为重验原请求恢复原固定身份；执行仍使用当前尝试的并行参数。
pub fn request_identity(cfg: &Config, launcher: &[String]) -> Result<(Config, Vec<String>)> {
    if !cfg.work_dir.join("run-binding.json").is_file() {
        return Ok((cfg.clone(), launcher.to_vec()));
    }
    let binding = RunBinding::load(&cfg.work_dir)?;
    binding.verify()?;
    let original = Config::load(&cfg.work_dir.join("input-init.toml"))?;
    if original.resources.mpi == cfg.resources.mpi {
        return Ok((cfg.clone(), launcher.to_vec()));
    }
    let mut allowed = original.clone();
    apply(&mut allowed, cfg.resources.mpi)?;
    ensure!(
        serde_json::to_value(&allowed)? == serde_json::to_value(cfg)?
            && launcher == cfg.programs.launcher,
        "input_conflict: 仅允许尝试级 MPI/pool 变化，固定科学配置或启动器不相容"
    );
    let original_launcher = original.programs.launcher.clone();
    Ok((original, original_launcher))
}
