#!/usr/bin/env python3
"""Converts the markup files of an upstream theme project to FerroUI markup.

The conversion is mechanical and changes names only:

* the default XML namespace `https://github.com/avaloniaui` -> `https://github.com/ferroui`
* the asset scheme `avares://` -> `ferres://`
* the root namespace `Avalonia.` -> `FerroUI.` (type, namespace and assembly names)

Everything else (resources, selectors, templates, setters, order, whitespace,
byte order mark) is kept byte for byte. A file that still contains the
upstream name after the conversion is reported and fails the run.

One structural change is supported, for control themes whose control is not
ported yet: `--exclude-list <file>` names a list (`<file name> | <reason>` per
line, `#` comments) of documents under `Controls/`; the element that includes
such a document (a line `<MergeResourceInclude Source=".../Controls/<file name>" />`)
is dropped from the including document. The excluded documents themselves are
still converted.

usage: convert_theme_xaml.py <upstream project dir> <target dir> [--link <upstream file>=<relative target path> ...]
                             [--exclude-list <file>] [--check]

`--check` writes nothing and fails when a target file differs from the
conversion of its upstream file.
"""
import re
import sys
from pathlib import Path

REPLACEMENTS = [
    (b"https://github.com/avaloniaui", b"https://github.com/ferroui"),
    (b"avares://", b"ferres://"),
    (b"Avalonia.", b"FerroUI."),
]
FORBIDDEN = re.compile(rb"avalonia|\bavn|axaml|avares", re.IGNORECASE)


def convert(data: bytes, excluded=()) -> bytes:
    for old, new in REPLACEMENTS:
        data = data.replace(old, new)
    if excluded:
        kept = []
        for line in data.splitlines(keepends=True):
            include = re.search(rb'<MergeResourceInclude\s+Source="[^"]*/Controls/([^"/]+)"\s*/>', line)
            if include and include.group(1).decode() in excluded and line.strip().startswith(b"<MergeResourceInclude"):
                continue
            kept.append(line)
        data = b"".join(kept)
    return data


def read_exclude_list(path: Path):
    names = []
    for line in path.read_text().splitlines():
        line = line.strip()
        if line and not line.startswith("#"):
            names.append(line.split("|", 1)[0].strip())
    return names


def main() -> int:
    args = [a for a in sys.argv[1:] if a != "--check"]
    check = "--check" in sys.argv[1:]
    links = []
    while "--link" in args:
        i = args.index("--link")
        source, target = args[i + 1].split("=", 1)
        links.append((Path(source), Path(target)))
        del args[i : i + 2]
    excluded = []
    if "--exclude-list" in args:
        i = args.index("--exclude-list")
        excluded = read_exclude_list(Path(args[i + 1]))
        del args[i : i + 2]
    if len(args) != 2:
        print(__doc__)
        return 2
    upstream, target = Path(args[0]), Path(args[1])

    files = [
        (p, target / p.relative_to(upstream))
        for p in sorted(upstream.rglob("*.xaml"))
        if not {"bin", "obj"} & set(p.relative_to(upstream).parts)
    ]
    files += [(source, target / relative) for source, relative in links]

    failed = False
    for source, destination in files:
        converted = convert(source.read_bytes(), excluded)
        if FORBIDDEN.search(converted):
            print(f"upstream name left in {destination}: {FORBIDDEN.findall(converted)[:5]}")
            failed = True
        if check:
            if not destination.exists() or destination.read_bytes() != converted:
                print(f"differs from upstream: {destination}")
                failed = True
        else:
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(converted)
    print(f"{len(files)} files {'checked' if check else 'converted'}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
