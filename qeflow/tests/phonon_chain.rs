//! 跨进程测试真实 CLI、脚本及提交协议；外部 QE/Slurm 使用本地替身。
use std::os::unix::fs::PermissionsExt;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};
struct Work(PathBuf);
impl Drop for Work {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn setup(mode: &str) -> Work {
    let dir = std::env::temp_dir().join(format!(
        "chain-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("bin")).unwrap();
    fs::create_dir_all(dir.join("pp")).unwrap();
    fs::write(dir.join("pp/Si.UPF"), "pseudo").unwrap();
    fs::write(
        dir.join("POSCAR"),
        "Si\n1\n3 0 0\n0 3 0\n0 0 3\nSi\n1\nDirect\n0 0 0\n",
    )
    .unwrap();
    fs::create_dir_all(dir.join("calc")).unwrap();
    fs::write(dir.join("calc/KPATH.in"), "0 0 0 G\n0.5 0 0 X\n").unwrap();
    let helper = r#"#!/usr/bin/env python3
import json,os,sys,re
from pathlib import Path
root=Path(os.environ['CHAIN_TESTROOT']);name=Path(sys.argv[0]).name
calls=root/'calls';calls.open('a').write(name+' '+str(Path.cwd())+' '+' '.join(sys.argv[1:])+'\n')
if name=='sbatch':
 n=int((root/'counter').read_text())+1 if (root/'counter').exists() else 101
 (root/'counter').write_text(str(n))
 if os.environ.get('FAIL_SUBMIT')==str(n): sys.exit(2)
 if os.environ.get('BAD_ID'): print('not a job');sys.exit(0)
 jobs=json.loads((root/'states').read_text()) if (root/'states').exists() else {}
 jobs[str(n)]='PENDING';(root/'states').write_text(json.dumps(jobs));print(n);sys.exit(0)
if name in ('squeue','sacct'):
 if os.environ.get('FAIL_QUERY'): sys.exit(2)
 jobs=json.loads((root/'states').read_text());j=sys.argv[sys.argv.index('-j')+1]
 print(jobs.get(j,'COMPLETED'));sys.exit(0)
s=sys.stdin.read()
if name=='pw.x':
 p=Path('out/Si.save');p.mkdir(parents=True,exist_ok=True)
 (p/'data-file-schema.xml').write_text('<root><b1>1 0.577350269 0</b1><b2>0 1.154700538 0</b2><b3>0 0 1</b3><rotation>1 0 0 0 1 0 0 0 1</rotation></root>')
 (p/'charge-density.dat').write_text('density');(p/'wfc1.dat').write_text('wavefunction')
 print('convergence has been achieved\nJOB DONE.');sys.exit(0)
if name=='ph.x':
 if 'only_init = .true.' in s:
  Path('Si.dyn0').write_text('2 2 1\n2\n0 0 0\n0.5 0.2886751345 0\n')
 else:
  if os.environ.get('FAIL_Q')==Path.cwd().name: print('error in routine mock');sys.exit(2)
  idx=int(re.search(r'start_q\s*=\s*(\d+)',s).group(1)) if 'start_q' in s else 1
  cart='0 0 0' if Path.cwd().name=='01' else '0.5 0.2886751345 0'
  dest=Path('Si.dyn'+str(idx)) if 'ldisp = .true.' in s else Path('Si.dyn')
  dest.write_text('q = ( '+cart+' )\nfreq ( 1) = 1 [THz] = 33 [cm-1]\n')
  p=Path('out/_ph0/Si.phsave');p.mkdir(parents=True,exist_ok=True);(p/('patterns.'+str(idx)+'.xml')).write_text('patterns')
 print('JOB DONE.');sys.exit(0)
if name=='q2r.x':Path('Si.fc').write_text('forceconstants')
if name=='matdyn.x':
 if 'fldos' in s:Path('Si.phdos.dat').write_text('0 0.1 0.1\n1 0.2 0.2\n2 0.1 0.1\n')
 else:Path('Si.phband.freq').write_text('&plot nbnd=3, nks=2 /\n0 0 0\n0 1 2\n0.5 0 0\n1 2 3\n')
print('JOB DONE.')
"#;
    for name in [
        "pw.x", "ph.x", "q2r.x", "matdyn.x", "sbatch", "squeue", "sacct",
    ] {
        let p = dir.join("bin").join(name);
        fs::write(&p, helper).unwrap();
        fs::set_permissions(p, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fs::write(dir.join("input.toml"),format!("pp_dir='{}'\n[qe]\nexecutable_path='{}'\nlattice_mode='cell_parameters'\n[qe_phonon]\nwork_dir='{}'\nprefix='Si'\n[qe_phonon.scf]\noutdir='./out'\nkpoints=[2,2,1]\necutwfc=40\n[qe_phonon.ph]\nqpoints=[2,2,1]\n[qe_phonon.phband]\nkpath_points=2\n[qe_phonon.phdos]\nqpoints=[2,2,2]\n",dir.join("pp").display(),dir.join("bin").display(),dir.join("calc").display())).unwrap();
    fs::write(dir.join("mode"), mode).unwrap();
    Work(dir)
}
fn command(w: &Work) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_qeflow"));
    c.env("CHAIN_TESTROOT", &w.0)
        .env(
            "PATH",
            format!(
                "{}:{}",
                w.0.join("bin").display(),
                std::env::var("PATH").unwrap()
            ),
        )
        .env_remove("SLURM_JOB_ID")
        .current_dir("/");
    c
}
fn cli(w: &Work, more: &[&str], env: &[(&str, &str)]) -> Output {
    let mode = fs::read_to_string(w.0.join("mode")).unwrap();
    let mut c = command(w);
    c.args([
        format!("qe-phonon-split-{mode}"),
        "-c".into(),
        w.0.join("input.toml").to_string_lossy().into_owned(),
        "-i".into(),
        w.0.join("POSCAR").to_string_lossy().into_owned(),
    ])
    .args(more);
    for (k, v) in env {
        c.env(k, v);
    }
    c.output().unwrap()
}
fn ok(o: Output) {
    assert!(
        o.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
}
fn pre(w: &Work, env: &[(&str, &str)]) -> Output {
    let mut c = command(w);
    c.args([
        "qe-phonon-chain-stage",
        "-c",
        w.0.join("input.toml").to_str().unwrap(),
        "-i",
        w.0.join("POSCAR").to_str().unwrap(),
        "--mode",
        &fs::read_to_string(w.0.join("mode")).unwrap(),
        "--stage",
        "pre",
    ])
    .env("SLURM_JOB_ID", "101");
    for (k, v) in env {
        c.env(k, v);
    }
    c.output().unwrap()
}
#[test]
fn single_submission_fanout_dependencies_and_duplicate_guard() {
    let w = setup("index");
    ok(cli(&w, &["-s"], &[]));
    ok(pre(&w, &[]));
    ok(cli(&w, &["-s"], &[]));
    let calls = fs::read_to_string(w.0.join("calls")).unwrap();
    assert_eq!(calls.lines().filter(|l| l.starts_with("pw.x")).count(), 1);
    assert_eq!(calls.lines().filter(|l| l.starts_with("sbatch")).count(), 4);
    assert!(calls.contains("--dependency=afterok:101:102:103"));
    let left = w.0.join("calc/01/out/Si.save/wfc1.dat");
    fs::write(&left, "changed").unwrap();
    assert_eq!(
        fs::read_to_string(w.0.join("calc/02/out/Si.save/wfc1.dat")).unwrap(),
        "wavefunction"
    );
    assert!(!w.0.join("calc/01/scf.in").exists());
    assert!(fs::read_to_string(w.0.join("calc/02/split_ph.in"))
        .unwrap()
        .contains("start_q = 2"));
}
#[test]
fn local_coords_finishes_and_uses_cartesian_q() {
    let w = setup("coords");
    ok(cli(&w, &["-r"], &[]));
    let ph = fs::read_to_string(w.0.join("calc/02/split_ph.in")).unwrap();
    assert!(ph.contains("2.886751345000000E-1") || ph.contains("2.886751345000000e-1"));
    assert!(w.0.join("calc/chain_complete.json").exists());
    assert!(w.0.join("calc/phband/phonon_band.png").exists());
    let before = fs::read_to_string(w.0.join("calls")).unwrap();
    ok(cli(&w, &["-r"], &[]));
    assert_eq!(before, fs::read_to_string(w.0.join("calls")).unwrap());
}
#[test]
fn failed_q_blocks_all_postprocessing() {
    let w = setup("index");
    assert!(!cli(&w, &["-r"], &[("FAIL_Q", "02")]).status.success());
    let calls = fs::read_to_string(w.0.join("calls")).unwrap();
    assert!(!calls.lines().any(|l| l.starts_with("q2r.x")));
    assert!(!w.0.join("calc/chain_complete.json").exists());
    ok(cli(&w, &["-r"], &[]));
    assert_eq!(
        fs::read_to_string(w.0.join("calls"))
            .unwrap()
            .lines()
            .filter(|l| l.starts_with("pw.x"))
            .count(),
        1
    );
}
#[test]
fn partial_submit_is_recorded_and_invalid_id_is_rejected() {
    let w = setup("coords");
    ok(cli(&w, &["-s"], &[]));
    assert!(!pre(&w, &[("FAIL_SUBMIT", "103")]).status.success());
    let j: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(w.0.join("calc/chain_jobs.json")).unwrap())
            .unwrap();
    assert_eq!(j["q:01"], "102");
    assert!(j.get("post").is_none());
    assert!(!cli(&w, &["-s"], &[("FAIL_QUERY", "1")]).status.success());
    fs::write(w.0.join("states"), r#"{"101":"FAILED","102":"CANCELLED"}"#).unwrap();
    ok(cli(&w, &["-s"], &[]));
    ok(pre(&w, &[]));
    let j: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(w.0.join("calc/chain_jobs.json")).unwrap())
            .unwrap();
    assert!(j.get("post").is_some());
    assert_eq!(
        fs::read_to_string(w.0.join("calls"))
            .unwrap()
            .lines()
            .filter(|l| l.starts_with("pw.x"))
            .count(),
        1
    );
    let w2 = setup("index");
    assert!(!cli(&w2, &["-s"], &[("BAD_ID", "1")]).status.success());
}

#[test]
fn changed_inputs_reject_stale_results_and_old_scf_dirs_are_recognized() {
    let w = setup("index");
    ok(cli(&w, &[], &[]));
    let path = w.0.join("input.toml");
    let old = fs::read_to_string(&path).unwrap();
    fs::write(&path, old.replace("ecutwfc=40", "ecutwfc=50")).unwrap();
    assert!(!cli(&w, &["-s"], &[]).status.success());
    let legacy = w.0.join("legacy");
    fs::create_dir_all(legacy.join("01")).unwrap();
    fs::write(
        legacy.join("q_index_map.json"),
        r#"{"qpoints":[{"directory":"01","index":1}]}"#,
    )
    .unwrap();
    fs::write(legacy.join("01/scf.out"), "JOB DONE.").unwrap();
    fs::write(legacy.join("01/split_ph.out"), "JOB DONE.").unwrap();
    let state = qf_state::refresh_split_task_status(&legacy).unwrap();
    assert_eq!(state["01"], qf_state::TaskState::Finished);
}
