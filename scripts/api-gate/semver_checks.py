#!/usr/bin/env python3
"""The API gate of the signatures: cargo-semver-checks against the last release (docs/release.md,
"API stability").

Within a minor version (0.4) the public API of the published crates only grows. This script runs
`cargo semver-checks` for every published crate that the baseline release has, with the release
type `minor` (additions allowed, nothing removed or changed) and the default features:

    semver_checks.py            run the gate
    semver_checks.py --list     print the baseline and the crates it checks, run nothing
    semver_checks.py --fetch    fetch the baseline into a shallow clone first (CI)

The baseline is the tag of the release (`v0.4.0`) when the repository has it, and otherwise
`BASELINE_COMMIT`. 0.4.0 is not published yet (no tag, not on crates.io) and the workspace on main
is still at 0.4.0, so the 0.4.0 that will ship is main: the baseline is the commit of main the gate
was merged on, and it moves to the tag `v0.4.0` once that is published. A crate the baseline does
not have (one that joined after it) is not checked.

The lints `constructible_struct_adds_field` and `repr_c_plain_struct_fields_reordered` are not
counted for the virtual tables of the classes (`*VTable` structs, which `ferro_class!` writes):
they are the dispatch machinery of the class model, every new virtual member adds a field to them,
and no application constructs one. Every other result counts. The defaults of properties, which no signature shows, are
guarded by the snapshot test `property_defaults_tests` of `ferroui-controls`.

Only the standard library, `git`, and the `cargo` and `cargo-semver-checks` on PATH.
"""

import json
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# The release the gate compares with, and the commit of its sources. 0.4.0 is not published (no tag,
# not on crates.io) and main is still at 0.4.0, so the baseline is the commit of main the gate was
# merged on (the merge of #332, 2026-10-29); the tag v0.4.0 replaces it once the release is
# published (baseline_rev). Updated by the release that starts the next minor version
# (docs/release.md, "API stability").
BASELINE_VERSION = "0.4.0"
BASELINE_COMMIT = "80e50ba2238ac2548b437122b39b11257fb4c1c9"

# Lints that are not counted for the virtual tables of the classes (`*VTable` structs written by
# `ferro_class!`): a new virtual member adds a field to the table and may move the others.
VTABLE_LINTS = {"constructible_struct_adds_field", "repr_c_plain_struct_fields_reordered"}
VTABLE_ITEM = re.compile(r"\b\w+VTable\.")
FAILURE = re.compile(r"^--- (failure|warning) (\S+):")
PROGRESS = re.compile(r"^(Building|Built|Parsing|Parsed|Checking|Checked|Finished|Summary|Warning|Compiling|Updating|Downloading|Downloaded|Cloning|Resolving)\b")


def counted_failures(output):
    """The failures of the output of cargo-semver-checks that count: (crate, lint, item)."""
    crate, kind, lint, listing, failures = None, None, None, False, []
    for line in output.splitlines():
        checking = re.search(r"Checking (\S+) v\S+ -> ", line)
        if checking:
            crate, listing = checking.group(1), False
            continue
        header = FAILURE.match(line.strip())
        if header:
            kind, lint, listing = header.group(1), header.group(2), False
            continue
        if line.strip() == "Failed in:":
            listing = True
            continue
        if listing:
            item = line.strip()
            if not item:
                listing = False
                continue
            if PROGRESS.match(item):
                # The progress of the next crate, written between the results.
                continue
            if kind != "failure":
                continue
            if lint in VTABLE_LINTS and VTABLE_ITEM.search(item):
                continue
            failures.append((crate, lint, item))
    return failures


def git(*args, check=True):
    return subprocess.run(["git", *args], cwd=ROOT, check=check, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)


def baseline_rev():
    tag = f"v{BASELINE_VERSION}"
    if git("rev-parse", "--verify", "--quiet", f"refs/tags/{tag}^{{commit}}", check=False).returncode == 0:
        return tag
    return BASELINE_COMMIT


def packages():
    output = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"], cwd=ROOT, check=True, stdout=subprocess.PIPE
    ).stdout
    return json.loads(output)["packages"]


def in_baseline(rev, package):
    """Whether the baseline has the crate: its manifest at the same path, with the same name."""
    manifest = os.path.relpath(package["manifest_path"], ROOT)
    shown = git("show", f"{rev}:{manifest}", check=False)
    if shown.returncode != 0:
        return False
    return f'name = "{package["name"]}"' in shown.stdout


def fetch():
    """Fetches the tag of the baseline release, or else its commit, into a shallow clone."""
    tag = f"v{BASELINE_VERSION}"
    if git("fetch", "--no-tags", "--depth=1", "origin", f"refs/tags/{tag}:refs/tags/{tag}", check=False).returncode == 0:
        return
    if git("cat-file", "-e", f"{BASELINE_COMMIT}^{{commit}}", check=False).returncode != 0:
        git("fetch", "--no-tags", "--depth=1", "origin", BASELINE_COMMIT)


def main(argv):
    if "--fetch" in argv:
        fetch()
    rev = baseline_rev()
    if git("cat-file", "-e", f"{rev}^{{commit}}", check=False).returncode != 0:
        print(
            f"error: the baseline {rev} is not in the clone; fetch it (git fetch origin {rev})",
            file=sys.stderr,
        )
        return 1
    published = [p for p in packages() if p["publish"] is None]
    version = {p["version"] for p in published}
    major_minor = ".".join(BASELINE_VERSION.split(".")[:2])
    if any(not v.startswith(major_minor + ".") for v in version):
        print(
            f"The workspace is at {', '.join(sorted(version))}, not at {major_minor}: the gate compares "
            f"within {major_minor} only. Set BASELINE_VERSION and BASELINE_COMMIT to the release of this "
            "minor version (docs/release.md, \"API stability\")."
        )
        return 1
    checked = sorted(p["name"] for p in published if in_baseline(rev, p))
    skipped = sorted(p["name"] for p in published if p["name"] not in checked)

    print(f"Baseline: {BASELINE_VERSION} ({rev})")
    print(f"Checked ({len(checked)}): {' '.join(checked)}")
    print(f"Not in the baseline ({len(skipped)}): {' '.join(skipped) or '-'}")
    if "--list" in argv:
        return 0

    command = [
        "cargo",
        "semver-checks",
        "--baseline-rev",
        rev,
        "--release-type",
        "minor",
        "--default-features",
    ]
    for name in checked:
        command += ["--package", name]
    sys.stdout.flush()
    process = subprocess.Popen(command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    output = []
    for line in process.stdout:
        sys.stdout.write(line)
        output.append(line)
    code = process.wait()
    output = "".join(output)
    failures = counted_failures(output)
    print()
    if failures:
        print(f"API gate: {len(failures)} breaking change(s) within {major_minor}:")
        for crate, lint, item in failures:
            print(f"  {crate}: {lint}: {item}")
        return 1
    if code != 0 and "Summary semver requires" not in output:
        print(f"API gate: cargo-semver-checks failed (exit {code}) without a semver result")
        return code
    print("API gate: no breaking change (the lints of the *VTable structs of the class model are not counted)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
