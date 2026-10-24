#!/usr/bin/env python3
"""The API gate of the signatures: cargo-semver-checks against the last release (docs/release.md,
"API stability").

Within a minor version (0.4) the public API of the published crates only grows. This script runs
`cargo semver-checks` for every published crate that the baseline release has, with the release
type `minor` (additions allowed, nothing removed or changed) and the default features:

    semver_checks.py            run the gate
    semver_checks.py --list     print the baseline and the crates it checks, run nothing
    semver_checks.py --fetch    fetch the baseline into a shallow clone first (CI)

The baseline is the tag of the release (`v0.4.0`) when the repository has it, and otherwise the
commit that set the workspace to that version: 0.4.0 has no tag and is not on crates.io yet, and
the sources of a release are that commit. A crate the baseline does not have (one that joined
after the release) is not checked. The defaults of properties, which no signature shows, are
guarded by the snapshot test `property_defaults_tests` of `ferroui-controls`.

Only the standard library, `git`, and the `cargo` and `cargo-semver-checks` on PATH.
"""

import json
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# The release the gate compares with, and the commit of its sources. Updated by the release that
# starts the next minor version (docs/release.md, "API stability").
BASELINE_VERSION = "0.4.0"
BASELINE_COMMIT = "2cdce0d4e7750259d252885bde6ffbd27ef5c591"


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
    return subprocess.run(command, cwd=ROOT).returncode


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
