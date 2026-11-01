"""Run the compiled Wasm fixture in an isolated Chromium browsing context."""
import argparse
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import subprocess
import tempfile
from threading import Event, Thread

parser = argparse.ArgumentParser()
parser.add_argument("--chrome", required=True)
parser.add_argument("--site", type=Path, required=True)
args = parser.parse_args()
done = Event()
report = []


class Handler(SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        super().end_headers()

    def do_POST(self):
        if self.path != "/result":
            self.send_error(404)
            return
        report.append(self.rfile.read(min(8192, int(self.headers["Content-Length"]))).decode())
        self.send_response(204)
        self.end_headers()
        done.set()


with ThreadingHTTPServer(("127.0.0.1", 0), partial(Handler, directory=args.site)) as site:
    thread = Thread(target=site.serve_forever, daemon=True)
    thread.start()
    with tempfile.TemporaryDirectory() as profile:
        browser = subprocess.Popen([
            args.chrome, "--headless", "--no-sandbox", "--disable-dev-shm-usage",
            "--disable-gpu", "--disable-background-networking", "--user-data-dir=" + profile,
            "http://127.0.0.1:" + str(site.server_port) + "/index.html",
        ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        try:
            if not done.wait(60):
                raise SystemExit("Browser fixture did not return a result")
            if report != ["passed"]:
                raise SystemExit(report[0])
            print("FerroUI QML browser conformance passed")
        finally:
            browser.terminate()
            browser.wait(timeout=15)
            site.shutdown()
