#!/usr/bin/env python3
"""Independently validate and render the SVG backend's generated examples.

Usage:
    python3 validate_exports.py samples/SvgExport/output
    python3 validate_exports.py /tmp/svg --preview-dir /tmp/svg-previews

Install Inkscape, or pass its executable with --renderer. Pillow and NumPy are
optional accelerators: the standard-library PNG reader handles the noninterlaced
8-bit PNGs produced by Inkscape. Add --reference to the Rust example to generate
a Skia replay for comparison; it is intentionally not a committed fixture.
"""

from __future__ import annotations

import argparse
from collections import Counter
from dataclasses import dataclass
import math
from pathlib import Path
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
import zlib


SVG_NS = "http://www.w3.org/2000/svg"
XLINK_NS = "http://www.w3.org/1999/xlink"
LOCAL_URL = re.compile(r"url\(\s*['\"]?#([^'\")\s]+)['\"]?\s*\)")


@dataclass(frozen=True)
class SvgStructure:
    ids: int
    references: int
    vector_layers: int
    raster_layers: int
    images: int


def validate_svg(path: Path) -> SvgStructure:
    """Verify XML, unique identifiers and every local resource reference."""
    root = ET.parse(path).getroot()
    if root.tag != f"{{{SVG_NS}}}svg":
        raise ValueError(f"{path.name}: the document root is not an SVG element")
    if "viewBox" in root.attrib:
        box = [float(value) for value in re.split(r"[\s,]+", root.attrib["viewBox"].strip())]
        if len(box) != 4 or not all(map(math.isfinite, box)) or min(box[2:]) <= 0:
            raise ValueError(f"{path.name}: invalid viewBox")

    ids = Counter(node.attrib["id"] for node in root.iter() if "id" in node.attrib)
    duplicate_ids = sorted(identifier for identifier, count in ids.items() if count > 1)
    if duplicate_ids or "" in ids:
        raise ValueError(f"{path.name}: empty or duplicate IDs: {duplicate_ids}")
    references: set[str] = set()
    layer_kinds: Counter[str] = Counter()
    images = 0
    for node in root.iter():
        if "data-rendering" in node.attrib:
            layer_kinds[node.attrib["data-rendering"]] += 1
        images += node.tag == f"{{{SVG_NS}}}image"
        for name, value in node.attrib.items():
            references.update(LOCAL_URL.findall(value))
            if name in ("href", f"{{{XLINK_NS}}}href") and value.startswith("#"):
                references.add(value[1:])
            if name in ("aria-labelledby", "aria-describedby"):
                references.update(value.split())
    missing = sorted(references - ids.keys())
    if missing:
        raise ValueError(f"{path.name}: unresolved local references: {missing}")
    return SvgStructure(len(ids), len(references), layer_kinds["vector"], layer_kinds["raster"], images)


def render(renderer: str, source: Path, destination: Path) -> None:
    result = subprocess.run(
        [renderer, str(source.resolve()), "--export-area-page", "--export-dpi=96",
         "--export-type=png", f"--export-filename={destination.resolve()}"],
        capture_output=True, text=True, check=False, timeout=120,
    )
    if result.returncode or not destination.is_file():
        detail = (result.stderr or result.stdout).strip()[-3000:]
        raise RuntimeError(f"Inkscape could not render {source.name}: {detail}")


def paeth(left: int, up: int, upper_left: int) -> int:
    estimate = left + up - upper_left
    dl, du, dul = abs(estimate - left), abs(estimate - up), abs(estimate - upper_left)
    if dl <= du and dl <= dul:
        return left
    return up if du <= dul else upper_left


def read_png_rgba_stdlib(path: Path) -> tuple[tuple[int, int], bytes]:
    """Decode noninterlaced 8-bit PNG to straight RGBA, without extras."""
    data = path.read_bytes()
    if not data.startswith(b"\x89PNG\r\n\x1a\n"):
        raise ValueError(f"{path.name}: not a PNG image")
    cursor, payload = 8, bytearray()
    palette, transparency = b"", b""
    header = None
    while cursor < len(data):
        if cursor + 12 > len(data):
            raise ValueError(f"{path.name}: truncated PNG chunk")
        length = struct.unpack_from(">I", data, cursor)[0]
        kind = data[cursor + 4:cursor + 8]
        end = cursor + 8 + length
        if end + 4 > len(data):
            raise ValueError(f"{path.name}: truncated PNG payload")
        chunk = data[cursor + 8:end]
        if zlib.crc32(kind + chunk) != struct.unpack_from(">I", data, end)[0]:
            raise ValueError(f"{path.name}: invalid PNG checksum")
        if kind == b"IHDR":
            header = struct.unpack(">IIBBBBB", chunk)
        elif kind == b"IDAT":
            payload.extend(chunk)
        elif kind == b"PLTE":
            palette = chunk
        elif kind == b"tRNS":
            transparency = chunk
        cursor = end + 4
        if kind == b"IEND":
            break
    if header is None:
        raise ValueError(f"{path.name}: PNG header is missing")
    width, height, depth, kind, compression, filtering, interlace = header
    channels = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}.get(kind)
    if depth != 8 or interlace or compression or filtering or channels is None:
        raise ValueError(f"{path.name}: install Pillow for this PNG encoding")
    if not width or not height or width * height > 256_000_000:
        raise ValueError(f"{path.name}: PNG dimensions exceed the validation budget")
    stride = width * channels
    rows = zlib.decompress(payload)
    if len(rows) != (stride + 1) * height:
        raise ValueError(f"{path.name}: incorrect PNG scanline length")
    rgba = bytearray(width * height * 4)
    previous = bytearray(stride)
    for y in range(height):
        start = y * (stride + 1)
        filter_type = rows[start]
        row = bytearray(rows[start + 1:start + stride + 1])
        if filter_type not in range(5):
            raise ValueError(f"{path.name}: unsupported PNG filter {filter_type}")
        if filter_type:
            for i in range(stride):
                left = row[i - channels] if i >= channels else 0
                up = previous[i]
                upper_left = previous[i - channels] if i >= channels else 0
                predictor = (left if filter_type == 1 else up if filter_type == 2 else
                             (left + up) // 2 if filter_type == 3 else paeth(left, up, upper_left))
                row[i] = (row[i] + predictor) & 255
        for x in range(width):
            offset = x * channels
            alpha = 255
            if kind in (2, 6):
                r, g, b = row[offset:offset + 3]
                if kind == 6:
                    alpha = row[offset + 3]
                elif transparency and (r, g, b) == struct.unpack(">HHH", transparency):
                    alpha = 0
            elif kind in (0, 4):
                r = g = b = row[offset]
                if kind == 4:
                    alpha = row[offset + 1]
                elif transparency and r == struct.unpack(">H", transparency)[0]:
                    alpha = 0
            else:
                index = row[offset]
                if index * 3 + 3 > len(palette):
                    raise ValueError(f"{path.name}: invalid PNG palette index")
                r, g, b = palette[index * 3:index * 3 + 3]
                if index < len(transparency):
                    alpha = transparency[index]
            target = (y * width + x) * 4
            rgba[target:target + 4] = bytes((r, g, b, alpha))
        previous = row
    return (width, height), bytes(rgba)


def read_png_stdlib(path: Path) -> tuple[tuple[int, int], bytes]:
    """Composite the dependency-free PNG decoder's RGBA result on white."""
    size, rgba = read_png_rgba_stdlib(path)
    rgb = bytearray(size[0] * size[1] * 3)
    for source in range(0, len(rgba), 4):
        alpha = rgba[source + 3]
        target = source // 4 * 3
        rgb[target:target + 3] = bytes(
            (value * alpha + 255 * (255 - alpha) + 127) // 255
            for value in rgba[source:source + 3]
        )
    return size, bytes(rgb)


def read_rgba(path: Path) -> tuple[tuple[int, int], bytes]:
    try:
        from PIL import Image
    except ImportError:
        return read_png_rgba_stdlib(path)
    with Image.open(path) as image:
        return image.size, image.convert("RGBA").tobytes()


def read_rgb(path: Path) -> tuple[tuple[int, int], bytes]:
    try:
        from PIL import Image
    except ImportError:
        return read_png_stdlib(path)
    with Image.open(path) as image:
        rgba = image.convert("RGBA")
        white = Image.new("RGBA", rgba.size, "white")
        white.alpha_composite(rgba)
        return rgba.size, white.convert("RGB").tobytes()


def compare(actual: Path, reference: Path, threshold: int) -> tuple[float, float, float]:
    size, actual_rgb = read_rgb(actual)
    reference_size, reference_rgb = read_rgb(reference)
    if size != reference_size:
        raise ValueError(f"Different image dimensions: {size} versus {reference_size}")
    pixels = size[0] * size[1]
    try:
        import numpy as np
    except ImportError:
        total = squares = changed = 0
        for offset in range(0, len(actual_rgb), 3):
            difference = [abs(actual_rgb[offset + c] - reference_rgb[offset + c]) for c in range(3)]
            total += sum(difference)
            squares += sum(value * value for value in difference)
            changed += max(difference) > threshold
        return total / (pixels * 3), math.sqrt(squares / (pixels * 3)), changed * 100 / pixels
    difference = np.abs(np.frombuffer(actual_rgb, dtype=np.uint8).astype(np.int16)
                        - np.frombuffer(reference_rgb, dtype=np.uint8).astype(np.int16))
    mean = float(difference.mean())
    rmse = float(np.sqrt(np.square(difference.astype(np.float64)).mean()))
    changed = float((difference.reshape(-1, 3).max(axis=1) > threshold).mean() * 100)
    return mean, rmse, changed


def compare_alpha(actual: Path, reference: Path, threshold: int) -> tuple[float, float, float]:
    """Compare coverage directly; transparent RGB payloads are deliberately ignored."""
    size, actual_rgba = read_rgba(actual)
    reference_size, reference_rgba = read_rgba(reference)
    if size != reference_size:
        raise ValueError(f"Different image dimensions: {size} versus {reference_size}")
    actual_alpha, reference_alpha = actual_rgba[3::4], reference_rgba[3::4]
    pixels = size[0] * size[1]
    try:
        import numpy as np
    except ImportError:
        total = squares = changed = 0
        for actual_value, reference_value in zip(actual_alpha, reference_alpha):
            difference = abs(actual_value - reference_value)
            total += difference
            squares += difference * difference
            changed += difference > threshold
        return total / pixels, math.sqrt(squares / pixels), changed * 100 / pixels
    difference = np.abs(np.frombuffer(actual_alpha, dtype=np.uint8).astype(np.int16)
                        - np.frombuffer(reference_alpha, dtype=np.uint8).astype(np.int16))
    return (float(difference.mean()),
            float(np.sqrt(np.square(difference.astype(np.float64)).mean())),
            float((difference > threshold).mean() * 100))


def validate_directory(args: argparse.Namespace, previews: Path, renderer: str) -> None:
    paths = sorted(args.directory.glob("*.svg"))
    if not paths:
        raise ValueError(f"No SVG files in {args.directory}")
    previews.mkdir(parents=True, exist_ok=True)
    for path in paths:
        structure = validate_svg(path)
        render(renderer, path, previews / f"{path.stem}.png")
        print(f"{path.name}: {path.stat().st_size:,} bytes; {structure.ids} IDs, "
              f"{structure.references} resolved references; {structure.vector_layers} vector / "
              f"{structure.raster_layers} raster layers; {structure.images} images; render OK")

    compared = 0
    for reference in paths:
        if not reference.name.endswith("-raster-reference.svg"):
            continue
        prefix = reference.name.removesuffix("-raster-reference.svg")
        normal = args.directory / f"{prefix}-normal.svg"
        if not normal.is_file():
            raise ValueError(f"{reference.name}: matching {normal.name} is missing")
        mean, rmse, changed = compare(previews / f"{normal.stem}.png",
                                      previews / f"{reference.stem}.png", args.channel_threshold)
        alpha_mean, alpha_rmse, alpha_changed = compare_alpha(
            previews / f"{normal.stem}.png", previews / f"{reference.stem}.png", args.channel_threshold)
        print(f"{prefix} vs Skia replay: mean absolute RGB error {mean:.4f}/255, "
              f"RMSE {rmse:.4f}/255, pixels with any channel >{args.channel_threshold}: {changed:.4f}%")
        print(f"{prefix} alpha vs Skia replay: mean absolute error {alpha_mean:.4f}/255, "
              f"RMSE {alpha_rmse:.4f}/255, pixels >{args.channel_threshold}: {alpha_changed:.4f}%")
        if args.max_mean_error is not None and mean > args.max_mean_error:
            raise ValueError(f"Mean RGB error {mean:.4f} exceeds {args.max_mean_error}")
        if args.max_alpha_error is not None and alpha_mean > args.max_alpha_error:
            raise ValueError(f"Mean alpha error {alpha_mean:.4f} exceeds {args.max_alpha_error}")
        compared += 1
    print(f"Validated {len(paths)} SVGs; {compared} pixel comparison(s).")
    if not compared:
        print("Use --reference on the Rust example to include a Skia raster comparison.")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("directory", type=Path, help="directory containing exported SVG files")
    parser.add_argument("--renderer", default="inkscape", help="Inkscape executable (default: inkscape on PATH)")
    parser.add_argument("--preview-dir", type=Path, help="keep PNG previews here; otherwise use temporary files")
    parser.add_argument("--channel-threshold", type=int, default=16, help="per-channel difference for changed-pixel count (0..255)")
    parser.add_argument("--max-mean-error", type=float, help="optional mean RGB error gate, in 0..255 channel values")
    parser.add_argument("--max-alpha-error", type=float, help="optional mean alpha error gate, in 0..255 channel values")
    args = parser.parse_args()
    if not 0 <= args.channel_threshold <= 255:
        parser.error("--channel-threshold must be in 0..255")
    if args.max_mean_error is not None and (not math.isfinite(args.max_mean_error) or args.max_mean_error < 0):
        parser.error("--max-mean-error must be finite and nonnegative")
    if args.max_alpha_error is not None and (not math.isfinite(args.max_alpha_error) or args.max_alpha_error < 0):
        parser.error("--max-alpha-error must be finite and nonnegative")
    renderer = shutil.which(args.renderer)
    if renderer is None:
        parser.error("Inkscape was not found; install it or provide --renderer /path/to/inkscape")
    try:
        version = subprocess.run([renderer, "--version"], capture_output=True, text=True, check=True, timeout=30)
        print(version.stdout.strip().splitlines()[0])
        if args.preview_dir is not None:
            validate_directory(args, args.preview_dir, renderer)
        else:
            with tempfile.TemporaryDirectory(prefix="ferroui-svg-validation-") as directory:
                validate_directory(args, Path(directory), renderer)
    except (OSError, ValueError, RuntimeError, ET.ParseError, struct.error, zlib.error, subprocess.SubprocessError) as error:
        print(f"Validation failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
