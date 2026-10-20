//! 尝试级单 image 并行度；固定科学输入和历史请求身份保持不变。
use super::{config::Config, snapshot::RunBinding};
use anyhow::{ensure, Context, Result};

/// 仅读取显式数量，不替用户修正或补充并行参数。
fn read_count(args: &[String], flags: &[&str], default: u32) -> Result<u32> {
    let mut found = None;
    for (i, value) in args.iter().enumerate() {
        if flags.contains(&value.as_str()) {
            ensure!(found.is_none(), "configuration: 并行参数重复");
            let count = args.get(i+1).context("configuration: 并行参数缺少数量")?.parse::<u32>()?;
            ensure!(count>0, "configuration: 并行数量必须为正");
            found=Some(count);
        } else {
            ensure!(!flags.iter().any(|f| value.starts_with(&format!("{f}="))),
                "configuration: 并行数量需要独立参数值");
        }
    }
    Ok(found.unwrap_or(default))
}

pub fn preflight(cfg: &Config, step: super::stages::Step, allocation: bool) -> Result<()> {
    let mpi=cfg.resources.mpi;
    let launched=read_count(&cfg.programs.launcher, &["-np","-n","--np","--ntasks"],1)?;
    ensure!(launched==mpi, "configuration: MPI 启动数量 {launched} 与 resources.mpi={mpi} 不一致");
    if allocation {
        let tasks=std::env::var("SLURM_NTASKS").context("allocation: 缺少 SLURM_NTASKS")?.parse::<u32>()?;
        ensure!(tasks==mpi,"allocation: Slurm ntasks={tasks} 与 MPI={mpi} 不一致");
    }
    let epw=step.epw();
    let args=if epw { &cfg.programs.epw.args } else if step==super::stages::Step::Phonon { &cfg.programs.ph.args } else { &cfg.programs.pw.args };
    let pools=read_count(args,&["-nk","-npool","--npool","-npools"],1)?;
    let images=read_count(args,&["-ni","-nimage","--nimage"],1)?;
    ensure!(mpi%images==0 && (mpi/images)%pools==0,"configuration: MPI={mpi}, image={images}, pool={pools} 不能整除");
    if epw {
        let grid=if step>=super::stages::Step::Fine {cfg.interpolation.k_grid} else {cfg.qe.nscf.k_grid};
        let mut nk=grid.iter().map(|&x|u64::from(x)).product::<u64>();
        if step<super::stages::Step::Fine {
            let xml=super::layout::path(&cfg.work_dir,"nscf/out").join(format!("{}.save/data-file-schema.xml",cfg.qe.prefix));
            if xml.is_file() && (!super::layout::enabled(&cfg.work_dir) || super::stages::reusable(cfg,super::stages::Step::Nscf)?) {
                let states=crate::qe_data::nscf::parse_xml(&xml)?;
                ensure!(states.kpoints.len() as u64==nk,"data_invalid: 实际 NSCF k 点 {} 与网格 {grid:?}={nk} 不一致",states.kpoints.len());
                nk=states.kpoints.len() as u64;
            }
        }
        ensure!(u64::from(pools)<=nk,
            "configuration: 网格={grid:?}, k 点数={nk}, MPI={mpi}, pool={pools}；存在空 pool 未分配 k 点，可能导致数组越界，停止执行");
        ensure!(images==1 && pools==mpi,
            "configuration: 当前 EPW 单 image 路径要求每 pool 一个 MPI：网格={grid:?}, k 点数={nk}, MPI={mpi}, pool={pools}, image={images}");
    }
    Ok(())
}

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
