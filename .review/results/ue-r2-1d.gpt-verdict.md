# ue-r2-1d — adjudication of review findings

reviewer: gpt-5.5 (codex exec, read-only, headroom proxy)
slice commit: `c08a5c1`
raw output: `ue-r2-1d.codex.md` (trimmed; findings + verdict retained)

VERDICT: FAIL → fix-then-ship. Both findings verified against source
and **Accepted**; both fixed in the fix commit below; gate re-run
green.

1. **High — nested-destination self-copy — Accepted.** Verified real:
   with writes concurrent to the walk, `blit copy src src/backup`
   (streaming leg) lets the walker re-enumerate freshly written
   destination files; each is "absent at destination/backup/backup" so
   the diff stage re-plans it — recursive self-copy the collect-all
   ordering made impossible. **Fix**: the engine computes the
   destination's source-relative prefix when `dest_root` sits under
   `src_root` and the streaming planner drops headers under it before
   planning (`StreamingPlanInputs.exclude_dest_subtree`). The fast
   paths keep walk-fully-then-copy ordering and are unaffected (a
   nested-dest Tiny run retains the old copy-dest-once-non-recursive
   quirk — divergence noted as a follow-up candidate, not a
   regression). Regression test
   `nested_destination_does_not_self_copy` runs twice so the second
   walk deterministically sees the pre-existing destination; on the
   unfixed code its `copied_files == 0` / no-`backup/backup`
   assertions must fail, because `plan_local_mirror` finds
   `backup/backup/f*.txt` absent and re-plans every file
   (deterministic from the diff semantics; a live revert-proof was
   skipped as the unfixed second run's recursion depth is
   race-dependent and could run long).

2. **Medium — scan task unobserved on early error returns —
   Accepted.** Verified: `pipeline_result?`/`plan_result?` preceded
   `scan_handle.await`, dropping scan panics/queue errors on those
   paths (the walker itself still terminated via send-abort). **Fix**:
   `scan_handle` is awaited into `scan_result` immediately after the
   `join!` (both arms complete ⇒ prompt), before error precedence
   applies; precedence order itself unchanged
   (pipeline → planner → scan).

## Fix commit

- Fix sha: `29159ca`. Gate after fixes:
  fmt clean, clippy clean, tests 1399 passed / 0 failed / 2 ignored
  (+1 regression test).
