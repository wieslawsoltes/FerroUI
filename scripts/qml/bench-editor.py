"""Measure warmed QML editor operations over the actual LSP stdio transport."""
import argparse
import json
import statistics
import subprocess
import tempfile
import time
from pathlib import Path


def measure(server, properties, iterations):
    proc = subprocess.Popen([server], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    counter = 0

    def send(method, params, request=False):
        nonlocal counter
        message = {"jsonrpc": "2.0", "method": method, "params": params}
        if request:
            counter += 1
            message["id"] = counter
        body = json.dumps(message, separators=(",", ":")).encode()
        proc.stdin.write(f"Content-Length: {len(body)}\r\n\r\n".encode() + body)
        proc.stdin.flush()

    def receive():
        headers = {}
        while True:
            line = proc.stdout.readline()
            if not line:
                raise RuntimeError("language server closed stdout")
            if not line.strip():
                break
            key, value = line.decode().split(":", 1)
            headers[key.lower()] = value.strip()
        return json.loads(proc.stdout.read(int(headers["content-length"])))

    def request(method, params):
        send(method, params, True)
        reply = receive()
        if reply.get("id") != counter or "error" in reply:
            raise RuntimeError(reply)
        return reply["result"]

    timings = {}
    try:
        with tempfile.TemporaryDirectory() as root:
            folder = Path(root).resolve()
            (folder / "Base.qml").write_text("import QtQuick\nItem {\n" + "".join(
                f"property int inherited{i}: {i}\n" for i in range(properties)) + "}\n")
            uri = (folder / "App.qml").as_uri()
            text = "import QtQuick\nBase {\n" + "".join(
                f"property int local{i}: {i}\n" for i in range(properties)) + (
                "property int combined: local0 + inherited0\n}\n")
            request("initialize", {"rootUri": folder.as_uri()})
            send("initialized", {})
            send("textDocument/didOpen", {"textDocument": {
                "uri": uri, "languageId": "qml", "text": text, "version": 1}})
            assert receive()["params"]["diagnostics"] == []
            doc = {"uri": uri}
            version = 1
            for label in ["local_hover", "inherited_definition", "completion", "edit_diagnostics"]:
                samples = []
                for _ in range(iterations + 1):
                    start = time.perf_counter_ns()
                    if label == "edit_diagnostics":
                        version += 1
                        point = {"line": 2, "character": len("property int local0: ")}
                        send("textDocument/didChange", {"textDocument": {**doc, "version": version},
                             "contentChanges": [{"range": {"start": point,
                             "end": {**point, "character": point["character"] + 1}}, "text": str(version % 10)}]})
                        reply = receive()
                        assert reply["params"]["version"] == version and not reply["params"]["diagnostics"]
                    else:
                        column = {"local_hover": 24, "inherited_definition": 33, "completion": 0}[label]
                        method = {"local_hover": "hover", "inherited_definition": "definition",
                                  "completion": "completion"}[label]
                        result = request("textDocument/" + method, {"textDocument": doc,
                            "position": {"line": properties + 2, "character": column}})
                        assert result, (label, result)
                        if label == "inherited_definition":
                            assert result["uri"] == (folder / "Base.qml").as_uri()
                        if label == "completion":
                            assert len(result) >= 2 * properties
                    samples.append((time.perf_counter_ns() - start) / 1e6)
                timings[label] = round(statistics.median(samples[1:]), 3)
            request("shutdown", None)
            send("exit", None)
    finally:
        proc.stdin.close()
        proc.wait(timeout=10)
    if proc.returncode:
        raise RuntimeError(proc.returncode)
    return timings


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("server")
    parser.add_argument("--properties", default=800, type=int)
    parser.add_argument("--iterations", default=11, type=int)
    args = parser.parse_args()
    print(json.dumps(measure(args.server, args.properties, args.iterations), sort_keys=True))
