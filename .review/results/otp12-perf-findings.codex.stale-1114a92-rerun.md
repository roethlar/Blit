Reading additional input from stdin...
OpenAI Codex v0.144.1
--------
workdir: /Users/michael/Dev/blit_v2
model: gpt-5.6-sol
provider: openai
approval: never
sandbox: read-only
reasoning effort: ultra
reasoning summaries: none
session id: 019f580b-0aee-7b52-863b-aa8b7ae1d302
--------
user
Review docs/plan/OTP12_PERF_FINDINGS.md (Draft, commit 1114a92) - an investigation+fix plan for two recorded perf findings, written BEFORE any code investigation. Its own Known gaps section says the hypotheses cite recorded mechanisms, not verified code paths, and asks a reviewer to grade them against the actual tree. Do exactly that, plus the usual design checks: (a) H1-H4 vs the code: read the TCP data-plane socket acquisition for both initiator layouts (dial vs accept per resize epoch), the shape-correction/resize controller, the need-list/tar-shard planner cadence, and the destination receive path (per-file open/write/flush/instrument costs) - for each hypothesis say SUPPORTED / CONTRADICTED / UNVERIFIABLE-FROM-CODE with file:line evidence; flag any obvious 5th mechanism the plan missed. (b) Evidence fidelity: the P1/P2 numbers vs the committed records (docs/bench/otp12-win-2026-07-12/README.md, docs/bench/otp12-zoey-2026-07-12/, docs/bench/otp2w-baseline-2026-07-10/summary.csv) - including the claim that zoey's pull_tcp_mixed PASSed 0.966 and the grpc-at-parity claims. (c) Method: will the pf-1 local two-layout phase-timing reproduction actually discriminate H1-H4? Is anything unfalsifiable as staged? Is the P2 bisect (precreate / flush-toggle / ramp-pin experiment flags) sound given the old drivers are deleted? (d) Fix criteria: are the pre-registered bars (wm_tcp_mixed <=1.10, pull_tcp_mixed <=1.10 same-session, push_tcp_small <=1.10 both rigs, unregressed grpc + suite floor 1483) complete - what regression could pass them while violating the parent plan's converge-up constraint? (e) Scope: no wire changes claimed - is that credible for each hypothesis' likely fix (esp. H1 epoch accept/dial and H4 ramp)? (f) Staging: is pf-1-before-any-fix enforced by the doc's wording, and are the CELLS escalation re-runs (not full matrices) sufficient rig evidence for the owner's otp-13 walk? Output a concise markdown findings list - each finding with file:line, severity, rationale - then a final VERDICT line. Be concise; do not invoke skills.
codex
I’ll grade the draft against the committed tree and benchmark artifacts, separating code support, evidence fidelity, experimental identifiability, acceptance gaps, scope, and staging. I’ll keep the output to review findings plus a verdict; no skills invoked.
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
2026-07-12T20:44:16.447960Z ERROR codex_core::tools::router: error=timeout_ms must be at least 10000
codex
The benchmark figures mostly transcribe correctly, but two interpretations do not: Zoey’s 0.966 is old-vs-new converge-up, not evidence that P1’s layout asymmetry is absent; and Windows gRPC small push is 0.801/0.835 versus old, not “0.98-ish parity.” I’m finishing the code-path and acceptance-gate grading now.
collab: Wait
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
collab: Wait
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
collab: Wait
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
mcp: ptk/ptk_invoke started
mcp: ptk/ptk_invoke (completed)
codex
### Findings

- **INFO — H1: SUPPORTED structurally; causality unverified** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:42-52`. Push sources dial while pull sources accept; pull resize synchronously dials before ACK, then the source accepts afterward ([data_plane.rs:640](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/data_plane.rs:640), [mod.rs:3110](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:3110)). Epochs are serialized one stream at a time ([dial.rs:349](/Users/michael/Dev/blit_v2/crates/blit-core/src/dial.rs:349)). However, small, large, and mixed all target eight streams ([dial.rs:934](/Users/michael/Dev/blit_v2/crates/blit-core/src/dial.rs:934)), so H1 alone does not explain mixed-only selectivity.

- **HIGH — H2: CONTRADICTED** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:53-58`. Both layouts use the same source/destination drivers, fixed 128-entry diff chunks, and NeedBatch emission ([mod.rs:2265](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:2265), [mod.rs:2809](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:2809), [mod.rs:3437](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:3437)). Resize/planning starts only after `ManifestComplete` ([mod.rs:1385](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:1385)); the alleged initiator-dependent need-emission/resize interleave cannot occur.

- **MEDIUM — H3: SUPPORTED, but its named candidates are wrong** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:59-65`. Tiny files are tar-sharded; their sink does parallel `create_dir_all` plus `fs::write`, with no explicit per-file flush ([transfer_plan.rs:98](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_plan.rs:98), [sink.rs:592](/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/sink.rs:592)). Old push used the same sink and served-push destination progress is disabled, so flush/directory/progress are not old/new deltas. The actual new TCP-only per-file cost is `NeedListSink` locking/removing the outstanding `HashSet` for every shard member ([data_plane.rs:1030](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/data_plane.rs:1030), [data_plane.rs:1174](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/data_plane.rs:1174)); in-stream claims a shard under one lock ([mod.rs:3047](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:3047)).

- **HIGH — H4: CONTRADICTED as written** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:66-70`. The TCP record implementation is byte-for-byte unchanged from `0f922de`, and old push did not have a fixed-count opening: it could open under-shaped from the early manifest prefix and then resize one ADD at a time (`0f922de:crates/blit-core/src/remote/push/client/mod.rs:517-550`). Only shard-boundary/ramp cadence remains plausible because old TCP planned each early batch while current code accumulates until `ManifestComplete`.

- **HIGH — obvious H5 omitted: lost scan/diff/TCP-transfer overlap** — [mod.rs:1385](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:1385). Old TCP intentionally negotiated and queued payloads on mid-manifest need batches (`0f922de:crates/blit-daemon/src/service/push/control.rs:227-324`; `0f922de:crates/blit-core/src/remote/push/client/mod.rs:863-940`), while old forced-gRPC already waited. Current code makes both carriers wait for `ManifestComplete`, exactly predicting “TCP regressed, gRPC did not.”

- **HIGH — Zoey’s 0.966 is misclassified as P1 absence evidence** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:26-28,127-129`. It is new-vs-old converge-up, not a two-layout invariance measurement ([verdicts.csv:14](/Users/michael/Dev/blit_v2/docs/bench/otp12-zoey-2026-07-12/verdicts.csv:14)); the rig record explicitly assigns invariance to rig W ([README.md:3](/Users/michael/Dev/blit_v2/docs/bench/otp12-zoey-2026-07-12/README.md:3)). It cannot show P1 was absent or masked on Zoey.

- **MEDIUM — evidence wording otherwise overclaims, though headline TCP figures match** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:20-38`. P1’s 1.237/1.313 and carrier/fixture ratios, P2’s 1.105/1.149, and committed 1284/1868 references are faithful ([win README.md:44](/Users/michael/Dev/blit_v2/docs/bench/otp12-win-2026-07-12/README.md:44), [Zoey README.md:68](/Users/michael/Dev/blit_v2/docs/bench/otp12-zoey-2026-07-12/README.md:68), [summary.csv:6](/Users/michael/Dev/blit_v2/docs/bench/otp2w-baseline-2026-07-10/summary.csv:6)). But Windows `push_grpc_small` is 0.801 same-session/0.835 committed—not “0.98-ish parity” ([verdicts.csv:26](/Users/michael/Dev/blit_v2/docs/bench/otp12-win-2026-07-12/verdicts.csv:26)). The 1.313 comparison is the same rig/session, not independent, and the 2080-vs-1922 precreation lead is explicitly recorded rather than attributed ([win README.md:92](/Users/michael/Dev/blit_v2/docs/bench/otp12-win-2026-07-12/README.md:92)).

- **HIGH — pf-1 cannot discriminate the hypotheses as staged** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:74-84`. The referenced otp-3 role suite forces in-stream transport and uses `in_process_pair` ([transfer_session_roles.rs:92](/Users/michael/Dev/blit_v2/crates/blit-core/tests/transfer_session_roles.rs:92), [transfer_session_roles.rs:151](/Users/michael/Dev/blit_v2/crates/blit-core/tests/transfer_session_roles.rs:151)). It cannot time H1/H4 TCP behavior; even a TCP loopback variant cannot falsify a Windows↔Mac topology-dependent H1 from a negative local result. H3’s per-member lock and H5’s overlap also lack registered probes/kill criteria.

- **HIGH — the P2 “bisect” is unsound** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:85-90`. The flush toggle misses the tar path, precreation measures an environmental component rather than old/new attribution, and the ramp pin reproduces behavior old push already had. The deleted driver remains directly inspectable and runnable at `0f922de`; pf-1 needs an instrumented historical old-vs-new control, plus timings for `NeedListSink` claims and manifest-to-first-TCP-payload overlap.

- **HIGH — fix bars are weaker than the parent constraint** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:97-104`. D2 requires every arm independently against both same-session and committed references and forbids using invariance as a substitute ceiling ([OTP12_ACCEPTANCE_RUN.md:145](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:145)). Otherwise two 1.10 tolerances permit 1.21×. Concretely, Windows `push_tcp_small` could pass the draft at ≤1.10×1811 ≈1992 ms while remaining far slower than the better old direction, 1280/1360 ×1.10 ≈1408/1496 ms, violating converge-up. The numeric suite floor is also stale: 1483 permits losing one test from the current 1484 ([OTP12_ACCEPTANCE_RUN.md:414](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:414)).

- **MEDIUM — no-wire scope is only conditionally credible** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:12-14`. H3 lock batching and narrow H1 pre-arming can remain local; H4 coalescing can preserve wire bytes. Reversing dial topology, batching/pipelining ADD epochs, or restoring H5 overlap changes the one-token/one-ADD or payload-after-`ManifestComplete` contract ([TRANSFER_SESSION.md:164](/Users/michael/Dev/blit_v2/docs/TRANSFER_SESSION.md:164), [TRANSFER_SESSION.md:211](/Users/michael/Dev/blit_v2/docs/TRANSFER_SESSION.md:211)). Those fixes must trigger the documented stop-and-amend rule.

- **MEDIUM — pf-1-before-fix is not enforced** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:112-115`. “pf-1 … No fix” constrains that slice, but does not make its committed, reviewed probe a prerequisite for beginning or landing pf-2.

- **HIGH — CELLS-only final reruns are insufficient for otp-13** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:116-119`. Controller, planner, sink, or phase-order fixes can regress untargeted fixtures, roles, and carriers; pre-fix PASS rows cannot establish the final binary. D2 covers every arm and otp-12d assembles the complete matrices ([OTP12_ACCEPTANCE_RUN.md:145](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:145), [OTP12_ACCEPTANCE_RUN.md:348](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:348)). Rerun complete affected-carrier matrices on both rigs from one final build, with gRPC controls where shared code changed.

**VERDICT: REQUEST CHANGES — draft `1114a92` is not implementation-ready; pf-1 must become a hard TCP-plus-historical-control gate, the hypotheses/evidence and acceptance bars need correction, and final-build full affected matrices are required before the otp-13 walk.**
tokens used
267,160
### Findings

- **INFO — H1: SUPPORTED structurally; causality unverified** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:42-52`. Push sources dial while pull sources accept; pull resize synchronously dials before ACK, then the source accepts afterward ([data_plane.rs:640](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/data_plane.rs:640), [mod.rs:3110](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:3110)). Epochs are serialized one stream at a time ([dial.rs:349](/Users/michael/Dev/blit_v2/crates/blit-core/src/dial.rs:349)). However, small, large, and mixed all target eight streams ([dial.rs:934](/Users/michael/Dev/blit_v2/crates/blit-core/src/dial.rs:934)), so H1 alone does not explain mixed-only selectivity.

- **HIGH — H2: CONTRADICTED** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:53-58`. Both layouts use the same source/destination drivers, fixed 128-entry diff chunks, and NeedBatch emission ([mod.rs:2265](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:2265), [mod.rs:2809](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:2809), [mod.rs:3437](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:3437)). Resize/planning starts only after `ManifestComplete` ([mod.rs:1385](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:1385)); the alleged initiator-dependent need-emission/resize interleave cannot occur.

- **MEDIUM — H3: SUPPORTED, but its named candidates are wrong** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:59-65`. Tiny files are tar-sharded; their sink does parallel `create_dir_all` plus `fs::write`, with no explicit per-file flush ([transfer_plan.rs:98](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_plan.rs:98), [sink.rs:592](/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/sink.rs:592)). Old push used the same sink and served-push destination progress is disabled, so flush/directory/progress are not old/new deltas. The actual new TCP-only per-file cost is `NeedListSink` locking/removing the outstanding `HashSet` for every shard member ([data_plane.rs:1030](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/data_plane.rs:1030), [data_plane.rs:1174](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/data_plane.rs:1174)); in-stream claims a shard under one lock ([mod.rs:3047](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:3047)).

- **HIGH — H4: CONTRADICTED as written** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:66-70`. The TCP record implementation is byte-for-byte unchanged from `0f922de`, and old push did not have a fixed-count opening: it could open under-shaped from the early manifest prefix and then resize one ADD at a time (`0f922de:crates/blit-core/src/remote/push/client/mod.rs:517-550`). Only shard-boundary/ramp cadence remains plausible because old TCP planned each early batch while current code accumulates until `ManifestComplete`.

- **HIGH — obvious H5 omitted: lost scan/diff/TCP-transfer overlap** — [mod.rs:1385](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:1385). Old TCP intentionally negotiated and queued payloads on mid-manifest need batches (`0f922de:crates/blit-daemon/src/service/push/control.rs:227-324`; `0f922de:crates/blit-core/src/remote/push/client/mod.rs:863-940`), while old forced-gRPC already waited. Current code makes both carriers wait for `ManifestComplete`, exactly predicting “TCP regressed, gRPC did not.”

- **HIGH — Zoey’s 0.966 is misclassified as P1 absence evidence** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:26-28,127-129`. It is new-vs-old converge-up, not a two-layout invariance measurement ([verdicts.csv:14](/Users/michael/Dev/blit_v2/docs/bench/otp12-zoey-2026-07-12/verdicts.csv:14)); the rig record explicitly assigns invariance to rig W ([README.md:3](/Users/michael/Dev/blit_v2/docs/bench/otp12-zoey-2026-07-12/README.md:3)). It cannot show P1 was absent or masked on Zoey.

- **MEDIUM — evidence wording otherwise overclaims, though headline TCP figures match** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:20-38`. P1’s 1.237/1.313 and carrier/fixture ratios, P2’s 1.105/1.149, and committed 1284/1868 references are faithful ([win README.md:44](/Users/michael/Dev/blit_v2/docs/bench/otp12-win-2026-07-12/README.md:44), [Zoey README.md:68](/Users/michael/Dev/blit_v2/docs/bench/otp12-zoey-2026-07-12/README.md:68), [summary.csv:6](/Users/michael/Dev/blit_v2/docs/bench/otp2w-baseline-2026-07-10/summary.csv:6)). But Windows `push_grpc_small` is 0.801 same-session/0.835 committed—not “0.98-ish parity” ([verdicts.csv:26](/Users/michael/Dev/blit_v2/docs/bench/otp12-win-2026-07-12/verdicts.csv:26)). The 1.313 comparison is the same rig/session, not independent, and the 2080-vs-1922 precreation lead is explicitly recorded rather than attributed ([win README.md:92](/Users/michael/Dev/blit_v2/docs/bench/otp12-win-2026-07-12/README.md:92)).

- **HIGH — pf-1 cannot discriminate the hypotheses as staged** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:74-84`. The referenced otp-3 role suite forces in-stream transport and uses `in_process_pair` ([transfer_session_roles.rs:92](/Users/michael/Dev/blit_v2/crates/blit-core/tests/transfer_session_roles.rs:92), [transfer_session_roles.rs:151](/Users/michael/Dev/blit_v2/crates/blit-core/tests/transfer_session_roles.rs:151)). It cannot time H1/H4 TCP behavior; even a TCP loopback variant cannot falsify a Windows↔Mac topology-dependent H1 from a negative local result. H3’s per-member lock and H5’s overlap also lack registered probes/kill criteria.

- **HIGH — the P2 “bisect” is unsound** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:85-90`. The flush toggle misses the tar path, precreation measures an environmental component rather than old/new attribution, and the ramp pin reproduces behavior old push already had. The deleted driver remains directly inspectable and runnable at `0f922de`; pf-1 needs an instrumented historical old-vs-new control, plus timings for `NeedListSink` claims and manifest-to-first-TCP-payload overlap.

- **HIGH — fix bars are weaker than the parent constraint** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:97-104`. D2 requires every arm independently against both same-session and committed references and forbids using invariance as a substitute ceiling ([OTP12_ACCEPTANCE_RUN.md:145](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:145)). Otherwise two 1.10 tolerances permit 1.21×. Concretely, Windows `push_tcp_small` could pass the draft at ≤1.10×1811 ≈1992 ms while remaining far slower than the better old direction, 1280/1360 ×1.10 ≈1408/1496 ms, violating converge-up. The numeric suite floor is also stale: 1483 permits losing one test from the current 1484 ([OTP12_ACCEPTANCE_RUN.md:414](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:414)).

- **MEDIUM — no-wire scope is only conditionally credible** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:12-14`. H3 lock batching and narrow H1 pre-arming can remain local; H4 coalescing can preserve wire bytes. Reversing dial topology, batching/pipelining ADD epochs, or restoring H5 overlap changes the one-token/one-ADD or payload-after-`ManifestComplete` contract ([TRANSFER_SESSION.md:164](/Users/michael/Dev/blit_v2/docs/TRANSFER_SESSION.md:164), [TRANSFER_SESSION.md:211](/Users/michael/Dev/blit_v2/docs/TRANSFER_SESSION.md:211)). Those fixes must trigger the documented stop-and-amend rule.

- **MEDIUM — pf-1-before-fix is not enforced** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:112-115`. “pf-1 … No fix” constrains that slice, but does not make its committed, reviewed probe a prerequisite for beginning or landing pf-2.

- **HIGH — CELLS-only final reruns are insufficient for otp-13** — `1114a92:docs/plan/OTP12_PERF_FINDINGS.md:116-119`. Controller, planner, sink, or phase-order fixes can regress untargeted fixtures, roles, and carriers; pre-fix PASS rows cannot establish the final binary. D2 covers every arm and otp-12d assembles the complete matrices ([OTP12_ACCEPTANCE_RUN.md:145](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:145), [OTP12_ACCEPTANCE_RUN.md:348](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:348)). Rerun complete affected-carrier matrices on both rigs from one final build, with gRPC controls where shared code changed.

**VERDICT: REQUEST CHANGES — draft `1114a92` is not implementation-ready; pf-1 must become a hard TCP-plus-historical-control gate, the hypotheses/evidence and acceptance bars need correction, and final-build full affected matrices are required before the otp-13 walk.**
