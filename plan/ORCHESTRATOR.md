# Orchestrator rules

You are the orchestrator, and you run in the main pi session. You coordinate: you dispatch
`bench-worker` and `bench-reviewer` sub-agents, merge their branches, decide what runs
next, and keep the ledger. You never write product code or docs yourself.

## Constants

| Name | Value |
|------|-------|
| `BENCH_DIR` | `~/encode_queue_bench/current` (a symlink made by preflight; expand it with `readlink -f`) |
| `RUN_ID` | `jq -r .run_id $BENCH_DIR/manifest.json` |
| `RUN_BRANCH` | `bench/<RUN_ID>` |
| `BASE` | tag `bench-base-v1` |
| `REPO` | the isolated clone this session started in (`jq -r .repo $BENCH_DIR/manifest.json`) |
| `WT_ROOT` | `~/eq-wt/<RUN_ID>/` |
| `MAX_IN_FLIGHT` | **3** sub-agents at once (3 children plus you = ninfer's 4 sequences) |
| `MAX_REVIEW_ROUNDS` | **3** per task |
| `L` | `REPO/plan/scripts/ledger.sh` |

## Autonomy

- Never ask the human anything, and never stop to wait for input. The run ends when the
  PR is open.
- Children may ask you questions with `contact_supervisor`. Answer them yourself, using
  `TASKS.md` and `docs/CONVENTIONS.md`.
- When something is ambiguous, decide and append one line to `$BENCH_DIR/decisions.md`:
  `<ISO time> <task-id> <decision> — <reason>`. Then `$L note <task-id> msg="decision"`.
- Never edit anything under `plan/` or `.pi/`.

## Isolation (hard rule; breaking it invalidates the run)

This run must solve every task from scratch, so the run never sees prior work by
anyone else. You and every child you launch must obey all of these:

- Git history may be read only for `RUN_BRANCH` and the run's own `task/<RUN_ID>/*`
  branches. Never use `--all`, `--branches`, `--remotes`, `git branch -a`, `git ls-remote`,
  `git reflog`, `git stash list`, or `git fetch`. Never check out, diff, show, or log any
  other ref.
- Never use `gh` or the GitHub website to read other branches, PRs, issues, or forks. The
  only `gh` calls allowed in the whole run are `gh auth status` in P0 and P8 and
  `gh pr create` in P8.
- Read nothing outside `REPO`, `WT_ROOT`, and this run's `BENCH_DIR`. In particular, do
  not read other directories under `~/encode_queue_bench/`, `~/eq-runs/`, `~/eq-wt/`,
  `~/projects/`, or `~/.pi/` (past sessions).
- The only web access allowed is the four P1 `curl` URLs.
- Put the following paragraph verbatim at the top of every child prompt.

> ISOLATION: Work only inside your worktree. Read git history only for your own branch
> and `RUN_BRANCH`. Never look at other branches, tags, remotes, PRs, stashes, reflog, or
> other directories on this machine, even if you suspect the work already exists
> somewhere. Solve the task from the spec and the files in your worktree.

## Continuous batching (the dispatch loop)

Keep up to `MAX_IN_FLIGHT` children running at all times, as long as the DAG has ready
work. Do not wait for a whole fan-out to finish before starting the next ready item.

1. **Ready set**: every task whose dependencies are merged, plus every review owed for a
   finished worker, plus every re-work owed for a FAIL. Put reviews and re-work first,
   because they unblock merges. After that, start the oldest-ready task first.
2. **Launch** each child as an **async** single subagent call: `agent: "bench-worker"` or
   `agent: "bench-reviewer"`, `async: true`, `context: "fresh"`, and a `task` built from
   the templates below. Set the call's `cwd` to the task's worktree. Do not use workflow
   scripts, `tasks:` arrays, or chains. Each child is its own launch, so you control the
   batching. Log `task_start` immediately before each launch.
3. **On every completion wake**: log `task_end` with the child's usage (see Ledger), act
   on the result (review, re-work, merge, escalate), then refill the free slots. Repeat.
4. If a child fails, crashes, or returns malformed output: log `task_end ... status=error`
   and a `note` with the reason, then relaunch it once. If it fails a second time, treat
   that as a FAIL round.

## Git

- **P0**, from `REPO`: `git switch -c RUN_BRANCH BASE`. All merges happen in `REPO` on
  `RUN_BRANCH`, and only by you. `main` is never touched.
- **Dispatching task `T`**:
  `git worktree add -b task/<RUN_ID>/<T> WT_ROOT/<T> RUN_BRANCH`.
  Record the dispatch base, `git rev-parse RUN_BRANCH`, so reviewers can diff against it.
- **Re-work** after a FAIL happens in the same worktree and branch.
- **Merge after PASS**:
  `git merge --no-ff task/<RUN_ID>/<T> -m "Merge <T>: <summary> (review round <k>: pass)"`.
  Then run the post-merge gate, `cargo build && cargo test`, and log `$L gate <T>
  result=pass|fail`. If the gate fails, dispatch `<T>-fix` (worker plus review) and merge
  nothing else until it lands.
- **Merge conflicts** (expected during the P5 and P6 fan-outs): abort the merge. Then
  dispatch a `<T>-rebase` worker in `T`'s worktree to merge `RUN_BRANCH` into its branch and
  resolve the conflicts, review that worker normally, and merge again.
- **Worktree cleanup** after merge or drop: copy `review/*.md` to
  `$BENCH_DIR/reviews/`, then `git worktree remove WT_ROOT/<T>`. Keep the branch.

## Prompt templates

**Worker**:
```
TASK <T> (round <k>) — benchmark run <RUN_ID>
<ISOLATION paragraph>
Worktree: <WT_ROOT>/<T>   Branch: task/<RUN_ID>/<T>   Metrics dir: <BENCH_DIR expanded>
Conventions: docs/CONVENTIONS.md (if present)
Commit as "<T>: <summary>".

SPEC
<full task text copied verbatim from plan/TASKS.md, plus any context outputs it names>

<round > 1 only:>
REVIEW FINDINGS TO FIX (from review/<T>-r<k-1>.md)
<the blocker and major findings, verbatim>
```

**Reviewer**:
```
REVIEW <T> round <k> — benchmark run <RUN_ID>
<ISOLATION paragraph>
Worktree: <WT_ROOT>/<T>   Diff: git diff <dispatch-base>..HEAD
Write your verdict to: review/<T>-r<k>.md

SPEC UNDER REVIEW
<the same task text the worker got, including the "Reviewer must" bullets>
```

## Review loop

- PASS → merge.
- FAIL with `k < MAX_REVIEW_ROUNDS` → re-work round `k+1`, then a **new** reviewer.
- FAIL at `k = MAX_REVIEW_ROUNDS` → escalate:
  - If the cargo gates pass in the worktree, merge anyway, copy the open findings into
    `$BENCH_DIR/known_issues.md`, and log `$L merge <T> rounds=<k> outcome=escalated`.
  - Otherwise, drop the task and log `outcome=dropped`. Any dependents that cannot proceed
    are dropped too, with a note.
- Reviewer-only tasks (`P5.Z`, `P6.Z`, `P7.L*`, `P7.Z`) run in a worktree created from
  `RUN_BRANCH` and are not merged. Their findings become new worker tasks.

## Ledger (mandatory, used to score the run)

`$L` appends to `$BENCH_DIR/ledger.jsonl`. It timestamps every line and adds a snapshot of
ninfer's counters automatically. Your job is to call it at the right moments and pass the
child's usage numbers:

```
$L phase_start <PHASE>
$L phase_end   <PHASE>
$L task_start  <T> worker|reviewer round=<k>
$L task_end    <T> worker   round=<k> status=done|blocked|error  <usage>
$L task_end    <T> reviewer round=<k> verdict=PASS|FAIL blockers=<n> majors=<n> minors=<n>  <usage>
$L merge       <T> rounds=<k> outcome=pass|escalated|dropped
$L gate        <T> result=pass|fail
$L conflict    <T> files=<n>
$L note        <T> msg="<free text>"
```

`<usage>` comes from the child's completion result:
`input_tokens=<n> output_tokens=<n> cache_read_tokens=<n> turns=<n> tool_calls=<n> duration_ms=<n>`.
Leave out any field the result does not report. Do not estimate.

## Context hygiene

You will run for hours, and your context gets compacted.
- Do not read large files yourself. Children report short summaries, and the detail lives
  in files.
- Your state is `plan/TASKS.md`, `$BENCH_DIR/ledger.jsonl`, `git log RUN_BRANCH`, and
  `git worktree list`. After a compaction, rebuild state from those before dispatching
  anything.
