# Contract-Version Gate — protocol number replaces same-build refusal

**Status**: Draft — direction RULED (D-2026-08-18-2); awaiting the
owner's Active flip, then per-slice gos.
**Created**: 2026-08-18
**Decision refs**: D-2026-08-18-2 (this plan's ruling; supersedes
D-2026-07-05-2's same-build gate), D-2026-08-17-2 (embeddability),
D-2026-08-18-1 (cargo channel — the wart this fixes).

## Ruling (owner, 2026-08-18)

"Yes, protocol version is the key." Two builds that speak the same
protocol version cooperate; the build fingerprint stops being a veto.
Additionally: "include protocol version information for discovered
daemons with blit scan and the future G/TUIs."

## Facts (verified 2026-08-18)

- The gate is one comparison: `transfer_session/mod.rs:895` refuses
  unless `peer_hello.build_id == hello.build_id` AND
  `peer_hello.contract_version == hello.contract_version`. Fault code
  `session_error::Code::BuildMismatch`, message cites D-2026-07-05-2.
- `CONTRACT_VERSION` is `transfer_session/mod.rs:92`, currently 6;
  the hello (`proto/blit.proto` ~989) carries `build_id` (string) +
  `contract_version` (u32, "Bumped on any wire-shape change; exact
  match required").
- mDNS advertisement (`blit-core/src/mdns.rs:139-154`) already
  publishes a TXT property map (`version` = crate version, modules,
  …); `MdnsDiscoveredService.properties` surfaces it on discovery.
- Consequence being fixed: every crates.io/cargo build gets a unique
  `unknown.<nonce>` identity (pm-1 fallback), so plain cargo installs
  refuse ALL peers, including each other (DEVLOG 2026-08-18 21:30Z);
  README documents a `BLIT_GIT_SHA` pin workaround.
- Interop arrives only when BOTH peers run post-cv-1 builds: existing
  released binaries veto on build_id forever. README's cargo caveat
  therefore cannot be softened until the first release carrying cv-1.

## Invariant after this plan

Session open refuses on `contract_version` mismatch only. `build_id`
remains in the hello, faults, and diagnostics as information. The
discipline this makes load-bearing (already the documented rule):
**any wire-shape or wire-behavior change bumps `CONTRACT_VERSION` in
the same commit.** A wire change without a bump is a defect class:
mismatched builds would mis-cooperate instead of refusing.

## Slices (each: own go, own commit, full gate, DEVLOG entry)

1. **cv-1 — the gate.** Drop the `build_id` half of the refusal;
   keep exact `contract_version` match. Keep the wire fault code
   `BuildMismatch` (tag compatibility; enum names are not on the
   wire) but reword the message to name the contract versions first,
   fingerprints second, citing D-2026-08-18-2. Tests: existing
   mismatch-injection tests flip meaning — new pins prove (a) same
   contract + different build_id OPENS (the new behavior, red under
   the old gate), (b) different contract still refuses, mutation-
   proved by reverting the gate line. Update the comment on
   `session_build_id` and the `SessionHello` doc. Grep for prose
   citing D-2026-07-05-2 in code comments; reword to the new rule.
   `docs/STATE.md`'s same-build lines update when this lands.
2. **cv-2 — surface the protocol version.** Daemon mDNS TXT gains
   `contract` = `CONTRACT_VERSION`; `blit scan` prints it per daemon
   (alongside the existing version column); `endpoint.rs`
   (`DaemonEndpoint`) carries an `Option<u32>` contract field
   populated by `discover.rs` from the TXT property so future
   TUIs/GUIs render it. Daemons predating cv-2 advertise nothing:
   display as unknown, never as a mismatch. No new CLI flags
   (SIMPLE).
3. **cv-3 — record + docs.** DECISIONS supersession edits already
   land with this plan; at the FIRST RELEASE containing cv-1, soften
   README's cargo same-build caveat to "peers need matching protocol
   versions (contract N)" and drop the pin recipe's necessity claim.
   Until then README stays as shipped.

## Non-goals

- No version negotiation, ranges, or back-compat shims: exact match
  on one integer, same as today, minus the fingerprint veto.
- No change to what bumps the contract (any wire change; judgment
  stays with the change author + review).
- No release; cv-1 ships whenever the next version does.

## Acceptance criteria

- [ ] Two builds differing only in build_id open a session (pinned,
      mutation-proved); differing contract still refuses with the
      reworded fault.
- [ ] `blit scan` shows each discovered daemon's protocol version;
      absent advertisement renders as unknown.
- [ ] No user-facing CLI flag added; full gate green per slice
      (fmt, clippy native + linux-cross at `-D warnings`, workspace
      tests, check-docs); test count never drops.
- [ ] Cross-platform: cv-1's pins live in ungated modules; CI green
      on all three OS legs before any Shipped claim.
