#!/usr/bin/env python3
"""The uses of deprecated FerroUI items in the samples (#423 C6, docs/idiomatic-rust.md).

The compiler knows which item a path names, so this reads its warnings instead of matching
names. It checks the samples (or the packages given), keeps the `deprecated` warnings whose code
is in `samples/` and whose item is a FerroUI item, and writes them as text that can be compared
between runs. With `--apply` it replaces each use of a deprecated type alias by the type the
alias names, which is the only change it can make without knowing more. Every other use is listed
for a person.

    deprecated_uses.py                       check the samples, print the uses
    deprecated_uses.py --write <file>        the same, into <file>
    deprecated_uses.py --apply               also rewrite the uses of deprecated type aliases
    deprecated_uses.py --from <log> ...      read the warnings of an earlier `cargo` run (its JSON
                                             messages or its rendered text) instead of checking
    deprecated_uses.py -p <package> ...      check these packages instead of the samples
    deprecated_uses.py --exclude <package>   leave a package out (one that builds for another target)

Only the standard library, and `cargo` on PATH for a check.
"""

import json
import os
import re
import subprocess
import sys
import tomllib

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
SAMPLES = "samples/"
# `use of deprecated type alias `ferroui_base::animation::TimeSpan``, and the other kinds.
MESSAGE = re.compile(r"use of deprecated ([a-z ]+?) `([^`]+)`")
RENDERED_AT = re.compile(r"^\s*--> (.+?):(\d+):(\d+)\s*$")
ALIAS = re.compile(
    r"#\[deprecated(?:\((?:[^()]|\([^()]*\))*\))?\]\s*(?:#\[[^\]]*\]\s*)*pub\s+type\s+(\w+)\s*=\s*(\w+)\s*;"
)


def sample_packages():
    """The packages of the workspace under samples/."""
    with open(os.path.join(ROOT, "Cargo.toml"), "rb") as handle:
        members = tomllib.load(handle)["workspace"]["members"]
    packages = []
    for member in members:
        if not member.startswith(SAMPLES):
            continue
        manifest = os.path.join(ROOT, member, "Cargo.toml")
        if os.path.exists(manifest):
            with open(manifest, "rb") as handle:
                packages.append(tomllib.load(handle)["package"]["name"])
    return sorted(packages)


def deprecated_aliases():
    """Each deprecated type alias of the FerroUI sources and the type it names."""
    aliases = {}
    for folder in ("src", "components"):
        for directory, _, files in os.walk(os.path.join(ROOT, folder)):
            for name in files:
                if not name.endswith(".rs"):
                    continue
                with open(os.path.join(directory, name), encoding="utf-8", errors="replace") as handle:
                    for alias, target in ALIAS.findall(handle.read()):
                        aliases[alias] = target
    return aliases


def relative(path):
    path = path.replace("\\", "/")
    root = ROOT.replace("\\", "/").rstrip("/") + "/"
    if path.startswith(root):
        return path[len(root):]
    # A path of another checkout (a log of CI): from the first `samples/` on.
    at = path.find("/" + SAMPLES)
    return path[at + 1:] if at >= 0 else path


def uses_of_messages(lines):
    """The deprecation warnings of `cargo --message-format=json` output."""
    uses = []
    for line in lines:
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        message = record.get("message") if record.get("reason") == "compiler-message" else None
        if not message or (message.get("code") or {}).get("code") != "deprecated":
            continue
        found = MESSAGE.search(message.get("message", ""))
        spans = [span for span in message.get("spans", []) if span.get("is_primary")]
        if found and spans:
            span = spans[0]
            uses.append((relative(span["file_name"]), span["line_start"], span["column_start"], found.group(1),
                         found.group(2)))
    return uses


def uses_of_rendered(lines):
    """The deprecation warnings of the rendered text of a `cargo` run (a log of CI)."""
    uses = []
    pending = None
    for line in lines:
        text = re.sub(r"^\S+Z ", "", line.rstrip("\n"))
        found = MESSAGE.search(text)
        if text.lstrip().startswith("warning:") and found:
            pending = found
            continue
        at = RENDERED_AT.match(text)
        if pending and at:
            uses.append((relative(at.group(1)), int(at.group(2)), int(at.group(3)), pending.group(1),
                         pending.group(2)))
            pending = None
    return uses


def is_kept(use):
    path, _, _, _, item = use
    return path.startswith(SAMPLES) and (item.startswith("ferroui") or item.startswith("ferrodock"))


def check(packages, excluded):
    command = ["cargo", "check", "--locked", "--all-targets", "--message-format=json"]
    for package in packages:
        if package not in excluded:
            command += ["-p", package]
    run = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, check=False)
    if run.returncode != 0:
        sys.stderr.write(run.stderr[-4000:])
        raise SystemExit("the check of the samples failed")
    return uses_of_messages(run.stdout.splitlines())


def apply(uses, aliases):
    """Replaces each use of a deprecated type alias by its type; the uses it did not change."""
    by_file = {}
    left = []
    for use in uses:
        path, line, column, kind, item = use
        alias = item.rsplit("::", 1)[-1]
        if kind == "type alias" and alias in aliases:
            by_file.setdefault(path, []).append((line, column, alias, aliases[alias]))
        else:
            left.append(use)
    for path, edits in by_file.items():
        full = os.path.join(ROOT, path)
        with open(full, encoding="utf-8", newline="") as handle:
            lines = handle.read().split("\n")
        # From the end of a line backwards, so that the columns before stay right.
        for line, column, alias, target in sorted(edits, reverse=True):
            text = lines[line - 1]
            start = column - 1
            if text[start:start + len(alias)] == alias:
                lines[line - 1] = text[:start] + target + text[start + len(alias):]
            else:
                left.append((path, line, column, "type alias", alias))
        with open(full, "w", encoding="utf-8", newline="") as handle:
            handle.write("\n".join(lines))
    return left


def report(uses, source):
    lines = [
        "# Uses of deprecated FerroUI items in the samples (#423 C6).",
        "# Written by scripts/idioms/deprecated_uses.py; rerun it on main to update this file.",
        f"# Source: {source}",
        f"# Uses: {len(uses)}",
    ]
    for path, line, column, kind, item in sorted(set(uses)):
        lines.append(f"{path}:{line}:{column}\t{kind}\t{item}")
    return "\n".join(lines) + "\n"


def main(argv):
    write, logs, packages, excluded, do_apply = None, [], [], set(), False
    index = 0
    while index < len(argv):
        argument = argv[index]
        if argument in ("--write", "--from", "-p", "--exclude") and index + 1 < len(argv):
            value = argv[index + 1]
            index += 2
            if argument == "--write":
                write = value
            elif argument == "--from":
                logs.append(value)
            elif argument == "-p":
                packages.append(value)
            else:
                excluded.add(value)
        elif argument == "--apply":
            do_apply, index = True, index + 1
        else:
            print(__doc__)
            return 2
    if logs:
        uses = []
        for log in logs:
            with open(log, encoding="utf-8", errors="replace") as handle:
                lines = handle.read().splitlines()
            uses += uses_of_messages(lines) or uses_of_rendered(lines)
        source = "the logs " + ", ".join(os.path.basename(log) for log in logs)
    else:
        packages = packages or sample_packages()
        uses = check(packages, excluded)
        checked = [package for package in packages if package not in excluded]
        source = f"cargo check --all-targets of {len(checked)} packages"
    uses = [use for use in uses if is_kept(use)]
    if do_apply:
        uses = apply(uses, deprecated_aliases())
    text = report(uses, source)
    if write:
        with open(os.path.join(ROOT, write), "w", encoding="utf-8") as handle:
            handle.write(text)
    sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
