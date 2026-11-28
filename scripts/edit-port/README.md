# Editor source traceability

This standard-library Python tool maintains the pinned source inventory for the editor and TextMate ports. It is deliberately separate from the framework port percentage: source correspondence does not establish member coverage or behavioral compatibility.

```sh
python3 scripts/edit-port/editor_port.py
python3 scripts/edit-port/editor_port.py --check
python3 scripts/edit-port/test_editor_port.py
```

Both generation and `--check` use committed JSON and local Rust sources. They do not require upstream checkouts, credentials, network access, or a .NET installation. Rust declarations are read by the existing `scripts/port-status/rustscan.py` scanner, which excludes comments, string literals, and test modules. It recognizes exact type names, aliases, functions, fields, variants, constants, and associated methods rather than substring matches. Macro expansion and conditional compilation are not executed; see the generated report for these limits.

## Inputs and outputs

- `docs/porting/data/editor-upstream.json`: every tracked source file under each pinned `src` root, its Git blob and SHA-256 hash, byte/line counts, category, and C# type declarations including nested types. AvaloniaEdit includes `.cs`, `.xaml`, and `.axaml`; TextMateSharp includes `.cs`. Demo/test/sample sources remain visible. Grammar/theme/resource assets have separate bundling provenance.
- `docs/porting/data/editor-mappings.json`: one explicit decision per captured file. Every declared type needs an exact alias or an explicit `unmapped_types` entry. A destination specifies a repository-relative `.rs` path and existing Rust symbols. Renamed and consolidated types use `adapted` status. `notes` explains the decision and `limitations` records known missing behavior.
- `docs/porting/data/editor-status.json` and `docs/porting/editor-source-map.md`: deterministic generated reports; `--check` fails if either differs, an input is missing, a pin changes, a mapping is incomplete, or a destination symbol disappears.

`mapped` means same-name type traceability, not a completed port. `adapted` means a concrete destination with changed names/structure or partial type coverage. `unmapped` means no verified destination. Never change a status solely to improve counts. Add the implementation and validation evidence, then update the explicit decision.

## Recapture the pinned inputs

Capture requires both local Git checkouts at the exact commits in `editor_port.py`. It reads Git blobs, so uncommitted checkout edits cannot alter the source hashes. It does not fetch, switch branches, or infer mappings.

```sh
python3 scripts/edit-port/editor_port.py --capture \
  --avaloniaedit-root /path/to/AvaloniaEdit \
  --textmatesharp-root /path/to/TextMateSharp
python3 scripts/edit-port/editor_port.py
```

When updating a pin, review the changed inventory, add explicit decisions for every new file/type, and review removed or renamed APIs. A capture intentionally leaves reports untouched until mappings have been reviewed. The parser inventories lexical declarations; it does not claim to replace a C# compiler, apply preprocessor symbols, or capture generated APIs.
