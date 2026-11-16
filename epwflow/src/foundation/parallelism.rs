//! 尝试级单 image 并行度；固定科学输入和历史请求身份保持不变。
use super::{config::Config, snapshot::RunBinding};
use anyhow::{ensure, Context, Result};

/// 仅供原生并行约束与展示，不生成调度参数或修改启动命令。
pub fn mpi_count(cfg: &Config) -> Result<u32> {
    read_count(&cfg.programs.launcher, &["-np", "-n", "--np", "--ntasks"], 1)
}

/// 仅读取显式数量，不替用户修正或补充并行参数。
pub(super) fn read_count(args: &[String], flags: &[&str], default: u32) -> Result<u32> {
    let mut found = None;
    for (i, value) in args.iter().enumerate() {
        if flags.contains(&value.as_str()) {
            ensure!(found.is_none(), "configuration: duplicate parallel parameter");
            let count = args.get(i+1).context("configuration: parallel parameter count missing")?.parse::<u32>()?;
            ensure!(count>0, "configuration: parallel process count must be positive");
            found=Some(count);
        } else {
            ensure!(!flags.iter().any(|f| value.starts_with(&format!("{f}="))),
                "configuration: parallel process count requires a separate argument value");
        }
    }
    Ok(found.unwrap_or(default))
}

pub fn preflight(cfg: &Config, step: super::stages::Step, allocation: bool) -> Result<()> {
    let mpi = mpi_count(cfg)?;
    // 分配状态由调度器校验；Slurm 分配可以大于实际启动数量。
    let _ = allocation;
    let epw=step.epw();
    let args=if epw { &cfg.programs.epw.args } else if step==super::stages::Step::Phonon { &cfg.programs.ph.args } else { &cfg.programs.pw.args };
    let pools=read_count(args,&["-nk","-npool","--npool","-npools"],1)?;
    let images=read_count(args,&["-ni","-nimage","--nimage"],1)?;
    ensure!(mpi%images==0 && (mpi/images)%pools==0,"configuration: MPI={mpi}, image={images}, pool={pools} are not divisible");
    if epw {
        let grid=if step>=super::stages::Step::Fine {cfg.interpolation.k_grid} else {cfg.qe.nscf.k_grid};
        let mut nk=grid.iter().map(|&x|u64::from(x)).product::<u64>();
        if step<super::stages::Step::Fine {
            let xml=super::layout::path(&cfg.work_dir,"nscf/out").join(format!("{}.save/data-file-schema.xml",cfg.qe.prefix));
            if xml.is_file() && (!super::layout::enabled(&cfg.work_dir) || super::stages::reusable(cfg,super::stages::Step::Nscf)?) {
                let states=crate::qe_data::nscf::parse_xml(&xml)?;
                ensure!(states.kpoints.len() as u64==nk,"data_invalid: actual NSCF k-point count {} differs from grid {grid:?}={nk}",states.kpoints.len());
                nk=states.kpoints.len() as u64;
            }
        }
        ensure!(u64::from(pools)<=nk,
            "configuration: grid={grid:?}, k points={nk}, MPI={mpi}, pool={pools}; empty pools could cause out-of-bounds access; execution stopped");
        ensure!(images==1 && pools==mpi,
            "configuration: current single-image EPW path requires one MPI process per pool: grid={grid:?}, k points={nk}, MPI={mpi}, pool={pools}, image={images}");
    }
    Ok(())
}

fn count_arg(args: &mut Vec<String>, flags: &[&str], count: u32, required: bool) -> Result<()> {
    ensure!(
        !args
            .iter()
            .any(|s| !flags.contains(&s.as_str()) && flags.iter().any(|flag| s.starts_with(flag))),
        "configuration: specify parallel counts as separate argument values; unsupported syntax rejected"
    );
    let indices: Vec<_> = args
        .iter()
        .enumerate()
        .filter_map(|(i, s)| flags.contains(&s.as_str()).then_some(i))
        .collect();
    ensure!(indices.len() <= 1, "configuration: duplicate or ambiguous parallel parameters");
    if let Some(&i) = indices.first() {
        let value = args
            .get_mut(i + 1)
            .context("configuration: parallel parameter count missing")?;
        ensure!(
            value.parse::<u32>().is_ok_and(|n| n > 0),
            "configuration: invalid parallel process count"
        );
        *value = count.to_string();
    } else {
        ensure!(
            !required,
            "configuration: MPI launcher requires an explicit -np/-n/--np/--ntasks count"
        );
        args.extend([flags[0].to_owned(), count.to_string()]);
    }
    Ok(())
}

pub fn apply(cfg: &mut Config, mpi: u32) -> Result<()> {
    ensure!(mpi > 0, "configuration: MPI process count must be positive");
    let nk: u64 = cfg.qe.nscf.k_grid.iter().map(|&n| u64::from(n)).product();
    ensure!(
        u64::from(mpi) <= nk,
        "configuration: pool count exceeds the complete coarse k-point count; empty pools prohibited"
    );
    for args in [&cfg.programs.launcher, &cfg.programs.epw.args] {
        let image_flags = ["-ni", "-nimage", "--nimage"];
        ensure!(
            !args.iter().any(|s| !image_flags.contains(&s.as_str())
                && image_flags.iter().any(|flag| s.starts_with(flag))),
            "configuration: MPI recovery supports only an explicit single image"
        );
        let images: Vec<_> = args
            .iter()
            .enumerate()
            .filter_map(|(i, s)| image_flags.contains(&s.as_str()).then_some(i))
            .collect();
        ensure!(images.len() <= 1, "configuration: duplicate image parameter");
        if let Some(&i) = images.first() {
            ensure!(
                args.get(i + 1).map(String::as_str) == Some("1"),
                "configuration: MPI recovery supports only a single image"
            );
        }
    }
    ensure!(
        !cfg.programs.launcher.is_empty(),
        "configuration: MPI recovery requires an explicit launcher"
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
        cfg.programs.launcher[2] = "64".into();
        cfg.programs.epw.args = vec!["-nk".into(), "64".into()];
        let qe = serde_json::to_value(&cfg.qe).unwrap();
        apply(&mut cfg, 32).unwrap();
        assert_eq!(mpi_count(&cfg).unwrap(), 32);
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
    if mpi_count(&original)? == mpi_count(cfg)? {
        return Ok((cfg.clone(), launcher.to_vec()));
    }
    let mut allowed = original.clone();
    apply(&mut allowed, mpi_count(cfg)?)?;
    ensure!(
        serde_json::to_value(&allowed)? == serde_json::to_value(cfg)?
            && launcher == cfg.programs.launcher,
        "input_conflict: only attempt-level MPI/pool changes allowed; fixed scientific configuration or launcher incompatible"
    );
    let original_launcher = original.programs.launcher.clone();
    Ok((original, original_launcher))
}
