#!/usr/bin/env python3
"""Measure a production WinUI consumer in an isolated Cargo target directory.

Run this script against each revision using --source. Use a separate, initially
empty --work-dir for each cold build. The primary compile measurement cleans
only WinUI and the probe, then regenerates the theme, compiles it, and links
the executable with warmed dependencies. Times are wall clock seconds.

python3 scripts/themes/bench/measure.py --work-dir /tmp/winui-before --source ../baseline
python3 scripts/themes/bench/measure.py --work-dir /tmp/winui-after
python3 scripts/themes/bench/measure.py --work-dir /tmp/winui-binary --binary
"""
import argparse
import json
import os
import platform
import shutil
import statistics
import subprocess
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
THEME = "ferroui-themes-winui"
PROBE = "winui-theme-probe"


def command_output(args, cwd=None):
    return subprocess.check_output(args, cwd=cwd, text=True).strip()


def build(cargo_args, directory, env):
    start = time.perf_counter()
    proc = subprocess.run(
        ["cargo", "build", "--release", "--timings", "--message-format=json-render-diagnostics", *cargo_args],
        cwd=directory, env=env, check=True, stdout=subprocess.PIPE, text=True,
    )
    return time.perf_counter() - start, [json.loads(line) for line in proc.stdout.splitlines()]


def positive_int(text):
    value = int(text)
    if value < 1:
        raise argparse.ArgumentTypeError("expected a positive integer")
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=HERE.parents[2])
    parser.add_argument("--work-dir", type=Path, required=True, help="dedicated directory for the consumer, target and results")
    parser.add_argument("--binary", action="store_true", help="enable the theme's binary-resources feature")
    parser.add_argument("--rebuilds", type=positive_int, default=1, help="number of dependency-warm theme rebuilds")
    parser.add_argument("--launches", type=positive_int, default=15, help="number of measured, fresh-process launches")
    args = parser.parse_args()
    source = args.source.resolve()
    work = args.work_dir.resolve()
    target = work / "target"
    work.mkdir(parents=True, exist_ok=True)
    target_was_empty = not target.exists() or not any(target.iterdir())
    package = work / "consumer"
    package.mkdir(exist_ok=True)
    deps = {
        "ferroui-base": source / "src/FerroUI.Base",
        "ferroui-controls": source / "src/FerroUI.Controls",
        THEME: source / "src/FerroUI.Themes.WinUI",
    }
    manifest = (
        '[workspace]\nresolver = "2"\n[package]\n'
        f'name = "{PROBE}"\nversion = "0.0.0"\nedition = "2021"\n'
        f'[[bin]]\nname = "{PROBE}"\npath = "main.rs"\n[dependencies]\n'
    )
    for name, path in deps.items():
        manifest += f'{name} = {{ path = {json.dumps(str(path))} }}\n'
    (package / "Cargo.toml").write_text(manifest)
    shutil.copyfile(HERE / "winui.rs", package / "main.rs")
    # Cargo restricts this lock to the consumer's dependency graph on the first build.
    shutil.copyfile(source / "Cargo.lock", package / "Cargo.lock")
    # The standalone shipping-dependency consumer uses Cargo's release profile at
    # opt-level 3, without cross-crate LTO. Stripping is a separate measured copy.
    env = {**os.environ, "CARGO_TARGET_DIR": str(target), "CARGO_INCREMENTAL": "0", "CARGO_PROFILE_RELEASE_LTO": "false"}
    cargo_args = ["--features", THEME + "/binary-resources"] if args.binary else []
    first_build, _ = build(cargo_args, package, env)
    rebuilds = []
    for _ in range(args.rebuilds):
        subprocess.run(["cargo", "clean", "--release", "-p", THEME, "-p", PROBE], cwd=package, env=env, check=True)
        duration, messages = build(["--locked", *cargo_args], package, env)
        rebuilds.append(duration)
    artifact = next(m for m in messages if m["reason"] == "compiler-artifact" and m["target"]["name"] == "ferroui_themes_winui")
    out = Path(next(m["out_dir"] for m in messages if m["reason"] == "build-script-executed" and m["package_id"] == artifact["package_id"]))
    executable = Path(next(m["executable"] for m in messages if m.get("executable") and m["target"]["name"] == PROBE))
    generated = out / "xaml"
    source_bytes = (generated / "compiled_xaml.rs").read_bytes()
    with tempfile.TemporaryDirectory(dir=work) as folder:
        stripped = Path(folder) / executable.name
        shutil.copy2(executable, stripped)
        subprocess.run(["strip", str(stripped)], check=True)
        stripped_bytes = stripped.stat().st_size
        # Warm the OS page cache. Every timed sample still starts a fresh process
        # and performs actual theme construction and a checked deferred lookup.
        command_output([stripped])
        launches = []
        for _ in range(args.launches):
            start = time.perf_counter_ns()
            output = command_output([stripped])
            elapsed = time.perf_counter_ns() - start
            create, lookup = map(int, output.split())
            launches.append({"create_ns": create, "create_and_button_ns": lookup, "process_ns": elapsed})
    result = {
        "revision": command_output(["git", "rev-parse", "HEAD"], source),
        "source_dirty": bool(command_output(["git", "status", "--porcelain", "--untracked-files=no"], source)),
        "rustc": command_output(["rustc", "-vV"]), "cargo": command_output(["cargo", "-V"]),
        "platform": platform.platform(), "jobs": env.get("CARGO_BUILD_JOBS", "cargo default"),
        "binary_resources": args.binary, "resource_roots": env.get("FERROUI_WINUI_THEME_ROOTS"),
        "profile": "release, LTO off, incremental off", "initial_target_empty": target_was_empty,
        "initial_build_seconds": first_build, "theme_rebuild_seconds": rebuilds,
        "generated_rust_bytes": len(source_bytes),
        "fxb_bytes": sum(p.stat().st_size for p in generated.rglob("*.fxb")),
        "stripped_executable_bytes": stripped_bytes,
        "launches": launches,
        "median": {k: statistics.median(sample[k] for sample in launches) for k in launches[0]},
    }
    report = json.dumps(result, indent=2) + "\n"
    (work / "report.json").write_text(report)
    print(report, end="")


if __name__ == "__main__":
    main()
