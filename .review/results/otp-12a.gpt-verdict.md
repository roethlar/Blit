# otp-12a harness review — adjudication

**Reviewed commit**: `8f4fbf9` (adds `scripts/bench_otp12_zoey.sh` + the
D5 schema amendment). **Raw review**: `.review/results/otp-12a.codex.md`
(gpt-5.6-sol, 107,052 tokens). **Verdict**: REQUEST CHANGES — 9 findings
(4 High, 2 Medium, 3 Low). All nine verified against the script and
ACCEPTED — no false positives this round.
reviewer: gpt-5.6-sol

## F1 (High) — INCOMPLETE handling broken

Confirmed: the verdict loop iterated `by_arm` keys, so a zero-valid cell
vanished from `verdicts.csv`, and `summary.csv` printed medians over 1–3
valid pairs. Fixed: verdicts iterate EVERY attempted comparison (`meta`);
`complete(cell)` gates both files; summary rows exist only for complete
cells — never a median below RUNS valid runs.

## F2 (High) — fixtures/pull sources trusted by existence

Confirmed (inherited from the frozen harness, but this is acceptance
evidence). Fixed: fixtures verified by file count + byte sum against the
pre-registered shapes (mismatch = hard stop with an explicit removal
instruction, never auto-delete); staged pull sources verified by remote
file count, re-staged convergently and re-verified on mismatch.

## F3 (High) — provenance not enforced; maskable manifest hashes

Confirmed: a stale-but-matching pair passes the handshake yet is labeled
`NEW_SHA`; `echo "$(zssh sha256sum …)"` masked failures. Fixed: all four
binaries must embed their arm's sha (`grep -q` on the binary —
`session_build_id`'s compile-time literal; the old commits postdate
otp-3 and embed it too); dirty tree is now FATAL, not a warning; hashes
captured via `sha256_local`/`sha256_remote` helpers that die on failure
or a non-64-hex result; the committed reference's sha256 joins the
manifest.

## F4 (High) — EXIT trap could kill an unowned PID

Confirmed: the unconditional trap killed whatever the fixed pidfile
named, including on preflight refusal. Fixed: `on_exit` acts only when
THIS session started a daemon (`DAEMON_EVER_STARTED`), and `stop_daemon`
kills only after verifying `/proc/<pid>/comm` matches `blit-daemon`.
`PREFLIGHT_ONLY` now provably writes nothing on zoey and kills nothing.

## F5 (Medium) — overrideable reference; unregistered outcome vocabulary

Confirmed. Fixed: `BASELINE_SUMMARY` is hardcoded to the committed path
(pre-registered; sha256 recorded); a missing reference row for a
complete cell ABORTS the verdict pass (fail closed); `NO-REFERENCE` is
gone; the vocabulary is registered in the design doc (per-reference rows
`PASS|FAIL`, combined rows the D2 set).

## F6 (Medium) — per-run logs inside the timed window

Confirmed: file-backed stdout differs from the frozen harness's
`/dev/null` and is arm-dependent. Fixed: timed stdout → `/dev/null`
exactly as the frozen harness; only stderr (silent unless failing) is
kept per run for diagnostics.

## F7 (Low) — reused pull destination path

Confirmed vs D5's never-seen rule. Fixed: unique
`dst_pull_<session>_<cell>_<arm>_<rid>` per run, removed after its flush
is measured; the EXIT trap sweeps any leftovers.

## F8 (Low) — RUNS unrestricted

Confirmed. Fixed: preflight refuses anything but 4 (standard) or 8 (the
D2 escalation).

## F9 (Low) — doc cell-grammar token order

Confirmed: labels are `<verb>_<carrier>_<fixture>` (matching the
committed reference CSV). Doc corrected.

## Fix commit

fix sha: `50dc135` (`bash -n` clean; check-docs green; no crates/proto
changes anywhere in the slice — suite stands at the recorded 1484).
