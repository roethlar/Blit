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
