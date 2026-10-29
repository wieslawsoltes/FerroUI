#!/usr/bin/env python3
"""Measure the first unchanged catalog build after newly generated catalog output.

Each sample is a population -> immediate-repeat pair of the real default-feature
`cargo build --lib -p control-catalog`. The first pair starts with an empty target;
later pairs first run `cargo clean -p control-catalog`, retaining dependencies.
Cleanup and population costs are reported separately and never enter the repeat
speedup. Baseline dirtiness and the original generated-file watch form are
reported. --require-baseline-self-input additionally requires the historical
placeholder/App.xaml failure for an isolated experiment. A baseline already
containing the fix is a clean control. Every candidate artifact must be fresh.
Noncatalog baseline dirtiness is fully reported: a whole-PR comparison may also
include the separate SampleBuild fix, while an EA26 baseline isolates this fix.

Cargo's public JSON supplies units and OUT_DIR locations. The harness preserves
raw logs, timing HTML, generated files, and provenance. It compares generated
Rust/metadata throughout the graph and every first-party OUT_DIR file. Exactly
catalog assets.rs has a permitted representation change: an independent Rust
program includes it with cfg(test) and serializes all four tables, whose complete
bytes must match. The exact xamlmeta-scan.txt timing diagnostic is recorded but
excluded from content equivalence. Only actual source/target/OUT_DIR locations
are normalized; asset bytes and table metadata are never normalized.
"""
from __future__ import annotations

import argparse
import filecmp
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import platform
import re
import shutil
import statistics
import struct
import subprocess
import sys
import tempfile
import time


PACKAGE = "control-catalog"
PROBE_MAGIC = b"FerroUI catalog asset tables v1\0"
TIMING_DIAGNOSTIC = "xamlmeta-scan.txt"
WRAPPERS = ("RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "CARGO_BUILD_RUSTC_WRAPPER", "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER")


def load_helpers(candidate: Path):
    """Use the adjacent shared helpers after adoption; allow the isolated prototype."""
    path = Path(__file__).with_name("compare.py")
    if not path.is_file():
        path = candidate / "scripts/build-perf/compare.py"
    spec = importlib.util.spec_from_file_location("catalog_compare", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module, path


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def write_json(path: Path, value) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def capture(command: list[str], cwd: Path, env: dict, directory: Path,
            stdout_name: str = "stdout", stderr_name: str = "stderr") -> dict:
    directory.mkdir(parents=True, exist_ok=True)
    record = {"command": command, "cwd": str(cwd), "stdout": str(directory / stdout_name),
              "stderr": str(directory / stderr_name)}
    write_json(directory / "command.json", record)
    started = time.perf_counter()
    with (directory / stdout_name).open("wb") as stdout, (directory / stderr_name).open("wb") as stderr:
        result = subprocess.run(command, cwd=cwd, env=env, stdout=stdout, stderr=stderr)
    record.update(seconds=time.perf_counter() - started, exit_code=result.returncode)
    write_json(directory / "command.json", record)
    if result.returncode:
        tail = (directory / stderr_name).read_text(encoding="utf-8", errors="replace")[-8000:]
        raise RuntimeError(f"command failed ({result.returncode}); see {directory}\n{tail}")
    return record


def json_messages(path: Path) -> list[dict]:
    messages = []
    for line in path.read_text(encoding="utf-8").splitlines():
        try:
            value = json.loads(line)
            if isinstance(value, dict):
                messages.append(value)
        except json.JSONDecodeError:
            pass  # The raw line is preserved, never interpreted as success.
    return messages


def source_files(source: Path) -> dict:
    listed = subprocess.check_output(["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"], cwd=source)
    result = {}
    for name in sorted(set(listed.decode("utf-8").split("\0")) - {""}):
        path = source / name
        if path.is_symlink():
            result[name] = {"kind": "symlink", "target": os.readlink(path),
                            "sha256": sha256(path) if path.is_file() else None}
        elif path.is_file():
            result[name] = {"kind": "file", "sha256": sha256(path)}
        else:
            result[name] = {"kind": "missing"}
    return result


def source_configuration(files: dict) -> dict:
    return {name: entry for name, entry in files.items()
            if Path(name).name in {"Cargo.toml", "Cargo.lock", "rust-toolchain", "rust-toolchain.toml"}
            or name in {".cargo/config", ".cargo/config.toml"}}


def original_self_input_form(source: str) -> bool:
    """Recognize the reviewed unconditional watch; never infer it from a version."""
    substitution = "substitute_placeholder_geometries(&root, &out_dir, &mut assets);" in source
    watch = re.search(r'for\s*\(asset_path,\s*path\)\s*in\s*&assets\s*\{\s*'
                      r'let path = path\.canonicalize\(\).*?;\s*'
                      r'println!\("cargo::rerun-if-changed=\{\}",\s*path\.display\(\)\);', source)
    return substitution and watch is not None


def external_configurations(source: Path, env: dict) -> dict:
    """Record config hashes without exposing possible credential values."""
    locations = [parent / ".cargo" for parent in source.parents]
    locations.append(Path(env.get("CARGO_HOME", str(Path.home() / ".cargo"))).resolve())
    return {str(path): sha256(path) for directory in locations for name in ("config", "config.toml")
            if (path := directory / name).is_file()}


def definitions(record: dict) -> list[dict]:
    return sorted(({key: value for key, value in unit.items() if key != "fresh"}
                   for unit in record["units"]), key=lambda value: json.dumps(value, sort_keys=True))


def catalog_library(record: dict, package_id: str) -> dict:
    found = [unit for unit in record["units"] if unit["package"] == package_id
             and unit["name"] == "control_catalog" and "lib" in unit["kind"]]
    if len(found) != 1:
        raise RuntimeError(f"expected exactly one catalog library artifact, found {len(found)}")
    return found[0]


def freshness_checks(record: dict, package_id: str, label: str, phase: str,
                     require_baseline_self_input: bool = False) -> dict:
    catalog = catalog_library(record, package_id)
    if "placeholder-branding" not in catalog["features"] or "runtime-markup" in catalog["features"]:
        raise RuntimeError("benchmark must use the real compiled catalog with default placeholder branding")
    other_dirty = [unit for unit in record["units"] if unit["package"] != package_id and not unit["fresh"]]
    result = {"catalog_library_fresh": catalog["fresh"],
              "all_artifacts_fresh": bool(record["units"]) and all(unit["fresh"] for unit in record["units"]),
              "non_catalog_dirty_artifacts": other_dirty}
    if phase == "population":
        if catalog["fresh"]:
            raise RuntimeError("population reused the catalog; the sample did not create new catalog output")
    elif label == "candidate":
        if not result["all_artifacts_fresh"]:
            raise RuntimeError("candidate's first repeat contains stale artifacts")
    else:
        result["self_output_dirty_reason"] = any(
            line.startswith("Dirty control-catalog ") and "placeholder/App.xaml" in line.replace("\\", "/")
            for line in record["dirty_reasons"])
        if require_baseline_self_input and (catalog["fresh"] or not result["self_output_dirty_reason"]):
            raise RuntimeError("baseline rebuilt without the expected placeholder/App.xaml self-input reason")
    return result


def build(source: Path, target: Path, destination: Path, env: dict, jobs: int | None,
          helpers, package_id: str, label: str, phase: str,
          require_baseline_self_input: bool = False) -> tuple[dict, list[dict]]:
    command = ["cargo", "build", "--locked", "--offline", "--timings",
               "--message-format=json-render-diagnostics", "--profile", "dev",
               "--verbose", "--verbose", "-p", PACKAGE, "--lib"]
    if jobs is not None:
        command.extend(["--jobs", str(jobs)])
    print(f"{label} {destination.parent.name} {phase}: {' '.join(command)}", flush=True)
    record = capture(command, source, env, destination, "cargo.jsonl", "cargo.stderr")
    messages = json_messages(destination / "cargo.jsonl")
    if not any(message.get("reason") == "build-finished" and message.get("success") for message in messages):
        raise RuntimeError(f"no successful Cargo build-finished message; see {destination}")
    record["units"] = helpers.units(messages, source, target)
    record["dirty_reasons"] = [helpers.normalized(line.strip(), source, target)
                               for line in (destination / "cargo.stderr").read_text(encoding="utf-8").splitlines()
                               if line.lstrip().startswith("Dirty ")]
    record["freshness"] = freshness_checks(record, package_id, label, phase, require_baseline_self_input)
    timing = target / "cargo-timings/cargo-timing.html"
    if not timing.is_file():
        raise RuntimeError(f"Cargo did not write timing HTML; see {destination}")
    shutil.copyfile(timing, destination / "cargo-timing.html")
    write_json(destination / "result.json", record)
    print(f"{label} {phase}: {record['seconds']:.3f}s; all artifacts fresh: {record['freshness']['all_artifacts_fresh']}", flush=True)
    return record, messages


def output_locations(messages: list[dict], source: Path, target: Path, helpers) -> list[tuple[bytes, bytes]]:
    locations = []
    for message in messages:
        if message.get("reason") == "build-script-executed":
            directory = Path(message["out_dir"])
            package = helpers.normalized(message["package_id"], source, target)
            locations.extend((spelling.encode(), ("$OUT/" + package).encode())
                             for spelling in {str(directory), directory.as_posix(), str(directory).replace("\\", "\\\\")})
    locations.extend((spelling.encode(), replacement.encode()) for path, replacement in ((source, "$SOURCE"), (target, "$TARGET"))
                     for spelling in {str(path), path.as_posix(), str(path).replace("\\", "\\\\")})
    return sorted(set(locations), key=lambda pair: -len(pair[0]))


def normalize_locations(content: bytes, locations: list[tuple[bytes, bytes]]) -> bytes:
    for location, replacement in locations:
        content = content.replace(location, replacement)
    return content


def file_role(package_id: str, catalog_id: str, relative: str) -> str:
    if relative == TIMING_DIAGNOSTIC:
        return "timing-diagnostic"
    if package_id == catalog_id and relative == "assets.rs":
        return "catalog-assets-representation-checked-by-probe"
    if relative.endswith((".rs", ".xamlmeta", ".map.json", ".xaml")):
        return "generated-semantic-output"
    return "additional-first-party-output-compared"


def snapshot_outputs(messages: list[dict], source: Path, target: Path, destination: Path,
                     metadata: dict, catalog_raw_id: str, helpers) -> dict:
    """Preserve all first-party bytes; compare them after location-only normalization."""
    destination.mkdir(parents=True)
    workspace = set(metadata["workspace_members"])
    catalog_id = helpers.normalized(catalog_raw_id, source, target)
    locations = output_locations(messages, source, target, helpers)
    generated = helpers.generated_outputs(messages, source, target)
    representation_key = catalog_id + "/assets.rs"
    if len(generated.pop(representation_key, [])) != 1:
        raise RuntimeError("expected exactly one catalog assets.rs representation exception")
    manifest, comparable, unknown = [], {}, []
    catalog_directory = None
    seen = set()
    for message in messages:
        if message.get("reason") != "build-script-executed" or message["package_id"] not in workspace:
            continue
        directory = Path(message["out_dir"])
        package = helpers.normalized(message["package_id"], source, target)
        if (package, directory) in seen:
            raise RuntimeError("duplicate first-party OUT_DIR message would obscure variant multiplicity")
        seen.add((package, directory))
        slot = len(seen)
        if message["package_id"] == catalog_raw_id:
            if catalog_directory is not None:
                raise RuntimeError("multiple catalog OUT_DIRs; this benchmark requires one native library variant")
            catalog_directory = directory
        for path in sorted(directory.rglob("*")):
            if path.is_symlink():
                raise RuntimeError(f"unexpected generated symlink requires explicit review: {path}")
            if not path.is_file():
                continue
            relative = path.relative_to(directory).as_posix()
            key = package + "/" + relative
            content = path.read_bytes()
            normalized = normalize_locations(content, locations)
            raw = destination / "raw" / str(slot) / relative
            normalized_path = destination / "normalized" / str(slot) / relative
            raw.parent.mkdir(parents=True, exist_ok=True)
            normalized_path.parent.mkdir(parents=True, exist_ok=True)
            raw.write_bytes(content)
            normalized_path.write_bytes(normalized)
            role = file_role(package, catalog_id, relative)
            entry = {"key": key, "package": package, "relative_path": relative, "role": role,
                     "bytes": len(content), "sha256": hashlib.sha256(content).hexdigest(),
                     "normalized_sha256": hashlib.sha256(normalized).hexdigest(),
                     "raw_file": str(raw), "normalized_file": str(normalized_path)}
            manifest.append(entry)
            if role not in {"timing-diagnostic", "catalog-assets-representation-checked-by-probe"}:
                comparable.setdefault(key, []).append(entry["normalized_sha256"])
            if role == "additional-first-party-output-compared":
                unknown.append(key)
    if catalog_directory is None:
        raise RuntimeError("Cargo provided no catalog OUT_DIR")
    catalog_manifest = [entry for entry in manifest if entry["package"] == catalog_id]
    if not any(entry["relative_path"] == "placeholder/App.xaml" for entry in catalog_manifest):
        raise RuntimeError("default catalog generated no placeholder/App.xaml evidence")
    if not any(entry["relative_path"].endswith(".xamlmeta") for entry in catalog_manifest):
        raise RuntimeError("compiled catalog generated no .xamlmeta evidence")
    result = {"generated_rust_and_metadata": generated,
              "first_party_semantic_files": {key: sorted(values) for key, values in sorted(comparable.items())},
              "files": manifest, "additional_first_party_outputs": unknown,
              "catalog_out_dir": str(catalog_directory), "catalog_assets_rs": str(catalog_directory / "assets.rs"),
              "representation_exception": representation_key,
              "timing_diagnostics": [entry["key"] for entry in manifest if entry["role"] == "timing-diagnostic"]}
    write_json(destination / "manifest.json", result)
    return result


def rust_literal(value: str) -> str:
    delimiter = "#"
    while '"' + delimiter in value:
        delimiter += "#"
    return "r" + delimiter + '"' + value + '"' + delimiter


def probe_source(assets: Path) -> str:
    return '''use std::io::{self, Write};
struct ExcludedDocument { path: &'static str, page_only: bool, reason: &'static str }
include!(ASSETS_PATH);
fn count(out: &mut impl Write, value: usize) { out.write_all(&(value as u64).to_le_bytes()).unwrap(); }
fn bytes(out: &mut impl Write, value: &[u8]) { count(out, value.len()); out.write_all(value).unwrap(); }
fn main() {
    assert!(!ASSETS.is_empty() && !DOCUMENT_ASSETS.is_empty() && !DOCUMENTS.is_empty());
    assert_eq!(ASSETS.iter().chain(DOCUMENT_ASSETS.iter()).filter(|(name, _)| *name == "/App.xaml").count(), 1);
    assert_eq!(DOCUMENTS.iter().filter(|(name, _)| *name == "/App.xaml").count(), 1);
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    out.write_all(b"FerroUI catalog asset tables v1\\0").unwrap();
    for (name, table) in [("ASSETS", ASSETS), ("DOCUMENT_ASSETS", DOCUMENT_ASSETS)] {
        bytes(&mut out, name.as_bytes()); count(&mut out, table.len());
        for (path, value) in table { bytes(&mut out, path.as_bytes()); bytes(&mut out, value); }
    }
    bytes(&mut out, b"DOCUMENTS"); count(&mut out, DOCUMENTS.len());
    for (path, class) in DOCUMENTS {
        bytes(&mut out, path.as_bytes());
        match class { Some(value) => { out.write_all(&[1]).unwrap(); bytes(&mut out, value.as_bytes()); },
                      None => out.write_all(&[0]).unwrap() }
    }
    bytes(&mut out, b"EXCLUDED"); count(&mut out, EXCLUDED.len());
    for entry in EXCLUDED {
        bytes(&mut out, entry.path.as_bytes()); out.write_all(&[u8::from(entry.page_only)]).unwrap();
        bytes(&mut out, entry.reason.as_bytes());
    }
    out.flush().unwrap();
}
'''.replace("ASSETS_PATH", rust_literal(str(assets)))


def inspect_probe(path: Path) -> dict:
    """Parse the independent serialization to reject truncated/vacuous evidence."""
    data = path.read_bytes()
    if not data.startswith(PROBE_MAGIC):
        raise RuntimeError("asset probe has the wrong binary format")
    offset = len(PROBE_MAGIC)

    def take(size):
        nonlocal offset
        if size < 0 or offset + size > len(data):
            raise RuntimeError("asset probe is truncated")
        value = data[offset:offset + size]
        offset += size
        return value

    def count():
        return struct.unpack("<Q", take(8))[0]

    def blob():
        return take(count())

    tables = {}
    for expected in (b"ASSETS", b"DOCUMENT_ASSETS", b"DOCUMENTS", b"EXCLUDED"):
        if blob() != expected:
            raise RuntimeError("asset probe table order/name changed")
        entries = count()
        if entries > len(data):
            raise RuntimeError("asset probe has an impossible table count")
        app_count, payload_bytes = 0, 0
        for _ in range(entries):
            name = blob()
            name.decode("utf-8")
            app_count += name == b"/App.xaml"
            if expected in (b"ASSETS", b"DOCUMENT_ASSETS"):
                payload_bytes += len(blob())
            else:
                tag = take(1)
                if tag not in (b"\0", b"\1"):
                    raise RuntimeError("invalid option/bool in asset probe")
                if expected == b"EXCLUDED" or tag == b"\1":
                    blob().decode("utf-8")
        tables[expected.decode()] = {"entries": entries, "app_xaml_entries": app_count,
                                    "asset_payload_bytes": payload_bytes}
    if offset != len(data):
        raise RuntimeError("asset probe has trailing unexamined bytes")
    if not all(tables[name]["entries"] for name in ("ASSETS", "DOCUMENT_ASSETS", "DOCUMENTS")):
        raise RuntimeError("asset probe has an empty required table")
    if tables["ASSETS"]["app_xaml_entries"] + tables["DOCUMENT_ASSETS"]["app_xaml_entries"] != 1 or tables["DOCUMENTS"]["app_xaml_entries"] != 1:
        raise RuntimeError("asset probe does not cover exactly one App.xaml")
    return {"sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data), "tables": tables}


def run_probe(assets: Path, destination: Path, source: Path, env: dict, rustc: str) -> dict:
    destination.mkdir(parents=True)
    code = destination / "probe.rs"
    code.write_text(probe_source(assets), encoding="utf-8")
    executable = destination / ("probe.exe" if os.name == "nt" else "probe")
    compiler = capture([rustc, "--edition=2024", "--cfg", "test", "--crate-name", "catalog_asset_probe",
                        str(code), "-o", str(executable)], source, env, destination / "compile")
    runtime = capture([str(executable)], source, env, destination / "run", "tables.bin")
    binary = destination / "run/tables.bin"
    result = {"compile": compiler, "run": runtime, "binary_file": str(binary), **inspect_probe(binary)}
    write_json(destination / "result.json", result)
    executable.unlink()  # Raw source, compile command, logs, and complete semantic bytes are preserved.
    return result


def compare_evidence(reference: dict, candidate: dict) -> dict:
    result = {
        "same_units_features_profiles": all(definitions(candidate[phase]) == definitions(reference["population"])
                                            for phase in ("population", "repeat")),
        "same_generated_rust_and_metadata": candidate["outputs"]["generated_rust_and_metadata"] == reference["outputs"]["generated_rust_and_metadata"],
        "same_first_party_semantic_files": candidate["outputs"]["first_party_semantic_files"] == reference["outputs"]["first_party_semantic_files"],
        "same_asset_table_bytes": filecmp.cmp(reference["asset_probe"]["binary_file"], candidate["asset_probe"]["binary_file"], shallow=False),
    }
    return result


def summarize(records: list[dict]) -> dict:
    baseline = [row for row in records if row["label"] == "baseline"]
    candidate = [row for row in records if row["label"] == "candidate"]
    base_times = [row["repeat"]["seconds"] for row in baseline]
    candidate_times = [row["repeat"]["seconds"] for row in candidate]
    reference = records[0]
    equivalence = [compare_evidence(reference, row) for row in records]
    return {"baseline_repeat_seconds": base_times, "candidate_repeat_seconds": candidate_times,
            "baseline_repeat_median_seconds": statistics.median(base_times),
            "candidate_repeat_median_seconds": statistics.median(candidate_times),
            "first_repeat_speedup": statistics.median(base_times) / statistics.median(candidate_times),
            "baseline_population_seconds": [row["population"]["seconds"] for row in baseline],
            "candidate_population_seconds": [row["population"]["seconds"] for row in candidate],
            "baseline_cleanup_seconds": [row["cleanup"]["seconds"] for row in baseline if row["cleanup"]],
            "candidate_cleanup_seconds": [row["cleanup"]["seconds"] for row in candidate if row["cleanup"]],
            **{name: all(row[name] for row in equivalence) for name in equivalence[0]},
            "candidate_every_repeat_all_fresh": all(row["repeat"]["freshness"]["all_artifacts_fresh"] for row in candidate),
            "baseline_every_repeat_catalog_self_input_dirty": all(row["repeat"]["freshness"]["self_output_dirty_reason"] for row in baseline),
            "baseline_every_repeat_all_fresh": all(row["repeat"]["freshness"]["all_artifacts_fresh"] for row in baseline),
            "baseline_non_catalog_dirty_artifacts": [row["repeat"]["freshness"]["non_catalog_dirty_artifacts"] for row in baseline],
            "compilation_artifacts_per_build": len(reference["population"]["units"]),
            "generated_rust_and_metadata_files": sum(map(len, reference["outputs"]["generated_rust_and_metadata"].values())),
            "first_party_semantic_files": sum(map(len, reference["outputs"]["first_party_semantic_files"].values())),
            "asset_tables": reference["asset_probe"]["tables"],
            "asset_serialization_bytes": reference["asset_probe"]["bytes"],
            "additional_first_party_outputs": sorted(set(key for row in records for key in row["outputs"]["additional_first_party_outputs"]))}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", required=True, type=Path)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--jobs", type=int)
    parser.add_argument("--target-root", type=Path, help="Parent for owned temporary targets; never a directory to clean")
    parser.add_argument("--keep-targets", action="store_true", help="Keep the owned targets after success for local investigation")
    parser.add_argument("--offline", action="store_true", help="Skip untimed cargo fetch; timed commands are always offline")
    parser.add_argument("--require-baseline-self-input", action="store_true",
                        help="Require the historical placeholder/App.xaml dirty reason; use with a known isolated baseline")
    parser.add_argument("--minimum-speedup", type=float, default=0,
                        help="Acceptance gate for first repeats only; requires at least three samples when nonzero")
    args = parser.parse_args()
    if args.repetitions < 1 or (args.jobs is not None and args.jobs < 1):
        parser.error("repetitions and jobs must be positive")
    if not math.isfinite(args.minimum_speedup) or args.minimum_speedup < 0:
        parser.error("minimum-speedup must be finite and nonnegative")
    if args.minimum_speedup and args.repetitions < 3:
        parser.error("a speedup acceptance gate requires at least three independent population/repeat pairs")
    sources = {"baseline": args.baseline.resolve(), "candidate": args.candidate.resolve()}
    if sources["baseline"] == sources["candidate"]:
        parser.error("baseline and candidate must be independent source directories")
    output = args.output.resolve()
    if any(output == source or source in output.parents for source in sources.values()):
        parser.error("output must be outside both source trees")
    helpers, helper_path = load_helpers(sources["candidate"])
    output.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ, CARGO_TERM_COLOR="never", **{name: "" for name in WRAPPERS})
    if any(os.environ.get(name) for name in WRAPPERS):
        parser.error("compiler cache wrappers must be disabled before this comparison")
    rustc = env.get("RUSTC", "rustc")
    report = {"schema_version": 1, "scenario": "catalog-first-repeat-after-new-output",
              "package": PACKAGE, "profile": "dev", "default_features": True,
              "explicit_build_tool_optimization": False, "repetitions": args.repetitions,
              "requested_minimum_first_repeat_speedup": args.minimum_speedup,
              "require_baseline_self_input": args.require_baseline_self_input,
              "harness_sha256": sha256(Path(__file__)), "compare_helper_sha256": sha256(helper_path),
              "platform": platform.platform(), "cpu_count": os.cpu_count(),
              "environment": {key: value for key, value in env.items()
                              if key.startswith("CARGO_PROFILE_") or key in {
                                  "CARGO_INCREMENTAL", "CARGO_BUILD_JOBS", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC",
                                  "CARGO_BUILD_TARGET", "CARGO_HOME", "RUSTUP_HOME", "RUSTUP_TOOLCHAIN",
                                  "SKIA_BINARIES_URL", "SKIA_USE_SYSTEM_LIBRARIES", "CC", "CXX", "CFLAGS", "CXXFLAGS", "LDFLAGS",
                                  "RUNNER_OS", "RUNNER_ARCH", "GITHUB_SHA", "GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT"}},
              "sources": {}, "records": [],
              "normalization": "Only actual checkout, target, and Cargo-reported OUT_DIR locations; no asset table normalization.",
              "exceptions": {"control-catalog/assets.rs": "Exact representation change; complete ASSETS, DOCUMENT_ASSETS, DOCUMENTS, EXCLUDED bytes compared by Rust probe.",
                             "*/OUT_DIR/xamlmeta-scan.txt": "Exact root-level build-scanner timing diagnostic; bytes preserved and inventoried."}}
    temporary = None
    success = False

    def save():
        write_json(output / "report.json", report)

    save()
    try:
        for command, name in (([rustc, "-Vv"], "rustc"), (["cargo", "-V"], "cargo")):
            values = []
            for label, source in sources.items():
                record = capture(command, source, env, output / "provenance" / label / name)
                values.append(Path(record["stdout"]).read_text(encoding="utf-8").strip())
            if len(set(values)) != 1:
                raise RuntimeError("both checkouts must use exactly the same Cargo and rustc")
            report[name] = values[0]
        for label, source in sources.items():
            files = source_files(source)
            provenance = output / "provenance" / label
            write_json(provenance / "source-files-before.json", files)
            capture(["git", "diff", "--binary", "HEAD"], source, env, provenance / "working-diff")
            report["sources"][label] = {**helpers.identity(source), "files": files,
                                         "external_configurations": external_configurations(source, env)}
        report["baseline_original_self_input_form"] = original_self_input_form(
            (sources["baseline"] / "samples/ControlCatalog/build.rs").read_text(encoding="utf-8"))
        if source_configuration(report["sources"]["baseline"]["files"]) != source_configuration(report["sources"]["candidate"]["files"]):
            raise RuntimeError("Cargo manifests, lockfile or checked-in build/toolchain configuration differ")
        catalog_inputs = {label: {name: entry for name, entry in report["sources"][label]["files"].items()
                                  if name.startswith("samples/ControlCatalog/") and name != "samples/ControlCatalog/build.rs"}
                          for label in sources}
        if catalog_inputs["baseline"] != catalog_inputs["candidate"]:
            raise RuntimeError("catalog source/assets differ beyond the intended build.rs change")
        if not args.offline:
            for label, source in sources.items():
                capture(["cargo", "fetch", "--locked"], source, env, output / "provenance" / label / "fetch")
        temporary = Path(tempfile.mkdtemp(prefix="ferroui-catalog-freshness-", dir=args.target_root)).resolve()
        report["owned_target_parent"] = str(temporary)
        targets = {label: temporary / ("target-" + label) for label in sources}
        environments = {label: dict(env, CARGO_TARGET_DIR=str(target), CARGO_BUILD_BUILD_DIR=str(target))
                        for label, target in targets.items()}
        metadata, package_ids = {}, {}
        for label, source in sources.items():
            command = capture(["cargo", "metadata", "--locked", "--offline", "--format-version=1"], source,
                              environments[label], output / "provenance" / label / "metadata", "metadata.json")
            metadata[label] = json.loads(Path(command["stdout"]).read_text(encoding="utf-8"))
            found = [package["id"] for package in metadata[label]["packages"] if package["name"] == PACKAGE
                     and package["id"] in metadata[label]["workspace_members"]]
            if len(found) != 1:
                raise RuntimeError("Cargo metadata did not identify one workspace catalog package")
            package_ids[label] = found[0]
        save()
        for sample in range(1, args.repetitions + 1):
            order = list(sources.items())
            if sample % 2 == 0:
                order.reverse()
            for label, source in order:
                target, build_env = targets[label], environments[label]
                directory = output / f"sample-{sample}" / label
                pair = {"sample": sample, "label": label, "target": str(target), "cleanup": None}
                report["records"].append(pair)
                if sample > 1:
                    pair["cleanup"] = capture(["cargo", "clean", "--offline", "--profile", "dev", "-p", PACKAGE],
                                              source, build_env, directory / "cleanup")
                normalized_id = helpers.normalized(package_ids[label], source, target)
                pair["population"], _ = build(source, target, directory / "population", build_env, args.jobs,
                                              helpers, normalized_id, label, "population", args.require_baseline_self_input)
                # No Cargo query, probe, source mutation, or output snapshot intervenes.
                # Only raw log/HTML capture and freshness checks precede the first repeat.
                pair["repeat"], messages = build(source, target, directory / "repeat", build_env, args.jobs,
                                                 helpers, normalized_id, label, "repeat", args.require_baseline_self_input)
                save()
                pair["outputs"] = snapshot_outputs(messages, source, target, directory / "generated", metadata[label],
                                                    package_ids[label], helpers)
                pair["asset_probe"] = run_probe(Path(pair["outputs"]["catalog_assets_rs"]), directory / "asset-probe", source, build_env, rustc)
                pair["equivalence_to_first_pair"] = compare_evidence(report["records"][0], pair)
                save()
                if not all(pair["equivalence_to_first_pair"].values()):
                    raise RuntimeError(f"{label} sample {sample} changed graph, generated output, or full asset table bytes")
        for label, source in sources.items():
            files = source_files(source)
            write_json(output / "provenance" / label / "source-files-after.json", files)
            if files != report["sources"][label]["files"] or external_configurations(source, env) != report["sources"][label]["external_configurations"]:
                raise RuntimeError(f"{label} source/configuration changed during measurement")
        report["summary"] = summary = summarize(report["records"])
        report["source_inputs_unchanged"] = True
        summary["speedup_gate_requested"] = args.minimum_speedup > 0
        summary["attribution"] = (
            "Baseline already fresh; this is a clean control. Any explicitly requested speedup gate still applies."
            if summary["baseline_every_repeat_all_fresh"] else
            "Whole-PR first-repeat comparison: baseline also rebuilds other packages; this ratio includes their fixes."
            if any(summary["baseline_non_catalog_dirty_artifacts"]) else
            "Catalog-only first-repeat comparison: every noncatalog package is fresh in both revisions.")
        text = ("## Catalog first-repeat comparison\n\n"
                "Workload: `cargo build --locked --offline --profile dev -p control-catalog --lib`, default features. "
                f"{args.repetitions} population → first-repeat pairs per revision. Every pair creates new catalog output; "
                "later pairs clean only `control-catalog`, retaining dependencies. Cleanup and population costs are recorded separately.\n\n"
                "| Baseline first-repeat median | Candidate first-repeat median | Speedup |\n|---:|---:|---:|\n"
                f"| {summary['baseline_repeat_median_seconds']:.3f}s | {summary['candidate_repeat_median_seconds']:.3f}s | {summary['first_repeat_speedup']:.3f}× |\n\n"
                f"Candidate artifacts all fresh: **{summary['candidate_every_repeat_all_fresh']}**. "
                f"Baseline catalog self-input reproduced every time: **{summary['baseline_every_repeat_catalog_self_input_dirty']}**. "
                f"Units/features/profiles equal: **{summary['same_units_features_profiles']}** "
                f"({summary['compilation_artifacts_per_build']} artifacts/build).\n\n"
                f"{summary['attribution']}\n\n"
                f"Generated Rust/metadata equal: **{summary['same_generated_rust_and_metadata']}**; "
                f"all first-party semantic outputs equal: **{summary['same_first_party_semantic_files']}**; "
                f"complete four-table asset bytes equal: **{summary['same_asset_table_bytes']}** "
                f"({summary['asset_serialization_bytes']} serialized bytes). Only catalog `assets.rs` representation and "
                "the explicit `xamlmeta-scan.txt` timing diagnostics are excluded from textual equivalence. "
                "All excluded bytes remain in the artifact; the asset representation has the independent semantic comparison.\n\n"
                "This measures the first unchanged build after catalog output generation. Population costs, individual repeats, "
                "cleanup commands, raw Cargo JSON/stderr, timing HTML, generated files and source provenance are in the artifact. "
                "It does not claim a cold compilation speedup.\n")
        (output / "summary.md").write_text(text, encoding="utf-8")
        print(text)
        if os.environ.get("GITHUB_STEP_SUMMARY"):
            with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as stream:
                stream.write(text)
        save()
        if summary["first_repeat_speedup"] < args.minimum_speedup:
            raise RuntimeError(f"first-repeat speedup {summary['first_repeat_speedup']:.3f}x is below required {args.minimum_speedup:.3f}x")
        success = True
        return 0
    except (RuntimeError, OSError, subprocess.SubprocessError, ValueError) as error:
        report["error"] = str(error)
        save()
        print(error, file=sys.stderr)
        return 1
    finally:
        if temporary is not None:
            if success and not args.keep_targets:
                shutil.rmtree(temporary)
            else:
                print(f"Owned targets preserved for inspection: {temporary}", flush=True)


if __name__ == "__main__":
    sys.exit(main())
