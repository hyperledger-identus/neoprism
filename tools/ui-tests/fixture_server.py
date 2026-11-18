"""Owned fixture server lifecycle; failure behavior remains in each suite's handler."""

import threading
from collections.abc import Iterator
from contextlib import contextmanager
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from shutil import copytree

ASSETS = Path(__file__).resolve().parents[2] / "bin/neoprism-node/assets"


@contextmanager
def fixture_server[Server: ThreadingHTTPServer](
    fixtures: Path,
    port: int,
    handler: type[SimpleHTTPRequestHandler] = SimpleHTTPRequestHandler,
    server_type: type[Server] = ThreadingHTTPServer,
) -> Iterator[Server]:
    copytree(ASSETS, fixtures / "assets", dirs_exist_ok=True)
    server = server_type(("127.0.0.1", port), partial(handler, directory=str(fixtures)))
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield server
    finally:
        server.shutdown()
        server.server_close()
        thread.join()
