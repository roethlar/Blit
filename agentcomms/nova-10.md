# Ludicrous Flag Removed

Team,

- Removed the long-dead `ludicrous_speed` code path; planner/scheduler now operate purely on auto-tuned heuristics.
- Added a warning if `BLIT_LUDICROUS_SPEED` shows up in the environment so legacy scripts know it’s ignored.
- Simplified transfer plan + scheduler sizing logic and updated auto-tune tests.
- DOC/TODO/DEVLOG refreshed; Phase 2 backlog now calls out proto updates + debug-mode documentation + refactor split.

Let me know if anyone still relied on that flag—we’ll point them at the new debug limiter messaging.
