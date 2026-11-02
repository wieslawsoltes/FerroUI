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
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
THEME = "ferroui-themes-winui"
PROBE = "winui-theme-probe"


def command_output(args, cwd=None):
    return subprocess.check_output(args, cwd=cwd, text=True).strip()


def build(cargo_args, directory, env, link_args=()):
    start = time.perf_counter()
    action = "rustc" if link_args else "build"
    final_args = ["--bin", PROBE, "--", *link_args] if link_args else []
    proc = subprocess.run(
        ["cargo", action, "--release", "--timings", "--message-format=json-render-diagnostics", *cargo_args, *final_args],
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
    return measure_commands({path: [path] for path in executables}, rounds)


def measure_commands(commands, rounds):
    # Rotate each executable through first and last positions so slow drift
    # during the run does not systematically favor the candidate or baseline.
    executables = list(commands)
    samples = {path: [] for path in executables}
    for command in commands.values():
        command_output(command)
    for index in range(rounds):
        offset = index % len(executables)
        for executable in executables[offset:] + executables[:offset]:
            start = time.perf_counter_ns()
            # The probe flushes a line after resolving Button. Waiting for process
            # exit here would also charge theme destruction to user-visible startup.
            with subprocess.Popen(commands[executable], stdout=subprocess.PIPE, text=True) as process:
                output = process.stdout.readline()
                ready = time.perf_counter_ns() - start
                return_code = process.wait()
                if return_code:
                    raise subprocess.CalledProcessError(return_code, commands[executable])
            elapsed = time.perf_counter_ns() - start
            create, lookup = map(int, output.split())
            samples[executable].append({
                "create_ns": create, "button_lookup_ns": lookup - create, "create_and_button_ns": lookup,
                "process_ready_ns": ready, "process_ns": elapsed,
            })
    return samples


def startup_comparison(samples, reference, case):
    baseline = statistics.median(row["process_ready_ns"] for row in samples[reference])
    candidate = statistics.median(row["process_ready_ns"] for row in samples[case])
    return {"baseline": str(reference), "baseline_median_ns": baseline, "candidate_median_ns": candidate,
            "ratio": candidate / baseline, "passed": candidate <= baseline}


def elf_sections(executable):
    """File and memory sizes of the allocated sections in an ELF image."""
    report = command_output(["readelf", "-SW", str(executable)])
    sections = {}
    for name, kind, size, flags in re.findall(
        r"^\s*\[\s*\d+\]\s+(\S+)\s+(\S+)\s+\w+\s+\w+\s+(\w+)\s+\w+\s+(\S+)", report, re.M
    ):
        if "A" in flags:
            sections[name] = {"allocated_bytes": int(size, 16), "file_bytes": 0 if kind == "NOBITS" else int(size, 16)}
    return sections


def lld_allocations(link_map, executable):
    """Attribute allocated input sections to the archive that supplied them."""
    sections = elf_sections(executable)
    by_archive = {}
    output = None
    archive_name = re.compile(r"(?:^|/)([^/(]+\.(?:rlib|a))\(")
    with link_map.open() as stream:
        for line in stream:
            fields = line.split(maxsplit=4)
            if len(fields) != 5:
                continue
            _, _, size, _, entry = fields
            entry = entry.strip()
            if entry.startswith("."):
                output = entry if entry in sections else None
            elif output and "):(" in entry:
                match = archive_name.search(entry)
                if match:
                    totals = by_archive.setdefault(match.group(1), {})
                    totals[output] = totals.get(output, 0) + int(size, 16)
    return {"elf_sections": sections, "archive_input_sections": by_archive}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=HERE.parents[2])
    parser.add_argument("--work-dir", type=Path, required=True, help="dedicated directory for the consumer, target and results")
    backend = parser.add_mutually_exclusive_group()
    backend.add_argument("--packed", action="store_true", help="embed the complete FXP theme archive")
    backend.add_argument("--solid", action="store_true", help="use the shipping solid-theme feature of the source revision")
    backend.add_argument("--portable-solid", action="store_true", help="interpret the solid archive using the runtime loader")
    backend.add_argument("--precompiled", action="store_true",
                         help="execute typed FXB and retain its editable solid FXP archive")
    parser.add_argument("--compact-app", action="store_true",
                        help="build the entire application with opt-level=s, ThinLTO and 16 CGUs")
    backend.add_argument("--binary", action="store_true", help="enable the theme's binary-resources feature")
    parser.add_argument("--linux-packed-relocations", action="store_true",
                        help="use DT_RELR for the final Linux ELF (requires a compatible dynamic loader)")
    parser.add_argument("--linux-safe-icf", action="store_true",
                        help="emit address-significance metadata during LTO and fold identical ELF sections with LLD")
    parser.add_argument("--fat-app-link", action="store_true",
                        help="use one whole-program LTO unit for the final compact executable")
    parser.add_argument("--max-executable-bytes", type=positive_int,
                        help="require the complete stripped application to fit this many bytes")
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
    parser.add_argument("--startup-baseline", type=Path,
                        help="interleave this reference executable and require median time to Button at or below its median")
    parser.add_argument("--compare-archive", type=Path, action="append", default=[],
                        help="also launch the precompiled consumer with an FXP file on disk")
    parser.add_argument("--require-archive-startup", type=Path, action="append", default=[],
                        help="time this external revision and require its median to meet --startup-baseline")
    parser.add_argument("--compiled-updates", action="store_true",
                        help="enable the shipping consumer to load FXU operand updates as well as FXP")
    args = parser.parse_args()
    source = args.source.resolve()
    with (source / "src/FerroUI.Themes.WinUI/Cargo.toml").open("rb") as stream:
        theme_features = tomllib.load(stream)["features"]
    # Older revisions used solid-theme for the compact interpreter.
    separate_portable = "portable-solid-theme" in theme_features
    precompiled = args.precompiled or (args.solid and separate_portable)
    if args.require_archive_startup and not args.startup_baseline:
        parser.error("--require-archive-startup requires --startup-baseline")
    if args.compiled_updates and (not precompiled or "compiled-updates" not in theme_features):
        parser.error("--compiled-updates requires a precompiled source with FXU support")
    if (args.compare_archive or args.require_archive_startup) and not precompiled:
        parser.error("--compare-archive requires a precompiled solid theme")
    if args.max_theme_bytes and not args.lld_link_map:
        parser.error("--max-theme-bytes requires --lld-link-map")
    if args.compact_app and args.lld_link_map:
        parser.error("cross-crate LTO cannot attribute all optimized code to its input archives")
    if args.linux_safe_icf and (platform.system() != "Linux" or not args.compact_app):
        parser.error("--linux-safe-icf requires --compact-app on a native Linux ELF target")
    if args.linux_packed_relocations and platform.system() != "Linux":
        parser.error("--linux-packed-relocations requires a Linux ELF target")
    if args.fat_app_link and not args.compact_app:
        parser.error("--fat-app-link requires --compact-app")
    if args.rebuilds < 0:
        parser.error("--rebuilds must be zero or positive")
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
    # The archive-enabled consumer can also load a changed pack from disk. Keep its
    # complete runtime interpreter in the measured executable, including with LTO.
    if precompiled:
        markup = source / "src/Markup"
        manifest += f'ferroui-markup-xaml = {{ path = {json.dumps(str(markup / "FerroUI.Markup.Xaml"))} }}\n'
        loader = json.dumps(str(markup / "FerroUI.Markup.Xaml.Loader"))
        manifest += f'ferroui-markup-xaml-loader = {{ path = {loader}, features = ["runtime", "portable-xaml"] }}\n'
    manifest += '\n[features]\nretain-archive = []\nlegacy-archive = []\noperand-updates = []\n'
    if args.compact_app:
        manifest += '\n[profile.release]\nopt-level = "s"\ncodegen-units = 16\n'
    if args.size_optimize_theme:
        manifest += f'\n[profile.release.package."{THEME}"]\nopt-level = "z"\ncodegen-units = 1\n'
    (package / "Cargo.toml").write_text(manifest)
    shutil.copyfile(HERE / "winui.rs", package / "main.rs")
    # Cargo restricts this lock to the consumer's dependency graph on the first build.
    shutil.copyfile(source / "Cargo.lock", package / "Cargo.lock")
    # Stripping is a separate measured copy for both release profiles.
    env = {**os.environ, "CARGO_TARGET_DIR": str(target), "CARGO_INCREMENTAL": "0", "CARGO_PROFILE_RELEASE_LTO": "thin" if args.compact_app else "false"}
    packed = args.packed or args.portable_solid or (args.solid and not precompiled)
    features = []
    if precompiled:
        feature = "solid-theme" if args.solid else "precompiled-theme"
        features = [f"{THEME}/{feature}", "retain-archive"]
        if not (markup / "FerroUI.Markup.Xaml.Loader/compiled_archive.rs").exists():
            features.append("legacy-archive")
        if args.compiled_updates:
            features.extend(["operand-updates", f"{THEME}/compiled-updates"])
    elif args.packed:
        features = [f"{THEME}/packed-theme"]
    elif packed:
        feature = "portable-solid-theme" if separate_portable else "solid-theme"
        features = [f"{THEME}/{feature}"]
    elif args.binary:
        features = [f"{THEME}/binary-resources"]
    cargo_args = ["--features", ",".join(features)] if features else []
    link_args = ["-Clink-arg=-Wl,-z,pack-relative-relocs"] if args.linux_packed_relocations else []
    if args.linux_safe_icf:
        link_args.extend(["-Cllvm-args=--addrsig", "-Clink-arg=-fuse-ld=lld", "-Clink-arg=-Wl,--icf=safe"])
    if args.fat_app_link:
        # Keep the same cached compact dependencies for both link policies.
        link_args.extend(["-Clto=fat", "-Ccodegen-units=1"])
    first_build, messages = build(cargo_args, package, env, link_args)
    initial_units = cargo_timings(target)
    rebuilds = []
    for _ in range(args.rebuilds):
        subprocess.run(["cargo", "clean", "--release", "-p", THEME, "-p", PROBE], cwd=package, env=env, check=True)
        duration, messages = build(["--locked", *cargo_args], package, env, link_args)
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
    allocations = None
    if args.lld_link_map:
        link_map = work / "consumer.link.map"
        subprocess.run(["cargo", "rustc", "--locked", "--release", *cargo_args, "--bin", PROBE, "--",
                        *link_args, "-Clink-arg=-fuse-ld=lld", "-Clink-arg=-Wl,-Map," + str(link_map)],
                       cwd=package, env=env, check=True)
        allocations = lld_allocations(link_map, executable)
        sections = allocations["archive_input_sections"].get(rlib.name)
        if not sections:
            raise ValueError(f"no allocated input sections from {rlib.name} in {link_map}")
    stripped = work / "stripped-probe"
    shutil.copy2(executable, stripped)
    subprocess.run(["strip", str(stripped)], check=True)
    stripped_bytes = stripped.stat().st_size
    if args.linux_packed_relocations and "(RELR)" not in command_output(["readelf", "-dW", str(stripped)]):
        raise ValueError("the linker did not emit the requested DT_RELR table")
    executables = [stripped, *(path.resolve() for path in args.compare_with)]
    baseline = args.startup_baseline.resolve() if args.startup_baseline else None
    if baseline == stripped:
        parser.error("the startup baseline must be a different executable")
    if baseline and baseline not in executables:
        executables.append(baseline)
    if len(set(executables)) != len(executables):
        parser.error("each probe must have a distinct path")
    # Warm the OS page cache first. Each timed sample performs actual theme
    # construction and a checked deferred lookup in a fresh process.
    commands = {path: [path] for path in executables}
    required = list(dict.fromkeys(p.resolve() for p in args.require_archive_startup))
    archives = list(dict.fromkeys(p.resolve() for p in [*args.compare_archive, *required]))
    for archive in archives:
        if archive in commands:
            parser.error("each comparison must have a distinct path")
        commands[archive] = [stripped, archive]
    samples = measure_commands(commands, args.launches)
    launches = samples[stripped]
    profile = "release, LTO off, incremental off"
    if args.compact_app:
        lto = "16 dependency CGUs, final fat LTO in one CGU" if args.fat_app_link else "ThinLTO, 16 CGUs"
        profile = f"release, opt-level=s, {lto}, incremental off"
    result = {
        "revision": command_output(["git", "rev-parse", "HEAD"], source),
        "source_dirty": bool(command_output(["git", "status", "--porcelain", "--untracked-files=no"], source)),
        "rustc": command_output(["rustc", "-vV"]), "cargo": command_output(["cargo", "-V"]),
        "platform": platform.platform(), "jobs": env.get("CARGO_BUILD_JOBS", "cargo default"),
        "packed_theme": packed, "solid_theme": args.solid or args.portable_solid or precompiled, "size_optimized_theme": args.size_optimize_theme,
        "precompiled_theme": precompiled, "compact_app": args.compact_app, "fat_app_link": args.fat_app_link,
        "linux_packed_relocations": args.linux_packed_relocations, "linux_safe_icf": args.linux_safe_icf,
        "elf_sections": elf_sections(stripped) if platform.system() == "Linux" and shutil.which("readelf") else None,
        "linked_allocations": allocations,
        "theme_allocated_sections": sections, "theme_allocated_bytes": sum(sections.values()) if sections else None,
        "fxp_bytes": (out / "winui.fxp").stat().st_size if packed or precompiled else 0,
        "generated_registration_rust_bytes": (generated / "mod.rs").stat().st_size,
        "binary_resources": args.binary or precompiled, "resource_roots": env.get("FERROUI_WINUI_THEME_ROOTS"),
        "profile": profile,
        "initial_target_empty": target_was_empty,
        "initial_build_seconds": first_build, "theme_rebuild_seconds": rebuilds,
        "initial_cargo_units": initial_units, "last_cargo_units": last_units,
        "generated_rust_bytes": len(source_bytes), "generated_rust_sha256": hashlib.sha256(source_bytes).hexdigest(),
        "theme_rlib_bytes": rlib.stat().st_size, "fxb_blocks": len(referenced),
        "fxb_bytes": sum((generated / name.decode("ascii")).stat().st_size for name in referenced),
        "stripped_executable_bytes": stripped_bytes,
        "launches": launches,
        "median": {k: statistics.median(sample[k] for sample in launches) for k in launches[0]},
    }
    if baseline:
        result["startup_comparison"] = startup_comparison(samples, baseline, stripped)
        result["archive_startup_comparisons"] = {str(p): startup_comparison(samples, baseline, p) for p in required}
    if baseline or args.compare_with or archives:
        result["interleaved_launches"] = {str(path): rows for path, rows in samples.items()}
    report = json.dumps(result, indent=2) + "\n"
    (work / "report.json").write_text(report)
    print(report, end="")
    if baseline:
        checks = {"installed": result["startup_comparison"], **result["archive_startup_comparisons"]}
        failures = [f"{name} {check['candidate_median_ns'] / 1e6:.3f} ms"
                    for name, check in checks.items() if not check["passed"]]
        if failures:
            reference_ms = result["startup_comparison"]["baseline_median_ns"] / 1e6
            raise SystemExit(f"median startup exceeded {reference_ms:.3f} ms: " + ", ".join(failures))
    if args.max_executable_bytes and stripped_bytes > args.max_executable_bytes:
        raise SystemExit(f"stripped application is {stripped_bytes} bytes; limit {args.max_executable_bytes}")
    if args.max_theme_bytes and result["theme_allocated_bytes"] > args.max_theme_bytes:
        raise SystemExit(f"WinUI contributed {result['theme_allocated_bytes']} bytes; limit {args.max_theme_bytes}")


if __name__ == "__main__":
    main()
