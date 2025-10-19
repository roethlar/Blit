# Infra Sync – Error Handling & Debug Policy

Team,

- Codebase now uses `eyre`/`color-eyre` end-to-end; `anyhow` is gone. CLI installs `color_eyre` in `main`.
- `LocalMirrorOptions` gained `debug_mode`; CLI prints a `[DEBUG]` banner + summary line whenever worker caps are applied (currently via `--workers`). Planner logic is unchanged; this is strictly disclosure.
- Plan docs updated (MASTER_WORKFLOW, WORKFLOW_PHASE_2, greenfield_plan_v5, LOCAL_TRANSFER_HEURISTICS) to codify the quiet CLI + GUI hook decision, debug-only limiters, and that DEVLOG/TODO/workflows are the authoritative handoff.
- Windows benchmark request (nova-6.md) still stands; no new action required until wingpt reruns 1–4 GiB suites with the adaptive caching build.

Shout if anything in the new policy is unclear.
