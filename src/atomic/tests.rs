//! Unit tests for the atomic write primitive: temp-file cleanup on every failure path, the

//! bounded replace-rename retry, and the platform promise that retry encodes.

static TMP_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn tmp_id(prefix: &str) -> String {
    let n = TMP_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    format!("{}-{}-{}", prefix, std::process::id(), n)
}

use super::*;
use std::fs;

fn tmp_dir(prefix: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(tmp_id(prefix));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
#[cfg_attr(not(unix), ignore)]
fn creates_new_file_and_cleans_temp() {
    let dir = tmp_dir("cutver-atomic-new");
    let path = dir.join("out.txt");
    write_atomic(&path, "hello").unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "hello");
    let entries: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(entries, vec!["out.txt"]);
}

#[test]
#[cfg_attr(not(unix), ignore)]
fn replaces_existing_file_atomically() {
    let dir = tmp_dir("cutver-atomic-replace");
    let path = dir.join("out.txt");
    fs::write(&path, "old").unwrap();
    write_atomic(&path, "new").unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "new");
    let entries: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(entries, vec!["out.txt"]);
}

#[test]
#[cfg_attr(not(unix), ignore)]
fn writes_empty_content() {
    let dir = tmp_dir("cutver-atomic-empty");
    let path = dir.join("empty.txt");
    write_atomic(&path, "").unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "");
    let entries: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(entries, vec!["empty.txt"]);
}

#[test]
fn temp_files_have_unique_names() {
    let path = Path::new("test.txt");
    let t1 = temp_file_path(path).unwrap();
    let t2 = temp_file_path(path).unwrap();
    assert_ne!(t1, t2);

    let t1_name = t1.file_name().unwrap().to_string_lossy();
    let pid = std::process::id();
    assert!(
        t1_name.starts_with(&format!(".test.txt.cutver-tmp-{pid}-")),
        "unexpected temp file name: {t1_name}"
    );
}

#[test]
fn retries_a_transient_rename_collision_up_to_the_planned_attempts() {
    let plan = RenamePlan {
        attempts: 3,
        delay: Duration::ZERO,
        is_transient: |_err: &io::Error| true,
    };
    let mut calls = 0;

    plan.apply(Path::new("tmp"), Path::new("dst"), |_, _| {
        calls += 1;
        if calls < 3 {
            Err(io::Error::from(io::ErrorKind::PermissionDenied))
        } else {
            Ok(())
        }
    })
    .unwrap();

    assert_eq!(calls, 3, "expected the third attempt to carry the rename");
}

#[test]
fn does_not_retry_a_permanent_rename_error() {
    let plan = RenamePlan {
        attempts: 3,
        delay: Duration::ZERO,
        is_transient: |_err: &io::Error| false,
    };
    let mut calls = 0;

    let err = plan
        .apply(Path::new("tmp"), Path::new("dst"), |_, _| {
            calls += 1;
            Err(io::Error::from(io::ErrorKind::PermissionDenied))
        })
        .unwrap_err();

    assert_eq!(calls, 1, "a permanent error must fail on the first attempt");
    assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
}

#[test]
fn exhausted_retries_return_the_last_error() {
    let plan = RenamePlan {
        attempts: 3,
        delay: Duration::ZERO,
        is_transient: |_err: &io::Error| true,
    };
    let mut calls = 0;

    let err = plan
        .apply(Path::new("tmp"), Path::new("dst"), |_, _| {
            calls += 1;
            Err(io::Error::other(format!("attempt-{calls}")))
        })
        .unwrap_err();

    assert_eq!(calls, 3, "the retry must stop at the bound");
    assert_eq!(err.to_string(), "attempt-3", "the last error is the one that surfaces");
}

#[test]
fn concurrent_writes_to_same_file_do_not_collide() {
    let dir = tmp_dir("cutver-atomic-concurrent");
    let path = dir.join("out.txt");

    let handles: Vec<_> = (0..20)
        .map(|i| {
            let path = path.clone();
            std::thread::spawn(move || {
                write_atomic(&path, format!("content-{}", i)).unwrap();
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let final_content = fs::read_to_string(&path).unwrap();
    assert!(
        final_content.starts_with("content-"),
        "unexpected content: {final_content}"
    );

    let entries: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(entries, vec!["out.txt"]);
}

#[test]
#[cfg_attr(not(unix), ignore)]
fn rename_to_directory_fails_cleanly_and_removes_temp() {
    let dir = tmp_dir("cutver-atomic-dir");
    let target = dir.join("blocked");
    fs::create_dir(&target).unwrap();

    let err = write_atomic(&target, "x").unwrap_err();
    match err {
        Error::Rename { .. } => {}
        other => panic!("expected Rename error, got: {other:?}"),
    }

    let entries: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(entries, vec!["blocked"]);
}

#[test]
#[cfg(windows)]
fn destination_held_without_share_delete_fails_closed_after_the_bounded_retries() {
    use std::os::windows::fs::OpenOptionsExt;

    let dir = tmp_dir("cutver-atomic-held-destination");
    let path = dir.join("out.txt");
    fs::write(&path, "old").unwrap();

    // Denying delete sharing is what makes `MoveFileEx` answer with a transient collision for the
    // replace below. Read and write sharing stay granted so that a scanner briefly holding the file
    // cannot turn this fixture into the flake it exists to explain.
    const FILE_SHARE_READ: u32 = 1;
    const FILE_SHARE_WRITE: u32 = 2;
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .open(&path)
        .unwrap();

    match write_atomic(&path, "new").unwrap_err() {
        Error::Rename { source, .. } => {
            assert!(
                matches!(source.raw_os_error(), Some(5) | Some(32)),
                "expected ERROR_ACCESS_DENIED or ERROR_SHARING_VIOLATION, got: {source}"
            );
            assert!(
                is_transient_rename_error(&source),
                "the collision the platform reports must be one the retry outlasts: {source}"
            );
        }
        other => panic!("expected Rename error, got: {other:?}"),
    }

    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "old",
        "the destination must keep its previous contents"
    );
    let entries: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(
        entries,
        vec!["out.txt"],
        "the temp file must be removed once the retries are exhausted"
    );

    drop(held);
    write_atomic(&path, "new").unwrap();
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "new",
        "the retry policy must leave the ordinary replace working"
    );
}

#[test]
fn write_error_cleans_up_temp_file() {
    let dir = tmp_dir("cutver-atomic-write-err");
    let path = dir.join("out.txt");

    let err = write_atomic_impl(&path, b"content", |_file, _data| {
        Err(io::Error::other("simulated write failure"))
    })
    .unwrap_err();

    match err {
        Error::TempWrite { .. } => {}
        other => panic!("expected TempWrite error, got: {other:?}"),
    }

    let entries: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert!(
        entries.is_empty(),
        "expected empty directory after failed write, found: {entries:?}"
    );
}

#[test]
fn invalid_path_fails() {
    let err = write_atomic(Path::new("/"), "content").unwrap_err();
    match err {
        Error::InvalidPath { .. } => {}
        other => panic!("expected InvalidPath error, got: {other:?}"),
    }
}

#[test]
fn nonexistent_parent_dir_fails_cleanly() {
    let dir = tmp_dir("cutver-atomic-nonexistent");
    let target = dir.join("no_such_subdir").join("out.txt");
    let err = write_atomic(&target, "data").unwrap_err();
    match err {
        Error::TempWrite { .. } => {}
        other => panic!("expected TempWrite error, got: {other:?}"),
    }
}
