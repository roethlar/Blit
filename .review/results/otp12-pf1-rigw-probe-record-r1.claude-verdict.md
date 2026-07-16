# otp12-pf1-rigw probe record — Claude adjudication

- Reviewer: `claude-fable-5` via Claude Code `2.1.211`, effort `max`
- Reviewed range: `f120f4c4a2321fe2cb8a3fb637a62f75bb9b9ff5..7ecc2f9152fa5a4413ab928fcbdd17c78d6d7c05`
- Review session: `66e213d0-6c66-48c5-b9af-6a45c484134a`
- Retained worktree: `/tmp/blit-review-pf1-record-7ecc2f9`
- Orchestrator record: `.review/results/otp12-pf1-rigw-probe-record-r1.claude.json`
- Independent verdict: `ACCEPTED`
- Guard confirmed: `true`

Claude independently accepted the exact immutable evidence record. It
recomputed the 290-file / 20,517,586-byte inventory and digest, all key
hashes, 128-arm schedule and per-arm validity, 768 clocks, client/fixture/
landed inventories, raw-to-exported trace identity, 11,392 events, 14,964
intervals, every summary statistic, observer bias, `N_resolution=329 ms`,
live 8/8 stream parity, and every number in the two-layout phase table.

The interpretation is also accepted without correction. The run proves live
worker/stream parity and a valid current-build P1 non-reproduction. It fails
the registered resolution check, its gRPC control fails, and the target
baseline is absent/reversed, so it supplies no causal grade and does not close
P1 or the formal hard gate. P2 and the pinned `0f922de` historical control
remain next; more P1 rig time requires an owner-approved amendment.

The retained-copy analyzer was green with byte-identical reports. Removing
only the final `runs.csv` data row failed closed at 127/128. The first restore
invoked `cp -i`, blocked on its overwrite prompt, and was interrupted; that
pass was counted as invalid and all files remained retained. Claude then
independently issued a separate explicit noninteractive byte restoration,
reran the analyzer green, and matched all six reports. The restored `runs.csv`
is byte-identical at SHA-256
`69c10ae12f7591b93585670fcbb62f9021fdeeaf6c4a60e78277190d112bc979`.

All 23 analyzer tests, the docs gate, and authored-file whitespace checks
passed. The independent audit script also reran clean after review. The
detached worktree ended clean at exact `7ecc2f9`; no benchmark endpoint was
contacted. Plugin updates were disabled, the live socket was directly to
Anthropic rather than the removed loopback proxy, and web search/fetch counts
were zero.

Three earlier Claude attempts remain recorded but supply no review evidence:
one was interrupted after proxy/plugin-updater external traffic, one still
loaded the stale loopback route, and one reached Anthropic directly but hit
the pre-reset account limit. Only the fourth, post-reset structured verdict
is authoritative.
