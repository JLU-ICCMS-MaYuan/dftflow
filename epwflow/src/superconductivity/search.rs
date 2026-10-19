//! 整数 centikelvin 的有界搜索；只消费有效原生分类，不计算经验 Tc。
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Superconducting,
    Normal,
    Uncertain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Search {
    pub min_centikelvin: i64,
    pub max_centikelvin: i64,
    pub precision_centikelvin: i64,
    pub points: BTreeMap<i64, Side>,
    pub interval_centikelvin: Option<[i64; 2]>,
    pub status: String,
    pub reason: Option<String>,
}

impl Search {
    pub fn interval_k(&self) -> Option<[f64; 2]> {
        self.interval_centikelvin
            .map(|[a, b]| [a as f64 / 100.0, b as f64 / 100.0])
    }

    pub fn estimate_k(&self) -> Option<f64> {
        if self.status != "located" {
            return None;
        }
        self.interval_k().map(|[a, b]| (a + b) / 2.0)
    }

    pub fn half_width_k(&self) -> Option<f64> {
        if self.status != "located" {
            return None;
        }
        self.interval_k().map(|[a, b]| (b - a) / 2.0)
    }

    pub fn new(min: i64, max: i64, precision: i64) -> Result<Self> {
        ensure!(
            min > 0 && max > min && precision > 0 && precision <= max - min,
            "configuration: 无效 Tc 搜索范围/精度"
        );
        Ok(Self {
            min_centikelvin: min,
            max_centikelvin: max,
            precision_centikelvin: precision,
            points: BTreeMap::new(),
            interval_centikelvin: None,
            status: "pending".into(),
            reason: None,
        })
    }

    pub fn stop(&mut self, status: &str, reason: &str) {
        self.status = status.into();
        self.reason = Some(reason.into());
    }

    /// 只有有效原生结果可调用；失败保留现场并由调用方 stop，不能当作正常侧。
    pub fn observe(&mut self, t: i64, side: Side) -> Result<()> {
        ensure!(
            t >= self.min_centikelvin && t <= self.max_centikelvin && !self.points.contains_key(&t),
            "data_invalid: 重复或越界温度"
        );
        let was_located = self.status == "located";
        self.points.insert(t, side);
        self.status = "running".into();
        let super_max = self
            .points
            .iter()
            .filter(|(_, s)| **s == Side::Superconducting)
            .map(|(t, _)| *t)
            .max();
        let normal_min = self
            .points
            .iter()
            .filter(|(_, s)| **s == Side::Normal)
            .map(|(t, _)| *t)
            .min();
        if let (Some(s), Some(n)) = (super_max, normal_min) {
            if s >= n {
                self.stop("inconsistent", "出现高温超导侧/低温正常侧矛盾");
                return Ok(());
            }
            self.interval_centikelvin = Some([s, n]);
            if was_located && n - s <= self.precision_centikelvin {
                self.status = "located".into();
            }
        }
        Ok(())
    }

    pub fn next_temperature(&mut self) -> Option<i64> {
        if !matches!(self.status.as_str(), "pending" | "running") {
            return None;
        }
        for t in [self.min_centikelvin, self.max_centikelvin] {
            if !self.points.contains_key(&t) {
                return Some(t);
            }
        }
        if self.points[&self.min_centikelvin] != Side::Superconducting
            || self.points[&self.max_centikelvin] != Side::Normal
        {
            let uncertain = self.points[&self.min_centikelvin] == Side::Uncertain
                || self.points[&self.max_centikelvin] == Side::Uncertain;
            self.stop(
                if uncertain {
                    "classification_uncertain"
                } else {
                    "not_bracketed"
                },
                "固定上下端点未形成低温超导/高温正常的有效区间",
            );
            return None;
        }
        let [lo, hi] = self.interval_centikelvin.unwrap();
        if hi - lo <= self.precision_centikelvin {
            self.stop("located", "有效区间达到目标宽度");
            return None;
        }
        let width = hi - lo;
        for t in [lo + width / 2, lo + width / 4, lo + 3 * width / 4] {
            if t > lo && t < hi && !self.points.contains_key(&t) {
                return Some(t);
            }
        }
        self.stop(
            "classification_uncertain",
            "中点和固定四分点已尝试，无法用有效分类继续缩小区间",
        );
        None
    }
}
