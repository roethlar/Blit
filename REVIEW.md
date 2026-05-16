# Review status

See `.review/README.md` for the workflow contract.
See `.review/findings/<id>.md` for per-finding details.

## Legend

- `[ ]` Open — coder may pick up
- `[~]` In progress / pending review — sentinel in `.review/ready/`
- `[x]` Verified — verdict in `.review/results/<id>.verified.json`

## Currently pending review

| ID                  | Severity | Title                                                       | Status | Branch                       | Commit    |
|---------------------|----------|-------------------------------------------------------------|--------|------------------------------|-----------|
| _none_              |          |                                                             |        |                              |           |

## Phase 5 A.0 — `blit-app` library extraction

Linear refactor sequence on `phase5/blit-app-extract`; each
sub-slice committed in dependency order. History snapshot below
for visibility — these were graded conversationally before the
workflow bootstrap, no per-finding docs backfilled. From the
sentinel-system bootstrap commit forward, every new slice
follows the full contract.

| Slice                        | Commit    | Status         |
|------------------------------|-----------|----------------|
| Crate scaffold + endpoints   | `b5d2414` | `[x]` graded   |
| df / du / find / list-modules / rm | `4800cfc` | `[x]` graded   |
| ls (initial)                 | `009583c` | `[x]` graded   |
| ls — LocalListing enum fix   | `af436b2` | `[x]` graded   |
| scan                         | `39966df` | `[x]` graded   |
| profile                      | `d6ee06a` | `[x]` graded   |
| diagnostics (perf + dump)    | `334a684` | `[x]` graded   |
| diagnostics — perf best-effort fix | `2626f9b` | `[x]` graded   |
| check                        | `e807f46` | `[x]` graded   |
| util.rs split                | `44a4f8c` | `[x]` graded   |
| transfers/local              | `2a37a3e` | `[x]` graded   |
| transfers/local — doc fix    | `(this branch)` | `[x]` graded (folded into 8c4174a) |
| transfers/filter             | `8c4174a` | `[x]` graded   |
| transfers/resolution         | `3639159` | `[x]` graded   |
| transfers/resolution — followups | `65f6031` | `[x]` verified |
| transfers/remote — pull-flow helpers (R1) | `de78151` | `[x]` reopened → r2 |
| transfers/remote — pull-flow helpers (R2) | `086fa49` | `[x]` reopened → r3 |
| transfers/remote — pull-flow helpers (R3) | `2c9029e` | `[x]` reopened → r4 |
| **transfers/remote — pull-flow helpers (R4 workflow fix)** | **`b2d6c9c`** | **`[x]` verified** |
| transfers/remote — pull entry-point (R1) | `7f75539` | `[x]` reopened → r2 |
| transfers/remote — pull entry-point (R2) | `e9e168f` | `[x]` reopened → r3 |
| **transfers/remote — pull entry-point (R3)** | **`e25707e`** | **`[x]` verified** |
| **transfers/remote — push entry-point** | **`cb96331`** | **`[x]` verified** |
| **transfers/remote_remote_direct — delegated execution** | **`1879019`** | **`[x]` verified** |
| **transfers/dispatch — route selector** | **`8860cac`** | **`[x]` verified** |
| endpoints — remote-transfer support gates (R1) | `c392f37` | `[x]` reopened → r2 |
| **endpoints — remote-transfer support gates (R2)** | **`6eeb214`** | **`[x]` verified** |

## Phase 5 A.0 — remaining slices

These will get individual rows + finding docs + sentinels as they land.

| Slice                                          | Status |
|------------------------------------------------|--------|
| transfers/remote — pull-flow helpers           | `[x]` (a0-remote-helpers verified — 4 rounds) |
| transfers/remote — pull entry-point            | `[x]` (a0-pull-execution verified — 3 rounds) |
| transfers/remote — push entry-point            | `[x]` (a0-push-execution verified) |
| transfers/remote_remote_direct                 | `[x]` (a0-delegated-execution verified) |
| transfers/dispatch (`TransferKind` + route selector)             | `[x]` (a0-dispatch verified; `run_transfer`/`run_move` intentionally stay in CLI — see finding doc) |
| Endpoints clap-coupled gates → primitive inputs | `[x]` (a0-endpoints-gates verified — 2 rounds) |
| Final cleanup (drop CLI shim re-exports if any remain) | `[ ]` |

## Bigger Phase 5 milestones (planned, not yet started)

See `docs/plan/TUI_DESIGN.md` §8 for full scope.

| Milestone                         | Status |
|-----------------------------------|--------|
| B — `GetState` + `ActiveJobs` + recent ring | `[ ]` |
| M-Jobs — daemon-owned lifecycle + `CancelJob` + `detach` | `[ ]` |
| C — `Subscribe` + byte-level instrumentation | `[ ]` |
| A.1 — the TUI itself              | `[ ]`  |
| D — Verify + diagnostics screens  | `[ ]`  |
| E — polish                        | `[ ]`  |

## Open P0s (release-plan scope, separate from Phase 5)

See `docs/plan/RELEASE_PLAN_v2_2026-05-04.md` §2.6.

| Item                              | Status |
|-----------------------------------|--------|
| §2.6 Live remote benchmark capture | `[ ]` — hardware-bound, gated on tester availability |
