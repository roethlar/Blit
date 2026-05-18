# e-6-verify-prefill: tui.toml prefills Verify Source/Destination

**Severity**: Feature (polish — fourth slice growing
the e-3 config scaffold)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

`[verify]` config section gains `default_source` and
`default_destination` string fields. When non-empty, the
TUI launches with those values already in the Verify
form — operator can `Enter` the run immediately instead
of typing the same paths every session.

```toml
[verify]
default_source = "/data/source"
default_destination = "/backups/2026/data"
```

Typical use cases:

- **Nightly backup verification**: operator has a
  scheduled compare against the same target every day.
- **Multi-host audit**: same Source path verified on
  every host, only the daemon-side flags differ.
- **Demo / dev**: known-good test paths pre-loaded
  during development.

Empty string (default) means no prefill — the operator
types both fields as before. Backward compatible: any
existing config without these fields keeps the d-2
behavior.

## Approach

### Config

`VerifyDefaults` gains two `String` fields with serde
defaults:

```rust
pub default_source: String,
pub default_destination: String,
```

### VerifyState constructor

New `VerifyState::with_defaults_and_paths(use_checksum,
one_way, source, destination)` constructor. The
existing `with_defaults(use_checksum, one_way)` now
delegates with empty strings — backward compatible for
tests that don't care about prefill.

### Apply

`main.rs` call site uses the new constructor:

```rust
verify::VerifyState::with_defaults_and_paths(
    tui_config.verify.default_use_checksum,
    tui_config.verify.default_one_way,
    tui_config.verify.default_source.clone(),
    tui_config.verify.default_destination.clone(),
)
```

## Files changed

- `crates/blit-tui/src/config.rs`:
  - `VerifyDefaults` gains `default_source` and
    `default_destination` fields.
- `crates/blit-tui/src/verify.rs`:
  - `with_defaults_and_paths` constructor.
  - `with_defaults` delegates with empty strings.
- `crates/blit-tui/src/main.rs`:
  - Call site uses `with_defaults_and_paths`.

## Tests

+4 unit tests (216 → 220):

In `verify::tests`:
- `with_defaults_and_paths_prefills_fields` —
  constructor seeds Source + Destination correctly,
  doesn't touch focus / status.
- `with_defaults_and_paths_empty_strings_equal_no_prefill`
  — empty strings preserve the "no prefill" contract.

In `config::tests`:
- `verify_path_prefill_round_trip` — `[verify]
  default_source / default_destination` parse from TOML
  exactly.
- `verify_path_prefill_defaults_to_empty` — missing
  fields default to empty (the d-2 behavior).

`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.

## Known gaps

1. **No auto-save / write-back.** Operator typing
   into the Verify fields at runtime doesn't update
   `tui.toml`. They have to hand-edit if they want a
   new pair to stick. A future polish slice could add
   a "save current paths as default" key (`Ctrl-S`?)
   that writes the current Verify fields back to the
   config file.

2. **String fields only.** No path validation or
   tilde-expansion. The TUI's Verify form accepts the
   raw string and `prepare_local_transfer` (used by
   `C`/`M`/`V`) validates at run time. Same contract as
   typing the path — config is just an alternate input
   source.

3. **No per-remote prefill.** Operator running the TUI
   against multiple `--remote` endpoints gets the same
   prefill for all of them. A future polish could key
   the prefill on the connected remote.

## Out of scope (next slices)

- **Color themes** (`[theme]`).
- **Save-current-paths-to-config hotkey.**
- **Per-pane refresh intervals.**

## Reviewer comments

### Round 1 verdict — reopened (`.review/results/e-6-verify-prefill.reopened.md`)

One Low-severity finding, addressed in round 2 — the
same doc-staleness shape as e-5 R2:

- **`config.rs` module-level schema doc omitted the new
  Verify prefill keys.** The "Current schema (grown
  through e-3 / e-4 / e-5)" block presented only the
  pre-e-6 `[verify]` shape (`default_use_checksum` /
  `default_one_way`), even though `default_source` and
  `default_destination` were now live below in
  `VerifyDefaults`. The future-slice note also still
  named "persisted form prefill" as future work, which
  contradicted what e-6 just shipped.

  Round 2 fixes both:
  - Schema block adds `default_source = ""` /
    `default_destination = ""` with e-6 attribution.
  - "Grown through..." line includes e-6.
  - Future-slice note rephrased: launch-time prefill is
    done; the still-pending polish is *runtime save-back*
    (operator types into the form, hits a hotkey,
    `tui.toml` gets updated).

### Round 2 file changes

- `crates/blit-tui/src/config.rs`:
  - Module-doc schema block + future-slice list both
    updated to reflect the e-6 reality.

No behavior change, no test count delta (still 220).
`cargo fmt`, `cargo clippy --workspace --all-targets
-- -D warnings`, and `cargo test --workspace` all green.
