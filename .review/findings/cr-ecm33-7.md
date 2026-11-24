# cr-ecm33-7: the release note did not tell a wire resume from a local one

**Severity**: MEDIUM — brought in by cr-ecm32-6. The `CHANGELOG.md` entry said that with `--resume` a destination that is no regular file is replaced by a fresh copy. That holds for a local resume only. Over the wire, a FIFO at the destination gets an ordinary whole-file record: the file fails, and the FIFO is left.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm33-7`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r34` over `70efef8a..0f912b06` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r34.codex.md`. It completes cr-ecm32-6 (`4e223b0a`).

## Evidence
"Local `--resume` replaces a destination FIFO. In a wire session with resume enabled, a FIFO already present at comparison received an ordinary file transfer, remained a FIFO, and produced one file failure. The wire grants block resume only for a nonempty regular destination."

## Intake
Admitted as the reviewer put it. The words were wrong; the behaviour of each route is as the reviewer observed.

## What
The entry now says:
- an ordinary copy fails the file and leaves the FIFO or device as it is;
- so does `--resume` to or from a daemon, which sends such a file whole;
- a local `--resume` replaces a destination that is no regular file with a fresh copy (cr-ecm26-5).

## Guard proof
Docs only. Each claim rests on a guard or an observation:
- `sink::tests::a_fifo_at_a_destination_does_not_hold_the_copy`: the ordinary write fails the file and leaves the FIFO, and a streamed record's create refuses a FIFO. A wire resume's whole-file record takes that path.
- `resume::tests::a_fifo_at_the_name_is_replaced_not_opened`: a local resume replaces the FIFO.
- The reviewer observed that the wire grants block resume only for a nonempty regular destination. No guard of this repo pins that selection for a FIFO.
- `git diff --check` and `scripts/agent/check-docs.sh` pass.

## Files changed
- `CHANGELOG.md`.

## Known gaps
- The two routes still differ: a local resume replaces a destination FIFO, while a wire resume fails the file and leaves it. The note now states the difference. Whether the routes should agree is not this finding's to decide.
