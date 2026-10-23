#!/usr/bin/env python3
"""Exercise source timestamp reuse against an isolated, offline Cargo build."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time


HELPER = Path(__file__).with_name("source_mtime_cache.py")


def run(command, *, cwd, env):
    result = subprocess.run(command, cwd=cwd, env=env, text=True,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=60)
    if result.returncode:
        raise RuntimeError(f"{command[0]} failed ({result.returncode}):\n"
                           + (result.stdout + result.stderr)[-12000:])
    return result.stdout, result.stderr


def probe(cargo: str, rustc: str | None):
    with tempfile.TemporaryDirectory(prefix="ferrodesign-source-mtime-cargo-") as temporary:
        repo = Path(temporary).resolve()
        target = repo / "target"
        target.mkdir()
        env = dict(os.environ)
        env.update(CARGO_HOME=str(repo / "cargo-home"), CARGO_TARGET_DIR=str(target),
                   CARGO_TERM_COLOR="never", CARGO_INCREMENTAL="0",
                   CARGO_BUILD_JOBS="1", RUSTUP_TOOLCHAIN="1.99.0")
        # Do not inherit the large project's fingerprint trace into this probe.
        env.pop("CARGO_LOG", None)
        if rustc:
            env["RUSTC"] = rustc
        version = run([cargo, "--version"], cwd=repo, env=env)[0].strip()
        if not version.startswith("cargo 1.99.0 "):
            raise RuntimeError(f"This probe requires the pinned Cargo 1.99.0; got {version}")
        sources = {
            "Cargo.toml": '[package]\nname = "mtime-app"\nversion = "0.1.0"\nedition = "2021"\n'
                          '[dependencies]\nmtime-dep = { path = "dep" }\n[workspace]\n',
            "src/main.rs": 'fn main() { println!("{}|{}", mtime_dep::value(), env!("PROBE_ASSET")); }\n',
            "dep/Cargo.toml": '[package]\nname = "mtime-dep"\nversion = "0.1.0"\nedition = "2021"\n',
            "dep/src/lib.rs": 'pub fn value() -> u32 { 42 }\n',
            "build.rs": 'fn main() {\n'
                        '    println!("cargo:rerun-if-changed=data.txt");\n'
                        '    let value = std::fs::read_to_string("data.txt").unwrap();\n'
                        '    println!("cargo:rustc-env=PROBE_ASSET={}", value.trim());\n}\n',
            "data.txt": "blue\n",
        }
        for name, content in sources.items():
            path = repo / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content, encoding="utf-8")
        run(["git", "init", "--quiet"], cwd=repo, env=env)
        run([cargo, "generate-lockfile", "--offline"], cwd=repo, env=env)
        tracked = sorted([*sources, "Cargo.lock"])
        run(["git", "add", "--", *tracked], cwd=repo, env=env)
        # Simulate an earlier checkout before compiling the initial target.
        initial_mtime = time.time_ns() - 120_000_000_000
        for name in tracked:
            info = (repo / name).stat()
            os.utime(repo / name, ns=(info.st_atime_ns, initial_mtime))

        def cache(operation):
            stdout, _ = run([sys.executable, str(HELPER), operation, "--repo", str(repo),
                             "--target-dir", str(target)], cwd=repo, env=env)
            result = json.loads(stdout)
            expected = "snapshotted" if operation == "snapshot" else "restored"
            if result["status"] != expected:
                raise AssertionError(f"Unexpected {operation}: {result}")
            return result

        def checkout():
            # Wall time comes from this filesystem, never a future Git date.
            time.sleep(0.02)
            now = time.time_ns()
            for name in tracked:
                info = (repo / name).stat()
                os.utime(repo / name, ns=(info.st_atime_ns, now))

        def build(expected_output):
            stdout, stderr = run([cargo, "build", "--offline", "--locked", "-vv"], cwd=repo, env=env)
            output = run([str(target / "debug" / "mtime-app")], cwd=repo, env=env)[0].strip()
            if output != expected_output:
                raise AssertionError(f"Built program returned {output!r}; expected {expected_output!r}")
            lines = (stdout + stderr).splitlines()
            return {"output": output,
                    "fresh": [line.strip().replace(str(repo), "<fixture>") for line in lines if line.lstrip().startswith("Fresh ")],
                    "compiling": [line.strip().replace(str(repo), "<fixture>") for line in lines if line.lstrip().startswith("Compiling ")],
                    "dirty": [line.strip().replace(str(repo), "<fixture>") for line in lines
                              if line.lstrip().startswith("Dirty ")]}

        baseline = build("42|blue")
        cache("snapshot")
        checkout()
        unchanged_restore = cache("restore")
        unchanged = build("42|blue")
        if unchanged["compiling"] or not all(
            any(line.startswith("Fresh " + name + " ") for line in unchanged["fresh"])
            for name in ("mtime-app", "mtime-dep")
        ):
            raise AssertionError(f"Unchanged source did not stay fresh: {unchanged}")
        if unchanged_restore["restored"] != len(tracked) or unchanged_restore["skipped"]:
            raise AssertionError(f"Unchanged fixture was not fully restored: {unchanged_restore}")

        cache("snapshot")
        checkout()
        (repo / "dep/src/lib.rs").write_text('pub fn value() -> u32 { 43 }\n', encoding="utf-8")
        source_restore = cache("restore")
        changed_source = build("43|blue")
        if source_restore["skip_reasons"].get("content_mismatch") != 1 or not all(
            any(line.startswith("Compiling " + name + " ") for line in changed_source["compiling"])
            for name in ("mtime-app", "mtime-dep")
        ):
            raise AssertionError(f"Changed source was not rebuilt: {source_restore}, {changed_source}")

        cache("snapshot")
        checkout()
        (repo / "data.txt").write_text("gold\n", encoding="utf-8")
        asset_restore = cache("restore")
        changed_asset = build("43|gold")
        if asset_restore["skip_reasons"].get("content_mismatch") != 1 or not any(
            line.startswith("Compiling mtime-app ") for line in changed_asset["compiling"]
        ) or not any(line.startswith("Fresh mtime-dep ") for line in changed_asset["fresh"]):
            raise AssertionError(f"Changed build-script input was not rebuilt: {asset_restore}, {changed_asset}")

        # Successful restore consumes its manifest; it cannot be saved with
        # later modified artifacts without a new post-build snapshot.
        if (target / ".ferrodesign-source-mtimes-v1.json").exists():
            raise AssertionError("A consumed manifest survived the later build")
        return {"passed": True, "cargo": version, "tracked_files": len(tracked),
                "helper_sha256": hashlib.sha256(HELPER.read_bytes()).hexdigest(),
                "scope": "Isolated local path dependency and build-script asset; offline, no FerroUI build",
                "baseline": baseline, "unchanged": unchanged,
                "changed_source": changed_source, "changed_build_script_asset": changed_asset,
                "restore_counts": {
                    "unchanged": {key: unchanged_restore[key] for key in ("restored", "skipped")},
                    "changed_source": {key: source_restore[key] for key in ("restored", "skipped")},
                    "changed_asset": {key: asset_restore[key] for key in ("restored", "skipped")},
                }}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cargo", default="cargo")
    parser.add_argument("--rustc")
    args = parser.parse_args()
    print(json.dumps(probe(args.cargo, args.rustc), indent=2))
