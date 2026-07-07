# otp-5b-2 — GPT review adjudication

reviewer: gpt-5.5 (codex-cli 0.142.5, reasoning xhigh)
reviewed commit: d579365
verdict file for: otp-5b-2 pull data-plane resize
codex VERDICT: NEEDS FIXES (1 Low finding)

## F1 (Low) — DEST initiator resize ceiling reads a fresh `local_receiver_capacity()` instead of the advertised profile — **ACCEPTED**

`crates/blit-core/src/transfer_session/mod.rs:1744`

**Claim**: the DESTINATION initiator's `resize_ceiling` is taken from a fresh
`local_receiver_capacity()` call rather than the `receiver_capacity` it
advertised in its `SessionOpen`, so the receiver-side defense-in-depth can
over-accept/dial beyond an explicitly lower advertised `max_streams`.

**Verified against source**: real. `run_destination` (mod.rs:1468) only fills
`open.receiver_capacity` when the caller left it `None` — a caller may advertise
an explicit profile with a lower `max_streams`. The SOURCE responder's dial IS
bounded by that advertised value (`accept_source_data_plane` receives
`negotiated.open.receiver_capacity`), but the DEST ceiling read a fresh local
value, so the two ends could disagree on the ceiling. In the only supported
configuration (same-build, D-2026-07-05-2) the SOURCE clamps to the advertised
value and never proposes beyond it, so there is no observable failure on the
happy path — hence Low — but the defense-in-depth ceiling should equal what this
end advertised (and what the SOURCE clamps to). The existing code comment already
claimed "this end's OWN advertised capacity"; the code did not match it.

**Fix** (`d579365`→follow-up): the ceiling now reads
`negotiated.open.receiver_capacity.max_streams` (fallback 0→1), the exact profile
advertised and the exact bound the SOURCE responder's dial uses. Code now matches
its comment; both ends agree on the ceiling. Guard proof re-run (ceiling forced to
0 ⇒ pull shape test fails "settled at 1"); gate re-run green.

Note (not a defect): the PUSH DEST responder computes its ceiling from
`local_receiver_capacity()` too, but its advertised profile is ALSO computed fresh
in `responder_finish` (no caller-override path on the responder side), so the two
never diverge there. The divergence is specific to the initiator, whose open can
carry a caller-provided capacity. Push left unchanged (out of slice; no
divergence exists).

## Fix commit

`773a877` — otp-5b-2: address review (1 finding).
