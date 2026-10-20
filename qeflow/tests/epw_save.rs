//! 现有 qeflow nosplit 主路径的进程回归；fixture 仅证明编排和归档，不作为科学数据。
#![cfg(unix)]
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Case(PathBuf);
impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
impl Case {
    fn new(retain: bool) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qe-epw-save-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("bin")).unwrap();
        fs::create_dir_all(root.join("calc")).unwrap();
        fs::create_dir_all(root.join("pp")).unwrap();
        fs::write(root.join("pp/Si.UPF"), "pseudo fixture").unwrap();
        fs::write(
            root.join("POSCAR"),
            "Si\n1\n3 0 0\n0 3 0\n0 0 3\nSi\n1\nDirect\n0 0 0\n",
        )
        .unwrap();
        fs::write(root.join("calc/KPATH.in"), "0 0 0 G\n0.5 0 0 X\n").unwrap();
        fs::write(root.join("input.toml"), format!("pp_dir='{}'\n[qe]\nexecutable_path='{}'\nlattice_mode='cell_parameters'\n[qe_phonon]\nwork_dir='{}'\nprefix='Si'\nretain_epw_data={retain}\n[qe_phonon.scf]\noutdir='./out'\nkpoints=[2,2,1]\necutwfc=40\n[qe_phonon.ph]\nqpoints=[2,2,1]\nlqdir=false\n[qe_phonon.phband]\nkpath_points=2\n[qe_phonon.phdos]\nqpoints=[2,2,2]\n",root.join("pp").display(),root.join("bin").display(),root.join("calc").display())).unwrap();
        let fixture = r#"#!/usr/bin/python3
import os,re,sys
from pathlib import Path
root=Path(os.environ['QE_EPW_TEST_ROOT']); name=Path(sys.argv[0]).name
(root/'calls').open('a').write(name+'\n')
s=sys.stdin.read()
if name=='pw.x':
 p=Path('out/Si.save'); p.mkdir(parents=True,exist_ok=True)
 (p/'data-file-schema.xml').write_text('<fixture/>')
 (p/'charge-density.dat').write_text('density'); (p/'wfc1.dat').write_text('wfc')
 print('convergence has been achieved\nJOB DONE.'); sys.exit(0)
if name=='ph.x':
 assert Path('out/Si.save/wfc1.dat').is_file()
 Path('Si.dyn0').write_text('2 2 1\n2\n0 0 0\n0.5 0 0\n')
 suffix='.xml' if "fildyn = 'Si.dyn.xml'" in s else ''
 for q in (1,2): Path('Si.dyn'+str(q)+suffix).write_text('<fixture-dyn/>')
 if suffix:
  assert 'lqdir = .true.' in s and "fildvscf = 'dvscf'" in s
  p=Path('out/_ph0/Si.phsave'); p.mkdir(parents=True,exist_ok=True)
  (p/'control_ph.xml').write_text('<fixture-control/>')
  for q in (1,2):
   (p/('patterns.'+str(q)+'.xml')).write_text('<fixture-pattern/>')
   d=Path('out/_ph0') if q==1 else Path('out/_ph0/Si.q_'+str(q))
   d.mkdir(parents=True,exist_ok=True); (d/'Si.dvscf1').write_bytes(bytes([q])*32)
if name=='q2r.x':
 assert Path('Si.dyn0').is_file()
 if "Si.dyn.xml" in s: assert Path('Si.dyn2.xml').is_file()
 if os.environ.get('QE_FAIL_Q2R'):
  print('Error in routine fixture'); sys.exit(2)
 Path('Si.fc.xml' if "Si.dyn.xml" in s else 'Si.fc').write_text('force constants')
if name=='matdyn.x':
 force=re.search(r"flfrc\s*=\s*'([^']+)'",s).group(1)
 assert Path(force).is_file(), 'q2r product does not match matdyn input'
 if 'fldos' in s: Path('Si.phdos.dat').write_text('0 0.1 0.1\n1 0.2 0.2\n2 0.1 0.1\n')
 else: Path('Si.phband.freq').write_text('&plot nbnd=3, nks=3 /\n0 0 0\n0 1 2\n0.25 0 0\n0.5 1.5 2.5\n0.5 0 0\n1 2 3\n')
print('JOB DONE.')
"#;
        for name in ["pw.x", "ph.x", "q2r.x", "matdyn.x"] {
            let p = root.join("bin").join(name);
            fs::write(&p, fixture).unwrap();
            fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
        }
        Self(root)
    }
    fn command(&self) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_qeflow"));
        c.args(["qe-phonon-nosplit", "-c", "input.toml", "-i", "POSCAR"])
            .current_dir(&self.0)
            .env("QE_EPW_TEST_ROOT", &self.0)
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    self.0.join("bin").display(),
                    std::env::var("PATH").unwrap()
                ),
            );
        c
    }
}

#[test]
fn submitted_nosplit_script_runs_post_once_and_exports_from_frozen_config() {
    let c = Case::new(true);
    let sbatch = c.0.join("bin/sbatch");
    fs::write(&sbatch, "#!/bin/sh\nfor script do :; done\nbash \"$script\" > worker.log 2>&1 || exit $?\necho 12345\n").unwrap();
    fs::set_permissions(&sbatch, fs::Permissions::from_mode(0o755)).unwrap();
    let result = c.command().arg("-s").output().unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stderr),
        fs::read_to_string(c.0.join("calc/worker.log")).unwrap_or_default()
    );
    assert_eq!(
        fs::read_to_string(c.0.join("calls")).unwrap(),
        "pw.x\nph.x\nq2r.x\nmatdyn.x\nmatdyn.x\n"
    );
    assert!(c.0.join("calc/save/manifest.json").is_file());
}

#[test]
fn retain_option_rejects_unadapted_split_execution_before_science() {
    let c = Case::new(true);
    let out = Command::new(env!("CARGO_BIN_EXE_qeflow"))
        .args([
            "qe-phonon-split-coords",
            "-c",
            "input.toml",
            "-i",
            "POSCAR",
            "-r",
        ])
        .current_dir(&c.0)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("retain_epw_data"));
    assert!(!c.0.join("calls").exists());
}

#[test]
fn nosplit_runs_postprocessing_once_and_exports_paired_epw_data() {
    let c = Case::new(true);
    let prepared = c.command().output().unwrap();
    assert!(
        prepared.status.success(),
        "{}",
        String::from_utf8_lossy(&prepared.stderr)
    );
    assert!(!c.0.join("calls").exists());
    let ph = fs::read_to_string(c.0.join("calc/ph.in")).unwrap();
    assert!(ph.contains("fildyn = 'Si.dyn.xml'") && ph.contains("lqdir = .true."));
    let result = c.command().arg("-r").output().unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::read_to_string(c.0.join("calls")).unwrap(),
        "pw.x\nph.x\nq2r.x\nmatdyn.x\nmatdyn.x\n"
    );
    let save = c.0.join("calc/save");
    let m: qf_phonon::epw::Manifest =
        serde_json::from_slice(&fs::read(save.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(m.qpoints, 2);
    assert_eq!(fs::read(save.join("Si.dvscf_q1")).unwrap(), vec![1; 32]);
    assert_eq!(fs::read(save.join("Si.dvscf_q2")).unwrap(), vec![2; 32]);
    assert!(c.0.join("calc/out/Si.save/charge-density.dat").is_file());
    assert!(c.0.join("calc/phonon_combined.jpg").is_file());
    let repeated =
        qf_phonon::epw::export_nosplit(&c.0.join("calc"), &c.0.join("calc/out"), "Si").unwrap();
    assert_eq!(repeated, save);
    fs::write(save.join("Si.dvscf_q2"), "tampered").unwrap();
    assert!(
        qf_phonon::epw::export_nosplit(&c.0.join("calc"), &c.0.join("calc/out"), "Si").is_err()
    );
}

#[test]
fn export_rejects_missing_responses_and_conflicting_destination() {
    let c = Case::new(true);
    let output = c.command().arg("-r").output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let work = c.0.join("calc");
    fs::remove_dir_all(work.join("save")).unwrap();
    fs::remove_file(work.join("out/_ph0/Si.q_2/Si.dvscf1")).unwrap();
    assert!(qf_phonon::epw::export_nosplit(&work, &work.join("out"), "Si").is_err());
    assert!(!work.join("save").exists());
    fs::write(work.join("out/_ph0/Si.q_2/Si.dvscf1"), "restored fixture").unwrap();
    fs::create_dir(work.join("save")).unwrap();
    fs::write(work.join("save/user-data"), "keep").unwrap();
    assert!(qf_phonon::epw::export_nosplit(&work, &work.join("out"), "Si").is_err());
    assert_eq!(
        fs::read_to_string(work.join("save/user-data")).unwrap(),
        "keep"
    );
}

#[test]
fn option_disabled_keeps_original_names_and_failure_stops_downstream() {
    let c = Case::new(false);
    assert!(c.command().output().unwrap().status.success());
    let ph = fs::read_to_string(c.0.join("calc/ph.in")).unwrap();
    assert!(ph.contains("fildyn = 'Si.dyn'") && ph.contains("lqdir = .false."));
    let result = c
        .command()
        .env("QE_FAIL_Q2R", "1")
        .arg("-r")
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert_eq!(
        fs::read_to_string(c.0.join("calls")).unwrap(),
        "pw.x\nph.x\nq2r.x\n"
    );
    assert!(!c.0.join("calc/save").exists());
    assert!(!c.0.join("calc/phonon_combined.jpg").exists());
}
