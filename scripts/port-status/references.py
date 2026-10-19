#!/usr/bin/env python3
"""The upstream references the port tracks (scripts/port-status/references.toml).

    python3 scripts/port-status/references.py --names            # the names, the default first
    python3 scripts/port-status/references.py --shell reactive   # the settings of one, as shell assignments (run.sh)

Imported by port_status.py and test_gaps.py: `load(repo, name)` returns one reference with its paths made absolute.
Standard library only (Python 3.11 or later, for `tomllib`).
"""

from __future__ import annotations

import argparse
import os
import shlex
import sys
import tomllib

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
CONFIG = os.path.join(HERE, "references.toml")

# The settings of a reference and their defaults (the behaviour of the framework reference).
DEFAULTS = {
    "title": "",
    "plan": "",
    "default": False,
    "checkout": "",
    "checkout_env": "",
    "projects": "scripts/api-extract/projects.json",
    "data": "docs/porting/data",
    "tracking": "docs/porting/tracking",
    "documents": "framework",
    "api": "all",
    "overloads": "count",
    "method_macros": False,
    "doc_citations": False,
    "waived_files": False,
    "arity_names": False,
    "test_projects": None,
    "test_roots": ["src", "tests"],
    "test_attributes": [],
    "stated_reasons": False,
    "test_files": "",
}
PATHS = ("projects", "data", "tracking")


def all_references(repo: str = REPO) -> list[dict]:
    with open(CONFIG, "rb") as handle:
        entries = tomllib.load(handle).get("reference", [])
    out = []
    for entry in entries:
        unknown = set(entry) - set(DEFAULTS) - {"name"}
        if unknown:
            sys.exit(f"{CONFIG}: unknown keys {sorted(unknown)} in the reference {entry.get('name')}")
        ref = dict(DEFAULTS)
        ref.update(entry)
        for key in PATHS:
            ref[key] = os.path.normpath(os.path.join(repo, ref[key]))
        ref["checkout_config"] = ref["checkout"]        # as written in references.toml (for the documents)
        checkout = os.environ.get(ref["checkout_env"]) if ref["checkout_env"] else None
        ref["checkout"] = os.path.normpath(os.path.join(repo, checkout or ref["checkout"]))
        ref["test_roots"] = [os.path.normpath(os.path.join(repo, r)) for r in ref["test_roots"]]
        ref["repo"] = repo
        out.append(ref)
    defaults = [r for r in out if r["default"]]
    if len(defaults) != 1:
        sys.exit(f"{CONFIG}: exactly one reference must have `default = true`")
    return sorted(out, key=lambda r: not r["default"])


def load(repo: str = REPO, name: str | None = None) -> dict:
    refs = all_references(repo)
    if name is None:
        return refs[0]
    for ref in refs:
        if ref["name"] == name:
            return ref
    sys.exit(f"no reference named {name!r} in {CONFIG} (known: {', '.join(r['name'] for r in refs)})")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--repo", default=REPO)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--names", action="store_true")
    group.add_argument("--shell", metavar="NAME")
    args = parser.parse_args()
    if args.names:
        print("\n".join(r["name"] for r in all_references(args.repo)))
        return 0
    ref = load(args.repo, args.shell)
    for key in ("name", "title", "checkout", "projects", "data", "tracking", "documents"):
        print(f"ref_{key}={shlex.quote(str(ref[key]))}")
    print(f"ref_default={1 if ref['default'] else 0}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
