"""Browser contract for reusable copy enhancement; isolated synthetic values only."""

import json
import threading
import unittest
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

from clipboard_permissions import grant_clipboard_permissions
from prism_shell import browser, evaluate

ROOT = Path(__file__).resolve().parents[2]
VALUE = "did:prism:" + "ab" * 32 + ':original\n"<&'


class CopyContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        handler = partial(SimpleHTTPRequestHandler, directory=str(ROOT))
        cls.server = ThreadingHTTPServer(("127.0.0.1", 18778), handler)
        threading.Thread(target=cls.server.serve_forever, daemon=True).start()

    @classmethod
    def tearDownClass(cls) -> None:
        browser("close")
        cls.server.shutdown()
        cls.server.server_close()

    def setUp(self) -> None:
        browser("open", "http://127.0.0.1:18778/tools/ui-tests/copy-fixture.html")
        evaluate(f"""document.querySelector('#original').textContent =
          {json.dumps(VALUE)};
          document.querySelector('button').dataset.copyValue = {json.dumps(VALUE)}""")
        browser("focus", "button")

    def test_keyboard_copy_writes_complete_original_and_politely_confirms(self) -> None:
        evaluate("""window.copied = null;
          Object.defineProperty(navigator, 'clipboard', {configurable: true,
            value: {writeText: async value => {window.copied = value}}})""")
        browser("press", "Enter")
        browser("wait", "--fn", "window.copied !== null")
        self.assertEqual(evaluate("window.copied"), VALUE)
        self.assertEqual(evaluate("document.activeElement.tagName"), "BUTTON")
        self.assertIn(
            "Copied", evaluate("document.querySelector('[role=status]').textContent")
        )
        self.assertEqual(
            evaluate(
                "document.querySelector('[role=status]').getAttribute('aria-live')"
            ),
            "polite",
        )

    def test_denied_or_missing_clipboard_offers_adjacent_selectable_original(
        self,
    ) -> None:
        for clipboard in (
            "undefined",
            "{writeText: async () => {throw new Error('denied')}}",
        ):
            evaluate(
                f"Object.defineProperty(navigator, 'clipboard', "
                f"{{configurable: true, value: {clipboard}}})"
            )
            browser("press", "Enter")
            browser(
                "wait",
                "--fn",
                "document.querySelector('.prism-copy-fallback') !== null",
            )
            self.assertEqual(evaluate("document.activeElement.tagName"), "BUTTON")
            self.assertEqual(
                evaluate(
                    "document.querySelector('button').nextElementSibling.className"
                ),
                "prism-copy-fallback",
            )
            self.assertEqual(
                evaluate("document.querySelector('textarea').value"), VALUE
            )
            self.assertTrue(evaluate("document.querySelector('textarea').readOnly"))
            self.assertIn(
                "Couldn’t copy",
                evaluate("document.querySelector('[role=status]').textContent"),
            )
            browser("press", "Tab")
            self.assertEqual(evaluate("document.activeElement.tagName"), "TEXTAREA")
            browser("press", "Control+a")
            self.assertEqual(
                evaluate(
                    "document.activeElement.selectionEnd - "
                    "document.activeElement.selectionStart"
                ),
                len(VALUE),
            )
            browser("focus", "button")

    def test_delegation_handles_replaced_content_and_real_browser_clipboard(
        self,
    ) -> None:
        self.enterContext(
            grant_clipboard_permissions(browser, "http://127.0.0.1:18778")
        )
        evaluate("""document.querySelector('button').outerHTML =
          '<button type="button" class="prism-copy"' +
          ' data-copy-value="complete original" aria-label="Copy replacement">' +
          '<span>Copy replacement</span></button>'""")
        browser("click", "button span")
        browser("wait", "--text", "Copied to clipboard.")
        self.assertEqual(browser("clipboard", "read")["text"], "complete original")
        self.assertEqual(evaluate("document.activeElement.tagName"), "BUTTON")
        self.assertIsNone(evaluate("document.querySelector('textarea')"))

    def test_manual_fallback_wraps_in_constrained_flex_consumer(self) -> None:
        browser("set", "viewport", "320", "900")
        evaluate("""const row = document.createElement('div');
          row.style.cssText = 'display:flex;gap:12px;width:180px';
          const text = document.createElement('code');
          text.textContent = 'original value';
          text.style.cssText = 'flex:1;min-width:0;overflow-wrap:anywhere';
          document.body.append(row); row.append(text, document.querySelector('button'));
          Object.defineProperty(navigator, 'clipboard', {configurable:true,
            value:{writeText: async () => {throw new Error('denied')}}})""")
        browser("click", "button")
        browser("wait", "--fn", "document.querySelector('textarea') !== null")
        self.assertTrue(
            evaluate("""const field = document.querySelector('textarea');
          const rowRect = field.parentElement.parentElement.getBoundingClientRect();
          const rect = field.getBoundingClientRect();
          rect.width >= 140 && rect.right <= rowRect.right
            && rect.top >= document.querySelector('button')
              .getBoundingClientRect().bottom""")
        )

    def test_no_script_value_is_selectable_and_button_has_touch_target(self) -> None:
        self.assertNotEqual(
            evaluate(
                "getComputedStyle(document.querySelector('#original')).userSelect"
            ),
            "none",
        )
        self.assertEqual(
            evaluate("document.querySelector('button').getBoundingClientRect().height"),
            48,
        )
        self.assertEqual(
            evaluate("document.querySelector('button').getBoundingClientRect().width"),
            48,
        )


if __name__ == "__main__":
    unittest.main()
