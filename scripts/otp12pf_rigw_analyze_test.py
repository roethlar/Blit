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
                                "settled_ms": "250",
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
