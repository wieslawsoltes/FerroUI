#!/usr/bin/env python3
"""Lists the upstream unit tests that the port does not have.

The tracking pages (port_status.py) count members. This script counts tests: the rule of the port is that
every upstream test is ported with the code it tests, under upstream's name in snake case.

Usage:

    python3 scripts/port-status/test_gaps.py UPSTREAM [PROJECT] [options]

    UPSTREAM   path of the upstream checkout (Avalonia); it is only read
    PROJECT    upstream test project, relative to UPSTREAM (default: tests/Avalonia.Base.UnitTests)

    --rev REV        read the upstream files of commit REV (`git archive`, nothing is checked out) instead of
                     the working tree; `--rev tracked` takes the commit named in docs/porting/TRACKING.md
    --rust DIR       directory of the port whose tests are scanned (default: src, relative to the repository);
                     may be given more than once
    --aliases FILE   the alias and waiver file (default: docs/porting/data/test-aliases.toml)
    --locate         also print, per upstream file, the Rust files that hold its tests
    --any-fn         match against every Rust function, not only test functions (the rule of the count of
                     2026-10-08 in docs/porting/CONTINUATION.md)
    --output FILE    write the report to FILE instead of standard output

    python3 scripts/port-status/test_gaps.py ../Avalonia tests/Avalonia.Base.UnitTests --rev tracked \
        --output docs/porting/data/test-gaps-base.txt

What is counted:

- An upstream test is a method that carries an attribute whose name ends in `Fact` or `Theory` (`[Fact]`,
  `[Theory]`, `[CrossFact]`, `[Fact(Skip = "..")]`). A theory is one test whatever the number of its
  `[InlineData]` rows. Comments are removed first, so a test that is commented out is not counted. Preprocessor
  conditions are not evaluated. A method of an abstract or generic base class counts once, in its file.
- A Rust test is a function that carries an attribute whose last path segment is `test` (`#[test]`), with any
  other attributes between it and the `fn`, or any function of test code: a file named `*_tests*.rs`, `tests.rs`
  or `test_*.rs`, a file under a `tests` directory, or what follows the first `#[cfg(test)]` of another file.
  The second form is there because the port writes an upstream theory as a function with the upstream name and
  the parameters of the theory, called once per row by generated `#[test]` functions (`theory!`), and the tests
  of the binding suites inside a macro that runs each for the reflection and the compiled form
  (`binding_tests!`). A helper function of test code therefore also counts as a name.
- Names are compared after normalisation: lower case, underscores removed, `avalonia` read as `ferro`,
  `avn` at the start of a word as `frn` and `avares` (the resource scheme) as `ferres`
  (docs/porting/PORTING-GUIDE.md, rule 2). `Foo_Bar` and `FooBar` are the same name, and so are
  `AvaloniaObject_Works` and `ferro_object_works`.
- The match is by name only: an upstream test is present when a Rust test of that name exists anywhere in the
  scanned directories. Two upstream tests of the same name in different files are both satisfied by one Rust
  test. The report states how many names are ambiguous in this way (the tests of the files concerned have to be
  compared by reading).

The data file (`docs/porting/data/test-aliases.toml`) is edited by hand:

    [[alias]]              # ported under a name the rule cannot derive
    project = "Avalonia.Base.UnitTests"     # optional
    file    = "Styling/SelectorTests_Or.cs" # glob on the path relative to the project; optional
    test    = "Upstream_Method_Name"        # glob on the upstream method name
    rust    = ["rust_test_a", "rust_test_b"]   # every one must exist, otherwise the alias is reported as broken
    reason  = "one theory became two tests"

    [[waive]]              # not ported, with the reason
    project = "Avalonia.Base.UnitTests"
    file    = "SourceGenerators/*"
    test    = "*"                           # optional, default every test of the file
    reason  = "tests a C# source generator"

A waiver or alias applies only to a test that was not found by name. Waived tests are listed with their reason
and are not counted as missing.

Requirements: Python 3.11+ (standard library only), and `git` when `--rev` is used.
"""

from __future__ import annotations

import argparse
import fnmatch
import io
import os
import re
import subprocess
import sys
import tarfile
import tomllib
from collections import defaultdict

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


# ---------------------------------------------------------------------------------------------------------------
# Names


def normalise(name: str) -> str:
    """The comparison key of a test name (see the module documentation)."""
    words = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", name)
    words = re.sub(r"(?i)(^|_)avn", r"\1frn", words)
    key = words.replace("_", "").lower()
    return key.replace("avalonia", "ferro").replace("avares", "ferres")


# ---------------------------------------------------------------------------------------------------------------
# Upstream (C#)


def strip_csharp(text: str) -> str:
    """Blanks comments and the contents of string and character literals, keeping line structure."""
    out = []
    i = 0
    n = len(text)
    while i < n:
        c = text[i]
        nxt = text[i + 1] if i + 1 < n else ""
        if c == "/" and nxt == "/":
            j = text.find("\n", i)
            j = n if j < 0 else j
            i = j
        elif c == "/" and nxt == "*":
            j = text.find("*/", i + 2)
            j = n if j < 0 else j + 2
            out.append("".join(ch if ch == "\n" else " " for ch in text[i:j]))
            i = j
        elif c == '"' and text.startswith('"""', i):
            # Raw string literal: ends at the same number of quotes.
            q = 3
            while i + q < n and text[i + q] == '"':
                q += 1
            j = text.find('"' * q, i + q)
            j = n if j < 0 else j + q
            out.append('""' + "".join("\n" for ch in text[i:j] if ch == "\n"))
            i = j
        elif c == "@" and (nxt == '"' or (nxt == "$" and text[i + 2 : i + 3] == '"')):
            j = i + (2 if nxt == '"' else 3)
            while j < n:
                if text[j] == '"':
                    if text[j + 1 : j + 2] == '"':
                        j += 2
                        continue
                    break
                j += 1
            out.append('""' + "".join("\n" for ch in text[i:j] if ch == "\n"))
            i = j + 1
        elif c == '"':
            j = i + 1
            while j < n and text[j] != '"' and text[j] != "\n":
                j += 2 if text[j] == "\\" else 1
            out.append('""')
            i = j + 1
        elif c == "'":
            j = i + 1
            while j < n and text[j] != "'" and text[j] != "\n":
                j += 2 if text[j] == "\\" else 1
            out.append("' '")
            i = j + 1
        else:
            out.append(c)
            i += 1
    return "".join(out)


ATTRIBUTE_NAME = re.compile(r"(?:^|[\[,]\s*)(?:[A-Za-z_][\w.]*\.)?([A-Za-z_]\w*)\s*(?=[(\],]|$)")
METHOD = re.compile(
    r"\b(?:public|internal|protected|private)\b[^;={(]*?\b([A-Za-z_]\w*)\s*(?:<[^>()]*>)?\s*\("
)


def split_attributes(line: str) -> tuple[list[str], str]:
    """Splits leading `[..]` attribute lists from a line: (attribute list texts, the rest)."""
    lists = []
    rest = line.lstrip()
    while rest.startswith("["):
        depth = 0
        end = -1
        for k, ch in enumerate(rest):
            if ch in "[(":
                depth += 1
            elif ch in "])":
                depth -= 1
                if depth == 0:
                    end = k
                    break
        if end < 0:
            # An attribute list that continues on the next line: take the whole line.
            lists.append(rest[1:])
            return lists, ""
        lists.append(rest[1:end])
        rest = rest[end + 1 :].lstrip()
    return lists, rest


def attribute_names(attribute_list: str) -> list[str]:
    """The attribute names of one `[A, B(..)]` list (arguments are skipped)."""
    names = []
    depth = 0
    token = ""
    for ch in attribute_list + ",":
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
        elif ch == "," and depth == 0:
            head = token.strip().split("(")[0].strip()
            if ":" in head:
                head = head.split(":", 1)[1].strip()
            if head:
                names.append(head.split(".")[-1])
            token = ""
            continue
        if depth == 0 or ch == "(":
            token += ch
    return names


def is_test_attribute(name: str) -> bool:
    name = name.removesuffix("Attribute")
    return name.endswith("Fact") or name.endswith("Theory")


def upstream_tests(text: str) -> list[str]:
    """The names of the test methods of one C# file, in order."""
    tests = []
    pending = False
    for line in strip_csharp(text).split("\n"):
        lists, rest = split_attributes(line)
        for attribute_list in lists:
            if any(is_test_attribute(name) for name in attribute_names(attribute_list)):
                pending = True
        if not pending or not rest:
            continue
        match = METHOD.search(rest)
        if match:
            tests.append(match.group(1))
            pending = False
        elif rest.strip() not in ("", "#if", "#endif") and not rest.lstrip().startswith("#"):
            # A method without an access modifier, or a declaration split over lines.
            match = re.search(r"\b([A-Za-z_]\w*)\s*(?:<[^>()]*>)?\s*\(", rest)
            if match and re.match(r"\s*(?:static\s+|async\s+|unsafe\s+)*[\w<>\[\],.? ]+\s+" + re.escape(match.group(1)), rest):
                tests.append(match.group(1))
                pending = False
    return tests


def read_upstream(upstream: str, project: str, rev: str | None) -> dict[str, str]:
    """path relative to the project -> text, for every C# file of the project."""
    files = {}
    if rev:
        data = subprocess.run(
            ["git", "-C", upstream, "archive", "--format=tar", rev, project],
            check=True,
            capture_output=True,
        ).stdout
        with tarfile.open(fileobj=io.BytesIO(data)) as tar:
            for member in tar:
                if member.isfile() and member.name.endswith(".cs"):
                    handle = tar.extractfile(member)
                    if handle is not None:
                        files[os.path.relpath(member.name, project)] = handle.read().decode("utf-8-sig", "replace")
    else:
        root = os.path.join(upstream, project)
        for directory, directories, names in os.walk(root):
            directories[:] = [d for d in directories if d not in ("bin", "obj")]
            for name in names:
                if name.endswith(".cs"):
                    path = os.path.join(directory, name)
                    with open(path, encoding="utf-8-sig", errors="replace") as handle:
                        files[os.path.relpath(path, root)] = handle.read()
    return {path.replace(os.sep, "/"): text for path, text in files.items()}


def tracked_commit() -> str:
    with open(os.path.join(REPO, "docs", "porting", "TRACKING.md"), encoding="utf-8") as handle:
        match = re.search(r"upstream commit `([0-9a-f]{40})`", handle.read())
    if not match:
        sys.exit("docs/porting/TRACKING.md does not name the upstream commit")
    return match.group(1)


# ---------------------------------------------------------------------------------------------------------------
# The port (Rust)


def strip_rust(text: str) -> str:
    """Blanks comments and string literals of Rust source, keeping line structure."""
    out = []
    i = 0
    n = len(text)
    while i < n:
        c = text[i]
        nxt = text[i + 1] if i + 1 < n else ""
        if c == "/" and nxt == "/":
            j = text.find("\n", i)
            i = n if j < 0 else j
        elif c == "/" and nxt == "*":
            depth = 1
            j = i + 2
            while j < n and depth:
                if text.startswith("/*", j):
                    depth += 1
                    j += 2
                elif text.startswith("*/", j):
                    depth -= 1
                    j += 2
                else:
                    j += 1
            out.append("".join(ch if ch == "\n" else " " for ch in text[i:j]))
            i = j
        elif c == "r" and re.match(r'r#*"', text[i : i + 12]) and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")):
            hashes = len(re.match(r"r(#*)", text[i:]).group(1))
            close = '"' + "#" * hashes
            j = text.find(close, i + 2 + hashes)
            j = n if j < 0 else j + len(close)
            out.append('""' + "".join("\n" for ch in text[i:j] if ch == "\n"))
            i = j
        elif c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            out.append('""' + "".join("\n" for ch in text[i:j] if ch == "\n"))
            i = j + 1
        elif c == "'":
            # A character literal ('a', '\n', '\u{1F}') or a lifetime.
            match = re.match(r"'(?:\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.)|[^\\'\n])'", text[i:])
            if match:
                out.append("' '")
                i += match.end()
            else:
                out.append(c)
                i += 1
        else:
            out.append(c)
            i += 1
    return "".join(out)


RUST_ITEM = re.compile(
    r"#\s*\[\s*([^\]\n]*?)\s*\]"  # an attribute
    r"|\b(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?(?:extern\s+\"\"\s+)?fn\s+([A-Za-z_]\w*)"
)


def rust_functions(text: str, test_file: bool) -> list[tuple[str, bool]]:
    """(function name, is a test) for every function of one Rust file."""
    functions = []
    stripped = strip_rust(text)
    test_code = re.search(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", stripped)
    test_code_start = 0 if test_file else (test_code.start() if test_code else len(stripped))
    is_test = False
    last_end = 0
    for match in RUST_ITEM.finditer(stripped):
        between = stripped[last_end : match.start()]
        if between.strip():
            # Something other than attributes since the last attribute: it did not belong to this item.
            is_test = False
        if match.group(2) is None:
            head = re.split(r"[\s(=]", match.group(1), maxsplit=1)[0]
            if head.split("::")[-1] == "test":
                is_test = True
        else:
            functions.append((match.group(2), is_test or match.start() >= test_code_start))
            is_test = False
        last_end = match.end()
    return functions


def is_test_file(path: str) -> bool:
    name = os.path.basename(path)
    parts = path.replace(os.sep, "/").split("/")
    return "_tests" in name or name == "tests.rs" or name.startswith("test_") or "tests" in parts[:-1]


def read_port(directories: list[str]) -> dict[str, list[tuple[str, bool]]]:
    """path relative to the repository -> functions, for every Rust file under the directories."""
    result = {}
    for directory in directories:
        root = directory if os.path.isabs(directory) else os.path.join(REPO, directory)
        for current, subdirectories, names in os.walk(root):
            subdirectories[:] = sorted(d for d in subdirectories if d not in ("target", ".git", "node_modules"))
            for name in sorted(names):
                if name.endswith(".rs"):
                    path = os.path.join(current, name)
                    with open(path, encoding="utf-8", errors="replace") as handle:
                        relative = os.path.relpath(path, REPO).replace(os.sep, "/")
                        functions = rust_functions(handle.read(), is_test_file(relative))
                    if functions:
                        result[relative] = functions
    return result


# ---------------------------------------------------------------------------------------------------------------
# Aliases and waivers


def load_aliases(path: str) -> tuple[list[dict], list[dict]]:
    if not os.path.exists(path):
        return [], []
    with open(path, "rb") as handle:
        data = tomllib.load(handle)
    return data.get("alias", []), data.get("waive", [])


def rule_applies(rule: dict, project: str, file: str, test: str) -> bool:
    if "project" in rule and rule["project"] != project:
        return False
    if "file" in rule and not fnmatch.fnmatchcase(file, rule["file"]):
        return False
    return fnmatch.fnmatchcase(test, rule.get("test", "*"))


# ---------------------------------------------------------------------------------------------------------------
# Report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("upstream")
    parser.add_argument("project", nargs="?", default="tests/Avalonia.Base.UnitTests")
    parser.add_argument("--rev")
    parser.add_argument("--rust", action="append")
    parser.add_argument("--aliases", default=os.path.join(REPO, "docs", "porting", "data", "test-aliases.toml"))
    parser.add_argument("--locate", action="store_true")
    parser.add_argument("--any-fn", action="store_true")
    parser.add_argument("--output")
    arguments = parser.parse_args()

    project = arguments.project.strip("/")
    project_name = project.split("/")[-1]
    rev = tracked_commit() if arguments.rev == "tracked" else arguments.rev
    upstream = read_upstream(arguments.upstream, project, rev)
    port = read_port(arguments.rust or ["src"])
    aliases, waivers = load_aliases(arguments.aliases)

    # Normalised name -> Rust files that hold a test of that name.
    rust_by_key: dict[str, list[str]] = defaultdict(list)
    rust_names: dict[str, list[str]] = defaultdict(list)
    for path, functions in port.items():
        for name, is_test in functions:
            if is_test or arguments.any_fn:
                rust_by_key[normalise(name)].append(path)
                rust_names[name].append(path)

    per_file = {}
    for path in sorted(upstream):
        tests = upstream_tests(upstream[path])
        if tests:
            per_file[path] = tests

    # Upstream names that occur in more than one file (or twice in a file).
    occurrences: dict[str, int] = defaultdict(int)
    for tests in per_file.values():
        for test in tests:
            occurrences[normalise(test)] += 1

    lines = []
    total = present = aliased = waived = missing = ambiguous = 0
    files_with_missing = 0
    by_area: dict[str, list[int]] = defaultdict(lambda: [0, 0, 0, 0])
    broken_aliases = []
    waived_lines = []
    aliased_lines = []
    file_lines = []
    located = []

    for path, tests in per_file.items():
        area = path.split("/")[0] if "/" in path else "(root)"
        file_missing = []
        homes: dict[str, int] = defaultdict(int)
        for test in tests:
            total += 1
            by_area[area][0] += 1
            key = normalise(test)
            if key in rust_by_key:
                present += 1
                if occurrences[key] > 1:
                    ambiguous += 1
                for home in set(rust_by_key[key]):
                    homes[home] += 1
                continue
            alias = next((rule for rule in aliases if rule_applies(rule, project_name, path, test)), None)
            if alias is not None:
                targets = alias.get("rust", [])
                absent = [target for target in targets if target not in rust_names]
                if targets and not absent:
                    aliased += 1
                    by_area[area][1] += 1
                    aliased_lines.append(f"  {path}: {test} -> {', '.join(targets)} ({alias.get('reason', 'no reason given')})")
                    for target in targets:
                        for home in set(rust_names[target]):
                            homes[home] += 1
                    continue
                broken_aliases.append(f"  {path}: {test} -> {', '.join(absent) or '(no `rust` names)'}")
            waiver = next((rule for rule in waivers if rule_applies(rule, project_name, path, test)), None)
            if waiver is not None:
                waived += 1
                by_area[area][2] += 1
                waived_lines.append((waiver.get("reason", "no reason given"), path, test))
                continue
            missing += 1
            by_area[area][3] += 1
            file_missing.append(test)
        if file_missing:
            files_with_missing += 1
            file_lines.append(f"{path}: {len(file_missing)}/{len(tests)}")
            file_lines.extend(f"  {test}" for test in file_missing)
        if arguments.locate and homes:
            located.append(f"{path} ({len(tests)})")
            located.extend(f"  {count:4d}  {home}" for home, count in sorted(homes.items(), key=lambda item: (-item[1], item[0])))

    lines.append(f"Upstream tests of {project} that the port does not have")
    lines.append(f"Upstream commit: {rev or 'working tree'}")
    lines.append(f"Port: test functions under {', '.join(arguments.rust or ['src'])}" + (" (every function)" if arguments.any_fn else ""))
    lines.append("Generated by scripts/port-status/test_gaps.py; the rules are at the top of the script.")
    lines.append("")
    lines.append(f"Tests: {total} in {len(per_file)} files")
    lines.append(f"Present by name: {present}")
    lines.append(f"Present under another name (aliases): {aliased}")
    lines.append(f"Waived: {waived}")
    lines.append(f"Missing: {missing} in {files_with_missing} files")
    lines.append(f"Present by a name that more than one upstream test of the project has: {ambiguous}")
    lines.append("")
    lines.append("By directory (tests, aliased, waived, missing):")
    for area in sorted(by_area):
        counts = by_area[area]
        lines.append(f"  {area}: {counts[0]}, {counts[1]}, {counts[2]}, {counts[3]}")
    lines.append("")
    lines.append("Missing (file: missing/tests):")
    lines.append("")
    lines.extend(file_lines or ["  none"])
    if broken_aliases:
        lines.append("")
        lines.append("Broken aliases (a Rust test they name does not exist):")
        lines.extend(broken_aliases)
    if aliased_lines:
        lines.append("")
        lines.append("Present under another name:")
        lines.extend(aliased_lines)
    if waived_lines:
        lines.append("")
        lines.append("Waived:")
        by_reason: dict[str, list[tuple[str, str]]] = defaultdict(list)
        for reason, path, test in waived_lines:
            by_reason[reason].append((path, test))
        for reason, items in by_reason.items():
            lines.append(f"  {reason} ({len(items)}):")
            lines.extend(f"    {path}: {test}" for path, test in items)
    if located:
        lines.append("")
        lines.append("Where the tests of each upstream file are in the port (tests found, Rust file):")
        lines.extend(located)

    report = "\n".join(lines) + "\n"
    if arguments.output:
        with open(arguments.output, "w", encoding="utf-8") as handle:
            handle.write(report)
        print(f"{project}: {total} tests, {missing} missing in {files_with_missing} files, {aliased} aliased, {waived} waived")
    else:
        sys.stdout.write(report)
    return 1 if broken_aliases else 0


if __name__ == "__main__":
    sys.exit(main())
