"""Summarize Apple sample call trees captured during full-theme Button builds.

Run `python3 profile.py OUT` after capturing OUT/{theme}-{variant}-profile.txt.
The raw stack symbols in stripped Rust release profiles use the v0 mangling.
"""
from collections import Counter
import json
from pathlib import Path
import re
import sys

directory = Path(sys.argv[1]).resolve()
frame = re.compile(r"^([ +!|:]*)([0-9]+) (.+?)  \(in ([^)]*)\)")
thread = re.compile(r"^\s+([0-9]+) Thread_")

def summarize(path):
    frames = []
    stack = []
    on_main = False
    body = path.read_text().split("Call graph:\n", 1)[1].split("Total number in stack", 1)[0]
    for line in body.splitlines():
        if thread.match(line):
            on_main = "com.apple.main-thread" in line
            stack.clear()
            continue
        m = frame.match(line)
        if not on_main or m is None:
            continue
        indent, count, symbol, library = m.groups()
        while stack and stack[-1]["depth"] >= len(indent):
            stack.pop()
        count = int(count)
        if stack:
            stack[-1]["self"] -= count
        item = dict(depth=len(indent), symbol=symbol, library=library, self=count,
                    parent=stack[-1] if stack else None)
        frames.append(item)
        stack.append(item)
    result = Counter()
    for node in frames:
        weight = node["self"]
        assert weight >= 0, node["symbol"]
        if not weight:
            continue
        ancestors = []
        cursor = node
        while cursor is not None:
            ancestors.append(cursor["symbol"])
            cursor = cursor["parent"]
        if not any("15ControlTemplate5build" in name for name in ancestors):
            continue
        result["inside_control_template_build"] += weight
        if any("runtime12construction" in name for name in ancestors):
            result["construction_vm_including_called_work"] += weight
        if "runtime12construction" in node["symbol"]:
            result["construction_module_exclusive"] += weight
        if any("bind_template_binding" in name for name in ancestors):
            result["template_binding_install_including_called_work"] += weight
        if any("ValueTypes8try_cast" in name for name in ancestors):
            result["value_types_try_cast_including_called_work"] += weight
        if node["library"] == "libsystem_malloc.dylib":
            result["malloc_library_exclusive"] += weight
    assert result["inside_control_template_build"] > 0
    return result

if __name__ == "__main__":
    summary = {}
    for theme in ("winui", "fluent"):
        summary[theme] = {}
        for variant in ("native", "stock", "single", "fused"):
            summary[theme][variant] = summarize(directory / f"{theme}-{variant}-profile.txt")
    (directory / "profiles.json").write_text(json.dumps(summary, indent=2) + "\n")
