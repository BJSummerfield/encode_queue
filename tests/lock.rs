//! Black-box lock tests (P6.F4; TST-03).
//!
//! Each test runs the real `encode_queue` binary against a fresh
//! `tempfile::TempDir` used as **both** `HOME` and `ENCODE_QUEUE_DIR`, with
//! `.env_clear()` plus a **controlled** `PATH` so ambient variables cannot
//! interfere and the shell's view is deterministic (the harness pattern of
//! `tests/failure.rs`: with a cleared environment, `sh` and its children
//! must find their binaries through the pinned `PATH` alone).
//!
//! Every command pins `ENCODE_QUEUE_SHELL=sh` (P6.F4 spec).

use assert_cmd::Command;
use predicates::str;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Find a real `name` binary on the current (test-process) `PATH`.
fn real_bin(name: &str) -> PathBuf {
    let path = std::env::var_os("PATH").expect("PATH set for the test process");
    std::env::split_paths(&path)
        .find_map(|dir| {
            let p = dir.join(name);
            p.is_file().then_some(p)
        })
        .unwrap_or_else(|| panic!("{name} present on the test process PATH"))
}

/// A `PATH` value that exposes only `sh`, `sleep`, and `true` (a tempdir
/// with one symlink per real binary; no `fish`/`bash`), so the pinned
/// commands find exactly what the queue entries need and nothing else.
struct LockTestPath(tempfile::TempDir);

impl LockTestPath {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("path tempdir");
        for name in ["sh", "sleep", "true"] {
            std::os::unix::fs::symlink(real_bin(name), dir.path().join(name))
                .expect("symlink into PATH");
        }
        Self(dir)
    }

    fn path(&self) -> &Path {
        self.0.path()
    }
}

/// Build a command pinned to `queue_dir`/`path_dir`: `HOME` and
/// `ENCODE_QUEUE_DIR` both point at the temp dir, the surrounding
/// environment is cleared, `EDITOR` is `true`, `PATH` is the controlled
/// directory, and `ENCODE_QUEUE_SHELL` is `sh`.
fn pinned_command(queue_dir: &Path, path_dir: &Path) -> Command {
    let mut cmd = Command::cargo_bin("encode_queue").expect("encode_queue binary available");
    cmd.env_clear()
        .env("HOME", queue_dir)
        .env("ENCODE_QUEUE_DIR", queue_dir)
        .env("EDITOR", "true")
        .env("PATH", path_dir)
        .env("ENCODE_QUEUE_SHELL", "sh");
    cmd
}

/// The queue file location for the pinned environment.
fn queue_file(queue_dir: &Path) -> PathBuf {
    queue_dir.join("commands.txt")
}

/// The lock file location: the queue directory (the parent of the queue
/// file), per the outbound lock adapter's convention.
fn lock_file(queue_dir: &Path) -> PathBuf {
    queue_dir.join("queue.lock")
}

/// Probe downward from a PID far above any realistic `pid_max` for one with
/// no `/proc/<pid>` entry (test-only `panic!` if every probed PID were
/// alive, which cannot happen; TST-03).
fn find_dead_pid() -> u32 {
    let mut pid: u32 = 4_000_000_000;
    while Path::new("/proc").join(pid.to_string()).exists() {
        pid -= 1;
    }
    pid
}

/// Poll until `queue.lock` exists AND holds a readable PID (the writer
/// creates the file before writing the PID, so an empty file is not yet
/// "held"); ≤5 s at 100 ms intervals, failing with a clear message if it
/// never becomes readable.
fn wait_for_held_lock(path: &Path) -> u32 {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Ok(contents) = fs::read_to_string(path) {
            if let Ok(pid) = contents.trim().parse::<u32>() {
                return pid;
            }
        }
        assert!(
            Instant::now() < deadline,
            "queue.lock never appeared (or stayed empty) within 5 s"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// A second `start` while the lock is held exits 3 with the exact stderr
/// line naming the holder PID read from the lock file.
#[test]
fn second_start_is_refused_with_exit_3() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = LockTestPath::new();
    fs::write(queue_file(dir.path()), "sleep 10\n").expect("queue file written");

    // Start #1 in the background: it holds the lock while `sleep 10` runs.
    // (`assert_cmd::Command::spawn` is private in the pinned version, so the
    // background child goes through `std::process::Command` on the same
    // `cargo_bin` path with the same pinned env.)
    let mut child = std::process::Command::new(std::env!("CARGO_BIN_EXE_encode_queue"))
        .env_clear()
        .env("HOME", dir.path())
        .env("ENCODE_QUEUE_DIR", dir.path())
        .env("EDITOR", "true")
        .env("PATH", path.path())
        .env("ENCODE_QUEUE_SHELL", "sh")
        .arg("start")
        .spawn()
        .expect("start #1 spawned");

    // Wait until the lock file is held (exists AND carries a PID).
    let holder = wait_for_held_lock(&lock_file(dir.path()));

    // Start #2 in the foreground: refused with exit 3 and the exact line.
    pinned_command(dir.path(), path.path())
        .arg("start")
        .assert()
        .code(3)
        .stderr(str::diff(format!(
            "error: queue already running (pid {holder})\n"
        )));

    // Kill and reap start #1.
    child.kill().expect("start #1 killed");
    child.wait().expect("start #1 reaped");
}

/// A stale lock (dead holder PID) is reclaimed: `start` exits 0, the queue
/// is drained, and the lock file is removed afterwards.
#[test]
fn a_stale_lock_is_reclaimed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = LockTestPath::new();
    fs::write(queue_file(dir.path()), "true\n").expect("queue file written");
    fs::write(lock_file(dir.path()), find_dead_pid().to_string()).expect("stale lock written");

    pinned_command(dir.path(), path.path())
        .arg("start")
        .assert()
        .success();

    // The queue was drained (the entry ran and was removed).
    let contents = fs::read_to_string(queue_file(dir.path())).expect("queue file readable");
    assert_eq!(contents, "");
    // The reclaimed lock was removed on the normal exit.
    assert!(!lock_file(dir.path()).exists());
}

/// A normal `start` exit removes the lock file.
#[test]
fn normal_exit_removes_the_lock() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = LockTestPath::new();
    fs::write(queue_file(dir.path()), "true\n").expect("queue file written");

    pinned_command(dir.path(), path.path())
        .arg("start")
        .assert()
        .success();

    assert!(!lock_file(dir.path()).exists());
}

/// The lock guards only `start`: while `queue.lock` holds a LIVE PID (this
/// test process), `add` and `ls` both exit 0 with their normal behaviour.
#[test]
fn the_lock_guards_only_start() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = LockTestPath::new();
    fs::write(queue_file(dir.path()), "one\n").expect("queue file written");
    // This test process is alive for the duration of the test.
    fs::write(lock_file(dir.path()), std::process::id().to_string())
        .expect("live-PID lock written");

    pinned_command(dir.path(), path.path())
        .args(["add", "two"])
        .assert()
        .success()
        .stdout(str::is_empty());

    pinned_command(dir.path(), path.path())
        .args(["ls"])
        .assert()
        .success()
        .stdout(str::diff("one\ntwo\n\n"));
}
