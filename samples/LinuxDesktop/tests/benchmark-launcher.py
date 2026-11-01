#!/usr/bin/env python3
"""Measure warm Start activation and idle CPU on a private X server.

Usage: python3 samples/LinuxDesktop/tests/benchmark-launcher.py target/debug/ferro-desktop
Requires Xvfb, xauth, dbus-run-session, Openbox, xdotool, libX11 and libXtst.
"""
import ctypes as C
import ctypes.util
import json
import os
from pathlib import Path
import statistics
import subprocess
import sys
import tempfile
import time
from contextlib import ExitStack, contextmanager


def symbol(lib, name, result, *arguments):
    function = getattr(lib, name)
    function.restype = result
    function.argtypes = arguments
    return function


class Probe:
    """Inject real X keyboard input and read a pixel from the mapped shell window."""

    def __init__(self, window):
        x = C.CDLL(ctypes.util.find_library("X11"))
        xt = C.CDLL(ctypes.util.find_library("Xtst"))
        ptr, integer, ulong = C.c_void_p, C.c_int, C.c_ulong
        self.display = symbol(x, "XOpenDisplay", ptr, C.c_char_p)(None)
        if not self.display:
            raise RuntimeError("Could not open the benchmark display")
        self.window = window
        self.image = symbol(x, "XGetImage", ptr, ptr, ulong, integer, integer, C.c_uint, C.c_uint, ulong, integer)
        self.pixel = symbol(x, "XGetPixel", ulong, ptr, integer, integer)
        self.destroy = symbol(x, "XDestroyImage", integer, ptr)
        self.flush = symbol(x, "XFlush", integer, ptr)
        self.disconnect = symbol(x, "XCloseDisplay", integer, ptr)
        keysym = symbol(x, "XStringToKeysym", ulong, C.c_char_p)
        keycode = symbol(x, "XKeysymToKeycode", C.c_uint8, ptr, ulong)
        self.inject = symbol(xt, "XTestFakeKeyEvent", integer, ptr, C.c_uint, integer, ulong)
        self.open_keys = [keycode(self.display, keysym(k)) for k in (b"Control_L", b"k")]
        self.close_keys = [keycode(self.display, keysym(b"Escape"))]

    def rgb(self):
        # On the 1280 x 800 Windows layout, this coordinate lies inside a Start
        # tile when open, and over the wallpaper when closed.
        image = self.image(self.display, self.window, 643, 683, 1, 1, C.c_ulong(-1), 2)
        if not image:
            raise RuntimeError("Shell window is not readable")
        try:
            pixel = self.pixel(image, 0, 0)
            return tuple((pixel >> offset) & 255 for offset in (16, 8, 0))
        finally:
            self.destroy(image)

    def transition(self, keys, previous):
        started = time.monotonic()
        for key in keys:
            self.inject(self.display, key, 1, 0)
        for key in reversed(keys):
            self.inject(self.display, key, 0, 0)
        self.flush(self.display)
        while time.monotonic() - started < 3:
            if max(abs(a - b) for a, b in zip(previous, self.rgb())) > 68:
                return (time.monotonic() - started) * 1000
            time.sleep(.002)
        raise RuntimeError(f"Launcher failed to change the sampled pixel from {previous}")


@contextmanager
def process(args):
    child = subprocess.Popen(args, stdout=subprocess.DEVNULL, stderr=sys.stderr)
    try:
        yield child
    finally:
        child.terminate()
        try:
            child.wait(3)
        except subprocess.TimeoutExpired:
            child.kill()
            child.wait()


def cpu_ticks(pid):
    # Fields 14 and 15 of /proc/<pid>/stat. The preceding command name can
    # include whitespace or parentheses.
    values = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
    return int(values[11]) + int(values[12])


def isolated_run(binary):
    def xdo(*args):
        return subprocess.check_output(["xdotool", *args], timeout=25, text=True).strip()

    with tempfile.TemporaryDirectory(prefix="ferro-launcher-bench-") as tmp, ExitStack() as stack:
        base = Path(tmp)
        for key, subdir in (("XDG_CONFIG_HOME", "config"), ("XDG_DATA_HOME", "data"),
                            ("XDG_CACHE_HOME", "cache"), ("XDG_RUNTIME_DIR", "runtime")):
            path = base / subdir
            path.mkdir(mode=0o700)
            os.environ[key] = str(path)
        apps = base / "data" / "applications"
        apps.mkdir()
        for index in range(6):
            (apps / f"ferro-benchmark-{index}.desktop").write_text(
                f"[Desktop Entry]\nType=Application\nName=Benchmark app {index + 1}\nExec=true\n",
                encoding="utf-8",
            )
        wm_config = Path(__file__).resolve().parents[1] / "session/openbox.xml"
        wm_started = time.monotonic()
        stack.enter_context(process(["openbox", "--sm-disable", "--config-file", wm_config]))
        while subprocess.run(["xdotool", "get_num_desktops"], capture_output=True).returncode != 0:
            if time.monotonic() - wm_started > 5:
                raise RuntimeError("Openbox did not initialize")
            time.sleep(.05)
        # Openbox needs its system data directories for its own fonts and theme.
        # Only the shell receives the isolated application catalog.
        os.environ["XDG_DATA_DIRS"] = str(base / "data")
        shell = stack.enter_context(process([binary, "--theme=windows"]))
        window = xdo("search", "--sync", "--onlyvisible", "--name", "^Ferro Desktop$").splitlines()[0]
        xdo("windowsize", "--sync", window, "1280", "800", "windowactivate", "--sync", window,
            "mousemove", "1420", "940")
        probe = Probe(int(window))
        stack.callback(probe.disconnect, probe.display)
        time.sleep(1.5)
        closed = probe.rgb()
        probe.transition(probe.open_keys, closed)
        time.sleep(.4)
        opened = probe.rgb()
        probe.transition(probe.close_keys, opened)
        time.sleep(.8)

        before = cpu_ticks(shell.pid)
        start = time.monotonic()
        time.sleep(10)
        elapsed = time.monotonic() - start
        delta = cpu_ticks(shell.pid) - before
        hz = os.sysconf("SC_CLK_TCK")
        opens, closes = [], []
        for _ in range(10):
            opens.append(probe.transition(probe.open_keys, closed))
            time.sleep(.25)
            closes.append(probe.transition(probe.close_keys, opened))
            time.sleep(.25)
        if shell.poll() is not None:
            raise RuntimeError("Shell exited during the benchmark")
        print(json.dumps({
            "idle_seconds": elapsed, "idle_cpu_ticks": delta, "ticks_per_second": hz,
            "idle_cpu_percent_one_core": 100 * delta / hz / elapsed,
            "open_ms": opens, "open_median_ms": statistics.median(opens),
            "close_ms": closes, "close_median_ms": statistics.median(closes),
        }, indent=2))


if __name__ == "__main__":
    if len(sys.argv) != 2 or not os.access(sys.argv[1], os.X_OK) or not Path(sys.argv[1]).is_file():
        sys.exit(__doc__)
    # The public entry point always allocates its own display and session bus.
    entry = "import runpy,sys; runpy.run_path(sys.argv[1])['isolated_run'](sys.argv[2])"
    sys.exit(subprocess.call(["xvfb-run", "-a", "-s", "-screen 0 1440x960x24 -nolisten tcp",
                              "dbus-run-session", "--", sys.executable, "-c", entry,
                              str(Path(__file__).resolve()), str(Path(sys.argv[1]).resolve())]))
