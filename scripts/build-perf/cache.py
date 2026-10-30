#!/usr/bin/env python3
"""Measure stock Cargo with no wrapper, an empty sccache, and a populated sccache.

All timed builds use the same source, profile, features, command and absolute
target path. The target is emptied before every sample. Only compiler-cache state
changes. This measures local cache replay, not a faster cold Rust compiler or a
remote CI cache restore. No existing target directory, daemon or cache is touched.
"""
from __future__ import annotations

import argparse
from contextlib import ExitStack
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import shutil
import socket
import statistics
import subprocess
import sys
import tempfile
import time


def write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def output(command: list[str], source: Path, env: dict | None = None) -> str:
    result = subprocess.run(command, cwd=source, env=env, text=True,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    if result.returncode:
        raise RuntimeError(f"{command[0]} failed ({result.returncode}): {result.stderr[-6000:]}")
    return result.stdout.strip()


def source_fingerprint(source: Path, excluded: Path) -> str:
    """Detect concurrent source changes, including non-ignored untracked inputs."""
    names = subprocess.check_output(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"], cwd=source)
    digest = hashlib.sha256()
    for name in sorted(set(names.split(b"\0")) - {b""}):
        path = source / os.fsdecode(name)
        if path.is_relative_to(excluded):
            continue
        digest.update(name + b"\0")
        if path.is_symlink():
            digest.update(b"symlink\0" + os.fsencode(os.readlink(path)))
        elif path.is_file():
            digest.update(hashlib.sha256(path.read_bytes()).digest())
        elif not path.exists():
            digest.update(b"deleted\0")
        else:
            raise RuntimeError(f"Unexpanded submodule or tracked directory in source fingerprint: {path}")
    return digest.hexdigest()


def normalized(text: str, replacements: list[tuple[Path, str]]) -> str:
    spellings = {}
    for path, token in replacements:
        for spelling in {str(path), path.as_posix(), str(path).replace("\\", "\\\\")}:
            spellings[spelling] = token
    for spelling in sorted(spellings, key=len, reverse=True):
        text = text.replace(spelling, spellings[spelling])
    return text


def build_evidence(messages: list[dict], source: Path, target: Path) -> dict:
    """Use Cargo JSON, preserving feature variants and generated-file multiplicity."""
    replacements = [(source, "$SOURCE"), (target, "$TARGET")]
    units = []
    directories = {}
    for message in messages:
        reason = message.get("reason")
        if reason == "compiler-artifact":
            units.append({
                "package": normalized(message["package_id"], replacements),
                "name": message["target"]["name"],
                "kind": message["target"]["kind"],
                "features": sorted(message["features"]),
                "profile": message["profile"],
                "fresh": message["fresh"],
            })
        elif reason == "build-script-executed":
            directory = Path(message["out_dir"]).resolve()
            if not directory.is_relative_to(target):
                raise RuntimeError(f"Build script output escaped the isolated target: {directory}")
            package = normalized(message["package_id"], replacements)
            directories[(package, directory)] = None
    for package, directory in directories:
        replacements.append((directory, "$OUT[" + package + "]"))
    generated: dict[str, list[str]] = {}
    for package, directory in sorted(directories, key=lambda item: (item[0], str(item[1]))):
        if not directory.is_dir():
            raise RuntimeError(f"Cargo reported a missing build-script output: {directory}")
        for path in sorted(directory.rglob("*")):
            if not path.is_file() or path.suffix not in {".rs", ".xamlmeta"}:
                continue
            if not path.resolve().is_relative_to(target):
                raise RuntimeError(f"Generated source symlink escaped the isolated target: {path}")
            content = normalized(path.read_text(encoding="utf-8"), replacements)
            key = package + "/" + path.relative_to(directory).as_posix()
            generated.setdefault(key, []).append(hashlib.sha256(content.encode()).hexdigest())
    return {
        "units": sorted(units, key=lambda unit: json.dumps(unit, sort_keys=True)),
        "generated": {key: sorted(value) for key, value in sorted(generated.items())},
    }


def reset_target(target: Path, owned_root: Path) -> float:
    # Only fixed direct children of our fresh TemporaryDirectory can be deleted.
    if target.parent != owned_root or target.name not in {"target", "fixture-target"}:
        raise RuntimeError("Refusing to remove a target not owned by this benchmark")
    if target.is_symlink():
        raise RuntimeError("Refusing to remove a symlink used as a benchmark target")
    start = time.perf_counter()
    if target.exists():
        shutil.rmtree(target)
    target.mkdir()
    return time.perf_counter() - start


def build_environment(base: dict, target: Path, wrapper: str) -> dict:
    env = dict(base)
    # Empty values also disable wrappers inherited from .cargo/config.toml.
    env.update({"CARGO_TARGET_DIR": str(target), "CARGO_BUILD_BUILD_DIR": str(target),
                "CARGO_INCREMENTAL": "0", "CARGO_TERM_COLOR": "never",
                "RUSTC_WRAPPER": wrapper, "CARGO_BUILD_RUSTC_WRAPPER": wrapper,
                "RUSTC_WORKSPACE_WRAPPER": "", "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER": ""})
    return env


def isolated_cache_environment(base: dict, root: Path, control: Path) -> dict:
    # Never inherit a user's remote cache, credentials/configuration, recache
    # override or daemon endpoint. Cargo still receives its normal environment.
    env = {key: value for key, value in base.items() if not key.startswith("SCCACHE_")}
    cache_dir = root / "cache"
    cache_dir.mkdir()
    config = control / "config.toml"
    config.write_text("[cache.disk]\ndir = " + json.dumps(str(cache_dir)) +
                      "\nsize = 10737418240\n", encoding="utf-8")
    env.update({"SCCACHE_CONF": str(config), "SCCACHE_CACHED_CONF": str(control / "cached-config"),
                "SCCACHE_DIR": str(cache_dir), "SCCACHE_CACHE_SIZE": "10G",
                "SCCACHE_IDLE_TIMEOUT": "3600"})
    if os.name != "nt":
        endpoint = control / "s"
        if len(os.fsencode(endpoint)) >= 100:
            raise RuntimeError("Temporary path is too long for an isolated Unix socket; choose a shorter TMPDIR")
        env["SCCACHE_SERVER_UDS"] = str(endpoint)
    else:
        with socket.socket() as reservation:
            reservation.bind(("127.0.0.1", 0))
            env["SCCACHE_SERVER_PORT"] = str(reservation.getsockname()[1])
    return env


def cache_counters(info: dict) -> dict:
    stats = info.get("stats")
    if not isinstance(stats, dict):
        raise RuntimeError("sccache JSON has no stats object")
    result = {}
    for name in ("cache_hits", "cache_misses", "cache_errors"):
        counts = stats.get(name, {}).get("counts", {})
        if not isinstance(counts, dict) or any(not isinstance(v, int) for v in counts.values()):
            raise RuntimeError(f"Unsupported sccache counter format: {name}")
        result[name] = sum(counts.values())
        result["rust_" + name] = sum(v for k, v in counts.items() if k.lower() == "rust")
    for name in ("compile_requests", "requests_not_cacheable", "non_cacheable_compilations",
                 "cache_write_errors", "cache_read_errors", "cache_writes", "compile_fails"):
        result[name] = stats.get(name, 0)
    return result


def cache_storage(directory: Path) -> dict:
    start = time.perf_counter()
    sizes = [path.stat().st_size for path in directory.rglob("*") if path.is_file()]
    return {"bytes": sum(sizes), "files": len(sizes),
            "measurement_seconds": time.perf_counter() - start}


def config_identity(paths: list[Path]) -> list[dict]:
    return [{"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()} for path in paths]


def cargo_prefix(args) -> list[str]:
    command = [args.cargo]
    for path in getattr(args, "cargo_config", []):
        command.extend(["--config", str(path)])
    return command


def cargo_command(args, operation: str = "build") -> list[str]:
    command = cargo_prefix(args) + [operation, "--locked", "--offline", "--lib", "--timings",
                                   "--message-format=json-render-diagnostics", "--profile", args.profile]
    for package in args.package:
        command.extend(["-p", package])
    for feature in args.features:
        command.extend(["--features", feature])
    if args.jobs is not None:
        command.extend(["--jobs", str(args.jobs)])
    return command


def run_build(command: list[str], source: Path, target: Path, env: dict,
              directory: Path, expect_failure: bool = False) -> tuple[dict, list[dict]]:
    directory.mkdir(parents=True, exist_ok=False)
    start = time.perf_counter()
    with (directory / "cargo.jsonl").open("w", encoding="utf-8") as stdout, \
            (directory / "cargo.stderr").open("w", encoding="utf-8") as stderr:
        result = subprocess.run(command, cwd=source, env=env, stdout=stdout, stderr=stderr, check=False)
    elapsed = time.perf_counter() - start
    messages = []
    for line in (directory / "cargo.jsonl").read_text(encoding="utf-8").splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue  # Tests may print text; raw logs retain it.
        if isinstance(message, dict):
            messages.append(message)
    record = {"seconds": elapsed, "command": command, "exit_code": result.returncode,
              "expected_failure": expect_failure}
    timing = target / "cargo-timings" / "cargo-timing.html"
    if timing.is_file():
        shutil.copy2(timing, directory / "cargo-timing.html")
    write_json(directory / "result.json", record)
    finished = any(m.get("reason") == "build-finished" and m.get("success") is True for m in messages)
    if expect_failure:
        failed = any(m.get("reason") == "build-finished" and m.get("success") is False for m in messages)
        # sccache can discover a compile error while asking rustc for dep-info.
        # That diagnostic is sent to stderr instead of Cargo's JSON stream.
        diagnostics = (directory / "cargo.stderr").read_text(encoding="utf-8")
        for message in messages:
            if message.get("reason") == "compiler-message" and isinstance(message.get("message"), dict):
                diagnostics += "\n" + (message["message"].get("rendered") or "")
        if result.returncode == 0 or not failed or finished or "cache_probe_intentional_failure" not in diagnostics:
            raise RuntimeError("Negative control did not fail with the intentional compiler diagnostic")
    else:
        if result.returncode or not finished:
            stderr = (directory / "cargo.stderr").read_text(encoding="utf-8")[-10000:]
            raise RuntimeError(f"Cargo failed or omitted successful build-finished: {directory}\n{stderr}")
        record.update(build_evidence(messages, source, target))
    write_json(directory / "result.json", record)
    return record, messages


def measure(args, root: Path, env: dict, label: str, sample: int) -> dict:
    target = root / "target"
    cleanup = reset_target(target, root)
    use_cache = label != "uncached"
    sample_dir = args.output / f"{label}-{sample}"
    before = time.perf_counter()
    if use_cache:
        output([args.sccache, "--zero-stats"], args.source, env)
    reset_seconds = time.perf_counter() - before
    print(f"{label} {sample}: profile={args.profile}, packages={','.join(args.package)}", flush=True)
    record, _ = run_build(cargo_command(args), args.source, target,
                           build_environment(env, target, args.sccache if use_cache else ""), sample_dir)
    record.update({"label": label, "sample": sample, "target_cleanup_seconds": cleanup,
                   "stats_reset_seconds": reset_seconds, "wrapper": args.sccache if use_cache else None})
    if use_cache:
        info = json.loads(output([args.sccache, "--show-stats", "--stats-format=json"], args.source, env))
        write_json(sample_dir / "sccache-stats.json", info)
        record["cache_counters"] = cache_counters(info)
        record["cache_storage"] = cache_storage(root / "cache")
    write_json(sample_dir / "result.json", record)
    print(f"{label} {sample}: {record['seconds']:.3f}s", flush=True)
    return record


def summarize(records: list[dict]) -> dict:
    grouped = {label: [row for row in records if row["label"] == label]
               for label in ("uncached", "population", "replay")}
    if not all(grouped.values()):
        raise RuntimeError("Missing an uncached, cache-population, or replay result")
    if len(grouped["uncached"]) != len(grouped["replay"]):
        raise RuntimeError("Unbalanced uncached/replay samples")
    if any(row["exit_code"] or not math.isfinite(row["seconds"]) or row["seconds"] <= 0 for row in records):
        raise RuntimeError("A failed or invalid timing cannot contribute to speedup")
    reference = grouped["uncached"][0]
    base_median = statistics.median(row["seconds"] for row in grouped["uncached"])
    replay_median = statistics.median(row["seconds"] for row in grouped["replay"])
    return {
        "uncached_seconds": [row["seconds"] for row in grouped["uncached"]],
        "population_seconds": [row["seconds"] for row in grouped["population"]],
        "replay_seconds": [row["seconds"] for row in grouped["replay"]],
        "uncached_median_seconds": base_median, "replay_median_seconds": replay_median,
        "local_populated_cache_speedup": base_median / replay_median,
        "same_cargo_command": all(row["command"] == reference["command"] for row in records),
        "same_units_features_profiles": all(row["units"] == reference["units"] for row in records),
        "same_generated_rust_and_metadata": all(row["generated"] == reference["generated"] for row in records),
        "generated_files_per_build": sum(len(values) for values in reference["generated"].values()),
        "all_cargo_units_rebuilt": bool(reference["units"]) and
                                  all(not unit["fresh"] for row in records for unit in row["units"]),
        "every_replay_has_rust_cache_hits": all(row["cache_counters"]["rust_cache_hits"] > 0 for row in grouped["replay"]),
        "no_cache_errors": all(not any(row.get("cache_counters", {}).get(key, 0)
                               for key in ("cache_errors", "cache_write_errors", "cache_read_errors")) for row in records),
    }


FIXTURE_BUILD_RS = r'''use std::{env, fs, path::PathBuf};
fn main() {
    println!("cargo:rerun-if-changed=input.txt");
    println!("cargo:rerun-if-env-changed=FERROUI_CACHE_PROBE_OFFSET");
    let value: i32 = fs::read_to_string("input.txt").unwrap().trim().parse().unwrap();
    let offset: i32 = env::var("FERROUI_CACHE_PROBE_OFFSET").unwrap().parse().unwrap();
    let generated = format!("pub const FROM_FILE: i32 = {value};\npub const FROM_ENV: i32 = {offset};\n");
    fs::write(PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("generated.rs"), generated).unwrap();
}
'''


def fixture_library(value: int) -> str:
    return ('include!(concat!(env!("OUT_DIR"), "/generated.rs"));\n'
            f'pub fn source_value() -> i32 {{ {value} }}\n'
            'pub fn marker() -> &\'static str { env!("FERROUI_CACHE_PROBE_MARKER") }\n')


def validate_invalidation(args, root: Path, env: dict) -> dict:
    """Compile/run a real dependency-free Cargo fixture; never edit FerroUI source.

    The rlib is cacheable and the final executable consumes it. Each mutation must
    be observed at runtime. A compiler-error negative control cannot reuse an old
    successful build. These checks are separate from all performance samples.
    """
    started = time.perf_counter()
    source = root / "invalidation-fixture"
    source.mkdir()
    (source / "Cargo.toml").write_text(
        '[workspace]\n[package]\nname="ferroui-cache-probe"\nversion="0.0.0"\nedition="2021"\n'
        '[lib]\npath="lib.rs"\n[[bin]]\nname="ferroui-cache-probe"\npath="main.rs"\n', encoding="utf-8")
    (source / "build.rs").write_text(FIXTURE_BUILD_RS, encoding="utf-8")
    (source / "lib.rs").write_text(fixture_library(10), encoding="utf-8")
    (source / "input.txt").write_text("20\n", encoding="utf-8")
    (source / "main.rs").write_text(
        'fn main() { println!("{}:{}:{}:{}", ferroui_cache_probe::source_value(), '
        'ferroui_cache_probe::FROM_FILE, ferroui_cache_probe::FROM_ENV, ferroui_cache_probe::marker()); }\n',
        encoding="utf-8")
    target = root / "fixture-target"
    probe_env = build_environment(env, target, args.sccache)
    probe_env.update(FERROUI_CACHE_PROBE_OFFSET="30", FERROUI_CACHE_PROBE_MARKER="A")
    output([args.cargo, "generate-lockfile", "--offline"], source, probe_env)
    # The fixture deliberately uses ordinary dev settings. An arbitrary custom
    # profile from FerroUI need not exist in this independent fixture.
    command = [args.cargo, "build", "--locked", "--offline", "--timings",
               "--message-format=json-render-diagnostics", "--profile", "dev"]
    states = [
        ("populate", "10:20:30:A"), ("replay", "10:20:30:A"),
        ("rust-source", "11:20:30:A"), ("generated-input", "11:21:30:A"),
        ("build-environment", "11:21:31:A"), ("compiler-environment", "11:21:31:B"),
        ("build-script-source", "11:21:32:B"), ("compiler-error", None),
        ("restore-valid-source", "11:21:32:B"),
    ]
    records = []
    for label, expected in states:
        if label in {"populate", "replay"}:
            reset_target(target, root)
        elif label in {"rust-source", "restore-valid-source"}:
            (source / "lib.rs").write_text(fixture_library(11), encoding="utf-8")
        elif label == "generated-input":
            (source / "input.txt").write_text("21\n", encoding="utf-8")
        elif label == "build-environment":
            probe_env["FERROUI_CACHE_PROBE_OFFSET"] = "31"
        elif label == "compiler-environment":
            probe_env["FERROUI_CACHE_PROBE_MARKER"] = "B"
        elif label == "build-script-source":
            (source / "build.rs").write_text(FIXTURE_BUILD_RS.replace(
                'let generated = format!', 'let offset = offset + 1;\n    let generated = format!'), encoding="utf-8")
        elif label == "compiler-error":
            (source / "lib.rs").write_text(fixture_library(11) +
                'compile_error!("cache_probe_intentional_failure");\n', encoding="utf-8")
        output([args.sccache, "--zero-stats"], source, probe_env)
        directory = args.output / "invalidation" / label
        record, messages = run_build(command, source, target, probe_env, directory, expected is None)
        record.update(label=label, expected=expected)
        if expected is not None:
            executables = {m["executable"] for m in messages if m.get("reason") == "compiler-artifact"
                           and m.get("executable") and m["target"]["name"] == "ferroui-cache-probe"}
            if len(executables) != 1:
                raise RuntimeError("Fixture executable missing or ambiguous in Cargo JSON")
            executable = Path(executables.pop()).resolve()
            if not executable.is_relative_to(target):
                raise RuntimeError("Fixture executable escaped the isolated target")
            record["actual"] = output([str(executable)], source, probe_env)
            if record["actual"] != expected:
                raise RuntimeError(f"Stale compiler output after {label}: expected {expected}, got {record['actual']}")
        info = json.loads(output([args.sccache, "--show-stats", "--stats-format=json"], source, probe_env))
        write_json(directory / "sccache-stats.json", info)
        record["cache_counters"] = cache_counters(info)
        if label == "replay" and record["cache_counters"]["rust_cache_hits"] < 1:
            raise RuntimeError("Invalidation fixture did not actually reuse a cached Rust artifact")
        write_json(directory / "result.json", record)
        records.append(record)
    shutil.copytree(source, args.output / "invalidation" / "fixture-source")
    return {"passed": True, "seconds": time.perf_counter() - started, "records": records,
            "scope": "isolated Cargo fixture: source, included generated code, environment, build.rs, compile error"}


def markdown_summary(report: dict) -> str:
    summary = report["summary"]
    return (
        "## Local compiler-cache experiment\n\n"
        f"Profile: `{report['profile']}`. Packages: `{', '.join(report['packages'])}`. "
        "Every timed build starts with an empty target directory.\n\n"
        f"Explicit Cargo configuration files (identical on all legs): **{len(report['cargo_configs'])}**; "
        "paths and content hashes are recorded in report.json.\n\n"
        "| No wrapper median | Empty-cache population | Populated-cache median | Local replay speedup |\n"
        "|---:|---:|---:|---:|\n"
        f"| {summary['uncached_median_seconds']:.3f}s | {summary['population_seconds'][0]:.3f}s | "
        f"{summary['replay_median_seconds']:.3f}s | {summary['local_populated_cache_speedup']:.3f}x |\n\n"
        f"Daemon startup: {report['overhead']['daemon_start_seconds']:.3f}s. "
        f"Cache storage after measured builds: {report['measured_cache_storage']['bytes']:,} bytes. "
        f"Generated Rust/metadata equal: **{summary['same_generated_rust_and_metadata']}** "
        f"({summary['generated_files_per_build']} files/build). "
        f"Units/features/profiles equal: **{summary['same_units_features_profiles']}**.\n\n"
        f"Cache invalidation fixture passed: **{report['invalidation']['passed']}**. "
        f"Selected package tests executed here: **{report['tests']['executed']}**.\n\n"
        "The speedup is for a populated local compiler cache. It is not a cold-compiler "
        "speedup or an end-to-end remote CI cache-restore measurement. Installation, "
        "network transfer and cache upload are not measured. Raw samples, cache hit/miss "
        "statistics, cleanup/setup costs, Cargo logs and timing HTML are in the artifact.\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--package", action="append")
    parser.add_argument("--features", action="append", default=[])
    parser.add_argument("--profile", default="dev")
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--jobs", type=int)
    parser.add_argument("--cargo", default="cargo")
    parser.add_argument("--cargo-config", action="append", type=Path, default=[], metavar="FILE",
                        help="Extra Cargo config file for every real-graph leg; repeatable, paths relative to --source")
    parser.add_argument("--sccache", default="sccache")
    parser.add_argument("--scratch-root", type=Path, help="Parent of owned temporary targets/cache, never a directory to clean")
    parser.add_argument("--skip-tests", action="store_true", help="Run package tests in a separate correctness job; fixture checks still run")
    parser.add_argument("--minimum-speedup", type=float, default=0,
                        help="Explicit local populated-cache acceptance target; default records evidence without a speed claim")
    args = parser.parse_args()
    if args.repetitions < 1 or (args.jobs is not None and args.jobs < 1):
        parser.error("repetitions and jobs must be positive")
    if not math.isfinite(args.minimum_speedup) or args.minimum_speedup < 0:
        parser.error("minimum-speedup must be finite and nonnegative")
    args.package = args.package or ["ferroui-controls"]
    args.source = args.source.resolve()
    args.output = args.output.resolve()
    args.cargo_config = [(path if path.is_absolute() else args.source / path).resolve() for path in args.cargo_config]
    for path in args.cargo_config:
        if not path.is_file():
            parser.error(f"Cargo configuration file is missing or not a regular file: {path}")
    for name in ("cargo", "sccache"):
        executable = shutil.which(getattr(args, name))
        if executable is None:
            parser.error(f"{name} executable is missing; install it explicitly before running")
        setattr(args, name, str(Path(executable).absolute()))
    args.output.mkdir(parents=True, exist_ok=False)
    report = {"schema_version": 1, "scenario": "empty-target-local-populated-compiler-cache",
              "profile": args.profile, "packages": args.package, "features": args.features,
              "cargo_configs": config_identity(args.cargo_config),
              "repetitions": args.repetitions, "platform": platform.platform(), "cpu_count": os.cpu_count(),
              "toolchain": output([os.environ.get("RUSTC", "rustc"), "-Vv"], args.source),
              "cargo": output([args.cargo, "-Vv"], args.source),
              "sccache": output([args.sccache, "--version"], args.source),
              "commit": output(["git", "rev-parse", "HEAD"], args.source),
              "lock_sha256": hashlib.sha256((args.source / "Cargo.lock").read_bytes()).hexdigest(),
              "source_sha256": source_fingerprint(args.source, args.output),
              "environment": {key: value for key, value in os.environ.items()
                              if key.startswith("CARGO_PROFILE_") or key in
                              {"RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_BUILD_JOBS", "RUSTC"}},
              "effective_incremental": False, "records": [], "overhead": {}}

    def save() -> None:
        write_json(args.output / "report.json", report)

    save()
    try:
        with ExitStack() as stack:
            root = Path(stack.enter_context(tempfile.TemporaryDirectory(prefix="ferroui-cache-perf-", dir=args.scratch_root))).resolve()
            control = Path(stack.enter_context(tempfile.TemporaryDirectory(prefix="fcpc-"))).resolve()
            env = isolated_cache_environment(dict(os.environ), root, control)
            fetch_started = time.perf_counter()
            output(cargo_prefix(args) + ["fetch", "--locked"], args.source, build_environment(env, root / "target", ""))
            report["overhead"]["fetch_seconds"] = time.perf_counter() - fetch_started
            report["records"].append(measure(args, root, env, "uncached", 1))
            save()
            daemon_started = time.perf_counter()
            output([args.sccache, "--start-server"], args.source, env)
            report["overhead"]["daemon_start_seconds"] = time.perf_counter() - daemon_started
            try:
                initial_info = json.loads(output([args.sccache, "--show-stats", "--stats-format=json"], args.source, env))
                if cache_counters(initial_info)["compile_requests"] != 0:
                    raise RuntimeError("The benchmark daemon is not empty; refusing mixed cache evidence")
                report["records"].append(measure(args, root, env, "population", 1))
                save()
                report["records"].append(measure(args, root, env, "replay", 1))
                save()
                for sample in range(2, args.repetitions + 1):
                    order = ["replay", "uncached"] if sample % 2 == 0 else ["uncached", "replay"]
                    for label in order:
                        report["records"].append(measure(args, root, env, label, sample))
                        save()
                report["summary"] = summary = summarize(report["records"])
                report["measured_cache_storage"] = cache_storage(root / "cache")
                save()
                required = ("same_cargo_command", "same_units_features_profiles", "same_generated_rust_and_metadata",
                            "all_cargo_units_rebuilt", "every_replay_has_rust_cache_hits", "no_cache_errors")
                failed = [key for key in required if not summary[key]]
                if failed:
                    raise RuntimeError("Invalid cache comparison: " + ", ".join(failed))
                report["invalidation"] = validate_invalidation(args, root, env)
                save()
                report["tests"] = {"executed": not args.skip_tests}
                if args.skip_tests:
                    report["tests"]["reason"] = "Explicit --skip-tests: run identical package tests in a separate correctness job"
                else:
                    test_record, _ = run_build(cargo_command(args, "test"), args.source, root / "target",
                                               build_environment(env, root / "target", args.sccache), args.output / "tests")
                    report["tests"]["result"] = test_record
                report["source_unchanged"] = source_fingerprint(args.source, args.output) == report["source_sha256"]
                if not report["source_unchanged"]:
                    raise RuntimeError("Source tree changed during the comparison; results are not comparable")
                report["cargo_configs_unchanged"] = config_identity(args.cargo_config) == report["cargo_configs"]
                if not report["cargo_configs_unchanged"]:
                    raise RuntimeError("Cargo configuration changed during the comparison; results are not comparable")
            finally:
                stop_started = time.perf_counter()
                try:
                    output([args.sccache, "--stop-server"], args.source, env)
                finally:
                    report["overhead"]["daemon_stop_seconds"] = time.perf_counter() - stop_started
                    save()
        text = markdown_summary(report)
        (args.output / "summary.md").write_text(text, encoding="utf-8")
        print(text)
        if os.environ.get("GITHUB_STEP_SUMMARY"):
            with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as handle:
                handle.write(text)
        measured = report["summary"]["local_populated_cache_speedup"]
        if measured < args.minimum_speedup:
            raise RuntimeError(f"Measured local-cache speedup {measured:.3f}x is below requested {args.minimum_speedup:.3f}x")
        return 0
    except (RuntimeError, OSError, ValueError, subprocess.CalledProcessError) as error:
        report["error"] = str(error)
        save()
        print(error, file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
