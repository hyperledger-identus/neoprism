"""Browser seam for test-only rendered Resolver fixtures; no production sample route.

Export with the ignored export_identity_browser_fixtures Rust test, setting
PRISM_FIXTURE_DIR to a directory outside the checkout; run just build-assets.
This suite serves those exact rendered pages and built assets on a private port.
"""

import functools
import http.server
import os
import shutil
import threading
import unittest
from pathlib import Path

from clipboard_permissions import grant_clipboard_permissions
from prism_shell import browser, evaluate

FIXTURES = Path(os.environ["PRISM_FIXTURE_DIR"])
ROOT = Path(__file__).resolve().parents[2]
PORT = int(os.environ.get("PRISM_IDENTITY_PORT", "18773"))
URL = f"http://127.0.0.1:{PORT}"


class PrismIdentity(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        shutil.copytree(
            ROOT / "bin/neoprism-node/assets", FIXTURES / "assets", dirs_exist_ok=True
        )
        handler = functools.partial(
            http.server.SimpleHTTPRequestHandler, directory=str(FIXTURES)
        )
        cls.server = http.server.ThreadingHTTPServer(("127.0.0.1", PORT), handler)
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()

    @classmethod
    def tearDownClass(cls) -> None:
        cls.server.shutdown()
        cls.server.server_close()
        browser("close")

    def setUp(self) -> None:
        browser("set", "viewport", "390", "900")
        browser("open", URL + "/indexed.html")

    def test_responsive_dossier_has_no_overflow_and_places_evidence_beside_or_below(
        self,
    ) -> None:
        for width in (320, 390, 720, 800, 801, 1024, 1280, 1440):
            with self.subTest(width=width):
                browser("set", "viewport", str(width), "900")
                evaluate(
                    "document.querySelectorAll('.prism-key')"
                    ".forEach(e => e.open = true)"
                )
                self.assertTrue(
                    evaluate("document.documentElement.scrollWidth <= innerWidth")
                )
                geometry = evaluate("""(() => {
                    const d = document.querySelector('.prism-dossier')
                        .getBoundingClientRect();
                    const r = document.querySelector('.prism-evidence-rail')
                        .getBoundingClientRect();
                    return {beside: r.left >= d.right, below: r.top >= d.bottom};
                })()""")
                self.assertTrue(geometry["beside" if width >= 1280 else "below"])
                self.assertTrue(
                    evaluate("""Array.from(document.querySelectorAll(
                    '.prism-key summary, .prism-copy, '
                    + '.prism-identity-actions a, .prism-evidence summary'))
                    .filter(e => e.checkVisibility())
                    .every(e => e.getBoundingClientRect().height >= 48)""")
                )

    def test_copy_success_preserves_complete_did_coordinate_and_evidence(self) -> None:
        self.enterContext(grant_clipboard_permissions(browser, URL))
        evaluate("""document.querySelectorAll('main details')
            .forEach(e => e.open = true);""")
        for label in (
            "Copy DID",
            "Copy key a-master X coordinate",
            "Copy Transaction ID",
            "Copy Operation ID",
        ):
            with self.subTest(label=label):
                selector = f'button[aria-label="{label}"]'
                expected = browser("get", "attr", selector, "data-copy-value")["value"]
                browser("focus", selector)
                browser("press", "Space")
                browser("wait", "--text", "Copied to clipboard.")
                self.assertEqual(browser("clipboard", "read")["text"], expected)
                self.assertEqual(evaluate("document.activeElement.ariaLabel"), label)
                self.assertTrue(
                    evaluate("""Array.from(document.querySelectorAll(
                    '[aria-live=polite], [role=status]'))
                    .some(e => /copied/i.test(e.textContent))""")
                )
                # Clear feedback so the next wait observes that copy's completion.
                evaluate(
                    "document.querySelector('.prism-copy-status').textContent = ''"
                )

    def test_copy_failure_announces_and_offers_exact_selectable_fallback(self) -> None:
        evaluate("""Object.defineProperty(navigator, 'clipboard', {configurable:true,
            value:{writeText: async () => {throw new Error('denied');}}});""")
        selector = 'button[aria-label="Copy DID"]'
        expected = browser("get", "attr", selector, "data-copy-value")["value"]
        browser("focus", selector)
        browser("press", "Space")
        browser(
            "wait",
            "--fn",
            """Array.from(document.querySelectorAll(
            'input[readonly], textarea[readonly]')).some(e => e.checkVisibility())""",
        )
        self.assertEqual(evaluate("document.activeElement.ariaLabel"), "Copy DID")
        self.assertEqual(
            evaluate("document.querySelector('[readonly]').value"), expected
        )
        self.assertTrue(
            evaluate("""Array.from(document.querySelectorAll(
            '[aria-live=polite], [role=status]'))
            .some(e => /couldn|unable|failed|could not/i.test(e.textContent))""")
        )
        self.assertTrue(
            evaluate("""getComputedStyle(
            document.querySelector('[readonly]')).userSelect !== 'none'""")
        )
        self.assertFalse(
            evaluate("!!document.querySelector('[role=dialog], dialog[open]')")
        )
        for width in (320, 390, 600, 800, 1440):
            with self.subTest(width=width):
                browser("set", "viewport", str(width), "900")
                self.assertTrue(
                    evaluate("document.documentElement.scrollWidth <= innerWidth")
                )
                self.assertTrue(
                    evaluate("""Array.from(document.querySelectorAll(
                    '.prism-identity-actions > *, .prism-copy-fallback textarea'))
                    .every(e => {
                        const r = e.getBoundingClientRect();
                        return r.left >= 0 && r.right <= innerWidth;
                    })""")
                )

    def test_pending_resolution_retains_value_and_control_geometry(self) -> None:
        browser("set", "viewport", "1440", "1000")
        value = evaluate("document.querySelector('#did').value")
        before = evaluate(
            "document.querySelector('button[type=submit]').getBoundingClientRect().width"
        )
        evaluate(
            "document.querySelector('form').addEventListener('submit', "
            "e => e.preventDefault(), {once:true})"
        )
        browser("focus", "#did")
        browser("press", "Enter")
        self.assertTrue(
            evaluate("document.querySelector('button[type=submit]').disabled")
        )
        self.assertEqual(
            evaluate("document.querySelector('button[type=submit]').textContent"),
            "Resolving…",
        )
        self.assertEqual(evaluate("document.querySelector('#did').value"), value)
        self.assertEqual(
            evaluate(
                "document.querySelector('button[type=submit]').getBoundingClientRect().width"
            ),
            before,
        )

    def test_desktop_evidence_is_open_mobile_is_a_native_disclosure(self) -> None:
        browser("set", "viewport", "1440", "1000")
        browser("open", URL + "/indexed.html")
        self.assertTrue(evaluate("document.querySelector('#ledger-evidence').open"))
        browser("set", "viewport", "390", "900")
        browser("open", URL + "/indexed.html")
        self.assertFalse(evaluate("document.querySelector('#ledger-evidence').open"))

    def test_keyboard_disclosures_and_independent_api_action(self) -> None:
        browser("focus", ".prism-key summary")
        browser("press", "Enter")
        self.assertTrue(evaluate("document.querySelector('.prism-key').open"))
        browser("press", "Space")
        self.assertFalse(evaluate("document.querySelector('.prism-key').open"))
        browser("focus", "#ledger-evidence summary")
        browser("press", "Enter")
        self.assertTrue(evaluate("document.querySelector('#ledger-evidence').open"))
        self.assertTrue(
            evaluate("""Array.from(document.querySelectorAll('a'))
            .filter(a => a.textContent === 'Open Resolver API')
            .every(a => a.target === '_blank' && a.rel.includes('noopener'))""")
        )
        self.assertNotIn(
            "Not requested", evaluate("document.querySelector('main').textContent")
        )

    def test_no_scripts_keep_complete_selectable_values_and_native_disclosures(
        self,
    ) -> None:
        browser("close")
        browser("open")
        browser("network", "route", "*", "--abort", "--resource-type", "script")
        try:
            browser("open", URL + "/indexed.html")
            browser("focus", ".prism-key summary")
            browser("press", "Enter")
            self.assertTrue(evaluate("document.querySelector('.prism-key').open"))
            self.assertIn(
                "eb5mfvncu6xVoGKVzocLBwKb_NstzijZWfKBWxb4F5g",
                evaluate("document.querySelector('.prism-key').textContent"),
            )
            self.assertNotEqual(
                evaluate(
                    "getComputedStyle(document.querySelector('.prism-key code'))"
                    ".userSelect"
                ),
                "none",
            )
        finally:
            browser("network", "unroute", "*")


if __name__ == "__main__":
    unittest.main()
