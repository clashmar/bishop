#!/usr/bin/env python3
"""Serve this exported Bishop game on a local HTTP origin."""

from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import webbrowser


class PlayerHandler(SimpleHTTPRequestHandler):
    extensions_map = {
        **SimpleHTTPRequestHandler.extensions_map,
        ".js": "text/javascript",
        ".mjs": "text/javascript",
        ".wasm": "application/wasm",
    }

    def send_head(self):
        path = Path(self.translate_path(self.path)).resolve()
        if not path.is_relative_to(Path(self.directory).resolve()):
            self.send_error(403, "Outside export folder")
            return None
        return super().send_head()


def main():
    site = Path(__file__).resolve().parent
    handler = partial(PlayerHandler, directory=str(site))
    with ThreadingHTTPServer(("127.0.0.1", 0), handler) as server:
        url = f"http://127.0.0.1:{server.server_port}/"
        print(f"Serving {site} at {url} (Ctrl-C to stop)", flush=True)
        webbrowser.open(url)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass


if __name__ == "__main__":
    main()
