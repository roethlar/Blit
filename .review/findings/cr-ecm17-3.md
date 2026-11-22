# cr-ecm17-3: a resume that failed part-way left private bytes readable by all

**Severity**: HIGH — with `--resume`, a copy of a private file that fails part-way leaves part of it in a partial of mode 644
**Status**: Reviewed, found incomplete — `combined-r19` (Codex openreview, 2026-11-22) reviewed `ca377805..17f4eb97` — the eight `combined-r18` fixes — with `guard_confirmed: true`. "A reader that opens a fresh partial under its initial default mode keeps access after `chmod`. I also interrupted a resume from a `0444` source. The partial became `0444`, and the retry failed with `Permission denied`. Apply the final source mode on success and decline reuse of a broader existing partial when its access cannot be secured." The remaining work is cr-ecm18-2
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `f2bbdb96`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r18` over `7912da16..ca377805` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r18.codex.md`. It completes cr-ecm16-1 (`cdfc2b95`).

## Evidence
"After a short source read, I found 1 KiB from a `0600` source in a `0644` partial, both at a fresh destination and in an existing partial. Create restrictively and narrow an existing partial through its handle before writing. Surface a failure if the required protection cannot be applied."

cr-ecm16-1 gives a resumed copy its source's mode when the copy COMPLETES. A resume keeps its partial when it fails (that is what makes it resumable), and the partial had the mode it was made with — the usual mask's — or one a partial left before had.

## Intake
Admitted. One mechanism instead of the two the reviewer names: the partial, made now or there already, is narrowed through its handle before a byte is written, so creating it restrictively as well would change nothing a test could see (tried: a mutation of the creation mode alone went unnoticed) — the narrowing covers a fresh partial too.

## What
`resume_copy_from` (Unix): once the destination is open, and before anything is written to it, its mode is narrowed to the bits its source allows (`now & source`), through its own handle. If that fails, the copy fails and nothing is written. When the copy completes, the source's mode is given, as before.

## Guard proof
- `copy::file_copy::resume::tests::an_interrupted_resume_leaves_nothing_readable_beyond_its_source`: a source of mode 600 that claims more than it holds, so the read fails after blocks of it have landed — into a fresh destination and into a partial of mode 644 left before; afterwards part of the source is in each, and neither is readable by anyone but its owner.
- Mutation, restored byte for byte: the partial not narrowed (red).
- `a_resumed_copy_has_its_sources_mode` and `local_copy_mode` pass.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — the narrowing; guard.

## Known gaps
- A partial made before this fix by a resume that failed keeps its mode until a resume touches it again.
- A non-resume copy is not touched: it creates its destination with the usual mask, writes, and gives the mode at the end; on failure it removes the destination (cr-ssc4-2), so nothing of it stays — but while it is being written it is readable by all. Not raised; recorded for the owner.
- Unix only.
