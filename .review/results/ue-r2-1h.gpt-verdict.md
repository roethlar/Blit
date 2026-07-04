# ue-r2-1h — review adjudication

**Slice commits**: `9f37a7a` (Windows clippy baseline + the mis-staged
`service/pull.rs` deletion) + `48c5a11` (win-1 push separator fix) +
`2a13f53` (the slice proper)
**Fix commit**: `f6f52d7`
**reviewer: gpt-5.5** (codex exec, read-only, headroom proxy; raw
output `.review/results/ue-r2-1h.codex.md` — the tail carries the
findings; the file is a full session transcript). A 3-lens adversarial
self-review panel (wire-compat / daemon-invariants / client-adapter,
Claude subagents) ran in parallel; attributed separately below.
Process note: the first codex invocation hung reading stdin (headless
quirk); rerun with stdin closed.

codex VERDICT: **NEEDS FIXES** (3 findings).

## codex findings

1. **High — delegated pull forwards `metadata_only`; destination
   materializes bare headers as empty files and reports success**
   (`delegated_pull.rs:208`, `:362`). **Accepted** — independently
   found by both panel daemon/wire lenses (their F1). The flag was
   documented as "ignored" by delegated pull but actually traveled
   verbatim to the source, whose new metadata branch skips comparison
   and sends header-only frames; the dst daemon's pull_sync client
   loop `File::create`s each header — truncating/creating zero-byte
   files, then a success summary. **Fixed**: `validate_spec` rejects
   `metadata_only=true` at the delegated boundary (fail closed, before
   any outbound connect), proto + operation_spec comments corrected to
   say REJECTS (not ignores), unit test
   `validate_spec_rejects_metadata_only` pins it.

2. **Medium — `RemoteFileStream` recursion / outside the
   `recv_fallback_message` chokepoint** (`pull.rs:1174/:1182/:1215`).
   **Split verdict.** Recursion: **Accepted** (also panel F2 —
   per-skipped-frame and per-empty-`file_data` native stack frames,
   unbounded under hostile chatter). **Fixed**: `poll_read` is now a
   loop; no recursion remains. Chokepoint half: **Rejected** — the
   audit-h3c module doc (`grpc_fallback.rs`, "Out of scope for slice
   1/2") explicitly names `RemoteFileStream`'s `poll_next`-inside-
   `AsyncRead` shape as off the h3c surface, needing its own adapter;
   that exclusion predates this slice and was consciously carried over
   (the slice updated the doc to say so). A stalled peer hanging this
   reader is the same pre-existing gap audit-h3c slice 2 owns.

3. **Low — `docs/API.md` still documents the deleted RPC**
   (`API.md:14`, the "Pull Operation" section). **Accepted** —
   **Fixed**: service block updated with a dated removal note (plus an
   honest pre-existing-drift caveat: that block already omitted
   PullSync/DelegatedPull/GetState/etc.), the Pull Operation section
   replaced with a removal record pointing at PullSync;
   `ARCHITECTURE.md`'s service block updated the same way.

## Self-review panel findings (Claude, 3 adversarial lenses)

- **F1 (wire + daemon lenses, Medium) — delegated `metadata_only`
  truncation**: same as codex #1. Accepted + fixed (above).
- **F2 (client + daemon lenses, Medium/Low) — adapter recursion**:
  same as codex #2's accepted half. Fixed (loop).
- **F3 (client lens, Low) — a second `file_header` mid-stream silently
  concatenates the next file's bytes**. **Accepted** (cheap hardening;
  today's consumers bound reads by declared size, so it surfaced as an
  error, not corruption). **Fixed**: `header_seen` guard — a second
  header before the summary is a protocol error; wire test
  `open_remote_file_rejects_second_file_header` pins it.
- **F4 (daemon + client lenses, Medium) — the two intermediate commits
  don't build** (`pull.rs` deleted at `9f37a7a` while `mod.rs`/
  `core.rs` still referenced it — the staging slip is a bisect break,
  not mere attribution). **Accepted as a records correction**: the
  finding doc's erratum now states the build break plainly; history
  left unrewritten per AGENTS.md git-safety; surfaced to the owner
  (who may authorize a history fix).
- **F5 (wire lens, Low) — relay subpath double-join** (`--relay-via-cli
  host:mod/sub` scans `sub/sub`; verified pre-existing in the deleted
  code — the port is faithful). **Deferred**: filed as
  `.review/findings/relay-1-subpath-double-join.md` + REVIEW.md
  pending-review row.

## Cross-checks recorded for the record

Both reviewers independently confirmed: proto residue clean; the
old-daemon scan degradation correct and pinned; empty-manifest scan
semantics total (every compare mode → all files New; no delete list);
relocated `collect_pull_entries_with_checksums`/`PullEntry`/
`build_file_header` byte-identical to the deleted originals;
half-close safe against every daemon read arm reachable by relay
specs; the 4 shrunk mock `pull` impls were pure `unimplemented!` stubs
(no coverage lost); win-1's `relative_path_to_posix` output identical
on unix (incl. the `""`/`"."` edge).

Validation after fixes: fmt/clippy clean, tests (Windows host)
**1393 / 0 / 3** (entering Windows baseline 1391; +1 delegated
rejection unit test, +1 second-header wire test. The historical unix
baseline 1413/0/2 is not host-comparable — unix-gated tests compile
out here; see the finding doc's test-count note.)
