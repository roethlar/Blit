# otp-10a — codex verdict adjudication

reviewer: gpt-5.6-sol (codex exec, read-only; raw output
`.review/results/otp-10a.codex.md`)
slice commit: `0fbc966`
verdict: NEEDS FIXES — 8 findings (3 High, 4 Med, 1 Low)
adjudication: **7 accepted + fixed; F1 accepted in part (move half
fixed, copy half deferred to the standing owner question)**
fix sha: `6b292ed`

## F1 (High) — SizeMtime same-size-newer skip + move deletes the source

**Accepted in part.** Verified: the session's data-safe skip
(same-size + dest-newer ⇒ skip, otp-4a, owner-ack'd) combined with
move's source-delete destroys the only copy of source bytes whose
content differs invisibly. Old push clobbered the dest, so old move
never lost the SOURCE side (it lost the dest's newer bytes instead).
Fix (move half): `PushExecution.compare_mode`; both move frontends
(CLI `run_remote_push_transfer_deferred`, TUI `PullKind::Move`) push
with `ComparisonMode::IgnoreTimes` — transfer unconditionally, so the
delete is safe by construction. This also closes the OLD documented
matching-size+mtime move hole (the R54-F2 bail text's own scenario).
Cost: move re-uploads unchanged files; correctness over speed on a
verb that destroys its source. Pins: e2e
`move_lands_source_bytes_over_same_size_newer_destination` (the
mutation run reproduced the exact data-loss: exit 0, wrong bytes at
dest), blit-app-level
`move_shaped_push_transfers_same_size_newer_destination`, TUI unit
`build_f1_push_execution_move_transfers_unconditionally`.
The COPY-verb half (skip semantics + `--force` not wired) is NOT
self-adjudicated: it is the standing owner question in STATE
(otp-4a); compare-flag wiring for copy is the otp-10b one-mapping
item. No change to copy.

## F2 (High) — native separators in `SessionOpen.path` on Windows

**Accepted.** Verified: `endpoint_module_path` used
`to_string_lossy`, and the CLI's rsync destination-resolution builds
`rel_path` via `PathBuf::join` (native `\` on Windows) — a
Windows→Unix push would create a literal `sub\dir` entry. Fixed with
`path_posix::relative_path_to_posix` (the win-1 rule: every
wire-bound relative path routes through it); fixes push AND pull
session clients (both share the helper). Pinned by
`joined_rel_path_reaches_the_wire_in_posix_form` (+ module-root
identity); Windows CI exercises the native-separator branch.

## F3 (High) — daemon `--force-grpc-data` ignored by served sessions

**Accepted.** Verified: `core.rs` threads `force_grpc_data` into the
old push/pull_sync handlers but `transfer` did not; the responder
granted a TCP data plane regardless. Fixed: threaded through
`run_transfer_session` → `run_responder` → `responder_finish`, which
now grants no data plane when forced (in-stream fallback, same as an
initiator request). Pinned by
`daemon_force_grpc_data_forces_the_in_stream_carrier` (client does
NOT force; summary attests in-stream); guard-proven by threading
mutation.

## F4 (Med) — `--relay-via-cli --resume` carrier-divergent failure

**Accepted.** Verified: `RemoteTransferSource::prepare_payload` bails
on composite `ResumeFile` payloads, so relay+resume faults
mid-transfer on the TCP data plane while the in-stream carrier
(`ResumeBlockDiff` via `source.open_file`) would succeed. Fixed:
refused up front in `run_remote_push` before any connection. Pinned
by `relay_source_with_resume_is_refused_before_any_connection`.

## F5 (Med) — fault stringification breaks `--retry` classification

**Accepted.** Verified: `is_retryable` requires a real `io::Error` in
the chain (downcast, no string matching), and
`fault_from_report`/`dp_fault` replace the chain with a stringified
`SessionFault`. Fixed: `SessionFault.io_kind: Option<io::ErrorKind>`
captured from the replaced report (`fault_from_report`, and
`dp_fault_io` at the four data-plane dial sites); `is_retryable`
classifies a chain `SessionFault` by the same kind set. Pinned by
`session_fault_io_kind_classifies_like_the_raw_error` (retry.rs
contract) + `fault_from_report_captures_the_underlying_io_kind`.
Note: peer-reported faults (`from_wire`) deliberately carry `None` —
retry classification stays local-transport testimony.

## F6 (Med) — resumed files emit no w6-1 progress

**Accepted.** (Was listed as a Known gap; codex is right that it
violates the contract rather than merely narrowing it.) Fixed on both
carriers: the sink pipeline's `ResumeFile` arm reports
`Payload{0, outcome.bytes_written}` + `FileComplete` after the record;
`send_resume_block_records` reports its counted stale bytes + one
`FileComplete`. Pinned by the extended
`push_verb_resume_patches_changed_partials_blockwise` (both carriers:
counted once, 0 < bytes < file size); each carrier's report
guard-proven by its own never-taken mutation.

## F7 (Med) — no test on the verb-level fault-summary print

**Accepted.** Fixed by splitting extraction from printing:
`session_fault_summary(&Report) -> Option<String>` + 4 unit pins
(context-wrapped `SessionFault` names the file + suggests re-run;
`TransferOpenRefusal` reached through the wrapper; no-file refusal →
`None`; plain errors → `None`). The `eprintln!` wrapper is one line;
the chain-walking — the part that can silently break — is what the
pins hold. End-to-end content is additionally covered by the
session-level `mid_resume_fault_names_the_file_in_the_end_of_operation_summary`.

## F8 (Low) — `build_spec` drops up-front glob validation

**Accepted.** Verified: `build` validates globs (R58-F12) but
`build_spec` only packed strings, so a malformed `--exclude` on push
errored only after a connection (at the session's OPEN validation).
Fixed: `build_spec` validates the globs at construction (same probe
`FileFilter::validate_globs` path); benefits pull too (same helper).
Pinned by `build_spec_rejects_malformed_glob_before_any_connection`.

## Guard proofs (fix round, run live)

- F3 threading mutation → `daemon_force_grpc_data…` FAILS;
- F6 data-plane report mutation → resume pin FAILS (first half);
- F6 in-stream report mutation → resume pin FAILS (second half);
- F1 CLI move-wiring mutation (SizeMtime) → `move_lands_source…`
  FAILS with the exact data-loss shape (exit 0, dest holds old bytes).
All restored; suite green.
