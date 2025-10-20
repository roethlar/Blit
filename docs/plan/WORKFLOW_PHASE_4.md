# Phase 4: Production Hardening & Packaging

**Goal**: Prepare Blit v2 for delivery – installers/packages, service documentation, cross-platform integration tests, and final polish on configuration/discovery.  
**Prerequisites**: Phase 3 gate complete.  
**Status**: Not started.

---

## 1. Success Criteria

- Installable artifacts produced for supported platforms (Linux, macOS, Windows).
- Installation & configuration documentation covering: daemon config (`config.toml` / `--root`), mDNS, service management.
- End-to-end integration suite covering local + remote use cases runs cleanly in CI.
- Logging, error messaging, and diagnostics reviewed for production readiness.
- Release notes summarise benchmark data, feature set, and support matrix.

---

## 2. Work Breakdown

### 4.1 Packaging & Distribution
| Task | Description | Deliverable |
|------|-------------|-------------|
| 4.1.1 | Define packaging targets (tarballs, Debian/RPM, Homebrew formula, Windows installer/script). | Packaging plan documented. |
| 4.1.2 | Create packaging scripts/pipelines (e.g., `scripts/package/*.sh`, WiX/NSIS for Windows). | Build artifacts + automated scripts. |
| 4.1.3 | Verify runtime dependencies (mdns, config dirs, service accounts) included or documented. | Dependency checklist + packaging notes. |

### 4.2 Installation & Configuration Docs
| Task | Description | Deliverable |
|------|-------------|-------------|
| 4.2.1 | Document daemon configuration (`/etc/blit/config.toml`, `--root`, module definitions, read-only flags). | Updated docs (`docs/cli/blit-daemon.1.md`, README snippets). |
| 4.2.2 | Provide service setup guides (systemd unit, launchd plist, Windows service instructions) with mDNS considerations. | OS-specific setup docs. |
| 4.2.3 | Document CLI usage (`copy`, `mirror`, `move`, `scan`, `list`) and `blit-utils` verbs with examples. | Updated CLI manpages + quick-start guide. |

### 4.3 Integration & Regression Testing
| Task | Description | Deliverable |
|------|-------------|-------------|
| 4.3.1 | Build end-to-end test suite covering local + remote workflows, admin verbs, gRPC fallback, error cases. | Automated integration tests (CI job or scripts). |
| 4.3.2 | Establish continuous regression runs (GitHub Actions/CI) with platform coverage. | CI pipeline updated; test matrix documented. |
| 4.3.3 | Capture benchmark snapshots (Phase 2.5 + remote scenarios) and include in docs/release notes. | Benchmark report + archived logs. |

### 4.4 UX & Diagnostics Polish
| Task | Description | Deliverable |
|------|-------------|-------------|
| 4.4.1 | Review logging levels and error messages across CLI/daemon/utils (no panic dumps, actionable errors). | Logging guide + code updates. |
| 4.4.2 | Ensure `blit diagnostics perf`, `blit-utils profile`, and other support tools produce useful output. | Sanity tests + docs. |
| 4.4.3 | Verify configuration precedence (CLI flag → config file) and document recommended deployment defaults. | Config documentation + tests. |

### 4.5 Release Preparation
| Task | Description | Deliverable |
|------|-------------|-------------|
| 4.5.1 | Assemble release notes (feature list, benchmarks, platform support, known issues). | Release notes draft. |
| 4.5.2 | Update change log / versioning metadata. | `CHANGELOG.md` (or equivalent) entry. |
| 4.5.3 | Final QA checklist: run through installation, configuration, transfer, admin tooling on all platforms. | QA report covering pass/fail and follow-up items. |

---

## 3. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Packaging complexity | Shipping delays | Automate builds early; reuse existing scripts where possible |
| Platform-specific issues (mdns, services) | Installation friction | Test on real/VM environments; document workarounds |
| Documentation drift | User confusion | Update docs alongside code changes; review before release |
| Benchmark regressions late in cycle | Release slip | Re-run key benchmarks after major changes; track in DEVLOG |

---

## 4. Exit Checklist

- [ ] Packaging scripts produce artifacts for all target platforms.
- [ ] Installation/configuration docs cover modules, root exports, mDNS, service management.
- [ ] Integration suite passes across supported OSes (including remote scenarios and admin tooling).
- [ ] Logging/error output reviewed and polished.
- [ ] Benchmark results recorded and referenced in release notes.
- [ ] Release notes + changelog drafted; QA checklist completed.

When every item above is complete, Blit v2 is ready for public packaging or internal deployment trials.
