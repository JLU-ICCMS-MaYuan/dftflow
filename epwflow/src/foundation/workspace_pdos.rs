//! QE 原子投影后处理及元素汇总。读取正式 NSCF，归集原生附加输出。
use super::{
    config::Config,
    layout,
    store::{self, Identity},
    workspace_science::{post_program, qe_run},
};
use crate::qe_data::{qe_config::QeSettings, save};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
type Rows = Vec<(f64, f64)>;
#[derive(Serialize, Deserialize)]
struct Receipt {
    key: String,
    sources: Vec<Identity>,
    outputs: Vec<Identity>,
}
fn dir(cfg: &Config) -> PathBuf {
    cfg.work_dir.join("3.qe-electronic/qe_eledos")
}
fn source(cfg: &Config) -> PathBuf {
    layout::path(&cfg.work_dir, "nscf/out")
}
fn source_ids(cfg: &Config) -> Result<Vec<Identity>> {
    let root = source(cfg);
    save::files(&root)?
        .iter()
        .map(|f| Identity::capture(&root.join(&f.relative_path)))
        .collect()
}
fn input(cfg: &Config) -> Result<String> {
    let sigma: f64 = cfg.native_value(&[
        super::native_config::ELECTRONIC,
        "qe_eledos",
        "projwfc",
        "PROJWFC",
        "degauss",
    ])?;
    let text=format!("&projwfc\n prefix='{}', outdir='./out', filpdos='{}.pdos',\n ngauss=0, degauss={sigma:.15}, Emin=-100, Emax=100, DeltaE=0.01,\n/\n",cfg.qe.prefix,cfg.qe.prefix);
    cfg.apply_native(
        &[super::native_config::ELECTRONIC, "qe_eledos", "projwfc"],
        &text.replace("outdir='./out'", &format!("outdir='{}'",super::workspace_data::relative(&dir(cfg),&source(cfg))?.display())),
    )
}
fn key(cfg: &Config, ids: &[Identity]) -> Result<String> {
    let program = post_program(cfg, "projwfc")?;
    Ok(store::hash_bytes(&serde_json::to_vec(&(
        1,
        input(cfg)?,
        store::hash_file(&program.path)?,
        program.args,
        ids.iter().map(|i| (&i.sha256, i.bytes)).collect::<Vec<_>>(),
    ))?))
}
pub fn files(cfg: &Config) -> Result<Vec<PathBuf>> {
    super::electronic_reuse::projection(cfg, || checked_files(cfg))
}

fn checked_files(cfg: &Config) -> Result<Vec<PathBuf>> {
    let Some(ids) = super::readable_state::completed_files(
        cfg,
        super::stages::Step::Electronic,
        "Electronic substep · projwfc",
    )?
    else {
        return Ok(vec![]);
    };
    let Some(old) = super::readable_state::checkpoint_config(
        cfg,
        super::stages::Step::Electronic,
        "Electronic substep · projwfc",
    )?
    else {
        return Ok(vec![]);
    };
    if input(&old)? != input(cfg)? {
        return Ok(vec![]);
    }
    Ok(ids.into_iter().map(|id| id.path).collect())
}
pub fn run(cfg: &Config) -> Result<()> {
    if !files(cfg)?.is_empty() {
        return Ok(());
    }
    super::electronic_reuse::invalidate(super::stages::Step::Electronic);
    let root = dir(cfg);
    fs::create_dir_all(&root)?;
    let ids = source_ids(cfg)?;

    let xml = source(cfg).join(format!("{}.save/data-file-schema.xml", cfg.qe.prefix));
    let text = fs::read_to_string(&xml)?;
    let doc = roxmltree::Document::parse(&text)?;
    ensure!(
        !doc.descendants()
            .filter(|n| matches!(n.tag_name().name(), "lsda" | "noncolin" | "spinorbit"))
            .any(|n| n
                .text()
                .is_some_and(|t| matches!(t.trim(), "true" | ".true." | "T"))),
        "unsupported: Elements PDOS Currently only non-magnetic labels are supported"
    );
    let native_complete = super::readable_state::completed_files(
        cfg,
        super::stages::Step::Electronic,
        "Electronic native substep · projwfc",
    )?
    .is_some();
    let continuing = super::native_jobs::continuing(&root, "projwfc")?;

    if !continuing && !native_complete {
        store::write(
            &root.join("decision.json"),
            &serde_json::json!({"decision":"necessary_postprocessing",
            "reason":"Lack of atomic projection compatible with current state/extension,TDOS without decomposition information;from completed NSCF The wave function seeks to project atoms, not by TDOS or Wannier Initial orbital restoration element contribution",
            "program":post_program(cfg,"projwfc")?,"sources":ids,"input":input(cfg)?,"recalculate_scf_nscf_ph_wannier_epw":false}),
        )?;
        fs::write(root.join("projwfc.in"), input(cfg)?)?;
    }
    ensure!(
        fs::read_to_string(root.join("projwfc.in"))? == input(cfg)?,
        "input_conflict: projwfc Input Changed"
    );
    if !native_complete {
        qe_run(
            cfg,
            &root,
            &post_program(cfg, "projwfc")?,
            "projwfc.in",
            "projwfc",
        )?;
    }
    for id in &ids {
        id.verify()?;
    }
    let projection=source(cfg).join(format!("{}.save/atomic_proj.xml",cfg.qe.prefix));
    if projection.is_file() { fs::rename(&projection,root.join("atomic_proj.xml"))?; }
    let paths = raw_files(cfg)?;
    let mut outputs = paths.clone();
    outputs.extend([
        root.join("projwfc.in"),
        root.join("projwfc.out"),
        root.join("decision.json"),
    ]);
    let receipt = Receipt {
        key: key(cfg, &ids)?,
        sources: ids,
        outputs: outputs
            .iter()
            .map(|p| Identity::capture(p))
            .collect::<Result<_>>()?,
    };
    // 科学执行检查点先保存；解析失败或派生收据缺失可以重验，不能重新执行成功投影。
    if !native_complete {
        super::readable_state::complete_files(
            cfg,
            super::stages::Step::Electronic,
            "Electronic native substep · projwfc",
            &outputs,
        )?;
    }
    aggregate(cfg, &paths, 0., &projected_total(cfg, 0.)?)?;
    super::readable_state::complete_files(
        cfg,
        super::stages::Step::Electronic,
        "Electronic substep · projwfc",
        &receipt
            .outputs
            .iter()
            .map(|id| id.path.clone())
            .collect::<Vec<_>>(),
    )
}
fn raw_files(cfg: &Config) -> Result<Vec<PathBuf>> {
    let mut paths = fs::read_dir(dir(cfg))?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.retain(|p| {
        p.file_name().is_some_and(|n| {
            n.to_string_lossy()
                .starts_with(&format!("{}.pdos", cfg.qe.prefix))
        })
    });
    paths.sort();
    Ok(paths)
}
fn numeric(file: &Path) -> Result<Vec<Vec<f64>>> {
    let mut rows = vec![];
    for line in fs::read_to_string(file)?
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
    {
        let row = line
            .split_whitespace()
            .map(str::parse::<f64>)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ensure!(
            row.len() >= 2 && row.iter().all(|v| v.is_finite()),
            "data_invalid: Invalid projection value {}",
            file.display()
        );
        rows.push(row);
    }
    ensure!(
        rows.len() > 1 && rows.windows(2).all(|r| r[0][0] < r[1][0]),
        "data_invalid: Projectivation axis is illegal"
    );
    Ok(rows)
}
fn total(cfg: &Config, fermi: f64) -> Result<Rows> {
    Ok(numeric(
        &cfg.work_dir
            .join("3.qe-electronic/qe_eledos")
            .join(format!("{}.tdos", cfg.qe.prefix)),
    )?
    .iter()
    .map(|r| (r[0] - fermi, r[1]))
    .collect())
}
/// projwfc 会将能网格原点裁到最低本征值附近，不能假定与 dos.x 的轴相同。
/// 使用同一次 projwfc 的 DOS 列（不是 PDOS 总和），与元素投影保持原生同网格。
pub fn projected_total(cfg: &Config, fermi: f64) -> Result<Rows> {
    Ok(
        numeric(&dir(cfg).join(format!("{}.pdos.pdos_tot", cfg.qe.prefix)))?
            .iter()
            .map(|r| (r[0] - fermi, r[1]))
            .collect(),
    )
}
fn aggregate(
    cfg: &Config,
    paths: &[PathBuf],
    fermi: f64,
    tdos: &[(f64, f64)],
) -> Result<Vec<(String, Rows)>> {
    let qe = QeSettings::from_config(cfg)?;
    let expected = qe
        .cell
        .elements
        .iter()
        .zip(&qe.cell.counts)
        .flat_map(|(s, n)| std::iter::repeat(s.clone()).take(*n))
        .collect::<Vec<_>>();
    let log = fs::read_to_string(dir(cfg).join("projwfc.out"))?;
    aggregate_native(&expected, paths, &log, fermi, tdos)
}
fn aggregate_native(
    expected: &[String],
    paths: &[PathBuf],
    log: &str,
    fermi: f64,
    tdos: &[(f64, f64)],
) -> Result<Vec<(String, Rows)>> {
    let re = regex::Regex::new(r"state #\s*\d+\s*:\s*atom\s*(\d+).*?wfc\s*(\d+)")?;
    let shells = re
        .captures_iter(&log)
        .map(|c| Ok((c[1].parse::<usize>()?, c[2].parse::<usize>()?)))
        .collect::<Result<BTreeSet<_>>>()?;
    ensure!(
        !shells.is_empty(),
        "data_invalid: projwfc manifest of non-natural atomic wave functions"
    );
    let mut seen = BTreeSet::new();
    let mut atoms = BTreeSet::new();
    let mut elements = BTreeMap::<String, Rows>::new();
    for path in paths {
        let Some((atom, element, wfc, l)) =
            qf_io::qe_proj::parse_pdos_filename(path.to_str().context("Projection Path Not UTF-8")?)
        else {
            continue;
        };
        ensure!(
            atom > 0 && expected.get(atom - 1) == Some(&element) && seen.insert((atom, wfc)),
            "data_invalid: Duplication/unmatching of projecting atoms, elements or wave functions"
        );
        atoms.insert(atom);
        let rows = numeric(path)?;
        let n = match l.as_str() {
            "s" => 1,
            "p" => 3,
            "d" => 5,
            "f" => 7,
            _ => unreachable!(),
        };
        ensure!(
            rows.len() == tdos.len() && rows.iter().all(|r| r.len() == n + 2),
            "data_invalid: Inconsistent projection format or points (only non-magnetic))"
        );
        let target = elements
            .entry(element)
            .or_insert_with(|| tdos.iter().map(|p| (p.0, 0.)).collect());
        for ((r, t), out) in rows.iter().zip(tdos).zip(target) {
            // QE partialdos 用三位有效数字分别写 LDOS 和各 m 分量。
            // 按各列末位的半单位相加核验舍入误差，不能把打印损失当作投影错误。
            let rounding = r[1..]
                .iter()
                .map(|v| {
                    if *v == 0. {
                        0.
                    } else {
                        0.005 * 10f64.powf(v.abs().log10().floor())
                    }
                })
                .sum::<f64>()
                + 1e-12;
            ensure!(
                (r[0] - fermi - t.0).abs() < 1e-5
                    && (r[1] - r[2..].iter().sum::<f64>()).abs() <= rounding,
                "data_invalid: Projection axis or LDOS It doesn't match the magnetic weight"
            );
            out.1 += r[1]; // LDOS 已是磁分量之和，只累加一次。
        }
    }
    ensure!(
        seen == shells && atoms.len() == expected.len(),
        "data_invalid: Missing atom/wave projection"
    );
    Ok(elements.into_iter().collect())
}
pub fn elements(cfg: &Config, fermi: f64, tdos: &[(f64, f64)]) -> Result<Vec<(String, Rows)>> {
    if files(cfg)?.is_empty() {
        return Ok(vec![]);
    }
    aggregate(cfg, &raw_files(cfg)?, fermi, tdos)
}
/// 结果包保留逐元素数值、投影残差与原生来源，重绘不再访问科学目录。
pub fn export(cfg: &Config, out: &Path, fermi: f64) -> Result<()> {
    let tdos = if files(cfg)?.is_empty() {
        total(cfg, fermi)?
    } else {
        projected_total(cfg, fermi)?
    };
    let elements = elements(cfg, fermi, &tdos)?;
    let mut csv = String::from("energy_minus_fermi_ev,tdos,projected_sum,tdos_minus_projected_sum");
    for (name, _) in &elements {
        csv.push_str(&format!(",{name}"));
    }
    csv.push('\n');
    let mut maximum: f64 = 0.;
    for (i, (e, total)) in tdos.iter().enumerate() {
        let sum = elements.iter().map(|(_, r)| r[i].1).sum::<f64>();
        maximum = maximum.max((total - sum).abs());
        csv.push_str(&format!(
            "{e:.12},{total:.12},{sum:.12},{:.12}",
            total - sum
        ));
        for (_, r) in &elements {
            csv.push_str(&format!(",{:.12}", r[i].1));
        }
        csv.push('\n');
    }
    fs::write(out.join("electronic-pdos.csv"), csv)?;
    store::write(
        &out.join("electronic-pdos.json"),
        &serde_json::json!({"complete":!elements.is_empty(),"elements":elements.iter().map(|(n,_)|n).collect::<Vec<_>>(),"aggregation":"sum each atom/wfc LDOS once; do not add m components twice","maximum_tdos_projection_residual":maximum,"fermi_ev":fermi,"sigma_ev":cfg.native_value::<f64>(&[super::native_config::ELECTRONIC,"qe_eledos","projwfc","PROJWFC","degauss"])?*13.605693122994,"sources":files(cfg)?.iter().map(|p|Identity::capture(p)).collect::<Result<Vec<_>>>()?,"rescale":false,"total_source":"projwfc pdos_tot DOS column when complete; dos.x otherwise","dos_x_reference":Identity::capture(&cfg.work_dir.join("3.qe-electronic/qe_eledos").join(format!("{}.tdos",cfg.qe.prefix)))?}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_ldos_aggregates_elements_once_and_rejects_incomplete_or_shifted_data() {
        let root = std::env::temp_dir().join(format!("pdos-{}", store::token()));
        fs::create_dir_all(&root).unwrap();
        let paths = [
            "x.pdos_atm#1(Mg)_wfc#1(p)",
            "x.pdos_atm#2(B)_wfc#1(s)",
            "x.pdos_atm#3(B)_wfc#1(s)",
        ]
        .map(|n| root.join(n));
        fs::write(&paths[0], "# E LDOS p1 p2 p3\n0 6 1 2 3\n1 9 2 3 4\n").unwrap();
        for path in &paths[1..] {
            fs::write(path, "# E LDOS s\n0 2 2\n1 3 3\n").unwrap();
        }
        let log="state # 1: atom 1 (Mg), wfc 1\nstate # 2: atom 2 (B), wfc 1\nstate # 3: atom 3 (B), wfc 1";
        let expected = ["Mg".into(), "B".into(), "B".into()];
        let total = [(0., 10.), (1., 15.)];
        let values = aggregate_native(&expected, &paths, log, 0., &total).unwrap();
        assert_eq!(values[0], ("B".into(), vec![(0., 4.), (1., 6.)]));
        assert_eq!(values[1], ("Mg".into(), vec![(0., 6.), (1., 9.)]));
        assert!(aggregate_native(&expected, &paths[..2], log, 0., &total).is_err());
        assert!(aggregate_native(&expected, &paths, log, 0.1, &total).is_err());
        // 原生输出分别舍入三位有效数字，1.43 与 1.01+.214+.214 相容。
        fs::write(
            &paths[0],
            "0 1.43 1.01 0.214 0.214\n1 11.3 5.67 2.79 2.79\n",
        )
        .unwrap();
        assert!(aggregate_native(&expected, &paths, log, 0., &total).is_ok());
        fs::write(&paths[0], "0 12 1 2 3\n1 18 2 3 4\n").unwrap();
        assert!(aggregate_native(&expected, &paths, log, 0., &total).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
