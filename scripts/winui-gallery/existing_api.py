#!/usr/bin/env python3
"""Guard the pre-WinUI FerroUI declared API surface; offline by default.

This is a source-declaration compatibility check, not a Rust semver, signature,
default-value, ABI or runtime-behavior proof. The independent baseline is pinned
to the repository commit before this compatibility work.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
import tarfile
import tempfile

import scan


REPO = Path(__file__).resolve().parents[2]
DATA = REPO / "docs/porting/winui-gallery/data"
BASE_COMMIT = "86b6b4ab41669bf830fba681504a4ee94752a4e6"
BASELINE_NAME = "ferroui-api-baseline.json"
LOCK_NAME = "ferroui-api-baseline.lock.json"
LIMITS = [
    "Source-declaration compatibility only; signatures, defaults, accessibility through module re-exports, ABI and behavior are not proved.",
    "The lexical collector covers crates with explicit namespace tables, declared public types, generated virtual traits and source-registered markup types.",
    "Member names are lexical sets, not overload signatures; inherited member relocation is conservatively reported as a removal.",
    "Source registration is lexical evidence, not runtime loader execution; conditional compilation is not evaluated.",
    "Existing duplicate declaration origins are retained, but adding another origin for an identity requires explicit review.",
]


def public_declaration(source: str, name: str) -> bool:
    """Recognize explicit external visibility without promoting pub(crate)."""
    escaped = re.escape(name)
    return bool(re.search(
        r"\bpub\s+(?:(?:unsafe|auto)\s+)*(?:struct|enum|union|trait|type)\s+"
        + escaped + r"\b", source
    ) or re.search(r"\bvirtuals\s+" + escaped + r"\s*:", source))


def collect_surface(repo: Path) -> dict[str, dict]:
    """Use the parity collector's namespace/member logic, filtering helpers."""
    result = {}
    source_cache = {}
    for identity, item in sorted(scan.collect_rust(repo).items()):
        declaration_paths = []
        public = False
        for location in item["sources"]:
            relative = location["path"]
            if relative not in source_cache:
                source_cache[relative] = scan.blank_comments_and_literals(
                    (repo / relative).read_text(encoding="utf-8"))
            is_public = public_declaration(source_cache[relative], item["name"])
            public |= is_public
            if is_public or item["registered"]:
                declaration_paths.append(relative)
        if not declaration_paths:
            continue
        result[identity] = {
            "id": identity,
            "namespace": item["namespace"],
            "name": item["name"],
            "base": item["base"],
            "members": sorted(item["members"]),
            "registered": bool(item["registered"]),
            "public_declaration": public,
            "declaration_paths": sorted(set(declaration_paths)),
        }
    return result


def compatibility_errors(baseline: dict[str, dict], current: dict[str, dict]) -> list[str]:
    """Only additive identities/members/registration are accepted automatically."""
    errors = []
    for identity, before in sorted(baseline.items()):
        after = current.get(identity)
        if after is None:
            errors.append(f"Removed or renamed type/namespace: {identity}")
            continue
        if before["base"] != after["base"]:
            errors.append(f"Base changed for {identity}: {before['base']!r} -> {after['base']!r}")
        removed = sorted(set(before["members"]) - set(after["members"]))
        if removed:
            errors.append(f"Removed declared members from {identity}: {', '.join(removed)}")
        if before["registered"] and not after["registered"]:
            errors.append(f"Source registration removed: {identity}")
        if before["public_declaration"] and not after["public_declaration"]:
            errors.append(f"Public declaration visibility removed: {identity}")
    for identity, after in sorted(current.items()):
        before = baseline.get(identity)
        allowed_origins = len(before["declaration_paths"]) if before else 1
        if len(after["declaration_paths"]) > allowed_origins:
            errors.append(
                f"Namespace identity collision needs review for {identity}: "
                + ", ".join(after["declaration_paths"]))
    return errors


def extract_reference(repo: Path, destination: Path) -> None:
    """Read the pinned Git object tree without changing any checkout/index."""
    archive = subprocess.Popen(
        ["git", "-C", str(repo), "archive", "--format=tar", BASE_COMMIT, "src"],
        stdout=subprocess.PIPE,
    )
    try:
        with tarfile.open(fileobj=archive.stdout, mode="r|") as entries:
            for entry in entries:
                path = PurePosixPath(entry.name)
                if not entry.isfile() or path.parts[0] != "src":
                    continue
                if path.is_absolute() or ".." in path.parts:
                    raise ValueError(f"Unsafe path in pinned Git tree: {entry.name}")
                if path.suffix != ".rs" and path.name != "Cargo.toml":
                    continue
                output = destination.joinpath(*path.parts)
                output.parent.mkdir(parents=True, exist_ok=True)
                source = entries.extractfile(entry)
                if source is None:
                    raise ValueError(f"Unreadable source in pinned Git tree: {entry.name}")
                with source, output.open("wb") as target:
                    shutil.copyfileobj(source, target)
        if archive.wait() != 0:
            raise ValueError("Pinned FerroUI Git tree is unavailable; an offline check only needs the committed baseline")
    finally:
        if archive.stdout:
            archive.stdout.close()
        if archive.poll() is None:
            archive.terminate()
            archive.wait()


def snapshot(repo: Path) -> tuple[dict, dict]:
    commit = scan.git(repo, "rev-parse", BASE_COMMIT + "^{commit}")
    if commit != BASE_COMMIT:
        raise ValueError("FerroUI baseline must use the exact pre-WinUI commit")
    with tempfile.TemporaryDirectory(prefix="ferroui-api-baseline-") as temporary:
        root = Path(temporary)
        extract_reference(repo, root)
        surface = collect_surface(root)
    if not surface:
        raise ValueError("Pinned FerroUI source produced an empty API baseline")
    baseline = {
        "schema_version": 1,
        "commit": commit,
        "meaning": "Existing FerroUI namespace-scoped public/source-registered declarations; additive compatibility guard only.",
        "limitations": LIMITS,
        "types": list(surface.values()),
    }
    lock = {
        "schema_version": 1,
        "repository": "https://github.com/wieslawsoltes/FerroUI",
        "commit": commit,
        "tree": scan.git(repo, "rev-parse", BASE_COMMIT + "^{tree}"),
        "src_tree": scan.git(repo, "rev-parse", BASE_COMMIT + ":src"),
        "snapshot": BASELINE_NAME,
        "snapshot_sha256": scan.digest(scan.canonical(baseline).encode("utf-8")),
        "scope": "Source-declaration compatibility only. Reviewed WinUI handmaps and behavioral evidence remain independent.",
    }
    return baseline, lock


def validate_baseline(baseline: dict, lock: dict) -> dict[str, dict]:
    if baseline.get("schema_version") != 1 or lock.get("schema_version") != 1:
        raise ValueError("Unsupported FerroUI baseline schema")
    if baseline.get("commit") != BASE_COMMIT or lock.get("commit") != BASE_COMMIT:
        raise ValueError("FerroUI API baseline pin changed; explicit baseline review is required")
    if lock.get("snapshot") != BASELINE_NAME:
        raise ValueError("FerroUI baseline lock names another snapshot")
    if scan.digest(scan.canonical(baseline).encode("utf-8")) != lock.get("snapshot_sha256"):
        raise ValueError("FerroUI baseline snapshot differs from its lock")
    rows = baseline.get("types", [])
    if not rows:
        raise ValueError("FerroUI baseline must not be empty")
    result = {}
    for row in rows:
        identity = row["id"]
        if identity != row["namespace"] + "." + row["name"] or identity in result:
            raise ValueError(f"Duplicate or invalid FerroUI baseline identity: {identity}")
        if row["members"] != sorted(set(row["members"])):
            raise ValueError(f"Invalid member set for {identity}")
        if not isinstance(row["registered"], bool) or not isinstance(row["public_declaration"], bool):
            raise ValueError(f"Invalid declaration flags for {identity}")
        if row["base"] is not None and not isinstance(row["base"], str):
            raise ValueError(f"Invalid base for {identity}")
        paths = row["declaration_paths"]
        if not paths or paths != sorted(set(paths)) or any(
            PurePosixPath(path).is_absolute() or ".." in PurePosixPath(path).parts
            for path in paths
        ):
            raise ValueError(f"Invalid declaration origins for {identity}")
        result[identity] = row
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Offline comparison against the committed baseline (default)")
    parser.add_argument("--refresh-baseline", action="store_true", help="Rebuild from the fixed pre-WinUI Git commit; does not change that commit")
    parser.add_argument("--verify-reference", action="store_true", help="Rebuild the pinned Git reference and require byte-for-byte snapshot/lock equality")
    parser.add_argument("--repo", type=Path, default=REPO)
    args = parser.parse_args()
    if args.refresh_baseline and args.verify_reference:
        parser.error("--refresh-baseline and --verify-reference are mutually exclusive")
    data = args.repo / "docs/porting/winui-gallery/data"
    if args.refresh_baseline or args.verify_reference:
        baseline, lock = snapshot(args.repo)
        for name, value in ((BASELINE_NAME, baseline), (LOCK_NAME, lock)):
            path = data / name
            expected = scan.canonical(value)
            if args.verify_reference:
                if not path.is_file() or path.read_text(encoding="utf-8") != expected:
                    raise ValueError(f"FerroUI baseline is not reproducible from pinned source: {name}")
            else:
                data.mkdir(parents=True, exist_ok=True)
                path.write_text(expected, encoding="utf-8")
    baseline = validate_baseline(scan.load(data / BASELINE_NAME), scan.load(data / LOCK_NAME))
    current = collect_surface(args.repo)
    errors = compatibility_errors(baseline, current)
    if errors:
        print("Existing FerroUI declared API changed:")
        print("\n".join("- " + error for error in errors))
        return 1
    members = sum(len(row["members"]) for row in baseline.values())
    registrations = sum(row["registered"] for row in baseline.values())
    additions = len(set(current) - set(baseline))
    print(f"Preserved {len(baseline)} existing namespaced types, {members} declared member names and {registrations} source registrations; {additions} additive types.")
    print("Source-declaration compatibility only; signatures, defaults and runtime behavior require separate review and tests.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        raise SystemExit(str(error)) from error
