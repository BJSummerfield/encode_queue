//! Black-box CLI tests for the selectable shell (P6.F2; TST-03).
//!
//! Each test runs the real `encode_queue` binary against a fresh
//! `tempfile::TempDir` used as **both** `HOME` and `ENCODE_QUEUE_DIR`, with
//! `.env_clear()` plus a **controlled** `PATH` so ambient variables cannot
//! interfere and shell availability is deterministic.
//!
//! The `dev` toolchain used for the gates actually ships `fish` on `PATH`
//! (the run brief assumed `fish` was absent), so relying on "fish is missing"
//! would be non-deterministic. Instead each command runs against a `PATH`
//! tempdir that exposes only `sh` (a single symlink): the `sh` tests are
//! guaranteed to find `sh`, and the default-`fish` test is guaranteed to hit
//! the spawn-failure path.

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

/// A `PATH` value that exposes only `sh` (a tempdir with one `sh` symlink; no
/// `fish`/`bash`).
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
/// point at the temp dir, the surrounding environment is cleared, `EDITOR` is
/// `true` (inert for `add`/`ls`/`start`), and `PATH` is the controlled
/// `sh`-only directory.
fn pinned_command(queue_dir: &Path, path_dir: &Path) -> Command {
    let mut cmd = Command::cargo_bin("encode_queue").expect("encode_queue binary available");
    cmd.env_clear()
        .env("HOME", queue_dir)
        .env("ENCODE_QUEUE_DIR", queue_dir)
        .env("EDITOR", "true")
        .env("PATH", path_dir);
    cmd
}

/// The queue file location for the pinned environment.
fn queue_file(queue_dir: &Path) -> PathBuf {
    queue_dir.join("commands.txt")
}

/// `ENCODE_QUEUE_SHELL=sh` + `add` + `start` with a no-op command: `start`
/// exits 0 and the queue is drained.
#[test]
fn start_with_sh_shell_runs_and_drains_queue() {
    let dir = tempfile::tempdir().expect("queue tempdir");
    let shpath = ShOnlyPath::new();

    pinned_command(dir.path(), shpath.path())
        .env("ENCODE_QUEUE_SHELL", "sh")
        .args(["add", "echo ok"])
        .assert()
        .success();

    pinned_command(dir.path(), shpath.path())
        .env("ENCODE_QUEUE_SHELL", "sh")
        .args(["start"])
        .assert()
        .success();

    let contents = fs::read_to_string(queue_file(dir.path())).expect("queue file readable");
    assert_eq!(contents, "");
}

/// `ENCODE_QUEUE_SHELL=zsh` exits 2 with the exact usage-class message listing
/// the valid values.
#[test]
fn invalid_shell_value_exits_two_with_valid_values_message() {
    let dir = tempfile::tempdir().expect("queue tempdir");
    let shpath = ShOnlyPath::new();

    pinned_command(dir.path(), shpath.path())
        .env("ENCODE_QUEUE_SHELL", "zsh")
        .args(["ls"])
        .assert()
        .code(2)
        .stderr(str::diff(
            "error: ENCODE_QUEUE_SHELL 'zsh' is not a valid shell: expected fish, bash, or sh\n",
        ));
}

/// Shell unset (default `fish`, absent from the controlled `PATH`): `add` +
/// `start` exits 1, the `error:` line mentions the spawn failure, and the
/// queue file is unchanged (the error propagates before the rewrite).
#[test]
fn default_fish_absent_start_fails_and_keeps_queue() {
    let dir = tempfile::tempdir().expect("queue tempdir");
    let shpath = ShOnlyPath::new();

    pinned_command(dir.path(), shpath.path())
        .args(["add", "echo ok"])
        .assert()
        .success();

    pinned_command(dir.path(), shpath.path())
        .args(["start"])
        .assert()
        .code(1)
        .stderr(str::contains("error: failed to spawn the shell"));

    let contents = fs::read_to_string(queue_file(dir.path())).expect("queue file readable");
    assert_eq!(contents, "echo ok\n");
}

/// `ls` output is unaffected by a valid `ENCODE_QUEUE_SHELL=sh` (exit 0, same
/// stdout as without the variable).
#[test]
fn ls_is_unaffected_by_valid_shell() {
    let dir = tempfile::tempdir().expect("queue tempdir");
    let shpath = ShOnlyPath::new();

    pinned_command(dir.path(), shpath.path())
        .args(["add", "first"])
        .assert()
        .success();

    let baseline = pinned_command(dir.path(), shpath.path())
        .args(["ls"])
        .output()
        .expect("ls runs");
    let with_shell = pinned_command(dir.path(), shpath.path())
        .env("ENCODE_QUEUE_SHELL", "sh")
        .args(["ls"])
        .output()
        .expect("ls runs");

    assert_eq!(baseline.status.code(), Some(0));
    assert_eq!(with_shell.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&baseline.stdout),
        String::from_utf8_lossy(&with_shell.stdout)
    );
    assert_eq!(
        std::str::from_utf8(&with_shell.stdout).expect("utf8"),
        "first\n\n"
    );
}
