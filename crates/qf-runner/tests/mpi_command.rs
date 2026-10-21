use qf_runner::{plain_scientific_argv, plain_scientific_command};
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

#[test]
fn only_scientific_prefix_changes() {
    for (before, after) in [
        (
            "mpirun -np 4 '/QE suite/pw.x' < in > out",
            "mpirun -np 4 '/QE suite/pw.x' < in > out",
        ),
        (
            "exec '/MPI suite/bin/mpirun' -np 4 \"$QE/pw.x\" >> \"$OUT\" 2>&1",
            "mpirun -np 4 \"$QE/pw.x\" >> \"$OUT\" 2>&1",
        ),
        ("  exec /mpi/mpirun\t-np 2 pw.x", "  mpirun\t-np 2 pw.x"),
        ("\"${MPI_ROOT}/bin/mpirun\" -np 2 pw.x", "mpirun -np 2 pw.x"),
        ("/MPI\\ suite/mpirun -np 2 pw.x", "mpirun -np 2 pw.x"),
        (
            "exec '/QE suite/wannier90.x' seed",
            "'/QE suite/wannier90.x' seed",
        ),
        ("/mpi/mpiexec -n 2 pw.x", "/mpi/mpiexec -n 2 pw.x"),
        ("/site/srun -n 2 pw.x", "/site/srun -n 2 pw.x"),
        (
            "/site/custom-launcher -n 2 pw.x",
            "/site/custom-launcher -n 2 pw.x",
        ),
        ("echo '/mpi/mpirun'", "echo '/mpi/mpirun'"),
        ("exec > log", "exec > log"),
    ] {
        assert_eq!(plain_scientific_command(before), after, "{before}");
    }
    let argv = [
        "exec",
        "/MPI suite/mpirun",
        "-np",
        "4",
        "/QE suite/pw.x",
        "$literal",
    ]
    .map(str::to_owned);
    assert_eq!(
        plain_scientific_argv(&argv),
        ["mpirun", "-np", "4", "/QE suite/pw.x", "$literal"]
    );
}

#[test]
fn generated_script_uses_environment_path_and_preserves_failures() {
    let root = std::env::temp_dir().join(format!(
        "mpi-command-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(root.join("MPI suite")).unwrap();
    let launcher = root.join("MPI suite/mpirun");
    fs::write(
        &launcher,
        "#!/bin/bash\n[[ $1 == -np && $2 == 4 ]] || exit 91\nshift 2\n\"$@\"\n",
    )
    .unwrap();
    fs::set_permissions(&launcher, fs::Permissions::from_mode(0o755)).unwrap();
    let program = root.join("science program");
    fs::write(&program, "#!/bin/bash\n[[ $1 == 'argument with spaces' ]] || exit 92\nprintf 'science-output\\n'\nexit \"${SCIENCE_EXIT:-0}\"\n").unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    let header = format!(
        "# user's header: exec /old/mpirun\nexport PATH={}\n",
        qf_runner::shlex_quote(root.join("MPI suite").to_str().unwrap())
    );
    let command = plain_scientific_command(&format!(
        "exec '/nonexistent MPI/mpirun' -np 4 {} 'argument with spaces' > {}",
        qf_runner::shlex_quote(program.to_str().unwrap()),
        qf_runner::shlex_quote(root.join("science.out").to_str().unwrap())
    ));
    let script = qf_runner::write_job_script(&root, "submit.sh", &header, &[command]).unwrap();
    let run = |code| {
        Command::new("/bin/bash")
            .arg(&script)
            .env("PATH", "/nonexistent-login-path")
            .env("SCIENCE_EXIT", code)
            .output()
            .unwrap()
    };
    assert!(run("0").status.success());
    assert_eq!(
        fs::read_to_string(root.join("science.out")).unwrap(),
        "science-output\n"
    );
    assert_eq!(run("17").status.code(), Some(17));
    fs::remove_file(launcher).unwrap();
    assert_eq!(run("0").status.code(), Some(127));
    fs::remove_dir_all(root).unwrap();
}
