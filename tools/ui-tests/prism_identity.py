"""Browser seam for test-only rendered Resolver fixtures; no production sample route.

Export with the ignored export_identity_browser_fixtures Rust test, setting
PRISM_FIXTURE_DIR to a directory outside the checkout; run just build-assets.
This suite serves those exact rendered pages and built assets on a private port.
"""

import json
import os
import unittest
from pathlib import Path

from clipboard_permissions import grant_clipboard_permissions
from fixture_server import fixture_server
from prism_shell import browser, evaluate

FIXTURES = Path(os.environ["PRISM_FIXTURE_DIR"])
PORT = int(os.environ.get("PRISM_IDENTITY_PORT", "18773"))
URL = f"http://127.0.0.1:{PORT}"


class PrismIdentity(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.server = cls.enterClassContext(fixture_server(FIXTURES, PORT))

    @classmethod
    def tearDownClass(cls) -> None:
        browser("close")

    def setUp(self) -> None:
        browser("set", "viewport", "390", "900")
        browser("open", URL + "/indexed.html")

    def test_captured_json_copy_preserves_crlf_bare_cr_unicode_and_markup(self) -> None:
        self.enterContext(grant_clipboard_permissions(browser, URL))
        browser("open", URL + "/lossless.html")
        evaluate(
            "document.querySelectorAll('main details').forEach(e => e.open = true)"
        )
        expected = (
            '{\r\n"uri":"https://example.test/雪?x=1&y=<tag>",\r"label":"café 🧪"\r\n}'
        )
        browser("focus", 'button[aria-label="Copy service lossless captured endpoint"]')
        browser("press", "Enter")
        browser("wait", "--text", "Copied to clipboard.")
        self.assertEqual(browser("clipboard", "read")["text"], expected)

    def test_manual_copy_preserves_captured_crlf_and_bare_cr_without_normalization(
        self,
    ) -> None:
        self.enterContext(grant_clipboard_permissions(browser, URL))
        browser("open", URL + "/lossless.html")
        browser("set", "viewport", "320", "900")
        evaluate(
            "document.querySelectorAll('main details').forEach(e => e.open = true)"
        )
        expected = (
            '{\r\n"uri":"https://example.test/雪?x=1&y=<tag>",\r"label":"café 🧪"\r\n}'
        )
        for clipboard in (
            "undefined",
            "{writeText: async () => {throw new Error('denied')}}",
        ):
            with self.subTest(clipboard=clipboard):
                evaluate(
                    "Object.defineProperty(navigator, 'clipboard', "
                    f"{{configurable: true, value: {clipboard}}})"
                )
                browser(
                    "focus",
                    'button[aria-label="Copy service lossless captured endpoint"]',
                )
                browser("press", "Enter")
                browser(
                    "wait",
                    "--text",
                    "Couldn’t copy. Select the value beside the button "
                    "to copy manually.",
                )
                self.assertEqual(
                    evaluate("document.activeElement.ariaLabel"),
                    "Copy service lossless captured endpoint",
                )
                # Observe the user-visible fallback, never the encoded transport.
                self.assertEqual(
                    evaluate(
                        """(() => {
                            const field = document.querySelector(
                                '.prism-copy-fallback').lastElementChild;
                            return field.value ?? field.textContent;
                        })()"""
                    ),
                    expected,
                )
                browser("press", "Tab")
                browser("press", "Control+a")
                self.assertEqual(evaluate("window.getSelection().toString()"), expected)
                browser("press", "Control+c")
                evaluate("delete navigator.clipboard")
                self.assertEqual(browser("clipboard", "read")["text"], expected)
                self.assertTrue(
                    evaluate(
                        """(() => {
                            const r = document.activeElement.getBoundingClientRect();
                            return r.left >= 0 && r.right <= innerWidth
                                && r.width >= 140;
                        })()"""
                    )
                )
                self.assertTrue(
                    evaluate("document.documentElement.scrollWidth <= innerWidth")
                )

    def test_context_and_service_copy_preserves_complete_captured_and_readable_values(
        self,
    ) -> None:
        evaluate(
            "document.querySelectorAll('#contexts details, #services details')"
            ".forEach(e => e.open = true)"
        )
        evaluate("""window.copied = null;
            Object.defineProperty(navigator, 'clipboard', {configurable: true,
            value: {writeText: async value => {window.copied = value}}})""")
        expected = {
            "context 1": "https://example.test/z?first=1&second=2",
            "context 2": "https://example.test/a/" + "long-context-" * 80,
            "service messaging ID": "messaging",
            "service messaging endpoint": "https://example.test/didcomm?x=1&y=2",
            "service profile captured types": '["LinkedDomains","Profile"]',
            "service profile captured endpoint": (
                '{ "z": {"accept": ["application/json",'
                'null,7,true]}, "a": "<profile>" }'
            ),
            "service mirrors captured types": '["LinkedDomains"]',
            "service mirrors captured endpoint": (
                '[ "https://example.test/one", '
                '{ "uri": "https://example.test/two", '
                '"nested": {"empty": [], "object": {}} } ]'
            ),
            (
                "service long-service-identifier-01234567890123456789012345 "
                "captured endpoint"
            ): "https://example.test/" + "long-path-" * 25 + "?x=1&y=2",
        }
        raw = json.loads((FIXTURES / "services-raw.json").read_text())
        for index, service in enumerate(
            (
                "messaging",
                "profile",
                "mirrors",
                "long-service-identifier-01234567890123456789012345",
            )
        ):
            expected[f"service {service} raw typed representation"] = raw[index]
        for name, original in expected.items():
            with self.subTest(value=name):
                selector = f'button[aria-label="Copy {name}"]'
                browser("focus", selector)
                evaluate("window.copied = null")
                browser("press", "Enter")
                browser("wait", "--fn", "window.copied !== null")
                self.assertEqual(evaluate("window.copied"), original)
                self.assertEqual(
                    evaluate("document.activeElement.ariaLabel"), f"Copy {name}"
                )
                self.assertEqual(
                    evaluate(
                        "document.querySelector('.prism-copy-status').textContent"
                    ),
                    "Copied to clipboard.",
                )
                self.assertEqual(
                    evaluate("document.querySelector('.prism-copy-status').ariaLive"),
                    "polite",
                )
        selector = 'button[aria-label="Copy service profile readable JSON"]'
        browser("focus", selector)
        evaluate("window.copied = null")
        browser("press", "Space")
        browser("wait", "--fn", "window.copied !== null")
        self.assertEqual(
            json.loads(evaluate("window.copied")),
            {
                "id": "profile",
                "type": ["LinkedDomains", "Profile"],
                "serviceEndpoint": {
                    "z": {"accept": ["application/json", None, 7, True]},
                    "a": "<profile>",
                },
            },
        )
        self.assertEqual(
            evaluate(
                """Array.from(document.querySelectorAll('#contexts ol code'))
                .map(e => e.textContent)"""
            ),
            [expected["context 1"], expected["context 2"]],
        )
        self.assertEqual(
            evaluate(
                """Array.from(document.querySelectorAll(
                '.prism-service > summary code')).map(e => e.textContent)"""
            ),
            [
                "messaging",
                "profile",
                "mirrors",
                "long-service-identifier-01234567890123456789012345",
            ],
        )

    def test_context_service_copy_failure_keeps_focus_and_selectable_fallback_in_bounds(
        self,
    ) -> None:
        browser("set", "viewport", "320", "900")
        evaluate(
            "document.querySelectorAll('#contexts details, #services details')"
            ".forEach(e => e.open = true)"
        )
        for clipboard in (
            "undefined",
            "{writeText: async () => {throw new Error('denied')}}",
        ):
            evaluate(
                "Object.defineProperty(navigator, 'clipboard', "
                f"{{configurable: true, value: {clipboard}}})"
            )
            for name in (
                "context 2",
                "service profile captured endpoint",
                "service messaging raw typed representation",
            ):
                with self.subTest(clipboard=clipboard, value=name):
                    selector = f'button[aria-label="Copy {name}"]'
                    expected = browser("get", "attr", selector, "data-copy-value")[
                        "value"
                    ]
                    browser("focus", selector)
                    browser("press", "Space")
                    browser(
                        "wait",
                        "--text",
                        "Couldn’t copy. Select the value beside the button "
                        "to copy manually.",
                    )
                    self.assertEqual(
                        evaluate("document.activeElement.ariaLabel"), f"Copy {name}"
                    )
                    result = evaluate("""(() => {
                        const button = document.activeElement;
                        const field = button.nextElementSibling
                            .querySelector('textarea');
                        const r = field.getBoundingClientRect();
                        return {value: field.value, readonly: field.readOnly,
                            labelled: field.labels.length > 0,
                            bounded: r.left >= 0 && r.right <= innerWidth
                                && r.width >= 140};
                    })()""")
                    self.assertEqual(result["value"], expected)
                    self.assertTrue(
                        result["readonly"] and result["labelled"] and result["bounded"]
                    )
                    browser("press", "Tab")
                    self.assertEqual(
                        evaluate("document.activeElement.tagName"), "TEXTAREA"
                    )
                    browser("press", "Control+a")
                    self.assertEqual(
                        evaluate(
                            "document.activeElement.selectionEnd - "
                            "document.activeElement.selectionStart"
                        ),
                        len(expected),
                    )
                    self.assertTrue(
                        evaluate("document.documentElement.scrollWidth <= innerWidth")
                    )

    def test_context_service_native_keyboard_and_no_script_disclosures(self) -> None:
        browser("close")
        browser("open")
        browser("network", "route", "*", "--abort", "--resource-type", "script")
        try:
            browser("open", URL + "/indexed.html")
            for selector in (
                "#contexts details",
                ".prism-service",
                ".prism-service-representation",
                ".prism-service > .prism-service-representation:nth-of-type(2)",
                ".prism-service-representation details",
            ):
                browser("focus", selector + " > summary")
                browser("press", "Enter")
                self.assertTrue(
                    evaluate(f"document.querySelector({json.dumps(selector)}).open"),
                    selector,
                )
                browser("press", "Space")
                self.assertFalse(
                    evaluate(f"document.querySelector({json.dumps(selector)}).open")
                )
                browser("press", "Enter")
            self.assertEqual(
                evaluate("document.querySelector('#contexts ol code').textContent"),
                "https://example.test/z?first=1&second=2",
            )
            raw = json.loads((FIXTURES / "services-raw.json").read_text())[0]
            self.assertEqual(
                evaluate(
                    "document.querySelector('.prism-service-representation "
                    "details pre').textContent"
                ),
                raw,
            )
            self.assertNotEqual(
                evaluate(
                    "getComputedStyle(document.querySelector("
                    "'.prism-service-representation details code')).userSelect"
                ),
                "none",
            )
            self.assertTrue(
                evaluate("document.documentElement.scrollWidth <= innerWidth")
            )
            browser("open", URL + "/deactivated.html")
            for selector, message in (
                ("#contexts", "No contexts"),
                ("#services", "No services"),
            ):
                self.assertIn(
                    message,
                    evaluate(f"document.querySelector('{selector}').textContent"),
                )
                self.assertIsNone(
                    evaluate(f"document.querySelector('{selector} details')")
                )
        finally:
            browser("network", "unroute", "*")

    def test_expanded_contexts_and_service_representations_stay_within_bounds(
        self,
    ) -> None:
        evaluate(
            "document.querySelectorAll('#contexts details, #services details')"
            ".forEach(e => e.open = true)"
        )
        for width in (320, 390, 600, 800, 801, 1024, 1280, 1440):
            with self.subTest(width=width):
                browser("set", "viewport", str(width), "900")
                self.assertTrue(
                    evaluate("document.documentElement.scrollWidth <= innerWidth")
                )
                self.assertTrue(
                    evaluate("""Array.from(document.querySelectorAll(
                    '#contexts code, #services pre, #services summary, '
                    + '#contexts summary, #services button'))
                    .filter(e => e.checkVisibility()).every(e => {
                        const r = e.getBoundingClientRect();
                        return r.left >= 0 && r.right <= innerWidth;
                    })""")
                )
                self.assertTrue(
                    evaluate("""Array.from(document.querySelectorAll(
                    '#contexts summary, #services summary, '
                    + '#contexts button, #services button'))
                    .filter(e => e.checkVisibility())
                    .every(e => e.getBoundingClientRect().height >= 48)""")
                )
                self.assertTrue(
                    evaluate("""Array.from(document.querySelectorAll(
                    '#contexts code, #services code'))
                    .every(e => getComputedStyle(e).userSelect !== 'none')""")
                )

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

    def test_labelled_copy_did_fits_one_line_without_shrinking_icon_targets(
        self,
    ) -> None:
        evaluate(
            "document.querySelectorAll('main details').forEach(e => e.open = true)"
        )
        for width in (320, 390, 720, 1024, 1440):
            with self.subTest(width=width):
                browser("set", "viewport", str(width), "900")
                browser("wait", "--fn", "document.fonts.status === 'loaded'")
                geometry = evaluate("""(() => {
                    const button = document.querySelector(
                        'button[aria-label="Copy DID"]');
                    const range = document.createRange();
                    range.selectNodeContents(button);
                    const label = range.getBoundingClientRect();
                    const box = button.getBoundingClientRect();
                    const style = getComputedStyle(button);
                    const left = parseFloat(style.paddingLeft);
                    const right = parseFloat(style.paddingRight);
                    return {
                        lines: range.getClientRects().length,
                        fits: label.left >= box.left + left
                            && label.right <= box.right - right,
                        height: box.height,
                        icons: Array.from(document.querySelectorAll(
                            '.prism-data-field .prism-copy')).every(e => {
                                const r = e.getBoundingClientRect();
                                return r.width === 48 && r.height === 48;
                            })
                    };
                })()""")
                self.assertEqual(geometry["lines"], 1, geometry)
                self.assertTrue(geometry["fits"], geometry)
                self.assertEqual(geometry["height"], 48, geometry)
                self.assertTrue(geometry["icons"], geometry)

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
        # Hold the fetch boundary; preventing native submission does not hold
        # the enhanced GET. prism_outcomes.py separately tests real delayed HTTP.
        evaluate("window.fetch = () => new Promise(() => {})")
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
