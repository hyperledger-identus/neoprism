"""Resolver UI outcomes at the approved HTTP/rendering and browser seams.

Serves real Maud fixture pages; delays/failures exist only at this HTTP boundary.
No production fixture routes or backend semantics are changed.
"""

import contextlib
import functools
import http.server
import json
import os
import shutil
import threading
import unittest
from pathlib import Path
from urllib.parse import parse_qs, urlencode, urlparse

from prism_shell import browser, evaluate

FIXTURES = Path(os.environ["PRISM_OUTCOME_FIXTURES"])
ROOT = Path(__file__).resolve().parents[2]
PORT = int(os.environ.get("PRISM_OUTCOME_PORT", "18777"))
URL = f"http://127.0.0.1:{PORT}"
INPUTS = json.loads((FIXTURES / "inputs.json").read_text())
OUTCOMES = {
    "invalid": "Check the DID format",
    "initial": "Check the encoded initial state",
    "absent": "No indexed state on this node",
    "unusable": "Stored operations cannot initialize DID state",
    "internal": "Couldn’t retrieve identity state",
    "local": "Locally derived",
    "published": "Indexed state available",
    "deactivated": "Deactivated",
}


class OutcomeServer(http.server.ThreadingHTTPServer):
    outcome: str = "absent"
    hold = threading.Event()
    started = threading.Event()
    delayed: bool = False
    failure: int = 0
    requests: list[str] = []


class Handler(http.server.SimpleHTTPRequestHandler):
    server: OutcomeServer

    def do_GET(self) -> None:
        query = parse_qs(urlparse(self.path).query, keep_blank_values=True)
        if urlparse(self.path).path == "/resolver":
            self.server.requests.append(self.path)
            if self.server.delayed:
                self.server.started.set()
                self.server.hold.wait(20)
            if self.server.failure < 0:
                self.close_connection = True
                return  # Deliberate network disconnect, no response fabricated.
            if self.server.failure:
                self.send_error(self.server.failure, "private-server-detail")
                return
            name = self.server.outcome if "did" in query else "entry"
            content = (FIXTURES / f"{name}.html").read_bytes()
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(content)))
            self.end_headers()
            # Restoration deliberately aborts a pending browser request.
            with contextlib.suppress(BrokenPipeError, ConnectionResetError):
                self.wfile.write(content)
            return
        super().do_GET()

    def log_message(self, format: str, *args: object) -> None:
        pass


class PrismOutcomes(unittest.TestCase):
    server: OutcomeServer

    @classmethod
    def setUpClass(cls) -> None:
        shutil.copytree(
            ROOT / "bin/neoprism-node/assets", FIXTURES / "assets", dirs_exist_ok=True
        )
        handler = functools.partial(Handler, directory=str(FIXTURES))
        cls.server = OutcomeServer(("127.0.0.1", PORT), handler)
        threading.Thread(target=cls.server.serve_forever, daemon=True).start()
        browser("open")
        browser("network", "route", "**/htmx.org/**", "--abort")

    @classmethod
    def tearDownClass(cls) -> None:
        cls.server.hold.set()
        cls.server.shutdown()
        cls.server.server_close()
        browser("close")

    def setUp(self) -> None:
        self.server.delayed = False
        self.server.failure = 0
        self.server.requests.clear()
        self.server.hold.clear()
        self.server.started.clear()
        self.server.outcome = "absent"
        browser("set", "viewport", "390", "900")
        browser("open", URL + "/resolver")

    def test_without_scripts_every_outcome_and_get_retry_remain_usable(self) -> None:
        browser("close")
        browser("open")
        browser("network", "route", "*", "--abort", "--resource-type", "script")
        try:
            browser("open", URL + "/resolver")
            for name, expected in OUTCOMES.items():
                with self.subTest(outcome=name):
                    self.server.outcome = name
                    browser("fill", "#did", INPUTS[name])
                    browser("press", "Enter")
                    browser("wait", "--text", expected)
                    self.assertEqual(
                        evaluate("document.querySelector('#did').value"), INPUTS[name]
                    )
                    self.assertEqual(
                        parse_qs(urlparse(browser("get", "url")["url"]).query)["did"],
                        [INPUTS[name]],
                    )
                    self.assertFalse(
                        evaluate(
                            "document.querySelector('button[type=submit]').disabled"
                        )
                    )
                    self.assertFalse(
                        evaluate(
                            "document.querySelector('#resolution-pending').checkVisibility()"
                        )
                    )
                    if name in ("invalid", "initial", "absent", "unusable", "internal"):
                        browser("focus", ".prism-outcome-diagnostics > summary")
                        browser("press", "Space")
                        self.assertTrue(
                            evaluate(
                                "document.querySelector('.prism-outcome-diagnostics').open"
                            )
                        )
                        self.assertEqual(
                            evaluate(
                                "document.querySelector('a[href=\"/explorer\"]')"
                                " !== null"
                            ),
                            True,
                        )
                        browser("click", "#resolution-result a[href='#did']")
                        self.assertEqual(evaluate("document.activeElement.id"), "did")
                        retry = evaluate(
                            "document.querySelector('#resolution-result '"
                            "+ '[data-resolver-retry]').href"
                        )
                        self.assertEqual(
                            parse_qs(urlparse(retry).query)["did"], [INPUTS[name]]
                        )
                        browser("focus", "#resolution-result [data-resolver-retry]")
                        browser("press", "Enter")
                        browser("wait", "--text", expected)
                        self.assertEqual(
                            evaluate("document.querySelector('#did').value"),
                            INPUTS[name],
                        )
                    if name == "deactivated":
                        self.assertIn(
                            "No public keys",
                            evaluate(
                                "document.querySelector('#resolution-result').textContent"
                            ),
                        )
                        browser("focus", ".prism-operation > summary")
                        browser("press", "Enter")
                        self.assertTrue(
                            evaluate("document.querySelector('.prism-operation').open")
                        )
                    self.assertNotIn(
                        "secret-password",
                        evaluate("document.querySelector('main').textContent"),
                    )
        finally:
            browser("network", "unroute", "*")
            browser("close")
            browser("open")
            browser("network", "route", "**/htmx.org/**", "--abort")

    def test_without_scripts_delivery_failures_use_back_then_native_get_retry(
        self,
    ) -> None:
        browser("close")
        browser("open")
        browser("network", "route", "*", "--abort", "--resource-type", "script")
        try:
            for failure in (503, -1):
                with self.subTest(failure=failure):
                    browser("open", URL + "/resolver")
                    browser("fill", "#did", INPUTS["absent"])
                    self.server.failure = failure
                    browser("press", "Enter")
                    browser("wait", "--fn", "document.querySelector('#did') === null")
                    self.server.failure = 0
                    browser("back")
                    browser("wait", "--text", "Identity resolver")
                    self.assertEqual(
                        evaluate("document.querySelector('#did').value"),
                        INPUTS["absent"],
                    )
                    self.assertFalse(
                        evaluate(
                            "document.querySelector('button[type=submit]').disabled"
                        )
                    )
                    browser("focus", "#did")
                    browser("press", "Enter")
                    browser("wait", "--text", "No indexed state on this node")
                    self.assertEqual(
                        evaluate("document.querySelector('#did').value"),
                        INPUTS["absent"],
                    )
        finally:
            self.server.failure = 0
            browser("network", "unroute", "*")
            browser("close")
            browser("open")
            browser("network", "route", "**/htmx.org/**", "--abort")

    def test_enhanced_published_result_keeps_desktop_evidence_and_shared_copy(
        self,
    ) -> None:
        browser("set", "viewport", "1440", "900")
        self.server.outcome = "published"
        browser("fill", "#did", INPUTS["published"])
        browser("press", "Enter")
        browser("wait", "--text", OUTCOMES["published"])
        self.assertTrue(evaluate("document.querySelector('#ledger-evidence').open"))
        evaluate("""Object.defineProperty(navigator, 'clipboard', {configurable:true,
            value:{writeText: async value => { window.copiedOriginal = value; }}});""")
        browser("focus", "button[aria-label='Copy DID']")
        browser("press", "Space")
        browser("wait", "--text", "Copied to clipboard.")
        self.assertEqual(evaluate("window.copiedOriginal"), INPUTS["published"])
        self.assertEqual(evaluate("document.activeElement.ariaLabel"), "Copy DID")

    def test_keyboard_submit_button_recovers_focus_on_failure(self) -> None:
        self.server.failure = 503
        browser("fill", "#did", INPUTS["absent"])
        browser("press", "Tab")
        self.assertEqual(evaluate("document.activeElement.type"), "submit")
        browser("press", "Space")
        browser("wait", "--text", "Couldn’t load this resolution")
        self.assertFalse(
            evaluate("document.querySelector('button[type=submit]').disabled")
        )
        self.assertEqual(evaluate("document.activeElement.type"), "submit")
        self.server.failure = 0
        browser("press", "Space")
        browser("wait", "--text", OUTCOMES["absent"])
        self.assertEqual(evaluate("document.activeElement.type"), "submit")

    def test_bookmark_back_forward_resolves_the_url_instead_of_another_identity(
        self,
    ) -> None:
        for name in ("invalid", "absent"):
            self.server.outcome = name
            browser("fill", "#did", INPUTS[name])
            browser("press", "Enter")
            browser("wait", "--text", OUTCOMES[name])
        self.server.outcome = "invalid"
        browser("back")
        browser("wait", "--text", OUTCOMES["invalid"])
        self.assertEqual(
            evaluate("document.querySelector('#did').value"), INPUTS["invalid"]
        )
        self.server.outcome = "absent"
        browser("forward")
        browser("wait", "--text", OUTCOMES["absent"])
        self.assertEqual(
            evaluate("document.querySelector('#did').value"), INPUTS["absent"]
        )
        self.assertFalse(
            evaluate("document.querySelector('button[type=submit]').disabled")
        )

    def test_outcomes_expanded_bounds_accessibility_and_local_typography(self) -> None:
        for name in OUTCOMES:
            self.server.outcome = name
            browser("open", URL + "/resolver?" + urlencode({"did": INPUTS[name]}))
            evaluate(
                "document.querySelectorAll('main details').forEach(e => e.open = true)"
            )
            for width in (320, 390, 600, 800, 801, 1024, 1280, 1440):
                with self.subTest(outcome=name, width=width):
                    browser("set", "viewport", str(width), "900")
                    self.assertTrue(
                        evaluate("document.documentElement.scrollWidth <= innerWidth")
                    )
                    self.assertTrue(
                        evaluate("""Array.from(document.querySelectorAll(
                        '.prism-outcome a, .prism-outcome summary, '
                        + '#did, .prism-resolve-form button'))
                        .filter(e => e.checkVisibility())
                        .every(e => e.getBoundingClientRect().height >= 48)""")
                    )
                    if width in (390, 1440):
                        audit = browser("a11y", "--selector", "#resolution-result")
                        self.assertEqual(audit["counts"]["violations"], 0, audit)
                        if os.environ.get("PRISM_SCREENSHOT_DIR"):
                            destination = Path(os.environ["PRISM_SCREENSHOT_DIR"])
                            destination.mkdir(parents=True, exist_ok=True)
                            browser(
                                "screenshot",
                                str(destination / f"{name}-{width}.png"),
                                "--full",
                            )
            browser("wait", "--fn", "document.fonts.status === 'loaded'")
            self.assertTrue(evaluate("document.fonts.check('400 16px Poppins')"))

    def test_every_outcome_transitions_with_js_and_announces_real_state(self) -> None:
        for name, expected in OUTCOMES.items():
            with self.subTest(outcome=name):
                self.server.outcome = name
                browser("fill", "#did", INPUTS[name])
                browser("press", "Enter")
                browser(
                    "wait",
                    "--fn",
                    "!document.querySelector('button[type=submit]').disabled",
                )
                self.assertIn(
                    expected,
                    evaluate(
                        "document.querySelector('#resolution-result').textContent"
                    ),
                )
                self.assertIn(
                    expected,
                    evaluate(
                        "document.querySelector('#resolution-status').textContent"
                    ),
                )
                self.assertEqual(
                    evaluate("document.querySelector('#did').value"), INPUTS[name]
                )
                self.assertEqual(evaluate("document.activeElement.id"), "did")
                self.assertFalse(
                    evaluate(
                        "document.querySelector('#resolution-pending').checkVisibility()"
                    )
                )
                self.assertFalse(
                    evaluate(
                        "document.querySelector('#resolution-recovery').checkVisibility()"
                    )
                )
                self.assertNotIn(
                    "secret-password",
                    evaluate("document.querySelector('main').innerHTML"),
                )
                self.assertEqual(
                    parse_qs(urlparse(browser("get", "url")["url"]).query)["did"],
                    [INPUTS[name]],
                )
                if name in ("published", "deactivated", "unusable"):
                    self.assertEqual(
                        evaluate(
                            "document.querySelectorAll('.prism-operation').length"
                        ),
                        1,
                    )
                else:
                    self.assertEqual(
                        evaluate(
                            "document.querySelectorAll('.prism-operation').length"
                        ),
                        0,
                    )

    def test_bfcache_restore_recovers_a_request_interrupted_by_navigation(self) -> None:
        self.server.delayed = True
        browser("fill", "#did", INPUTS["absent"])
        browser("press", "Enter")
        try:
            self.assertTrue(self.server.started.wait(5))
            evaluate(
                "window.dispatchEvent(new PageTransitionEvent('pageshow',"
                " {persisted:true}))"
            )
            self.assertFalse(
                evaluate("document.querySelector('button[type=submit]').disabled")
            )
            self.assertFalse(
                evaluate(
                    "document.querySelector('#resolution-pending').checkVisibility()"
                )
            )
            self.assertFalse(evaluate("document.querySelector('#did').readOnly"))
            self.assertEqual(
                evaluate("document.querySelector('#did').value"), INPUTS["absent"]
            )
        finally:
            self.server.hold.set()
        self.server.delayed = False
        browser("press", "Enter")
        browser("wait", "--text", "No indexed state on this node")
        self.assertFalse(
            evaluate("document.querySelector('button[type=submit]').disabled")
        )

    def test_network_failure_can_retry_without_losing_value_or_fabricating_history(
        self,
    ) -> None:
        browser("fill", "#did", INPUTS["local"])
        browser("network", "route", "**/resolver?*", "--abort")
        try:
            browser("press", "Enter")
            browser("wait", "--text", "Couldn’t load this resolution")
            self.assertEqual(evaluate("document.activeElement.id"), "did")
            self.assertEqual(
                evaluate("document.querySelector('#did').value"), INPUTS["local"]
            )
            self.assertFalse(
                evaluate("document.querySelector('button[type=submit]').disabled")
            )
            self.assertEqual(
                evaluate("document.querySelectorAll('.prism-operation').length"), 0
            )
            self.assertFalse(
                evaluate(
                    "document.querySelector('#resolution-previous').checkVisibility()"
                )
            )
            self.assertFalse(
                evaluate(
                    "document.querySelector('#resolution-pending').checkVisibility()"
                )
            )
        finally:
            browser("network", "unroute", "**/resolver?*")
        self.server.outcome = "local"
        browser("focus", "#resolution-retry")
        browser("press", "Enter")
        browser("wait", "--text", "Locally derived")
        self.assertEqual(
            evaluate("document.querySelector('#did').value"), INPUTS["local"]
        )
        self.assertFalse(
            evaluate("document.querySelector('#resolution-recovery').checkVisibility()")
        )
        self.assertEqual(
            evaluate("document.querySelectorAll('.prism-operation').length"), 0
        )

    def test_http_failure_retains_prior_history_and_exact_retry_get_then_recovers(
        self,
    ) -> None:
        self.server.outcome = "published"
        browser("open", URL + "/resolver?" + urlencode({"did": INPUTS["published"]}))
        browser("focus", ".prism-operation > summary")
        browser("press", "Enter")
        original = evaluate("document.querySelector('#operation-history').textContent")
        self.server.failure = 503
        browser("fill", "#did", INPUTS["invalid"])
        browser("press", "Enter")
        browser("wait", "--text", "Couldn’t load this resolution")
        self.assertFalse(
            evaluate("document.querySelector('button[type=submit]').disabled")
        )
        self.assertFalse(evaluate("document.querySelector('#did').readOnly"))
        self.assertFalse(
            evaluate("document.querySelector('#resolution-pending').checkVisibility()")
        )
        self.assertEqual(evaluate("document.activeElement.id"), "did")
        self.assertEqual(
            evaluate("document.querySelector('#did').value"), INPUTS["invalid"]
        )
        self.assertEqual(
            evaluate("document.querySelector('#operation-history').textContent"),
            original,
        )
        self.assertTrue(evaluate("document.querySelector('.prism-operation').open"))
        self.assertIn(
            "Previous result",
            evaluate("document.querySelector('#resolution-recovery').textContent"),
        )
        retry = evaluate("document.querySelector('#resolution-retry').href")
        self.assertEqual(parse_qs(urlparse(retry).query)["did"], [INPUTS["invalid"]])
        self.assertNotIn(
            "private-server-detail",
            evaluate("document.querySelector('main').textContent"),
        )
        self.server.failure = 0
        self.server.outcome = "invalid"
        browser("focus", "#resolution-retry")
        browser("press", "Enter")
        browser("wait", "--text", "Check the DID format")
        self.assertFalse(
            evaluate("document.querySelector('#resolution-recovery').checkVisibility()")
        )
        self.assertFalse(
            evaluate("document.querySelector('button[type=submit]').disabled")
        )
        self.assertTrue(
            evaluate(
                "document.querySelector('#did').getAttribute('aria-invalid') === 'true'"
            )
        )
        self.assertEqual(
            evaluate("document.querySelector('#did').value"), INPUTS["invalid"]
        )
        self.assertIn(
            "Check the DID format",
            evaluate("document.querySelector('#resolution-status').textContent"),
        )

    def test_pending_is_real_repeat_protected_and_preserves_keyboard_focus(
        self,
    ) -> None:
        browser("focus", "#did")
        browser("press", "Enter")
        self.assertFalse(
            evaluate("document.querySelector('button[type=submit]').disabled")
        )
        self.server.delayed = True
        browser("fill", "#did", INPUTS["absent"])
        browser("press", "Enter")
        try:
            self.assertTrue(self.server.started.wait(5), "no actual GET request")
            self.assertTrue(
                evaluate("document.querySelector('button[type=submit]').disabled")
            )
            self.assertTrue(
                evaluate(
                    "document.querySelector('#resolution-pending')?.checkVisibility()"
                )
            )
            self.assertEqual(evaluate("document.activeElement.id"), "did")
            self.assertEqual(
                evaluate("document.querySelector('#did').value"), INPUTS["absent"]
            )
            self.assertIn(
                "Resolving",
                evaluate("document.querySelector('#resolution-status').textContent"),
            )
            browser("press", "Enter")
            self.assertEqual(len(self.server.requests), 2)  # entry + one pending GET
        finally:
            self.server.hold.set()
        browser("wait", "--text", "No indexed state on this node")
        self.assertFalse(
            evaluate("document.querySelector('button[type=submit]').disabled")
        )
        self.assertFalse(
            evaluate("document.querySelector('#resolution-pending').checkVisibility()")
        )
        self.assertEqual(evaluate("document.activeElement.id"), "did")
        self.assertEqual(
            parse_qs(urlparse(browser("get", "url")["url"]).query)["did"],
            [INPUTS["absent"]],
        )


if __name__ == "__main__":
    unittest.main()
