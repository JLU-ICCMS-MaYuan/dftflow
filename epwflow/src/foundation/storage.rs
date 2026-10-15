use anyhow::{bail, ensure, Result};
use std::path::Path;

/// 运行中存储监测：达到上限停止追加并请求停止本流程超限写入任务，保留现场与实际峰值。
#[derive(Debug, Clone)]
pub struct StorageMonitor {
    limit_bytes: u64,
    peak: u64,
    stop: bool,
}

impl StorageMonitor {
    pub fn new(limit_bytes: u64) -> Self {
        StorageMonitor {
            limit_bytes,
            peak: 0,
            stop: false,
        }
    }
    /// 记录一次写入增长；超限时请求停止并返回 storage_denied，不删除已有数据。
    pub fn add(&mut self, bytes: u64) -> Result<()> {
        self.peak += bytes;
        if self.peak > self.limit_bytes {
            self.stop = true;
            bail!(
                "storage_denied: 写入增长超过存储上限，已请求停止；实际峰值 {} 字节",
                self.peak
            );
        }
        Ok(())
    }
    pub fn should_stop(&self) -> bool {
        self.stop
    }
    pub fn peak(&self) -> u64 {
        self.peak
    }
}

/// 递归统计目录字节数；不可访问时按 0 计入，避免误判。
pub fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for e in entries.flatten() {
            let p = e.path();
            if let Ok(meta) = e.metadata() {
                if meta.is_dir() {
                    total += dir_size(&p);
                } else {
                    total += meta.len();
                }
            }
        }
    }
    total
}

/// 投影用量 = 既有存储根占用 + 新增预留；用于阶段开始前拒绝超限。
pub fn project_usage(storage_roots: &[std::path::PathBuf], extra: u64) -> u64 {
    let existing: u64 = storage_roots.iter().map(|p| dir_size(p)).sum();
    existing + extra
}

/// 阶段开始前检查实际占用、预留与文件系统可用空间；预计超过任一限制时拒绝。
pub fn check_before_stage(projected: u64, limit_bytes: u64, available: u64) -> Result<()> {
    ensure!(
        projected <= limit_bytes,
        "storage_denied: 预计占用 {} 超过授权上限 {}",
        projected,
        limit_bytes
    );
    ensure!(
        projected <= available,
        "storage_denied: 预计占用 {} 超过文件系统可用空间 {}",
        projected,
        available
    );
    Ok(())
}
