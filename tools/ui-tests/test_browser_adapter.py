"""Validate the external CLI boundary without launching a browser."""

import os
import subprocess
import unittest
from unittest.mock import patch

from pydantic import ValidationError

os.environ.setdefault("PRISM_TEST_SESSION", "prism-envelope-unit")
from prism_shell import browser  # noqa: E402


class BrowserEnvelope(unittest.TestCase):
    def test_rejects_unvalidated_external_envelopes(self) -> None:
        for raw in (
            '{"success":"false","data":{}}',
            '{"success":true,"data":[]}',
            '{"success":true}',
            '{"data":{}}',
            "null",
        ):
            with (
                self.subTest(raw=raw),
                patch(
                    "subprocess.run",
                    return_value=subprocess.CompletedProcess([], 0, raw),
                ),
                self.assertRaises(ValidationError),
            ):
                browser("get", "url")

    def test_accepts_cli_data_and_reports_failure(self) -> None:
        with patch(
            "subprocess.run",
            return_value=subprocess.CompletedProcess(
                [],
                0,
                '{"success":true,"data":{"url":"http://127.0.0.1/"},"error":null}',
            ),
        ):
            self.assertEqual(browser("get", "url"), {"url": "http://127.0.0.1/"})
        with (
            patch(
                "subprocess.run",
                return_value=subprocess.CompletedProcess(
                    [], 0, '{"success":false,"data":null,"error":"denied"}'
                ),
            ),
            self.assertRaises(AssertionError),
        ):
            browser("get", "url")


if __name__ == "__main__":
    unittest.main()
