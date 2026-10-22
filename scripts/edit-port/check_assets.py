#!/usr/bin/env python3
"""Verify committed editor resources against the captured source hashes."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def main():
    manifest = json.loads((ROOT / "docs/porting/data/editor-assets.json").read_text())
    checked = set()
    errors = []
    for item in manifest["files"]:
        path = ROOT / item["path"]
        checked.add(item["path"])
        if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != item["sha256"]:
            errors.append(f"Changed or missing pinned resource: {item['path']}")
    for directory in ("src/FerroUI.TextMate/resources/grammars", "src/FerroUI.TextMate/resources/themes", "src/FerroUI.Edit.Core/highlighting/resources"):
        for path in (ROOT / directory).rglob("*"):
            if path.is_file() and path.name != "NOTICE.md" and path.relative_to(ROOT).as_posix() not in checked:
                errors.append(f"Resource lacks source provenance: {path.relative_to(ROOT)}")
    crate = ROOT / "src/FerroUI.TextMate"
    versions = json.loads((crate / "resources/versions.json").read_text())
    for path, expected in versions["browser"]["files"].items():
        file = crate / path
        if not file.is_file() or "sha256:" + hashlib.sha256(file.read_bytes()).hexdigest() != expected:
            errors.append(f"Changed or missing pinned browser scanner: {path}")
    if errors:
        raise SystemExit("\n".join(errors))
    print(f"{len(checked)} pinned grammar/theme/XSHD resources and four browser scanner files verified")


if __name__ == "__main__":
    main()
