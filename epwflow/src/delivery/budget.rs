use anyhow::Result;
use serde_json::Value;
use super::super::foundation::budget;
use super::super::foundation::config;

/// 统一账本摘要：读取历史实际用量与当前并发登记，不公布时间／存储额度。
pub fn ledger_summary(cfg: &config::Config) -> Result<Value> {
    budget::status_json(&cfg.budget.ledger, &cfg.budget.campaign_id)
}
