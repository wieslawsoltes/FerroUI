#!/usr/bin/env python3
"""Install a checksummed, isolated Rust toolchain for the native theme benchmarks.

This downloads official binary components; it does not compile the repository,
run rustup, change the user's shell, or modify a system installation. Keep the
work directory on a filesystem with room for the downloaded and installed
components, rather than in an automatically cleaned temporary directory.

    python3 scripts/themes/bench/bootstrap_toolchain.py \
        --work-dir /path/to/benchmark-cache/toolchain
"""
from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import platform
import shlex
import shutil
import subprocess
import tarfile
import tempfile
import time
import tomllib
from urllib.parse import urlparse


HOST = "x86_64-unknown-linux-gnu"
COMPONENTS = ("rustc", "rust-std", "cargo", "llvm-tools-preview")
RECORDED_COMPILER = {
    "release": "1.99.0",
    "commit-hash": "b940084d7eb6a299eb4bfeb8e34901bc051e7ac4",
    "LLVM version": "23.1.1",
}


def sha256(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def identity(path: Path) -> dict:
    return {"path": str(path), "bytes": path.stat().st_size, "sha256": sha256(path)}


def save(path: Path, value: dict) -> None:
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")
    temporary.replace(path)


def download(url: str, destination: Path, expected: str | None = None) -> dict:
    if urlparse(url).scheme != "https" or urlparse(url).hostname != "static.rust-lang.org":
        raise ValueError(f"Unexpected distribution origin: {url}")
    if destination.is_file() and (expected is None or sha256(destination) == expected):
        return {**identity(destination), "url": url, "reused": True}
    if destination.exists():
        raise RuntimeError(f"Cached component checksum mismatch: {destination}")
    partial = destination.with_suffix(destination.suffix + ".download")
    command = [
        "curl", "--fail", "--location", "--connect-timeout", "15",
        "--max-time", "300", "--retry", "3", "--retry-delay", "1",
        "--silent", "--show-error", url, "--output", str(partial),
    ]
    subprocess.run(command, check=True)
    actual = sha256(partial)
    if expected is not None and actual != expected:
        raise RuntimeError(f"Downloaded component checksum mismatch: {partial}")
    partial.replace(destination)
    return {**identity(destination), "url": url, "reused": False, "command": command}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--work-dir", type=Path, required=True)
    parser.add_argument("--version", default="1.99.0")
    args = parser.parse_args()
    if platform.system() != "Linux" or platform.machine() != "x86_64":
        parser.error("this benchmark toolchain recipe targets native x86-64 GNU/Linux")
    if not shutil.which("curl"):
        parser.error("curl is required to fetch official distribution components")
    work = args.work_dir.resolve()
    dist = work / "dist"
    prefix = work / "install"
    dist.mkdir(parents=True, exist_ok=True)
    report_path = work / "installation.json"
    if prefix.exists() and any(prefix.iterdir()) and not report_path.exists():
        parser.error(f"refusing to modify an unrelated existing installation: {prefix}")
    manifest = dist / f"channel-rust-{args.version}.toml"
    manifest_url = f"https://static.rust-lang.org/dist/{manifest.name}"
    checksum_file = manifest.with_suffix(manifest.suffix + ".sha256")
    download(manifest_url + ".sha256", checksum_file)
    manifest_record = download(manifest_url, manifest, checksum_file.read_text().split()[0])
    channel = tomllib.loads(manifest.read_text(encoding="utf-8"))
    components = {}
    for name in COMPONENTS:
        package = channel["pkg"][name]
        target = package["target"][HOST]
        if not target["available"]:
            raise RuntimeError(f"Component unavailable for {HOST}: {name}")
        components[name] = {
            "version": package["version"], "url": target["xz_url"],
            "expected_sha256": target["xz_hash"],
        }
    report = {
        "scope": __doc__, "producer": identity(Path(__file__).resolve()),
        "manifest": manifest_record, "channel_date": channel["date"],
        "host": HOST, "prefix": str(prefix), "components": components,
        "started_unix": time.time(), "repository_compilation": False,
    }
    save(report_path, report)

    def fetch(name: str) -> tuple[str, dict]:
        row = components[name]
        destination = dist / Path(urlparse(row["url"]).path).name
        return name, download(row["url"], destination, row["expected_sha256"])

    with ThreadPoolExecutor(max_workers=len(COMPONENTS)) as pool:
        for name, record in pool.map(fetch, COMPONENTS):
            components[name]["download"] = record
            save(report_path, report)
            print(f"Downloaded and verified {name}: {record['bytes']} bytes", flush=True)
    prefix.mkdir(exist_ok=True)
    for name in COMPONENTS:
        archive = Path(components[name]["download"]["path"])
        with tempfile.TemporaryDirectory(prefix=name + "-", dir=work) as directory:
            extracted = Path(directory)
            with tarfile.open(archive, "r:xz") as bundle:
                bundle.extractall(extracted, filter="data")
            roots = list(extracted.iterdir())
            if len(roots) != 1 or not (roots[0] / "install.sh").is_file():
                raise RuntimeError(f"Unexpected Rust distribution layout: {archive}")
            command = ["sh", str(roots[0] / "install.sh"), f"--prefix={prefix}", "--disable-ldconfig"]
            log_path = work / f"install-{name}.log"
            with log_path.open("w", encoding="utf-8") as log:
                result = subprocess.run(command, stdout=log, stderr=log)
            components[name]["installation"] = {
                "command": command, "returncode": result.returncode, "log": identity(log_path),
            }
            save(report_path, report)
            if result.returncode:
                raise RuntimeError(f"Component installation failed; see {log_path}")
        print(f"Installed {name}", flush=True)
    tool_dir = prefix / "lib/rustlib" / HOST / "bin"
    environment = {**os.environ, "LD_LIBRARY_PATH": str(prefix / "lib") + (
        os.pathsep + os.environ["LD_LIBRARY_PATH"] if os.environ.get("LD_LIBRARY_PATH") else "")}
    compiler = subprocess.check_output([prefix / "bin/rustc", "-vV"], env=environment, text=True)
    fields = dict(line.split(": ", 1) for line in compiler.splitlines() if ": " in line)
    if fields.get("release") != args.version or fields.get("host") != HOST:
        raise RuntimeError(f"Installed compiler identity differs from the requested toolchain: {compiler}")
    if args.version == RECORDED_COMPILER["release"]:
        for key, expected in RECORDED_COMPILER.items():
            if fields.get(key) != expected:
                raise RuntimeError(f"Compiler differs from the retained benchmark: {key}")
    tools = {
        "rustc": prefix / "bin/rustc", "cargo": prefix / "bin/cargo",
        "llvm-profdata": tool_dir / "llvm-profdata", "llvm-ar": tool_dir / "llvm-ar",
        "llvm-dis": tool_dir / "llvm-dis", "llvm-nm": tool_dir / "llvm-nm",
        "rust-lld": tool_dir / "rust-lld",
    }
    report.update(
        compiler=compiler,
        cargo=subprocess.check_output([tools["cargo"], "-V"], env=environment, text=True).strip(),
        llvm_profdata=subprocess.check_output([tools["llvm-profdata"], "--version"], env=environment, text=True).strip(),
        executables={name: identity(path) for name, path in tools.items()},
        finished_unix=time.time(),
    )
    shell_environment = work / "environment.sh"
    shell_environment.write_text(
        "# Source explicitly; no user or system shell configuration is modified.\n"
        f"export FERRO_TOOLCHAIN_PREFIX={shlex.quote(str(prefix))}\n"
        f"export PATH={shlex.quote(str(prefix / 'bin'))}:{shlex.quote(str(tool_dir / 'gcc-ld'))}:\"$PATH\"\n"
        f"export LD_LIBRARY_PATH={shlex.quote(str(prefix / 'lib'))}${{LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}}\n",
        encoding="utf-8",
    )
    report["shell_environment"] = identity(shell_environment)
    save(report_path, report)
    print(json.dumps({"manifest": str(report_path), "compiler": compiler, "cargo": report["cargo"]}, indent=2))


if __name__ == "__main__":
    main()
