use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::PathBuf;
use super::report_store::ResultsBundle;
use super::super::foundation::config;
use super::super::foundation::status;

/// delivery 子命令：一次入口收集、绘图、结果核验（FR-001/002）。
#[derive(Subcommand)]
pub enum DeliveryCommand {
    /// 收集各阶段交付到 results/manifest.json。
    Collect {
        #[arg(short = 'w', long)]
        work: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// 读取 results/manifest.json 并核验六组合覆盖（FR-002/004）。
    Results {
        #[arg(short = 'w', long)]
        work: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// 读取配置并打印统一预算账本摘要（FR-005）。
    Budget {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

/// 入口。
pub fn run(cmd: DeliveryCommand) -> Result<()> {
    match cmd {
        DeliveryCommand::Collect { work, json } => collect(&work, json),
        DeliveryCommand::Results { work, json } => results(&work, json),
        DeliveryCommand::Budget { config: cfg_path, json } => budget(&cfg_path, json),
    }
}

fn collect(work: &PathBuf, json: bool) -> Result<()> {
    let b = super::scheduler::run(work)?;
    print_bundle(&b, json);
    Ok(())
}

fn results(work: &PathBuf, json: bool) -> Result<()> {
    let path = work.join("results").join("manifest.json");
    let b = ResultsBundle::load(&path).with_context(|| format!("data_invalid: 无法加载 {}", path.display()))?;
    print_bundle(&b, json);
    Ok(())
}

fn budget(cfg_path: &PathBuf, json: bool) -> Result<()> {
    let cfg = config::Config::load(cfg_path)?;
    let v = super::budget::ledger_summary(&cfg)?;
    if json {
        status::print_json(&v);
    } else {
        println!("{}", serde_json::to_string_pretty(&v)?);
    }
    Ok(())
}

fn print_bundle(b: &ResultsBundle, json: bool) {
    if json {
        status::print_json(&serde_json::json!({
            "run_id": b.run_id,
            "tc_k": b.tc,
            "lambda_eff": b.lambda_eff,
            "gaps_mev": b.gaps_mev,
            "cells": b.cells.len(),
            "validation_status": b.validation.status,
        }));
    } else {
        println!("run_id: {}", b.run_id);
        println!("tc_k: {:?}", b.tc);
        println!("lambda_eff: {}", b.lambda_eff);
        println!("gaps_mev: {:?}", b.gaps_mev);
        println!("cells: {}", b.cells.len());
        println!("validation_status: {}", b.validation.status);
    }
}
