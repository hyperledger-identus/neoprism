import tempfile
import unittest
from pathlib import Path

from indexer_parity.metrics import MetricsRecorder, parse_memory_bytes


class MetricsTest(unittest.TestCase):
    def test_parses_docker_memory_units(self) -> None:
        self.assertEqual(parse_memory_bytes("512B"), 512)
        self.assertEqual(parse_memory_bytes("1.5MiB"), 1_572_864)
        self.assertEqual(parse_memory_bytes("2GB"), 2_000_000_000)

    def test_summarizes_cursor_and_resource_comparison(self) -> None:
        now = [2.0]
        recorder = MetricsRecorder(scan_started_monotonic=0.0, clock=lambda: now[0])
        recorder.record_cursors(10_718_600, 10_718_550)
        now[0] = 4.0
        recorder.record_cursors(10_719_513, 10_719_513)
        recorder.record_resource("baseline", 20.0, 100, 1.0, 4)
        recorder.record_resource("candidate", 25.0, 120, 1.2, 5)

        summary = recorder.summarize(
            network="preprod",
            required_cursor=10_719_513,
            total_duration_seconds=5.0,
            scan_duration_seconds=4.5,
            restart_counts={"baseline": 0, "candidate": 1},
        )

        self.assertEqual(
            summary["cursor"]["baseline"]["effective_slots_per_second"], 250.0
        )
        self.assertEqual(
            summary["resources"]["candidate_to_baseline"]["mean_memory_ratio"],
            1.2,
        )
        self.assertEqual(summary["resources"]["candidate"]["restart_count"], 1)

    def test_writes_raw_csv_evidence(self) -> None:
        recorder = MetricsRecorder(scan_started_monotonic=0.0, clock=lambda: 1.0)
        recorder.record_cursors(10, 11)
        recorder.record_resource("baseline", 1.0, 2, 3.0, 4)

        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            recorder.write_samples(output)

            self.assertIn(
                "baseline_cursor", (output / "cursor-samples.csv").read_text()
            )
            self.assertIn(
                "memory_bytes", (output / "resource-samples.csv").read_text()
            )


if __name__ == "__main__":
    unittest.main()
