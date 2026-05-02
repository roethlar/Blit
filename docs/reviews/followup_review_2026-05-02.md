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

