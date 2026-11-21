# cr-uv3-7: `--json` gave two names as one string

**Severity**: MEDIUM — what a script reads from `find --json`, `du --json` or `rm --json` does not say which entry is meant
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-uv3-7`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `utility-slices-r3` over `eb0cc4d8..dd0a3179` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/utility-slices-r3.codex.md`. It completes cr-uv2-5.

## Evidence
"I observed `find --json` return the same path twice for two distinct names, one containing byte `FF` and the other containing the literal replacement character. `rm --json` likewise produced indistinguishable failure paths. Keep the display string and supply an exact byte representation when conversion loses information."

Cause. cr-uv2-5 made what these commands PRINT lose nothing of a name, and left `--json` as it was — "`--json` holds text: there such a name is still the text nearest it" (CHANGELOG). JSON holds only text; nothing was put beside it.

## Intake
Admitted as the reviewer put it.

## What
A name that is not text has its bytes beside its text; a name that is text has nothing added, so output for ordinary names is what it was but for one new key in `find`'s.

- **The form**: `raw` — printable ASCII as itself, every other byte and `\` as `\xNN` (`raw_name::escape_raw`, the form a job's log already gives a name that is not text; `unescape_raw` reads it back). It is the whole of what `path` is the text of: the same bytes the command prints without `--json`, and so an operand these commands take.
- `du --json`: a row, or a failure, whose path is not text has `raw` beside `path`. `rm --json`: a failure likewise.
- `find --json`: `paths` stays a list of strings. `raw_paths` — new, always there — lists each path that is not text as `{"index": N, "raw": "…"}`, `N` its place in `paths`. A failure has `raw` beside `path`.
- `df --json` names no path.

## Guard proof
- `rm.rs::json_tells_a_name_that_is_not_text_from_the_text_shown_for_it` (Unix), through the binary: `rm --json` of two operands that are not there — `bad\xff.dat` and the name with U+FFFD in that place — two failures with one `path`, the first with `raw` that reads back to the operand's bytes, the second without; then, where the filesystem takes such names, `find --json` — one string twice, one entry of `raw_paths`, whose bytes are the line `find` prints at that place — and `du --json` — two rows that read alike, one with `raw`.
- `admin::quote::json_tests::a_name_that_is_not_text_has_its_bytes_beside_it`: five names that are text, four that are not (a `\` in one), each read back.
- Mutations, each restored byte for byte: no name given its bytes; the bytes those of the text; `find` listing no path that is not text, or naming the path after it; `du` giving a row, `rm` a failure, its text alone — six, each red on the guards named for it.

## Files changed
- `crates/blit-core/src/admin/quote.rs` — `in_json`, `exact`; guard.
- `crates/blit-cli/src/{find,du,rm}.rs`; `crates/blit-cli/tests/rm.rs` — guard.
- `CHANGELOG.md`.

## Known gaps
- **`find --json` has a new key, `raw_paths`**, in every output. A reader that refuses keys it does not know will refuse it.
- A failure of `find` or `du` on a name that is not text has no end-to-end guard of its own; it goes through the same function as the rows.
- Run on Linux only.
