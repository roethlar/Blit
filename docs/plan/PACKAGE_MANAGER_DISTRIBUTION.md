# Package-manager distribution

**Status**: Draft
**Created**: 2026-08-12
**Supersedes**: nothing
**Decision ref**: pending owner Draft→Active flip (and D1+ below)

## Goal

A user can install the shipped `blit` and `blit-daemon` binaries through
Homebrew, AUR, winget, Scoop, and Cargo, at the same version identity as the
GitHub Release for that tag. macOS and Windows package payloads are the
already-signed release binaries (Developer ID + notarization; Azure Trusted
Signing). Linux package payloads are the unsigned Linux release archive, which
is the existing policy. GitHub Releases remain the canonical binary host.

## Non-goals

- New platform installers (macOS `.pkg`/`.dmg`, Windows Setup/MSI, AppImage)
  unless a later owner decision requires one for a specific channel.
- systemd, launchd, Windows service, or login-item install of `blit-daemon`.
- Packaging `blit-tui`, `blit-console-core`, the unfinished console GUI, or
  `blit-prometheus-bridge`.
- Extra CPU targets (Intel macOS, Linux ARM, Windows ARM). First pass uses
  the three archives CI already publishes.
- Mac App Store, Microsoft Store, Flatpak, Snap, nixpkgs, Chocolatey, apt,
  dnf, or official `homebrew/core` submission unless later authorized.
- Changing transfer behavior, CLI surface, same-build handshake
  (D-2026-07-05-2), or the `build-release` / `publish-release` job shape.
- Copying `../certs` into this repo, committing signing material, or adding
  a second signing path beside the existing CI secrets.
- Advertising a channel in README/docs before that channel is live and
  installable by a stranger.

## Constraints

- **FAST, SIMPLE, RELIABLE** applies to the user-facing install contract. No
  new user-visible CLI flags. Package managers install the two shipped
  binaries onto `PATH` and stop.
- **One binary source.** brew / AUR / winget / Scoop install the exact GitHub
  Release archive for that tag (same URL, same SHA-256 as the `.sha256`
  sidecar). They do not rebuild or re-sign the payload. Cargo is source and
  is specified separately below because it cannot consume those archives.
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
  `CARGO_PKG_VERSION + BLIT_GIT_SHA`. Clean git builds at a tag share
  identity with the CI archives for that tag. `crates.io` source tarballs
  have no git: `crates/blit-core/build_identity.rs` then emits
  `unknown.<nonce>`, so two independent `cargo install`s of the same crate
  version refuse each other and refuse the GitHub binaries. Cargo publish
  is not a metadata-only change.
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
- **Outward acts stay owner-gated.** Creating/updating AUR, a Homebrew tap
  or cask PR, winget-pkgs, a Scoop bucket, and `cargo publish` all publish.
  Generators and tests may land in-repo; the first live push/PR/publish of
  each channel waits for an explicit go naming that channel.
- **Do not block `publish-release` on package-manager merge.** External
  review latency must not delay the canonical GitHub Release.
- **No secret leakage.** External package repos receive only public asset
  URLs and digests.

## Acceptance criteria

- [ ] D1 (payload/channel shape) and the identifier set (D2) recorded in
      `docs/DECISIONS.md`; this plan flipped Draft → Active.
- [ ] Deterministic in-repo generator produces Homebrew, AUR, winget, and
      Scoop stubs from a release's version + three SHA-256 sidecars.
      Snapshot-tested against golden files. Digests are never hand-edited.
- [ ] Each authorized channel is actually installable by a stranger on that
      OS, putting `blit` and `blit-daemon` on `PATH` at the release version.
      `--version` on both binaries equals `<version>+<12-char tag SHA>` for
      binary channels.
- [ ] macOS and Windows installed binaries still carry the CI signatures
      (Developer ID Application / valid Authenticode). Not re-signed.
- [ ] Cargo path, if D3 authorizes crates.io: `cargo install <chosen-cli-crate>
      --locked` and `cargo install <chosen-daemon-crate> --locked` of the
      same version produce peers that pass the same-build hello with each
      other **and** with the GitHub binaries of that tag. If D3 is git-only,
      `cargo install --git --tag` is the documented command and crates.io
      is not claimed.
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
  → in-repo generator reads version + the three .sha256 sidecars
  → writes stubs under dist/package-managers/ (or packaging/)
  → maintainer (owner-gated) updates:
       Homebrew tap formula | AUR blit-bin | Scoop manifest | winget PR
  → cargo (if authorized) is a separate source publish, not an archive wrap
```

GitHub Releases stay the download path. Package managers are pointers.

### Signing

No new signing implementation. Confirm, do not replace:

| Platform | Mechanism already in CI | Secret / cert source |
|---|---|---|
| macOS | `codesign` Developer ID + `notarytool` | `APPLE_CERTIFICATE` (p12 from `../certs`) + API key p8 |
| Windows | `Invoke-TrustedSigning` | Azure Trusted Signing (`roethlar-app-signing`) |
| Linux | none | standing policy |

Package-manager CI and external PRs never receive those secrets.

### Binary channels (brew, AUR, winget, Scoop)

Each package:

1. Downloads the platform archive from
   `https://github.com/roethlar/Blit/releases/download/v<ver>/blit-<target>.<ext>`.
2. Verifies SHA-256 against the sidecar value baked into the manifest.
3. Installs `blit` and `blit-daemon` onto the manager's bindir.
4. Does not start the daemon, write `/etc/blit`, or register a service.

Channel-specific shape (recommendations; identifiers are D2):

| Channel | Payload | Landing place (recommended) | Notes |
|---|---|---|---|
| Homebrew | `blit-aarch64-apple-darwin.tar.gz` | owner tap `roethlar/blit`, formula `blit` | Formula, not cask: two CLI binaries, no `.app`. arm64-only until an Intel archive exists. Official `homebrew/core` is a later, separate go (source rebuild, unsigned). |
| AUR | `blit-x86_64-unknown-linux-gnu.tar.gz` | `blit-bin` | Thin `-bin` wrap. Source AUR package is out of scope for the first pass. |
| Scoop | `blit-x86_64-pc-windows-msvc.zip` | owner bucket, app `blit` | Native zip + `bin` shims. |
| winget | same zip, `InstallerType: zip` + portable nested installer | `microsoft/winget-pkgs` id `Roethlar.Blit` | No Setup.exe exists. If winget-pkgs rejects portable zip, stop and ask; do not invent an installer in that slice. |

### Cargo

Two implementable shapes. D3 picks one before any publish-metadata work.

**Git-only (smaller, already identity-correct).** Document

```
cargo install --git https://github.com/roethlar/Blit.git --tag v<ver> --locked --bin blit
cargo install --git https://github.com/roethlar/Blit.git --tag v<ver> --locked --bin blit-daemon
```

A clean tag clone yields `BLIT_GIT_SHA` = the tag commit, matching CI
archives. No crates.io. Workspace path deps and `../../proto` keep working
because the full repo is present.

**crates.io (requested channel; extra slices).** Required before first
`cargo publish`:

1. **Names.** Cannot publish crate `blit`. Publish the existing `blit-cli`
   (binary name stays `blit`) and `blit-daemon`, plus their library
   dependencies `blit-core` and `blit-app`. Do not publish `blit-tui`,
   `blit-console-core`, or `blit-prometheus-bridge` unless later asked.
2. **Identity bake.** At publish of a tag, write the 12-char tag SHA into a
   crate-local file that `build_identity.rs` reads when `git rev-parse`
   cannot see the original repo. Independent `cargo install`s of that
   version must emit `<ver>+<tagsha>`, not `unknown.<nonce>`. The nonce
   path stays for genuinely unidentifiable trees.
3. **proto in the crate.** `blit-core` package `include`s the workspace
   `proto/` tree (or a copied `blit-core/proto/`) so `build.rs` compiles
   without `../../` walking out of the crates.io unpack.
4. **Path → version deps** on the four published crates; publish order
   `blit-core` → `blit-app` → `blit-cli` / `blit-daemon`. Add the crates.io
   required package fields (`description`, `repository`, `homepage`,
   `readme`, `keywords`/`categories` as needed). `publish = false` on the
   crates that must not go up.
5. **Lockfile.** Binary crates include enough lock information that
   `--locked` is the documented install.

Do not relax D-2026-07-05-2 to make crates.io easier.

### Generator

New in-repo tool, same job as AMKB-GUI's `build_tools/package_managers`
without copying that Python package. Prefer `scripts/package_managers/`
(Python 3, no new runtime; this repo already uses Python for
`scripts/release_smoke.py`).

Input: `--version`, directory of the three `.sha256` sidecars (or a
downloaded GitHub release), `--out`.
Output: written stub trees only. No network on the generate path except an
optional later `fetch` helper. No AUR/brew/winget/Scoop push.

Snapshot tests under `scripts/package_managers/testdata/` (or
`tests/` next to the script) with a fixed fake version and fake digests.
A focused test fails if a digest is rewritten by hand in a fixture that
claims to be generated.

### Docs

After a channel is live: add its install command as one option in README
(and a short `docs/installing.md` if the README section would overflow).
Keep the GitHub Release and source-build paths. Options only — no trust
essay, no "unsigned Linux" framing, no instruction to disable Gatekeeper
or SmartScreen.

### Release process (after first live success)

Maintainer steps, manual until a later automation slice:

1. Wait for tag CI `publish-release` green and the six assets live.
2. Run the generator against those sidecars.
3. Update the external package repos / open the winget PR.
4. After the package is queryable, add or bump the README option.

Do not fold those updates into `publish-release`.

## Slices

One coherent, testable change each. No slice starts without Draft→Active
and the decisions it names. Owner-gated publish is a go per channel, not
covered by Active.

1. **pm-0 — Record D1/D2/D3; flip Active.** Docs only. Bind payload model,
   identifiers, and cargo shape. Rewrite any recommendation below that
   lost. No product code.
2. **pm-1 — Generator + snapshot tests.** Homebrew formula, AUR
   `PKGBUILD`+`.SRCINFO`, Scoop manifest, winget multi-file manifest,
   all from version + sidecars. Refuse a missing platform digest. No
   network, no push. Red-prove one digest assertion.
3. **pm-2 — Homebrew tap live.** Create/update owner tap with the generated
   formula from a published tag (v0.1.2 is a valid first payload). Prove
   `brew install` on arm64 macOS: both binaries on PATH, version identity,
   `codesign` still Developer ID. Docs option only after that proof.
   Official `homebrew/core` is not this slice.
4. **pm-3 — AUR `blit-bin` live.** Generate from the Linux archive; owner
   pushes `ssh://aur@aur.archlinux.org/blit-bin.git`. Prove install on
   Arch (or an Arch container) puts both binaries on PATH. Docs after
   `https://aur.archlinux.org/packages/blit-bin` exists. If AUR create is
   locked (as it was for AMKB-GUI on 2026-08-08), park this slice and
   continue; do not spin.
5. **pm-4 — Scoop bucket live.** Owner bucket; prove `scoop install` on
   Windows; Authenticode still Valid; both binaries shimmed. Docs after.
6. **pm-5 — winget portable zip.** Generate manifests; owner PR to
   `microsoft/winget-pkgs`. Prove `winget install --manifest` locally
   before the PR. If the official repo rejects zip/portable, stop and
   ask — do not add a Windows installer inside this slice.
7. **pm-6 — Cargo, only if D3 = crates.io.** Identity bake + proto include
   + publish metadata + `publish = false` on unpublished members + a
   dry-run `cargo publish --dry-run` from a clean tag worktree for each
   of the four crates, in order. First real `cargo publish` is owner-gated
   and uses the owner's crates.io token. Prove two independent
   `cargo install`s of that version hello each other and a GitHub binary
   of the same tag. If D3 = git-only, this slice is docs-only: add the
   `--git --tag` commands, do not publish.
8. **pm-7 — Release-process note.** Short maintainer section (README or
   `docs/installing.md`) listing the generator command and the owner-gated
   external updates. No CI automation in this slice.

## Open questions

Answered in chat one at a time; each recorded as a `docs/DECISIONS.md`
entry before the slice that needs it. Recommendations are not decisions.

- **D1 — Payload for brew / AUR / winget / Scoop.** Recommended: GitHub
  Release archives only (the table above). Alternative: source rebuilds
  (homebrew-core / AUR source), which are unsigned and a different
  maintenance path.
- **D2 — Identifiers.** Recommended: tap `roethlar/blit` formula `blit`;
  AUR `blit-bin`; Scoop `blit`; winget `Roethlar.Blit`; crates `blit-cli`
  + `blit-daemon` + `blit-core` + `blit-app` (binary names unchanged).
- **D3 — Cargo shape.** Recommended: git-tag install first (identity
  already correct); crates.io as a follow-up only if the owner wants
  `cargo install blit-cli` without `--git`.
- **D4 — First version to publish.** Recommended: wrap existing `v0.1.2`
  assets; do not wait for 1.0 or the console.
- **D5 — Automation.** Recommended: manual generator + owner push/PR
  until each channel has one successful live update.

## Risks

| Risk | Handling |
|---|---|
| winget-pkgs rejects portable zip | pm-5 stops and asks; no installer surprise |
| AUR package creation locked | park pm-3; other channels continue |
| `blit` name collisions (brew/scoop/AUR) | D2 can pick `blit-bin` / `blit-transfer`; do not squat |
| crates.io identity/`proto` miss | cargo install peers refuse each other; pm-6 guards this |
| Docs advertise a dead command | docs only after live proof |
| Second signing path / certs in repo | forbidden by Constraints |
| Package rebuilds unsigned macOS/Windows binaries | forbidden by one-binary-source |
| `publish-release` waits on brew/AUR/winget | forbidden |
| SmartScreen / first-run Gatekeeper | existing changelog caveats; do not tell users to disable OS security |
