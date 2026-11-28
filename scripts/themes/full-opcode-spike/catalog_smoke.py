"""Compare desktop Catalog frames on macOS (sips) or Windows (Pillow)."""
from pathlib import Path
import os
import sys
import subprocess
import struct
import tempfile

def pixels(path, temporary):
    if sys.platform == "win32":
        from PIL import Image
        with Image.open(path) as frame:
            return frame.width, frame.height, memoryview(frame.convert("RGBA").tobytes()).cast("I")
    subprocess.run(["sips", "-s", "format", "bmp", path, "--out", temporary],
                   check=True, stdout=subprocess.DEVNULL)
    raw = temporary.read_bytes()
    offset = struct.unpack_from("<I", raw, 10)[0]
    width, height, planes, bits = struct.unpack_from("<iiHH", raw, 18)
    assert planes == 1 and bits == 32 and len(raw) - offset == width * abs(height) * 4
    return width, height, memoryview(raw)[offset:].cast("I")

def compare(left, right):
    with tempfile.TemporaryDirectory() as directory:
        tmp = Path(directory) / "frame.bmp"
        width, height, a = pixels(left, tmp)
        other_width, other_height, b = pixels(right, tmp)
        assert (width, height) == (other_width, other_height)
        different = sum(x != y for x, y in zip(a, b))
        return width, abs(height), different

out = Path(os.environ.get("SPIKE_CATALOG_DIR", Path.cwd() / "target/full-opcode-spike/catalog"))
pages = os.environ.get("SPIKE_SCREENSHOT_PAGES", "Buttons,ListBox,Calendar").split(",")
mismatches = 0
for mode in sys.argv[1:] or ["native", "ast"]:
    for variant in ["light", "dark", "highcontrast"]:
        images = out / f"{mode}-{variant}{os.environ.get('SPIKE_CAPTURE_SUFFIX', '')}"
        images.mkdir(exist_ok=True)
        env = dict(os.environ, FERROUI_CATALOG_THEME_VARIANT=variant, FERROUI_SMOKE_WINDOW_SIZE="950x680",
                   FERROUI_SMOKE_EXTEND_CLIENT_AREA="0", FERROUI_SMOKE_WINDOW_POSITION="20,40", FERROUI_SMOKE_SCREENSHOTS=str(images),
                   FERROUI_SMOKE_SCREENSHOT_PAGES=",".join(pages), FERROUI_SMOKE_PAGES="1800")
        with (images / "run.log").open("w") as log:
            subprocess.run([out / mode], stdout=log, stderr=log, env=env, check=True, timeout=75)
        frames = sorted(path for path in images.glob("*.png") if not path.stem.endswith(("-window", "-composed")))
        assert len(frames) == len(pages), frames
        if mode != "native":
            for path in frames:
                width, height, different = compare(out / f"native-{variant}{os.environ.get('SPIKE_CAPTURE_SUFFIX', '')}" / path.name, path)
                print(variant, path.name, f"{width} x {height} pixels, {different} differ", flush=True)
                mismatches += int(different != 0)

if mismatches:
    raise SystemExit(f"{mismatches} screenshot pairs differ")
