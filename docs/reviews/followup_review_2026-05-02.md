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
