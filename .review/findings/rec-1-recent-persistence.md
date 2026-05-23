# rec-1-recent-persistence: persist `GetState.recent[]` across daemon restarts

**Severity**: Feature (recent-persistence, step 1)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `7c095b2`

## What

Resolves TUI_DESIGN open-question #1 (owner decision 2026-05-23): the
daemon's recent-runs ring (`GetState.recent[]`) was in-memory only and
lost on restart. This slice persists it to disk so the TUI's F2 recent
list survives daemon restarts.

**Separation constraint (owner):** clearing recents (future rec-2) must
never touch the planner/predictor's historical telemetry
(`perf_local.jsonl`). This slice establishes that separation: recents
live in a **dedicated** `recents.jsonl`, in the same `config_dir()` but
an entirely separate file from `perf_local.jsonl`. Nothing here reads or
writes the planner's store.

## Approach

- **`recents_store` module** (new, `crates/blit-daemon/src/recents_store.rs`):
  - `recents_path()` → `blit_core::config::config_dir()?/recents.jsonl`.
  - `load(path, limit)` — tolerant: missing/unreadable file → empty;
    malformed line skipped (a hand-edited/partial store must not block
    startup); trimmed to the newest `limit` (oldest-first).
  - `write_atomic(path, records)` — temp file + `sync_all` + rename, so
    a crash mid-write never leaves a torn store. The ring is bounded
    (`DEFAULT_RECENT_LIMIT` = 50), so the full rewrite stays small.
- **`ActiveJobs` persistence (opt-in)**:
  - `Inner.persist_tx: OnceLock<UnboundedSender<()>>` — empty by default,
    so `ActiveJobs::new` and all test/default paths touch no disk.
  - `arm_persistence()` / `arm_persistence_at(path)` — hydrate the ring
    from disk (drain-then-extend, trimmed to limit) and install the
    signal sender; return a `RecentsWriter`.
  - `Drop` (after `push_recent`) does a **non-blocking** `tx.send(())` —
    no file I/O on the runtime (`Drop` is synchronous, on a tokio
    worker). The unbounded channel never awaits/blocks.
  - `RecentsWriter::run` coalesces queued signals (a burst of
    completions → one rewrite) and atomically rewrites the bounded ring.
    `spawn_recents_writer` drives it for the daemon's lifetime, mirroring
    `spawn_progress_ticker` (the daemon has no graceful-shutdown hook, so
    write-through — not save-on-exit — is what makes recents durable).
- **`main`** arms persistence + spawns the writer before `serve`, so the
  first `GetState` already reflects pre-restart recents.
- serde `Serialize`/`Deserialize` on `TransferRecord` + `ActiveJobKind`
  (`snake_case`); `serde_json` dep added to `blit-daemon`.

## Why this design

- **Non-blocking `Drop`**: blocking file I/O in a `Drop` on the async
  runtime is the class of bug this reviewer has reopened before
  (tokio-Mutex-in-Drop, hung awaits). The signal-channel + writer-task
  keeps `Drop` synchronous and instant.
- **Atomic rewrite over append**: keeps the file bounded (no unbounded
  growth / compaction needed) and crash-safe; the ring is small so full
  rewrites are cheap.
- **Opt-in persistence**: tests and any non-daemon `ActiveJobs` stay
  in-memory — no global config-dir dependency, no test races, no
  surprise disk writes.

## Files changed

- `crates/blit-daemon/src/recents_store.rs` (new): store module + tests.
- `crates/blit-daemon/src/active_jobs.rs`: serde derives; `persist_tx`
  `OnceLock`; `arm_persistence[_at]`; `RecentsWriter` + `run` +
  `spawn_recents_writer`; `Drop` signal; tests.
- `crates/blit-daemon/src/main.rs`: `mod recents_store`; arm + spawn at
  startup.
- `crates/blit-daemon/Cargo.toml`: `serde_json`.

## Tests

`blit-daemon` 137 (was 128; +9):

- `recents_store`: load-missing→empty; write→load round-trips oldest-first;
  malformed line skipped; trim-to-limit keeps newest; zero-limit empty;
  `write_atomic` replaces + leaves no `.tmp`.
- `active_jobs`: `arm_persistence_hydrates_ring_from_disk`;
  `completed_transfer_writes_through_to_disk` (spawned writer, polled
  flush); `unarmed_table_does_not_persist` (opt-in guard — a never-armed
  table writes no file).

## Scope / next

rec-1 is daemon-only, **no wire change** — `GetState.recent[]` is
unchanged on the wire; it's just now hydrated from disk and written
through. Next:
- **rec-2**: `ClearRecent` RPC — clears the in-memory ring **and**
  `recents.jsonl`, with a test asserting `perf_local.jsonl` is untouched.
- **rec-3**: TUI "clear recent" action on F2 (key via `resolved()`
  collision policy, footer hint, dispatch → RPC).

## Reviewer comments

(empty — pending review)
