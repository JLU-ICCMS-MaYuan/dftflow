use super::super::foundation::store;
use super::super::qe_data::manifest::{Check, Validation};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// 单次尝试记录。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttemptRecord {
    pub temperature_k: f64,
    pub mode: String,
    pub converged: bool,
    pub gap_mev: f64,
}

/// F4 交付清单（FR-012）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuperconductivityManifest {
    pub schema_version: u32,
    pub run_id: String,
    pub interpolation_digest: String,
    pub mode: String,
    pub tc: Option<f64>,
    pub located: bool,
    pub converged: bool,
    pub gaps_mev: Vec<f64>,
    pub attempts: Vec<AttemptRecord>,
    pub validation: Validation,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub native_modes: BTreeMap<String, super::engine::ModeResult>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<store::Identity>,
}

impl SuperconductivityManifest {
    pub fn load(path: &Path) -> Result<Self> {
        let m: SuperconductivityManifest = store::read(path)?;
        if !matches!(m.schema_version, 1 | 2) {
            anyhow::bail!(
                "data_invalid: 超导清单 schema_version={} 不支持",
                m.schema_version
            );
        }
        if m.schema_version == 2 {
            let expected = match m.mode.as_str() {
                "iso" => vec!["iso"],
                "aniso" => vec!["aniso"],
                "both" => vec!["aniso", "iso"],
                _ => anyhow::bail!("data_invalid: 未知原生求解模式"),
            };
            anyhow::ensure!(
                m.native_modes
                    .keys()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    == expected,
                "data_invalid: 原生交付的所选模式不完整"
            );
            for a in &m.artifacts {
                a.verify()?;
            }
            anyhow::ensure!(
                !m.native_modes.is_empty() && m.tc.is_none(),
                "data_invalid: 原生交付必须按模式保存区间，不能压为旧 Tc 标量"
            );
            for (mode, result) in &m.native_modes {
                super::verify::mode(result, &m.interpolation_digest)?;
                anyhow::ensure!(
                    result.schema_version == 1
                        && matches!(
                            (mode.as_str(), result.mode),
                            ("iso", crate::foundation::config::Mode::Iso)
                                | ("aniso", crate::foundation::config::Mode::Aniso)
                        ),
                    "data_invalid: 原生模式身份不符"
                );
                for a in &result.attempts {
                    if let Some(record) = &a.record {
                        record.verify()?;
                        let r: super::attempt::AttemptResult = store::read(&record.path)?;
                        r.executable.verify()?;
                        r.source.verify(r.request.mode)?;
                        for x in &r.artifacts {
                            x.verify()?;
                        }
                    }
                }
            }
            let complete = m.native_modes.values().all(|r| r.complete);
            anyhow::ensure!(
                m.converged == complete
                    && m.located
                        == m.native_modes
                            .values()
                            .all(|r| r.search.status == "located")
                    && (!m.validation.all_passed() || complete),
                "data_invalid: 阶段状态与分模式证据冲突"
            );
        }
        Ok(m)
    }
}

/// 便捷校验条目。
pub fn check(name: &str, passed: bool, detail: &str) -> Check {
    Check {
        name: name.to_string(),
        passed,
        detail: detail.to_string(),
    }
}

/// 占位以满足 BTreeMap 导入（避免未使用告警）。
#[allow(dead_code)]
pub fn _thresholds() -> BTreeMap<String, f64> {
    BTreeMap::new()
}
