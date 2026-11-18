"""Operation browser seam using real Maud pages exported by Rust tests.

No production fixture route. Run export_operation_browser_fixtures with
PRISM_OPERATION_FIXTURES set to an external directory, then just build-assets.
"""

import os
import unittest
from pathlib import Path

from fixture_server import fixture_server
from prism_shell import browser, evaluate

FIXTURES = Path(os.environ["PRISM_OPERATION_FIXTURES"])
PORT = int(os.environ.get("PRISM_OPERATION_PORT", "18776"))
URL = f"http://127.0.0.1:{PORT}"


class PrismOperations(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.server = cls.enterClassContext(fixture_server(FIXTURES, PORT))
        browser("open")
        browser("network", "route", "**/htmx.org/**", "--abort")

    @classmethod
    def tearDownClass(cls) -> None:
        browser("close")

    def setUp(self) -> None:
        browser("set", "viewport", "390", "900")
        browser("open", URL + "/operations.html")

    def test_raw_payload_inspection_and_keyboard_copy_preserve_the_complete_capture(
        self,
    ) -> None:
        evaluate(
            "document.querySelectorAll('main details').forEach(e => e.open = true)"
        )
        evaluate("""Object.defineProperty(navigator, 'clipboard', {configurable:true,
            value:{writeText: async value => { window.copiedOriginal = value; }}});""")
        for index in (1, 2):
            with self.subTest(index=index):
                expected = (FIXTURES / f"payload-{index}.txt").read_text()
                label = f"Copy operation {index} complete signed payload"
                selector = f'button[aria-label="{label}"]'
                self.assertEqual(
                    evaluate(
                        "document.querySelectorAll('.prism-operation-payload pre')"
                        f"[{index - 1}].textContent"
                    ),
                    expected,
                )
                browser("focus", selector)
                browser("press", "Space")
                browser("wait", "--text", "Copied to clipboard.")
                self.assertEqual(evaluate("window.copiedOriginal"), expected)
                self.assertEqual(
                    evaluate("document.activeElement.ariaLabel"),
                    label,
                )
                evaluate(
                    "document.querySelector('.prism-copy-status').textContent = ''"
                )

    def test_denied_copy_keeps_exact_fallback_focus_and_mobile_bounds(
        self,
    ) -> None:
        evaluate(
            "document.querySelectorAll('main details').forEach(e => e.open = true)"
        )
        evaluate("""Object.defineProperty(navigator, 'clipboard', {configurable:true,
            value:{writeText: async () => {throw new Error('denied');}}});""")
        for label, expected in (
            (
                "Copy operation 2 complete signed payload",
                (FIXTURES / "payload-2.txt").read_text(),
            ),
            ("Copy operation 2 Signer", "signer<>&\"'\\\n" + "long-original-" * 300),
        ):
            selector = f'button[aria-label="{label}"]'
            browser("focus", selector)
            browser("press", "Space")
            browser("wait", "--text", "Couldn’t copy.")
            self.assertEqual(evaluate("document.activeElement.ariaLabel"), label)
            self.assertEqual(
                evaluate(
                    "document.activeElement.nextElementSibling.querySelector('textarea').value"
                ),
                expected,
            )
            self.assertTrue(
                evaluate(
                    "document.activeElement.nextElementSibling.querySelector('textarea').readOnly"
                )
            )
            for width in (320, 390, 800, 1440):
                browser("set", "viewport", str(width), "900")
                self.assertTrue(
                    evaluate("document.documentElement.scrollWidth <= innerWidth")
                )
                self.assertTrue(
                    evaluate("""Array.from(document.querySelectorAll(
                        '.prism-copy-fallback textarea'))
                        .every(e => { const r = e.getBoundingClientRect();
                          return r.left >= 0 && r.right <= innerWidth; })""")
                )
            evaluate("document.querySelector('.prism-copy-status').textContent = ''")

    def test_no_scripts_keep_keyboard_disclosures_and_selectable_complete_payloads(
        self,
    ) -> None:
        browser("close")
        browser("open")
        browser("network", "route", "*", "--abort", "--resource-type", "script")
        try:
            browser("open", URL + "/operations.html")
            self.assertFalse(
                evaluate(
                    "document.documentElement.classList.contains('prism-copy-ready')"
                )
            )
            browser("focus", ".prism-operation > summary")
            browser("press", "Enter")
            self.assertTrue(evaluate("document.querySelector('.prism-operation').open"))
            browser("press", "Space")
            self.assertFalse(
                evaluate("document.querySelector('.prism-operation').open")
            )
            browser("press", "Enter")
            browser("focus", ".prism-operation-payload > summary")
            browser("press", "Space")
            self.assertTrue(
                evaluate("document.querySelector('.prism-operation-payload').open")
            )
            self.assertEqual(
                evaluate(
                    "document.querySelector('.prism-operation-payload pre').textContent"
                ),
                (FIXTURES / "payload-1.txt").read_text(),
            )
            self.assertNotEqual(
                evaluate(
                    "getComputedStyle(document.querySelector("
                    "'.prism-operation-payload pre')).userSelect"
                ),
                "none",
            )
            self.assertTrue(
                evaluate("document.documentElement.scrollWidth <= innerWidth")
            )
        finally:
            browser("network", "unroute", "*")
            browser("close")
            browser("open")
            browser("network", "route", "**/htmx.org/**", "--abort")

    def test_unusable_history_and_explicit_empty_survive_without_internal_secrets(
        self,
    ) -> None:
        browser("open", URL + "/unusable.html")
        self.assertEqual(
            evaluate("document.querySelectorAll('.prism-operation').length"), 2
        )
        self.assertEqual(
            evaluate(
                "Array.from(document.querySelectorAll('.prism-operation-outcome'))"
                ".map(e => e.textContent)"
            ),
            ["Rejected", "Rejected"],
        )
        browser("focus", ".prism-operation > summary")
        browser("press", "Enter")
        browser("focus", ".prism-operation-domain > summary")
        browser("press", "Space")
        self.assertTrue(
            evaluate(
                "document.querySelector('.prism-operation-diagnostic').checkVisibility()"
            )
        )
        self.assertIn(
            "public key id is not a valid uri fragment",
            evaluate(
                "document.querySelector('.prism-operation-diagnostic').textContent"
            ),
        )
        for name in ("empty", "internal"):
            browser("open", URL + f"/{name}.html")
            self.assertEqual(
                evaluate("document.querySelectorAll('.prism-operation').length"), 0
            )
            self.assertIn(
                "No operation history was retrieved"
                if name == "internal"
                else "No indexed operations",
                evaluate("document.querySelector('#operation-history').textContent"),
            )
            self.assertNotIn(
                "secret-password", evaluate("document.documentElement.outerHTML")
            )
            self.assertNotIn(
                "/private/db", evaluate("document.documentElement.outerHTML")
            )

    def test_long_expanded_history_stays_in_bounds_with_readable_raw_payload(
        self,
    ) -> None:
        evaluate(
            "document.querySelectorAll('main details').forEach(e => e.open = true)"
        )
        for width in (320, 390, 600, 800, 801, 1024, 1280, 1440):
            with self.subTest(width=width):
                browser("set", "viewport", str(width), "900")
                self.assertTrue(
                    evaluate("document.documentElement.scrollWidth <= innerWidth")
                )
                self.assertTrue(
                    evaluate("""Array.from(document.querySelectorAll(
                        '#operation-history summary, #operation-history button'))
                        .every(e => e.getBoundingClientRect().height >= 48)""")
                )
                self.assertTrue(
                    evaluate("""Array.from(document.querySelectorAll(
                        '#operation-history pre')).every(e => {
                          const s = getComputedStyle(e);
                          const r = e.getBoundingClientRect();
                          return r.left >= 0 && r.right <= innerWidth &&
                            s.whiteSpace === 'pre-wrap' && s.userSelect !== 'none';
                        })""")
                )
                if width in (390, 1440):
                    audit = browser("a11y", "--selector", "#operation-history")
                    self.assertEqual(audit["counts"]["violations"], 0, audit)
                    if os.environ.get("PRISM_SCREENSHOT_DIR"):
                        destination = Path(os.environ["PRISM_SCREENSHOT_DIR"])
                        destination.mkdir(parents=True, exist_ok=True)
                        # Keep one real payload expanded for page-level review.
                        evaluate(
                            "document.querySelectorAll('.prism-operation')[1]"
                            ".open = false"
                        )
                        browser(
                            "screenshot",
                            str(destination / f"operations-{width}.png"),
                            "--full",
                        )
                        evaluate(
                            "document.querySelectorAll('.prism-operation')[1]"
                            ".open = true"
                        )


if __name__ == "__main__":
    unittest.main()
