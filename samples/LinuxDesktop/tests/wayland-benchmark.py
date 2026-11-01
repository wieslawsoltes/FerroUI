#!/usr/bin/python3
"""Measure a warm global launcher toggle through an actual compositor capture."""
import json
import os
import re
import statistics
import subprocess
import sys
import time
from pathlib import Path


def main(binary, directory, bus_name):
    if os.environ.get("FERRO_TEST_BENCHMARK") != "1" or os.environ.get("WLR_BACKENDS") != "headless":
        raise RuntimeError("Run through FERRO_TEST_BENCHMARK=1 tests/wayland-session.sh")

    def read_frame():
        raw = subprocess.check_output(["grim", "-s", "0.25", "-t", "ppm", "-o", "HEADLESS-1", "-"], timeout=5)
        magic, dimensions, maxval, rgb = raw.split(b"\n", 3)
        width, height = map(int, dimensions.split())
        if magic != b"P6" or maxval != b"255" or len(rgb) != width * height * 3:
            raise ValueError("Invalid compositor frame")
        # Exclude the clock, panel and output edges. Read the same grid in every frame.
        sample = []
        for y in range(height // 8, 7 * height // 8, 5):
            for x in range(width // 8, 7 * width // 8, 5):
                offset = (y * width + x) * 3
                sample.append(rgb[offset:offset + 3])
        return sample

    def toggle():
        subprocess.run([binary, "--platform=wayland", "--show=applications"], check=True, timeout=8)

    def difference(a, b):
        return max(abs(x - y) for x, y in zip(a, b))

    def until(read_matches):
        end = time.monotonic() + 5
        while time.monotonic() < end:
            pixels = read_frame()
            if read_matches(pixels):
                return pixels
        raise TimeoutError("The compositor did not present the expected launcher frame")

    # Calibrate against the current workspace. In the unified compositor
    # fixture this includes two static native application windows.
    desktop = read_frame()
    toggle()
    previous = until(lambda frame: sum(difference(a, b) > 40 for a, b in zip(desktop, frame)) > len(frame) / 10)

    def stable(frame):
        nonlocal previous
        matching = sum(difference(a, b) < 8 for a, b in zip(previous, frame))
        previous = frame
        return matching > len(frame) * .98

    launcher = until(stable)
    indices = [i for i in range(len(desktop)) if difference(desktop[i], launcher[i]) > 40]

    def matches(frame, reference):
        return sum(difference(frame[i], reference[i]) <= 24 for i in indices) >= len(indices) * .90

    def sample(reference):
        start = time.monotonic_ns()
        toggle()
        until(lambda frame: matches(frame, reference))
        return round((time.monotonic_ns() - start) / 1_000_000, 3)

    sample(desktop)
    sample(launcher)
    sample(desktop)
    opens, closes = [], []
    for _ in range(10):
        opens.append(sample(launcher))
        closes.append(sample(desktop))

    owner = subprocess.check_output(["gdbus", "call", "--session", "--dest", "org.freedesktop.DBus",
        "--object-path", "/org/freedesktop/DBus", "--method", "org.freedesktop.DBus.GetConnectionUnixProcessID",
        bus_name], text=True)
    pid = int(re.search(r"uint32 (\d+)", owner).group(1))

    def cpu_ticks():
        fields = Path(f"/proc/{pid}/stat").read_text().rsplit(") ", 1)[1].split()
        return int(fields[11]) + int(fields[12])  # Linux proc_pid_stat utime + stime.

    start = time.monotonic_ns()
    first = cpu_ticks()
    time.sleep(5)
    delta = cpu_ticks() - first
    elapsed = (time.monotonic_ns() - start) / 1_000_000_000
    hz = os.sysconf("SC_CLK_TCK")
    banner = next(line for line in (directory / "session.log").read_text().splitlines()
                  if line.startswith("Ferro Desktop / "))
    result = {
        "scene_composition": "FerroUI" if "FerroUI compositor ready." in (Path(directory) / "session.log").read_text() else "Sway",
        "shell": banner, "compositor": subprocess.check_output(["sway", "--version"], text=True).strip(),
        "compositor_renderer": os.environ["WLR_RENDERER"], "output": "HEADLESS-1",
        "method": "Warm CLI toggle to a quarter-scale grim frame matching >=90% of discriminating pixels "
                  "within 24/255 per RGB channel. Includes process startup, D-Bus, rendering, compositor "
                  "presentation, screencopy readback and observer time.",
        "warmup_cycles": 2, "sampled_cycles": len(opens), "discriminating_pixels": len(indices),
        "open_ms": {"median": round(statistics.median(opens), 3), "samples": opens},
        "close_ms": {"median": round(statistics.median(closes), 3), "samples": closes},
        "idle_shell": {"pid": pid, "seconds": round(elapsed, 4), "cpu_ticks": delta, "ticks_per_second": hz,
                       "percent_of_one_core": round(100 * delta / hz / elapsed, 3)},
    }
    (directory / "wayland-benchmark.json").write_text(json.dumps(result, indent=2) + "\n")
    print(f"Launcher compositor capture median: open {result['open_ms']['median']} ms, "
          f"close {result['close_ms']['median']} ms. Idle shell: {delta} CPU ticks in {elapsed:.3f}s.")


if __name__ == "__main__":
    main(sys.argv[1], Path(sys.argv[2]), sys.argv[3])
