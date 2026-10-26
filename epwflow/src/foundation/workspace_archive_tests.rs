use super::*;
use std::io::Write;
use std::os::unix::fs::{symlink, MetadataExt};
use std::process::Stdio;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("stable epw {}", store::token()));
        fs::create_dir_all(root.join("7.epw-fine")).unwrap();
        Self(root)
    }
    fn dir(&self) -> PathBuf {
        self.0.join("7.epw-fine")
    }
    fn history(&self) -> PathBuf {
        self.0.join("history/calculation")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn shell_already_inside_stage_observes_new_files_without_cd() {
    let f = Fixture::new();
    fs::write(f.dir().join("old.out"), b"old science").unwrap();
    fs::create_dir(f.dir().join("old-data")).unwrap();
    fs::write(f.dir().join("old-data/input"), b"original input").unwrap();
    let inode = fs::metadata(f.dir()).unwrap().ino();
    let mut shell = Command::new("/bin/bash")
        .args([
            "-c",
            "read -r signal; test -f fresh.in && test ! -e old.out && pwd -P",
        ])
        .current_dir(f.dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    archive_contents(&f.dir(), &f.history(), None).unwrap();
    fs::write(f.dir().join("fresh.in"), b"new input").unwrap();
    shell
        .stdin
        .take()
        .unwrap()
        .write_all(b"continue\n")
        .unwrap();
    let output = shell.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        f.dir().to_str().unwrap()
    );
    assert_eq!(inode, fs::metadata(f.dir()).unwrap().ino());
    assert_eq!(
        fs::read(f.history().join("old.out")).unwrap(),
        b"old science"
    );
    assert_eq!(
        fs::read(f.history().join("old-data/input")).unwrap(),
        b"original input"
    );
}

#[test]
fn active_log_stays_in_place_and_keeps_open_writer() {
    let f = Fixture::new();
    for name in ["slurm-fine.log", "slurm-123.out", "slurm-123.err"] {
        let path = f.dir().join(name);
        let mut writer = fs::File::create(&path).unwrap();
        writer.write_all(b"before\n").unwrap();
        let inode = writer.metadata().unwrap().ino();
        let history = f.0.join(format!("archive-{name}"));
        archive_contents(&f.dir(), &history, Some(123)).unwrap();
        writer.write_all(b"after\n").unwrap();
        assert_eq!(inode, fs::metadata(&path).unwrap().ino());
        assert_eq!(inode, fs::metadata(history.join(name)).unwrap().ino());
        assert_eq!(fs::read(path).unwrap(), b"before\nafter\n");
    }
}

#[test]
fn inactive_logs_and_symlinks_are_archived_without_following_targets() {
    let f = Fixture::new();
    fs::write(f.0.join("external"), b"keep").unwrap();
    fs::write(f.dir().join("slurm-99.out"), b"old log").unwrap();
    symlink(f.0.join("external"), f.dir().join("slurm-fine.log")).unwrap();
    archive_contents(&f.dir(), &f.history(), Some(123)).unwrap();
    assert!(!f.dir().join("slurm-99.out").exists());
    assert!(fs::symlink_metadata(f.history().join("slurm-fine.log"))
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(fs::read(f.0.join("external")).unwrap(), b"keep");
}

#[test]
fn destination_conflict_stops_before_moving_any_source() {
    let f = Fixture::new();
    fs::write(f.dir().join("a.in"), b"a").unwrap();
    fs::write(f.dir().join("z.out"), b"z").unwrap();
    fs::create_dir_all(f.history()).unwrap();
    fs::write(f.history().join("z.out"), b"unknown").unwrap();
    let error = archive_contents(&f.dir(), &f.history(), None)
        .unwrap_err()
        .to_string();
    assert!(error.contains("归档目标已存在"));
    assert_eq!(fs::read(f.dir().join("a.in")).unwrap(), b"a");
    assert_eq!(fs::read(f.dir().join("z.out")).unwrap(), b"z");
    assert_eq!(fs::read(f.history().join("z.out")).unwrap(), b"unknown");
}

#[test]
fn invalid_destination_leaves_stage_available() {
    let f = Fixture::new();
    fs::write(f.dir().join("old.out"), b"keep").unwrap();
    fs::write(f.0.join("history"), b"not a directory").unwrap();
    assert!(archive_contents(&f.dir(), &f.history(), None).is_err());
    assert_eq!(fs::read(f.dir().join("old.out")).unwrap(), b"keep");
}
