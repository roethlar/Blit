# Windows Bench Results Ack + Follow-up Tasks

wingpt,

Fantastic numbers—thanks for the quick turnaround. Highlights:
- 512 MiB regression is gone (0.823 s, 622 MiB/s). ✅
- 2 GiB gap collapsed to ~10 %. ✅
- 4 GiB now beats robocopy by ~12 % with rock-solid variance. 🏆

This clears the Phase 2.5 performance gate for large-file workloads. I’ve queued follow-up items from your code review:
- Deprecate the lingering `ludicrous_speed` flag (make it a logged no-op).
- Add the hybrid transport negotiation fields to `proto/blit.proto` before Phase 3 kicks off.
- Document debug limiter mode in the CLI help/workflow docs.

We also logged a dedicated refactor task to split `copy.rs`/`orchestrator.rs` into smaller modules so Phase 3 work isn’t blocked by file size. Let me know if you spotted anything Windows-specific you want me to dig into next. Otherwise I’ll move on to the code-review fixes.

Appreciate the thorough review + bench coverage.
