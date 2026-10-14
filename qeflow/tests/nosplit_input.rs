use std::{fs, path::PathBuf, process::Command};

struct Work(PathBuf);
impl Drop for Work {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn setup() -> Work {
    let dir = std::env::temp_dir().join(format!(
        "nosplit-input-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    fs::create_dir_all(dir.join("calc")).unwrap();
    fs::create_dir_all(dir.join("pp")).unwrap();
    fs::write(dir.join("pp/Si.UPF"), "pseudo").unwrap();
    fs::write(dir.join("input.toml"), "pp_dir = 'pp'\n[qe]\nlattice_mode = 'cell_parameters'\n[qe_phonon]\nwork_dir = 'calc'\nprefix = 'Si'\n").unwrap();
    fs::write(dir.join("calc/KPATH.in"), "0 0 0 G\n0.5 0 0 X\n").unwrap();
    Work(dir)
}

fn structure(a: usize) -> String {
    format!("Si\n1\n{a} 0 0\n0 {a} 0\n0 0 {a}\nSi\n1\nDirect\n0 0 0\n")
}

#[test]
fn explicit_input_and_aliases_override_existing_structure() {
    let w = setup();
    let selected = structure(4);
    fs::write(w.0.join("selected.vasp"), &selected).unwrap();
    fs::write(w.0.join("POSCAR"), structure(3)).unwrap();
    for option in ["-i", "--input", "-p", "--poscar"] {
        fs::write(w.0.join("calc/POSCAR"), structure(2)).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_qeflow"))
            .args([
                "qe-phonon-nosplit",
                "-c",
                "input.toml",
                option,
                "selected.vasp",
            ])
            .current_dir(&w.0)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            fs::read_to_string(w.0.join("calc/POSCAR")).unwrap(),
            selected
        );
        assert!(fs::read_to_string(w.0.join("calc/scf.in"))
            .unwrap()
            .contains("4.0000000000"));
    }
}

#[test]
fn omitted_input_preserves_automatic_lookup_and_missing_input_fails() {
    let w = setup();
    for (source, size) in [("CONTCAR", 3), ("POSCAR", 4), ("calc/POSCAR", 5)] {
        fs::write(w.0.join(source), structure(size)).unwrap();
        if source != "calc/POSCAR" {
            let _ = fs::remove_file(w.0.join("calc/POSCAR"));
        }
        let out = Command::new(env!("CARGO_BIN_EXE_qeflow"))
            .args(["qe-phonon-nosplit", "-c", "input.toml"])
            .current_dir(&w.0)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            fs::read_to_string(w.0.join("calc/POSCAR")).unwrap(),
            structure(size)
        );
    }
    let out = Command::new(env!("CARGO_BIN_EXE_qeflow"))
        .args(["qe-phonon-nosplit", "-i", "missing.vasp"])
        .current_dir(&w.0)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("找不到结构文件: missing.vasp"));
    assert_eq!(
        fs::read_to_string(w.0.join("calc/POSCAR")).unwrap(),
        structure(5)
    );
}
