//! 阶段依赖和科学身份；执行资源不参与科学结果的缓存键。
use super::{config::Config, layout, store};
use anyhow::{ensure, Result};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Step {
    Scf,
    Nscf,
    Electronic,
    Phonon,
    Wannier,
    Coarse,
    Fine,
    Superconductivity,
}
impl Step {
    pub const ALL: [Self; 8] = [
        Self::Scf,
        Self::Nscf,
        Self::Electronic,
        Self::Phonon,
        Self::Wannier,
        Self::Coarse,
        Self::Fine,
        Self::Superconductivity,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Scf => "scf",
            Self::Nscf => "nscf",
            Self::Electronic => "electronic",
            Self::Phonon => "phonon",
            Self::Wannier => "wannier",
            Self::Coarse => "coarse",
            Self::Fine => "fine",
            Self::Superconductivity => "superconductivity",
        }
    }
    pub fn directory(self, cfg: &Config) -> PathBuf {
        self.directory_in(&cfg.work_dir)
    }
    pub fn directory_in(self, root: &Path) -> PathBuf {
        match self {
            Self::Scf => layout::path(root, "scf"),
            Self::Nscf => layout::path(root, "nscf"),
            Self::Electronic => root.join("3.qe-electronic"),
            Self::Phonon => layout::path(root, "qe-phonon"),
            Self::Wannier => root.join("5.epw-wannier"),
            Self::Coarse => layout::path(root, "coarse"),
            Self::Fine => layout::path(root, "fine"),
            Self::Superconductivity => layout::path(root, "superconductivity"),
        }
    }
    pub fn dependencies(self) -> &'static [Self] {
        match self {
            Self::Scf => &[],
            Self::Nscf => &[Self::Scf],
            Self::Electronic => &[Self::Nscf],
            Self::Phonon => &[Self::Scf],
            Self::Wannier => &[Self::Nscf],
            Self::Coarse => &[Self::Phonon, Self::Wannier],
            Self::Fine => &[Self::Coarse],
            Self::Superconductivity => &[Self::Fine],
        }
    }
    pub fn closure(self) -> Vec<Self> {
        fn visit(s: Step, out: &mut Vec<Step>) {
            for &d in s.dependencies() {
                visit(d, out);
            }
            if !out.contains(&s) {
                out.push(s);
            }
        }
        let mut out = vec![];
        visit(self, &mut out);
        if self == Self::Phonon || self >= Self::Wannier {
            visit(Self::Electronic, &mut out);
        }
        out.sort();
        out
    }
    pub fn epw(self) -> bool {
        self >= Self::Wannier
    }
}

fn program(path: &Path) -> Result<Value> {
    Ok(json!({"sha256":store::hash_file(path)?}))
}

/// 参数所有权保持局部：尤其 Wannier 不读取 PH 网格，粗网格不读取细网格。
pub fn parameters(cfg: &Config, step: Step) -> Result<Value> {
    let qe = &cfg.qe;
    Ok(match step {
        Step::Scf => json!({"structure":store::hash_file(&cfg.structure)?,
            "pseudos":cfg.pseudos.iter().map(|(el,p)| Ok((el.clone(),store::hash_file(p)?))).collect::<Result<BTreeMap<_,_>>>()?,
            "pw":program(&cfg.programs.pw.path)?,"prefix":qe.prefix,"ecutwfc":qe.ecutwfc_ry,"ecutrho":qe.ecutrho_ry,
            "occupations":qe.occupations,"smearing":qe.smearing,"degauss":qe.degauss_ry,"scf":qe.scf}),
        Step::Nscf => json!({"nscf":qe.nscf,"pw":program(&cfg.programs.pw.path)?}),
        Step::Electronic => {
            json!({"kpath":cfg.kpath.as_ref().map(|p|store::hash_file(p)).transpose()?,
            "points":cfg.comparison.clone().unwrap_or_default().points_per_edge})
        }
        Step::Phonon => {
            json!({"phonon":cfg.phonon,"masses":cfg.atomic_masses,"ph":program(&cfg.programs.ph.path)?})
        }
        Step::Wannier => json!({"wannier":cfg.wannier,"epw":program(&cfg.programs.epw.path)?}),
        Step::Coarse => json!({"quality":cfg.interpolation.quality,"masses":cfg.atomic_masses,
            "epw":program(&cfg.programs.epw.path)?}),
        Step::Fine => {
            let mut v = serde_json::to_value(&cfg.interpolation)?;
            v.as_object_mut().unwrap().remove("quality");
            json!({"interpolation":v,"epw":program(&cfg.programs.epw.path)?,"temperature":cfg.superconductivity.gap_temperatures_k[0]})
        }
        Step::Superconductivity => {
            json!({"solver":cfg.superconductivity,"epw":program(&cfg.programs.epw.path)?})
        }
    })
}

pub fn scientific_key(cfg: &Config, step: Step) -> Result<String> {
    let deps = step
        .dependencies()
        .iter()
        .map(|&d| Ok((d.name(), scientific_key(cfg, d)?)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    Ok(store::hash_bytes(&serde_json::to_vec(&(
        1,
        step,
        parameters(cfg, step)?,
        deps,
    ))?))
}

pub fn request_digest(
    cfg: &Config,
    step: Step,
    sources: &[store::Identity],
    extra: &str,
) -> Result<String> {
    Ok(store::hash_bytes(&serde_json::to_vec(&(
        scientific_key(cfg, step)?,
        extra,
        sources
            .iter()
            .map(|id| (&id.sha256, id.bytes))
            .collect::<Vec<_>>(),
    ))?))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Completion {
    pub schema_version: u32,
    pub step: Step,
    pub scientific_key: String,
    pub upstream: BTreeMap<String, String>,
    pub outputs: Vec<store::Identity>,
    pub provenance: Option<store::Identity>,
}
pub fn receipt(root: &Path, step: Step) -> PathBuf {
    step.directory_in(root).join("completion.json")
}

/// 同一调用内并发读取独立产物；不跨调用缓存，仍逐份校验完整内容。
fn verify_outputs(
    outputs: &[store::Identity],
    check: impl Fn(&store::Identity) -> Result<bool> + Sync,
) -> Result<bool> {
    std::thread::scope(|scope| {
        let check = &check;
        let workers: Vec<_> = outputs.chunks(outputs.len().div_ceil(8).max(1))
            .map(|part| scope.spawn(move || part.iter().map(check).collect::<Result<Vec<_>>>()))
            .collect();
        // 先归集所有线程，再传播错误；缺失不能掩盖另一份产物损坏。
        let results: Vec<_> = workers.into_iter().map(|worker| worker.join()).collect();
        let mut available = true;
        for result in results {
            for valid in result.map_err(|_| anyhow::anyhow!("execution_failed: 产物核验线程失败"))?? {
                available &= valid;
            }
        }
        Ok(available)
    })
}

impl Completion {
    /// 缺失可以重建；已存在文件仍逐项严格检查，不掩盖篡改或读取错误。
    pub fn available(&self) -> Result<bool> {
        ensure!(self.schema_version == 1 && !self.outputs.is_empty(), "data_invalid: 阶段清单缺证据");
        verify_outputs(&self.outputs, store::identity_available)
    }
    pub fn verify(&self) -> Result<()> {
        ensure!(
            self.schema_version == 1 && !self.outputs.is_empty(),
            "data_invalid: 阶段清单缺证据"
        );
        verify_outputs(&self.outputs, |id| id.verify().map(|_| true))?;
        // 来源保留追溯；复制后的数据不再依赖旧目录永久在线。
        Ok(())
    }
    pub fn output_key(&self) -> Result<String> {
        Ok(store::hash_bytes(&serde_json::to_vec(
            &self
                .outputs
                .iter()
                .map(|id| (&id.path, &id.sha256, id.bytes))
                .collect::<Vec<_>>(),
        )?))
    }
}

pub fn upstream(cfg: &Config, step: Step) -> Result<BTreeMap<String, String>> {
    step.dependencies()
        .iter()
        .map(|&d| {
            let c: Completion = store::read(&receipt(&cfg.work_dir, d))?;
            c.verify()?;
            ensure!(
                c.scientific_key == scientific_key(cfg, d)?,
                "input_conflict: 上游 {} 需要更新",
                d.name()
            );
            Ok((d.name().into(), c.output_key()?))
        })
        .collect()
}

pub fn reusable(cfg: &Config, step: Step) -> Result<bool> {
    let path = receipt(&cfg.work_dir, step);
    if !path.exists() {
        return Ok(false);
    }
    let c: Completion = store::read(&path)?;
    if c.step != step || c.scientific_key != scientific_key(cfg, step)? {
        return Ok(false);
    }
    for &d in step.dependencies() {
        if !reusable(cfg, d)? {
            return Ok(false);
        }
    }
    if !c.available()? { return Ok(false); }
    if step == Step::Wannier && ["bvec", "mmn"].iter().any(|suffix| {
        let path = step.directory(cfg).join(format!("{}.{suffix}", cfg.qe.prefix));
        !c.outputs.iter().any(|id| id.path == path)
    }) {
        // 旧收据没有绑定速度矩阵依赖，不能事后把未绑定文件补认成原始产物。
        return Ok(false);
    }
    if c.upstream != upstream(cfg, step)? { return Ok(false); }
    if step == Step::Electronic {
        return super::workspace_electronic::completed(cfg);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cfg() -> (Config, PathBuf) {
        let root = std::env::temp_dir().join(format!("step-keys-{}", store::token()));
        std::fs::create_dir_all(&root).unwrap();
        let input = root.join("source");
        std::fs::write(&input, "fixed source").unwrap();
        let mut cfg: Config =
            toml::from_str(include_str!("../../tests/fixtures/input.toml")).unwrap();
        cfg.structure = input.clone();
        cfg.pseudos = [("Si".into(), input.clone())].into();
        cfg.kpath = Some(input.clone());
        cfg.programs.pw.path = input.clone();
        cfg.programs.ph.path = input.clone();
        cfg.programs.epw.path = input.clone();
        cfg.work_dir = root.clone();
        (cfg, root)
    }
    #[test]
    fn output_verification_never_hides_damage_behind_missing_files_or_previous_calls() {
        let (cfg, root) = cfg();
        let paths: Vec<_> = (0..17).map(|i| root.join(format!("artifact-{i}"))).collect();
        for path in &paths { std::fs::write(path, "original").unwrap(); }
        let completion = Completion {
            schema_version: 1, step: Step::Scf,
            scientific_key: scientific_key(&cfg, Step::Scf).unwrap(),
            upstream: BTreeMap::new(),
            outputs: paths.iter().map(|p| store::Identity::capture(p).unwrap()).collect(),
            provenance: None,
        };
        assert!(completion.available().unwrap());
        completion.verify().unwrap();
        std::fs::remove_file(&paths[0]).unwrap();
        std::fs::write(&paths[16], "damaged").unwrap();
        assert!(completion.available().is_err(), "先发现缺失也必须拒绝另一产物损坏");
        assert!(completion.verify().is_err());
        std::fs::write(&paths[16], "original").unwrap();
        assert!(!completion.available().unwrap());
        assert!(completion.verify().is_err());
        std::fs::write(&paths[0], "original").unwrap();
        assert!(completion.available().unwrap());
        // 前次通过不得成为持久缓存；下一次调用仍读取真实文件。
        std::fs::write(&paths[8], "changed after verification").unwrap();
        assert!(completion.available().is_err());
        assert!(completion.verify().is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn parameter_ownership_matches_the_dependency_matrix() {
        let (cfg, root) = cfg();
        let keys = |cfg: &Config| Step::ALL.map(|s| scientific_key(cfg, s).unwrap());
        let old = keys(&cfg);
        let mut new = cfg.clone();
        new.resources.mpi = 32;
        new.resources.partition = "other".into();
        new.resources.wall_seconds = 60;
        new.programs.launcher = vec!["mpirun".into(), "-np".into(), "32".into()];
        new.programs.epw.args = vec!["-nk".into(), "32".into()];
        assert_eq!(old, keys(&new));
        new = cfg.clone();
        new.interpolation.q_grid = [4; 3];
        let k = keys(&new);
        assert_eq!(old[..6], k[..6]);
        assert_ne!(old[6], k[6]);
        assert_ne!(old[7], k[7]);
        new = cfg.clone();
        new.wannier.outer_window_ev = Some([-2., 10.]);
        let k = keys(&new);
        assert_eq!(old[..4], k[..4]);
        assert!(old[4..].iter().zip(&k[4..]).all(|(a, b)| a != b));
        new = cfg.clone();
        new.phonon.q_grid = [1; 3];
        let k = keys(&new);
        for i in [0, 1, 2, 4] {
            assert_eq!(old[i], k[i]);
        }
        for i in [3, 5, 6, 7] {
            assert_ne!(old[i], k[i]);
        }
        new = cfg.clone();
        new.comparison = Some(super::super::config::Comparison {
            electronic_sigma_ev: 0.3,
            ..Default::default()
        });
        // 仅展宽不会让科学拟合或 EPC 失效。
        let mut base = cfg.clone();
        base.comparison = Some(Default::default());
        assert_eq!(keys(&base), keys(&new));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn corrupted_reusable_outputs_are_rejected_and_stale_requests_do_not_block_recalculation() {
        let (mut cfg, root) = cfg();
        let output = root.join("output");
        std::fs::write(&output, "scientific output").unwrap();
        let completion = Completion {
            schema_version: 1,
            step: Step::Scf,
            scientific_key: scientific_key(&cfg, Step::Scf).unwrap(),
            upstream: BTreeMap::new(),
            outputs: vec![store::Identity::capture(&output).unwrap()],
            provenance: None,
        };
        store::write(&receipt(&root, Step::Scf), &completion).unwrap();
        assert!(reusable(&cfg, Step::Scf).unwrap());
        std::fs::write(output, "damaged").unwrap();
        assert!(reusable(&cfg, Step::Scf).is_err());
        cfg.qe.ecutwfc_ry += 1.;
        assert!(!reusable(&cfg, Step::Scf).unwrap());
        std::fs::remove_dir_all(root).unwrap();
    }
}
