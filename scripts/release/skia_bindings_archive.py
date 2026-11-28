#!/usr/bin/env python3
"""The patched Skia binding as an asset of a FerroUI release (#622, docs/apps.md).

FerroUI's published crates depend on `skia-bindings` from crates.io, which cannot carry FerroUI's
Skia fixes. An application built from the published crates takes them with a `[patch.crates-io]`
of its own that names the patched binding of the FerroUI release it uses. This script makes that
binding a file of the release, and checks that the binding and the crates that use it agree:

    skia_bindings_archive.py --check         the versions and the files of the binding agree
    skia_bindings_archive.py --write <dir>   the archive of the binding and its SHA-256 into <dir>

The archive is `git archive` of `external/rust-skia-bindings` at HEAD, under `skia-bindings/`,
gzip-compressed without a time stamp, so the same commit gives the same bytes.

Only the standard library and `git`.
"""

import gzip
import hashlib
import io
import os
import re
import subprocess
import sys
import tarfile
import tomllib

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
BINDING = "external/rust-skia-bindings"
SKIA_CRATE = "src/Skia/FerroUI.Skia/Cargo.toml"
# What an application needs of the binding besides its sources.
REQUIRED = [
    "Cargo.toml",
    "LICENSE",
    "FERROUI-PATCH.md",
    "patches/graphite-metal-ownership.patch",
    "patches/LICENSE-SKIA",
    "upstream/README.md",
]


def read_toml(path):
    with open(os.path.join(ROOT, path), "rb") as handle:
        return tomllib.load(handle)


def binding_version():
    return read_toml(f"{BINDING}/Cargo.toml")["package"]["version"]


def ferroui_version():
    return read_toml("Cargo.toml")["workspace"]["package"]["version"]


def problems():
    """What does not agree, as sentences; nothing when everything does."""
    found = []
    version = binding_version()
    manifest = read_toml(SKIA_CRATE)
    tables = [manifest.get("dependencies", {})]
    tables += [target.get("dependencies", {}) for target in manifest.get("target", {}).values()]
    wanted = []
    for table in tables:
        required = table.get("skia-bindings")
        if required is not None:
            wanted.append(required.get("version", "") if isinstance(required, dict) else required)
    if not wanted:
        found.append(f"{SKIA_CRATE} does not depend on skia-bindings")
    for requirement in wanted:
        if requirement != f"={version}":
            found.append(f"{SKIA_CRATE} requires skia-bindings {requirement!r}, the binding is {version}")
    with open(os.path.join(ROOT, "Cargo.lock"), encoding="utf-8") as handle:
        locked = re.findall(r'name = "skia-bindings"\nversion = "([^"]+)"', handle.read())
    if locked != [version]:
        found.append(f"Cargo.lock has skia-bindings {locked}, the binding is {version}")
    for name in REQUIRED:
        if not os.path.isfile(os.path.join(ROOT, BINDING, name)):
            found.append(f"{BINDING}/{name} is missing")
    with open(os.path.join(ROOT, BINDING, "FERROUI-PATCH.md"), encoding="utf-8") as handle:
        if version not in handle.read():
            found.append(f"{BINDING}/FERROUI-PATCH.md does not name the version {version}")
    return found


def archive_name():
    return f"skia-bindings-{binding_version()}-ferroui-{ferroui_version()}.tar.gz"


def archive_bytes():
    tar = subprocess.run(
        ["git", "-C", ROOT, "archive", "--format=tar", "--prefix=skia-bindings/", f"HEAD:{BINDING}"],
        capture_output=True,
        check=True,
    ).stdout
    # `git archive` of a tree (`HEAD:<path>`) stamps every entry with the current time, so two
    # archives made in different seconds differ. Every entry gets the time of the commit instead.
    commit_time = int(
        subprocess.run(
            ["git", "-C", ROOT, "log", "-1", "--format=%ct", "HEAD"], capture_output=True, check=True, text=True
        ).stdout.strip()
    )
    tar = restamp(tar, commit_time)
    buffer = io.BytesIO()
    with gzip.GzipFile(filename="", mode="wb", fileobj=buffer, mtime=0) as packed:
        packed.write(tar)
    return buffer.getvalue()


def restamp(tar, mtime):
    """The tar archive `tar` with the modification time of every entry set to `mtime`."""
    source = tarfile.open(fileobj=io.BytesIO(tar), mode="r:")
    output = io.BytesIO()
    with tarfile.open(fileobj=output, mode="w:", format=tarfile.PAX_FORMAT) as packed:
        for member in source.getmembers():
            member.mtime = mtime
            packed.addfile(member, source.extractfile(member) if member.isfile() else None)
    source.close()
    return output.getvalue()


def main(argv):
    if argv == ["--check"]:
        found = problems()
        for problem in found:
            print(f"error: {problem}")
        if not found:
            print(f"skia-bindings {binding_version()} and the crates that use it agree")
        return 1 if found else 0
    if len(argv) == 2 and argv[0] == "--write":
        found = problems()
        if found:
            for problem in found:
                print(f"error: {problem}")
            return 1
        os.makedirs(argv[1], exist_ok=True)
        data = archive_bytes()
        path = os.path.join(argv[1], archive_name())
        with open(path, "wb") as handle:
            handle.write(data)
        digest = hashlib.sha256(data).hexdigest()
        with open(path + ".sha256", "w", encoding="utf-8") as handle:
            handle.write(f"{digest}  {archive_name()}\n")
        print(f"{path}\n{digest}")
        return 0
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
