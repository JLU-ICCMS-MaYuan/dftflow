//! F3 Wannier 化与 EPW 插值模块族（spec 0012，FR-001~FR-013）。
pub mod adapter;
mod attempt;
pub mod cli;
pub mod coarse;
pub mod compare;
pub mod epw_config;
pub mod fine;
pub mod manifest;
pub mod native;
pub mod native_matrices;
pub mod native_quality;
pub mod quality;
pub mod reuse;
pub mod scheduler;
pub mod spectral;
pub mod wannier;
pub mod comparison;

pub mod comparison_data;

pub mod restart;
pub mod quality_binding;

mod reference_asr;
mod references;
pub(crate) mod automatic_comparison;
