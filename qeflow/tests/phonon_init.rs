//! 验证真实 CLI/生成脚本，只用替身替换外部 QE 与 Slurm。
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
fn setup() -> Work {
    let root = std::env::temp_dir().join(format!(
        "init-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("bin")).unwrap();
    fs::create_dir_all(root.join("calc/custom-save/Si.save")).unwrap();
    fs::write(
        root.join("POSCAR"),
        "Si\n1\n3 0 0\n0 3 0\n0 0 3\nSi\n1\nDirect\n0 0 0\n",
    )
    .unwrap();
    for name in ["data-file-schema.xml", "charge-density.dat", "wfc1.dat"] {
        fs::write(root.join("calc/custom-save/Si.save").join(name), "fixture").unwrap();
    }
    let helper = r#"#!/usr/bin/env python3
import os,sys
from pathlib import Path
root=Path(os.environ['INIT_ROOT']); name=Path(sys.argv[0]).name
(root/'calls').open('a').write(name+' '+' '.join(sys.argv[1:])+'\n')
if name=='sbatch':
 if os.environ.get('BAD_SUBMIT'): print('invalid');sys.exit(0)
 (root/'submission').write_text(str(Path.cwd()/sys.argv[-1])); print('1234;cluster');sys.exit(0)
s=sys.stdin.read();(root/'consumed').write_text(s)
if os.environ.get('FAIL_PH'): print('JOB DONE.\nerror in routine mock');sys.exit(0)
if not os.environ.get('NO_DYN'):
 Path('Si.dyn0').write_text('2 2 1\n2\n0 0 0\n0 -0.577350269189585 0\n' if not os.environ.get('BAD_DYN') else '2 2 1\n2\n0 0 0\n')
print('JOB DONE.')
"#;
    for name in ["ph.x", "pw.x", "q2r.x", "matdyn.x", "sbatch"] {
        let p = root.join("bin").join(name);
        fs::write(&p, helper).unwrap();
        fs::set_permissions(p, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fs::write(root.join("input.toml"),format!("[qe]\nexecutable_path='{}'\n[qe_phonon]\nwork_dir='calc'\n[qe_phonon.scf]\nprefix='Si'\noutdir='custom-save'\n[qe_phonon.ph]\nqpoints=[2,2,1]\nonly_init=false\nonly_wfc=true\nldisp=false\nlqdir=false\nrecover=true\nstart_q=2\nlast_q=2\nstart_irr=2\nlast_irr=2\n",root.join("bin").display())).unwrap();
    Work(root)
}
fn env(w: &Work, c: &mut Command) {
    c.current_dir("/").env("INIT_ROOT", &w.0).env(
        "PATH",
        format!(
            "{}:{}",
            w.0.join("bin").display(),
            std::env::var("PATH").unwrap()
        ),
    );
}
fn cli(w: &Work, args: &[&str], extras: &[(&str, &str)]) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_qeflow"));
    env(w, &mut c);
    c.args(["qe-ph-nosplit", "--dyn0", "-c"])
        .arg(w.0.join("input.toml"))
        .args(args);
    for (k, v) in extras {
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
#[test]
fn prepare_and_execute_only_initialization_from_other_cwd() {
    let w = setup();
    ok(cli(&w, &[], &[]));
    assert!(!w.0.join("calls").exists());
    let input = fs::read_to_string(w.0.join("calc/ph_init.in")).unwrap();
    for key in ["ldisp", "only_init", "lqdir"] {
        assert!(input.contains(&format!("{key} = .true.")));
    }
    for key in ["recover", "only_wfc"] {
        assert!(input.contains(&format!("{key} = .false.")));
    }
    for key in ["start_q", "last_q", "start_irr", "last_irr"] {
        assert!(!input.contains(key));
    }
    assert!(!cli(&w, &["-r", "-s"], &[]).status.success());
    ok(cli(&w, &["-r"], &[]));
    assert_eq!(fs::read_to_string(w.0.join("calls")).unwrap(), "ph.x \n");
    assert!(w.0.join("calc/ph_init_complete.json").exists());
    for name in [
        "ph.out",
        "scf.in",
        "workflow_status.json",
        "chain_complete.json",
    ] {
        assert!(!w.0.join("calc").join(name).exists());
    }
}
#[test]
fn submit_executes_same_checked_initialization_script() {
    let w = setup();
    ok(cli(&w, &["-s"], &[]));
    assert!(!w.0.join("calc/ph_init_complete.json").exists());
    let script = fs::read_to_string(w.0.join("submission")).unwrap();
    let mut c = Command::new("bash");
    env(&w, &mut c);
    ok(c.arg(script).output().unwrap());
    assert!(w.0.join("calc/ph_init_complete.json").exists());
    assert_eq!(
        fs::read_to_string(w.0.join("calls"))
            .unwrap()
            .lines()
            .filter(|l| l.starts_with("ph.x"))
            .count(),
        1
    );
    assert!(!cli(&w, &["-s"], &[("BAD_SUBMIT", "1")]).status.success());
}
#[test]
fn missing_save_or_wavefunctions_blocks_run_and_submit() {
    let w = setup();
    fs::remove_file(w.0.join("calc/custom-save/Si.save/wfc1.dat")).unwrap();
    assert!(!cli(&w, &["-r"], &[]).status.success());
    assert!(!cli(&w, &["-s"], &[]).status.success());
    assert!(!w.0.join("calls").exists());
    fs::write(w.0.join("calc/custom-save/Si.save/wfc1.dat"), "wfc").unwrap();
    fs::remove_file(w.0.join("calc/custom-save/Si.save/charge-density.dat")).unwrap();
    assert!(!cli(&w, &["-r"], &[]).status.success());
}
#[test]
fn qe_failure_bad_dyn0_and_stale_dyn0_never_mark_complete() {
    let w = setup();
    for key in ["FAIL_PH", "BAD_DYN", "NO_DYN"] {
        ok(cli(&w, &["-r"], &[]));
        assert!(!cli(&w, &["-r"], &[(key, "1")]).status.success());
        assert!(!w.0.join("calc/ph_init_complete.json").exists());
    }
}
