# Encode Queue

**Version:** 1.0  
**Author:** [Brian Summerfield](mailto:brianjsummerfield@gmail.com)

## Overview

Encode Queue is a command-line tool that manages a queue of shell commands,
particularly useful for sequentially encoding files. Commands are stored as
lines in a plain-text queue file. `start` executes the queued commands one at
a time with the configured shell, removing each entry from the queue after it
has run; the queue file is re-read before every entry, so the queue can be
added to or edited while it is being processed.

## Requirements

- **Rust** is required to build the project.
- Queued commands are executed through one of the shells **`fish`, `bash`,
  or `sh`**, selected with the `ENCODE_QUEUE_SHELL` environment variable
  (default: `fish`). The chosen shell only needs to be installed for `start`;
  the other subcommands never spawn a shell.
- `edit` opens the queue file in the editor named by `EDITOR`
  (default: `nvim`).

## Installation

1. **Clone the repository:**

   ```sh
   git clone https://github.com/BJSummerfield/encode_queue.git
   cd encode_queue
   ```

2. **Build the project:**

   ```sh
   cargo build --release
   ```

3. **Run the executable:**

   ```sh
   ./target/release/encode_queue <COMMAND>
   ```

The subcommands are `add`, `start`, `ls`, `rm`, and `edit`.

## Usage

### Add a Command to the Queue

```sh
encode_queue add "your_command_here"
```

Appends the command, verbatim, as a new line to the queue file and exits 0
with no output. The argument is not trimmed and blank/whitespace-only
commands are accepted as-is.

**Example:**

```sh
encode_queue add "ffmpeg -i input.mp4 -c:v libx264 output.mp4"
```

### Start Processing the Queue

```sh
encode_queue start
```

Drains the queue: reads the entries one at a time, runs the first entry with
`<shell> -c <entry>` (the shell selected by `ENCODE_QUEUE_SHELL`, default
`fish`) with inherited standard I/O — the entry is trimmed before it runs and
a blank first line is skipped (not run) but still removed — then removes the
entry and repeats until the queue is empty. A `start` with no failures exits
0.

If a queued command exits non-zero, the failure is recorded in the
[failure log](#failure-log) and the failed entry is removed; `start` then
continues with the next entry and still exits 0. If the shell itself cannot
be spawned, nothing is logged, the entry stays queued, and `start` exits 1.
If the queue file does not exist, `start` exits 1 with
`error: queue file not found at <path>`.

`start` is also guarded by a [single-runner lock](#single-runner-lock-queuelock):
a second `start` while another live process is draining the queue exits 3.

**Stop after the first failure:**

```sh
encode_queue start --stop-on-error
```

With `--stop-on-error`, a failed command stops the drain after it has been
logged and removed: `start` exits 1 and prints
`error: stopped after a failed command (--stop-on-error)` on standard error.
The remaining entries stay queued, so a later `start` picks up where it
stopped.

### List All Commands in the Queue

```sh
encode_queue ls
```

Prints every queue entry verbatim, one per line, followed by a final blank
line. A missing queue file is treated as an empty queue: `ls` exits 0 and
prints only the final blank line.

**Numbered listing:**

```sh
encode_queue ls -n        # or: encode_queue ls --numbered
```

Prefixes each line with its 1-based position:

```
1: first
2: second
3: third

```

(An empty queue still prints only the final blank line.)

### Remove an Entry from the Queue

```sh
encode_queue rm <N>
```

Removes the N-th (1-based) command from the queue, keeping the remaining
entries in order, and exits 0 with no output.

- `rm 0` or a non-numeric `N` is rejected with exit 2
  (`invalid command: invalid entry index '<N>': expected a positive integer`),
  leaving the queue file unchanged.
- An `N` that is valid but beyond the end of the queue is also rejected with
  exit 2 (`invalid command: no queue entry at position <N>`), leaving the
  queue file unchanged.
- A missing `N` is a usage error (exit 2).
- `rm` on a missing queue file exits 1 with
  `error: queue file not found at <path>`.

### Edit the Queue

```sh
encode_queue edit
```

Opens the queue file in the editor given by `EDITOR` (default `nvim`). The
editor's own exit status is discarded; `edit` only fails (exit 1) if the
editor cannot be spawned or the wait on it fails.

## Configuration

The queue is stored in the text file **`commands.txt`** inside the queue
directory. The queue directory is created if it is missing. By default the
queue file is:

```
~/.config/encode_queue/commands.txt
```

Four environment variables control the tool:

| Variable | Effect |
|---|---|
| `HOME` | Required. The default queue directory is `$HOME/.config/encode_queue`. A missing or non-Unicode `HOME` is a config error (exit 1), even when `ENCODE_QUEUE_DIR` is set. |
| `ENCODE_QUEUE_DIR` | Overrides the queue **directory**: the queue file is then `<ENCODE_QUEUE_DIR>/commands.txt`. Unset, empty, or whitespace-only values are treated as unset (the default path is used). A non-Unicode value is a config error (exit 1). |
| `ENCODE_QUEUE_SHELL` | Selects the shell `start` spawns: `fish`, `bash`, or `sh` (case-sensitive, exact match). Default `fish`; unset, empty, or whitespace-only values also default to `fish`. Any other value is a usage error (exit 2): `error: ENCODE_QUEUE_SHELL '<value>' is not a valid shell: expected fish, bash, or sh`. |
| `EDITOR` | The editor `edit` opens the queue file with. Unset or empty defaults to `nvim`. |

## Failure Log

When a queued command runs and exits non-zero, `start` appends one line to
the failure log **`failed.txt`** in the queue directory:

```
<unix_timestamp_seconds>\t<exit_code>\t<command>\n
```

where `<command>` is the entry as it was run (trimmed). The file is created
only when the first failure is logged and is appended to on every failure.
Logging the failure does not stop the drain (see `--stop-on-error` above),
and a command that never ran (a shell spawn failure) is **not** logged.

## Single-Runner Lock (queue.lock)

`start` is guarded by a lock file **`queue.lock`** in the queue directory:
the draining process creates it and records its own PID, and removes it when
it exits.

- A second `start` while another **live** process holds the lock is refused
  with exit 3 and the message `error: queue already running (pid <pid>)`,
  where `<pid>` is the PID recorded in the lock file. No queue entry is run
  or removed in that case.
- A lock whose recorded PID is no longer alive is treated as stale and
  reclaimed, so a crashed `start` does not block the queue.
- The lock guards `start` only: `add`, `ls`, `rm`, and `edit` work normally
  while it is held.

## Exit Codes

| Exit code | Meaning |
|---|---|
| `0` | Success: the subcommand completed (including `start` draining the queue, even if some commands failed, without `--stop-on-error`). |
| `1` | Configuration or service errors: missing/non-Unicode `HOME`, non-Unicode `ENCODE_QUEUE_DIR`, a queue-file I/O error, a missing queue file for `start` or `rm`, a shell or editor that cannot be spawned or waited on, and `start --stop-on-error` stopping after a failed command. |
| `2` | Usage errors: an unknown or missing subcommand or argument, an invalid `rm` index (`0`, non-numeric, or out of range), and an invalid `ENCODE_QUEUE_SHELL` value. |
| `3` | `start` is already running: the `queue.lock` is held by another live process. |

## License

This project is licensed under the MIT License.
