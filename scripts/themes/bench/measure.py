#!/usr/bin/env python3
"""Measure a production WinUI consumer in an isolated Cargo target directory.

Run this script against each revision using --source. Use a separate, initially
empty --work-dir for each cold build. The primary compile measurement cleans
only WinUI and the probe, then regenerates the theme, compiles it, and links
the executable with warmed dependencies. Times are wall clock seconds.

python3 scripts/themes/bench/measure.py --work-dir /tmp/winui-before --source ../baseline
python3 scripts/themes/bench/measure.py --work-dir /tmp/winui-after
python3 scripts/themes/bench/measure.py --work-dir /tmp/winui-binary --binary
python3 scripts/themes/bench/measure.py --work-dir /tmp/winui-packed --packed
"""
import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import statistics
import subprocess
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


def cargo_timings(target, packages=(THEME, PROBE)):
    # Keep the documented HTML report as the canonical raw artifact. Cargo's
    # embedded table is optional diagnostic detail, and can change with Cargo.
    report = target / "cargo-timings/cargo-timing.html"
    if report.is_file():
        marker = "const UNIT_DATA = "
        text = report.read_text()
        if marker in text:
            try:
                units, _ = json.JSONDecoder().raw_decode(text.split(marker, 1)[1])
                return [
                    {"package": u["name"], "target": u["target"].strip(), "seconds": u["duration"]}
                    for u in units if u["name"] in packages
                ]
            except (ValueError, KeyError, TypeError):
                pass
    return []


def positive_int(text):
    value = int(text)
    if value < 1:
        raise argparse.ArgumentTypeError("expected a positive integer")
    return value


def measure_launches(executables, rounds):
    # Rotate each executable through first and last positions so slow drift
    # during the run does not systematically favor the candidate or baseline.
    samples = {path: [] for path in executables}
    for executable in executables:
        command_output([executable])
    for index in range(rounds):
        offset = index % len(executables)
        for executable in executables[offset:] + executables[:offset]:
            start = time.perf_counter_ns()
            # The probe flushes a line after resolving Button. Waiting for process
            # exit here would also charge theme destruction to user-visible startup.
            with subprocess.Popen([executable], stdout=subprocess.PIPE, text=True) as process:
                output = process.stdout.readline()
                ready = time.perf_counter_ns() - start
                return_code = process.wait()
                if return_code:
                    raise subprocess.CalledProcessError(return_code, [executable])
            elapsed = time.perf_counter_ns() - start
            create, lookup = map(int, output.split())
            samples[executable].append({
                "create_ns": create, "button_lookup_ns": lookup - create, "create_and_button_ns": lookup,
                "process_ready_ns": ready, "process_ns": elapsed,
            })
    return samples


def lld_sections(link_map, archive, executable):
    # LLD prints input sections, including anonymous constants, unwind records and TLS.
    # Restrict to SHF_ALLOC output sections. Padding and synthesized dynamic relocations
    # belong to the linker and have no unique input crate.
    report = command_output(["readelf", "-SW", str(executable)])
    allocated = set(re.findall(r"^\s*\[\s*\d+\]\s+(\S+)\s+\S+\s+\w+\s+\w+\s+\w+\s+\w+\s+\S*A\S*\s", report, re.M))
    sections = {}
    output = None
    with link_map.open() as stream:
        for line in stream:
            fields = line.split(maxsplit=4)
            if len(fields) != 5:
                continue
            _, _, size, _, entry = fields
            entry = entry.strip()
            if entry in allocated:
                output = entry
            elif entry.startswith("."):
                output = None
            elif output and f"/{archive}(" in entry and "):(" in entry:
                sections[output] = sections.get(output, 0) + int(size, 16)
    if not sections:
        raise ValueError(f"no allocated input sections from {archive} in {link_map}")
    return sections


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=HERE.parents[2])
    parser.add_argument("--work-dir", type=Path, required=True, help="dedicated directory for the consumer, target and results")
    backend = parser.add_mutually_exclusive_group()
    backend.add_argument("--packed", action="store_true", help="embed the complete FXP theme archive")
    backend.add_argument("--solid", action="store_true", help="embed the solid-compressed complete FXP archive")
    backend.add_argument("--binary", action="store_true", help="enable the theme's binary-resources feature")
    parser.add_argument("--size-optimize-theme", action="store_true",
                        help="compile just the WinUI crate with opt-level=z and one codegen unit")
    parser.add_argument("--lld-link-map", action="store_true",
                        help="measure allocated WinUI input sections with the ELF LLD linker")
    parser.add_argument("--max-theme-bytes", type=positive_int,
                        help="fail after writing the report if allocated WinUI sections exceed this limit")
    parser.add_argument("--rebuilds", type=int, default=1, help="number of dependency-warm theme rebuilds, zero to measure the initial build only")
    parser.add_argument("--launches", type=positive_int, default=15, help="number of measured, fresh-process launches")
    parser.add_argument("--compare-with", type=Path, action="append", default=[],
                        help="also time a previous work directory's stripped-probe, interleaved with the new one")
    args = parser.parse_args()
    if args.max_theme_bytes and not args.lld_link_map:
        parser.error("--max-theme-bytes requires --lld-link-map")
    if args.rebuilds < 0:
        parser.error("--rebuilds must be zero or positive")
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
    if args.size_optimize_theme:
        manifest += f'\n[profile.release.package."{THEME}"]\nopt-level = "z"\ncodegen-units = 1\n'
    (package / "Cargo.toml").write_text(manifest)
    shutil.copyfile(HERE / "winui.rs", package / "main.rs")
    # Cargo restricts this lock to the consumer's dependency graph on the first build.
    shutil.copyfile(source / "Cargo.lock", package / "Cargo.lock")
    # The standalone shipping-dependency consumer uses Cargo's release profile at
    # opt-level 3, without cross-crate LTO. Stripping is a separate measured copy.
    env = {**os.environ, "CARGO_TARGET_DIR": str(target), "CARGO_INCREMENTAL": "0", "CARGO_PROFILE_RELEASE_LTO": "false"}
    packed = args.packed or args.solid
    feature = "solid-theme" if args.solid else "packed-theme" if args.packed else "binary-resources" if args.binary else None
    cargo_args = ["--features", f"{THEME}/{feature}"] if feature else []
    first_build, messages = build(cargo_args, package, env)
    initial_units = cargo_timings(target)
    rebuilds = []
    for _ in range(args.rebuilds):
        subprocess.run(["cargo", "clean", "--release", "-p", THEME, "-p", PROBE], cwd=package, env=env, check=True)
        duration, messages = build(["--locked", *cargo_args], package, env)
        rebuilds.append(duration)
    artifact = next(m for m in messages if m["reason"] == "compiler-artifact" and m["target"]["name"] == "ferroui_themes_winui")
    rlib = next(Path(name) for name in artifact["filenames"] if name.endswith(".rlib"))
    out = Path(next(m["out_dir"] for m in messages if m["reason"] == "build-script-executed" and m["package_id"] == artifact["package_id"]))
    executable = Path(next(m["executable"] for m in messages if m.get("executable") and m["target"]["name"] == PROBE))
    generated = out / "xaml"
    # Packed mode exports the same entry points from a handwritten adapter and emits no
    # per-document Rust module. Count its small generated registration module separately.
    source_bytes = b"" if packed else (generated / "compiled_xaml.rs").read_bytes()
    # Earlier feature/root combinations can have left other blocks in OUT_DIR.
    # Only files included by this compiled consumer contribute to its payload.
    referenced = set(re.findall(rb'include_bytes!\("([a-zA-Z0-9_-]+\.fxb)"\)', source_bytes))
    last_units = cargo_timings(target)
    sections = None
    if args.lld_link_map:
        link_map = work / "consumer.link.map"
        subprocess.run(["cargo", "rustc", "--locked", "--release", *cargo_args, "--bin", PROBE, "--",
                        "-Clink-arg=-fuse-ld=lld", "-Clink-arg=-Wl,-Map," + str(link_map)],
                       cwd=package, env=env, check=True)
        sections = lld_sections(link_map, rlib.name, executable)
    stripped = work / "stripped-probe"
    shutil.copy2(executable, stripped)
    subprocess.run(["strip", str(stripped)], check=True)
    stripped_bytes = stripped.stat().st_size
    executables = [stripped, *(path.resolve() for path in args.compare_with)]
    if len(set(executables)) != len(executables):
        parser.error("each probe must have a distinct path")
    # Warm the OS page cache first. Each timed sample performs actual theme
    # construction and a checked deferred lookup in a fresh process.
    samples = measure_launches(executables, args.launches)
    launches = samples[stripped]
    result = {
        "revision": command_output(["git", "rev-parse", "HEAD"], source),
        "source_dirty": bool(command_output(["git", "status", "--porcelain", "--untracked-files=no"], source)),
        "rustc": command_output(["rustc", "-vV"]), "cargo": command_output(["cargo", "-V"]),
        "platform": platform.platform(), "jobs": env.get("CARGO_BUILD_JOBS", "cargo default"),
        "packed_theme": packed, "solid_theme": args.solid, "size_optimized_theme": args.size_optimize_theme,
        "theme_allocated_sections": sections, "theme_allocated_bytes": sum(sections.values()) if sections else None,
        "fxp_bytes": (out / "winui.fxp").stat().st_size if packed else 0,
        "generated_registration_rust_bytes": (generated / "mod.rs").stat().st_size,
        "binary_resources": args.binary, "resource_roots": env.get("FERROUI_WINUI_THEME_ROOTS"),
        "profile": "release, LTO off, incremental off", "initial_target_empty": target_was_empty,
        "initial_build_seconds": first_build, "theme_rebuild_seconds": rebuilds,
        "initial_cargo_units": initial_units, "last_cargo_units": last_units,
        "generated_rust_bytes": len(source_bytes), "generated_rust_sha256": hashlib.sha256(source_bytes).hexdigest(),
        "theme_rlib_bytes": rlib.stat().st_size, "fxb_blocks": len(referenced),
        "fxb_bytes": sum((generated / name.decode("ascii")).stat().st_size for name in referenced),
        "stripped_executable_bytes": stripped_bytes,
        "launches": launches,
        "median": {k: statistics.median(sample[k] for sample in launches) for k in launches[0]},
    }
    if args.compare_with:
        result["interleaved_launches"] = {str(path): rows for path, rows in samples.items()}
    report = json.dumps(result, indent=2) + "\n"
    (work / "report.json").write_text(report)
    print(report, end="")
    if args.max_theme_bytes and result["theme_allocated_bytes"] > args.max_theme_bytes:
        raise SystemExit(f"WinUI contributed {result['theme_allocated_bytes']} bytes; limit {args.max_theme_bytes}")


if __name__ == "__main__":
    main()
