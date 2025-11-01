## Transfer facade refactor – 2025-10-28
- Replaced monolithic `transfer_facade.rs` with `transfer_facade/{mod,types,aggregator,planner}.rs`.
- `types.rs` holds exposed structs (TransferFacade, LocalPlan* types, PlannerEvent, PlanTaskStats).
- `aggregator.rs` encapsulates task batching logic plus unit tests.
- `planner.rs` implements `TransferFacade` methods (stream/build plans, pull plan, normalized key) using the aggregator.
- `LocalPlanStream::into_parts` behaviour preserved via re-export.
- Ran `cargo fmt`; `cargo check -p blit-core`.
- TODO/DEVLOG/docs updated to reflect partial completion (remote/push/client.rs still pending).