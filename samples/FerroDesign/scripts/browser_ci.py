"""Strict CI orchestration for the unchanged FerroDesign browser harness.

One verified distribution is shared by eight independent renderer/group jobs.
Every job retains the runner's ordinary common gates. This module supplies no
browser commands, retries, assertion overrides or alternative rendering path.
The final gate checks raw reports, process exits and GitHub dependency outcomes;
a missing final report (including a job timeout) is never partial success.

Only the Python standard library is required. Run test_browser_ci.py before use.
"""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import math
import os
from pathlib import Path, PurePosixPath
import re
import selectors
import subprocess
import sys


SAMPLE = Path("samples/FerroDesign")
CONFIG = SAMPLE / "scripts/browser-ci-groups.json"
MANIFEST = "ferrodesign-provenance.json"
MODES = ["Software2D", "WebGL2"]
GROUP_SIZES = [8, 8, 10, 12]
SCENARIO_COUNT = 38
PAIR_COUNT = 76
DEPENDENCIES = ["browser-build", "browser-scenarios", "browser-data"]
COMMANDS = {
    "scenario": ["node", "tests/browser.mjs"],
    "persistence": ["node", "tests/document-store-browser.mjs"],
    "benchmark": ["node", "tests/text-render-benchmark.mjs"],
}
JOBS = {"scenario": "browser-scenarios", "persistence": "browser-data", "benchmark": "browser-data"}
PERSISTENCE_CHECKS = [
    "Legacy migration and current/previous snapshots commit atomically",
    "A write request that succeeds before transaction abort is not acknowledged or persisted",
    "Two real browser tabs cannot overwrite a snapshot loaded at an older revision",
    "A document exceeding 10 MiB survives a real browser reload without localStorage",
    "An aborted migration keeps the original localStorage document readable and intact",
]


class GateError(ValueError):
    """Missing, inconsistent or unsuccessful validation evidence."""


def require(condition, message):
    if not condition:
        raise GateError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def read_json(path):
    # Duplicate keys must not let a later forged status replace an earlier one.
    def unique(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, f"Duplicate JSON key: {key}")
            result[key] = value
        return result

    def invalid(value):
        raise GateError(f"Non-finite JSON constant: {value}")

    return json.loads(Path(path).read_text(), object_pairs_hook=unique, parse_constant=invalid)


def write_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, ensure_ascii=False, allow_nan=False) + "\n")
    temporary.replace(path)


def now():
    return datetime.now(timezone.utc).isoformat()


def attempt_number(value):
    require(isinstance(value, str) and re.fullmatch(r"[1-9][0-9]*", value), "Invalid CI run attempt")
    return int(value)


def file_record(path, name=None):
    path = Path(path)
    require(path.is_file() and not path.is_symlink(), f"Expected a regular file: {path}")
    checksum = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            checksum.update(chunk)
    return {"path": name or path.name, "bytes": path.stat().st_size, "sha256": checksum.hexdigest()}


def relative_file(root, name):
    require(isinstance(name, str) and bool(name), "Missing relative evidence path")
    relative = PurePosixPath(name)
    require(not relative.is_absolute() and relative.as_posix() == name
            and all(part not in (".", "..") for part in relative.parts)
            and "\\" not in name, f"Unsafe evidence path: {name}")
    path = Path(root).joinpath(*relative.parts)
    require(path.resolve().is_relative_to(Path(root).resolve()), f"Evidence escapes its directory: {name}")
    for parent in [path, *path.parents]:
        if parent == Path(root):
            break
        require(not parent.is_symlink(), f"Symlink in evidence path: {name}")
    return path


def verify_file(root, record, expected_name=None):
    require(isinstance(record, dict) and set(record) == {"path", "bytes", "sha256"}, "Invalid file record")
    if expected_name is not None:
        require(record["path"] == expected_name, f"Unexpected evidence file: {record['path']}")
    require(type(record["bytes"]) is int and record["bytes"] >= 0, "Invalid evidence byte count")
    require(isinstance(record["sha256"], str) and re.fullmatch(r"[0-9a-f]{64}", record["sha256"]), "Invalid SHA-256")
    path = relative_file(root, record["path"])
    require(file_record(path, record["path"]) == record, f"File bytes or digest differ: {path}")
    return path


def registrations(source):
    # Keep this deliberately narrow: a newly dynamic registration fails instead
    # of silently disappearing from the independently checked coverage plan.
    calls = re.findall(r"\bawait\s+scenario\s*\(", source)
    names = re.findall(r'\bawait\s+scenario\s*\(\s*[\"\']([a-z0-9-]+)[\"\']\s*,', source)
    require(len(calls) == len(names), "Every await scenario call must have a literal registered name")
    require(len(names) == SCENARIO_COUNT and len(set(names)) == len(names), "Expected 38 unique scenario registrations")
    return names


def validate_config(config, source):
    names = registrations(source)
    require(config.get("schema_version") == 1 and config.get("modes") == MODES, "Unexpected partition schema or modes")
    groups = config.get("groups")
    require(isinstance(groups, list) and len(groups) == 4, "Exactly four groups are required")
    group_names = [group.get("name") for group in groups]
    require(all(isinstance(name, str) and re.fullmatch(r"[a-z][a-z0-9-]*", name) for name in group_names)
            and len(set(group_names)) == 4, "Group names must be unique safe identifiers")
    require([len(group.get("scenarios", [])) for group in groups] == GROUP_SIZES, "Expected contiguous group sizes 8, 8, 10, 12")
    flattened = [name for group in groups for name in group["scenarios"]]
    require(flattened == names, "Groups must cover every registered scenario exactly once in registration order")
    require(len(flattened) * len(MODES) == PAIR_COUNT, "Expected exactly 76 scenario/mode pairs")
    return names


def load_plan(repo):
    config = read_json(Path(repo) / CONFIG)
    names = validate_config(config, (Path(repo) / SAMPLE / "tests/browser.mjs").read_text())
    return config, names


def matrix(config):
    return {"include": [{"group": group["name"], "mode": mode}
                        for group in config["groups"] for mode in MODES]}


def git(repo, *arguments):
    return subprocess.check_output(["git", *arguments], cwd=repo).decode().strip()


def source_identity(repo, environment):
    commit = git(repo, "rev-parse", "HEAD")
    require(commit == environment.get("GITHUB_SHA"), "Checkout is not the requested GitHub commit")
    require(re.fullmatch(r"[0-9a-f]{40}", commit), "Invalid source commit")
    subprocess.run(["git", "diff", "--quiet", "HEAD", "--"], cwd=repo, check=True)
    tracked = subprocess.check_output([
        "git", "ls-files", "-z", "--", str(SAMPLE / "tests"), str(SAMPLE / "web"),
        str(SAMPLE / "scripts/browser_ci.py"), str(SAMPLE / "scripts/test_browser_ci.py"), str(CONFIG),
        str(SAMPLE / "package.json"), str(SAMPLE / "package-lock.json"), ".github/workflows/ferrodesign.yml",
    ], cwd=repo).decode().split("\0")
    records = [file_record(Path(repo) / name, name) for name in sorted(filter(None, tracked))]
    require(any(row["path"] == str(CONFIG) for row in records), "The CI partition must be committed with the harness")
    for key in ("GITHUB_REPOSITORY", "GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT"):
        require(bool(environment.get(key)), f"Missing CI identity: {key}")
    attempt_number(environment["GITHUB_RUN_ATTEMPT"])
    return {
        "commit": commit, "tree": git(repo, "rev-parse", "HEAD^{tree}"),
        "repository": environment["GITHUB_REPOSITORY"], "run_id": environment["GITHUB_RUN_ID"],
        "harness": {"sha256": digest(canonical(records)), "files": records},
    }


def distribution_files(dist):
    records = []
    for path in sorted(Path(dist).rglob("*")):
        require(not path.is_symlink(), f"Distribution symlinks are not allowed: {path}")
        if path.is_file() and path.relative_to(dist).as_posix() != MANIFEST:
            records.append(file_record(path, path.relative_to(dist).as_posix()))
    return records


def validate_distribution(dist, manifest, source, expected_sha256, execution_attempt):
    require(file_record(Path(dist) / MANIFEST)["sha256"] == expected_sha256, "Distribution manifest differs from the build job output")
    require(manifest.get("schema_version") == 1 and manifest.get("kind") == "ferrodesign-browser-ci", "Unexpected distribution provenance")
    require(manifest.get("source") == source, "Distribution source/tree/harness/run identity differs")
    require(attempt_number(manifest.get("run_attempt")) <= attempt_number(execution_attempt), "Distribution claims a future build attempt")
    require(manifest.get("profile") == "browser" and manifest.get("target") == "wasm32-unknown-emscripten", "Unexpected build profile or target")
    files = manifest.get("files")
    require(isinstance(files, list) and files, "Distribution payload list is missing")
    names = [row.get("path") for row in files]
    require(len(names) == len(set(names)), "Duplicate distribution payload")
    require({"index.html", "main.js", "ferrodesign-app.js", "favicon.svg", "dependencies.txt"}.issubset(names)
            and any(name.endswith(".wasm") for name in names), "Distribution is missing a required host asset or WASM")
    for row in files:
        verify_file(dist, row)
    require(distribution_files(dist) == files, "Distribution contains missing, extra or changed payloads")
    graph = (Path(dist) / "dependencies.txt").read_text()
    require("ferroui-vello" in graph and not re.search(r"ferroui-skia|skia-safe|skia-bindings", graph), "Distribution does not retain the active Vello/no-Skia graph")
    return {**source, "distribution_manifest_sha256": expected_sha256,
            "build_run_attempt": manifest["run_attempt"]}


def verify_distribution(repo, dist, expected_sha256, environment):
    source = source_identity(repo, environment)
    manifest = read_json(Path(dist) / MANIFEST)
    identity = validate_distribution(dist, manifest, source, expected_sha256, environment["GITHUB_RUN_ATTEMPT"])
    return identity, manifest


def finite_number(value, minimum=0):
    return type(value) in (int, float) and math.isfinite(value) and value >= minimum


def validate_scenarios(report, config, group_name, mode):
    group = next(group for group in config["groups"] if group["name"] == group_name)
    all_names = [name for entry in config["groups"] for name in entry["scenarios"]]
    require(report.get("passed") is True and not report.get("error"), "Scenario report did not pass")
    require(report.get("scope") == {"full": False, "modes": [mode], "scenarios": group["scenarios"]}, "Scenario scope differs from its assigned group/mode")
    results = report.get("results", [])
    require(len(results) == 1, "A shard must contain exactly one renderer result")
    result = results[0]
    require(result.get("requestedMode") == mode and result.get("requestedRenderer") == "Vello"
            and result.get("surface", {}).get("kind") == mode, "Requested/observed renderer does not match the shard")
    require(result.get("status") == "passed" and not result.get("error") and result.get("pageErrors") == [], "Renderer result failed or has page errors")
    require(isinstance(result.get("consoleErrors"), list)
            and not any(re.search(r"panicked|unreachable|RuntimeError|TypeError|ReferenceError|RangeError", line, re.I)
                        for line in result["consoleErrors"]), "Renderer console contains a runtime error")
    for name in ("initial", "final"):
        require(result.get(name, {}).get("renderer") == "FerroUI Vello", f"Missing native {name} Vello state")
    require(result["final"].get("unsupportedEffects") == 0, "Unsupported effects remain in the final native state")
    entries = result.get("scenarios", [])
    require([entry.get("name") for entry in entries] == all_names, "Scenario registry is missing, duplicated, reordered or unknown")
    checks = result.get("checks")
    require(isinstance(checks, list) and checks and all(isinstance(check, str) and check for check in checks), "Missing completed native/common checks")
    selected_checks = 0
    pairs = []
    for entry in entries:
        if entry["name"] in group["scenarios"]:
            require(entry.get("ran") is True and entry.get("status") == "passed", f"Incomplete/failed selected scenario: {mode}/{entry['name']}")
            require(type(entry.get("checks")) is int and entry["checks"] > 0
                    and finite_number(entry.get("durationMs")), "Selected scenario has no completed checks or duration")
            require(not any(entry.get(key) for key in ("error", "failureStage", "captureError", "captureTimedOut")), "A selected scenario retains failure evidence")
            selected_checks += entry["checks"]
            pairs.append((entry["name"], mode))
        else:
            require(entry.get("status") == "not-requested" and "ran" not in entry
                    and "checks" not in entry and "durationMs" not in entry, "An unassigned scenario was executed or forged")
    require(len(checks) > selected_checks, "The original common gates are absent from the shard report")
    return {"pairs": pairs, "checks": len(checks), "scenario_checks": selected_checks}


def validate_persistence(report):
    require(report.get("passed") is True and bool(report.get("browser")), "Persistence report did not pass")
    require(report.get("checks") == PERSISTENCE_CHECKS, "Persistence checks are missing, duplicated or changed")
    large = report.get("largeDocument", {})
    require(large.get("flushed") is True and type(large.get("characters")) is int
            and large["characters"] > 10 * 1024 * 1024 and large.get("state", {}).get("dirty") is False,
            "The original large-document persistence gate is incomplete")
    return {"checks": len(PERSISTENCE_CHECKS)}


def validate_benchmark(report, directory, manifest, commit):
    require(report.get("schemaVersion") == 2 and report.get("benchmark") == "native-text-pan-v2", "Unexpected benchmark schema")
    require(report.get("warmupCount") == 12 and report.get("measuredCount") == 36
            and report.get("traceDiagnosticRequested") is True, "The original 12/36/6 traced benchmark is required")
    require(report.get("browserTransport") == "owned-playwright-server-websocket" and bool(report.get("finished")), "Benchmark did not finish with owned browser transport")
    require(not any(report.get(key) for key in ("error", "serverCleanupError", "reportPersistenceError", "traceDiagnosticStoppedBrowser")), "Benchmark stopped or retained a runtime/cleanup error")
    builds = report.get("builds", [])
    require(len(builds) == 1 and builds[0].get("id") == "current" and builds[0].get("label") == commit, "Benchmark did not use exactly the current build")
    expected_files = [{"name": row["path"], "bytes": row["bytes"], "sha256": row["sha256"]}
                      for row in manifest["files"] if "/" not in row["path"]
                      and PurePosixPath(row["path"]).suffix in (".wasm", ".data", ".js", ".html")]
    require(builds[0].get("files") == expected_files, "Benchmark build payload differs from the verified distribution")
    fingerprint = digest(json.dumps(expected_files, separators=(",", ":"), ensure_ascii=False).encode())
    require(builds[0].get("fingerprint") == fingerprint, "Benchmark payload fingerprint differs")
    results = report.get("results", [])
    require([result.get("requestedMode") for result in results] == MODES, "Benchmark is missing or duplicated a renderer mode")
    fixture = file_record(Path(directory) / "fixture.json")
    require(report.get("workload", {}).get("inputSha256") == fixture["sha256"]
            and report["workload"].get("labels") == 150, "Benchmark fixture identity or workload differs")
    for result in results:
        mode = result["requestedMode"]
        require(result.get("status") == "passed" and result.get("build") == "current" and result.get("label") == commit,
                f"Benchmark result did not pass: {mode}")
        require(result.get("environment", {}).get("surface") == mode
                and result.get("metricBridge", {}).get("kind") == "compact-v1", "Benchmark surface/metric bridge differs")
        require(result.get("pageErrors") == [] and result.get("consoleErrors") == [] and result.get("renderPanics") == []
                and not result.get("error") and not result.get("contextCleanupError"), "Benchmark has runtime or cleanup errors")
        samples = result.get("samples", [])
        require([sample.get("index") for sample in samples] == list(range(12, 48)), "Benchmark requires all 36 unique normal samples")
        for sample in samples:
            require(all(finite_number(sample.get(key)) for key in
                        ("durationMs", "dispatchMs", "frameObservationMs", "presentationMs", "settledObservationMs"))
                    and finite_number(sample.get("frameDelta"), 1)
                    and finite_number(sample.get("presentedFrameDelta"), 1), "A normal sample has incomplete frame/presentation evidence")
        trace = result.get("traceDiagnostic", {})
        require(trace.get("status") == "completed" and trace.get("normalSamplesIncluded") is False
                and trace.get("fatalBrowserClosure") is False and trace.get("mode") == mode
                and trace.get("build") == "current", "Separate trace cohort is incomplete or mixed into normal samples")
        require(trace.get("cleanupErrors") == [] and trace.get("pageErrors") == []
                and trace.get("consoleErrors") == [] and trace.get("droppedErrors") == 0, "Trace retained errors")
        observation = trace.get("observation", {})
        observed = observation.get("samples", [])
        require(trace.get("limits", {}).get("samples") == 6 and len(trace.get("motions", [])) == 6
                and [sample.get("index") for sample in observed] == list(range(6))
                and observation.get("droppedInputEvents") == 0, "Trace requires all six unique motion observations")
        for sample in observed:
            require(sample.get("complete") is True and sample.get("input", {}).get("capture", {}).get("trusted") is True
                    and sample.get("input", {}).get("bubble", {}).get("trusted") is True, "Trace motion is incomplete or untrusted")
        analysis = trace.get("analysis", {})
        require(analysis.get("status") == "complete" and [sample.get("index") for sample in analysis.get("samples", [])] == list(range(6)), "Trace analysis is incomplete")
        raw = trace.get("trace", {})
        require(raw.get("streamTruncated") is False and raw.get("dataLossOccurred") is False, "Trace is truncated or lost data")
        verify_file(directory, {"path": raw.get("file"), "bytes": raw.get("bytes"), "sha256": raw.get("sha256")}, f"current-{mode}-pan-trace.json")
    shutdown = report.get("browserShutdown", {})
    require(shutdown.get("status") == "closed" and shutdown.get("processExit", {}).get("observed") is True
            and shutdown.get("errors") == [], "Owned browser shutdown was not confirmed successful")
    return {"modes": MODES, "normal_samples_per_mode": 36, "trace_samples_per_mode": 6}


def run_child(command, cwd, environment, output):
    """Tee each stream without merging warnings into persistence's JSON stdout."""
    with (output / "stdout.log").open("wb") as stdout, (output / "stderr.log").open("wb") as stderr:
        child = subprocess.Popen(command, cwd=cwd, env=environment, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        try:
            with selectors.DefaultSelector() as selector:
                selector.register(child.stdout, selectors.EVENT_READ, (stdout, sys.stdout.buffer))
                selector.register(child.stderr, selectors.EVENT_READ, (stderr, sys.stderr.buffer))
                while selector.get_map():
                    for key, _ in selector.select():
                        chunk = os.read(key.fileobj.fileno(), 65536)
                        if not chunk:
                            selector.unregister(key.fileobj)
                            key.fileobj.close()
                        else:
                            for stream in key.data:
                                stream.write(chunk)
                                stream.flush()
            return child.wait()
        finally:
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait(timeout=5)


def validate_report(kind, report, directory, config, manifest, identity, group=None, mode=None):
    if kind == "scenario":
        return validate_scenarios(report, config, group, mode)
    if kind == "persistence":
        return validate_persistence(report)
    return validate_benchmark(report, directory, manifest, identity["commit"])


def run_validation(repo, dist, output, expected_sha256, kind, group, mode, environment):
    config, _ = load_plan(repo)
    require(kind in COMMANDS, "Unknown validation kind")
    if kind == "scenario":
        require(group in [item["name"] for item in config["groups"]] and mode in MODES, "Unknown group or mode")
    else:
        require(group is None and mode is None, "Data gates cannot select a scenario group or renderer")
    identity, manifest = verify_distribution(repo, dist, expected_sha256, environment)
    require(environment.get("GITHUB_JOB") == JOBS[kind], "Unexpected owning CI job")
    require(not output.exists(), "Refusing to reuse an existing report directory")
    output.mkdir(parents=True)
    child_env = dict(environment)
    for key in ("FERRODESIGN_SCENARIOS", "FERRODESIGN_MODES", "FERRODESIGN_BASELINE_DIST", "FERRODESIGN_BASELINE_LABEL", "FERRODESIGN_CHROMIUM"):
        child_env.pop(key, None)
    child_env.update(FERRODESIGN_DIST=str(dist), FERRODESIGN_REPORT=str(output),
                     FERRODESIGN_BENCHMARK_LABEL=identity["commit"], FERRODESIGN_TRACE_TEXT_PAN="1")
    if kind == "scenario":
        child_env["FERRODESIGN_SCENARIOS"] = ",".join(next(item["scenarios"] for item in config["groups"] if item["name"] == group))
        child_env["FERRODESIGN_MODES"] = mode
    proof = {
        "schema_version": 1, "kind": kind, "group": group, "mode": mode, "identity": identity,
        "ci": {"job": environment["GITHUB_JOB"], "run_attempt": environment["GITHUB_RUN_ATTEMPT"]},
        "command": COMMANDS[kind], "status": "running", "validation_errors": [],
        "process": {"started_utc": now(), "finished_utc": None, "exit_code": None},
    }
    write_json(output / "proof.json", proof)
    try:
        proof["process"]["exit_code"] = run_child(COMMANDS[kind], repo / SAMPLE, child_env, output)
        proof["process"]["finished_utc"] = now()
        require(proof["process"]["exit_code"] == 0, f"Original {kind} process exited {proof['process']['exit_code']}")
        if kind == "persistence":
            (output / "report.json").write_bytes((output / "stdout.log").read_bytes())
        after, _ = verify_distribution(repo, dist, expected_sha256, environment)
        require(after == identity, "Source or distribution changed during validation")
        proof["identity_after"] = after
        proof["coverage"] = validate_report(kind, read_json(output / "report.json"), output, config, manifest, identity, group, mode)
        proof["status"] = "passed"
    except Exception as error:
        proof["status"] = "failed"
        proof["validation_errors"].append(f"{type(error).__name__}: {error}")
        print(proof["validation_errors"][-1], file=sys.stderr)
    finally:
        proof["process"]["finished_utc"] = proof["process"]["finished_utc"] or now()
        proof["files"] = {name: file_record(output / name) for name in ("report.json", "stdout.log", "stderr.log") if (output / name).is_file()}
        write_json(output / "proof.json", proof)
    return 0 if proof["status"] == "passed" else 1


def validate_proof(directory, kind, config, manifest, identity, execution_attempt, group=None, mode=None):
    proof = read_json(directory / "proof.json")
    require(proof.get("schema_version") == 1 and proof.get("kind") == kind
            and proof.get("group") == group and proof.get("mode") == mode, "Wrong or duplicate proof assignment")
    require(proof.get("identity") == identity and proof.get("identity_after") == identity, "Proof source/tree/harness/distribution identity differs")
    require(proof.get("status") == "passed" and proof.get("validation_errors") == [], "Proof did not pass independently")
    require(proof.get("command") == COMMANDS[kind] and proof.get("ci", {}).get("job") == JOBS[kind], "Proof did not execute the required original command in its assigned job")
    # A retained successful sibling may precede a rerun-failed-jobs attempt.
    # Its exact build/source identity and original successful exit must still
    # match. Future attempts and an execution preceding its build are invalid.
    require(attempt_number(identity["build_run_attempt"]) <= attempt_number(proof["ci"].get("run_attempt"))
            <= attempt_number(execution_attempt), "Proof claims an invalid execution attempt")
    process = proof.get("process", {})
    require(type(process.get("exit_code")) is int and process["exit_code"] == 0, "Original process did not complete successfully")
    require(isinstance(process.get("started_utc"), str) and isinstance(process.get("finished_utc"), str)
            and datetime.fromisoformat(process["finished_utc"]) >= datetime.fromisoformat(process["started_utc"]), "Process completion timestamps are absent or reversed")
    records = proof.get("files", {})
    require(set(records) == {"report.json", "stdout.log", "stderr.log"}, "Proof is missing report or process streams")
    for name, record in records.items():
        verify_file(directory, record, name)
    coverage = validate_report(kind, read_json(directory / "report.json"), directory, config, manifest, identity, group, mode)
    require(canonical(proof.get("coverage")) == canonical(coverage), "Proof coverage summary disagrees with the original report")
    return coverage


def aggregate(artifacts, config, manifest, identity, needs, execution_attempt):
    require(set(needs) == set(DEPENDENCIES), "Missing or unexpected dependency job outcomes")
    require(all(needs[name].get("result") == "success" for name in DEPENDENCIES), "Build, every matrix shard and data job must all succeed")
    require(needs["browser-build"].get("outputs", {}).get("manifest_sha256") == identity["distribution_manifest_sha256"], "Aggregate did not receive the build job's manifest digest")
    commit = identity["commit"]
    expected = {f"FerroDesign-browser-evidence-{item['group']}-{item['mode']}-{commit}" for item in matrix(config)["include"]}
    data_name = f"FerroDesign-browser-evidence-data-{commit}"
    expected.add(data_name)
    require(artifacts.is_dir() and {path.name for path in artifacts.iterdir()} == expected, "Evidence artifacts are missing, duplicated or unexpected")
    pairs = []
    shards = []
    for item in matrix(config)["include"]:
        name = f"FerroDesign-browser-evidence-{item['group']}-{item['mode']}-{commit}"
        coverage = validate_proof(artifacts / name, "scenario", config, manifest, identity, execution_attempt, item["group"], item["mode"])
        pairs.extend(coverage["pairs"])
        shards.append({**item, **coverage})
    expected_pairs = {(name, mode) for group in config["groups"] for name in group["scenarios"] for mode in MODES}
    require(len(pairs) == PAIR_COUNT and len(set(pairs)) == PAIR_COUNT and set(pairs) == expected_pairs, "Exactly 76 unique scenario/mode pairs must pass")
    data = artifacts / data_name
    require({path.name for path in data.iterdir()} == {"persistence", "benchmark"}, "Data artifact is missing or contains unexpected gates")
    persistence = validate_proof(data / "persistence", "persistence", config, manifest, identity, execution_attempt)
    benchmark = validate_proof(data / "benchmark", "benchmark", config, manifest, identity, execution_attempt)
    return {"passed": True, "scope": "All registered scenarios in independent group/mode sessions with the unchanged common gates; not a single sequential browser session or a speed claim.",
            "identity": identity, "aggregate_run_attempt": execution_attempt,
            "scenario_mode_pairs": len(pairs), "shards": shards,
            "persistence": persistence, "benchmark": benchmark}


def github_output(path, key, value):
    if path:
        with Path(path).open("a") as stream:
            stream.write(f"{key}={value}\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("plan", "seal", "verify", "run", "aggregate"))
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument("--dist", type=Path, default=Path("dist"))
    parser.add_argument("--manifest-sha256")
    parser.add_argument("--github-output")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--kind", choices=tuple(COMMANDS))
    parser.add_argument("--group")
    parser.add_argument("--mode", choices=MODES)
    parser.add_argument("--artifacts", type=Path)
    args = parser.parse_args()
    repo, dist = args.repo.resolve(), args.dist.resolve()
    try:
        config, names = load_plan(repo)
        if args.operation == "plan":
            plan = matrix(config)
            github_output(args.github_output, "matrix", json.dumps(plan, separators=(",", ":")))
            print(json.dumps({"registered_scenarios": len(names), "scenario_mode_pairs": PAIR_COUNT, "matrix": plan}, indent=2))
        elif args.operation == "seal":
            source = source_identity(repo, os.environ)
            manifest = {"schema_version": 1, "kind": "ferrodesign-browser-ci", "source": source,
                        "repository": source["repository"], "commit": source["commit"], "tree": source["tree"],
                        "run_id": source["run_id"], "run_attempt": os.environ["GITHUB_RUN_ATTEMPT"],
                        "created_utc": now(), "profile": "browser", "target": "wasm32-unknown-emscripten",
                        "renderer_graph": "dependencies.txt", "files": distribution_files(dist)}
            write_json(dist / MANIFEST, manifest)
            checksum = file_record(dist / MANIFEST)["sha256"]
            validate_distribution(dist, manifest, source, checksum, os.environ["GITHUB_RUN_ATTEMPT"])
            require(args.output is not None, "seal requires --output for its retained build proof")
            write_json(args.output, manifest)
            github_output(args.github_output, "manifest_sha256", checksum)
            print(json.dumps({"manifest_sha256": checksum, "payloads": len(manifest["files"]), "commit": source["commit"]}))
        elif args.operation == "run":
            require(args.kind is not None and args.output is not None, "run requires --kind and --output")
            return run_validation(repo, dist, args.output.resolve(), args.manifest_sha256, args.kind, args.group, args.mode, os.environ)
        else:
            identity, manifest = verify_distribution(repo, dist, args.manifest_sha256, os.environ)
            if args.operation == "verify":
                print(json.dumps({"verified": True, "identity": identity}, indent=2))
            else:
                require(args.artifacts is not None and args.output is not None, "aggregate requires --artifacts and --output")
                needs = json.loads(os.environ.get("FERRODESIGN_CI_NEEDS", "{}"))
                summary = aggregate(args.artifacts.resolve(), config, manifest, identity, needs, os.environ["GITHUB_RUN_ATTEMPT"])
                write_json(args.output, summary)
                print(json.dumps({"passed": True, "scenario_mode_pairs": summary["scenario_mode_pairs"], "shards": len(summary["shards"]), "benchmark": summary["benchmark"]}))
        return 0
    except Exception as error:
        failure = {"passed": False, "error": f"{type(error).__name__}: {error}"}
        if args.operation == "aggregate" and args.output is not None:
            write_json(args.output, failure)
        print(json.dumps(failure), file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
