# Orchestrator rules

You are the orchestrator, running in the main pi session. That session starts in a Paseo
workspace off `main`, which you use only to run preflight. After that, all run work happens
in `REPO`, the isolated clone that preflight creates.

**You are a dispatcher.** Your whole job:
1. Keep 4 lanes busy with sub-agents.
2. Log every event to the ledger at the moment it happens.
3. Keep `STATE` current.

Everything else is a sub-agent task, including writing specs, judging review findings,
merging, resolving conflicts, and deciding what to fix. You never read source code, review
files, notes, or plans. You act only on the short final messages sub-agents return.

## Constants

| Name | Value |
|------|-------|
| `RUN_ID` | the `RUN_ID=` line at the top of your kickoff prompt |
| `BENCH_DIR` | `~/encode_queue_bench/<RUN_ID>` (always use the expanded absolute path) |
| `REPO` | `~/eq-runs/<RUN_ID>`, the isolated clone made by preflight |
| `RUN_BRANCH` | `bench/<RUN_ID>` (checked out in `REPO`) |
| `BASE` | tag `bench-base-v1` |
| `WT_ROOT` | `~/eq-wt/<RUN_ID>/` |
| `DEV` | `<BENCH_DIR>/dev`, the pinned toolchain (Rust is not on PATH) |
| `L` | `<REPO>/plan/scripts/ledger.sh` |
| `STATE` | `<BENCH_DIR>/state.md`, your memory |
| `REPORTS` | `<BENCH_DIR>/reports/`, where every sub-agent writes its full output |
| `LANES` | **4** sub-agents in flight at once |
| `MAX_REVIEW_ROUNDS` | **3** per task |

## Autonomy

- Never ask the human anything, and never stop to wait for input. The run ends when P8 is
  complete.
- Answer a sub-agent's `contact_supervisor` question only from `plan/TASKS.md`. If it needs
  judgment, tell it to decide and record the decision in its report.
- Never edit anything under `plan/` or `.pi/`.

## Isolation (hard rule; breaking it invalidates the run)

- Git history may be read only for `RUN_BRANCH` and the run's own `task/<RUN_ID>/*`
  branches. Never use `--all`, `--branches`, `--remotes`, `git branch -a`, `git ls-remote`,
  `git reflog`, `git stash list`, or `git fetch`.
- No `gh` except `gh auth status` (in P0 and P8) and `gh pr create` (in P8).
- Read nothing outside `REPO`, `WT_ROOT`, and `BENCH_DIR`. That rules out other
  directories under `~/encode_queue_bench/`, `~/eq-runs/`, `~/eq-wt/`, `~/projects/`, and
  `~/.pi/`.
- The only network access allowed is the four pinned P1 source URLs, fetched with `curl`.
  No other GitHub access: in particular, not the `howtocodeit/article-md` history or any
  other `howtocodeit` repository (`hexarch` holds a complete solution).
- Put this paragraph verbatim at the top of every sub-agent prompt:

> ISOLATION: Work only inside the paths in this task. Read git history only for your own
> branch and the run branch. Never look at other branches, tags, remotes, PRs, stashes,
> reflog, or other directories on this machine, even if you suspect the work already
> exists somewhere.

## STATE (your memory)

`STATE` is a short markdown file. Keep it under about 60 lines, and overwrite it rather
than appending. It holds:
- the current phase;
- **in flight**: task, role, round, lane, and the dispatch base SHA for each;
- **ready queue**: tasks whose dependencies are met, in dispatch order;
- **waiting to merge**: tasks that passed review;
- **done**: task IDs only, with outcome (pass, escalated, dropped);
- the path of the current DAG source (`plan/TASKS.md`, `docs/refactor/PLAN.md`, a fixes file).

Re-read `STATE` before every dispatch decision, and rewrite it after every dispatch and
every completion. After a compaction, `STATE`, `$BENCH_DIR/ledger.jsonl`, and
`git -C REPO log --oneline RUN_BRANCH` are all you need. Rebuild from them before acting.

## The loop (continuous batching)

Whenever a lane is free and the ready set is not empty, dispatch. Do not wait for a whole
fan-out to finish.

**Ready-set priority:**
1. merges (only one merger at a time);
2. re-work after a FAIL;
3. reviews of finished work;
4. new tasks, those with the **longest remaining dependency chain** first, then oldest
   first.

**Agents:** only your global pi-subagents profiles are used. Never pass `model`, because that
would bypass the role's settings.

| Logical role | `agent` | Why |
|---|---|---|
| worker (implementation, research, docs, planning, triage) | `worker` | has bash, edit, write |
| merger (merges, P8.SHIP) | `worker` | needs bash and edit in `REPO` |
| reviewer | `oracle` | has bash to run the cargo gates; read-only; xhigh. (`reviewer` has no bash) |

**Dispatch:** an **async** single subagent call with these parameters:
- `agent` from the table;
- `async: true`;
- `context: "fresh"`, which is **mandatory**: `oracle` defaults to forking *your* context;
- `output: "<REPORTS>/<file>"` (absolute path) and `outputMode: "file-only"`;
- `cwd` set to the task's worktree (for merges and P8.SHIP: `REPO`);
- `task` filled in from the templates below.

Never use workflows, `tasks:` arrays, or chains. Log `task_start` immediately before the
call, then update `STATE`.

**On each completion:** the result is `Output saved to: <path>`. Run `head -n 3 <path>` and
nothing more. Log `task_end` with that status or verdict, the counts, and the usage numbers
from the completion result. Then update `STATE` and act on the outcome:

| First line of output | Next |
|---|---|
| worker `STATUS: done` | dispatch a reviewer, round k |
| worker `STATUS: blocked` | log a `note`, re-dispatch once with the same prompt; if it is blocked again, treat that as a FAIL round |
| reviewer `VERDICT: PASS` | queue a merge |
| reviewer `VERDICT: FAIL`, k < 3 | re-work: a worker with round k+1, pointed at that review file |
| reviewer `VERDICT: FAIL`, k = 3 | escalate: queue a merge (the merger's gate decides), and log the escalation |
| merger `STATUS: done` | log `merge` and `gate`, mark the task done, and unlock its dependents |
| merger `STATUS: conflict-unresolved` or `gate-fail` | dispatch worker task `<T>-rebase` in T's worktree (spec: "plan/ORCHESTRATOR.md section Rebase task spec"), then review and merge it again. If an escalated task fails its gate, log `merge ... outcome=dropped` and drop its dependents instead |
| file missing, or the first line is malformed | log `task_end ... status=error` and a `note`, then re-dispatch once |

**Phase boundaries:** log `phase_end` the moment the last task of a phase is merged or
dropped, then `phase_start` for the next phase right away. Phases never overlap.

## Git (yours: worktree bookkeeping only)

- **P0**: `git -C REPO switch -c RUN_BRANCH BASE`.
- **Before dispatching worker task `T`, round 1**:
  `git -C REPO worktree add -b task/<RUN_ID>/<T> WT_ROOT/<T> RUN_BRANCH`. Record
  `git -C REPO rev-parse RUN_BRANCH` in `STATE` as T's dispatch base.
- Re-work and rebase rounds reuse the same worktree and branch.
- **Reviewer-only tasks** (`P5.Z`, `P6.Z`, `P7.L*`, `P7.Z`) get a fresh worktree from
  `RUN_BRANCH` and are never merged.
- **After a task is merged or dropped**: `git -C REPO worktree remove --force WT_ROOT/<T>`.
  Keep the branch.
- Merging happens only through a merger task (a `worker`), one at a time, never by you.

## Prompt templates

Fill these in mechanically. Never paste spec text, findings, or file contents into a
prompt; give the path instead. The `output` parameter carries the same path as the
`Output:` line.

**Worker** (`agent: "worker"`, `cwd`: the worktree):
```
ROLE: worker. TASK <T> round <k> — run <RUN_ID>
<ISOLATION paragraph>
Worktree: <WT_ROOT>/<T>   Branch: task/<RUN_ID>/<T>   Metrics dir: <BENCH_DIR>
Toolchain: <BENCH_DIR>/dev <command>
Spec: <plan/TASKS.md section "<id>" | docs/refactor/PLAN.md section "<WPnn>" | <fixes file> section "<id>">
<only if k>1:> Fix every blocker and major in: <REPORTS>/<T>-review-r<k-1>.md
Output: <REPORTS>/<T>-worker-r<k>.md (first 3 lines per AGENTS.md)
```

**Reviewer** (`agent: "oracle"`, `context: "fresh"`, `cwd`: the worktree):
```
ROLE: reviewer. REVIEW <T> round <k> — run <RUN_ID>. One-shot adversarial review report per AGENTS.md; not a consultation.
<ISOLATION paragraph>
Worktree: <WT_ROOT>/<T>   Diff: git diff <dispatch-base>..HEAD   Metrics dir: <BENCH_DIR>
Toolchain: <BENCH_DIR>/dev <command>
Spec under review: <same spec pointer as the worker>
Worker output: <REPORTS>/<T>-worker-r<k>.md
Output: <REPORTS>/<T>-review-r<k>.md (first 3 lines per AGENTS.md)
```
For reviewer-only tasks (`P5.Z`, `P6.Z`, `P7.L*`, `P7.Z`), use `Diff: (none — review the
whole tree at RUN_BRANCH)` and leave out `Worker output`.

**Merger** (`agent: "worker"`, `cwd`: `REPO`):
```
ROLE: merger. MERGE <T> — run <RUN_ID>
<ISOLATION paragraph>
Run repo: <REPO>   Run branch: <RUN_BRANCH>   Task branch: task/<RUN_ID>/<T>   Review rounds: <k> (<pass|escalated>)
Toolchain: <BENCH_DIR>/dev <command>
Spec: plan/ORCHESTRATOR.md section "Merge task spec"
Output: <REPORTS>/<T>-merge.md (first 3 lines per AGENTS.md)
```

## Merge task spec (read by merger workers)

1. In the run repo, confirm the run branch is checked out and the tree is clean. If not,
   stop with `STATUS: blocked`.
2. `git merge --no-ff <task-branch> -m "Merge <T>: <summary> (review round <k>: <pass|escalated>)"`.
3. Resolve any conflicts yourself, keeping the intent of both sides (use the task specs and
   `docs/CONVENTIONS.md`). If you cannot resolve them with confidence, `git merge --abort`
   and report `STATUS: conflict-unresolved`, listing the files.
4. Post-merge gate: `<dev> cargo build` and `<dev> cargo test`. If the merge broke it, fix
   it in a commit `<T>-merge-fix: <summary>` and gate again. If you still cannot get it
   green, `git reset --hard ORIG_HEAD` and report `STATUS: gate-fail`.
5. `STATUS: done` with the merge SHA. In `Details`: conflicted files, how you resolved them,
   and the gate tails.

## Rebase task spec (read by `<T>-rebase` workers)

In T's worktree, `git merge <RUN_BRANCH>` into the task branch, resolve every conflict while
keeping T's spec intact, and make all of T's worker gates green. Commit, then report
`STATUS: done`.

## Ledger (mandatory, written as things happen)

`$L` appends to `$BENCH_DIR/ledger.jsonl` and timestamps every line itself. Never backfill
entries, never batch them, and never log an event you did not just observe.

```
$L phase_start <PHASE>
$L phase_end   <PHASE>
$L task_start  <T> worker|reviewer|merger round=<k>
$L task_end    <T> worker   round=<k> status=done|blocked|error  <usage>
$L task_end    <T> reviewer round=<k> verdict=PASS|FAIL blockers=<n> majors=<n> minors=<n>  <usage>
$L task_end    <T> merger   status=done|conflict-unresolved|gate-fail|blocked|error  <usage>
$L merge       <T> rounds=<k> outcome=pass|escalated|dropped
$L gate        <T> result=pass|fail
$L note        <T> msg="<one line>"
```

`<usage>` = `input_tokens=<n> output_tokens=<n> cache_read_tokens=<n> turns=<n>
tool_calls=<n> duration_ms=<n>`, taken from the completion result. Leave out any field it
does not report. Never write 0 for a missing value, and never estimate.
