//! 持锁命令内共享第 3 步确认；不跨配置、来源重算或命令保留。
use super::{config::Config, stages::Step};
use anyhow::Result;
use std::{cell::RefCell, path::PathBuf};

struct Confirmed {
    configuration: Vec<u8>,
    electronic: bool,
    projection: Option<Vec<PathBuf>>,
    refreshed: bool,
}

thread_local! {
    static CURRENT: RefCell<Option<Confirmed>> = const { RefCell::new(None) };
}

pub(super) struct Scope;

impl Scope {
    pub(super) fn enter(cfg: &Config) -> Result<Self> {
        let current = Confirmed {
            configuration: serde_json::to_vec(cfg)?,
            electronic: false,
            projection: None,
            refreshed: false,
        };
        CURRENT.with(|slot| slot.replace(Some(current)));
        Ok(Self)
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        CURRENT.with(|slot| slot.replace(None));
    }
}

fn read<T>(cfg: &Config, value: impl FnOnce(&Confirmed) -> Option<T>) -> Result<Option<T>> {
    let configuration = serde_json::to_vec(cfg)?;
    Ok(CURRENT.with(|slot| {
        slot.borrow().as_ref().filter(|s| s.configuration == configuration).and_then(value)
    }))
}

fn remember(cfg: &Config, value: impl FnOnce(&mut Confirmed)) -> Result<()> {
    let configuration = serde_json::to_vec(cfg)?;
    CURRENT.with(|slot| {
        if let Some(current) = slot.borrow_mut().as_mut().filter(|s| s.configuration == configuration) {
            value(current);
        }
    });
    Ok(())
}

pub(super) fn active(cfg: &Config) -> Result<bool> {
    Ok(read(cfg, |_| Some(()))?.is_some())
}

pub(super) fn electronic(cfg: &Config, check: impl FnOnce() -> Result<bool>) -> Result<bool> {
    if read(cfg, |s| s.electronic.then_some(true))?.is_some() {
        return Ok(true);
    }
    let valid = check()?;
    if valid {
        remember(cfg, |s| s.electronic = true)?;
    }
    Ok(valid)
}

pub(super) fn projection(
    cfg: &Config,
    check: impl FnOnce() -> Result<Vec<PathBuf>>,
) -> Result<Vec<PathBuf>> {
    if let Some(files) = read(cfg, |s| s.projection.clone())? {
        return Ok(files);
    }
    let files = check()?;
    if !files.is_empty() {
        remember(cfg, |s| s.projection = Some(files.clone()))?;
    }
    Ok(files)
}

pub(super) fn refresh(cfg: &Config, run: impl FnOnce() -> Result<()>) -> Result<()> {
    if read(cfg, |s| s.refreshed.then_some(()))?.is_some() {
        return Ok(());
    }
    run()?;
    remember(cfg, |s| s.refreshed = true)
}

pub(super) fn invalidate(step: Step) {
    if !matches!(step, Step::Scf | Step::Nscf | Step::Electronic) {
        return;
    }
    CURRENT.with(|slot| {
        if let Some(current) = slot.borrow_mut().as_mut() {
            current.electronic = false;
            current.projection = None;
            current.refreshed = false;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn cfg() -> Config {
        toml::from_str(include_str!("../../tests/fixtures/input.toml")).unwrap()
    }

    #[test]
    fn electronic_and_projection_checks_and_delivery_run_once_per_command() {
        let cfg = cfg();
        let electronic_count = Cell::new(0);
        let projection_count = Cell::new(0);
        let refresh_count = Cell::new(0);
        let _scope = Scope::enter(&cfg).unwrap();
        for _ in 0..4 {
            assert!(electronic(&cfg, || {
                electronic_count.set(electronic_count.get() + 1);
                Ok(true)
            }).unwrap());
            assert_eq!(projection(&cfg, || {
                projection_count.set(projection_count.get() + 1);
                Ok(vec![PathBuf::from("projection.dat")])
            }).unwrap(), vec![PathBuf::from("projection.dat")]);
            refresh(&cfg, || {
                refresh_count.set(refresh_count.get() + 1);
                Ok(())
            }).unwrap();
        }
        assert_eq!((electronic_count.get(), projection_count.get(), refresh_count.get()), (1, 1, 1));
    }

    #[test]
    fn source_recalculation_invalidates_all_three_confirmations() {
        let cfg = cfg();
        let _scope = Scope::enter(&cfg).unwrap();
        for source in [Step::Scf, Step::Nscf, Step::Electronic] {
            electronic(&cfg, || Ok(true)).unwrap();
            projection(&cfg, || Ok(vec!["old.dat".into()])).unwrap();
            refresh(&cfg, || Ok(())).unwrap();
            invalidate(source);
            assert!(!electronic(&cfg, || Ok(false)).unwrap());
            assert!(projection(&cfg, || Ok(vec![])).unwrap().is_empty());
            assert!(refresh(&cfg, || anyhow::bail!("source changed")).is_err());
        }
    }

    #[test]
    fn unrelated_phonon_or_wannier_work_keeps_electronic_confirmations() {
        let cfg = cfg();
        let _scope = Scope::enter(&cfg).unwrap();
        electronic(&cfg, || Ok(true)).unwrap();
        projection(&cfg, || Ok(vec!["pdos.dat".into()])).unwrap();
        refresh(&cfg, || Ok(())).unwrap();
        for stage in [Step::Phonon, Step::Wannier, Step::Coarse, Step::Fine] {
            invalidate(stage);
            assert!(electronic(&cfg, || anyhow::bail!("duplicate electronic check")).unwrap());
            assert_eq!(projection(&cfg, || anyhow::bail!("duplicate PDOS check")).unwrap().len(), 1);
            refresh(&cfg, || anyhow::bail!("duplicate electronic delivery")).unwrap();
        }
    }

    #[test]
    fn changed_configuration_and_new_command_check_again() {
        let cfg = cfg();
        {
            let _scope = Scope::enter(&cfg).unwrap();
            assert!(active(&cfg).unwrap());
            electronic(&cfg, || Ok(true)).unwrap();
            projection(&cfg, || Ok(vec!["pdos.dat".into()])).unwrap();
            refresh(&cfg, || Ok(())).unwrap();
            let mut changed = cfg.clone();
            changed.qe.prefix.push_str("-changed");
            assert!(!active(&changed).unwrap());
            assert!(!electronic(&changed, || Ok(false)).unwrap());
            assert!(projection(&changed, || Ok(vec![])).unwrap().is_empty());
            assert!(refresh(&changed, || anyhow::bail!("new configuration")).is_err());
        }
        assert!(!active(&cfg).unwrap());
        let _next = Scope::enter(&cfg).unwrap();
        assert!(!electronic(&cfg, || Ok(false)).unwrap());
        assert!(projection(&cfg, || Ok(vec![])).unwrap().is_empty());
        assert!(refresh(&cfg, || anyhow::bail!("new command")).is_err());
    }

    #[test]
    fn errors_and_incomplete_results_are_not_remembered() {
        let cfg = cfg();
        let _scope = Scope::enter(&cfg).unwrap();
        for _ in 0..2 {
            assert!(electronic(&cfg, || anyhow::bail!("bad electronic result")).is_err());
            assert!(!electronic(&cfg, || Ok(false)).unwrap());
            assert!(projection(&cfg, || anyhow::bail!("bad PDOS result")).is_err());
            assert!(projection(&cfg, || Ok(vec![])).unwrap().is_empty());
            assert!(refresh(&cfg, || anyhow::bail!("plot failed")).is_err());
        }
        assert!(electronic(&cfg, || Ok(true)).unwrap());
        assert_eq!(projection(&cfg, || Ok(vec!["repaired.dat".into()])).unwrap().len(), 1);
        refresh(&cfg, || Ok(())).unwrap();
    }

    #[test]
    fn unscoped_calls_always_validate() {
        let cfg = cfg();
        electronic(&cfg, || Ok(true)).unwrap();
        projection(&cfg, || Ok(vec!["pdos.dat".into()])).unwrap();
        refresh(&cfg, || Ok(())).unwrap();
        assert!(!electronic(&cfg, || Ok(false)).unwrap());
        assert!(projection(&cfg, || Ok(vec![])).unwrap().is_empty());
        assert!(refresh(&cfg, || anyhow::bail!("unscoped")).is_err());
    }
}
