# c-2-subscribe-skeleton reopened

Verdict: Reopened
Reviewed sha: `0ffaac7ead9fe82deec9b4d15c58f138c187765f`
Reviewer: `reviewer`
Timestamp: `2026-05-17T05:54:12Z`

Validation:

- `cargo fmt --all -- --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo test --workspace`: pass

## Findings

1. Low - The reserved `SubscribeRequest` field-number contract conflicts with the design doc.

   This slice explicitly reserves future `SubscribeRequest` tags so the wire shape is stable, but the two authoritative descriptions disagree about which future field owns which tag. `proto/blit.proto:839` through `proto/blit.proto:843` documents `transfer_id_filter = 2` and `replay_recent = 3`, while `docs/plan/TUI_DESIGN.md:283` through `docs/plan/TUI_DESIGN.md:296` documents `replay_recent = 2` and `transfer_id_filter = 3`.

   That leaves the next C sub-slice with two plausible sources of truth for a wire contract this commit says it is locking. A future client/server pair can implement opposite field numbers if one side follows the proto comment and another follows the design doc. Pick one order and align the proto comments plus `TUI_DESIGN.md` before verifying this slice.
