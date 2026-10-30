#!/usr/bin/env python3
"""Validate native capture evidence; never infer pixel/motion equivalence.

Inventory checks file integrity and metric shape. State coverage additionally
requires light/dark, active/inactive observations. Optics prerequisites additionally
require WindowServer captures and the unreduced appearance configuration.
All checks are offline, deterministic and standard-library only.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path, PurePosixPath
import re
import struct
import sys
import tempfile
import os
import zlib
from typing import Any

MAX_MANIFEST_BYTES = 8 * 1024 * 1024
MAX_IMAGE_BYTES = 64 * 1024 * 1024
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
SCOPES = ("inventory", "state-coverage", "optics-prerequisites")


class EvidenceError(ValueError):
    pass


def finite(value: Any, name: str, *, positive: bool = False) -> float:
    try:
        valid = not isinstance(value, bool) and isinstance(value, (int, float)) and math.isfinite(value)
    except OverflowError:
        valid = False
    if not valid:
        raise EvidenceError(f"{name} must be a finite number")
    if positive and value <= 0:
        raise EvidenceError(f"{name} must be positive")
    return float(value)


def boolean(value: Any, name: str) -> bool:
    if type(value) is not bool:
        raise EvidenceError(f"{name} must be a Boolean")
    return value


def asset_path(root: Path, name: Any) -> Path:
    if not isinstance(name, str) or not name or "\\" in name or "\x00" in name:
        raise EvidenceError("asset must be a nonempty relative POSIX path")
    relative = PurePosixPath(name)
    if relative.is_absolute() or ".." in relative.parts or re.match(r"^[A-Za-z]:", name):
        raise EvidenceError(f"unsafe asset path: {name!r}")
    path = root
    for segment in relative.parts:
        path = path / segment
        if path.is_symlink():
            raise EvidenceError(f"symlink assets are not accepted: {name!r}")
    if not path.is_file():
        raise EvidenceError(f"missing asset: {name}")
    if not path.resolve().is_relative_to(root.resolve()):
        raise EvidenceError(f"asset escapes capture directory: {name!r}")
    return path


def png_info(path: Path) -> dict[str, Any]:
    size = path.stat().st_size
    if not 45 <= size <= MAX_IMAGE_BYTES:
        raise EvidenceError(f"PNG size out of bounds: {path.name}")
    data = path.read_bytes()
    if not data.startswith(PNG_SIGNATURE):
        raise EvidenceError(f"not a PNG: {path.name}")
    offset, dimensions, idat, ended = 8, None, False, False
    chunk_count = 0
    while offset < len(data):
        chunk_count += 1
        if chunk_count > 100_000 or offset + 12 > len(data):
            raise EvidenceError(f"malformed PNG chunks: {path.name}")
        length, kind = struct.unpack_from(">I4s", data, offset)
        end = offset + 12 + length
        if end > len(data):
            raise EvidenceError(f"truncated PNG: {path.name}")
        payload = data[offset + 8:offset + 8 + length]
        expected = struct.unpack_from(">I", data, offset + 8 + length)[0]
        if zlib.crc32(kind + payload) & 0xffffffff != expected:
            raise EvidenceError(f"PNG CRC mismatch: {path.name}")
        if chunk_count == 1 and kind != b"IHDR":
            raise EvidenceError(f"PNG must begin with IHDR: {path.name}")
        if kind == b"IHDR":
            if dimensions is not None or length != 13:
                raise EvidenceError(f"invalid PNG IHDR: {path.name}")
            width, height, depth, color, compression, filtering, interlace = struct.unpack(">IIBBBBB", payload)
            legal_depths = {0: (1, 2, 4, 8, 16), 2: (8, 16), 3: (1, 2, 4, 8), 4: (8, 16), 6: (8, 16)}
            if not (0 < width <= 32768 and 0 < height <= 32768 and width * height <= 268435456):
                raise EvidenceError(f"invalid PNG dimensions: {path.name}")
            if depth not in legal_depths.get(color, ()) or compression != 0 or filtering != 0 or interlace not in (0, 1):
                raise EvidenceError(f"unsupported PNG header: {path.name}")
            dimensions = (width, height)
        elif kind == b"IDAT":
            idat = idat or length > 0
        elif kind == b"IEND":
            if length != 0 or end != len(data):
                raise EvidenceError(f"invalid PNG ending: {path.name}")
            ended = True
            break
        offset = end
    if not (dimensions and idat and ended):
        raise EvidenceError(f"incomplete PNG: {path.name}")
    # This verifies PNG framing/checksums, not image-decoder semantics or fidelity.
    return {"sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data),
            "widthPixels": dimensions[0], "heightPixels": dimensions[1]}


def object_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise EvidenceError(f"duplicate JSON member: {key}")
        result[key] = value
    return result


def validate(manifest_path: Path, scope: str = "state-coverage") -> dict[str, Any]:
    if scope not in SCOPES:
        raise ValueError(f"unknown scope: {scope}")
    findings: list[dict[str, Any]] = []
    images: list[dict[str, Any]] = []
    observed: set[tuple[str, bool]] = set()
    seen_images: set[str] = set()
    controls_by_capture: list[set[tuple[str, str]]] = []
    manifest: dict[str, Any] = {}
    data = b""

    def issue(code: str, detail: str, *, required: str = "inventory", capture: int | None = None) -> None:
        severity = "error" if SCOPES.index(scope) >= SCOPES.index(required) else "warning"
        entry: dict[str, Any] = {"severity": severity, "code": code, "detail": detail}
        if capture is not None:
            entry["capture"] = capture
        findings.append(entry)

    try:
        if manifest_path.stat().st_size > MAX_MANIFEST_BYTES:
            raise EvidenceError("manifest exceeds size limit")
        data = manifest_path.read_bytes()
        manifest = json.loads(data, object_pairs_hook=object_pairs,
                              parse_constant=lambda v: (_ for _ in ()).throw(EvidenceError(f"invalid JSON number: {v}")))
        if not isinstance(manifest, dict):
            raise EvidenceError("manifest must be an object")
        schema = manifest.get("schemaVersion")
        if type(schema) is not int or schema not in (1, 2):
            raise EvidenceError("only manifest schema 1 or 2 is supported")
        if not re.fullmatch(r"26\.\d+\.\d+", str(manifest.get("osVersion", ""))):
            raise EvidenceError("reference must declare an exact macOS 26.x.y version")
        for key in ("reduceMotion", "reduceTransparency", "increaseContrast"):
            value = boolean(manifest.get(key), key)
            if value:
                issue("nondefault_accessibility", f"{key}=true: keep this as a separate accessibility reference",
                      required="optics-prerequisites")
        captures = manifest.get("captures")
        if not isinstance(captures, list) or not 1 <= len(captures) <= 100:
            raise EvidenceError("captures must contain between 1 and 100 entries")
        for index, capture in enumerate(captures):
            try:
                if not isinstance(capture, dict):
                    raise EvidenceError("capture must be an object")
                appearance = capture.get("appearance")
                if appearance not in ("light", "dark"):
                    raise EvidenceError("appearance must be light or dark")
                active = boolean(capture.get("activeWindow"), "activeWindow")
                finite(capture.get("scale"), "scale", positive=True)
                key = (appearance, active)
                if key in observed:
                    issue("duplicate_observed_state", f"duplicate {appearance}/{'active' if active else 'inactive'} observation",
                          required="state-coverage", capture=index)
                observed.add(key)
                view_name = capture.get("viewImage")
                if schema == 2:
                    requested = boolean(capture.get("requestedActiveWindow"), "requestedActiveWindow")
                    app_active = boolean(capture.get("applicationActive"), "applicationActive")
                    main = boolean(capture.get("mainWindow"), "mainWindow")
                    if active and (not app_active or not main):
                        issue("incoherent_activation", "active evidence requires application-active, main and key window",
                              required="state-coverage", capture=index)
                else:
                    # Legacy filenames contain the requested state, not necessarily
                    # the observed state. Never promote them to active evidence.
                    match = re.search(r"-(active|inactive)-view\.png$", str(view_name))
                    requested = (match.group(1) == "active") if match else active
                    issue("legacy_activation_metadata", "schema 1 lacks app-active/main-window observations",
                          required="optics-prerequisites", capture=index)
                if requested != active:
                    issue("activation_mismatch", f"requested active={requested}, observed active={active}",
                          required="state-coverage", capture=index)
                for role in ("viewImage", "windowServerImage"):
                    name = capture.get(role)
                    if name is None and role == "windowServerImage":
                        issue("missing_windowserver_image", "view cache images do not establish compositor output",
                              required="optics-prerequisites", capture=index)
                        continue
                    path = asset_path(manifest_path.parent, name)
                    if name in seen_images:
                        raise EvidenceError(f"image reused across captures/roles: {name}")
                    seen_images.add(name)
                    images.append({"capture": index, "role": role, "path": name, **png_info(path)})
                surface = capture.get("glassSurface")
                if surface is None:
                    issue("missing_glass_geometry", "material reference has no measured visible surface",
                          required="optics-prerequisites", capture=index)
                else:
                    if not isinstance(surface, dict) or not isinstance(surface.get("className"), str):
                        raise EvidenceError("glassSurface must identify the measured class")
                    bounds = surface.get("boundsInContent")
                    if not isinstance(bounds, list) or len(bounds) != 4:
                        raise EvidenceError("glassSurface bounds require x, y, width and height")
                    for dimension, value in enumerate(bounds):
                        finite(value, "glass bounds", positive=dimension >= 2)
                    if not boolean(surface.get("fullyVisible"), "glassSurface.fullyVisible"):
                        issue("clipped_glass_surface", "material reference is not fully visible",
                              required="optics-prerequisites", capture=index)
                colors = capture.get("colors")
                if not isinstance(colors, dict) or not colors:
                    raise EvidenceError("semantic colors are missing")
                for name, color in colors.items():
                    if not isinstance(color, dict):
                        raise EvidenceError(f"color {name} must be an object")
                    for channel in ("red", "green", "blue", "alpha"):
                        value = finite(color.get(channel), f"{name}.{channel}")
                        # Extended sRGB components may exceed [0,1]; alpha may not.
                        if channel == "alpha" and not 0 <= value <= 1:
                            raise EvidenceError(f"{name}.alpha is outside [0,1]")
                controls = capture.get("controls")
                if not isinstance(controls, list) or not 1 <= len(controls) <= 4096:
                    raise EvidenceError("controls inventory is missing or too large")
                ids: set[tuple[str, str]] = set()
                for control in controls:
                    if not isinstance(control, dict):
                        raise EvidenceError("control metric must be an object")
                    identity = (control.get("identifier"), control.get("size"))
                    if not all(isinstance(value, str) and value for value in identity) or identity in ids:
                        raise EvidenceError("empty/duplicate control identity")
                    ids.add(identity)
                    boolean(control.get("enabled"), "control.enabled")
                    for dimension in ("width", "height"):
                        finite(control.get(dimension), f"control.{dimension}", positive=True)
                    # -1 is AppKit's valid no-intrinsic-metric sentinel.
                    for dimension in ("intrinsicWidth", "intrinsicHeight", "fittingWidth", "fittingHeight"):
                        finite(control.get(dimension), f"control.{dimension}")
                    for metric in ("fontSize", "ascender", "descender", "leading"):
                        if control.get(metric) is not None:
                            finite(control[metric], f"control.{metric}")
                    insets = control.get("alignmentInsets")
                    if not isinstance(insets, list) or len(insets) != 4:
                        raise EvidenceError("alignmentInsets requires four values")
                    for inset in insets:
                        finite(inset, "alignment inset")
                controls_by_capture.append(ids)
            except (EvidenceError, OSError, TypeError, KeyError) as error:
                issue("invalid_capture", str(error), capture=index)
        for appearance in ("light", "dark"):
            for active in (True, False):
                if (appearance, active) not in observed:
                    issue("missing_observed_state", f"missing {appearance}/{'active' if active else 'inactive'}",
                          required="state-coverage")
        if controls_by_capture and any(ids != controls_by_capture[0] for ids in controls_by_capture[1:]):
            issue("inconsistent_control_inventory", "capture control identities differ", required="state-coverage")
    except (EvidenceError, OSError, ValueError, TypeError) as error:
        issue("invalid_manifest", str(error))
    return {
        "schemaVersion": 1,
        "scope": scope,
        "status": "fail" if any(f["severity"] == "error" for f in findings) else "pass",
        "nativeFidelity": "unverified",
        "meaning": "Evidence prerequisites only; no image similarity, animation trajectory or native equivalence assertion.",
        "manifestSha256": hashlib.sha256(data).hexdigest() if data else None,
        "osVersion": manifest.get("osVersion") if isinstance(manifest, dict) else None,
        "observedStates": [{"appearance": a, "activeWindow": s} for a, s in sorted(observed)],
        "images": images,
        "findings": findings,
    }


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    text = json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n"
    handle, temporary = tempfile.mkstemp(prefix=path.name + ".", dir=path.parent)
    try:
        with os.fdopen(handle, "w", encoding="utf-8") as output:
            output.write(text)
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("--scope", choices=SCOPES, default="state-coverage")
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    if args.report and args.report.resolve() == args.manifest.resolve():
        parser.error("report must not overwrite the input manifest")
    report = validate(args.manifest, args.scope)
    if args.report:
        write_json(args.report, report)
    print(json.dumps(report, indent=2, sort_keys=True, allow_nan=False))
    return 0 if report["status"] == "pass" else 1


if __name__ == "__main__":
    sys.exit(main())
