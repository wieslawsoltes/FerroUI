#!/usr/bin/env python3
"""The API gate of the signatures: cargo-semver-checks against the last release (docs/release.md,
"API stability").

Within a minor version (0.4) the public API of the published crates only grows. This script runs
`cargo semver-checks` for every published crate that the baseline release has, with the release
type `minor` (additions allowed, nothing removed or changed):

    semver_checks.py                 run the gate
    semver_checks.py --list          print the baseline and the crates it checks, run nothing
    semver_checks.py --fetch         fetch the baseline into a shallow clone first (CI)
    semver_checks.py --baseline-key  print the key of the cached baseline documentation (CI)
    semver_checks.py --baseline-only document the baseline into BASELINE_DOCS, check nothing (CI)

cargo-semver-checks compares the rustdoc JSON of a crate with that of the baseline. Left to itself
it builds both for each crate on its own, compiling the dependencies again for every one of the
published crates. The script instead documents all of them in one `cargo doc` (the dependencies are
checked once) for the sources of the pull request and for the baseline, which a worktree of the
baseline commit provides, and then runs cargo-semver-checks on the two JSON files of each crate.
Both sides are documented the same way, with the features cargo unifies for the published crates
together. The documentation of the baseline is kept in `BASELINE_DOCS` (target/api-gate/baseline),
which CI caches under the key `--baseline-key` prints (the baseline commit and the rustc version).

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
import shutil
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
# The rustdoc JSON of the pull request and of the baseline, and the worktree of the baseline.
GATE_DIR = os.path.join(ROOT, "target", "api-gate")
BASELINE_DOCS = os.path.join(GATE_DIR, "baseline")
CURRENT_DOCS = os.path.join(GATE_DIR, "current")
BASELINE_TREE = os.path.join(GATE_DIR, "baseline-tree")
FAILURE = re.compile(r"^--- (failure|warning) (\S+):")
PROGRESS = re.compile(r"^(Building|Built|Parsing|Parsed|Checking|Checked|Finished|Summary|Warning|Compiling|Updating|Downloading|Downloaded|Cloning|Resolving)\b")


def counted_failures(output, crate=None):
    """The failures of the output of cargo-semver-checks that count: (crate, lint, item)."""
    kind, lint, listing, failures = None, None, False, []
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


def rustc_version():
    return subprocess.run(["rustc", "-V"], check=True, stdout=subprocess.PIPE, text=True).stdout.strip()


def library_names(root, names):
    """The library target of each package of `names` in the workspace at `root`: package -> crate."""
    output = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"], cwd=root, check=True, stdout=subprocess.PIPE
    ).stdout
    libraries = {}
    for package in json.loads(output)["packages"]:
        if package["name"] not in names:
            continue
        for target in package["targets"]:
            if {"lib", "proc-macro", "rlib"} & set(target["kind"]):
                libraries[package["name"]] = target["name"].replace("-", "_")
    return libraries


def document(root, names, out_dir):
    """The rustdoc JSON of the libraries of `names` at `root`, in one cargo invocation, copied to
    `out_dir`: package -> path of its JSON."""
    target = os.path.join(GATE_DIR, "target-" + os.path.basename(out_dir))
    env = dict(os.environ)
    env["RUSTC_BOOTSTRAP"] = "1"
    env["RUSTDOCFLAGS"] = (env.get("RUSTDOCFLAGS", "") + " -Z unstable-options --output-format json").strip()
    env["CARGO_TARGET_DIR"] = target
    libraries = library_names(root, set(names))
    command = ["cargo", "doc", "--no-deps", "--lib"]
    for name in sorted(libraries):
        command += ["--package", name]
    print(f"Documenting {len(libraries)} crates in {root}", flush=True)
    subprocess.run(command, cwd=root, env=env, check=True)
    os.makedirs(out_dir, exist_ok=True)
    docs = {}
    for name, library in libraries.items():
        source = os.path.join(target, "doc", f"{library}.json")
        destination = os.path.join(out_dir, f"{name}.json")
        shutil.copyfile(source, destination)
        docs[name] = destination
    return docs


def baseline_docs(rev, names):
    """The rustdoc JSON of the baseline, from BASELINE_DOCS when it was made for this baseline and
    this rustc (the cache of CI), and otherwise documented in a worktree of the baseline."""
    stamp_path = os.path.join(BASELINE_DOCS, "STAMP")
    stamp = f"{rev} {rustc_version()}"
    if os.path.exists(stamp_path) and open(stamp_path).read().strip() == stamp:
        docs = {name: os.path.join(BASELINE_DOCS, f"{name}.json") for name in names}
        if all(os.path.exists(path) for path in docs.values()):
            print(f"Baseline documentation from {BASELINE_DOCS}")
            return docs
    shutil.rmtree(BASELINE_DOCS, ignore_errors=True)
    if os.path.exists(BASELINE_TREE):
        git("worktree", "remove", "--force", BASELINE_TREE, check=False)
        shutil.rmtree(BASELINE_TREE, ignore_errors=True)
    git("worktree", "add", "--detach", BASELINE_TREE, rev)
    try:
        docs = document(BASELINE_TREE, names, BASELINE_DOCS)
    finally:
        git("worktree", "remove", "--force", BASELINE_TREE, check=False)
    with open(stamp_path, "w") as file:
        file.write(stamp + "\n")
    return docs


def main(argv):
    if "--baseline-key" in argv:
        print(re.sub(r"[^A-Za-z0-9.-]+", "_", f"api-gate-baseline-{baseline_rev()}-{rustc_version()}").strip("_"))
        return 0
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

    baseline = baseline_docs(rev, checked)
    if "--baseline-only" in argv:
        return 0
    current = document(ROOT, checked, CURRENT_DOCS)
    failures, errors = [], []
    for name in checked:
        if name not in current or name not in baseline:
            continue
        command = [
            "cargo",
            "semver-checks",
            "check-release",
            "--baseline-rustdoc",
            baseline[name],
            "--current-rustdoc",
            current[name],
            "--release-type",
            "minor",
        ]
        print(f"Checking {name}", flush=True)
        result = subprocess.run(command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        crate_failures = counted_failures(result.stdout, name)
        if result.returncode != 0 and not crate_failures and "Summary semver requires" not in result.stdout:
            errors.append(name)
        if result.returncode != 0 or crate_failures:
            sys.stdout.write(result.stdout)
        failures += crate_failures
    print()
    if failures:
        print(f"API gate: {len(failures)} breaking change(s) within {major_minor}:")
        for crate, lint, item in failures:
            print(f"  {crate}: {lint}: {item}")
        return 1
    if errors:
        print(f"API gate: cargo-semver-checks failed without a semver result for {', '.join(errors)}")
        return 1
    print("API gate: no breaking change (the lints of the *VTable structs of the class model are not counted)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
