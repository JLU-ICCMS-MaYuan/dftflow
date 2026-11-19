//! 历史原生Tc结果的数据类型，仅供读取旧结果；不执行搜索或双初值求解。
use super::{attempt::Request, search::Search};
use crate::foundation::{config::{self,Mode},store::Identity};
use serde::{Deserialize,Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub request: Request,
    pub directory: std::path::PathBuf,
    pub record: Option<Identity>,
    pub status: String,
    pub reason: Option<String>,
    #[serde(default)]
    pub job_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModeResult {
    pub schema_version: u32,
    pub mode: Mode,
    pub binding_digest: String,
    pub method: String,
    pub thresholds: config::Tc,
    pub search: Search,
    pub attempts: Vec<Entry>,
    #[serde(default)]
    pub solve_count: u32,
    pub complete: bool,
    pub reason: Option<String>,
    #[serde(default)]
    pub gap_temperatures_centikelvin: Vec<i64>,
    #[serde(default = "default_seeds")]
    pub initial_gaps_mev: [f64; 2],
}

fn default_seeds() -> [f64; 2] {
    [1.0, 10.0]
}
