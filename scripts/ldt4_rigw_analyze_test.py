#!/usr/bin/env python3
"""Mutation-sensitive synthetic guards for ldt4_rigw_analyze.py."""

from __future__ import annotations

import base64
import csv
import hashlib
import importlib.util
import json
import math
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Optional
from unittest import mock


MODULE_PATH = Path(__file__).with_name("ldt4_rigw_analyze.py")
SPEC = importlib.util.spec_from_file_location("ldt4_rigw_analyze", MODULE_PATH)
assert SPEC and SPEC.loader
analyzer = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = analyzer
SPEC.loader.exec_module(analyzer)


TEST_FIXTURES = {
    "large": (2, 3),
    "small": (2, 3),
    "mixed": (2, 3),
}
TEST_HARNESS_SHA = "1" * 40
TEST_SAFE_ID = "ldt4-test-session"
TEST_CELL_ORDER = (
    "q_to_windows_large",
    "windows_to_q_large",
    "windows_to_q_small",
    "q_to_windows_small",
    "q_to_windows_mixed",
    "windows_to_q_mixed",
)
TEST_FIRST_ROLE = (
    "source_init",
    "destination_init",
    "destination_init",
    "source_init",
    "source_init",
    "destination_init",
    "destination_init",
    "source_init",
)
TEST_SOURCE_PATHS = {
    ("q_to_windows", "large"): "/Users/michael/blit-ldt4-staging/fixtures/src_large",
    ("q_to_windows", "small"): "/Users/michael/blit-ldt4-staging/fixtures/src_small",
    ("q_to_windows", "mixed"): "/Users/michael/blit-ldt4-staging/fixtures/src_mixed",
    ("windows_to_q", "large"): "D:/blit-test/rigw-module/src_large",
    ("windows_to_q", "small"): "D:/blit-test/ldt4-staging/fixtures/src_small",
    ("windows_to_q", "mixed"): "D:/blit-test/rigw-module/src_mixed",
}
TEST_DESTINATION_ROOTS = {
    "q_to_windows": "D:/blit-test/ldt4-sessions",
    "windows_to_q": "/Users/michael/blit-ldt4-sessions",
}
TEST_BINARY_PATHS = {
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


def _write_csv(path: Path, fields: tuple[str, ...], rows: list[dict[str, str]]) -> None:
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


def _manifest_payload(content_suffix: str = "") -> str:
    entries = []
    for relative, size, content in (("a.bin", 1, b"a"), ("sub/b.bin", 2, b"bc")):
        encoded = base64.b64encode(relative.encode()).decode()
        digest = hashlib.sha256(content + content_suffix.encode()).hexdigest()
        entries.append((encoded, size, digest))
    entries.sort()
    return "".join(f"{encoded},{size},{digest}\n" for encoded, size, digest in entries)


class SyntheticSession:
    def __init__(self, root: Path) -> None:
        self.root = root
        self.fixture_rows: list[dict[str, str]] = []
        self.run_rows: list[dict[str, str]] = []
        self._session_number = 1
        (root / "MEASUREMENTS-COMPLETE").write_text(
            f"artifact_sha={analyzer.ARTIFACT_SHA}\n"
            f"harness_sha={TEST_HARNESS_SHA}\n"
            "arm_count=96\n",
            encoding="ascii",
        )
        _write_csv(
            root / "provenance.csv",
            ("name", "sha"),
            [
                {"name": "artifact", "sha": analyzer.ARTIFACT_SHA},
                {"name": "harness", "sha": TEST_HARNESS_SHA},
            ],
        )
        staging_rows: list[dict[str, str]] = []
        for (endpoint, role), (staged_path, runtime_path) in TEST_BINARY_PATHS.items():
            staging_rows.append(
                {
                    "endpoint": endpoint,
                    "role": role,
                    "artifact_sha": analyzer.ARTIFACT_SHA,
                    "build_id": analyzer.ARTIFACT_SHA[:12],
                    "sha256": hashlib.sha256(f"{endpoint}-{role}".encode()).hexdigest(),
                    "staged_path": staged_path,
                    "runtime_path": runtime_path,
                }
            )
        _write_csv(
            root / "staging-manifest.csv", analyzer.STAGING_FIELDS, staging_rows
        )
        self._write_harness_evidence(staging_rows)
        self._write_fixtures()
        self._write_runs()

    @staticmethod
    def _schedule_rows() -> list[tuple[str, str, str, str, str]]:
        rows: list[tuple[str, str, str, str, str]] = []
        sequence = 0
        for cell in TEST_CELL_ORDER:
            direction = next(
                candidate
                for candidate in ("q_to_windows", "windows_to_q")
                if cell.startswith(f"{candidate}_")
            )
            fixture = cell[len(direction) + 1 :]
            for first in TEST_FIRST_ROLE:
                second = (
                    "destination_init" if first == "source_init" else "source_init"
                )
                for initiator in (first, second):
                    sequence += 1
                    rows.append(
                        (f"{sequence:03d}", cell, direction, fixture, initiator)
                    )
        return rows

    def _write_harness_evidence(
        self, staging_rows: list[dict[str, str]]
    ) -> None:
        (self.root / "artifact-build.txt").write_text(
            f"artifact_sha={analyzer.ARTIFACT_SHA} build_id={analyzer.BUILD_ID} "
            f"cargo_lock_sha256={analyzer.CARGO_LOCK_SHA256} "
            f"q_artifact_repo={analyzer.Q_ARTIFACT_REPO}\n",
            encoding="ascii",
        )
        (self.root / "schedule.csv").write_text(
            "".join(",".join(row) + "\n" for row in self._schedule_rows()),
            encoding="ascii",
        )
        environment_tail = (
            f"q_ip={analyzer.Q_IP} q_nic={analyzer.Q_NIC} q_mtu=9000 "
            f"q_media=autoselect (10Gbase-T <full-duplex>) q_route={analyzer.Q_NIC} "
            f"q_route_mtu=9000 q_local_hostname={analyzer.Q_LOCAL_HOSTNAME} "
            f"q_computer_name={analyzer.Q_COMPUTER_NAME} q_mac={analyzer.Q_MAC} "
            f"q_peer={analyzer.WINDOWS_MAC} "
            "q_free=40000000000 q_load1=1.25 q_spotlight_cpu=2.5 "
            "time_machine_auto=0 time_machine_running=0 "
            "q_to_windows_mss=8948 windows_to_q_mss=8960 "
            "windows_powershell=7.6.3 "
            f"windows=W|40000000000|9000|10 Gbps|{analyzer.WINDOWS_NIC}|"
            "D:/blit-test/bins/active/blit-daemon.exe|00-01-D2-19-04-A3|7.6.3 "
            "windows_cpu_avg=5.5\n"
        )
        for phase in ("start", "end"):
            (self.root / f"environment-{phase}.txt").write_text(
                f"phase={phase} {environment_tail}", encoding="ascii"
            )
        runtime_rows = []
        for index, schedule in enumerate(self._schedule_rows()):
            if index % 2:
                continue
            runtime_rows.append(
                {
                    "sequence": schedule[0],
                    "cell": schedule[1],
                    "pair": str((index // 2) % 8 + 1),
                    "q_free_bytes": "40000000000",
                    "windows_free_bytes": "40000000000",
                    "q_quiet": (
                        "q_load1=1.25;q_spotlight_cpu=2.5;"
                        "time_machine_auto=0;time_machine_running=0"
                    ),
                    "windows_quiet": "windows_cpu_avg=5.5",
                }
            )
        _write_csv(
            self.root / "runtime-gates.csv", analyzer.RUNTIME_GATE_FIELDS, runtime_rows
        )
        staged_sha = next(
            row["sha256"]
            for row in staging_rows
            if row["endpoint"] == "windows" and row["role"] == "daemon"
        )
        prior_sha = "f" * 64
        (self.root / "windows-runtime-swap.txt").write_text(
            f"had_prior=1 prior_sha={prior_sha} staged_sha={staged_sha}\n",
            encoding="ascii",
        )
        (self.root / "windows-runtime-restoration.txt").write_text(
            "RESTORED|mode=normal|active=True|tested=True|"
            f"tested_sha={staged_sha}|restored_sha={prior_sha}\n",
            encoding="ascii",
        )

    def _write_fixtures(self) -> None:
        for direction in analyzer.DIRECTIONS:
            for fixture in analyzer.FIXTURES:
                relative = f"manifests/source/{direction}_{fixture}.csv"
                path = self.root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(_manifest_payload(), encoding="ascii")
                self.fixture_rows.append(
                    {
                        "direction": direction,
                        "fixture": fixture,
                        "source_manifest": relative,
                    }
                )
        _write_csv(
            self.root / "fixture-manifests.csv",
            analyzer.FIXTURE_INDEX_FIELDS,
            self.fixture_rows,
        )

    @staticmethod
    def _event(
        run_id: str,
        session_id: str,
        endpoint_role: str,
        initiator_role: str,
        event: str,
        **fields: object,
    ) -> dict[str, object]:
        value: dict[str, object] = {
            "schema": 1,
            "run_id": run_id,
            "session_id": session_id,
            "producer_seq": 0,
            "unix_ns": 0,
            "elapsed_ns": 0,
            "endpoint_role": endpoint_role,
            "initiator_role": initiator_role,
            "event": event,
        }
        value.update(fields)
        return value

    def _events(
        self, run_id: str, session_id: str, initiator: str
    ) -> tuple[list[dict[str, object]], list[dict[str, object]]]:
        initiator_role = "SOURCE" if initiator == "source_init" else "DESTINATION"
        source_transport = "dial" if initiator == "source_init" else "accept"
        destination_transport = "accept" if initiator == "source_init" else "dial"
        source: list[dict[str, object]] = []
        destination: list[dict[str, object]] = []

        def add(target: list[dict[str, object]], role: str, name: str, **fields: object) -> None:
            target.append(
                self._event(run_id, session_id, role, initiator_role, name, **fields)
            )

        for socket in range(4):
            add(source, "SOURCE", f"socket_{source_transport}_begin", epoch=0, socket=socket)
            add(source, "SOURCE", f"socket_{source_transport}_end", epoch=0, socket=socket)
            add(source, "SOURCE", "socket_trace_attached", epoch=0, socket=socket)
            add(
                destination,
                "DESTINATION",
                f"socket_{destination_transport}_begin",
                epoch=0,
                socket=socket,
            )
            add(
                destination,
                "DESTINATION",
                f"socket_{destination_transport}_end",
                epoch=0,
                socket=socket,
            )
            add(destination, "DESTINATION", "socket_trace_attached", epoch=0, socket=socket)

        def sample(
            *,
            reason: str,
            epoch: int,
            live: int,
            peak: int,
            chunk: int,
            prefetch: int,
            tcp: int,
            valid: bool = True,
            ratio: float = 0.0,
            action: Optional[str] = None,
            target: Optional[int] = None,
            sample_streams: Optional[int] = None,
        ) -> None:
            streams = sample_streams if sample_streams is not None else (live if valid else 0)
            fields: dict[str, object] = {
                "reason": reason,
                "epoch": epoch,
                "live_streams": live,
                "sample_bytes": 1024 if valid else 0,
                "sample_blocked_ns": int(ratio * analyzer.DIAL_TUNER_TICK_NS * streams),
                "sample_elapsed_ns": analyzer.DIAL_TUNER_TICK_NS,
                "sample_streams": streams,
                "sample_valid": valid,
                "blocked_ratio": ratio,
                "chunk_bytes": chunk,
                "prefetch_count": prefetch,
                "tcp_buffer_bytes": tcp,
                "receiver_ceiling": 32,
                "peak_streams": peak,
            }
            if action is not None and target is not None:
                fields.update(action=action, target_streams=target)
            add(source, "SOURCE", "dial_sample", **fields)

        clean_snapshots = (
            (32 * 1024 * 1024, 6, 8 * 1024 * 1024),
            (64 * 1024 * 1024, 9, 8 * 1024 * 1024),
            (64 * 1024 * 1024, 13, 8 * 1024 * 1024),
            (64 * 1024 * 1024, 19, 8 * 1024 * 1024),
            (64 * 1024 * 1024, 28, 8 * 1024 * 1024),
            (64 * 1024 * 1024, 32, 8 * 1024 * 1024),
            (64 * 1024 * 1024, 32, 8 * 1024 * 1024),
        )
        for index, (chunk, prefetch, tcp) in enumerate(clean_snapshots):
            proposing = index == len(clean_snapshots) - 1
            sample(
                reason="add" if proposing else "cheap-up",
                epoch=1 if proposing else 0,
                live=4,
                peak=4,
                chunk=chunk,
                prefetch=prefetch,
                tcp=tcp,
                action="ADD" if proposing else None,
                target=5 if proposing else None,
            )
        add(
            source,
            "SOURCE",
            "dial_pending",
            action="ADD",
            reason="pending",
            epoch=1,
            target_streams=5,
            live_streams=4,
            receiver_ceiling=32,
            peak_streams=4,
        )
        for name in ("resize_proposed", "resize_send_begin", "resize_sent"):
            add(
                source,
                "SOURCE",
                name,
                action="ADD",
                epoch=1,
                target_streams=5,
                live_streams=4,
            )
        add(source, "SOURCE", "resize_ack_received", epoch=1, live_streams=5, accepted=True)
        add(source, "SOURCE", f"socket_{source_transport}_begin", epoch=1, socket=0)
        add(source, "SOURCE", f"socket_{source_transport}_end", epoch=1, socket=0)
        add(source, "SOURCE", "socket_trace_attached", epoch=1, socket=0)
        add(
            source,
            "SOURCE",
            "dial_settlement",
            action="ADD",
            reason="add",
            epoch=1,
            target_streams=5,
            live_streams=5,
            accepted=True,
            receiver_ceiling=32,
            peak_streams=5,
        )
        add(
            source,
            "SOURCE",
            "source_settled",
            action="ADD",
            epoch=1,
            target_streams=5,
            live_streams=5,
            accepted=True,
        )

        sample(
            reason="rebaseline",
            epoch=1,
            live=5,
            peak=5,
            chunk=64 * 1024 * 1024,
            prefetch=32,
            tcp=8 * 1024 * 1024,
            valid=False,
            sample_streams=5,
        )

        blocked_snapshots = (
            (32 * 1024 * 1024, 16),
            (16 * 1024 * 1024, 8),
            (16 * 1024 * 1024, 4),
            (16 * 1024 * 1024, 4),
        )
        for index, (chunk, prefetch) in enumerate(blocked_snapshots):
            proposing = index == len(blocked_snapshots) - 1
            sample(
                reason="remove" if proposing else "cheap-down",
                epoch=2 if proposing else 1,
                live=5,
                peak=5,
                chunk=chunk,
                prefetch=prefetch,
                tcp=8 * 1024 * 1024,
                ratio=0.5,
                action="REMOVE" if proposing else None,
                target=4 if proposing else None,
            )
        add(
            source,
            "SOURCE",
            "dial_pending",
            action="REMOVE",
            reason="pending",
            epoch=2,
            target_streams=4,
            live_streams=5,
            receiver_ceiling=32,
            peak_streams=5,
        )
        for name in ("resize_proposed", "resize_send_begin", "resize_sent"):
            add(
                source,
                "SOURCE",
                name,
                action="REMOVE",
                epoch=2,
                target_streams=4,
                live_streams=5,
            )
        add(source, "SOURCE", "resize_ack_received", epoch=2, live_streams=4, accepted=True)
        add(
            source,
            "SOURCE",
            "dial_settlement",
            action="REMOVE",
            reason="remove",
            epoch=2,
            target_streams=4,
            live_streams=4,
            accepted=True,
            receiver_ceiling=32,
            peak_streams=5,
        )
        add(
            source,
            "SOURCE",
            "source_settled",
            action="REMOVE",
            epoch=2,
            target_streams=4,
            live_streams=4,
            accepted=True,
        )

        add(destination, "DESTINATION", "resize_received", epoch=1, target_streams=5, live_streams=4)
        if initiator == "source_init":
            add(destination, "DESTINATION", "resize_arm_queue_begin", epoch=1, target_streams=5)
            add(
                destination,
                "DESTINATION",
                "destination_prepared",
                action="add_prepared",
                epoch=1,
                target_streams=5,
                accepted=True,
            )
            add(
                destination,
                "DESTINATION",
                "resize_ack_send_begin",
                action="add",
                epoch=1,
                live_streams=5,
                accepted=True,
            )
            add(
                destination,
                "DESTINATION",
                "resize_ack_sent",
                action="add",
                epoch=1,
                live_streams=5,
                accepted=True,
            )
            add(destination, "DESTINATION", "resize_arm_ready", epoch=1)
            add(destination, "DESTINATION", "socket_accept_begin", epoch=1, socket=0)
            add(destination, "DESTINATION", "socket_accept_end", epoch=1, socket=0)
            add(destination, "DESTINATION", "socket_trace_attached", epoch=1, socket=0)
        else:
            add(destination, "DESTINATION", "socket_dial_begin", epoch=1, socket=0)
            add(destination, "DESTINATION", "socket_dial_end", epoch=1, socket=0)
            add(destination, "DESTINATION", "socket_trace_attached", epoch=1, socket=0)
            add(
                destination,
                "DESTINATION",
                "destination_prepared",
                action="add_prepared",
                epoch=1,
                target_streams=5,
                accepted=True,
            )
            add(
                destination,
                "DESTINATION",
                "resize_ack_send_begin",
                action="add",
                epoch=1,
                live_streams=5,
                accepted=True,
            )
            add(
                destination,
                "DESTINATION",
                "resize_ack_sent",
                action="add",
                epoch=1,
                live_streams=5,
                accepted=True,
            )
        add(destination, "DESTINATION", "resize_received", epoch=2, target_streams=4, live_streams=5)
        for name in ("resize_ack_send_begin", "resize_ack_sent"):
            add(
                destination,
                "DESTINATION",
                name,
                action="logical_remove",
                epoch=2,
                live_streams=4,
                accepted=True,
            )

        add(source, "SOURCE", "first_payload_queued")
        add(source, "SOURCE", "socket_write_begin", epoch=0, socket=0)
        add(source, "SOURCE", "first_socket_write", epoch=0, socket=0)
        add(destination, "DESTINATION", "first_payload_received", epoch=0, socket=0)

        completion = {"live_streams": 4, "receiver_ceiling": 32, "peak_streams": 5}
        add(source, "SOURCE", "membership_sealed", **completion)
        add(source, "SOURCE", "data_plane_complete", **completion)
        for socket in range(4):
            add(destination, "DESTINATION", "receive_task_stopped", epoch=0, socket=socket)
        add(destination, "DESTINATION", "receive_task_stopped", epoch=1, socket=0)
        add(destination, "DESTINATION", "data_plane_complete", **completion)

        for endpoint_index, events in enumerate((source, destination)):
            elapsed_ns = 0
            for sequence, event in enumerate(events):
                event["producer_seq"] = sequence
                # Deliberately unrelated host clocks. The analyzer must not compare them.
                event["unix_ns"] = (endpoint_index + 1) * 10**18 + sequence
                if endpoint_index == 0 and event["event"] == "dial_sample":
                    elapsed_ns += analyzer.DIAL_TUNER_TICK_NS
                else:
                    elapsed_ns += 1_000_000 + endpoint_index * 9_000_000
                event["elapsed_ns"] = elapsed_ns
        return source, destination

    def _write_trace(self, relative: str, events: list[dict[str, object]]) -> None:
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(
            "".join(
                f"noise before event\n{analyzer.TRACE_PREFIX}{json.dumps(event, separators=(',', ':'))}\n"
                for event in events
            ),
            encoding="utf-8",
        )

    def _write_runs(self) -> None:
        for cell in TEST_CELL_ORDER:
            direction = next(
                value for value in analyzer.DIRECTIONS if cell.startswith(f"{value}_")
            )
            fixture = cell.removeprefix(f"{direction}_")
            for pair, first in zip(range(1, 9), TEST_FIRST_ROLE):
                second = next(role for role in analyzer.INITIATORS if role != first)
                for initiator in (first, second):
                    number = self._session_number
                    self._session_number += 1
                    run_id = f"ldt4-{number:03d}"
                    session_id = f"{number:016x}"
                    source_events, destination_events = self._events(
                        run_id, session_id, initiator
                    )
                    source_trace, destination_trace = analyzer._registered_trace_paths(
                        direction, initiator, run_id
                    )
                    self._write_trace(source_trace, source_events)
                    self._write_trace(destination_trace, destination_events)
                    landed_manifest = f"manifests/landed/{run_id}.csv"
                    landed_path = self.root / landed_manifest
                    landed_path.parent.mkdir(parents=True, exist_ok=True)
                    landed_path.write_text(_manifest_payload(), encoding="ascii")
                    source_path = TEST_SOURCE_PATHS[(direction, fixture)]
                    destination_root = TEST_DESTINATION_ROOTS[direction]
                    active_path = f"{destination_root}/{TEST_SAFE_ID}/active/{fixture}"
                    archive_path = f"{destination_root}/{TEST_SAFE_ID}/retained/{run_id}"
                    duration = 8000 if initiator == "source_init" else 8100
                    self.run_rows.append(
                        {
                            "cell": cell,
                            "direction": direction,
                            "fixture": fixture,
                            "pair": str(pair),
                            "initiator": initiator,
                            "run_id": run_id,
                            "session_id": session_id,
                            "duration_ms": str(duration),
                            "files": "2",
                            "bytes": "3",
                            "source_path": source_path,
                            "active_destination_path": active_path,
                            "archive_path": archive_path,
                            "source_manifest": f"manifests/source/{direction}_{fixture}.csv",
                            "landed_manifest": landed_manifest,
                            "source_trace": source_trace,
                            "destination_trace": destination_trace,
                            "exit": "0",
                            "valid": "yes",
                        }
                    )
        self.write_runs()

    def write_runs(self) -> None:
        _write_csv(self.root / "runs.csv", analyzer.RUN_FIELDS, self.run_rows)

    def row(self, **wanted: object) -> dict[str, str]:
        return next(
            row
            for row in self.run_rows
            if all(row[key] == str(value) for key, value in wanted.items())
        )

    def read_events(self, row: dict[str, str], role: str = "source") -> list[dict[str, object]]:
        path = self.root / row[f"{role}_trace"]
        events = []
        for line in path.read_text(encoding="utf-8").splitlines():
            if line.startswith(analyzer.TRACE_PREFIX):
                events.append(json.loads(line[len(analyzer.TRACE_PREFIX) :]))
        return events

    def write_events(
        self, row: dict[str, str], events: list[dict[str, object]], role: str = "source"
    ) -> None:
        for sequence, event in enumerate(events):
            event["producer_seq"] = sequence
        self._write_trace(row[f"{role}_trace"], events)


class DialPolicyReplayTests(unittest.TestCase):
    def test_replay_covers_hysteresis_cooldown_sustain_and_bound(self) -> None:
        maxed = {
            "chunk_bytes": analyzer.CEILING_CHUNK_BYTES,
            "prefetch_count": analyzer.CEILING_PREFETCH,
            "tcp_buffer_bytes": analyzer.CEILING_TCP_BUFFER_BYTES,
        }
        cases = (
            ("hysteresis", {}, 4, 0.10),
            ("cooldown", maxed, 4, 0.0),
            ("sustain", {**maxed, "ticks_since_settle": 3}, 4, 0.0),
            (
                "bound",
                {**maxed, "ticks_since_settle": 3, "sustain": 1},
                analyzer.EXPECTED_CEILING,
                0.0,
            ),
        )
        for expected_reason, policy_fields, live_streams, ratio in cases:
            with self.subTest(reason=expected_reason):
                policy = analyzer.DialPolicyReplay(**policy_fields)
                event = {
                    "sample_valid": True,
                    "sample_bytes": 1024,
                    "blocked_ratio": ratio,
                    "reason": expected_reason,
                    "chunk_bytes": policy.chunk_bytes,
                    "prefetch_count": policy.prefetch_count,
                    "tcp_buffer_bytes": policy.tcp_buffer_bytes,
                }
                reason, proposal = policy.apply_sample(
                    event,
                    live_streams=live_streams,
                    settled_epoch=0,
                    context=f"unit {expected_reason}",
                )
                self.assertEqual(reason, expected_reason)
                self.assertIsNone(proposal)

        fixture_guarded = {
            "idle",
            "rebaseline",
            "cheap-up",
            "cheap-down",
            "add",
            "remove",
        }
        self.assertEqual(
            fixture_guarded | {case[0] for case in cases},
            analyzer.SAMPLE_REASONS,
        )


class AnalyzerTests(unittest.TestCase):
    def setUp(self) -> None:
        patcher = mock.patch.dict(analyzer.EXPECTED_FIXTURES, TEST_FIXTURES, clear=True)
        patcher.start()
        self.addCleanup(patcher.stop)
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name) / TEST_SAFE_ID
        self.root.mkdir()
        self.session = SyntheticSession(self.root)

    def analyze(self, expected_harness_sha: str = TEST_HARNESS_SHA) -> analyzer.AnalysisResult:
        return analyzer.analyze(self.root, expected_harness_sha)

    def test_registered_windows_endpoint_matches_current_identity(self) -> None:
        self.assertEqual(analyzer.WINDOWS_IP, "10.1.10.173")

    def make_zero_sample_arm(self, row: dict[str, str], phase_span_ns: int) -> None:
        source = [
            event
            for event in self.session.read_events(row)
            if not event["event"].startswith("dial_")
            and event["event"] not in analyzer.SOURCE_CONTROL_FIELDS
            and not (
                event["event"].startswith("socket_") and event.get("epoch", 0) > 0
            )
        ]
        source_complete = next(
            event for event in source if event["event"] == "data_plane_complete"
        )
        source_complete.update(live_streams=4, peak_streams=4)
        membership_sealed = next(
            event for event in source if event["event"] == "membership_sealed"
        )
        membership_sealed.update(live_streams=4, peak_streams=4)
        latest_attachment_ns = max(
            event["elapsed_ns"]
            for event in source
            if event["event"] == "socket_trace_attached" and event.get("epoch") == 0
        )
        membership_sealed["elapsed_ns"] = latest_attachment_ns + phase_span_ns
        source_complete["elapsed_ns"] = (
            latest_attachment_ns + analyzer.DIAL_TUNER_TICK_NS + 100_000_000
        )

        destination = [
            event
            for event in self.session.read_events(row, "destination")
            if event["event"] not in analyzer.DESTINATION_CONTROL_FIELDS
            and not (
                event["event"].startswith("socket_") and event.get("epoch", 0) > 0
            )
            and not (
                event["event"] == "receive_task_stopped" and event.get("epoch", 0) > 0
            )
        ]
        destination_complete = next(
            event for event in destination if event["event"] == "data_plane_complete"
        )
        destination_complete.update(live_streams=4, peak_streams=4)
        self.session.write_events(row, source)
        self.session.write_events(row, destination, "destination")

    def test_valid_matrix_writes_immutable_adaptive_reports(self) -> None:
        self.assertEqual(analyzer.CELL_ORDER, TEST_CELL_ORDER)
        result = self.analyze()
        self.assertEqual(result.arm_count, 96)
        self.assertEqual(result.arm_review_count, 0)
        self.assertEqual(result.status, "STRUCTURALLY_VALID_NO_ROLE_SKEW")
        summary = json.loads((result.output_dir / "summary.json").read_text())
        self.assertIsNone(summary["preselected_worker_target"])
        self.assertEqual(summary["floor_streams_observed"], 4)
        self.assertEqual(summary["receiver_safety_ceiling_observed"], 32)
        with (result.output_dir / "arms.csv").open(newline="") as handle:
            arms = list(csv.DictReader(handle))
        self.assertEqual({row["duration_ms"] for row in arms}, {"8000", "8100"})
        self.assertEqual({row["peak_streams"] for row in arms}, {"5"})
        self.assertEqual({row["final_streams"] for row in arms}, {"4"})
        self.assertEqual({row["sample_count"] for row in arms}, {"12"})
        self.assertEqual({row["sample_observation"] for row in arms}, {"sampled"})
        self.assertEqual(summary["no_sample_arm_count"], 0)
        self.assertEqual(summary["arm_review_count"], 0)
        self.assertTrue((result.output_dir / "dial-samples.csv").is_file())
        with self.assertRaisesRegex(analyzer.AnalysisError, "already exists"):
            self.analyze()

    def test_source_path_must_be_the_registered_physical_fixture(self) -> None:
        self.session.run_rows[1]["source_path"] = "/different/source"
        self.session.write_runs()
        with self.assertRaisesRegex(analyzer.AnalysisError, "registered physical fixture"):
            self.analyze()

    def test_source_manifest_must_use_registered_evidence_path(self) -> None:
        fixture_row = self.session.fixture_rows[0]
        original = fixture_row["source_manifest"]
        relocated = "manifests/source/relocated.csv"
        (self.root / relocated).write_bytes((self.root / original).read_bytes())
        fixture_row["source_manifest"] = relocated
        for row in self.session.run_rows:
            if (
                row["direction"] == fixture_row["direction"]
                and row["fixture"] == fixture_row["fixture"]
            ):
                row["source_manifest"] = relocated
        _write_csv(
            self.root / "fixture-manifests.csv",
            analyzer.FIXTURE_INDEX_FIELDS,
            self.session.fixture_rows,
        )
        self.session.write_runs()
        with self.assertRaisesRegex(analyzer.AnalysisError, "registered evidence path"):
            self.analyze()

    def test_landed_manifest_must_use_registered_evidence_path(self) -> None:
        row = self.session.run_rows[0]
        original = row["landed_manifest"]
        relocated = "manifests/landed/relocated.csv"
        (self.root / relocated).write_bytes((self.root / original).read_bytes())
        row["landed_manifest"] = relocated
        self.session.write_runs()
        with self.assertRaisesRegex(analyzer.AnalysisError, "registered evidence path"):
            self.analyze()

    def test_traces_must_use_registered_endpoint_component_paths(self) -> None:
        row = self.session.run_rows[0]
        original = row["source_trace"]
        relocated = "endpoint/q/ldt4-001/relocated.err"
        (self.root / relocated).write_bytes((self.root / original).read_bytes())
        row["source_trace"] = relocated
        self.session.write_runs()
        with self.assertRaisesRegex(analyzer.AnalysisError, "registered endpoint/component"):
            self.analyze()

    def test_every_archive_path_must_be_its_registered_retained_run_path(self) -> None:
        self.session.run_rows[1]["archive_path"] = self.session.run_rows[0]["archive_path"]
        self.session.write_runs()
        with self.assertRaisesRegex(analyzer.AnalysisError, "retained run path"):
            self.analyze()

    def test_endpoint_session_safe_id_must_equal_evidence_directory(self) -> None:
        for row in self.session.run_rows:
            row["active_destination_path"] = row["active_destination_path"].replace(
                f"/{TEST_SAFE_ID}/", "/different-session/"
            )
            row["archive_path"] = row["archive_path"].replace(
                f"/{TEST_SAFE_ID}/", "/different-session/"
            )
        self.session.write_runs()
        with self.assertRaisesRegex(analyzer.AnalysisError, "evidence directory basename"):
            self.analyze()

    def test_content_hash_change_fails_even_when_path_and_size_match(self) -> None:
        row = self.session.run_rows[0]
        (self.root / row["landed_manifest"]).write_text(
            _manifest_payload("changed"), encoding="ascii"
        )
        with self.assertRaisesRegex(analyzer.AnalysisError, "path, size, or content"):
            self.analyze()

    def test_missing_arm_fails_exact_six_cell_matrix(self) -> None:
        self.session.run_rows.pop()
        self.session.write_runs()
        with self.assertRaisesRegex(analyzer.AnalysisError, "exactly 96"):
            self.analyze()

    def test_abba_first_role_schedule_is_enforced(self) -> None:
        first, second = self.session.run_rows[:2]
        saved_events = {
            (index, role): self.session.read_events(row, role)
            for index, row in enumerate((first, second))
            for role in ("source", "destination")
        }
        first["initiator"] = "destination_init"
        second["initiator"] = "source_init"
        for target_index, template_index in ((0, 1), (1, 0)):
            row = (first, second)[target_index]
            initiator_role = (
                "SOURCE" if row["initiator"] == "source_init" else "DESTINATION"
            )
            source_trace, destination_trace = analyzer._registered_trace_paths(
                row["direction"], row["initiator"], row["run_id"]
            )
            row["source_trace"] = source_trace
            row["destination_trace"] = destination_trace
            for role, trace in (("source", source_trace), ("destination", destination_trace)):
                events = [dict(event) for event in saved_events[(template_index, role)]]
                for event in events:
                    event["run_id"] = row["run_id"]
                    event["session_id"] = row["session_id"]
                    event["initiator_role"] = initiator_role
                self.session._write_trace(trace, events)
        self.session.write_runs()
        with self.assertRaisesRegex(analyzer.AnalysisError, "ABBAABBA"):
            self.analyze()

    def test_run_id_must_equal_its_exact_schedule_identity(self) -> None:
        row = self.session.run_rows[0]
        old_run_id = row["run_id"]
        row["run_id"] = "ldt4-000"
        row["archive_path"] = row["archive_path"].replace(old_run_id, row["run_id"])
        for role in ("source", "destination"):
            events = self.session.read_events(row, role)
            for event in events:
                event["run_id"] = row["run_id"]
            self.session.write_events(row, events, role)
        self.session.write_runs()
        with self.assertRaisesRegex(analyzer.AnalysisError, "exact schedule identity ldt4-001"):
            self.analyze()

    def test_producer_sequence_gap_fails(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        events[3]["producer_seq"] = 99
        # Do not use write_events: this mutation intentionally preserves the gap.
        self.session._write_trace(row["source_trace"], events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "producer_seq"):
            self.analyze()

    def test_missing_staging_manifest_fails_before_analysis_output(self) -> None:
        (self.root / "staging-manifest.csv").rename(
            self.root / "staging-manifest.missing"
        )
        with self.assertRaisesRegex(analyzer.AnalysisError, "staging-manifest.csv"):
            self.analyze()
        self.assertFalse((self.root / "analysis").exists())

    def test_invalid_staged_binary_identity_is_rejected(self) -> None:
        path = self.root / "staging-manifest.csv"
        with path.open(newline="") as handle:
            rows = list(csv.DictReader(handle))
        rows[0]["sha256"] = "A" * 64
        _write_csv(path, analyzer.STAGING_FIELDS, rows)
        with self.assertRaisesRegex(analyzer.AnalysisError, "lowercase 64-hex"):
            self.analyze()

    def test_staged_binary_path_must_match_registered_location(self) -> None:
        path = self.root / "staging-manifest.csv"
        with path.open(newline="") as handle:
            rows = list(csv.DictReader(handle))
        rows[0]["runtime_path"] = "/Users/michael/Dev/blit_v2/target/release/blit"
        _write_csv(path, analyzer.STAGING_FIELDS, rows)
        with self.assertRaisesRegex(analyzer.AnalysisError, "registered staging/runtime"):
            self.analyze()

    def test_artifact_build_must_bind_exact_source_and_lock_identity(self) -> None:
        path = self.root / "artifact-build.txt"
        path.write_text(
            path.read_text(encoding="ascii").replace(
                f"build_id={analyzer.BUILD_ID}", "build_id=wrong"
            ),
            encoding="ascii",
        )
        with self.assertRaisesRegex(analyzer.AnalysisError, "exact accepted build binding"):
            self.analyze()

    def test_schedule_file_must_be_exact_registered_schedule(self) -> None:
        path = self.root / "schedule.csv"
        payload = path.read_text(encoding="ascii")
        path.write_text(payload.replace("001,", "000,", 1), encoding="ascii")
        with self.assertRaisesRegex(analyzer.AnalysisError, "exact registered 96-arm"):
            self.analyze()

    def test_environment_end_gate_must_retain_registered_topology(self) -> None:
        path = self.root / "environment-end.txt"
        path.write_text(
            path.read_text(encoding="ascii").replace("q_mtu=9000", "q_mtu=1500"),
            encoding="ascii",
        )
        with self.assertRaisesRegex(analyzer.AnalysisError, "endpoint gate shape"):
            self.analyze()

    def test_environment_gate_requires_stable_q_identity(self) -> None:
        path = self.root / "environment-start.txt"
        path.write_text(
            path.read_text(encoding="ascii").replace(
                "q_local_hostname=Q", "q_local_hostname=Q.local"
            ),
            encoding="ascii",
        )
        with self.assertRaisesRegex(analyzer.AnalysisError, "endpoint gate shape"):
            self.analyze()

    def test_environment_gate_requires_byte_preserving_powershell(self) -> None:
        path = self.root / "environment-start.txt"
        path.write_text(
            path.read_text(encoding="ascii").replace(
                "windows_powershell=7.6.3", "windows_powershell=7.3.9"
            ),
            encoding="ascii",
        )
        with self.assertRaisesRegex(analyzer.AnalysisError, "PowerShell 7.3.9 is below 7.4"):
            self.analyze()

    def test_environment_gate_binds_duplicate_powershell_evidence(self) -> None:
        path = self.root / "environment-start.txt"
        path.write_text(
            path.read_text(encoding="ascii").replace(
                "00-01-D2-19-04-A3|7.6.3", "00-01-D2-19-04-A3|7.5.0"
            ),
            encoding="ascii",
        )
        with self.assertRaisesRegex(analyzer.AnalysisError, "evidence fields disagree"):
            self.analyze()

    def test_runtime_gate_must_bind_every_pair_boundary_and_quiet_state(self) -> None:
        path = self.root / "runtime-gates.csv"
        with path.open(newline="") as handle:
            rows = list(csv.DictReader(handle))
        rows[0]["q_quiet"] = (
            "q_load1=3.1;q_spotlight_cpu=2.5;"
            "time_machine_auto=0;time_machine_running=0"
        )
        _write_csv(path, analyzer.RUNTIME_GATE_FIELDS, rows)
        with self.assertRaisesRegex(analyzer.AnalysisError, "q_load1 exceeds"):
            self.analyze()

    def test_windows_runtime_swap_must_bind_staged_daemon_hash(self) -> None:
        path = self.root / "windows-runtime-swap.txt"
        path.write_text(
            path.read_text(encoding="ascii").replace(
                "staged_sha=", f"staged_sha={'0' * 64}#", 1
            ),
            encoding="ascii",
        )
        with self.assertRaisesRegex(analyzer.AnalysisError, "malformed runtime identity"):
            self.analyze()

    def test_windows_runtime_restoration_must_prove_original_state(self) -> None:
        path = self.root / "windows-runtime-restoration.txt"
        path.write_text(
            path.read_text(encoding="ascii").replace(
                "active=True", "active=False", 1
            ),
            encoding="ascii",
        )
        with self.assertRaisesRegex(analyzer.AnalysisError, "tested daemon.*restored"):
            self.analyze()

    def test_expected_harness_sha_must_match_provenance(self) -> None:
        other_sha = "2" * 40
        (self.root / "MEASUREMENTS-COMPLETE").write_text(
            f"artifact_sha={analyzer.ARTIFACT_SHA}\n"
            f"harness_sha={other_sha}\n"
            "arm_count=96\n",
            encoding="ascii",
        )
        with self.assertRaisesRegex(analyzer.AnalysisError, "expected reviewed harness"):
            self.analyze(other_sha)

    def test_empty_measurements_complete_marker_is_rejected(self) -> None:
        (self.root / "MEASUREMENTS-COMPLETE").write_bytes(b"")
        with self.assertRaisesRegex(analyzer.AnalysisError, "exact registered binding"):
            self.analyze()

    def test_add_target_must_be_one_stream_step(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        for event in events:
            if event.get("action") == "ADD":
                event["target_streams"] = 6
                if event["event"] == "dial_settlement":
                    event.update(live_streams=6, peak_streams=6)
            elif event.get("action") == "REMOVE":
                event["target_streams"] = 5
                event.update(live_streams=6, peak_streams=6)
                if event["event"] == "dial_settlement":
                    event["live_streams"] = 5
            elif event["event"] == "dial_sample" and event.get("epoch") == 1:
                event.update(
                    live_streams=6,
                    peak_streams=6,
                    sample_streams=6,
                    sample_blocked_ns=600_000_000,
                )
            elif event["event"] == "data_plane_complete":
                event.update(live_streams=5, peak_streams=6)
        destination = self.session.read_events(row, "destination")
        next(event for event in destination if event["event"] == "data_plane_complete").update(
            live_streams=5, peak_streams=6
        )
        self.session.write_events(row, events)
        self.session.write_events(row, destination, "destination")
        with self.assertRaisesRegex(analyzer.AnalysisError, "one-stream step"):
            self.analyze()

    def test_pending_reason_and_action_are_exact(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        pending = next(event for event in events if event["event"] == "dial_pending")
        pending["reason"] = "add"
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "pending event"):
            self.analyze()

    def test_valid_sample_registry_count_must_equal_live_membership(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        sample = next(
            event
            for event in events
            if event["event"] == "dial_sample" and event.get("reason") == "cheap-up"
        )
        sample["sample_streams"] = 3
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "sample_streams must equal"):
            self.analyze()

    def test_zero_stream_rebaseline_is_rejected_without_division_error(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        samples = [
            event
            for event in events
            if event["event"] == "dial_sample" and event.get("reason") == "rebaseline"
        ]
        self.assertEqual([sample["sample_streams"] for sample in samples], [5])
        samples[0]["sample_streams"] = 0
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "new settled membership"):
            self.analyze()

    def test_invalid_first_sample_is_not_a_startup_rebaseline(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        first_sample_index = next(
            index for index, event in enumerate(events) if event["event"] == "dial_sample"
        )
        sample = events[first_sample_index].copy()
        sample.update(
            reason="rebaseline",
            sample_bytes=0,
            sample_blocked_ns=0,
            blocked_ratio=0.0,
            sample_valid=False,
            chunk_bytes=16 * 1024 * 1024,
            prefetch_count=4,
            tcp_buffer_bytes=0,
        )
        events.insert(first_sample_index, sample)
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "no immediately preceding"):
            self.analyze()

    def test_repeated_invalid_rebaseline_is_rejected(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        rebaseline_index = next(
            index
            for index, event in enumerate(events)
            if event["event"] == "dial_sample" and event.get("reason") == "rebaseline"
        )
        events.insert(rebaseline_index + 1, events[rebaseline_index].copy())
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "no immediately preceding"):
            self.analyze()

    def test_first_post_settlement_sample_must_rebaseline(self) -> None:
        row = self.session.run_rows[0]
        events = [
            event
            for event in self.session.read_events(row)
            if not (
                event["event"] == "dial_sample" and event.get("reason") == "rebaseline"
            )
        ]
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "first sample.*not rebaseline"):
            self.analyze()

    def test_sample_interval_cannot_be_shorter_than_production_tick(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        sample = next(event for event in events if event["event"] == "dial_sample")
        sample["sample_elapsed_ns"] = analyzer.DIAL_TUNER_TICK_NS - 1
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "production tuner cadence"):
            self.analyze()

    def test_first_sample_event_cannot_precede_first_tuner_tick(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        latest_attachment_ns = max(
            event["elapsed_ns"]
            for event in events
            if event["event"] == "socket_trace_attached" and event.get("epoch") == 0
        )
        sample = next(event for event in events if event["event"] == "dial_sample")
        sample["elapsed_ns"] = (
            latest_attachment_ns + analyzer.DIAL_TUNER_TICK_NS - 1
        )
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "before one full tuner tick"):
            self.analyze()

    def test_consecutive_sample_events_must_be_one_tick_apart(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        samples = [event for event in events if event["event"] == "dial_sample"]
        samples[1]["elapsed_ns"] = (
            samples[0]["elapsed_ns"] + analyzer.DIAL_TUNER_TICK_NS - 1
        )
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "less than one tuner tick apart"):
            self.analyze()

    def test_sub_tick_zero_sample_arm_is_accepted_and_reported(self) -> None:
        row = self.session.run_rows[0]
        self.make_zero_sample_arm(row, analyzer.DIAL_TUNER_TICK_NS - 1)
        source = self.session.read_events(row)
        latest_attachment_ns = max(
            event["elapsed_ns"]
            for event in source
            if event["event"] == "socket_trace_attached" and event.get("epoch") == 0
        )
        sealed = next(event for event in source if event["event"] == "membership_sealed")
        complete = next(event for event in source if event["event"] == "data_plane_complete")
        self.assertLess(
            sealed["elapsed_ns"] - latest_attachment_ns,
            analyzer.DIAL_TUNER_TICK_NS,
        )
        self.assertGreater(
            complete["elapsed_ns"] - latest_attachment_ns,
            analyzer.DIAL_TUNER_TICK_NS,
        )
        result = self.analyze()
        self.assertEqual(result.status, "REVIEW_REQUIRED")
        summary = json.loads((result.output_dir / "summary.json").read_text())
        self.assertEqual(summary["no_sample_arm_count"], 1)
        with (result.output_dir / "arms.csv").open(newline="") as handle:
            arms = list(csv.DictReader(handle))
        arm = next(item for item in arms if item["run_id"] == row["run_id"])
        self.assertEqual(arm["sample_count"], "0")
        self.assertEqual(arm["sample_observation"], "no-sample")
        with (result.output_dir / "pairs.csv").open(newline="") as handle:
            pairs = list(csv.DictReader(handle))
        pair = next(
            item
            for item in pairs
            if item["cell"] == row["cell"] and item["pair"] == row["pair"]
        )
        self.assertIn("reason_distribution", pair["decision_differences"])
        self.assertIn("operation_sequence", pair["decision_differences"])

    def test_zero_sample_arm_at_first_tick_is_explicit_arm_review(self) -> None:
        row = self.session.run_rows[0]
        self.make_zero_sample_arm(row, analyzer.DIAL_TUNER_TICK_NS)
        result = self.analyze()
        self.assertEqual(result.status, "REVIEW_REQUIRED")
        self.assertEqual(result.arm_review_count, 1)
        summary = json.loads((result.output_dir / "summary.json").read_text())
        self.assertEqual(summary["arm_review_count"], 1)
        with (result.output_dir / "arms.csv").open(newline="") as handle:
            arms = list(csv.DictReader(handle))
        arm = next(item for item in arms if item["run_id"] == row["run_id"])
        self.assertEqual(arm["arm_verdict"], "REVIEW_REQUIRED")
        self.assertEqual(
            json.loads(arm["review_reasons"]),
            ["NO_SAMPLE_AT_OR_AFTER_FIRST_TICK"],
        )

    def test_sub_tick_zero_sample_destination_initiator_is_accepted(self) -> None:
        row = self.session.row(
            direction="q_to_windows",
            fixture="large",
            pair=1,
            initiator="destination_init",
        )
        self.make_zero_sample_arm(row, analyzer.DIAL_TUNER_TICK_NS - 1)
        result = self.analyze()
        self.assertEqual(result.status, "REVIEW_REQUIRED")
        with (result.output_dir / "arms.csv").open(newline="") as handle:
            arms = list(csv.DictReader(handle))
        arm = next(item for item in arms if item["run_id"] == row["run_id"])
        self.assertEqual(arm["sample_observation"], "no-sample")

    def test_policy_replay_rejects_premature_add(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        sample = next(
            event
            for event in events
            if event["event"] == "dial_sample" and event.get("reason") == "cheap-up"
        )
        sample.update(reason="add", action="ADD", target_streams=5, epoch=1)
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "policy replay"):
            self.analyze()

    def test_policy_replay_rejects_wrong_reason(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        sample = next(
            event
            for event in events
            if event["event"] == "dial_sample" and event.get("reason") == "cheap-up"
        )
        sample["reason"] = "hysteresis"
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "sample reason.*policy replay"):
            self.analyze()

    def test_blocked_ratio_cannot_cross_policy_threshold_within_tolerance(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        sample = next(
            event
            for event in events
            if event["event"] == "dial_sample" and event.get("reason") == "cheap-up"
        )
        sample.update(
            sample_elapsed_ns=analyzer.DIAL_TUNER_TICK_NS,
            sample_streams=4,
            sample_blocked_ns=100_000_000,
            blocked_ratio=math.nextafter(analyzer.STEP_UP_THRESHOLD, 0.0),
        )
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "blocked_ratio does not match"):
            self.analyze()

    def test_policy_replay_rejects_wrong_cheap_value(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        sample = next(
            event
            for event in events
            if event["event"] == "dial_sample" and event.get("reason") == "cheap-up"
        )
        sample["chunk_bytes"] = 64 * 1024 * 1024
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "cheap dial snapshot"):
            self.analyze()

    def test_clean_same_build_refusal_is_invalid(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        settlement = next(event for event in events if event["event"] == "dial_settlement")
        settlement.update(accepted=False, reason="refused", live_streams=4, peak_streams=4)
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "refused a resize"):
            self.analyze()

    def test_destination_cannot_emit_dial_policy(self) -> None:
        row = self.session.run_rows[0]
        destination = self.session.read_events(row, "destination")
        source_sample = next(
            event for event in self.session.read_events(row) if event["event"] == "dial_sample"
        ).copy()
        source_sample.update(endpoint_role="DESTINATION")
        destination.append(source_sample)
        self.session.write_events(row, destination, "destination")
        with self.assertRaisesRegex(analyzer.AnalysisError, "not registered for DESTINATION"):
            self.analyze()

    def test_unknown_source_phase_event_is_rejected(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        bogus = next(
            event for event in events if event["event"] == "resize_proposed"
        ).copy()
        bogus["event"] = "resize_bogus"
        events.append(bogus)
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "not registered for SOURCE"):
            self.analyze()

    def test_destination_only_phase_event_is_rejected_on_source(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        wrong_role = next(
            event
            for event in self.session.read_events(row, "destination")
            if event["event"] == "destination_prepared"
        ).copy()
        wrong_role.update(endpoint_role="SOURCE")
        events.append(wrong_role)
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "not registered for SOURCE"):
            self.analyze()

    def test_epoch_zero_topology_follows_initiator_not_byte_direction(self) -> None:
        row = self.session.row(
            direction="q_to_windows", fixture="large", pair=1, initiator="destination_init"
        )
        events = self.session.read_events(row)
        events[0]["event"] = "socket_dial_begin"
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "four accept"):
            self.analyze()

    def test_epoch_zero_socket_end_cannot_precede_begin(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        events[0], events[1] = events[1], events[0]
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "ended before it began"):
            self.analyze()

    def test_accepted_add_requires_resize_socket_on_both_endpoints(self) -> None:
        row = self.session.row(
            direction="q_to_windows", fixture="large", pair=1, initiator="source_init"
        )
        events = [
            event
            for event in self.session.read_events(row)
            if not (
                event["event"] in analyzer.SOCKET_ACQUISITION_EVENTS
                and event.get("epoch") == 1
            )
        ]
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "every accepted ADD"):
            self.analyze()

    def test_resize_socket_transport_follows_initiator_layout(self) -> None:
        row = self.session.row(
            direction="q_to_windows",
            fixture="large",
            pair=1,
            initiator="destination_init",
        )
        events = self.session.read_events(row)
        begin = next(
            event
            for event in events
            if event["event"] == "socket_accept_begin" and event.get("epoch") == 1
        )
        begin["event"] = "socket_dial_begin"
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "exact accept begin/end"):
            self.analyze()

    def test_remove_epoch_cannot_acquire_a_socket(self) -> None:
        row = self.session.row(
            direction="q_to_windows", fixture="large", pair=1, initiator="source_init"
        )
        events = self.session.read_events(row)
        template = [
            event.copy()
            for event in events
            if event["event"] in {"socket_dial_begin", "socket_dial_end"}
            and event.get("epoch") == 1
        ]
        for event in template:
            event["epoch"] = 2
        completion_index = next(
            index
            for index, event in enumerate(events)
            if event["event"] == "data_plane_complete"
        )
        events[completion_index:completion_index] = template
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "non-ADD/unknown epoch 2"):
            self.analyze()

    def test_dial_observation_cannot_follow_data_plane_completion(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        completion = next(
            event for event in events if event["event"] == "data_plane_complete"
        )
        events.remove(completion)
        payload = [
            event
            for event in events
            if event["event"]
            in {"first_payload_queued", "socket_write_begin", "first_socket_write"}
        ]
        events = [event for event in events if event not in payload]
        first_dial = next(
            index for index, event in enumerate(events) if event["event"].startswith("dial_")
        )
        events[first_dial:first_dial] = payload + [completion]
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "after SOURCE completion"):
            self.analyze()

    def test_source_destination_completion_counts_must_match(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row, "destination")
        complete = next(event for event in events if event["event"] == "data_plane_complete")
        complete["live_streams"] = 3
        self.session.write_events(row, events, "destination")
        with self.assertRaisesRegex(analyzer.AnalysisError, "final membership differs"):
            self.analyze()

    def test_source_membership_sealed_must_match_dial_lifecycle(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        sealed = next(event for event in events if event["event"] == "membership_sealed")
        sealed["peak_streams"] = 4
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "values differ from dial lifecycle"):
            self.analyze()

    def test_source_membership_sealed_must_precede_completion(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        sealed_index = next(
            index for index, event in enumerate(events) if event["event"] == "membership_sealed"
        )
        complete_index = next(
            index
            for index, event in enumerate(events)
            if event["event"] == "data_plane_complete"
        )
        sealed = events.pop(sealed_index)
        if sealed_index < complete_index:
            complete_index -= 1
        events.insert(complete_index + 1, sealed)
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "did not precede completion"):
            self.analyze()

    def test_accepted_operation_requires_complete_source_control_lane(self) -> None:
        row = self.session.run_rows[0]
        events = [
            event
            for event in self.session.read_events(row)
            if not (
                event["event"] == "resize_ack_received" and event.get("epoch") == 1
            )
        ]
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "control-lane event inventory"):
            self.analyze()

    def test_destination_prepared_values_must_match_add(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row, "destination")
        prepared = next(
            event
            for event in events
            if event["event"] == "destination_prepared" and event.get("epoch") == 1
        )
        prepared["action"] = "add"
        self.session.write_events(row, events, "destination")
        with self.assertRaisesRegex(analyzer.AnalysisError, "ADD preparation is invalid"):
            self.analyze()

    def test_every_member_requires_exact_socket_trace_attachment(self) -> None:
        row = self.session.run_rows[0]
        events = [
            event
            for event in self.session.read_events(row, "destination")
            if not (
                event["event"] == "socket_trace_attached"
                and event.get("epoch") == 0
                and event.get("socket") == 0
            )
        ]
        self.session.write_events(row, events, "destination")
        with self.assertRaisesRegex(analyzer.AnalysisError, "socket_trace_attached"):
            self.analyze()

    def test_every_receive_member_must_stop_before_completion(self) -> None:
        row = self.session.run_rows[0]
        events = [
            event
            for event in self.session.read_events(row, "destination")
            if not (
                event["event"] == "receive_task_stopped"
                and event.get("epoch") == 0
                and event.get("socket") == 0
            )
        ]
        self.session.write_events(row, events, "destination")
        with self.assertRaisesRegex(analyzer.AnalysisError, "receive_task_stopped"):
            self.analyze()

    def test_payload_socket_markers_are_required(self) -> None:
        row = self.session.run_rows[0]
        source = [
            event
            for event in self.session.read_events(row)
            if event["event"] not in {"socket_write_begin", "first_socket_write"}
        ]
        destination = [
            event
            for event in self.session.read_events(row, "destination")
            if event["event"] != "first_payload_received"
        ]
        self.session.write_events(row, source)
        self.session.write_events(row, destination, "destination")
        with self.assertRaisesRegex(analyzer.AnalysisError, "no payload socket markers"):
            self.analyze()

    def test_payload_marker_for_unknown_socket_is_rejected(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        complete_index = next(
            index
            for index, event in enumerate(events)
            if event["event"] == "data_plane_complete"
        )
        unknown = self.session._event(
            row["run_id"],
            row["session_id"],
            "SOURCE",
            "SOURCE",
            "socket_write_begin",
            epoch=0,
            socket=99,
        )
        unknown["elapsed_ns"] = events[complete_index]["elapsed_ns"] + 1
        events.insert(complete_index + 1, unknown)
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "marker keys differ"):
            self.analyze()

    def test_payload_marker_duplicate_is_rejected(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        index = next(
            index
            for index, event in enumerate(events)
            if event["event"] == "first_socket_write"
        )
        events.insert(index + 1, dict(events[index]))
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "duplicate first_socket_write"):
            self.analyze()

    def test_payload_write_must_precede_source_completion(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row)
        write = next(event for event in events if event["event"] == "first_socket_write")
        events.remove(write)
        complete_index = next(
            index
            for index, event in enumerate(events)
            if event["event"] == "data_plane_complete"
        )
        events.insert(complete_index + 1, write)
        self.session.write_events(row, events)
        with self.assertRaisesRegex(analyzer.AnalysisError, "payload write ordering"):
            self.analyze()

    def test_payload_receive_must_precede_receive_task_stop(self) -> None:
        row = self.session.run_rows[0]
        events = self.session.read_events(row, "destination")
        received = next(
            event for event in events if event["event"] == "first_payload_received"
        )
        events.remove(received)
        stop_index = next(
            index
            for index, event in enumerate(events)
            if event["event"] == "receive_task_stopped"
            and event.get("epoch") == 0
            and event.get("socket") == 0
        )
        events.insert(stop_index + 1, received)
        self.session.write_events(row, events, "destination")
        with self.assertRaisesRegex(analyzer.AnalysisError, "payload receive ordering"):
            self.analyze()

    def test_destination_initiator_attaches_add_socket_before_prepared_ack(self) -> None:
        row = self.session.row(
            direction="q_to_windows",
            fixture="large",
            pair=1,
            initiator="destination_init",
        )
        events = self.session.read_events(row, "destination")
        attached_index = next(
            index
            for index, event in enumerate(events)
            if event["event"] == "socket_trace_attached" and event.get("epoch") == 1
        )
        prepared_index = next(
            index
            for index, event in enumerate(events)
            if event["event"] == "destination_prepared" and event.get("epoch") == 1
        )
        events[attached_index], events[prepared_index] = (
            events[prepared_index],
            events[attached_index],
        )
        self.session.write_events(row, events, "destination")
        with self.assertRaisesRegex(analyzer.AnalysisError, "initiator ADD ordering"):
            self.analyze()

    def test_source_initiator_add_requires_responder_arm_events(self) -> None:
        row = self.session.row(
            direction="q_to_windows", fixture="large", pair=1, initiator="source_init"
        )
        events = [
            event
            for event in self.session.read_events(row, "destination")
            if not (
                event["event"] == "resize_arm_ready" and event.get("epoch") == 1
            )
        ]
        self.session.write_events(row, events, "destination")
        with self.assertRaisesRegex(analyzer.AnalysisError, "control-lane event inventory"):
            self.analyze()

    def test_decision_distribution_difference_is_review_required_not_rejected(self) -> None:
        row = self.session.row(
            direction="q_to_windows", fixture="large", pair=1, initiator="destination_init"
        )
        events = self.session.read_events(row)
        rebaseline_index = next(
            index
            for index, event in enumerate(events)
            if event["event"] == "dial_sample" and event.get("reason") == "rebaseline"
        )
        idle = events[rebaseline_index].copy()
        idle.update(
            reason="idle",
            sample_valid=True,
            sample_streams=idle["live_streams"],
            elapsed_ns=idle["elapsed_ns"] + analyzer.DIAL_TUNER_TICK_NS,
        )
        for event in events[rebaseline_index + 1 :]:
            event["elapsed_ns"] += analyzer.DIAL_TUNER_TICK_NS
        events.insert(rebaseline_index + 1, idle)
        self.session.write_events(row, events)
        result = self.analyze()
        self.assertEqual(result.status, "REVIEW_REQUIRED")
        self.assertEqual(result.decision_review_count, 1)
        with (result.output_dir / "pairs.csv").open(newline="") as handle:
            pairs = list(csv.DictReader(handle))
        changed = next(
            item for item in pairs if item["cell"] == "q_to_windows_large" and item["pair"] == "1"
        )
        self.assertIn("reason_distribution", changed["decision_differences"])

    def test_ordered_reason_difference_requires_review_with_equal_counts_and_ops(self) -> None:
        source_row = self.session.row(
            direction="q_to_windows",
            fixture="large",
            pair=1,
            initiator="source_init",
        )
        source_events = self.session.read_events(source_row)
        rebaseline_index = next(
            index
            for index, event in enumerate(source_events)
            if event["event"] == "dial_sample" and event.get("reason") == "rebaseline"
        )
        source_idle = source_events[rebaseline_index].copy()
        source_idle.update(
            reason="idle",
            sample_valid=True,
            sample_streams=source_idle["live_streams"],
            elapsed_ns=source_idle["elapsed_ns"] + analyzer.DIAL_TUNER_TICK_NS,
        )
        for event in source_events[rebaseline_index + 1 :]:
            event["elapsed_ns"] += analyzer.DIAL_TUNER_TICK_NS
        source_events.insert(rebaseline_index + 1, source_idle)
        self.session.write_events(source_row, source_events)

        destination_row = self.session.row(
            direction="q_to_windows",
            fixture="large",
            pair=1,
            initiator="destination_init",
        )
        destination_events = self.session.read_events(destination_row)
        first_sample_index = next(
            index
            for index, event in enumerate(destination_events)
            if event["event"] == "dial_sample"
        )
        first_sample = destination_events[first_sample_index]
        destination_idle = first_sample.copy()
        destination_idle.update(
            reason="idle",
            sample_bytes=0,
            sample_blocked_ns=0,
            sample_streams=4,
            sample_valid=True,
            blocked_ratio=0.0,
            chunk_bytes=analyzer.FLOOR_CHUNK_BYTES,
            prefetch_count=analyzer.FLOOR_PREFETCH,
            tcp_buffer_bytes=0,
        )
        for event in destination_events[first_sample_index:]:
            event["elapsed_ns"] += analyzer.DIAL_TUNER_TICK_NS
        destination_events.insert(first_sample_index, destination_idle)
        self.session.write_events(destination_row, destination_events)

        result = self.analyze()
        self.assertEqual(result.status, "REVIEW_REQUIRED")
        with (result.output_dir / "pairs.csv").open(newline="") as handle:
            pairs = list(csv.DictReader(handle))
        changed = next(
            item
            for item in pairs
            if item["cell"] == "q_to_windows_large" and item["pair"] == "1"
        )
        self.assertEqual(changed["decision_differences"], "reason_sequence")
        self.assertEqual(
            sorted(json.loads(changed["source_reason_sequence"])),
            sorted(json.loads(changed["destination_reason_sequence"])),
        )
        self.assertEqual(changed["source_operations"], changed["destination_operations"])

    def test_client_completion_cannot_exceed_wrapper_duration_beyond_rounding(self) -> None:
        self.session.run_rows[0]["duration_ms"] = "1"
        self.session.write_runs()
        with self.assertRaisesRegex(
            analyzer.AnalysisError, "client data_plane_complete.*exceeds runs.csv duration_ms"
        ):
            self.analyze()

    def test_performance_uses_durable_median_ratio_without_worker_target(self) -> None:
        for row in self.session.run_rows:
            if row["cell"] == "q_to_windows_large" and row["initiator"] == "destination_init":
                row["duration_ms"] = "12000"
        self.session.write_runs()
        result = self.analyze()
        self.assertEqual(result.performance_review_count, 1)
        summary = json.loads((result.output_dir / "summary.json").read_text())
        cell = next(item for item in summary["cells"] if item["cell"] == "q_to_windows_large")
        self.assertEqual(cell["performance_verdict"], "REVIEW_REQUIRED")
        self.assertIsNone(summary["preselected_worker_target"])

    def test_dedicated_trace_rejects_a_foreign_session_event(self) -> None:
        row = self.session.run_rows[0]
        path = self.root / row["source_trace"]
        unrelated = {
            "schema": 99,
            "session_id": "f" * 16,
            "event": "unrelated",
        }
        with path.open("a", encoding="utf-8") as handle:
            handle.write(f"{analyzer.TRACE_PREFIX}{json.dumps(unrelated)}\n")
        with self.assertRaisesRegex(analyzer.AnalysisError, "foreign session"):
            self.analyze()

    def test_trace_rejects_duplicate_json_keys_before_last_wins_collapse(self) -> None:
        row = self.session.run_rows[0]
        path = self.root / row["source_trace"]
        text = path.read_text(encoding="utf-8")
        needle = '"reason":"cheap-up"'
        self.assertIn(needle, text)
        path.write_text(
            text.replace(
                needle,
                '"reason":"hysteresis","reason":"cheap-up"',
                1,
            ),
            encoding="utf-8",
        )
        with self.assertRaisesRegex(analyzer.AnalysisError, "duplicate JSON key 'reason'"):
            self.analyze()

    def test_input_inventory_hashes_every_pre_analysis_plain_file(self) -> None:
        runs_sha = hashlib.sha256((self.root / "runs.csv").read_bytes()).hexdigest()
        expected_count = sum(path.is_file() for path in self.root.rglob("*"))
        result = self.analyze()
        inventory_path = result.output_dir / "input-files.csv"
        with inventory_path.open(newline="") as handle:
            inventory = list(csv.DictReader(handle))
        self.assertEqual(len(inventory), expected_count)
        self.assertEqual(
            [row["path"] for row in inventory],
            sorted(row["path"] for row in inventory),
        )
        self.assertFalse(any(row["path"].startswith("analysis/") for row in inventory))
        runs = next(row for row in inventory if row["path"] == "runs.csv")
        self.assertEqual(runs["sha256"], runs_sha)
        summary = json.loads((result.output_dir / "summary.json").read_text())
        self.assertEqual(summary["input_file_count"], expected_count)
        self.assertEqual(
            summary["input_inventory_sha256"],
            hashlib.sha256(inventory_path.read_bytes()).hexdigest(),
        )

    def test_existing_analysis_marker_is_never_modified(self) -> None:
        analysis = self.root / "analysis"
        analysis.mkdir()
        marker = analysis / "owner.txt"
        marker.write_text("keep", encoding="utf-8")
        with self.assertRaisesRegex(analyzer.AnalysisError, "never overwritten"):
            self.analyze()
        self.assertEqual(marker.read_text(encoding="utf-8"), "keep")


if __name__ == "__main__":
    unittest.main()
