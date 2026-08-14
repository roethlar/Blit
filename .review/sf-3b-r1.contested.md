# sf-3b r1 contested: completed review returned no accessible verdict

**Kind**: output-contract failure, not a disagreement over the change.
Claude Code completed the owner-ordered review, but the captured result
contained only an inaccessible content-reference token rather than the
required verdict schema. The observed envelope is preserved at
`.review/results/sf-3b-r1.claude.json`.

## What happened

The neutral openreview dispatch used Claude Code 2.1.232,
`claude-fable-5` at `xhigh`, competitive grade, over exact range
`d5f5781ddc5503093a68df7cf23cdd779172d4a8..dab7bb82f86d726a5ced0b21ee277b6b1a8454e0`.
It ran in a disposable worktree with read-only repository tools and the
focused sink test authorized. Transport completed successfully after 10
turns (`is_error: false`, `terminal_reason: completed`), session UUID
`e9737624-35c0-4292-ab27-fde444812c73`, at a reported cost of
`$2.184326`.

The result field was only
`<<ccr:cabb9af7635b,string,3.7KB>>`. No verdict, `capability_ok`,
goal/approach assessment, or findings were accessible. The single
playbook-permitted re-emission-only attempt failed immediately because the
original invocation used `--no-session-persistence` and Claude reported
`No conversation found with session ID`. The review was not rerun.

## Consequence

Fail closed: there is no accepted neutral-review verdict and no findings to
admit or decline. This record does not weaken the local tests, mutation-style
cost proxy, cross-target checks, or rig A/B evidence already recorded under
`docs/bench/sf3b-parent-readiness-2026-08-13/`; those remain local evidence,
not a substitute reviewer verdict. sf-3b remains pending and sf-3c has not
begun.

## What the owner must decide

1. **Fresh review** (explicit go required): repeat the exact-range paid
   dispatch with `--json-schema` so the structured payload is captured and
   retain session persistence for a re-emission fallback.
2. **Local closure**: explicitly waive/amend this slice's neutral-review gate
   and accept the recorded local evidence.
3. **Leave pending**: keep sf-3b open and do not begin sf-3c.

## Resolution — 2026-08-14 (D-2026-08-14-1)

The owner chose none of the three options above: they ordered the review
performed in the working session itself, explicitly without playbooks and
without another paid dispatch. Review of exact `d5f5781d..dab7bb82` by
claude-fable-5 — the session's own working agent, primed by session context,
not a neutral external reviewer.

**Verdict: sound, no defects.** Traced and confirmed:

- Per-file containment classification is unchanged — the chain-walking
  classifier (`failure_is_containable`, sink.rs) sees through the new
  `with_context` layers, and the fatal classes (read-only volume, ENOSPC,
  dead root) still escape containment.
- The R58-F4 dry-run gate stays above the new path; dry-run remains
  side-effect-free.
- The readiness-map mutex is never held across an await.
- Generation invalidation by `Arc::ptr_eq` identity holds under the traced
  races: a late failure cannot evict a newer success; an evicted cell's
  concurrent success is merely uncached (one redundant idempotent mkdir
  later); waiter-retry on failed init matches pre-change cost.
- `resolve_destination` path-safety still precedes the cache for every wire
  path; cache keys are resolved paths.

**Guard proof (re-run independently, Linux):** mutating `parent_readiness`
to bypass the map (fresh once-cell per call) reds
`fs_sink_prepares_a_shared_parent_once` at exactly `16 != 1` — the pre-fix
behavior; byte-identical restore; all 20 focused sink tests green; GitHub CI
green on the covering runs.

**Non-blocking observations (no action required):**

1. The blocking-pool paths — `write_file_payload` and the tar-shard member
   writes — still run one `create_dir_all` per file/member. Out of sf-3b's
   stated scope; sf-3c/sf-4 should not rediscover it.
2. If the NotFound retry's mkdir succeeds but its second create fails on
   another raced removal, the fresh generation stays cached "ready" until the
   next file self-heals through the same NotFound path. Nothing sticks.
3. `prepare_destination` and non-NotFound create failures evict the parent
   entry even when the parent is fine — under persistent per-file errors this
   degrades to exactly the pre-change per-file mkdir cost. Conservative, safe.

Runtime verification is Linux-only; Windows coverage is compile parity
(cross-clippy) plus the ungated tests on Windows CI. Owner accepted
2026-08-14. **sf-3b is closed; sf-3c may be selected.**
