//! Black-box CLI tests (P6.H; TST-03).
//!
//! Each test runs the real `encode_queue` binary against a fresh
//! `tempfile::TempDir` used as **both** `HOME` and `ENCODE_QUEUE_DIR`, with
//! `.env_clear()` plus explicit env so ambient variables cannot interfere
//! and the real config dir is never touched. `EDITOR=true` is set on every
//! command (the `add`/`ls` subcommands never launch an editor, so `true` is
//! inert and just pins the environment).

use assert_cmd::Command;
use predicates::str;
use std::fs;
use std::path::{Path, PathBuf};

/// Build a command with the environment pinned to `queue_dir`: `HOME` and
/// `ENCODE_QUEUE_DIR` both point at the temp dir, the surrounding
/// environment is cleared, and `EDITOR` is `true`.
fn pinned_command(queue_dir: &Path) -> Command {
    let mut cmd = Command::cargo_bin("encode_queue").expect("encode_queue binary available");
    cmd.env_clear()
        .env("HOME", queue_dir)
        .env("ENCODE_QUEUE_DIR", queue_dir)
        .env("EDITOR", "true");
    cmd
}

/// The queue file location for the pinned environment.
fn queue_file(queue_dir: &Path) -> PathBuf {
    queue_dir.join("commands.txt")
}

/// `add` on a fresh dir exits 0 and creates `commands.txt` containing
/// exactly `<cmd>\n`.
#[test]
fn add_creates_queue_file_in_fresh_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    pinned_command(dir.path())
        .args(["add", "nix build ."])
        .assert()
        .success()
        .stdout(str::is_empty());

    let contents = fs::read_to_string(queue_file(dir.path())).expect("queue file readable");
    assert_eq!(contents, "nix build .\n");
}

/// A second `add` appends: the file ends with both lines, order preserved.
#[test]
fn second_add_appends_preserving_order() {
    let dir = tempfile::tempdir().expect("tempdir");
    pinned_command(dir.path())
        .args(["add", "first"])
        .assert()
        .success();
    pinned_command(dir.path())
        .args(["add", "second"])
        .assert()
        .success();

    let contents = fs::read_to_string(queue_file(dir.path())).expect("queue file readable");
    assert_eq!(contents, "first\nsecond\n");
}

/// `ls` prints exactly the enqueued lines, one per line, followed by the
/// final blank line the frozen `cli.rs` `ls_output` always emits — asserted
/// verbatim (`predicates::str::diff` is the exact-equality string predicate;
/// the expectation is each line + `'\n'`, then one trailing `'\n'`).
#[test]
fn ls_prints_enqueued_lines_verbatim() {
    let dir = tempfile::tempdir().expect("tempdir");
    pinned_command(dir.path())
        .args(["add", "first"])
        .assert()
        .success();
    pinned_command(dir.path())
        .args(["add", "second"])
        .assert()
        .success();

    pinned_command(dir.path())
        .args(["ls"])
        .assert()
        .success()
        .stdout(str::diff("first\nsecond\n\n"));
}

/// `ls` with no queue file: exit 0, and stdout is exactly one `"\n"` —
/// missing queue ≡ empty queue: the single trailing newline from the frozen
/// `cli.rs` `ls_output`, legacy empty-file `ls` parity (supervisor decision
/// 2026-10-16; deviation from the brief's "stdout empty" wording).
#[test]
fn ls_on_missing_queue_file_exits_zero_with_single_trailing_newline() {
    let dir = tempfile::tempdir().expect("tempdir");
    pinned_command(dir.path())
        .args(["ls"])
        .assert()
        .success()
        .stdout(str::diff("\n"));
}
