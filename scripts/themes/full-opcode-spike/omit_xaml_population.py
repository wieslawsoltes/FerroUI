#!/usr/bin/env python3
"""Rustc wrapper for the ControlCatalog XAML headroom measurement.

Supply a fresh, private CARGO_TARGET_DIR equal to ATTRIBUTION_TARGET. The generated
Rust sources and build products there will be intentionally nonfunctional.
"""
import os
from pathlib import Path
import re
import sys


FUNCTION = re.compile(r"^(?:pub )?fn ((?:build|populate)_\w+_xaml(?:_unit_\d+)?)\(")
MARKER = "    // body omitted for size attribution\n"


def omit_population(path):
    lines = path.read_text().splitlines(keepends=True)
    output = []
    changed = 0
    i = 0
    while i < len(lines):
        header = FUNCTION.match(lines[i])
        if not header:
            output.append(lines[i])
            i += 1
            continue

        end = next(j for j in range(i + 1, len(lines)) if lines[j].rstrip() == "}")
        body = lines[i:end]
        i = end + 1
        signature_end = next(j for j, line in enumerate(body) if line.startswith(") -> "))
        if header[1].startswith("populate_") and any(
            line.startswith("    static PROGRAM: rt::ObjectDocumentProgram =") for line in body
        ):
            output.extend(body[:signature_end + 1])
            output.extend((MARKER, "    ::core::result::Result::Ok(())\n", lines[end]))
            changed += 1
            continue
        if "_unit_" in header[1]:
            keep = signature_end + 1
        else:
            keep = next((j for j, line in enumerate(body)
                         if line.startswith("    let context = rt::populate_context(")), None)
        if MARKER in body or (keep is None and len(body) <= signature_end + 4):
            output.extend(body)
        else:
            if keep is None or not body[-1].startswith("    ::core::result::Result::Ok("):
                raise ValueError(f"Unexpected generated population body in {path}, {header[1]}")
            output.extend(body[:keep])
            output.extend((MARKER, body[-1]))
            changed += 1
        output.append(lines[end])
    if changed:
        path.write_text("".join(output))
    return changed


def main():
    compiler, *args = sys.argv[1:]
    root = Path(os.environ["ATTRIBUTION_TARGET"]).resolve()
    if Path(os.environ["CARGO_TARGET_DIR"]).resolve() != root:
        raise ValueError("ATTRIBUTION_TARGET must equal the private CARGO_TARGET_DIR")
    out = Path(os.environ.get("OUT_DIR", "/nonexistent")).resolve()
    if out.is_relative_to(root) and "--crate-name" in args and (out / "xaml/mod.rs").is_file():
        for path in (out / "xaml").glob("*.rs"):
            count = omit_population(path)
            if count:
                print(f"attribution: omitted {count} in {path}", file=sys.stderr)
    os.execvp(compiler, [compiler, *args])


if __name__ == "__main__":
    main()
