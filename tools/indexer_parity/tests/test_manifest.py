import json
import tempfile
import unittest
from pathlib import Path
from typing import Any

from indexer_parity.manifest import (
    FIELDS,
    ManifestError,
    compare,
    read_manifest,
    write_reports,
)


def manifest(*rows: str) -> str:
    return ",".join(FIELDS) + "\n" + "\n".join(rows) + "\n"


BASELINE_ROW = "aa,bb,cc,10,5,1,0,true,ssi,dd,,,"


class ManifestTest(unittest.TestCase):
    def test_equal_manifests_ignore_input_order_and_hex_case(self) -> None:
        first = read_manifest(
            manifest(BASELINE_ROW, "ee,ff,00,11,6,2,0,true,vdr,AA,11,22,33")
        )
        second = read_manifest(
            manifest("EE,FF,00,11,6,2,0,true,VDR,aa,11,22,33", BASELINE_ROW)
        )

        self.assertTrue(compare(first, second).equal)

    def test_reports_missing_and_changed_operations(self) -> None:
        baseline = read_manifest(
            manifest(BASELINE_ROW, "ee,ff,00,11,6,2,0,true,vdr,aa,11,22,33")
        )
        candidate = read_manifest(
            manifest(
                "aa,bb,99,10,5,1,0,true,ssi,dd,,,",
                "11,22,33,12,7,3,0,false,none,,,,",
            )
        )

        result = compare(baseline, candidate)

        self.assertFalse(result.equal)
        self.assertEqual(
            [item.operation_id for item in result.missing_from_candidate], ["ee"]
        )
        self.assertEqual(
            [item.operation_id for item in result.missing_from_baseline], ["11"]
        )
        self.assertEqual([item.operation_id for item in result.changed], ["aa"])

    def test_duplicate_operation_ids_fail_comparison(self) -> None:
        records = read_manifest(manifest(BASELINE_ROW, BASELINE_ROW))

        result = compare(records, records[:1])

        self.assertFalse(result.equal)
        self.assertEqual(result.duplicate_baseline_ids, ("aa",))

    def test_vdr_hash_changes_fail_comparison(self) -> None:
        baseline = read_manifest(manifest("aa,bb,cc,10,5,1,0,true,vdr,dd,11,22,33"))
        candidate = read_manifest(manifest("aa,bb,cc,10,5,1,0,true,vdr,dd,99,22,33"))

        result = compare(baseline, candidate)

        self.assertFalse(result.equal)
        self.assertEqual([item.operation_id for item in result.changed], ["aa"])

    def test_rejects_invalid_headers(self) -> None:
        with self.assertRaises(ManifestError):
            read_manifest("operation_id,slot\naa,10\n")

    def test_report_records_observed_drain_evidence(self) -> None:
        comparison = compare([], [])
        metadata = {
            "network": "preprod",
            "comparison_slot": 100,
            "baseline_image": "baseline:immutable",
            "baseline_identity": "sha256:a",
            "candidate_image": "candidate:immutable",
            "candidate_identity": "sha256:b",
            "compose_project": "neoprism-parity-test",
            "baseline_cursor": 102,
            "candidate_cursor": 103,
            "drain_assurance": {
                "minimum_quiescence_seconds": 10,
                "observed_quiescence_seconds": 11.25,
                "required_cursor": 102,
                "baseline_operation_count": 5,
                "candidate_operation_count": 5,
                "baseline_pending_count": 0,
                "candidate_pending_count": 0,
            },
        }
        cursor = {
            "time_to_boundary_seconds": 1.0,
            "effective_slots_per_second": 2.0,
        }
        resources = {
            "mean_cpu_percent": 1.0,
            "p95_cpu_percent": 1.0,
            "peak_cpu_percent": 1.0,
            "mean_memory_bytes": 1024,
            "peak_memory_bytes": 1024,
            "peak_pids": 1,
            "restart_count": 0,
        }
        metrics: dict[str, Any] = {
            "durations": {"total_run_seconds": 2.0, "scan_seconds": 1.0},
            "cursor": {
                "baseline": cursor,
                "candidate": cursor,
                "candidate_to_baseline": {
                    "time_to_boundary_ratio": 1.0,
                    "effective_slots_per_second_ratio": 1.0,
                },
            },
            "resources": {
                "baseline": resources,
                "candidate": resources,
                "candidate_to_baseline": {
                    "mean_cpu_ratio": 1.0,
                    "peak_cpu_ratio": 1.0,
                    "mean_memory_ratio": 1.0,
                    "peak_memory_ratio": 1.0,
                },
            },
            "warnings": [],
        }

        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            write_reports(output, comparison, metadata, metrics)
            report = (output / "report.md").read_text(encoding="utf-8")
            machine_report = json.loads(
                (output / "report.json").read_text(encoding="utf-8")
            )

        self.assertIn("11.250 s", report)
        self.assertIn("both cursors at or above `102`", report)
        self.assertEqual(
            machine_report["metadata"]["drain_assurance"]["baseline_pending_count"],
            0,
        )


if __name__ == "__main__":
    unittest.main()
