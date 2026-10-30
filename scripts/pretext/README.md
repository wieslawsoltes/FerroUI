# Pretext reference fixtures

This tooling produces deterministic behavioral fixtures from the unchanged
PretextSharp implementation. It is development tooling only: the shipped Rust
crates have no .NET, C#, Uno or native reference-harness dependency.

## Provenance

Source repository: <https://github.com/wieslawsoltes/PretextSharp>

Pinned source commit: `e70a544e43f7ecffee76d2d22a0224213d7fbba3`

The reference project compiles the core and contracts directly from that clean
checkout, together with the source's delegate measurement factory. The directory
name `tests/Pretext.Uno.Tests` is retained in the include path because that is
where the portable factory lives; no Uno project, package or application is
referenced. The harness uses the deterministic scalar measurement behavior of
the source parity tests, including contextual decimal advances, rather than
installed fonts.

The inputs are in `reference/cases.json`. The committed output is
`src/Pretext/FerroUI.Pretext/fixtures/reference.json`. It records preparation,
break metadata, glyph-independent widths, chunks, bidi levels, line counts,
materialized lines and incremental cursor results. `sourceCommit` identifies
the implementation that produced those expectations.

## Requirements

- Python 3.10 or later.
- Git.
- .NET 10 SDK, used only to regenerate or independently verify fixtures.
- A separate, unchanged checkout of the pinned source commit.

The runner never clones, downloads, edits or resets that checkout. Obtain it
explicitly, then select the recorded commit:

```sh
git clone https://github.com/wieslawsoltes/PretextSharp.git ../PretextSharp
git -C ../PretextSharp checkout e70a544e43f7ecffee76d2d22a0224213d7fbba3
```

## Verify existing expectations

From the FerroUI repository root:

```sh
python3 scripts/pretext/generate-reference.py --source ../PretextSharp --check
```

Use an explicit SDK executable if it is not on `PATH`:

```sh
python3 scripts/pretext/generate-reference.py \
  --source ../PretextSharp \
  --dotnet /path/to/dotnet \
  --check
```

The runner verifies `HEAD`, unstaged and staged changes with `git diff
--exit-code`, rejects untracked files, and confirms every compiled C# file is
tracked. It rechecks source cleanliness after execution. Build products and
regenerated JSON are temporary. `--check` compares deterministic JSON
canonicalizations so indentation and escaped-versus-literal Unicode do not
create false failures; every field, sequence position and numeric value still
participates. It leaves the checked fixture untouched.

Exit status is `0` for success, `1` for a differing fixture, and `2` for invalid
source, input, SDK or generation failures.

## Regenerate

After reviewing changes to the input cases:

```sh
python3 scripts/pretext/generate-reference.py --source ../PretextSharp
git diff -- src/Pretext/FerroUI.Pretext/fixtures/reference.json
```

The generated fixture is written atomically with UTF-8 text, two-space
indentation, and one terminal newline. `--cases` and `--output` select alternate
files for isolated experiments; do not replace expected results with values
computed by the Rust implementation under test.

Native ICU and Unicode data shipped with the .NET runtime or operating system
can affect locale-sensitive segmentation. Investigate any discrepancy and
record the runtime and OS before accepting a fixture update. Normal Rust test
runs consume the committed fixture and therefore do not require .NET.
