#!/usr/bin/env python3
"""Drive the real ControlCatalog over both MCP transports (no third-party Python packages).
Build first: cargo build -p control-catalog-desktop --features mcp
Run: python3 scripts/test-mcp.py [path/to/control-catalog-desktop]
"""
import base64
import json
import os
import queue
import subprocess
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.request

VERSION = "2025-11-25"


class Catalog:
    def __init__(self, binary, http=False, writable=True):
        self.http = http
        self.sequence = 0
        self.session = None
        self.log_dir = tempfile.TemporaryDirectory(prefix="ferroui-mcp-")
        self.stderr = open(os.path.join(self.log_dir.name, "catalog.log"), "w+")
        args = [binary, "--mcp-headless", "--mcp-http=0" if http else "--mcp-stdio"]
        if writable:
            args += ["--mcp-allow-input", "--mcp-allow-writes"]
        env = {key: value for key, value in os.environ.items() if not key.startswith("FERROUI_SMOKE_")}
        # The catalog's own theme (WinUI): the Simple and Fluent themes need the feature
        # `classic-themes` of the catalog (docs/binary-size.md).
        env.pop("FERROUI_CATALOG_THEME", None)
        self.process = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                        stderr=self.stderr, text=True, env=env)
        self.messages = queue.Queue()
        if http:
            deadline = time.monotonic() + 60
            while time.monotonic() < deadline:
                with open(self.stderr.name) as log:
                    lines = log.read().splitlines()
                urls = [line.split("FerroUI MCP: ", 1)[1] for line in lines if "FerroUI MCP: http" in line]
                tokens = [line.split("Bearer ", 1)[1] for line in lines if line.startswith("Authorization: Bearer ")]
                if urls and tokens:
                    self.url, self.token = urls[-1], tokens[-1]
                    break
                if self.process.poll() is not None:
                    raise RuntimeError("Catalog exited: " + "\n".join(lines))
                time.sleep(.05)
            else:
                raise TimeoutError("HTTP startup")
        else:
            def read():
                for line in self.process.stdout:
                    try:
                        self.messages.put(json.loads(line))
                    except ValueError:
                        self.messages.put(RuntimeError("Non-JSON on MCP stdout: " + line))
                self.messages.put(RuntimeError("MCP stdout closed"))
            threading.Thread(target=read, daemon=True).start()
        self.rpc("initialize", {"protocolVersion": VERSION, "capabilities": {},
                                "clientInfo": {"name": "ferroui-smoke", "version": "1"}})
        self.rpc("notifications/initialized", notification=True)

    def rpc(self, method, params=None, notification=False):
        self.sequence += 1
        message = {"jsonrpc": "2.0", "method": method, "params": params or {}}
        if not notification:
            message["id"] = self.sequence
        if self.http:
            headers = {"Authorization": "Bearer " + self.token, "Content-Type": "application/json",
                       "Accept": "application/json, text/event-stream"}
            if self.session:
                headers.update({"Mcp-Session-Id": self.session, "MCP-Protocol-Version": VERSION})
            with urllib.request.urlopen(urllib.request.Request(self.url, json.dumps(message).encode(), headers), timeout=70) as response:
                self.session = response.headers.get("Mcp-Session-Id", self.session)
                data = response.read()
                if notification:
                    assert response.status == 202
                    return
                result = json.loads(data)
        else:
            self.process.stdin.write(json.dumps(message) + "\n")
            self.process.stdin.flush()
            if notification:
                return
            while True:
                result = self.messages.get(timeout=70)
                if isinstance(result, Exception):
                    raise result
                if result.get("id") == self.sequence:
                    break
        assert "error" not in result, result
        return result["result"]

    def tool(self, name, args=None, error=False):
        result = self.rpc("tools/call", {"name": name, "arguments": args or {}})
        assert result.get("isError", False) == error, result
        if error:
            return result
        if name == "screenshot":
            png = base64.b64decode(result["content"][0]["data"], validate=True)
            assert png.startswith(b"\x89PNG\r\n\x1a\n")
            return png
        return json.loads(result["content"][0]["text"])

    def finish_stdio(self):
        if not self.http:
            self.process.stdin.close()
            assert self.process.wait(timeout=10) == 0, "Catalog failed to shut down on MCP EOF"

    def close(self):
        self.process.terminate()
        try:
            self.process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait()
        self.stderr.close()
        self.log_dir.cleanup()


def exercise(binary, http=False, writable=True):
    catalog = Catalog.__new__(Catalog)
    try:
        catalog.__init__(binary, http, writable)
        assert len(catalog.rpc("tools/list")["tools"]) == 16
        catalog.tool("wait_for", {"type": "MainWindow", "timeout_ms": 30000})
        window = catalog.tool("list_windows")[0]["node_id"]
        if not writable:
            catalog.tool("set_property", {"node_id": window, "property": "Title", "value": "denied"}, error=True)
            catalog.tool("click", {"node_id": window}, error=True)
            catalog.finish_stdio()
            print("PASS stdio read-only")
            return
        for tree in ("visual", "logical", "automation"):
            assert catalog.tool("get_tree", {"tree": tree, "depth": 4, "properties": ["Width"]})
        editor = catalog.tool("find_elements", {"selector": "TextBox#SearchBox"})[0]["node_id"]
        catalog.tool("get_properties", {"node_id": editor, "properties": ["Text", "IsVisible"]})
        catalog.tool("click", {"node_id": editor})
        catalog.tool("type_text", {"node_id": editor, "text": "Button"})
        catalog.tool("wait_for", {"node_id": editor, "condition": "property_equals", "property": "Text", "value": "Button"})
        catalog.tool("press_key", {"node_id": editor, "key": "Ctrl+A"})
        catalog.tool("type_text", {"node_id": editor, "text": "TextBox"})
        catalog.tool("wait_for", {"node_id": editor, "condition": "property_equals", "property": "Text", "value": "TextBox"})
        catalog.tool("invoke_pattern", {"node_id": editor, "pattern": "set_value", "value": ""})
        catalog.tool("set_property", {"node_id": window, "property": "Title", "value": "MCP verified"})
        assert catalog.tool("get_properties", {"node_id": window, "properties": ["Title"]})[0]["value"] == "MCP verified"
        catalog.tool("scroll", {"node_id": editor, "delta_y": -1})
        catalog.tool("get_styles", {"node_id": editor})
        catalog.tool("get_resources", {"node_id": editor, "keys": ["SystemAccentColor"]})
        assert len(catalog.tool("screenshot", {"node_id": window, "scale": .5})) > 100
        catalog.tool("get_frame_stats")
        catalog.tool("get_logs")
        for uri in ("ferroui://tree", "ferroui://logs"):
            json.loads(catalog.rpc("resources/read", {"uri": uri})["contents"][0]["text"])
        if http:
            try:
                urllib.request.urlopen(catalog.url, timeout=5)
                raise AssertionError("Unauthenticated HTTP request was accepted")
            except urllib.error.HTTPError as error:
                assert error.code == 401
        catalog.finish_stdio()
        print("PASS", "HTTP" if http else "stdio", "writable" if writable else "read-only")
    except Exception:
        if hasattr(catalog, "stderr"):
            with open(catalog.stderr.name) as log:
                print(log.read()[-12000:], file=sys.stderr)
        raise
    finally:
        if hasattr(catalog, "process"):
            catalog.close()


if __name__ == "__main__":
    target = os.environ.get("CARGO_TARGET_DIR", "target")
    binary = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else os.path.join(target, "debug", "control-catalog-desktop"))
    exercise(binary)
    exercise(binary, http=True)
    exercise(binary, writable=False)
