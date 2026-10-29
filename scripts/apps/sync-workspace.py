#!/usr/bin/env python3
"""Writes, or checks, the parts of an application's workspace manifest that come from FerroUI.

An application in a repository of its own has this repository as a git submodule (docs/apps.md).
Its root Cargo.toml is a workspace of its own, and cargo takes neither the profiles nor the
workspace dependencies from another workspace. This script copies them from the Cargo.toml of
this repository, so that the application builds with the same settings and the same versions as
the framework, and nothing is copied by hand:

- the `[workspace.dependencies]` of FerroUI (its crates, with their paths made relative to the
  application, and the pinned third-party versions such as wasm-bindgen, which the browser
  toolchain installs at that version) go between the markers of the dependency block, inside the
  `[workspace.dependencies]` table of the application, before or after its own entries;
- every `[profile.*]` section (release, fast-release, dist, browser with its per-package
  optimisation levels) and any `[patch.*]` section goes between the markers of the settings block.

    python3 FerroUI/scripts/apps/sync-workspace.py [APP_DIR]          write the blocks
    python3 FerroUI/scripts/apps/sync-workspace.py --check [APP_DIR]  change nothing; fail when a
                                                                       block differs (app CI)

APP_DIR is the directory of the application's root Cargo.toml (default: the current directory).
Run it after every update of the submodule. A manifest without the markers gets the blocks: the
dependency block right after its `[workspace.dependencies]` header (or in a new table), the
settings block at the end.
"""

import difflib
import os
import re
import sys

FERROUI = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
HEADER = re.compile(r"^\s*\[\[?\s*([^\]\s]+)\s*\]\]?\s*(#.*)?$")
PATH_VALUE = re.compile(r'(\bpath\s*=\s*")([^"]+)(")')
NOTE = "written by FerroUI/scripts/apps/sync-workspace.py from FerroUI/Cargo.toml; do not edit"
DEPENDENCIES = ("# >>> FerroUI workspace dependencies (" + NOTE + ")",
                "# <<< FerroUI workspace dependencies")
SETTINGS = ("# >>> FerroUI profiles (" + NOTE + ")",
            "# <<< FerroUI profiles")


def sections(lines):
    """(name, start, end) of every table: start is the first line of the comment block right above
    its header, end the line after its last non-blank line."""
    headers = [(i, HEADER.match(line).group(1)) for i, line in enumerate(lines) if HEADER.match(line)]
    starts = []
    for i, name in headers:
        start = i
        while start > 0 and lines[start - 1].lstrip().startswith("#"):
            start -= 1
        starts.append((name, start, i))
    result = []
    for k, (name, start, header) in enumerate(starts):
        end = starts[k + 1][1] if k + 1 < len(starts) else len(lines)
        while end > header + 1 and not lines[end - 1].strip():
            end -= 1
        result.append((name, start, header, end))
    return result


def rebase_paths(lines, prefix):
    return [PATH_VALUE.sub(lambda m: m.group(1) + prefix + "/" + m.group(2) + m.group(3), line)
            for line in lines]


def blocks(prefix):
    with open(os.path.join(FERROUI, "Cargo.toml"), encoding="utf-8") as f:
        lines = f.read().splitlines()
    dependencies, settings = None, []
    for name, start, header, end in sections(lines):
        if name == "workspace.dependencies":
            dependencies = rebase_paths(lines[header + 1:end], prefix)
        elif name.startswith("profile.") or name.startswith("patch.") or name == "replace":
            if settings:
                settings.append("")
            settings.extend(rebase_paths(lines[start:end], prefix))
    if dependencies is None:
        sys.exit("FerroUI/Cargo.toml has no [workspace.dependencies]")
    return dependencies, settings


def replace_block(lines, markers, body):
    begin = [i for i, line in enumerate(lines) if line.strip() == markers[0]]
    end = [i for i, line in enumerate(lines) if line.strip() == markers[1]]
    if not begin and not end:
        return None
    if len(begin) != 1 or len(end) != 1 or end[0] < begin[0]:
        sys.exit("the markers of '%s' are not one pair in order" % markers[1][6:])
    return lines[:begin[0] + 1] + body + lines[end[0]:]


def synchronized(text, prefix):
    dependencies, settings = blocks(prefix)
    lines = text.splitlines()
    updated = replace_block(lines, DEPENDENCIES, dependencies)
    if updated is None:
        found = [header for name, _, header, _ in sections(lines) if name == "workspace.dependencies"]
        block = [DEPENDENCIES[0]] + dependencies + [DEPENDENCIES[1]]
        if found:
            updated = lines[:found[0] + 1] + block + lines[found[0] + 1:]
        else:
            updated = lines + ["", "[workspace.dependencies]"] + block
    lines = updated
    updated = replace_block(lines, SETTINGS, settings)
    if updated is None:
        while lines and not lines[-1].strip():
            lines.pop()
        updated = lines + ["", SETTINGS[0]] + settings + [SETTINGS[1]]
    return "\n".join(updated) + "\n"


def main(argv):
    check = "--check" in argv
    rest = [a for a in argv if a != "--check"]
    if any(a.startswith("-") for a in rest) or len(rest) > 1:
        sys.exit(__doc__)
    app = os.path.abspath(rest[0] if rest else ".")
    manifest = os.path.join(app, "Cargo.toml")
    prefix = os.path.relpath(FERROUI, app).replace(os.sep, "/")
    if prefix.startswith(".."):
        sys.exit("FerroUI (%s) is not inside the application (%s)" % (FERROUI, app))
    with open(manifest, encoding="utf-8") as f:
        text = f.read()
    expected = synchronized(text, prefix)
    if expected == text:
        print("%s: the FerroUI blocks are up to date" % manifest)
        return 0
    if check:
        sys.stdout.writelines(difflib.unified_diff(
            text.splitlines(True), expected.splitlines(True), "Cargo.toml", "Cargo.toml (expected)"))
        print("::error file=Cargo.toml::the FerroUI blocks of Cargo.toml differ from FerroUI/Cargo.toml; "
              "run python3 %s/scripts/apps/sync-workspace.py and commit the result" % prefix)
        return 1
    with open(manifest, "w", encoding="utf-8") as f:
        f.write(expected)
    print("%s: FerroUI blocks written" % manifest)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
