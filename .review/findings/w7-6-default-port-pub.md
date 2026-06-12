# w7-6-default-port-pub — DEFAULT_PORT pub; production 9031 literals deleted

**Branch**: `master` (owner-authorized session 2026-06-12, "Continue with 1")
**Commit**: `de04054`
**Source finding**: boundaries-private-default-port-literal-duplication — `docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md`

## What

`RemoteEndpoint::DEFAULT_PORT` was private, so production code restated
the 9031 decision as bare literals. The const is now `pub` and the three
production restatements route through it.

## Approach

- `endpoint.rs`: `pub const DEFAULT_PORT`, doc naming its consumers.
- `blit-daemon/runtime.rs`: bind default `unwrap_or(DEFAULT_PORT)`.
- `blit-cli/scan.rs`: default-port display elision compares against it.
- `blit-tui/daemons.rs`: local-row loopback fallback formats with it.

## Files changed

4 files, 12 insertions, 4 deletions. No behavior change.

## Tests added

None (pure constant routing; suite flat at 1369). The existing
endpoint tests already pin DEFAULT_PORT semantics.

## Known gaps

- ~90 remaining `9031` occurrences are test fixtures / fixture strings
  (`"nas:9031:/…"`) — sample data, not policy restatements; deliberately
  left.
- `blit-tui/screens/f1.rs` placeholder copy ("fetching GetState from
  127.0.0.1:9031...") and two comments still hardcode the number in
  display text; if the default port ever changes, grep for `9031` —
  the doc on DEFAULT_PORT says so.
- Manpages/README mention 9031 — docs sweep is w10's slice.
