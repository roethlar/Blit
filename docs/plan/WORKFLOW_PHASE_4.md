# Phase 4: Production Hardening & Packaging
*Phase 4 is OPTIONAL*

**Goal**: Production-ready, secure, packaged, and documented system
**Duration**: 5-7 days
**Prerequisites**: Phase 3 complete
**Status**: Not started
**Critical Path**: Authentication, comprehensive testing, packaging

## Overview

Phase 4 transforms the functional v2 implementation into a production-ready system suitable for deployment. This includes security hardening, comprehensive testing, packaging for distribution, and complete documentation.

### Success Criteria

- ✅ Authentication system functional (token-based)
- ✅ Comprehensive integration test suite
- ✅ Platform-specific packages (Linux, macOS, Windows)
- ✅ Complete user documentation
- ✅ Deployment automation
- ✅ Migration guide from v1
- ✅ All quality gates passed

## Day 1-2: Security Hardening (4-6 hours)

### Task 4.1.1: Implement Token-Based Authentication
**Priority**: 🔴 Critical
**Effort**: 3-4 hours
**Security**: Essential for production

**Token-based authentication** (already implemented for data plane in Phase 3):

```rust
// crates/blit-core/src/auth.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AuthService {
    tokens: Arc<RwLock<HashMap<String, UserInfo>>>,
}

#[derive(Clone, Debug)]
pub struct UserInfo {
    pub username: String,
    pub permissions: Vec<String>,
}

impl AuthService {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn validate_token(&self, token: &str) -> Option<UserInfo> {
        let tokens = self.tokens.read().await;
        tokens.get(token).cloned()
    }

    pub async fn create_token(&self, username: String, permissions: Vec<String>) -> String {
        use rand::Rng;
        let token: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        let mut tokens = self.tokens.write().await;
        tokens.insert(token.clone(), UserInfo { username, permissions });

        token
    }
}

// Interceptor for authentication
pub fn auth_interceptor(
    auth_service: Arc<AuthService>,
) -> impl Fn(Request<()>) -> Result<Request<()>, Status> + Clone {
    move |mut req: Request<()>| {
        let auth_service = auth_service.clone();

        // Extract token from metadata
        let token = req
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "));

        match token {
            Some(token) => {
                // Validate asynchronously in production
                // For now, simplified synchronous check
                req.extensions_mut().insert(token.to_string());
                Ok(req)
            }
            None => Err(Status::unauthenticated("Missing authorization token")),
        }
    }
}
```

**Wire into daemon**:

```rust
// In BlitService, check authentication before operations
async fn push(&self, request: Request<...>) -> Result<...> {
    // Extract user info from request extensions
    let token = request.extensions().get::<String>()
        .ok_or_else(|| Status::unauthenticated("No token"))?;

    let user = self.auth_service.validate_token(token).await
        .ok_or_else(|| Status::unauthenticated("Invalid token"))?;

    // Check permissions
    if !user.permissions.contains(&"write".to_string()) {
        return Err(Status::permission_denied("Write permission required"));
    }

    // ... proceed with operation
}
```

## Day 2-3: Comprehensive Testing (10-14 hours)

### Task 4.2.1: Expand Integration Test Suite
**Priority**: 🔴 Critical
**Effort**: 6-8 hours
**Quality**: Essential for reliability

**Create comprehensive test suite** (`tests/comprehensive_test.rs`):

```rust
// tests/comprehensive_test.rs

mod common;
use common::*;

#[tokio::test]
async fn test_large_file_transfer() {
    let ctx = TestContext::new().await;

    // Generate 1GB file
    let src = ctx.temp_dir.path().join("large.bin");
    generate_random_file(&src, 1024 * 1024 * 1024).await;

    // Push to daemon
    ctx.push_file(&src, "large.bin").await.unwrap();

    // Pull and verify
    let dst = ctx.temp_dir.path().join("large_pulled.bin");
    ctx.pull_file("large.bin", &dst).await.unwrap();

    assert_files_equal(&src, &dst).await;
}

#[tokio::test]
async fn test_mirror_with_deletions() {
    let ctx = TestContext::new().await;

    // Initial mirror
    let src = create_test_tree(&ctx, 100).await;
    ctx.mirror_push(&src, "test_module").await.unwrap();

    // Modify source (delete files)
    std::fs::remove_file(src.join("file_50.txt")).unwrap();

    // Mirror again
    ctx.mirror_push(&src, "test_module").await.unwrap();

    // Verify deletion on remote
    let files = ctx.list_remote("test_module").await.unwrap();
    assert!(!files.contains(&"file_50.txt".to_string()));
}

#[tokio::test]
async fn test_concurrent_transfers() {
    let ctx = TestContext::new().await;

    // Start 10 concurrent transfers
    let mut handles = vec![];
    for i in 0..10 {
        let ctx = ctx.clone();
        handles.push(tokio::spawn(async move {
            let src = ctx.temp_dir.path().join(format!("file_{}.txt", i));
            std::fs::write(&src, format!("content {}", i)).unwrap();
            ctx.push_file(&src, &format!("file_{}.txt", i)).await
        }));
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // Verify all files present
    let files = ctx.list_remote(".").await.unwrap();
    assert_eq!(files.len(), 10);
}

#[tokio::test]
async fn test_network_interruption_recovery() {
    let ctx = TestContext::new().await;

    // Start transfer
    let src = ctx.temp_dir.path().join("test.bin");
    generate_random_file(&src, 100 * 1024 * 1024).await;

    // Simulate network interruption mid-transfer
    // (Would need infrastructure to inject failures)

    // Verify retry or graceful failure
    // ...
}

#[tokio::test]
async fn test_permission_preservation() {
    let ctx = TestContext::new().await;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let src = ctx.temp_dir.path().join("executable.sh");
        std::fs::write(&src, "#!/bin/bash\necho test").unwrap();

        let mut perms = std::fs::metadata(&src).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&src, perms).unwrap();

        // Transfer
        ctx.push_file(&src, "executable.sh").await.unwrap();
        let dst = ctx.temp_dir.path().join("pulled.sh");
        ctx.pull_file("executable.sh", &dst).await.unwrap();

        // Verify permissions
        let dst_perms = std::fs::metadata(&dst).unwrap().permissions();
        assert_eq!(dst_perms.mode() & 0o777, 0o755);
    }
}

// Add 20-30 more comprehensive tests...
```

### Task 4.2.2: Performance Regression Tests
**Priority**: 🟡 Important
**Effort**: 3-4 hours

**Automated performance benchmarks**:

```rust
// benches/transfer_bench.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_local_mirror(c: &mut Criterion) {
    let mut group = c.benchmark_group("local_mirror");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                // Create temp directory with `size` files
                // Run mirror operation
                // Measure time
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_local_mirror);
criterion_main!(benches);
```

**Add to CI pipeline**:
```yaml
# .github/workflows/ci.yml
- name: Run benchmarks
  run: cargo bench --no-fail-fast
```

### Task 4.2.3: Cross-Platform Testing
**Priority**: 🟡 Important
**Effort**: 2-3 hours

**Test on multiple platforms**:

```bash
# Linux
cargo test --all

# macOS (if available)
cargo test --all

# Windows (if available)
cargo test --all

# Cross-compile and test
cross test --target x86_64-pc-windows-gnu
```

**Platform-specific behavior tests**:
```rust
#[cfg(target_os = "linux")]
#[test]
fn test_sendfile_used() {
    // Verify sendfile is actually being used
    // Could use strace or similar
}

#[cfg(target_os = "windows")]
#[test]
fn test_windows_paths() {
    // Test Windows path handling
    // UNC paths, drive letters, etc.
}
```

## Day 4-5: Packaging & Distribution (8-12 hours)

### Task 4.3.1: Create Platform Packages
**Priority**: 🔴 Critical
**Effort**: 6-8 hours

**Debian package** (`scripts/build_deb.sh`):

```bash
#!/bin/bash
# scripts/build_deb.sh

set -e

VERSION=${1:-"0.1.0"}
ARCH=${2:-"amd64"}

# Build release binaries
cargo build --release

# Create package structure
PKG_DIR="blit_${VERSION}_${ARCH}"
mkdir -p "$PKG_DIR/DEBIAN"
mkdir -p "$PKG_DIR/usr/bin"
mkdir -p "$PKG_DIR/etc/systemd/system"
mkdir -p "$PKG_DIR/etc/blit"

# Copy binaries
cp target/release/blit-cli "$PKG_DIR/usr/bin/blit"
cp target/release/blit-daemon "$PKG_DIR/usr/bin/blitd"

# Create control file
cat > "$PKG_DIR/DEBIAN/control" <<EOF
Package: blit
Version: $VERSION
Architecture: $ARCH
Maintainer: Your Name <your@email.com>
Description: Fast file transfer utility
 Blit is a high-performance file transfer system with support
 for efficient local and remote operations.
EOF

# Create systemd service
cat > "$PKG_DIR/etc/systemd/system/blitd.service" <<EOF
[Unit]
Description=Blit Daemon
After=network.target

[Service]
Type=simple
ExecStart=/usr/bin/blitd
Restart=on-failure

[Install]
WantedBy=multi-user.target
EOF

# Build package
dpkg-deb --build "$PKG_DIR"

echo "Package built: ${PKG_DIR}.deb"
```

**RPM package** (`scripts/build_rpm.sh`):

```bash
#!/bin/bash
# scripts/build_rpm.sh

# Create RPM spec file
cat > blit.spec <<EOF
Name:           blit
Version:        0.1.0
Release:        1%{?dist}
Summary:        Fast file transfer utility

License:        MIT
Source0:        blit-%{version}.tar.gz

%description
Blit is a high-performance file transfer system.

%prep
%setup -q

%build
cargo build --release

%install
mkdir -p %{buildroot}/%{_bindir}
install -m 755 target/release/blit-cli %{buildroot}/%{_bindir}/blit
install -m 755 target/release/blit-daemon %{buildroot}/%{_bindir}/blitd

%files
%{_bindir}/blit
%{_bindir}/blitd

%changelog
* $(date "+%a %b %d %Y") Your Name <your@email.com> - 0.1.0-1
- Initial package
EOF

# Build RPM
rpmbuild -ba blit.spec
```

**macOS installer** (`scripts/build_macos.sh`):

```bash
#!/bin/bash
# scripts/build_macos.sh

# Build universal binary
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin

# Create universal binary
lipo -create \
  target/x86_64-apple-darwin/release/blit-cli \
  target/aarch64-apple-darwin/release/blit-cli \
  -output blit-universal

# Create installer package
pkgbuild --root . \
  --identifier com.yourcompany.blit \
  --version 0.1.0 \
  --install-location /usr/local/bin \
  blit-0.1.0.pkg
```

**Windows installer** (using WiX or Inno Setup):

```toml
# Using cargo-wix
[package.metadata.wix]
upgrade-guid = "..."
path-guid = "..."
license = false
eula = false
```

### Task 4.3.2: Distribution Automation
**Priority**: 🟡 Important
**Effort**: 2-3 hours

**GitHub Actions for releases**:

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Build
        run: cargo build --release

      - name: Package (Linux)
        if: matrix.os == 'ubuntu-latest'
        run: ./scripts/build_deb.sh

      - name: Upload artifacts
        uses: actions/upload-artifact@v2
        with:
          name: blit-${{ matrix.os }}
          path: target/release/blit*

  release:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - name: Create Release
        uses: actions/create-release@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tag_name: ${{ github.ref }}
          release_name: Release ${{ github.ref }}
          draft: false
          prerelease: false
```

## Day 6: Documentation & Migration (6-8 hours)

### Task 4.4.1: User Documentation
**Priority**: 🔴 Critical
**Effort**: 3-4 hours

**Create** `docs/user_guide.md`:

```markdown
# Blit v2 User Guide

## Installation

### Linux (Debian/Ubuntu)
```bash
wget https://github.com/yourorg/blit/releases/download/v0.1.0/blit_0.1.0_amd64.deb
sudo dpkg -i blit_0.1.0_amd64.deb
```

### macOS
```bash
brew install blit
# Or download from releases
```

### Windows
Download installer from releases page.

## Quick Start

### Local Operations

Mirror a directory:
```bash
blit mirror /source/path /destination/path
```

Copy files:
```bash
blit copy /source/file.txt /dest/file.txt
```

### Remote Operations

Start daemon:
```bash
blitd --config /etc/blit/config.toml
```

Push to remote:
```bash
blit push /local/path blit://hostname:port/module
```

Pull from remote:
```bash
blit pull blit://hostname:port/module/path /local/destination
```

List remote files:
```bash
blit list blit://hostname:port/module/path
```

## Configuration

Create `/etc/blit/config.toml`:
```toml
[daemon]
listen_addr = "0.0.0.0:50051"

[[modules]]
name = "data"
path = "/srv/blit/data"
read_only = false

[[modules]]
name = "backups"
path = "/srv/blit/backups"
read_only = true
```

## Authentication

... (token-based auth documentation)

## Performance Tuning

... (buffer sizes, zero-copy settings, etc.)

## Troubleshooting

... (common issues and solutions)
```

### Task 4.4.2: Migration Guide
**Priority**: 🟡 Important
**Effort**: 2-3 hours

**Create** `docs/migration_v1_to_v2.md`:

```markdown
# Migrating from Blit v1 to v2

## Breaking Changes

### URL Format
**v1**: `hostname:module/path`
**v2**: `blit://hostname:port/module/path`

Migration script:
```bash
# Convert v1 URLs to v2
sed -i 's|blit\([^:]*\):\([^/]*\)/|blit://\1:50051/\2/|g' your_scripts.sh
```

### Command Changes
- `blit <src> <dst>` → `blit copy <src> <dst>` or `blit mirror <src> <dst>`
- Daemon config format changed (see new config.toml format)

### Module Configuration
v1 used `/etc/blit.conf`, v2 uses `/etc/blit/config.toml`.

Migration:
```bash
# Convert old config
python3 scripts/migrate_config.py /etc/blit.conf > /etc/blit/config.toml
```

## New Features in v2

- Authentication tokens
- Improved progress reporting
- Hybrid transport for maximum performance (gRPC control + TCP data)
- Zero-copy data transfer
- Better error messages

## Side-by-Side Deployment

You can run v1 and v2 simultaneously:
- v1 daemon on default port
- v2 daemon on port 50051

## Performance Comparison

... (benchmark results showing v2 parity or improvement)

## Support

... (where to get help)
```

### Task 4.4.3: API Documentation
**Priority**: 🟢 Nice to have
**Effort**: 1-2 hours

```bash
# Generate API docs
cargo doc --no-deps --workspace

# Publish to GitHub Pages or docs site
```

## Quality Gate Checklist

Final quality gate before v2 release:

### Security
- [ ] Token-based authentication functional
- [ ] No secrets in code or logs
- [ ] Cryptographically strong tokens (JWT with nonce/expiry)
- [ ] Socket binding prevents replay attacks

### Testing
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] Performance regression tests pass
- [ ] Cross-platform tests pass
- [ ] Manual test scenarios complete

### Packaging
- [ ] Linux package builds
- [ ] macOS package builds
- [ ] Windows package builds
- [ ] Installation tested on clean systems

### Documentation
- [ ] User guide complete
- [ ] Migration guide complete
- [ ] API documentation generated
- [ ] README updated
- [ ] CHANGELOG maintained

### Quality
- [ ] No compiler warnings
- [ ] Clippy clean (cargo clippy)
- [ ] Code formatted (cargo fmt)
- [ ] Code coverage >80% (if measured)

### Operational
- [ ] Systemd service file working
- [ ] Log rotation configured
- [ ] Monitoring/metrics (if applicable)
- [ ] Deployment automation tested

## Definition of Done

Phase 4 is complete when:

1. ✅ All security features implemented
2. ✅ Comprehensive test suite passing
3. ✅ Platform packages built and tested
4. ✅ Complete documentation published
5. ✅ Migration path validated
6. ✅ All quality gates passed
7. ✅ v2 ready for production deployment

## Release Checklist

Before tagging v2.0.0:

1. [ ] Update version in `Cargo.toml` files
2. [ ] Update CHANGELOG.md
3. [ ] Run full test suite one final time
4. [ ] Build all platform packages
5. [ ] Test packages on clean VMs
6. [ ] Create Git tag: `git tag -a v2.0.0 -m "Blit v2.0.0 Release"`
7. [ ] Push tag: `git push origin v2.0.0`
8. [ ] GitHub Actions builds release artifacts
9. [ ] Create GitHub Release with artifacts
10. [ ] Announce release
11. [ ] Update documentation site
12. [ ] Monitor early adoption for issues

## Post-Release

### Week 1
- Monitor issue tracker
- Quick bug fixes if needed
- User support

### Month 1
- Gather performance feedback
- Identify improvement areas
- Plan v2.1 features

## Success Metrics

### Functional
- [ ] All v1 operations supported
- [ ] Feature parity achieved
- [ ] No critical bugs

### Performance
- [ ] ≥95% of v1 speed (validated in Phase 2.5)
- [ ] Memory usage acceptable
- [ ] CPU utilization reasonable

### Adoption
- [ ] Successful installations on target platforms
- [ ] Positive user feedback
- [ ] Migration from v1 proceeding

## Appendix: Release Artifacts

Expected release artifacts:
- `blit_2.0.0_amd64.deb` - Debian package
- `blit-2.0.0-1.x86_64.rpm` - RPM package
- `blit-2.0.0-macos-universal.pkg` - macOS installer
- `blit-2.0.0-windows-x64.msi` - Windows installer
- `blit-2.0.0-source.tar.gz` - Source tarball
- Checksums and signatures

## Document History

| Date | Author | Change |
|------|--------|--------|
| 2025-10-16 | Claude | Initial Phase 4 workflow |
