import contextlib
import io
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import MagicMock, patch

from indexer_parity.cli import (
    ComposeHarness,
    HarnessError,
    RunOptions,
    _run,  # pyright: ignore[reportPrivateUsage]
    _validate_comparison_slot,  # pyright: ignore[reportPrivateUsage]
    _wait_for_drain,  # pyright: ignore[reportPrivateUsage]
    main,
    parse_args,
    run,
)


def options(output_dir: Path) -> RunOptions:
    return RunOptions(
        baseline_image="baseline:immutable",
        candidate_image="candidate:immutable",
        network="preprod",
        relay_address="relay.example:3001",
        target_slot=10_718_513,
        sample_seconds=1,
        boundary_lag=1,
        timeout_seconds=10,
        metrics_sample_seconds=1,
        drain_seconds=1,
        output_dir=output_dir,
        keep=False,
    )


class CliTest(unittest.TestCase):
    def test_compose_project_is_unique_per_harness(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            run_options = options(Path(directory))
            first = ComposeHarness(run_options, deadline=10.0)
            second = ComposeHarness(run_options, deadline=10.0)

        self.assertNotEqual(first.project_name, second.project_name)
        self.assertTrue(first.project_name.startswith("neoprism-parity-"))

    def test_rejects_slot_before_network_genesis(self) -> None:
        with self.assertRaisesRegex(HarnessError, "precedes preprod PRISM genesis"):
            _validate_comparison_slot("preprod", 10_718_512)

    def test_argument_parser_rejects_slot_before_network_genesis(self) -> None:
        with (
            contextlib.redirect_stderr(io.StringIO()),
            self.assertRaises(SystemExit) as raised,
        ):
            parse_args(
                [
                    "--candidate-image",
                    "candidate:immutable",
                    "--target-slot",
                    "10718512",
                ]
            )

        self.assertEqual(raised.exception.code, 2)

    def test_subprocess_timeout_is_a_harness_error(self) -> None:
        with self.assertRaisesRegex(HarnessError, "command timed out"):
            _run(
                [sys.executable, "-c", "import time; time.sleep(1)"],
                timeout_seconds=0.01,
            )

    def test_drain_waits_for_cursor_pending_and_stability_conditions(self) -> None:
        harness = MagicMock()
        metrics = MagicMock()
        harness.operation_state.side_effect = [
            (5, 0),
            (5, 0),
            (5, 0),
            (5, 0),
            (5, 0),
            (5, 0),
            (5, 1),
            (5, 0),
            (5, 0),
            (5, 0),
            (5, 0),
            (5, 0),
        ]
        harness.cursor.side_effect = [
            101,
            101,
            101,
            101,
            102,
            102,
            103,
            103,
            104,
            104,
            105,
            105,
        ]
        clock = [0.0, 1.0, 2.0, 2.1, 2.1, 5.0, 6.0, 6.1, 6.1, 17.0, 17.1]

        with (
            patch("indexer_parity.cli.time.monotonic", side_effect=clock),
            patch("indexer_parity.cli.time.sleep"),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            evidence = _wait_for_drain(
                harness,
                metrics,
                comparison_slot=100,
                boundary_lag=1,
                drain_seconds=10,
                deadline=100,
            )

        self.assertEqual(evidence.cursors, (105, 105))
        self.assertEqual(evidence.baseline_state, (5, 0))
        self.assertGreaterEqual(evidence.stable_seconds, 10)
        self.assertEqual(harness.operation_state.call_count, 12)

    def test_finalization_errors_do_not_replace_primary_failure(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            run_options = options(Path(directory) / "evidence")
            with (
                patch("indexer_parity.cli.ComposeHarness") as harness_type,
                patch(
                    "indexer_parity.cli._run",
                    side_effect=HarnessError("primary failure"),
                ),
                contextlib.redirect_stderr(io.StringIO()),
            ):
                harness = harness_type.return_value
                harness.project_name = "neoprism-parity-test"
                harness.write_logs.side_effect = OSError("logs unavailable")
                harness.cleanup.side_effect = HarnessError("cleanup unavailable")

                with (
                    contextlib.redirect_stdout(io.StringIO()),
                    self.assertRaisesRegex(HarnessError, "primary failure"),
                ):
                    run(run_options)

                harness.write_logs.assert_called_once_with()
                harness.cleanup.assert_called_once_with()

    def test_main_maps_evidence_io_failure_to_harness_exit(self) -> None:
        with (
            patch("indexer_parity.cli.parse_args") as parse,
            patch("indexer_parity.cli.run", side_effect=OSError("disk full")),
            contextlib.redirect_stderr(io.StringIO()),
            self.assertRaises(SystemExit) as raised,
        ):
            main()

        parse.assert_called_once_with()
        self.assertEqual(raised.exception.code, 2)

    def test_finalization_errors_do_not_replace_parity_difference(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            run_options = options(Path(directory) / "evidence")
            with (
                patch("indexer_parity.cli.ComposeHarness") as harness_type,
                patch("indexer_parity.cli._run"),
                patch("indexer_parity.cli.MetricsRecorder") as metrics_type,
                patch("indexer_parity.cli.ResourceSampler") as sampler_type,
                patch("indexer_parity.cli._wait_for_databases"),
                patch("indexer_parity.cli._select_boundary", return_value=100),
                patch("indexer_parity.cli._wait_for_boundary"),
                patch("indexer_parity.cli._wait_for_drain") as wait_for_drain,
                patch("indexer_parity.cli.read_manifest", side_effect=[[], []]),
                patch("indexer_parity.cli.write_manifest"),
                patch("indexer_parity.cli.compare") as compare,
                patch("indexer_parity.cli.write_reports"),
                contextlib.redirect_stdout(io.StringIO()),
                contextlib.redirect_stderr(io.StringIO()),
            ):
                harness = harness_type.return_value
                harness.project_name = "neoprism-parity-test"
                harness.image_identity.side_effect = ["sha256:a", "sha256:b"]
                harness.write_logs.side_effect = OSError("logs unavailable")
                harness.cleanup.side_effect = HarnessError("cleanup unavailable")
                wait_for_drain.return_value.cursors = (102, 102)
                wait_for_drain.return_value.stable_seconds = 10.0
                wait_for_drain.return_value.baseline_state = (5, 0)
                wait_for_drain.return_value.candidate_state = (5, 0)
                sampler_type.return_value.warnings = []
                metrics_type.return_value.summarize.return_value = {}
                compare.return_value.equal = False

                result = run(run_options)

        self.assertEqual(result, 1)
        harness.write_logs.assert_called_once_with()
        harness.cleanup.assert_called_once_with()


if __name__ == "__main__":
    unittest.main()
