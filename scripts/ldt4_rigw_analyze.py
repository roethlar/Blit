#!/usr/bin/env python3
"""Validate and summarize ldt-4 adaptive rig-W evidence.

The analyzer validates evidence, not a preferred worker count.  It accepts the
registered q <-> netwatch-01 matrix only when every arm has a complete,
role-correct trace and a content manifest identical to its canonical source.
Timing arithmetic is endpoint-local: clocks from different hosts are never
subtracted from one another.
"""

from __future__ import annotations

import argparse
import base64
import binascii
import csv
import hashlib
import io
import json
import math
import os
import re
import stat
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass
from decimal import Decimal, InvalidOperation
from pathlib import Path, PurePosixPath
from statistics import median
from typing import Any, Iterable, Optional, Sequence


ARTIFACT_SHA = "406a7e5854593b7a7a151f9b6d9cdf1be8a9cd77"
BUILD_ID = ARTIFACT_SHA[:12]
CARGO_LOCK_SHA256 = "ec1ce3fbe4208c7f7993e27ed997555b60bfef46c4bcec323b90bf9e6b4daa52"
Q_ARTIFACT_REPO = "/Users/michael/Dev/blit_v2_artifact_406a7e5"
Q_IP = "10.1.10.54"
Q_NIC = "en8"
Q_MAC = "00:01:d2:19:04:a3"
Q_LOCAL_HOSTNAME = "Q"
Q_COMPUTER_NAME = "Q"
WINDOWS_IP = "10.1.10.173"
WINDOWS_NIC = "Ethernet"
WINDOWS_MAC = "34:5a:60:3e:78:8b"
MIN_FREE_BYTES = 33_000_000_000
MIN_POWERSHELL_VERSION = (7, 4)
DIRECTIONS = ("q_to_windows", "windows_to_q")
FIXTURES = ("large", "small", "mixed")
CELL_ORDER = (
    "q_to_windows_large",
    "windows_to_q_large",
    "windows_to_q_small",
    "q_to_windows_small",
    "q_to_windows_mixed",
    "windows_to_q_mixed",
)
INITIATORS = ("source_init", "destination_init")
PAIRS = tuple(range(1, 9))
FIRST_ROLE = (
    "source_init",
    "destination_init",
    "destination_init",
    "source_init",
    "source_init",
    "destination_init",
    "destination_init",
    "source_init",
)
EXPECTED_FIXTURES: dict[str, tuple[int, int]] = {
    "large": (1, 1_073_741_824),
    "small": (10_000, 40_960_000),
    "mixed": (5_001, 547_110_912),
    "sustained": (5, 5_368_709_120),
    "horizon": (40, 42_949_672_960),
}
SUSTAINED_FIXTURES = ("sustained",)
SUSTAINED_CELL_ORDER = ("q_to_windows_sustained", "windows_to_q_sustained")
SUSTAINED_FIRST_ROLE = ("source_init", "destination_init")
HORIZON_FIXTURES = ("horizon",)
HORIZON_CELL_ORDER = ("q_to_windows_horizon", "windows_to_q_horizon")
HORIZON_FIRST_ROLE = ("source_init", "destination_init")
PARENT_SESSION = "ldt4-20260721T224319Z-96a4e3b03caf"
PARENT_EVIDENCE = "docs/bench/ldt4-rigw-2026-07-21"
PARENT_INVENTORY_SHA256 = "713cb4624e6f64a3863b67101fb9a3f3df288306d3e6f418c19501428711990b"
PREDECESSOR_SESSION = "ldt4-20260722T001611Z-04e80082e12c"
PREDECESSOR_EVIDENCE = "docs/bench/ldt4-rigw-sustained-2026-07-22"
PREDECESSOR_INVENTORY_SHA256 = (
    "17348aaa261b936e04c104553d7b5c4bbcf008968306a29c4dea922535110eef"
)
SUSTAINED_DESTINATION_BYTES = 10_737_418_240
HORIZON_DESTINATION_BYTES = 85_899_345_920
Q_PAYLOAD_VOLUME = "/Volumes/Apps"
Q_PAYLOAD_VOLUME_UUID = "33BAD653-9FA1-4236-966F-5BC4B221B34F"
EXPECTED_FLOOR = 4
EXPECTED_CEILING = 32
FLOOR_CHUNK_BYTES = 16 * 1024 * 1024
FLOOR_PREFETCH = 4
CEILING_CHUNK_BYTES = 64 * 1024 * 1024
CEILING_PREFETCH = 32
CEILING_TCP_BUFFER_BYTES = 8 * 1024 * 1024
STEP_UP_THRESHOLD = 0.05
STEP_DOWN_THRESHOLD = 0.30
RESIZE_COOLDOWN_TICKS = 4
RESIZE_SUSTAIN_TICKS = 2
DIAL_TUNER_TICK_NS = 500_000_000
DURATION_ROUNDING_ALLOWANCE_MS = Decimal("1")
PERFORMANCE_RATIO_LIMIT = Decimal("1.10")
TRACE_PREFIX = "[session-phase] "
SESSION_ID_RE = re.compile(r"^[0-9a-f]{16}$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
UINT_RE = re.compile(r"^(0|[1-9][0-9]*)$")
DRIVE_ABSOLUTE_RE = re.compile(r"^[A-Za-z]:[\\/]")
FULL_SHA_RE = re.compile(r"^[0-9a-f]{40}$")
SAFE_SESSION_ID_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]*$")
NONNEG_DECIMAL_RE = re.compile(r"^(?:0|[1-9][0-9]*)(?:\.[0-9]+)?$")
POWERSHELL_VERSION_PATTERN = (
    r"(?:0|[1-9][0-9]*)\."
    r"(?:0|[1-9][0-9]*)\."
    r"(?:0|[1-9][0-9]*)(?:\.(?:0|[1-9][0-9]*))?"
)

RUN_FIELDS = (
    "cell",
    "direction",
    "fixture",
    "pair",
    "initiator",
    "run_id",
    "session_id",
    "duration_ms",
    "files",
    "bytes",
    "source_path",
    "active_destination_path",
    "archive_path",
    "source_manifest",
    "landed_manifest",
    "source_trace",
    "destination_trace",
    "exit",
    "valid",
)
FIXTURE_INDEX_FIELDS = ("direction", "fixture", "source_manifest")
STAGING_FIELDS = (
    "endpoint",
    "role",
    "artifact_sha",
    "build_id",
    "sha256",
    "staged_path",
    "runtime_path",
)
RUNTIME_GATE_FIELDS = (
    "sequence",
    "cell",
    "pair",
    "q_free_bytes",
    "windows_free_bytes",
    "q_quiet",
    "windows_quiet",
)
INPUT_INVENTORY_FIELDS = ("path", "size", "sha256")
REGISTERED_BINARY_PATHS = {
    ("q", "client"): (
        "/Users/michael/Dev/blit_v2_artifact_406a7e5/target/release/blit",
        "/Users/michael/Dev/blit_v2_artifact_406a7e5/target/release/blit",
    ),
    ("q", "daemon"): (
        "/Users/michael/Dev/blit_v2_artifact_406a7e5/target/release/blit-daemon",
        "/Users/michael/Dev/blit_v2_artifact_406a7e5/target/release/blit-daemon",
    ),
    ("windows", "client"): (
        "D:/blit-test/bins/406a7e5/blit.exe",
        "D:/blit-test/bins/406a7e5/blit.exe",
    ),
    ("windows", "daemon"): (
        "D:/blit-test/bins/406a7e5/blit-daemon.exe",
        "D:/blit-test/bins/active/blit-daemon.exe",
    ),
}
COMMON_EVENT_FIELDS = {
    "schema",
    "run_id",
    "session_id",
    "producer_seq",
    "unix_ns",
    "elapsed_ns",
    "endpoint_role",
    "initiator_role",
    "event",
}
SAMPLE_FIELDS = {
    "reason",
    "epoch",
    "live_streams",
    "sample_bytes",
    "sample_blocked_ns",
    "sample_elapsed_ns",
    "sample_streams",
    "sample_valid",
    "blocked_ratio",
    "chunk_bytes",
    "prefetch_count",
    "tcp_buffer_bytes",
    "receiver_ceiling",
    "peak_streams",
}
PENDING_FIELDS = {
    "action",
    "reason",
    "epoch",
    "target_streams",
    "live_streams",
    "receiver_ceiling",
    "peak_streams",
}
SETTLEMENT_FIELDS = PENDING_FIELDS | {"accepted"}
SAMPLE_REASONS = {
    "idle",
    "rebaseline",
    "hysteresis",
    "cheap-up",
    "cheap-down",
    "sustain",
    "cooldown",
    "bound",
    "add",
    "remove",
}
SOCKET_ACQUISITION_EVENTS = {
    "socket_dial_begin",
    "socket_dial_end",
    "socket_accept_begin",
    "socket_accept_end",
}
SOCKET_MEMBERSHIP_EVENTS = SOCKET_ACQUISITION_EVENTS | {"socket_trace_attached"}
SOCKET_EVENTS = SOCKET_MEMBERSHIP_EVENTS | {"socket_write_begin"}
SOURCE_EVENT_NAMES = SOCKET_EVENTS | {
    "first_payload_queued",
    "first_socket_write",
    "need_batch_received",
    "need_complete_received",
    "resize_ack_received",
    "manifest_complete_send_begin",
    "manifest_complete_sent",
    "planner_begin",
    "planner_end",
    "summary_received",
    "dial_sample",
    "dial_pending",
    "dial_settlement",
    "resize_proposed",
    "resize_send_begin",
    "resize_sent",
    "source_settled",
    "membership_sealed",
    "data_plane_complete",
    "data_plane_aborted",
}
DESTINATION_EVENT_NAMES = SOCKET_MEMBERSHIP_EVENTS | {
    "first_payload_received",
    "manifest_complete_received",
    "need_batch_send_begin",
    "need_batch_sent",
    "need_complete_sent",
    "resize_received",
    "resize_arm_queue_begin",
    "resize_arm_ready",
    "destination_prepared",
    "resize_ack_send_begin",
    "resize_ack_sent",
    "receive_task_stopped",
    "summary_send_begin",
    "summary_sent",
    "data_plane_complete",
    "data_plane_aborted",
}
EVENT_NAMES_BY_ROLE = {
    "SOURCE": SOURCE_EVENT_NAMES,
    "DESTINATION": DESTINATION_EVENT_NAMES,
}


class AnalysisError(RuntimeError):
    """The evidence is incomplete, inconsistent, or unsafe to grade."""


class DuplicateJsonKeyError(ValueError):
    def __init__(self, key: str) -> None:
        super().__init__(key)
        self.key = key


def _reject_duplicate_json_keys(pairs: Sequence[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise DuplicateJsonKeyError(key)
        result[key] = value
    return result


@dataclass(frozen=True)
class ManifestEntry:
    encoded_path: str
    path: str
    size: int
    sha256: str


@dataclass(frozen=True)
class Manifest:
    relative_path: str
    file_sha256: str
    entries: tuple[ManifestEntry, ...]

    @property
    def files(self) -> int:
        return len(self.entries)

    @property
    def bytes(self) -> int:
        return sum(entry.size for entry in self.entries)


@dataclass(frozen=True)
class RunRow:
    csv_line: int
    schedule_index: int
    cell: str
    direction: str
    fixture: str
    pair: int
    initiator: str
    run_id: str
    session_id: str
    duration_ms: Decimal
    files: int
    bytes: int
    source_path: str
    active_destination_path: str
    archive_path: str
    source_manifest: str
    landed_manifest: str
    source_trace: str
    destination_trace: str


@dataclass(frozen=True)
class DialResult:
    floor: int
    peak: int
    final: int
    ceiling: int
    add_count: int
    remove_count: int
    reasons: Counter[str]
    samples: tuple[dict[str, Any], ...]
    operations: tuple[tuple[int, str, int], ...]


@dataclass(frozen=True)
class ArmResult:
    row: RunRow
    dial: DialResult
    source_complete_elapsed_ns: int
    destination_complete_elapsed_ns: int
    review_reasons: tuple[str, ...]


@dataclass(frozen=True)
class AnalysisResult:
    output_dir: Path
    status: str
    arm_count: int
    arm_review_count: int
    decision_review_count: int
    performance_review_count: int


def _cell(direction: str, fixture: str) -> str:
    return f"{direction}_{fixture}"


def _expected_cells() -> tuple[str, ...]:
    return CELL_ORDER


def _int(value: Any, field: str, context: str, *, minimum: int = 0) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < minimum:
        raise AnalysisError(f"{context}: {field} must be an integer >= {minimum}")
    return value


def _csv_int(value: str, field: str, line: int, *, minimum: int = 0) -> int:
    if not UINT_RE.fullmatch(value):
        raise AnalysisError(f"runs.csv line {line}: {field} is not a canonical unsigned integer")
    result = int(value)
    if result < minimum:
        raise AnalysisError(f"runs.csv line {line}: {field} must be >= {minimum}")
    return result


def _decimal(value: str, field: str, line: int) -> Decimal:
    try:
        result = Decimal(value)
    except InvalidOperation as exc:
        raise AnalysisError(f"runs.csv line {line}: {field} is not numeric") from exc
    if not result.is_finite() or result <= 0:
        raise AnalysisError(f"runs.csv line {line}: {field} must be finite and positive")
    return result


def _plain_directory(path: Path, label: str) -> None:
    try:
        mode = path.lstat().st_mode
    except FileNotFoundError as exc:
        raise AnalysisError(f"{label} does not exist: {path}") from exc
    if not stat.S_ISDIR(mode) or stat.S_ISLNK(mode):
        raise AnalysisError(f"{label} must be a plain directory: {path}")


def _plain_file(path: Path, label: str) -> None:
    try:
        mode = path.lstat().st_mode
    except FileNotFoundError as exc:
        raise AnalysisError(f"{label} does not exist: {path}") from exc
    if not stat.S_ISREG(mode) or stat.S_ISLNK(mode):
        raise AnalysisError(f"{label} must be a plain regular file: {path}")


def _evidence_file(root: Path, relative: str, label: str) -> Path:
    if not relative or "\\" in relative:
        raise AnalysisError(f"{label} must be a non-empty POSIX evidence path")
    pure = PurePosixPath(relative)
    if (
        pure.is_absolute()
        or str(pure) != relative
        or any(part in ("", ".", "..") for part in pure.parts)
    ):
        raise AnalysisError(f"{label} escapes the session directory: {relative!r}")
    current = root
    for part in pure.parts:
        current = current / part
        try:
            mode = current.lstat().st_mode
        except FileNotFoundError as exc:
            raise AnalysisError(f"{label} does not exist: {relative!r}") from exc
        if stat.S_ISLNK(mode):
            raise AnalysisError(f"{label} traverses a symbolic link: {relative!r}")
    _plain_file(current, label)
    return current


def _absolute_endpoint_path(
    value: str, field: str, line: int, source: str = "runs.csv"
) -> None:
    if not value or "\n" in value or "\r" in value:
        raise AnalysisError(f"{source} line {line}: {field} is blank or multiline")
    if not (value.startswith("/") or DRIVE_ABSOLUTE_RE.match(value)):
        raise AnalysisError(f"{source} line {line}: {field} must be an absolute endpoint path")


def _windows_path(value: str) -> bool:
    return DRIVE_ABSOLUTE_RE.match(value) is not None


def _registered_source_path(direction: str, fixture: str) -> str:
    if fixture == "horizon":
        if direction == "q_to_windows":
            return "/Volumes/Apps/blit-ldt4-f13/staging/fixtures/src_horizon"
        return "D:/blit-test/ldt4-f13/staging/fixtures/src_horizon"
    if direction == "q_to_windows":
        return f"/Users/michael/blit-ldt4-staging/fixtures/src_{fixture}"
    if fixture in {"small", "sustained"}:
        return f"D:/blit-test/ldt4-staging/fixtures/src_{fixture}"
    return f"D:/blit-test/rigw-module/src_{fixture}"


def _session_destination_root(direction: str, fixture: str) -> str:
    if fixture == "horizon":
        if direction == "q_to_windows":
            return "D:/blit-test/ldt4-f13/sessions"
        return "/Volumes/Apps/blit-ldt4-f13/sessions"
    if direction == "q_to_windows":
        return "D:/blit-test/ldt4-sessions"
    return "/Users/michael/blit-ldt4-sessions"


def _registered_trace_paths(
    direction: str, initiator: str, run_id: str
) -> tuple[str, str]:
    return {
        ("q_to_windows", "source_init"): (
            f"endpoint/q/{run_id}/client.err",
            f"endpoint/windows/{run_id}/daemon.err",
        ),
        ("q_to_windows", "destination_init"): (
            f"endpoint/q/{run_id}/daemon.err",
            f"endpoint/windows/{run_id}/client.err",
        ),
        ("windows_to_q", "source_init"): (
            f"endpoint/windows/{run_id}/client.err",
            f"endpoint/q/{run_id}/daemon.err",
        ),
        ("windows_to_q", "destination_init"): (
            f"endpoint/windows/{run_id}/daemon.err",
            f"endpoint/q/{run_id}/client.err",
        ),
    }[(direction, initiator)]


def _read_csv(path: Path, expected_fields: Sequence[str], label: str) -> list[dict[str, str]]:
    _plain_file(path, label)
    with path.open("r", encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle)
        if tuple(reader.fieldnames or ()) != tuple(expected_fields):
            raise AnalysisError(
                f"{label} header mismatch: expected {','.join(expected_fields)}"
            )
        rows = list(reader)
    for offset, row in enumerate(rows, start=2):
        if None in row or any(value is None for value in row.values()):
            raise AnalysisError(f"{label} line {offset}: malformed CSV row")
    return rows


def _read_single_lf_line(path: Path, label: str) -> str:
    _plain_file(path, label)
    payload = path.read_bytes()
    if b"\r" in payload or not payload.endswith(b"\n") or payload.count(b"\n") != 1:
        raise AnalysisError(f"{label} must contain exactly one LF-terminated line")
    try:
        return payload[:-1].decode("utf-8", errors="strict")
    except UnicodeDecodeError as exc:
        raise AnalysisError(f"{label} must be valid UTF-8") from exc


def _load_artifact_build(root: Path) -> None:
    expected = (
        f"artifact_sha={ARTIFACT_SHA} build_id={BUILD_ID} "
        f"cargo_lock_sha256={CARGO_LOCK_SHA256} q_artifact_repo={Q_ARTIFACT_REPO}"
    )
    if _read_single_lf_line(root / "artifact-build.txt", "artifact-build.txt") != expected:
        raise AnalysisError("artifact-build.txt is not the exact accepted build binding")


def _expected_schedule_rows() -> list[tuple[str, str, str, str, str]]:
    rows: list[tuple[str, str, str, str, str]] = []
    sequence = 0
    for cell in CELL_ORDER:
        direction = next(
            candidate for candidate in DIRECTIONS if cell.startswith(f"{candidate}_")
        )
        fixture = cell[len(direction) + 1 :]
        for pair, first in zip(PAIRS, FIRST_ROLE):
            second = next(role for role in INITIATORS if role != first)
            for initiator in (first, second):
                sequence += 1
                rows.append(
                    (f"{sequence:03d}", cell, direction, fixture, initiator)
                )
    return rows


def _expected_sustained_schedule_rows() -> list[tuple[str, str, str, str, str]]:
    rows: list[tuple[str, str, str, str, str]] = []
    sequence = 0
    for cell, first in zip(SUSTAINED_CELL_ORDER, SUSTAINED_FIRST_ROLE):
        direction = next(
            candidate for candidate in DIRECTIONS if cell.startswith(f"{candidate}_")
        )
        second = next(role for role in INITIATORS if role != first)
        for initiator in (first, second):
            sequence += 1
            rows.append((f"{sequence:03d}", cell, direction, "sustained", initiator))
    return rows


def _expected_horizon_schedule_rows() -> list[tuple[str, str, str, str, str]]:
    rows: list[tuple[str, str, str, str, str]] = []
    sequence = 0
    for cell, first in zip(HORIZON_CELL_ORDER, HORIZON_FIRST_ROLE):
        direction = next(
            candidate for candidate in DIRECTIONS if cell.startswith(f"{candidate}_")
        )
        second = next(role for role in INITIATORS if role != first)
        for initiator in (first, second):
            sequence += 1
            rows.append((f"{sequence:03d}", cell, direction, "horizon", initiator))
    return rows


def _schedule_rows(matrix: str) -> list[tuple[str, str, str, str, str]]:
    if matrix == "fixed":
        return _expected_schedule_rows()
    if matrix == "sustained":
        return _expected_sustained_schedule_rows()
    if matrix == "horizon":
        return _expected_horizon_schedule_rows()
    raise AnalysisError(f"unregistered matrix {matrix!r}")


def _matrix_fixtures(matrix: str) -> tuple[str, ...]:
    if matrix == "fixed":
        return FIXTURES
    if matrix == "sustained":
        return SUSTAINED_FIXTURES
    if matrix == "horizon":
        return HORIZON_FIXTURES
    raise AnalysisError(f"unregistered matrix {matrix!r}")


def _matrix_cells(matrix: str) -> tuple[str, ...]:
    if matrix == "fixed":
        return CELL_ORDER
    if matrix == "sustained":
        return SUSTAINED_CELL_ORDER
    if matrix == "horizon":
        return HORIZON_CELL_ORDER
    raise AnalysisError(f"unregistered matrix {matrix!r}")


def _matrix_first_roles(matrix: str) -> tuple[str, ...]:
    if matrix == "fixed":
        return FIRST_ROLE
    if matrix == "sustained":
        return SUSTAINED_FIRST_ROLE
    if matrix == "horizon":
        return HORIZON_FIRST_ROLE
    raise AnalysisError(f"unregistered matrix {matrix!r}")


def _load_schedule(root: Path, matrix: str) -> None:
    path = root / "schedule.csv"
    _plain_file(path, "schedule.csv")
    expected = "".join(",".join(row) + "\n" for row in _schedule_rows(matrix)).encode(
        "ascii"
    )
    if path.read_bytes() != expected:
        label = "96-arm" if matrix == "fixed" else matrix
        raise AnalysisError(f"schedule.csv is not the exact registered {label} schedule")


def _gate_decimal(value: str, field: str, maximum: Decimal) -> Decimal:
    if not NONNEG_DECIMAL_RE.fullmatch(value):
        raise AnalysisError(f"{field} is not a canonical non-negative number")
    parsed = Decimal(value)
    if parsed > maximum:
        raise AnalysisError(f"{field} exceeds {maximum}")
    return parsed


def _load_environment_gate(root: Path, phase: str) -> None:
    label = f"environment-{phase}.txt"
    line = _read_single_lf_line(root / label, label)
    pattern = re.compile(
        rf"^phase={re.escape(phase)} "
        rf"q_ip={re.escape(Q_IP)} q_nic={re.escape(Q_NIC)} q_mtu=9000 "
        rf"q_media=(?P<q_media>.+?) q_route={re.escape(Q_NIC)} q_route_mtu=9000 "
        rf"q_local_hostname={re.escape(Q_LOCAL_HOSTNAME)} "
        rf"q_computer_name={re.escape(Q_COMPUTER_NAME)} "
        rf"q_mac={re.escape(Q_MAC)} q_peer={re.escape(WINDOWS_MAC)} "
        rf"q_free=(?P<q_free>(?:0|[1-9][0-9]*)) "
        r"q_load1=(?P<q_load1>(?:0|[1-9][0-9]*)(?:\.[0-9]+)?) "
        r"q_spotlight_cpu=(?P<q_spotlight_cpu>(?:0|[1-9][0-9]*)(?:\.[0-9]+)?) "
        r"time_machine_auto=0 time_machine_running=0 "
        r"q_to_windows_mss=8948 windows_to_q_mss=8960 "
        rf"windows_powershell=(?P<windows_powershell>{POWERSHELL_VERSION_PATTERN}) "
        rf"windows=W\|(?P<windows_free>(?:0|[1-9][0-9]*))\|9000\|10 Gbps\|"
        rf"{re.escape(WINDOWS_NIC)}\|D:/blit-test/bins/active/blit-daemon\.exe\|"
        rf"00-01-D2-19-04-A3\|(?P<windows_topology_powershell>{POWERSHELL_VERSION_PATTERN}) "
        r"windows_cpu_avg=(?P<windows_cpu_avg>(?:0|[1-9][0-9]*)(?:\.[0-9]+)?)$"
    )
    match = pattern.fullmatch(line)
    if match is None or "10Gbase-T" not in match.group("q_media"):
        raise AnalysisError(f"{label} is not the exact registered endpoint gate shape")
    if int(match.group("q_free")) < MIN_FREE_BYTES:
        raise AnalysisError(f"{label}: q_free is below {MIN_FREE_BYTES}")
    if int(match.group("windows_free")) < MIN_FREE_BYTES:
        raise AnalysisError(f"{label}: Windows free bytes are below {MIN_FREE_BYTES}")
    powershell_version = tuple(
        int(component) for component in match.group("windows_powershell").split(".")
    )
    if powershell_version[:2] < MIN_POWERSHELL_VERSION:
        raise AnalysisError(
            f"{label}: PowerShell {match.group('windows_powershell')} is below 7.4"
        )
    if match.group("windows_topology_powershell") != match.group(
        "windows_powershell"
    ):
        raise AnalysisError(f"{label}: PowerShell evidence fields disagree")
    _gate_decimal(match.group("q_load1"), f"{label}: q_load1", Decimal("3.0"))
    _gate_decimal(
        match.group("q_spotlight_cpu"),
        f"{label}: q_spotlight_cpu",
        Decimal("10.0"),
    )
    _gate_decimal(
        match.group("windows_cpu_avg"),
        f"{label}: windows_cpu_avg",
        Decimal("20.0"),
    )


def _load_payload_volume_gate(root: Path, phase: str) -> None:
    label = f"payload-volume-{phase}.txt"
    line = _read_single_lf_line(root / label, label)
    pattern = re.compile(
        rf"^phase={re.escape(phase)} mount={re.escape(Q_PAYLOAD_VOLUME)} "
        rf"uuid={re.escape(Q_PAYLOAD_VOLUME_UUID)} "
        r"filesystem=case-sensitive_apfs protocol=PCI-Express "
        r"solid_state=true writable=true "
        r"backing=/dev/disk[0-9]+s[0-9]+ "
        r"free_bytes=(?P<free_bytes>(?:0|[1-9][0-9]*))$"
    )
    match = pattern.fullmatch(line)
    if match is None:
        raise AnalysisError(f"{label} is not the exact registered payload-volume gate")
    if int(match.group("free_bytes")) < MIN_FREE_BYTES:
        raise AnalysisError(f"{label}: free_bytes is below {MIN_FREE_BYTES}")


def _load_runtime_gates(root: Path, matrix: str) -> None:
    rows = _read_csv(root / "runtime-gates.csv", RUNTIME_GATE_FIELDS, "runtime-gates.csv")
    expected_schedule = _schedule_rows(matrix)
    expected = [
        (schedule[0], schedule[1], str((index // 2) % len(PAIRS) + 1 if matrix == "fixed" else 1))
        for index, schedule in enumerate(expected_schedule)
        if index % 2 == 0
    ]
    if len(rows) != len(expected):
        raise AnalysisError(
            f"runtime-gates.csv must contain exactly {len(expected)} pair-boundary rows"
        )
    q_quiet_re = re.compile(
        r"^q_load1=((?:0|[1-9][0-9]*)(?:\.[0-9]+)?);"
        r"q_spotlight_cpu=((?:0|[1-9][0-9]*)(?:\.[0-9]+)?);"
        r"time_machine_auto=0;time_machine_running=0$"
    )
    windows_quiet_re = re.compile(
        r"^windows_cpu_avg=((?:0|[1-9][0-9]*)(?:\.[0-9]+)?)$"
    )
    for line_number, (row, expected_key) in enumerate(zip(rows, expected), start=2):
        if (row["sequence"], row["cell"], row["pair"]) != expected_key:
            raise AnalysisError(
                f"runtime-gates.csv line {line_number}: boundary schedule mismatch"
            )
        for field in ("q_free_bytes", "windows_free_bytes"):
            value = row[field]
            if not UINT_RE.fullmatch(value) or int(value) < MIN_FREE_BYTES:
                raise AnalysisError(
                    f"runtime-gates.csv line {line_number}: {field} is below the registered minimum"
                )
        if matrix in {"sustained", "horizon"}:
            destination_bytes = (
                SUSTAINED_DESTINATION_BYTES
                if matrix == "sustained"
                else HORIZON_DESTINATION_BYTES
            )
            q_required = MIN_FREE_BYTES + destination_bytes
            windows_required = MIN_FREE_BYTES + (
                destination_bytes
                if row["cell"] == f"q_to_windows_{_matrix_fixtures(matrix)[0]}"
                else 0
            )
            if int(row["q_free_bytes"]) < q_required:
                raise AnalysisError(
                    f"runtime-gates.csv line {line_number}: q_free_bytes cannot retain the remaining {matrix} destinations"
                )
            if int(row["windows_free_bytes"]) < windows_required:
                raise AnalysisError(
                    f"runtime-gates.csv line {line_number}: windows_free_bytes cannot retain the remaining {matrix} destinations"
                )
        q_match = q_quiet_re.fullmatch(row["q_quiet"])
        windows_match = windows_quiet_re.fullmatch(row["windows_quiet"])
        if q_match is None or windows_match is None:
            raise AnalysisError(
                f"runtime-gates.csv line {line_number}: quiet gate shape mismatch"
            )
        _gate_decimal(
            q_match.group(1),
            f"runtime-gates.csv line {line_number}: q_load1",
            Decimal("3.0"),
        )
        _gate_decimal(
            q_match.group(2),
            f"runtime-gates.csv line {line_number}: q_spotlight_cpu",
            Decimal("10.0"),
        )
        _gate_decimal(
            windows_match.group(1),
            f"runtime-gates.csv line {line_number}: windows_cpu_avg",
            Decimal("20.0"),
        )


def _load_windows_runtime_evidence(
    root: Path, staging: Sequence[dict[str, str]]
) -> None:
    staged_sha = next(
        row["sha256"]
        for row in staging
        if (row["endpoint"], row["role"]) == ("windows", "daemon")
    )
    swap = _read_single_lf_line(
        root / "windows-runtime-swap.txt", "windows-runtime-swap.txt"
    )
    swap_match = re.fullmatch(
        r"had_prior=([01]) prior_sha=(none|[0-9a-f]{64}) staged_sha=([0-9a-f]{64})",
        swap,
    )
    if swap_match is None:
        raise AnalysisError("windows-runtime-swap.txt has malformed runtime identity")
    had_prior, prior_sha, observed_staged_sha = swap_match.groups()
    if observed_staged_sha != staged_sha:
        raise AnalysisError(
            "windows-runtime-swap.txt staged_sha differs from staging-manifest.csv"
        )
    if (had_prior == "1") != (prior_sha != "none"):
        raise AnalysisError("windows-runtime-swap.txt prior state is inconsistent")
    expected_active = "True" if had_prior == "1" else "False"
    expected_restored = prior_sha if had_prior == "1" else "none"
    expected_restoration = (
        f"RESTORED|mode=normal|active={expected_active}|tested=True|"
        f"tested_sha={staged_sha}|restored_sha={expected_restored}"
    )
    restoration = _read_single_lf_line(
        root / "windows-runtime-restoration.txt",
        "windows-runtime-restoration.txt",
    )
    if restoration != expected_restoration:
        raise AnalysisError(
            "windows-runtime-restoration.txt does not prove the tested daemon was retained and prior state restored"
        )


def _csv_payload(fields: Sequence[str], rows: Iterable[dict[str, str]]) -> str:
    handle = io.StringIO(newline="")
    writer = csv.DictWriter(handle, fieldnames=fields, lineterminator="\n")
    writer.writeheader()
    writer.writerows(rows)
    return handle.getvalue()


def _inventory_input_files(root: Path) -> tuple[list[dict[str, str]], str]:
    rows: list[dict[str, str]] = []
    pending = [root]
    while pending:
        directory = pending.pop()
        try:
            with os.scandir(directory) as iterator:
                entries = sorted(iterator, key=lambda entry: entry.name)
        except OSError as exc:
            raise AnalysisError(f"cannot inventory input directory {directory}") from exc
        for entry in entries:
            path = Path(entry.path)
            relative = path.relative_to(root).as_posix()
            try:
                mode = entry.stat(follow_symlinks=False).st_mode
            except OSError as exc:
                raise AnalysisError(f"cannot stat input evidence {relative}") from exc
            if stat.S_ISLNK(mode):
                raise AnalysisError(f"input evidence contains a symbolic link: {relative}")
            if stat.S_ISDIR(mode):
                pending.append(path)
                continue
            if not stat.S_ISREG(mode):
                raise AnalysisError(f"input evidence is not a plain file: {relative}")
            digest = hashlib.sha256()
            size = 0
            with path.open("rb") as handle:
                for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                    digest.update(chunk)
                    size += len(chunk)
            rows.append(
                {"path": relative, "size": str(size), "sha256": digest.hexdigest()}
            )
    rows.sort(key=lambda row: row["path"])
    payload = _csv_payload(INPUT_INVENTORY_FIELDS, rows)
    return rows, hashlib.sha256(payload.encode("utf-8")).hexdigest()


def _load_manifest(root: Path, relative: str, cache: dict[str, Manifest]) -> Manifest:
    if relative in cache:
        return cache[relative]
    path = _evidence_file(root, relative, f"manifest {relative}")
    payload = path.read_bytes()
    if b"\r" in payload:
        raise AnalysisError(f"manifest {relative}: CR bytes are forbidden")
    if payload and not payload.endswith(b"\n"):
        raise AnalysisError(f"manifest {relative}: final LF is required")
    try:
        lines = payload.decode("ascii").splitlines()
    except UnicodeDecodeError as exc:
        raise AnalysisError(f"manifest {relative}: manifest encoding must be ASCII") from exc
    entries: list[ManifestEntry] = []
    previous_encoded: Optional[str] = None
    decoded_paths: set[str] = set()
    folded_paths: set[str] = set()
    for line_number, line in enumerate(lines, start=1):
        parts = line.split(",")
        if len(parts) != 3:
            raise AnalysisError(
                f"manifest {relative} line {line_number}: expected path,size,sha256"
            )
        encoded, size_text, content_sha = parts
        if previous_encoded is not None and encoded <= previous_encoded:
            raise AnalysisError(
                f"manifest {relative} line {line_number}: paths are not strictly sorted"
            )
        previous_encoded = encoded
        try:
            raw_path = base64.b64decode(encoded, validate=True)
        except (binascii.Error, ValueError) as exc:
            raise AnalysisError(
                f"manifest {relative} line {line_number}: path is not canonical base64"
            ) from exc
        if base64.b64encode(raw_path).decode("ascii") != encoded:
            raise AnalysisError(
                f"manifest {relative} line {line_number}: path base64 is not canonical"
            )
        try:
            decoded = raw_path.decode("utf-8", errors="strict")
        except UnicodeDecodeError as exc:
            raise AnalysisError(
                f"manifest {relative} line {line_number}: path is not UTF-8"
            ) from exc
        pure = PurePosixPath(decoded)
        if (
            not decoded
            or "\\" in decoded
            or "\x00" in decoded
            or pure.is_absolute()
            or str(pure) != decoded
            or any(part in ("", ".", "..") for part in pure.parts)
        ):
            raise AnalysisError(
                f"manifest {relative} line {line_number}: unsafe relative path {decoded!r}"
            )
        if decoded in decoded_paths:
            raise AnalysisError(f"manifest {relative}: duplicate decoded path {decoded!r}")
        decoded_paths.add(decoded)
        folded = decoded.casefold()
        if folded in folded_paths:
            raise AnalysisError(
                f"manifest {relative}: case-colliding cross-platform path {decoded!r}"
            )
        folded_paths.add(folded)
        if not UINT_RE.fullmatch(size_text):
            raise AnalysisError(f"manifest {relative} line {line_number}: invalid size")
        if not SHA256_RE.fullmatch(content_sha):
            raise AnalysisError(
                f"manifest {relative} line {line_number}: invalid content SHA-256"
            )
        entries.append(ManifestEntry(encoded, decoded, int(size_text), content_sha))
    manifest = Manifest(relative, hashlib.sha256(payload).hexdigest(), tuple(entries))
    cache[relative] = manifest
    return manifest


def _load_fixture_index(
    root: Path, cache: dict[str, Manifest], matrix: str
) -> dict[tuple[str, str], Manifest]:
    rows = _read_csv(
        root / "fixture-manifests.csv", FIXTURE_INDEX_FIELDS, "fixture-manifests.csv"
    )
    registered_fixtures = _matrix_fixtures(matrix)
    expected = {
        (direction, fixture)
        for direction in DIRECTIONS
        for fixture in registered_fixtures
    }
    found: dict[tuple[str, str], Manifest] = {}
    for line, row in enumerate(rows, start=2):
        key = (row["direction"], row["fixture"])
        if key not in expected or key in found:
            raise AnalysisError(f"fixture-manifests.csv line {line}: unexpected/duplicate cell {key}")
        expected_manifest = f"manifests/source/{key[0]}_{key[1]}.csv"
        if row["source_manifest"] != expected_manifest:
            raise AnalysisError(
                f"fixture-manifests.csv line {line}: source_manifest is not the registered evidence path"
            )
        manifest = _load_manifest(root, row["source_manifest"], cache)
        wanted_files, wanted_bytes = EXPECTED_FIXTURES[key[1]]
        if (manifest.files, manifest.bytes) != (wanted_files, wanted_bytes):
            raise AnalysisError(
                f"fixture {key}: got {manifest.files} files/{manifest.bytes} bytes, "
                f"expected {wanted_files}/{wanted_bytes}"
            )
        found[key] = manifest
    if set(found) != expected:
        raise AnalysisError(
            f"fixture-manifests.csv must contain every {matrix} direction/fixture cell"
        )
    for fixture in registered_fixtures:
        q_source = found[("q_to_windows", fixture)]
        windows_source = found[("windows_to_q", fixture)]
        if q_source.entries != windows_source.entries:
            raise AnalysisError(
                f"canonical q and Windows {fixture} fixtures differ by path, size, or content"
            )
    return found


def _load_staging_manifest(root: Path) -> list[dict[str, str]]:
    rows = _read_csv(root / "staging-manifest.csv", STAGING_FIELDS, "staging-manifest.csv")
    expected = {
        ("q", "client"),
        ("q", "daemon"),
        ("windows", "client"),
        ("windows", "daemon"),
    }
    found: dict[tuple[str, str], dict[str, str]] = {}
    for line, row in enumerate(rows, start=2):
        key = (row["endpoint"], row["role"])
        if key not in expected or key in found:
            raise AnalysisError(
                f"staging-manifest.csv line {line}: unexpected/duplicate endpoint-role"
            )
        if row["artifact_sha"] != ARTIFACT_SHA:
            raise AnalysisError(
                f"staging-manifest.csv line {line}: artifact_sha is not the accepted artifact"
            )
        if row["build_id"] != ARTIFACT_SHA[:12]:
            raise AnalysisError(
                f"staging-manifest.csv line {line}: build_id is not exact accepted short identity"
            )
        if not SHA256_RE.fullmatch(row["sha256"]):
            raise AnalysisError(
                f"staging-manifest.csv line {line}: binary SHA-256 must be lowercase 64-hex"
            )
        for field in ("staged_path", "runtime_path"):
            _absolute_endpoint_path(row[field], field, line, "staging-manifest.csv")
            is_windows = _windows_path(row[field])
            if is_windows != (row["endpoint"] == "windows"):
                raise AnalysisError(
                    f"staging-manifest.csv line {line}: {field} is on the wrong platform"
                )
        expected_staged, expected_runtime = REGISTERED_BINARY_PATHS[key]
        if (
            row["staged_path"] != expected_staged
            or row["runtime_path"] != expected_runtime
        ):
            raise AnalysisError(
                f"staging-manifest.csv line {line}: binary paths are not the registered staging/runtime paths"
            )
        found[key] = row
    if set(found) != expected:
        raise AnalysisError("staging-manifest.csv must contain exactly q/windows client/daemon")
    return [found[key] for key in sorted(found)]


def _load_runs(
    root: Path,
    fixtures: dict[tuple[str, str], Manifest],
    manifest_cache: dict[str, Manifest],
    matrix: str,
) -> list[RunRow]:
    raw_rows = _read_csv(root / "runs.csv", RUN_FIELDS, "runs.csv")
    schedule_rows = _schedule_rows(matrix)
    expected_count = len(schedule_rows)
    if len(raw_rows) != expected_count:
        raise AnalysisError(f"runs.csv must contain exactly {expected_count} rows")
    if not SAFE_SESSION_ID_RE.fullmatch(root.name):
        raise AnalysisError("session directory basename is not a safe endpoint path component")

    rows: list[RunRow] = []
    seen_run_ids: set[str] = set()
    seen_session_ids: set[str] = set()
    seen_archives: set[str] = set()
    seen_landed_manifests: set[str] = set()
    paths_by_cell: dict[str, tuple[str, str, str]] = {}
    groups_by_cell: dict[str, list[tuple[int, str]]] = defaultdict(list)
    session_safe_id: Optional[str] = None

    for index, raw in enumerate(raw_rows):
        line = index + 2
        direction = raw["direction"]
        fixture = raw["fixture"]
        registered_fixtures = _matrix_fixtures(matrix)
        if direction not in DIRECTIONS or fixture not in registered_fixtures:
            raise AnalysisError(f"runs.csv line {line}: unregistered direction/fixture")
        expected_cell = _cell(direction, fixture)
        if raw["cell"] != expected_cell:
            raise AnalysisError(
                f"runs.csv line {line}: cell must be {expected_cell!r}, got {raw['cell']!r}"
            )
        initiator = raw["initiator"]
        if initiator not in INITIATORS:
            raise AnalysisError(f"runs.csv line {line}: invalid initiator {initiator!r}")
        pair = _csv_int(raw["pair"], "pair", line, minimum=1)
        registered_pairs = PAIRS if matrix == "fixed" else (1,)
        if pair not in registered_pairs:
            raise AnalysisError(
                f"runs.csv line {line}: pair is outside the registered {matrix} matrix"
            )
        expected_run_id = f"ldt4-{index + 1:03d}"
        if raw["run_id"] != expected_run_id or raw["run_id"] in seen_run_ids:
            raise AnalysisError(
                f"runs.csv line {line}: run_id must be exact schedule identity {expected_run_id}"
            )
        seen_run_ids.add(raw["run_id"])
        if not SESSION_ID_RE.fullmatch(raw["session_id"]) or raw["session_id"] in seen_session_ids:
            raise AnalysisError(f"runs.csv line {line}: session_id is malformed or duplicated")
        seen_session_ids.add(raw["session_id"])
        if raw["exit"] != "0" or raw["valid"] != "yes":
            raise AnalysisError(f"runs.csv line {line}: arm is not a clean valid success")

        duration = _decimal(raw["duration_ms"], "duration_ms", line)
        files = _csv_int(raw["files"], "files", line)
        byte_count = _csv_int(raw["bytes"], "bytes", line)
        if (files, byte_count) != EXPECTED_FIXTURES[fixture]:
            raise AnalysisError(f"runs.csv line {line}: fixture file/byte totals mismatch")
        for field in ("source_path", "active_destination_path", "archive_path"):
            _absolute_endpoint_path(raw[field], field, line)
        path_platforms = tuple(
            _windows_path(raw[field])
            for field in ("source_path", "active_destination_path", "archive_path")
        )
        expected_platforms = (
            (False, True, True) if direction == "q_to_windows" else (True, False, False)
        )
        if path_platforms != expected_platforms:
            raise AnalysisError(
                f"runs.csv line {line}: physical paths disagree with semantic byte direction"
            )
        registered_source = _registered_source_path(direction, fixture)
        if raw["source_path"] != registered_source:
            raise AnalysisError(
                f"runs.csv line {line}: source_path is not the registered physical fixture"
            )
        destination_root = _session_destination_root(direction, fixture)
        active_match = re.fullmatch(
            rf"{re.escape(destination_root)}/([^/]+)/active/{re.escape(fixture)}",
            raw["active_destination_path"],
        )
        if active_match is None or not SAFE_SESSION_ID_RE.fullmatch(active_match.group(1)):
            raise AnalysisError(
                f"runs.csv line {line}: active destination is outside the fresh session pattern"
            )
        row_safe_id = active_match.group(1)
        if row_safe_id != root.name:
            raise AnalysisError(
                f"runs.csv line {line}: endpoint session safe-id must equal the evidence directory basename"
            )
        if session_safe_id is None:
            session_safe_id = row_safe_id
        elif row_safe_id != session_safe_id:
            raise AnalysisError(f"runs.csv line {line}: session safe-id drift")
        expected_archive = f"{destination_root}/{row_safe_id}/retained/{raw['run_id']}"
        if raw["archive_path"] != expected_archive:
            raise AnalysisError(
                f"runs.csv line {line}: archive_path is not the retained run path"
            )
        archive_key = raw["archive_path"].casefold()
        if archive_key in seen_archives:
            raise AnalysisError(f"runs.csv line {line}: archive_path must be globally unique")
        seen_archives.add(archive_key)
        if raw["archive_path"].casefold() == raw["active_destination_path"].casefold():
            raise AnalysisError(f"runs.csv line {line}: retained archive equals active destination")

        indexed = fixtures[(direction, fixture)]
        if raw["source_manifest"] != indexed.relative_path:
            raise AnalysisError(f"runs.csv line {line}: source_manifest disagrees with fixture index")
        expected_landed_manifest = f"manifests/landed/{raw['run_id']}.csv"
        if raw["landed_manifest"] != expected_landed_manifest:
            raise AnalysisError(
                f"runs.csv line {line}: landed_manifest is not the registered evidence path"
            )
        landed_manifest_key = raw["landed_manifest"].casefold()
        if landed_manifest_key in seen_landed_manifests:
            raise AnalysisError(f"runs.csv line {line}: landed_manifest must be globally unique")
        seen_landed_manifests.add(landed_manifest_key)
        landed = _load_manifest(root, raw["landed_manifest"], manifest_cache)
        if landed.entries != indexed.entries:
            raise AnalysisError(
                f"runs.csv line {line}: retained archive manifest differs by path, size, or content"
            )

        path_tuple = (
            raw["source_path"],
            raw["active_destination_path"],
            raw["source_manifest"],
        )
        prior_paths = paths_by_cell.setdefault(expected_cell, path_tuple)
        if path_tuple != prior_paths:
            raise AnalysisError(
                f"runs.csv line {line}: source/active path drift within {expected_cell}"
            )

        trace_paths = _registered_trace_paths(direction, initiator, raw["run_id"])
        if (raw["source_trace"], raw["destination_trace"]) != trace_paths:
            raise AnalysisError(
                f"runs.csv line {line}: trace paths are not the registered endpoint/component paths"
            )
        _evidence_file(root, raw["source_trace"], f"source trace line {line}")
        _evidence_file(root, raw["destination_trace"], f"destination trace line {line}")
        rows.append(
            RunRow(
                line,
                index,
                expected_cell,
                direction,
                fixture,
                pair,
                initiator,
                raw["run_id"],
                raw["session_id"],
                duration,
                files,
                byte_count,
                raw["source_path"],
                raw["active_destination_path"],
                raw["archive_path"],
                raw["source_manifest"],
                raw["landed_manifest"],
                raw["source_trace"],
                raw["destination_trace"],
            )
        )

    expected_cells = _matrix_cells(matrix)
    if set(paths_by_cell) != set(expected_cells):
        raise AnalysisError(f"runs.csv does not cover every {matrix} cell")
    if len(rows) % 2:
        raise AnalysisError("runs.csv role-pair schedule is not adjacent")
    for offset in range(0, len(rows), 2):
        first, second = rows[offset : offset + 2]
        if (first.cell, first.pair) != (second.cell, second.pair):
            raise AnalysisError(
                f"runs.csv lines {first.csv_line}-{second.csv_line}: role pair is not adjacent"
            )
        if {first.initiator, second.initiator} != set(INITIATORS):
            raise AnalysisError(
                f"runs.csv lines {first.csv_line}-{second.csv_line}: role pair is incomplete"
            )
        groups_by_cell[first.cell].append((first.pair, first.initiator))
    actual_group_order = [(rows[offset].cell, rows[offset].pair) for offset in range(0, len(rows), 2)]
    expected_group_order = [
        (schedule_rows[index][1], (index // 2) % len(PAIRS) + 1 if matrix == "fixed" else 1)
        for index in range(0, len(schedule_rows), 2)
    ]
    if actual_group_order != expected_group_order:
        raise AnalysisError("runs.csv global cell/pair order differs from the registered schedule")
    for cell in expected_cells:
        if matrix == "fixed":
            wanted = list(zip(PAIRS, FIRST_ROLE))
        else:
            cell_index = expected_cells.index(cell)
            wanted = [(1, _matrix_first_roles(matrix)[cell_index])]
        if groups_by_cell[cell] != wanted:
            if matrix == "fixed":
                raise AnalysisError(
                    f"runs.csv {cell}: expected pair 1..8 ABBAABBA first-role order"
                )
            raise AnalysisError(
                f"runs.csv {cell}: first-role order differs from the registered {matrix} schedule"
            )
    return rows


def _load_trace_file(root: Path, relative: str, cache: dict[str, list[dict[str, Any]]]) -> list[dict[str, Any]]:
    if relative in cache:
        return cache[relative]
    path = _evidence_file(root, relative, f"trace {relative}")
    events: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8", errors="strict") as handle:
        for line_number, line in enumerate(handle, start=1):
            line = line.rstrip("\n")
            if not line.startswith(TRACE_PREFIX):
                continue
            payload = line[len(TRACE_PREFIX) :]
            try:
                event = json.loads(payload, object_pairs_hook=_reject_duplicate_json_keys)
            except DuplicateJsonKeyError as exc:
                raise AnalysisError(
                    f"trace {relative} line {line_number}: duplicate JSON key {exc.key!r}"
                ) from exc
            except json.JSONDecodeError as exc:
                raise AnalysisError(
                    f"trace {relative} line {line_number}: malformed session-phase JSON"
                ) from exc
            if not isinstance(event, dict):
                raise AnalysisError(f"trace {relative} line {line_number}: event must be an object")
            events.append(event)
    cache[relative] = events
    return events


def _session_events(
    root: Path,
    relative: str,
    row: RunRow,
    endpoint_role: str,
    cache: dict[str, list[dict[str, Any]]],
) -> list[dict[str, Any]]:
    context = f"{row.cell} pair {row.pair} {row.initiator} {endpoint_role}"
    events = _load_trace_file(root, relative, cache)
    if not events:
        raise AnalysisError(f"{context}: dedicated trace has no session events")
    if any(
        event.get("run_id") != row.run_id
        or event.get("session_id") != row.session_id
        for event in events
    ):
        raise AnalysisError(f"{context}: dedicated trace contains a foreign session event")
    initiator_role = "SOURCE" if row.initiator == "source_init" else "DESTINATION"
    for event in events:
        missing = COMMON_EVENT_FIELDS - set(event)
        if missing:
            raise AnalysisError(f"{context}: event missing common fields {sorted(missing)}")
        if _int(event["schema"], "schema", context, minimum=1) != 1:
            raise AnalysisError(f"{context}: unsupported trace schema")
        if event["run_id"] != row.run_id or event["session_id"] != row.session_id:
            raise AnalysisError(f"{context}: trace correlation mismatch")
        if event["endpoint_role"] != endpoint_role:
            raise AnalysisError(f"{context}: endpoint role mismatch")
        if event["initiator_role"] != initiator_role:
            raise AnalysisError(f"{context}: initiator role mismatch")
        _int(event["producer_seq"], "producer_seq", context)
        _int(event["unix_ns"], "unix_ns", context)
        _int(event["elapsed_ns"], "elapsed_ns", context)
        if not isinstance(event["event"], str) or not event["event"]:
            raise AnalysisError(f"{context}: event name is malformed")
        if event["event"] not in EVENT_NAMES_BY_ROLE[endpoint_role]:
            raise AnalysisError(
                f"{context}: event {event['event']!r} is not registered for {endpoint_role}"
            )
        if event["event"] == "data_plane_aborted" or "fault" in event["event"]:
            raise AnalysisError(f"{context}: aborted/faulted data plane")
    ordered = sorted(events, key=lambda event: event["producer_seq"])
    if [event["producer_seq"] for event in ordered] != list(range(len(ordered))):
        raise AnalysisError(f"{context}: producer_seq is not exact contiguous 0..n-1")
    return ordered


def _validated_socket_events(
    events: Sequence[dict[str, Any]], context: str
) -> list[dict[str, Any]]:
    sockets: list[dict[str, Any]] = []
    for event in events:
        if not event["event"].startswith("socket_"):
            continue
        if event["event"] not in SOCKET_EVENTS:
            raise AnalysisError(f"{context}: unknown socket event {event['event']!r}")
        _event_exact(event, {"epoch", "socket"}, context)
        _int(event["epoch"], "epoch", context)
        _int(event["socket"], "socket", context)
        sockets.append(event)
    return sockets


def _validate_epoch0_sockets(
    socket_events: Sequence[dict[str, Any]], expected_transport: str, context: str
) -> None:
    expected_begin = f"socket_{expected_transport}_begin"
    expected_end = f"socket_{expected_transport}_end"
    acquisition_events = [
        event
        for event in socket_events
        if event["event"] in SOCKET_ACQUISITION_EVENTS and event["epoch"] == 0
    ]
    expected = sorted(
        [(expected_begin, socket_id) for socket_id in range(EXPECTED_FLOOR)]
        + [(expected_end, socket_id) for socket_id in range(EXPECTED_FLOOR)]
    )
    actual: list[tuple[str, int]] = []
    for event in acquisition_events:
        actual.append((event["event"], event["socket"]))
    if sorted(actual) != expected:
        raise AnalysisError(
            f"{context}: epoch-0 topology must have four {expected_transport} begin/end socket pairs"
        )
    for socket_id in range(EXPECTED_FLOOR):
        begin = next(
            event
            for event in acquisition_events
            if event["event"] == expected_begin and event["socket"] == socket_id
        )
        end = next(
            event
            for event in acquisition_events
            if event["event"] == expected_end and event["socket"] == socket_id
        )
        if begin["producer_seq"] >= end["producer_seq"]:
            raise AnalysisError(f"{context}: socket {socket_id} ended before it began")


def _validate_resize_sockets(
    socket_events: Sequence[dict[str, Any]],
    expected_transport: str,
    operations: Sequence[tuple[int, str, int]],
    context: str,
) -> None:
    add_epochs = {epoch for epoch, action, _target in operations if action == "ADD"}
    nonzero = [event for event in socket_events if event["epoch"] > 0]
    for event in nonzero:
        if event["epoch"] not in add_epochs:
            raise AnalysisError(
                f"{context}: socket event belongs to non-ADD/unknown epoch {event['epoch']}"
            )

    expected_begin = f"socket_{expected_transport}_begin"
    expected_end = f"socket_{expected_transport}_end"
    acquisitions = [
        event for event in nonzero if event["event"] in SOCKET_ACQUISITION_EVENTS
    ]
    expected = sorted(
        (name, epoch, 0)
        for epoch in add_epochs
        for name in (expected_begin, expected_end)
    )
    actual = sorted(
        (event["event"], event["epoch"], event["socket"]) for event in acquisitions
    )
    if actual != expected:
        raise AnalysisError(
            f"{context}: every accepted ADD must have one exact {expected_transport} "
            "begin/end socket pair"
        )
    for epoch in add_epochs:
        begin = next(
            event
            for event in acquisitions
            if event["event"] == expected_begin and event["epoch"] == epoch
        )
        end = next(
            event
            for event in acquisitions
            if event["event"] == expected_end and event["epoch"] == epoch
        )
        if begin["producer_seq"] >= end["producer_seq"]:
            raise AnalysisError(
                f"{context}: ADD epoch {epoch} socket ended before it began"
            )


def _completion(events: Sequence[dict[str, Any]], context: str) -> dict[str, Any]:
    complete = [event for event in events if event["event"] == "data_plane_complete"]
    if len(complete) != 1:
        raise AnalysisError(f"{context}: expected exactly one data_plane_complete")
    event = complete[0]
    expected = COMMON_EVENT_FIELDS | {"live_streams", "receiver_ceiling", "peak_streams"}
    if set(event) != expected:
        raise AnalysisError(f"{context}: data_plane_complete fields are not exact")
    for field in ("live_streams", "receiver_ceiling", "peak_streams"):
        _int(event[field], field, context, minimum=1)
    return event


def _membership_sealed(
    events: Sequence[dict[str, Any]],
    complete: dict[str, Any],
    dial: DialResult,
    context: str,
) -> dict[str, Any]:
    sealed = [event for event in events if event["event"] == "membership_sealed"]
    if len(sealed) != 1:
        raise AnalysisError(f"{context}: expected exactly one membership_sealed")
    event = sealed[0]
    _event_exact(event, {"live_streams", "receiver_ceiling", "peak_streams"}, context)
    for field in ("live_streams", "receiver_ceiling", "peak_streams"):
        _int(event[field], field, context, minimum=1)
    if (
        event["live_streams"],
        event["receiver_ceiling"],
        event["peak_streams"],
    ) != (dial.final, dial.ceiling, dial.peak):
        raise AnalysisError(f"{context}: membership_sealed values differ from dial lifecycle")
    if event["producer_seq"] >= complete["producer_seq"]:
        raise AnalysisError(f"{context}: membership_sealed did not precede completion")
    return event


def _event_exact(event: dict[str, Any], fields: set[str], context: str) -> None:
    expected = COMMON_EVENT_FIELDS | fields
    if set(event) != expected:
        extra = sorted(set(event) - expected)
        missing = sorted(expected - set(event))
        raise AnalysisError(f"{context}: event fields are not exact; missing={missing}, extra={extra}")


@dataclass
class DialPolicyReplay:
    chunk_bytes: int = FLOOR_CHUNK_BYTES
    prefetch_count: int = FLOOR_PREFETCH
    tcp_buffer_bytes: int = 0
    ticks_since_settle: int = 0
    sustain: int = 0

    def apply_sample(
        self,
        event: dict[str, Any],
        live_streams: int,
        settled_epoch: int,
        context: str,
    ) -> tuple[str, Optional[tuple[int, str, int]]]:
        self.ticks_since_settle += 1
        valid = event["sample_valid"]
        sample_bytes = event["sample_bytes"]
        ratio = float(event["blocked_ratio"])
        cheap_reason: Optional[str] = None

        if not valid or sample_bytes == 0:
            self.sustain = 0
            resize_reason = "idle"
        else:
            if ratio < STEP_UP_THRESHOLD:
                next_chunk = min(self.chunk_bytes * 2, CEILING_CHUNK_BYTES)
                next_prefetch = min(
                    self.prefetch_count + max(self.prefetch_count // 2, 1),
                    CEILING_PREFETCH,
                )
                next_tcp = CEILING_TCP_BUFFER_BYTES
                moved = (
                    next_chunk != self.chunk_bytes
                    or next_prefetch != self.prefetch_count
                    or next_tcp != self.tcp_buffer_bytes
                )
                self.chunk_bytes = next_chunk
                self.prefetch_count = next_prefetch
                self.tcp_buffer_bytes = next_tcp
                if moved:
                    cheap_reason = "cheap-up"
            elif ratio > STEP_DOWN_THRESHOLD:
                next_chunk = max(self.chunk_bytes // 2, FLOOR_CHUNK_BYTES)
                next_prefetch = max(self.prefetch_count // 2, FLOOR_PREFETCH, 1)
                moved = (
                    next_chunk != self.chunk_bytes
                    or next_prefetch != self.prefetch_count
                )
                self.chunk_bytes = next_chunk
                self.prefetch_count = next_prefetch
                if moved:
                    cheap_reason = "cheap-down"

            if (
                ratio < STEP_UP_THRESHOLD
                and self.chunk_bytes >= CEILING_CHUNK_BYTES
                and self.prefetch_count >= CEILING_PREFETCH
            ):
                self.sustain = max(self.sustain, 0) + 1
            elif (
                ratio > STEP_DOWN_THRESHOLD
                and self.chunk_bytes <= FLOOR_CHUNK_BYTES
                and self.prefetch_count <= FLOOR_PREFETCH
            ):
                self.sustain = min(self.sustain, 0) - 1
            else:
                self.sustain = 0

            if self.sustain == 0:
                resize_reason = "hysteresis"
            elif self.ticks_since_settle < RESIZE_COOLDOWN_TICKS:
                resize_reason = "cooldown"
            elif self.sustain >= RESIZE_SUSTAIN_TICKS:
                resize_reason = "add"
            elif self.sustain <= -RESIZE_SUSTAIN_TICKS:
                resize_reason = "remove"
            else:
                resize_reason = "sustain"

        proposal: Optional[tuple[int, str, int]] = None
        if resize_reason in {"add", "remove"}:
            action = resize_reason.upper()
            target = live_streams + (1 if action == "ADD" else -1)
            target = max(1, min(target, EXPECTED_CEILING))
            if target == live_streams:
                self.sustain = 0
                resize_reason = "bound"
            else:
                proposal = (settled_epoch + 1, action, target)
                self.sustain = 0

        expected_reason = "rebaseline" if not valid else cheap_reason or resize_reason
        if proposal is not None:
            expected_reason = proposal[1].lower()
        if event["reason"] != expected_reason:
            raise AnalysisError(
                f"{context}: sample reason {event['reason']!r} does not match policy replay "
                f"{expected_reason!r}"
            )
        observed_cheap = (
            event["chunk_bytes"],
            event["prefetch_count"],
            event["tcp_buffer_bytes"],
        )
        expected_cheap = (
            self.chunk_bytes,
            self.prefetch_count,
            self.tcp_buffer_bytes,
        )
        if observed_cheap != expected_cheap:
            raise AnalysisError(
                f"{context}: cheap dial snapshot {observed_cheap} does not match policy "
                f"replay {expected_cheap}"
            )
        return expected_reason, proposal

    def settled(self) -> None:
        self.ticks_since_settle = 0
        self.sustain = 0


def _validate_dial(events: Sequence[dict[str, Any]], context: str) -> DialResult:
    dial_events = [event for event in events if event["event"].startswith("dial_")]
    if not dial_events:
        return DialResult(
            EXPECTED_FLOOR,
            EXPECTED_FLOOR,
            EXPECTED_FLOOR,
            EXPECTED_CEILING,
            0,
            0,
            Counter(),
            (),
            (),
        )
    if dial_events[0]["event"] != "dial_sample":
        raise AnalysisError(f"{context}: first dial event is not a sample")

    current = EXPECTED_FLOOR
    peak = EXPECTED_FLOOR
    settled_epoch = 0
    awaiting_pending: Optional[tuple[int, str, int]] = None
    pending: Optional[tuple[int, str, int]] = None
    rebaseline_due = False
    reasons: Counter[str] = Counter()
    samples: list[dict[str, Any]] = []
    operations: list[tuple[int, str, int]] = []
    add_count = 0
    remove_count = 0
    policy = DialPolicyReplay()

    for event in dial_events:
        name = event["event"]
        if name == "dial_sample":
            if awaiting_pending is not None or pending is not None:
                raise AnalysisError(f"{context}: sample overtook an unsettled proposal")
            reason = event.get("reason")
            observed_proposal = reason in {"add", "remove"}
            fields = SAMPLE_FIELDS | (
                {"action", "target_streams"} if observed_proposal else set()
            )
            _event_exact(event, fields, context)
            if reason not in SAMPLE_REASONS:
                raise AnalysisError(f"{context}: unknown dial sample reason {reason!r}")
            reasons[reason] += 1
            for field in (
                "epoch",
                "live_streams",
                "sample_bytes",
                "sample_blocked_ns",
                "sample_elapsed_ns",
                "sample_streams",
                "chunk_bytes",
                "prefetch_count",
                "tcp_buffer_bytes",
                "receiver_ceiling",
                "peak_streams",
            ):
                minimum = 1 if field in {
                    "live_streams",
                    "sample_elapsed_ns",
                    "chunk_bytes",
                    "prefetch_count",
                    "receiver_ceiling",
                    "peak_streams",
                } else 0
                _int(event[field], field, context, minimum=minimum)
            if not isinstance(event["sample_valid"], bool):
                raise AnalysisError(f"{context}: sample_valid must be boolean")
            if event["sample_elapsed_ns"] < DIAL_TUNER_TICK_NS:
                raise AnalysisError(
                    f"{context}: sample_elapsed_ns is below the {DIAL_TUNER_TICK_NS}ns "
                    "production tuner cadence"
                )
            if (
                event["sample_valid"]
                and event["sample_streams"] != event["live_streams"]
            ):
                raise AnalysisError(
                    f"{context}: valid sample_streams must equal settled live_streams"
                )
            ratio = event["blocked_ratio"]
            if isinstance(ratio, bool) or not isinstance(ratio, (int, float)) or not math.isfinite(ratio):
                raise AnalysisError(f"{context}: blocked_ratio must be finite numeric")
            denominator = event["sample_elapsed_ns"] * event["sample_streams"]
            computed = (
                0.0
                if denominator == 0
                else min(1.0, event["sample_blocked_ns"] / denominator)
            )
            if float(ratio) != computed:
                raise AnalysisError(f"{context}: blocked_ratio does not match raw sample counters")
            if event["receiver_ceiling"] != EXPECTED_CEILING:
                raise AnalysisError(f"{context}: receiver ceiling is not the accepted safety ceiling")
            if event["live_streams"] != current or event["peak_streams"] != peak:
                raise AnalysisError(f"{context}: sample live/peak membership drift")
            if not event["sample_valid"]:
                if not rebaseline_due:
                    raise AnalysisError(
                        f"{context}: invalid rebaseline has no immediately preceding "
                        "membership settlement"
                    )
                if event["sample_streams"] != current:
                    raise AnalysisError(
                        f"{context}: rebaseline sample_streams must equal new settled membership"
                    )
                if reason != "rebaseline" or any(
                    event[field] != 0
                    for field in ("sample_bytes", "sample_blocked_ns", "blocked_ratio")
                ):
                    raise AnalysisError(f"{context}: invalid sample is not an exact rebaseline")
                rebaseline_due = False
            elif rebaseline_due:
                raise AnalysisError(
                    f"{context}: first sample after membership settlement is not rebaseline"
                )
            elif event["sample_bytes"] == 0 and reason != "idle":
                raise AnalysisError(f"{context}: zero-byte valid sample must be idle")
            elif event["sample_bytes"] > 0 and reason in {"idle", "rebaseline"}:
                raise AnalysisError(f"{context}: busy valid sample has an impossible reason")

            _expected_reason, expected_proposal = policy.apply_sample(
                event, current, settled_epoch, context
            )
            if (expected_proposal is not None) != observed_proposal:
                raise AnalysisError(
                    f"{context}: proposal presence does not match policy replay"
                )

            sample_record = {
                field: event[field]
                for field in (
                    "producer_seq",
                    "elapsed_ns",
                    "reason",
                    "epoch",
                    "live_streams",
                    "peak_streams",
                    "receiver_ceiling",
                    "sample_bytes",
                    "sample_blocked_ns",
                    "sample_elapsed_ns",
                    "sample_streams",
                    "sample_valid",
                    "blocked_ratio",
                    "chunk_bytes",
                    "prefetch_count",
                    "tcp_buffer_bytes",
                )
            }
            if observed_proposal:
                action = event["action"]
                target = _int(event["target_streams"], "target_streams", context, minimum=1)
                expected_action = reason.upper()
                if action != expected_action:
                    raise AnalysisError(f"{context}: proposal reason/action mismatch")
                expected_target = current + (1 if action == "ADD" else -1)
                if target != expected_target or not 1 <= target <= EXPECTED_CEILING:
                    raise AnalysisError(f"{context}: proposal target is not an exact one-stream step")
                if event["epoch"] != settled_epoch + 1:
                    raise AnalysisError(f"{context}: proposal epoch is not contiguous")
                if expected_proposal != (event["epoch"], action, target):
                    raise AnalysisError(
                        f"{context}: proposal does not match policy replay"
                    )
                awaiting_pending = (event["epoch"], action, target)
                sample_record.update({"action": action, "target_streams": target})
            elif event["epoch"] != settled_epoch:
                raise AnalysisError(f"{context}: ordinary sample epoch differs from settled epoch")
            samples.append(sample_record)
        elif name == "dial_pending":
            _event_exact(event, PENDING_FIELDS, context)
            if awaiting_pending is None or pending is not None:
                raise AnalysisError(f"{context}: orphaned/duplicate pending event")
            epoch, action, target = awaiting_pending
            if (
                event["reason"] != "pending"
                or event["epoch"] != epoch
                or event["action"] != action
                or event["target_streams"] != target
                or event["live_streams"] != current
                or event["peak_streams"] != peak
                or event["receiver_ceiling"] != EXPECTED_CEILING
            ):
                raise AnalysisError(f"{context}: pending event does not match proposal")
            pending = awaiting_pending
            awaiting_pending = None
        elif name == "dial_settlement":
            _event_exact(event, SETTLEMENT_FIELDS, context)
            if pending is None or awaiting_pending is not None:
                raise AnalysisError(f"{context}: settlement has no matching pending proposal")
            epoch, action, target = pending
            if (
                event["epoch"] != epoch
                or event["action"] != action
                or event["target_streams"] != target
                or event["receiver_ceiling"] != EXPECTED_CEILING
            ):
                raise AnalysisError(f"{context}: settlement does not match pending proposal")
            if not isinstance(event["accepted"], bool):
                raise AnalysisError(f"{context}: settlement accepted must be boolean")
            if not event["accepted"]:
                if event["reason"] != "refused" or event["live_streams"] != current:
                    raise AnalysisError(f"{context}: refused settlement fields are inconsistent")
                raise AnalysisError(f"{context}: clean same-build session refused a resize")
            if event["reason"] != action.lower() or event["live_streams"] != target:
                raise AnalysisError(f"{context}: accepted settlement reason/live count mismatch")
            expected_peak = max(peak, target)
            if event["peak_streams"] != expected_peak:
                raise AnalysisError(f"{context}: accepted settlement peak is not monotonic")
            current = target
            peak = expected_peak
            settled_epoch = epoch
            policy.settled()
            rebaseline_due = True
            operations.append((epoch, action, target))
            if action == "ADD":
                add_count += 1
            else:
                remove_count += 1
            pending = None
        else:
            raise AnalysisError(f"{context}: unknown dial observer event {name!r}")

    if awaiting_pending is not None or pending is not None:
        raise AnalysisError(f"{context}: trace ends with an unsettled proposal")
    return DialResult(
        EXPECTED_FLOOR,
        peak,
        current,
        EXPECTED_CEILING,
        add_count,
        remove_count,
        reasons,
        tuple(samples),
        tuple(operations),
    )


SOURCE_CONTROL_FIELDS: dict[str, set[str]] = {
    "resize_proposed": {"action", "epoch", "target_streams", "live_streams"},
    "resize_send_begin": {"action", "epoch", "target_streams", "live_streams"},
    "resize_sent": {"action", "epoch", "target_streams", "live_streams"},
    "resize_ack_received": {"epoch", "live_streams", "accepted"},
    "source_settled": {
        "action",
        "epoch",
        "target_streams",
        "live_streams",
        "accepted",
    },
}
DESTINATION_CONTROL_FIELDS: dict[str, set[str]] = {
    "resize_received": {"epoch", "target_streams", "live_streams"},
    "destination_prepared": {"action", "epoch", "target_streams", "accepted"},
    "resize_ack_send_begin": {"action", "epoch", "live_streams", "accepted"},
    "resize_ack_sent": {"action", "epoch", "live_streams", "accepted"},
    "resize_arm_queue_begin": {"epoch", "target_streams"},
    "resize_arm_ready": {"epoch"},
}


def _one_epoch_event(
    events: Sequence[dict[str, Any]], name: str, epoch: int, context: str
) -> dict[str, Any]:
    found = [
        event for event in events if event["event"] == name and event.get("epoch") == epoch
    ]
    if len(found) != 1:
        raise AnalysisError(f"{context}: expected exactly one {name} for epoch {epoch}")
    return found[0]


def _one_socket_event(
    events: Sequence[dict[str, Any]],
    name: str,
    epoch: int,
    socket: int,
    context: str,
) -> dict[str, Any]:
    found = [
        event
        for event in events
        if event["event"] == name
        and event.get("epoch") == epoch
        and event.get("socket") == socket
    ]
    if len(found) != 1:
        raise AnalysisError(
            f"{context}: expected exactly one {name} for epoch/socket {epoch}/{socket}"
        )
    return found[0]


def _validate_control_event_shapes(
    events: Sequence[dict[str, Any]],
    shapes: dict[str, set[str]],
    context: str,
) -> list[dict[str, Any]]:
    selected = [event for event in events if event["event"] in shapes]
    for event in selected:
        _event_exact(event, shapes[event["event"]], context)
        for field in ("epoch", "target_streams", "live_streams"):
            if field in event:
                _int(event[field], field, context, minimum=1)
        if "accepted" in event and not isinstance(event["accepted"], bool):
            raise AnalysisError(f"{context}: {event['event']} accepted must be boolean")
    return selected


def _validate_membership_observation(
    source_events: Sequence[dict[str, Any]],
    destination_events: Sequence[dict[str, Any]],
    source_sockets: Sequence[dict[str, Any]],
    destination_sockets: Sequence[dict[str, Any]],
    operations: Sequence[tuple[int, str, int]],
    destination_complete: dict[str, Any],
    context: str,
) -> None:
    expected_members = {(0, socket_id) for socket_id in range(EXPECTED_FLOOR)}
    expected_members.update(
        (epoch, 0) for epoch, action, _target in operations if action == "ADD"
    )
    for role, socket_events in (
        ("SOURCE", source_sockets),
        ("DESTINATION", destination_sockets),
    ):
        attached = [
            event for event in socket_events if event["event"] == "socket_trace_attached"
        ]
        attached_keys = [(event["epoch"], event["socket"]) for event in attached]
        if len(attached_keys) != len(expected_members) or set(attached_keys) != expected_members:
            raise AnalysisError(
                f"{context} {role}: socket_trace_attached does not exactly cover membership"
            )
        for event in attached:
            acquisition_end = _one_socket_event(
                socket_events,
                "socket_dial_end",
                event["epoch"],
                event["socket"],
                f"{context} {role}",
            ) if any(
                candidate["event"] == "socket_dial_end"
                and candidate["epoch"] == event["epoch"]
                and candidate["socket"] == event["socket"]
                for candidate in socket_events
            ) else _one_socket_event(
                socket_events,
                "socket_accept_end",
                event["epoch"],
                event["socket"],
                f"{context} {role}",
            )
            if acquisition_end["socket"] != event["socket"]:
                raise AnalysisError(f"{context} {role}: attachment socket identity drift")
            if acquisition_end["producer_seq"] >= event["producer_seq"]:
                raise AnalysisError(f"{context} {role}: trace attached before socket acquisition")

    if any(event["event"] == "receive_task_stopped" for event in source_events):
        raise AnalysisError(f"{context}: SOURCE unexpectedly emitted receive_task_stopped")
    stopped = [
        event for event in destination_events if event["event"] == "receive_task_stopped"
    ]
    for event in stopped:
        _event_exact(event, {"epoch", "socket"}, f"{context} DESTINATION")
        _int(event["epoch"], "epoch", f"{context} DESTINATION")
        _int(event["socket"], "socket", f"{context} DESTINATION")
    stopped_keys = [(event["epoch"], event["socket"]) for event in stopped]
    if len(stopped_keys) != len(expected_members) or set(stopped_keys) != expected_members:
        raise AnalysisError(
            f"{context} DESTINATION: receive_task_stopped does not exactly cover membership"
        )
    attached_by_key = {
        (event["epoch"], event["socket"]): event
        for event in destination_sockets
        if event["event"] == "socket_trace_attached"
    }
    for event in stopped:
        key = (event["epoch"], event["socket"])
        if not (
            attached_by_key[key]["producer_seq"]
            < event["producer_seq"]
            < destination_complete["producer_seq"]
        ):
            raise AnalysisError(
                f"{context} DESTINATION: receive task stop ordering is invalid for {key}"
            )


def _payload_event_map(
    events: Sequence[dict[str, Any]], name: str, context: str
) -> dict[tuple[int, int], dict[str, Any]]:
    found: dict[tuple[int, int], dict[str, Any]] = {}
    for event in events:
        if event["event"] != name:
            continue
        _event_exact(event, {"epoch", "socket"}, context)
        key = (
            _int(event["epoch"], "epoch", context),
            _int(event["socket"], "socket", context),
        )
        if key in found:
            raise AnalysisError(f"{context}: duplicate {name} for member {key}")
        found[key] = event
    return found


def _validate_payload_socket_observation(
    source_events: Sequence[dict[str, Any]],
    destination_events: Sequence[dict[str, Any]],
    source_sockets: Sequence[dict[str, Any]],
    destination_sockets: Sequence[dict[str, Any]],
    operations: Sequence[tuple[int, str, int]],
    source_complete: dict[str, Any],
    destination_complete: dict[str, Any],
    context: str,
) -> None:
    write_begins = _payload_event_map(
        source_events, "socket_write_begin", f"{context} SOURCE"
    )
    first_writes = _payload_event_map(
        source_events, "first_socket_write", f"{context} SOURCE"
    )
    first_receives = _payload_event_map(
        destination_events, "first_payload_received", f"{context} DESTINATION"
    )
    used_members = set(write_begins)
    if not used_members:
        raise AnalysisError(f"{context}: transfer has no payload socket markers")
    if used_members != set(first_writes) or used_members != set(first_receives):
        raise AnalysisError(f"{context}: payload socket marker keys differ")

    admitted_members = {(0, socket_id) for socket_id in range(EXPECTED_FLOOR)}
    admitted_members.update(
        (epoch, 0) for epoch, action, _target in operations if action == "ADD"
    )
    if not used_members.issubset(admitted_members):
        raise AnalysisError(f"{context}: payload marker belongs to an unadmitted member")

    source_attachments = {
        (event["epoch"], event["socket"]): event
        for event in source_sockets
        if event["event"] == "socket_trace_attached"
    }
    destination_attachments = {
        (event["epoch"], event["socket"]): event
        for event in destination_sockets
        if event["event"] == "socket_trace_attached"
    }
    destination_stops = {
        (event["epoch"], event["socket"]): event
        for event in destination_events
        if event["event"] == "receive_task_stopped"
    }
    for key in used_members:
        if not (
            source_attachments[key]["producer_seq"]
            < write_begins[key]["producer_seq"]
            < first_writes[key]["producer_seq"]
            < source_complete["producer_seq"]
        ):
            raise AnalysisError(
                f"{context} SOURCE: payload write ordering is invalid for {key}"
            )
        if not (
            destination_attachments[key]["producer_seq"]
            < first_receives[key]["producer_seq"]
            < destination_stops[key]["producer_seq"]
            < destination_complete["producer_seq"]
        ):
            raise AnalysisError(
                f"{context} DESTINATION: payload receive ordering is invalid for {key}"
            )


def _validate_control_lane(
    source: Sequence[dict[str, Any]],
    destination: Sequence[dict[str, Any]],
    source_sockets: Sequence[dict[str, Any]],
    destination_sockets: Sequence[dict[str, Any]],
    operations: Sequence[tuple[int, str, int]],
    initiator: str,
    context: str,
) -> None:
    source_control = _validate_control_event_shapes(
        source, SOURCE_CONTROL_FIELDS, f"{context} SOURCE"
    )
    destination_control = _validate_control_event_shapes(
        destination, DESTINATION_CONTROL_FIELDS, f"{context} DESTINATION"
    )
    if len(source_control) != len(operations) * len(SOURCE_CONTROL_FIELDS):
        raise AnalysisError(f"{context} SOURCE: control-lane event inventory mismatch")
    add_count = sum(action == "ADD" for _epoch, action, _target in operations)
    expected_destination_count = len(operations) * 3 + add_count
    if initiator == "source_init":
        expected_destination_count += add_count * 2
    if len(destination_control) != expected_destination_count:
        raise AnalysisError(f"{context} DESTINATION: control-lane event inventory mismatch")

    live = EXPECTED_FLOOR
    for epoch, action, target in operations:
        source_events = {
            name: _one_epoch_event(source_control, name, epoch, f"{context} SOURCE")
            for name in SOURCE_CONTROL_FIELDS
        }
        for name in ("resize_proposed", "resize_send_begin", "resize_sent"):
            event = source_events[name]
            if (
                event["action"] != action
                or event["target_streams"] != target
                or event["live_streams"] != live
            ):
                raise AnalysisError(f"{context} SOURCE: {name} values disagree with epoch {epoch}")
        ack = source_events["resize_ack_received"]
        if ack["live_streams"] != target or ack["accepted"] is not True:
            raise AnalysisError(f"{context} SOURCE: resize_ack_received values disagree")
        settled = source_events["source_settled"]
        if (
            settled["action"] != action
            or settled["target_streams"] != target
            or settled["live_streams"] != target
            or settled["accepted"] is not True
        ):
            raise AnalysisError(f"{context} SOURCE: source_settled values disagree")
        pending = _one_epoch_event(source, "dial_pending", epoch, f"{context} SOURCE")
        dial_settlement = _one_epoch_event(
            source, "dial_settlement", epoch, f"{context} SOURCE"
        )
        source_order = [
            pending,
            source_events["resize_proposed"],
            source_events["resize_send_begin"],
            source_events["resize_sent"],
            ack,
            dial_settlement,
            settled,
        ]
        if [event["producer_seq"] for event in source_order] != sorted(
            event["producer_seq"] for event in source_order
        ):
            raise AnalysisError(f"{context} SOURCE: control ordering is invalid for epoch {epoch}")

        received = _one_epoch_event(
            destination_control, "resize_received", epoch, f"{context} DESTINATION"
        )
        if received["target_streams"] != target or received["live_streams"] != live:
            raise AnalysisError(f"{context} DESTINATION: resize_received values disagree")
        ack_begin = _one_epoch_event(
            destination_control, "resize_ack_send_begin", epoch, f"{context} DESTINATION"
        )
        ack_sent = _one_epoch_event(
            destination_control, "resize_ack_sent", epoch, f"{context} DESTINATION"
        )
        destination_action = "add" if action == "ADD" else "logical_remove"
        for event in (ack_begin, ack_sent):
            if (
                event["action"] != destination_action
                or event["live_streams"] != target
                or event["accepted"] is not True
            ):
                raise AnalysisError(f"{context} DESTINATION: resize ACK values disagree")
        if not (
            received["producer_seq"]
            < ack_begin["producer_seq"]
            < ack_sent["producer_seq"]
        ):
            raise AnalysisError(f"{context} DESTINATION: ACK ordering is invalid")

        if action == "ADD":
            prepared = _one_epoch_event(
                destination_control, "destination_prepared", epoch, f"{context} DESTINATION"
            )
            if (
                prepared["action"] != "add_prepared"
                or prepared["target_streams"] != target
                or prepared["accepted"] is not True
                or not (
                    received["producer_seq"]
                    < prepared["producer_seq"]
                    < ack_begin["producer_seq"]
                )
            ):
                raise AnalysisError(f"{context} DESTINATION: ADD preparation is invalid")
            source_begin = _one_epoch_event(
                source_sockets,
                "socket_dial_begin" if initiator == "source_init" else "socket_accept_begin",
                epoch,
                f"{context} SOURCE",
            )
            source_end = _one_epoch_event(
                source_sockets,
                "socket_dial_end" if initiator == "source_init" else "socket_accept_end",
                epoch,
                f"{context} SOURCE",
            )
            source_attached = _one_epoch_event(
                source_sockets, "socket_trace_attached", epoch, f"{context} SOURCE"
            )
            if not (
                ack["producer_seq"]
                < source_begin["producer_seq"]
                < source_end["producer_seq"]
                < source_attached["producer_seq"]
                < dial_settlement["producer_seq"]
            ):
                raise AnalysisError(f"{context} SOURCE: ADD socket/control ordering is invalid")
            destination_begin_name = (
                "socket_accept_begin" if initiator == "source_init" else "socket_dial_begin"
            )
            destination_end_name = (
                "socket_accept_end" if initiator == "source_init" else "socket_dial_end"
            )
            destination_begin = _one_epoch_event(
                destination_sockets, destination_begin_name, epoch, f"{context} DESTINATION"
            )
            destination_end = _one_epoch_event(
                destination_sockets, destination_end_name, epoch, f"{context} DESTINATION"
            )
            destination_attached = _one_epoch_event(
                destination_sockets,
                "socket_trace_attached",
                epoch,
                f"{context} DESTINATION",
            )
            if not (
                destination_begin["producer_seq"]
                < destination_end["producer_seq"]
                < destination_attached["producer_seq"]
            ):
                raise AnalysisError(f"{context} DESTINATION: ADD socket ordering is invalid")
            if initiator == "source_init":
                arm_queue = _one_epoch_event(
                    destination_control,
                    "resize_arm_queue_begin",
                    epoch,
                    f"{context} DESTINATION",
                )
                arm_ready = _one_epoch_event(
                    destination_control,
                    "resize_arm_ready",
                    epoch,
                    f"{context} DESTINATION",
                )
                if (
                    arm_queue["target_streams"] != target
                    or not (
                        received["producer_seq"]
                        < arm_queue["producer_seq"]
                        < arm_ready["producer_seq"]
                        < destination_begin["producer_seq"]
                    )
                ):
                    raise AnalysisError(f"{context} DESTINATION: responder arm ordering is invalid")
            elif not (
                received["producer_seq"]
                < destination_begin["producer_seq"]
                < destination_end["producer_seq"]
                < destination_attached["producer_seq"]
                < prepared["producer_seq"]
                < ack_begin["producer_seq"]
                < ack_sent["producer_seq"]
            ):
                raise AnalysisError(
                    f"{context} DESTINATION: initiator ADD ordering is invalid"
                )
            if initiator != "source_init" and any(
                event["event"] in {"resize_arm_queue_begin", "resize_arm_ready"}
                and event["epoch"] == epoch
                for event in destination_control
            ):
                raise AnalysisError(f"{context} DESTINATION: initiator emitted responder arm events")
        else:
            if any(
                event["event"]
                in {"destination_prepared", "resize_arm_queue_begin", "resize_arm_ready"}
                and event["epoch"] == epoch
                for event in destination_control
            ):
                raise AnalysisError(f"{context} DESTINATION: REMOVE emitted ADD-only events")
        live = target


def _validate_no_sample_arm(
    source: Sequence[dict[str, Any]],
    destination: Sequence[dict[str, Any]],
    source_sockets: Sequence[dict[str, Any]],
    membership_sealed: dict[str, Any],
    context: str,
) -> tuple[str, ...]:
    source_attachments = [
        event
        for event in source_sockets
        if event["event"] == "socket_trace_attached" and event["epoch"] == 0
    ]
    if len(source_attachments) != EXPECTED_FLOOR or {
        event["socket"] for event in source_attachments
    } != set(range(EXPECTED_FLOOR)):
        raise AnalysisError(
            f"{context} SOURCE: zero-sample arm lacks four epoch-0 trace attachments"
        )
    if any(
        event["event"].startswith("resize_")
        or event["event"] in {"source_settled", "destination_prepared"}
        for event in (*source, *destination)
    ):
        raise AnalysisError(f"{context}: zero-sample arm emitted resize control events")
    latest_attachment_seq = max(event["producer_seq"] for event in source_attachments)
    if latest_attachment_seq >= membership_sealed["producer_seq"]:
        raise AnalysisError(
            f"{context} SOURCE: zero-sample membership seal precedes socket trace setup"
        )
    latest_attachment_ns = max(event["elapsed_ns"] for event in source_attachments)
    phase_span_ns = membership_sealed["elapsed_ns"] - latest_attachment_ns
    if phase_span_ns < 0:
        raise AnalysisError(
            f"{context} SOURCE: zero-sample membership seal precedes trace attachment time"
        )
    if phase_span_ns >= DIAL_TUNER_TICK_NS:
        return ("NO_SAMPLE_AT_OR_AFTER_FIRST_TICK",)
    return ()


def _validate_sample_event_timing(
    source_sockets: Sequence[dict[str, Any]],
    samples: Sequence[dict[str, Any]],
    context: str,
) -> None:
    if not samples:
        return
    source_attachments = [
        event
        for event in source_sockets
        if event["event"] == "socket_trace_attached" and event["epoch"] == 0
    ]
    if len(source_attachments) != EXPECTED_FLOOR:
        raise AnalysisError(
            f"{context} SOURCE: sampled arm lacks four epoch-0 trace attachments"
        )
    latest_attachment_seq = max(event["producer_seq"] for event in source_attachments)
    latest_attachment_ns = max(event["elapsed_ns"] for event in source_attachments)
    first = samples[0]
    if (
        first["producer_seq"] <= latest_attachment_seq
        or first["elapsed_ns"] - latest_attachment_ns < DIAL_TUNER_TICK_NS
    ):
        raise AnalysisError(
            f"{context} SOURCE: first dial sample occurred before one full tuner tick"
        )
    for previous, current in zip(samples, samples[1:]):
        if current["elapsed_ns"] - previous["elapsed_ns"] < DIAL_TUNER_TICK_NS:
            raise AnalysisError(
                f"{context} SOURCE: consecutive dial samples are less than one tuner tick apart"
            )


def _validate_arm(
    root: Path, row: RunRow, trace_cache: dict[str, list[dict[str, Any]]]
) -> ArmResult:
    source = _session_events(root, row.source_trace, row, "SOURCE", trace_cache)
    destination = _session_events(
        root, row.destination_trace, row, "DESTINATION", trace_cache
    )
    context = f"{row.cell} pair {row.pair} {row.initiator}"
    if any(event["event"].startswith("dial_") for event in destination):
        raise AnalysisError(f"{context}: DESTINATION emitted dial policy events")
    source_sockets = _validated_socket_events(source, f"{context} SOURCE")
    destination_sockets = _validated_socket_events(
        destination, f"{context} DESTINATION"
    )
    if row.initiator == "source_init":
        source_transport = "dial"
        destination_transport = "accept"
    else:
        source_transport = "accept"
        destination_transport = "dial"
    _validate_epoch0_sockets(source_sockets, source_transport, f"{context} SOURCE")
    _validate_epoch0_sockets(
        destination_sockets, destination_transport, f"{context} DESTINATION"
    )
    dial = _validate_dial(source, f"{context} SOURCE")
    _validate_sample_event_timing(
        source_sockets,
        dial.samples,
        context,
    )
    _validate_resize_sockets(
        source_sockets,
        source_transport,
        dial.operations,
        f"{context} SOURCE",
    )
    _validate_resize_sockets(
        destination_sockets,
        destination_transport,
        dial.operations,
        f"{context} DESTINATION",
    )
    source_complete = _completion(source, f"{context} SOURCE")
    destination_complete = _completion(destination, f"{context} DESTINATION")
    _validate_control_lane(
        source,
        destination,
        source_sockets,
        destination_sockets,
        dial.operations,
        row.initiator,
        context,
    )
    _validate_membership_observation(
        source,
        destination,
        source_sockets,
        destination_sockets,
        dial.operations,
        destination_complete,
        context,
    )
    _validate_payload_socket_observation(
        source,
        destination,
        source_sockets,
        destination_sockets,
        dial.operations,
        source_complete,
        destination_complete,
        context,
    )
    if any(
        event["producer_seq"] >= source_complete["producer_seq"]
        for event in source
        if event["event"].startswith("dial_")
    ):
        raise AnalysisError(f"{context}: dial observation occurred after SOURCE completion")
    source_counts = (
        source_complete["live_streams"],
        source_complete["peak_streams"],
        source_complete["receiver_ceiling"],
    )
    destination_counts = (
        destination_complete["live_streams"],
        destination_complete["peak_streams"],
        destination_complete["receiver_ceiling"],
    )
    if source_counts != destination_counts:
        raise AnalysisError(f"{context}: SOURCE/DESTINATION final membership differs")
    if source_counts != (dial.final, dial.peak, dial.ceiling):
        raise AnalysisError(f"{context}: completion membership differs from dial lifecycle")
    sealed = _membership_sealed(
        source,
        source_complete,
        dial,
        f"{context} SOURCE",
    )
    review_reasons: tuple[str, ...] = ()
    if not dial.samples:
        review_reasons = _validate_no_sample_arm(
            source,
            destination,
            source_sockets,
            sealed,
            context,
        )
    client_complete = (
        source_complete if row.initiator == "source_init" else destination_complete
    )
    client_complete_ms = Decimal(client_complete["elapsed_ns"]) / Decimal(1_000_000)
    if client_complete_ms > row.duration_ms + DURATION_ROUNDING_ALLOWANCE_MS:
        raise AnalysisError(
            f"{context}: client data_plane_complete at {_decimal_text(client_complete_ms)}ms "
            f"exceeds runs.csv duration_ms {_decimal_text(row.duration_ms)} by more than "
            f"{_decimal_text(DURATION_ROUNDING_ALLOWANCE_MS)}ms"
        )
    return ArmResult(
        row,
        dial,
        source_complete["elapsed_ns"],
        destination_complete["elapsed_ns"],
        review_reasons,
    )


def _decimal_text(value: Decimal) -> str:
    rendered = format(value, "f")
    if "." in rendered:
        rendered = rendered.rstrip("0").rstrip(".")
    return rendered or "0"


def _counter_json(counter: Counter[str]) -> str:
    return json.dumps(dict(sorted(counter.items())), separators=(",", ":"), sort_keys=True)


def _role_differences(left: ArmResult, right: ArmResult) -> list[str]:
    differences: list[str] = []
    for name in ("peak", "final", "add_count", "remove_count"):
        if getattr(left.dial, name) != getattr(right.dial, name):
            differences.append(name)
    if left.dial.reasons != right.dial.reasons:
        differences.append("reason_distribution")
    if tuple(sample["reason"] for sample in left.dial.samples) != tuple(
        sample["reason"] for sample in right.dial.samples
    ):
        differences.append("reason_sequence")
    if left.dial.operations != right.dial.operations:
        differences.append("operation_sequence")
    return differences


def _build_reports(
    session_dir: Path,
    arms: Sequence[ArmResult],
    fixtures: dict[tuple[str, str], Manifest],
    staging: Sequence[dict[str, str]],
    harness_sha: str,
) -> tuple[dict[str, Any], list[dict[str, str]], list[dict[str, str]], list[dict[str, str]], str]:
    arm_by_key = {(arm.row.cell, arm.row.pair, arm.row.initiator): arm for arm in arms}
    arm_rows: list[dict[str, str]] = []
    sample_rows: list[dict[str, str]] = []
    for arm in arms:
        source_endpoint = "q" if arm.row.direction == "q_to_windows" else "netwatch-01"
        destination_endpoint = (
            "netwatch-01" if arm.row.direction == "q_to_windows" else "q"
        )
        client_endpoint = (
            source_endpoint if arm.row.initiator == "source_init" else destination_endpoint
        )
        client_complete_elapsed_ns = (
            arm.source_complete_elapsed_ns
            if arm.row.initiator == "source_init"
            else arm.destination_complete_elapsed_ns
        )
        throughput = (Decimal(arm.row.bytes) / Decimal(1024 * 1024)) / (
            arm.row.duration_ms / Decimal(1000)
        )
        arm_rows.append(
            {
                "cell": arm.row.cell,
                "pair": str(arm.row.pair),
                "initiator": arm.row.initiator,
                "run_id": arm.row.run_id,
                "session_id": arm.row.session_id,
                "source_endpoint": source_endpoint,
                "destination_endpoint": destination_endpoint,
                "client_endpoint": client_endpoint,
                "duration_ms": _decimal_text(arm.row.duration_ms),
                "throughput_mib_s": _decimal_text(throughput.quantize(Decimal("0.001"))),
                "floor_streams": str(arm.dial.floor),
                "peak_streams": str(arm.dial.peak),
                "final_streams": str(arm.dial.final),
                "receiver_ceiling": str(arm.dial.ceiling),
                "accepted_adds": str(arm.dial.add_count),
                "accepted_removes": str(arm.dial.remove_count),
                "sample_count": str(len(arm.dial.samples)),
                "sample_observation": (
                    "sampled" if arm.dial.samples else "no-sample"
                ),
                "operations": json.dumps(
                    [
                        {"epoch": epoch, "action": action, "target_streams": target}
                        for epoch, action, target in arm.dial.operations
                    ],
                    separators=(",", ":"),
                ),
                "reason_counts": _counter_json(arm.dial.reasons),
                "reason_sequence": json.dumps(
                    [sample["reason"] for sample in arm.dial.samples],
                    separators=(",", ":"),
                ),
                "source_complete_elapsed_ns": str(arm.source_complete_elapsed_ns),
                "destination_complete_elapsed_ns": str(arm.destination_complete_elapsed_ns),
                "client_complete_elapsed_ns": str(client_complete_elapsed_ns),
                "review_reasons": json.dumps(
                    arm.review_reasons, separators=(",", ":")
                ),
                "arm_verdict": (
                    "REVIEW_REQUIRED" if arm.review_reasons else "STRUCTURALLY_VALID"
                ),
                "source_path": arm.row.source_path,
                "active_destination_path": arm.row.active_destination_path,
                "archive_path": arm.row.archive_path,
                "landed_manifest": arm.row.landed_manifest,
                "landed_manifest_sha256": hashlib.sha256(
                    (session_dir / arm.row.landed_manifest).read_bytes()
                ).hexdigest(),
                "source_trace": arm.row.source_trace,
                "destination_trace": arm.row.destination_trace,
            }
        )
        for sample in arm.dial.samples:
            sample_rows.append(
                {
                    "cell": arm.row.cell,
                    "pair": str(arm.row.pair),
                    "initiator": arm.row.initiator,
                    "run_id": arm.row.run_id,
                    **{key: str(value).lower() if isinstance(value, bool) else str(value) for key, value in sample.items()},
                }
            )

    pair_rows: list[dict[str, str]] = []
    decision_review_count = 0
    cell_summaries: list[dict[str, Any]] = []
    performance_review_count = 0
    for cell in _expected_cells():
        source_durations: list[Decimal] = []
        destination_durations: list[Decimal] = []
        cell_decision_reviews = 0
        for pair in PAIRS:
            source = arm_by_key[(cell, pair, "source_init")]
            destination = arm_by_key[(cell, pair, "destination_init")]
            source_durations.append(source.row.duration_ms)
            destination_durations.append(destination.row.duration_ms)
            differences = _role_differences(source, destination)
            review = bool(differences)
            decision_review_count += int(review)
            cell_decision_reviews += int(review)
            pair_rows.append(
                {
                    "cell": cell,
                    "pair": str(pair),
                    "source_init_ms": _decimal_text(source.row.duration_ms),
                    "destination_init_ms": _decimal_text(destination.row.duration_ms),
                    "destination_minus_source_ms": _decimal_text(
                        destination.row.duration_ms - source.row.duration_ms
                    ),
                    "slower_faster_ratio": _decimal_text(
                        max(source.row.duration_ms, destination.row.duration_ms)
                        / min(source.row.duration_ms, destination.row.duration_ms)
                    ),
                    "source_peak": str(source.dial.peak),
                    "destination_peak": str(destination.dial.peak),
                    "source_final": str(source.dial.final),
                    "destination_final": str(destination.dial.final),
                    "source_adds": str(source.dial.add_count),
                    "destination_adds": str(destination.dial.add_count),
                    "source_removes": str(source.dial.remove_count),
                    "destination_removes": str(destination.dial.remove_count),
                    "source_operations": json.dumps(source.dial.operations, separators=(",", ":")),
                    "destination_operations": json.dumps(
                        destination.dial.operations, separators=(",", ":")
                    ),
                    "source_reasons": _counter_json(source.dial.reasons),
                    "destination_reasons": _counter_json(destination.dial.reasons),
                    "source_reason_sequence": json.dumps(
                        [sample["reason"] for sample in source.dial.samples],
                        separators=(",", ":"),
                    ),
                    "destination_reason_sequence": json.dumps(
                        [sample["reason"] for sample in destination.dial.samples],
                        separators=(",", ":"),
                    ),
                    "decision_differences": ";".join(differences),
                    "decision_verdict": "REVIEW_REQUIRED" if review else "MATCH",
                }
            )
        source_median = median(source_durations)
        destination_median = median(destination_durations)
        ratio = max(source_median, destination_median) / min(source_median, destination_median)
        paired_deltas = [
            destination - source
            for source, destination in zip(source_durations, destination_durations)
        ]
        first_half_delta = median(paired_deltas[:4])
        last_half_delta = median(paired_deltas[4:])
        source_first_deltas = [
            paired_deltas[index]
            for index, role in enumerate(FIRST_ROLE)
            if role == "source_init"
        ]
        destination_first_deltas = [
            paired_deltas[index]
            for index, role in enumerate(FIRST_ROLE)
            if role == "destination_init"
        ]
        source_first_delta = median(source_first_deltas)
        destination_first_delta = median(destination_first_deltas)
        performance_review = ratio > PERFORMANCE_RATIO_LIMIT
        performance_review_count += int(performance_review)
        cell_summaries.append(
            {
                "cell": cell,
                "source_init_median_ms": _decimal_text(source_median),
                "destination_init_median_ms": _decimal_text(destination_median),
                "source_init_durations_ms": [_decimal_text(value) for value in source_durations],
                "destination_init_durations_ms": [
                    _decimal_text(value) for value in destination_durations
                ],
                "paired_destination_minus_source_ms": [
                    _decimal_text(value) for value in paired_deltas
                ],
                "paired_delta_median_ms": _decimal_text(median(paired_deltas)),
                "first_half_delta_median_ms": _decimal_text(first_half_delta),
                "last_half_delta_median_ms": _decimal_text(last_half_delta),
                "first_last_drift_ms": _decimal_text(last_half_delta - first_half_delta),
                "source_first_delta_median_ms": _decimal_text(source_first_delta),
                "destination_first_delta_median_ms": _decimal_text(destination_first_delta),
                "role_order_drift_ms": _decimal_text(
                    destination_first_delta - source_first_delta
                ),
                "slower_faster_median_ratio": _decimal_text(ratio),
                "performance_limit": _decimal_text(PERFORMANCE_RATIO_LIMIT),
                "performance_verdict": "REVIEW_REQUIRED" if performance_review else "WITHIN_1.10",
                "decision_review_pairs": cell_decision_reviews,
                "decision_verdict": "REVIEW_REQUIRED" if cell_decision_reviews else "MATCH",
            }
        )

    arm_review_count = sum(bool(arm.review_reasons) for arm in arms)
    status = (
        "REVIEW_REQUIRED"
        if arm_review_count or decision_review_count or performance_review_count
        else "STRUCTURALLY_VALID_NO_ROLE_SKEW"
    )
    summary: dict[str, Any] = {
        "schema": 1,
        "status": status,
        "artifact_sha": ARTIFACT_SHA,
        "harness_sha": harness_sha,
        "arm_count": len(arms),
        "no_sample_arm_count": sum(not arm.dial.samples for arm in arms),
        "arm_review_count": arm_review_count,
        "pair_count": len(_expected_cells()) * len(PAIRS),
        "floor_streams_observed": EXPECTED_FLOOR,
        "receiver_safety_ceiling_observed": EXPECTED_CEILING,
        "preselected_worker_target": None,
        "decision_review_count": decision_review_count,
        "performance_review_count": performance_review_count,
        "performance_materiality_ratio": _decimal_text(PERFORMANCE_RATIO_LIMIT),
        "timing_rule": "endpoint-local elapsed_ns only; no cross-host clock subtraction",
        "staged_binaries": list(staging),
        "fixture_manifests": [
            {
                "direction": direction,
                "fixture": fixture,
                "path": fixtures[(direction, fixture)].relative_path,
                "sha256": fixtures[(direction, fixture)].file_sha256,
                "files": fixtures[(direction, fixture)].files,
                "bytes": fixtures[(direction, fixture)].bytes,
            }
            for direction in DIRECTIONS
            for fixture in FIXTURES
        ],
        "cells": cell_summaries,
    }
    lines = [
        "# ldt-4 rig-W adaptive evidence",
        "",
        f"Status: **{status}**",
        "",
        f"Validated {len(arms)} arms in {len(_expected_cells())} cells and {len(PAIRS)} paired repetitions.",
        "No worker target was selected or graded; observed floor, peak, final, operations, reasons, and raw samples are retained.",
        "All interval fields are endpoint-local. No timestamp from q was subtracted from a netwatch-01 timestamp.",
        "",
        "| Cell | source-init median ms | destination-init median ms | ratio | performance | decision pairs |",
        "|---|---:|---:|---:|---|---:|",
    ]
    for cell in cell_summaries:
        lines.append(
            f"| {cell['cell']} | {cell['source_init_median_ms']} | "
            f"{cell['destination_init_median_ms']} | {cell['slower_faster_median_ratio']} | "
            f"{cell['performance_verdict']} | {cell['decision_review_pairs']} |"
        )
    lines.extend(
        [
            "",
            "Any peak/final/operation/reason-distribution/reason-sequence difference is exported as REVIEW_REQUIRED; no undocumented decision threshold is applied.",
            "A structurally valid zero-sample arm whose membership remained open through the first tuner tick is explicitly REVIEW_REQUIRED.",
            "A median initiator-layout performance ratio above 1.10 is REVIEW_REQUIRED under the durable parent invariance bound.",
            "",
        ]
    )
    return summary, arm_rows, pair_rows, sample_rows, "\n".join(lines)


def _build_sustained_reports(
    session_dir: Path,
    arms: Sequence[ArmResult],
    fixtures: dict[tuple[str, str], Manifest],
    staging: Sequence[dict[str, str]],
    harness_sha: str,
    matrix: str,
) -> tuple[dict[str, Any], list[dict[str, str]], list[dict[str, str]], list[dict[str, str]], str]:
    if matrix not in {"sustained", "horizon"}:
        raise AnalysisError(f"unregistered controller-supplement matrix {matrix!r}")
    fixture = _matrix_fixtures(matrix)[0]
    cells = _matrix_cells(matrix)
    accepted_arm_verdict = (
        "SUSTAINED_ADD_ACCEPTED" if matrix == "sustained" else "HORIZON_ADD_ACCEPTED"
    )
    accepted_status = (
        "STRUCTURALLY_VALID_SUSTAINED_ROLE_PARITY"
        if matrix == "sustained"
        else "STRUCTURALLY_VALID_HORIZON_ROLE_PARITY"
    )
    arm_by_key = {(arm.row.cell, arm.row.initiator): arm for arm in arms}
    arm_rows: list[dict[str, str]] = []
    sample_rows: list[dict[str, str]] = []
    requirement_reasons: dict[tuple[str, str], tuple[str, ...]] = {}
    for arm in arms:
        source_endpoint = "q" if arm.row.direction == "q_to_windows" else "netwatch-01"
        destination_endpoint = (
            "netwatch-01" if arm.row.direction == "q_to_windows" else "q"
        )
        client_endpoint = (
            source_endpoint if arm.row.initiator == "source_init" else destination_endpoint
        )
        client_complete_elapsed_ns = (
            arm.source_complete_elapsed_ns
            if arm.row.initiator == "source_init"
            else arm.destination_complete_elapsed_ns
        )
        throughput = (Decimal(arm.row.bytes) / Decimal(1024 * 1024)) / (
            arm.row.duration_ms / Decimal(1000)
        )
        reasons = list(arm.review_reasons)
        accepted_add = any(
            action == "ADD" and target > EXPECTED_FLOOR
            for _, action, target in arm.dial.operations
        )
        if not accepted_add or arm.dial.add_count < 1 or arm.dial.peak <= EXPECTED_FLOOR:
            reasons.append("NO_ACCEPTED_ADD_ABOVE_FLOOR")
        requirement_reasons[(arm.row.cell, arm.row.initiator)] = tuple(reasons)
        arm_rows.append(
            {
                "cell": arm.row.cell,
                "pair": str(arm.row.pair),
                "initiator": arm.row.initiator,
                "run_id": arm.row.run_id,
                "session_id": arm.row.session_id,
                "source_endpoint": source_endpoint,
                "destination_endpoint": destination_endpoint,
                "client_endpoint": client_endpoint,
                "duration_ms": _decimal_text(arm.row.duration_ms),
                "throughput_mib_s": _decimal_text(throughput.quantize(Decimal("0.001"))),
                "floor_streams": str(arm.dial.floor),
                "peak_streams": str(arm.dial.peak),
                "final_streams": str(arm.dial.final),
                "receiver_ceiling": str(arm.dial.ceiling),
                "accepted_adds": str(arm.dial.add_count),
                "accepted_removes": str(arm.dial.remove_count),
                "sample_count": str(len(arm.dial.samples)),
                "sample_observation": "sampled" if arm.dial.samples else "no-sample",
                "operations": json.dumps(
                    [
                        {"epoch": epoch, "action": action, "target_streams": target}
                        for epoch, action, target in arm.dial.operations
                    ],
                    separators=(",", ":"),
                ),
                "reason_counts": _counter_json(arm.dial.reasons),
                "reason_sequence": json.dumps(
                    [sample["reason"] for sample in arm.dial.samples],
                    separators=(",", ":"),
                ),
                "source_complete_elapsed_ns": str(arm.source_complete_elapsed_ns),
                "destination_complete_elapsed_ns": str(
                    arm.destination_complete_elapsed_ns
                ),
                "client_complete_elapsed_ns": str(client_complete_elapsed_ns),
                "review_reasons": json.dumps(tuple(reasons), separators=(",", ":")),
                "arm_verdict": "REVIEW_REQUIRED" if reasons else accepted_arm_verdict,
                "source_path": arm.row.source_path,
                "active_destination_path": arm.row.active_destination_path,
                "archive_path": arm.row.archive_path,
                "landed_manifest": arm.row.landed_manifest,
                "landed_manifest_sha256": hashlib.sha256(
                    (session_dir / arm.row.landed_manifest).read_bytes()
                ).hexdigest(),
                "source_trace": arm.row.source_trace,
                "destination_trace": arm.row.destination_trace,
            }
        )
        for sample in arm.dial.samples:
            sample_rows.append(
                {
                    "cell": arm.row.cell,
                    "pair": str(arm.row.pair),
                    "initiator": arm.row.initiator,
                    "run_id": arm.row.run_id,
                    **{
                        key: str(value).lower() if isinstance(value, bool) else str(value)
                        for key, value in sample.items()
                    },
                }
            )

    pair_rows: list[dict[str, str]] = []
    cell_summaries: list[dict[str, Any]] = []
    decision_review_count = 0
    trace_timing_difference_count = 0
    material_fields = {"peak", "final", "add_count", "remove_count", "operation_sequence"}
    for cell in cells:
        source = arm_by_key[(cell, "source_init")]
        destination = arm_by_key[(cell, "destination_init")]
        differences = _role_differences(source, destination)
        material = [item for item in differences if item in material_fields]
        timing = [item for item in differences if item not in material_fields]
        decision_review_count += int(bool(material))
        trace_timing_difference_count += int(bool(timing))
        ratio = max(source.row.duration_ms, destination.row.duration_ms) / min(
            source.row.duration_ms, destination.row.duration_ms
        )
        pair_rows.append(
            {
                "cell": cell,
                "pair": "1",
                "source_init_ms": _decimal_text(source.row.duration_ms),
                "destination_init_ms": _decimal_text(destination.row.duration_ms),
                "destination_minus_source_ms": _decimal_text(
                    destination.row.duration_ms - source.row.duration_ms
                ),
                "slower_faster_ratio": _decimal_text(ratio),
                "source_peak": str(source.dial.peak),
                "destination_peak": str(destination.dial.peak),
                "source_final": str(source.dial.final),
                "destination_final": str(destination.dial.final),
                "source_adds": str(source.dial.add_count),
                "destination_adds": str(destination.dial.add_count),
                "source_removes": str(source.dial.remove_count),
                "destination_removes": str(destination.dial.remove_count),
                "source_operations": json.dumps(source.dial.operations, separators=(",", ":")),
                "destination_operations": json.dumps(
                    destination.dial.operations, separators=(",", ":")
                ),
                "source_reasons": _counter_json(source.dial.reasons),
                "destination_reasons": _counter_json(destination.dial.reasons),
                "source_reason_sequence": json.dumps(
                    [sample["reason"] for sample in source.dial.samples],
                    separators=(",", ":"),
                ),
                "destination_reason_sequence": json.dumps(
                    [sample["reason"] for sample in destination.dial.samples],
                    separators=(",", ":"),
                ),
                "decision_differences": ";".join(material),
                "trace_timing_differences": ";".join(timing),
                "decision_verdict": "REVIEW_REQUIRED" if material else "TRANSITIONS_MATCH",
            }
        )
        cell_summaries.append(
            {
                "cell": cell,
                "source_init_ms": _decimal_text(source.row.duration_ms),
                "destination_init_ms": _decimal_text(destination.row.duration_ms),
                "source_operations": [list(operation) for operation in source.dial.operations],
                "destination_operations": [
                    list(operation) for operation in destination.dial.operations
                ],
                "decision_differences": material,
                "trace_timing_differences": timing,
                "verdict": "REVIEW_REQUIRED" if material else "TRANSITIONS_MATCH",
            }
        )

    arm_review_count = sum(bool(reasons) for reasons in requirement_reasons.values())
    status = (
        "REVIEW_REQUIRED"
        if arm_review_count or decision_review_count
        else accepted_status
    )
    summary: dict[str, Any] = {
        "schema": 1,
        "matrix": matrix,
        "status": status,
        "artifact_sha": ARTIFACT_SHA,
        "harness_sha": harness_sha,
        "parent_session": PARENT_SESSION,
        "parent_evidence": PARENT_EVIDENCE,
        "parent_inventory_sha256": PARENT_INVENTORY_SHA256,
        "arm_count": len(arms),
        "no_sample_arm_count": sum(not arm.dial.samples for arm in arms),
        "arm_review_count": arm_review_count,
        "pair_count": len(cells),
        "floor_streams_observed": EXPECTED_FLOOR,
        "receiver_safety_ceiling_observed": EXPECTED_CEILING,
        "preselected_worker_target": None,
        "decision_review_count": decision_review_count,
        "trace_timing_difference_count": trace_timing_difference_count,
        "performance_review_count": 0,
        "performance_materiality_ratio": None,
        "timing_rule": "endpoint-local elapsed_ns only; no cross-host clock subtraction",
        "staged_binaries": list(staging),
        "fixture_manifests": [
            {
                "direction": direction,
                "fixture": fixture,
                "path": fixtures[(direction, fixture)].relative_path,
                "sha256": fixtures[(direction, fixture)].file_sha256,
                "files": fixtures[(direction, fixture)].files,
                "bytes": fixtures[(direction, fixture)].bytes,
            }
            for direction in DIRECTIONS
        ],
        "cells": cell_summaries,
    }
    if matrix == "horizon":
        summary.update(
            {
                "predecessor_session": PREDECESSOR_SESSION,
                "predecessor_evidence": PREDECESSOR_EVIDENCE,
                "predecessor_inventory_sha256": PREDECESSOR_INVENTORY_SHA256,
            }
        )
    title = (
        "# ldt-4 rig-W sustained controller supplement"
        if matrix == "sustained"
        else "# ldt-4 rig-W admission-horizon controller supplement"
    )
    lines = [
        title,
        "",
        f"Status: **{status}**",
        "",
        f"Validated {len(arms)} {matrix} arms in two physical byte directions.",
        f"Parent fixed-matrix evidence: `{PARENT_EVIDENCE}` ({PARENT_INVENTORY_SHA256}).",
    ]
    if matrix == "horizon":
        lines.append(
            f"Predecessor sustained evidence: `{PREDECESSOR_EVIDENCE}` ({PREDECESSOR_INVENTORY_SHA256})."
        )
    lines.extend(
        [
            "Every arm must accept an ADD above the four-stream floor; accepted transition sequences must match within each initiator-layout pair.",
            "Reason-only trailing sample differences are exported separately and do not override matching accepted membership transitions.",
            "",
            "| Cell | source-init ms | destination-init ms | source operations | destination operations | verdict |",
            "|---|---:|---:|---|---|---|",
        ]
    )
    for cell in cell_summaries:
        lines.append(
            f"| {cell['cell']} | {cell['source_init_ms']} | {cell['destination_init_ms']} | "
            f"`{json.dumps(cell['source_operations'], separators=(',', ':'))}` | "
            f"`{json.dumps(cell['destination_operations'], separators=(',', ':'))}` | {cell['verdict']} |"
        )
    lines.append("")
    return summary, arm_rows, pair_rows, sample_rows, "\n".join(lines)


def _load_provenance(session_dir: Path, expected_harness_sha: str) -> str:
    path = session_dir / "provenance.csv"
    rows = _read_csv(path, ("name", "sha"), "provenance.csv")
    values: dict[str, str] = {}
    for line, row in enumerate(rows, start=2):
        if row["name"] in values or row["name"] not in {"artifact", "harness"}:
            raise AnalysisError(f"provenance.csv line {line}: unexpected/duplicate name")
        if not FULL_SHA_RE.fullmatch(row["sha"]):
            raise AnalysisError(f"provenance.csv line {line}: SHA must be full lowercase hex")
        values[row["name"]] = row["sha"]
    if values.get("artifact") != ARTIFACT_SHA or "harness" not in values:
        raise AnalysisError("provenance.csv does not identify the exact accepted artifact and harness")
    if values["harness"] == values["artifact"]:
        raise AnalysisError("artifact and harness SHAs must be independently identified")
    if values["harness"] != expected_harness_sha:
        raise AnalysisError("provenance.csv harness SHA differs from the expected reviewed harness")
    return values["harness"]


def _validate_expected_harness_sha(value: str) -> None:
    if not FULL_SHA_RE.fullmatch(value):
        raise AnalysisError("expected harness SHA must be full lowercase 40-hex")
    if value == ARTIFACT_SHA:
        raise AnalysisError("expected harness SHA must be distinct from the artifact SHA")


def _load_parent_evidence(session_dir: Path) -> None:
    expected = (
        f"session={PARENT_SESSION} evidence={PARENT_EVIDENCE} "
        f"inventory_sha256={PARENT_INVENTORY_SHA256}"
    )
    if _read_single_lf_line(
        session_dir / "parent-evidence.txt", "parent-evidence.txt"
    ) != expected:
        raise AnalysisError("parent-evidence.txt is not the exact valid 96-arm binding")


def _load_predecessor_evidence(session_dir: Path) -> None:
    expected = (
        f"session={PREDECESSOR_SESSION} evidence={PREDECESSOR_EVIDENCE} "
        f"inventory_sha256={PREDECESSOR_INVENTORY_SHA256}"
    )
    if _read_single_lf_line(
        session_dir / "predecessor-evidence.txt", "predecessor-evidence.txt"
    ) != expected:
        raise AnalysisError(
            "predecessor-evidence.txt is not the exact sustained evidence binding"
        )


def _validate_measurements_complete(
    session_dir: Path, expected_harness_sha: str, matrix: str
) -> None:
    path = session_dir / "MEASUREMENTS-COMPLETE"
    _plain_file(path, "MEASUREMENTS-COMPLETE")
    if matrix == "fixed":
        expected_text = (
            f"artifact_sha={ARTIFACT_SHA}\n"
            f"harness_sha={expected_harness_sha}\n"
            "arm_count=96\n"
        )
    elif matrix == "sustained":
        expected_text = (
            f"artifact_sha={ARTIFACT_SHA}\n"
            f"harness_sha={expected_harness_sha}\n"
            "matrix=sustained\n"
            "arm_count=4\n"
            f"parent_inventory_sha256={PARENT_INVENTORY_SHA256}\n"
        )
    elif matrix == "horizon":
        expected_text = (
            f"artifact_sha={ARTIFACT_SHA}\n"
            f"harness_sha={expected_harness_sha}\n"
            "matrix=horizon\n"
            "arm_count=4\n"
            f"parent_inventory_sha256={PARENT_INVENTORY_SHA256}\n"
            f"predecessor_inventory_sha256={PREDECESSOR_INVENTORY_SHA256}\n"
        )
    else:
        raise AnalysisError(f"unregistered matrix {matrix!r}")
    expected = expected_text.encode("ascii")
    if path.read_bytes() != expected:
        raise AnalysisError("MEASUREMENTS-COMPLETE content is not the exact registered binding")


def _write_csv_exclusive(path: Path, fields: Sequence[str], rows: Iterable[dict[str, str]]) -> None:
    with path.open("x", encoding="utf-8", newline="") as handle:
        handle.write(_csv_payload(fields, rows))


def analyze(
    session_dir: Path, expected_harness_sha: str, matrix: str = "fixed"
) -> AnalysisResult:
    session_dir = session_dir.absolute()
    _plain_directory(session_dir, "session directory")
    _validate_expected_harness_sha(expected_harness_sha)
    _validate_measurements_complete(session_dir, expected_harness_sha, matrix)
    if (session_dir / "SESSION-VOID").exists():
        raise AnalysisError("SESSION-VOID exists; refusing to analyze a void session")
    output_dir = session_dir / "analysis"
    if output_dir.exists() or output_dir.is_symlink():
        raise AnalysisError("analysis output already exists; immutable evidence is never overwritten")

    # Provenance is validated before large evidence files are trusted.
    harness_sha = _load_provenance(session_dir, expected_harness_sha)
    if matrix in {"sustained", "horizon"}:
        _load_parent_evidence(session_dir)
    if matrix == "horizon":
        _load_predecessor_evidence(session_dir)
    _load_artifact_build(session_dir)
    staging = _load_staging_manifest(session_dir)
    _load_windows_runtime_evidence(session_dir, staging)
    _load_schedule(session_dir, matrix)
    _load_environment_gate(session_dir, "start")
    _load_environment_gate(session_dir, "end")
    if matrix == "horizon":
        _load_payload_volume_gate(session_dir, "start")
        _load_payload_volume_gate(session_dir, "end")
    _load_runtime_gates(session_dir, matrix)
    manifest_cache: dict[str, Manifest] = {}
    fixtures = _load_fixture_index(session_dir, manifest_cache, matrix)
    rows = _load_runs(session_dir, fixtures, manifest_cache, matrix)
    trace_cache: dict[str, list[dict[str, Any]]] = {}
    arms = [_validate_arm(session_dir, row, trace_cache) for row in rows]
    if matrix == "fixed":
        summary, arm_rows, pair_rows, sample_rows, markdown = _build_reports(
            session_dir, arms, fixtures, staging, harness_sha
        )
    else:
        summary, arm_rows, pair_rows, sample_rows, markdown = _build_sustained_reports(
            session_dir, arms, fixtures, staging, harness_sha, matrix
        )
    inventory_rows, inventory_sha = _inventory_input_files(session_dir)
    summary["input_file_count"] = len(inventory_rows)
    summary["input_inventory_sha256"] = inventory_sha

    try:
        output_dir.mkdir()
    except FileExistsError as exc:
        raise AnalysisError("analysis output appeared concurrently; refusing overwrite") from exc
    _write_csv_exclusive(
        output_dir / "input-files.csv", INPUT_INVENTORY_FIELDS, inventory_rows
    )
    with (output_dir / "summary.json").open("x", encoding="utf-8") as handle:
        handle.write(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    with (output_dir / "summary.md").open("x", encoding="utf-8") as handle:
        handle.write(markdown)
    arm_fields = tuple(arm_rows[0])
    pair_fields = tuple(pair_rows[0])
    sample_fields = (
        "cell",
        "pair",
        "initiator",
        "run_id",
        "producer_seq",
        "elapsed_ns",
        "reason",
        "epoch",
        "live_streams",
        "peak_streams",
        "receiver_ceiling",
        "sample_bytes",
        "sample_blocked_ns",
        "sample_elapsed_ns",
        "sample_streams",
        "sample_valid",
        "blocked_ratio",
        "chunk_bytes",
        "prefetch_count",
        "tcp_buffer_bytes",
        "action",
        "target_streams",
    )
    normalized_samples = [
        {field: row.get(field, "") for field in sample_fields} for row in sample_rows
    ]
    _write_csv_exclusive(output_dir / "arms.csv", arm_fields, arm_rows)
    _write_csv_exclusive(output_dir / "pairs.csv", pair_fields, pair_rows)
    _write_csv_exclusive(output_dir / "dial-samples.csv", sample_fields, normalized_samples)
    return AnalysisResult(
        output_dir,
        summary["status"],
        len(arms),
        summary["arm_review_count"],
        summary["decision_review_count"],
        summary["performance_review_count"],
    )


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--session-dir", required=True, type=Path)
    parser.add_argument("--expected-harness-sha", required=True)
    parser.add_argument(
        "--matrix", choices=("fixed", "sustained", "horizon"), default="fixed"
    )
    args = parser.parse_args(argv)
    try:
        result = analyze(args.session_dir, args.expected_harness_sha, args.matrix)
    except (AnalysisError, OSError, UnicodeError) as exc:
        print(f"ldt-4 analysis refused: {exc}", file=sys.stderr)
        return 2
    print(
        f"ldt-4 analysis {result.status}: {result.arm_count} arms; "
        f"arm_review={result.arm_review_count}, "
        f"decision_review={result.decision_review_count}, "
        f"performance_review={result.performance_review_count}; {result.output_dir}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
