# cr-ecm35-3: the log and the saved record named failures past the reason cap differently

**Severity**: HIGH — a saved retry can address the wrong file, or miss one. The run log and the saved run record each joined a run's failures — the reasons the report kept, at most 64 — to its failed paths by the text alone:
- With 65 failures and no name heard live, the record named the 65th by its text only, losing its bytes, while it counted the list whole.
- Two failed files of one text (`dup\xff`, `dup\xfe`), the cap falling between them, were logged once.
**Status**: Reviewed, found incomplete — `combined-r37` (Codex openreview, 2026-11-24) reviewed `d2494965..2c55cb3a` — the `combined-r36` fixes — with `guard_confirmed: true`: "Exclusive creation, applying permissions through the open handle, and sharing failure accounting between the log and saved record are sound choices. The unretried count also works through the CLI. With one requested name over the wire budget, it reported one failure and one file not retried." "For one missing 4,618-byte path, the CLI saved both the full name and its 4,096-byte `[truncated]…` rendering. It marked the list complete. `jobs retry` then reported two failures. Attach each bounded diagnostic to its exact path identity." "With 70 Unix filenames sharing one display spelling, one copied and 69 failed. The saved record named only 65, so `jobs retry` refused. The retry scan also assigned one unreadable entry's reason to two distinct raw names. Use exact identities in both producers and in the summary join. Its growing-vector search took 0.17, 0.68 and 2.72 seconds for 8,000, 16,000 and 32,000 unique names in the test build." The remaining work is cr-ecm36-3 and -4
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `4146e1fa`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r36` over `dc14afc4..d2494965` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r36.codex.md`. It completes cr-ecm34-5 (`183408ae`) in part.

## Evidence
"With 65 failures and no live name event, the saved record claimed a complete list while losing the 65th filename's raw bytes. In a separate probe, `dup\xff` and `dup\xfe` produced only one log entry when the reason cap fell between them. Join reasons to exact identities and use that result for both writers."

## Intake
Admitted as the reviewer put it.

## What
One join, `RunTotals::failed_files()`, gives every failed file the account names, each once, by its identity: its text, and its bytes when its name is not text. With it come the reason the report kept for it (if it kept one) and whether its failure left this run's copy.
- **Kept failures first.** The failures the report kept come first.
- **Then the paths none of them is.** Each failed path that is not one of them follows, with the bytes the account carries at its place (cr-ecm34-5).
- **How a path is matched.** A path is one of the kept failures by its identity when the account carries the paths' bytes, and by its text when it carries none, as from a peer that sent none.

Both writers use it:
- the run log's end pass, which still falls back to a name heard live for a file named without bytes;
- the CLI's saved record (`finish_record`).

## Guard proof
- `run_log::tests::two_failures_of_one_text_past_the_cap_are_two_files`: `dup\xff` with its reason and `dup\xfe` past the cap are both logged, each by its own bytes.
- `run_log::tests::a_failure_past_the_cap_is_saved_by_its_own_bytes` (CLI): the saved record names the failure past the cap by the summary's bytes for its path.
- The existing writer tests (`each_failure_is_logged_by_its_own_bytes`, the record's identity tests) still hold.
- Mutations, each restored byte for byte:
  - joined by the text alone → red;
  - the path's bytes not carried → red (both guards, and cr-ecm34-5's).
- Checks:
  - native clippy is clean;
  - the workspace passes on Linux: 2,094 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/run_log.rs` — `FailedFile`, `RunTotals::failed_files`; the end pass; guard.
- `crates/blit-cli/src/run_log.rs` — `finish_record` uses the join; guard.

## Known gaps
- What a file past the cap missing from a cut left-in-place list left is still logged as known: cr-ecm35-5.
