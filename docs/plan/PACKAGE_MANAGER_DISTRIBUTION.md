# Package-manager distribution

**Status**: Draft
**Created**: 2026-08-12
**Supersedes**: nothing
**Decision ref**: D-2026-08-12-1 (D1), D-2026-08-12-2 (identifiers), D-2026-08-12-3 (one cargo command or no cargo user channel). Remaining D3 A/B + D4–D5 + Draft→Active pending.

## Goal

A user can install `blit` and `blit-daemon` through Homebrew, AUR, winget,
Scoop, and Cargo. Per D-2026-08-12-1 there are two payload lanes:

- **Archive lane:** the GitHub Release binaries for that tag (macOS/Windows
  already signed in CI; Linux unsigned by standing policy).
- **Source lane:** rebuild from the tagged source (Homebrew core, AUR
  source). Unsigned by nature. Must still emit the same
  `<version>+<12-char tag SHA>` identity as the CI archives so mixed
  archive/source peers pass D-2026-07-05-2.

GitHub Releases remain the canonical prebuilt host. Cargo is source and
is still D3.

## Non-goals

- New platform installers (macOS `.pkg`/`.dmg`, Windows Setup/MSI, AppImage)
  unless a later owner decision requires one for a specific channel.
- systemd, launchd, Windows service, or login-item install of `blit-daemon`.
- Packaging `blit-tui`, `blit-console-core`, the unfinished console GUI, or
  `blit-prometheus-bridge`.
- Extra **archive** CPU targets (Intel macOS, Linux ARM, Windows ARM).
  The archive lane uses the three CI artifacts. The source lane may
  compile on whatever arch the manager builds (Intel Mac, Homebrew-on-Linux)
  without us adding new CI archives.
- Mac App Store, Microsoft Store, Flatpak, Snap, nixpkgs, Chocolatey, apt,
  dnf. Homebrew core **is** in scope (D-2026-08-12-1).
- Changing transfer behavior, CLI surface, same-build handshake
  (D-2026-07-05-2), or the `build-release` / `publish-release` job shape.
- Copying `../certs` into this repo, committing signing material, or adding
  a second signing path beside the existing CI secrets.
- Advertising a channel in README/docs before that channel is live and
  installable by a stranger.
- A user-facing `cargo install` that names two crates or is run twice
  to get `blit` and `blit-daemon`. Owner, 2026-08-12: two-crate cargo
  install is a terrible user experience. Rejected.

## Constraints

- **FAST, SIMPLE, RELIABLE** applies to the user-facing install contract. No
  new user-visible CLI flags. Package managers install the two shipped
  binaries onto `PATH` and stop.
- **Two lanes, one version identity (D-2026-08-12-1).** Archive packages
  install the exact GitHub Release archive for that tag (same URL, same
  SHA-256 as the `.sha256` sidecar) and do not rebuild or re-sign.
  Source packages rebuild from the tagged source and do not re-sign.
  Cargo is specified under D3. Both lanes of the same tag must produce
  peers that pass the same-build hello.
- **One version.** Workspace `[workspace.package] version` is canonical.
  Tag is `v<version>`. Manifests report that version, not a floating `latest`.
- **Signing already exists and is the signing path.**
  `.github/workflows/ci.yml` `build-release` signs `blit`/`blit-daemon`
  when the Apple and Azure secret sets are present: Developer ID Application
  + hardened runtime + timestamp + `notarytool` (unstapled by design; flat
  executables have no staple target), and Azure Trusted Signing with RFC 3161.
  Those secrets are the same certificate family used by sibling apps
  (`../certs` Developer ID p12 + App Store Connect API key; Azure account
  `roethlar-app-signing`). Package-manager work consumes the signed bytes.
  It does not import `../certs` locally and does not change the sign/notarize
  steps. Forks without secrets still package unsigned, as today.
- **D-2026-07-05-2 same-build only.** Session hello is
  `CARGO_PKG_VERSION + BLIT_GIT_SHA`. CI tag builds have `.git` and emit
  the 12-char tag SHA. GitHub tag tarballs, AUR/homebrew source trees, and
  `crates.io` tarballs do **not** have `.git`; today's
  `crates/blit-core/build_identity.rs` then emits `unknown.<nonce>`.
  Source-lane packages must therefore export the tag's 12-char SHA into
  the build (see Design: source identity). Do not relax the hello.
- **`blit-core` build.rs reads `../../proto`.** A crates.io package of
  `blit-core` does not include that tree unless the crate `include`s it (or
  generated prost sources are vendored). Path deps across the workspace must
  become versioned deps in publish order.
- **Crate name `blit` is taken on crates.io** (unrelated 2D sprite library,
  `blit` 0.8.3). The workspace crate that produces the `blit` binary is
  already named `blit-cli`.
- **Current release assets** (v0.1.2 and the `publish-release` contract):
  `blit-x86_64-unknown-linux-gnu.tar.gz`,
  `blit-aarch64-apple-darwin.tar.gz`,
  `blit-x86_64-pc-windows-msvc.zip`, each with a `.sha256` sidecar. Each
  archive contains `blit`/`blit.exe`, `blit-daemon`/`blit-daemon.exe`,
  `README.md`, `LICENSE`, `CHANGELOG.md`, `BUILD.txt`.
- **Outward acts stay owner-gated.** Creating/updating AUR packages, a
  Homebrew tap, a `homebrew/core` PR, winget-pkgs, a Scoop bucket, and
  `cargo publish` all publish. Generators and tests may land in-repo; the
  first live push/PR/publish of each channel waits for an explicit go
  naming that channel.
- **Do not block `publish-release` on package-manager merge.** External
  review latency must not delay the canonical GitHub Release.
- **No secret leakage.** External package repos receive only public asset
  URLs and digests.

## Acceptance criteria

- [x] D1 recorded (D-2026-08-12-1): both archive and source lanes.
- [x] Identifier set recorded (D-2026-08-12-2). Draft→Active still waits on remaining interview decisions.
- [ ] `build.rs` honors a pre-set `BLIT_GIT_SHA` (or `BLIT_RELEASE_SHA`)
      so a git-less tree can emit the tag SHA. Guard: without git and
      without the env, identity is still a non-colliding `unknown.<nonce>`;
      with the env set to a 12-char SHA, `--version` is `<ver>+<sha>` and
      two such builds hello. Red-prove the nonce path still refuses.
- [ ] Deterministic generator produces **archive** stubs (Homebrew tap
      formula, AUR `-bin`, Scoop, winget) from version + three sidecar
      SHA-256s, **and source** stubs (Homebrew core formula, AUR source)
      from version + GitHub tag-tarball SHA-256 + the tag's 12-char SHA.
      Snapshot-tested. Digests never hand-edited.
- [ ] Each authorized channel is installable by a stranger on that OS,
      putting `blit` and `blit-daemon` on `PATH` at the release version.
      `--version` on both binaries is `<version>+<12-char tag SHA>` for
      **both** lanes.
- [ ] Archive-lane macOS/Windows binaries still carry the CI signatures
      (Developer ID Application / valid Authenticode). Not re-signed.
      Source-lane binaries are unsigned; docs do not claim otherwise.
- [ ] Cargo path, if D3 = one crates.io package: a single
      `cargo install <name> --locked` installs both `blit` and
      `blit-daemon`, both reporting `<ver>+<tagsha>`, and those binaries
      hello the GitHub archives of that tag. Docs never show two cargo
      install commands. If D3 drops cargo as a user channel, README does
      not advertise `cargo install` at all.
- [ ] README lists only live channels, as install options next to the
      existing source-build / GitHub Release path. No channel advertised
      before it is queryable.
- [ ] `scripts/agent/check-docs.sh` green. Generator tests red-prove a
      digest mismatch. No user-facing CLI change.

## Design

### Architecture

```
tag vX.Y.Z
  → existing CI build-release (sign macOS/Windows when secrets present)
  → existing publish-release attaches the six files atomically
  → in-repo generator reads:
       version + three archive .sha256 sidecars
       + GitHub tag-tarball sha256 + 12-char tag SHA
  → writes stubs under dist/package-managers/ (or packaging/)
  → maintainer (owner-gated) updates:
       tap (archive) | homebrew/core (source)
       AUR -bin (archive) | AUR source
       Scoop | winget
  → cargo (if authorized) is a separate source publish
```

GitHub Releases stay the prebuilt download path. Archive packages are
pointers. Source packages rebuild and must inject the tag SHA.

### Signing

No new signing implementation. Confirm, do not replace:

| Platform | Mechanism already in CI | Secret / cert source |
|---|---|---|
| macOS | `codesign` Developer ID + `notarytool` | `APPLE_CERTIFICATE` (p12 from `../certs`) + API key p8 |
| Windows | `Invoke-TrustedSigning` | Azure Trusted Signing (`roethlar-app-signing`) |
| Linux | none | standing policy |

Package-manager CI and external PRs never receive those secrets.

### Source identity (required by D1 = both lanes)

`crates/blit-core/build.rs` today always derives `BLIT_GIT_SHA` from git.
Change (pm-1, before any source package is advertised):

1. If the process environment already has a non-empty `BLIT_GIT_SHA` or
   `BLIT_RELEASE_SHA`, use that value (12-char lowercase hex; reject
   anything else rather than passing it through).
2. Else use today's git derivation.
3. Else `unknown.<nonce>` — never a shared fallback.

Source-lane formulas/PKGBUILDs export the **tag commit's** 12-char SHA
(the same suffix CI archives print). Generator bakes that SHA into the
stub. Two source builds of the same stub, and a source build vs a CI
archive of that tag, must hello.

Do not amend a commit to insert a SHA. Do not put certs or tokens in
manifests. crates.io still cannot set env for `cargo install`; that
remains a D3 file-bake if crates.io is chosen.

### Archive channels

Each archive package:

1. Downloads the platform archive from
   `https://github.com/roethlar/Blit/releases/download/v<ver>/blit-<target>.<ext>`.
2. Verifies SHA-256 against the sidecar value baked into the manifest.
3. Installs `blit` and `blit-daemon` onto the manager's bindir.
4. Does not start the daemon, write `/etc/blit`, or register a service.

### Source channels

Each source package:

1. Fetches the GitHub tag source tarball
   `https://github.com/roethlar/Blit/archive/refs/tags/v<ver>.tar.gz`
   (or the equivalent `.../archive/<tagsha>.tar.gz` if a formula needs a
   content-addressed URL). Verifies SHA-256.
2. Exports `BLIT_GIT_SHA=<12-char tag SHA>`.
3. `cargo build --release --locked` (or Homebrew `cargo install --locked`
   from the workspace) of `blit` and `blit-daemon` only.
4. Installs those two binaries onto the manager's bindir. Same no-service
   rule as the archive lane.

Winget and Scoop have no useful source-package convention; they stay
archive-only.

Channel-specific shape (identifiers bound by D-2026-08-12-2):

| Channel | Lane | Payload | Landing place | Notes |
|---|---|---|---|---|
| Homebrew tap | archive | `blit-aarch64-apple-darwin.tar.gz` | tap `roethlar/blit`, formula `blit-bin` | Signed CLI binaries. Name is `blit-bin` so it can coexist with core `blit`. arm64-only until an Intel archive exists. |
| Homebrew core | source | GitHub tag tarball | `homebrew/core` formula `blit` | Unsigned local build. May compile on Intel Mac and Linuxbrew. PR is owner-gated; long review is expected — park, do not spin. |
| AUR `-bin` | archive | `blit-x86_64-unknown-linux-gnu.tar.gz` | `blit-bin` | Thin wrap. |
| AUR source | source | GitHub tag tarball | `blit` | `makedepends=(cargo)` (and the usual base-devel). Exports `BLIT_GIT_SHA`. |
| Scoop | archive | `blit-x86_64-pc-windows-msvc.zip` | owner bucket, app `blit` | Zip + shims. No source package. |
| winget | archive | same zip, portable nested installer | `microsoft/winget-pkgs` id `Roethlar.Blit` | No Setup.exe. If winget-pkgs rejects zip/portable, stop and ask; do not invent an installer in that slice. |

### Cargo

Owner rejected a two-crate / two-command `cargo install` (2026-08-12).
D3 is no longer “git vs crates.io as two user crates.” It is:

**A — one crates.io package, one command, both binaries (recommended).**
User runs `cargo install blit-bin --locked` (name matches the tap/AUR
archive package; crate name `blit` is taken). That one package declares
both `[[bin]]` targets (`blit` and `blit-daemon`). Workspace crate names
`blit-cli` / `blit-daemon` may remain as members; they are not what the
user types. Libraries `blit-core` and `blit-app` publish only as
dependencies. Still required:

1. **Identity bake.** Tag SHA written into a crate-local file that
   `build_identity.rs` reads when git cannot see the original repo.
   `cargo install` of that version emits `<ver>+<tagsha>`, not
   `unknown.<nonce>`.
2. **proto in `blit-core`.** Package `include`s the workspace `proto/`
   tree so `build.rs` compiles from a crates.io unpack.
3. **Path → version deps**; publish order libraries then the one
   install package. `publish = false` on `blit-tui`,
   `blit-console-core`, `blit-prometheus-bridge`, and any leftover
   binary crate the user must not `cargo install`.
4. **`--locked`** is the documented install.

This qualifies D-2026-08-12-2: users do not install `blit-cli` and
`blit-daemon` as two crates. Binary names stay `blit` / `blit-daemon`.

**B — cargo is not a user channel.** No crates.io user package. README
does not advertise `cargo install`. Developers clone and
`cargo build --release`. brew / AUR / winget / Scoop are the install UX.

Do not relax D-2026-07-05-2 to make crates.io easier. Do not document
two `cargo install` lines, including `--git --bin` twice.

### Generator

New in-repo tool, same job as AMKB-GUI's `build_tools/package_managers`
without copying that Python package. Prefer `scripts/package_managers/`
(Python 3, no new runtime; this repo already uses Python for
`scripts/release_smoke.py`).

Input: `--version`, `--tag-sha` (12-char), directory of the three archive
`.sha256` sidecars, and the GitHub source-tarball SHA-256 (passed in or
hashed from a provided file). `--out`.
Output: written stub trees only. Snapshot tests never hit the network.
A live generate may fetch the GitHub tag tarball to hash it; that is a
separate `fetch` helper, not the generate path. No AUR/brew/winget/Scoop
push.

Source stubs must contain the exported `BLIT_GIT_SHA=<tag-sha>` line.
Archive stubs must not rebuild.

Snapshot tests under `scripts/package_managers/testdata/` with a fixed
fake version, fake tag SHA, and fake digests. A focused test fails if a
digest is rewritten by hand in a fixture that claims to be generated.

### Docs

After a channel is live: add its install command as one option in README
(and a short `docs/installing.md` if the README section would overflow).
Keep the GitHub Release and source-build paths. Options only — no trust
essay, no "unsigned Linux" framing, no instruction to disable Gatekeeper
or SmartScreen.

### Release process (after first live success)

Maintainer steps, manual until a later automation slice:

1. Wait for tag CI `publish-release` green and the six assets live.
2. Hash the GitHub tag source tarball; run the generator with archive
   sidecars + source sha256 + tag SHA.
3. Update tap, AUR `-bin`, AUR source, Scoop, winget PR, and (when
   live) the homebrew/core bump.
4. After each package is queryable, add or bump that README option.

Do not fold those updates into `publish-release`.

## Slices

One coherent, testable change each. No slice starts without Draft→Active
and the decisions it names. Owner-gated publish is a go per channel, not
covered by Active.

1. **pm-0 — Record remaining D3 (and D4/D5 if ruled); flip Active.**
   Docs only. D1/D2 are D-2026-08-12-1/-2. No product code.
2. **pm-1 — Honor pre-set `BLIT_GIT_SHA` / `BLIT_RELEASE_SHA` in
   `crates/blit-core/build.rs`.** Validation + nonce fallback unchanged.
   Tests: env set → exact suffix; env absent + no git → nonce, two
   independent builds disagree; env garbage → fail the build. This lands
   before any source package is advertised.
3. **pm-2 — Generator + snapshot tests.** Archive stubs (tap `blit-bin`,
   AUR `-bin`, Scoop, winget) from version + three sidecars. Source stubs
   (Homebrew core, AUR source) from version + source-tarball sha256 +
   tag SHA (the `BLIT_GIT_SHA=` export). Refuse a missing digest. No
   network on generate, no push. Red-prove one digest assertion.
4. **pm-3 — Homebrew tap live (archive).** Owner tap, generated
   `blit-bin` formula from a published tag. Prove `brew install` on arm64
   macOS: both binaries on PATH, version identity, `codesign` still
   Developer ID. Docs after proof.
5. **pm-4 — Homebrew core PR (source).** Generated source formula.
   Prove a local `brew install --build-from-source` emits
   `<ver>+<tagsha>` and hellos a CI archive of that tag. Owner opens the
   `homebrew/core` PR. If review stalls, park; do not spin. Docs after
   `brew install blit` from core actually works.
6. **pm-5 — AUR `blit-bin` live (archive).** Owner pushes
   `ssh://aur@aur.archlinux.org/blit-bin.git`. Prove install. Docs after
   the AUR page exists. If AUR create is locked, park; continue.
7. **pm-6 — AUR source live.** Same AUR caveats. Prove `makepkg` exports
   `BLIT_GIT_SHA`, both binaries match the tag identity, and a source
   install hellos an archive-lane binary of that tag.
8. **pm-7 — Scoop bucket live (archive).** Prove `scoop install` on
   Windows; Authenticode still Valid; both binaries shimmed. Docs after.
9. **pm-8 — winget portable zip (archive).** Prove
   `winget install --manifest` locally, then owner PR to
   `microsoft/winget-pkgs`. Zip rejection → stop and ask; no installer.
10. **pm-9 — Cargo, only if D3 = A.** One install package with both
    `[[bin]]`s; file-bake of the tag SHA; proto include; publish
    metadata; `publish = false` on everything the user must not type;
    `cargo publish --dry-run` in order. First real publish is
    owner-gated. Prove `cargo install <name> --locked` drops both
    binaries, they hello each other and a GitHub binary of that tag.
    If D3 = B, this slice is: delete any cargo-install claim from docs;
    do not publish.
11. **pm-10 — Release-process note.** Maintainer section listing the
    generator command and the owner-gated external updates for **both**
    lanes. No CI automation in this slice.

## Open questions

Answered in chat one at a time; each recorded as a `docs/DECISIONS.md`
entry before the slice that needs it. Recommendations are not decisions.

- **D1 — Payload.** **Recorded D-2026-08-12-1:** both archive and source
  lanes. Homebrew core and AUR source are in scope. Winget/Scoop stay
  archive-only.
- **D2 — Identifiers.** **Recorded D-2026-08-12-2** for brew / AUR /
  Scoop / winget and for binary names `blit` / `blit-daemon`. The
  crates.io *user* package name is reopened by the two-crate UX
  rejection; D3 binds it.
- **D3 — Cargo user install.** Two-crate / two-command install is
  rejected. Remaining pick: **A** one crates.io package
  (`cargo install blit-bin --locked` → both binaries) or **B** cargo
  is not a user channel.
- **D4 — First version to publish.** Recommended: start from existing
  `v0.1.2` assets (source lane needs the tag tarball of that same tag).
  Do not wait for 1.0 or the console.
- **D5 — Automation.** Recommended: manual generator + owner push/PR
  until each channel has one successful live update.

## Risks

| Risk | Handling |
|---|---|
| winget-pkgs rejects portable zip | pm-8 stops and asks; no installer surprise |
| AUR package creation locked | park pm-5/pm-6; other channels continue |
| homebrew/core review is slow or rejects the name | park pm-4; tap `blit-bin` still ships signed macOS |
| `blit` name collisions (brew/scoop/AUR) | identifiers are bound (D-2026-08-12-2); if a registry rejects one, stop and ask |
| Source build without injected SHA | `unknown.<nonce>` refuse; pm-1 + stub export prevent this |
| crates.io identity/`proto` miss | cargo install peers refuse each other; pm-9 guards this |
| Docs advertise a dead command | docs only after live proof |
| Second signing path / certs in repo | forbidden by Constraints |
| Archive lane rebuilds or re-signs | forbidden |
| Source lane claimed as signed | forbidden; docs must not say that |
| `publish-release` waits on brew/AUR/winget | forbidden |
| SmartScreen / first-run Gatekeeper | existing changelog caveats; do not tell users to disable OS security |
