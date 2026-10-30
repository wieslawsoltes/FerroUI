#!/usr/bin/env python3
"""Verify the isolated dependency against the published package and declared patch."""
from __future__ import annotations
import hashlib
import json
from pathlib import Path
import sys

def verify(root: Path, package_name: str = "ferroui-base") -> list[str]:
    vendor = root / "vendor"
    manifest = json.loads((vendor / f"{package_name}-provenance.json").read_text())
    package = vendor / package_name
    expected = dict(manifest["original_sha256"])
    expected.update(manifest["modified_sha256"])
    errors: list[str] = []
    if set(manifest["modified_files"]) != set(manifest["modified_sha256"]):
        errors.append("modified file list and hashes differ")
    actual = {str(p.relative_to(package)) for p in package.rglob("*") if p.is_file()}
    for name in sorted(actual - expected.keys()):
        errors.append(f"undeclared dependency file: {name}")
    for name, digest in expected.items():
        path = package / name
        if not path.is_file():
            errors.append(f"missing dependency file: {name}")
        elif hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            errors.append(f"dependency file differs from its recorded source: {name}")
    patch = vendor / manifest.get("patch_file", "ferroui-base-resource-api.patch")
    if hashlib.sha256(patch.read_bytes()).hexdigest() != manifest["patch_sha256"]:
        errors.append("dependency patch differs from its declared hash")
    return errors

def main() -> int:
    root = Path(__file__).resolve().parents[2]
    errors = verify(root) + verify(root, "ferroui-markup-xaml") + verify(root, "ferroui-markup-xaml-loader") + verify(root, "ferroui-build") + verify(root, "ferroui-build-scan") + verify(root, "ferroui-controls") + verify(root, "ferroui-native")
    for error in errors:
        print(f"ERROR: {error}", file=sys.stderr)
    if not errors:
        print("All seven isolated FerroUI 0.3.0 packages match their published sources and declared resource/cast/weak-interface restoration, instance-check, object namescope, public constructor value-path, object-setter normalization and checked boxed-cast scanner, native nullable item boxing and explicit typed Int32 enum factory and borrowed native storage URI ownership and released-control accessibility callback patches.")
    return int(bool(errors))

if __name__ == "__main__":
    raise SystemExit(main())
