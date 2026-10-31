#!/usr/bin/env python3
"""Extract the pinned WebScene NuGet distribution for QML integration tests."""
import argparse
import hashlib
import json
import os
import platform
import shutil
import tempfile
import urllib.request
import zipfile
from pathlib import Path

VERSION = "1.0.37"
PACKAGES = {
    "linux-x64": "7f274f3f18eddc79aa73a12d2eb0022f53622f58b35a1f9fc689f4b8599a4830",
    "linux-arm64": "6d95eca6af1e768e0595a4ca8e6d016f35f799c45f8e01a8f63280019ab077b2",
    "osx-arm64": "8ec5fc7d57a8ccf711cc9557549506073812c2d9520193f1e56e189a729a9b7d",
    "osx-x64": "e8640256b431b5882a83df355687e61b4e4563ee0e7919d50334db9058844cee",
    "win-x64": "daeeb81b4fcda098504f233779e016296069160d46afe928177bae7f10ac9588",
}


def native_rid():
    os_name = {"Linux": "linux", "Darwin": "osx", "Windows": "win"}.get(platform.system())
    arch = {"x86_64": "x64", "amd64": "x64", "aarch64": "arm64", "arm64": "arm64"}.get(
        platform.machine().lower()
    )
    return f"{os_name}-{arch}"


def unpack(package, directory, rid):
    digest = hashlib.file_digest(package, "sha256").hexdigest()
    if digest != PACKAGES[rid]:
        raise ValueError(f"SHA-256 mismatch for WebScene {VERSION} {rid}")
    prefix = f"runtimes/{rid}/native/"
    with zipfile.ZipFile(package) as archive:
        manifest = json.loads(archive.read(prefix + "webscene-native-runtime.json"))
        if (manifest["packageVersion"], manifest["runtimeIdentifier"], manifest["abiVersion"],
                manifest["v8Revision"]) != (VERSION, rid, 3, "15.3.10"):
            raise ValueError("unexpected native runtime manifest")
        for info in archive.infolist():
            if info.is_dir():
                continue
            if info.filename.startswith(prefix):
                destination = directory / info.filename.removeprefix(prefix)
            elif info.filename == "LICENSE" or info.filename.startswith("licenses/"):
                destination = directory / info.filename
            else:
                continue
            if not destination.resolve().is_relative_to(directory):
                raise ValueError("invalid package path")
            destination.parent.mkdir(exist_ok=True, parents=True)
            with archive.open(info) as source, destination.open("wb") as target:
                shutil.copyfileobj(source, target)
    return manifest["fileName"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path, help="destination for runtime binaries and notices")
    parser.add_argument("--rid", choices=PACKAGES, default=native_rid())
    parser.add_argument("--package", type=Path, help="use a previously downloaded .nupkg")
    parser.add_argument("--github-env", action="store_true", help="export FERROUI_WEBSCENE_LIBRARY to Actions")
    args = parser.parse_args()
    if args.rid not in PACKAGES:
        parser.error("unsupported host; choose an explicit --rid")
    directory = args.directory.resolve()
    directory.mkdir(parents=True, exist_ok=True)
    name = "webscene.nativeengine.runtime." + args.rid
    if args.package:
        with args.package.open("rb") as package:
            filename = unpack(package, directory, args.rid)
    else:
        url = f"https://api.nuget.org/v3-flatcontainer/{name}/{VERSION}/{name}.{VERSION}.nupkg"
        with tempfile.TemporaryFile() as package:
            with urllib.request.urlopen(url, timeout=120) as response:
                shutil.copyfileobj(response, package)
            package.seek(0)
            filename = unpack(package, directory, args.rid)
    library = directory / filename
    if args.github_env:
        with open(os.environ["GITHUB_ENV"], "a", encoding="utf-8") as env:
            env.write(f"FERROUI_WEBSCENE_LIBRARY={library}\n")
    print(library)


if __name__ == "__main__":
    main()
