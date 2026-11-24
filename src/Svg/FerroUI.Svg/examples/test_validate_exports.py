"""Coverage checks for the independent SVG validator; no optional packages needed."""

from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch
import zlib

import validate_exports as validator


def png(path: Path, kind: int, scanline: bytes, *, palette: bytes = b"",
        transparency: bytes = b"") -> None:
    def chunk(name: bytes, data: bytes) -> bytes:
        return (struct.pack(">I", len(data)) + name + data
                + struct.pack(">I", zlib.crc32(name + data)))
    result = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 8, kind, 0, 0, 0))
    if palette:
        result += chunk(b"PLTE", palette)
    if transparency:
        result += chunk(b"tRNS", transparency)
    result += chunk(b"IDAT", zlib.compress(b"\0" + scanline)) + chunk(b"IEND", b"")
    path.write_bytes(result)


class CoverageTests(unittest.TestCase):
    def test_alpha_detects_opacity_loss_when_white_composites_are_identical(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            opaque = Path(directory) / "opaque.png"
            transparent = Path(directory) / "transparent.png"
            png(opaque, 6, bytes((255, 255, 255, 255)))
            png(transparent, 6, bytes((255, 255, 255, 0)))
            # Also exercise the dependency-free branch explicitly even on a
            # development machine with Pillow and NumPy installed.
            with patch.dict("sys.modules", {"PIL": None, "numpy": None}):
                self.assertEqual((0.0, 0.0, 0.0), validator.compare(opaque, transparent, 16))
                self.assertEqual((255.0, 255.0, 100.0), validator.compare_alpha(opaque, transparent, 16))
            self.assertEqual((255.0, 255.0, 100.0), validator.compare_alpha(opaque, transparent, 16))

    def test_transparent_rgb_payloads_do_not_create_coverage_error(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            red, blue = Path(directory) / "red.png", Path(directory) / "blue.png"
            png(red, 6, bytes((255, 0, 0, 0)))
            png(blue, 6, bytes((0, 0, 255, 0)))
            self.assertEqual((0.0, 0.0, 0.0), validator.compare_alpha(red, blue, 16))
            self.assertEqual((0.0, 0.0, 0.0), validator.compare(red, blue, 16))

    def test_stdlib_decoder_retains_alpha_for_supported_color_encodings(self) -> None:
        cases = [
            (6, bytes((10, 20, 30, 64)), b"", b"", bytes((10, 20, 30, 64))),
            (4, bytes((100, 128)), b"", b"", bytes((100, 100, 100, 128))),
            (3, bytes((0,)), bytes((10, 20, 30)), bytes((64,)), bytes((10, 20, 30, 64))),
            (2, bytes((10, 20, 30)), b"", struct.pack(">HHH", 10, 20, 30), bytes((10, 20, 30, 0))),
            (0, bytes((100,)), b"", struct.pack(">H", 100), bytes((100, 100, 100, 0))),
        ]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "color.png"
            for kind, scanline, palette, transparency, expected in cases:
                with self.subTest(kind=kind):
                    png(path, kind, scanline, palette=palette, transparency=transparency)
                    self.assertEqual(((1, 1), expected), validator.read_png_rgba_stdlib(path))


if __name__ == "__main__":
    unittest.main()
