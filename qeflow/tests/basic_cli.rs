//! MgB2 参考输入 + 真正子进程边界；不依赖计算集群即可暴露空 -r 实现。
use std::os::unix::fs::PermissionsExt;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

struct Work(PathBuf);
impl Drop for Work {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn setup() -> Work {
    let dir = std::env::temp_dir().join(format!(
        "mgb2-qe-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    fs::create_dir_all(dir.join("pp")).unwrap();
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_auto/MgB2");
    fs::copy(assets.join("MgB2.vasp"), dir.join("POSCAR")).unwrap();
    for entry in fs::read_dir(assets.join("pp")).unwrap() {
        let e = entry.unwrap();
        if e.file_type().unwrap().is_file() {
            fs::copy(e.path(), dir.join("pp").join(e.file_name())).unwrap();
        }
    }
    let bin = dir.join("qe bin");
    fs::create_dir_all(&bin).unwrap();
    script(
        &bin.join("pw.x"),
        "#!/bin/sh\ncat > consumed.in\nprintf 'JOB DONE.\\n'\n",
    );
    fs::write(
        dir.join("input.toml"),
        fs::read_to_string(assets.join("input_qe.toml"))
            .unwrap()
            .replace("@QE_EXEC@", &format!("'{}'", bin.display())),
    )
    .unwrap();
    Work(dir)
}
fn script(p: &Path, s: &str) {
    fs::write(p, s).unwrap();
    fs::set_permissions(p, fs::Permissions::from_mode(0o755)).unwrap();
}
fn cli(w: &Work, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_qeflow"))
        .args(args)
        .current_dir(&w.0)
        .output()
        .unwrap()
}
fn ok(o: std::process::Output) {
    assert!(
        o.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
}
fn semantic(text: &str) -> (std::collections::BTreeMap<String, String>, Vec<String>) {
    let mut keys = std::collections::BTreeMap::new();
    let mut cards = Vec::new();
    let mut section = String::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if line.starts_with('&') {
            section = line.into();
            continue;
        }
        if line == "/" {
            section.clear();
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            keys.insert(format!("{section}.{}", k.trim()), v.trim().into());
        } else {
            cards.extend(
                line.replace(['{', '}'], "")
                    .split_whitespace()
                    .map(String::from),
            );
        }
    }
    (keys, cards)
}
fn scalar(a: &str, b: &str) {
    match (a.parse::<f64>(), b.parse::<f64>()) {
        (Ok(a), Ok(b)) => assert!((a - b).abs() <= 1e-8 + 1e-6 * a.abs(), "{a} != {b}"),
        _ => assert_eq!(a, b),
    }
}
#[test]
fn mpi_submit_script_uses_loaded_environment_instead_of_configured_directory() {
    let w = setup();
    let mpi = w.0.join("MPI runtime");
    fs::create_dir(&mpi).unwrap();
    script(&mpi.join("mpirun"), "#!/bin/bash\n[[ $1 == -np && $2 == 2 ]] || exit 91\nshift 2\n\"$@\"\n");
    let path = w.0.join("input.toml");
    let old = format!("'{}'", w.0.join("qe bin").display());
    let cfg = fs::read_to_string(&path).unwrap()
        .replace(&old, &format!("exec '/obsolete MPI/mpirun' -np 2 {old}"))
        .replace("#SBATCH --job-name=test_qe", &format!("#SBATCH --job-name=test_qe\nexport PATH={}:$PATH", qf_runner::shlex_quote(mpi.to_str().unwrap())));
    fs::write(&path, cfg).unwrap();
    ok(cli(&w, &["qe-scf", "-i", "POSCAR"]));
    let submit = w.0.join("qe_scf/run_qe.sh");
    let body = fs::read_to_string(&submit).unwrap();
    assert!(!body.contains("obsolete MPI") && !body.contains("exec "));
    ok(Command::new("/bin/bash").arg(&submit).current_dir(w.0.join("qe_scf")).env("PATH", "/usr/bin:/bin").output().unwrap());
    assert!(w.0.join("qe_scf/consumed.in").is_file());
    script(&w.0.join("qe bin/pw.x"), "#!/bin/bash\nexit 17\n");
    let failure = Command::new("/bin/bash").arg(&submit).current_dir(w.0.join("qe_scf")).output().unwrap();
    assert_eq!(failure.status.code(), Some(17));
}

#[test]
fn mgb2_scf_nscf_reference_run_and_save_copy() {
    let w = setup();
    ok(cli(&w, &["qe-scf", "--input", "POSCAR", "--run"]));
    let save = w.0.join("qe_scf/tmp/Mg1B2.save");
    fs::create_dir_all(&save).unwrap();
    for name in [
        "data-file-schema.xml",
        "charge-density.dat",
        "paw.txt",
        "wfc1.dat",
    ] {
        fs::write(save.join(name), name).unwrap();
    }
    ok(cli(&w, &["qe-nscf", "-i", "POSCAR", "-r"]));
    for kind in ["scf", "nscf"] {
        let work = w.0.join(format!("qe_{kind}"));
        assert!(fs::read_to_string(work.join(format!("{kind}.out")))
            .unwrap()
            .contains("JOB DONE"));
        let input = fs::read_to_string(work.join(format!("{kind}.in"))).unwrap();
        assert_eq!(input, fs::read_to_string(work.join("consumed.in")).unwrap());
        let reference = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("../test_auto/MgB2/rust-reference/{kind}.in")),
        )
        .unwrap();
        let (mut a, ac) = semantic(&reference);
        let (mut b, bc) = semantic(&input);
        // Both roots vary; verify target pseudo directory, then omit only that absolute root.
        assert!(b["&CONTROL.pseudo_dir"].contains("/pp'"));
        a.remove("&CONTROL.pseudo_dir");
        b.remove("&CONTROL.pseudo_dir");
        assert_eq!(a.keys().collect::<Vec<_>>(), b.keys().collect::<Vec<_>>());
        for (k, v) in a {
            scalar(&v, &b[&k]);
        }
        assert_eq!(ac.len(), bc.len());
        for (a, b) in ac.iter().zip(&bc) {
            scalar(a, b);
        }
    }
    for name in ["data-file-schema.xml", "charge-density.dat", "paw.txt"] {
        assert_eq!(
            fs::read(save.join(name)).unwrap(),
            fs::read(w.0.join("qe_nscf/tmp/Mg1B2.save").join(name)).unwrap()
        );
    }
    assert!(!w.0.join("qe_nscf/tmp/Mg1B2.save/wfc1.dat").exists());
}
#[test]
fn failed_calculation_propagates_and_submit_runs_sbatch() {
    let w = setup();
    script(&w.0.join("qe bin/pw.x"), "#!/bin/sh\nexit 23\n");
    assert!(!cli(&w, &["qe-scf", "-r"]).status.success());
    script(
        &w.0.join("qe bin/sbatch"),
        "#!/bin/sh\npwd > submitted.cwd\nprintf '%s' \"$1\" > submitted.script\n",
    );
    let o = Command::new(env!("CARGO_BIN_EXE_qeflow"))
        .args(["qe-nscf", "-s"])
        .env(
            "PATH",
            format!(
                "{}:{}",
                w.0.join("qe bin").display(),
                std::env::var("PATH").unwrap()
            ),
        )
        .current_dir(&w.0)
        .output()
        .unwrap();
    ok(o);
    assert_eq!(
        fs::read_to_string(w.0.join("qe_nscf/submitted.script")).unwrap(),
        "run_qe.sh"
    );
    assert!(fs::read_to_string(w.0.join("qe_nscf/submitted.cwd"))
        .unwrap()
        .trim()
        .ends_with("/qe_nscf"));
    assert!(!cli(&w, &["qe-scf", "-r", "-s"]).status.success());
}

#[test]
fn mgb2_eband_and_eledos_run_all_external_steps() {
    let w = setup();
    let bin = w.0.join("qe bin");
    for name in ["bands.x", "projwfc.x", "dos.x"] {
        script(
            &bin.join(name),
            "#!/bin/sh\ncat > \"$(basename \"$0\").consumed\"\necho DONE\n",
        );
    }
    let kpath =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_auto/MgB2/rust-reference/KPATH.in");
    script(
        &bin.join("vaspkit"),
        &format!(
            "#!/bin/sh\ncat > menu.txt\ncp '{}' KPATH.in\n",
            kpath.display()
        ),
    );
    let save = w.0.join("qe_scf/tmp/Mg1B2.save");
    fs::create_dir_all(&save).unwrap();
    fs::write(save.join("charge-density.dat"), "charge").unwrap();
    fs::write(save.join("wfc1.dat"), "wave").unwrap();
    for command in ["qe-eband", "qe-el-edos"] {
        ok(Command::new(env!("CARGO_BIN_EXE_qeflow"))
            .args([command, "-i", "POSCAR", "-r"])
            .env(
                "PATH",
                format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
            )
            .current_dir(&w.0)
            .output()
            .unwrap());
    }
    for output in [
        "qe_eband/eband.out",
        "qe_eband/elebanddata.out",
        "qe_eband/elebandprojdata.out",
        "qe_eledos/eletdos.out",
        "qe_eledos/elepdos.out",
    ] {
        assert!(w.0.join(output).is_file(), "未执行: {output}");
    }
    assert_eq!(
        fs::read_to_string(w.0.join("qe_eband/menu.txt")).unwrap(),
        "3\n303\n"
    );
    assert!(w
        .0
        .join("qe_eband/tmp/Mg1B2.save/charge-density.dat")
        .is_file());
    assert!(!w.0.join("qe_eband/tmp/Mg1B2.save/wfc1.dat").exists());
    assert!(w.0.join("qe_eledos/tmp/Mg1B2.save/wfc1.dat").is_file());
}
