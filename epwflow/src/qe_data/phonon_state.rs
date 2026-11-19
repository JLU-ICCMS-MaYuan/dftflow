//! 分 q 作业的持久状态；所有读改写共用同一把短锁。
use crate::foundation::store;
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Submitted,
    Running,
    Completed,
    Failed,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Submitted => "submitted",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitState {
    pub status: TaskStatus,
    pub job_id: Option<u64>,
    pub script: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QPointState {
    pub global_index: usize,
    pub directory: String,
    pub crystal: [f64; 3],
    pub local_index: usize,
    pub status: TaskStatus,
    pub job_id: Option<u64>,
    pub script: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectState {
    pub status: TaskStatus,
    pub job_id: Option<u64>,
    pub script: PathBuf,
    pub dependencies: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhononState {
    pub schema_version: u32,
    pub mode: String,
    pub init: InitState,
    pub qpoints: Vec<QPointState>,
    pub collect: CollectState,
}

impl PhononState {
    pub fn new(mode: &str) -> Result<Self> {
        let state = Self {
            schema_version: 1,
            mode: mode.into(),
            init: InitState {
                status: TaskStatus::Pending,
                job_id: None,
                script: "init/ph_init.sh".into(),
            },
            qpoints: Vec::new(),
            collect: CollectState {
                status: TaskStatus::Pending,
                job_id: None,
                script: "collect/collect.sh".into(),
                dependencies: Vec::new(),
            },
        };
        state.validate()?;
        Ok(state)
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema_version == 1 && matches!(self.mode.as_str(), "index" | "coords"),
            "data_invalid: unsupported phonon q-point state schema or mode"
        );
        ensure!(
            self.init.script == Path::new("init/ph_init.sh")
                && self.collect.script == Path::new("collect/collect.sh"),
            "data_invalid: invalid phonon state script paths"
        );
        for (i, q) in self.qpoints.iter().enumerate() {
            let index = i + 1;
            ensure!(
                q.global_index == index
                    && q.directory == format!("q{index:04}")
                    && q.script == PathBuf::from(format!("q/q{index:04}/ph.sh"))
                    && q.local_index == if self.mode == "coords" { 1 } else { index }
                    && q.crystal.iter().all(|v| v.is_finite()),
                "data_invalid: invalid phonon q-point metadata at {index}"
            );
        }
        let tasks = std::iter::once((self.init.status, self.init.job_id))
            .chain(self.qpoints.iter().map(|q| (q.status, q.job_id)))
            .chain(std::iter::once((self.collect.status, self.collect.job_id)));
        let mut jobs = std::collections::BTreeSet::new();
        for (status, job) in tasks {
            if let Some(job) = job {
                ensure!(
                    job > 0 && jobs.insert(job),
                    "data_invalid: invalid or duplicate phonon job ID"
                );
            }
            ensure!(
                !matches!(status, TaskStatus::Running | TaskStatus::Completed) || job.is_some(),
                "data_invalid: executed phonon task has no job ID"
            );
        }
        ensure!(
            self.collect.dependencies.iter().all(|id| *id > 0),
            "data_invalid: invalid collect dependency"
        );
        ensure!(
            self.collect.status != TaskStatus::Completed
                || (self.init.status == TaskStatus::Completed
                    && !self.qpoints.is_empty()
                    && self
                        .qpoints
                        .iter()
                        .all(|q| q.status == TaskStatus::Completed)),
            "data_invalid: completed collect requires completed init and q points"
        );
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let state: Self = store::read(path)?;
        state.validate()?;
        Ok(state)
    }

    pub fn create(&self, path: &Path) -> Result<()> {
        let _lock = lock(path)?;
        ensure!(
            !path.try_exists()?,
            "input_conflict: phonon q-point state already exists"
        );
        self.save(path)
    }

    pub(crate) fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;
        store::atomic_bytes(path, &serde_json::to_vec_pretty(self)?)
    }

    pub fn update<T>(path: &Path, change: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        let _lock = lock(path)?;
        let mut state = Self::load(path)?;
        let value = change(&mut state)?;
        state.save(path)?;
        Ok(value)
    }

    pub fn failed_qpoints(&self) -> impl Iterator<Item = &QPointState> {
        self.qpoints
            .iter()
            .filter(|q| q.status == TaskStatus::Failed)
    }
}

/// store::Lock 不等待；短事务竞争时有界重试，未知锁最终仍报错。
pub(crate) fn lock(path: &Path) -> Result<store::Lock> {
    for _ in 0..200 {
        match store::Lock::acquire(path) {
            Ok(lock) => return Ok(lock),
            Err(_)
                if path
                    .with_file_name(format!(
                        "{}.lock",
                        path.file_name().unwrap().to_string_lossy()
                    ))
                    .is_dir() =>
            {
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            Err(error) => return Err(error),
        }
    }
    store::Lock::acquire(path)
}

#[cfg(test)]
mod phonon_state_tests {
    use super::*;

    #[test]
    fn phonon_state_roundtrip_and_failures() {
        for mode in ["index", "coords"] {
            let mut state = PhononState::new(mode).unwrap();
            for index in 1..=3 {
                state.qpoints.push(QPointState {
                    global_index: index,
                    directory: format!("q{index:04}"),
                    crystal: [(index - 1) as f64 / 3., 0., 0.],
                    local_index: if mode == "coords" { 1 } else { index },
                    status: if index == 2 {
                        TaskStatus::Failed
                    } else {
                        TaskStatus::Completed
                    },
                    job_id: Some(index as u64 + 10),
                    script: format!("q/q{index:04}/ph.sh").into(),
                });
            }
            let path = std::env::temp_dir().join(format!(
                "phonon-state-{}/phonon-qpoints.json",
                store::token()
            ));
            state.create(&path).unwrap();
            assert!(state.create(&path).is_err());
            let loaded = PhononState::load(&path).unwrap();
            assert_eq!(
                loaded
                    .failed_qpoints()
                    .map(|q| q.global_index)
                    .collect::<Vec<_>>(),
                [2]
            );
            PhononState::update(&path, |s| {
                s.qpoints[1].status = TaskStatus::Submitted;
                Ok(())
            })
            .unwrap();
            assert_eq!(
                PhononState::load(&path).unwrap().failed_qpoints().count(),
                0
            );
            std::fs::write(&path, "{}").unwrap();
            assert!(PhononState::load(&path).is_err());
            std::fs::remove_file(&path).unwrap();
            assert!(PhononState::update(&path, |_| Ok(())).is_err());
            std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
        }
        assert!(PhononState::new("nosplit").is_err());
        let mut state = PhononState::new("index").unwrap();
        state.schema_version = 2;
        assert!(state.validate().is_err());
    }

    #[test]
    fn phonon_state_concurrent_updates_keep_all_receipts() {
        let root = std::env::temp_dir().join(format!("phonon-concurrent-{}", store::token()));
        let path = root.join("phonon-qpoints.json");
        PhononState::new("index").unwrap().create(&path).unwrap();
        std::thread::scope(|scope| {
            for job in 1..=12 {
                let path = &path;
                scope.spawn(move || {
                    PhononState::update(path, |s| {
                        s.collect.dependencies.push(job);
                        Ok(())
                    })
                    .unwrap()
                });
            }
        });
        let mut jobs = PhononState::load(&path).unwrap().collect.dependencies;
        jobs.sort();
        assert_eq!(jobs, (1..=12).collect::<Vec<_>>());
        std::fs::remove_dir_all(root).unwrap();
    }
}
