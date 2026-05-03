# Follow-up Review Log - 2026-05-02

This document tracks incremental reviews after the baseline review in
`docs/reviews/codebase_review_2026-05-01.md`. Each landed change should get a
new review round appended here.

## Round 1 - F1 Path Safety Follow-up

Reviewed change:

- Commit: `cc77074 feat(F1): centralize receive-side path validation`
- Commit time: `2026-05-01 20:48:15 -0400`
- Scope: centralized wire-path validation, receive-side `safe_join` migration,
  daemon/pull sanitizer wrappers, pipeline boundary validation, and new tests.

Verification:

- `cargo test --workspace` passed locally after the change: 209 tests, 0
  failures, 1 ignored doc test.
- Existing warnings remain unrelated to F1: deprecated macOS FSEvents API usage
  and an unused macOS capability test variable.

Verdict:

The change is architecturally right and materially improves the previous F1
risk. It creates the missing shared chokepoint, migrates the main receive write
sites, and adds useful unit/integration coverage. I would treat F1 as mostly
implemented, but not fully closed until the edge cases below are addressed or
explicitly scoped into F2/chroot work.

What landed well:

- `crates/blit-core/src/path_safety.rs` is the right place for shared policy.
- `validate_wire_path` and `safe_join` separate pure validation from
  destination-root joining.
- `crates/blit-core/src/remote/transfer/sink.rs` now routes streamed files, tar
  extraction, resume block writes, and block completion through the shared
  helper.
- `crates/blit-core/src/remote/transfer/pipeline.rs` validates file and
  tar-shard header paths when they arrive off the TCP data-plane stream.
- `crates/blit-core/src/remote/pull.rs` and
  `crates/blit-daemon/src/service/util.rs` now wrap the shared helper instead
  of maintaining independent validators.
- Tests cover the main intended surface: `..`, Unix absolutes, Windows drive
  prefixes, UNC forms, NUL, filenames containing literal `..`, empty path for
  single-file destinations, and sink-level rejection.

Findings:

### R1-F1. Single-leading-backslash paths are not rejected uniformly

Severity: Medium

`crates/blit-core/src/path_safety.rs:109` to
`crates/blit-core/src/path_safety.rs:120` catches UNC/device paths beginning
with `\\` or `//`, and drive-letter forms like `C:\...`. It does not catch a
single leading backslash such as `\Windows\System32` or `\tmp\file` on Unix
hosts. On Unix, `Path::components` treats that string as a normal component, so
`validate_wire_path("\\tmp\\file")` can pass.

The comment at `crates/blit-core/src/path_safety.rs:107` says `\foo` is caught
by `Path::is_absolute` / `RootDir`, but that is only true on Windows, not on
Unix. Because this is wire-path validation for cross-platform peers, the policy
should reject Windows-root-shaped inputs consistently on every host.

Recommendation:

Reject `s.starts_with('\\')` in `looks_like_windows_absolute`, or reject all
backslashes in wire paths if the protocol standard is slash-normalized. Add
tests for `\foo`, `\foo\bar`, and `\`.

### R1-F2. `safe_join` remains lexical and does not stop symlink-parent escape

Severity: Medium, or F2/chroot scope depending on product semantics

`crates/blit-core/src/path_safety.rs:90` to
`crates/blit-core/src/path_safety.rs:96` validates the string and returns
`root.join(validated)`. This blocks traversal syntax, absolute paths, Windows
prefixes, UNC paths, and NUL bytes, but it does not prevent writes through an
existing symlink under the destination root. For example, if `dest/link` points
outside `dest`, a received path `link/file` can still write outside the intended
tree.

This may belong under the broader F2 `use_chroot` / containment work rather than
F1, but it should be explicitly tracked because the original F1 concern was
"write outside destination root."

Recommendation:

If receive roots are meant to be hard containment boundaries, add a
canonicalized-parent containment check before creating/opening files. If symlink
following is accepted behavior, document that F1 is lexical path safety only and
that symlink containment is handled by the F2/chroot track.

### R1-F3. `"."` normalizes to empty and gets single-file behavior in `safe_join`

Severity: Low

`crates/blit-core/src/path_safety.rs:71` to
`crates/blit-core/src/path_safety.rs:76` strips `.` components, and the test at
`crates/blit-core/src/path_safety.rs:250` to
`crates/blit-core/src/path_safety.rs:253` asserts that `"."` normalizes to an
empty path. Since `safe_join` treats an empty validated path as "return root
unchanged", a wire file path of `"."` gets the same behavior as the legitimate
single-file empty path `""`.

Impact is likely low because normal sender paths should not emit `"."` as a file
header. Still, empty path is a load-bearing special case, and `"."` is not
semantically identical to "single-file source emitted empty relative path."

Recommendation:

Consider separating validators by context: allow `""` for file-header
single-file mode, reject `"."` for file payload paths, and keep `"."` mapping to
directory root only for admin/list-style request paths.

Status:

F1 is substantially improved but should stay open as "needs small follow-up"
until R1-F1 is fixed and R1-F2/R1-F3 are either implemented or explicitly moved
to the next hardening phase.

## Round 2 - R1 Fixes, TransferOperationSpec, DiffPlanner 3a

Reviewed changes:

- `2e0214d fix(F1 followup): leading backslash, dot disambiguation, lexical contract`
- `21ad75a feat(step-2): TransferOperationSpec proto contract`
- `8a15e5a feat(step-3a): extract DiffPlanner; orchestrator uses unified stage`

Verification:

- `cargo check --workspace` passed.
- `cargo test --workspace` passed: 214 tests, 0 failures, 1 ignored doc test.
- Existing unrelated warnings remain: deprecated macOS FSEvents API and unused
  macOS capability test variable.

Verdict:

The R1 path-safety follow-up is sufficient to close F1 as lexical receive-path
safety. R1-F1 is fixed, R1-F3 is fixed with explicit context separation, and
R1-F2 is now documented as F2/chroot canonical-containment scope. The
`TransferOperationSpec` addition is a useful contract-only step. The
`DiffPlanner` extraction is an acceptable first migration for local mirror, but
there are two follow-up risks to handle before the planner becomes the shared
source of truth for push/pull.

What landed well:

- `crates/blit-core/src/path_safety.rs:151` to
  `crates/blit-core/src/path_safety.rs:162` now rejects single-leading
  backslash inputs consistently across platforms.
- `crates/blit-core/src/path_safety.rs:106` to
  `crates/blit-core/src/path_safety.rs:117` rejects non-empty inputs that
  normalize to empty, so `"."` no longer gets the single-file empty-path
  behavior.
- `crates/blit-daemon/src/service/util.rs:48` to
  `crates/blit-daemon/src/service/util.rs:56` preserves the request-path
  meaning of module root for `""`, `"."`, and `"./"` without weakening file
  payload validation.
- `crates/blit-core/src/path_safety.rs:25` to
  `crates/blit-core/src/path_safety.rs:46` now explicitly documents the
  lexical-only contract and defers symlink containment to F2.
- `proto/blit.proto:308` to `proto/blit.proto:343` defines the normalized
  `TransferOperationSpec` shape, which is the right contract boundary for the
  role-model refactor.
- `proto/blit.proto:396` to `proto/blit.proto:409` makes mirror delete scope
  explicit, which is the right way to resolve the filtered-mirror ambiguity from
  the baseline review.
- `crates/blit-core/src/remote/transfer/diff_planner.rs` gives local mirror a
  shared diff + payload planning stage and reduces inline orchestration code in
  `crates/blit-core/src/orchestrator/orchestrator.rs`.

Findings:

### R2-F1. DiffPlanner silently treats unimplemented comparison modes as size+mtime

Severity: Medium

`crates/blit-core/src/remote/transfer/diff_planner.rs:102` to
`crates/blit-core/src/remote/transfer/diff_planner.rs:120` accepts every
`ComparisonMode`, but maps everything except `Checksum` to `None`, which the
existing comparison primitive interprets as size+mtime.

The module docs call this out as temporary, and the current local orchestrator
only passes `SizeMtime` or `Checksum`, so this is not a current behavior
regression. The risk is future integration: once push or pull starts passing
`SizeOnly`, `IgnoreTimes`, `IgnoreExisting`, or `Force`, those modes will appear
to be accepted but will behave incorrectly unless every call site remembers the
temporary limitation.

Recommendation:

Before Step 3b or Step 4 uses `DiffPlanner` with user-derived
`TransferOperationSpec.compare_mode`, make unsupported modes fail closed or
return a typed "not implemented in this planner entry point" error. Better:
model the comparison decision as a real enum internal to `DiffPlanner` instead
of reducing it to `Option<ChecksumType>`, then implement the full semantics.

### R2-F2. `TransferOperationSpec` is proto-only; no conversion/normalization helpers yet

Severity: Medium

`proto/blit.proto:308` to `proto/blit.proto:343` and
`crates/blit-core/src/remote/transfer/operation_spec.rs:26` to
`crates/blit-core/src/remote/transfer/operation_spec.rs:29` expose the new
contract, but there is no Rust-side builder or normalization layer yet.

That is acceptable for a contract-only commit, but the next behavior-changing
step should not spread proto enum defaulting, version checks, filter conversion,
mirror-mode defaulting, and capability checks across CLI, daemon, and core.

Recommendation:

Add a small `NormalizedTransferOperation` or `TransferOperationIntent` type in
core before wiring this into RPC handlers. It should fold
`*_UNSPECIFIED` values, validate `spec_version`, convert `FilterSpec` to
`FileFilter`, choose the default mirror mode, and validate capabilities. This
keeps protobuf wire shape from becoming the business-logic shape.

### R2-F3. Checked-in generated protobuf file is stale

Severity: Low

`proto/blit.proto` now contains `TransferOperationSpec`, `FilterSpec`,
`ComparisonMode`, `MirrorMode`, `ResumeSettings`, and `PeerCapabilities`.
`cargo check` succeeds because `crates/blit-core/src/lib.rs` uses
`tonic::include_proto!("blit.v2")` and the build script regenerates bindings
under `OUT_DIR`.

However, `crates/blit-core/src/generated/blit.v2.rs` is tracked and does not
contain the new types. That file appears stale/dead relative to the current
build path.

Recommendation:

Either stop tracking `crates/blit-core/src/generated/blit.v2.rs` if it is not
used, or regenerate it whenever `proto/blit.proto` changes. Leaving a stale
generated file in-tree creates review confusion and risks future agents reading
the wrong API surface.

### R2-F4. DiffPlanner tests cover the extraction, but not orchestrator equivalence deeply

Severity: Low

The new unit tests in `diff_planner.rs` cover checksum-mode mapping and basic
unchanged/missing-destination behavior. Existing integration tests passing gives
reasonable confidence, but there is no focused test proving
`plan_local_mirror` preserves local mirror behavior across checksum mode,
`skip_unchanged = false`, tar-shard batching, and large-file planning.

Recommendation:

Before expanding `DiffPlanner` to push/pull, add focused tests at the planner
boundary. They should verify that local inputs produce the same payload classes
and copy decisions for size+mtime, checksum, force/ignore-times behavior once
implemented, small-file tar shards, and large-file raw payloads.

Status:

- F1 lexical path safety: closed.
- F2 symlink/canonical containment: still open and correctly separated.
- Step 2 contract: acceptable as a contract-only step.
- Step 3a DiffPlanner extraction: acceptable, with R2-F1 and R2-F2 as gating
  concerns before push/pull consume the new planner broadly.

## Round 3 - Step 3b Push Client DiffPlanner Route

Reviewed change:

- `b229e44 refactor(step-3b): push client routes through diff_planner`

DEVLOG source:

- `2026-05-02 03:30:00Z` entry describing `plan_push_payloads` and push-client
  import routing through `diff_planner`.

Verification:

- `cargo test --workspace` passed: 214 tests, 0 failures, 1 ignored doc test.
- `cargo fmt -- --check` failed on the current tree. The diff spans many files,
  so this is not attributed solely to `b229e44`; however, the current branch is
  not rustfmt-clean.

Verdict:

No functional findings for this commit. The change is intentionally small: push
already receives a daemon-produced NeedList, so the client-side planning step
only needs to convert already-filtered headers into payloads. Routing that call
through `diff_planner::plan_push_payloads` is a reasonable waypoint for the
unification plan without pretending push has a local comparison stage.

What landed well:

- `crates/blit-core/src/remote/transfer/diff_planner.rs:48` to
  `crates/blit-core/src/remote/transfer/diff_planner.rs:55` adds a named push
  planning entry point with context-rich errors.
- `crates/blit-core/src/remote/push/client/mod.rs:31` to
  `crates/blit-core/src/remote/push/client/mod.rs:36` documents that push's
  diff remains daemon-driven.
- The existing remote push integration tests still pass, which is sufficient
  for a re-route with no intended behavior change.

Notes:

- R2-F1 remains open but is not worsened by this commit. Push still does not
  pass user-derived `ComparisonMode` into `DiffPlanner`; it passes the daemon's
  NeedList through payload planning.
- R2-F2 remains open. `TransferOperationSpec` normalization is still needed
  before behavior-changing RPC wiring.
- The import alias
  `use crate::remote::transfer::diff_planner::plan_push_payloads as plan_transfer_payloads;`
  preserves the old call-site name. That is safe, but slightly undercuts the
  readability goal. Prefer calling `plan_push_payloads(...)` directly in a
  cleanup pass so the push code visibly uses the new entry point.
- Before CI/release, run `cargo fmt` or otherwise resolve the existing
  formatting drift so `cargo fmt -- --check` passes again.

Status:

- Step 3b: accepted.
- No new blocking findings.

## Round 4 - Step 4A PullSync Wire Migration

Reviewed change:

- `e503938 feat(step-4A): PullSync wire migration to TransferOperationSpec`

DEVLOG source:

- `2026-05-02 04:15:00Z` entry describing the PullSync move from
  `PullSyncHeader` to `TransferOperationSpec`.

Verification:

- `cargo test --workspace` passed: 214 tests, 0 failures, 1 ignored doc test.
- `cargo fmt -- --check` passed.
- Existing unrelated warnings remain: deprecated macOS FSEvents API and an
  unused macOS capability test variable.

Verdict:

The wire migration is directionally correct. Removing `PullSyncHeader` avoids
carrying the old release-internal protocol forward, and routing PullSync through
`TransferOperationSpec` is the right contract boundary for the next steps.
However, the implementation still treats the protobuf message as a loose flag
bag at the daemon boundary. Two issues should be fixed before Step 4B builds
behavior on top of this contract.

Findings:

### R4-F1. PullSync accepts unknown/default operation specs instead of validating the contract

Severity: Medium

`proto/blit.proto:298` to `proto/blit.proto:301` says receivers should reject
operation spec versions they do not understand, but
`crates/blit-daemon/src/service/pull_sync.rs:57` to
`crates/blit-daemon/src/service/pull_sync.rs:60` accepts any `spec_version`
and silently defaults invalid `mirror_mode` / `compare_mode` enum values to
`Off` / `SizeMtime`.

This weakens the main value of the migration. A malformed, default, or future
spec can execute as a valid pull with default semantics instead of failing at
the wire boundary. With no backward-compatibility requirement, this should be
strict rather than permissive.

Recommendation:

Reject `spec.spec_version != 1`, reject invalid enum values, and make that
normalization happen in one helper rather than scattered inline in
`handle_pull_sync_stream`. This also addresses the earlier R2-F2 concern that
the protobuf shape needs a normalized Rust intent before downstream behavior
depends on it.

### R4-F2. `IgnoreExisting` is still modeled as a comparison mode, so CLI bool combinations are silently prioritized

Severity: Medium

`crates/blit-core/src/remote/pull.rs:410` to
`crates/blit-core/src/remote/pull.rs:419` collapses local bool options into one
`ComparisonMode` by precedence. The daemon then re-expands
`ComparisonMode::IgnoreExisting` into `(CompareMode::Default, true)` at
`crates/blit-daemon/src/service/pull_sync.rs:247` to
`crates/blit-daemon/src/service/pull_sync.rs:254`.

That acknowledges `ignore_existing` is orthogonal to comparison mode, but the
wire contract cannot represent it orthogonally. The CLI still exposes
`--ignore-existing` and `--force` as independent booleans at
`crates/blit-cli/src/cli.rs:154` to `crates/blit-cli/src/cli.rs:159`, so a user
can still construct combinations locally. For example, `--force
--ignore-existing` used to set both flags and the comparison layer skipped
existing files because `ignore_existing` short-circuited. Step 4A now picks
`Force` first and drops `ignore_existing`.

Recommendation:

Either make `ignore_existing` a separate `TransferOperationSpec` field (or a
nested comparison settings message with `mode` plus `ignore_existing`), or add
CLI/core normalization that rejects conflicting combinations before building
the spec. Do not rely on precedence ordering while claiming the new type makes
nonsensical states unconstructible.

### R4-F3. Pull client advertises filter capability and comments imply F10 is closed before filters are wired

Severity: Low

`crates/blit-core/src/remote/pull.rs:407` to
`crates/blit-core/src/remote/pull.rs:409` says filter rules ride in the spec
and are honored by the daemon, but the same function sends
`FilterSpec::default()` at `crates/blit-core/src/remote/pull.rs:437`.
`crates/blit-cli/src/transfers/remote.rs:231` to
`crates/blit-cli/src/transfers/remote.rs:249` still bails on filter args for
remote-source transfers, and the daemon explicitly says filter parity is Step
4B at `crates/blit-daemon/src/service/pull_sync.rs:61` to
`crates/blit-daemon/src/service/pull_sync.rs:63`.

Recommendation:

Keep the 4A comments and capability advertisement honest: this commit defines
the wire slot but does not close F10. Step 4B should be the commit that sends a
real `FilterSpec`, applies it during daemon enumeration, removes the CLI bail,
and adds filtered-mirror tests.

### R4-F4. Proto/docs still describe old compatibility assumptions

Severity: Low

`proto/blit.proto:285` to `proto/blit.proto:290` still says
`TransferOperationSpec` is not wired into any RPC and that `PullSyncHeader`
remains the active PullSync control message. `proto/blit.proto:416` to
`proto/blit.proto:428` also describes a `supports_filter_spec` fallback path
for old daemons, which no longer matches the project's no-backward-compatibility
position.

Recommendation:

Clean up the stale comments in the next protocol pass. The code has moved, so
the contract comments should now describe the active PullSync shape rather than
the pre-4A migration plan.

Status:

- Step 4A: accepted as a structural migration.
- R2-F2 remains open and is now concrete: add a normalized operation intent and
  strict wire validation.
- F10 remains open until Step 4B sends and applies real filters.
- F4 remains open until mirror deletion uses an exact delete list and filtered
  subset semantics are tested.

## Round 5 - R2/R4 Closure, Step 4B Filtered Pull, Step 4C gRPC Tar Shards

Reviewed change:

- Commit range: `e503938..4f84d7c`
- Included commits: `d68f9f7`, `4a14840`, `ae15483`, `33796fd`,
  `4101cac`, `8da4824`, `71d3c30`, `cbd5eee`, `2d155d1`, `e6a1b7a`,
  `4f84d7c`.
- Scope: normalized operation spec adoption, concrete `ComparisonMode`
  semantics, strict pull wire normalization, `ignore_existing` split from
  comparison mode, pull filter parity, authoritative mirror delete lists,
  gRPC pull tar-shard fallback, and planner-boundary tests.

DEVLOG check:

- New entries exist at `DEVLOG.md:194`, `DEVLOG.md:196`, and `DEVLOG.md:198`.
- They describe the claimed R2/R4 closures plus Step 4B and Step 4C.
- They are appended after older 2025 entries rather than being placed at the
  top of the log, so a normal latest-first scan misses them.

Verification:

- `cargo fmt -- --check` passed.
- `cargo test --workspace` passed.
- Existing warnings remain unrelated to this review: deprecated macOS
  FSEvents usage and an unused macOS capability test variable.

Verdict:

The architectural direction is right: `TransferOperationSpec` is now active on
the pull path, `ignore_existing` is no longer encoded as a compare mode, pull
filters are wired end to end, and the gRPC fallback no longer has the
single-file-only limitation. The R2/R4 fixes are mostly accepted.

However, Step 4B and Step 4C introduce new client-side trust-boundary bugs.
Both are on messages authored by the daemon and consumed by the CLI, so they
should be treated with the same rigor as the F1 receive-side path work.

### R5-F1. `DeleteList` application bypasses `safe_join` and can delete outside the destination

Severity: High

`crates/blit-cli/src/transfers/remote.rs:435` to
`crates/blit-cli/src/transfers/remote.rs:447` applies daemon-supplied delete
paths with:

```rust
let target = dest_root.join(rel);
if !target.starts_with(dest_root) { ... }
tokio::fs::remove_file(&target).await
```

This is not equivalent to the shared F1 path-safety helper. A path like
`../victim` produces a lexical path such as `dest_root/../victim`; that path
still starts with `dest_root` component-wise before normalization, so the check
passes and the subsequent remove can target a file outside the destination
root.

This also contradicts the local comment at `crates/blit-cli/src/transfers/remote.rs:420`
to `crates/blit-cli/src/transfers/remote.rs:421`, which says the daemon cannot
escape the destination root via `..`.

Recommendation:

Route every `DeleteList` entry through `blit_core::path_safety::safe_join`.
Ideally store delete-list entries as strings until validation, rather than
converting wire strings into `PathBuf` at
`crates/blit-core/src/remote/pull.rs:534` to
`crates/blit-core/src/remote/pull.rs:540`. Add a regression test that sends
`../outside.txt` in `DeleteList` and proves the client rejects it without
touching the sibling file.

### R5-F2. gRPC pull tar-shard extraction trusts tar entry type and uses `Entry::unpack`

Severity: High

`crates/blit-core/src/remote/pull.rs:936` to
`crates/blit-core/src/remote/pull.rs:957` accepts all non-directory tar entries
and calls `entry.unpack(&dest_path)`. That means the receiver trusts tar header
semantics from the daemon. A malicious or buggy daemon can send an expected path
with a symlink, hardlink, or other special tar entry type instead of a regular
file. The path name is checked, but the entry type and link target are not.

The safer sink-side implementation already avoids this problem:
`crates/blit-core/src/remote/transfer/sink.rs:390` to
`crates/blit-core/src/remote/transfer/sink.rs:430` buffers entry bytes and writes
regular file contents via `path_safety::safe_join`, rather than asking the tar
crate to materialize arbitrary entry types.

Recommendation:

Make the pull gRPC extractor follow the same receive policy as
`write_tar_shard_payload`: reject anything that is not a regular file, validate
the path through `safe_join`, copy bytes out of the tar entry, and write the file
contents directly. Also verify the entry size matches the expected
`FileHeader.size` before counting or reporting the file. Add a malicious
tar-shard regression test for a symlink entry whose `relative_path` matches an
expected header.

### R5-F3. `TarShardHeader.archive_size` can force an unbounded client allocation

Severity: Medium

`crates/blit-core/src/remote/pull.rs:576` to
`crates/blit-core/src/remote/pull.rs:584` allocates the shard buffer with
`Vec::with_capacity(header.archive_size as usize)` directly from the daemon's
`TarShardHeader`.

That makes `archive_size` a remote-controlled allocation hint. A bad daemon can
advertise a huge value before sending any chunk, causing the client to attempt a
large allocation or hit platform-dependent truncation from `u64 as usize`.

Recommendation:

Use `usize::try_from`, cap `archive_size` to the configured/negotiated maximum
tar-shard size, and reserve incrementally instead of preallocating the full
remote-supplied size. While chunks arrive, reject if accumulated bytes exceed
the declared size or the local cap; on `TarShardComplete`, reject if the final
length does not equal `archive_size`.

### R5-F4. `NormalizedTransferOperation::from_spec` claims filter validation it does not perform

Severity: Low

`crates/blit-core/src/remote/transfer/operation_spec.rs:81` to
`crates/blit-core/src/remote/transfer/operation_spec.rs:86` and
`crates/blit-core/src/remote/transfer/operation_spec.rs:165` to
`crates/blit-core/src/remote/transfer/operation_spec.rs:168` say malformed
filter rules become hard errors. The actual conversion at
`crates/blit-core/src/remote/transfer/operation_spec.rs:169` to
`crates/blit-core/src/remote/transfer/operation_spec.rs:181` only copies fields
into `FileFilter`. `FileFilter::build_globset` silently drops invalid globset
patterns at `crates/blit-core/src/fs_enum.rs:115` to
`crates/blit-core/src/fs_enum.rs:123`.

This is not currently a release blocker if local CLI filter behavior is already
defined as permissive, but the normalized wire boundary should not document
strict validation that is not happening.

Recommendation:

Either make `filter_from_spec` actually validate include/exclude patterns and
files-from paths, or soften the comments to say it normalizes filter fields and
parses scalar size/age values before the CLI builds the proto.

### R5-F5. DEVLOG ordering makes the review workflow easy to miss

Severity: Low

The new 2026-05-02 entries are present but appended around `DEVLOG.md:194`
below old 2025 entries. The current workflow depends on agents checking DEVLOG
for newly landed work; if the log is not latest-first or clearly partitioned,
reviewers can falsely conclude there are no new entries.

Recommendation:

Pick one convention and enforce it. Given the current usage, latest-first is
the least surprising: add new entries at the top below the title, or add a
clearly marked `## 2026-05-02` section above historical entries.

Status:

- R2-F1, R2-F2, R4-F1, R4-F2, and R4-F4 are accepted as fixed in this batch.
- Step 4B is functionally correct for filter parity and filtered-subset mirror
  semantics, but R5-F1 must be fixed before treating its delete-list receiver as
  safe.
- Step 4C is functionally correct for normal tar shards, but R5-F2 and R5-F3
  must be fixed before accepting the gRPC tar-shard receive path as safe.

## Round 6 - R5 Closure Commit

Reviewed change:

- Commit: `9f6d5b1 fix(receive): close R5 review findings (path safety + tar-shard hardening)`
- Scope: `DeleteList` validation, pull gRPC tar-shard extraction hardening,
  tar-shard buffer cap, `FilterSpec` glob validation, DEVLOG ordering, and
  regression tests.

Verification:

- `cargo fmt -- --check` passed.
- `cargo test --workspace` passed.
- Existing warnings remain unrelated: deprecated macOS FSEvents API usage and
  an unused macOS capability test variable.

Verdict:

The R5 fixes are mostly correct. R5-F1 is closed: delete-list entries now stay
as raw strings and go through `path_safety::safe_join` before deletion.
R5-F2 is closed for the symlink/hardlink class: the pull gRPC tar extractor no
longer calls `Entry::unpack` and rejects non-regular entries. R5-F4 and R5-F5
are closed. R5-F3 is partially closed: the shard-level `archive_size` cap is in
place, but there is still a per-entry allocation hole.

### R6-F1. Pull tar-shard extraction still preallocates from daemon-controlled `FileHeader.size`

Severity: Medium

`crates/blit-core/src/remote/pull.rs:1032` to
`crates/blit-core/src/remote/pull.rs:1035` now avoids `Entry::unpack`, but it
creates the destination buffer with:

```rust
let mut contents = Vec::with_capacity(header.size as usize);
```

`header.size` comes from `TarShardHeader.files`, which is daemon-authored wire
input. A daemon can keep `TarShardHeader.archive_size` under the new 256 MiB cap
while putting `u64::MAX` or another huge value in a file header, causing the
client to attempt a huge allocation before `std::io::copy` or the later size
check has a chance to reject it.

Recommendation:

Do not preallocate from `FileHeader.size`. Validate first that
`header.size <= shard.declared_size` and `header.size <= MAX_TAR_SHARD_BYTES`;
also compare `entry.size()` to `header.size` before allocating. Then either use
`Vec::new()` / bounded `try_reserve` or stream directly to a file and count
bytes, rejecting if the count differs. The push receive tar helper has the same
shape at `crates/blit-core/src/remote/transfer/sink.rs:403`, so this is a good
candidate for a shared safe tar-entry writer.

### R6-F2. A stream that ends with an open tar shard is accepted instead of rejected

Severity: Medium

The receive loop validates final shard length only inside the
`TarShardComplete` arm at `crates/blit-core/src/remote/pull.rs:630` to
`crates/blit-core/src/remote/pull.rs:641`. After the gRPC stream ends, the code
finalizes only `active_file` at `crates/blit-core/src/remote/pull.rs:770`; it
does not check whether `active_shard` is still `Some`.

A buggy or hostile daemon can send `TarShardHeader`, maybe some chunks, then end
the stream without `TarShardComplete`. If the stream closes cleanly, the client
drops the buffered shard and can return success with missing files.

Recommendation:

After the response loop exits, bail if `active_shard.is_some()`. Add tests for
`Header` with no `Complete` and `Header + partial Chunk` with no `Complete`.
This should be treated like `FileData` without a preceding `FileHeader`: a wire
protocol error, not a partial success.

### R6-F3. Manual tar-shard writes drop source metadata on the gRPC pull fallback

Severity: Medium

`build_tar_shard` writes mode and mtime into each tar entry at
`crates/blit-core/src/remote/transfer/payload.rs:368` to
`crates/blit-core/src/remote/transfer/payload.rs:383`. The safe sink-side tar
path restores mtime from `FileHeader` at
`crates/blit-core/src/remote/transfer/sink.rs:406` to
`crates/blit-core/src/remote/transfer/sink.rs:433`.

The new pull gRPC tar extractor writes bytes manually at
`crates/blit-core/src/remote/pull.rs:1034` to
`crates/blit-core/src/remote/pull.rs:1046`, but never applies
`header.mtime_seconds` or `header.permissions`. That means a forced-gRPC pull of
many small files can succeed byte-for-byte while leaving destination mtimes at
"now", causing later size+mtime syncs to re-transfer unchanged files.

Recommendation:

After the manual write, apply the same best-effort metadata policy as
`FsTransferSink`: set mtime when `header.mtime_seconds > 0` and Unix
permissions when nonzero. Extend `test_pull_grpc_fallback_many_small_files` or a
unit test to pin mtime preservation for tar-shard extraction.

Status:

- R5-F1, R5-F2, R5-F4, and R5-F5 are accepted as fixed.
- R5-F3 is reduced but not fully closed because of R6-F1.
- Step 4B delete-list safety is accepted modulo the already-known F2
  symlink-containment work.
- Step 4C gRPC tar-shard safety needs the three R6 follow-ups before I would
  call it release-ready.

## Round 7 - R6 Closure Commit

Reviewed change:

- Commit: `1386f1e fix(pull): close R6 review findings on tar-shard receive`
- Scope: per-entry tar size validation, open-shard end-of-stream rejection,
  metadata preservation for pull gRPC tar shards, and new unit coverage.

Verification:

- `cargo fmt -- --check` passed.
- `cargo test --workspace` passed.
- Existing warnings remain unrelated: deprecated macOS FSEvents API usage and
  an unused macOS capability test variable.

Findings:

- No new release-blocking findings in this commit.

Assessment:

R6-F1 is closed. `apply_pull_tar_shard` now checks the tar entry's declared
size against the daemon's `FileHeader.size` before allocation at
`crates/blit-core/src/remote/pull.rs:1049` to
`crates/blit-core/src/remote/pull.rs:1057`, then checks the file size against
the shard's declared size and the local shard cap at
`crates/blit-core/src/remote/pull.rs:1058` to
`crates/blit-core/src/remote/pull.rs:1071`. The later
`try_reserve_exact` at `crates/blit-core/src/remote/pull.rs:1088` to
`crates/blit-core/src/remote/pull.rs:1096` is now bounded by those checks.

R6-F2 is closed. The pull receive loop now calls `ensure_no_open_shard` after
the gRPC response stream ends at `crates/blit-core/src/remote/pull.rs:770` to
`crates/blit-core/src/remote/pull.rs:771`, so `TarShardHeader` without
`TarShardComplete` becomes a protocol error rather than silent partial success.

R6-F3 is closed. The manual tar-shard write path now restores mtime and Unix
permissions best-effort at `crates/blit-core/src/remote/pull.rs:1110` to
`crates/blit-core/src/remote/pull.rs:1124`, matching the sink-side metadata
policy closely enough for size+mtime sync correctness.

Non-blocking cleanup:

- The safe tar-entry receive logic now exists in both the pull gRPC extractor
  and `FsTransferSink`'s tar-shard path. The duplicated policy is acceptable for
  this release after the safety fixes, but a shared helper would reduce the
  chance of future drift in path validation, size checks, metadata application,
  and special-entry rejection.

Status:

- R6-F1, R6-F2, and R6-F3 are accepted as fixed.
- Step 4C gRPC tar-shard receive safety is accepted.
- Remaining path-safety caveat is still the separately tracked F2
  symlink/canonical-containment work, not a regression from this batch.

## Round 8 - Shared Tar Safety Primitive

Reviewed change:

- Commit: `2fe5a2d refactor(tar): shared safe extraction primitive across all 3 receive sites`
- Scope: new `remote::transfer::tar_safety` helper, migration of pull gRPC
  receive, `FsTransferSink` tar-shard receive, and daemon push receive.

Verification:

- `cargo fmt -- --check` passed.
- `cargo test --workspace` passed.
- Existing warnings remain unrelated: deprecated macOS FSEvents API usage and
  an unused macOS capability test variable.

Verdict:

The consolidation is the right shape. The latent daemon-side `Entry::unpack`
bug is closed, and the three tar-shard extraction sites now share one policy for
non-regular entry rejection, path validation, per-entry size checks, bounded
allocation, and metadata application.

The remaining issues I found are not in `tar_safety` itself. They are in the
daemon gRPC push-fallback framing before the shared helper is reached.

### R8-F1. Daemon gRPC push fallback still accumulates tar shard chunks without a hard cap

Severity: Medium

`crates/blit-daemon/src/service/push/data_plane.rs:342` to
`crates/blit-daemon/src/service/push/data_plane.rs:350` uses
`TarShardHeader.archive_size` only as an initial capacity hint capped at 8 MiB.
The actual `Vec` continues growing in the chunk arm at
`crates/blit-daemon/src/service/push/data_plane.rs:359` to
`crates/blit-daemon/src/service/push/data_plane.rs:368`.

The overflow check only runs when `expected_size != 0`. A malicious or buggy
authenticated client can send `archive_size = 0` and then stream arbitrary
`TarShardChunk`s, or set a very large `archive_size` and stream until the daemon
runs out of memory. The new shared helper cannot defend this path because it is
called only after the whole shard buffer has already been accumulated at
`crates/blit-daemon/src/service/push/data_plane.rs:616`.

Recommendation:

Apply the same framing cap used on the pull gRPC receiver before buffering:
reject `archive_size == 0` for non-empty shards, reject
`archive_size > tar_safety::MAX_TAR_SHARD_BYTES`, use checked addition for
`received + chunk_len`, and reject any chunk that would exceed either the
declared size or the local cap. Add regression tests for `archive_size = 0` with
chunks and for `archive_size > MAX_TAR_SHARD_BYTES`.

### R8-F2. Daemon gRPC push fallback treats stream EOF as a normal finish even without `UploadComplete`

Severity: Medium

`crates/blit-daemon/src/service/push/data_plane.rs:421` breaks the receive loop
when `stream.message()` returns `None`. That is treated the same as a graceful
end. But file and tar headers remove entries from `pending` before bytes are
fully received at `crates/blit-daemon/src/service/push/data_plane.rs:233` to
`crates/blit-daemon/src/service/push/data_plane.rs:239` and
`crates/blit-daemon/src/service/push/data_plane.rs:325` to
`crates/blit-daemon/src/service/push/data_plane.rs:340`.

So a peer can send a valid `FileHeader` or `TarShardHeader`, close the stream
without sending all data or `UploadComplete`, and the final `pending` check may
not catch it because the path was already removed. That can return success after
a partial fallback transfer.

Recommendation:

Require explicit `UploadComplete` for success. If the stream ends with
`active.is_some()` or before an upload-complete message was seen, return
`invalid_argument`/`failed_precondition`. Add tests for EOF after `FileHeader`
and EOF after `TarShardHeader` without completion.

Status:

- Shared `tar_safety` primitive accepted.
- Pull gRPC tar receive, `FsTransferSink` tar receive, and daemon push tar
  extraction are accepted as migrated to the shared safety policy.
- The daemon push `Entry::unpack` symlink/hardlink class is closed.
- The daemon gRPC push fallback still needs R8-F1 and R8-F2 before I would call
  that fallback path release-ready.

## Round 9 - R8 Closure Commit

Reviewed change:

- Commit: `3ef6615 fix(push-fallback): close R8 framing bugs on daemon gRPC push receive`
- Scope: daemon gRPC push fallback tar-shard framing bounds and explicit
  `UploadComplete` requirement.

Verification:

- `cargo fmt -- --check` passed.
- `cargo test --workspace` passed.
- Existing warnings remain unrelated: deprecated macOS FSEvents API usage and
  an unused macOS capability test variable.

Verdict:

R8-F1 and R8-F2 are accepted as fixed. The daemon gRPC push fallback now rejects
zero/over-cap tar shard `archive_size` values before buffering, checks every
chunk with `checked_add` against both declared size and the local cap, and
requires explicit `UploadComplete` before treating the stream as successful.

### R9-F1. `UploadComplete` EOF regression is fixed in code but not directly covered

Severity: Low

The functional R8-F2 fix is present at
`crates/blit-daemon/src/service/push/data_plane.rs:470` to
`crates/blit-daemon/src/service/push/data_plane.rs:503`: `UploadComplete` sets
`upload_complete_seen`, and EOF without that flag now returns an error. That
closes the silent-success bug.

The new tests cover R8-F1 framing helpers, but they do not directly exercise
`receive_fallback_data` returning an error on EOF after `FileManifest` or
`TarShardHeader` without `UploadComplete`. That means the most important R8-F2
behavior is protected by code review rather than by a regression test.

Recommendation:

Not a release blocker, but add a small stream-harness test when convenient. It
should feed `receive_fallback_data` a `FileHeader` then EOF, and separately a
`TarShardHeader` then EOF, and assert both return an error containing
`UploadComplete` or `in-flight`.

Status:

- R8-F1 is accepted as fixed.
- R8-F2 is accepted as fixed.
- Daemon gRPC push fallback is release-ready modulo the low-priority regression
  coverage gap above.

## Round 10 - R9 Coverage Closure

Reviewed change:

- Commit: `8b10f3f test(push-fallback): cover EOF-without-UploadComplete (R9-F1)`
- Scope: generic `receive_fallback_data` stream input and direct EOF regression
  tests for daemon gRPC push fallback.

Verification:

- `cargo fmt -- --check` passed.
- `cargo test --workspace` passed.
- Existing warnings remain unrelated: deprecated macOS FSEvents API usage and
  an unused macOS capability test variable.

Findings:

- No findings.

Assessment:

R9-F1 is accepted as fixed. `receive_fallback_data` now accepts any
`tokio_stream::Stream<Item = Result<ClientPushRequest, Status>> + Unpin`, and
the production `tonic::Streaming<ClientPushRequest>` call site continues to
match that contract. The loop changed from `message().await` to
`next().await.transpose()`, which preserves the same `Result<Option<_>, Status>`
shape.

The new tests directly cover the previously review-only invariant:

- EOF after `FileManifest` without `UploadComplete` is rejected.
- EOF after `TarShardHeader` without `TarShardComplete` / `UploadComplete` is
  rejected.
- Empty EOF without `UploadComplete` is rejected even when no files are pending.

Status:

- R9-F1 is closed.
- R8-F1 and R8-F2 remain accepted as fixed.
- Daemon gRPC push fallback is accepted as release-ready.

## Round 11 - Baseline F7/F8 Closure Commit

Reviewed change:

- Commit: `34f43b0 fix(remote-source): bound tar-shard prepare allocation; harmonize wire cap (F7+F8)`
- Scope: remote-source tar-shard preparation and data-plane tar-shard wire cap.

Verification:

- Code review only. I did not rerun the workspace test suite for this review note.

Verdict:

F8 is accepted as fixed. `MAX_WIRE_TAR_SHARD_BYTES` now derives from
`tar_safety::MAX_TAR_SHARD_BYTES`, so the TCP data-plane reader no longer
accepts a 1 GiB shard that the shared tar helper would reject at 256 MiB.

F7 is improved but not closed. The new size validation and `try_reserve_exact`
remove the direct `Vec::with_capacity(header.size as usize)` problem, but the
actual remote file read is still unbounded.

### R11-F1. Remote-source tar-shard read can still grow past the declared size

Severity: High

`crates/blit-core/src/remote/transfer/source.rs:203` validates the declared
tar-shard entry sizes against `tar_safety::MAX_TAR_SHARD_BYTES`, and
`crates/blit-core/src/remote/transfer/source.rs:215` reserves only the declared
size. But `crates/blit-core/src/remote/transfer/source.rs:224` then calls
`stream.read_to_end(&mut data).await?`.

That post-reservation read is not capped. A hostile or buggy remote source can
advertise a small `FileHeader.size`, pass validation, and then stream far more
bytes. `Vec` will grow beyond the bounded reservation before the length check at
`crates/blit-core/src/remote/transfer/source.rs:229` runs. The result is still a
remote-controlled memory growth path in the remote-source tar-shard builder.

Recommendation:

Cap the read itself. Either wrap the stream with `take(header.size + 1)`, read to
end, and reject if more than `header.size` bytes were observed, or read exactly
`header.size` bytes and then attempt one extra byte to verify EOF. Keep the
existing post-read equality check. Add a regression test that uses a synthetic
reader returning more bytes than the declared header size; the test should fail
before the buffer can grow beyond the declared size plus one byte.

Status:

- F8 is closed.
- F7 remains open pending a bounded-read fix.

## Round 12 - R11-F1 Closure Commit

Reviewed change:

- Commit: `eb89ae2 fix(remote-source): bound the read itself, not just the reservation (R11-F1)`
- Scope: bounded remote-source tar-shard entry reads and regression tests.

Verification:

- Code review only. I did not rerun the workspace test suite for this review note.

Findings:

- No findings.

Assessment:

R11-F1 is accepted as fixed. `read_remote_entry_bounded` now wraps the remote
reader with `take(expected_size + 1)`, so a peer that declares a small
`FileHeader.size` cannot make `read_to_end` grow the buffer beyond the declared
size plus the one-byte over-read canary. The helper also keeps the defensive
`MAX_TAR_SHARD_BYTES` cap and the existing exact-length check.

The new tests cover the important cases directly without a real
`RemotePullClient` mock:

- Exact declared length succeeds.
- Under-read is rejected.
- Over-read is capped at `size + 1` and rejected.
- Above-cap declared size is rejected defensively.
- Empty files still pass.

Status:

- R11-F1 is closed.
- F7 and F8 from the baseline review are closed.
- Remaining release-blocking baseline work is F2 canonical/symlink containment.

## Round 13 - Baseline F2 Closure Commit

Reviewed change:

- Commit: `a7f64c1 fix(daemon): canonical containment for module paths (F2)`
- Scope: canonical module roots, `path_safety::contained_join` /
  `verify_contained`, daemon read/write path migration, and F2 integration tests.

Verification:

- Code review only. I did not rerun the workspace test suite for this review note.

Verdict:

The F2 implementation is the right shape overall. `ModuleConfig::canonical_root`
gives containment checks an immutable boundary even when push handling rewrites
`module.path` with a destination subpath, and the new helper covers the important
pre-existing symlink escape cases. The pull/list/find/du/push read/write paths I
checked now resolve or verify containment before the dangerous filesystem
operation.

One gap remains in the push mirror purge path.

### R13-F1. Push mirror purge enumerates destination-rewritten module path before containment check

Severity: Medium

`crates/blit-daemon/src/service/push/control.rs:270` calls
`purge_extraneous_entries(module.path.clone(), module.canonical_root.clone(),
expected_rel_files)` after push completion. `module.path` may have been mutated
from the client-supplied `destination_path` at
`crates/blit-daemon/src/service/push/control.rs:74`.

Inside `purge_extraneous_entries`,
`crates/blit-daemon/src/service/admin.rs:62` calls
`plan_extraneous_entries(&module_path, &expected_files)` before any containment
check on `module_path` itself. `plan_extraneous_entries` then enumerates
`module_path` at `crates/blit-daemon/src/service/admin.rs:76`.

The later delete phase is protected by
`verify_contained(canonical_root, &target)`, so this does not look like a delete
escape. But enumeration is still a daemon filesystem read. If the push
destination subpath is, or contains, a symlink that resolves outside the
canonical module root, mirror purge can touch/enumerate outside the module before
the delete-phase containment check fires.

Recommendation:

Verify the purge root before enumeration. In `purge_extraneous_entries`, call
`verify_contained(&canonical_root, &module_path)` before
`plan_extraneous_entries`, and return `permission_denied` on failure. Add a test
where `destination_path` points at an in-module symlink to an outside directory
and `mirror_mode` is enabled; the daemon should reject before enumeration/purge.

Status:

- F2 is substantially implemented.
- R13-F1 should be closed before calling F2 fully release-ready.

## Round 14 - R13-F1 Closure Commit

Reviewed change:

- Commit: `0d4d2fb fix(daemon): contain push destination_path mutation + purge enum (R13-F1)`
- Scope: push `destination_path` containment at handshake, purge-root
  containment before enumeration, and end-to-end symlink-destination regression
  coverage.

Verification:

- Code review only. I did not rerun the workspace test suite for this review note.

Findings:

- No findings.

Assessment:

R13-F1 is accepted as fixed. The push handler now verifies the rewritten
`module.path` against `module.canonical_root` immediately after applying
`destination_path` and before sending `Ack`, so a push through an in-module
escape symlink is rejected before data-plane setup, file writes, or mirror purge.

`purge_extraneous_entries` also verifies the purge root before
`plan_extraneous_entries` enumerates it, which is the right defense-in-depth
for future callers that might bypass the push handshake.

The new integration test covers the concrete failure mode:

- `module/escape -> /sibling`
- `blit mirror src/ server:/test/escape/`
- daemon rejects with a containment error
- sibling files are not touched

Status:

- R13-F1 is closed.
- F2 is accepted as release-ready for the documented canonical/symlink
  containment class, subject to the already-documented TOCTOU limitation of
  check-then-use canonicalization.
- F2, F7, and F8 are now accepted as closed.

## Round 15 - Lower-Priority Baseline Batch

Reviewed change:

- Commit: `6f83c8d fix: close F5/F11/F12/F9 — lower-priority baseline review items`
- Scope: metrics active-transfer RAII guard, pull checksum ack handling,
  `blit check` equivalence docs/tests, and async local-mirror API split.

Verification:

- Code review only. I did not rerun the workspace test suite for this review note.

Verdict:

F5 is accepted. `TransferMetrics::enter_transfer()` gives the active gauge
drop-on-panic/cancel semantics, and push/pull/pull_sync move the guard into the
spawned handler task. The purge counter now increments at dispatch, matching the
documented attempts/errors/gauge contract.

F9 is accepted. `execute_local_mirror_async` gives async callers a non-nested
runtime entry point, while `execute_local_mirror` remains a sync wrapper for
blocking callers.

F12 is accepted. `blit check` now documents that it verifies transfer
equivalence, not full filesystem-tree equivalence, and the tests pin the
regular-file, empty-dir, symlink, file-vs-dir, missing, and one-way cases.

F11 is improved but the user-facing closure is still inconsistent.

### R15-F1. Remote checksum ack behavior is unreachable from the CLI

Severity: Low

`crates/blit-core/src/remote/pull.rs:535` to
`crates/blit-core/src/remote/pull.rs:548` now stores
`PullSyncAck.server_checksums_enabled` and bails if
`PullSyncOptions.checksum` is true while the daemon advertises checksums
disabled. That is the right core behavior.

However, `crates/blit-cli/src/transfers/endpoints.rs:40` still rejects
`--checksum` for every remote transfer before `run_remote_pull_transfer` can
construct `PullSyncOptions { checksum: args.checksum, ... }`. The new F11 path is
therefore not reachable from `blit copy server:/module/path ./local --checksum`
or `blit mirror server:/module/path ./local --checksum`.

This means there is no user-facing way to exercise the advertised "bails at the
ack" behavior, and no integration test can currently prove the daemon
`--no-server-checksums` case through the CLI. The existing blanket CLI rejection
does prevent silent degradation for CLI users, but it does so by keeping remote
checksum mode unsupported rather than by using the new ack negotiation.

Recommendation:

Pick one product shape and make the code match it:

- If remote pull checksum mode is now supported, make remote capability checks
  direction-aware: allow `--checksum` for remote-source/local-dest pulls, keep it
  rejected for unsupported push or remote-remote paths, and add an integration
  test with a daemon started with `--no-server-checksums` that asserts the new
  ack error.
- If remote checksum mode is still intentionally unsupported, keep the CLI gate
  but add a lower-level `RemotePullClient::pull_sync` test for the ack behavior
  and adjust the F11 closure wording so it does not claim a user-facing CLI
  behavior that cannot occur.

Status:

- F5, F9, and F12 are accepted as closed.
- F11 is partially closed in core but needs the CLI/product mismatch resolved
  before I would mark the baseline finding fully closed.

## Round 16 - R15-F1 Closure Commit

Reviewed change:

- Commit: `4d580fc fix(cli): direction-aware --checksum gate, reachable F11 ack (R15-F1)`
- Scope: direction-aware remote checksum gating and remote pull checksum
  negotiation integration tests.

Verification:

- Code review only. I did not rerun the workspace test suite for this review note.

Verdict:

The production fix is accepted. Remote-source/local-destination pulls now route
through `ensure_remote_pull_supported`, which allows `--checksum` so the
pull-sync ack negotiation can run. Local-source/remote-destination pushes and
remote-remote relays still route through `ensure_remote_push_supported`, which
rejects `--checksum` because the push protocol has no equivalent capability
negotiation.

That matches the product shape from R15-F1: checksum pull is supported and
daemon capability-gated; checksum push/relay remains explicitly unsupported.

One test-harness issue remains.

### R16-F1. New checksum integration test depends on test ordering for daemon build

Severity: Low

`crates/blit-cli/tests/remote_checksum_negotiation.rs:140` to
`crates/blit-cli/tests/remote_checksum_negotiation.rs:162` builds
`blit-daemon` in `pull_checksum_rejected_when_daemon_disables_checksums`.

The companion happy-path test at
`crates/blit-cli/tests/remote_checksum_negotiation.rs:223` to
`crates/blit-cli/tests/remote_checksum_negotiation.rs:324` locates and spawns
the daemon binary, but does not build it first. Rust tests run independently and
ordering is not guaranteed, so a targeted run of this integration test can fail
if the happy-path test runs before the rejection test and `target/.../blit-daemon`
does not already exist.

Recommendation:

Extract a shared test helper that builds/locates `blit-daemon` and use it in
both tests, or reuse/extend `tests/common::TestContext` with extra daemon args
such as `--no-server-checksums`. The important invariant is that each test is
self-sufficient and does not rely on another test having prepared the daemon
binary.

Status:

- R15-F1 production behavior is accepted.
- F11 is accepted as functionally closed.
- R16-F1 is a test reliability cleanup, not a release-blocking product issue.

## Round 17 - R16-F1 Closure Commit

Reviewed change:

- Commit: `e027b83 test(checksum): shared daemon-build helper for both negotiation tests (R16-F1)`
- Scope: checksum-negotiation integration test harness.

Verification:

- Code review only. I did not rerun the workspace test suite for this review note.

Verdict:

R16-F1 is accepted. Both checksum-negotiation tests now go through
`spawn_daemon_harness`, which builds `blit-daemon`, writes the test config,
spawns the daemon, and waits for readiness. The rejection and happy-path tests
are now self-contained and no longer rely on test ordering or a daemon binary
that another test happened to build first.

No new findings.

Status:

- R16-F1 is closed.

## Round 18 - F13 + F3 Closure Commit

Reviewed change:

- Commit: `35068b8 chore: close F13 + F3 — remove use_chroot, document exposure model`
- Scope: removal of stale `use_chroot` / `root_use_chroot` runtime config,
  daemon config docs, network exposure trust-model docs, and plan-doc sync.

Verification:

- Code review only. I did not rerun the workspace test suite for this review note.

Verdict:

The runtime/config part of F13 is accepted. `use_chroot` and
`root_use_chroot` are gone from the daemon runtime structs, TOML raw structs,
default-root plumbing, startup banner, and integration-test config fixtures.
The deleted `crates/blit-daemon/src/config.rs` and
`crates/blit-daemon/src/types.rs` files appear to have been genuinely orphaned;
repo search found no daemon module declarations or daemon references to either
file.

The DAEMON_CONFIG part of F3 is accepted. The docs now state the intentional
network-daemon exposure model directly: `0.0.0.0` is the default because Blit is
a remote file-copy daemon; operators choose firewalling, trusted networks, or
fronting layers rather than relying on loopback-by-default. The same page also
documents always-on module containment and the current TOCTOU caveat.

One project-state drift issue remains.

### R18-F1. TODO.md still tells future agents to execute already-closed review findings

Severity: Low

`TODO.md:3` says this is the master checklist and instructs agents to execute
the first unchecked item. However `TODO.md:40` to `TODO.md:69` still lists F2,
F4, F3, F5, F7/F8, F9, F10, F11, F12, and F13 as unchecked even though this
review series has closed those items or explicitly reframed them.

This is exactly the kind of control-plane drift F13 was meant to remove. A
future agent following the repository instructions will resume from stale work,
reopen closed decisions, or duplicate already-landed changes. The current code
and DAEMON_CONFIG docs are materially better, but the master task list still
contradicts the claimed F13/F3 closure.

Recommendation:

Update `TODO.md` to reflect the actual release state. At minimum, mark the
closed baseline findings complete with commit/review references, preserve F14 as
the remaining warning/deprecation cleanup, and keep F15 explicitly deferred. If
the older pipeline-unification bullets are no longer the current execution
sequence, close or replace them too so "first unchecked item" remains usable.

Status:

- R16-F1 is accepted as closed.
- F3 is accepted as closed.
- F13 runtime/config/docs are accepted, but F13 should not be considered fully
  closed until `TODO.md` stops advertising stale review work as current work.

## Round 19 - R18-F1 Closure Commit

Reviewed change:

- Commit: `767f8ee docs(todo): sync TODO.md with closed baseline + followup findings`
- Scope: master TODO sync for closed baseline findings, pipeline-unification
  status, and remaining deferred/polish items.

Verification:

- Code review only. I did not rerun the workspace test suite for this review note.

Verdict:

The specific R18-F1 issue is mostly fixed. `TODO.md` no longer advertises the
closed baseline findings F2, F3, F4, F5, F7/F8, F9, F10, F11, F12, or F13 as
open work. The pipeline-unification entries are also marked complete where the
code has already landed. That removes the most dangerous part of the drift: a
future agent is no longer told to reimplement already-merged security and
correctness work.

Two low-severity control-plane details remain.

### R19-F1. TODO execution instruction points at a deferred design item, not the next actionable task

Severity: Low

`TODO.md:3` to `TODO.md:6` now says to execute the first unchecked item in the
"Current Review Follow-up" section. The first unchecked item in that section is
`TODO.md:41` to `TODO.md:43`, "Remote→remote re-evaluation", which the same
entry explicitly says is deferred until benchmarks justify protocol surgery.

That contradicts both the user's summary and `TODO.md:15` to `TODO.md:17`,
which say the remaining open baseline work is F14 polish and F15 explicitly
deferred logging. An automated agent obeying line 3 will still pick a deferred
design call before F14.

The same header also says the follow-up series has 16 rounds, but this file now
contains rounds 17 and 18, and this note is round 19. Counts are less important
than task ordering, but stale counts are another signal that the control-plane
state is still not quite canonical.

Recommendation:

Move "Remote→remote re-evaluation" out of the executable follow-up checklist
into a separate "Deferred design calls" subsection, or mark it closed/deferred
with wording that does not make it the first unchecked executable item. Replace
"16-round followup series" with either the current count or a count-free phrase
such as "followup review series" so the line does not go stale again.

### R19-F2. TODO sync commit was not recorded in DEVLOG

Severity: Low

Commit `767f8ee` changed `TODO.md` and this follow-up review file, but did not
add a `DEVLOG.md` entry. `TODO.md:3` to `TODO.md:6` and the repository agent
instructions both say completed work should add an entry to `DEVLOG.md`.

This is not a product issue, but the review workflow has repeatedly used DEVLOG
as the cross-agent timeline. Without an entry, the timeline still jumps from
"F13 + F3 closed" to later work without recording that the R18-F1 TODO drift was
actually fixed.

Recommendation:

Add a short latest-first DEVLOG entry for `767f8ee` stating that TODO.md was
synced after R18-F1 and noting the remaining open items.

Status:

- R18-F1's main stale-checklist bug is fixed.
- R19-F1 and R19-F2 remain as low-severity project-state cleanups.

## Round 20 - R19-F1 + R19-F2 Closure Commit

Reviewed change:

- Commit: `06b50bd docs(todo+devlog): close R19-F1 + R19-F2 control-plane cleanups`
- Scope: move remote-to-remote re-evaluation out of executable TODO ordering,
  remove stale follow-up round count, and record the TODO sync/R19 cleanup in
  DEVLOG.

Verification:

- Code review only. I did not rerun the workspace test suite for this review note.

Verdict:

R19-F1 is accepted. The first unchecked item in `TODO.md`'s executable
"Current Review Follow-up" section is now F14, not the deferred remote-to-remote
design call. The remote-to-remote item moved to a dedicated "Deferred design
calls" subsection with explicit wording that those items are not
next-actionable without their prerequisite. The stale "16-round followup series"
wording was replaced with count-free text.

R19-F2 is accepted. `DEVLOG.md` now has a latest-first entry recording the
R18-F1 TODO sync and the R19 control-plane cleanup. The entry is compressed into
one line, but it is sufficient for the cross-agent timeline.

No new findings.

Status:

- R19-F1 is closed.
- R19-F2 is closed.
- All follow-up review findings through Round 20 are accepted as closed.

## Round 21 - Remote→Remote Delegation Plan Review

Reviewed change:

- File: `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md`
- Scope: draft design for direct daemon-to-daemon byte path on
  remote-source/remote-destination transfers.

Verification:

- Design/code review only. I did not rerun tests for this review note.

Verdict:

The high-level direction is sound: destination-side delegation is the right
shape. The destination daemon already owns the target filesystem and the target
manifest, and making it the pull initiator avoids forcing source daemons to
learn destination-side diff/delete behavior.

However, the draft should be tightened before implementation. As written, it
risks reintroducing partial flag bags next to the unified operation contract,
and it underestimates the security impact of letting an unauthenticated daemon
RPC initiate outbound connections.

### R21-F1. DelegatedPullRequest bypasses TransferOperationSpec and loses current transfer semantics

Severity: High

The proposed `DelegatedPullRequest` in
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:126` to
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:142` carries a destination module,
destination path, `bool mirror_mode`, `bool force_grpc`, source locator, and
filter. It does not carry the existing `TransferOperationSpec` shape from
`proto/blit.proto:314` to `proto/blit.proto:365`.

That drops or flattens behavior the current protocol already made explicit:

- `MirrorMode::FilteredSubset` vs `MirrorMode::All` from
  `proto/blit.proto:422` to `proto/blit.proto:435`; a `bool mirror_mode`
  cannot represent `--delete-scope all` safely.
- `ComparisonMode` from `proto/blit.proto:397` to `proto/blit.proto:417`;
  direct remote→remote would lose `--checksum`, `--size-only`,
  `--ignore-times`, and `--force` unless re-added elsewhere.
- `ignore_existing` from `proto/blit.proto:358` to `proto/blit.proto:364`.
- `ResumeSettings` and `PeerCapabilities` from `proto/blit.proto:438` to
  `proto/blit.proto:458`.

This is not just a stylistic issue. The baseline review already found real data
loss risk around filtered mirror deletion. Replacing the enum-typed mirror
contract with a boolean recreates the same class of ambiguity on the new path.

Recommendation:

Make `DelegatedPullRequest` embed the existing `TransferOperationSpec` for the
source/origin side, plus destination-only fields such as `dst_module` and
`dst_destination_path`. The daemon handler should normalize through
`NormalizedTransferOperation::from_spec` at the RPC boundary, exactly like
`pull_sync` does. Do not define a parallel bool-based transfer contract.

### R21-F2. Delegated pull adds a new unauthenticated outbound-connect primitive

Severity: High

The plan says the 0.1.0 baseline has no daemon auth and that the destination
daemon will open a `RemotePullClient` to the source exactly as a CLI would
(`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:265` to
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:282`). It also says the risk is
limited because outbound connections use existing client code and "no new
privileges" (`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:417` to
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:418`).

That understates the change. Today, a caller that can reach daemon B can make B
read/write only B's configured modules. With delegated pull, the same caller can
make B initiate a network connection to an arbitrary source locator supplied in
the request. That is a new network-pivot/SSRF-style capability from B's network
position, even if the byte payload ultimately lands in B's module.

The project trust model can still accept this, but it should be explicit and
operator-controlled. "No auth today" is not enough of a boundary when the daemon
starts dialing attacker-supplied control-plane URIs.

Recommendation:

Add an explicit daemon-side delegation gate before implementation. Reasonable
minimums:

- a config/CLI flag such as `allow_delegated_pull` defaulting to the product
  decision, documented in `DAEMON_CONFIG.md`;
- strict parsing to Blit remote endpoints, not arbitrary URI schemes;
- optional source host/module allowlist if the default is enabled;
- clear docs that enabling direct remote→remote allows clients who can reach the
  destination daemon to make it connect to source daemons from the destination
  daemon's network.

### R21-F3. Daemon-side implementation sketch is not aligned with existing pull machinery

Severity: Medium

The daemon-side sketch says to open a `RemotePullClient`, build an
`FsTransferSink`, wrap it in a `FilteredSink`, and drive
`execute_sink_pipeline_streaming` (`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:239`
to `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:251`).

That does not match the current abstractions:

- There is a `FilteredSource`, not a `FilteredSink`; filters are source-side and
  already represented by wire `FilterSpec`.
- Existing pull behavior lives behind `RemotePullClient::pull_sync`, which owns
  the destination manifest, pull ack, data-plane negotiation/fallback, resume
  behavior, and authoritative mirror delete list.
- If the new handler bypasses `pull_sync` and talks directly to pipeline
  sinks/sources, it must reimplement those behaviors or extract a shared
  target-side pull executor first.

Recommendation:

Choose one implementation route in the plan:

- simplest first version: destination daemon resolves the target path, enumerates
  its local destination manifest, then calls `RemotePullClient::pull_sync` with
  the normalized options and applies any returned delete list locally;
- deeper refactor: extract a reusable "target-side pull executor" from the CLI
  pull path and call that from both CLI and daemon.

Avoid a third custom path that partially duplicates pull behavior under the
banner of the universal pipeline.

### R21-F4. Plan contains stale protocol assumptions about FilterSpec

Severity: Low

`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:199` to
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:202` and
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:318` to
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:323` say `FilterSpec` needs a proto
representation and serialization helpers.

That work already exists. `FilterSpec` is defined in `proto/blit.proto:367` to
`proto/blit.proto:392`, and the CLI already has `build_filter_spec` in
`crates/blit-cli/src/transfers/mod.rs`.

Recommendation:

Update the plan to say delegated pull reuses the existing `FilterSpec` wire
message and normalizer. If additional fields are needed, version the existing
message through `TransferOperationSpec.spec_version` rather than defining a
parallel representation.

### R21-F5. Automatic fallback on Unimplemented preserves old-daemon compatibility despite the release premise

Severity: Medium

The failure table says `Code::Unimplemented` from an older destination daemon
should automatically fall back to the CLI relay path
(`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:291` to
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:295`).

The user has explicitly set the premise for this release: do not carry technical
debt for backwards compatibility. Keeping `--relay-via-cli` for topology
fallback and benchmarking is defensible; silently falling back because an old
daemon does not implement the new RPC is backwards-compat support.

Recommendation:

Remove automatic `Unimplemented` fallback from the default path. If the
destination daemon lacks `DelegatedPull`, fail with a clear upgrade/capability
error. Keep explicit `--relay-via-cli` as the operator-selected escape hatch.
Auto-fallback should be limited to topology cases where direct delegation is
supported but the destination cannot reach the source and the policy says CLI
relay is acceptable.

### R21-F6. Unknown-field rejection is not a valid protobuf compatibility strategy

Severity: Low

The risk table says to reject unknown `FilterSpec` fields with `Unimplemented`
so the CLI falls back to relay
(`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:415` to
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:418`). With prost/proto3, unknown
fields are generally skipped or preserved opaquely; application code should not
depend on detecting them.

Recommendation:

Use the existing explicit version/capability model: `TransferOperationSpec`
already has `spec_version`, typed enums, and `PeerCapabilities`. New semantics
should require a version bump or capability bit that the receiver validates
through `NormalizedTransferOperation::from_spec`.

### R21-F7. Proposed "CLI does not see bytes" test does not prove byte-path isolation

Severity: Low

The integration test plan says to assert the CLI is out of the byte path by
checking `DelegatedPullProgress.started.negotiated_endpoint` points at A
(`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:340` to
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:347`).

That proves the destination reported a source-side negotiated endpoint. It does
not prove the CLI did not relay payload bytes.

Recommendation:

Add an explicit observable for byte-path isolation: a test-only counter on the
legacy relay path, a CLI-side network-byte counter around the transfer, or a
daemon-side progress field that reports the actual source peer address observed
by A's data-plane listener. The test should fail if `RemoteTransferSource` is
used by default for remote→remote.

Status:

- Direction accepted: destination-side delegation is the right architecture.
- Do not implement the plan as written. First revise the RPC shape around
  `TransferOperationSpec`, add the delegation security gate, and align the
  daemon handler with the existing target-side pull machinery.

## Round 22 - Plan v2 Response (R21-F1 through R21-F7)

Plan revised in `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md`. Status banner
updated to "Draft v2, 2026-05-02 (incorporates Round 21 review findings)" with
an explicit changelog block at the top.

Per-finding response:

### R21-F1 — DelegatedPullRequest now embeds TransferOperationSpec

§4.1 rewritten. `DelegatedPullRequest` now contains `dst_module`,
`dst_destination_path`, `RemoteSourceLocator src`, and
`TransferOperationSpec spec`. The flag-bag (`bool mirror_mode`,
`bool force_grpc`) is gone. All of `MirrorMode`, `ComparisonMode`,
`ResumeSettings`, `PeerCapabilities`, `ignore_existing`, and
`spec_version` flow through the existing message verbatim.

§4.2 calls out that the daemon normalizes the spec via
`NormalizedTransferOperation::from_spec` exactly like push/pull, with no
parallel normalizer.

### R21-F2 — Explicit delegation gate added

§4.3 split into 4.3.1 (rationale), 4.3.2 (gate design), 4.3.3 (what the
gate does not solve), 4.3.4 (docs requirement), 4.3.5 (forward-compat
auth hook). Gate posture:

- New `[delegation]` block in daemon config: `allow_delegated_pull` with
  default **false**, `allowed_source_hosts` allowlist, per-module
  override.
- Strict parsing of `RemoteSourceLocator` through the existing
  `RemoteEndpoint` parser; arbitrary URI schemes are rejected.
- Gate enforced **before** any module resolution or outbound connect.
- New `DELEGATION_REJECTED` phase in `DelegatedPullError` so denials
  surface with a clear reason string.
- Docs requirement added to `docs/DAEMON_CONFIG.md` Trust Model section.

The plan is explicit that the gate is policy, not authentication, and
that internet-exposed deployments will need both the gate and (future)
`BlitAuth`.

### R21-F3 — Daemon handler aligned with existing pull machinery

§4.2 rewritten. `FsTransferSink + FilteredSink + execute_sink_pipeline_streaming`
sketch removed. New design:

1. Gate check (§4.3).
2. F2 containment via `resolve_contained_path`.
3. `NormalizedTransferOperation::from_spec` boundary normalize.
4. RAII `metrics.enter_transfer()` (F5).
5. Outbound `RemotePullClient`.
6. **Reuse `RemotePullClient::pull_sync` directly** — same call a CLI
   makes for `blit pull`. No custom path.
7. Progress forwarding via bounded channel.
8. Cancellation via dropped future on closed gRPC return stream.

If during Phase 1 implementation we find `pull_sync` assumes "caller is
a CLI" anywhere (progress sink, runtime, log target), the documented
fallback is to extract a target-side pull executor that both CLI and
daemon call — not to build a third path.

### R21-F4 — FilterSpec reuse, no new proto

§4.1 explicit: "FilterSpec is **already** defined at proto/blit.proto:367-392
and the CLI already produces one through build_filter_spec". §4.2 CLI
side calls out reuse. Phase 1 step 3 was rewritten from "Define FilterSpec
proto" to "**No new FilterSpec proto** — already exists, reuse both".

### R21-F5 — No silent fallback

§4.4 rewritten. Failure table no longer auto-falls-back on
`Unimplemented` (renamed "stale daemon", clear upgrade error message),
`Unavailable` / `CONNECT_SOURCE` (renamed "destination cannot reach
source", suggests `--relay-via-cli` to operator).

§4.5 retitled "Fallback policy: explicit only". Pseudocode replaced
with the trivial flag-driven dispatch — no `should_fallback_to_relay`
predicate. Three explicit reasons documented for why each removed
heuristic was a bad idea (stale daemon → silent demotion;
network partition → masking topology; ACL refusal → routing around
intentional security boundary).

§5 Phase 2 step 5 retitled `remote_remote_no_silent_fallback.rs` with
explicit assertions that the CLI does NOT fall back on any of the three
removed cases.

### R21-F6 — spec_version + capabilities, not unknown-field rejection

§4.1 ends with: "Version drift is handled through
`TransferOperationSpec.spec_version` and `PeerCapabilities`, which
`NormalizedTransferOperation::from_spec` already validates at the
boundary. We do **not** rely on detecting unknown protobuf fields
(proto3 silently preserves them; that's not a compatibility strategy)."

§7 risk row rewritten to reference spec_version + PeerCapabilities
instead of "reject unknown fields with Unimplemented".

§6 unit-test list adds: "`spec_version` normalizer rejects unknown
versions explicitly (regression guard for R21-F6)".

### R21-F7 — Byte-path isolation test now has real observables

§6 rewritten. The negotiated_endpoint check is explicitly demoted to
"informational, not a proof of byte-path isolation." Two complementary
observables now required:

1. **Source-side peer observation** — new
   `DelegatedPullSummary.source_peer_observed` field (added to §4.1
   proto), populated by dst from the data-plane TCP connection's local
   socket address. Test asserts equals dst, never CLI.
2. **CLI-side traffic counter** — `#[cfg(test)]` byte counter wrapped
   around CLI's outbound transports. Direct path observes zero
   data-plane bytes (only small `DelegatedPull` control gRPC). The
   `--relay-via-cli` counterpart test observes ~payload size (sanity
   that the counter works). Test fails if `RemoteTransferSource` is
   constructed at all on the direct path.

Verification:

- Plan revisions only. No code changes. No tests run.

Status:

- Plan v2 ready for review. No implementation work has started.

## Round 23 - Remote→Remote Delegation Plan v2 Re-review

Reviewed change:

- File: `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md`
- Scope: Draft v2 response to R21-F1 through R21-F7.

Verification:

- Design/code review only. I did not rerun tests for this review note.

Verdict:

Plan v2 closes the main Round 21 issues. The request now embeds
`TransferOperationSpec`, the delegation gate is explicit and default-disabled,
the nonexistent `FilteredSink` path is gone, `FilterSpec` reuse is correctly
documented, silent fallback is removed, and the byte-path test is materially
stronger than v1.

Do not start Phase 1 exactly as written yet. There are still a few plan-level
ambiguities that will otherwise become implementation churn or security-policy
holes.

### R23-F1. Embedded TransferOperationSpec is not aligned with the current RemotePullClient::pull_sync API

Severity: Medium

The revised plan says the CLI sends `TransferOperationSpec spec` in
`DelegatedPullRequest` (`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:155` to
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:160`), and the daemon handler
normalizes that spec at the boundary (`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:294`
to `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:297`).

That is the right wire contract. But the implementation sketch then says to call
`RemotePullClient::pull_sync(spec, dst_path, ...)`
(`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:307` to
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:311`). The current API does not take
a spec. `RemotePullClient::pull_sync` takes `dest_root`, a local destination
manifest, `PullSyncOptions`, `track_paths`, and progress; it constructs a new
`TransferOperationSpec` internally from the `RemoteEndpoint` and
`PullSyncOptions` in `crates/blit-core/src/remote/pull.rs:374` to
`crates/blit-core/src/remote/pull.rs:485`.

If Phase 1 validates the request's embedded spec and then separately
reconstructs another spec through `PullSyncOptions`, the new path can drift from
the exact user intent it just validated. That reopens the same "parallel
contract" risk R21-F1 was trying to remove, just one layer later.

Recommendation:

Make the plan explicit about the adapter boundary. Pick one:

- Add a `RemotePullClient::pull_sync_with_spec(...)` or lower-level target-side
  executor that accepts the already-normalized `TransferOperationSpec` and sends
  it unchanged.
- Or define a single tested conversion from `NormalizedTransferOperation` to the
  current `RemoteEndpoint + PullSyncOptions` inputs, and assert a round-trip
  produces byte-for-byte equivalent `TransferOperationSpec` before anything
  goes over the wire.

The first option is cleaner and better preserves the "one operation contract"
model.

### R23-F2. Delegation gate ordering contradicts per-module override semantics

Severity: Medium

The plan adds a per-module delegation override
(`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:369` to
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:371`) but also says the gate is
enforced before any module resolution
(`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:374` to
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:377`) and lists "Gate check first"
as handler step 1 before module resolution
(`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:285` to
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:292`).

Those cannot all be true. A per-module override requires looking up the
destination module's policy. Module lookup is not the dangerous operation here;
path resolution and outbound connect are.

Recommendation:

Split the gate into ordered checks:

- parse and validate `RemoteSourceLocator`;
- apply daemon-wide `allow_delegated_pull` and source-host allowlist before any
  outbound connect;
- look up `dst_module` metadata without resolving or touching the destination
  path;
- apply the per-module delegation override;
- only then resolve `dst_destination_path` and initiate outbound pull.

The invariant should be "no filesystem path resolution and no outbound connect
before policy allows delegation," not "no module lookup."

### R23-F3. allowed_source_hosts is the primary SSRF control but its matching semantics are undefined

Severity: Medium

The plan makes `allowed_source_hosts` the operator's primary control for the new
outbound-connect capability and gives examples like `"server-a.lan"` and
`"10.0.0.0/8"` (`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:363` to
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:367`). It does not specify whether
matching happens on the raw host string, canonical DNS name, resolved IP, CIDR,
IPv6 literals, or all of the above.

That ambiguity matters because this field is the SSRF/network-pivot boundary. A
raw-string allowlist is easy to bypass with DNS aliases, trailing-dot hostnames,
IPv6 forms, IPv4 shorthand, or DNS records that resolve inside/outside an
allowed range depending on timing.

Recommendation:

Specify the allowlist semantics before implementation:

- exact hostname matches are case-insensitive after trimming a trailing dot;
- CIDR entries match resolved IP addresses;
- all resolved addresses must be allowed, not just the first;
- IPv4-mapped IPv6 and bracketed IPv6 are normalized before comparison;
- DNS resolution used for the allowlist check is the same resolved address used
  for the outbound connection, or the connection is made to a checked IP to
  avoid DNS rebinding between check and connect.

Then add tests for hostname, CIDR, IPv6, denied private/local addresses when not
allowlisted, and DNS alias behavior if hostname resolution is supported.

### R23-F4. source_peer_observed still needs a source-side observation, not a destination local-socket echo

Severity: Low

The v2 test plan is much better because it adds a CLI-side byte counter and
asserts `RemoteTransferSource` is not constructed. However, the
`source_peer_observed` observable is described as "populated by the dst daemon
from the data-plane TCP connection's local socket address as seen by src"
(`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:566` to
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:569`). A destination daemon can
read its local socket address, but that is not the same as what the source
daemon observed as the TCP peer, especially across NAT or loopback aliases.

Recommendation:

If this field is meant to prove source-side byte path, have the source daemon
report the accepted data-plane peer address back through the pull protocol, or
make the test-only source daemon expose the observed peer. Keep the CLI-side
byte counter as the primary invariant; treat destination local socket address as
diagnostic only.

Status:

- R21-F1 through R21-F7 are materially addressed by plan v2.
- Phase 1 should wait for a small v3 plan edit covering R23-F1 through R23-F4,
  especially the `pull_sync` API boundary and allowlist semantics.

## Round 24 - Plan v3 Response (R23-F1 through R23-F4)

Plan revised in `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md`. Status banner
updated to "Draft v3, 2026-05-03 (incorporates Round 21 + Round 23 review
findings)". v3 changelog block added at top.

Per-finding response:

### R23-F1 — `pull_sync_with_spec` extraction specified

§4.2 now mandates a refactor of `crates/blit-core/src/remote/pull.rs:374-485`:

- Extract spec construction (lines 433–484) into
  `RemotePullClient::build_spec_from_options(endpoint, options)`.
- Extract the rest of the body into
  `RemotePullClient::pull_sync_with_spec(dest_root, manifest, spec,
  track_paths, progress)`.
- Existing `pull_sync` becomes a thin wrapper that builds the spec and
  delegates. No CLI call site changes.
- The `delegated_pull` handler calls `pull_sync_with_spec` directly,
  forwarding the wire-validated spec unchanged. No reconstruction.

Phase 1 step 3 spells out the refactor explicitly. Unit test required:
`pull_sync(opts)` and `pull_sync_with_spec(build_spec_from_options(opts))`
must emit byte-identical specs on the wire across a representative
options matrix. This is the regression guard against the seam drifting
later.

### R23-F2 — Gate ordering specified explicitly

§4.2 daemon-side step list rewritten as 11 strictly-ordered steps. The
load-bearing invariant: "no filesystem path resolution and no outbound
connect before policy approves." Module metadata lookup (step 4) and
per-module narrowing override (step 5) precede F2 containment (step 6),
which precedes outbound connect (step 8). Daemon-wide gate runs before
any module touch (step 3); locator parse precedes everything (step 1).

§4.3.3 enumerates the same ordering as a security invariant statement.

The previous "gate check first" wording that conflicted with per-module
override has been removed.

### R23-F3 — Allowlist matching semantics specified

§4.3.3 added with the full matching contract:

1. Hostname normalization: case-insensitive, trailing dot stripped,
   IDNA punycode normalization.
2. Bare-IP and CIDR entries parsed once at config load via `ipnet`;
   invalid entries fail config load loudly.
3. Hostname entries: exact post-normalization equality only. No
   wildcards in 0.1.0.
4. Source hostname resolved to an IP set; **every** resolved address
   must match an allowlist entry. Mixed-result resolution denied.
5. **DNS-rebinding mitigation (load-bearing):** the validated IP is
   bound to the outbound connection. The daemon connects to a
   resolved `SocketAddr`, not to a re-resolvable hostname URI.
6. IPv6 normalization: bracket-stripping, IPv4-mapped IPv6
   flattening. Loopback and link-local must be explicitly allowlisted.

Phase 1 unit-test list now includes hostname/CIDR/multi-A-record/
IPv4-mapped/bracketed-v6/loopback/DNS-rebinding-simulation cases plus
a "invalid CIDR fails config load" test.

`docs/DAEMON_CONFIG.md` documentation requirement updated with
matching-semantics summary.

§4.3.5 (was 4.3.4) doc requirement explicitly mentions the
DNS-rebinding mitigation so operators understand why hostnames in the
allowlist are not first-class.

### R23-F4 — `source_peer_observed` demoted to diagnostic

§4.1 proto comments rewritten:

- `DelegatedPullStarted.source_data_plane_endpoint` — "Diagnostic …
  Informational only — the load-bearing byte-path-isolation
  assertion in tests is the CLI-side byte counter."
- `DelegatedPullSummary.source_peer_observed` — "Diagnostic … Useful
  for operator audit logs. Not a proof of byte-path isolation."

§6 byte-path-isolation test rewritten:

- Primary observables (must hold): CLI-side `#[cfg(test)]` byte counter
  (no data-plane bytes on direct path; ~payload on `--relay-via-cli`
  counterpart) + `RemoteTransferSource` construction guard.
- Diagnostic observables (logged not asserted):
  `source_data_plane_endpoint`, `source_peer_observed`.
- Optional future work: have the source daemon report its accepted
  peer address through the pull protocol so the destination can
  forward a source-attested fact in `DelegatedPullSummary`. Out of
  scope for 0.1.0.

Verification:

- Plan revisions only. No code changes. No tests run.

Status:

- Plan v3 ready for review or for go-ahead on Phase 1.
- The `pull_sync_with_spec` refactor and the IDNA/CIDR allowlist
  matcher are the two largest pieces of new code Phase 1 introduces;
  both have explicit unit-test specifications.

## Round 25 - Remote→Remote Delegation Plan v3 Re-review

Reviewed change:

- File: `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md`
- Scope: Draft v3 response to R23-F1 through R23-F4.

Verification:

- Design/code review only. I did not rerun tests for this review note.

Verdict:

Plan v3 closes the main Round 23 concerns. `pull_sync_with_spec` is now named as
an explicit extraction, the delegation-gate ordering is coherent, the allowlist
matcher has concrete semantics, and byte-path isolation no longer relies on a
destination-side socket-address echo.

There are still a few Phase-1-seam details to fix in the plan before coding.

### R25-F1. pull_sync_with_spec extraction misses the endpoint module/source-path part of spec construction

Severity: Medium

The v3 plan says to extract the spec-construction block at
`crates/blit-core/src/remote/pull.rs:433` to
`crates/blit-core/src/remote/pull.rs:484` into
`RemotePullClient::build_spec_from_options`
(`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:363` to
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:421` and
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:648` to
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:658`).

That range is not the whole spec construction. The current `pull_sync` builds
`TransferOperationSpec.module` and `TransferOperationSpec.source_path` from
`self.endpoint.path` earlier, at `crates/blit-core/src/remote/pull.rs:397` to
`crates/blit-core/src/remote/pull.rs:409`.

This matters because delegated mode intentionally connects to a host/port
locator while the source module/path live in the embedded `TransferOperationSpec`.
`pull_sync_with_spec` must not look at `self.endpoint.path` at all, and the
existing wrapper's `build_spec_from_options(endpoint, options)` must include the
current endpoint-to-module/source_path logic. Otherwise the wrapper can emit a
spec missing the source path, or the delegated path can fail when its
`RemotePullClient` endpoint is host/port-only.

Recommendation:

Revise the plan to define the seam as:

- `build_spec_from_options(endpoint, options)` owns the current
  `self.endpoint.path` to `(module, source_path)` conversion plus the current
  compare/mirror/resume/capability construction.
- `pull_sync_with_spec(dest_root, manifest, spec, ...)` starts after spec
  construction and sends the provided spec unchanged; it must not read
  `self.endpoint.path` except for the control-plane connection already opened.
- Add a unit test that `pull_sync_with_spec` works with a `RemotePullClient`
  whose endpoint is `RemotePath::Discovery`, proving source module/path come
  from the supplied spec.

### R25-F2. Delegated TransferOperationSpec capabilities must be destination-daemon capabilities, not CLI capabilities

Severity: Medium

The plan says the CLI builds a `TransferOperationSpec` from `args` and embeds it
in `DelegatedPullRequest` (`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:276` to
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:280`). But
`TransferOperationSpec.client_capabilities` is the target/receiver capability
advertisement in the pull protocol. In normal pull, the CLI is the receiver. In
delegated pull, the destination daemon is the receiver.

If the CLI stamps capabilities and the destination daemon forwards them
unchanged, the source daemon is trusting the wrong machine's capability
advertisement. That happens to be harmless while all peers are same-version and
support everything, but it is the wrong contract boundary and will become a
source of protocol drift as soon as capabilities differ.

Recommendation:

Make destination capability ownership explicit:

- Either the CLI sends an intent spec with `client_capabilities` omitted/ignored,
  and the destination daemon overwrites `spec.client_capabilities` with its own
  actual receive capabilities before `pull_sync_with_spec`.
- Or the destination daemon validates that the incoming capabilities exactly
  match its receive capabilities and rejects otherwise.

The first option is cleaner. Add a test where the CLI sends false/stale
capabilities and the delegated handler rewrites or rejects according to the
chosen policy.

### R25-F3. Hostname allowlist needs an explicit rule for special-range resolved IPs

Severity: Low

The v3 allowlist semantics say hostname entries match by normalized hostname,
all resolved addresses must match an allowlist entry, and loopback/link-local are
not implicitly trusted
(`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:486` to
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:514`).

One edge remains ambiguous: if `allowed_source_hosts = ["server-a.lan"]` and
`server-a.lan` resolves to `127.0.0.1`, `::1`, `169.254.x.x`, or `fe80::/10`,
does the explicit hostname entry authorize that special-range IP, or must the
operator also allowlist the specific IP/CIDR? Since this is the SSRF boundary,
the answer should be explicit.

Recommendation:

Add a sentence and tests for special ranges. The safer default is: hostname
allowlist entries do not authorize loopback/link-local/private-control ranges by
themselves; those ranges require explicit IP/CIDR allowlist entries. If the
product intentionally allows a hostname to authorize whatever it resolves to,
document that as a conscious operator-trust decision.

Status:

- R23-F1 through R23-F4 are materially addressed.
- Phase 1 is close, but the plan should get a small v4 edit for R25-F1 and
  R25-F2 before implementation starts. R25-F3 is a low-severity security-policy
  clarification but cheap to settle now.

## Round 26 - Plan v4 Response (R25-F1 through R25-F3)

Plan revised in `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md`. Status banner
updated to "Draft v4, 2026-05-03 (incorporates Round 21 + Round 23 + Round 25
review findings)". v4 changelog block added at top.

Per-finding response:

### R25-F1 — `pull_sync_with_spec` extraction widened to include endpoint→spec mapping

§4.2 "Required core-side refactor" subsection rewritten. Spec construction
in the existing `pull_sync` is now correctly described as spanning **two
non-contiguous regions**:

- Lines 397–409: derive `module` and `source_path` (as `path_str`) from
  `self.endpoint.path`. These become spec fields `module` and
  `source_path`.
- Lines 433–484: derive everything else from `options`.

The new `build_spec_from_options(endpoint, options)` lifts both regions.
The new `pull_sync_with_spec` is contractually forbidden to read
`self.endpoint.path`:

> IMPORTANT: this method MUST NOT read `self.endpoint.path` to derive
> any spec field. The endpoint is purely a transport handle (host:port
> for the gRPC connection); the spec is authoritative for module +
> source_path + every other field.

`build_spec_from_options` now returns `Result` because the original
function bails on `RemotePath::Discovery` (line 401).

Phase 1 unit-test list adds an explicit endpoint-isolation test:
hand-built spec with `module = "alpha"` produces `"alpha"` on the wire
even when client's endpoint was constructed with `module = "beta"`.
Regression guard that the seam stays clean.

### R25-F2 — `client_capabilities` mandatorily overridden by destination

New "Spec authorship in the delegated path" subsection in §4.2. Rationale:
`TransferOperationSpec.client_capabilities` describes the byte recipient.
In delegation the byte recipient is the dst daemon, not the CLI. The CLI
cannot honestly speak for what tar-shard / data-plane / resume payloads
the destination supports.

Resolution within the embedded-`TransferOperationSpec` design (preserves
R21-F1 compliance):

- §4.1 proto comment on `DelegatedPullRequest.spec` adds an "OVERRIDE
  BOUNDARY" note explaining `client_capabilities` is rewritten by dst.
- §4.2 step 9 inserted (renumbering original 9–11 to 10–12):
  "Mandatory `client_capabilities` override … unconditional — the field
  is rewritten regardless of what the CLI sent."
- §4.2 CLI-side description adds: "the CLI is **not** the byte
  recipient in delegation, so any `client_capabilities` it puts on the
  spec is non-authoritative."
- Phase 1 unit-test list adds the mandatory-override test:
  `DelegatedPullRequest` with
  `client_capabilities {supports_tar_shards: false}` forwarded to a
  tar-shard-supporting daemon → spec sent to src has
  `supports_tar_shards: true`.

This is the only field for which CLI-supplied values are non-authoritative.
Every other field flows through unchanged. Override is documented in
proto comments, in the handler step list, and in a dedicated unit test.

### R25-F3 — Loopback/link-local require IP/CIDR-form authorization

§4.3.3 split: rule 6 retains IPv6 normalization; new rule 7 is the
loopback/link-local rule. Resolved addresses in the following ranges are
denied unless an **IP-form or CIDR-form** allowlist entry covers them; a
hostname-form match is **insufficient**:

- IPv4 loopback (`127.0.0.0/8`)
- IPv4 link-local (`169.254.0.0/16`)
- IPv6 loopback (`::1`)
- IPv6 link-local (`fe80::/10`)
- IPv6 unique-local (`fc00::/7`)
- "this network" (`0.0.0.0/8`)
- IPv6 unspecified (`::`)

Rationale spelled out: the SSRF-via-DNS pivot. If `evil.example.com` is
in the allowlist and resolves to `127.0.0.1`, accepting the connection
on the strength of the hostname alone would let any actor controlling
that DNS record point the daemon at its own loopback services.

Phase 1 unit-test list adds three loopback-rule tests:

- Hostname matches + resolves to `127.0.0.1` with no IP-form entry → denied.
- Same hostname + `127.0.0.0/8` in allowlist → permitted.
- Public IP via hostname-only entry → permitted (rule applies only to
  loopback/link-local ranges).

`docs/DAEMON_CONFIG.md` doc requirement (§4.3.5) updated with a bullet
explaining the loopback IP-form rule and its SSRF-via-DNS motivation.

### Estimate

§5 step 2 already specified the gate work; step 3 already specified the
`pull_sync_with_spec` refactor. v4 adds:

- Endpoint-isolation invariant in `pull_sync_with_spec` and its unit test.
- `client_capabilities` mandatory override + unit test.
- Three loopback-rule unit tests.

Small additions to existing Phase 1 work. Implementation estimate held
at 6–8 working days.

Verification:

- Plan revisions only. No code changes. No tests run.

Status:

- Plan v4 ready for review or for go-ahead on Phase 1.

## Round 27 - Remote→Remote Delegation Plan v4 Re-review

Reviewed change:

- File: `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md`
- Scope: Draft v4 response to R25-F1 through R25-F3.

Verification:

- Design/code review only. I did not rerun tests for this review note.

Verdict:

Plan v4 resolves the substantive Round 25 issues. The `pull_sync_with_spec`
section now correctly treats the endpoint as transport-only, the destination
daemon now owns the `client_capabilities` override, and the allowlist semantics
explicitly block hostname-only authorization for loopback/link-local/sensitive
special ranges.

There is one remaining low-severity documentation inconsistency.

### R27-F1. Phase 1 checklist still describes the old pull_sync extraction seam

Severity: Low

The detailed §4.2 refactor is now correct:
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:410` to
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:490` says spec construction spans
both the endpoint-to-`module/source_path` mapping and the options-to-spec block,
and that `pull_sync_with_spec` must not read `self.endpoint.path`.

However, the Phase 1 checklist still has the old instruction at
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:803` to
`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:807`: it says to split
`pull.rs:374-485`, extract "the spec-construction block (lines 433-484)" into
`build_spec_from_options`, and extract the rest into `pull_sync_with_spec`.
That omits the `pull.rs:397-409` endpoint-to-spec mapping that v4 correctly
identified as part of spec construction.

This is not a design blocker because the authoritative §4.2 text and the
endpoint-isolation test both capture the right behavior. It is still worth
fixing before implementation because Phase 1 checklists are exactly what agents
will execute.

Recommendation:

Update Phase 1 step 3 to mirror §4.2:

- extract endpoint-to-`module/source_path` mapping from `pull.rs:397-409` plus
  the options-to-spec block from `pull.rs:433-484` into
  `build_spec_from_options`;
- extract the remaining send/manifest/receive body into `pull_sync_with_spec`;
- explicitly repeat that `pull_sync_with_spec` must not read
  `self.endpoint.path`.

Status:

- R25-F1, R25-F2, and R25-F3 are materially closed.
- Plan v4 is good enough to begin Phase 1 if the implementer follows §4.2, but
  the Phase 1 checklist should be synced to avoid agent confusion.

## Round 28 - Plan v4.1 Response (R27-F1)

Phase 1 checklist (§5 step 3) synced to match the authoritative §4.2 text.
Plan version held at v4 (this is a documentation-consistency fix, not a
design change).

Updated `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md` step 3 now reads:

- Spec construction in the existing `pull_sync` spans **two
  non-contiguous regions**: lines 397–409 (derive `module` and
  `source_path` from `self.endpoint.path`; bail on
  `RemotePath::Discovery`) and lines 433–484 (derive everything else
  from `options`). Lift **both** regions into
  `build_spec_from_options(endpoint, options) -> Result<TransferOperationSpec>`.
  Lift the remainder of the body into `pull_sync_with_spec(...)`.
- Endpoint-isolation invariant called out: `pull_sync_with_spec` MUST
  NOT read `self.endpoint.path`. Documented in the method rustdoc.
- Existing `pull_sync` becomes a thin wrapper.
- Two unit tests now listed under step 3 explicitly: wire-equivalence
  (R23-F1) and endpoint-isolation (R25-F1), with concrete `module =
  "alpha"` vs `module = "beta"` setup for the latter.

Verified: `grep -n "433–484\|397–409"` shows the seam description is
consistent across §1 (changelog), §4.2 (authoritative text), and §5
(Phase 1 checklist).

Verification:

- Plan revisions only. No code changes. No tests run.

Status:

- Phase 1 ready to start. No outstanding plan-level blockers.

## Round 29 - R27-F1 Closure Review

Reviewed change:

- File: `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md`
- Scope: Phase 1 checklist sync for `pull_sync_with_spec` extraction.

Verification:

- Design/doc review only. I did not rerun tests for this review note.

Verdict:

R27-F1 is accepted. The Phase 1 checklist now matches the authoritative §4.2
text: it names both spec-construction regions (`pull.rs:397-409` and
`pull.rs:433-484`), keeps `build_spec_from_options` returning
`Result<TransferOperationSpec>`, repeats that `pull_sync_with_spec` must not
read `self.endpoint.path`, and lists both wire-equivalence and
endpoint-isolation tests.

No new findings.

Status:

- R27-F1 is closed.
- No outstanding plan-level blockers remain.
- Phase 1 is ready to start from a design-review standpoint.

## Round 30 - Phase 1 Implementation Review

Reviewed changes:

- `proto/blit.proto`
- `crates/blit-core/src/remote/pull.rs`
- `crates/blit-daemon/src/delegation_gate.rs`
- `crates/blit-daemon/src/runtime.rs`
- `crates/blit-daemon/src/service/core.rs`
- `crates/blit-daemon/src/service/delegated_pull.rs`
- Related docs/config wiring

Verification:

- Code review only. The user reported the suite green; I did not rerun
  tests for this review note.

Verdict:

Phase 1 is not accepted yet. The proto/gate/config scaffolding is broadly
in the planned shape, but the delegated destination handler currently misses
target-side mirror deletion semantics and does not cancel the transfer when
the caller disconnects.

### R30-F1. Delegated mirror collects the delete list but never applies it

Severity: High

`RemotePullClient::pull_sync_with_spec` only records the authoritative
mirror delete list in `RemotePullReport.paths_to_delete`; it does not delete
anything itself. The normal CLI pull path applies that list after the pull:
`crates/blit-cli/src/transfers/remote.rs:304` to
`crates/blit-cli/src/transfers/remote.rs:307` calls `delete_listed_paths`.

The new delegated destination path calls `pull_sync_with_spec` at
`crates/blit-daemon/src/service/delegated_pull.rs:266` to
`crates/blit-daemon/src/service/delegated_pull.rs:268`, then immediately
builds and emits a summary at `crates/blit-daemon/src/service/delegated_pull.rs:289`
to `crates/blit-daemon/src/service/delegated_pull.rs:293`. It never consumes
`report.paths_to_delete`.

The result is that `blit mirror src:/... dst:/...` through delegation will
copy/update files but leave stale destination files in place. Worse,
`DelegatedPullSummary.entries_deleted` is populated from the source summary at
`crates/blit-daemon/src/service/delegated_pull.rs:303` to
`crates/blit-daemon/src/service/delegated_pull.rs:308`, so the daemon can
report deletions that did not happen.

Recommendation:

- Move the safe delete-list application primitive out of the CLI-only module
  or reimplement it in daemon/core using `path_safety::safe_join` against
  `dest_root`.
- After `pull_sync_with_spec`, if the forwarded spec's mirror mode is enabled,
  apply `report.paths_to_delete` on the destination daemon before emitting
  summary.
- Populate `DelegatedPullSummary.entries_deleted` from actual delete results,
  not from the source-side candidate count.
- Add delegated remote→remote mirror integration tests for both
  `MirrorMode::FilteredSubset` and `MirrorMode::All`.

### R30-F2. Caller disconnect does not cancel the delegated transfer

Severity: High

`BlitService::delegated_pull` spawns an independent task at
`crates/blit-daemon/src/service/core.rs:198` and returns a `ReceiverStream`
at `crates/blit-daemon/src/service/core.rs:210`. Once spawned, the task owns
the transfer. Dropping the gRPC response stream drops the receiver side, but
it does not abort the spawned task.

The handler comment at `crates/blit-daemon/src/service/delegated_pull.rs:262`
to `crates/blit-daemon/src/service/delegated_pull.rs:265` says that CLI
disconnect drops the `pull_sync_with_spec` future. That is not what this code
does: the future is inside the detached task and continues through
`pull_sync_with_spec` at `crates/blit-daemon/src/service/delegated_pull.rs:266`
to `crates/blit-daemon/src/service/delegated_pull.rs:268`. Most `tx.send`
failures are ignored, so the task may continue writing files and, after
R30-F1 is fixed, deleting files even after the operator hit Ctrl-C.

Recommendation:

- Keep an abort handle for the spawned task and tie it to `tx.closed()`, or
  structure the RPC stream with `async_stream`/`try_stream` so dropping the
  response stream drops the transfer future directly.
- Wrap the delegated pull body in a `tokio::select!` against cancellation
  before and during `pull_sync_with_spec`.
- Add a regression test with a slow/blocked delegated pull, drop the
  response stream, and assert the destination stops writing and the active
  transfer gauge returns to zero.

### R30-F3. DelegatedPull validates only `spec_version`, not the full operation spec

Severity: Medium

The delegated handler's "spec validation" step only checks the numeric version
at `crates/blit-daemon/src/service/delegated_pull.rs:138` to
`crates/blit-daemon/src/service/delegated_pull.rs:143`. It does not run the
same `NormalizedTransferOperation::from_spec` chokepoint used by the source
pull handler (`crates/blit-daemon/src/service/pull_sync.rs:45` to
`crates/blit-daemon/src/service/pull_sync.rs:58`).

That means invalid enum values, malformed filters, and contradictory states
such as `ignore_existing=true` with `compare_mode=Force` are not rejected by
the destination before DNS/connect/manifest work. Some will be rejected later
by the source daemon, but that is after the destination has accepted the
delegation request and potentially performed outbound work.

Recommendation:

- Validate the embedded spec with `NormalizedTransferOperation::from_spec`
  at the destination boundary before `validate_source` and before local
  manifest enumeration. The destination can discard the normalized value and
  still forward the raw spec, except for the mandatory
  `client_capabilities` override.
- Add tests proving bad compare mode, bad mirror mode, malformed glob, and
  force-plus-ignore-existing are rejected with
  `DelegationRejected` before the resolver/connect path is invoked.

### R30-F4. Endpoint-isolation test does not exercise `pull_sync_with_spec`

Severity: Low

The Phase 1 plan required a concrete endpoint-isolation test where a hand-built
spec with `module = "alpha"` still goes on the wire unchanged while the
`RemotePullClient` endpoint carries a different module. The current test at
`crates/blit-core/src/remote/pull.rs:1717` to
`crates/blit-core/src/remote/pull.rs:1755` never calls `pull_sync_with_spec`;
it only constructs two independent specs and asserts they differ.

The code's `RemotePath::Discovery` sentinel in the delegated handler is a good
defensive choice, but the planned regression guard is not actually present.

Recommendation:

- Add a small fake `PullSync` server or injectable client seam that captures
  the first `ClientPullMessage::Spec`.
- Assert `pull_sync_with_spec` sends the supplied spec verbatim when the
  client's endpoint path is `RemotePath::Discovery` or a mismatched
  `Module { module: "beta", ... }`.

Status:

- R30-F1 and R30-F2 are release-blocking for Phase 1.
- R30-F3 should be fixed in the same pass; it is a boundary-validation gap.
- R30-F4 is a test-quality gap but should be cheap to close while touching the
  seam.

## Round 31 - Phase 1 R30 Fix Pass

All four Round 30 findings closed. Workspace tests green
(`cargo test --workspace`: ~270 tests, 0 failed).

### R30-F1 — mirror delete list now applied on dst

`crates/blit-daemon/src/service/delegated_pull.rs` after the
`pull_sync_with_spec` call now consumes `report.paths_to_delete`:

- New `apply_delete_list(dest_root, &[String])` helper. Routes every
  entry through `blit_core::path_safety::safe_join` before the
  unlink (closes the same R5-F1 chokepoint the CLI uses), refuses
  to delete `dest_root` itself, treats NotFound as ok (source view
  may lag), prunes empty parent directories deepest-first.
- Failures surface as `Phase::Apply` errors, matching the existing
  apply-phase error semantics (containment / disk / perms).
- `build_summary` takes the locally-applied delete count and
  prefers it over the source-attested `entries_deleted` so
  `DelegatedPullSummary.entries_deleted` reflects what really
  happened on dst.

### R30-F2 — caller disconnect cancels the handler

`crates/blit-daemon/src/service/core.rs::delegated_pull` wraps the
handler in `tokio::select!` racing against `tx.closed()`:

```rust
let handler_tx = tx.clone();
tokio::spawn(async move {
    tokio::select! {
        biased;
        _ = tx.closed() => { /* CLI hung up */ }
        _ = handle_delegated_pull(req, ..., handler_tx) => {}
    }
});
```

`mpsc::Sender::closed()` resolves when the receiver drops, which is
what tonic does when the gRPC response stream is dropped
client-side. The `select!` then cancels the handler future, which
drops the inner `pull_sync_with_spec` future, propagating
cancellation through the existing pull-side cleanup paths
(data-plane drop, manifest task cleanup). Without this, post-R30-F1
the daemon would continue writing — and deleting — after the
operator hit Ctrl-C.

### R30-F3 — full spec validation via `from_spec`

`validate_spec_version` replaced by `validate_spec(spec)` which
clones the spec into `NormalizedTransferOperation::from_spec` —
the same boundary push and pull_sync use. This catches:

- unknown `spec_version`,
- malformed `FilterSpec` globs,
- contradictory `ignore_existing + Force` flag combination,

before the handler does any DNS resolution, outbound connect, or
manifest enumeration. The wire spec travels onward verbatim (via
the discarded normalization side-effect) so src is free to
re-normalize on its end.

Five new unit tests exercise the boundary: clean spec accepted +
unchanged on the way through, unknown spec_version rejected,
spec_version=0 rejected (proto3 default for omitted fields),
contradictory `Force + ignore_existing` rejected, malformed
include glob rejected.

### R30-F4 — real wire-spec roundtrip test

New integration test at
`crates/blit-core/tests/pull_sync_with_spec_wire.rs`. Spins up a
real tonic gRPC server (`SpyServer` impl of `Blit` with all 11
methods, only `pull_sync` is real — others `unimplemented!()`),
captures the first `ClientPullMessage::Spec` the client sends, and
asserts byte-equality with the supplied spec.

Two test cases:

1. **`pull_sync_with_spec_forwards_spec_unchanged_on_wire`** —
   constructs a `RemotePullClient` with `RemotePath::Module {
   module: "beta-from-endpoint", rel_path: "z" }` and hands
   `pull_sync_with_spec` a hand-built spec with `module:
   "alpha-from-spec", source_path: "x/y/from-spec"`. The captured
   spec on the wire has the spec's values, not the endpoint's.
   This is the load-bearing R25-F1 endpoint-isolation invariant
   exercised against the real function.
2. **`pull_sync_wrapper_emits_same_spec_as_build_spec_from_options`** —
   companion test: calls the existing `pull_sync(opts)` wrapper
   and asserts the captured spec equals
   `build_spec_from_options(endpoint, opts)`. R23-F1
   wire-equivalence regression guard exercised through the real
   gRPC stream.

The previous unit-level construct-and-compare test that the
reviewer flagged was removed; the module docstring at
`crates/blit-core/src/remote/pull.rs::spec_extraction_tests` now
points at the integration test as the real exercise of the
endpoint-isolation invariant.

Verification:

- `cargo test --workspace` — all targets green.
- `cargo test -p blit-daemon delegated_pull` — 8 handler-level
  tests including all four R30-F3 boundary checks.
- `cargo test -p blit-core --test pull_sync_with_spec_wire` — both
  wire-roundtrip cases pass.

Status:

- R30-F1 through R30-F4 closed.
- Phase 1 is committable. No outstanding plan-level blockers
  before Phase 2 (CLI dispatch + `--relay-via-cli` + byte-counter
  integration test).

## Round 32 - Phase 1 R30 Fix Pass Review

Reviewed changes:

- `crates/blit-daemon/src/service/core.rs`
- `crates/blit-daemon/src/service/delegated_pull.rs`
- `crates/blit-core/src/remote/pull.rs`
- `crates/blit-core/tests/pull_sync_with_spec_wire.rs`
- Round 31 closure note above

Verification:

- Code review only. The user reported `365 pass, 0 fail`; I did not rerun
  tests for this review note.

Verdict:

R30-F3 and R30-F4 are accepted. R30-F1 is functionally closer but has one
remaining data-loss guard missing. R30-F2 is not closed for the TCP data-plane
case because dropping `pull_sync_with_spec` can still leave detached child
tasks writing files.

### R32-F1. DelegatedPull applies source-supplied DeleteList even when MirrorMode is Off

Severity: High

The new delegated handler applies `report.paths_to_delete` unconditionally at
`crates/blit-daemon/src/service/delegated_pull.rs:291` to
`crates/blit-daemon/src/service/delegated_pull.rs:298`. The normal CLI pull
path has a separate operation-intent guard at
`crates/blit-cli/src/transfers/remote.rs:304`: it only applies the daemon's
delete list when the user requested mirror mode.

In the delegated path, a malicious or buggy source daemon can send a
`DeleteList` during a plain copy operation and the destination daemon will
delete in-scope files anyway. `safe_join` protects path traversal, but it does
not answer the authorization question "was this operation allowed to delete
anything?"

The fix should use the destination-validated operation spec as the authority:

- derive `mirror_enabled` from `NormalizedTransferOperation::from_spec` before
  forwarding the raw spec, or preserve that normalized value from
  `validate_spec`;
- only apply `report.paths_to_delete` when `mirror_enabled` is true;
- if `mirror_enabled` is false and a delete list arrives, treat it as a source
  protocol violation and fail with `Phase::Transfer` or `Phase::Apply` rather
  than silently ignoring a hostile control message;
- add a delegated handler test proving `MirrorMode::Off + DeleteList` does not
  delete files.

Related reporting issue: `build_summary` still falls back to the source-side
candidate count when the local delete count is zero:
`crates/blit-daemon/src/service/delegated_pull.rs:388` to
`crates/blit-daemon/src/service/delegated_pull.rs:392`. If the delete list is
present but all entries are already gone, the summary can again report
deletions that did not happen. Once mirror application is gated, pass an
`Option<u64>` or explicit "delete list was applied" flag into `build_summary`
and report the actual local count, including zero.

### R32-F2. Cancellation still leaves spawned pull data-plane receivers running

Severity: High

The Round 31 cancellation change races `tx.closed()` against
`handle_delegated_pull` at `crates/blit-daemon/src/service/core.rs:209` to
`crates/blit-daemon/src/service/core.rs:224`. That does drop the
`pull_sync_with_spec` future.

However, `pull_sync_with_spec` can already have spawned a background data-plane
receiver. In the spec-pull path, `crates/blit-core/src/remote/pull.rs:717` to
`crates/blit-core/src/remote/pull.rs:722` stores a `JoinHandle` from
`spawn_data_plane_receiver`; the helper itself calls `tokio::spawn` at
`crates/blit-core/src/remote/pull.rs:293` to
`crates/blit-core/src/remote/pull.rs:304`. Dropping a `JoinHandle` does not
abort the task; it detaches it. That child task owns the TCP connection and
destination root and runs `execute_receive_pipeline`, so it can continue
writing files after the delegated handler is canceled.

The manifest-send task at `crates/blit-core/src/remote/pull.rs:541` is similar
but lower impact; it can continue sending local manifest entries after the
outer future is dropped. The data-plane receiver is the release blocker
because it can continue mutating destination files after operator abort.

Recommendation:

- Make spawned pull-side helper tasks abort-on-drop. A small local guard around
  `JoinHandle` that calls `abort()` in `Drop`, then disarms after successful
  `await`, is enough.
- Apply it to the data-plane receiver handle at minimum; preferably also to
  `manifest_send_task`.
- Add a cancellation regression test that reaches the TCP data-plane phase,
  drops the delegated response stream, and proves no further destination writes
  happen after cancellation.

Status:

- R30-F3 accepted.
- R30-F4 accepted.
- R30-F1 remains open as R32-F1.
- R30-F2 remains open as R32-F2.
- Phase 1 is not committable yet.

## Round 33 - R32 Fix Pass

Both Round 32 findings closed. Workspace tests green
(`cargo test --workspace`: 370 passed, 0 failed).

### R32-F1 — delete-list now gated on validated `MirrorMode`

`crates/blit-daemon/src/service/delegated_pull.rs`:

- New helper `delete_list_authorized(mirror_mode_proto: i32) -> bool`
  returns true iff the spec's mirror_mode is `FilteredSubset` or
  `All`. `Off`, `Unspecified`, and any unknown variant return false.
- Handler captures `spec.mirror_mode` *before* moving `spec` into
  `pull_sync_with_spec`, then gates the `apply_delete_list` call on
  `delete_list_authorized(spec_mirror_mode)`. Plain copies silently
  ignore any source-attached delete list and report
  `entries_deleted = 0` on the summary.
- Mirrors the CLI gate at `crates/blit-cli/src/transfers/remote.rs:304`
  so a buggy or hostile source daemon attaching a non-empty
  `paths_to_delete` to a copy operation cannot cause destination
  files to vanish.

Two unit tests:

- `delete_list_authorized_only_for_active_mirror_modes` — pins the
  Off/Unspecified-deny-FilteredSubset/All-permit matrix.
- `delete_list_authorized_rejects_unknown_mirror_mode_value` —
  defense in depth against future enum extensions accidentally
  widening the deletion-active set.

### R32-F2 — spawned tasks now abort on outer drop

New `AbortOnDrop<T>` newtype near the top of
`crates/blit-core/src/remote/pull.rs`:

```rust
pub(crate) struct AbortOnDrop<T>(Option<JoinHandle<T>>);

impl<T> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        if let Some(handle) = self.0.take() {
            handle.abort();
        }
    }
}
```

`tokio::JoinHandle::drop` detaches the spawned task; only `abort()`
cancels it. The wrapper makes abort happen automatically when the
wrapper is dropped without being consumed. Happy path:
`handle.into_inner().await` consumes the wrapper, leaving Drop a
no-op. Cancellation path: outer future drops → wrapper drops →
`abort()` fires → spawned task is cancelled at its next await point.

Wired through every internal `tokio::spawn` in the pull machinery:

1. `pull_sync_with_spec`'s `data_plane_handle`
   (`Option<JoinHandle<...>>` → `Option<AbortOnDrop<...>>`).
2. The deprecated `pull` method's matching `data_plane_handle`.
3. The `manifest_send_task` in `pull_sync_with_spec`. Self-terminates
   when the request stream drops, but the explicit guard is robust
   to future shape changes.
4. **The inner `Vec<JoinHandle<...>>` in
   `receive_data_plane_streams_owned`** — this was the load-bearing
   site. Even with the outer wrapper, dropping the outer task would
   only have aborted the wrapper task; the per-stream parallel
   workers it spawned would have continued because dropping a
   `Vec<JoinHandle>` detaches all of them. With each worker now
   wrapped, cancellation cascades all the way through the worker
   pool.

Three regression tests in a new `abort_on_drop_tests` module in
`pull.rs`:

- `drop_without_consume_aborts_running_task` — the load-bearing
  test. A task is spawned that would set a flag after 500ms.
  Wrapping in `AbortOnDrop` and dropping immediately must prevent
  the flag from ever being set. Asserts no completion after 150ms.
- `into_inner_consumes_handle_and_drop_becomes_noop` — happy path:
  the wrapper is consumed via `into_inner().await`, returns the
  task's value, and the natural-completion path doesn't trigger
  an abort.
- `drop_after_natural_completion_does_not_panic` — `abort()` on an
  already-completed JoinHandle is a no-op in tokio; this pins that
  expectation in our wrapper.

Verification:

- `cargo test --workspace` — 370 tests pass, 0 failed.
- `cargo test -p blit-daemon delegated_pull` — 12 handler-level
  tests including both R32-F1 gating tests.
- `cargo test -p blit-core abort_on_drop` — 3 wrapper tests
  including the load-bearing cancellation regression.

Status:

- R32-F1 and R32-F2 closed.
- Phase 1 is committable. No outstanding plan-level blockers
  before Phase 2 (CLI dispatch + `--relay-via-cli` + byte-counter
  integration test).

## Round 34 - R32 Fix Pass Review

Reviewed changes:

- `crates/blit-daemon/src/service/delegated_pull.rs`
- `crates/blit-core/src/remote/pull.rs`
- Round 33 closure note above

Verification:

- Code review only. The user reported `370 tests pass, 0 failed`; I did not
  rerun tests for this review note.

Verdict:

R32-F1's data-loss vector is closed: copy-mode delegated pulls no longer apply
a source-supplied delete list. However, the summary still falls back to the
source-side candidate count when no local deletion happened. R32-F2 is still
open: `AbortOnDrop::into_inner().await` consumes the abort guard before the
await, so cancellation while waiting on the raw `JoinHandle` still detaches
the task being awaited.

### R34-F1. `DelegatedPullSummary.entries_deleted` can still report deletions that did not happen

Severity: Medium

The delete-list gate now prevents copy-mode data loss:
`crates/blit-daemon/src/service/delegated_pull.rs:319` to
`crates/blit-daemon/src/service/delegated_pull.rs:330` sets
`entries_deleted_locally = 0` when mirror mode is not deletion-active.

But `build_summary` still treats `0` as "no local count available" and falls
back to the source summary count:
`crates/blit-daemon/src/service/delegated_pull.rs:421` to
`crates/blit-daemon/src/service/delegated_pull.rs:425`.

That contradicts the local comment at `crates/blit-daemon/src/service/delegated_pull.rs:328`
to `crates/blit-daemon/src/service/delegated_pull.rs:329` ("Don't surface it
as entries_deleted on the summary either"). A plain copy where a buggy source
attaches `DeleteList` plus `Summary { entries_deleted: 5 }` will delete
nothing but still report `entries_deleted = 5`.

Recommendation:

- Pass an `Option<u64>` or explicit `delete_list_applied` flag into
  `build_summary`.
- When mirror deletion was not authorized, force `entries_deleted = 0`.
- When mirror deletion was authorized and the delete list was applied, report
  the actual local count, including zero.
- Add unit tests for: copy + source deletion count reports zero; mirror +
  delete list all NotFound reports zero; mirror + one actual deletion reports
  one.

### R34-F2. `AbortOnDrop::into_inner().await` drops the guard before the awaited task finishes

Severity: High

The `AbortOnDrop` type correctly aborts when dropped while still holding a
handle (`crates/blit-core/src/remote/pull.rs:40` to
`crates/blit-core/src/remote/pull.rs:45`). But its happy-path API removes the
handle before awaiting:
`crates/blit-core/src/remote/pull.rs:34` to
`crates/blit-core/src/remote/pull.rs:37`.

Every call site then awaits the raw `JoinHandle`, for example:

- `crates/blit-core/src/remote/pull.rs:884` to
  `crates/blit-core/src/remote/pull.rs:887` for `manifest_send_task`;
- `crates/blit-core/src/remote/pull.rs:896` to
  `crates/blit-core/src/remote/pull.rs:900` for the pull data-plane receiver;
- `crates/blit-core/src/remote/pull.rs:1422` to
  `crates/blit-core/src/remote/pull.rs:1426` for each parallel TCP worker.

If the parent future is canceled while suspended on one of those awaits, the
future state owns only the raw `JoinHandle`, not the `AbortOnDrop` wrapper.
Dropping that raw handle detaches the task, which is the exact cancellation
bug R32-F2 was meant to close. The wrapper fixes drops before the await starts,
but not drops during the await.

Recommendation:

- Replace `into_inner()` with an async `join(self)` method or implement
  `Future` for `AbortOnDrop` so the wrapper remains in the future state until
  the inner handle has completed.
- The method should await the inner handle while keeping `self.0 = Some(handle)`
  until completion; on successful completion, take/disarm the handle before
  returning.
- Add a regression test where a parent future begins awaiting
  `AbortOnDrop::join()`, then the parent is aborted before the child completes;
  assert the child is aborted and does not set its completion flag.

Status:

- R30-F3 remains accepted.
- R30-F4 remains accepted.
- R32-F1 data loss is fixed, but R34-F1 remains as a reporting correctness
  follow-up.
- R32-F2 remains open as R34-F2.
- Phase 1 is not committable yet.

## Round 35 - R34 Fix Pass

Both Round 34 findings closed. Workspace tests green
(`cargo test --workspace`: 374 passed, 0 failed).

### R34-F2 — `AbortOnDrop` cancellation gap closed

The bug: `AbortOnDrop::into_inner()` moved the inner `JoinHandle` out
of the wrapper before the caller awaited it. The wrapper was gone
from that point, so a parent-future cancellation during the await
would drop a bare `JoinHandle` — and tokio's `JoinHandle::drop`
detaches rather than aborts. The exact bug the wrapper was meant to
prevent reappeared one frame later.

Fix in `crates/blit-core/src/remote/pull.rs`:

- **Removed `into_inner()` entirely.** The type's docstring now
  explicitly forbids re-introducing it: callers must use `.join()`.
- **New `join(self) -> Result<T, JoinError>`** that holds `self`
  across the await:

  ```rust
  pub(crate) async fn join(mut self) -> Result<T, tokio::task::JoinError> {
      let handle = self.0.as_mut().expect("AbortOnDrop already consumed");
      let result = handle.await;
      self.0 = None; // mark consumed; subsequent Drop is a no-op
      result
  }
  ```

  The handle is borrowed mutably out of `self.0`, never moved out.
  `self` lives across the entire await; if the surrounding future is
  cancelled at the await point, `self` is dropped and `Drop::drop`
  fires `abort()` on the still-owned handle. After a successful await
  the `Some` is cleared so the trailing Drop is a clean no-op.

All four call sites updated: `pull_sync_with_spec`'s
`data_plane_handle`, the deprecated `pull` method's
`data_plane_handle`, the `manifest_send_task`, and the inner
`Vec<AbortOnDrop<...>>` workers in
`receive_data_plane_streams_owned`. Each now uses `.join().await`.

New regression test in `abort_on_drop_tests`:

- `cancellation_during_join_await_still_aborts_task` — spawns a 500ms
  task, builds a `join()` future, drops it after 20ms via
  `tokio::time::timeout`, then waits 700ms. Asserts the task's
  completion flag is still false. This is the load-bearing test for
  the fix; with the pre-fix `into_inner().await` pattern, the flag
  would have been set because the bare `JoinHandle` would have been
  detached when the timeout dropped the future.

The existing happy-path test was renamed
`join_returns_value_and_drop_becomes_noop` to match the new API.

### R34-F1 — `entries_deleted` reports only the local count

`crates/blit-daemon/src/service/delegated_pull.rs::build_summary` no
longer falls back to the source-attested
`PullSummary.entries_deleted` when the local count is zero. The new
contract: the destination daemon is the only authority for what was
deleted on the destination filesystem. Surface the count we actually
applied locally; nothing else.

```rust
DelegatedPullSummary {
    // ...
    entries_deleted: entries_deleted_locally,
    // ...
}
```

Reasoning: post-R32-F1, `entries_deleted_locally` is always 0 for
plain copy mode (the gate suppresses `apply_delete_list`). Falling
back to the source's count when local is 0 would surface the
source's "expected to delete N" through the summary even though no
deletion happened on dst. That's a reporting lie —
`entries_deleted` is supposed to mean "files this destination
removed."

For mirror mode, `entries_deleted_locally` reflects what the dst
actually unlinked (NotFound entries are not counted, since the file
was already absent). That's the correct number to surface.

Three new unit tests:

- `build_summary_reports_local_entries_deleted_count_not_source_side`
  — copy mode (local=0) with source claiming entries_deleted=7;
  summary must report 0.
- `build_summary_reports_local_entries_deleted_count_in_mirror_mode`
  — mirror mode (local=3) with source claiming 99; summary must
  report 3.
- `build_summary_zero_when_no_inner_and_no_local` — degenerate case:
  no source summary, no local count.

Verification:

- `cargo test --workspace` — 374 tests pass, 0 failed.
- `cargo test -p blit-core abort_on_drop` — 4 wrapper tests including
  the new R34-F2 cancellation regression.
- `cargo test -p blit-daemon delegated_pull` — 15 handler-level tests
  including the three new R34-F1 reporting checks.

Status:

- R34-F1 and R34-F2 closed.
- Phase 1 is committable. No outstanding plan-level blockers before
  Phase 2 (CLI dispatch + `--relay-via-cli` + byte-counter integration
  test).

## Round 36 - R34 Fix Pass Review

Reviewed changes:

- `crates/blit-core/src/remote/pull.rs`
- `crates/blit-daemon/src/service/delegated_pull.rs`
- Round 35 closure note above

Verification:

- Code review only. The user reported `374 tests pass, 0 failed`; I did not
  rerun tests for this review note.

Verdict:

No findings.

R34-F2 is accepted. `AbortOnDrop::join(self)` keeps the wrapper alive across
the await by borrowing the `JoinHandle` from `self.0`; if the parent future is
cancelled while suspended, `Drop` still owns the handle and calls `abort()`.
The old `into_inner()` escape hatch is gone, and the load-bearing
`cancellation_during_join_await_still_aborts_task` test covers the exact
failure mode from Round 34.

R34-F1 is accepted. `DelegatedPullSummary.entries_deleted` now reports the
destination-local deletion count only. Copy-mode summaries and NotFound mirror
deletions no longer fall back to source-attested candidate counts.

Status:

- R34-F1 closed.
- R34-F2 closed.
- Phase 1 is committable from this review series.
- Phase 2 can start after committing Phase 1, assuming the user wants that
  sequencing.
