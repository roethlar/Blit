#!/usr/bin/env python3
"""Write Homebrew / AUR / Scoop / winget stubs from a release's digests.

No network. No push. Digests come from sidecar files and an explicit
source-tarball SHA-256. Live fetch belongs to a later helper.
"""

from __future__ import annotations

import argparse
import hashlib
import re
from pathlib import Path

LINUX_ARCHIVE = "blit-x86_64-unknown-linux-gnu.tar.gz"
MAC_ARCHIVE = "blit-aarch64-apple-darwin.tar.gz"
WINDOWS_ARCHIVE = "blit-x86_64-pc-windows-msvc.zip"

HOMEPAGE = "https://github.com/roethlar/Blit"
DEFAULT_ASSET_BASE = "https://github.com/roethlar/Blit/releases/download/v{version}"
SOURCE_TARBALL_URL = (
    "https://github.com/roethlar/Blit/archive/refs/tags/v{version}.tar.gz"
)
DESC = "High-performance file transfer CLI and daemon"
VERSION_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+$")
SHA_RE = re.compile(r"^[0-9a-f]{12}$")
DIGEST_RE = re.compile(r"^[0-9a-f]{64}$")


class GenerateError(RuntimeError):
    """A generator input was missing or malformed."""


def validate_version(version: str) -> str:
    if not VERSION_RE.fullmatch(version):
        raise GenerateError(f"version must be N.N.N, got {version!r}")
    return version


def validate_tag_sha(tag_sha: str) -> str:
    if not SHA_RE.fullmatch(tag_sha):
        raise GenerateError(
            f"tag SHA must be 12 lowercase hex digits, got {tag_sha!r}"
        )
    return tag_sha


def validate_digest(name: str, digest: str) -> str:
    digest = digest.lower()
    if not DIGEST_RE.fullmatch(digest):
        raise GenerateError(f"invalid SHA-256 for {name}: {digest!r}")
    return digest


def parse_sidecar(path: Path) -> tuple[str, str]:
    fields = path.read_text(encoding="utf-8-sig").strip().split()
    if len(fields) != 2 or not DIGEST_RE.fullmatch(fields[0].lower()):
        raise GenerateError(f"invalid SHA-256 sidecar: {path}")
    return fields[1], fields[0].lower()


def load_sidecars(directory: Path) -> dict[str, str]:
    if not directory.is_dir():
        raise GenerateError(f"sidecar directory is not a directory: {directory}")
    digests: dict[str, str] = {}
    for path in sorted(directory.glob("*.sha256")):
        name, digest = parse_sidecar(path)
        digests[name] = digest
    return digests


def require_digest(digests: dict[str, str], filename: str) -> str:
    try:
        return validate_digest(filename, digests[filename])
    except KeyError as exc:
        raise GenerateError(f"missing digest for {filename}") from exc


def asset_url(version: str, filename: str, asset_base: str) -> str:
    return f"{asset_base.rstrip('/')}/{filename}"


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


def homebrew_bin_formula(*, version: str, mac_sha: str, mac_url: str) -> str:
    return f"""class BlitBin < Formula
  desc "{DESC}"
  homepage "{HOMEPAGE}"
  version "{version}"
  license "MIT"

  on_macos do
    on_arm do
      url "{mac_url}"
      sha256 "{mac_sha}"
    end
    on_intel do
      odie "blit-bin ships only the aarch64-apple-darwin archive"
    end
  end

  def install
    bin.install "blit", "blit-daemon"
  end

  test do
    assert_match version.to_s, shell_output("#{{bin}}/blit --version")
    assert_match version.to_s, shell_output("#{{bin}}/blit-daemon --version")
  end
end
"""


def homebrew_core_formula(
    *, version: str, source_sha: str, source_url: str, tag_sha: str
) -> str:
    return f"""class Blit < Formula
  desc "{DESC}"
  homepage "{HOMEPAGE}"
  url "{source_url}"
  sha256 "{source_sha}"
  license "MIT"

  depends_on "rust" => :build

  def install
    ENV["BLIT_GIT_SHA"] = "{tag_sha}"
    system "cargo", "build", "--release", "--locked", "-p", "blit-cli", "-p", "blit-daemon"
    bin.install "target/release/blit", "target/release/blit-daemon"
  end

  test do
    assert_match version.to_s, shell_output("#{{bin}}/blit --version")
    assert_match "{tag_sha}", shell_output("#{{bin}}/blit --version")
    assert_match version.to_s, shell_output("#{{bin}}/blit-daemon --version")
  end
end
"""


def aur_bin_pkgbuild(*, version: str, linux_sha: str, linux_url: str) -> str:
    return f"""pkgname=blit-bin
pkgver={version}
pkgrel=1
pkgdesc='{DESC}'
arch=('x86_64')
url='{HOMEPAGE}'
license=('MIT')
provides=('blit')
conflicts=('blit')
source=('{linux_url}')
sha256sums=('{linux_sha}')

package() {{
  cd "blit-x86_64-unknown-linux-gnu"
  install -Dm755 blit "$pkgdir/usr/bin/blit"
  install -Dm755 blit-daemon "$pkgdir/usr/bin/blit-daemon"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}}
"""


def aur_bin_srcinfo(*, version: str, linux_sha: str, linux_url: str) -> str:
    return f"""pkgbase = blit-bin
	pkgdesc = {DESC}
	pkgver = {version}
	pkgrel = 1
	url = {HOMEPAGE}
	arch = x86_64
	license = MIT
	provides = blit
	conflicts = blit
	source = {linux_url}
	sha256sums = {linux_sha}

pkgname = blit-bin
"""


def aur_src_pkgbuild(
    *, version: str, source_sha: str, source_url: str, tag_sha: str
) -> str:
    return f"""pkgname=blit
pkgver={version}
pkgrel=1
pkgdesc='{DESC}'
arch=('x86_64' 'aarch64')
url='{HOMEPAGE}'
license=('MIT')
makedepends=('cargo')
provides=('blit')
conflicts=('blit-bin')
source=("$pkgname-$pkgver.tar.gz::{source_url}")
sha256sums=('{source_sha}')

prepare() {{
  cd "Blit-$pkgver"
  export RUSTUP_TOOLCHAIN=stable
  cargo fetch --locked --target "$CARCH-unknown-linux-gnu"
}}

build() {{
  cd "Blit-$pkgver"
  export BLIT_GIT_SHA={tag_sha}
  export RUSTUP_TOOLCHAIN=stable
  export CARGO_TARGET_DIR=target
  cargo build --release --locked -p blit-cli -p blit-daemon
}}

package() {{
  cd "Blit-$pkgver"
  install -Dm755 target/release/blit "$pkgdir/usr/bin/blit"
  install -Dm755 target/release/blit-daemon "$pkgdir/usr/bin/blit-daemon"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}}
"""


def aur_src_srcinfo(*, version: str, source_sha: str, source_url: str) -> str:
    return f"""pkgbase = blit
	pkgdesc = {DESC}
	pkgver = {version}
	pkgrel = 1
	url = {HOMEPAGE}
	arch = x86_64
	arch = aarch64
	license = MIT
	makedepends = cargo
	provides = blit
	conflicts = blit-bin
	source = blit-$pkgver.tar.gz::{source_url}
	sha256sums = {source_sha}

pkgname = blit
"""


def scoop_manifest(*, version: str, windows_sha: str, windows_url: str) -> str:
    return f"""{{
  "version": "{version}",
  "description": "{DESC}",
  "homepage": "{HOMEPAGE}",
  "license": "MIT",
  "architecture": {{
    "64bit": {{
      "url": "{windows_url}",
      "hash": "{windows_sha}",
      "extract_dir": "blit-x86_64-pc-windows-msvc"
    }}
  }},
  "bin": [
    "blit.exe",
    "blit-daemon.exe"
  ],
  "checkver": "github",
  "autoupdate": {{
    "architecture": {{
      "64bit": {{
        "url": "https://github.com/roethlar/Blit/releases/download/v$version/{WINDOWS_ARCHIVE}"
      }}
    }}
  }}
}}
"""


def winget_version_manifest(*, version: str) -> str:
    return f"""PackageIdentifier: Roethlar.Blit
PackageVersion: {version}
DefaultLocale: en-US
ManifestType: version
ManifestVersion: 1.6.0
"""


def winget_locale_manifest(*, version: str) -> str:
    return f"""PackageIdentifier: Roethlar.Blit
PackageVersion: {version}
PackageLocale: en-US
Publisher: Roethlar
PackageName: Blit
License: MIT
ShortDescription: {DESC}
PackageUrl: {HOMEPAGE}
ManifestType: defaultLocale
ManifestVersion: 1.6.0
"""


def winget_installer_manifest(
    *, version: str, windows_sha: str, windows_url: str
) -> str:
    return f"""PackageIdentifier: Roethlar.Blit
PackageVersion: {version}
InstallerLocale: en-US
InstallerType: zip
NestedInstallerType: portable
Installers:
- Architecture: x64
  InstallerUrl: {windows_url}
  InstallerSha256: {windows_sha.upper()}
  NestedInstallerFiles:
  - RelativeFilePath: blit-x86_64-pc-windows-msvc/blit.exe
    PortableCommandAlias: blit
  - RelativeFilePath: blit-x86_64-pc-windows-msvc/blit-daemon.exe
    PortableCommandAlias: blit-daemon
ManifestType: installer
ManifestVersion: 1.6.0
"""


def generate(
    *,
    version: str,
    tag_sha: str,
    digests: dict[str, str],
    source_sha256: str,
    out: Path,
    asset_base: str | None = None,
) -> list[Path]:
    version = validate_version(version)
    tag_sha = validate_tag_sha(tag_sha)
    source_sha = validate_digest("source tarball", source_sha256)
    base = (asset_base or DEFAULT_ASSET_BASE).format(version=version)
    linux_sha = require_digest(digests, LINUX_ARCHIVE)
    mac_sha = require_digest(digests, MAC_ARCHIVE)
    windows_sha = require_digest(digests, WINDOWS_ARCHIVE)
    linux_url = asset_url(version, LINUX_ARCHIVE, base)
    mac_url = asset_url(version, MAC_ARCHIVE, base)
    windows_url = asset_url(version, WINDOWS_ARCHIVE, base)
    source_url = SOURCE_TARBALL_URL.format(version=version)

    written = [
        out / "homebrew" / "blit-bin.rb",
        out / "homebrew" / "blit.rb",
        out / "aur" / "blit-bin" / "PKGBUILD",
        out / "aur" / "blit-bin" / ".SRCINFO",
        out / "aur" / "blit" / "PKGBUILD",
        out / "aur" / "blit" / ".SRCINFO",
        out / "scoop" / "blit.json",
        out / "winget" / "Roethlar.Blit" / "Roethlar.Blit.yaml",
        out / "winget" / "Roethlar.Blit" / "Roethlar.Blit.locale.en-US.yaml",
        out / "winget" / "Roethlar.Blit" / "Roethlar.Blit.installer.yaml",
    ]
    contents = [
        homebrew_bin_formula(version=version, mac_sha=mac_sha, mac_url=mac_url),
        homebrew_core_formula(
            version=version,
            source_sha=source_sha,
            source_url=source_url,
            tag_sha=tag_sha,
        ),
        aur_bin_pkgbuild(version=version, linux_sha=linux_sha, linux_url=linux_url),
        aur_bin_srcinfo(version=version, linux_sha=linux_sha, linux_url=linux_url),
        aur_src_pkgbuild(
            version=version,
            source_sha=source_sha,
            source_url=source_url,
            tag_sha=tag_sha,
        ),
        aur_src_srcinfo(version=version, source_sha=source_sha, source_url=source_url),
        scoop_manifest(
            version=version, windows_sha=windows_sha, windows_url=windows_url
        ),
        winget_version_manifest(version=version),
        winget_locale_manifest(version=version),
        winget_installer_manifest(
            version=version, windows_sha=windows_sha, windows_url=windows_url
        ),
    ]
    if len(written) != len(contents):
        raise GenerateError("internal stub list is unbalanced")
    for path, text in zip(written, contents):
        write(path, text)
    return written


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True)
    parser.add_argument("--tag-sha", required=True)
    parser.add_argument("--sidecars", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--source-sha256")
    parser.add_argument("--source-tarball", type=Path)
    parser.add_argument("--asset-base")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    if args.source_sha256:
        source_sha = args.source_sha256
    elif args.source_tarball:
        source_sha = sha256_file(args.source_tarball)
    else:
        raise GenerateError("pass --source-sha256 or --source-tarball")
    generate(
        version=args.version,
        tag_sha=args.tag_sha,
        digests=load_sidecars(args.sidecars),
        source_sha256=source_sha,
        out=args.out,
        asset_base=args.asset_base,
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except GenerateError as error:
        raise SystemExit(f"generate: {error}") from error
