#!/usr/bin/env python3
"""Builds the tests of a `cargo test` selection, then runs the test binaries side by side.

`cargo test` runs its test binaries one after another: in the macOS job of CI the 197 binaries of
the workspace took 14.5 minutes, most of it in a few long ones while the other cores waited. This
script builds the same selection (`cargo test --no-run` with the arguments it is given), then runs
the binaries in a pool of `--jobs` processes, the largest binaries first (their size stands for
their length), each the way `cargo test` would: from the directory of its package, with the
`CARGO_*` variables of the package set. The output of a binary is printed in one piece when it
ends (a group in the log of GitHub Actions); a binary that fails is printed again at the end. It
ends with 1 when a binary failed, like `cargo test --no-fail-fast`.

A binary with many tests is split: its tests are listed (`--list`) and run in up to `--jobs` parts
of at least `--split` tests each (`--exact` with the names of the part, dealt out in turn so that a
slow module is spread over the parts), each part a process of its own. The catalog alone (about a
thousand tests) took longer than all the other binaries together.

    scripts/ci/parallel_tests.py [--jobs N] [-- <arguments of the test binaries>] <cargo test arguments>

    scripts/ci/parallel_tests.py --jobs 2 --workspace --locked --lib --tests --exclude ferroui-vello
"""

import argparse
import json
import os
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed


def build(cargo_args):
    command = [os.environ.get("CARGO", "cargo"), "test", "--no-run", "--message-format=json-render-diagnostics", *cargo_args]
    print("$ " + " ".join(command), flush=True)
    process = subprocess.Popen(command, stdout=subprocess.PIPE, text=True)
    binaries = []
    for line in process.stdout:
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        if message.get("reason") != "compiler-artifact" or not message.get("profile", {}).get("test"):
            continue
        executable = message.get("executable")
        if not executable:
            continue
        target = message["target"]
        binaries.append({
            "executable": executable,
            "manifest": message["manifest_path"],
            "package": message["package_id"],
            "label": f"{target['name']} ({'/'.join(target['kind'])})",
        })
    if process.wait() != 0:
        sys.exit(process.returncode)
    return binaries


def library_directories():
    """Where `cargo test` lets a test binary find shared libraries: the standard library of the
    toolchain (a proc-macro crate's tests link it dynamically) and the dependencies of the build."""
    sysroot = subprocess.run(["rustc", "--print", "sysroot"], capture_output=True, text=True).stdout.strip()
    host = next((line.split(": ", 1)[1] for line in
                 subprocess.run(["rustc", "-vV"], capture_output=True, text=True).stdout.splitlines()
                 if line.startswith("host: ")), "")
    return [os.path.join(sysroot, "lib"), os.path.join(sysroot, "lib", "rustlib", host, "lib")]


LIBRARIES = None


def package_environment(binary):
    global LIBRARIES
    if LIBRARIES is None:
        LIBRARIES = library_directories()
    environment = dict(os.environ)
    directories = [os.path.dirname(binary["executable"]), *LIBRARIES]
    for variable in ("LD_LIBRARY_PATH", "DYLD_FALLBACK_LIBRARY_PATH"):
        environment[variable] = os.pathsep.join(
            directories + ([environment[variable]] if environment.get(variable) else []))
    directory = os.path.dirname(binary["manifest"])
    environment["CARGO_MANIFEST_DIR"] = directory
    # A package id is `path+file:///dir#name@version` or `...#version` when the name is the
    # directory's.
    tail = binary["package"].rsplit("#", 1)[-1]
    name, _, version = tail.rpartition("@")
    environment["CARGO_PKG_NAME"] = name or os.path.basename(directory)
    environment["CARGO_PKG_VERSION"] = version or tail
    return directory, environment


def list_tests(binary, test_args):
    directory, environment = package_environment(binary)
    result = subprocess.run([binary["executable"], "--list", "--format", "terse", *test_args],
                            cwd=directory, env=environment, capture_output=True, text=True, errors="replace")
    if result.returncode != 0:
        return None  # run it whole: its failure is reported by the run
    return [line[:-len(": test")] for line in result.stdout.splitlines() if line.endswith(": test")]


def parts(binary, test_args, jobs, split):
    """The runs of a binary: (label, arguments), one run or the parts of a large binary."""
    names = list_tests(binary, test_args) if jobs > 1 else None
    count = min(jobs, len(names) // split) if names else 0
    if count < 2:
        return [(binary["label"], test_args)]
    return [(f"{binary['label']} [{index + 1}/{count}]", ["--exact", *names[index::count], *test_args])
            for index in range(count)]


def run(binary, label, arguments):
    directory, environment = package_environment(binary)
    started = time.monotonic()
    result = subprocess.run([binary["executable"], *arguments], cwd=directory, env=environment,
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
    return label, result.returncode, result.stdout, time.monotonic() - started


def main():
    arguments = sys.argv[1:]
    test_args = []
    if "--" in arguments:
        split = arguments.index("--")
        # Arguments before `--` belong to this script and to cargo; after it, to the test binaries.
        arguments, test_args = arguments[:split], arguments[split + 1:]
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--jobs", type=int, default=os.cpu_count() or 2)
    parser.add_argument("--split", type=int, default=150)
    options, cargo_args = parser.parse_known_args(arguments)

    binaries = build(cargo_args)
    binaries.sort(key=lambda b: os.path.getsize(b["executable"]), reverse=True)

    failed, durations = [], []
    started = time.monotonic()
    with ThreadPoolExecutor(max_workers=options.jobs) as pool:
        planned = list(pool.map(lambda b: parts(b, test_args, options.jobs, options.split), binaries))
        runs = [(binary, label, arguments)
                for binary, binary_parts in zip(binaries, planned) for label, arguments in binary_parts]
        print(f"running {len(binaries)} test binaries in {len(runs)} processes, {options.jobs} at a time",
              flush=True)
        # The parts of the split binaries first: they are the longest runs.
        runs.sort(key=lambda run_: ("[" not in run_[1], -os.path.getsize(run_[0]["executable"])))
        futures = [pool.submit(run, binary, label, arguments) for binary, label, arguments in runs]
        for future in as_completed(futures):
            label, code, output, seconds = future.result()
            durations.append((seconds, label))
            status = "ok" if code == 0 else f"FAILED (exit {code})"
            print(f"::group::{label}: {status}, {seconds:.0f} s", flush=True)
            print(output, flush=True)
            print("::endgroup::", flush=True)
            if code != 0:
                failed.append((label, output))

    print(f"{len(runs)} test processes in {time.monotonic() - started:.0f} s; the longest:")
    for seconds, label in sorted(durations, reverse=True)[:10]:
        print(f"  {seconds:6.0f} s  {label}")
    for label, output in failed:
        print(f"\n===== {label} failed =====\n{output}")
    if failed:
        print(f"::error::{len(failed)} of {len(runs)} test processes failed: "
              + ", ".join(label for label, _ in failed))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
