"""Prism shell browser checks against a running, empty development node.

Run inside nix develop after just build-assets and cargo build -p neoprism-node:
  target/debug/neoprism-node dev --db-url sqlite::memory: --port 18772
  python tools/ui-tests/prism_shell.py
Requires agent-browser and Chromium. Uses a named, isolated browser session.
"""

import os
import subprocess
import unittest
from urllib.parse import parse_qs, urlparse

from browser_adapter import Browser

BASE_URL = os.environ.get("PRISM_TEST_URL", "http://127.0.0.1:18772")
SESSION = (
    os.environ.get("PRISM_TEST_SESSION")
    or subprocess.check_output(
        [
            "agent-browser",
            "session",
            "id",
            "--scope",
            "worktree",
            "--prefix",
            "prism-shell",
        ],
        text=True,
    ).strip()
)


browser = Browser(SESSION)
evaluate = browser.evaluate


class PrismShell(unittest.TestCase):
    def setUp(self) -> None:
        browser("set", "viewport", "390", "898")
        browser("open", BASE_URL + "/resolver")

    def test_mobile_brand_home_remains_clickable_beside_menu(self) -> None:
        browser("click", ".prism-mobile-header .prism-brand")
        browser("wait", "--url", "**/resolver")
        self.assertTrue(browser("get", "url")["url"].endswith("/resolver"))

    def test_responsive_layout_preserves_reading_order_targets_and_local_fonts(
        self,
    ) -> None:
        for width in (320, 390, 720, 800, 801, 1024, 1440):
            with self.subTest(width=width):
                browser("set", "viewport", str(width), "900")
                self.assertTrue(
                    evaluate("document.documentElement.scrollWidth <= innerWidth")
                )
                self.assertTrue(
                    evaluate(
                        """Array.from(document.querySelectorAll(
                          'a, button, input, summary'))
                          .filter(e => e.checkVisibility()
                            && !e.classList.contains('skip-link'))
                          .every(e => e.getBoundingClientRect().height >= 48)"""
                    )
                )
                self.assertTrue(
                    evaluate(
                        "getComputedStyle(document.querySelector('main'))"
                        ".overflowY === 'visible'"
                    )
                )
                self.assertTrue(
                    evaluate(
                        """document.querySelector('header.prism-page-header')
                          .getBoundingClientRect().bottom
                          <= document.querySelector('form')
                          .getBoundingClientRect().top"""
                    )
                )
        browser("wait", "--fn", "document.fonts.status === 'loaded'")
        self.assertTrue(
            evaluate(
                """document.fonts.check('400 16px Poppins')
                  && document.fonts.check('500 14px Poppins')
                  && document.fonts.check('600 30px Poppins')
                  && document.fonts.check('400 14px "JetBrains Mono"')"""
            )
        )
        self.assertTrue(
            evaluate(
                r"""performance.getEntriesByType('resource')
                  .filter(e => /\.(ttf|woff2?)(\?|$)/.test(e.name))
                  .every(e => new URL(e.name).origin === location.origin)"""
            )
        )
        browser("set", "media", "light", "reduced-motion")
        self.assertEqual(
            evaluate(
                "getComputedStyle(document.querySelector('button')).transitionDuration"
            ),
            "0s",
        )

    def test_landmarks_keyboard_focus_and_full_page_accessibility(self) -> None:
        for width in (390, 1440):
            with self.subTest(width=width):
                browser("set", "viewport", str(width), "900")
                self.assertEqual(
                    evaluate("document.querySelectorAll('main').length"), 1
                )
                self.assertEqual(evaluate("document.querySelectorAll('h1').length"), 1)
                self.assertTrue(
                    evaluate("document.querySelector('label[for=did]') !== null")
                )
                audit = browser("a11y")
                self.assertEqual(audit["counts"]["violations"], 0, audit)
                browser("focus", "#did")
                browser("press", "Tab")
                ring = evaluate("""(() => {
                    const e = document.activeElement;
                    const s = getComputedStyle(e);
                    return {tag: e.tagName, width: s.outlineWidth,
                        style: s.outlineStyle, offset: s.outlineOffset};
                })()""")
                self.assertEqual(
                    ring,
                    {
                        "tag": "BUTTON",
                        "width": "2px",
                        "style": "solid",
                        "offset": "2px",
                    },
                )

    def test_two_hundred_percent_layout_zoom_and_opaque_capability_fallback(
        self,
    ) -> None:
        browser("set", "viewport", "1440", "900")
        # Layout magnification, not a claim about the native browser zoom UI.
        evaluate("document.documentElement.style.zoom = '2'")
        try:
            self.assertTrue(
                evaluate("document.documentElement.scrollWidth <= innerWidth")
            )
            self.assertTrue(
                evaluate("document.querySelector('#did').checkVisibility()")
            )
            browser("focus", "#did")
            browser("press", "Tab")
            self.assertEqual(evaluate("document.activeElement.type"), "submit")
        finally:
            evaluate("document.documentElement.style.zoom = ''")
        # At the stylesheet capability boundary, an unsupported @supports block
        # is discarded. Exercise the shipped opaque base, not replacement CSS.
        evaluate("""Array.from(document.styleSheets).forEach(sheet => {
            for (let i = sheet.cssRules.length - 1; i >= 0; i--) {
                const rule = sheet.cssRules[i];
                if (rule instanceof CSSSupportsRule
                    && rule.conditionText.includes('backdrop-filter')) {
                    sheet.deleteRule(i);
                }
            }
        })""")
        self.assertEqual(
            evaluate(
                "getComputedStyle(document.querySelector('.prism-shell')).backgroundColor"
            ),
            "rgb(238, 238, 247)",
        )
        self.assertEqual(
            evaluate(
                "getComputedStyle(document.querySelector('.prism-shell')).backdropFilter"
            ),
            "none",
        )
        self.assertEqual(
            evaluate(
                "getComputedStyle(document.querySelector('.prism-sidebar')).backgroundColor"
            ),
            "rgb(247, 245, 252)",
        )
        audit = browser("a11y")
        self.assertEqual(audit["counts"]["violations"], 0, audit)

    def test_without_scripts_native_menu_and_long_form_get_handoff_work(self) -> None:
        long_form = (
            "did:prism:9b5118411248d9663b6ab15128fba810"
            "6511230ff654e7514cdcc4ce919bde9b:"
            "Cj8KPRI7CgdtYXN0ZXIwEAFKLgoJc2VjcDI1NmsxEiEDHpf-"
            "yhIns-LP3tLvA8icC5FJ1ZlBwbllPtIdNZ3q0jU"
        )
        # Fresh context: cached scripts must not bypass request interception.
        browser("close")
        browser("open")
        browser("set", "viewport", "390", "898")
        browser("network", "route", "*", "--abort", "--resource-type", "script")
        try:
            browser("open", BASE_URL + "/resolver")
            self.assertIsNone(
                evaluate(
                    "document.querySelector('#menu-toggle').getAttribute('aria-expanded')"
                )
            )
            browser("focus", "#menu-toggle")
            browser("press", "Space")
            self.assertTrue(evaluate("document.querySelector('#mobile-menu').open"))
            browser("press", "Tab")
            browser("press", "Tab")
            browser("press", "Enter")
            browser("wait", "--url", "**/explorer")
            browser("open", BASE_URL + "/resolver")
            browser("fill", "#did", long_form)
            browser("press", "Enter")
            browser("wait", "--url", "**/resolver?did=*")
            query = parse_qs(urlparse(browser("get", "url")["url"]).query)
            self.assertEqual(query["did"], [long_form])
            self.assertEqual(
                evaluate("document.querySelector('#did').value"), long_form
            )
            self.assertIn(
                "DID state", evaluate("document.querySelector('main').textContent")
            )
            self.assertIn(
                "master0", evaluate("document.querySelector('main').textContent")
            )
        finally:
            browser("network", "unroute", "*")

    def test_composition_retains_exact_value_then_enter_submits_canonical_did(
        self,
    ) -> None:
        canonical = "did:prism:" + "ab" * 32
        browser("fill", "#did", canonical)
        evaluate(
            """document.querySelector('#did').dispatchEvent(
              new CompositionEvent('compositionstart',
                {data:'あ', bubbles:true}))"""
        )
        evaluate(
            """document.querySelector('#did').dispatchEvent(
              new KeyboardEvent('keydown',
                {key:'Enter', isComposing:true, bubbles:true}))"""
        )
        self.assertEqual(browser("get", "url")["url"], BASE_URL + "/resolver")
        self.assertFalse(
            evaluate("document.querySelector('button[type=submit]').disabled")
        )
        self.assertEqual(evaluate("document.querySelector('#did').value"), canonical)
        evaluate(
            """document.querySelector('#did').dispatchEvent(
              new CompositionEvent('compositionend', {bubbles:true}))"""
        )
        browser("press", "Enter")
        browser("wait", "--url", "**/resolver?did=*")
        self.assertEqual(evaluate("document.querySelector('#did').value"), canonical)
        self.assertEqual(
            parse_qs(urlparse(browser("get", "url")["url"]).query)["did"], [canonical]
        )
        self.assertIn(
            "No indexed state on this node",
            evaluate("document.querySelector('main').textContent"),
        )

    def test_native_validation_precedes_pending_feedback_and_pageshow_recovers(
        self,
    ) -> None:
        browser("focus", "#did")
        browser("press", "Enter")
        self.assertTrue(
            evaluate("document.querySelector('#did').validity.valueMissing")
        )
        self.assertFalse(
            evaluate("document.querySelector('button[type=submit]').disabled")
        )
        browser("fill", "#did", "did:prism:" + "ab" * 32)
        # Hold the HTTP boundary, not the submit event. The outcome suite also
        # verifies real delayed GETs and network/HTTP failure recovery.
        evaluate("""window.fetch = () => new Promise(() => {})""")
        browser("press", "Enter")
        self.assertTrue(
            evaluate("document.querySelector('button[type=submit]').disabled")
        )
        self.assertEqual(
            evaluate("document.querySelector('button[type=submit]').textContent"),
            "Resolving…",
        )
        self.assertIn(
            "Resolving", evaluate("document.querySelector('[role=status]').textContent")
        )
        evaluate(
            "window.dispatchEvent(new PageTransitionEvent('pageshow',"
            " {persisted:true}))"
        )
        self.assertFalse(
            evaluate("document.querySelector('button[type=submit]').disabled")
        )
        self.assertEqual(
            evaluate("document.querySelector('button[type=submit]').textContent"),
            "Resolve",
        )

    def test_mobile_navigation_is_keyboard_operable_and_escape_restores_focus(
        self,
    ) -> None:
        self.assertEqual(
            evaluate(
                "getComputedStyle(document.querySelector('.prism-sidebar')).display"
            ),
            "none",
        )
        self.assertFalse(evaluate("document.querySelector('#mobile-menu').open"))
        browser("focus", "#menu-toggle")
        browser("press", "Enter")
        self.assertTrue(evaluate("document.querySelector('#mobile-menu').open"))
        browser("press", "Tab")
        self.assertEqual(
            evaluate("document.activeElement.textContent.trim()"), "Resolver"
        )
        browser("press", "Escape")
        self.assertFalse(evaluate("document.querySelector('#mobile-menu').open"))
        self.assertEqual(evaluate("document.activeElement.id"), "menu-toggle")
        browser("press", "Space")
        self.assertTrue(evaluate("document.querySelector('#mobile-menu').open"))
        browser("press", "Tab")
        browser("press", "Tab")
        browser("press", "Enter")
        browser("wait", "--url", "**/explorer")
        self.assertTrue(browser("get", "url")["url"].endswith("/explorer"))


if __name__ == "__main__":
    try:
        unittest.main()
    finally:
        browser("close")
