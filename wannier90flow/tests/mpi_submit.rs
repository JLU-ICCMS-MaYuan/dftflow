use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

#[test]
fn generated_wannier_scripts_use_environment_mpi_and_stop_after_failure() {
    let root = std::env::temp_dir().join(format!("w90-mpi-{}", std::process::id()));
    fs::create_dir_all(root.join("MPI runtime")).unwrap();
    for (path, body) in [
        (
            root.join("MPI runtime/mpirun"),
            "#!/bin/bash\n[[ $1 == -np && $2 == 2 ]] || exit 91\nshift 2\n\"$@\"\n",
        ),
        (
            root.join("science program"),
            "#!/bin/bash\necho called >> calls\nexit \"${SCIENCE_EXIT:-0}\"\n",
        ),
    ] {
        fs::write(&path, body).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let mut cfg = wannier90flow::config::base_template();
    let call = format!(
        "exec '/obsolete MPI/mpirun' -np 2 {}",
        qf_runner::shlex_quote(root.join("science program").to_str().unwrap())
    );
    for section in ["wannier90", "pw2wannier90", "postw90"] {
        cfg[section]["executable_path"] = json!(call);
    }
    cfg["slurm"]["header"] = json!(format!(
        "#!/bin/bash\nexport PATH={}:$PATH\n",
        qf_runner::shlex_quote(root.join("MPI runtime").to_str().unwrap())
    ));
    wannier90flow::init::create_run_script(root.to_str().unwrap(), "seed", &cfg).unwrap();
    wannier90flow::post::create_run_script(root.to_str().unwrap(), "seed", &cfg).unwrap();
    for (name, count) in [("run_wannier90.sh", 3), ("run_postw90.sh", 1)] {
        let path = root.join(name);
        let body = fs::read_to_string(&path).unwrap();
        assert!(!body.contains("exec ") && !body.contains("obsolete MPI"));
        let run = |code| {
            Command::new("/bin/bash")
                .arg(&path)
                .env("PATH", "/usr/bin:/bin")
                .env("SCIENCE_EXIT", code)
                .output()
                .unwrap()
        };
        let _ = fs::remove_file(root.join("calls"));
        assert!(run("0").status.success());
        assert_eq!(
            fs::read_to_string(root.join("calls"))
                .unwrap()
                .lines()
                .count(),
            count
        );
        fs::remove_file(root.join("calls")).unwrap();
        assert_eq!(run("17").status.code(), Some(17));
        assert_eq!(
            fs::read_to_string(root.join("calls"))
                .unwrap()
                .lines()
                .count(),
            1
        );
    }
    fs::remove_dir_all(root).unwrap();
}
