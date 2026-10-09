#!/usr/bin/env python3
"""Lists the upstream unit tests of one upstream test project that the port does not have.

    python3 scripts/port-status/test_gaps_controls.py <upstream checkout> <test project> [options]

    python3 scripts/port-status/test_gaps_controls.py ../Avalonia tests/Avalonia.Controls.UnitTests \
        --commit 17350180c33b063f0e98abbfd19aa3cae63f5d56 \
        --aliases docs/porting/data/test-aliases-controls.toml \
        > docs/porting/data/test-gaps-controls.txt

Arguments

    <upstream checkout>   path of the Avalonia checkout (only read)
    <test project>        the test project, as a path below the checkout (`tests/Avalonia.Controls.UnitTests`)
                          or as its name (`Avalonia.Controls.UnitTests`, looked up under `tests/`)

Options

    --commit <rev>        read the project at this revision of the checkout (through `git archive`, the working
                          tree is not touched) instead of the files on disk. The port tracks the commit named at
                          the top of docs/porting/TRACKING.md, and the checkout is usually ahead of it.
    --root <dir>          a directory of the port to search for Rust tests; may be repeated. Default: `src` and
                          `tests` of the repository this script is in.
    --aliases <file>      a TOML file with `[[alias]]` and `[[waive]]` tables (see below); may be repeated.
                          Default: docs/porting/data/test-aliases-controls.toml when it exists.
    --summary             print only the line of totals, tab separated: project, tests, missing, files with
                          missing tests, waived, aliased.

What is counted

    Upstream: every method of a `.cs` file of the project that carries an attribute whose name ends in `Fact`,
    `Theory`, `Test` or `TestCase` (`[Fact]`, `[Theory]`, `[AvaloniaFact]`, `[Fact(Skip = "...")]`, the NUnit
    `[Test]`, ...). A `[Theory]` is one test, however many `[InlineData]` rows it has. Comments, strings and
    preprocessor lines are blanked before the search; code in an inactive `#if` branch is therefore counted too.
    A method that two classes of one file declare is one test per class, listed as `Class.Method` (the class is
    the one declared last before the method); overloads within a class are one test.

    Port: every `fn` that follows a `#[test]` attribute (other attributes may stand between them) in a `.rs`
    file below the roots, and every other `fn` of a file that has at least one such test: the port writes a
    `[Theory]`, and a test that upstream runs once per subclass of its test class, as a function with upstream's
    name that takes the parameters, called by `#[test]` functions named after the rows or written by a macro.
    The identifiers handed to a macro of the file whose body has a `#[test]` count as well
    (`tests! { name_a, name_b }`). Comments and string literals are blanked first.

    Match: by name alone. Both names are lower-cased and their underscores removed; in the upstream name
    `Avalonia` reads as `Ferro` and a word-initial `Avn` as `Frn` (the renames of docs/porting/PORTING-GUIDE.md).
    A name found in any Rust file of the roots counts as present, wherever the file is. A name that n tests of
    the project have (in several classes or files) needs n functions of the port: the n-th of them, in the order
    of the paths, is present when the port has n functions of the name, so the file reported for such a test may
    not be the one whose test is missing. A test that the port has under a different name counts as missing until
    it is renamed or given an alias. The same name in another upstream project is not accounted for: a test of
    the base library and one of the controls with one name are both satisfied by one function.

The alias file

    [[alias]]                       # the port has the test under a name the rule cannot derive
    project = "Avalonia.Controls.UnitTests"
    file = "GridTests.cs"           # path below the project, with forward slashes
    test = "Upstream_Method_Name"   # or "Class.Upstream_Method_Name" in a file with several test classes
    rust = "the_rust_test_function" # must exist as a test of the port, or the alias is reported as stale
    reason = "why the name differs"

    [[waive]]                       # the test cannot be ported or does not apply
    project = "Avalonia.Controls.UnitTests"
    file = "DesignTests.cs"
    test = "Upstream_Method_Name"   # or: tests = ["A", "B"]; or neither, which waives the whole file
    reason = "what is missing or why it does not apply"

    Entries of other projects are ignored. An entry that names no upstream test, an alias whose Rust test does
    not exist and a waiver of a test that the port has are listed at the end of the output as stale.

Output

    The totals, then one line per upstream file that has tests which are neither present, aliased nor waived
    (`missing/total  file`) followed by the missing names, then the waived tests with their reasons, the aliases
    and the stale entries. Files are sorted by the number of missing tests, then by path, so the output is
    stable and can be committed and compared.

Standard library only (Python 3.11 or later, for `tomllib`). To be merged with `test_gaps.py`, the count of the
base library, which is written separately.
"""

from __future__ import annotations

import argparse
import io
import os
import re
import subprocess
import sys
import tarfile

try:
    import tomllib
except ModuleNotFoundError:
    sys.exit("test_gaps_controls.py needs Python 3.11 or later (tomllib)")
from collections import defaultdict

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
DEFAULT_ALIASES = os.path.join(REPO, "docs", "porting", "data", "test-aliases-controls.toml")

# --------------------------------------------------------------------------------------------------------------
# Upstream side
# --------------------------------------------------------------------------------------------------------------


def blank_csharp(text: str) -> str:
    """Replaces comments, string and character literals and preprocessor lines with spaces, keeping newlines."""
    out = []
    i = 0
    n = len(text)
    line_start = True

    def blank(segment: str) -> str:
        return "".join("\n" if c == "\n" else " " for c in segment)

    while i < n:
        c = text[i]
        if line_start and c in " \t":
            out.append(c)
            i += 1
            continue
        if line_start and c == "#":
            j = text.find("\n", i)
            j = n if j < 0 else j
            out.append(blank(text[i:j]))
            i = j
            continue
        if c == "\n":
            out.append(c)
            i += 1
            line_start = True
            continue
        line_start = False
        two = text[i:i + 2]
        if two == "//":
            j = text.find("\n", i)
            j = n if j < 0 else j
            out.append(blank(text[i:j]))
            i = j
            continue
        if two == "/*":
            j = text.find("*/", i + 2)
            j = n if j < 0 else j + 2
            out.append(blank(text[i:j]))
            i = j
            continue
        # Raw string literals: three or more quotes, optionally preceded by `$`.
        m = re.compile(r'\$*("{3,})').match(text, i)
        if m:
            quotes = m.group(1)
            j = text.find(quotes, m.end())
            j = n if j < 0 else j + len(quotes)
            while j < n and text[j] == '"':
                j += 1
            out.append(blank(text[i:j]))
            i = j
            continue
        # Verbatim strings: @"..." with "" as the escape, also $@"..." and @$"...".
        m = re.compile(r'(?:\$@|@\$|@)"').match(text, i)
        if m:
            j = m.end()
            while j < n:
                if text[j] == '"':
                    if text[j + 1:j + 2] == '"':
                        j += 2
                        continue
                    j += 1
                    break
                j += 1
            out.append(blank(text[i:j]))
            i = j
            continue
        if c == '"' or two == '$"':
            j = i + (2 if c == "$" else 1)
            while j < n and text[j] != '"' and text[j] != "\n":
                j += 2 if text[j] == "\\" else 1
            j = min(j + 1, n)
            out.append(blank(text[i:j]))
            i = j
            continue
        if c == "'":
            m = re.compile(r"'(?:\\.[^']*|[^'\\\n])'").match(text, i)
            if m:
                out.append(blank(m.group(0)))
                i = m.end()
                continue
        out.append(c)
        i += 1
    return "".join(out)


TEST_ATTRIBUTE = re.compile(r"\[\s*(?:[A-Za-z_][\w.]*\.)?\w*(?:Fact|Theory|Test|TestCase)(?:Attribute)?\s*[\](,]")
# The declaration that follows the attributes: modifiers, a return type, the name, `(` or `<`.
METHOD = re.compile(
    r"(?:\b(?:public|internal|private|protected|static|async|virtual|override|sealed|unsafe|new)\s+)*"
    r"[A-Za-z_][\w.<>\[\],?\s]*?\s+([A-Za-z_]\w*)\s*(?:<[^>()]*>)?\s*\("
)


def skip_attribute(text: str, i: int) -> int:
    """`i` is at `[`; returns the index after the matching `]`."""
    depth = 0
    n = len(text)
    while i < n:
        c = text[i]
        if c == "[":
            depth += 1
        elif c == "]":
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    return n


CLASS = re.compile(r"\b(?:class|struct|record)\s+([A-Za-z_]\w*)")


def upstream_tests(text: str) -> list[tuple[str, str]]:
    """The test methods of one C# file as (class, method), in order, each pair once. The class is the one
    declared last before the method."""
    text = blank_csharp(text)
    classes = [(m.start(), m.group(1)) for m in CLASS.finditer(text)]
    names: list[tuple[str, str]] = []
    seen = set()
    position = 0
    while True:
        m = TEST_ATTRIBUTE.search(text, position)
        if not m:
            break
        i = skip_attribute(text, m.start())
        # Further attribute lists ([InlineData(...)], [MemberData(...)]) up to the declaration.
        while True:
            j = i
            while j < len(text) and text[j].isspace():
                j += 1
            if j < len(text) and text[j] == "[":
                i = skip_attribute(text, j)
                continue
            i = j
            break
        d = METHOD.match(text, i)
        position = i if not d else d.end()
        if not d:
            position = max(position, m.end())
            continue
        owner = ""
        for start, name in classes:
            if start > m.start():
                break
            owner = name
        pair = (owner, d.group(1))
        if pair not in seen:
            seen.add(pair)
            names.append(pair)
    return names


def read_project(upstream: str, project: str, commit: str | None) -> dict[str, str]:
    """Path below the project (forward slashes) -> text, for every `.cs` file of the project."""
    files: dict[str, str] = {}
    if commit:
        result = subprocess.run(
            ["git", "-C", upstream, "archive", "--format=tar", commit, project],
            capture_output=True,
        )
        if result.returncode != 0:
            sys.exit(f"cannot read {project} at {commit}: {result.stderr.decode(errors='replace').strip()}")
        data = result.stdout
        with tarfile.open(fileobj=io.BytesIO(data)) as tar:
            for member in tar:
                if member.isfile() and member.name.endswith(".cs"):
                    relative = member.name[len(project.rstrip("/")) + 1:]
                    files[relative] = tar.extractfile(member).read().decode("utf-8-sig", "replace")
        return files
    base = os.path.join(upstream, project)
    if not os.path.isdir(base):
        sys.exit(f"no such test project: {base}")
    for directory, directories, names in os.walk(base):
        directories[:] = sorted(d for d in directories if d not in ("bin", "obj"))
        for name in sorted(names):
            if name.endswith(".cs"):
                path = os.path.join(directory, name)
                with open(path, encoding="utf-8-sig", errors="replace") as handle:
                    files[os.path.relpath(path, base).replace(os.sep, "/")] = handle.read()
    return files


# --------------------------------------------------------------------------------------------------------------
# Port side
# --------------------------------------------------------------------------------------------------------------


def blank_rust(text: str) -> str:
    """Replaces comments and string literals with spaces, keeping newlines."""
    out = []
    i = 0
    n = len(text)

    def blank(segment: str) -> str:
        return "".join("\n" if c == "\n" else " " for c in segment)

    raw = re.compile(r'b?r(#*)"')
    while i < n:
        c = text[i]
        two = text[i:i + 2]
        if two == "//":
            j = text.find("\n", i)
            j = n if j < 0 else j
            out.append(blank(text[i:j]))
            i = j
            continue
        if two == "/*":
            depth = 1
            j = i + 2
            while j < n and depth:
                if text[j:j + 2] == "/*":
                    depth += 1
                    j += 2
                elif text[j:j + 2] == "*/":
                    depth -= 1
                    j += 2
                else:
                    j += 1
            out.append(blank(text[i:j]))
            i = j
            continue
        if c in "rb" and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")):
            m = raw.match(text, i)
            if m:
                end = '"' + m.group(1)
                j = text.find(end, m.end())
                j = n if j < 0 else j + len(end)
                out.append(blank(text[i:j]))
                i = j
                continue
        if c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            j = min(j + 1, n)
            out.append(blank(text[i:j]))
            i = j
            continue
        if c == "'":
            # A character literal ('a', '\n', '\u{1F600}'); a lifetime has no closing quote.
            m = re.compile(r"'(?:\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.)|[^'\\\n])'").match(text, i)
            if m:
                out.append(blank(m.group(0)))
                i = m.end()
                continue
        out.append(c)
        i += 1
    return "".join(out)


RUST_TEST = re.compile(
    r"#\s*\[\s*(?:[\w:]+::)?test\s*\]"          # #[test]
    r"(?:\s*#\s*\[[^\]]*\])*"                    # #[ignore = ...], #[should_panic(...)], #[cfg(...)]
    r"\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_]\w*)"
)


RUST_TEST_ATTRIBUTE = re.compile(r"#\s*\[\s*(?:[\w:]+::)?test\s*\]")
RUST_FN = re.compile(r"\bfn\s+([A-Za-z_]\w*)")


MACRO_RULES = re.compile(r"\bmacro_rules\s*!\s*([A-Za-z_]\w*)\s*[{(\[]")
IDENTIFIER = re.compile(r"(?<![\w$])[a-z_][a-z0-9_]*\b(?!\s*[!(])")


def balanced_end(text: str, i: int) -> int:
    """`i` is at an opening bracket; returns the index after the bracket that closes it."""
    depth = 0
    for j in range(i, len(text)):
        if text[j] in "{([":
            depth += 1
        elif text[j] in "})]":
            depth -= 1
            if depth == 0:
                return j + 1
    return len(text)


def macro_test_names(blanked: str) -> list[str]:
    """The identifiers handed to the macros of the file that write tests: `tests! { name_a, name_b, }` where
    `macro_rules! tests` has a `#[test]` in its body. The names of the tests are among them."""
    names: list[str] = []
    for m in MACRO_RULES.finditer(blanked):
        body_end = balanced_end(blanked, m.end() - 1)
        if not RUST_TEST_ATTRIBUTE.search(blanked, m.end(), body_end):
            continue
        for call in re.finditer(r"\b" + re.escape(m.group(1)) + r"\s*!\s*[{(\[]", blanked):
            if blanked[:call.start()].rstrip().endswith("macro_rules"):
                continue
            end = balanced_end(blanked, call.end() - 1)
            names.extend(IDENTIFIER.findall(blanked, call.end(), end))
    return names


def rust_tests(roots: list[str]) -> dict[str, list[tuple[str, str]]]:
    """Normalised name -> [(function name, path relative to the repository)] of every Rust test and of every
    other function of a file with tests. The `#[test]` functions alone are under the key ""."""
    found: dict[str, list[tuple[str, str]]] = defaultdict(list)
    for root in roots:
        for directory, directories, names in os.walk(root):
            directories[:] = sorted(d for d in directories if d not in ("target", ".git", "node_modules"))
            for name in sorted(names):
                if not name.endswith(".rs"):
                    continue
                path = os.path.join(directory, name)
                with open(path, encoding="utf-8", errors="replace") as handle:
                    text = handle.read()
                if "test" not in text:
                    continue
                # String literals are blanked with spaces, which leaves `#[ignore = "..."]` as an attribute
                # without brackets inside it, as the expression above expects.
                blanked = blank_rust(text)
                tests = [m.group(1) for m in RUST_TEST.finditer(blanked)]
                # A file whose tests are all written by a macro (`#[test] fn $name()`) is a file with tests.
                if not tests and not RUST_TEST_ATTRIBUTE.search(blanked):
                    continue
                relative = os.path.relpath(path, REPO)
                found[""].extend((name, relative) for name in tests)
                functions = RUST_FN.findall(blanked)
                declared = set(functions)
                functions += [name for name in dict.fromkeys(macro_test_names(blanked)) if name not in declared]
                for name in functions:
                    found[normalise_rust(name)].append((name, relative))
    return found


# --------------------------------------------------------------------------------------------------------------
# Matching
# --------------------------------------------------------------------------------------------------------------


def normalise_upstream(name: str) -> str:
    name = re.sub(r"(?<![a-z])Avn(?=[A-Z_]|$)", "Frn", name)
    return name.lower().replace("avalonia", "ferro").replace("_", "")


def normalise_rust(name: str) -> str:
    return name.lower().replace("_", "")


def load_aliases(paths: list[str], project_name: str):
    aliases: dict[tuple[str, str], dict] = {}
    waivers: dict[tuple[str, str], dict] = {}
    file_waivers: dict[str, dict] = {}
    for path in paths:
        with open(path, "rb") as handle:
            data = tomllib.load(handle)
        for entry in data.get("alias", []):
            if entry.get("project") == project_name:
                aliases[(entry["file"], entry["test"])] = entry
        for entry in data.get("waive", []):
            if entry.get("project") != project_name:
                continue
            tests = entry.get("tests", [])
            if "test" in entry:
                tests = [entry["test"], *tests]
            if not tests:
                file_waivers[entry["file"]] = entry
            for test in tests:
                waivers[(entry["file"], test)] = entry
    return aliases, waivers, file_waivers


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("upstream")
    parser.add_argument("project")
    parser.add_argument("--commit")
    parser.add_argument("--root", action="append")
    parser.add_argument("--aliases", action="append")
    parser.add_argument("--summary", action="store_true")
    arguments = parser.parse_args()

    project = arguments.project.strip("/")
    if "/" not in project:
        project = "tests/" + project
    project_name = project.rsplit("/", 1)[1]
    roots = arguments.root or [os.path.join(REPO, "src"), os.path.join(REPO, "tests")]
    alias_paths = arguments.aliases
    if alias_paths is None:
        alias_paths = [DEFAULT_ALIASES] if os.path.exists(DEFAULT_ALIASES) else []

    sources = read_project(arguments.upstream, project, arguments.commit)
    port = rust_tests(roots)
    aliases, waivers, file_waivers = load_aliases(alias_paths, project_name)

    total = 0
    files_with_tests = 0
    missing: dict[str, list[str]] = {}
    counts: dict[str, int] = {}
    waived: list[tuple[str, str, str]] = []
    aliased: list[tuple[str, str, str, str]] = []
    stale: list[str] = []
    used_aliases = set()
    used_waivers = set()
    used_file_waivers = set()

    occurrence: dict[str, int] = defaultdict(int)
    for path in sorted(sources):
        pairs = upstream_tests(sources[path])
        if not pairs:
            continue
        files_with_tests += 1
        counts[path] = len(pairs)
        total += len(pairs)
        several = len({owner for owner, _ in pairs}) > 1
        for owner, method in pairs:
            # A method that several classes or files of the project declare is one test in each: the n-th of
            # them, in the order of the paths, is present when the port has n functions of the name.
            occurrence[method] += 1
            present = len(port.get(normalise_upstream(method), ())) >= occurrence[method]
            name = f"{owner}.{method}" if several else method
            key = (path, name)
            if key not in aliases and key not in waivers:
                key = (path, method)
            if key in aliases:
                used_aliases.add(key)
                entry = aliases[key]
                if normalise_rust(entry["rust"]) in port:
                    aliased.append((path, name, entry["rust"], entry.get("reason", "")))
                    continue
                stale.append(f"alias {path}: {name} -> {entry['rust']}: the port has no test of that name")
            if key in waivers:
                used_waivers.add(key)
                if present:
                    stale.append(f"waiver {path}: {name}: the port has the test")
                    continue
                waived.append((path, name, waivers[key].get("reason", "")))
                continue
            if present:
                continue
            if path in file_waivers:
                used_file_waivers.add(path)
                waived.append((path, name, file_waivers[path].get("reason", "")))
                continue
            missing.setdefault(path, []).append(name)

    for key in sorted(set(aliases) - used_aliases):
        stale.append(f"alias {key[0]}: {key[1]}: no such upstream test")
    for key in sorted(set(waivers) - used_waivers):
        stale.append(f"waiver {key[0]}: {key[1]}: no such upstream test")
    for path in sorted(set(file_waivers) - used_file_waivers):
        stale.append(f"waiver {path}: no test of the file is missing, or no such file")

    missing_count = sum(len(names) for names in missing.values())
    if arguments.summary:
        print("\t".join(str(v) for v in (project_name, total, missing_count, len(missing), len(waived), len(aliased))))
        return 0

    print(f"Test gaps: {project_name}")
    print(f"Upstream: {project}" + (f" at {arguments.commit}" if arguments.commit else " (working tree)"))
    print("Port: " + ", ".join(os.path.relpath(root, REPO) for root in roots)
          + f" ({len(port[''])} Rust tests)")
    print("Generated by scripts/port-status/test_gaps_controls.py; the rules of the match are at its top.")
    print()
    print(f"Tests: {total} in {files_with_tests} files")
    print(f"Present: {total - missing_count - len(waived)} ({len(aliased)} of them through an alias)")
    print(f"Waived: {len(waived)}")
    print(f"Missing: {missing_count} in {len(missing)} files")

    if missing:
        print()
        print("Missing (missing/tests of the file)")
        for path in sorted(missing, key=lambda p: (-len(missing[p]), p)):
            print()
            print(f"{len(missing[path])}/{counts[path]}  {path}")
            for name in missing[path]:
                print(f"    {name}")
    if waived:
        print()
        print("Waived")
        by_file: dict[str, list[tuple[str, str]]] = defaultdict(list)
        for path, name, reason in waived:
            by_file[path].append((name, reason))
        for path in sorted(by_file):
            print()
            print(f"{len(by_file[path])}/{counts[path]}  {path}")
            last = None
            for name, reason in by_file[path]:
                if reason != last:
                    print(f"  reason: {reason}")
                    last = reason
                print(f"    {name}")
    if aliased:
        print()
        print("Aliases (upstream test -> test of the port)")
        last = None
        for path, name, rust, reason in aliased:
            if path != last:
                print()
                print(path)
                last = path
            print(f"    {name} -> {rust}: {reason}")
    if stale:
        print()
        print("Stale entries of the alias file")
        for line in stale:
            print(f"    {line}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
