# otp-10b-1 — codex verdict adjudication

reviewer: gpt-5.6-sol (codex exec, read-only; raw output
`.review/results/otp-10b-1.codex.md`)
slice commit: `e82859e`
verdict: NEEDS FIXES — 5 findings (1 High, 3 Med, 1 Low)
adjudication: **5/5 accepted + fixed**
fix sha: (appended below after commit)

## F1 (High) — hash failure dropped the header ⇒ silent absence

**Accepted.** Verified: `ChecksummingSource` recorded the file
unreadable and DROPPED it; only the SOURCE end sees its own unreadable
list, so a daemon-as-SOURCE pull (no `require_complete_scan`) would
report success with the file silently absent — a byte-identity hole,
and a violation of this slice's own conservative rule (missing
checksum ⇒ transfer). Fixed: an unhashable file is EMITTED with an
empty checksum — the destination's missing-checksum arm transfers it
unconditionally, and a genuinely unreadable file then fails loudly at
payload time like any other read failure. Pinned by
`unhashable_files_are_emitted_with_empty_checksums` (stub source
whose `open_file` errors for one header: both headers emitted, the
hashable one with its real Blake3, the unhashable one empty).

## F2 (Med) — detached hashing task unowned by the session

**Accepted.** Verified: the forwarding task exits when its channel
closes, but a hash of one arbitrarily large file would run to
completion after teardown. Fixed: `hash_header_content` takes a stop
probe (`tx.is_closed()`) checked between 64 KiB chunks — residual
work after a session ends is bounded to one chunk; `Ok(None)` stops
the forwarding loop.

## F3 (Med) — destination hash chunk non-cancellable

**Accepted.** Verified: the diff's `spawn_blocking` chunk (up to 128
entries) is detached once its awaiting future drops; Checksum mode
made it hash-heavy. Fixed with the otp-9b F2 pattern, hoisted:
`AbortFlagOnDrop` is now a module-level guard shared by the mirror
pass and the diff chunk; the chunk checks the flag per entry and the
new `hash_file_abortable` (chunked sync Blake3) checks it per 64 KiB —
a dropped session stops the blocking work within one chunk. An abort
mid-hash propagates as an error (never decays to the empty-checksum
conservative-transfer arm).

## F4 (Med) — `CHECKSUM_DISABLED` missing from delegated phase map

**Accepted.** Verified: `session_error_phase` classified the new OPEN
refusal as `TRANSFER`. Fixed: added to the NEGOTIATE arm (it is
refused at OPEN, before any transfer work);
`session_error_phase_classifies_structurally` extended to pin it.

## F5 (Low) — STATE residue line contradicted the commit

**Accepted.** The queue's post-REV4 residue bullet still said
delegated Checksum compare degrades because the session destination
computes no hashes — exactly what this slice fixed. Rewritten as a
closed-gap note pointing at otp-10b-1.

## Guard proofs (fix round)

F1's pin fails against the pre-fix drop behavior by construction (the
test asserts both emission and the empty checksum; the pre-fix code
emitted one header). F4's pin fails on the pre-fix arm (Transfer ≠
Negotiate). F2/F3 are teardown-bounding changes whose stop probes are
exercised structurally (closed-channel/dropped-future paths); their
observable contract — no behavior change on live sessions — is held
by the whole checksum suite staying green.
