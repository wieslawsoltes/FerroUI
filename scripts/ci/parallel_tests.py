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


def package_environment(binary):
    environment = dict(os.environ)
    directory = os.path.dirname(binary["manifest"])
    environment["CARGO_MANIFEST_DIR"] = directory
    # A package id is `path+file:///dir#name@version` or `...#version` when the name is the
    # directory's.
    tail = binary["package"].rsplit("#", 1)[-1]
    name, _, version = tail.rpartition("@")
    environment["CARGO_PKG_NAME"] = name or os.path.basename(directory)
    environment["CARGO_PKG_VERSION"] = version or tail
    return directory, environment


def run(binary, test_args):
    directory, environment = package_environment(binary)
    started = time.monotonic()
    result = subprocess.run([binary["executable"], *test_args], cwd=directory, env=environment,
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
    return binary, result.returncode, result.stdout, time.monotonic() - started


def main():
    arguments = sys.argv[1:]
    test_args = []
    if "--" in arguments:
        split = arguments.index("--")
        # Arguments before `--` belong to this script and to cargo; after it, to the test binaries.
        arguments, test_args = arguments[:split], arguments[split + 1:]
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--jobs", type=int, default=os.cpu_count() or 2)
    options, cargo_args = parser.parse_known_args(arguments)

    binaries = build(cargo_args)
    binaries.sort(key=lambda b: os.path.getsize(b["executable"]), reverse=True)
    print(f"running {len(binaries)} test binaries, {options.jobs} at a time", flush=True)

    failed, durations = [], []
    started = time.monotonic()
    with ThreadPoolExecutor(max_workers=options.jobs) as pool:
        futures = [pool.submit(run, binary, test_args) for binary in binaries]
        for future in as_completed(futures):
            binary, code, output, seconds = future.result()
            durations.append((seconds, binary["label"]))
            status = "ok" if code == 0 else f"FAILED (exit {code})"
            print(f"::group::{binary['label']}: {status}, {seconds:.0f} s", flush=True)
            print(output, flush=True)
            print("::endgroup::", flush=True)
            if code != 0:
                failed.append((binary, output))

    print(f"{len(binaries)} test binaries in {time.monotonic() - started:.0f} s; the longest:")
    for seconds, label in sorted(durations, reverse=True)[:10]:
        print(f"  {seconds:6.0f} s  {label}")
    for binary, output in failed:
        print(f"\n===== {binary['label']} failed =====\n{output}")
    if failed:
        print(f"::error::{len(failed)} of {len(binaries)} test binaries failed: "
              + ", ".join(b["label"] for b, _ in failed))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
