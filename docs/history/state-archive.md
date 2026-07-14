# STATE archive

Landed or superseded `## Now` entries rotated verbatim out of `docs/STATE.md`
by the `drift` operator (AGENTS.md, Operator Requests), so that STATE stays
under its 200-line cap and describes only what is *still* true.

**This file is history, not state.** Nothing here is authoritative for current
work — `docs/STATE.md` is. Entries are newest-first, each stamped with the
commit at which it was rotated out.

---

## Rotated 2026-07-14 (at `7fc48d3`) — ONE_TRANSFER_PATH: the closed-slice record

Landed: every slice otp-1..otp-11 was `[x]` when this was rotated. The parent
plan `docs/plan/ONE_TRANSFER_PATH.md` remains **ACTIVE**, so its invariant and
its open slices stay in `docs/STATE.md`; only the closed-slice detail moved
here.

Verbatim, as it stood in `docs/STATE.md`:

> - **ONE_TRANSFER_PATH ACTIVE (D-2026-07-05-1 directive,
>   D-2026-07-05-4 "flip the plan and go") — otp-4a landed.** The
>   invariant (plan doc, verbatim): ONE block of transfer code;
>   direction/initiator/verb can NEVER affect wall time by blit's doing
>   — impossible by construction because the per-direction drivers and
>   `Push`/`PullSync` are deleted at cutover. Slices otp-1..13;
>   converge-up per cell (±10%); symmetric-fs disk-to-disk verdict
>   cells. **D-2026-07-05-2: same-build peers only, refusal at session
>   open.** Progress (each slice through the codex loop; per-slice
>   detail lives in DEVLOG + `.review/`, NOT here):
>   - **Closed `[x]`: otp-1 … otp-11** — the whole session machine, the
>     baselines (otp-2/2w), the **CUTOVER DELETION** (4 drivers +
>     `Push`/`PullSync` + 13 messages out of tree AND proto, −13.8k lines,
>     no bridge; relay removed D-2026-07-11-1), and **otp-11b's deletion of
>     the entire old orchestration** (−6.2k lines: orchestrator, engine,
>     local_worker, auto_tune, change_journal — the last an UNSOUND fast
>     path that silently lost data). The deletion-proof acceptance line
>     COMPLETES. Detail: DEVLOG 2026-07-10/11/12; evidence
>     `docs/bench/otp2{,w}-baseline-2026-07-10/`, `otp11-local-2026-07-11/`.
