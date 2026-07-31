# cr-clp2-4 — DECLINED at intake (coder), 2026-07-31

Finding: "The perf-history change alters and can suppress sink-less
diagnostics" (LOW). Evidence: `transfer_session/local.rs:775-779` moved a
verbose-gated ad-hoc `eprintln!` onto `log::warn!`, which adds the
`blit: warn:` prefix and honors `BLIT_LOG` filtering.

Declined because the change is the intended convention, not a defect:
that print was the last ad-hoc raw-stderr writer on the local path
(swept in the clp-2 review); every other diagnostic in the binary rides
the facade with exactly these prefix/filter semantics; the `-v` gate is
preserved; no test or documented contract pins the old bytes. The
plan's "sink-less byte-identical" constraint governs the redirect seam
(no sink installed ⇒ the backend emits the same bytes it always has —
which holds), not the migration of an ad-hoc print onto the facade.
The proposed raw-bytes helper would reintroduce an unroutable,
unsanitized stderr writer — the class clp-2 exists to eliminate.

Recorded per the codereview playbook; reviewer verdict stands as
received in `.review/results/clp-2-range.codex.json`.
