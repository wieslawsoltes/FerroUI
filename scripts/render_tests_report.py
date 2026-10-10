#!/usr/bin/env python3
"""Prints the measured errors of a run of the render tests as a Markdown table.

    python3 scripts/render_tests_report.py <output directory>/<backend>/results.tsv [--failing] [--groups]

The render tests (tests/FerroUI.RenderTests) append one line to `results.tsv` of their output directory for
every comparison of an output with its expected image: the output file, the error (the root mean square
error of upstream's `TestRenderHelper.CompareImages`) and `pass` or `fail` (allowed: 0.022). A file is
compared again by every run, and the lines of all runs are in the file: the last line of an output counts.
Delete the file before a run whose table is to hold that run alone.

    --failing   only the tests with an output beyond the allowed error
    --groups    one line per directory: tests, tests within the allowed error, largest error

Standard library only.
"""

import sys
from collections import OrderedDict, defaultdict

SUFFIXES = [
    (".immediate.out.png", "immediate"),
    (".composited.out.png", "composited"),
    (".skia.out.png", "composited"),
    (".out.png", "other"),
]


def main() -> int:
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if len(args) != 1:
        print(__doc__)
        return 2
    failing_only = "--failing" in sys.argv
    groups = "--groups" in sys.argv

    last = OrderedDict()
    with open(args[0], encoding="utf-8") as f:
        for line in f:
            parts = line.rstrip("\n").split("\t")
            if len(parts) == 3:
                last[parts[0]] = (parts[1], parts[2])

    tests = defaultdict(dict)
    for name, (error, verdict) in last.items():
        for suffix, kind in SUFFIXES:
            if name.endswith(suffix):
                tests[name[: -len(suffix)]][kind] = (error, verdict)
                break

    def cell(test, kind):
        if kind not in test:
            return ""
        error, verdict = test[kind]
        return ("%.4f" % float(error)) if error else "not comparable"

    if groups:
        by_dir = defaultdict(list)
        for name, test in tests.items():
            by_dir[name.rsplit("/", 1)[0]].append(test)
        print("| Directory | Tests | Within the allowed error | Largest error |")
        print("|---|---|---|---|")
        total = passed_total = 0
        for directory in sorted(by_dir):
            items = by_dir[directory]
            passed = sum(1 for t in items if all(v == "pass" for _, v in t.values()))
            largest = max((float(e) for t in items for e, _ in t.values() if e), default=0.0)
            total += len(items)
            passed_total += passed
            print("| `%s` | %d | %d | %.4f |" % (directory, len(items), passed, largest))
        print("| **all** | **%d** | **%d** | |" % (total, passed_total))
        return 0

    print("| Test | Immediate | Composited | Other output | Result |")
    print("|---|---|---|---|---|")
    for name in sorted(tests):
        test = tests[name]
        ok = all(v == "pass" for _, v in test.values())
        if failing_only and ok:
            continue
        print(
            "| `%s` | %s | %s | %s | %s |"
            % (name, cell(test, "immediate"), cell(test, "composited"), cell(test, "other"), "pass" if ok else "fail")
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
