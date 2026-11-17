"""#379 cursor browser tests using deterministic HTML exported through real HTTP routes.

Inside nix develop, set PRISM_CURSOR_FIXTURES when running cargo test -p
neoprism-node cursor_tests, then run this file with that same environment variable.
The fixture-only server binds a ticket-specific port; it never adds app routes.
"""

import functools
import json
import os
import subprocess
import sys
import threading
import unittest
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any

from clipboard_permissions import grant_clipboard_permissions

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = Path(os.environ["PRISM_CURSOR_FIXTURES"])
PORT = int(os.environ.get("PRISM_CURSOR_PORT", "18779"))
BASE = f"http://127.0.0.1:{PORT}"
SESSION = os.environ.get("PRISM_TEST_SESSION") or (
    "prism-379-"
    + subprocess.check_output(
        ["agent-browser", "session", "id", "--scope", "worktree"], text=True
    ).strip()
)
MODE = "supplied"
RELEASE = threading.Event()


def browser(*args: str) -> dict[str, Any]:
    result = subprocess.run(
        ["agent-browser", "--session", SESSION, "--json", *args],
        check=True,
        capture_output=True,
        text=True,
    )
    payload = json.loads(result.stdout)
    if not payload["success"]:
        raise AssertionError(payload)
    return payload["data"]


def evaluate(script: str) -> Any:
    return browser("eval", script)["result"]


class Handler(SimpleHTTPRequestHandler):
    def do_GET(self) -> None:
        if self.path.startswith("/assets/"):
            self.path = self.path.removeprefix("/assets")
            return super().do_GET()
        if self.path.startswith("/explorer/did-list?"):
            data = (FIXTURES / "empty-list.fragment").read_bytes()
        elif self.path == "/explorer/dlt-cursor":
            if MODE == "held-failure":
                RELEASE.wait(timeout=10)
                self.send_error(503)
                return
            data = (FIXTURES / f"{MODE}.fragment").read_bytes()
        elif self.path.endswith(".html"):
            data = (FIXTURES / Path(self.path).name).read_bytes()
        else:
            self.send_error(404)
            return
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.end_headers()
        self.wfile.write(data)

    def log_message(self, format: str, *args: Any) -> None:
        pass


class CursorBrowser(unittest.TestCase):
    def setUp(self) -> None:
        global MODE
        MODE = "supplied"
        RELEASE.clear()
        browser("set", "viewport", "390", "898")

    def test_native_disclosure_and_all_values_fit_narrow_widths_without_scripts(
        self,
    ) -> None:
        browser("close")
        browser("open")
        browser("network", "route", "*", "--abort", "--resource-type", "script")
        try:
            for fixture in ("supplied", "missing-timestamp", "long", "waiting"):
                browser("open", BASE + f"/{fixture}.html")
                if fixture == "waiting":
                    self.assertEqual(
                        evaluate(
                            "document.querySelector('[data-cursor-status]').textContent"
                        ),
                        "Waiting for sync information",
                    )
                    self.assertIsNone(
                        evaluate("document.querySelector('.prism-sync details')")
                    )
                else:
                    browser("focus", ".prism-sync summary")
                    browser("press", "Enter")
                    self.assertTrue(
                        evaluate("document.querySelector('.prism-sync details').open")
                    )
                    browser("press", "Space")
                    self.assertFalse(
                        evaluate("document.querySelector('.prism-sync details').open")
                    )
                    browser("press", "Enter")
                    if fixture in ("missing-timestamp", "long"):
                        self.assertEqual(
                            evaluate(
                                "document.querySelector('.prism-sync-summary "
                                "> div:last-child dd').textContent"
                            ),
                            "Not provided",
                        )
                        self.assertIsNone(
                            evaluate("document.querySelector('.prism-sync time')")
                        )
                    self.assertTrue(
                        evaluate(
                            "Array.from(document.querySelectorAll('.prism-sync code'))"
                            ".every(e => getComputedStyle(e).userSelect === 'text')"
                        )
                    )
                audit = browser("a11y", "--selector", "#dlt-cursor-card")
                self.assertEqual(audit["counts"]["violations"], 0, audit)
                for width in (320, 390, 600, 720, 800, 801, 1024, 1440):
                    with self.subTest(fixture=fixture, width=width):
                        browser("set", "viewport", str(width), "898")
                        self.assertTrue(
                            evaluate(
                                "Array.from(document.querySelectorAll("
                                "'.prism-sync, .prism-sync *'))"
                                ".filter(e => e.checkVisibility()).every(e => {"
                                "const r=e.getBoundingClientRect();"
                                "return r.left >= 0 && r.right <= innerWidth})"
                            )
                        )
                        self.assertTrue(
                            evaluate(
                                "Array.from(document.querySelectorAll("
                                "'.prism-sync button, .prism-sync summary'))"
                                ".every(e => e.getBoundingClientRect().height >= 48)"
                            )
                        )
                        columns = evaluate(
                            "getComputedStyle(document.querySelector("
                            "'.prism-sync-summary') || document.querySelector("
                            "'.prism-sync')).gridTemplateColumns"
                        )
                        if fixture != "waiting":
                            self.assertEqual(
                                len(columns.split()), 1 if width <= 600 else 2
                            )
        finally:
            browser("network", "unroute", "*")
            browser("close")

    def test_pending_and_failed_requests_report_only_observable_browser_facts(
        self,
    ) -> None:
        global MODE
        MODE = "held-failure"
        browser("open", BASE + "/supplied.html")
        receipt = evaluate(
            "document.querySelector('[data-cursor-receipt]').textCon"
            "tent.split(' · ')[0]"
        )
        browser(
            "wait",
            "--fn",
            "document.querySelector('#dlt-cursor-card').ariaBusy === 'true'",
        )
        self.assertEqual(
            evaluate("document.querySelector('[data-cursor-status]').textContent"),
            "Checking for sync information…",
        )
        RELEASE.set()
        browser(
            "wait",
            "--fn",
            "document.querySelector('[data-cursor-status]')"
            ".textContent.includes('Couldn’t refresh')",
        )
        self.assertEqual(
            evaluate("document.querySelector('[data-cursor-status]').textContent"),
            "Couldn’t refresh · showing last received cursor",
        )
        self.assertEqual(
            evaluate(
                "document.querySelector('[data-cursor-receipt]').textCon"
                "tent.split(' · ')[0]"
            ),
            receipt,
        )
        self.assertEqual(
            evaluate("document.querySelector('[data-copy-value]').dataset.copyValue"),
            "1042816",
        )
        self.assertIn(
            "Distance to chain tip unknown",
            evaluate("document.querySelector('#dlt-cursor-card').textContent"),
        )
        self.assertNotIn(
            "503", evaluate("document.querySelector('#dlt-cursor-card').textContent")
        )
        self.assertNotIn(
            "source failed",
            evaluate("document.querySelector('#dlt-cursor-card').textContent"),
        )

    def test_missing_cursor_failure_does_not_invent_source_configuration(self) -> None:
        global MODE
        MODE = "held-failure"
        browser("open", BASE + "/waiting.html")
        self.assertIn(
            "No cursor was supplied",
            evaluate("document.querySelector('[data-cursor-receipt]').textContent"),
        )
        browser(
            "wait",
            "--fn",
            "document.querySelector('#dlt-cursor-card').ariaBusy === 'true'",
        )
        RELEASE.set()
        browser(
            "wait",
            "--fn",
            "document.querySelector('[data-cursor-status]')"
            ".textContent.includes('Couldn’t refresh')",
        )
        self.assertEqual(
            evaluate("document.querySelector('[data-cursor-status]').textContent"),
            "Couldn’t refresh sync information · no cursor received",
        )
        self.assertIsNone(evaluate("document.querySelector('.prism-sync details')"))
        self.assertIn(
            "Distance to chain tip unknown",
            evaluate("document.querySelector('.prism-sync').textContent"),
        )

    def test_repeated_snapshot_updates_browser_receipt_not_block_time(self) -> None:
        browser("open", BASE + "/supplied.html")
        receipt = evaluate(
            "document.querySelector('[data-cursor-receipt]').textCon"
            "tent.split(' · ')[0]"
        )
        browser(
            "wait",
            "--fn",
            "document.querySelector('[data-cursor-receipt]').textCon"
            "tent.split(' · ')[0] !== " + json.dumps(receipt),
        )
        self.assertEqual(
            evaluate("document.querySelector('.prism-sync-summary time').dateTime"),
            "2026-11-17T14:51:00.123456789Z",
        )
        self.assertEqual(
            evaluate("document.querySelector('[data-copy-value]').dataset.copyValue"),
            "1042816",
        )
        self.assertIn(
            "does not prove cursor advancement",
            evaluate("document.querySelector('.prism-sync-explanation').textContent"),
        )

    def test_shared_copy_preserves_complete_values_and_offers_manual_fallback(
        self,
    ) -> None:
        # Keep actual refresh delivery active: focused copy controls must survive it.
        browser("close")
        browser("open")
        browser("network", "route", "*htmx*", "--abort", "--resource-type", "script")
        try:
            self.enterContext(grant_clipboard_permissions(browser, BASE))
            for fixture, values in (
                ("supplied", ["1042816", "ab" * 32, "2026-11-17T14:51:00.123456789Z"]),
                ("long", ["18446744073709551615", "ab" * 512]),
            ):
                browser("open", BASE + f"/{fixture}.html")
                browser("focus", ".prism-sync summary")
                browser("press", "Enter")
                for expected in values:
                    selector = (
                        ".prism-sync [data-copy-value=" + json.dumps(expected) + "]"
                    )
                    browser("focus", selector)
                    browser("press", "Enter")
                    browser("wait", "--text", "Copied to clipboard.")
                    self.assertEqual(browser("clipboard", "read")["text"], expected)
                    evaluate(
                        "document.querySelector('.prism-copy-status').textContent = ''"
                    )
                    self.assertEqual(
                        evaluate("document.activeElement.dataset.copyValue"), expected
                    )
            evaluate("""Object.defineProperty(navigator, 'clipboard', {
              configurable: true, value: {
                writeText: () => Promise.reject(new Error('denied'))
              }
            })""")
            browser("click", '.prism-sync [aria-label="Copy block hash"]')
            browser(
                "wait",
                "--fn",
                "!!document.querySelector("
                "'.prism-sync textarea[readonly], .prism-sync input[readonly]')",
            )
            self.assertEqual(
                evaluate(
                    "document.querySelector('.prism-sync textarea[readonly], "
                    " .prism-sync input[readonly]').value"
                ),
                "ab" * 512,
            )
            browser("set", "viewport", "320", "898")
            self.assertTrue(
                evaluate(
                    "Array.from(document.querySelectorAll("
                    "'.prism-sync textarea, .prism-sync input')).every("
                    "e => e.getBoundingClientRect().right <= innerWidth)"
                )
            )
        finally:
            browser("network", "unroute", "*htmx*")
            browser("close")

    def test_browser_receipt_is_distinct_from_block_time(self) -> None:
        browser("open", BASE + "/supplied.html")
        self.assertIn(
            "This browser last successfully received the cursor snapshot",
            evaluate("document.querySelector('[data-cursor-receipt]').textContent"),
        )
        self.assertEqual(
            evaluate("document.querySelector('.prism-sync-summary time').dateTime"),
            "2026-11-17T14:51:00.123456789Z",
        )
        self.assertNotIn(
            "upstream",
            evaluate("document.querySelector('[data-cursor-status]').textContent"),
        )


if __name__ == "__main__":
    server = ThreadingHTTPServer(
        ("127.0.0.1", PORT),
        functools.partial(Handler, directory=str(ROOT / "bin/neoprism-node/assets")),
    )
    if sys.argv[1:] == ["--serve"]:
        try:
            server.serve_forever()
        finally:
            server.server_close()
        sys.exit(0)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    try:
        unittest.main()
    finally:
        RELEASE.set()
        browser("close")
        server.shutdown()
        server.server_close()
