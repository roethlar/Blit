# prose-endpoints: a transfer endpoint was classified by its refusal's words

**Severity**: LOW — a breach of D-2026-11-24-1 with a narrow misreading. `parse_transfer_endpoint` decided "local path" and "refused for backslashes" by searching the remote parser's error message for "appears to be a local path" and "forward slashes". Some of that parser's refusals quote the input, so an input holding those words was misclassified.
**Status**: Verified — `combined-r35` (Codex openreview, 2026-11-24) reviewed `0f912b06..dc14afc4` — the `combined-r34` fixes and the three prose fixes — with `guard_confirmed: true`: "The ordinary Linux copy now succeeds through a valid destination symlink and retains its writable handle. A Unix resume refuses a write when it can no longer establish the destination directory. The typed endpoint and resume-error classifications also fit the goal. The remaining problems are at identifiable points where the implementation drops a handle, a file identity, or a known cleanup result."
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `a310829a`
**Reviewer**: none yet. Found by the working agent on 2026-11-24, scanning for decisions read from prose under D-2026-11-24-1.

## Evidence
`crates/blit-core/src/endpoints.rs` read `err.to_string()` and tested `contains("appears to be a local path")` and `contains("forward slashes")`. `parse_host_port`'s refusals quote the authority (`unterminated IPv6 address: {authority}`, `invalid host specification: {authority}`). Two examples:
- `[input appears to be a local path:/m/x`, a remote-shaped typo, was taken as a local path. It should be refused like every input with `:/`: treating a typo'd remote as local silently is the footgun this parser exists to stop.
- `[must use forward slashes:x`, with no remote shape, was refused. It should be a local path.

## What
`RemoteEndpoint::parse` refuses an input it reads as not remote with a typed error, `remote::endpoint::NotRemote`:
- `LocalPath`;
- `Backslashes(input)`.

Its `Display` keeps the former messages word for word. `parse_transfer_endpoint` downcasts the error and decides by the variant.

## Guard proof
- `endpoints::tests::a_refusals_words_do_not_classify_an_endpoint`:
  - the two inputs above are refused and local respectively;
  - `C:/data` is still local;
  - `srv:\m\x` is still refused.
- Mutations, each restored byte for byte:
  - local by the words → red;
  - refused by the words → red.
- Checks: native clippy is clean, and the endpoint tests pass.

## Files changed
- `crates/blit-core/src/remote/endpoint.rs` — `NotRemote`, returned by `parse`.
- `crates/blit-core/src/endpoints.rs` — classified by type; guard.

## Known gaps
- `NotRemote` is a new public type in `blit-core`: an addition, not a break.
