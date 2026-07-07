# otp-6a — codex review (raw findings)

**Reviewer**: gpt-5.5 via codex-cli 0.142.5, read-only sandbox, reasoning xhigh.
**Commit reviewed**: `c026692` (otp-6a: filters on the session).
**Prompt**: review the diff; check correctness regressions, filter applied
consistently whichever end initiates, malformed peer glob refused at OPEN and
peer-notified, no bytes on a refused handshake, initiator/verb invariance, test
count not dropped.

(Full reasoning/exec trace elided — durable content is the findings list and
verdict below. Adjudication in `otp-6a.gpt-verdict.md`.)

## Findings

- `crates/blit-core/src/transfer_session/mod.rs:936` — **Medium** —
  `SessionOpen.filter` is only passed through the `TransferSource::scan`
  argument, but that is not a universal chokepoint: `RemoteTransferSource::scan`
  ignores it and `FilteredSource::scan` ignores caller-provided filters. Any
  session using those source implementations can silently manifest/transfer
  unfiltered files, breaking the "SOURCE scan honors open.filter" and
  initiator/verb invariance claims. The new test covers only `FsTransferSource`.

## VERDICT

**FAIL** — core `FsTransferSource` push/pull behavior looks covered, invalid
pull-side peer globs are refused at OPEN, and the test attr count increased by 1,
but filtering is still implementation-dependent instead of guaranteed by the
session source path.
