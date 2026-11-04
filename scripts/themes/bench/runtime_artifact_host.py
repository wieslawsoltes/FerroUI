#!/usr/bin/env python3
"""Read-only host checks for imported production runtime-comparison artifacts.

The caller supplies already authenticated CI manifests and a sanitized process
environment. These checks neither compile Rust nor start an application. ELF
dependency checks use the host interpreter's --verify/--list modes, never ldd.
Each call owns a new output directory and returns an explicit pending result
when the required toolchain, sysroot or ABI proof is unavailable.
"""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import struct
import subprocess
import sys
import tarfile


HOST = "x86_64-unknown-linux-gnu"
COMPONENTS = {"rustc", "rust-std", "cargo", "llvm-tools-preview"}
RUST_TOOLS = {"rustc", "cargo", "llvm-profdata", "llvm-ar", "llvm-dis", "llvm-nm", "rust-lld"}


def identity(path: Path) -> dict:
    path = path.resolve(strict=True)
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    return {"path": str(path), "bytes": path.stat().st_size, "sha256": digest}


def content_identity(record: dict) -> tuple:
    return record["bytes"], record["sha256"]


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def json_digest(value) -> str:
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


class Evidence:
    def __init__(self, output: Path, scope: str, env: dict):
        self.output = output.resolve()
        self.output.mkdir(parents=True, exist_ok=False)
        self.env = {**env, "LC_ALL": "C"}
        self.result = {
            "schema_version": 1, "scope": scope, "producer": identity(Path(__file__)),
            "complete": False, "pending": [], "commands": [],
            "environment": {name: self.env.get(name) for name in
                            ("PATH", "LD_LIBRARY_PATH", "LD_PRELOAD", "LD_AUDIT", "LC_ALL")},
            "result_path": str(self.output / "result.json"),
        }
        self.save()

    def save(self):
        temporary = self.output / "result.tmp"
        temporary.write_text(json.dumps(self.result, indent=2, sort_keys=True) + "\n")
        temporary.replace(self.output / "result.json")

    def pending(self, error):
        self.result["pending"].append(str(error))
        self.save()

    def check_environment(self):
        require(not self.env.get("LD_PRELOAD") and not self.env.get("LD_AUDIT"),
                "Remove LD_PRELOAD/LD_AUDIT before host verification; injected loaders are unsupported")

    def run(self, argv: list, label: str) -> tuple[dict, str, str]:
        stdout = self.output / (label + ".stdout")
        stderr = self.output / (label + ".stderr")
        require(not stdout.exists() and not stderr.exists(), f"Duplicate host probe label: {label}")
        row = {"argv": list(map(str, argv)), "label": label, "exit_code": None}
        self.result["commands"].append(row)
        self.save()
        try:
            with stdout.open("wb") as out, stderr.open("wb") as err:
                process = subprocess.run(row["argv"], env=self.env, cwd=self.output,
                                         stdout=out, stderr=err, timeout=30)
                row["exit_code"] = process.returncode
        except (OSError, subprocess.TimeoutExpired) as error:
            row["error"] = str(error)
            raise
        finally:
            for name, path in (("stdout", stdout), ("stderr", stderr)):
                if path.exists():
                    row[name] = identity(path)
            self.save()
        return row, stdout.read_text(errors="replace"), stderr.read_text(errors="replace")

    def finish(self):
        self.result["complete"] = not self.result["pending"]
        self.save()
        return self.result


def mapped_tool(expected: dict, recorded_prefix: Path, local_prefix: Path) -> dict:
    original = Path(expected["path"])
    require(original.is_absolute() and ".." not in original.parts, "Invalid recorded tool path")
    relative = original.relative_to(recorded_prefix)
    actual = identity(local_prefix / relative)
    require(content_identity(actual) == content_identity(expected), f"Rust/LLVM tool differs: {relative}")
    return {"recorded": expected, "relative_path": str(relative), "local": actual}


def verify_sysroot(prefix: Path, archive: Path, expected_sha: str, evidence: Evidence,
                   llvm_archive: tuple[Path, str] | None = None) -> dict:
    distributions = [("rust-std", archive, expected_sha)]
    if llvm_archive is not None:
        distributions.append(("llvm-tools-preview", *llvm_archive))
    result = {"archives": {}, "files": {}, "scope": "Exact target sysroot .rlib/.rmeta/.so files from verified rust-std and llvm-tools distributions; no extraction. Sanitizer .a archives from rustc are outside the unchanged companion flags."}
    evidence.result["sysroot"] = result
    evidence.save()
    root = prefix / "lib/rustlib" / HOST / "lib"

    def relevant(path):
        return path.name.endswith((".rlib", ".rmeta")) or ".so" in path.name

    # Tar member names include the release/component directory before lib/.
    anchor = ("lib", "rustlib", HOST, "lib")
    for component, distribution, expected_hash in distributions:
        archive_identity = identity(distribution)
        require(archive_identity["sha256"] == expected_hash, f"Retained {component} archive has the wrong distribution hash")
        result["archives"][component] = archive_identity
        with tarfile.open(distribution, "r:xz") as bundle:
            for member in bundle:
                parts = PurePosixPath(member.name).parts
                offsets = [i for i in range(len(parts) - len(anchor) + 1) if tuple(parts[i:i + len(anchor)]) == anchor]
                if not offsets or not relevant(PurePosixPath(member.name)):
                    continue
                require(len(offsets) == 1 and not member.name.startswith("/") and ".." not in parts,
                        f"Unsupported distribution archive path: {member.name}")
                require(member.isfile(), f"Sysroot proof requires a regular distribution member: {member.name}")
                relative = Path(*parts[offsets[0] + len(anchor):])
                require(str(relative) not in result["files"], f"Duplicate sysroot distribution member: {relative}")
                with bundle.extractfile(member) as stream:
                    expected = {"bytes": member.size, "sha256": hashlib.file_digest(stream, "sha256").hexdigest()}
                actual = identity(root / relative)
                require(content_identity(actual) == content_identity(expected), f"Installed sysroot differs: {relative}")
                result["files"][str(relative)] = {"component": component, "archive_member": member.name,
                                                "expected": expected, "installed": actual}
        require(identity(distribution) == archive_identity, f"{component} distribution archive changed during verification")
    require(result["files"], "No target .rlib/.rmeta/.so members found in retained rust-std distribution")
    installed = {str(path.relative_to(root)) for path in root.rglob("*") if path.is_file() and relevant(path)}
    require(installed == set(result["files"]), "Installed sysroot .rlib/.rmeta/.so file set differs from verified distribution members")
    require(any(name.startswith("libstd-") and name.endswith(".rlib") for name in installed), "Missing libstd rlib proof")
    result["complete"] = True
    evidence.save()
    return result


def verify_toolchain(ci_toolchain: dict, arm_builds: dict[str, dict], prefix: Path,
                     local_installation: Path, output: Path, env: dict) -> dict:
    evidence = Evidence(output, "Exact CI/local Rust+LLVM tools and installed target sysroot identity; no compilation.", env)
    try:
        evidence.check_environment()
        prefix = prefix.resolve(strict=True)
        original_installation = identity(local_installation)
        local = json.loads(local_installation.read_text())
        require(Path(local["prefix"]).resolve(strict=True) == prefix, "Local bootstrap installation belongs to another prefix")
        require(ci_toolchain["host"] == local["host"] == HOST, "Toolchain host is not native x86-64 GNU/Linux")
        require(set(ci_toolchain["components"]) == set(local["components"]) == COMPONENTS,
                "Unexpected bootstrap component inventory")
        require(set(ci_toolchain["executables"]) == set(local["executables"]) == RUST_TOOLS,
                "Unexpected bootstrap Rust/LLVM executable inventory")
        require(set(arm_builds) == {"reference", "candidate"}, "Both production arm manifests are required")
        evidence.result.update(ci_toolchain=ci_toolchain, ci_toolchain_canonical_json_sha256=json_digest(ci_toolchain),
                               local_installation=original_installation, local_prefix=str(prefix), components={}, tools={}, arms={})
        for name in sorted(COMPONENTS):
            ci_component, local_component = ci_toolchain["components"][name], local["components"][name]
            require(ci_component["expected_sha256"] == local_component["expected_sha256"],
                    f"Local and CI distribution component differ: {name}")
            require(ci_component["version"] == local_component["version"], f"Component version differs: {name}")
            evidence.result["components"][name] = {
                "expected_sha256": ci_component["expected_sha256"], "version": ci_component["version"],
                "ci_url": ci_component["url"], "local_url": local_component["url"],
            }
        for name, expected in ci_toolchain["executables"].items():
            evidence.result["tools"][name] = mapped_tool(expected, Path(ci_toolchain["prefix"]), prefix)
        for label, build in arm_builds.items():
            require(build.get("complete") is True, f"{label}: production graph has not completed")
            arm_prefix = Path(build["controlled_environment"]["RUSTC"]).parent.parent
            arm = {"build_canonical_json_sha256": json_digest(build), "recorded_prefix": str(arm_prefix),
                   "tools": {}, "unmatched_host_tools": {}}
            evidence.result["arms"][label] = arm
            require(build["rustc"] == ci_toolchain["compiler"], f"{label}: compiler record differs from CI bootstrap")
            require(RUST_TOOLS | {"lld-shim"} <= set(build["tools"]), f"{label}: missing recorded Rust/LLVM tools")
            require(any(name.startswith("librustc_driver") for name in build["tools"]) and
                    any(name.startswith("libLLVM") for name in build["tools"]),
                    f"{label}: missing recorded compiler/LLVM shared-library identities")
            for name, expected in build["tools"].items():
                if name == "cc":
                    arm["unmatched_host_tools"][name] = expected
                else:
                    arm["tools"][name] = mapped_tool(expected, arm_prefix, prefix)
        evidence.save()
        row, stdout, _ = evidence.run([prefix / "bin/rustc", "-vV"], "rustc-version")
        require(row["exit_code"] == 0 and stdout == ci_toolchain["compiler"] == local["compiler"],
                "Actual local rustc -vV differs from the CI compiler")
        std = local["components"]["rust-std"]
        llvm = local["components"]["llvm-tools-preview"]
        verify_sysroot(prefix, Path(std["download"]["path"]), std["expected_sha256"], evidence,
                       (Path(llvm["download"]["path"]), llvm["expected_sha256"]))
        require(identity(local_installation) == original_installation, "Local bootstrap manifest changed during verification")
        for group in [evidence.result["tools"], *[arm["tools"] for arm in evidence.result["arms"].values()]]:
            for record in group.values():
                require(identity(Path(record["local"]["path"])) == record["local"], "Verified Rust/LLVM tool changed")
        for record in evidence.result["sysroot"]["files"].values():
            require(identity(Path(record["installed"]["path"])) == record["installed"], "Verified target sysroot changed")
    except (OSError, ValueError, KeyError, TypeError, RuntimeError, tarfile.TarError, subprocess.TimeoutExpired) as error:
        evidence.pending(f"{type(error).__name__}: {error}")
    return evidence.finish()


def elf_header(path: Path) -> dict:
    with path.open("rb") as stream:
        header = stream.read(64)
    require(len(header) == 64 and header[:7] == b"\x7fELF\x02\x01\x01", "Require ELF64 little-endian version1")
    kind, machine = struct.unpack_from("<HH", header, 16)
    require(kind in (2, 3) and machine == 62, "Require an x86-64 ELF executable/shared library")
    return {"class": "ELF64", "byte_order": "little-endian", "machine": "x86-64", "type": kind}


def interpreter(text: str) -> str | None:
    found = re.findall(r"\[Requesting program interpreter:\s*([^\]]+)\]", text)
    require(len(found) <= 1, "Multiple ELF interpreters are unsupported")
    return found[0].strip() if found else None


def version_requirements(text: str) -> dict:
    requirements = {}
    active = False
    provider = None
    for line in text.splitlines():
        if line.startswith("Version ") and " section " in line:
            active = line.startswith("Version needs section ")
            provider = None
        if not active:
            continue
        match = re.search(r"File:\s*(\S+)\s+Cnt:", line)
        if match:
            provider = match.group(1)
        match = re.search(r"Name:\s*(\S+)\s+Flags:\s*(.*?)\s+Version:", line)
        if match:
            require(provider is not None, "Version need lacks its provider")
            requirements.setdefault(provider, []).append({"name": match.group(1), "flags": match.group(2)})
    return requirements


def version_definitions(text: str) -> set[str]:
    definitions = set()
    active = False
    for line in text.splitlines():
        if line.startswith("Version ") and " section " in line:
            active = line.startswith("Version definition section ")
        if active:
            match = re.search(r"Name:\s*(\S+)", line)
            if match:
                definitions.add(match.group(1))
    return definitions


def loader_resolutions(text: str, loader: Path) -> tuple[dict, list]:
    resolved, virtual = {}, []
    for line in text.splitlines():
        if not line.strip():
            continue
        match = re.fullmatch(r"\s*(\S+)\s+=>\s+(/.+?)\s+\(0x[0-9a-fA-F]+\)\s*", line)
        if match:
            name, location = match.groups()
        else:
            match = re.fullmatch(r"\s*(/.+?)\s+\(0x[0-9a-fA-F]+\)\s*", line)
            if match:
                location = match.group(1)
                name = Path(location).name
            elif re.fullmatch(r"\s*linux-vdso\.so\.1\s+\(0x[0-9a-fA-F]+\)\s*", line):
                virtual.append(line.strip())
                continue
            else:
                raise RuntimeError(f"Unresolved/unsupported loader output: {line.strip()}")
        item = identity(Path(location))
        require(name not in resolved or resolved[name] == item, f"Ambiguous loader resolution: {name}")
        resolved[name] = item
    resolved.setdefault(loader.name, identity(loader))
    return resolved, virtual


def verify_native(paths: list[Path], output: Path, env: dict) -> dict:
    evidence = Evidence(output, "Host ELF loader dependency/version compatibility; --list/--verify only, no application execution.", env)
    inspected = {}
    provider_cache = {}
    try:
        evidence.check_environment()
        require(bool(paths), "At least one native artifact is required")
        readelf = shutil.which("readelf", path=evidence.env.get("PATH"))
        require(readelf is not None, "readelf is required for native ABI proof")
        readelf = Path(readelf).resolve(strict=True)
        evidence.result.update(readelf=identity(readelf), artifacts={})
        python = Path(sys.executable).resolve(strict=True)
        elf_header(python)
        row, text, _ = evidence.run([readelf, "--program-headers", "--wide", python], "host-interpreter")
        require(row["exit_code"] == 0, "Cannot inspect the actual host interpreter")
        requested_loader = interpreter(text)
        require(requested_loader is not None, "Host Python has no ELF interpreter; explicit ABI proof is unavailable")
        loader_path = Path(requested_loader)
        loader = loader_path.resolve(strict=True)
        elf_header(loader)
        evidence.result["loader"] = {"requested_path": str(loader_path), **identity(loader)}
        for index, original in enumerate(paths):
            label = f"artifact-{index:03d}"
            path = original.resolve(strict=True)
            before = identity(path)
            inspected[str(path)] = before
            artifact = {"input_path": str(original), "identity": before, "elf": elf_header(path), "complete": False}
            evidence.result["artifacts"][str(original)] = artifact
            evidence.save()
            try:
                row, text, _ = evidence.run([readelf, "--file-header", "--program-headers", "--dynamic", "--version-info", "--wide", path], label + "-elf")
                require(row["exit_code"] == 0, "readelf could not inspect the artifact")
                require("(AUDIT)" not in text and "(DEPAUDIT)" not in text,
                        "ELF audit modules are outside this read-only loader proof")
                needed = re.findall(r"\(NEEDED\).*?Shared library: \[([^\]]+)\]", text)
                artifact.update(interpreter=interpreter(text), needed=needed,
                                search_paths=re.findall(r"\((RPATH|RUNPATH)\).*?\[([^\]]*)\]", text),
                                version_requirements=version_requirements(text))
                if artifact["interpreter"] is not None:
                    require(Path(artifact["interpreter"]).resolve(strict=True) == loader,
                            "Artifact interpreter differs from the actual host ELF loader")
                    row, _, _ = evidence.run([loader, "--verify", path], label + "-verify")
                    require(row["exit_code"] == 0, "Host loader --verify rejected the executable")
                else:
                    require(artifact["elf"]["type"] == 3, "Static executables are outside this dynamic ABI proof")
                row, listing, errors = evidence.run([loader, "--list", path], label + "-list")
                require(row["exit_code"] == 0 and "not found" not in listing + errors,
                        "Host loader could not resolve required libraries/symbol versions")
                resolved, virtual = loader_resolutions(listing, loader_path)
                artifact.update(resolved_libraries=resolved, virtual_libraries=virtual)
                require(set(needed) <= set(resolved), "Loader output omits a direct DT_NEEDED dependency")
                for provider, requirements in artifact["version_requirements"].items():
                    require(provider in resolved, f"Version provider was not resolved: {provider}")
                    local = resolved[provider]
                    if local["path"] not in provider_cache:
                        number = len(provider_cache)
                        row, definitions, _ = evidence.run([readelf, "--version-info", "--wide", local["path"]], f"provider-{number:03d}-versions")
                        require(row["exit_code"] == 0, "Cannot inspect resolved provider versions")
                        provider_cache[local["path"]] = {"identity": local, "definitions": sorted(version_definitions(definitions))}
                    available = set(provider_cache[local["path"]]["definitions"])
                    missing = [item["name"] for item in requirements if item["name"] not in available and "WEAK" not in item["flags"].upper()]
                    require(not missing, f"{provider} lacks required versions: {missing}")
                artifact["complete"] = True
            except (OSError, ValueError, KeyError, TypeError, RuntimeError, subprocess.TimeoutExpired) as error:
                artifact["pending"] = f"{type(error).__name__}: {error}"
                evidence.pending(f"{original}: {artifact['pending']}")
            evidence.save()
        evidence.result["version_providers"] = provider_cache
        for expected in [*inspected.values(), evidence.result["readelf"],
                         {key: evidence.result["loader"][key] for key in ("path", "bytes", "sha256")},
                         *[item for artifact in evidence.result["artifacts"].values() for item in artifact.get("resolved_libraries", {}).values()]]:
            require(identity(Path(expected["path"])) == expected, "Native artifact, loader or resolved library changed during verification")
    except (OSError, ValueError, KeyError, TypeError, RuntimeError, struct.error, subprocess.TimeoutExpired) as error:
        evidence.pending(f"{type(error).__name__}: {error}")
    return evidence.finish()
