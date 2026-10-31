#!/usr/bin/env python3
"""Measures the .NET-shaped types and mechanisms of the Rust sources (#423, docs/idiomatic-rust.md).

The port kept many shapes of the .NET base class library: interface-style traits, ports of `System.*`
types (`TimeSpan`, `Uri`, `Dictionary`, `Task`, ...), `IDisposable`, exception-shaped error types.
The names of the control API stay .NET-like; the types and mechanisms it is made of become idiomatic
Rust and std (docs/idiomatic-rust.md). This script counts, per crate, each pattern of the inventory
table of #423, exactly: it reads every `.rs` file under `src/`, `samples/` and `tests/`, blanks
comments, string literals and char literals (newlines kept), and matches the patterns below on what
is left. Generated files are skipped (see `is_generated`), and so are build directories.

    scripts/idioms/inventory.py                   the summary table, over the crates of src/
    scripts/idioms/inventory.py --by-crate        and, for each pattern, the count of each crate
    scripts/idioms/inventory.py --all             the summary over src/, samples/ and tests/
    scripts/idioms/inventory.py --json            everything as JSON (all roots, per crate)
    scripts/idioms/inventory.py --check           fail when a declaration of a .NET-shaped
                                                  infrastructure type is not in the baseline (the
                                                  conventions job of CI runs this)
    scripts/idioms/inventory.py --update-baseline write the baseline from the sources

The check. A declaration (`struct`, `enum`, `union`, `trait` or `type`, at any depth, also inside a
macro invocation) is .NET-shaped infrastructure when its name ends in `Exception` (an error type is
named `...Error` and implements `std::error::Error`), or when it is the name of a type of the .NET
base class library that has a std or chosen-crate counterpart (`SYSTEM_TYPES` below: `TimeSpan`,
`Uri`, `Guid`, `Dictionary`, `Task`, `IDisposable`, ...). `scripts/idioms/baseline.json` holds, for
each crate, how many such declarations of each name it has. `--check` fails when a crate declares a
name more often than the baseline allows (a new port, or a new duplicate), and when the baseline
allows more than the sources have (the baseline only shrinks: run `--update-baseline` in the pull
request that removes a declaration). Names that stay .NET-like by the owner's decision are not
checked: control types, `I...` traits of upstream interfaces, `...EventArgs`, `INotifyPropertyChanged`,
`ObservableCollection`.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from collections import defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
ROOTS = ["src", "samples", "tests"]
SKIPPED_DIRECTORIES = {"target", ".git", "node_modules", ".claude", "obj", "bin"}
BASELINE = os.path.join(ROOT, "scripts", "idioms", "baseline.json")

# Types of the .NET base class library (System, System.Collections.*, System.Threading.*,
# System.Globalization, System.Text, System.IO, System.Numerics and the Rx disposables) that have a
# counterpart in std or in the crate chosen for them (docs/idiomatic-rust.md, "Type replacements").
# A new declaration of one of these names is a new port, or a new duplicate of one. Names that a
# control or a media type of the framework uses as well (`Path`, `Image`, `Point`, `Timer` of the
# dispatcher, ...) are not listed: they are not ports.
SYSTEM_TYPES = {
    # System
    "TimeSpan", "DateTime", "DateTimeOffset", "DateTimeKind", "DayOfWeek", "Guid", "Uri",
    "UriBuilder", "UriKind", "UriFormatException", "Version", "Lazy", "WeakReference", "Nullable",
    "Tuple", "ValueTuple", "KeyValuePair", "Random", "Convert", "BitConverter", "Environment", "GC",
    "Math", "MathF", "Decimal", "Half", "HashCode", "Delegate", "MulticastDelegate", "EventHandler",
    "Comparison", "Converter", "IDisposable", "IAsyncDisposable", "IComparable", "IEquatable",
    "ICloneable", "IConvertible", "IFormatProvider", "IFormattable", "IServiceProvider", "IProgress",
    "Progress", "IObservable", "IObserver", "Activator", "ValueType", "Span", "ReadOnlySpan",
    "Memory", "ReadOnlyMemory", "ArraySegment", "StringComparison", "StringComparer", "Exception",
    "AggregateException",
    # System.Collections, System.Collections.Generic, System.Collections.Concurrent and .Immutable
    "Dictionary", "List", "HashSet", "SortedSet", "SortedDictionary", "SortedList", "Queue", "Stack",
    "LinkedList", "LinkedListNode", "PriorityQueue", "BitArray", "Hashtable", "ArrayList",
    "ConcurrentDictionary", "ConcurrentQueue", "ConcurrentBag", "ConcurrentStack",
    "BlockingCollection", "IEnumerable", "IEnumerator", "ICollection", "IList", "IDictionary",
    "ISet", "IReadOnlyList", "IReadOnlyCollection", "IReadOnlyDictionary", "IReadOnlySet",
    "IComparer", "IEqualityComparer", "EqualityComparer", "Comparer", "ReadOnlyDictionary",
    "ImmutableArray", "ImmutableList", "ImmutableDictionary", "ImmutableHashSet",
    # System.Threading and System.Threading.Tasks
    "Thread", "ThreadPool", "Monitor", "Interlocked", "Volatile", "Mutex", "Semaphore",
    "SemaphoreSlim", "ManualResetEvent", "ManualResetEventSlim", "AutoResetEvent", "CountdownEvent",
    "Barrier", "ReaderWriterLock", "ReaderWriterLockSlim", "SpinLock", "SpinWait", "WaitHandle",
    "CancellationToken", "CancellationTokenSource", "CancellationTokenRegistration",
    "SynchronizationContext", "ExecutionContext", "Task", "ValueTask", "TaskCompletionSource",
    "TaskScheduler", "TaskFactory", "TaskStatus", "TaskCreationOptions", "TaskContinuationOptions",
    "Parallel", "AsyncLocal", "ThreadLocal",
    # System.Globalization
    "CultureInfo", "NumberFormatInfo", "DateTimeFormatInfo", "TextInfo", "CompareInfo",
    "CompareOptions", "Calendar", "GregorianCalendar", "NumberStyles", "DateTimeStyles",
    "TimeZoneInfo", "RegionInfo", "UnicodeCategory", "CharUnicodeInfo", "StringInfo",
    # System.Text and System.Text.RegularExpressions
    "StringBuilder", "Encoding", "UTF8Encoding", "ASCIIEncoding", "UnicodeEncoding", "Rune",
    "Regex", "RegexOptions",
    # System.IO
    "Stream", "MemoryStream", "FileStream", "BufferedStream", "TextReader", "TextWriter",
    "StreamReader", "StreamWriter", "StringReader", "StringWriter", "BinaryReader", "BinaryWriter",
    "FileInfo", "DirectoryInfo", "FileSystemWatcher", "SeekOrigin", "FileMode", "FileAccess",
    # System.Numerics
    "BigInteger", "Complex", "Vector2", "Vector3", "Vector4", "Matrix3x2", "Matrix4x4",
    "Quaternion", "Plane",
    # System.Reactive.Disposables (the Rx port): RAII guards in Rust
    "Disposable", "CompositeDisposable", "SerialDisposable", "SingleAssignmentDisposable",
    "MultipleAssignmentDisposable", "RefCountDisposable", "BooleanDisposable",
    "CancellationDisposable",
}

DECLARATION = re.compile(r"\b(struct|enum|union|trait|type)\s+([A-Za-z_]\w*)")
# `ferro_markup_type!(struct TimeSpan { .. })` describes the markup type of an existing Rust type:
# it declares no Rust type, and markup names never change. Its keyword is blanked before matching.
MARKUP_TYPE = re.compile(r"(\bferro_markup_type!\s*[({]\s*)(struct|enum|union|trait|type)\b")


def declared(names):
    """Declarations of the names `names` (a regex) matches, with any visibility, at any depth."""
    return re.compile(r"\b(?:struct|enum|union|trait|type)\s+(" + names + r")\b")


# The patterns of the inventory table of #423, in its order. Each is (key, title, regex, what is
# counted, direction). A `decl` pattern counts declarations, a `use` pattern counts the occurrences
# of the name (the files column is the number of files with at least one).
PATTERNS = [
    ("i_traits", "Traits named `I...`", re.compile(r"\btrait\s+(I[A-Z]\w*)"), "decl",
     "keep (upstream interface names)"),
    ("idisposable_impls", "`impl IDisposable for`",
     re.compile(r"\bimpl\b[^{};]*?\bIDisposable\s+for\b"), "decl", "Drop and RAII guards"),
    ("idisposable_uses", "`IDisposable` (uses)", re.compile(r"\bIDisposable\b"), "use",
     "Drop and RAII guards"),
    ("get_accessors", "`pub fn get_x(&self)` getters",
     re.compile(r"\bpub\s+fn\s+get_\w+\s*(?:<[^>()]*>)?\s*\(\s*&\s*(?:'\w+\s+)?self\s*,?\s*\)"),
     "decl", "keep where they mirror a .NET member"),
    ("exception_types", "`...Exception` types", declared(r"\w*Exception"), "decl",
     "`...Error` with `std::error::Error`"),
    ("error_types", "`...Error` types", declared(r"\w*Error"), "decl", "(target shape)"),
    ("std_error_impls", "`impl std::error::Error for`",
     re.compile(r"\bimpl\b(?:\s*<[^>{}]*>)?\s*(?:::)?(?:(?:std|core)::)?(?:error::)?Error\s+for\b"),
     "decl", "(target shape)"),
    ("time_span", "`TimeSpan`", re.compile(r"\bTimeSpan\b"), "use", "`std::time::Duration`"),
    ("date_time", "`DateTime`", re.compile(r"\bDateTime\b"), "use",
     "`SystemTime` and a calendar type (decide once)"),
    ("date_time_offset", "`DateTimeOffset`", re.compile(r"\bDateTimeOffset\b"), "use",
     "`SystemTime` and a calendar type (decide once)"),
    ("culture_info", "`CultureInfo`", re.compile(r"\bCultureInfo\b"), "use", "ICU4X `Locale`"),
    ("uri", "`Uri`", re.compile(r"\bUri\b"), "use", "`url::Url`"),
    ("guid_types", "`Guid` type declarations", declared(r"Guid"), "decl", "one type, or `uuid`"),
    ("guid_uses", "`Guid` (uses)", re.compile(r"\bGuid\b"), "use", "one type, or `uuid`"),
    ("dictionary", "`Dictionary` (uses)", re.compile(r"\bDictionary\b"), "use",
     "`HashMap`, or `IndexMap` where the order matters"),
    ("task", "`Task<`", re.compile(r"\bTask\s*<"), "use", "futures on the dispatcher's executor"),
    ("task_completion_source", "`TaskCompletionSource`", re.compile(r"\bTaskCompletionSource\b"),
     "use", "a oneshot channel"),
    ("synchronization_context", "`SynchronizationContext`",
     re.compile(r"\bSynchronizationContext\b"), "use", "the dispatcher's executor"),
    ("cancellation_token", "`CancellationToken...`",
     re.compile(r"\bCancellationToken(?:Source|Registration)?\b"), "use",
     "one cancellation type (decide once)"),
    ("event_args", "`...EventArgs` structs", re.compile(r"\bstruct\s+(\w*EventArgs)\b"), "decl",
     "keep (control API)"),
    ("inotify_property_changed", "`INotifyPropertyChanged`",
     re.compile(r"\bINotifyPropertyChanged\b"), "use", "keep"),
    ("inotify_collection_changed", "`INotifyCollectionChanged`",
     re.compile(r"\bINotifyCollectionChanged\b"), "use", "keep"),
    ("observable_collection", "`ObservableCollection`", re.compile(r"\bObservableCollection\b"),
     "use", "keep"),
]

# A file is generated when one of its first lines is a plain `//` comment that says so (the headers
# of scripts/generate_composition.py, generate_markup_types.py, generate_view_builders.py, the
# Unicode tables, the compiled markup of the themes and the other checked-in outputs).
GENERATED_HEADER = re.compile(r"^\s*//(?![/!]).*(@generated|\bgenerated by\b|\bdo not edit\b)", re.I)
GENERATED_HEADER_LINES = 8


def is_generated(text):
    for line in text.split("\n", GENERATED_HEADER_LINES)[:GENERATED_HEADER_LINES]:
        if GENERATED_HEADER.match(line):
            return True
    return False


# --- Blanking of comments and literals --------------------------------------------------------

_SPECIAL = re.compile(r"""/[/*]|"|\b[bc]?r\#*"|\bb"|\bc"|'""")
_CHAR = re.compile(r"""'(?:[^'\\\n]|\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F_]{1,8}\}|.))'""")
_STRING_BODY = re.compile(r'(?:[^"\\]|\\.)*"', re.S)


def _blank(text):
    """Spaces for every character but newlines."""
    return re.sub(r"[^\n]", " ", text)


def blank_comments_and_literals(src):
    """`src` with comments, string literals and char literals replaced by spaces, newlines kept."""
    out = []
    pos = 0
    n = len(src)
    while True:
        match = _SPECIAL.search(src, pos)
        if not match:
            out.append(src[pos:])
            break
        start = match.start()
        token = match.group(0)
        out.append(src[pos:start])
        if token == "//":
            end = src.find("\n", start)
            end = n if end < 0 else end
        elif token == "/*":
            depth = 1
            end = start + 2
            while depth and end < n:
                opening = src.find("/*", end)
                closing = src.find("*/", end)
                if closing < 0:
                    end = n
                    break
                if 0 <= opening < closing:
                    depth += 1
                    end = opening + 2
                else:
                    depth -= 1
                    end = closing + 2
        elif token == "'":
            char = _CHAR.match(src, start)
            if not char:
                # A lifetime or a label: kept.
                out.append("'")
                pos = start + 1
                continue
            end = char.end()
        elif "r" in token:
            hashes = token.count("#")
            closing = src.find('"' + "#" * hashes, match.end())
            end = n if closing < 0 else closing + 1 + hashes
        else:
            body = _STRING_BODY.match(src, match.end())
            end = body.end() if body else n
        out.append(_blank(src[start:end]))
        pos = end
    return "".join(out)


# --- Files and crates -------------------------------------------------------------------------

_PACKAGE_NAME = re.compile(r'^\[package\][^\[]*?^name\s*=\s*"([^"]+)"', re.M | re.S)
_crates = {}


def crate_of(path):
    """The package name of the manifest the file belongs to."""
    directory = os.path.dirname(path)
    seen = []
    while directory.startswith(ROOT) and directory != ROOT:
        if directory in _crates:
            name = _crates[directory]
            break
        seen.append(directory)
        manifest = os.path.join(directory, "Cargo.toml")
        if os.path.exists(manifest):
            with open(manifest, encoding="utf-8") as f:
                match = _PACKAGE_NAME.search(f.read())
            if match:
                name = match.group(1)
                break
        directory = os.path.dirname(directory)
    else:
        name = "(no crate)"
    for entry in seen:
        _crates[entry] = name
    return name


def rust_files(roots):
    for top in roots:
        for directory, directories, files in os.walk(os.path.join(ROOT, top)):
            directories[:] = sorted(d for d in directories if d not in SKIPPED_DIRECTORIES)
            for name in sorted(files):
                if name.endswith(".rs"):
                    full = os.path.join(directory, name)
                    yield top, full, os.path.relpath(full, ROOT).replace(os.sep, "/")


# --- Measurement ------------------------------------------------------------------------------


def measure(roots):
    """Per root and crate: the matches and files of each pattern, and the checked declarations."""
    counts = defaultdict(lambda: defaultdict(lambda: defaultdict(lambda: [0, 0])))
    declarations = defaultdict(lambda: defaultdict(list))
    skipped = []
    scanned = 0
    for top, full, rel in rust_files(roots):
        with open(full, encoding="utf-8", errors="replace") as f:
            text = f.read()
        if is_generated(text):
            skipped.append(rel)
            continue
        scanned += 1
        code = MARKUP_TYPE.sub(lambda m: m.group(1) + " " * len(m.group(2)),
                               blank_comments_and_literals(text))
        crate = crate_of(full)
        for key, _, pattern, _, _ in PATTERNS:
            found = len(pattern.findall(code))
            if found:
                entry = counts[top][crate][key]
                entry[0] += found
                entry[1] += 1
        for match in DECLARATION.finditer(code):
            name = match.group(2)
            if name.endswith("Exception") or name in SYSTEM_TYPES:
                line = code.count("\n", 0, match.start()) + 1
                declarations[crate][name].append(f"{rel}:{line}")
    return counts, declarations, skipped, scanned


def totals(counts, tops):
    """Per pattern: matches, files and crates over the roots `tops`."""
    result = {}
    for key, *_ in PATTERNS:
        matches = files = crates = 0
        for top in tops:
            for per_pattern in counts[top].values():
                if key in per_pattern:
                    matches += per_pattern[key][0]
                    files += per_pattern[key][1]
                    crates += 1
        result[key] = (matches, files, crates)
    return result


def print_summary(counts, tops, title):
    print(f"## {title}\n")
    print("| Pattern | Counts | Matches | Files | Crates | Direction |")
    print("|---|---|---:|---:|---:|---|")
    for key, label, _, kind, direction in PATTERNS:
        matches, files, crates = totals(counts, tops)[key]
        what = "declarations" if kind == "decl" else "occurrences"
        print(f"| {label} | {what} | {matches} | {files} | {crates} | {direction} |")
    print()


def print_by_crate(counts, tops):
    for key, label, *_ in PATTERNS:
        rows = defaultdict(lambda: [0, 0])
        for top in tops:
            for crate, per_pattern in counts[top].items():
                if key in per_pattern:
                    rows[crate][0] += per_pattern[key][0]
                    rows[crate][1] += per_pattern[key][1]
        if not rows:
            continue
        print(f"### {label}\n")
        print("| Crate | Matches | Files |")
        print("|---|---:|---:|")
        for crate, (matches, files) in sorted(rows.items(), key=lambda item: (-item[1][0], item[0])):
            print(f"| {crate} | {matches} | {files} |")
        print()


def checked_counts(declarations):
    return {crate: {name: len(places) for name, places in sorted(names.items())}
            for crate, names in sorted(declarations.items())}


def check(declarations):
    try:
        with open(BASELINE, encoding="utf-8") as f:
            baseline = json.load(f)["declarations"]
    except FileNotFoundError:
        print(f"no baseline at {os.path.relpath(BASELINE, ROOT)}: run --update-baseline", file=sys.stderr)
        return 1
    current = checked_counts(declarations)
    new, stale = [], []
    for crate, names in current.items():
        for name, count in names.items():
            allowed = baseline.get(crate, {}).get(name, 0)
            if count > allowed:
                places = ", ".join(declarations[crate][name])
                new.append(f"{crate}: {count} declaration(s) of `{name}`, the baseline allows "
                           f"{allowed} ({places})")
    for crate, names in baseline.items():
        for name, allowed in names.items():
            count = current.get(crate, {}).get(name, 0)
            if count < allowed:
                stale.append(f"{crate}: the baseline allows {allowed} declaration(s) of `{name}`, "
                             f"the sources have {count}")
    for entry in new:
        print(f"::error::{entry}")
    for entry in stale:
        print(f"::error::{entry}")
    if new:
        print(f"{len(new)} new declaration(s) of .NET-shaped infrastructure types. Use the std or "
              "chosen-crate type of docs/idiomatic-rust.md (\"Type replacements\"); an error type is "
              "named ...Error and implements std::error::Error. A deliberate exception goes into "
              "scripts/idioms/baseline.json (--update-baseline) with the reason in the pull request.",
              file=sys.stderr)
    if stale:
        print(f"{len(stale)} baseline entr(y/ies) above the sources: the baseline only shrinks, run "
              "scripts/idioms/inventory.py --update-baseline and commit it.", file=sys.stderr)
    if new or stale:
        return 1
    total = sum(sum(names.values()) for names in current.values())
    print(f"no new .NET-shaped infrastructure declarations ({total} in the baseline)")
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--by-crate", action="store_true", help="the count of each crate, per pattern")
    parser.add_argument("--all", action="store_true", help="the summary over src/, samples/ and tests/")
    parser.add_argument("--json", action="store_true", help="everything as JSON")
    parser.add_argument("--check", action="store_true", help="fail on declarations beyond the baseline")
    parser.add_argument("--update-baseline", action="store_true", help="write the baseline")
    args = parser.parse_args()

    counts, declarations, skipped, scanned = measure(ROOTS)

    if args.update_baseline:
        with open(BASELINE, "w", encoding="utf-8") as f:
            json.dump({
                "comment": "Declarations of .NET-shaped infrastructure types per crate, written by "
                           "scripts/idioms/inventory.py --update-baseline. It only shrinks "
                           "(docs/idiomatic-rust.md).",
                "declarations": checked_counts(declarations),
            }, f, indent=2, sort_keys=False, ensure_ascii=False)
            f.write("\n")
        total = sum(len(places) for names in declarations.values() for places in names.values())
        print(f"wrote {os.path.relpath(BASELINE, ROOT)}: {total} declarations in {len(declarations)} crates")
        return 0
    if args.check:
        return check(declarations)
    if args.json:
        json.dump({
            "scanned_files": scanned,
            "generated_files_skipped": len(skipped),
            "patterns": {key: {"title": label, "counts": kind, "direction": direction}
                         for key, label, _, kind, direction in PATTERNS},
            "totals": {top: {key: dict(zip(("matches", "files", "crates"), value))
                             for key, value in totals(counts, [top]).items()} for top in ROOTS},
            "per_crate": {top: {crate: {key: dict(zip(("matches", "files"), value))
                                        for key, value in sorted(per_pattern.items())}
                                for crate, per_pattern in sorted(counts[top].items())}
                          for top in ROOTS},
            "declarations": {crate: dict(sorted(names.items()))
                             for crate, names in sorted(declarations.items())},
        }, sys.stdout, indent=2, ensure_ascii=False)
        print()
        return 0

    print(f"{scanned} Rust files scanned, {len(skipped)} generated files skipped.\n")
    print_summary(counts, ["src"], "src/ (the crates of the framework)")
    if args.all:
        print_summary(counts, ["samples", "tests"], "samples/ and tests/")
    if args.by_crate:
        print_by_crate(counts, ROOTS if args.all else ["src"])
    return 0


if __name__ == "__main__":
    sys.exit(main())
