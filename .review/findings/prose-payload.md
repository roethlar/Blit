# prose-payload: a Windows metadata failure's reason is chosen by its words

**Severity**: LOW — a breach of D-2026-11-24-1, misreporting a reason. When a Windows source cannot hydrate a file's metadata (named streams, attributes), the reason chooses between two classes:
- "changed size during transfer", the drift class;
- "cannot read metadata".

The choice searches the error's text for "changed size while reading" or "Windows metadata changed". Those messages also carry the file's path, so a path holding the words puts any metadata failure in the drift class.
**Status**: Admitted; BLOCKED — the fix changes code that runs only on Windows, and no Windows host is reachable
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: none yet. Found by the working agent on 2026-11-24, scanning for decisions read from prose under D-2026-11-24-1.

## Evidence
`crates/blit-core/src/remote/transfer/payload.rs` `hydration_failure_reason` does `text.contains("changed size while reading") || text.contains("Windows metadata changed")` on `format!("{err:#}")`. The two producers are in `crates/blit-core/src/windows_metadata.rs`:
- `hydrate_payload_header`'s context "Windows metadata changed for {path}" (cross-platform code, but reached only with a Windows manifest);
- the named-stream reader's "Windows named stream … on {path} changed size while reading" (`cfg(windows)`).

An error from `read_payload` — "cannot open", say — whose path holds either phrase is reported in the drift class.

## Intended fix
- A typed marker, `windows_metadata::MetadataDrift`, made at both producers.
- `hydration_failure_reason` finds it by type.
- `source_side_containment.rs`'s injected hydrator failure makes the typed error. That test is cross-platform and proves the classification.
- The named-stream producer is `cfg(windows)` and runs only there. Under the standing rule, unverifiable Windows code is not landed, so this waits for a Windows host on which the suite (`scripts/windows/run-blit-tests.ps1`) can run.

## Known gaps
- Unfixed until a Windows host is reachable (`.agents/machines.md`; at 2026-11-24, `10.1.10.77` timed out, and `.222` and `.173` had no route).

## Restated by `combined-r35`
`combined-r35` (Codex openreview, 2026-11-24) reviewed `0f912b06..dc14afc4` — the `combined-r34` fixes and the three prose fixes — with `guard_confirmed: true` restated it (the metadata reason chosen by its words): "Two previously recorded Windows gaps also apply. High: the receiver discards an `Unjustified` resume baseline and can proceed against a changed destination. Low: a metadata read error whose path contains `Windows metadata changed` is reported as size drift. The former needs a validated Windows handle and the latter a typed error." Still blocked: no Windows host answers.
