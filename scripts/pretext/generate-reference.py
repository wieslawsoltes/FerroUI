#!/usr/bin/env python3
"""Regenerate the Pretext reference fixtures from a pinned, clean C# checkout.

This development-only runner never downloads or modifies the source checkout.
It builds the unchanged upstream algorithms with the local .NET SDK and a
deterministic measurement backend; no Uno or native renderer is required.
"""

from __future__ import annotations

import argparse
import difflib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from typing import Any


SOURCE_COMMIT = "e70a544e43f7ecffee76d2d22a0224213d7fbba3"
SCRIPT_DIR = Path(__file__).resolve().parent
REPOSITORY_ROOT = SCRIPT_DIR.parents[1]
REFERENCE_PROJECT = SCRIPT_DIR / "reference" / "Reference.csproj"
DEFAULT_CASES = SCRIPT_DIR / "reference" / "cases.json"
DEFAULT_OUTPUT = (
    REPOSITORY_ROOT
    / "src"
    / "Pretext"
    / "FerroUI.Pretext"
    / "fixtures"
    / "reference.json"
)


class VerificationError(Exception):
    """A source or fixture does not satisfy the reference-generation contract."""


def git(source: Path, *arguments: str) -> str:
    completed = subprocess.run(
        ["git", "-C", str(source), *arguments],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return completed.stdout.strip()


def verify_source(source: Path) -> None:
    if not source.is_dir():
        raise VerificationError(f"source checkout does not exist: {source}")
    root = Path(git(source, "rev-parse", "--show-toplevel")).resolve()
    if root != source:
        raise VerificationError(f"--source must identify the checkout root: {root}")
    actual_commit = git(source, "rev-parse", "HEAD")
    if actual_commit != SOURCE_COMMIT:
        raise VerificationError(
            f"source must be at {SOURCE_COMMIT}; found {actual_commit}"
        )

    # Both unstaged and staged changes matter: the harness compiles working-tree
    # files, not blobs from the committed tree. Untracked source files also
    # change wildcard compilation, so require a clean nonignored worktree.
    git(source, "diff", "--exit-code", "--no-ext-diff")
    git(source, "diff", "--cached", "--exit-code", "--no-ext-diff")
    status = git(source, "status", "--porcelain", "--untracked-files=all")
    if status:
        raise VerificationError(
            "source checkout contains untracked files; use an unchanged checkout"
        )

    # Verify every file matched by the project's wildcard includes is tracked.
    # This also catches a locally ignored .cs file that status would omit.
    compiled = [
        *sorted((source / "src" / "Pretext.Contracts").glob("*.cs")),
        *sorted((source / "src" / "Pretext").glob("*.cs")),
        *sorted((source / "src" / "Pretext" / "Generated").glob("*.cs")),
        source
        / "tests"
        / "Pretext.Uno.Tests"
        / "DelegateTextMeasurerFactory.cs",
    ]
    if not compiled or any(not path.is_file() for path in compiled):
        raise VerificationError("required upstream C# source files are missing")
    for path in compiled:
        git(source, "ls-files", "--error-unmatch", "--", path.relative_to(source).as_posix())


def reject_nonfinite(value: str) -> Any:
    raise VerificationError(f"nonfinite JSON number in reference data: {value}")


def read_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as stream:
        return json.load(stream, parse_constant=reject_nonfinite)


def canonical_json(value: Any) -> str:
    # The exact field order comes from the reference harness. Canonicalizing
    # both sides removes differences in indentation and Unicode escaping while
    # retaining every field, array element and floating-point value.
    return json.dumps(value, ensure_ascii=False, allow_nan=False, indent=2) + "\n"


def validate_result(result: Any, cases: Any) -> None:
    if not isinstance(result, dict) or result.get("sourceCommit") != SOURCE_COMMIT:
        raise VerificationError("reference output has an unexpected sourceCommit")
    snapshots = result.get("cases")
    if not isinstance(snapshots, list) or len(snapshots) != len(cases):
        raise VerificationError("reference output does not contain every input case")
    for index, (snapshot, case) in enumerate(zip(snapshots, cases)):
        if not isinstance(snapshot, dict) or snapshot.get("input") != case:
            raise VerificationError(f"reference output input mismatch at case {index}")


def generate(source: Path, dotnet: str, cases_path: Path, output: Path, check: bool) -> int:
    verify_source(source)
    cases = read_json(cases_path)
    if not isinstance(cases, list) or any(not isinstance(case, dict) for case in cases):
        raise VerificationError("cases input must be a JSON array of objects")
    if not REFERENCE_PROJECT.is_file():
        raise VerificationError(f"reference project is missing: {REFERENCE_PROJECT}")

    with tempfile.TemporaryDirectory(prefix="pretext-reference-") as temporary:
        build_dir = Path(temporary)
        generated = build_dir / "reference.json"
        environment = os.environ.copy()
        environment.setdefault("DOTNET_SKIP_FIRST_TIME_EXPERIENCE", "1")
        environment.setdefault("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
        environment.setdefault("DOTNET_NOLOGO", "1")
        environment.setdefault("DOTNET_GENERATE_ASPNET_CERTIFICATE", "false")
        environment.setdefault("DOTNET_CLI_HOME", str(build_dir / "dotnet-home"))
        command = [
            dotnet,
            "run",
            "--project",
            str(REFERENCE_PROJECT),
            "--configuration",
            "Release",
            "--no-launch-profile",
            f"-p:PretextSource={source}",
            f"-p:BaseIntermediateOutputPath={build_dir / 'obj'}/",
            f"-p:BaseOutputPath={build_dir / 'bin'}/",
            "--",
            str(cases_path),
            str(generated),
        ]
        subprocess.run(command, check=True, cwd=REPOSITORY_ROOT, env=environment)
        # Recheck after running so concurrent edits cannot silently change the
        # claimed provenance of the generated fixture.
        verify_source(source)
        result = read_json(generated)

    validate_result(result, cases)
    actual_text = canonical_json(result)
    if check:
        if not output.is_file():
            raise VerificationError(f"checked fixture does not exist: {output}")
        expected_text = canonical_json(read_json(output))
        if actual_text != expected_text:
            differences = list(
                difflib.unified_diff(
                    expected_text.splitlines(),
                    actual_text.splitlines(),
                    fromfile=str(output),
                    tofile="regenerated reference",
                    lineterm="",
                )
            )
            print("\n".join(differences[:120]), file=sys.stderr)
            if len(differences) > 120:
                print(f"... {len(differences) - 120} additional diff lines", file=sys.stderr)
            print("Reference fixture differs; regenerate and review the change.", file=sys.stderr)
            return 1
        print(f"Verified {len(cases)} cases against unchanged source {SOURCE_COMMIT}.")
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    # Publish only a fully generated and validated fixture, replacing the file
    # atomically on the same filesystem to avoid interrupted partial JSON.
    temporary_output = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w", encoding="utf-8", newline="\n", prefix=f".{output.name}.",
            dir=output.parent, delete=False,
        ) as stream:
            temporary_output = Path(stream.name)
            stream.write(actual_text)
        temporary_output.replace(output)
    finally:
        if temporary_output is not None:
            temporary_output.unlink(missing_ok=True)
    print(f"Wrote {len(cases)} cases to {output}.")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True, help="clean checkout of the pinned PretextSharp source commit")
    parser.add_argument("--dotnet", default="dotnet", help=".NET 10 SDK executable (default: dotnet from PATH)")
    parser.add_argument("--cases", type=Path, default=DEFAULT_CASES, help="input cases JSON (default: reference/cases.json)")
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT, help="fixture to generate or check")
    parser.add_argument("--check", action="store_true", help="regenerate in a temporary directory and compare without editing the fixture")
    arguments = parser.parse_args()
    try:
        return generate(
            arguments.source.resolve(), arguments.dotnet, arguments.cases.resolve(),
            arguments.output.resolve(), arguments.check,
        )
    except (VerificationError, OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"Reference generation failed: {error}", file=sys.stderr)
        if isinstance(error, subprocess.CalledProcessError) and error.stderr:
            print(error.stderr.strip(), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
