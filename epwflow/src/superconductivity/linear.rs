//! 仅按用户显式温度列表执行原生 iso 线性化求解，不搜索或追加温度。
use crate::foundation::{config::{Config,Mode},stages::Step,stage::Backend,store,workspace_data};
use super::{attempt::{Request,SolveKind},source};
use anyhow::{ensure,Context,Result};
use std::{fs,path::Path};

pub fn run(cfg:&Config,work:&Path,launcher:&[String],backend:Backend)->Result<()> {
    ensure!(cfg.superconductivity.mode==Mode::Iso,"configuration: native linearized Tc currently requires mode=iso; anisotropic gap uses the native temperature list");
    ensure!(backend==Backend::Local,"allocation: native linearized solve requires allocated execution");
    let manifest=crate::interpolation::manifest::InterpolationManifest::load(&crate::foundation::layout::path(work,"interpolation/manifest.json"))?;
    let input=source::bind(work,cfg,&manifest)?;
    let settings=cfg.solver_for(Mode::Iso)?;
    let first=*settings.gap_temperatures_k.first().context("configuration: native temperatures missing")?;
    let request=Request {
        mode:Mode::Iso,kind:SolveKind::Linear,temperature_centikelvin:crate::foundation::config::temperature(first,"temps")?,seed_mev:None,
        mu_star:settings.mu_star,cutoff_ev:settings.frequency_cutoff_ev,max_iterations:settings.max_iterations,
        convergence_relative:settings.convergence_relative,mixing_beta:settings.mixing_beta,mixing_history:settings.mixing_history,initial_shape:settings.initial_shape,
    };
    let mut text=super::attempt::render_base(&request,&input)?;
    let original=format!(" nstemp = 1\n temps(1) = {:.2}\n",first);
    let mut temps=format!(" nstemp = {}\n",settings.gap_temperatures_k.len());
    for (i,t) in settings.gap_temperatures_k.iter().enumerate(){temps.push_str(&format!(" temps({}) = {:.2}\n",i+1,t));}
    ensure!(text.contains(&original),"data_invalid: native temperature template missing");
    text=text.replacen(&original,&temps,1);
    text=super::attempt::configured(&request,&input,&text)?;
    ensure!(regex::Regex::new(r"(?im)^\s*tc_linear\s*=\s*\.true\.")?.is_match(&text),
        "configuration: explicit native linearized task requires tc_linear=.true.");
    let dir=Step::Superconductivity.directory(cfg).join(super::batch::directory_name(cfg,Mode::Iso));fs::create_dir_all(&dir)?;
    workspace_data::link(&input.a2f.path,&dir.join("source.a2f"))?;
    workspace_data::link(&input.crystal.path,&dir.join("crystal.fmt"))?;
    let file=dir.join("epw.in");store::atomic_bytes(&file,text.as_bytes())?;
    let epw=crate::interpolation::epw_config::EpwConfig::from_config(cfg)?;
    crate::interpolation::coarse::run_epw(&epw,&dir,"epw.in",launcher,backend)?;
    let log=fs::read_to_string(dir.join("epw.out"))?;
    validate(cfg,&log)?;
    crate::foundation::readable_state::complete_files(cfg,Step::Superconductivity,"Native linearized temperatures",&[file,dir.join("epw.out")])
}

/// 原生线性化表的每一行必须对应用户已请求的温度；不估算或追加搜索点。
pub fn validate(cfg:&Config,log:&str)->Result<()> {
    crate::interpolation::native::checked_log(log,0)?;
    ensure!(log.contains("linearized Eliashberg"),"data_invalid: missing native linearized solver evidence");
    let rows=log.lines().filter_map(|line| {
        let cells=line.split_whitespace().collect::<Vec<_>>();
        if cells.len()!=5{return None;}
        let parsed=cells.iter().map(|v|v.replace(['D','d'],"E").parse::<f64>()).collect::<std::result::Result<Vec<_>,_>>().ok()?;
        if parsed.iter().all(|v|v.is_finite()) && parsed[2]>0. && parsed[4]>=0. {Some(parsed)} else {None}
    }).collect::<Vec<_>>();
    let requested=&cfg.superconductivity.gap_temperatures_k;
    ensure!(rows.len()==requested.len() && rows.iter().zip(requested).all(|(r,t)|(r[0]-t).abs()<0.005),
        "data_invalid: native linearized temperature/eigenvalue table incomplete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn eigenvalue_table_must_match_exact_requested_temperatures() {
        let mut cfg:Config=toml::from_str(include_str!("../../tests/fixtures/input.toml")).unwrap();
        cfg.superconductivity.gap_temperatures_k=vec![20.,30.];
        let text="Program EPW v.5.9\nStart: Solving (isotropic) linearized Eliashberg equation with solver = lapack\n20.00 1.2 10 0.10 1\n30.00 0.9 8 0.10 1\nTotal program execution\nEPW : 1s CPU 1s WALL\n";
        validate(&cfg,text).unwrap();
        assert!(validate(&cfg,&text.replace("30.00 0.9","31.00 0.9")).is_err());
        assert!(validate(&cfg,&text.replace("30.00 0.9 8 0.10 1\n","")).is_err());
        assert!(validate(&cfg,&text.replace("0.9","NaN")).is_err());
    }
}
