# Repository Guidelines

## Project Structure & Module Organization
- `crates/blit-core/`: Core library (enumeration, planner, transfer engine, orchestrator). Most logic and unit tests live here.
- `crates/blit-cli/` and `crates/blit-daemon/`: Binaries for the CLI and daemon. Each has a minimal `main.rs` that wires into `blit-core`.
- `crates/blit-utils/`: Supporting utilities shared across binaries.
- `proto/`: gRPC definitions (`blit.proto`); build script in `blit-core` venders `protoc` automatically.
- `scripts/`: Helper tooling. Notable: `scripts/windows/run-blit-tests.ps1` wraps fmt/check/test with log capture.
- `tests/`: Workspace-level integration tests when needed.

## Build, Test, and Development Commands
- `cargo fmt`: Format the entire workspace.
- `cargo fmt -- --check`: CI-safe formatting validation.
- `cargo check`: Fast compilation pass across the workspace.
- `cargo test`: Run all unit and integration tests.
- `cargo test -p blit-core`: Target core library tests (used for streaming/orchestrator work).
- `scripts/windows/run-blit-tests.ps1`: Windows-friendly wrapper that runs fmt, check, and targeted tests, teeing results into `logs/`.

## Coding Style & Naming Conventions
- Rust edition 2021; format with `rustfmt` (via `cargo fmt`).
- Keep modules snake_case, types in PascalCase, constants SHOUT_CASE; match existing module names (`transfer_engine`, `TransferOrchestrator`, `PLAN_OPTIONS`).
- Avoid blocking calls inside async contexts (use async send APIs in Tokio).
- When adding modules, re-export in `crates/blit-core/src/lib.rs` to keep the workspace graph explicit.

## Testing Guidelines
- Use `cargo test` locally before every PR. Add unit tests alongside the feature (e.g., `transfer_engine::tests`).
- Prefer async-aware tests (`#[tokio::test]`) for planner/engine work.
- Windows parity matters: rerun `scripts/windows/run-blit-tests.ps1` after changes touching platform-specific code (`win_fs`, planners).
- Keep tests deterministic; capture long logs under `logs/` for debugging.

## Commit & Pull Request Guidelines
- Commit messages: short imperative summary (“Add streaming planner heartbeat”); group related changes logically.
- PR checklist: describe the change, note tests executed (`cargo test`, scripts), link relevant issues, and mention doc updates (update `TODO.md`, workflow docs, `DEVLOG.md`).
- Include new commands/scripts or data formats in `README`/`AGENTS.md` as applicable, and attach log snippets when debugging failures.

## Agent-Specific Expectations
- After meaningful work, update `TODO.md`, appropriate `docs/plan/WORKFLOW_PHASE_{N}.md` document, and `DEVLOG.md` so other agents can resume quickly.
- Persist session context via Serena memories (`write_memory`) when the platform is available, especially before resets or restarts.
- Use MCP tools whenever possible and appropriate.
