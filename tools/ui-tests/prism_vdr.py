"""VDR browser checks over real Resolver HTML, without production fixture routes."""

import json
import os
import unittest
from pathlib import Path

from fixture_server import fixture_server
from prism_shell import browser, evaluate

FIXTURES = Path(os.environ["PRISM_VDR_FIXTURES"])
PORT = int(os.environ.get("PRISM_VDR_PORT", "18775"))
URL = f"http://127.0.0.1:{PORT}"


class PrismVdr(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.server = cls.enterClassContext(fixture_server(FIXTURES, PORT))
        browser("network", "route", "https://unpkg.com/*", "--abort")

    @classmethod
    def tearDownClass(cls) -> None:
        browser("close")

    def setUp(self) -> None:
        browser("set", "viewport", "390", "900")
        browser("open", URL + "/vdr-long.html")

    def test_long_raw_and_field_copy_has_exact_success_and_denied_fallback(
        self,
    ) -> None:
        browser("set", "viewport", "320", "900")
        evaluate(
            "document.querySelectorAll('#vdr-entries details')"
            ".forEach(e => e.open = true)"
        )
        evaluate("""Object.defineProperty(navigator, 'clipboard', {configurable:true,
            value:{writeText: async text => {window.vdrWritten = text;}}})""")
        detail = "https://issuer.example/status/" * 128 + "\nsecond line & <end>"
        cid = "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi/"
        # Known literal/fixture expectations, not values read from copy attributes.
        cases = {
            "VDR entry 1 Initial operation hash": "12" * 32,
            "VDR entry 1 Last operation hash": "ab" * 32,
            "VDR entry 1 Bytes · hex": "000a80ff" * 128,
            "VDR entry 1 raw data": "Bytes(["
            + ", ".join(["0", "10", "128", "255"] * 128)
            + "])",
            "VDR entry 2 State": "-42",
            "VDR entry 2 Name": 'Credential "résumé" <list>',
            "VDR entry 2 Detail": detail,
            "VDR entry 2 raw data": (
                "StatusList(StatusListData { state: -42, "
                'name: "Credential \\"résumé\\" <list>", detail: "'
            )
            + detail.replace("\n", "\\n")
            + '" })',
            "VDR entry 3 IPFS value": cid + "credential-data" * 128,
            "VDR entry 3 raw data": 'Ipfs("' + cid + "credential-data" * 128 + '")',
        }
        for label, expected in cases.items():
            with self.subTest(label=label):
                selector = f'button[aria-label="Copy {label}"]'
                browser("focus", selector)
                browser("press", "Space")
                browser("wait", "--text", "Copied to clipboard.")
                self.assertEqual(evaluate("window.vdrWritten"), expected)
                self.assertEqual(
                    evaluate("document.activeElement.ariaLabel"), "Copy " + label
                )
                evaluate("""navigator.clipboard.writeText = async () => {
                    throw Error('denied')};
                    document.querySelector('.prism-copy-status').textContent = ''""")
                browser("press", "Enter")
                browser("wait", "--text", "Couldn’t copy.")
                fallback = evaluate("""(() => {
                    const parent = document.activeElement.parentElement;
                    const t = parent.querySelector('textarea');
                    const r = t.getBoundingClientRect();
                    return {value:t.value, readonly:t.readOnly,
                        label:t.labels[0].textContent,
                        bounds:r.left >= 0 && r.right <= innerWidth};
                })()""")
                self.assertEqual(fallback["value"], expected)
                self.assertTrue(fallback["readonly"])
                self.assertTrue(fallback["bounds"])
                self.assertIn("Select and copy manually", fallback["label"])
                self.assertEqual(
                    evaluate("document.activeElement.ariaLabel"), "Copy " + label
                )
                self.assertTrue(
                    evaluate("document.documentElement.scrollWidth <= innerWidth")
                )
                evaluate("""navigator.clipboard.writeText = async text => {
                    window.vdrWritten = text;};
                    document.querySelector('.prism-copy-status').textContent = ''""")

    def test_native_keyboard_no_script_disclosures_keep_exact_captured_text(
        self,
    ) -> None:
        browser("close")
        browser("open")
        browser("network", "route", "*", "--abort", "--resource-type", "script")
        try:
            browser("open", URL + "/vdr.html")
            expected = [
                "Bytes([0, 10, 128, 255])",
                (
                    "StatusList(StatusListData { state: -42, "
                    'name: "Credential \\"résumé\\" <list>", '
                    'detail: "https://issuer.example/status?a=1&b=2\\nsecond line" })'
                ),
                'Ipfs("bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi")',
            ]
            self.assertFalse(
                evaluate(
                    "document.documentElement.classList.contains('prism-copy-ready')"
                )
            )
            for index, raw in enumerate(expected, 1):
                selector = f".prism-vdr-entry:nth-child({index}) summary"
                browser("focus", selector)
                browser("press", "Enter")
                self.assertTrue(
                    evaluate(
                        "document.querySelectorAll('#vdr-entries details')"
                        f"[{index - 1}].open"
                    )
                )
                self.assertEqual(
                    evaluate(
                        "document.querySelectorAll('#vdr-entries pre')"
                        f"[{index - 1}].textContent"
                    ),
                    raw,
                )
                self.assertTrue(
                    evaluate(f"""(() => {{
                    const values = document.querySelectorAll('#vdr-entries pre');
                    const e = values[{index - 1}];
                    const range = document.createRange(); range.selectNodeContents(e);
                    getSelection().removeAllRanges(); getSelection().addRange(range);
                    return getSelection().toString() === {json.dumps(raw)};
                }})()""")
                )
                browser("press", "Space")
                self.assertFalse(
                    evaluate(
                        "document.querySelectorAll('#vdr-entries details')"
                        f"[{index - 1}].open"
                    )
                )
            self.assertTrue(
                evaluate("document.documentElement.scrollWidth <= innerWidth")
            )
        finally:
            browser("network", "unroute", "*")

    def test_empty_and_populated_sections_have_names_and_no_scoped_a11y_violations(
        self,
    ) -> None:
        for page in ("vdr", "vdr-long", "vdr-empty"):
            with self.subTest(page=page):
                browser("open", URL + f"/{page}.html")
                evaluate(
                    "document.querySelectorAll('#vdr-entries details')"
                    ".forEach(e => e.open = true)"
                )
                self.assertEqual(
                    browser("a11y", "--selector", "#vdr-entries")["counts"][
                        "violations"
                    ],
                    0,
                )
                self.assertTrue(
                    evaluate("""[...document.querySelectorAll('#vdr-entries button')]
                    .every(e => e.type === 'button'
                        && e.ariaLabel.startsWith('Copy VDR entry '))""")
                )
                if page == "vdr-empty":
                    self.assertIn(
                        "No VDR entries",
                        evaluate("document.querySelector('#vdr-entries').textContent"),
                    )
                    self.assertEqual(
                        evaluate(
                            "document.querySelectorAll("
                            "'#vdr-entries details, #vdr-entries button').length"
                        ),
                        0,
                    )
                    self.assertLess(
                        evaluate(
                            "document.querySelector('#vdr-entries').getBoundingClientRect().height"
                        ),
                        150,
                    )

    def test_expanded_long_data_and_hashes_stay_in_bounds(self) -> None:
        for width in (320, 390, 720, 800, 801, 1024, 1280, 1440):
            with self.subTest(width=width):
                browser("set", "viewport", str(width), "900")
                evaluate(
                    "document.querySelectorAll('#vdr-entries details')"
                    ".forEach(e => e.open = true)"
                )
                self.assertTrue(
                    evaluate("document.documentElement.scrollWidth <= innerWidth")
                )
                self.assertTrue(
                    evaluate("""[...document.querySelectorAll(
                    '#vdr-entries pre, #vdr-entries dd')]
                    .every(e => e.scrollWidth <= e.clientWidth + 1)""")
                )
                self.assertTrue(
                    evaluate("""[...document.querySelectorAll(
                    '#vdr-entries summary, #vdr-entries button')]
                    .every(e => e.getBoundingClientRect().height >= 48)""")
                )
                self.assertEqual(
                    evaluate("""getComputedStyle(
                    document.querySelector('#vdr-entries pre'))
                    .whiteSpace"""),
                    "pre-wrap",
                )


if __name__ == "__main__":
    unittest.main()
