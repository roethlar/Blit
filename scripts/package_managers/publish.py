#!/usr/bin/env python3
"""Generate package-manager stubs for a tag and update configured channels.

Missing secrets skip that channel. A failed channel does not touch the
GitHub Release. Signing secrets are never read here.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import tempfile
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path

import generate

DEFAULT_RELEASE_REPO = "roethlar/Blit"
DEFAULT_TAP_REPO = "roethlar/homebrew-blit"
DEFAULT_SCOOP_REPO = "roethlar/scoop-blit"

# Third-party PRs go through owner forks when the matching token is set.
DEFAULT_WINGET_FORK = "roethlar/winget-pkgs"
DEFAULT_HOMEBREW_CORE_FORK = "roethlar/homebrew-core"

CHANNEL_SECRETS = (
    ("homebrew-tap", "HOMEBREW_TAP_TOKEN"),
    ("scoop", "SCOOP_BUCKET_TOKEN"),
    ("aur", "AUR_SSH_KEY"),
    ("winget", "WINGET_PKGS_TOKEN"),
    ("homebrew-core", "HOMEBREW_CORE_TOKEN"),
)


class PublishError(RuntimeError):
    """A channel or fetch step failed."""


@dataclass(frozen=True)
class ChannelOutcome:
    name: str
    status: str
    detail: str


def normalize_tag(tag: str) -> tuple[str, str]:
    raw = tag.strip()
    if not raw:
        raise PublishError("tag is empty")
    version = raw[1:] if raw.startswith("v") else raw
    try:
        generate.validate_version(version)
    except generate.GenerateError as exc:
        raise PublishError(str(exc)) from exc
    return f"v{version}", version


def read_secret(name: str, environ: dict[str, str] | None = None) -> str | None:
    value = (environ if environ is not None else os.environ).get(name, "")
    stripped = value.strip()
    return stripped or None


def env_or(name: str, default: str, environ: dict[str, str] | None = None) -> str:
    value = read_secret(name, environ)
    return value if value is not None else default


def tag_sha_from_git(repo: Path, tag: str) -> str:
    output = subprocess.check_output(
        ["git", "rev-parse", "--short=12", tag],
        cwd=repo,
        text=True,
    ).strip()
    return generate.validate_tag_sha(output)


def download(url: str, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    try:
        with urllib.request.urlopen(url) as response:
            dest.write_bytes(response.read())
    except urllib.error.URLError as exc:
        raise PublishError(f"download failed for {url}: {exc}") from exc
    if dest.stat().st_size == 0:
        raise PublishError(f"downloaded empty file: {url}")


def release_asset_url(release_repo: str, tag: str, filename: str) -> str:
    return f"https://github.com/{release_repo}/releases/download/{tag}/{filename}"


def fetch_release_inputs(
    *,
    tag: str,
    version: str,
    dest: Path,
    release_repo: str = DEFAULT_RELEASE_REPO,
) -> tuple[Path, str]:
    sidecars = dest / "sidecars"
    for archive in (
        generate.LINUX_ARCHIVE,
        generate.MAC_ARCHIVE,
        generate.WINDOWS_ARCHIVE,
    ):
        name = f"{archive}.sha256"
        download(release_asset_url(release_repo, tag, name), sidecars / name)
    source = dest / f"Blit-{version}.tar.gz"
    download(generate.SOURCE_TARBALL_URL.format(version=version), source)
    return sidecars, generate.sha256_file(source)


def git(*args: str, cwd: Path, env: dict[str, str] | None = None) -> str:
    merged = os.environ.copy()
    if env:
        merged.update(env)
    result = subprocess.run(
        ["git", *args],
        cwd=cwd,
        env=merged,
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        raise PublishError(
            f"git {' '.join(args)} failed: {result.stderr.strip() or result.stdout.strip()}"
        )
    return result.stdout


def commit_if_changed(repo: Path, message: str) -> bool:
    git("add", "-A", cwd=repo)
    status = git("status", "--porcelain", cwd=repo)
    if not status.strip():
        return False
    # CI runners have no global git identity; commit as the Actions bot
    # via one-shot config so nothing persists in the clone.
    git(
        "-c",
        "user.name=github-actions[bot]",
        "-c",
        "user.email=41898282+github-actions[bot]@users.noreply.github.com",
        "commit",
        "-m",
        message,
        cwd=repo,
    )
    return True


def clone_https(repo: str, dest: Path, token: str) -> None:
    url = f"https://x-access-token:{token}@github.com/{repo}.git"
    result = subprocess.run(
        ["git", "clone", "--depth", "1", url, str(dest)],
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        raise PublishError(
            f"clone {repo} failed: {result.stderr.strip() or result.stdout.strip()}"
        )


def publish_homebrew_tap(stub_dir: Path, token: str, repo: str) -> str:
    with tempfile.TemporaryDirectory() as temp:
        dest = Path(temp) / "tap"
        clone_https(repo, dest, token)
        formula = dest / "Formula"
        formula.mkdir(exist_ok=True)
        shutil.copy2(stub_dir / "homebrew" / "blit-bin.rb", formula / "blit-bin.rb")
        if commit_if_changed(dest, f"blit-bin {read_version(stub_dir)}"):
            git("push", "origin", "HEAD", cwd=dest)
            return f"pushed {repo}"
        return f"{repo} already current"


def publish_scoop(stub_dir: Path, token: str, repo: str) -> str:
    with tempfile.TemporaryDirectory() as temp:
        dest = Path(temp) / "bucket"
        clone_https(repo, dest, token)
        bucket = dest / "bucket"
        bucket.mkdir(exist_ok=True)
        shutil.copy2(stub_dir / "scoop" / "blit.json", bucket / "blit.json")
        if commit_if_changed(dest, f"blit {read_version(stub_dir)}"):
            git("push", "origin", "HEAD", cwd=dest)
            return f"pushed {repo}"
        return f"{repo} already current"


def publish_aur(stub_dir: Path, ssh_key: str) -> str:
    with tempfile.TemporaryDirectory() as temp:
        key_path = Path(temp) / "aur_key"
        key_path.write_text(ssh_key + ("" if ssh_key.endswith("\n") else "\n"), encoding="utf-8")
        key_path.chmod(0o600)
        env = {
            "GIT_SSH_COMMAND": f"ssh -i {key_path} -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new"
        }
        notes = []
        for package in ("blit-bin", "blit"):
            dest = Path(temp) / package
            result = subprocess.run(
                [
                    "git",
                    "clone",
                    "--depth",
                    "1",
                    f"ssh://aur@aur.archlinux.org/{package}.git",
                    str(dest),
                ],
                env={**os.environ, **env},
                text=True,
                capture_output=True,
                check=False,
            )
            if result.returncode != 0:
                raise PublishError(
                    f"clone AUR {package} failed: {result.stderr.strip() or result.stdout.strip()}"
                )
            for name in ("PKGBUILD", ".SRCINFO"):
                shutil.copy2(stub_dir / "aur" / package / name, dest / name)
            if commit_if_changed(dest, f"{package} {read_version(stub_dir)}"):
                git("push", "origin", "HEAD", cwd=dest, env=env)
                notes.append(f"pushed {package}")
            else:
                notes.append(f"{package} already current")
        return "; ".join(notes)


def publish_copied_tree(
    *,
    stub_files: list[tuple[Path, Path]],
    token: str,
    repo: str,
    branch: str,
    message: str,
) -> str:
    """Clone a fork, copy files, push a branch. Caller opens the PR."""
    with tempfile.TemporaryDirectory() as temp:
        dest = Path(temp) / "fork"
        clone_https(repo, dest, token)
        # A re-dispatch must extend the fork's existing PR branch: the
        # shallow clone only has the default branch, and a fresh branch
        # from it is rejected as non-fast-forward once the remote branch
        # exists. Fetch the branch tip if it exists and build on it.
        try:
            git("fetch", "--depth", "1", "origin", branch, cwd=dest)
            git("checkout", "-B", branch, "FETCH_HEAD", cwd=dest)
        except PublishError:
            git("checkout", "-B", branch, cwd=dest)
        for source, relative in stub_files:
            target = dest / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)
        if commit_if_changed(dest, message):
            git("push", "-u", "origin", branch, cwd=dest)
            return f"pushed {repo} {branch}"
        return f"{repo} already current"


def publish_winget(stub_dir: Path, token: str, fork: str) -> str:
    version = read_version(stub_dir)
    winget = stub_dir / "winget" / "Roethlar.Blit"
    prefix = Path("manifests") / "r" / "Roethlar" / "Blit" / version
    files = [
        (winget / name, prefix / name)
        for name in (
            "Roethlar.Blit.yaml",
            "Roethlar.Blit.locale.en-US.yaml",
            "Roethlar.Blit.installer.yaml",
        )
    ]
    return publish_copied_tree(
        stub_files=files,
        token=token,
        repo=fork,
        branch=f"blit-{version}",
        message=f"Roethlar.Blit version {version}",
    )


def publish_homebrew_core(stub_dir: Path, token: str, fork: str) -> str:
    version = read_version(stub_dir)
    return publish_copied_tree(
        stub_files=[(stub_dir / "homebrew" / "blit.rb", Path("Formula") / "blit.rb")],
        token=token,
        repo=fork,
        branch=f"blit-{version}",
        message=f"blit {version}",
    )


def read_version(stub_dir: Path) -> str:
    path = stub_dir / "scoop" / "blit.json"
    try:
        version = json.loads(path.read_text(encoding="utf-8"))["version"]
    except (OSError, KeyError, json.JSONDecodeError) as exc:
        raise PublishError(f"cannot read version from {path}: {exc}") from exc
    return generate.validate_version(str(version))


class LivePublisher:
    def homebrew_tap(self, stub_dir: Path, token: str) -> str:
        return publish_homebrew_tap(
            stub_dir, token, env_or("HOMEBREW_TAP_REPO", DEFAULT_TAP_REPO)
        )

    def scoop(self, stub_dir: Path, token: str) -> str:
        return publish_scoop(
            stub_dir, token, env_or("SCOOP_BUCKET_REPO", DEFAULT_SCOOP_REPO)
        )

    def aur(self, stub_dir: Path, token: str) -> str:
        return publish_aur(stub_dir, token)

    def winget(self, stub_dir: Path, token: str) -> str:
        return publish_winget(
            stub_dir, token, env_or("WINGET_PKGS_FORK", DEFAULT_WINGET_FORK)
        )

    def homebrew_core(self, stub_dir: Path, token: str) -> str:
        return publish_homebrew_core(
            stub_dir,
            token,
            env_or("HOMEBREW_CORE_FORK", DEFAULT_HOMEBREW_CORE_FORK),
        )


def run_channels(
    stub_dir: Path,
    environ: dict[str, str],
    publisher: LivePublisher | None = None,
) -> list[ChannelOutcome]:
    actor = publisher or LivePublisher()
    actions = {
        "homebrew-tap": actor.homebrew_tap,
        "scoop": actor.scoop,
        "aur": actor.aur,
        "winget": actor.winget,
        "homebrew-core": actor.homebrew_core,
    }
    outcomes: list[ChannelOutcome] = []
    for name, secret_name in CHANNEL_SECRETS:
        secret = read_secret(secret_name, environ)
        if secret is None:
            outcomes.append(
                ChannelOutcome(name, "skipped", f"{secret_name} is empty")
            )
            continue
        try:
            detail = actions[name](stub_dir, secret)
        except Exception as exc:  # noqa: BLE001 — channel isolation
            outcomes.append(ChannelOutcome(name, "failed", str(exc)))
            continue
        outcomes.append(ChannelOutcome(name, "updated", detail))
    return outcomes


def prepare_stubs(
    *,
    tag: str,
    out: Path,
    repo_root: Path,
    sidecars: Path | None,
    source_tarball: Path | None,
    tag_sha: str | None,
    release_repo: str,
) -> Path:
    tag, version = normalize_tag(tag)
    sha = tag_sha or tag_sha_from_git(repo_root, tag)
    if sidecars is None or source_tarball is None:
        fetched = out / "_fetched"
        fetched_sidecars, source_sha = fetch_release_inputs(
            tag=tag, version=version, dest=fetched, release_repo=release_repo
        )
        sidecars = sidecars or fetched_sidecars
        source_sha256 = (
            generate.sha256_file(source_tarball)
            if source_tarball is not None
            else source_sha
        )
    else:
        source_sha256 = generate.sha256_file(source_tarball)
    stub_dir = out / "stubs"
    generate.generate(
        version=version,
        tag_sha=sha,
        digests=generate.load_sidecars(sidecars),
        source_sha256=source_sha256,
        out=stub_dir,
    )
    return stub_dir


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--out", type=Path, default=Path("dist/package-managers"))
    parser.add_argument("--repo-root", type=Path, default=Path("."))
    parser.add_argument("--sidecars", type=Path)
    parser.add_argument("--source-tarball", type=Path)
    parser.add_argument("--tag-sha")
    parser.add_argument("--release-repo", default=DEFAULT_RELEASE_REPO)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    stub_dir = prepare_stubs(
        tag=args.tag,
        out=args.out,
        repo_root=args.repo_root,
        sidecars=args.sidecars,
        source_tarball=args.source_tarball,
        tag_sha=args.tag_sha,
        release_repo=args.release_repo,
    )
    outcomes = run_channels(stub_dir, dict(os.environ))
    failed = False
    for outcome in outcomes:
        print(f"{outcome.name}: {outcome.status} — {outcome.detail}")
        if outcome.status == "failed":
            failed = True
    return 1 if failed else 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (PublishError, generate.GenerateError) as error:
        raise SystemExit(f"publish: {error}") from error
