//! Black-box CLI tests for failure handling (P6.F3; TST-03).
//!
//! Each test runs the real `encode_queue` binary against a fresh
//! `tempfile::TempDir` used as **both** `HOME` and `ENCODE_QUEUE_DIR`, with
//! `.env_clear()` plus a **controlled** `PATH` so ambient variables cannot
//! interfere and shell availability is deterministic (the harness pattern of
//! `tests/shell.rs`: the `dev` toolchain actually ships `fish` on `PATH`, so
//! "fish is absent" must be enforced via `PATH`, not assumed).
//!
//! Every `start` command sets `ENCODE_QUEUE_SHELL=sh` (P6.F3 spec), except
//! test 6, which clears it to exercise the spawn-failure path (spawn failure
//! ≠ command failure: no failure-log record, queue unchanged, exit 1).

use assert_cmd::Command;
use predicates::str;
use std::fs;
use std::path::{Path, PathBuf};

/// Find a real `sh` binary on the current (test-process) `PATH`.
fn real_sh() -> PathBuf {
    let path = std::env::var_os("PATH").expect("PATH set for the test process");
    std::env::split_paths(&path)
        .find_map(|dir| {
            let p = dir.join("sh");
            p.is_file().then_some(p)
        })
        .expect("sh present on the test process PATH")
}

/// A `PATH` value that exposes only `sh` (a tempdir with one `sh` symlink;
/// no `fish`/`bash`), so the `sh` tests are guaranteed to find `sh` and any
/// default-`fish` command is guaranteed to hit the spawn-failure path.
struct ShOnlyPath(tempfile::TempDir);

impl ShOnlyPath {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("path tempdir");
        std::os::unix::fs::symlink(real_sh(), dir.path().join("sh")).expect("symlink sh into PATH");
        Self(dir)
    }

    fn path(&self) -> &Path {
        self.0.path()
    }
}

/// Build a command pinned to `queue_dir`: `HOME` and `ENCODE_QUEUE_DIR` both
/// point at the temp dir, the surrounding environment is cleared, `EDITOR`
/// is `true` (inert for `start`), `PATH` is the controlled `sh`-only
/// directory, and `ENCODE_QUEUE_SHELL=sh` (P6.F3 spec: every test pins the
/// shell explicitly — test 6 unsets it via `env_remove`).
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

/// The failure-log location for the pinned environment.
fn failed_file(queue_dir: &Path) -> PathBuf {
    queue_dir.join("failed.txt")
}

/// `start` on `[false, true]` (no flag): exit 0 even though a command
/// failed; the queue file is empty (both entries drained); `failed.txt`
/// exists with exactly one line — three tab-separated parts, part1 a
/// parseable Unix timestamp (`u64`), part2 the exit code "1", part3 the
/// failed command "false".
#[test]
fn start_logs_non_zero_exit_and_continues_to_drain_the_queue() {
    let dir = tempfile::tempdir().expect("queue tempdir");
    let shpath = ShOnlyPath::new();

    for cmd in ["false", "true"] {
        pinned_command(dir.path(), shpath.path())
            .args(["add", cmd])
            .assert()
            .success();
    }

    pinned_command(dir.path(), shpath.path())
        .args(["start"])
        .assert()
        .success();

    // Both entries drained (the failed one removed, the next one run).
    let contents = fs::read_to_string(queue_file(dir.path())).expect("queue file readable");
    assert_eq!(contents, "");

    let failed = fs::read_to_string(failed_file(dir.path())).expect("failure log readable");
    let lines: Vec<&str> = failed.lines().collect();
    assert_eq!(lines.len(), 1, "exactly one failure line: {failed:?}");
    let parts: Vec<&str> = lines[0].split('\t').collect();
    assert_eq!(
        parts.len(),
        3,
        "exactly 3 tab-separated parts: {:?}",
        lines[0]
    );
    assert!(
        parts[0].parse::<u64>().is_ok(),
        "part1 is a Unix timestamp: {parts:?}"
    );
    assert_eq!(parts[1], "1");
    assert_eq!(parts[2], "false");
}

/// `start` on `[true, false]` (no flag): exit 0; the success runs silently;
/// `failed.txt` has exactly one line whose command part is "false"; the
/// queue is drained.
#[test]
fn start_logs_only_the_failed_entry() {
    let dir = tempfile::tempdir().expect("queue tempdir");
    let shpath = ShOnlyPath::new();

    for cmd in ["true", "false"] {
        pinned_command(dir.path(), shpath.path())
            .args(["add", cmd])
            .assert()
            .success();
    }

    pinned_command(dir.path(), shpath.path())
        .args(["start"])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(queue_file(dir.path())).expect("queue file readable"),
        ""
    );

    let failed = fs::read_to_string(failed_file(dir.path())).expect("failure log readable");
    let lines: Vec<&str> = failed.lines().collect();
    assert_eq!(lines.len(), 1, "exactly one failure line: {failed:?}");
    let parts: Vec<&str> = lines[0].split('\t').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[2], "false");
}

/// `start --stop-on-error` on `[false, true]`: exit **1**; stderr contains
/// `stopped after a failed command`; `failed.txt` has exactly one line
/// (command "false"); the queue file now contains exactly `true\n` — the
/// failed entry removed, the next one intact.
#[test]
fn stop_on_error_exits_one_and_keeps_the_remaining_queue() {
    let dir = tempfile::tempdir().expect("queue tempdir");
    let shpath = ShOnlyPath::new();

    for cmd in ["false", "true"] {
        pinned_command(dir.path(), shpath.path())
            .args(["add", cmd])
            .assert()
            .success();
    }

    pinned_command(dir.path(), shpath.path())
        .args(["start", "--stop-on-error"])
        .assert()
        .code(1)
        .stderr(str::contains("stopped after a failed command"));

    let failed = fs::read_to_string(failed_file(dir.path())).expect("failure log readable");
    let lines: Vec<&str> = failed.lines().collect();
    assert_eq!(lines.len(), 1, "exactly one failure line: {failed:?}");
    let parts: Vec<&str> = lines[0].split('\t').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[2], "false");

    // The failed entry was removed; the next one is still queued.
    let contents = fs::read_to_string(queue_file(dir.path())).expect("queue file readable");
    assert_eq!(contents, "true\n");
}

/// A second `start` (no flag) after a `--stop-on-error` run: exit 0, the
/// queue drains empty, and `failed.txt` is STILL exactly one line (`true`
/// succeeded; nothing new is logged).
#[test]
fn second_start_after_stop_on_error_drains_without_new_failures() {
    let dir = tempfile::tempdir().expect("queue tempdir");
    let shpath = ShOnlyPath::new();

    for cmd in ["false", "true"] {
        pinned_command(dir.path(), shpath.path())
            .args(["add", cmd])
            .assert()
            .success();
    }

    pinned_command(dir.path(), shpath.path())
        .args(["start", "--stop-on-error"])
        .assert()
        .code(1);

    pinned_command(dir.path(), shpath.path())
        .args(["start"])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(queue_file(dir.path())).expect("queue file readable"),
        ""
    );

    let failed = fs::read_to_string(failed_file(dir.path())).expect("failure log readable");
    let lines: Vec<&str> = failed.lines().collect();
    assert_eq!(lines.len(), 1, "still exactly one failure line: {failed:?}");
    let parts: Vec<&str> = lines[0].split('\t').collect();
    assert_eq!(parts[2], "false");
}

/// `start` on `[true]` (no flag): exit 0 and `failed.txt` is NOT created
/// (no failure means no failure log).
#[test]
fn start_with_only_successes_never_creates_the_failure_log() {
    let dir = tempfile::tempdir().expect("queue tempdir");
    let shpath = ShOnlyPath::new();

    pinned_command(dir.path(), shpath.path())
        .args(["add", "true"])
        .assert()
        .success();

    pinned_command(dir.path(), shpath.path())
        .args(["start"])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(queue_file(dir.path())).expect("queue file readable"),
        ""
    );
    assert!(!failed_file(dir.path()).exists());
}

/// Spawn failure ≠ command failure (P6.F3): `ENCODE_QUEUE_SHELL` cleared
/// (default `fish`, absent from the controlled `PATH`) with queue
/// `[echo hi]`: `start` exits 1, `failed.txt` is NOT created, and the queue
/// file is unchanged (the error propagates before the rewrite).
#[test]
fn spawn_failure_does_not_create_the_failure_log_and_keeps_the_queue() {
    let dir = tempfile::tempdir().expect("queue tempdir");
    let shpath = ShOnlyPath::new();

    pinned_command(dir.path(), shpath.path())
        .args(["add", "echo hi"])
        .assert()
        .success();

    pinned_command(dir.path(), shpath.path())
        .env_remove("ENCODE_QUEUE_SHELL")
        .args(["start"])
        .assert()
        .code(1)
        .stderr(str::contains("error: failed to spawn the shell"));

    assert!(!failed_file(dir.path()).exists());
    let contents = fs::read_to_string(queue_file(dir.path())).expect("queue file readable");
    assert_eq!(contents, "echo hi\n");
}
