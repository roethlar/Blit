#!/usr/bin/env python3
"""Synthetic guards for otp12pf_rigw_analyze.py."""

from __future__ import annotations

import base64
import csv
import hashlib
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("otp12pf_rigw_analyze.py")
SPEC = importlib.util.spec_from_file_location("otp12pf_rigw_analyze", MODULE_PATH)
assert SPEC and SPEC.loader
analyzer = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = analyzer
SPEC.loader.exec_module(analyzer)


class SyntheticSession:
    def __init__(self, root: Path) -> None:
        self.root = root
        self.rows: list[dict[str, str]] = []
        self.events: list[dict[str, object]] = []
        self.clock_rows: list[dict[str, str]] = []
        self._session_counter = 1
        self._build()
        self._build_manifest_evidence()
        self._build_clock_samples()
        self.write()

    @staticmethod
    def _delta(trace_state: str, cell: str, pair: int) -> int:
        if cell != analyzer.TARGET_CELL:
            return 5
        if trace_state == "off":
            return (10, 20, 30, 40, 50, 60, 70, 80)[pair - 1]
        return (20, 20, 20, 20, 30, 30, 30, 30)[pair - 1]

    def _trace_events(
        self, run_id: str, session_id: str, scheduled_role: str
    ) -> list[dict[str, object]]:
        initiator = "SOURCE" if scheduled_role == "source_init" else "DESTINATION"
        source_action = "dial" if initiator == "SOURCE" else "accept"
        destination_action = "accept" if initiator == "SOURCE" else "dial"

        def event(
            endpoint_role: str,
            seq: int,
            elapsed: int,
            name: str,
            **extra: object,
        ) -> dict[str, object]:
            value: dict[str, object] = {
                "schema": 1,
                "run_id": run_id,
                "session_id": session_id,
                "producer_seq": seq,
                "unix_ns": 1_000_000 + elapsed,
                "elapsed_ns": elapsed,
                "endpoint_role": endpoint_role,
                "initiator_role": initiator,
                "event": name,
            }
            value.update(extra)
            return value

        source: list[dict[str, object]] = []
        destination: list[dict[str, object]] = []

        def source_event(name: str, **extra: object) -> None:
            seq = len(source)
            source.append(event("SOURCE", seq, seq, name, **extra))

        def destination_event(name: str, **extra: object) -> None:
            seq = len(destination)
            destination.append(event("DESTINATION", seq, seq, name, **extra))

        source_event(f"socket_{source_action}_begin", epoch=0, socket=0)
        source_event(f"socket_{source_action}_end", epoch=0, socket=0)
        source_event("socket_trace_attached", epoch=0, socket=0)
        source_event("manifest_complete_send_begin")
        source_event("manifest_complete_sent", count=1)
        source_event("need_batch_received", batch=0, count=1)
        source_event("planner_begin", batch=0, count=1)
        source_event("planner_end", batch=0, count=1)
        for epoch in range(1, 8):
            target = epoch + 1
            source_event(
                "resize_proposed",
                epoch=epoch,
                target_streams=target,
                live_streams=epoch,
            )
            source_event(
                "resize_send_begin",
                epoch=epoch,
                target_streams=target,
                live_streams=epoch,
            )
            source_event(
                "resize_sent",
                epoch=epoch,
                target_streams=target,
                live_streams=epoch,
            )
            source_event(
                "resize_ack_received",
                epoch=epoch,
                accepted=True,
                live_streams=target,
            )
            source_event(f"socket_{source_action}_begin", epoch=epoch, socket=0)
            source_event(f"socket_{source_action}_end", epoch=epoch, socket=0)
            source_event("socket_trace_attached", epoch=epoch, socket=0)
            source_event(
                "source_settled",
                epoch=epoch,
                target_streams=target,
                live_streams=target,
                accepted=True,
            )
        source_event("first_payload_queued")
        source_event("socket_write_begin", epoch=0, socket=0)
        source_event("first_socket_write", epoch=0, socket=0)
        source_event("data_plane_complete")
        source_event("summary_received")

        destination_event(f"socket_{destination_action}_begin", epoch=0, socket=0)
        destination_event(f"socket_{destination_action}_end", epoch=0, socket=0)
        destination_event("socket_trace_attached", epoch=0, socket=0)
        destination_event("manifest_complete_received")
        destination_event("need_batch_send_begin", batch=0, count=1)
        destination_event("need_batch_sent", batch=0, count=1)
        for epoch in range(1, 8):
            target = epoch + 1
            destination_event(
                "resize_received",
                epoch=epoch,
                target_streams=target,
                live_streams=epoch,
            )
            if initiator == "SOURCE":
                destination_event(
                    "resize_arm_queue_begin",
                    epoch=epoch,
                    target_streams=target,
                )
                destination_event(
                    "destination_prepared",
                    epoch=epoch,
                    target_streams=target,
                    action="arm_queued",
                )
                destination_event(
                    "resize_ack_send_begin",
                    epoch=epoch,
                    accepted=True,
                    live_streams=target,
                )
                destination_event(
                    "resize_ack_sent",
                    epoch=epoch,
                    accepted=True,
                    live_streams=target,
                )
                destination_event("resize_arm_ready", epoch=epoch)
                destination_event("socket_accept_begin", epoch=epoch, socket=0)
                destination_event("socket_accept_end", epoch=epoch, socket=0)
                destination_event("socket_trace_attached", epoch=epoch, socket=0)
            else:
                destination_event("socket_dial_begin", epoch=epoch, socket=0)
                destination_event("socket_dial_end", epoch=epoch, socket=0)
                destination_event("socket_trace_attached", epoch=epoch, socket=0)
                destination_event(
                    "destination_prepared",
                    epoch=epoch,
                    target_streams=target,
                    action="dial_complete",
                )
                destination_event(
                    "resize_ack_send_begin",
                    epoch=epoch,
                    accepted=True,
                    live_streams=target,
                )
                destination_event(
                    "resize_ack_sent",
                    epoch=epoch,
                    accepted=True,
                    live_streams=target,
                )
        destination_event("first_payload_received", epoch=0, socket=0)
        destination_event("data_plane_complete")
        destination_event("summary_send_begin")
        destination_event("summary_sent")
        return source + destination

    def _build(self) -> None:
        client_dir = self.root / "client"
        client_dir.mkdir(parents=True)
        for block in analyzer.BLOCKS:
            run_id = f"rigw-block-{block.number}"
            for round_index, pair in enumerate(block.pairs):
                cells = block.cells if round_index in (0, 3) else tuple(reversed(block.cells))
                for cell in cells:
                    for role_order, role in enumerate(
                        analyzer.expected_roles(pair), start=1
                    ):
                        source_ms = 100
                        transfer_ms = (
                            source_ms
                            if role == "source_init"
                            else source_ms + self._delta(block.trace_state, cell, pair)
                        )
                        settled_ms = 250
                        flush_ms = 1
                        client_log = (
                            f"client/b{block.number}-{cell}-p{pair}-{role}.log"
                        )
                        (self.root / client_log).write_text("synthetic client log\n")
                        traced_tcp = block.trace_state == "on" and cell in analyzer.TCP_CELLS
                        session_id = ""
                        if traced_tcp:
                            session_id = f"{self._session_counter:016x}"
                            self._session_counter += 1
                            self.events.extend(self._trace_events(run_id, session_id, role))
                        self.rows.append(
                            {
                                "block": str(block.number),
                                "trace_state": block.trace_state,
                                "pass": block.pass_name,
                                "cell": cell,
                                "role": role,
                                "pair": str(pair),
                                "role_order": str(role_order),
                                "transfer_ms": str(transfer_ms),
                                "settled_ms": str(settled_ms),
                                "flush_ms": str(flush_ms),
                                "total_ms": str(
                                    transfer_ms
                                    + settled_ms
                                    - analyzer.SETTLE_MIN_MS
                                    + flush_ms
                                ),
                                "exit": "0",
                                "drain": "drained",
                                "valid": "yes",
                                "run_id": run_id,
                                "session_id": session_id,
                                "client_log": client_log,
                            }
                        )

    def _build_clock_samples(self) -> None:
        q_clock = 1_000_000_000
        for row in self.rows:
            for phase in ("before", "after"):
                for sample in range(1, 4):
                    rtt = 10 + sample
                    q_before = q_clock
                    q_after = q_before + rtt
                    offset = int(row["block"]) * 1_000 + (100 if phase == "after" else 0)
                    midpoint = q_before + rtt // 2
                    windows = midpoint + offset
                    self.clock_rows.append(
                        {
                            "block": row["block"],
                            "run_id": row["run_id"],
                            "cell": row["cell"],
                            "pair": row["pair"],
                            "role": row["role"],
                            "phase": phase,
                            "sample": str(sample),
                            "q_before_ns": str(q_before),
                            "windows_ns": str(windows),
                            "q_after_ns": str(q_after),
                            "rtt_ns": str(rtt),
                            "offset_windows_minus_q_ns": str(offset),
                        }
                    )
                    q_clock = q_after + 100

    @staticmethod
    def _manifest_data(shape: str) -> bytes:
        entries = (
            (("a.txt", 1), ("sub/b.txt", 2))
            if shape == "mixed"
            else (("large.bin", 3),)
        )
        lines = sorted(
            f"{base64.b64encode(path.encode()).decode()},{size}"
            for path, size in entries
        )
        return "".join(f"{line}\n" for line in lines).encode("ascii")

    def _build_manifest_evidence(self) -> None:
        fixtures = self.root / "fixtures"
        landed = self.root / "landed"
        fixtures.mkdir()
        landed.mkdir()
        index_rows: list[dict[str, str]] = []
        fixture_data: dict[str, tuple[bytes, str]] = {}
        for shape in ("mixed", "large"):
            data = self._manifest_data(shape)
            digest = hashlib.sha256(data).hexdigest()
            q_relative = f"fixtures/src_{shape}.manifest"
            win_relative = f"fixtures/windows-src_{shape}.manifest"
            (self.root / q_relative).write_bytes(data)
            (self.root / win_relative).write_bytes(data)
            index_rows.append(
                {
                    "shape": shape,
                    "sha256": digest,
                    "q_manifest": q_relative,
                    "windows_manifest": win_relative,
                }
            )
            fixture_data[shape] = (data, digest)
        with (self.root / "fixture-manifests.csv").open("w", newline="") as handle:
            writer = csv.DictWriter(
                handle,
                fieldnames=("shape", "sha256", "q_manifest", "windows_manifest"),
            )
            writer.writeheader()
            writer.writerows(index_rows)
        for row in self.rows:
            shape = row["cell"].rsplit("_", 1)[1]
            data, digest = fixture_data[shape]
            row["landed_root"] = f"src_{shape}"
            row["tree_manifest_sha256"] = digest
            rid = (
                f"b{row['block']}_{row['cell']}_p{row['pair']}_{row['role']}"
            )
            (landed / f"{rid}.manifest").write_bytes(data)

    def write(self) -> None:
        with (self.root / "runs.csv").open("w", newline="") as handle:
            writer = csv.DictWriter(handle, fieldnames=analyzer.CSV_FIELDS)
            writer.writeheader()
            writer.writerows(self.rows)
        with (self.root / "clock-samples.csv").open("w", newline="") as handle:
            writer = csv.DictWriter(handle, fieldnames=analyzer.CLOCK_FIELDS)
            writer.writeheader()
            writer.writerows(self.clock_rows)
        trace = self.root / "trace" / "nested"
        trace.mkdir(parents=True, exist_ok=True)
        client_by_session = {
            row["session_id"]: self.root / row["client_log"]
            for row in self.rows
            if row["session_id"]
        }
        for client_log in client_by_session.values():
            client_log.write_text("synthetic client log\n")
        with (trace / "daemon.log").open("w") as handle:
            handle.write("ignored daemon line\n")
            for event in self.events:
                line = analyzer.TRACE_PREFIX + json.dumps(event, sort_keys=True) + "\n"
                if (
                    event["endpoint_role"] == event["initiator_role"]
                    and event["session_id"] in client_by_session
                ):
                    with client_by_session[event["session_id"]].open("a") as client_handle:
                        client_handle.write(line)
                else:
                    handle.write(line)


class RigWAnalyzerTests(unittest.TestCase):
    def make_session(self) -> tuple[tempfile.TemporaryDirectory[str], SyntheticSession]:
        temporary = tempfile.TemporaryDirectory()
        return temporary, SyntheticSession(Path(temporary.name))

    @staticmethod
    def traced_session_id(session: SyntheticSession, initiator_role: str) -> str:
        return str(
            next(
                event["session_id"]
                for event in session.events
                if event["initiator_role"] == initiator_role
            )
        )

    @staticmethod
    def phase_event(
        session: SyntheticSession,
        session_id: str,
        endpoint_role: str,
        event_name: str,
        epoch: int | None,
    ) -> dict[str, object]:
        return next(
            event
            for event in session.events
            if event["session_id"] == session_id
            and event["endpoint_role"] == endpoint_role
            and event["event"] == event_name
            and event.get("epoch") == epoch
        )

    @staticmethod
    def reorder_local_events(desired_order: list[dict[str, object]]) -> None:
        fields = ("producer_seq", "elapsed_ns", "unix_ns")
        assert len({event["session_id"] for event in desired_order}) == 1
        assert len({event["endpoint_role"] for event in desired_order}) == 1
        slots = sorted(
            (tuple(event[field] for field in fields) for event in desired_order),
            key=lambda slot: int(slot[0]),
        )
        for event, slot in zip(desired_order, slots):
            for field, value in zip(fields, slot):
                event[field] = value

    def test_complete_schedule_exact_floor_bias_and_exports(self) -> None:
        temporary, session = self.make_session()
        self.addCleanup(temporary.cleanup)
        result = analyzer.analyze(session.root)
        self.assertEqual(str(result.observer_bias), "20")
        self.assertEqual(str(result.n_resolution), "70")
        with result.summary_csv.open(newline="") as handle:
            rows = {
                (row["cell"], row["trace_state"]): row
                for row in csv.DictReader(handle)
            }
        off = rows[(analyzer.TARGET_CELL, "off")]
        on = rows[(analyzer.TARGET_CELL, "on")]
        self.assertEqual(off["measurand"], "durable_total_ms")
        self.assertEqual(off["delta_ms"], "45")
        self.assertEqual(off["paired_delta_median_ms"], "45")
        self.assertEqual(off["first4_delta_median_ms"], "25")
        self.assertEqual(off["last4_delta_median_ms"], "65")
        self.assertEqual(off["first_last_drift_ms"], "40")
        self.assertEqual(off["odd_even_drift_ms"], "10")
        self.assertEqual(off["source_first_delta_median_ms"], "45")
        self.assertEqual(off["destination_first_delta_median_ms"], "45")
        self.assertEqual(off["role_order_drift_ms"], "0")
        self.assertEqual(off["n_pair_split_ms"], "40")
        self.assertEqual(off["paired_delta_range_ms"], "70")
        self.assertEqual(off["n_pair_ms"], "70")
        self.assertEqual(on["delta_ms"], "25")
        self.assertEqual(on["n_pair_split_ms"], "10")
        self.assertEqual(on["n_pair_ms"], "10")
        self.assertEqual(on["observer_bias_ms"], "20")
        self.assertEqual(on["n_resolution_ms"], "70")
        self.assertTrue(result.summary_md.is_file())
        self.assertTrue(result.distributions_csv.is_file())
        with result.clock_summary_csv.open(newline="") as handle:
            clocks = list(csv.DictReader(handle))
        self.assertEqual(len(clocks), 128)
        self.assertTrue(all(row["before_sample"] == "1" for row in clocks))
        self.assertTrue(all(row["after_sample"] == "1" for row in clocks))
        self.assertTrue(all(row["selected_offset_change_ns"] == "100" for row in clocks))
        with result.phase_events_csv.open(newline="") as handle:
            phase_rows = list(csv.DictReader(handle))
        self.assertEqual(len(phase_rows), len(session.events))
        self.assertTrue(any(row["source_file"].startswith("client/") for row in phase_rows))
        self.assertTrue(any(row["source_file"].startswith("trace/") for row in phase_rows))
        self.assertTrue(
            all(
                row["total_ms"]
                == str(
                    int(row["transfer_ms"])
                    + int(row["settled_ms"])
                    - analyzer.SETTLE_MIN_MS
                    + int(row["flush_ms"])
                )
                for row in phase_rows
            )
        )
        with result.phase_intervals_csv.open(newline="") as handle:
            intervals = list(csv.DictReader(handle))
        self.assertTrue(intervals)
        self.assertTrue(all(int(row["duration_ns"]) >= 0 for row in intervals))
        self.assertTrue(all(row["endpoint_role"] in {"SOURCE", "DESTINATION"} for row in intervals))

    def test_registered_schedule_is_pair_outer_with_reverse_block_bases(self) -> None:
        schedule = analyzer.expected_schedule()

        def cells_for(block_number: int, pair: int) -> list[str]:
            return [
                cell
                for block, cell, scheduled_pair, _role, role_order in schedule
                if block.number == block_number
                and scheduled_pair == pair
                and role_order == 1
            ]

        base = list(analyzer.CELLS)
        reverse = list(reversed(base))
        self.assertEqual(cells_for(1, 1), base)
        self.assertEqual(cells_for(1, 2), reverse)
        self.assertEqual(cells_for(2, 1), reverse)
        self.assertEqual(cells_for(2, 2), base)
        self.assertEqual(cells_for(3, 5), base)
        self.assertEqual(cells_for(4, 5), reverse)
        self.assertEqual(
            [
                role
                for block, _cell, pair, role, role_order in schedule
                if block.number == 1 and role_order == 1 and _cell == base[0]
            ],
            ["source_init", "destination_init", "destination_init", "source_init"],
        )

    def test_missing_trace_endpoint_is_rejected(self) -> None:
        temporary, session = self.make_session()
        self.addCleanup(temporary.cleanup)
        first_session = next(event["session_id"] for event in session.events)
        session.events = [
            event
            for event in session.events
            if not (
                event["session_id"] == first_session
                and event["endpoint_role"] == "DESTINATION"
            )
        ]
        session.write()
        with self.assertRaisesRegex(analyzer.AnalysisError, "missing endpoint role"):
            analyzer.analyze(session.root)

    def test_trace_off_leak_is_rejected(self) -> None:
        temporary, session = self.make_session()
        self.addCleanup(temporary.cleanup)
        leaked = dict(session.events[0])
        leaked["run_id"] = "rigw-block-1"
        leaked["session_id"] = "ffffffffffffffff"
        session.events.append(leaked)
        session.write()
        with self.assertRaisesRegex(analyzer.AnalysisError, "trace leak: trace-off block 1"):
            analyzer.analyze(session.root)

    def test_grpc_trace_leak_is_rejected(self) -> None:
        temporary, session = self.make_session()
        self.addCleanup(temporary.cleanup)
        leaked = dict(session.events[0])
        leaked["run_id"] = "rigw-block-2"
        leaked["session_id"] = "eeeeeeeeeeeeeeee"
        session.events.append(leaked)
        session.write()
        with self.assertRaisesRegex(analyzer.AnalysisError, "possible gRPC"):
            analyzer.analyze(session.root)

    def test_schedule_mismatch_is_rejected(self) -> None:
        temporary, session = self.make_session()
        self.addCleanup(temporary.cleanup)
        session.rows[0]["cell"] = "wm_tcp_large"
        session.write()
        with self.assertRaisesRegex(analyzer.AnalysisError, "schedule mismatch"):
            analyzer.analyze(session.root)

    def test_settled_ms_schema_and_bounds_are_fail_closed(self) -> None:
        for value in ("249", "1000", "not-an-integer"):
            with self.subTest(settled_ms=value):
                temporary, session = self.make_session()
                self.addCleanup(temporary.cleanup)
                session.rows[0]["settled_ms"] = value
                session.write()
                with self.assertRaisesRegex(analyzer.AnalysisError, "settled_ms"):
                    analyzer.analyze(session.root)

        temporary, session = self.make_session()
        self.addCleanup(temporary.cleanup)
        with (session.root / "runs.csv").open() as handle:
            lines = handle.readlines()
        lines[0] = lines[0].replace("settled_ms,", "")
        (session.root / "runs.csv").write_text("".join(lines))
        with self.assertRaisesRegex(analyzer.AnalysisError, "header mismatch"):
            analyzer.analyze(session.root)

    def test_corrupt_total_is_rejected(self) -> None:
        temporary, session = self.make_session()
        self.addCleanup(temporary.cleanup)
        session.rows[0]["total_ms"] = "999"
        session.write()
        with self.assertRaisesRegex(
            analyzer.AnalysisError,
            "total_ms must equal transfer_ms \\+ \\(settled_ms - 250\\) \\+ flush_ms",
        ):
            analyzer.analyze(session.root)

    def test_role_specific_flush_is_included_in_delta_and_floor(self) -> None:
        temporary, session = self.make_session()
        self.addCleanup(temporary.cleanup)
        destination_flush = (18, 16, 14, 12, 10, 8, 6, 4)
        for row in session.rows:
            if row["cell"] != analyzer.TARGET_CELL or row["trace_state"] != "off":
                continue
            flush_ms = (
                10
                if row["role"] == "source_init"
                else destination_flush[int(row["pair"]) - 1]
            )
            row["flush_ms"] = str(flush_ms)
            row["total_ms"] = str(
                int(row["transfer_ms"])
                + int(row["settled_ms"])
                - analyzer.SETTLE_MIN_MS
                + flush_ms
            )
        session.write()

        result = analyzer.analyze(session.root)
        with result.summary_csv.open(newline="") as handle:
            rows = {
                (row["cell"], row["trace_state"]): row
                for row in csv.DictReader(handle)
            }
        off = rows[(analyzer.TARGET_CELL, "off")]
        self.assertEqual(off["delta_ms"], "46")
        self.assertEqual(off["paired_delta_median_ms"], "46")
        self.assertEqual(off["paired_delta_range_ms"], "56")
        self.assertEqual(off["n_pair_ms"], "56")
        self.assertEqual(str(result.n_resolution), "56")

    def test_excess_settle_is_charged_without_false_role_delta(self) -> None:
        temporary, session = self.make_session()
        self.addCleanup(temporary.cleanup)
        old_formula_totals: dict[str, set[int]] = {
            "source_init": set(),
            "destination_init": set(),
        }
        actual_elapsed: set[int] = set()
        for row in session.rows:
            if row["cell"] != analyzer.TARGET_CELL or row["trace_state"] != "off":
                continue
            transfer_ms = 100
            if row["role"] == "source_init":
                settled_ms, flush_ms = 999, 1
            else:
                settled_ms, flush_ms = 250, 750
            row["transfer_ms"] = str(transfer_ms)
            row["settled_ms"] = str(settled_ms)
            row["flush_ms"] = str(flush_ms)
            row["total_ms"] = str(
                transfer_ms
                + settled_ms
                - analyzer.SETTLE_MIN_MS
                + flush_ms
            )
            old_formula_totals[row["role"]].add(transfer_ms + flush_ms)
            actual_elapsed.add(transfer_ms + settled_ms + flush_ms)
        self.assertEqual(actual_elapsed, {1100})
        self.assertEqual(old_formula_totals["source_init"], {101})
        self.assertEqual(old_formula_totals["destination_init"], {850})
        session.write()

        result = analyzer.analyze(session.root)
        with result.summary_csv.open(newline="") as handle:
            rows = {
                (row["cell"], row["trace_state"]): row
                for row in csv.DictReader(handle)
            }
        off = rows[(analyzer.TARGET_CELL, "off")]
        self.assertEqual(off["source_init_median_ms"], "850")
        self.assertEqual(off["destination_init_median_ms"], "850")
        self.assertEqual(off["delta_ms"], "0")
        self.assertEqual(off["paired_delta_median_ms"], "0")
        self.assertEqual(off["paired_delta_range_ms"], "0")
        self.assertEqual(off["n_pair_ms"], "0")

    def test_landed_manifest_rejects_swapped_sizes_and_renamed_paths(self) -> None:
        mutations = (
            (("a.txt", 2), ("sub/b.txt", 1)),
            (("renamed.txt", 1), ("sub/b.txt", 2)),
        )
        for entries in mutations:
            with self.subTest(entries=entries):
                temporary, session = self.make_session()
                self.addCleanup(temporary.cleanup)
                row = next(
                    row
                    for row in session.rows
                    if row["cell"].endswith("_mixed")
                )
                lines = sorted(
                    f"{base64.b64encode(path.encode()).decode()},{size}"
                    for path, size in entries
                )
                data = "".join(f"{line}\n" for line in lines).encode("ascii")
                digest = hashlib.sha256(data).hexdigest()
                rid = (
                    f"b{row['block']}_{row['cell']}_p{row['pair']}_{row['role']}"
                )
                (session.root / "landed" / f"{rid}.manifest").write_bytes(data)
                row["tree_manifest_sha256"] = digest
                session.write()
                with self.assertRaisesRegex(
                    analyzer.AnalysisError,
                    "landed relative-path/size manifest does not match canonical",
                ):
                    analyzer.analyze(session.root)

    def test_landed_root_and_recorded_manifest_digest_are_exact(self) -> None:
        temporary, session = self.make_session()
        self.addCleanup(temporary.cleanup)
        session.rows[0]["landed_root"] = "wrapper/src_large"
        session.write()
        with self.assertRaisesRegex(analyzer.AnalysisError, "landed_root must be"):
            analyzer.analyze(session.root)

        temporary_digest, digest_session = self.make_session()
        self.addCleanup(temporary_digest.cleanup)
        digest_session.rows[0]["tree_manifest_sha256"] = "0" * 64
        digest_session.write()
        with self.assertRaisesRegex(
            analyzer.AnalysisError, "landed manifest digest mismatch"
        ):
            analyzer.analyze(digest_session.root)

    def test_sequence_gap_and_missing_terminal_are_rejected(self) -> None:
        temporary, session = self.make_session()
        self.addCleanup(temporary.cleanup)
        first_session = next(event["session_id"] for event in session.events)
        for event in session.events:
            if (
                event["session_id"] == first_session
                and event["endpoint_role"] == "SOURCE"
                and event["producer_seq"] == 2
            ):
                event["producer_seq"] = 9
                break
        session.write()
        with self.assertRaisesRegex(analyzer.AnalysisError, "producer_seq"):
            analyzer.analyze(session.root)

    def test_payload_write_must_precede_source_completion(self) -> None:
        temporary, session = self.make_session()
        self.addCleanup(temporary.cleanup)
        first_session = next(event["session_id"] for event in session.events)
        write = next(
            event
            for event in session.events
            if event["session_id"] == first_session
            and event["endpoint_role"] == "SOURCE"
            and event["event"] == "first_socket_write"
        )
        complete = next(
            event
            for event in session.events
            if event["session_id"] == first_session
            and event["endpoint_role"] == "SOURCE"
            and event["event"] == "data_plane_complete"
        )
        for field in ("producer_seq", "elapsed_ns", "unix_ns"):
            write[field], complete[field] = complete[field], write[field]
        session.write()
        with self.assertRaisesRegex(
            analyzer.AnalysisError,
            "SOURCE/first_socket_write -> SOURCE/data_plane_complete",
        ):
            analyzer.analyze(session.root)

    def test_socket_action_end_must_precede_trace_attachment(self) -> None:
        temporary, session = self.make_session()
        self.addCleanup(temporary.cleanup)
        first_session = next(event["session_id"] for event in session.events)
        action_end = next(
            event
            for event in session.events
            if event["session_id"] == first_session
            and event["endpoint_role"] == "SOURCE"
            and event.get("epoch") == 0
            and str(event["event"]).startswith("socket_")
            and str(event["event"]).endswith("_end")
        )
        attached = next(
            event
            for event in session.events
            if event["session_id"] == first_session
            and event["endpoint_role"] == "SOURCE"
            and event["event"] == "socket_trace_attached"
            and event.get("epoch") == 0
        )
        for field in ("producer_seq", "elapsed_ns", "unix_ns"):
            action_end[field], attached[field] = attached[field], action_end[field]
        session.write()
        with self.assertRaisesRegex(
            analyzer.AnalysisError,
            "SOURCE/socket_.*_end -> SOURCE/socket_trace_attached",
        ):
            analyzer.analyze(session.root)

    def test_causal_elapsed_time_cannot_run_backwards(self) -> None:
        temporary, session = self.make_session()
        self.addCleanup(temporary.cleanup)
        first_session = next(event["session_id"] for event in session.events)
        attached = next(
            event
            for event in session.events
            if event["session_id"] == first_session
            and event["endpoint_role"] == "SOURCE"
            and event["event"] == "socket_trace_attached"
            and event.get("epoch") == 0
        )
        write_begin = next(
            event
            for event in session.events
            if event["session_id"] == first_session
            and event["endpoint_role"] == "SOURCE"
            and event["event"] == "socket_write_begin"
            and event.get("epoch") == 0
        )
        write_begin["elapsed_ns"] = int(attached["elapsed_ns"]) - 1
        session.write()
        with self.assertRaisesRegex(
            analyzer.AnalysisError,
            "SOURCE/socket_trace_attached -> SOURCE/socket_write_begin",
        ):
            analyzer.analyze(session.root)

    def test_destination_resize_prerequisites_are_causal(self) -> None:
        cases = (
            (
                "SOURCE",
                "resize_received",
                "resize_arm_queue_begin",
            ),
            (
                "SOURCE",
                "resize_arm_ready",
                "socket_accept_begin",
            ),
            (
                "DESTINATION",
                "resize_received",
                "socket_dial_begin",
            ),
            (
                "DESTINATION",
                "socket_trace_attached",
                "destination_prepared",
            ),
        )
        for initiator_role, start_name, end_name in cases:
            with self.subTest(
                initiator_role=initiator_role,
                edge=f"{start_name}->{end_name}",
            ):
                temporary, session = self.make_session()
                self.addCleanup(temporary.cleanup)
                session_id = self.traced_session_id(session, initiator_role)
                start = self.phase_event(
                    session, session_id, "DESTINATION", start_name, 1
                )
                end = self.phase_event(
                    session, session_id, "DESTINATION", end_name, 1
                )
                self.reorder_local_events([end, start])
                session.write()
                with self.assertRaisesRegex(
                    analyzer.AnalysisError,
                    f"DESTINATION/{start_name} -> DESTINATION/{end_name}",
                ):
                    analyzer.analyze(session.root)

    def test_source_resize_prerequisites_are_causal(self) -> None:
        for initiator_role, source_action in (
            ("SOURCE", "dial"),
            ("DESTINATION", "accept"),
        ):
            with self.subTest(
                initiator_role=initiator_role,
                edge=f"resize_sent->socket_{source_action}_begin",
            ):
                temporary, session = self.make_session()
                self.addCleanup(temporary.cleanup)
                session_id = self.traced_session_id(session, initiator_role)
                sent = self.phase_event(
                    session, session_id, "SOURCE", "resize_sent", 1
                )
                ack = self.phase_event(
                    session, session_id, "SOURCE", "resize_ack_received", 1
                )
                action_begin = self.phase_event(
                    session,
                    session_id,
                    "SOURCE",
                    f"socket_{source_action}_begin",
                    1,
                )
                self.reorder_local_events([ack, action_begin, sent])
                session.write()
                with self.assertRaisesRegex(
                    analyzer.AnalysisError,
                    f"SOURCE/resize_sent -> SOURCE/socket_{source_action}_begin",
                ):
                    analyzer.analyze(session.root)

            with self.subTest(
                initiator_role=initiator_role,
                edge="socket_trace_attached->source_settled",
            ):
                temporary, session = self.make_session()
                self.addCleanup(temporary.cleanup)
                session_id = self.traced_session_id(session, initiator_role)
                attached = self.phase_event(
                    session, session_id, "SOURCE", "socket_trace_attached", 1
                )
                settled = self.phase_event(
                    session, session_id, "SOURCE", "source_settled", 1
                )
                self.reorder_local_events([settled, attached])
                session.write()
                with self.assertRaisesRegex(
                    analyzer.AnalysisError,
                    "SOURCE/socket_trace_attached -> SOURCE/source_settled",
                ):
                    analyzer.analyze(session.root)

    def test_final_resize_settlement_precedes_data_plane_completion(self) -> None:
        for initiator_role in ("SOURCE", "DESTINATION"):
            with self.subTest(
                initiator_role=initiator_role,
                edge="SOURCE/source_settled->data_plane_complete",
            ):
                temporary, session = self.make_session()
                self.addCleanup(temporary.cleanup)
                session_id = self.traced_session_id(session, initiator_role)
                settled = self.phase_event(
                    session, session_id, "SOURCE", "source_settled", 7
                )
                first_queued = self.phase_event(
                    session, session_id, "SOURCE", "first_payload_queued", None
                )
                write_begin = self.phase_event(
                    session, session_id, "SOURCE", "socket_write_begin", 0
                )
                first_write = self.phase_event(
                    session, session_id, "SOURCE", "first_socket_write", 0
                )
                complete = self.phase_event(
                    session, session_id, "SOURCE", "data_plane_complete", None
                )
                self.reorder_local_events(
                    [first_queued, write_begin, first_write, complete, settled]
                )
                session.write()
                with self.assertRaisesRegex(
                    analyzer.AnalysisError,
                    "SOURCE/source_settled -> SOURCE/data_plane_complete",
                ):
                    analyzer.analyze(session.root)

            with self.subTest(
                initiator_role=initiator_role,
                edge="DESTINATION/resize_ack_sent->data_plane_complete",
            ):
                temporary, session = self.make_session()
                self.addCleanup(temporary.cleanup)
                session_id = self.traced_session_id(session, initiator_role)
                ack_sent = self.phase_event(
                    session, session_id, "DESTINATION", "resize_ack_sent", 7
                )
                first_received = self.phase_event(
                    session,
                    session_id,
                    "DESTINATION",
                    "first_payload_received",
                    0,
                )
                complete = self.phase_event(
                    session,
                    session_id,
                    "DESTINATION",
                    "data_plane_complete",
                    None,
                )
                self.reorder_local_events([first_received, complete, ack_sent])
                session.write()
                with self.assertRaisesRegex(
                    analyzer.AnalysisError,
                    "DESTINATION/resize_ack_sent -> DESTINATION/data_plane_complete",
                ):
                    analyzer.analyze(session.root)

    def test_destination_preparation_action_is_role_correlated(self) -> None:
        temporary, session = self.make_session()
        self.addCleanup(temporary.cleanup)
        prepared = next(
            event
            for event in session.events
            if event["event"] == "destination_prepared"
            and event["initiator_role"] == "SOURCE"
        )
        prepared["action"] = "dial_complete"
        session.write()
        with self.assertRaisesRegex(analyzer.AnalysisError, "must be arm_queued"):
            analyzer.analyze(session.root)

    def test_resize_ramp_requires_all_seven_epochs(self) -> None:
        temporary, session = self.make_session()
        self.addCleanup(temporary.cleanup)
        first_session = next(event["session_id"] for event in session.events)
        session.events = [
            event
            for event in session.events
            if not (
                event["session_id"] == first_session and event.get("epoch") == 7
            )
        ]
        for endpoint_role in ("SOURCE", "DESTINATION"):
            role_events = [
                event
                for event in session.events
                if event["session_id"] == first_session
                and event["endpoint_role"] == endpoint_role
            ]
            for producer_seq, event in enumerate(
                sorted(role_events, key=lambda item: int(item["producer_seq"]))
            ):
                event["producer_seq"] = producer_seq
        session.write()
        with self.assertRaisesRegex(
            analyzer.AnalysisError, "resize epochs must be exactly 1..7"
        ):
            analyzer.analyze(session.root)

    def test_final_resize_target_and_live_fields_are_exact_on_both_roles(self) -> None:
        mutations = (
            ("SOURCE", "source_settled", "target_streams"),
            ("SOURCE", "source_settled", "live_streams"),
            ("DESTINATION", "resize_received", "target_streams"),
            ("DESTINATION", "resize_ack_sent", "live_streams"),
        )
        for endpoint_role, event_name, field in mutations:
            with self.subTest(
                endpoint_role=endpoint_role, event=event_name, field=field
            ):
                temporary, session = self.make_session()
                self.addCleanup(temporary.cleanup)
                first_session = next(event["session_id"] for event in session.events)
                marker = next(
                    event
                    for event in session.events
                    if event["session_id"] == first_session
                    and event["endpoint_role"] == endpoint_role
                    and event["event"] == event_name
                    and event.get("epoch") == 7
                )
                marker[field] = 7
                session.write()
                with self.assertRaisesRegex(
                    analyzer.AnalysisError,
                    f"{endpoint_role}/{event_name} epoch 7 {field} must be 8",
                ):
                    analyzer.analyze(session.root)

    def test_clock_inventory_and_midpoint_math_are_fail_closed(self) -> None:
        temporary, session = self.make_session()
        self.addCleanup(temporary.cleanup)
        session.clock_rows[0]["offset_windows_minus_q_ns"] = "999"
        session.write()
        with self.assertRaisesRegex(analyzer.AnalysisError, "offset mismatch"):
            analyzer.analyze(session.root)

        session.clock_rows[0]["offset_windows_minus_q_ns"] = "1000"
        session.clock_rows.pop()
        session.write()
        with self.assertRaisesRegex(analyzer.AnalysisError, "inventory incomplete"):
            analyzer.analyze(session.root)


if __name__ == "__main__":
    unittest.main()
