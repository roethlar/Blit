#!/usr/bin/env python3
"""Synthetic guards for otp12pf_rigw_analyze.py."""

from __future__ import annotations

import csv
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

        source = [
            event("SOURCE", 0, 0, f"socket_{source_action}_begin", epoch=0, socket=0),
            event("SOURCE", 1, 1, f"socket_{source_action}_end", epoch=0, socket=0),
            event("SOURCE", 2, 2, "socket_trace_attached", epoch=0, socket=0),
            event("SOURCE", 3, 3, "manifest_complete_send_begin"),
            event("SOURCE", 4, 4, "manifest_complete_sent", count=1),
            event("SOURCE", 5, 5, "resize_proposed", epoch=1, target_streams=2),
            event("SOURCE", 6, 6, "resize_send_begin", epoch=1, target_streams=2),
            event("SOURCE", 7, 7, "resize_sent", epoch=1, target_streams=2),
            event("SOURCE", 8, 8, "need_batch_received", batch=0, count=1),
            event("SOURCE", 9, 9, "planner_begin", batch=0, count=1),
            event("SOURCE", 10, 10, "planner_end", batch=0, count=1),
            event("SOURCE", 11, 11, "resize_ack_received", epoch=1, accepted=True),
            event("SOURCE", 12, 12, f"socket_{source_action}_begin", epoch=1, socket=0),
            event("SOURCE", 13, 13, f"socket_{source_action}_end", epoch=1, socket=0),
            event("SOURCE", 14, 14, "socket_trace_attached", epoch=1, socket=0),
            event("SOURCE", 15, 15, "source_settled", epoch=1, accepted=True),
            event("SOURCE", 16, 16, "first_payload_queued"),
            event("SOURCE", 17, 17, "socket_write_begin", epoch=0, socket=0),
            event("SOURCE", 18, 18, "first_socket_write", epoch=0, socket=0),
            event("SOURCE", 19, 19, "data_plane_complete"),
            event("SOURCE", 20, 20, "summary_received"),
        ]
        destination = [
            event(
                "DESTINATION",
                0,
                0,
                f"socket_{destination_action}_begin",
                epoch=0,
                socket=0,
            ),
            event(
                "DESTINATION",
                1,
                1,
                f"socket_{destination_action}_end",
                epoch=0,
                socket=0,
            ),
            event("DESTINATION", 2, 2, "socket_trace_attached", epoch=0, socket=0),
            event("DESTINATION", 3, 3, "manifest_complete_received"),
            event("DESTINATION", 4, 4, "need_batch_send_begin", batch=0, count=1),
            event("DESTINATION", 5, 5, "need_batch_sent", batch=0, count=1),
            event("DESTINATION", 6, 6, "resize_received", epoch=1, target_streams=2),
        ]
        if initiator == "SOURCE":
            destination.extend(
                [
                    event("DESTINATION", 7, 7, "resize_arm_queue_begin", epoch=1),
                    event(
                        "DESTINATION",
                        8,
                        8,
                        "destination_prepared",
                        epoch=1,
                        action="arm_queued",
                    ),
                    event("DESTINATION", 9, 9, "resize_ack_send_begin", epoch=1, accepted=True),
                    event("DESTINATION", 10, 10, "resize_ack_sent", epoch=1, accepted=True),
                    event("DESTINATION", 11, 11, "resize_arm_ready", epoch=1),
                    event(
                        "DESTINATION",
                        12,
                        12,
                        "socket_accept_begin",
                        epoch=1,
                        socket=0,
                    ),
                    event(
                        "DESTINATION",
                        13,
                        13,
                        "socket_accept_end",
                        epoch=1,
                        socket=0,
                    ),
                    event("DESTINATION", 14, 14, "socket_trace_attached", epoch=1, socket=0),
                ]
            )
        else:
            destination.extend(
                [
                    event("DESTINATION", 7, 7, "socket_dial_begin", epoch=1, socket=0),
                    event("DESTINATION", 8, 8, "socket_dial_end", epoch=1, socket=0),
                    event("DESTINATION", 9, 9, "socket_trace_attached", epoch=1, socket=0),
                    event(
                        "DESTINATION",
                        10,
                        10,
                        "destination_prepared",
                        epoch=1,
                        action="dial_complete",
                    ),
                    event("DESTINATION", 11, 11, "resize_ack_send_begin", epoch=1, accepted=True),
                    event("DESTINATION", 12, 12, "resize_ack_sent", epoch=1, accepted=True),
                ]
            )
        next_seq = len(destination)
        destination.extend(
            [
                event(
                    "DESTINATION",
                    next_seq,
                    next_seq,
                    "first_payload_received",
                    epoch=0,
                    socket=0,
                ),
                event("DESTINATION", next_seq + 1, next_seq + 1, "data_plane_complete"),
                event("DESTINATION", next_seq + 2, next_seq + 2, "summary_send_begin"),
                event("DESTINATION", next_seq + 3, next_seq + 3, "summary_sent"),
            ]
        )
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
                                "flush_ms": "1",
                                "total_ms": str(transfer_ms + 1),
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
