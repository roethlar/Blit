#!/usr/bin/env python3
"""Validate and analyze the registered otp-12 pf-1 rig-W session.

The analyzer is intentionally fail closed.  It accepts only the registered
four-block schedule and writes reports only after the CSV and every structured
TCP trace have passed validation.  Phase intervals are derived exclusively
from one endpoint's ``elapsed_ns`` clock; ``unix_ns`` is retained as evidence
but is never subtracted across hosts.
"""

from __future__ import annotations

import argparse
import base64
import binascii
import csv
import hashlib
import json
import os
import re
import sys
import tempfile
from dataclasses import dataclass
from decimal import Decimal, InvalidOperation
from pathlib import Path, PurePosixPath
from statistics import median
from typing import Any, Iterable, Sequence


CELLS = (
    "wm_tcp_mixed",
    "mw_tcp_mixed",
    "wm_grpc_mixed",
    "wm_tcp_large",
)
TCP_CELLS = frozenset(cell for cell in CELLS if "_tcp_" in cell)
TARGET_CELL = "wm_tcp_mixed"
ROLES = ("source_init", "destination_init")
CSV_FIELDS = (
    "block",
    "trace_state",
    "pass",
    "cell",
    "role",
    "pair",
    "role_order",
    "transfer_ms",
    "settled_ms",
    "flush_ms",
    "total_ms",
    "landed_root",
    "tree_manifest_sha256",
    "exit",
    "drain",
    "valid",
    "run_id",
    "session_id",
    "client_log",
)
CLOCK_FIELDS = (
    "block",
    "run_id",
    "cell",
    "pair",
    "role",
    "phase",
    "sample",
    "q_before_ns",
    "windows_ns",
    "q_after_ns",
    "rtt_ns",
    "offset_windows_minus_q_ns",
)
TRACE_PREFIX = "[session-phase] "
SESSION_ID_RE = re.compile(r"^[0-9a-f]{16}$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
SETTLE_MIN_MS = 250
SETTLE_MAX_MS = 1000
MEASURAND = "durable_total_ms"


@dataclass(frozen=True)
class BlockSpec:
    number: int
    trace_state: str
    pass_name: str
    pairs: range
    cells: tuple[str, ...]


BLOCKS = (
    BlockSpec(1, "off", "forward", range(1, 5), CELLS),
    BlockSpec(2, "on", "reverse", range(1, 5), tuple(reversed(CELLS))),
    BlockSpec(3, "on", "forward", range(5, 9), CELLS),
    BlockSpec(4, "off", "reverse", range(5, 9), tuple(reversed(CELLS))),
)


class AnalysisError(RuntimeError):
    """The evidence is incomplete, contaminated, or off schedule."""


@dataclass(frozen=True)
class RunRow:
    csv_line: int
    schedule_index: int
    block: int
    trace_state: str
    pass_name: str
    cell: str
    role: str
    pair: int
    role_order: int
    transfer_ms: Decimal
    settled_ms: int
    flush_ms: Decimal
    total_ms: Decimal
    landed_root: str
    tree_manifest_sha256: str
    exit_code: int
    drain: str
    valid: str
    run_id: str
    session_id: str
    client_log: str


@dataclass(frozen=True)
class TraceEvent:
    source_file: str
    source_line: int
    raw: dict[str, Any]

    @property
    def run_id(self) -> str:
        return self.raw["run_id"]

    @property
    def session_id(self) -> str:
        return self.raw["session_id"]

    @property
    def endpoint_role(self) -> str:
        return self.raw["endpoint_role"]

    @property
    def producer_seq(self) -> int:
        return self.raw["producer_seq"]

    @property
    def elapsed_ns(self) -> int:
        return self.raw["elapsed_ns"]

    @property
    def event(self) -> str:
        return self.raw["event"]


@dataclass(frozen=True)
class ClockSample:
    csv_line: int
    run: RunRow
    phase: str
    sample: int
    q_before_ns: int
    windows_ns: int
    q_after_ns: int
    rtt_ns: int
    offset_ns: int


@dataclass(frozen=True)
class ModeDescription:
    gap: Decimal
    left: tuple[Decimal, ...]
    right: tuple[Decimal, ...]

    def render(self) -> str:
        left = ";".join(decimal_text(value) for value in self.left)
        if not self.right:
            return f"[{left}]"
        right = ";".join(decimal_text(value) for value in self.right)
        return f"[{left}] | [{right}]"


@dataclass(frozen=True)
class ConditionStats:
    cell: str
    trace_state: str
    source_values: tuple[Decimal, ...]
    destination_values: tuple[Decimal, ...]
    paired_deltas: tuple[Decimal, ...]
    source_median: Decimal
    destination_median: Decimal
    delta: Decimal
    paired_delta_median: Decimal
    first4_delta_median: Decimal
    last4_delta_median: Decimal
    first_last_drift: Decimal
    odd_delta_median: Decimal
    even_delta_median: Decimal
    odd_even_drift: Decimal
    source_first_delta_median: Decimal
    destination_first_delta_median: Decimal
    role_order_drift: Decimal
    paired_delta_range: Decimal
    n_pair_split: Decimal
    n_pair: Decimal


@dataclass(frozen=True)
class AnalysisResult:
    summary_csv: Path
    summary_md: Path
    distributions_csv: Path
    phase_events_csv: Path
    phase_intervals_csv: Path
    clock_summary_csv: Path
    observer_bias: Decimal
    n_resolution: Decimal
    trace_event_count: int


def decimal_text(value: Decimal) -> str:
    if value == value.to_integral_value():
        return str(int(value))
    return format(value, "f").rstrip("0").rstrip(".")


def parse_decimal(value: str, field: str, line: int) -> Decimal:
    try:
        result = Decimal(value)
    except InvalidOperation as exc:
        raise AnalysisError(f"runs.csv line {line}: {field} is not numeric: {value!r}") from exc
    if not result.is_finite() or result < 0:
        raise AnalysisError(
            f"runs.csv line {line}: {field} must be a finite non-negative number"
        )
    return result


def parse_int(value: str, field: str, line: int, source: str = "runs.csv") -> int:
    try:
        return int(value)
    except ValueError as exc:
        raise AnalysisError(
            f"{source} line {line}: {field} is not an integer: {value!r}"
        ) from exc


def expected_roles(pair: int) -> tuple[str, str]:
    """The registered S/D/D/S first-arm pattern in each four-round block."""
    round_index = (pair - 1) % 4
    return ROLES if round_index in (0, 3) else tuple(reversed(ROLES))


def expected_schedule() -> list[tuple[BlockSpec, str, int, str, int]]:
    expected: list[tuple[BlockSpec, str, int, str, int]] = []
    for block in BLOCKS:
        for round_index, pair in enumerate(block.pairs):
            cells = block.cells if round_index in (0, 3) else tuple(reversed(block.cells))
            for cell in cells:
                for role_order, role in enumerate(
                    expected_roles(pair), start=1
                ):
                    expected.append((block, cell, pair, role, role_order))
    return expected


def _safe_client_log(root: Path, value: str, line: int) -> None:
    if not value:
        raise AnalysisError(f"runs.csv line {line}: client_log is blank")
    relative = Path(value)
    if relative.is_absolute():
        raise AnalysisError(f"runs.csv line {line}: client_log must be relative: {value!r}")
    root_resolved = root.resolve()
    candidate = (root / relative).resolve()
    if candidate != root_resolved and root_resolved not in candidate.parents:
        raise AnalysisError(f"runs.csv line {line}: client_log escapes output dir: {value!r}")
    if not candidate.is_file():
        raise AnalysisError(f"runs.csv line {line}: client_log does not exist: {value!r}")


def _read_tree_manifest(path: Path, label: str) -> tuple[bytes, str]:
    if not path.is_file():
        raise AnalysisError(f"missing {label}: {path}")
    data = path.read_bytes()
    if not data or not data.endswith(b"\n"):
        raise AnalysisError(f"{label}: manifest must be non-empty and newline-terminated")
    try:
        text = data.decode("ascii")
    except UnicodeDecodeError as exc:
        raise AnalysisError(f"{label}: manifest is not ASCII") from exc
    lines = text.splitlines()
    if lines != sorted(lines) or len(lines) != len(set(lines)):
        raise AnalysisError(f"{label}: manifest lines are not exact sorted unique inventory")
    for line_number, line in enumerate(lines, start=1):
        try:
            encoded, size_text = line.split(",", 1)
        except ValueError as exc:
            raise AnalysisError(
                f"{label} line {line_number}: expected base64_path,decimal_size"
            ) from exc
        if not size_text.isascii() or not size_text.isdecimal():
            raise AnalysisError(f"{label} line {line_number}: invalid decimal size")
        try:
            relative = base64.b64decode(encoded, validate=True).decode("utf-8")
        except (binascii.Error, UnicodeDecodeError, ValueError) as exc:
            raise AnalysisError(f"{label} line {line_number}: invalid UTF-8 base64 path") from exc
        parsed = PurePosixPath(relative)
        if (
            not relative
            or parsed.is_absolute()
            or relative != parsed.as_posix()
            or any(part in ("", ".", "..") for part in parsed.parts)
        ):
            raise AnalysisError(f"{label} line {line_number}: unsafe/noncanonical path")
    return data, hashlib.sha256(data).hexdigest()


def _load_fixture_manifests(root: Path) -> dict[str, tuple[bytes, str]]:
    index_path = root / "fixture-manifests.csv"
    if not index_path.is_file():
        raise AnalysisError(f"missing {index_path}")
    with index_path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        fields = ("shape", "sha256", "q_manifest", "windows_manifest")
        if tuple(reader.fieldnames or ()) != fields:
            raise AnalysisError("fixture-manifests.csv header mismatch")
        rows = list(reader)
    if [row["shape"] for row in rows] != ["mixed", "large"]:
        raise AnalysisError("fixture-manifests.csv must contain mixed,large exactly")
    fixture_dir = root / "fixtures"
    expected_files = {
        f"src_{shape}.manifest" for shape in ("mixed", "large")
    } | {f"windows-src_{shape}.manifest" for shape in ("mixed", "large")}
    actual_files = (
        {
            path.relative_to(fixture_dir).as_posix()
            for path in fixture_dir.rglob("*")
            if path.is_file()
        }
        if fixture_dir.is_dir()
        else set()
    )
    if actual_files != expected_files:
        raise AnalysisError(
            "fixture manifest file inventory mismatch: expected "
            f"{sorted(expected_files)}, got {sorted(actual_files)}"
        )

    result: dict[str, tuple[bytes, str]] = {}
    for row in rows:
        shape = row["shape"]
        q_relative = f"fixtures/src_{shape}.manifest"
        win_relative = f"fixtures/windows-src_{shape}.manifest"
        if row["q_manifest"] != q_relative or row["windows_manifest"] != win_relative:
            raise AnalysisError(f"fixture-manifests.csv {shape}: path mapping mismatch")
        q_data, q_digest = _read_tree_manifest(root / q_relative, f"q src_{shape}")
        win_data, win_digest = _read_tree_manifest(
            root / win_relative, f"Windows src_{shape}"
        )
        if q_data != win_data or q_digest != win_digest:
            raise AnalysisError(f"canonical q/Windows src_{shape} manifests differ")
        if row["sha256"] != q_digest:
            raise AnalysisError(f"fixture-manifests.csv {shape}: digest mismatch")
        result[shape] = (q_data, q_digest)
    return result


def load_runs(root: Path) -> list[RunRow]:
    fixture_manifests = _load_fixture_manifests(root)
    runs_path = root / "runs.csv"
    if not runs_path.is_file():
        raise AnalysisError(f"missing {runs_path}")
    with runs_path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        if tuple(reader.fieldnames or ()) != CSV_FIELDS:
            raise AnalysisError(
                "runs.csv header mismatch: expected "
                + ",".join(CSV_FIELDS)
                + "; got "
                + ",".join(reader.fieldnames or ())
            )
        raw_rows = list(reader)

    schedule = expected_schedule()
    if len(raw_rows) != len(schedule):
        raise AnalysisError(
            f"runs.csv schedule incomplete: expected {len(schedule)} rows, got {len(raw_rows)}"
        )

    rows: list[RunRow] = []
    for index, (raw, expected) in enumerate(zip(raw_rows, schedule), start=0):
        line = index + 2
        block_spec, cell, pair, role, role_order = expected
        actual_schedule = (
            parse_int(raw["block"], "block", line),
            raw["trace_state"],
            raw["pass"],
            raw["cell"],
            parse_int(raw["pair"], "pair", line),
            raw["role"],
            parse_int(raw["role_order"], "role_order", line),
        )
        wanted_schedule = (
            block_spec.number,
            block_spec.trace_state,
            block_spec.pass_name,
            cell,
            pair,
            role,
            role_order,
        )
        if actual_schedule != wanted_schedule:
            raise AnalysisError(
                f"runs.csv line {line}: schedule mismatch; expected {wanted_schedule}, "
                f"got {actual_schedule}"
            )
        exit_code = parse_int(raw["exit"], "exit", line)
        if exit_code != 0 or raw["drain"] != "drained" or raw["valid"] != "yes":
            raise AnalysisError(
                f"runs.csv line {line}: SESSION-VOID arm "
                f"(exit={exit_code}, drain={raw['drain']!r}, valid={raw['valid']!r})"
            )
        run_id = raw["run_id"]
        if not run_id:
            raise AnalysisError(f"runs.csv line {line}: run_id is blank")
        session_id = raw["session_id"]
        traced_tcp = block_spec.trace_state == "on" and cell in TCP_CELLS
        if traced_tcp:
            if not SESSION_ID_RE.fullmatch(session_id):
                raise AnalysisError(
                    f"runs.csv line {line}: trace-on TCP session_id must be 16 lowercase hex"
                )
        elif session_id:
            raise AnalysisError(
                f"runs.csv line {line}: session_id must be blank for trace-off or gRPC arms"
            )
        _safe_client_log(root, raw["client_log"], line)
        settled_ms = parse_int(raw["settled_ms"], "settled_ms", line)
        if not SETTLE_MIN_MS <= settled_ms < SETTLE_MAX_MS:
            raise AnalysisError(
                f"runs.csv line {line}: settled_ms must be in "
                f"[{SETTLE_MIN_MS},{SETTLE_MAX_MS}), got {settled_ms}"
            )
        transfer_ms = parse_decimal(raw["transfer_ms"], "transfer_ms", line)
        flush_ms = parse_decimal(raw["flush_ms"], "flush_ms", line)
        total_ms = parse_decimal(raw["total_ms"], "total_ms", line)
        settle_excess_ms = Decimal(settled_ms - SETTLE_MIN_MS)
        expected_total_ms = transfer_ms + settle_excess_ms + flush_ms
        if total_ms != expected_total_ms:
            raise AnalysisError(
                f"runs.csv line {line}: total_ms must equal transfer_ms + "
                f"(settled_ms - {SETTLE_MIN_MS}) + flush_ms "
                f"exactly; got {decimal_text(total_ms)} != "
                f"{decimal_text(transfer_ms)} + ({settled_ms} - "
                f"{SETTLE_MIN_MS}) + {decimal_text(flush_ms)}"
            )
        shape = cell.rsplit("_", 1)[1]
        landed_root = raw["landed_root"]
        expected_root = f"src_{shape}"
        if landed_root != expected_root:
            raise AnalysisError(
                f"runs.csv line {line}: landed_root must be {expected_root!r}"
            )
        recorded_digest = raw["tree_manifest_sha256"]
        if not SHA256_RE.fullmatch(recorded_digest):
            raise AnalysisError(
                f"runs.csv line {line}: tree_manifest_sha256 must be 64 lowercase hex"
            )
        rid = f"b{block_spec.number}_{cell}_p{pair}_{role}"
        landed_data, landed_digest = _read_tree_manifest(
            root / "landed" / f"{rid}.manifest", f"landed manifest {rid}"
        )
        canonical_data, canonical_digest = fixture_manifests[shape]
        if landed_digest != recorded_digest:
            raise AnalysisError(f"runs.csv line {line}: landed manifest digest mismatch")
        if landed_data != canonical_data or landed_digest != canonical_digest:
            raise AnalysisError(
                f"runs.csv line {line}: landed relative-path/size manifest "
                f"does not match canonical src_{shape}"
            )
        rows.append(
            RunRow(
                csv_line=line,
                schedule_index=index,
                block=block_spec.number,
                trace_state=block_spec.trace_state,
                pass_name=block_spec.pass_name,
                cell=cell,
                role=role,
                pair=pair,
                role_order=role_order,
                transfer_ms=transfer_ms,
                settled_ms=settled_ms,
                flush_ms=flush_ms,
                total_ms=total_ms,
                landed_root=landed_root,
                tree_manifest_sha256=recorded_digest,
                exit_code=exit_code,
                drain=raw["drain"],
                valid=raw["valid"],
                run_id=run_id,
                session_id=session_id,
                client_log=raw["client_log"],
            )
        )

    landed_dir = root / "landed"
    expected_landed = {
        f"b{row.block}_{row.cell}_p{row.pair}_{row.role}.manifest"
        for row in rows
    }
    actual_landed = (
        {
            path.relative_to(landed_dir).as_posix()
            for path in landed_dir.rglob("*")
            if path.is_file()
        }
        if landed_dir.is_dir()
        else set()
    )
    if actual_landed != expected_landed:
        raise AnalysisError(
            "landed manifest file inventory mismatch: expected exactly 128 registered "
            f"files, got {len(actual_landed)}"
        )

    block_ids: dict[int, set[str]] = {}
    for row in rows:
        block_ids.setdefault(row.block, set()).add(row.run_id)
    for block, ids in sorted(block_ids.items()):
        if len(ids) != 1:
            raise AnalysisError(f"block {block}: run_id is not block-level: {sorted(ids)}")
    run_ids = [next(iter(block_ids[block.number])) for block in BLOCKS]
    if len(set(run_ids)) != len(run_ids):
        raise AnalysisError("block run_id values must be unique across the four blocks")

    session_keys = [
        (row.run_id, row.session_id)
        for row in rows
        if row.trace_state == "on" and row.cell in TCP_CELLS
    ]
    if len(session_keys) != len(set(session_keys)):
        raise AnalysisError("trace-on TCP (run_id, session_id) values must be unique")
    return rows


def load_clock_samples(root: Path, rows: Sequence[RunRow]) -> list[ClockSample]:
    path = root / "clock-samples.csv"
    if not path.is_file():
        raise AnalysisError(f"missing {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        if tuple(reader.fieldnames or ()) != CLOCK_FIELDS:
            raise AnalysisError(
                "clock-samples.csv header mismatch: expected "
                + ",".join(CLOCK_FIELDS)
                + "; got "
                + ",".join(reader.fieldnames or ())
            )
        raw_samples = list(reader)

    expected = [
        (run, phase, sample)
        for run in rows
        for phase in ("before", "after")
        for sample in range(1, 4)
    ]
    if len(raw_samples) != len(expected):
        raise AnalysisError(
            "clock-samples.csv inventory incomplete: expected "
            f"{len(expected)} samples (3 before + 3 after per arm), got {len(raw_samples)}"
        )

    result: list[ClockSample] = []
    for index, (raw, (run, phase, sample)) in enumerate(zip(raw_samples, expected)):
        line = index + 2
        actual_key = (
            parse_int(raw["block"], "block", line, "clock-samples.csv"),
            raw["run_id"],
            raw["cell"],
            parse_int(raw["pair"], "pair", line, "clock-samples.csv"),
            raw["role"],
            raw["phase"],
            parse_int(raw["sample"], "sample", line, "clock-samples.csv"),
        )
        expected_key = (
            run.block,
            run.run_id,
            run.cell,
            run.pair,
            run.role,
            phase,
            sample,
        )
        if actual_key != expected_key:
            raise AnalysisError(
                f"clock-samples.csv line {line}: schedule mismatch; expected "
                f"{expected_key}, got {actual_key}"
            )
        q_before = parse_int(
            raw["q_before_ns"], "q_before_ns", line, "clock-samples.csv"
        )
        windows = parse_int(
            raw["windows_ns"], "windows_ns", line, "clock-samples.csv"
        )
        q_after = parse_int(
            raw["q_after_ns"], "q_after_ns", line, "clock-samples.csv"
        )
        rtt = parse_int(raw["rtt_ns"], "rtt_ns", line, "clock-samples.csv")
        offset = parse_int(
            raw["offset_windows_minus_q_ns"],
            "offset_windows_minus_q_ns",
            line,
            "clock-samples.csv",
        )
        if q_before <= 0 or windows <= 0 or q_after <= q_before:
            raise AnalysisError(
                f"clock-samples.csv line {line}: q/windows times must be positive and "
                "q_before_ns < q_after_ns"
            )
        computed_rtt = q_after - q_before
        computed_offset = windows - (q_before + computed_rtt // 2)
        if rtt <= 0 or rtt != computed_rtt:
            raise AnalysisError(
                f"clock-samples.csv line {line}: rtt_ns mismatch; expected "
                f"{computed_rtt}, got {rtt}"
            )
        if offset != computed_offset:
            raise AnalysisError(
                f"clock-samples.csv line {line}: offset mismatch; expected "
                f"{computed_offset}, got {offset}"
            )
        result.append(
            ClockSample(
                csv_line=line,
                run=run,
                phase=phase,
                sample=sample,
                q_before_ns=q_before,
                windows_ns=windows,
                q_after_ns=q_after,
                rtt_ns=rtt,
                offset_ns=offset,
            )
        )
    return result


def _require_json_string(raw: dict[str, Any], name: str, where: str) -> None:
    if not isinstance(raw.get(name), str) or not raw[name]:
        raise AnalysisError(f"{where}: {name} must be a non-empty string")


def _require_json_int(raw: dict[str, Any], name: str, where: str) -> None:
    value = raw.get(name)
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise AnalysisError(f"{where}: {name} must be a non-negative integer")


def load_trace_events(root: Path) -> list[TraceEvent]:
    evidence_roots = (root / "trace", root / "client")
    for evidence_root in evidence_roots:
        if not evidence_root.is_dir():
            raise AnalysisError(f"missing trace evidence directory: {evidence_root}")
    paths: list[Path] = []
    seen_resolved: set[Path] = set()
    for evidence_root in evidence_roots:
        for candidate in sorted(path for path in evidence_root.rglob("*") if path.is_file()):
            resolved = candidate.resolve()
            if resolved in seen_resolved:
                continue
            seen_resolved.add(resolved)
            paths.append(candidate)
    events: list[TraceEvent] = []
    for path in paths:
        relative = path.relative_to(root).as_posix()
        try:
            handle = path.open(encoding="utf-8")
            with handle:
                for line_number, line in enumerate(handle, start=1):
                    if not line.startswith(TRACE_PREFIX):
                        continue
                    payload = line[len(TRACE_PREFIX) :].rstrip("\r\n")
                    where = f"{relative}:{line_number}"
                    try:
                        raw = json.loads(payload)
                    except json.JSONDecodeError as exc:
                        raise AnalysisError(f"{where}: malformed session-phase JSON: {exc}") from exc
                    if not isinstance(raw, dict):
                        raise AnalysisError(f"{where}: session-phase payload is not an object")
                    if raw.get("schema") != 1 or isinstance(raw.get("schema"), bool):
                        raise AnalysisError(f"{where}: unsupported session-phase schema")
                    for name in (
                        "run_id",
                        "session_id",
                        "endpoint_role",
                        "initiator_role",
                        "event",
                    ):
                        _require_json_string(raw, name, where)
                    for name in ("producer_seq", "unix_ns", "elapsed_ns"):
                        _require_json_int(raw, name, where)
                    if not SESSION_ID_RE.fullmatch(raw["session_id"]):
                        raise AnalysisError(f"{where}: session_id is not 16 lowercase hex")
                    if raw["endpoint_role"] not in ("SOURCE", "DESTINATION"):
                        raise AnalysisError(f"{where}: invalid endpoint_role")
                    if raw["initiator_role"] not in ("SOURCE", "DESTINATION"):
                        raise AnalysisError(f"{where}: invalid initiator_role")
                    for name in (
                        "epoch",
                        "socket",
                        "batch",
                        "count",
                        "target_streams",
                        "live_streams",
                    ):
                        if name in raw:
                            _require_json_int(raw, name, where)
                    if "accepted" in raw and not isinstance(raw["accepted"], bool):
                        raise AnalysisError(f"{where}: accepted must be boolean")
                    events.append(TraceEvent(relative, line_number, raw))
        except UnicodeDecodeError as exc:
            raise AnalysisError(f"{relative}: trace log is not UTF-8") from exc
    return events


def _one_event(events: Sequence[TraceEvent], role: str, name: str, label: str) -> TraceEvent:
    found = [event for event in events if event.endpoint_role == role and event.event == name]
    if len(found) != 1:
        raise AnalysisError(f"{label}: expected one {role}/{name}, got {len(found)}")
    return found[0]


def _correlation_keys(
    events: Sequence[TraceEvent], role: str, name: str, label: str
) -> set[tuple[int, int]]:
    selected = [event for event in events if event.endpoint_role == role and event.event == name]
    keys: list[tuple[int, int]] = []
    for event in selected:
        epoch = event.raw.get("epoch")
        socket = event.raw.get("socket")
        if not isinstance(epoch, int) or not isinstance(socket, int):
            raise AnalysisError(f"{label}: {role}/{name} lacks epoch/socket correlation")
        keys.append((epoch, socket))
    if len(keys) != len(set(keys)):
        raise AnalysisError(f"{label}: duplicate {role}/{name} epoch/socket marker")
    return set(keys)


def _marker_map(
    events: Sequence[TraceEvent],
    role: str,
    name: str,
    key_fields: tuple[str, ...],
    label: str,
    *,
    required: bool = True,
) -> dict[tuple[int, ...], TraceEvent]:
    selected = [event for event in events if event.endpoint_role == role and event.event == name]
    if required and not selected:
        raise AnalysisError(f"{label}: missing {role}/{name} inventory")
    result: dict[tuple[int, ...], TraceEvent] = {}
    for event in selected:
        values: list[int] = []
        for field in key_fields:
            value = event.raw.get(field)
            if isinstance(value, bool) or not isinstance(value, int) or value < 0:
                raise AnalysisError(f"{label}: {role}/{name} lacks {field} correlation")
            values.append(value)
        key = tuple(values)
        if key in result:
            raise AnalysisError(f"{label}: duplicate {role}/{name} marker for {key}")
        result[key] = event
    return result


def _assert_same_keys(
    label: str, named_maps: Sequence[tuple[str, dict[tuple[int, ...], TraceEvent]]]
) -> set[tuple[int, ...]]:
    first_name, first = named_maps[0]
    wanted = set(first)
    for name, markers in named_maps[1:]:
        if set(markers) != wanted:
            raise AnalysisError(
                f"{label}: correlation mismatch {first_name}={sorted(wanted)} "
                f"vs {name}={sorted(markers)}"
            )
    return wanted


def _assert_before(label: str, start: TraceEvent, end: TraceEvent) -> None:
    if (
        start.endpoint_role != end.endpoint_role
        or start.producer_seq >= end.producer_seq
        or start.elapsed_ns > end.elapsed_ns
    ):
        raise AnalysisError(
            f"{label}: invalid local sequence {start.endpoint_role}/{start.event} "
            f"-> {end.endpoint_role}/{end.event}"
        )


def _assert_event_fields(
    label: str, event: TraceEvent, expected: dict[str, Any]
) -> None:
    epoch = event.raw.get("epoch")
    for field, wanted in expected.items():
        actual = event.raw.get(field)
        if actual != wanted:
            raise AnalysisError(
                f"{label}: {event.endpoint_role}/{event.event} epoch {epoch} "
                f"{field} must be {wanted!r}, got {actual!r}"
            )


def validate_traces(
    rows: Sequence[RunRow], events: Sequence[TraceEvent]
) -> dict[tuple[str, str], list[TraceEvent]]:
    block_by_run: dict[str, int] = {}
    for row in rows:
        block_by_run[row.run_id] = row.block
    expected_rows = {
        (row.run_id, row.session_id): row
        for row in rows
        if row.trace_state == "on" and row.cell in TCP_CELLS
    }
    grouped: dict[tuple[str, str], list[TraceEvent]] = {}
    for event in events:
        key = (event.run_id, event.session_id)
        if key not in expected_rows:
            if event.run_id in block_by_run:
                block = block_by_run[event.run_id]
                state = next(row.trace_state for row in rows if row.block == block)
                if state == "off":
                    raise AnalysisError(
                        f"trace leak: trace-off block {block} emitted {event.session_id} "
                        f"at {event.source_file}:{event.source_line}"
                    )
                raise AnalysisError(
                    f"trace leak: block {block} emitted an unregistered (including possible "
                    f"gRPC) session {event.session_id} at "
                    f"{event.source_file}:{event.source_line}"
                )
            raise AnalysisError(
                f"stale/foreign trace run_id {event.run_id!r} at "
                f"{event.source_file}:{event.source_line}"
            )
        grouped.setdefault(key, []).append(event)

    missing = sorted(set(expected_rows) - set(grouped))
    if missing:
        run_id, session_id = missing[0]
        row = expected_rows[(run_id, session_id)]
        raise AnalysisError(
            f"missing trace for block {row.block} {row.cell} pair {row.pair} "
            f"{row.role} ({run_id}/{session_id}); {len(missing)} session(s) missing"
        )

    for key, row in expected_rows.items():
        group = grouped[key]
        label = (
            f"block {row.block} {row.cell} pair {row.pair} {row.role} "
            f"({row.run_id}/{row.session_id})"
        )
        expected_initiator = "SOURCE" if row.role == "source_init" else "DESTINATION"
        roles = {event.endpoint_role for event in group}
        if roles != {"SOURCE", "DESTINATION"}:
            raise AnalysisError(
                f"{label}: missing endpoint role; expected SOURCE+DESTINATION, got {sorted(roles)}"
            )
        if {event.raw["initiator_role"] for event in group} != {expected_initiator}:
            raise AnalysisError(f"{label}: initiator_role does not match scheduled role")

        by_role: dict[str, list[TraceEvent]] = {}
        for event in group:
            by_role.setdefault(event.endpoint_role, []).append(event)
        for endpoint_role, endpoint_events in by_role.items():
            seqs = sorted(event.producer_seq for event in endpoint_events)
            if seqs != list(range(len(endpoint_events))):
                raise AnalysisError(
                    f"{label}: {endpoint_role} producer_seq is not exact contiguous 0..n-1: "
                    f"{seqs}"
                )

        manifest_begin = _one_event(
            group, "SOURCE", "manifest_complete_send_begin", label
        )
        manifest_sent = _one_event(group, "SOURCE", "manifest_complete_sent", label)
        _one_event(group, "DESTINATION", "manifest_complete_received", label)
        first_queued = _one_event(group, "SOURCE", "first_payload_queued", label)
        _assert_before(label, manifest_begin, manifest_sent)
        _assert_before(label, manifest_sent, first_queued)

        need_begin = _marker_map(
            group, "DESTINATION", "need_batch_send_begin", ("batch",), label
        )
        need_sent = _marker_map(
            group, "DESTINATION", "need_batch_sent", ("batch",), label
        )
        need_received = _marker_map(
            group, "SOURCE", "need_batch_received", ("batch",), label
        )
        need_keys = _assert_same_keys(
            label,
            (
                ("need_send_begin", need_begin),
                ("need_sent", need_sent),
                ("need_received", need_received),
            ),
        )
        if need_keys != {(batch,) for batch in range(len(need_keys))}:
            raise AnalysisError(f"{label}: need batch correlation is not contiguous from zero")
        for key in need_keys:
            _assert_before(label, need_begin[key], need_sent[key])
            counts = {
                need_begin[key].raw.get("count"),
                need_sent[key].raw.get("count"),
                need_received[key].raw.get("count"),
            }
            if len(counts) != 1 or not all(
                isinstance(value, int) and not isinstance(value, bool) and value > 0
                for value in counts
            ):
                raise AnalysisError(f"{label}: need batch {key[0]} count correlation mismatch")

        planner_begin = _marker_map(
            group, "SOURCE", "planner_begin", ("batch",), label
        )
        planner_end = _marker_map(group, "SOURCE", "planner_end", ("batch",), label)
        planner_keys = _assert_same_keys(
            label, (("planner_begin", planner_begin), ("planner_end", planner_end))
        )
        if planner_keys != {(batch,) for batch in range(len(planner_keys))}:
            raise AnalysisError(f"{label}: planner batch correlation is not contiguous from zero")
        for key in planner_keys:
            _assert_before(label, planner_begin[key], planner_end[key])
        earliest_planner_end = min(
            planner_end.values(), key=lambda event: event.producer_seq
        )
        _assert_before(label, earliest_planner_end, first_queued)

        resize_maps = (
            ("resize_proposed", _marker_map(group, "SOURCE", "resize_proposed", ("epoch",), label)),
            (
                "resize_send_begin",
                _marker_map(group, "SOURCE", "resize_send_begin", ("epoch",), label),
            ),
            ("resize_sent", _marker_map(group, "SOURCE", "resize_sent", ("epoch",), label)),
            (
                "resize_received",
                _marker_map(group, "DESTINATION", "resize_received", ("epoch",), label),
            ),
            (
                "destination_prepared",
                _marker_map(group, "DESTINATION", "destination_prepared", ("epoch",), label),
            ),
            (
                "resize_ack_send_begin",
                _marker_map(
                    group, "DESTINATION", "resize_ack_send_begin", ("epoch",), label
                ),
            ),
            (
                "resize_ack_sent",
                _marker_map(group, "DESTINATION", "resize_ack_sent", ("epoch",), label),
            ),
            (
                "resize_ack_received",
                _marker_map(group, "SOURCE", "resize_ack_received", ("epoch",), label),
            ),
            (
                "source_settled",
                _marker_map(group, "SOURCE", "source_settled", ("epoch",), label),
            ),
        )
        resize_epochs = _assert_same_keys(label, resize_maps)
        expected_resize_epochs = {(epoch,) for epoch in range(1, 8)}
        if resize_epochs != expected_resize_epochs:
            raise AnalysisError(
                f"{label}: resize epochs must be exactly 1..7, got "
                f"{sorted(epoch[0] for epoch in resize_epochs)}"
            )
        resize = dict(resize_maps)
        expected_prepared_action = (
            "arm_queued" if expected_initiator == "SOURCE" else "dial_complete"
        )
        for key in sorted(resize_epochs):
            epoch = key[0]
            target = epoch + 1
            _assert_before(label, resize["resize_proposed"][key], resize["resize_send_begin"][key])
            _assert_before(label, resize["resize_send_begin"][key], resize["resize_sent"][key])
            _assert_before(
                label,
                resize["resize_received"][key],
                resize["destination_prepared"][key],
            )
            _assert_before(
                label,
                resize["destination_prepared"][key],
                resize["resize_ack_send_begin"][key],
            )
            _assert_before(
                label,
                resize["resize_ack_send_begin"][key],
                resize["resize_ack_sent"][key],
            )
            _assert_before(
                label, resize["resize_ack_received"][key], resize["source_settled"][key]
            )
            for name in ("resize_proposed", "resize_send_begin", "resize_sent"):
                _assert_event_fields(
                    label,
                    resize[name][key],
                    {"target_streams": target, "live_streams": epoch},
                )
            _assert_event_fields(
                label,
                resize["resize_received"][key],
                {"target_streams": target, "live_streams": epoch},
            )
            _assert_event_fields(
                label,
                resize["destination_prepared"][key],
                {"target_streams": target},
            )
            for name in ("resize_ack_send_begin", "resize_ack_sent"):
                _assert_event_fields(
                    label,
                    resize[name][key],
                    {"accepted": True, "live_streams": target},
                )
            _assert_event_fields(
                label,
                resize["resize_ack_received"][key],
                {"accepted": True, "live_streams": target},
            )
            _assert_event_fields(
                label,
                resize["source_settled"][key],
                {
                    "accepted": True,
                    "target_streams": target,
                    "live_streams": target,
                },
            )
            if resize["destination_prepared"][key].raw.get("action") != expected_prepared_action:
                raise AnalysisError(
                    f"{label}: resize epoch {key[0]} destination_prepared action must be "
                    f"{expected_prepared_action}"
                )
        for epoch in range(1, 7):
            _assert_before(
                label,
                resize["source_settled"][(epoch,)],
                resize["resize_proposed"][(epoch + 1,)],
            )
            _assert_before(
                label,
                resize["resize_ack_sent"][(epoch,)],
                resize["resize_received"][(epoch + 1,)],
            )

        source_complete = _one_event(group, "SOURCE", "data_plane_complete", label)
        source_summary = _one_event(group, "SOURCE", "summary_received", label)
        destination_complete = _one_event(group, "DESTINATION", "data_plane_complete", label)
        destination_summary_begin = _one_event(
            group, "DESTINATION", "summary_send_begin", label
        )
        destination_summary = _one_event(group, "DESTINATION", "summary_sent", label)
        if source_complete.producer_seq >= source_summary.producer_seq:
            raise AnalysisError(f"{label}: SOURCE terminal inventory is out of sequence")
        if not (
            destination_complete.producer_seq
            < destination_summary_begin.producer_seq
            < destination_summary.producer_seq
        ):
            raise AnalysisError(f"{label}: DESTINATION terminal inventory is out of sequence")

        source_attachment_events = _marker_map(
            group,
            "SOURCE",
            "socket_trace_attached",
            ("epoch", "socket"),
            label,
        )
        destination_attachment_events = _marker_map(
            group,
            "DESTINATION",
            "socket_trace_attached",
            ("epoch", "socket"),
            label,
        )
        source_attached = set(source_attachment_events)
        destination_attached = set(destination_attachment_events)
        if not source_attached or source_attached != destination_attached:
            raise AnalysisError(f"{label}: two-role socket attachment correlation mismatch")
        expected_attached = {(0, 0)} | {(epoch[0], 0) for epoch in resize_epochs}
        if source_attached != expected_attached:
            raise AnalysisError(
                f"{label}: socket attachment inventory {sorted(source_attached)} does not "
                f"match epoch-0 plus accepted resize epochs {sorted(expected_attached)}"
            )
        for endpoint_role, complete in (
            ("SOURCE", source_complete),
            ("DESTINATION", destination_complete),
        ):
            for attached in (
                event
                for event in group
                if event.endpoint_role == endpoint_role
                and event.event == "socket_trace_attached"
            ):
                _assert_before(label, attached, complete)

        source_action = "dial" if expected_initiator == "SOURCE" else "accept"
        destination_action = "accept" if expected_initiator == "SOURCE" else "dial"
        action_events: dict[
            str, tuple[dict[tuple[int, ...], TraceEvent], dict[tuple[int, ...], TraceEvent]]
        ] = {}
        for endpoint_role, action in (
            ("SOURCE", source_action),
            ("DESTINATION", destination_action),
        ):
            begins = _marker_map(
                group,
                endpoint_role,
                f"socket_{action}_begin",
                ("epoch", "socket"),
                label,
            )
            ends = _marker_map(
                group,
                endpoint_role,
                f"socket_{action}_end",
                ("epoch", "socket"),
                label,
            )
            action_keys = _assert_same_keys(
                label,
                ((f"{endpoint_role}_{action}_begin", begins), (f"{endpoint_role}_{action}_end", ends)),
            )
            if action_keys != expected_attached:
                raise AnalysisError(f"{label}: {endpoint_role} socket action inventory mismatch")
            other_action = "accept" if action == "dial" else "dial"
            if _marker_map(
                group,
                endpoint_role,
                f"socket_{other_action}_begin",
                ("epoch", "socket"),
                label,
                required=False,
            ):
                raise AnalysisError(
                    f"{label}: {endpoint_role} unexpectedly mixed dial and accept actions"
                )
            attachments = (
                source_attachment_events
                if endpoint_role == "SOURCE"
                else destination_attachment_events
            )
            for action_key in action_keys:
                _assert_before(label, begins[action_key], ends[action_key])
                _assert_before(label, ends[action_key], attachments[action_key])
            action_events[endpoint_role] = (begins, ends)

        source_action_begins, source_action_ends = action_events["SOURCE"]
        destination_action_begins, destination_action_ends = action_events[
            "DESTINATION"
        ]
        for (epoch,) in sorted(resize_epochs):
            action_key = (epoch, 0)
            _assert_before(
                label,
                resize["resize_ack_received"][(epoch,)],
                source_action_begins[action_key],
            )
            _assert_before(
                label,
                source_action_ends[action_key],
                resize["source_settled"][(epoch,)],
            )

        arm_begin = _marker_map(
            group,
            "DESTINATION",
            "resize_arm_queue_begin",
            ("epoch",),
            label,
            required=False,
        )
        arm_ready = _marker_map(
            group,
            "DESTINATION",
            "resize_arm_ready",
            ("epoch",),
            label,
            required=False,
        )
        if expected_initiator == "SOURCE":
            arm_epochs = _assert_same_keys(
                label, (("resize_arm_queue_begin", arm_begin), ("resize_arm_ready", arm_ready))
            )
            if arm_epochs != resize_epochs:
                raise AnalysisError(f"{label}: destination resize-arm inventory mismatch")
            for arm_key in arm_epochs:
                _assert_event_fields(
                    label,
                    arm_begin[arm_key],
                    {"target_streams": arm_key[0] + 1},
                )
                _assert_before(
                    label,
                    arm_begin[arm_key],
                    resize["destination_prepared"][arm_key],
                )
                _assert_before(label, arm_begin[arm_key], arm_ready[arm_key])
                _assert_before(
                    label,
                    arm_begin[arm_key],
                    destination_action_begins[(arm_key[0], 0)],
                )
        elif arm_begin or arm_ready:
            raise AnalysisError(f"{label}: destination initiator unexpectedly emitted arm events")
        else:
            for (epoch,) in sorted(resize_epochs):
                _assert_before(
                    label,
                    destination_action_ends[(epoch, 0)],
                    resize["destination_prepared"][(epoch,)],
                )
        write_begin_events = _marker_map(
            group,
            "SOURCE",
            "socket_write_begin",
            ("epoch", "socket"),
            label,
        )
        write_events = _marker_map(
            group,
            "SOURCE",
            "first_socket_write",
            ("epoch", "socket"),
            label,
        )
        receive_events = _marker_map(
            group,
            "DESTINATION",
            "first_payload_received",
            ("epoch", "socket"),
            label,
        )
        write_begins = set(write_begin_events)
        writes = set(write_events)
        receives = set(receive_events)
        if not writes or write_begins != writes or writes != receives:
            raise AnalysisError(f"{label}: payload socket correlation mismatch")
        if not writes.issubset(source_attached):
            raise AnalysisError(f"{label}: SOURCE payload socket was not trace-attached")
        if not receives.issubset(destination_attached):
            raise AnalysisError(f"{label}: DESTINATION payload socket was not trace-attached")
        for action_key in writes:
            begin = write_begin_events[action_key]
            write = write_events[action_key]
            received = receive_events[action_key]
            _assert_before(label, first_queued, write)
            _assert_before(label, source_attachment_events[action_key], begin)
            _assert_before(label, begin, write)
            _assert_before(label, write, source_complete)
            _assert_before(
                label, destination_attachment_events[action_key], received
            )
            _assert_before(label, received, destination_complete)
    return grouped


def condition_stats(rows: Sequence[RunRow], cell: str, trace_state: str) -> ConditionStats:
    selected = [
        row for row in rows if row.cell == cell and row.trace_state == trace_state
    ]
    by_pair: dict[int, dict[str, Decimal]] = {}
    for row in selected:
        if row.role in by_pair.setdefault(row.pair, {}):
            raise AnalysisError(
                f"duplicate timing for {cell}/{trace_state}/pair {row.pair}/{row.role}"
            )
        by_pair[row.pair][row.role] = row.total_ms
    if sorted(by_pair) != list(range(1, 9)):
        raise AnalysisError(
            f"{cell}/{trace_state}: expected paired observations 1..8, got {sorted(by_pair)}"
        )
    for pair, arms in by_pair.items():
        if set(arms) != set(ROLES):
            raise AnalysisError(f"{cell}/{trace_state}/pair {pair}: incomplete role pair")
    source = tuple(by_pair[pair]["source_init"] for pair in range(1, 9))
    destination = tuple(by_pair[pair]["destination_init"] for pair in range(1, 9))
    deltas = tuple(dest - src for src, dest in zip(source, destination))
    source_first_pairs = {
        row.pair for row in selected if row.role == "source_init" and row.role_order == 1
    }
    destination_first_pairs = {
        row.pair
        for row in selected
        if row.role == "destination_init" and row.role_order == 1
    }
    if source_first_pairs | destination_first_pairs != set(range(1, 9)):
        raise AnalysisError(f"{cell}/{trace_state}: incomplete role-order partition")
    if source_first_pairs & destination_first_pairs:
        raise AnalysisError(f"{cell}/{trace_state}: overlapping role-order partition")
    source_first = median(
        tuple(deltas[pair - 1] for pair in sorted(source_first_pairs))
    )
    destination_first = median(
        tuple(deltas[pair - 1] for pair in sorted(destination_first_pairs))
    )
    first4 = median(deltas[:4])
    last4 = median(deltas[4:])
    odd = median(deltas[0::2])
    even = median(deltas[1::2])
    source_median = median(source)
    destination_median = median(destination)
    return ConditionStats(
        cell=cell,
        trace_state=trace_state,
        source_values=source,
        destination_values=destination,
        paired_deltas=deltas,
        source_median=source_median,
        destination_median=destination_median,
        delta=destination_median - source_median,
        paired_delta_median=median(deltas),
        first4_delta_median=first4,
        last4_delta_median=last4,
        first_last_drift=abs(first4 - last4),
        odd_delta_median=odd,
        even_delta_median=even,
        odd_even_drift=abs(odd - even),
        source_first_delta_median=source_first,
        destination_first_delta_median=destination_first,
        role_order_drift=abs(source_first - destination_first),
        paired_delta_range=max(deltas) - min(deltas),
        n_pair_split=max(abs(first4 - last4), abs(odd - even)),
        n_pair=max(
            abs(first4 - last4),
            abs(odd - even),
            abs(source_first - destination_first),
            max(deltas) - min(deltas),
        ),
    )


def largest_gap_modes(values: Iterable[Decimal]) -> ModeDescription:
    ordered = tuple(sorted(values))
    if not ordered:
        raise AnalysisError("cannot describe an empty distribution")
    gaps = tuple(right - left for left, right in zip(ordered, ordered[1:]))
    if not gaps or max(gaps) == 0:
        return ModeDescription(Decimal(0), ordered, ())
    gap = max(gaps)
    split = gaps.index(gap) + 1
    return ModeDescription(gap, ordered[:split], ordered[split:])


def _atomic_csv(path: Path, fields: Sequence[str], rows: Iterable[dict[str, Any]]) -> None:
    with tempfile.NamedTemporaryFile(
        mode="w", newline="", encoding="utf-8", dir=path.parent, delete=False
    ) as handle:
        temporary = Path(handle.name)
        writer = csv.DictWriter(handle, fieldnames=fields, extrasaction="raise")
        writer.writeheader()
        writer.writerows(rows)
    os.replace(temporary, path)


def _atomic_text(path: Path, contents: str) -> None:
    with tempfile.NamedTemporaryFile(
        mode="w", encoding="utf-8", dir=path.parent, delete=False
    ) as handle:
        temporary = Path(handle.name)
        handle.write(contents)
    os.replace(temporary, path)


def _summary_rows(
    stats: Sequence[ConditionStats], observer_bias: Decimal, n_resolution: Decimal
) -> list[dict[str, str]]:
    result: list[dict[str, str]] = []
    for item in stats:
        source_modes = largest_gap_modes(item.source_values)
        destination_modes = largest_gap_modes(item.destination_values)
        delta_modes = largest_gap_modes(item.paired_deltas)
        target = item.cell == TARGET_CELL
        result.append(
            {
                "cell": item.cell,
                "trace_state": item.trace_state,
                "measurand": MEASURAND,
                "pairs": "8",
                "source_init_median_ms": decimal_text(item.source_median),
                "destination_init_median_ms": decimal_text(item.destination_median),
                "delta_ms": decimal_text(item.delta),
                "paired_delta_median_ms": decimal_text(item.paired_delta_median),
                "first4_delta_median_ms": decimal_text(item.first4_delta_median),
                "last4_delta_median_ms": decimal_text(item.last4_delta_median),
                "first_last_drift_ms": decimal_text(item.first_last_drift),
                "odd_delta_median_ms": decimal_text(item.odd_delta_median),
                "even_delta_median_ms": decimal_text(item.even_delta_median),
                "odd_even_drift_ms": decimal_text(item.odd_even_drift),
                "source_first_delta_median_ms": decimal_text(
                    item.source_first_delta_median
                ),
                "destination_first_delta_median_ms": decimal_text(
                    item.destination_first_delta_median
                ),
                "role_order_drift_ms": decimal_text(item.role_order_drift),
                "paired_delta_range_ms": decimal_text(item.paired_delta_range),
                "n_pair_split_ms": decimal_text(item.n_pair_split),
                "n_pair_ms": decimal_text(item.n_pair),
                "observer_bias_ms": decimal_text(observer_bias) if target else "",
                "n_resolution_ms": decimal_text(n_resolution) if target else "",
                "source_init_sorted_ms": ";".join(
                    decimal_text(value) for value in sorted(item.source_values)
                ),
                "destination_init_sorted_ms": ";".join(
                    decimal_text(value) for value in sorted(item.destination_values)
                ),
                "paired_delta_sorted_ms": ";".join(
                    decimal_text(value) for value in sorted(item.paired_deltas)
                ),
                "source_init_largest_gap_ms": decimal_text(source_modes.gap),
                "source_init_descriptive_modes_ms": source_modes.render(),
                "destination_init_largest_gap_ms": decimal_text(destination_modes.gap),
                "destination_init_descriptive_modes_ms": destination_modes.render(),
                "paired_delta_largest_gap_ms": decimal_text(delta_modes.gap),
                "paired_delta_descriptive_modes_ms": delta_modes.render(),
            }
        )
    return result


SUMMARY_FIELDS = (
    "cell",
    "trace_state",
    "measurand",
    "pairs",
    "source_init_median_ms",
    "destination_init_median_ms",
    "delta_ms",
    "paired_delta_median_ms",
    "first4_delta_median_ms",
    "last4_delta_median_ms",
    "first_last_drift_ms",
    "odd_delta_median_ms",
    "even_delta_median_ms",
    "odd_even_drift_ms",
    "source_first_delta_median_ms",
    "destination_first_delta_median_ms",
    "role_order_drift_ms",
    "paired_delta_range_ms",
    "n_pair_split_ms",
    "n_pair_ms",
    "observer_bias_ms",
    "n_resolution_ms",
    "source_init_sorted_ms",
    "destination_init_sorted_ms",
    "paired_delta_sorted_ms",
    "source_init_largest_gap_ms",
    "source_init_descriptive_modes_ms",
    "destination_init_largest_gap_ms",
    "destination_init_descriptive_modes_ms",
    "paired_delta_largest_gap_ms",
    "paired_delta_descriptive_modes_ms",
)


def _distribution_rows(stats: Sequence[ConditionStats]) -> list[dict[str, str]]:
    output: list[dict[str, str]] = []
    for item in stats:
        for metric, values in (
            ("source_init_total", item.source_values),
            ("destination_init_total", item.destination_values),
            ("paired_total_delta", item.paired_deltas),
        ):
            modes = largest_gap_modes(values)
            ordered = tuple(sorted(values))
            split = len(modes.left) if modes.right else None
            for rank, value in enumerate(ordered, start=1):
                mode = "single" if split is None else ("lower" if rank <= split else "upper")
                output.append(
                    {
                        "cell": item.cell,
                        "trace_state": item.trace_state,
                        "measurand": MEASURAND,
                        "metric": metric,
                        "rank": str(rank),
                        "value_ms": decimal_text(value),
                        "descriptive_mode": mode,
                        "largest_gap_after": "yes" if split == rank else "no",
                        "largest_gap_ms": decimal_text(modes.gap),
                    }
                )
    return output


CLOCK_SUMMARY_FIELDS = (
    "block",
    "run_id",
    "pass",
    "cell",
    "pair",
    "role",
    "role_order",
    "before_sample",
    "before_min_rtt_ns",
    "before_offset_windows_minus_q_ns",
    "after_sample",
    "after_min_rtt_ns",
    "after_offset_windows_minus_q_ns",
    "selected_max_rtt_ns",
    "selected_offset_change_ns",
)


def _clock_summary_rows(samples: Sequence[ClockSample]) -> list[dict[str, str]]:
    grouped: dict[int, dict[str, list[ClockSample]]] = {}
    for sample in samples:
        grouped.setdefault(sample.run.schedule_index, {}).setdefault(sample.phase, []).append(sample)
    output: list[dict[str, str]] = []
    for schedule_index in sorted(grouped):
        phases = grouped[schedule_index]
        if set(phases) != {"before", "after"}:
            raise AnalysisError(f"clock samples for schedule row {schedule_index} lack a phase")
        before = min(phases["before"], key=lambda item: (item.rtt_ns, item.sample))
        after = min(phases["after"], key=lambda item: (item.rtt_ns, item.sample))
        run = before.run
        output.append(
            {
                "block": str(run.block),
                "run_id": run.run_id,
                "pass": run.pass_name,
                "cell": run.cell,
                "pair": str(run.pair),
                "role": run.role,
                "role_order": str(run.role_order),
                "before_sample": str(before.sample),
                "before_min_rtt_ns": str(before.rtt_ns),
                "before_offset_windows_minus_q_ns": str(before.offset_ns),
                "after_sample": str(after.sample),
                "after_min_rtt_ns": str(after.rtt_ns),
                "after_offset_windows_minus_q_ns": str(after.offset_ns),
                "selected_max_rtt_ns": str(max(before.rtt_ns, after.rtt_ns)),
                "selected_offset_change_ns": str(after.offset_ns - before.offset_ns),
            }
        )
    return output


EVENT_FIELDS = (
    "block",
    "trace_state",
    "pass",
    "cell",
    "pair",
    "role",
    "role_order",
    "transfer_ms",
    "settled_ms",
    "flush_ms",
    "total_ms",
    "run_id",
    "session_id",
    "endpoint_role",
    "initiator_role",
    "producer_seq",
    "elapsed_ns",
    "unix_ns",
    "event",
    "action",
    "epoch",
    "socket",
    "batch",
    "count",
    "target_streams",
    "live_streams",
    "accepted",
    "source_file",
    "source_line",
)


def _event_row(row: RunRow, event: TraceEvent) -> dict[str, str]:
    raw = event.raw
    output = {
        "block": str(row.block),
        "trace_state": row.trace_state,
        "pass": row.pass_name,
        "cell": row.cell,
        "pair": str(row.pair),
        "role": row.role,
        "role_order": str(row.role_order),
        "transfer_ms": decimal_text(row.transfer_ms),
        "settled_ms": str(row.settled_ms),
        "flush_ms": decimal_text(row.flush_ms),
        "total_ms": decimal_text(row.total_ms),
        "run_id": row.run_id,
        "session_id": row.session_id,
        "endpoint_role": event.endpoint_role,
        "initiator_role": raw["initiator_role"],
        "producer_seq": str(event.producer_seq),
        "elapsed_ns": str(event.elapsed_ns),
        "unix_ns": str(raw["unix_ns"]),
        "event": event.event,
        "source_file": event.source_file,
        "source_line": str(event.source_line),
    }
    for name in (
        "action",
        "epoch",
        "socket",
        "batch",
        "count",
        "target_streams",
        "live_streams",
        "accepted",
    ):
        value = raw.get(name, "")
        if isinstance(value, bool):
            output[name] = str(value).lower()
        else:
            output[name] = str(value)
    return output


INTERVAL_FIELDS = (
    "block",
    "trace_state",
    "pass",
    "cell",
    "pair",
    "role",
    "run_id",
    "session_id",
    "endpoint_role",
    "initiator_role",
    "interval_kind",
    "interval_name",
    "correlation",
    "start_event",
    "end_event",
    "start_producer_seq",
    "end_producer_seq",
    "start_elapsed_ns",
    "end_elapsed_ns",
    "duration_ns",
)


SPAN_SPECS = (
    ("socket_dial_begin", "socket_dial_end", ("epoch", "socket")),
    ("socket_accept_begin", "socket_accept_end", ("epoch", "socket")),
    ("manifest_complete_send_begin", "manifest_complete_sent", ()),
    ("need_batch_send_begin", "need_batch_sent", ("batch",)),
    ("planner_begin", "planner_end", ("batch",)),
    ("resize_send_begin", "resize_sent", ("epoch",)),
    ("resize_ack_send_begin", "resize_ack_sent", ("epoch",)),
    ("resize_arm_queue_begin", "resize_arm_ready", ("epoch",)),
    ("socket_write_begin", "first_socket_write", ("epoch", "socket")),
    ("data_plane_complete", "summary_received", ()),
    ("data_plane_complete", "summary_send_begin", ()),
    ("summary_send_begin", "summary_sent", ()),
)


def _interval_base(row: RunRow, endpoint_role: str, initiator_role: str) -> dict[str, str]:
    return {
        "block": str(row.block),
        "trace_state": row.trace_state,
        "pass": row.pass_name,
        "cell": row.cell,
        "pair": str(row.pair),
        "role": row.role,
        "run_id": row.run_id,
        "session_id": row.session_id,
        "endpoint_role": endpoint_role,
        "initiator_role": initiator_role,
    }


def _make_interval(
    row: RunRow,
    start: TraceEvent,
    end: TraceEvent,
    kind: str,
    name: str,
    correlation: str,
) -> dict[str, str]:
    if start.endpoint_role != end.endpoint_role:
        raise AnalysisError("internal error: attempted a cross-endpoint interval")
    duration = end.elapsed_ns - start.elapsed_ns
    if duration < 0:
        raise AnalysisError(
            f"{row.run_id}/{row.session_id}/{start.endpoint_role}: negative local interval "
            f"{start.event}->{end.event}"
        )
    output = _interval_base(
        row, start.endpoint_role, start.raw["initiator_role"]
    )
    output.update(
        {
            "interval_kind": kind,
            "interval_name": name,
            "correlation": correlation,
            "start_event": start.event,
            "end_event": end.event,
            "start_producer_seq": str(start.producer_seq),
            "end_producer_seq": str(end.producer_seq),
            "start_elapsed_ns": str(start.elapsed_ns),
            "end_elapsed_ns": str(end.elapsed_ns),
            "duration_ns": str(duration),
        }
    )
    return output


def _phase_rows(
    rows: Sequence[RunRow], grouped: dict[tuple[str, str], list[TraceEvent]]
) -> tuple[list[dict[str, str]], list[dict[str, str]]]:
    event_rows: list[dict[str, str]] = []
    interval_rows: list[dict[str, str]] = []
    traced_rows = [
        row for row in rows if row.trace_state == "on" and row.cell in TCP_CELLS
    ]
    for row in traced_rows:
        group = grouped[(row.run_id, row.session_id)]
        for endpoint_role in ("SOURCE", "DESTINATION"):
            endpoint = [event for event in group if event.endpoint_role == endpoint_role]
            for event in sorted(endpoint, key=lambda item: item.producer_seq):
                event_rows.append(_event_row(row, event))

            chronological = sorted(
                endpoint, key=lambda item: (item.elapsed_ns, item.producer_seq)
            )
            for start, end in zip(chronological, chronological[1:]):
                interval_rows.append(
                    _make_interval(
                        row,
                        start,
                        end,
                        "adjacent_local_timeline",
                        f"{start.event}->{end.event}",
                        "",
                    )
                )

            for start_name, end_name, keys in SPAN_SPECS:
                starts = [event for event in endpoint if event.event == start_name]
                ends = [event for event in endpoint if event.event == end_name]
                start_groups: dict[tuple[Any, ...], list[TraceEvent]] = {}
                end_groups: dict[tuple[Any, ...], list[TraceEvent]] = {}
                for event in starts:
                    start_groups.setdefault(tuple(event.raw.get(key) for key in keys), []).append(event)
                for event in ends:
                    end_groups.setdefault(tuple(event.raw.get(key) for key in keys), []).append(event)
                for correlation_key in sorted(set(start_groups) & set(end_groups)):
                    if len(start_groups[correlation_key]) != 1 or len(end_groups[correlation_key]) != 1:
                        continue
                    correlation = ";".join(
                        f"{key}={value}" for key, value in zip(keys, correlation_key)
                    )
                    interval_rows.append(
                        _make_interval(
                            row,
                            start_groups[correlation_key][0],
                            end_groups[correlation_key][0],
                            "named_local_span",
                            f"{start_name}->{end_name}",
                            correlation,
                        )
                    )
    return event_rows, interval_rows


def _markdown(
    stats: Sequence[ConditionStats],
    observer_bias: Decimal,
    n_resolution: Decimal,
    trace_event_count: int,
    interval_count: int,
    clock_arm_count: int,
) -> str:
    target = {item.trace_state: item for item in stats if item.cell == TARGET_CELL}
    lines = [
        "# otp-12 pf-1 rig-W phase report",
        "",
        "Validation: PASS — exact four-block OFF–ON–ON–OFF schedule, forward/reverse "
        "cell and role ordering, 8 valid role pairs per trace state/cell, trace-off and "
        "gRPC trace absence, and correlated two-role TCP terminal traces.",
        "",
        "## Durable total wall-time summaries",
        "",
        "| cell | trace | source total median ms | destination total median ms | Δ total ms | paired total d median ms | N_pair_split total ms | role-order drift total ms | paired range total ms | N_pair total ms |",
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for item in stats:
        lines.append(
            "| "
            + " | ".join(
                (
                    item.cell,
                    item.trace_state,
                    decimal_text(item.source_median),
                    decimal_text(item.destination_median),
                    decimal_text(item.delta),
                    decimal_text(item.paired_delta_median),
                    decimal_text(item.n_pair_split),
                    decimal_text(item.role_order_drift),
                    decimal_text(item.paired_delta_range),
                    decimal_text(item.n_pair),
                )
            )
            + " |"
        )
    lines.extend(
        [
            "",
            "The authoritative wall-time measurand is `total_ms = transfer_ms + "
            f"(settled_ms - {SETTLE_MIN_MS}) + flush_ms`: client execution plus every "
            f"millisecond beyond the common {SETTLE_MIN_MS} ms observation budget and "
            "the destination durability "
            f"probe. Only the common first {SETTLE_MIN_MS} ms is excluded from summaries, "
            "deltas, "
            "distributions, observer bias, and resolution floors.",
            "",
            "`Δ = median(destination_init total_ms) − median(source_init total_ms)`. "
            "Each paired `d_i = destination_init total_ms_i − source_init total_ms_i`. "
            "`N_pair_split = max(|median(d_1..d_4) − median(d_5..d_8)|, "
            "|median(d_odd) − median(d_even)|)`. The conservative operative "
            "The independent role-order drift is "
            "`|median(d_source-first) − median(d_destination-first)|`; the S,D,D,S "
            "schedule means this is not the odd/even partition. The conservative "
            "operative `N_pair = max(N_pair_split, role-order drift, max(d) − min(d))`, "
            "so a balanced bimodal mixture cannot produce a zero floor.",
            "",
            f"For target `{TARGET_CELL}`: Δ_off={decimal_text(target['off'].delta)} ms, "
            f"Δ_on={decimal_text(target['on'].delta)} ms, observer_bias="
            f"|Δ_on−Δ_off|={decimal_text(observer_bias)} ms, N_pair_off="
            f"{decimal_text(target['off'].n_pair)} ms, N_pair_on="
            f"{decimal_text(target['on'].n_pair)} ms, and N_resolution="
            f"{decimal_text(n_resolution)} ms.",
            "",
            "This run measures the observer and paired resolution floors; it does not "
            "grade any hypothesis recovery.",
            "",
            "## Sorted distributions and descriptive largest-gap modes",
            "",
            "The split is descriptive only; it does not assert statistical modality.",
            "",
            "| cell | trace | metric | sorted ms | largest gap ms | descriptive modes |",
            "|---|---:|---|---|---:|---|",
        ]
    )
    for item in stats:
        for metric, values in (
            ("source_init total_ms", item.source_values),
            ("destination_init total_ms", item.destination_values),
            ("paired total_ms d", item.paired_deltas),
        ):
            modes = largest_gap_modes(values)
            ordered = ";".join(decimal_text(value) for value in sorted(values))
            lines.append(
                f"| {item.cell} | {item.trace_state} | {metric} | {ordered} | "
                f"{decimal_text(modes.gap)} | {modes.render()} |"
            )
    lines.extend(
        [
            "",
            "## Phase evidence",
            "",
            f"`phase_events.csv` contains {trace_event_count} structured events. "
            f"`phase_intervals.csv` contains {interval_count} local-clock intervals.",
            "",
            "Each phase-event row carries the arm's validated `transfer_ms`, `settled_ms`, "
            "`flush_ms`, and authoritative `total_ms`.",
            "",
            "Every interval uses `elapsed_ns` from one endpoint only. `unix_ns` is retained "
            "in the event export for provenance and is never used for cross-host subtraction.",
            "",
            "## Clock-offset evidence",
            "",
            f"`clock_summary.csv` selects the minimum-RTT before and after sample for each "
            f"of {clock_arm_count} scheduled arms and reports its midpoint offset. These "
            "samples document cross-host uncertainty only; no cross-host phase duration is "
            "computed or graded.",
            "",
        ]
    )
    return "\n".join(lines)


def analyze(root: Path | str) -> AnalysisResult:
    output_dir = Path(root)
    if not output_dir.is_dir():
        raise AnalysisError(f"output directory does not exist: {output_dir}")
    rows = load_runs(output_dir)
    clock_samples = load_clock_samples(output_dir, rows)
    events = load_trace_events(output_dir)
    grouped = validate_traces(rows, events)
    stats = [
        condition_stats(rows, cell, trace_state)
        for cell in CELLS
        for trace_state in ("off", "on")
    ]
    target = {item.trace_state: item for item in stats if item.cell == TARGET_CELL}
    observer_bias = abs(target["on"].delta - target["off"].delta)
    n_resolution = max(target["off"].n_pair, target["on"].n_pair, observer_bias)
    event_rows, interval_rows = _phase_rows(rows, grouped)

    summary_csv = output_dir / "summary.csv"
    summary_md = output_dir / "summary.md"
    distributions_csv = output_dir / "distributions.csv"
    phase_events_csv = output_dir / "phase_events.csv"
    phase_intervals_csv = output_dir / "phase_intervals.csv"
    clock_summary_csv = output_dir / "clock_summary.csv"
    _atomic_csv(
        summary_csv,
        SUMMARY_FIELDS,
        _summary_rows(stats, observer_bias, n_resolution),
    )
    _atomic_csv(
        distributions_csv,
        (
            "cell",
            "trace_state",
            "measurand",
            "metric",
            "rank",
            "value_ms",
            "descriptive_mode",
            "largest_gap_after",
            "largest_gap_ms",
        ),
        _distribution_rows(stats),
    )
    _atomic_csv(phase_events_csv, EVENT_FIELDS, event_rows)
    _atomic_csv(phase_intervals_csv, INTERVAL_FIELDS, interval_rows)
    clock_rows = _clock_summary_rows(clock_samples)
    _atomic_csv(clock_summary_csv, CLOCK_SUMMARY_FIELDS, clock_rows)
    _atomic_text(
        summary_md,
        _markdown(
            stats,
            observer_bias,
            n_resolution,
            len(events),
            len(interval_rows),
            len(clock_rows),
        ),
    )
    return AnalysisResult(
        summary_csv=summary_csv,
        summary_md=summary_md,
        distributions_csv=distributions_csv,
        phase_events_csv=phase_events_csv,
        phase_intervals_csv=phase_intervals_csv,
        clock_summary_csv=clock_summary_csv,
        observer_bias=observer_bias,
        n_resolution=n_resolution,
        trace_event_count=len(events),
    )


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output_dir", type=Path, help="rig-W harness output directory")
    args = parser.parse_args(argv)
    try:
        result = analyze(args.output_dir)
    except AnalysisError as exc:
        print(f"ANALYSIS-INVALID: {exc}", file=sys.stderr)
        return 2
    print(f"ANALYSIS-PASS: {result.summary_md}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
