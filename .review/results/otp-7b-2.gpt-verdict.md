# otp-7b-2 — codex verdict adjudication

Reviewer: gpt-5.6-sol (codex exec, read-only, ultra effort). Raw
output: `otp-7b-2.codex.md`. Codex VERDICT: NEEDS FIXES (4 findings).
Codex explicitly cleared: `tag_path` SessionFault handling, contract
v2 / wire tag 5, the flush fix's sufficiency, the other tokio sink
write paths, cancellation, DECISIONS coherence, test count 1545→1548.

- **G1 (Medium) — "" is a valid single-file-root identity but decoded
  as absence: ACCEPTED, fixed.** `SessionError.relative_path` is now
  proto3 `optional` (explicit presence; same tag 5), `from_wire`/
  `to_wire` carry the Option verbatim, and the summary renders the
  empty identity as `<the transfer root file>` instead of a blank
  name. Unit-pinned (empty-path wire round trip + non-blank render).
  CONTRACT_VERSION stays 2 — v2 has never crossed a wire outside this
  session's local commits.
- **G2 (Medium) — in-stream resume record only partly tagged:
  ACCEPTED, fixed.** `ResumeBlockDiff::open` now tags the source-open
  failure (covers both carriers), and the in-stream call site wraps
  the whole record with `tag_path`, matching the data-plane carrier's
  outer wrap.
- **G3 (Medium) — receive-side record-scoped failures incompletely
  tagged: ACCEPTED, fixed.** Data-plane BLOCK offset/length reads and
  BLOCK_COMPLETE metadata reads now tag with the already-known path;
  the in-stream `transport.recv()` inside file and block records tags
  with the open record's path.
- **G4 (Low) — contract doc omits `relative_path`: ACCEPTED, fixed.**
  `docs/TRANSFER_SESSION.md` §Errors now documents the optional field
  and its single-file-root presence rationale.

Fix sha: (appended after the fix commit) — see below.
