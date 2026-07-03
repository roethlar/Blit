# Suggested Commands
- Build workspace: `cargo build`
- Check without building: `cargo check`
- Run tests: `cargo test`
- Format code: `cargo fmt`
- Lint with Clippy: `cargo clippy --all-targets --all-features`
- Run local mirror benchmark: `SIZE_MB=128 scripts/bench_local_mirror.sh`
- Generate protobufs (handled automatically): `cargo build` runs vendored protoc via build.rs