# cr-ecm18-2: a resume partial's working mode

**Severity**: HIGH — a reader who opens a fresh partial before it is narrowed keeps its bytes; a read-only source's partial cannot be written by the retry
**Status**: Reviewed, found incomplete — `combined-r20` (Codex openreview, 2026-11-22) reviewed `17f4eb97..c15ebf5a` — the six `combined-r19` fixes — with `guard_confirmed: true`. "I ran `copy --resume` with a three-byte source and a writable `0644` destination in a `0555` directory. Replacement failed with `Permission denied` and exit 2, yet cleanup truncated the existing 29-byte file to `prev`. Mark an acquired file through its handle. An acquisition failure must preserve the existing contents." And: "The wire block writer left a `0644` destination at `0644` after patching it. A fresh local partial under umask `0200` was `0400`. The local symlink arm also bypasses the working-mode check, and a previously supported dangling destination link now fails with `ENOENT`. Select the reusable file before computing block hashes, establish its working permissions through the handle, and preserve contained-link behavior." The remaining work is cr-ecm19-1 and cr-ecm19-4. Seen in intake: replacing every existing partial others could read also replaced every ordinary destination, which `--resume` exists to patch in place
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `7413b6c6`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r19` over `ca377805..17f4eb97` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r19.codex.md`. It completes cr-ecm17-3 (`f2bbdb96`).

## Evidence
"A reader that opens a fresh partial under its initial default mode keeps access after `chmod`. I also interrupted a resume from a `0444` source. The partial became `0444`, and the retry failed with `Permission denied`. Give a resume partial a private, owner-writable working mode from creation … Apply the final source mode on success and decline reuse of a broader existing partial when its access cannot be secured."

cr-ecm17-3 narrowed the partial to its source's bits after opening it: a reader who opened it in between keeps what it could open it for; and a read-only source made a read-only partial.

## Intake
Admitted as the reviewer put it.

## What
`resume_copy_from` (Unix) opens its partial by `open_partial`:
- none there: made with `create_new` and mode 600 — private and writable by its owner from the moment it exists;
- a regular file there that is mode 600 at most and writable by its owner: resumed (the same file, checked by device and number after opening);
- any other regular file there — one others could read (a reader may hold it already; no mode change takes that back), or one this run cannot write: removed, and a fresh private one made; nothing of it is reused;
- a destination that is not a regular file (a link): opened as it always was.
The partial stays 600 while it is worked on; the source's own mode is given when the copy completes (cr-ecm16-1). cr-ecm17-3's narrowing is gone.

## Guard proof
- `resume::tests::a_partial_is_private_and_writable_while_it_is_worked_on`: sources of mode 644 and 444, interrupted — the partial is 600; the retry writes it and it has its source's mode at the end.
- `resume::tests::a_partial_is_reused_only_if_it_is_private`: a partial of mode 644 with a reader holding it open is replaced — the reader keeps what it held, nothing is skipped, another file stands at the name; one of mode 600 is resumed — the same file, blocks skipped.
- `an_interrupted_resume_leaves_nothing_readable_beyond_its_source` (cr-ecm17-3's) and `a_resumed_copy_has_its_sources_mode` (cr-ecm16-1's) pass; the older resume tests make their partials as a resume does (mode 600), since that is the kind reused.
- Mutations, each restored byte for byte: a partial made with the usual mask (red, and cr-ecm17-3's guard); any partial reused (red); none reused (red, and two older resume tests).
- The whole suite in the scratch copy: 1,953 passed, 0 failed.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `open_partial`; guards; older tests' partials.
- `CHANGELOG.md`.

## Known gaps
- A partial left by an earlier version of blit (made with the usual mask) is not resumed: it is replaced, and copied whole once.
- A destination that is a link is resumed through it as before, without these rules.
- The partial of a resume over the wire (the sink's block-hash resume, `hold_resume_partial`) is made by other code and is not changed here; not raised.
- Unix only.
