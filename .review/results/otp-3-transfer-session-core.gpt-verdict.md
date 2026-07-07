# otp-3 — codex review adjudication

**Commit reviewed**: `ef9ffa1`
**Raw output**: `.review/results/otp-3-transfer-session-core.codex.md`
**reviewer: gpt-5.5** (codex exec, read-only, superpowers plugin disabled)
**Codex verdict**: FAIL — 2 findings (1 High, 1 Medium)

## F1 — build.rs: same-build identity not actually guaranteed (High)

**Claim**: (a) with git unavailable every build collapses to
`0.1.0+unknown`, so two different builds exact-match; (b) only
`.git/HEAD`/`.git/refs` are watched, so a source edit rebuilds the
crate without re-running the build script — a dirty build can carry a
stale clean sha. Contradicts D-2026-07-05-2.

**Adjudication: Accepted.** Both prongs verified against
`crates/blit-core/build.rs` as committed: no `rerun-if-changed` on
any source path, and the `unknown` fallback is a shared constant.
The handshake is the only compatibility gate (no negotiation exists,
by design), so identity precision is load-bearing.

**Fix**: imprecise identities are made non-collapsing —
- git unavailable → `unknown.<per-compilation entropy>`: two
  independent compilations can never false-match; a single binary
  deployed to both ends still matches itself.
- dirty tree → `<sha>.dirty.<content hash>`: the nonce is a
  deterministic hash of `git status --porcelain -z` + `git diff
  HEAD`, so byte-identical dirty trees (and no-op rebuilds) keep a
  stable id while any content difference changes it.
- Re-trigger coverage: `.git/HEAD`, `.git/refs`, **`.git/index`**
  (add/commit/checkout/stash), **each currently-dirty path**, and
  **blit-core's own `src/` tree + `proto/`** (the wire-owning
  sources) now re-run the script.
- Residual window, documented in build.rs: a first edit to a
  previously-clean file *outside* blit-core/proto, with no git
  operation, keeps the last sampled identity until the next script
  trigger. Watching the whole workspace would close it at the cost
  of a full-workspace recompile on every edit of any crate;
  deliberately not taken — flagged here for the owner. The
  contract's stated precision (commit hash + dirty state) §Invariants
  2 is annotated with the extended imprecise-identity forms.
- No unit test ships for this finding: the behavior lives in the
  build script (outside the compiled crate); the existing
  `build_id_has_version_and_git_components` pins the composed shape.

## F2 — mod.rs: early `NeedComplete` accepted silently (Medium)

**Claim**: the SOURCE receive half accepts `NeedComplete`
unconditionally; per `docs/TRANSFER_SESSION.md` it may only be sent
after the source's `ManifestComplete` has been received, so a
misordered peer should be failed fast.

**Adjudication: Accepted.** Verified: `handle_source_event` set
`need_complete = true` with no phase awareness, so a peer sending
NeedComplete straight after ACCEPT ended the session as an empty
transfer instead of a PROTOCOL_VIOLATION.

**Fix**: the send half publishes "ManifestComplete sent" via an
`AtomicBool` shared with the receive half; a `NeedComplete` arriving
while it is false is provably premature (ordered transport: the peer
cannot have received what we have not sent) →
`SessionError{PROTOCOL_VIOLATION}` + abort. New test
`need_complete_before_manifest_complete_faults_the_source` (500-entry
manifest so the send half is provably still mid-manifest when the
scripted peer's early NeedComplete is processed; asserts the source
faults with PROTOCOL_VIOLATION and the peer observes the error frame
before any ManifestComplete). Guard proven: with the AtomicBool check
reverted the new test fails (source completes with SourceDone);
restored, suite green.

**Fix sha**: `d5796a1` (both findings; gate re-run: fmt + clippy
clean, workspace suite 1501 passed / 0 failed).
