#!/usr/bin/env python3
"""Lists the upstream unit tests of an upstream test project that the port does not have.

The tracking pages (port_status.py) count members. This script counts tests: the rule of the port is that every
upstream test is ported with the code it tests, under upstream's name in snake case.

    python3 scripts/port-status/test_gaps.py <upstream checkout> [<test project>] [options]

    python3 scripts/port-status/test_gaps.py ../Avalonia --all
    python3 scripts/port-status/test_gaps.py ../Avalonia tests/Avalonia.Controls.UnitTests
    python3 scripts/port-status/test_gaps.py ../Avalonia Avalonia.Base.UnitTests --output docs/porting/data/test-gaps-base.txt
    python3 scripts/port-status/test_gaps.py --reference reactive --all     # another reference of references.toml

Arguments

    <upstream checkout>   path of the Avalonia checkout (only read)
    <test project>        the test project, as a path below the checkout (`tests/Avalonia.Controls.UnitTests`)
                          or as its name (`Avalonia.Controls.UnitTests`, looked up under `tests/`)

Options

    --rev <rev>           the revision of the checkout whose files are read (through `git archive`: nothing is
                          checked out, the working tree is not touched). Default: `tracked`, the commit named at
                          the top of docs/porting/TRACKING.md, which the port follows; the checkout is usually
                          ahead of it. `--rev worktree` reads the files on disk.
    --rust <dir>          a directory of the port to search for Rust tests; may be repeated. Default: `src` and
                          `tests` of the repository this script is in.
    --aliases <file>      the alias and waiver file. Default: docs/porting/data/test-aliases.toml.
    --output <file>       write the report to the file and print the line of totals.
    --summary             print only the line of totals, tab separated: project, tests, missing, files with
                          missing tests, waived, aliased.
    --all                 every project of the table PROJECTS below: writes
                          docs/porting/data/test-gaps-<short name>.txt for each and prints the table of totals
                          in Markdown.
    --reference <name>    a reference of scripts/port-status/references.toml other than the framework: its checkout
                          (when <upstream checkout> is not given), its tracked commit (the `upstreamCommit` of its
                          upstream-api.json), its test projects (`test_projects`, for --all), its data directory
                          (the alias file and the outputs), the directories of its Rust tests (`test_roots`), the
                          attributes of its test methods (`test_attributes`) and its stated reasons (below).

Stated reasons (a reference with `stated_reasons = true`)

    A line `// Not ported: <Name> - <reason>` (or `// Not ported (in part): <Name>, <Name> - <reason>`) of the Rust
    test file of an upstream test file states why that upstream test is not ported. The Rust test file of
    `Tests/Linq/Observable/SelectTest.cs` is `tests/linq/observable/select_test.rs` below the first test root (every
    component in snake case); a Rust file that no upstream file maps to that way is taken for the one upstream file
    with its name. A test that is not present by name and has a stated reason is not missing: it is *deferred* when
    the reason names a later phase (`phase`, `later`), else *waived*. An entry of `test_projects` may name, as its
    third element, the directories whose lines apply to that project (its tests are ported in several crates); the
    first reason found for a test is taken. The report lists the stated reasons, and those
    that name no upstream test of their file (which do not change the exit status).

What is counted

    Upstream: every method of a `.cs` file of the project that carries an attribute whose name ends in `Fact`,
    `Theory`, `Test`, `TestCase` or `Benchmark` (`[Fact]`, `[Theory]`, `[AvaloniaFact]`, `[Fact(Skip = "...")]`, the
    NUnit `[Test]`, the `[Benchmark]` of the benchmark project, ...). A `[Theory]` is one test, however many `[InlineData]` rows it has. Comments, strings and
    preprocessor lines are blanked before the search; code in an inactive `#if` branch is therefore counted too.
    A method that two classes of one file declare is one test per class, listed as `Class.Method` (the class is
    the one declared last before the method); overloads within a class are one test.

    Port: the functions of test code under the directories searched. Test code is
      - every `fn` of a `.rs` file that has a `#[test]` attribute anywhere (also inside a macro): the port
        writes a `[Theory]`, and a test that upstream runs once per subclass of its test class, as a function
        with upstream's name that takes the parameters, called by `#[test]` functions named after the rows or
        written by a macro;
      - the identifiers handed to a macro of such a file whose body has a `#[test]` (`tests! { name_a, name_b }`);
      - every `fn` of a file named `*_tests*.rs`, `tests.rs` or `test_*.rs` or below a directory named `tests`,
        and every `fn` after the first `#[cfg(test)]` of another file (the tests of the binding suites are
        written inside a macro of another file, which runs each for the reflection and the compiled form).
    Comments and string literals are blanked first. A helper function of test code therefore also counts as a
    name.

    Match: by name alone, and by number. Both names are lower-cased and their underscores removed; in the
    upstream name `Avalonia` reads as `Ferro`, a word-initial `Avn` as `Frn` and `avares` (the resource scheme)
    as `ferres` (the renames of docs/porting/PORTING-GUIDE.md). `Foo_Bar` and `FooBar` are one name. A name
    found in any Rust file of the directories counts as present, wherever the file is. A name that n tests of
    the project have (in several classes or files) needs n functions of the port: the n-th of them, in the
    order of the paths, is present when the port has n functions of the name, so the file reported for such a
    test may not be the one whose test is missing. A test that the port has under a different name counts as
    missing until it is renamed or given an alias. The same name in another upstream project is not accounted
    for: a test of the base library and one of the controls with one name are both satisfied by one function.

The alias and waiver file (docs/porting/data/test-aliases.toml), edited by hand, one section per project

    [[alias]]                       # the port has the test under a name the rule cannot derive
    project = "Avalonia.Controls.UnitTests"   # required: an entry applies to the tests of its project only
    file = "GridTests.cs"           # path below the project, with forward slashes; may be a glob
    test = "Upstream_Method_Name"   # or "Class.Upstream_Method_Name" in a file with several test classes;
                                    # may be a glob
    rust = "the_rust_test_function" # or a list, when one upstream test became several: each must exist as a
                                    # function of test code of the port, or the alias is reported as stale
    reason = "why the name differs"

    [[waive]]                       # the test cannot be ported or does not apply
    project = "Avalonia.Controls.UnitTests"
    file = "DesignTests.cs"         # may be a glob
    test = "Upstream_Method_Name"   # or: tests = ["A", "B"]; may be globs; neither waives the whole file
    reason = "what is missing or why it does not apply"

    An alias or a waiver applies only to a test that is not present by name; the first entry of the file that
    matches is taken, aliases before waivers. A waiver is for a test that cannot be expressed or whose subject
    the port does not have by decision. A test that is only not ported yet is not waived: it stays in the list
    of missing tests. An entry that matched no missing test (the test does not exist upstream, or the port has
    it by name) and an alias whose Rust function does not exist are listed at the end of the output as stale,
    and the script exits with 1.

Output

    The totals, then one line per upstream file that has tests which are neither present, aliased nor waived
    (`missing/total  file`) followed by the missing names, then the waived tests with their reasons, the aliases
    and the stale entries. Files are sorted by the number of missing tests, then by path, so the output is
    stable and can be committed and compared.

Standard library only (Python 3.11 or later, for `tomllib`), and `git` unless `--rev worktree` is given.
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

try:
    import tomllib
except ModuleNotFoundError:
    sys.exit("test_gaps.py needs Python 3.11 or later (tomllib)")
from collections import defaultdict

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
DATA = os.path.join(REPO, "docs", "porting", "data")
DEFAULT_ALIASES = os.path.join(DATA, "test-aliases.toml")
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import references  # noqa: E402

# The upstream test projects that are counted by `--all`: the project, and the short name of its output
# (docs/porting/data/test-gaps-<short name>.txt). The test projects that are not counted, with the reasons, are
# listed in docs/porting/CONTINUATION.md.
PROJECTS = [
    ("tests/Avalonia.Base.UnitTests", "base"),
    ("tests/Avalonia.Controls.UnitTests", "controls"),
    ("tests/Avalonia.Markup.UnitTests", "markup"),
    ("tests/Avalonia.Markup.Xaml.UnitTests", "markup-xaml"),
    ("tests/Avalonia.Skia.UnitTests", "skia"),
    ("tests/Avalonia.Headless.UnitTests", "headless"),
    ("tests/Avalonia.Themes.UnitTests", "themes"),
    ("tests/Avalonia.Build.Tasks.UnitTest", "build-tasks"),
    ("tests/Avalonia.DesignerSupport.Tests", "designer-support"),
    # The render tests: the sources are in this project; `Avalonia.Skia.RenderTests` compiles them and has no
    # tests of its own.
    ("tests/Avalonia.RenderTests", "render"),
    ("tests/Avalonia.LeakTests", "leak-tests"),
    ("tests/Avalonia.Benchmarks", "benchmarks"),
]

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


TEST_ATTRIBUTE_NAMES = ["Fact", "Theory", "Test", "TestCase", "Benchmark"]


def test_attribute(extra: list[str] | None = None) -> re.Pattern:
    """The attribute of a test method: a name that ends in one of TEST_ATTRIBUTE_NAMES, or in one of `extra`
    (`TestMethod` for MSTest)."""
    names = "|".join(TEST_ATTRIBUTE_NAMES + list(extra or []))
    return re.compile(r"\[\s*(?:[A-Za-z_][\w.]*\.)?\w*(?:" + names + r")(?:Attribute)?\s*[\](,]")


TEST_ATTRIBUTE = test_attribute()
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


def upstream_tests(text: str, attribute: re.Pattern = TEST_ATTRIBUTE) -> list[tuple[str, str]]:
    """The test methods of one C# file as (class, method), in order, each pair once. The class is the one
    declared last before the method."""
    text = blank_csharp(text)
    classes = [(m.start(), m.group(1)) for m in CLASS.finditer(text)]
    names: list[tuple[str, str]] = []
    seen = set()
    position = 0
    while True:
        m = attribute.search(text, position)
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


def read_project(upstream: str, project: str, rev: str | None) -> dict[str, str]:
    """Path below the project (forward slashes) -> text, for every `.cs` file of the project."""
    files: dict[str, str] = {}
    if rev:
        result = subprocess.run(
            ["git", "-C", upstream, "archive", "--format=tar", rev, project],
            capture_output=True,
        )
        if result.returncode != 0:
            sys.exit(f"cannot read {project} at {rev}: {result.stderr.decode(errors='replace').strip()}")
        with tarfile.open(fileobj=io.BytesIO(result.stdout)) as tar:
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


def tracked_commit(ref: dict | None = None) -> str:
    """The upstream commit the port follows, as docs/porting/TRACKING.md names it; for another reference, the
    `upstreamCommit` of its upstream-api.json."""
    if ref is not None and ref["documents"] != "framework":
        with open(os.path.join(ref["data"], "upstream-api.json"), encoding="utf-8") as handle:
            match = re.search(r'"upstreamCommit": "([0-9a-f]{40})"', handle.read(4000))
        if not match:
            sys.exit(f"{ref['data']}/upstream-api.json does not name the upstream commit")
        return match.group(1)
    with open(os.path.join(REPO, "docs", "porting", "TRACKING.md"), encoding="utf-8") as handle:
        match = re.search(r"upstream commit `([0-9a-f]{40})`", handle.read())
    if not match:
        sys.exit("docs/porting/TRACKING.md does not name the upstream commit")
    return match.group(1)


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


CFG_TEST = re.compile(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]")


def is_test_file(path: str) -> bool:
    """A file that is test code by its name or place, whether or not it has a `#[test]` of its own."""
    name = os.path.basename(path)
    parts = path.replace(os.sep, "/").split("/")
    return "_tests" in name or name == "tests.rs" or name.startswith("test_") or "tests" in parts[:-1]


def rust_tests(roots: list[str]) -> dict[str, list[tuple[str, str]]]:
    """Normalised name -> [(function name, path relative to the repository)] of every function of test code
    (see the module documentation)."""
    found: dict[str, list[tuple[str, str]]] = defaultdict(list)
    for root in roots:
        for directory, directories, names in os.walk(root):
            directories[:] = sorted(d for d in directories if d not in ("target", ".git", "node_modules"))
            for name in sorted(names):
                if not name.endswith(".rs"):
                    continue
                path = os.path.join(directory, name)
                relative = os.path.relpath(path, REPO).replace(os.sep, "/")
                with open(path, encoding="utf-8", errors="replace") as handle:
                    text = handle.read()
                if "test" not in text and not is_test_file(relative):
                    continue
                # String literals are blanked with spaces, which leaves `#[ignore = "..."]` as an attribute
                # without brackets inside it.
                blanked = blank_rust(text)
                if RUST_TEST_ATTRIBUTE.search(blanked):
                    # A file with tests, also when a macro writes them all (`#[test] fn $name()`).
                    functions = RUST_FN.findall(blanked)
                    declared = set(functions)
                    functions += [n for n in dict.fromkeys(macro_test_names(blanked)) if n not in declared]
                elif is_test_file(relative):
                    functions = RUST_FN.findall(blanked)
                else:
                    start = CFG_TEST.search(blanked)
                    functions = RUST_FN.findall(blanked, start.start()) if start else []
                for function in functions:
                    found[normalise_rust(function)].append((function, relative))
    return found


# --------------------------------------------------------------------------------------------------------------
# Matching
# --------------------------------------------------------------------------------------------------------------


def normalise_upstream(name: str) -> str:
    name = re.sub(r"(?<![a-z])Avn(?=[A-Z_]|$)", "Frn", name)
    return name.lower().replace("avalonia", "ferro").replace("avares", "ferres").replace("_", "")


def normalise_rust(name: str) -> str:
    return name.lower().replace("_", "")


def load_rules(path: str, project_name: str) -> tuple[list[dict], list[dict]]:
    """The aliases and the waivers of the project, in the order of the file."""
    if not os.path.exists(path):
        return [], []
    with open(path, "rb") as handle:
        data = tomllib.load(handle)
    aliases = []
    waivers = []
    for kind, target in (("alias", aliases), ("waive", waivers)):
        for entry in data.get(kind, []):
            if "project" not in entry:
                sys.exit(f"{path}: an [[{kind}]] entry without `project`: {entry}")
            if entry["project"] != project_name:
                continue
            tests = list(entry.get("tests", []))
            if "test" in entry:
                tests.insert(0, entry["test"])
            rust = entry.get("rust", [])
            target.append({
                "file": entry.get("file", "*"),
                "tests": tests or ["*"],
                "rust": [rust] if isinstance(rust, str) else list(rust),
                "reason": entry.get("reason", ""),
                "used": False,
            })
    return aliases, waivers


def find_rule(rules: list[dict], path: str, method: str, qualified: str) -> dict | None:
    for rule in rules:
        if not fnmatch.fnmatchcase(path, rule["file"]):
            continue
        if any(fnmatch.fnmatchcase(method, t) or fnmatch.fnmatchcase(qualified, t) for t in rule["tests"]):
            return rule
    return None


def describe(rule: dict) -> str:
    tests = ", ".join(rule["tests"])
    return f"{rule['file']}: {tests}"


NOT_PORTED = re.compile(r"^[ \t]*//[ \t]*Not ported(?:[ \t]*\(in part\))?:[ \t]*(.+?)[ \t]+-[ \t]+(.+?)[ \t]*$", re.M)
DEFERRED = re.compile(r"\b(?:phase|later)\b", re.I)


def snake(name: str) -> str:
    """PascalCase -> snake_case, as port_status.py writes the paths of the port (`SelectTest` -> `select_test`)."""
    out = []
    for i, c in enumerate(name):
        if c.isupper() and i > 0:
            p = name[i - 1]
            nx = name[i + 1] if i + 1 < len(name) else ""
            if p.islower() or (p.isdigit() and nx.islower()) or (p.isupper() and nx.islower()):
                out.append("_")
        out.append(c.lower() if c.isalnum() or c == "_" else "_")
    return re.sub(r"__+", "_", "".join(out)).strip("_")


def rust_test_file(upstream_path: str) -> str:
    """`Tests/Linq/Observable/SelectTest.cs` -> `tests/linq/observable/select_test.rs`."""
    parts = upstream_path.split("/")
    return "/".join([snake(c) for c in parts[:-1]] + [snake(parts[-1].rsplit(".", 1)[0]) + ".rs"])


def stated_reasons(root: str, upstream: dict[str, set[str]]) -> tuple[dict, list[str]]:
    """The lines `// Not ported: <Name> - <reason>` of the Rust test files below `root`: upstream path ->
    {test name: reason}, and the lines that name no upstream file. `upstream`: upstream path -> its test names. A
    line of a Rust file that no upstream file maps to (a test file the port splits off, such as
    `repeat_source_test.rs`) is taken for the one upstream file that has a test of the name."""
    upstream_paths = sorted(upstream)
    by_test: dict[str, list[str]] = defaultdict(list)
    for p in upstream_paths:
        for test in upstream[p]:
            by_test[test].append(p)
    by_rust = {rust_test_file(p): p for p in upstream_paths}
    by_stem: dict[str, list[str]] = defaultdict(list)
    for rust, p in by_rust.items():
        by_stem[rust.rsplit("/", 1)[-1]].append(p)
    out: dict[str, dict[str, str]] = defaultdict(dict)
    orphans: list[str] = []
    for directory, directories, names in os.walk(root):
        directories[:] = sorted(d for d in directories if d not in ("target", ".git"))
        for name in sorted(names):
            if not name.endswith(".rs"):
                continue
            path = os.path.join(directory, name)
            rel = os.path.relpath(path, root).replace(os.sep, "/")
            with open(path, encoding="utf-8", errors="replace") as handle:
                text = handle.read()
            matches = NOT_PORTED.findall(text)
            if not matches:
                continue
            upstream_path = by_rust.get(rel)
            if upstream_path is None and len(by_stem.get(name, [])) == 1:
                upstream_path = by_stem[name][0]
            for tests, reason in matches:
                for test in (t.strip() for t in tests.split(",")):
                    target = upstream_path
                    if target is None and len(by_test.get(test, [])) == 1:
                        target = by_test[test][0]
                    if target is None:
                        orphans.append(f"{os.path.relpath(path, REPO)}: {test}: no upstream test file maps to the file")
                    else:
                        out[target].setdefault(test, reason)
    return out, orphans


def count(upstream: str, project: str, rev: str | None, port: dict, alias_path: str,
          attribute: re.Pattern = TEST_ATTRIBUTE, stated_root: list[str] | None = None) -> dict:
    """Counts one project; returns the totals and the text of the report. `stated_root`: the directories of the
    Rust test files whose `// Not ported:` lines are read (a reference with `stated_reasons`)."""
    project_name = project.rsplit("/", 1)[1]
    sources = read_project(upstream, project, rev)
    aliases, waivers = load_rules(alias_path, project_name)
    parsed = {path: upstream_tests(sources[path], attribute) for path in sorted(sources)}
    upstream_names = {path: {method for _, method in pairs} for path, pairs in parsed.items()}
    stated: dict[str, dict[str, str]] = defaultdict(dict)
    stated_stale: list[str] = []
    for root in stated_root or []:
        found, orphans = stated_reasons(root, upstream_names)
        for path, tests in found.items():
            for test, reason in tests.items():
                stated[path].setdefault(test, reason)
        stated_stale.extend(orphans)
    stated_waived: list[tuple[str, str, str]] = []
    deferred: list[tuple[str, str, str]] = []

    total = 0
    files_with_tests = 0
    missing: dict[str, list[str]] = {}
    counts: dict[str, int] = {}
    waived: list[tuple[str, str, str]] = []
    aliased: list[tuple[str, str, str, str]] = []
    stale: list[str] = []

    occurrence: dict[str, int] = defaultdict(int)
    for path in sorted(sources):
        pairs = parsed[path]
        if not pairs:
            continue
        files_with_tests += 1
        counts[path] = len(pairs)
        total += len(pairs)
        several = len({owner for owner, _ in pairs}) > 1
        for owner, method in pairs:
            # A method that several classes or files of the project declare is one test in each: the n-th of
            # them, in the order of the paths, is present when the port has n functions of the name.
            key = normalise_upstream(method)
            occurrence[key] += 1
            if len(port.get(key, ())) >= occurrence[key]:
                continue
            qualified = f"{owner}.{method}"
            name = qualified if several else method
            alias = find_rule(aliases, path, method, qualified)
            if alias is not None:
                alias["used"] = True
                absent = [rust for rust in alias["rust"] if normalise_rust(rust) not in port]
                if alias["rust"] and not absent:
                    aliased.append((path, name, ", ".join(alias["rust"]), alias["reason"]))
                    continue
                stale.append(f"alias {path}: {name} -> {', '.join(absent) or '(no `rust` name)'}: "
                             "the port has no function of that name in test code")
            waiver = find_rule(waivers, path, method, qualified)
            if waiver is not None:
                waiver["used"] = True
                waived.append((path, name, waiver["reason"]))
                continue
            reason = stated.get(path, {}).get(method)
            if reason is not None:
                (deferred if DEFERRED.search(reason) else stated_waived).append((path, name, reason))
                continue
            missing.setdefault(path, []).append(name)

    for rule in aliases:
        if not rule["used"]:
            stale.append(f"alias {describe(rule)}: no upstream test of that name is missing by name")
    for rule in waivers:
        if not rule["used"]:
            stale.append(f"waiver {describe(rule)}: no upstream test of that name is missing")

    for path in sorted(stated):
        for test in stated[path]:
            if test not in upstream_names.get(path, ()):
                stated_stale.append(f"{path}: {test}: no upstream test of that name in the file")

    missing_count = sum(len(names) for names in missing.values())
    lines = []
    lines.append(f"Test gaps: {project_name}")
    lines.append(f"Upstream: {project}" + (f" at {rev}" if rev else " (working tree)"))
    lines.append("Generated by scripts/port-status/test_gaps.py; the rules of the match are at its top.")
    lines.append("")
    lines.append(f"Tests: {total} in {files_with_tests} files")
    if stated_root:
        lines.append(f"Present: {total - missing_count - len(waived) - len(stated_waived) - len(deferred)} "
                     f"({len(aliased)} of them through an alias)")
        lines.append(f"Waived: {len(waived) + len(stated_waived)} ({len(stated_waived)} of them stated in the Rust test files)")
        lines.append(f"Deferred: {len(deferred)}")
        lines.append(f"Stated not ported: {len(stated_waived) + len(deferred)}")
    else:
        lines.append(f"Present: {total - missing_count - len(waived)} ({len(aliased)} of them through an alias)")
        lines.append(f"Waived: {len(waived)}")
    lines.append(f"Missing: {missing_count} in {len(missing)} files")

    if missing:
        lines.append("")
        lines.append("Missing (missing/tests of the file)")
        for path in sorted(missing, key=lambda p: (-len(missing[p]), p)):
            lines.append("")
            lines.append(f"{len(missing[path])}/{counts[path]}  {path}")
            lines.extend(f"    {name}" for name in missing[path])
    if waived:
        lines.append("")
        lines.append("Waived")
        by_file: dict[str, list[tuple[str, str]]] = defaultdict(list)
        for path, name, reason in waived:
            by_file[path].append((name, reason))
        for path in sorted(by_file):
            lines.append("")
            lines.append(f"{len(by_file[path])}/{counts[path]}  {path}")
            last = None
            for name, reason in by_file[path]:
                if reason != last:
                    lines.append(f"  reason: {reason}")
                    last = reason
                lines.append(f"    {name}")
    for title, entries in (("Stated not ported, deferred to a later phase", deferred),
                           ("Stated not ported, waived (the reason is in the Rust test file)", stated_waived)):
        if not entries:
            continue
        lines.append("")
        lines.append(title)
        by_file = defaultdict(list)
        for path, name, reason in entries:
            by_file[path].append((name, reason))
        for path in sorted(by_file):
            lines.append("")
            lines.append(f"{len(by_file[path])}/{counts[path]}  {path}")
            for name, reason in by_file[path]:
                lines.append(f"    {name} - {reason}")
    if stated_stale:
        lines.append("")
        lines.append("Stated reasons that name no upstream test (not counted)")
        lines.extend(f"    {line}" for line in stated_stale)
    if aliased:
        lines.append("")
        lines.append("Aliases (upstream test -> function of the port)")
        last = None
        for path, name, rust, reason in aliased:
            if path != last:
                lines.append("")
                lines.append(path)
                last = path
            lines.append(f"    {name} -> {rust}: {reason}")
    if stale:
        lines.append("")
        lines.append("Stale entries of the alias file")
        lines.extend(f"    {line}" for line in stale)

    return {
        "project": project_name,
        "tests": total,
        "missing": missing_count,
        "files": len(missing),
        "waived": len(waived) + len(stated_waived),
        "deferred": len(deferred),
        "aliased": len(aliased),
        "stale": len(stale),
        "report": "\n".join(lines) + "\n",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("upstream", nargs="?")
    parser.add_argument("project", nargs="?")
    parser.add_argument("--reference")
    parser.add_argument("--rev", "--commit", default="tracked")
    parser.add_argument("--rust", "--root", action="append")
    parser.add_argument("--aliases", default=DEFAULT_ALIASES)
    parser.add_argument("--output")
    parser.add_argument("--summary", action="store_true")
    parser.add_argument("--all", action="store_true")
    arguments = parser.parse_args()

    ref = references.load(REPO, arguments.reference) if arguments.reference else None
    framework = ref is None or ref["documents"] == "framework"
    if arguments.upstream is None:
        if ref is None:
            parser.error("give the upstream checkout")
        arguments.upstream = ref["checkout"]
    if arguments.all == (arguments.project is not None):
        parser.error("give a test project, or --all")
    rev = {"tracked": tracked_commit(ref), "worktree": None}.get(arguments.rev, arguments.rev)
    default_roots = ref["test_roots"] if ref is not None else ["src", "tests"]
    roots = [root if os.path.isabs(root) else os.path.join(REPO, root) for root in arguments.rust or default_roots]
    port = rust_tests(roots)
    data = DATA if framework else ref["data"]
    if arguments.aliases == DEFAULT_ALIASES and not framework:
        arguments.aliases = os.path.join(data, "test-aliases.toml")
    attribute = test_attribute(ref["test_attributes"]) if ref is not None and ref["test_attributes"] else TEST_ATTRIBUTE
    stated_root = [roots[0]] if ref is not None and ref["stated_reasons"] else None
    projects = [(path, short) for short, path, *_ in ref["test_projects"]] if ref is not None and ref["test_projects"] else PROJECTS
    # A third element of an entry of `test_projects` names the directories whose `// Not ported:` lines apply to
    # that project (its tests are ported in several crates); by default the first test root.
    stated_roots = {path: [os.path.join(REPO, d) for d in rest[0]]
                    for short, path, *rest in (ref["test_projects"] or []) if rest} if ref is not None else {}

    def run(project: str) -> dict:
        roots_of = stated_roots.get(project, stated_root) if stated_root else None
        return count(arguments.upstream, project, rev, port, arguments.aliases, attribute, roots_of)

    if arguments.all and stated_root:
        print("| Upstream project | Tests | Missing | Files with missing tests | Waived | Deferred | Aliased | Output |")
        print("|---|---|---|---|---|---|---|---|")
        stale = 0
        for project, short in projects:
            result = run(project)
            output = f"test-gaps-{short}.txt"
            with open(os.path.join(data, output), "w", encoding="utf-8") as handle:
                handle.write(result["report"])
            stale += result["stale"]
            print(f"| `{result['project']}` | {result['tests']:,} | {result['missing']} | {result['files']} "
                  f"| {result['waived']} | {result['deferred']} | {result['aliased']} | `{output}` |")
        if stale:
            print(f"\n{stale} stale entries of the alias file: see the ends of the outputs", file=sys.stderr)
        return 1 if stale else 0

    if arguments.all:
        print("| Upstream project | Tests | Missing | Files with missing tests | Waived | Aliased | Output |")
        print("|---|---|---|---|---|---|---|")
        stale = 0
        for project, short in projects:
            result = run(project)
            output = f"test-gaps-{short}.txt"
            with open(os.path.join(data, output), "w", encoding="utf-8") as handle:
                handle.write(result["report"])
            stale += result["stale"]
            print(f"| `{result['project']}` | {result['tests']:,} | {result['missing']} | {result['files']} "
                  f"| {result['waived']} | {result['aliased']} | `{output}` |")
        if stale:
            print(f"\n{stale} stale entries of the alias file: see the ends of the outputs", file=sys.stderr)
        return 1 if stale else 0

    project = arguments.project.strip("/")
    if "/" not in project:
        project = "tests/" + project
    result = run(project)
    totals = "\t".join(str(result[k]) for k in ("project", "tests", "missing", "files", "waived", "aliased"))
    if arguments.summary:
        print(totals)
    elif arguments.output:
        with open(arguments.output, "w", encoding="utf-8") as handle:
            handle.write(result["report"])
        print(totals)
    else:
        sys.stdout.write(result["report"])
    return 1 if result["stale"] else 0


if __name__ == "__main__":
    sys.exit(main())
