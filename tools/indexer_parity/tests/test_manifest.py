import unittest

from indexer_parity.manifest import FIELDS, ManifestError, compare, read_manifest


def manifest(*rows: str) -> str:
    return ",".join(FIELDS) + "\n" + "\n".join(rows) + "\n"


BASELINE_ROW = "aa,bb,cc,10,5,1,0,true,ssi,dd"


class ManifestTest(unittest.TestCase):
    def test_equal_manifests_ignore_input_order_and_hex_case(self) -> None:
        first = read_manifest(manifest(BASELINE_ROW, "ee,ff,00,11,6,2,0,true,vdr,AA"))
        second = read_manifest(manifest("EE,FF,00,11,6,2,0,true,VDR,aa", BASELINE_ROW))

        self.assertTrue(compare(first, second).equal)

    def test_reports_missing_and_changed_operations(self) -> None:
        baseline = read_manifest(
            manifest(BASELINE_ROW, "ee,ff,00,11,6,2,0,true,vdr,aa")
        )
        candidate = read_manifest(
            manifest("aa,bb,99,10,5,1,0,true,ssi,dd", "11,22,33,12,7,3,0,false,none,")
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

    def test_rejects_invalid_headers(self) -> None:
        with self.assertRaises(ManifestError):
            read_manifest("operation_id,slot\naa,10\n")


if __name__ == "__main__":
    unittest.main()
