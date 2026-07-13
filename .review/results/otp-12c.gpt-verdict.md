# otp-12c — adjudication of the codex review

**Slice**: ONE_TRANSFER_PATH otp-12c (rig-D delegated parity session +
the rig-W re-baseline at the cutover sha + the new harness).
**Reviewed range**: `dcbd6ea..9350b24`.
**reviewer: gpt-5.5** (codex `gpt-5.6-sol`, 202,399 tokens, read-only).
**Raw**: `.review/results/otp-12c.codex.md`.
**Codex verdict**: FAIL — "methodology, D2 recording, and provenance
fixes required."
**Adjudication**: **7 findings, 7 accepted, 0 rejected, 0 deferred.**
**Fix sha**: `0fb4a64` (+ `docs/STATE.md` cap trim in the follow-up).

Codex independently confirmed, and I re-verified: the otp-12b F5
asymmetry does **not** recur (both arms pass contents-form sources,
land the identical tree, and pay the same in-window destination
mkdir); the verdict arithmetic, pair-voiding, valid-only medians and
`INCOMPLETE` handling all recompute exactly; both READMEs otherwise
reserve acceptance to otp-13; and the range touches no `crates/`,
`proto/` or Cargo files, so the 1484 test count is unchanged by
construction (`git diff dcbd6ea..HEAD -- crates proto` is empty —
verified, not re-run).

---

## F1 — cold-cache enforcement fails open (HIGH) — **ACCEPTED**

`scripts/bench_otp12_delegated.sh:190` + `:377–379`. A missing sudo
grant only `log`ged a WARNING, and each per-run `drop_caches` / standby
purge ended in `|| true`. A silently-failed purge therefore produced a
**warm** run that still counted as valid — the identical failure class
fixed in `a2dea3f`, which is what makes this finding sharp rather than
theoretical.

**Fix**: the grant is now a hard preflight gate (`die`, with an explicit
`COLD_REQUIRED=0` opt-out for a deliberately-warm session); each run
records its cold-cache outcome, and a failure on **either** end voids
the pair (`cold_ok` gates `RUN_VALID` in both arms).

**Does this invalidate the recorded session?** No — and I checked
rather than assumed. The grant was probed live before the run
(`sudo -n -l` shows `(root) NOPASSWD: /usr/bin/tee
/proc/sys/vm/drop_caches`, and a direct `echo 1 | sudo -n tee …`
returned `DROP_OK`), and the session ran at `a2dea3f`, which invokes
exactly that grant. The recorded drain outcomes also vary run to run
(`drained_6s`, `drained_8s`, `drained_4x2s`, `drained_9x2s`), which a
no-op purge would not produce. The bug is a real **latent fail-open**
that future sessions must not carry; it did not corrupt this one.

## F2 — "neither supersedes" contradicts D2 (HIGH) — **ACCEPTED**

`docs/bench/otp12c-delegated-2026-07-13/README.md:67`, repeated in
`.review/findings/otp-12c.md:74` and `docs/STATE.md:160`.

Codex is right and I was wrong. D2's escalation rule and its
2026-07-12 supersession amendment sit **after all four bar
definitions** and speak of "**a comparison**" generically — converge-up,
invariance, **delegated parity**, cross-direction alike. Nothing scopes
them to converge-up rows. And the trigger fired exactly as written:
both cells straddle the bar (1.119 / 1.129 vs 1.10) with an arm spread
over 25% (86.0% / 55.4%), so the RUNS=8 re-run **is** the pre-registered
mechanism, and the amendment says its medians **govern**.

The error mattered: re-interpreting a pre-registered rule *after seeing
the numbers* is precisely what the plan's pre-registration exists to
prevent, and my reading conveniently let the slice duck a verdict.

**Fix**: README, findings doc and STATE now record that the escalation
trigger was met, that the RUNS=8 medians govern per the amendment, and
that the RUNS=4 rows remain committed and visible. **Governing outcome:
rig D 7/7 PASS.** Acceptance remains the owner's at otp-13; the
evidence applies only the pre-registered arithmetic.

## F3 — provenance gate accepts dirty builds; `proto/` unguarded (MEDIUM) — **ACCEPTED**

`scripts/bench_otp12_delegated.sh:175`. Two real holes: the clean-tree
check ran `git status --porcelain -- crates Cargo.toml Cargo.lock`,
omitting **`proto/`** (blit-core's build script compiles
`proto/blit.proto` into the binaries, so proto dirt changes the build
exactly as crates dirt does); and `grep -qa -- "+$EXPECT_SHA"`
substring-matches a **dirty** build id (`+<sha>.dirty.<hash>`) — the
very shape that fooled otp-12a's provenance check on zoey.

**Fix**: `proto` added to the dirty-tree gate; new
`embeds_clean_{local,skippy,win}` helpers assert the id is present
**and** that no `.dirty` variant of it is. No evidence either hole was
exercised in the recorded session (the manifest hashes pin the actual
binaries).

## F4 — machine-readable build fields are false (MEDIUM) — **ACCEPTED**

`scripts/bench_otp12_delegated.sh:221` (manifest) and `:517`
(`runs.csv`). Both recorded `NEW_SHA` (= harness HEAD, `a2dea3f`) as the
build identity, while the gated-and-hashed binaries embed
`EXPECT_SHA` (`f35702a`). Prose and hashes made it recoverable, but the
machine-readable field said something untrue.

**Fix**: the `sha` / `build` columns now carry `EXPECT_SHA` — the
identity that was actually gated and hashed — and the harness checkout's
HEAD is recorded separately as itself (`# harness_head=… binary_identity=…`).
The default staging paths key off `EXPECT_SHA` too, so the paths and the
gate can no longer disagree.

## F5 — silent `sync` / drain failures (MEDIUM) — **ACCEPTED**

`scripts/bench_otp12_delegated.sh:156` — a failed skippy `sync` was
masked by the succeeding `echo`, yielding a plausible numeric flush on
unflushed bytes and a **valid** run. `:359` — if `SKIPPY_DISK_REGEX`
matches no device, `awk` sums nothing, every delta is 0, and the loop
reports `drained` on a disk it never watched.

**Fix**: the flush is sentinel-framed and emitted only on `sync` rc=0
(failure → `NA` → the run voids); the drain asserts the regex matches
≥1 device before polling (no match → `DRAIN-NODEV`, which is not
`drained*`, so the pair voids). Codex correctly notes the recorded
drain outcomes vary, so the no-match path did not fire live.

## F6 — teardown claims success without verifying (MEDIUM) — **ACCEPTED**

`scripts/bench_otp12_delegated.sh:291` (and the Windows twin). Both
stop paths suppressed every failure, cleared `*_DAEMON_STARTED`, and
logged "stopped" unconditionally — so the harness could exit **0** with
a daemon still holding `:9031`, and the EXIT trap would never retry. The
next session would then die on its own stale-listener refusal, with the
previous run's log claiming a clean shutdown.

**Fix**: both paths now verify the pid is actually gone, log `ERROR` and
set `TEARDOWN_LEAK` if it survived; `on_exit` turns a leak into a
non-zero exit ("never exit 0 on a leak").

## F7 — README miscounts a PASS among the FAILs (LOW) — **ACCEPTED**

`docs/bench/otp12c-delegated-2026-07-13/README.md:55`. The sentence
framing "the primary FAILs sit on high delegated-arm spread" listed
`sw_tcp_small` (93.6%) alongside the two real FAILs — but `sw_tcp_small`
**passed** at 1.034.

**Fix**: the note now names only the two FAIL cells and calls out
`sw_tcp_small` explicitly as the widest-spread cell that nonetheless
passed — spread alone does not decide a cell.

---

## Gate after fixes

`bash -n scripts/bench_otp12_delegated.sh` OK;
`bash scripts/agent/check-docs.sh` OK. Docs+scripts only — no
`crates/`/`proto/` change, so the cargo suite stands at **1484**
untouched. shellcheck is not installed on this host (recorded, as in
otp-12a/12b).

**The harness fixes are not re-validated against the rig**: they change
gates and teardown, not the measurement path, and re-running the matrix
to exercise them would produce a *different* session rather than
validate this one. The recorded evidence stands on the harness as it ran
(`a2dea3f`); the fixes bind the **next** session.
