#!/usr/bin/env python3
"""Run bounded integrity smoke checks against one packaged Blit release."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import socket
import stat
import subprocess
import sys
import tarfile
import tempfile
import time
import zipfile


class SmokeError(RuntimeError):
    """A release artifact failed a bounded smoke assertion."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def verify_checksum(archive: Path, checksum: Path) -> str:
    fields = checksum.read_text(encoding="utf-8-sig").strip().split()
    if len(fields) != 2 or not re.fullmatch(r"[0-9a-fA-F]{64}", fields[0]):
        raise SmokeError(f"invalid SHA-256 sidecar: {checksum}")
    if fields[1] != archive.name:
        raise SmokeError(
            f"checksum names {fields[1]!r}, expected archive {archive.name!r}"
        )
    expected = fields[0].lower()
    actual = sha256_file(archive)
    if actual != expected:
        raise SmokeError(f"SHA-256 mismatch for {archive}: {actual} != {expected}")
    return actual


def safe_parts(name: str) -> tuple[str, ...]:
    normalized = name.replace("\\", "/")
    path = PurePosixPath(normalized)
    parts = tuple(part for part in path.parts if part not in ("", "."))
    if (
        path.is_absolute()
        or not parts
        or ".." in parts
        or re.match(r"^[A-Za-z]:", parts[0])
    ):
        raise SmokeError(f"unsafe archive member: {name!r}")
    return parts


def extract_tar(archive: Path, destination: Path) -> None:
    seen: set[tuple[str, ...]] = set()
    with tarfile.open(archive, "r:gz") as bundle:
        for member in bundle.getmembers():
            parts = safe_parts(member.name)
            if parts in seen:
                raise SmokeError(f"duplicate archive member: {member.name!r}")
            seen.add(parts)
            target = destination.joinpath(*parts)
            if member.isdir():
                target.mkdir(parents=True, exist_ok=True)
                continue
            if not member.isfile():
                raise SmokeError(f"unsupported archive member type: {member.name!r}")
            target.parent.mkdir(parents=True, exist_ok=True)
            source = bundle.extractfile(member)
            if source is None:
                raise SmokeError(f"cannot read archive member: {member.name!r}")
            with source, target.open("wb") as output:
                shutil.copyfileobj(source, output)
            target.chmod(member.mode & 0o777)


def extract_zip(archive: Path, destination: Path) -> None:
    seen: set[tuple[str, ...]] = set()
    with zipfile.ZipFile(archive) as bundle:
        for member in bundle.infolist():
            parts = safe_parts(member.filename)
            if parts in seen:
                raise SmokeError(f"duplicate archive member: {member.filename!r}")
            seen.add(parts)
            unix_mode = member.external_attr >> 16
            if stat.S_ISLNK(unix_mode):
                raise SmokeError(f"archive symlink is not allowed: {member.filename!r}")
            target = destination.joinpath(*parts)
            if member.is_dir():
                target.mkdir(parents=True, exist_ok=True)
                continue
            target.parent.mkdir(parents=True, exist_ok=True)
            with bundle.open(member) as source, target.open("wb") as output:
                shutil.copyfileobj(source, output)
            permissions = unix_mode & 0o777
            if permissions:
                target.chmod(permissions)


def extract_archive(archive: Path, destination: Path) -> Path:
    if archive.name.endswith(".tar.gz"):
        package_name = archive.name[: -len(".tar.gz")]
        extract_tar(archive, destination)
    elif archive.suffix == ".zip":
        package_name = archive.stem
        extract_zip(archive, destination)
    else:
        raise SmokeError(f"unsupported release archive: {archive}")
    package_root = destination / package_name
    if not package_root.is_dir():
        raise SmokeError(f"archive is missing package root {package_name!r}")
    return package_root


def run_command(
    command: list[str], *, timeout: float = 30.0, check: bool = True
) -> subprocess.CompletedProcess[str]:
    try:
        completed = subprocess.run(
            command,
            stdin=subprocess.DEVNULL,
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired as error:
        raise SmokeError(f"command timed out after {timeout}s: {command!r}") from error
    if check and completed.returncode != 0:
        raise SmokeError(
            f"command failed ({completed.returncode}): {command!r}\n"
            f"stdout:\n{completed.stdout}\nstderr:\n{completed.stderr}"
        )
    return completed


def assert_cli_surface(client: Path, daemon: Path, expected_commit: str) -> None:
    short_commit = expected_commit[:12]
    client_version = run_command([str(client), "--version"]).stdout.strip()
    daemon_version = run_command([str(daemon), "--version"]).stdout.strip()
    client_prefix = "blit "
    daemon_prefix = "blit-daemon "
    if not client_version.startswith(client_prefix):
        raise SmokeError(f"unexpected client version output: {client_version!r}")
    if not daemon_version.startswith(daemon_prefix):
        raise SmokeError(f"unexpected daemon version output: {daemon_version!r}")
    client_identity = client_version[len(client_prefix) :]
    daemon_identity = daemon_version[len(daemon_prefix) :]
    if client_identity != daemon_identity:
        raise SmokeError(
            f"client/daemon build identities differ: {client_identity!r} != "
            f"{daemon_identity!r}"
        )
    if not client_identity.endswith(f"+{short_commit}"):
        raise SmokeError(
            f"build identity {client_identity!r} does not name clean commit {short_commit}"
        )
    for executable in (client, daemon):
        help_text = run_command([str(executable), "--help"]).stdout
        if "Usage:" not in help_text:
            raise SmokeError(f"{executable.name} --help did not render a usage block")


def assert_exact_bytes(path: Path, expected: bytes, label: str) -> None:
    if not path.is_file():
        raise SmokeError(f"{label} is missing: {path}")
    actual = path.read_bytes()
    if actual != expected:
        raise SmokeError(f"{label} content differs: {actual!r} != {expected!r}")


def free_loopback_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.bind(("127.0.0.1", 0))
        return int(listener.getsockname()[1])


def strip_windows_extended_prefix(path: str) -> str:
    if path.startswith("\\\\?\\UNC\\"):
        return "\\\\" + path[len("\\\\?\\UNC\\") :]
    if path.startswith("\\\\?\\"):
        return path[len("\\\\?\\") :]
    return path


def comparable_filesystem_path(path: os.PathLike[str] | str) -> str:
    resolved = os.path.realpath(path)
    if os.name == "nt":
        resolved = strip_windows_extended_prefix(resolved)
    return os.path.normcase(os.path.normpath(resolved))


def daemon_diagnostics(stdout_path: Path, stderr_path: Path) -> str:
    def tail(path: Path) -> str:
        if not path.exists():
            return ""
        with path.open("rb") as stream:
            stream.seek(0, os.SEEK_END)
            size = stream.tell()
            stream.seek(max(0, size - 256 * 1024))
            return stream.read().decode("utf-8", errors="replace")

    return f"daemon stdout:\n{tail(stdout_path)}\ndaemon stderr:\n{tail(stderr_path)}"


def stop_daemon(process: subprocess.Popen[bytes]) -> None:
    if process.poll() is not None:
        process.wait(timeout=1)
        return
    process.terminate()
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=10)
        raise SmokeError("daemon required a forced kill during teardown")


def wait_for_owned_daemon(
    client: Path,
    process: subprocess.Popen[bytes],
    port: int,
    remote_root: Path,
    timeout: float = 20.0,
) -> None:
    deadline = time.monotonic() + timeout
    expected_root = comparable_filesystem_path(remote_root)
    last_error = "daemon did not answer"
    while time.monotonic() < deadline:
        status = process.poll()
        if status is not None:
            raise SmokeError(f"daemon exited during startup with status {status}")
        try:
            result = run_command(
                [str(client), "list-modules", f"127.0.0.1:{port}", "--json"],
                timeout=2,
                check=False,
            )
        except SmokeError as error:
            last_error = str(error)
            time.sleep(0.1)
            continue
        if result.returncode == 0:
            try:
                modules = json.loads(result.stdout)
                if not isinstance(modules, list):
                    raise TypeError("module response is not a JSON array")
                owned = any(
                    isinstance(module, dict)
                    and module.get("name") == "test"
                    and comparable_filesystem_path(module.get("path", "")) == expected_root
                    for module in modules
                )
                if owned:
                    return
                last_error = f"unexpected module list: {result.stdout}"
            except (TypeError, json.JSONDecodeError) as error:
                last_error = f"invalid module JSON: {error}: {result.stdout}"
        else:
            last_error = result.stderr.strip() or result.stdout.strip()
        time.sleep(0.1)
    raise SmokeError(f"daemon readiness timed out: {last_error}")


def run_transfer_smoke(client: Path, daemon: Path, workspace: Path) -> None:
    local_payload = b"blit release local smoke\n"
    local_source = workspace / "local-source.txt"
    local_destination = workspace / "local-destination.txt"
    local_source.write_bytes(local_payload)
    run_command(
        [str(client), "copy", str(local_source), str(local_destination), "--yes"]
    )
    assert_exact_bytes(local_destination, local_payload, "local copy")

    remote_root = workspace / "remote-root"
    remote_root.mkdir()
    remote_payload = b"blit release loopback remote smoke\n"
    remote_source = workspace / "remote-source.txt"
    remote_source.write_bytes(remote_payload)
    config = workspace / "daemon.toml"
    stdout_path = workspace / "daemon.stdout.log"
    stderr_path = workspace / "daemon.stderr.log"
    port = free_loopback_port()
    config.write_text(
        "[daemon]\n"
        'bind = "127.0.0.1"\n'
        f"port = {port}\n"
        "no_mdns = true\n\n"
        "[[module]]\n"
        'name = "test"\n'
        f"path = {json.dumps(str(remote_root), ensure_ascii=False)}\n"
        "read_only = false\n",
        encoding="utf-8",
    )

    failure: BaseException | None = None
    with stdout_path.open("wb") as daemon_stdout, stderr_path.open("wb") as daemon_stderr:
        process = subprocess.Popen(
            [
                str(daemon),
                "--config",
                str(config),
                "--bind",
                "127.0.0.1",
                "--port",
                str(port),
                "--no-mdns",
            ],
            stdin=subprocess.DEVNULL,
            stdout=daemon_stdout,
            stderr=daemon_stderr,
        )
        try:
            wait_for_owned_daemon(client, process, port, remote_root)
            endpoint = f"127.0.0.1:{port}:/test/"
            run_command([str(client), "copy", str(remote_source), endpoint, "--yes"])
            assert_exact_bytes(
                remote_root / remote_source.name, remote_payload, "remote copy"
            )
        except BaseException as error:
            failure = error
        try:
            stop_daemon(process)
        except BaseException as teardown_error:
            if failure is None:
                failure = teardown_error
    if failure is not None:
        raise SmokeError(f"{failure}\n{daemon_diagnostics(stdout_path, stderr_path)}")


def smoke(archive: Path, checksum: Path, expected_commit: str) -> str:
    if not re.fullmatch(r"[0-9a-fA-F]{40}", expected_commit):
        raise SmokeError("--expected-commit must be a full 40-character Git commit")
    expected_commit = expected_commit.lower()
    archive = archive.resolve(strict=True)
    checksum = checksum.resolve(strict=True)
    digest = verify_checksum(archive, checksum)
    with tempfile.TemporaryDirectory(prefix="blit-release-smoke-") as temp:
        workspace = Path(temp)
        package_root = extract_archive(archive, workspace / "install")
        build_file = package_root / "BUILD.txt"
        if build_file.read_text(encoding="utf-8-sig").strip().lower() != expected_commit:
            raise SmokeError("BUILD.txt does not match the workflow commit")
        executable_suffix = ".exe" if os.name == "nt" else ""
        client = package_root / f"blit{executable_suffix}"
        daemon = package_root / f"blit-daemon{executable_suffix}"
        for executable in (client, daemon):
            if not executable.is_file():
                raise SmokeError(f"required executable is missing: {executable}")
        assert_cli_surface(client, daemon, expected_commit)
        run_transfer_smoke(client, daemon, workspace)
    return digest


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--checksum", type=Path, required=True)
    parser.add_argument("--expected-commit", required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    try:
        digest = smoke(args.archive, args.checksum, args.expected_commit)
    except (OSError, SmokeError, tarfile.TarError, zipfile.BadZipFile) as error:
        print(f"release-smoke: FAIL: {error}", file=sys.stderr)
        return 1
    print(f"release-smoke: OK sha256={digest} commit={args.expected_commit.lower()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
