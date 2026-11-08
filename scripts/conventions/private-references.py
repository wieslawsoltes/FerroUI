#!/usr/bin/env python3
"""Refuses private references in the tracked files: home directories of a machine, the folders and
branch names of local tools, other hosts of the repository, and access tokens.

    python3 scripts/conventions/private-references.py          # the whole tree
    python3 scripts/conventions/private-references.py FILE...  # the given files

Paths in documentation use placeholders (`<checkout>`, `<evidence>`, `~/`, `../<repository>`),
and generic examples (`/home/user/`, `C:\\Users\\username\\`) are allowed. Exit status 1 lists
every offending line.
"""

import re
import subprocess
import sys

# Account names that are placeholders, not a real machine.
GENERIC = {
    "user", "username", "u", "me", "you", "name", "path", "runner", "runneradmin", "example",
    "someone", "foo", "bar", "test", "tester", "shared", "public", "default", "admin", "<user>", "<name>",
}

# An account directory: the name followed by a separator (`/home/Projects.txt` is a file, not a home).
HOME = re.compile(r"(?:/Users/|/home/|[A-Za-z]:\\{1,2}Users\\{1,2})([A-Za-z0-9_.<>-]+)(?=/|\\)")
FORBIDDEN = [
    (re.compile(r"\.codex\b|\bcodex/", re.I), "a local tool's folder or branch name"),
    (re.compile(r"gitlab\.com|ferro-group", re.I), "another host of the repository"),
    (re.compile(r"\bglpat-[A-Za-z0-9_-]{8,}|\bghp_[A-Za-z0-9]{20,}|\bgithub_pat_[A-Za-z0-9_]{20,}"), "an access token"),
]

# Third-party sources kept as they were published, and this check and its documentation, which name
# the patterns.
SKIP_PREFIXES = ("external/", "components/FerroBoard/reference/", "scripts/conventions/private-references.py",
                 "docs/private-references.md")


def tracked_files():
    output = subprocess.run(["git", "ls-files", "-z"], capture_output=True, check=True).stdout
    return [name for name in output.decode().split("\0") if name]


def check(path):
    problems = []
    try:
        with open(path, encoding="utf-8") as file:
            lines = file.readlines()
    except (UnicodeDecodeError, IsADirectoryError, FileNotFoundError):
        return problems
    for number, line in enumerate(lines, 1):
        for match in HOME.finditer(line):
            if match.group(1).lower() not in GENERIC:
                problems.append((path, number, "the home directory of a machine", line.strip()))
        for pattern, what in FORBIDDEN:
            if pattern.search(line):
                problems.append((path, number, what, line.strip()))
    return problems


def main():
    paths = sys.argv[1:] or tracked_files()
    problems = []
    for path in paths:
        if path.startswith(SKIP_PREFIXES):
            continue
        problems.extend(check(path))
    for path, number, what, line in problems:
        print(f"{path}:{number}: {what}: {line[:160]}")
    if problems:
        print(f"{len(problems)} private references; use a placeholder (docs/private-references.md)",
              file=sys.stderr)
        return 1
    print("no private references")
    return 0


if __name__ == "__main__":
    sys.exit(main())
