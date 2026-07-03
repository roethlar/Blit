# Code Style & Conventions
- Language: Rust 2021 edition; rely on `rustfmt` defaults.
- Error handling: prefer `anyhow::Result` for top level, `thiserror` not yet in use.
- Async: Tokio 1.x with full features; blocking filesystem work generally wrapped in spawn_blocking or synchronous helpers.
- Naming: snake_case for functions/modules, CamelCase structs/enums; follow idiomatic Rust.
- Docs: Workflow plans in `docs/plan` guide implementation phases; update DEVLOG and TODO checklist after changes.