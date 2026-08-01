# cr-ls4-1 — the apply-concurrency guard never observed concurrent execution

**Severity**: LOW
**Status**: FIXED — awaiting reviewer verification
**Source**: `clp3-ls4-range` codex dispatch over `23760d8d..ac74e059`
Reviewer: codex / gpt-5.6-sol / xhigh / workspace-write (detached, disposable
worktree); codex-cli 0.146.0; **`guard_confirmed: false`**.
Record: `.review/results/clp3-ls4-range.codex.json`.

## The finding, as returned

> The new guard checks the configured worker accessor and final tree
> equality but never measures concurrent in-flight work. Forcing the
> production wiring back to one worker left the intended test green; the
> exact bytes were restored and the test remained green. Thus the
> performance capability can regress silently. Add an injected sink or
> pipeline probe that blocks workers and asserts peak concurrency exceeds
> one through `run_local_session`.

## Why it is admitted — twelfth vacuous guard, second repeat of sub-species 2

Testing the implementation (the accessor, the constant, the resulting tree)
instead of the seam where the behaviour lives (the pipeline actually running
workers). cr-ls1-9 was this exact gap for the checker pool; I closed it
there with pool injection and an `installs()` observation, then rebuilt the
same blind spot for the sink one slice later. A single worker produces the
same correct tree, just slower — so tree equality can never see this, and
the accessor is one `if` away from the wiring it claims to describe.

## Fix

- `LocalMirrorOptions::sink_override: Option<SinkOverride>` — the write-side
  twin of the `checker_pool` injection seam. `SinkOverride` is a newtype
  with a manual `Debug` (the trait object is not `Debug`); no production
  caller sets it, and `run_local_session` prefers it over building
  `FsTransferSink`/`NullSink`.
- `ConcurrencyProbeSink` wraps the real `FsTransferSink`, holds each
  `write_payload` open 40 ms, and records PEAK in-flight calls.
- `a_normal_session_holds_multiple_payloads_in_flight` drives
  `run_local_session` — the public entry every CLI copy takes — over 12
  files each >1 MiB (so each plans as its own `File` payload) and asserts
  `peak > 1`, plus `copied_files == 12` so the probe cannot pass by
  dropping work.

**Guard proof**: the reviewer's exact mutation — `sink_workers:
options.effective_sink_workers()` → `sink_workers: 1` at the `LocalApply`
construction — reds it with `peak in-flight payloads was 1; the apply
pipeline executed sequentially, whatever its configuration says`. Restored
byte-identical by SHA-256 (`0748CA70…08D5`), green after. Restore was done
by contextual edit rather than blanket replace, because test fixtures in
the same file legitimately pin `sink_workers: 1`.

## Standing note, updated

Twelve vacuous guards. The three sub-species list in `cr-ls1-15-16.md`
stands; this is another instance of №2 (implementation, not seam), caught —
as every one before it — by a reviewer running the revert, never by reading
the assertion.
