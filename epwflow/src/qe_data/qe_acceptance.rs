use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use super::super::foundation::store;

/// 独立真实接口验收记录（FR-012）：实际版本、输入/输出身份、作业号、SCF 次数与数值差异。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceRecord {
    pub schema_version: u32,
    pub run_id: String,
    pub ph_mode: String,
    pub program_identities: BTreeMap<String, String>,
    pub input_identities: BTreeMap<String, String>,
    pub scf_attempt_count: u32,
    pub scf_success_count: u32,
    pub numeric_differences: BTreeMap<String, f64>,
    pub budget_campaign: String,
    pub evidence: Vec<String>,
}

impl AcceptanceRecord {
    pub fn new(run_id: &str, ph_mode: &str, budget_campaign: &str) -> Self {
        AcceptanceRecord {
            schema_version: 1,
            run_id: run_id.to_string(),
            ph_mode: ph_mode.to_string(),
            program_identities: BTreeMap::new(),
            input_identities: BTreeMap::new(),
            scf_attempt_count: 0,
            scf_success_count: 0,
            numeric_differences: BTreeMap::new(),
            budget_campaign: budget_campaign.to_string(),
            evidence: Vec::new(),
        }
    }

    /// 断言已记录真实证据；未运行项保留未知，不以模拟结果冒充真实通过。
    pub fn require_evidence(&self) -> Result<()> {
        anyhow::ensure!(
            !self.program_identities.is_empty(),
            "data_invalid: 验收缺少真实程序身份证据"
        );
        anyhow::ensure!(
            self.scf_attempt_count > 0,
            "data_invalid: 验收缺少真实 SCF 执行证据"
        );
        Ok(())
    }
}

/// 落盘验收记录；原子写，禁止覆盖符号链接。
pub fn write(path: &Path, record: &AcceptanceRecord) -> Result<()> {
    store::write(path, record)
}

/// 从已有清单与来源身份构建验收记录骨架（不代替真实运行证据）。
pub fn from_manifest(
    run_id: &str,
    ph_mode: &str,
    budget_campaign: &str,
    program_identities: BTreeMap<String, String>,
    input_identities: BTreeMap<String, String>,
) -> AcceptanceRecord {
    let mut r = AcceptanceRecord::new(run_id, ph_mode, budget_campaign);
    r.program_identities = program_identities;
    r.input_identities = input_identities;
    r
}
