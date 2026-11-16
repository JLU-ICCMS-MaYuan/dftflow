//! 电荷密度／波函数不参与核验；复制与其他科学证据仍保留。
use epwflow::{foundation::store::{self, Identity}, qe_data::save};
use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    let root = std::env::temp_dir().join(format!("epw-unchecked-{}", store::token()));
    fs::create_dir(&root).unwrap();
    root
}

#[test]
fn raw_data_is_copied_but_not_part_of_integrity_records() {
    let root = root();
    let source = root.join("source");
    let target = root.join("target");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("data-file-schema.xml"), "protected XML").unwrap();
    for name in ["wfc1.dat", "charge-density.dat", "MgB2.wfc2", "wfcup1.hdf5"] {
        fs::write(source.join(name), vec![37; 65537]).unwrap();
    }
    let checked = save::copy_tree(&source, &target).unwrap();
    assert_eq!(checked.len(), 1);
    for name in ["wfc1.dat", "charge-density.dat", "MgB2.wfc2", "wfcup1.hdf5"] {
        assert_eq!(fs::read(source.join(name)).unwrap(), fs::read(target.join(name)).unwrap());
        fs::write(target.join(name), "").unwrap();
    }
    assert_eq!(save::files(&source).unwrap(), save::files(&target).unwrap());
    fs::write(target.join("data-file-schema.xml"), "changed XML").unwrap();
    assert_ne!(save::files(&source).unwrap(), save::files(&target).unwrap());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn old_snapshot_and_identity_records_ignore_missing_or_changed_raw_data() {
    let root = root();
    let snapshot = root.join("scf/snapshot");
    fs::create_dir_all(&snapshot).unwrap();
    fs::write(snapshot.join("data-file-schema.xml"), "protected XML").unwrap();
    let mut expected = save::files(&snapshot).unwrap();
    for name in ["wfc1.dat", "charge-density.dat"] {
        expected.push(save::TreeFile { relative_path: name.into(), byte_size: 999999, sha256: "old hash".into() });
        let id = Identity { path: snapshot.join(name), target: "/old/target".into(), sha256: "old hash".into(), bytes: 999999 };
        id.verify().unwrap();
        assert!(store::identity_available(&id).unwrap());
        save::nonempty(&snapshot.join(name)).unwrap();
    }
    store::write(&root.join("scf/snapshot-files.json"), &expected).unwrap();
    save::verify_snapshot(&root).unwrap();
    fs::write(snapshot.join("charge-density.dat"), "different size and content").unwrap();
    save::verify_snapshot(&root).unwrap();
    fs::write(snapshot.join("data-file-schema.xml"), "broken XML").unwrap();
    assert!(save::verify_snapshot(&root).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[cfg(unix)]
fn verification_does_not_open_wavefunction_or_density_fifos() {
    use std::{ffi::CString, os::unix::ffi::OsStrExt, sync::mpsc, time::Duration};
    let root = root();
    fs::write(root.join("data-file-schema.xml"), "protected XML").unwrap();
    for name in ["wfc1.dat", "charge-density.dat", "MgB2.wfc2", "wfcup1.hdf5"] {
        let path = CString::new(root.join(name).as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    }
    let (send, receive) = mpsc::channel();
    let worker_root = root.clone();
    let worker = std::thread::spawn(move || {
        let result = save::files(&worker_root).unwrap();
        for name in ["wfc1.dat", "charge-density.dat"] {
            let id = Identity { path: worker_root.join(name), target: worker_root.join(name), sha256: "old".into(), bytes: 123 };
            id.verify().unwrap();
            assert!(store::identity_available(&id).unwrap());
        }
        send.send(result.len()).unwrap();
    });
    assert_eq!(receive.recv_timeout(Duration::from_secs(3)).expect("Verify Error FIFO ,Waiting for big data content"), 1);
    worker.join().unwrap();
    fs::remove_dir_all(root).unwrap();
}


#[test]
fn copying_a_payload_only_directory_does_not_require_protected_records() {
    let root = root();
    let source = root.join("source");
    let target = root.join("target");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("wfc1.dat"), "raw wavefunction").unwrap();
    assert!(save::copy_tree(&source, &target).unwrap().is_empty());
    assert_eq!(fs::read(target.join("wfc1.dat")).unwrap(), b"raw wavefunction");
    fs::remove_dir_all(root).unwrap();
}
