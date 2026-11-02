#!/usr/bin/env python3
"""Compare runtime XML and FXD on a production theme dictionary with a fresh process per load."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess

from measure import HERE, build, cargo_timings, command_output, measure_launches, positive_int

LOADER = "ferroui-markup-xaml-loader"
CONSUMER = "runtime-document-probe"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=HERE.parents[2])
    parser.add_argument("--work-dir", type=Path, required=True)
    parser.add_argument("--launches", type=positive_int, default=25)
    parser.add_argument("--fixture", choices=("winui", "fluent"), default="winui")
    parser.add_argument("--encoder", type=Path, help="use an already built ferroui-fxd from the same revision")
    args = parser.parse_args()
    source = args.source.resolve()
    work = args.work_dir.resolve()
    work.mkdir(parents=True, exist_ok=True)
    loader = source / "src/Markup/FerroUI.Markup.Xaml.Loader"
    base = source / "src/FerroUI.Base"
    fixtures = {"winui": ("src/FerroUI.Themes.WinUI/Resources/ThemeShadows.xaml", "WinUIThemeShadow"),
                "fluent": ("src/FerroUI.Themes.Fluent/Accents/BaseResources.xaml", "SystemControlTransparentBrush")}
    relative_input, resource = fixtures[args.fixture]
    xml = source / relative_input
    fxd = work / xml.with_suffix(".fxd").name
    target = work / "target"
    env = {**os.environ, "CARGO_TARGET_DIR": str(target), "CARGO_INCREMENTAL": "0",
           "CARGO_PROFILE_RELEASE_LTO": "false"}

    # Keep the encoder in its own process and out of both consumer executables.
    encoder = [str(args.encoder.resolve())] if args.encoder else [
        "cargo", "run", "--release", "--no-default-features", "--features", "portable-xaml",
        "--bin", "ferroui-fxd", "--manifest-path", str(loader / "Cargo.toml"), "--",
    ]
    subprocess.run([*encoder, str(xml), str(fxd)], env=env, check=True)

    (work / "Cargo.toml").write_text(
        f'[workspace]\nresolver = "2"\n[package]\nname = "{CONSUMER}"\n'
        'version = "0.0.0"\nedition = "2021"\n[[bin]]\n'
        f'name = "{CONSUMER}"\npath = "main.rs"\n[dependencies]\n'
        f'ferroui-base = {{ path = {json.dumps(str(base))} }}\n'
        f'{LOADER} = {{ path = {json.dumps(str(loader))} }}\n'
    )
    shutil.copyfile(source / "Cargo.lock", work / "Cargo.lock")
    shutil.copyfile(HERE / "runtime_document.rs", work / "main.rs")
    cases = {}
    for mode, data in (("xml", xml), ("fxd", fxd)):
        compiled_env = {**env, "FERROUI_BENCH_DOCUMENT": str(data), "FERROUI_BENCH_RESOURCE": resource}
        features = ["--features", LOADER + "/portable-xaml"] if mode == "fxd" else []
        _, messages = build(features, work, compiled_env)
        units = cargo_timings(target, (LOADER, CONSUMER))
        exe = Path(next(m["executable"] for m in messages if m.get("executable") and m["target"]["name"] == CONSUMER))
        dest = work / mode
        shutil.copyfile(exe, dest)
        dest.chmod(0o755)
        subprocess.run(["strip", str(dest)], check=True)
        # Measure a reproducible downstream compile and link with this backend's dependencies warm.
        subprocess.run(["cargo", "clean", "--release", "-p", CONSUMER], cwd=work, env=compiled_env, check=True)
        warm_build, _ = build(features, work, compiled_env)
        cases[mode] = {"payload_bytes": data.stat().st_size, "payload_sha256": hashlib.sha256(data.read_bytes()).hexdigest(),
                       "stripped_bytes": dest.stat().st_size, "initial_units": units,
                       "warm_consumer_build_seconds": warm_build, "warm_consumer_units": cargo_timings(target, (LOADER, CONSUMER))}
    paths = [work / mode for mode in cases]
    samples = measure_launches(paths, args.launches)
    metrics = [("create_ns", "load_ns"), ("button_lookup_ns", "lookup_ns"), ("process_ready_ns", "process_ready_ns")]
    for path, case in zip(paths, cases.values()):
        rows = [{name: row[key] for key, name in metrics} for row in samples[path]]
        case["samples_ns"] = rows
        case["median_ns"] = {name: int(statistics.median(row[name] for row in rows)) for _, name in metrics}
    result = {"source_tree": command_output(["git", "rev-parse", "HEAD^{tree}"], source),
              "consumer": "scripts/themes/bench/runtime_document.rs",
              "consumer_sha256": hashlib.sha256((HERE / "runtime_document.rs").read_bytes()).hexdigest(),
              "input": relative_input, "resource": resource,
              "rustc": command_output(["rustc", "-vV"]), "jobs": env.get("CARGO_BUILD_JOBS", "cargo default"),
              "profile": "release, opt-level 3, cross-crate LTO off, incremental off",
              "method": "Embedded bytes, normal runtime load_stream, checked deferred resource and typed first lookup. Fresh interleaved processes, warm file cache. Consumer timings include link time; they exclude the standalone encoder.",
              "cases": cases}
    report = json.dumps(result, indent=2) + "\n"
    (work / "report.json").write_text(report)
    print(report, end="")


if __name__ == "__main__":
    main()
