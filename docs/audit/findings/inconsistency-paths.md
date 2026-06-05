# Inconsistency Findings: Path handling
**Generated**: 2026-06-04
**Findings**: 8 (H: 2 / M: 4 / L: 2)

The dimension covers POSIX-form rendering, separator normalization, trailing-slash semantics, display formatting, and case-fold comparison. The canonical chokepoint is `blit_core::path_posix::relative_path_to_posix` (component-walk, joined with `/`, empty → `""`); the audit asks whether every `Path → wire` conversion routes through it. The answer is "mostly yes, with three rogue ad-hoc reimplementations that disagree about how to encode the empty path."

## High severity

### h-paths-1 — Two surviving ad-hoc `Path → wire` helpers bypass `path_posix::relative_path_to_posix`

**Dimension**: path-handling — wire-form encoding chokepoint compliance.

**Instances**:
1. `crates/blit-core/src/remote/pull.rs:1795-1804` — `fn normalize_for_request(path: &Path) -> String { if empty { ".".to_string() } else { path.iter().map(to_string_lossy).join("/") } }`. Used for `PullRequest.path`, `pull_sync` scan paths, `open_remote_file` paths. **Does not call `path_posix::relative_path_to_posix`** and **encodes empty as `"."`**.
2. `crates/blit-app/src/transfers/remote.rs:638-647` — `fn normalize_for_request(path: &Path) -> String` — identical body to (1). Used by `destination_spec_fields` to build `DelegatedPullRequest.dst_destination_path`. **Also empty → `"."`**.
3. `crates/blit-core/src/remote/push/client/helpers.rs:262-271` — `pub fn destination_path(rel: &Path) -> String { if empty { String::new() } else { rel.iter().map(to_string_lossy).join("/") } }`. Used for `PushHeader.destination_path`. **Empty → `""`** (different from 1/2).
4. `crates/blit-core/src/remote/transfer/payload.rs:214-219`, `crates/blit-core/src/remote/push/client/helpers.rs:52-57`, `crates/blit-daemon/src/service/util.rs:153-158`, `crates/blit-core/src/remote/endpoint.rs:215-218`, `crates/blit-app/src/endpoints.rs:190-192` — the **canonical** delegators (5 sites) that route through `path_posix::relative_path_to_posix`. All return empty as `""`.

Same logical operation ("render relative Path to wire string"), three encodings — and the empty-path encoding actively diverges across push vs pull.

**Canonical**: `blit_core::path_posix::relative_path_to_posix`. Empty path → empty string is the documented "module root" convention used by the receive sinks' empty-rel guards (`source.rs:91-106`, `payload.rs:347-385`, `helpers.rs:200-211`, `pull.rs:1741-1747` all special-case `relative_path == ""`). Encoding empty as `"."` instead means the receive sink's empty-rel single-file guard never fires.

**Recommendation**: Replace both `normalize_for_request` bodies and the `destination_path` body with `blit_core::path_posix::relative_path_to_posix(path)`. Where the historical "empty → `.`" semantics matter (sending a root request via the gRPC `PullRequest.path` field that the daemon `resolve_relative_path` then folds back to `"."`), do the empty → `"."` fold at the **wire-build** layer, not inside a helper that pretends to be a generic POSIX renderer. The daemon already folds `""` and `"."` both to module-root (`util.rs:51-67`), so the fold is unnecessary on the client side.

---

### h-paths-2 — Two case-fold path-comparison schemes, each canonicalizes separators differently

**Dimension**: path-handling — case-folded comparison normalization.

**Instances**:
1. `crates/blit-core/src/mirror_planner.rs:38-42` (Windows) — `CasefoldKey::new(path)` → `path_posix::relative_path_to_posix(path).to_ascii_lowercase()`. Joins components with `/`, then ASCII-lowercases. Used to build the source set against which destination entries are matched for mirror-deletion.
2. `crates/blit-core/src/win_fs.rs:85-89` — `compare_paths_case_insensitive(a, b)` → `normalize_path(a).to_string_lossy().to_lowercase()`, same for b. Native separators preserved (`\`), Unicode-aware lowercase. Currently has **no callers** in the production tree but is `pub` and re-exportable.

Two helpers, two different keys. The mirror-planner key strips `\` (canonicalizes to `/`), the win_fs helper preserves `\`. The Unicode-vs-ASCII lowercase axis is also different: a Turkish-locale `I` would round-trip via win_fs but not via mirror_planner (irrelevant in practice because Rust's `to_lowercase` is locale-independent and just covers more Unicode characters than ASCII).

The data-loss risk if win_fs is ever called from the same codepath that builds a CasefoldKey: `compare_paths_case_insensitive(Path::new("Foo\\Bar"), Path::new("Foo/Bar"))` returns **false** on Windows (the two normalized forms differ in separator). But mirror_planner would treat both as the same key. A mirror would then delete a destination that the comparator said was distinct.

**Canonical**: `path_posix::relative_path_to_posix` + `to_ascii_lowercase` (the mirror_planner approach). On Windows the wire side already canonicalizes to `/` everywhere, and the deletion plan compares against entries returned from local enumeration that may carry native `\`. Normalizing both sides via the POSIX form is the only consistent answer.

**Recommendation**: Either delete `win_fs::compare_paths_case_insensitive` (it has no callers) or rewrite it to use the same `relative_path_to_posix` + `to_ascii_lowercase` form as the CasefoldKey. Keeping a public helper that disagrees with the production canonical form is a footgun for the next path-comparison call site.

## Medium severity

### m-paths-1 — Empty-path encoding on the daemon side: `.` vs `""` depending on which helper

**Dimension**: path-handling — empty-rel-path convention.

**Instances**:
1. `crates/blit-daemon/src/service/util.rs:153-158` `normalize_relative_path` — delegates to `path_posix::relative_path_to_posix`; empty → `""`. Used for push manifest entries and daemon-emitted wire paths (e.g. `pull.rs:474, 488, 810`).
2. `crates/blit-daemon/src/service/util.rs:160-168` `pathbuf_to_display` — reimplements component-walk-with-`/` BUT special-cases `Path::new(".")` → `"."` while everything else (including empty input via the unconditional join) → `""`. Used for `DiskUsageEntry.relative_path`, `FindEntry.relative_path`, `FilesystemStatsResponse.module`, and du sort keys.
3. `crates/blit-daemon/src/service/util.rs:51-67` `resolve_relative_path` (request-path context) — folds both `""` and `"."` to `PathBuf::from(".")`. Then admin handlers emit `pathbuf_to_display(&start_rel)` which renders the `.` literally.

A `du` or `find` client thus sees a top-level entry with `relative_path: "."`, but a `pull` client receives top-level entries with `relative_path: ""`. Same daemon process, same notion of "module root", two encodings on the wire.

**Canonical**: `path_posix::relative_path_to_posix` (empty → `""`). Other helpers like push manifest emit `""` for root; the `du`/`find` API would consistently mean root with `""` too. The `.` form is a leftover from when admin used `PathBuf` natively and got `.` for free.

**Recommendation**: Fold `pathbuf_to_display` into `path_posix::relative_path_to_posix` and accept that the du/find wire emits `""` for the start-rel-is-root case (or, conversely, the orchestrator can keep `.` and `du`/`find` callers will canonicalize back). The current state of "same daemon, different empty encoding per RPC" violates the principle that the canonical chokepoint exists exactly so the wire layer doesn't have to know.

---

### m-paths-2 — `path_safety::validate_wire_path` rejects `"."` as "normalizes to empty," but two daemon helpers actively produce `"."` for the same logical "module root"

**Dimension**: path-handling — request-path vs file-path normalization.

**Instances**:
1. `crates/blit-core/src/path_safety.rs:71-133` `validate_wire_path` — non-empty input that normalizes to nothing (i.e. just `.` components) **bails** with `"path normalizes to empty (only `.` components)"`. This is the strict file-path validator used by manifest entries and sink paths.
2. `crates/blit-daemon/src/service/util.rs:51-67` `resolve_relative_path` — folds `""`, `.`, `./` all to `PathBuf::from(".")` for request-path contexts (list/find/du). Used by `core.rs:1154-1158`, `core.rs:1211-1216`.
3. `crates/blit-core/src/remote/pull.rs:1795-1804` `normalize_for_request` — emits `"."` for empty rel_path. Sent as `PullRequest.path` to the daemon.

If a hypothetical caller routed a `normalize_for_request("")` output through `validate_wire_path` (the canonical wire chokepoint), it would bail. Today the daemon protects this by special-casing `""` and `"."` in `resolve_relative_path` before `validate_wire_path` runs (`util.rs:56-59`), but the two layers carry contradictory contracts: the safety chokepoint says "`.` is unsafe," the request normalizer says "`.` is fine, use it freely."

**Canonical**: One convention per axis. Either:
- All callers send `""` for root, daemon folds to `PathBuf::from(".")` once, and `validate_wire_path` keeps rejecting `"."`; OR
- All callers send `"."` for root, daemon's strict validator accepts `"."` as legitimate in request-path contexts and `path_safety` adds a `validate_request_path` variant.

**Recommendation**: Standardize on `""` for root across the wire. Delete the empty→`"."` fold in `normalize_for_request` (both copies). The daemon's `resolve_relative_path` already handles empty correctly. Removes one of the three differing wire encodings.

---

### m-paths-3 — `FsTransferSink` canonical-fallback ladder: 2 sites `log::warn!`, 2 sites silent

**Dimension**: path-handling — defense-in-depth fallback observability.

**Instances**:
1. `crates/blit-core/src/remote/transfer/sink.rs:190-205` `resolve_destination` — `None => log::warn!("...has no canonical root..."); safe_join(...)`. The warn surfaces that R46-F3 escape protection is unavailable.
2. `crates/blit-core/src/remote/transfer/sink.rs:463-481` `write_file_payload` — `None => log::warn!("...write_file_payload at '{}' has no canonical root..."); safe_join(...)`. Same warn pattern.
3. `crates/blit-core/src/remote/transfer/sink.rs:651-657` `write_file_block_payload` — `None => crate::path_safety::safe_join(dst_root, relative_path)...` — **silent**, no warn.
4. `crates/blit-core/src/remote/transfer/sink.rs:696-702` `write_file_block_complete` — same as (3), silent.

If a sink is constructed without a canonical root (the rare path), file writes log a warning but resume blocks proceed quietly. An operator scanning logs for "R46-F3 escape protection unavailable" warnings will miss the resume-block calls, which write to the same destination via the same fallback.

**Canonical**: All four call sites should `log::warn!` the same message — the R46-F3 contract is the same regardless of payload type.

**Recommendation**: Extract the `Some(canonical) -> safe_join_contained, None -> warn + safe_join` ladder into a single helper (e.g. `safe_join_with_warn_fallback`) and have all four sites call it. This is also mentioned in `code-core-transfer.md`'s smells (duplicated-canonical-fallback-ladder). Folding fixes the observability gap as a side effect.

---

### m-paths-4 — `tar_stream::sanitize_rel_path` extracted helper coexists with verbatim-inline duplicate

**Dimension**: path-handling — tar entry path sanitization.

**Instances**:
1. `crates/blit-core/src/tar_stream.rs:43-59` `fn sanitize_rel_path(rel: &Path) -> Result<PathBuf>` — rejects absolute, `..`, `RootDir`, `Prefix`; allows `Normal` and `CurDir`.
2. `crates/blit-core/src/tar_stream.rs:204-224` (inside `tar_stream_transfer_cb`) — inlines the same logic verbatim instead of calling the helper. `tar_stream_transfer_list_cb` (line ~315 region) **does** call `sanitize_rel_path`.

Same file, same operation, half the call sites use the helper, half re-implement it. If the helper is tightened (e.g. add `is_reserved_name` rejection for Windows reserved devices), the inline copy drifts.

**Canonical**: `sanitize_rel_path` — it has a name, it's testable in isolation, the other site already uses it.

**Recommendation**: Replace lines 211-223 of `tar_stream.rs` with `let rel_path = sanitize_rel_path(rp)?;`. Already flagged in `code-core-io.md` smells (#4) — included here because the inline form means a single hardening sweep on the helper won't apply uniformly.

## Low severity

### l-paths-1 — `is_deletable_remote_path` checks `rel.to_string_lossy()` against `"."`, not the canonical empty/dot form

**Dimension**: path-handling — empty-or-dot rel-path detection.

**Instances**:
1. `crates/blit-tui/src/del_request.rs:65-74` `is_deletable_remote_path` — converts rel_path via `to_string_lossy()` then checks `!rel.is_empty() && rel != "."`. The lossy form on Windows could be `Folder\sub`, on POSIX `Folder/sub` — irrelevant for the actual check ("is this exactly `.` or empty?") because both forms agree on empty/`.`.
2. `crates/blit-core/src/path_safety.rs:71-133` `validate_wire_path` — uses `path.components()` and explicitly handles `Component::CurDir` separately. The canonical "is this just `.`" check.
3. `crates/blit-daemon/src/service/util.rs:51-67` `resolve_relative_path` — `trimmed == "."` on the wire string.

The TUI doesn't actually misbehave today (the check semantically works), but each layer has its own way of asking the same question. A future refactor that gives `PathBuf` a `.is_root_marker()` extension method could replace all three with one expression.

**Canonical**: Inspect `path.components()` directly, treating an iterator with only `CurDir` (or empty) as the "root" marker. Mirrors `validate_wire_path`.

**Recommendation**: Low-priority cleanup. The check works; the drift is purely stylistic.

---

### l-paths-2 — `display_endpoint` in CLI collapses `//+` runs; nowhere else does, including TUI

**Dimension**: path-handling — human-display normalization.

**Instances**:
1. `crates/blit-cli/src/transfers/mod.rs:48-72` `display_endpoint` + `collapse_slashes` — collapses runs of `/` in the local-path portion of an endpoint's display string. The header comment notes "users stare at `src//foo` when a script appended `/` to an already-trailing-slash `$SRC`."
2. TUI side (`crates/blit-tui/src/dual_pane.rs:76` `Location::display()`) — uses `endpoint.display()` directly with no `//+` collapse. A `nas://photos` (legitimate root-form spec) renders correctly because the root form uses `://`, but a local pane location like `/a//b/c` would render with the doubled slash.

CLI and TUI show the same logical endpoint differently when there's a double slash in the local segment. Not a data-loss issue — filesystems ignore `//+` — but the CLI explicitly fixed this as an observed-by-users wart while the TUI doesn't.

**Canonical**: `collapse_slashes` is the right behavior. Should live next to `display_endpoint` in `blit_app` so both consumers reach the same helper.

**Recommendation**: Move `collapse_slashes` into `blit_app::transfers::resolution` (alongside `dest_is_container` / `source_is_contents`) and have the TUI's `Location::display()` and `Endpoint`-display call sites route through it. Cosmetic but ends one piece of CLI/TUI drift.
