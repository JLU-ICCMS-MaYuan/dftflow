//! F3 Wannier 化与 EPW 插值模块族（spec 0012，FR-001~FR-013）。
pub mod adapter;
mod attempt;
pub mod cli;
pub mod coarse;
pub mod comparison;
pub mod electronic_sampling;
pub mod epw_config;
pub mod fine;
pub mod manifest;
pub mod native;
pub mod native_matrices;
pub mod reuse;
pub mod scheduler;
pub mod spectral;
pub mod wannier;

pub mod comparison_data;

pub mod restart;

pub(crate) mod automatic_comparison;

pub mod differences;

pub mod direct;

pub(crate) mod native_sampling;
