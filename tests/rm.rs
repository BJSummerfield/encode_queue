//! Black-box tests for `rm <N>` and `ls -n`/`ls --numbered` (P6.F1; TST-03).
//!
//! Each test runs the real `encode_queue` binary against a fresh
//! `tempfile::TempDir` used as **both** `HOME` and `ENCODE_QUEUE_DIR`, with
//! `.env_clear()` plus explicit env so ambient variables cannot interfere and
//! the real config dir is never touched. `EDITOR=true` is set on every command
//! (the `add`/`ls`/`rm` subcommands never launch an editor, so `true` is inert
//! and just pins the environment). Harness copied from `tests/cli.rs`.

use assert_cmd::Command;
use predicates::str;
use std::fs;
use std::path::{Path, PathBuf};

/// Build a command with the environment pinned to `queue_dir`: `HOME` and
/// `ENCODE_QUEUE_DIR` both point at the temp dir, the surrounding environment
/// is cleared, and `EDITOR` is `true`.
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

/// Seed a fresh dir with the given entries (one `add` per entry), asserting
/// each exits 0.
fn seed(queue_dir: &Path, entries: &[&str]) {
    for e in entries {
        pinned_command(queue_dir)
            .arg("add")
            .arg(e)
            .assert()
            .success();
    }
}

/// `rm` on the middle of a 3-entry queue exits 0 and leaves the other two in
/// their original order.
#[test]
fn rm_middle_of_three_leaves_the_other_two_in_order() {
    let dir = tempfile::tempdir().expect("tempdir");
    seed(dir.path(), &["first", "second", "third"]);

    pinned_command(dir.path())
        .args(["rm", "2"])
        .assert()
        .success()
        .stdout(str::is_empty());

    let contents = fs::read_to_string(queue_file(dir.path())).expect("queue file readable");
    assert_eq!(contents, "first\nthird\n");
}

/// `rm 1` removes the first entry; `rm <last>` removes the last.
#[test]
fn rm_first_and_last() {
    let dir = tempfile::tempdir().expect("tempdir");
    seed(dir.path(), &["first", "second", "third"]);
    pinned_command(dir.path())
        .args(["rm", "1"])
        .assert()
        .success();
    let contents = fs::read_to_string(queue_file(dir.path())).expect("queue file readable");
    assert_eq!(contents, "second\nthird\n");

    let dir = tempfile::tempdir().expect("tempdir");
    seed(dir.path(), &["first", "second", "third"]);
    pinned_command(dir.path())
        .args(["rm", "3"])
        .assert()
        .success();
    let contents = fs::read_to_string(queue_file(dir.path())).expect("queue file readable");
    assert_eq!(contents, "first\nsecond\n");
}

/// `rm 0` is rejected at parse time: exit 2, non-empty stderr, and the queue
/// file is byte-for-byte unchanged.
#[test]
fn rm_zero_is_rejected_and_leaves_the_file_unchanged() {
    let dir = tempfile::tempdir().expect("tempdir");
    seed(dir.path(), &["first", "second", "third"]);
    let before = fs::read(queue_file(dir.path())).expect("queue file readable");

    pinned_command(dir.path())
        .args(["rm", "0"])
        .assert()
        .code(2)
        .stderr(str::contains("invalid entry index"));

    assert_eq!(
        fs::read(queue_file(dir.path())).expect("queue file readable"),
        before
    );
}

/// `rm abc` is rejected at parse time: exit 2 and the queue file unchanged.
#[test]
fn rm_non_numeric_is_rejected_and_leaves_the_file_unchanged() {
    let dir = tempfile::tempdir().expect("tempdir");
    seed(dir.path(), &["first", "second", "third"]);
    let before = fs::read(queue_file(dir.path())).expect("queue file readable");

    pinned_command(dir.path())
        .args(["rm", "abc"])
        .assert()
        .code(2);

    assert_eq!(
        fs::read(queue_file(dir.path())).expect("queue file readable"),
        before
    );
}

/// `rm 4` on a 3-entry queue is out of range: exit 2, stderr names the
/// missing position, and the queue file is unchanged.
///
/// (The supervisor's D2' — adjudicated after this brief — routes the
/// out-of-range case through `QueueError::NoEntry`, whose message is
/// "no queue entry at position N", superseding the brief's original
/// "out of range" wording for this assertion.)
#[test]
fn rm_out_of_range_is_rejected_and_leaves_the_file_unchanged() {
    let dir = tempfile::tempdir().expect("tempdir");
    seed(dir.path(), &["first", "second", "third"]);
    let before = fs::read(queue_file(dir.path())).expect("queue file readable");

    pinned_command(dir.path())
        .args(["rm", "4"])
        .assert()
        .code(2)
        .stderr(str::contains("no queue entry at position"));

    assert_eq!(
        fs::read(queue_file(dir.path())).expect("queue file readable"),
        before
    );
}

/// `rm` with no argument exits 2 (clap's usage error on the required `N`).
#[test]
fn rm_with_no_argument_exits_two() {
    let dir = tempfile::tempdir().expect("tempdir");
    seed(dir.path(), &["first", "second"]);

    pinned_command(dir.path()).arg("rm").assert().code(2);
}

/// `ls -n` prints `N: <cmd>` per line (1-based), with the same final blank
/// line plain `ls` emits — asserted verbatim. `--numbered` is equivalent.
#[test]
fn ls_numbered_prints_position_prefix() {
    let dir = tempfile::tempdir().expect("tempdir");
    seed(dir.path(), &["a", "b", "c"]);

    pinned_command(dir.path())
        .args(["ls", "-n"])
        .assert()
        .success()
        .stdout(str::diff("1: a\n2: b\n3: c\n\n"));

    pinned_command(dir.path())
        .args(["ls", "--numbered"])
        .assert()
        .success()
        .stdout(str::diff("1: a\n2: b\n3: c\n\n"));
}

/// Plain `ls` output is identical to today's behaviour for the same file:
/// every line verbatim + the final blank line.
#[test]
fn plain_ls_output_is_unchanged() {
    let dir = tempfile::tempdir().expect("tempdir");
    seed(dir.path(), &["a", "b", "c"]);

    pinned_command(dir.path())
        .arg("ls")
        .assert()
        .success()
        .stdout(str::diff("a\nb\nc\n\n"));
}

/// `rm` on a missing queue file (no `add` was run) exits 1: the missing file
/// is a store failure, not an empty queue.
#[test]
fn rm_on_missing_queue_file_exits_one() {
    let dir = tempfile::tempdir().expect("tempdir");

    pinned_command(dir.path())
        .args(["rm", "1"])
        .assert()
        .code(1);
}
