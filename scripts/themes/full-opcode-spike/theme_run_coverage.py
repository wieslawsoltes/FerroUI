"""Audit the packed programs of all extracted unit functions in a generated theme module."""

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re


RUN = re.compile(r"^fn (\w+_unit_\d+)\(\n.*?XamlLoadError> \{\n(.*?)^\}", re.M | re.S)
CALL = re.compile(r"rt::apply_theme_run\(&const \{ rt::program_bytes::<(\d+)>\(__FERROUI_XAML_IMAGE, (\d+)\) \}")
# Layouts are in theme_run_opcodes.rs and theme_run_program.rs. Each 'v' is a
# variable-length import index or position, 'n' a packed double, 'q' an i64.
OPERANDS = {
    1: ("alias_rows", "v"), 2: ("literal_brush_rows", "v"), 3: ("dynamic_brush_rows", "v"),
    4: ("static_brush_rows", "v"), 5: ("double_rows", "v"), 6: ("thickness_rows", "v"),
    7: ("corner_radius_rows", "v"), 8: ("color_rows", "v"), 9: ("text_rows", "v"),
    10: ("defer_text", "vvvv"), 11: ("defer_type", "vvvvvv"), 12: ("constant", "vvvv"),
    13: ("parse", "vvvvvvv"), 14: ("int", "vvvv"), 15: ("bool", "vvvb"),
    16: ("from_double", "vvvvn"), 17: ("from_i64", "vvvvq"),
}


def instructions(data):
    if data[:1] != b"\x01":
        raise ValueError("unexpected theme-run version")
    pos = 1
    while pos < len(data):
        opcode, layout = OPERANDS[data[pos]]
        pos += 1
        for operand in layout:
            if operand == "v":
                while data[pos] & 128:
                    pos += 1
            elif operand == "q":
                pos += 7
            elif operand == "n" and data[pos] == 255:
                pos += 8
            pos += 1
            if pos > len(data):
                raise ValueError("truncated theme-run operand")
        yield opcode


def audit(module, per_run):
    source = module.read_text(encoding="utf-8")
    runs = RUN.findall(source)
    if not runs:
        raise ValueError(f"no extracted unit functions in {module}")
    artifact = re.search(r'const __FERROUI_XAML_IMAGE: &\[u8\] = include_bytes!\("([^"]+)"\)', source)
    image = (module.parent / artifact[1]).read_bytes() if artifact else b""
    counts = Counter()
    details = {}
    fallbacks = []
    streams = set()
    for name, body in runs:
        calls = CALL.findall(body)
        if not calls and "rt::apply_theme_run(" not in body:
            fallbacks.append(name)
            continue
        if len(calls) != 1 or body.count("rt::apply_theme_run(") != 1:
            raise ValueError(f"{name}: expected one packed schedule")
        size, offset = map(int, calls[0])
        stream = image[offset:offset + size]
        if len(stream) != size:
            raise ValueError(f"{name}: schedule outside image")
        decoded = Counter(instructions(stream))
        counts.update(decoded)
        streams.add((offset, size))
        if per_run:
            details[name] = dict(decoded)
    result = {
        "extracted_runs": len(runs), "scheduled_runs": len(runs) - len(fallbacks),
        "native_fallbacks": fallbacks, "instructions": dict(sorted(counts.items())),
        "distinct_schedule_bytes_in_image": sum(size for _, size in streams),
        "image_sha256": hashlib.sha256(image).hexdigest() if artifact else None,
    }
    if per_run:
        result["by_run"] = details
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("module", nargs="+", type=Path, help="generated out/xaml/compiled_xaml.rs")
    parser.add_argument("--per-run", action="store_true", help="include decoded operation counts for each scheduled function")
    args = parser.parse_args()
    print(json.dumps({str(path): audit(path, args.per_run) for path in args.module}, indent=2))
