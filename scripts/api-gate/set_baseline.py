#!/usr/bin/env python3
"""Moves the baseline of the API gate (`semver_checks.py`) to a release (#423 C6, docs/release.md,
"API stability").

The release that starts a minor version moves the baseline to itself. This rewrites the three
places of `semver_checks.py` that name it, so that the change is the same every time:

    set_baseline.py 0.6.0                  the commit of the tag v0.6.0 of this clone
    set_baseline.py 0.6.0 --commit <sha>   a commit before the tag exists (the commit of main the
                                           release pull request was made from)
    set_baseline.py --check                the baseline commit is the commit of its tag, when the
                                           clone has the tag

Only the standard library and `git`.
"""

import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
GATE = os.path.join(ROOT, "scripts", "api-gate", "semver_checks.py")

VERSION = re.compile(r'^BASELINE_VERSION = "([^"]+)"$', re.M)
COMMIT = re.compile(r'^BASELINE_COMMIT = "([0-9a-f]{40})"$', re.M)
COMMENT = re.compile(r"^# The release the gate compares with, and the commit of its sources: .*$", re.M)


def git(*args):
    return subprocess.run(["git", "-C", ROOT, *args], capture_output=True, text=True, check=False)


def tag_commit(version):
    """The commit of the tag of `version`, or None when the clone does not have it."""
    found = git("rev-parse", "--verify", "--quiet", f"v{version}^{{commit}}")
    return found.stdout.strip() if found.returncode == 0 else None


def commit_date(commit):
    found = git("show", "-s", "--format=%cs", commit)
    return found.stdout.strip() if found.returncode == 0 else None


def rewrite(text, version, commit, date):
    """`text` of the gate with the baseline `version` at `commit`; raises if a place is missing."""
    for pattern, name in ((VERSION, "BASELINE_VERSION"), (COMMIT, "BASELINE_COMMIT"), (COMMENT, "its comment")):
        if len(pattern.findall(text)) != 1:
            raise ValueError(f"{GATE} does not have exactly one {name}")
    when = f" ({date})" if date else ""
    text = COMMENT.sub(
        f"# The release the gate compares with, and the commit of its sources: the tag v{version}{when}.", text
    )
    text = VERSION.sub(f'BASELINE_VERSION = "{version}"', text)
    return COMMIT.sub(f'BASELINE_COMMIT = "{commit}"', text)


def main(argv):
    with open(GATE, encoding="utf-8") as handle:
        text = handle.read()
    if argv == ["--check"]:
        version = VERSION.search(text).group(1)
        commit = COMMIT.search(text).group(1)
        tagged = tag_commit(version)
        if tagged is None:
            print(f"The clone has no tag v{version}: nothing to compare {commit} with.")
            return 0
        if tagged != commit:
            print(f"BASELINE_COMMIT is {commit}, the tag v{version} is {tagged}: run set_baseline.py {version}.")
            return 1
        print(f"The baseline {version} is the commit of its tag ({commit}).")
        return 0
    if len(argv) not in (1, 3) or (len(argv) == 3 and argv[1] != "--commit"):
        print(__doc__)
        return 2
    version = argv[0]
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        print(f"{version} is not a release version (major.minor.patch).")
        return 2
    commit = argv[2] if len(argv) == 3 else tag_commit(version)
    if commit is None:
        print(f"The clone has no tag v{version}: pass --commit with the commit of the release.")
        return 1
    resolved = git("rev-parse", "--verify", "--quiet", f"{commit}^{{commit}}")
    if resolved.returncode != 0:
        print(f"{commit} is not a commit of this clone.")
        return 1
    commit = resolved.stdout.strip()
    with open(GATE, "w", encoding="utf-8") as handle:
        handle.write(rewrite(text, version, commit, commit_date(commit)))
    print(f"The API gate compares with {version} ({commit}).")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
