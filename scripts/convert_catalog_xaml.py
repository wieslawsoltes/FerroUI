#!/usr/bin/env python3
"""Converts the markup files of the upstream ControlCatalog sample to FerroUI markup.

The conversion is mechanical and changes names only, in this order:

1. the default XML namespace `https://github.com/avaloniaui` -> `https://github.com/ferroui`
2. the asset scheme `avares://` -> `ferres://`
3. links to the upstream web site (`https://avaloniaui.net/...`, `https://docs.avaloniaui.net/...`)
   -> the repository of this project (`https://github.com/wieslawsoltes/FerroUI`)
4. the root namespace `Avalonia.` -> `FerroUI.` (type, namespace and assembly names)
5. what is left of the upstream name, which may not appear in a markup file of this
   repository: `AVALONIA` -> `FERRO`, `AvaloniaUI` -> `FerroUI`, the name as the prefix of
   an identifier (`AvaloniaFlixHomeView`, `AvaloniaIcon_OnTapped`) -> `Ferro`, the name as a
   word of a text (`Text="Avalonia"`) -> `FerroUI`, and `an avares URI` -> `a ferres URI`

Everything else (elements, attributes, order, whitespace, byte order mark, line ends) is
kept byte for byte. A file that still contains the upstream name after the conversion is
reported and fails the run.

File names: `.axaml` becomes `.xaml`, and the upstream name in a file name is replaced as in
rule 5 (`AvaloniaFlixAppPage.xaml` -> `FerroFlixAppPage.xaml`). Directories are kept.

usage: convert_catalog_xaml.py <upstream sample dir> <target dir> [--check]

`--check` writes nothing and fails when a target file differs from the conversion of its
upstream file, when a converted file is missing, or when the target holds a markup file
that has no upstream file.
"""
import re
import sys
from pathlib import Path

REPOSITORY = b"https://github.com/wieslawsoltes/FerroUI"

FORBIDDEN = re.compile(rb"avalonia|\bavn|axaml|avares", re.IGNORECASE)


def convert(data: bytes) -> bytes:
    data = data.replace(b"https://github.com/avaloniaui", b"https://github.com/ferroui")
    data = data.replace(b"avares://", b"ferres://")
    data = re.sub(rb"https://(?:docs\.)?avaloniaui\.net[^\"&<\s]*", REPOSITORY, data)
    data = data.replace(b"Avalonia.", b"FerroUI.")
    data = data.replace(b"AVALONIA", b"FERRO")
    data = data.replace(b"AvaloniaUI", b"FerroUI")
    data = re.sub(rb"Avalonia(?=[A-Z_])", b"Ferro", data)
    data = data.replace(b"Avalonia", b"FerroUI")
    data = data.replace(b"an avares URI", b"a ferres URI")
    return data


def convert_name(name: str) -> str:
    name = re.sub(r"\.axaml$", ".xaml", name)
    name = re.sub(r"Avalonia(?=[A-Z_])", "Ferro", name)
    return name.replace("Avalonia", "FerroUI")


def is_markup(path: Path) -> bool:
    return path.suffix in (".xaml", ".axaml")


def main() -> int:
    args = [a for a in sys.argv[1:] if a != "--check"]
    check = "--check" in sys.argv[1:]
    if len(args) != 2:
        print(__doc__)
        return 2
    upstream, target = Path(args[0]), Path(args[1])

    files = []
    for path in sorted(upstream.rglob("*")):
        relative = path.relative_to(upstream)
        if path.is_file() and is_markup(path) and not {"bin", "obj"} & set(relative.parts):
            files.append((path, target / relative.parent / convert_name(relative.name)))

    failed = False
    destinations = set()
    for source, destination in files:
        if destination in destinations:
            print(f"two upstream files convert to {destination}")
            failed = True
        destinations.add(destination)
        converted = convert(source.read_bytes())
        if FORBIDDEN.search(converted) or FORBIDDEN.search(str(destination.relative_to(target)).encode()):
            print(f"upstream name left in {destination}: {FORBIDDEN.findall(converted)[:5]}")
            failed = True
        if check:
            if not destination.exists():
                print(f"missing: {destination}")
                failed = True
            elif destination.read_bytes() != converted:
                print(f"differs from upstream: {destination}")
                failed = True
        else:
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(converted)

    if check and target.exists():
        for path in sorted(target.rglob("*.xaml")):
            if "target" not in path.relative_to(target).parts and path not in destinations:
                print(f"no upstream file for: {path}")
                failed = True

    print(f"{len(files)} files {'checked' if check else 'converted'}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
