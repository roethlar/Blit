# otp-10b-1 — checksum compare on the session (contract v3)

**What**: First otp-10b (pull-shaped verb) sub-slice, staged like
9a/9b: before the pull verb can cut over, the session must support
the one compare capability the old pull has that the session lacked —
`--checksum` content compare. The session's `COMPARISON_MODE_CHECKSUM`
previously degraded to transfer-everything (no end computed hashes;
`compare_file`'s conservative arm). Now it is a real content compare,
role-agnostic: whichever end holds SOURCE fills each manifest header's
Blake3, whichever holds DESTINATION hashes its same-size candidates
during the diff, so a content-equal file SKIPS regardless of mtime and
a content-differing same-size+mtime file transfers. A daemon whose
operator disabled hashing (`--no-server-checksums`) refuses a Checksum
open with the new `CHECKSUM_DISABLED` code — the session never
silently degrades a content-compare request (the old pull's F11 ack
refusal, reborn as an OPEN refusal).

**Approach**:

- **Wire (contract v2 → v3)**: `SessionError.Code::CHECKSUM_DISABLED
  = 10` (proto + `docs/TRANSFER_SESSION.md` §Errors + new §Checksum
  compare). No new fields — `FileHeader.checksum` existed, it is now
  populated on Checksum sessions. Same-build fleets make the bump
  free (D-2026-07-05-2).
- **SOURCE**: new `ChecksummingSource` decorator
  (`remote/transfer/source.rs`) — fills `header.checksum` by hashing
  through the inner source's own `open_file` (source-impl-agnostic,
  the `FilteredSource` chokepoint reasoning; composed OUTSIDE the
  filter so only in-scope files pay). A file that cannot be hashed is
  recorded unreadable and dropped — it could not have transferred
  either; `scan_complete` turns false exactly like a scan-time
  failure. `source_send_half` wraps when
  `open.compare_mode == Checksum`.
- **DESTINATION**: `destination_needs` hashes the local candidate —
  only when the mode is Checksum AND the sizes match (a size mismatch
  is already Modified), inside the diff's existing blocking-pool
  chunk (the resume-hash precedent). Hash failure degrades to the
  empty checksum ⇒ conservative transfer, never a false skip.
- **Policy**: the otp-10a F3 `force_in_stream` bool became
  `ResponderPolicy { force_in_stream, refuse_checksum_compare }`
  (both flags are the same shape of operator config), threaded
  `core.rs` → `run_transfer_session` → `run_responder` →
  `responder_finish`, which refuses a Checksum open BEFORE accept
  with an operator-facing message naming the knob.

**Files**:

- `proto/blit.proto` — `CHECKSUM_DISABLED = 10`.
- `crates/blit-core/src/transfer_session/mod.rs` — CONTRACT_VERSION 3,
  `ResponderPolicy`, OPEN refusal, source wrap, destination hashing.
- `crates/blit-core/src/remote/transfer/source.rs` —
  `ChecksummingSource` + `hash_header_content`.
- `crates/blit-daemon/src/service/{core,transfer}.rs` — policy from
  runtime config (`server_checksums_enabled`).
- `docs/TRANSFER_SESSION.md` — contract v3 sections.
- Tests: role suite + daemon e2e below.

**Tests** (suite 1576 → 1580):

- Role suite, BOTH initiator layouts, each with its SizeMtime control
  proving non-vacuity:
  - `checksum_compare_skips_content_equal_files_regardless_of_mtime`
    (control: SizeMtime transfers the mtime-differing file).
  - `checksum_compare_transfers_content_change_size_mtime_misses`
    (control: SizeMtime skips the stealth change; Checksum lands it
    byte-identically) — the cell `--checksum` exists for.
- Daemon e2e:
  - `checksum_push_skips_content_equal_dest_over_served_session` —
    the served DESTINATION hashes its own candidates.
  - `checksum_open_refused_when_daemon_disables_checksums` — BOTH
    roles refused with `CHECKSUM_DISABLED`; message names
    checksum + disabled (the CLI e2e refusal-grep shape).
- Guard proofs by temporary mutation, run live: (K) drop the source
  wrap → skip pin fails; (L) drop the destination hashing → skip pin
  fails; (M) drop the refusal → refusal e2e fails. All restored.

**Known gaps**:

- The verb is NOT cut over yet — old pull still serves `--checksum`
  via the F11 ack flow until otp-10b-2, which maps CLI compare flags
  onto the open for BOTH verbs (and lifts push's `--checksum` gate).
- `ChecksummingSource` hashes inline with the (I/O-bound) read on the
  scan forwarding task — adequate for Blake3; a parallel-hash
  optimization (old pull used rayon on the client manifest) is
  available if a checksum-mode benchmark cell ever warrants it.
- A relay source under Checksum would hash by reading the whole file
  over PullSync before pushing it — legal but wasteful; push's
  `--checksum` gate still stands until 10b-2 decides the mapping, and
  relay's fate is 10c.
- The old pull hashed BOTH ends up front (client manifest + daemon
  scan); the session hashes the destination side only for same-size
  candidates — strictly less work, same verdicts.
