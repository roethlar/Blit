# Blit v2 Overview
- Purpose: rewrite of the Blit file transfer tool with modular core (`blit-core`), CLI (`blit-cli`), daemon (`blit-daemon`), and shared utilities.
- Focus: high-performance local/remote mirroring, transfer planning, and future gRPC-based push/pull services.
- Structure: Cargo workspace with crates for core logic, CLI entrypoint, daemon, and util helpers. Docs and workflow plans live under `docs/`.
- Key modules: enumerator/mirror planner/transfer engine in `crates/blit-core`; CLI wiring in `crates/blit-cli`; proto definitions in `proto/blit.proto`.