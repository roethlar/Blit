# ue-r2-1d — codex (gpt-5.5) review output

Reviewer: `codex exec -s read-only` on `gpt-5.5` (headroom proxy), slice
commit `c08a5c1`. Findings excerpt (full transcript not retained, per
ue-r2-1a precedent; 199,475 tokens used):

---

## Findings

- `crates/blit-core/src/engine/mod.rs:503` — **High** — The pipeline
  now writes while the source walk is still active. There is no guard
  rejecting `dest_root` under `src_root`, so a local copy/mirror can
  create destination files inside the tree being walked; unlike the old
  collect-all path, those self-created files can be enumerated,
  planned, and copied recursively. This violates "same as one big plan"
  semantics and can cause unbounded self-copy.

- `crates/blit-core/src/engine/mod.rs:510` — **Medium** —
  Pipeline/planner errors return via `?` before `scan_handle` is
  awaited. That means the scan task is detached on those error paths,
  and the documented walker abort-on-send-failure path is not actually
  observed; scan panic/queue errors are also dropped. Capture the scan
  result before applying pipeline→planner→scan precedence.

VERDICT: FAIL — streaming foundation is close, but the concurrent
write-during-walk nested-destination regression needs fixing before
acceptance.
