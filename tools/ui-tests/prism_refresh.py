"""Deterministic Explorer clock/network checks at exported real HTTP/HTML seams."""

import json
import os
import socket
import subprocess
import threading
import unittest
from contextlib import suppress
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any
from urllib.parse import parse_qs, urlparse

from prism_shell import browser as shell_browser
from prism_shell import evaluate


def browser(*args: str) -> dict[str, Any]:
    try:
        return shell_browser(*args)
    except subprocess.CalledProcessError as error:
        raise AssertionError(error.stdout + error.stderr) from error


ROOT = Path(__file__).resolve().parents[2]
FIXTURES = Path(os.environ["PRISM_FIXTURE_DIR"])
CURSORS = Path(os.environ["PRISM_CURSOR_FIXTURES"])
MODES = {"cursor": "supplied", "list": "success"}
HUNG_RELEASE = threading.Event()
CLOCK = """
(() => {
  window.$ = document.querySelector.bind(document);
  const RealDate = Date;
  let now = 1800000000000, next = 0;
  const timers = new Map();
  window.Date = class extends RealDate {
    constructor(...args) { super(...(args.length ? args : [now])); }
    static now() { return now; }
  };
  window.setTimeout = (fn, delay = 0) => {
    const id = ++next; timers.set(id, {fn, at: now + delay}); return id;
  };
  window.clearTimeout = id => timers.delete(id);
  window.clock = {advance(ms) {
    const end = now + ms;
    for (let n = 0; n < 10000; n++) {
      const due = [...timers].filter(([, t]) => t.at <= end)
        .sort((a, b) => a[1].at - b[1].at)[0];
      if (!due) break;
      now = due[1].at; timers.delete(due[0]); due[1].fn();
    }
    now = end;
  }};
  const nativeFetch = fetch;
  window.net = {calls: [], hold: {}, releases: {}, done: 0};
  window.fetch = async (url, options) => {
    const resource = String(url).includes('dlt-cursor') ? 'cursor' : 'list';
    net.calls.push({resource, url: String(url), at: Date.now(),
      signal: options?.signal});
    // A deliberately uncooperative network exercises stale-response guards too.
    const held = net.hold[resource];
    let response;
    try {
      response = await nativeFetch(url,
        held ? {...options, signal: undefined} : options);
      if (held) await new Promise(resolve => {net.releases[resource] = resolve});
      return response;
    } finally { net.done++; }
  };
})();
"""


class Handler(SimpleHTTPRequestHandler):
    def do_GET(self) -> None:
        url = urlparse(self.path)
        if url.path.startswith("/assets/"):
            self.path = "/bin/neoprism-node/assets/" + url.path.removeprefix("/assets/")
            return super().do_GET()
        if url.path == "/explorer/dlt-cursor":
            resource = "cursor"
        elif url.path == "/explorer/did-list":
            resource = "list"
        elif url.path == "/explorer":
            page = parse_qs(url.query).get("page", ["3"])[0]
            name = "partial" if page == "12" else "refresh"
            payload = (FIXTURES / f"{name}.html").read_text()
            if parse_qs(url.query).get("initial") == ["failure"]:
                payload = (FIXTURES / "failure.html").read_text()
            payload = payload.replace("<head>", "<head><script>" + CLOCK + "</script>")
            self.reply(200, payload.encode())
            return
        else:
            self.send_error(404)
            return
        mode = MODES[resource]
        if mode == "network":
            self.connection.shutdown(socket.SHUT_RDWR)
            self.connection.close()
            return
        if mode == "hung":
            HUNG_RELEASE.wait(timeout=20)
            mode = "supplied"
        if mode == "http":
            self.reply(503, b"private database error must not reach the page")
            return
        if resource == "cursor":
            payload = (CURSORS / f"{mode}.fragment").read_bytes()
        else:
            page = parse_qs(url.query).get("page", ["3"])[0]
            suffix = "" if mode == "success" else f"-{mode}"
            payload = (FIXTURES / f"page{page}{suffix}.fragment").read_bytes()
        self.reply(200, payload)

    def reply(self, status: int, payload: bytes) -> None:
        self.send_response(status)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.end_headers()
        # Native navigation may cancel a response while the server is writing it.
        with suppress(BrokenPipeError, ConnectionResetError):
            self.wfile.write(payload)

    def log_message(self, format: str, *args: Any) -> None:
        pass

    def __init__(self, *args: object, **kwargs: object) -> None:
        super().__init__(*args, directory=str(ROOT), **kwargs)


class RefreshBrowser(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.server = ThreadingHTTPServer(("127.0.0.1", 18880), Handler)
        cls.url = "http://127.0.0.1:18880"
        threading.Thread(target=cls.server.serve_forever, daemon=True).start()

    @classmethod
    def tearDownClass(cls) -> None:
        browser("close")
        HUNG_RELEASE.set()
        cls.server.shutdown()
        cls.server.server_close()

    def setUp(self) -> None:
        HUNG_RELEASE.clear()
        MODES.update(cursor="supplied", list="success")
        browser("network", "route", "*unpkg.com*", "--abort")
        browser("set", "viewport", "390", "898")
        browser("open", self.url + "/explorer?page=3")
        self.enhanced()

    def enhanced(self) -> None:
        browser(
            "wait",
            "--fn",
            "$('[data-refresh-action]')?.hasAttribute('aria-disabled')",
        )

    def advance(self, ms: int, done: int | None = None) -> None:
        evaluate(f"clock.advance({ms})")
        if done is not None:
            browser("wait", "--fn", f"net.done >= {done}")
            # Wait beyond body consumption and DOM work, not an arbitrary timeout.
            browser("wait", "--fn", "!$('[aria-busy=true]')")

    def visibility(self, hidden: bool) -> None:
        evaluate(
            """Object.defineProperty(document, 'hidden', {configurable: true,
          value: """
            + json.dumps(hidden)
            + """});
          document.dispatchEvent(new Event('visibilitychange'));"""
        )

    def calls(self, resource: str) -> list[dict[str, Any]]:
        return evaluate(
            f"net.calls.filter(call => call.resource === {json.dumps(resource)})"
        )

    def test_independent_cadences_keep_page_without_phantom_initial_pending(
        self,
    ) -> None:
        self.assertEqual(evaluate("net.calls.length"), 0)
        self.assertFalse(evaluate("!!$('[aria-busy=true]')"))
        self.advance(1999)
        self.assertEqual(evaluate("net.calls.length"), 0)
        self.advance(1, 1)
        self.assertEqual(len(self.calls("cursor")), 1)
        self.assertEqual(len(self.calls("list")), 0)
        self.advance(2000, 2)
        self.advance(1000, 3)
        self.assertEqual(len(self.calls("cursor")), 2)
        self.assertEqual(len(self.calls("list")), 1)
        self.assertTrue(
            self.calls("list")[0]["url"].endswith("/explorer/did-list?page=3")
        )
        self.assertIn("Page 3 of 12", evaluate("$('#did-list').textContent"))
        self.advance(1000, 4)
        self.assertEqual(len(self.calls("cursor")), 3)
        self.assertEqual(len(self.calls("list")), 1)

    def test_hidden_pause_and_immediate_return_restart_both_cadences(self) -> None:
        self.advance(2000, 1)
        self.visibility(True)
        self.advance(60000)
        self.assertEqual(len(self.calls("cursor")), 1)
        self.assertEqual(len(self.calls("list")), 0)
        self.visibility(False)
        browser(
            "wait",
            "--fn",
            "net.done === 3 && !$('[aria-busy=true]')",
        )
        self.assertEqual(len(self.calls("cursor")), 2)
        self.assertEqual(len(self.calls("list")), 1)
        self.advance(2000, 4)
        self.assertEqual(len(self.calls("cursor")), 3)
        self.advance(3000, 6)
        self.assertEqual(len(self.calls("cursor")), 4)
        self.assertEqual(len(self.calls("list")), 2)

    def test_repeated_failures_retain_independent_evidence_then_recover(
        self,
    ) -> None:
        before_cursor = evaluate(
            "$('#dlt-cursor-card [data-refresh-content]').innerHTML"
        )
        before_list = evaluate("$('#did-list [data-refresh-content]').innerHTML")
        MODES.update(cursor="http", list="network")
        self.advance(2000, 1)
        self.assertIn(
            "Couldn’t refresh · showing last received cursor",
            evaluate("$('[data-cursor-status]').textContent"),
        )
        self.assertIn(
            "2 seconds ago",
            evaluate("$('[data-cursor-receipt]').textContent"),
        )
        self.advance(2000, 2)
        self.advance(1000, 3)
        self.assertEqual(len(self.calls("cursor")), 2)
        self.assertIn(
            "Couldn’t refresh",
            evaluate("$('#did-list [data-refresh-status]').textContent"),
        )
        self.assertIn(
            "5 seconds ago",
            evaluate("$('#did-list [data-refresh-receipt]').textContent"),
        )
        self.assertIn(
            "Retries every 2 seconds",
            evaluate("$('[data-cursor-receipt]').textContent"),
        )
        self.assertIn(
            "Retries every 5 seconds",
            evaluate("$('#did-list [data-refresh-receipt]').textContent"),
        )
        self.assertEqual(
            before_cursor,
            evaluate("$('#dlt-cursor-card [data-refresh-content]').innerHTML"),
        )
        self.assertEqual(
            before_list,
            evaluate("$('#did-list [data-refresh-content]').innerHTML"),
        )
        self.assertIn(
            "Retry",
            evaluate("$('#did-list [data-refresh-action]').textContent"),
        )
        self.assertNotIn("private database", evaluate("document.body.textContent"))
        MODES["cursor"] = "waiting"
        self.advance(1000, 4)
        self.assertEqual(
            evaluate("$('[data-cursor-status]').textContent"),
            "Waiting for sync information",
        )
        self.assertIsNone(evaluate("$('#cursor-technical')"))
        self.assertIn(
            "No cursor was supplied",
            evaluate("$('[data-cursor-receipt]').textContent"),
        )
        self.assertIn(
            "Couldn’t refresh",
            evaluate("$('#did-list [data-refresh-status]').textContent"),
        )
        self.advance(2000, 5)
        self.advance(2000, 7)
        self.assertEqual(len(self.calls("list")), 2)
        self.assertIn(
            "10 seconds ago",
            evaluate("$('#did-list [data-refresh-receipt]').textContent"),
        )
        MODES["list"] = "success"
        self.advance(2000, 8)
        self.advance(2000, 9)
        self.advance(1000, 10)
        self.assertEqual(len(self.calls("list")), 3)
        self.assertNotIn(
            "Couldn’t",
            evaluate("$('#did-list [data-refresh-status]').textContent"),
        )
        self.assertIn(
            "0 seconds ago",
            evaluate("$('#did-list [data-refresh-receipt]').textContent"),
        )
        self.assertEqual(
            evaluate("document.querySelectorAll('.prism-did-row').length"), 100
        )

    def test_changed_cursor_preserves_interaction_then_applies_latest(
        self,
    ) -> None:
        browser("focus", "#cursor-technical summary")
        browser("press", "Enter")
        browser("focus", '[aria-label="Copy block hash"]')
        evaluate("""window.kept = {
          root: $('#dlt-cursor-card'),
          details: $('#cursor-technical'),
          button: document.activeElement
        };
        const code = $('.prism-sync-raw div:nth-child(2) code');
        const range = document.createRange(); range.selectNodeContents(code);
        getSelection().removeAllRanges(); getSelection().addRange(range);""")
        MODES["cursor"] = "long"
        self.advance(2000, 1)
        self.assertTrue(
            evaluate(
                "kept.root === $('#dlt-cursor-card') && kept.details.ope"
                "n && document.activeElement === kept.button"
            )
        )
        self.assertEqual(evaluate("getSelection().toString()"), "ab" * 32)
        self.assertEqual(
            evaluate("$('.prism-sync-raw code').textContent"),
            "1042816",
        )
        self.assertTrue(
            evaluate("$('#dlt-cursor-card [data-refresh-deferred]').checkVisibility()")
        )
        MODES["cursor"] = "missing-timestamp"
        self.advance(2000, 2)
        self.assertEqual(evaluate("getSelection().toString()"), "ab" * 32)
        evaluate("document.activeElement.blur(); getSelection().removeAllRanges();")
        browser(
            "wait",
            "--fn",
            "$('.prism-sync-summary > div:last-child dd').textConten"
            "t.includes('Not provided')",
        )
        self.assertTrue(
            evaluate("$('#cursor-technical') === kept.details && kept.details.open")
        )
        self.assertEqual(
            evaluate("$('.prism-sync-raw code').textContent"),
            "1042816",
        )
        self.assertFalse(
            evaluate("$('#dlt-cursor-card [data-refresh-deferred]').checkVisibility()")
        )
        self.assertEqual(
            evaluate(
                "document.querySelectorAll('[aria-label=\"Copy block tim"
                "estamp\"]').length"
            ),
            0,
        )

    def test_inflight_visibility_ordering_discards_old_snapshot_and_never_overlaps(
        self,
    ) -> None:
        MODES["cursor"] = "long"
        evaluate("""net.hold.cursor = true; window.seenSlots = [];
          new MutationObserver(() => seenSlots.push(
            $('.prism-sync-slot')?.textContent || 'missing'))
            .observe($('#dlt-cursor-card'), {
              subtree:true, childList:true, characterData:true});""")
        self.advance(2000)
        browser("wait", "--fn", "!!net.releases.cursor")
        self.assertEqual(evaluate("$('#dlt-cursor-card').ariaBusy"), "true")
        self.assertEqual(
            evaluate("$('[data-cursor-status]').textContent"),
            "Checking for sync information…",
        )
        self.visibility(True)
        MODES["cursor"] = "waiting"
        self.visibility(False)
        self.advance(10000)
        browser("wait", "--fn", "net.done >= 1")
        self.assertEqual(len(self.calls("cursor")), 1)
        self.assertGreaterEqual(len(self.calls("list")), 1)
        evaluate("net.hold.cursor = false; net.releases.cursor()")
        browser(
            "wait",
            "--fn",
            "$('[data-cursor-status]').textContent === 'Waiting for sync information'",
        )
        self.assertEqual(len(self.calls("cursor")), 2)
        self.assertFalse(evaluate("seenSlots.includes('18,446,744,073,709,551,615')"))
        self.assertIn(
            "Distance to chain tip unknown",
            evaluate("$('#dlt-cursor-card').textContent"),
        )

    def test_hung_request_times_out_without_overlap_and_normal_cadence_recovers(
        self,
    ) -> None:
        MODES["cursor"] = "hung"
        self.advance(2000)
        browser("wait", "--fn", "net.calls.length === 1")
        self.advance(14000)
        self.assertEqual(len(self.calls("cursor")), 1)
        self.assertEqual(evaluate("$('#dlt-cursor-card').ariaBusy"), "true")
        self.advance(1000)
        self.assertTrue(evaluate("net.calls[0].signal?.aborted === true"))
        browser(
            "wait",
            "--fn",
            "$('[data-cursor-status]').textContent.includes('Couldn’t refresh')",
        )
        self.assertEqual(len(self.calls("cursor")), 1)
        HUNG_RELEASE.set()
        MODES["cursor"] = "supplied"
        self.advance(1000)
        browser(
            "wait",
            "--fn",
            "$('[data-cursor-status]').textContent === 'Cursor received'",
        )
        self.assertEqual(len(self.calls("cursor")), 2)
        self.assertEqual(
            evaluate("$('.prism-sync-slot').textContent"),
            "1,042,816",
        )

    def test_initial_failure_and_independent_real_retry(
        self,
    ) -> None:
        browser("open", self.url + "/explorer?page=3&initial=failure")
        self.enhanced()
        self.assertEqual(evaluate("net.calls.length"), 0)
        self.assertFalse(evaluate("!!$('[aria-busy=true]')"))
        self.assertIn(
            "No successful response",
            evaluate("$('#did-list [data-refresh-receipt]').textContent"),
        )
        MODES["list"] = "http"
        evaluate("net.hold.list = true")
        browser("click", "#did-list [data-refresh-action]")
        browser("wait", "--fn", "!!net.releases.list")
        self.assertEqual(evaluate("$('#did-list').ariaBusy"), "true")
        self.assertIsNone(evaluate("$('#dlt-cursor-card').ariaBusy"))
        self.assertEqual(
            evaluate("document.querySelectorAll('.prism-did-row').length"), 0
        )
        evaluate("$('#did-list [data-refresh-action]').click()")
        self.advance(5000)
        self.assertEqual(len(self.calls("list")), 1)
        evaluate("net.hold.list = false; net.releases.list()")
        browser(
            "wait",
            "--fn",
            "$('#did-list [data-refresh-status]').textContent.includ"
            "es('Couldn’t load')",
        )
        self.assertIn(
            "No successful response",
            evaluate("$('#did-list [data-refresh-receipt]').textContent"),
        )
        MODES["list"] = "success"
        browser("click", "#did-list [data-refresh-action]")
        browser(
            "wait", "--fn", "document.querySelectorAll('.prism-did-row').length === 100"
        )
        self.assertIn("Page 3 of 12", evaluate("$('#did-list').textContent"))
        self.assertIn(
            "0 seconds ago",
            evaluate("$('#did-list [data-refresh-receipt]').textContent"),
        )
        self.assertEqual(len(self.calls("list")), 2)

    def test_list_preserves_copy_fallback_selection_and_keyed_rows(
        self,
    ) -> None:
        browser("focus", ".prism-did-row:first-child button")
        evaluate("""window.keptRow = $('.prism-did-row');
          window.keptCopy = document.activeElement;
          Object.defineProperty(navigator, 'clipboard', {configurable: true,
            value: {writeText: async () => {throw new Error('denied')}}});""")
        browser("press", "Enter")
        browser("wait", "--text", "Couldn’t copy.")
        evaluate("""const field = $('.prism-copy-fallback textarea');
          field.focus(); field.select(); window.manual = field;""")
        MODES["list"] = "updated"
        self.advance(5000, 2)
        self.assertTrue(
            evaluate(
                "document.activeElement === manual && manual.isConnected"
                " && manual.selectionEnd === manual.value.length"
            )
        )
        self.assertEqual(evaluate("manual.value"), "did:prism:" + f"{200:064x}")
        self.assertEqual(evaluate("$('.prism-did-row') === keptRow"), True)
        self.assertTrue(
            evaluate("$('#did-list [data-refresh-deferred]').checkVisibility()")
        )
        MODES["list"] = "http"
        self.advance(5000, 4)
        self.assertTrue(
            evaluate(
                "document.activeElement === manual && manual.value === k"
                "eptCopy.dataset.copyValue"
            )
        )
        self.assertIn(
            "Couldn’t refresh",
            evaluate("$('#did-list [data-refresh-status]').textContent"),
        )
        evaluate("manual.blur(); getSelection().removeAllRanges();")
        browser(
            "wait",
            "--fn",
            "$('.prism-did-row button').dataset.copyValue.endsWith('00c7')",
        )
        self.assertTrue(
            evaluate("keptRow === document.querySelectorAll('.prism-did-row')[1]")
        )
        self.assertEqual(
            evaluate("document.querySelectorAll('.prism-did-row').length"), 100
        )
        self.assertIn("Page 3 of 12", evaluate("$('#did-list').textContent"))
        self.assertIn(
            "Couldn’t refresh",
            evaluate("$('#did-list [data-refresh-status]').textContent"),
        )

    def test_successful_empty_and_out_of_range_responses_are_not_request_failures(
        self,
    ) -> None:
        MODES["list"] = "empty"
        self.advance(5000, 2)
        self.assertIn(
            "No indexed DIDs yet",
            evaluate("$('#did-list').textContent"),
        )
        self.assertEqual(
            evaluate("document.querySelectorAll('.prism-did-row').length"), 0
        )
        self.assertNotIn(
            "Couldn’t",
            evaluate("$('#did-list [data-refresh-status]').textContent"),
        )
        MODES["list"] = "beyond"
        self.advance(5000, 4)
        self.assertIn(
            "Page 3 is beyond the last page",
            evaluate("$('#did-list').textContent"),
        )
        self.assertNotIn(
            "No indexed DIDs yet",
            evaluate("$('#did-list').textContent"),
        )
        self.assertTrue(self.calls("list")[-1]["url"].endswith("page=3"))
        MODES["list"] = "success"
        self.advance(5000, 6)
        self.assertEqual(
            evaluate("document.querySelectorAll('.prism-did-row').length"), 100
        )
        self.assertNotIn(
            "Couldn’t",
            evaluate("$('#did-list [data-refresh-status]').textContent"),
        )

    def test_native_navigation_wins_over_pending_refresh_and_keeps_new_page_polling(
        self,
    ) -> None:
        evaluate("net.hold.list = true")
        self.advance(5000)
        browser("wait", "--fn", "!!net.releases.list")
        browser(
            "scrollintoview",
            ".prism-pagination:first-child .prism-page-mobile a[href"
            '="/explorer?page=12"]',
        )
        browser(
            "click",
            ".prism-pagination:first-child .prism-page-mobile a[href"
            '="/explorer?page=12"]',
        )
        browser("wait", "--url", "**/explorer?page=12")
        self.enhanced()
        self.assertEqual(
            evaluate("document.querySelectorAll('.prism-did-row').length"), 3
        )
        self.advance(5000, 2)
        self.assertTrue(self.calls("list")[-1]["url"].endswith("page=12"))
        self.assertIn("Page 12 of 12", evaluate("$('#did-list').textContent"))
        self.assertEqual(
            evaluate("document.querySelectorAll('.prism-did-row').length"), 3
        )

    def test_no_script_pagination_and_deliberate_refresh_retry_remain_native_gets(
        self,
    ) -> None:
        browser("close")
        browser("open")
        browser("set", "viewport", "390", "898")
        browser("network", "route", "*", "--abort", "--resource-type", "script")
        try:
            browser("open", self.url + "/explorer?page=3")
            self.assertIsNone(evaluate("$('.prism-copy-status')"))
            browser("click", "#dlt-cursor-card [data-refresh-action]")
            browser("wait", "--load", "load")
            self.assertTrue(browser("get", "url")["url"].endswith("/explorer?page=3"))
            browser("click", "#did-list [data-refresh-action]")
            browser("wait", "--load", "load")
            self.assertTrue(browser("get", "url")["url"].endswith("/explorer?page=3"))
            browser(
                "scrollintoview",
                ".prism-pagination:first-child .prism-page-mobile a[href"
                '="/explorer?page=12"]',
            )
            browser(
                "click",
                ".prism-pagination:first-child .prism-page-mobile a[href"
                '="/explorer?page=12"]',
            )
            browser("wait", "--url", "**/explorer?page=12")
            self.assertEqual(
                evaluate("document.querySelectorAll('.prism-did-row').length"), 3
            )
            browser("open", self.url + "/explorer?page=3&initial=failure")
            browser("click", "#did-list [data-refresh-action]")
            browser("wait", "--url", "**/explorer?page=3")
            self.assertEqual(
                evaluate("document.querySelectorAll('.prism-did-row').length"), 100
            )
        finally:
            browser("network", "unroute", "*")

    def test_selecting_receipt_text_is_not_destroyed_by_age_or_response_updates(
        self,
    ) -> None:
        evaluate("""const node = $('[data-cursor-receipt]');
          const range = document.createRange(); range.selectNodeContents(node);
          getSelection().removeAllRanges(); getSelection().addRange(range);
          window.selectedReceipt = getSelection().toString();""")
        self.advance(1000)
        self.assertEqual(
            evaluate("getSelection().toString()"), evaluate("selectedReceipt")
        )
        self.advance(1000, 1)
        self.assertEqual(
            evaluate("getSelection().toString()"), evaluate("selectedReceipt")
        )
        self.assertTrue(
            evaluate("$('#dlt-cursor-card [data-refresh-deferred]').checkVisibility()")
        )
        evaluate("getSelection().removeAllRanges()")
        browser(
            "wait",
            "--fn",
            "$('[data-cursor-receipt]').textContent !== selectedReceipt",
        )
        self.assertFalse(
            evaluate("$('#dlt-cursor-card [data-refresh-deferred]').checkVisibility()")
        )

    def test_multiple_visibility_changes_consume_only_one_queued_return_refresh(
        self,
    ) -> None:
        evaluate("net.hold.cursor = true")
        self.advance(2000)
        browser("wait", "--fn", "!!net.releases.cursor")
        self.visibility(True)
        self.visibility(False)
        browser("wait", "--fn", "net.done >= 1")
        self.visibility(True)
        evaluate("net.hold.cursor = false; net.releases.cursor()")
        browser("wait", "--fn", "!$('#dlt-cursor-card').hasAttribute('aria-busy')")
        self.visibility(False)
        browser("wait", "--fn", "net.done >= 4 && !$('[aria-busy=true]')")
        self.assertEqual(len(self.calls("cursor")), 2)
        self.assertEqual(len(self.calls("list")), 2)

    def test_deferred_first_success_then_failure_is_truthful(self) -> None:
        browser("open", self.url + "/explorer?page=3&initial=failure")
        self.enhanced()
        browser("focus", "#did-list [data-refresh-content] a")
        evaluate("""const range = document.createRange();
          range.selectNodeContents($('.prism-sync-distance'));
          getSelection().removeAllRanges(); getSelection().addRange(range);""")
        self.advance(5000, 2)
        self.assertEqual(
            evaluate("document.querySelectorAll('.prism-did-row').length"), 0
        )
        self.assertIsNone(evaluate("$('#cursor-technical')"))
        MODES.update(cursor="http", list="http")
        self.advance(5000, 4)
        self.assertIn(
            "update waiting",
            evaluate("$('#did-list [data-refresh-status]').textContent"),
        )
        self.assertIn(
            "update waiting", evaluate("$('[data-cursor-status]').textContent")
        )
        self.assertNotIn(
            "no cursor received", evaluate("$('[data-cursor-status]').textContent")
        )
        evaluate("document.activeElement.blur(); getSelection().removeAllRanges();")
        browser(
            "wait",
            "--fn",
            "document.querySelectorAll('.prism-did-row').length === "
            "100 && !!$('#cursor-technical')",
        )
        self.assertIn(
            "Couldn’t refresh", evaluate("$('[data-cursor-status]').textContent")
        )
        self.assertIn(
            "Couldn’t refresh",
            evaluate("$('#did-list [data-refresh-status]').textContent"),
        )

    def test_failed_feedback_has_accessible_retry_and_narrow_bounds(self) -> None:
        MODES.update(cursor="http", list="http")
        self.advance(5000, 2)
        for width in (320, 390, 600, 800, 1024, 1440):
            with self.subTest(width=width):
                browser("set", "viewport", str(width), "898")
                self.assertTrue(
                    evaluate("document.documentElement.scrollWidth <= innerWidth")
                )
                self.assertTrue(
                    evaluate("""[...document.querySelectorAll(
                  `[data-refresh-action], [data-refresh-status], [data-cursor-status],
                   [data-refresh-receipt]`)].every(node => {
                    const rect = node.getBoundingClientRect();
                    return rect.left >= 0 && rect.right <= innerWidth
                      && (!node.matches('a') || rect.height >= 48);
                  })""")
                )
        for selector in ("#dlt-cursor-card", "#did-list"):
            audit = browser("a11y", "--selector", selector)
            self.assertEqual(audit["counts"]["violations"], 0, audit)
        browser("focus", "#dlt-cursor-card [data-refresh-action]")
        self.assertEqual(
            evaluate("document.activeElement.textContent"), "Retry sync information"
        )
        MODES["cursor"] = "supplied"
        browser("press", "Enter")
        browser(
            "wait",
            "--fn",
            "$('[data-cursor-status]').textContent === 'Cursor received'",
        )
        self.assertTrue(
            evaluate("document.activeElement.matches('[data-refresh-action]')")
        )


if __name__ == "__main__":
    unittest.main()
