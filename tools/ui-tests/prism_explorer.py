"""Explorer browser checks using exported HTTP fixtures, never production data.

See README.md for the opt-in Rust export command. Polling behavior belongs to #380.
"""

import os
import threading
import unittest
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse

from prism_shell import browser, evaluate

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = Path(os.environ["PRISM_FIXTURE_DIR"])


class FixtureHandler(SimpleHTTPRequestHandler):
    def do_GET(self) -> None:
        url = urlparse(self.path)
        if url.path.startswith("/assets/"):
            self.path = "/bin/neoprism-node/assets/" + url.path.removeprefix("/assets/")
            return super().do_GET()
        fixture = parse_qs(url.query).get("fixture", ["full"])[0]
        if url.path == "/explorer" and "page=12" in url.query:
            fixture = "partial"
        if fixture not in {"full", "partial", "beyond", "empty", "failure", "invalid"}:
            self.send_error(404)
            return
        payload = (FIXTURES / (fixture + ".html")).read_bytes()
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.end_headers()
        self.wfile.write(payload)

    def __init__(self, *args: object, **kwargs: object) -> None:
        super().__init__(*args, directory=str(ROOT), **kwargs)


class ExplorerBrowser(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.server = ThreadingHTTPServer(("127.0.0.1", 0), FixtureHandler)
        cls.url = f"http://127.0.0.1:{cls.server.server_port}"
        threading.Thread(target=cls.server.serve_forever, daemon=True).start()

    @classmethod
    def tearDownClass(cls) -> None:
        browser("close")
        cls.server.shutdown()
        cls.server.server_close()

    def setUp(self) -> None:
        # Deterministic rendered-HTML seam: do not hit a CDN or run polling.
        browser("network", "route", "*unpkg.com*", "--abort")
        browser("set", "viewport", "1440", "900")
        browser("open", self.url + "/explorer?page=3")

    def test_all_100_rows_have_compact_pitch_and_independent_full_value_actions(
        self,
    ) -> None:
        self.assertEqual(
            evaluate("document.querySelectorAll('.prism-did-row').length"), 100
        )
        for width in (1440, 1024, 801, 800, 390, 320):
            browser("set", "viewport", str(width), "900")
            with self.subTest(width=width):
                self.assertTrue(
                    evaluate("document.documentElement.scrollWidth <= innerWidth")
                )
                self.assertTrue(
                    evaluate("""[...document.querySelectorAll('.prism-did-row')]
                .every(row => {
                  const a = row.querySelector('a'), b = row.querySelector('button');
                  const r = row.getBoundingClientRect();
                  return r.height === 57 && a.getBoundingClientRect().height === 48
                    && b.getBoundingClientRect().height === 48
                    && b.getBoundingClientRect().width === 48
                    && !a.contains(b) && new URL(a.href).searchParams.get('did')
                      === b.dataset.copyValue
                    && getComputedStyle(a.querySelector('span')).fontSize === '14px';
                })""")
                )
                self.assertTrue(
                    evaluate("""[...document.querySelectorAll(
                      '#did-list a, #did-list button')]
                  .filter(e => e.checkVisibility()).every(e => {
                    const r = e.getBoundingClientRect();
                    return r.left >= 0 && r.right <= innerWidth;
                  })""")
                )

    def test_top_and_bottom_pagination_are_centered_with_native_get_and_equal_numbers(
        self,
    ) -> None:
        for width in (1440, 390, 320):
            browser("set", "viewport", str(width), "900")
            self.assertTrue(
                evaluate("""[...document.querySelectorAll('.prism-pagination')]
            .every(nav => {
              const visible = [...nav.querySelectorAll('.prism-page-number')]
                .filter(e => e.checkVisibility());
              const rect = nav.getBoundingClientRect();
              const controls = nav.querySelector('.prism-page-controls')
                .getBoundingClientRect();
              return visible.every(e => e.getBoundingClientRect().width === 48
                && e.getBoundingClientRect().height === 48)
                && Math.abs((rect.left + rect.right
                  - controls.left - controls.right) / 2) < 1
                && nav.querySelector('[aria-current=page]').textContent === '3'
                && [...nav.querySelectorAll('a')]
                  .every(a => a.href.includes('/explorer?page='));
            })""")
            )
            if width <= 390:
                self.assertEqual(
                    evaluate("""[...[...document.querySelectorAll(
                    '.prism-page-numbers')].find(e => e.checkVisibility()).children]
                    .filter(e => e.checkVisibility())
                    .map(e => e.textContent).join(' ')"""),
                    "1 … 3 … 12",
                )
        browser("close")
        browser("open")
        browser("set", "viewport", "320", "900")
        browser("network", "route", "*", "--abort", "--resource-type", "script")
        try:
            browser("open", self.url + "/explorer?page=3")
            self.assertIsNone(evaluate("document.querySelector('.prism-copy-status')"))
            self.assertTrue(
                evaluate("document.querySelector('.prism-did-full').checkVisibility()")
            )
            browser(
                "click",
                ".prism-pagination:first-child .prism-page-mobile "
                "a[href='/explorer?page=12']",
            )
            browser("wait", "--url", "**/explorer?page=12")
            self.assertEqual(
                evaluate("document.querySelectorAll('.prism-did-row').length"), 3
            )
            self.assertIn(
                "Page 12 of 12",
                evaluate("document.querySelector('#did-list').textContent"),
            )
            self.assertNotEqual(
                evaluate(
                    "getComputedStyle(document.querySelector('.prism-did-link')).userSelect"
                ),
                "none",
            )
        finally:
            browser("network", "unroute", "*")

    def test_row_keyboard_copy_and_failure_fallback_preserve_original_and_focus(
        self,
    ) -> None:
        browser("set", "viewport", "320", "900")
        evaluate("""window.copied = null;
          Object.defineProperty(navigator, 'clipboard', {configurable: true,
            value: {writeText: async value => {window.copied = value}}})""")
        browser("focus", ".prism-did-row:first-child a")
        browser("press", "Tab")
        self.assertTrue(
            evaluate(
                "document.activeElement.matches('.prism-did-row:first-child button')"
            )
        )
        browser("press", "Enter")
        browser("wait", "--text", "Copied to clipboard.")
        self.assertEqual(evaluate("window.copied"), "did:prism:" + f"{200:064x}")
        evaluate(
            "Object.defineProperty(navigator, 'clipboard', "
            "{configurable: true, value: undefined})"
        )
        browser("press", "Enter")
        browser("wait", "--text", "Couldn’t copy.")
        self.assertTrue(
            evaluate("document.activeElement.matches('button[data-copy-value]')")
        )
        self.assertTrue(
            evaluate("""const field = document.querySelector(
            '.prism-copy-fallback textarea');
          field.value === window.copied && field.readOnly
            && field.getBoundingClientRect().right <= innerWidth""")
        )

    def test_empty_out_of_range_invalid_and_failure_offer_visible_recovery(
        self,
    ) -> None:
        for fixture, expected in (
            ("empty", "No indexed DIDs yet"),
            ("beyond", "beyond the last page"),
            ("failure", "Couldn’t load indexed DIDs"),
            ("invalid", "Choose a valid page"),
        ):
            browser("open", self.url + "/explorer?fixture=" + fixture)
            for width in (1440, 390, 320):
                browser("set", "viewport", str(width), "900")
                self.assertIn(
                    expected,
                    evaluate("document.querySelector('#did-list').textContent"),
                )
                self.assertTrue(
                    evaluate("""[...document.querySelectorAll('#did-list a')]
                .every(a => {
                  const r = a.getBoundingClientRect();
                  return r.height >= 48 && r.right <= innerWidth;
                })""")
                )
                if os.environ.get("PRISM_SCREENSHOT_DIR") and width in (1440, 390):
                    browser(
                        "screenshot",
                        str(
                            Path(os.environ["PRISM_SCREENSHOT_DIR"])
                            / f"{fixture}-{width}.png"
                        ),
                        "--full",
                    )

    def test_desktop_full_and_mobile_abbreviation_keep_complete_accessible_original(
        self,
    ) -> None:
        self.assertTrue(
            evaluate("""const full = document.querySelector('.prism-did-full');
          full.checkVisibility() && full.textContent.length === 74
          && full.getBoundingClientRect().right
            < full.parentElement.getBoundingClientRect().right""")
        )
        for width in (1440, 390):
            browser("set", "viewport", str(width), "900")
            self.assertTrue(
                evaluate("""(() => {
              const link = document.querySelector('.prism-did-link');
              return link.getAttribute('aria-label') === 'Resolve '
                + link.nextElementSibling.dataset.copyValue; })()""")
            )
            if os.environ.get("PRISM_SCREENSHOT_DIR"):
                browser(
                    "screenshot",
                    str(Path(os.environ["PRISM_SCREENSHOT_DIR"]) / f"full-{width}.png"),
                    "--full",
                )


if __name__ == "__main__":
    unittest.main()
