#!/usr/bin/env python3
"""Which jobs of the CI workflow a pull request needs (docs/ci.md).

A pull request runs a job of .github/workflows/ci.yml only when one of the files it changes is
compiled, tested or run by that job. Each job is described below by the packages it builds (its
roots, with their features and whether their tests compile the dev-dependencies) and the target it
builds them for. From the manifests of the workspace (`cargo metadata --no-deps`) the script works
out every package of the workspace the job compiles: the dependencies of the roots on paths, with
the optional ones that the features turn on and the platform-specific ones of the target. A changed
file belongs to the package whose directory holds it, and to every package whose sources name it
with a relative path (`#[path = "../..."]`, `include_str!`, test fixtures). A job runs when one of
those packages is in its set, or when a file matches one of its own paths (its scripts).

Everything runs, for every job, when:
- the event is not a pull request (a push to main, the nightly run, a run started by hand);
- the pull request changes the workflow, the actions, the root manifest, the lock file, the cargo
  configuration, this directory, the vendored crates, or a file outside every known area;
- the list of files is not complete (the API lists at most 3000) or the manifests cannot be read.

The documentation (docs/, Markdown) runs nothing, unless a package reads the file. The source
conventions job always runs: it is cheap and every change is checked by it.

    scripts/ci/changed_areas.py --event pull_request --files changed.txt [--output $GITHUB_OUTPUT]
    git diff --name-only origin/main... | scripts/ci/changed_areas.py --files -
"""

import argparse
import fnmatch
import json
import os
import re
import subprocess
import sys
from collections import defaultdict

JOBS = ("api-gate", "linux", "macos", "windows", "browser", "x11")

# Paths that run every job: what every job builds with.
FULL = (
    ".github/workflows/ci.yml",
    ".github/actions/**",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain",
    "rust-toolchain.toml",
    ".cargo/**",
    "scripts/ci/**",
    "external/**",
)

# Paths that run no job of ci.yml (beyond the conventions): the documentation, the other
# workflows (which have path filters of their own), crates outside the workspace that no job of
# ci.yml builds, and the scripts no job runs.
NOTHING = (
    "docs/**",
    "*.md",
    "**/*.md",
    "LICENSE",
    "NOTICE*",
    ".gitignore",
    ".github/FUNDING.yml",
    ".github/workflows/*",
    "benchmarks/comparison/**",
    "tests/FerroUI.Text.Comparison/**",
    "tests/FerroUI.TreeDataGrid.GpuiComparison/**",
)

# The scripts and other files outside the packages that a job runs itself.
JOB_PATHS = {
    "api-gate": ("scripts/api-gate/**",),
    "linux": ("components/**",),
    "macos": (),
    "windows": (
        "scripts/windows/**",
        "scripts/winui/**",
        "scripts/winui-theme/**",
        "scripts/text-quality.py",
    ),
    "browser": ("scripts/browser/**", "scripts/build-browser.sh"),
    "x11": ("scripts/x11/**",),
}

# Every other script runs in the conventions job or in a workflow of its own.
SCRIPTS = ("scripts/**",)

# The API gate compares the published crates with the baseline: it runs for a change of the
# sources or of a manifest of the workspace (the rule of the gate itself, scripts/api-gate).
API_GATE = ("src/**",)

LINUX = {"family": {"unix"}, "target_os": {"linux"}, "target_arch": {"x86_64"},
         "target_vendor": {"unknown"}, "target_env": {"gnu"}, "triple": "x86_64-unknown-linux-gnu"}
MACOS = {"family": {"unix"}, "target_os": {"macos"}, "target_arch": {"aarch64"},
         "target_vendor": {"apple"}, "target_env": {""}, "triple": "aarch64-apple-darwin"}
WINDOWS = {"family": {"windows"}, "target_os": {"windows"}, "target_arch": {"x86_64"},
           "target_vendor": {"pc"}, "target_env": {"msvc"}, "triple": "x86_64-pc-windows-msvc"}
EMSCRIPTEN = {"family": {"unix", "wasm"}, "target_os": {"emscripten"}, "target_arch": {"wasm32"},
              "target_vendor": {"unknown"}, "target_env": {""}, "triple": "wasm32-unknown-emscripten"}


def root(package, dev=True, features=(), default=True):
    return {"package": package, "dev": dev, "features": tuple(features), "default": default}


# What each job builds on a pull request (the steps of the job in ci.yml). `dev`: the job builds
# tests or examples of the package, which compile its dev-dependencies.
TARGETS = {
    "linux": {
        "platform": LINUX,
        # `--workspace --lib --tests` without these packages (still built where another needs them).
        "workspace": True,
        "exclude": ("ferroui-render-backends-comparison", "ferroui-vello",
                    "ferroui-benchmarks", "ferroui-treedatagrid-benchmarks"),
        "roots": [root("ferroui-markup-xaml")],
    },
    "macos": {
        "platform": MACOS,
        "roots": [
            root("ferroui-native"), root("ferroui-metal"), root("ferroui-desktop"),
            root("ferroui-skia"), root("virtualization-demo"), root("ferroui-hot-reload"),
            root("control-catalog-desktop", dev=False),
        ],
    },
    "windows": {
        "platform": WINDOWS,
        "roots": [
            root("ferroui-win32"), root("ferroui-desktop"),
            root("hello-world", dev=False, features=("direct2d",)),
            root("ferroui-integration-tests-win32"),
            root("control-catalog-desktop", dev=False),
            root("ferroui-winui"),
            root("control-gallery-winui", dev=False), root("winui-embed-sample", dev=False),
            # The long steps of the Windows sources (their own rule in the job, which runs them
            # when the Windows sources change) build these too.
            root("hello-world-builders", dev=False), root("hello-world-macros", dev=False),
            root("hello-world-hooks", dev=False), root("hello-world-component-host", dev=False),
            root("integration-test-app", dev=False), root("ferroui-integration-tests-appium"),
            root("gpu-interop", dev=False), root("winui-theme-gallery", dev=False),
        ],
    },
    "browser": {
        "platform": EMSCRIPTEN,
        "roots": [root(name, dev=False) for name in (
            "ferroui-base", "ferroui-controls", "ferroui-markup", "ferroui-markup-xaml",
            "ferroui-markup-xaml-loader", "ferroui-opengl", "ferroui-skia", "ferroui-harfbuzz",
            "ferroui-fonts-inter", "ferroui-themes-simple", "ferroui-themes-fluent", "mini-mvvm",
            "control-catalog", "control-catalog-browser",
        )] + [root("ferroui-browser")],
    },
    "x11": {
        "platform": LINUX,
        "roots": [
            root("ferroui-x11", features=("example",)), root("ferroui-vulkan"), root("ferroui-skia"),
            root("ferroui-freedesktop"), root("ferroui-desktop"),
            root("control-catalog-desktop", dev=False, features=("diagnostics",)),
            root("gpu-interop"),
        ],
    },
}


def matches(path, patterns):
    for pattern in patterns:
        if pattern.endswith("/**"):
            if path.startswith(pattern[:-2]):
                return True
        elif pattern.startswith("**/"):
            if fnmatch.fnmatchcase(os.path.basename(path), pattern[3:]):
                return True
        elif "/" not in pattern and "/" not in path and fnmatch.fnmatchcase(path, pattern):
            return True
        elif fnmatch.fnmatchcase(path, pattern) and path.count("/") == pattern.count("/"):
            return True
    return False


# The `cfg(...)` of a target-specific dependency: True, False, or None when the expression names
# something the script does not know (a custom cfg), which keeps the dependency.
def evaluate_cfg(expression, platform):
    if not expression.startswith("cfg("):
        return expression == platform["triple"]
    tokens = re.findall(r'[A-Za-z_][A-Za-z0-9_]*|"[^"]*"|[(),=]', expression[4:-1])
    position = 0

    def parse():
        nonlocal position
        name = tokens[position]
        position += 1
        if position < len(tokens) and tokens[position] == "(":
            position += 1
            values = []
            while tokens[position] != ")":
                values.append(parse())
                if tokens[position] == ",":
                    position += 1
            position += 1
            if name == "not":
                return None if values[0] is None else not values[0]
            if name == "any":
                return True if True in values else (None if None in values else False)
            if name == "all":
                return False if False in values else (None if None in values else True)
            return None
        if position < len(tokens) and tokens[position] == "=":
            value = tokens[position + 1].strip('"')
            position += 2
            if name == "target_family":
                return value in platform["family"]
            if name in platform:
                return value in platform[name]
            return None
        if name in ("unix", "windows"):
            return name in platform["family"]
        return None

    return parse()


class Workspace:
    def __init__(self, metadata):
        self.root = metadata["workspace_root"].rstrip("/") + "/"
        self.packages = {package["name"]: package for package in metadata["packages"]}
        # The directory of each package and of each of its targets outside it.
        self.directories = []
        for package in metadata["packages"]:
            directories = {os.path.dirname(package["manifest_path"])}
            directories |= {os.path.dirname(target["src_path"]) for target in package.get("targets", [])}
            for directory in directories:
                relative = os.path.relpath(directory, self.root)
                if relative != "." and not relative.startswith(".."):
                    self.directories.append((relative + "/", package["name"]))
        self.directories.sort(key=lambda entry: len(entry[0]), reverse=True)
        self.references = set()

    def owner(self, path):
        for directory, name in self.directories:
            if path.startswith(directory):
                return name
        return None

    def scan_references(self, files):
        """The files outside its directory that a package names: (package, path, tests) triples.

        A string with a slash that is the path of a file or a directory of the repository, from
        the file that holds it, from its package or from the root of the repository
        (`include_str!("../x")`, `#[path = "../x.rs"]`, `root.join("samples/x.xaml")`). The root
        and the top-level directories are left out: a test names them to join a path to them.
        `tests`: the name is in a test or a benchmark of the package (its `tests/` or `benches/`,
        a `testing/` module, a `*_tests.rs` file), so the file changes the tests of the package,
        not what it builds.
        """
        literal = re.compile(r'"/?([A-Za-z0-9_.\-]*/[A-Za-z0-9_.\-/]*)"')
        for source in files:
            reader = self.owner(source)
            if reader is None:
                continue
            try:
                with open(os.path.join(self.root, source), encoding="utf-8", errors="replace") as handle:
                    text = handle.read()
            except OSError:
                continue
            package_directory = os.path.dirname(os.path.relpath(self.packages[reader]["manifest_path"], self.root))
            parts = os.path.relpath(source, package_directory).split("/")
            tests = parts[0] in ("tests", "benches") or "testing" in parts[:-1] \
                or re.search(r"(^|_)tests?\.rs$", parts[-1]) is not None
            for value in set(literal.findall(text)):
                for base in (os.path.dirname(source), package_directory, ""):
                    target = os.path.normpath(os.path.join(base, value))
                    absolute = os.path.join(self.root, target)
                    if target.startswith(("..", "/")) or "/" not in target or not os.path.exists(absolute):
                        continue
                    if self.owner(target + "/" if os.path.isdir(absolute) else target) != reader:
                        self.references.add((reader, target, tests))
                    break

    def readers(self, path, tests):
        """The packages that name the file: in what they build, or (`tests`) in their tests."""
        return {reader for reader, target, in_tests in self.references
                if in_tests == tests and (path == target or path.startswith(target + "/"))}

    def compiled(self, description):
        """The packages of the workspace a job compiles, and those of them whose tests it builds."""
        platform = description["platform"]
        roots = list(description.get("roots", []))
        if description.get("workspace"):
            excluded = set(description.get("exclude", ()))
            roots += [root(name) for name in self.packages if name not in excluded]
        # The features of each package compiled (unified, as cargo unifies them), whether its
        # default features are on, and whether its dev-dependencies are compiled.
        features, defaults, dev = {}, set(), set()
        pending = []

        def request(name, wanted, default, with_dev):
            if name not in self.packages:
                return
            grows = name not in features or not wanted <= features[name]
            grows |= (default and name not in defaults) or (with_dev and name not in dev)
            features.setdefault(name, set()).update(wanted)
            if default:
                defaults.add(name)
            if with_dev:
                dev.add(name)
            if grows:
                pending.append(name)

        for entry in roots:
            request(entry["package"], set(entry["features"]), entry["default"], entry["dev"])
        while pending:
            name = pending.pop()
            package = self.packages[name]
            _, optional, dependency_features = self.expand(package, features[name], name in defaults)
            for dependency in package["dependencies"]:
                if dependency["name"] not in self.packages or not dependency.get("path"):
                    continue
                kind = dependency["kind"]
                if kind == "dev" and name not in dev:
                    continue
                target = dependency.get("target")
                if target:
                    active = evaluate_cfg(target, platform)
                    if kind == "build" and active is False:
                        active = evaluate_cfg(target, LINUX)
                    if active is False:
                        continue
                alias = dependency.get("rename") or dependency["name"]
                if dependency["optional"] and alias not in optional:
                    continue
                wanted = set(dependency["features"]) | dependency_features.get(alias, set())
                request(dependency["name"], wanted, dependency["uses_default_features"], False)
        return set(features), dev

    @staticmethod
    def expand(package, requested, default):
        table = package.get("features", {})
        queue = list(requested) + (["default"] if default and "default" in table else [])
        enabled, optional = set(), set()
        dependency_features = defaultdict(set)
        while queue:
            feature = queue.pop()
            if feature in enabled:
                continue
            enabled.add(feature)
            if feature not in table:
                optional.add(feature)
                continue
            for item in table[feature]:
                if item.startswith("dep:"):
                    optional.add(item[4:])
                elif "/" in item:
                    dependency, dependency_feature = item.split("/", 1)
                    if not dependency.endswith("?"):
                        optional.add(dependency)
                    dependency_features[dependency.rstrip("?")].add(dependency_feature)
                else:
                    queue.append(item)
        return enabled, optional, dependency_features


def decide(files, event, workspace, complete=True):
    """The jobs to run and why: {job: reason or None}."""
    reasons = {job: None for job in JOBS}

    def run_all(reason):
        return {job: reason for job in JOBS}

    if event != "pull_request":
        return run_all(f"a {event} run runs every job")
    if not complete:
        return run_all("the list of changed files is not complete")
    if workspace is None:
        return run_all("the manifests of the workspace could not be read")
    compiled = {job: workspace.compiled(TARGETS[job]) for job in TARGETS}
    for path in files:
        if matches(path, FULL):
            return run_all(f"{path} is used by every job")
        packages = workspace.readers(path, tests=False)
        owner = workspace.owner(path)
        if owner is not None:
            packages.add(owner)
        tested = workspace.readers(path, tests=True)
        if matches(path, API_GATE) and reasons["api-gate"] is None:
            reasons["api-gate"] = f"{path} (a source of the workspace)"
        for job, patterns in JOB_PATHS.items():
            if reasons[job] is None and matches(path, patterns):
                reasons[job] = f"{path} (a script of the job)"
        for job, (names, with_tests) in compiled.items():
            hit = sorted(packages & names)
            test_hit = sorted(tested & with_tests)
            if reasons[job] is None and hit:
                reasons[job] = f"{path} (package {hit[0]})"
            elif reasons[job] is None and test_hit:
                reasons[job] = f"{path} (the tests of {test_hit[0]})"
        if packages or tested or matches(path, NOTHING) or matches(path, SCRIPTS):
            continue
        if any(matches(path, patterns) for patterns in JOB_PATHS.values()):
            continue
        return run_all(f"{path} is outside every known area")
    return reasons


def load_workspace(metadata_file, root):
    try:
        if metadata_file:
            with open(metadata_file, encoding="utf-8") as handle:
                metadata = json.load(handle)
        else:
            output = subprocess.run(
                [os.environ.get("CARGO", "cargo"), "metadata", "--no-deps", "--format-version", "1"],
                cwd=root, check=True, capture_output=True, text=True).stdout
            metadata = json.loads(output)
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"::warning::cargo metadata failed ({error}): every job runs", file=sys.stderr)
        return None
    workspace = Workspace(metadata)
    tracked = subprocess.run(
        ["git", "ls-files", "--", "*.rs", "*.toml", "*.json", "*.mjs", "*.js", "*.ts"],
        cwd=workspace.root, capture_output=True, text=True).stdout.splitlines()
    workspace.scan_references(tracked)
    return workspace


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--files", required=True, help="the changed files, one per line ('-': standard input)")
    parser.add_argument("--event", default="pull_request", help="the event of the run (github.event_name)")
    parser.add_argument("--incomplete", action="store_true", help="the list of files is not complete")
    parser.add_argument("--metadata", help="the output of `cargo metadata --no-deps` (default: run it)")
    parser.add_argument("--output", help="the file of the step outputs ($GITHUB_OUTPUT)")
    parser.add_argument("--summary", help="the file of the step summary ($GITHUB_STEP_SUMMARY)")
    arguments = parser.parse_args()

    handle = sys.stdin if arguments.files == "-" else open(arguments.files, encoding="utf-8")
    files = sorted({line.strip() for line in handle if line.strip()})
    root = subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True).stdout.strip()
    workspace = load_workspace(arguments.metadata, root or ".") if arguments.event == "pull_request" else None
    reasons = decide(files, arguments.event, workspace, complete=not arguments.incomplete)

    lines = [f"{job}={'true' if reason else 'false'}" for job, reason in reasons.items()]
    print("\n".join(lines))
    if arguments.output:
        with open(arguments.output, "a", encoding="utf-8") as output:
            output.write("\n".join(lines) + "\n")
    table = ["| Job | Runs | Because of |", "| --- | --- | --- |"]
    table += [f"| {job} | {'yes' if reason else 'no'} | {reason or 'no changed file is built by it'} |"
              for job, reason in reasons.items()]
    print("\n".join(table), file=sys.stderr)
    if arguments.summary:
        with open(arguments.summary, "a", encoding="utf-8") as summary:
            summary.write(f"### Jobs of this run ({len(files)} changed files)\n\n" + "\n".join(table) + "\n")


if __name__ == "__main__":
    main()
