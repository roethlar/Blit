# Baseline Findings Triage — 2026-05-02

Triage of `docs/reviews/codebase_review_2026-05-01.md` against the
post-session codebase. The review series in
`docs/reviews/followup_review_2026-05-02.md` (Rounds 1–10) closed the
F1 path-safety class plus several finding-adjacent bugs that GPT
surfaced during follow-up review. This document re-ranks the
remaining baseline findings for release-readiness work.

No code changes — analysis only. Per-finding status is verified
against the current tree (commit 503342c).

## Summary

| Finding | Original severity | Status now | Triage rank |
|---------|-------------------|------------|-------------|
| F1 | High | **Closed** (path safety primitive shared) | — |
| F2 | High | **Open** | **1 (release-blocking)** |
| F3 | Medium | Open | 4 (security default) |
| F4 | Medium | **Closed** (Step 4B + MirrorMode) | — |
| F5 | Medium | Open (partial) | 5 |
| F6 | Medium | **Closed** (HTTP server removed) | — |
| F7 | Medium | **Open** (now elevated — same class as R6-F1) | **2 (release-blocking)** |
| F8 | Medium | **Open** (now inconsistent with R5-F3 cap) | **3 (release-blocking)** |
| F9 | Medium | Open | 8 (code smell) |
| F10 | Medium | **Closed** (Step 4B filter parity) | — |
| F11 | Medium | Open | 6 |
| F12 | Low–Medium | Open | 7 |
| F13 | Low–Medium | Open (now also impacts F2 truthfulness) | 4 (with F2) |
| F14 | Low | Open (reduced — only FSEvents deprecation) | 9 |
| F15 | Low | Open (deferred per project state) | 10 |

The original review's architecture observation — "path safety is not
yet treated as a first-class shared abstraction" — is **closed** by
this session. `crates/blit-core/src/path_safety.rs` plus
`crates/blit-core/src/remote/transfer/tar_safety.rs` are the shared
chokepoints all receive sites route through.

## Closed in this session

### F1 — receive-side path sanitization
Closed via `crates/blit-core/src/path_safety.rs` (`validate_wire_path`
+ `safe_join`) applied at every receive site, plus
`crates/blit-core/src/remote/transfer/tar_safety.rs` for the
tar-extract chokepoint. Six review rounds (R1, R5-F1, R5-F2, R6-F1,
R6-F2, R6-F3) plus the Round-7 consolidation hardened the entire
class.

### F4 — filtered mirror delete semantics
Closed in Step 4B. Daemon now sends an authoritative `DeleteList`
scoped per `MirrorMode::FilteredSubset` (default) or
`MirrorMode::All` (opt-in via `--delete-scope all`). Out-of-scope
files survive by default; the CLI no longer walks the dest tree.
Two integration tests pin the FilteredSubset/All split.

### F6 — metrics HTTP server hardening
Closed by removing the HTTP server entirely. Counters are now
opt-in via the daemon's `--metrics` flag and live in-process for
future TUI consumers. No exposed endpoint to harden.

### F10 — remote pull filter parity
Closed in Step 4B. Daemon honors `FilterSpec` via `FileEnumerator`
during enumeration; CLI builds the wire spec from
`--exclude/--include/--min-size/--max-size/--min-age/--max-age/--files-from`.
Help text/code comments updated.

## Open — release-blocking

### Rank 1: F2 — `use_chroot` containment (User priority)

Severity: **High**. Status: **fully open**.

`crates/blit-core/src/path_safety.rs:29` documents the lexical-only
contract explicitly:

> NOT canonicalize the result or resolve symlinks. […]
> `use_chroot` / canonical-containment work tracked as F2 […]

`_use_chroot` is loaded through to `runtime.rs:200` and
`service/util.rs:21` but never enforced. A symlink inside a module
root that points outside it will be followed by any operation that
calls `metadata`, `File::open`, `File::create`, or `read_dir` on a
joined path.

This matters because:
- Docs (`docs/DAEMON_CONFIG.md`, `docs/plan/WORKFLOW_PHASE_3.md`)
  claim chroot enforcement that doesn't exist (couples to F13).
- The push receive `apply_tar_shard_sync` symlink-creation hole
  closed in this session blocked one creation vector. The
  follow-symlink-on-read vector remains.
- Any operator who reads the docs and configures `use_chroot = true`
  on a writable module root inherits implicit trust without an
  enforcement ground truth.

Recommendation: implement canonical containment. For each daemon
operation that touches a path under `module.path`, walk to the
deepest existing ancestor, `canonicalize` it, and confirm it
`starts_with(canonicalized_module_root)`. For write paths that
create new files, validate each existing parent before creation.
List/pull/push/purge/find/du/completions all need symlink-escape
tests. Block on this before release; otherwise rename or remove the
option and the doc claims (the lesser fix that closes the
truthfulness gap without the enforcement work).

Effort: substantial. Bigger blast radius than any individual review
round in this session because it touches every daemon read/write
site. Likely 1–2 sessions of targeted work + an integration test
pass.

### Rank 2: F7 — remote-source tar shard prepare_payload allocation

Severity: **upgraded to High** (was Medium in 2026-05-01).

`crates/blit-core/src/remote/transfer/source.rs:170` still does
`Vec::with_capacity(header.size as usize)` from a daemon-controlled
`FileHeader.size`. This is the **send-side mirror of R6-F1** —
remote→remote transfers (one daemon prepping a tar shard from
another daemon's manifest) are vulnerable to the same per-entry
allocation attack we closed on the receive side.

The fix shape is the same as `tar_safety::safe_extract_tar_shard`:
validate `header.size` against a cap and `try_reserve_exact` instead
of `Vec::with_capacity`. The TODO at line 164 acknowledges the
double-buffering but doesn't gate on size.

Recommendation: reuse `tar_safety::MAX_TAR_SHARD_BYTES` as the
per-entry cap. Use `try_reserve_exact`. Reject early if
`header.size` exceeds the cap; the daemon should fall back to
single-file File payloads. A streaming tar builder would also help
but is bigger surgery — bounded allocation alone closes the bug.
Add a regression test with a fake remote source advertising a
huge `FileHeader.size`.

Effort: small. Pattern is well-defined from the R6-F1 fix; the test
fixture infrastructure exists already.

### Rank 3: F8 — wire data-plane tar shard cap

Severity: **Medium**. Status: **open and now inconsistent**.

`crates/blit-core/src/remote/transfer/pipeline.rs:334` still has:

```rust
const MAX_WIRE_TAR_SHARD_BYTES: usize = 1024 * 1024 * 1024;
```

The new `tar_safety::MAX_TAR_SHARD_BYTES` is **256 MiB**. The two
caps disagree: the data-plane wire reader will accept a 1 GiB tar
shard that the helper would later reject. That widens the in-memory
window during which a bad daemon can pin a gigabyte of memory
before the bounds check trips.

Recommendation: harmonize. Either lower the wire cap to 256 MiB to
match the helper, or thread a single shared constant. Keep the
larger 1 GiB constant only if there's a documented planner reason —
there isn't currently. Worth a 5-line change and a test that
attempts a `tar_shard_bytes > 256 MiB` and asserts rejection at
the wire boundary.

Effort: trivial.

## Open — release-prep, non-blocking

### Rank 4: F3 — default bind on `0.0.0.0`

Severity: Medium. Status: open.

`crates/blit-daemon/src/runtime.rs:156` still defaults to
`0.0.0.0`. With no built-in TLS or auth, the daemon is exposed to
the entire LAN by default. Mostly a security-defaults issue.

Recommendation: default to `127.0.0.1`. Require explicit
`--bind 0.0.0.0` (or config opt-in) for LAN exposure. If LAN is
intentional default, emit a startup warning when binding non-loop.

Effort: trivial. ~10 LOC + one test.

### Rank 4 (tied): F13 — docs drift on chroot status

Severity: Low–Medium. Status: open and load-bearing under F2.

`docs/plan/WORKFLOW_PHASE_3.md:15`, `:35`, `:44`, `:57`, `:114` and
`docs/plan/PROJECT_STATE_ASSESSMENT.md:24` claim chroot/read-only
enforcement is in place. F2 is the truth. Best to fold this into
the F2 fix or removal: when F2 lands, the doc claims become true;
if F2 is descoped, the docs and the option both go.

Effort: trivial once F2 is decided.

### Rank 5: F5 — `active_transfers` RAII guard + counter semantics

Severity: Medium. Status: open.

`crates/blit-daemon/src/service/core.rs:90,97,118,123,144,157`
inc/dec around the awaited handler. A panic between the inc and
dec leaks the gauge; aborted spawn tasks do the same. `inc_purge`
fires only after a successful delete (line 258), contradicting the
metrics-module doc that says counters mark dispatch attempts.

Less operationally urgent now that metrics are opt-in (F6
collapsed the public surface), but a misleading gauge during
failure scenarios is exactly when it matters.

Recommendation: RAII guard struct that holds `Arc<TransferMetrics>`
and decrements on `Drop`. Move into the spawned task. Decide
whether `inc_*` are attempts or successes; pick one and align all
counters.

Effort: small.

### Rank 6: F11 — pull `PullSyncAck.server_checksums_enabled` discarded

Severity: Medium. Status: open.

`crates/blit-core/src/remote/pull.rs:528` still has
`let _ = ack.server_checksums_enabled;` and the TODO at line 527.
The capability bit reaches the client but is unused; the comparison
strategy doesn't react to "daemon has checksums disabled."

Recommendation: store the ack on a client-side state struct, use
it to drive comparison-mode fallback (e.g., daemon disabled →
client refuses `--checksum` mode with a clear error rather than a
silent size+mtime fall-through). Add a test where daemon checksums
are disabled and assert intentional behavior.

Effort: small.

### Rank 7: F12 — `blit check` directory/symlink semantics

Severity: Low–Medium. Status: open.

`crates/blit-cli/src/check.rs:127,202` still skips non-file entries
silently. Documentation hasn't picked a side.

Recommendation: pick "transfer equivalence" (current behavior,
matches mirror semantics), document it in `--help` and the man
page, and reject non-file entries explicitly with a one-line
warning when verbose. Or pick "tree equivalence" and include
directory/symlink in the diff model. The first is the smaller fix.

Effort: small (option A) or substantial (option B).

### Rank 8: F9 — nested Tokio runtime in `execute_local_mirror`

Severity: Medium. Status: open.

`crates/blit-core/src/orchestrator/orchestrator.rs:331,353` still
constructs a runtime and `block_on`s. Library API hazard, not a
correctness or security issue. Bites if a future caller invokes
from an async context.

Recommendation: split into `execute_local_mirror_async` plus a
small sync wrapper. Let the CLI's existing async context call
the async version directly.

Effort: small to medium (touches one public API and a few callers).

### Rank 9: F14 — remaining warnings (FSEvents deprecation)

Severity: Low. Status: reduced.

The unused-variable warning is gone. Two FSEvents deprecation
warnings remain (`crates/blit-core/src/change_journal/snapshot.rs:85`
and `:111`).

Recommendation: migrate to `objc2-core-services` per the deprecation
hint. Could also `#[allow(deprecated)]` with a tracking comment
if the migration is non-trivial — not great long term but acceptable
short term.

Effort: small if `objc2-core-services` exposes a drop-in replacement;
medium if not.

### Rank 10: F15 — structured logging deferred

Severity: Low. Status: explicitly deferred per project state.

`println!`/`eprintln!` throughout daemon and transfer paths.
Already documented as deferred. Re-confirming the deferral is the
right call for 0.1.0.

Recommendation: keep deferred. Track as a 0.2.0 epic.

Effort: large; whole-codebase migration to `tracing`.

## Recommended release-track sequencing

1. **F2 first.** Either implement canonical containment or remove the
   option + doc claims. This is the only remaining High-severity
   item, and F13 is gated on the decision.
2. **F7 next.** Reuses the R6-F1 pattern; closes the last
   tar-shard allocation hole on the send side. Small surgery.
3. **F8.** Harmonize the wire cap with the helper cap. Trivial.
4. **F3 + F13 cleanup.** Default-bind change + docs sync after F2.
5. Stop here for 0.1.0 if time-constrained. F5/F11/F12/F9 are all
   real but non-release-blocking. F14/F15 explicitly deferred.

## Out-of-scope items surfaced during triage

- The session's tar-safety consolidation eliminated drift between
  three receive sites. F7 is the one tar-shard primitive that
  *isn't* on a receive path; consolidating its safety policy with
  `tar_safety` would close the last source-side allocation gap.
- F2's symlink-escape exposure also affects the existing receive-side
  `safe_join` work: a symlink at `module/inbox` pointing at `/etc`
  still resolves outside the module root despite all the lexical
  validation, because `safe_join` is documented as lexical-only.
  This is not a regression from this session — it's the same gap
  the original F2 flagged.
