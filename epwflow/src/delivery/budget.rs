use anyhow::Result;
use serde_json::Value;
use super::super::foundation::budget;
use super::super::foundation::config;

/// 统一预算账本摘要（FR-005）：读取 campaign ledger 并汇总结余/限制。
pub fn ledger_summary(cfg: &config::Config) -> Result<Value> {
    budget::status_json(&cfg.budget.ledger, &cfg.budget.campaign_id)
}
