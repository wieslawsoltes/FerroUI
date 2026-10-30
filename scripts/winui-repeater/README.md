# WinUI Repeater inventory tooling

`inventory.py` extracts and maps the immutable Repeater reference used by the
native FerroUI port. Python 3.11+ and the standard library are sufficient.

```sh
python3 scripts/winui-repeater/inventory.py
python3 scripts/winui-repeater/inventory.py --check
python3 scripts/winui-repeater/inventory.py --extract /path/to/pinned/winui
```

Normal runs use the committed `docs/porting/winui-repeater/data/upstream.json`.
Extraction reads `controls/dev/Repeater` from the pinned checkout/archive,
retains SHA-256/size for every file, scans defined IDL types/members and direct
C++ include boundaries, and inventories C# `[TestMethod]` declarations. Three
commented-out upstream tests remain visible. Forward IDL declarations and
commented-out IDL members are excluded; overload signatures stay separate.

`mapping.json` explicitly records consolidated source families, reused native
services, type locations and dependency boundaries. `test-aliases.json` records
reviewed aliases with reasons. The generator rejects stale target paths and
aliases that name missing Rust `#[test]` functions. Generated Markdown and
`status.json` are deterministic and `--check` fails when they need regeneration.

`api_signature_overrides` keys include the full IDL signature. This distinguishes
native `*_with_options`, `*_with_owner` and `*_with_group` overload names from
the shorter source-name match. The output records the actual identifiers found
for each row; this remains a source-navigation check, not signature typechecking.

The scanner is intentionally a navigation/inventory tool, not a Rust/C++ semantic
equivalence checker. It does not run tests, prove module reachability, inspect
native platform output or certify full behavior. The IDL symbol check includes
macro declarations and preserves each overload as a separate row without
pretending that one matching method name proves all overloads. See the
[port record](../../docs/porting/winui-repeater/README.md) for review and execution
requirements.
