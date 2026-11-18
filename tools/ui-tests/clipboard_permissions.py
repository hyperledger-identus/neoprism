"""Grant clipboard access only to a fixture origin in the caller's owned browser.

agent-browser launches an isolated named session; its CDP endpoint never attaches
us to a shared/user browser. Node 24's WebSocket avoids additional dependencies.
Permission/CDP failures are test failures, never grounds for skipping copy tests.
"""

import subprocess
from collections.abc import Callable, Iterator
from contextlib import contextmanager
from typing import Any
from urllib.parse import urlparse


@contextmanager
def grant_clipboard_permissions(
    browser: Callable[..., dict[str, Any]], origin: str
) -> Iterator[None]:
    endpoint = browser("get", "cdp-url")["cdpUrl"]
    for value, scheme in ((origin, "http"), (endpoint, "ws")):
        parsed = urlparse(value)
        if parsed.scheme != scheme or parsed.hostname != "127.0.0.1":
            raise AssertionError(f"expected owned loopback fixture/browser: {value}")
    process = subprocess.Popen(
        [
            "node",
            "--input-type=module",
            "-e",
            """
            const [endpoint, origin] = process.argv.slice(1);
            const socket = new WebSocket(endpoint);
            try {
              await new Promise((resolve, reject) => {
                const timeout = setTimeout(
                  () => reject(new Error('CDP timeout')), 10000);
                socket.addEventListener('error', () =>
                  reject(new Error('CDP connection failed')));
                socket.addEventListener('open', () => socket.send(JSON.stringify({
                  id: 1, method: 'Browser.grantPermissions', params: {
                    origin,
                    permissions: ['clipboardReadWrite', 'clipboardSanitizedWrite']
                  }
                })));
                socket.addEventListener('message', ({data}) => {
                  const response = JSON.parse(data);
                  if (response.id !== 1) return;
                  clearTimeout(timeout);
                  if (response.error) reject(new Error(JSON.stringify(response.error)));
                  else resolve();
                });
              });
              // Chromium revokes overrides when this CDP connection closes.
              // Keep it alive for the test; Python closes stdin during cleanup.
              process.stdout.write('ready\\n');
              await new Promise(resolve => process.stdin.on('end', resolve).resume());
            } finally {
              socket.close();
            }
            """,
            endpoint,
            origin,
        ],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        assert process.stdout is not None
        if process.stdout.readline().strip() != "ready":
            raise AssertionError("clipboard permission setup failed")
        yield
    finally:
        stdout, stderr = process.communicate(timeout=15)
        if process.returncode:
            raise AssertionError(
                f"clipboard permission helper failed: {stdout}{stderr}"
            )
