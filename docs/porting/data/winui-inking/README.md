# Inking source and compatibility ledger

Baseline: Microsoft WinUI
`7e4b0c2096be402f51cc8f7d203ebf67cff183d0` (2026-10-21).

| File | What it records |
|---|---|
| `source-map.json` | Every file under `controls/dev/InkCanvas` and `controls/dev/InkToolbar`: path, immutable Git blob SHA, byte size, category, disposition and Rust mapping. The pinned inventory contains 164 files. |
| `upstream/*.idl` | Byte-for-byte copies of all six IDLs in those directories. Their Git blob hashes are checked against the source inventory. |
| `api-map.json` | Every public type and declared member in those IDLs, including enum values and dependency-property registrations: 45 types and 181 members. Each member has its own disposition. |
| `tests-map.json` | The 65 methods in the two upstream API-test suites and eight methods in the interaction suites, with explicit local test mappings. Source UI harness adaptations are explained separately from the original behavioral assertions. |
| `upstream/*.cs` | Exact snapshots of those four test suites, checked against their original Git blobs and used to detect omitted scenario mappings. |
| `dependencies.json` | OS services used by the source, portable replacements, compatibility boundaries and adjacent Windows Ink services. These are not presented as source files in WinUI. |
| `service-api-map.json` | 136 methods/getters/setters across 12 selected Windows Ink dependency classes, pinned to the `windows` 0.62.2 projection. Each records whether WinUI actually invokes it, exposes the containing service, or does not require the optional API. |
| `upstream/windows-0.62.2-inking-services.txt` | Immutable declaration extract for those external classes, with the original projection revision and source digest. This is separate from the six WinUI IDLs. |

## Status meanings

- `implemented`: the mapped contract has an implementation with no known gap at
  that record's scope. This is not a claim that the entire control or OS engine
  is equivalent.
- `adapted`: the behavior is expressed through a documented FerroUI/Rust
  equivalent. Ownership, framework types or invocation style can differ.
- `partial`: mapped code exists, with a specific remaining semantic or UI gap.
- `not_ported`: no equivalent is claimed; the reason or remaining work is explicit.
- `not_applicable`: build/language scaffolding replaced by the existing toolchain.
- `external`: the implementation belongs to an OS dependency outside the pinned
  WinUI source. Its replacement and validation boundary are explained.

A source can be `partial` while several of its public members are `adapted`.
For example, mapping a stroke container's in-memory selection does not establish
its ISF persistence or native clipboard interoperability. A constructor mapping
does not establish its template, accessibility patterns or localization.

Run from the repository root:

```sh
python scripts/inking/check_port.py
python scripts/inking/check_port.py --json
python scripts/inking/check_port.py --require-control-complete
python scripts/inking/check_port.py --require-complete
```

The normal check validates ledger structure, immutable IDL/test-suite hashes,
exhaustive type/member/test accounting, the external service declaration extract
and local evidence paths/symbols. It deliberately does not infer behavior from symbol
names. `--require-control-complete` rejects gaps in the pinned control sources,
IDLs, tests, required dependencies and directly invoked external methods.
`--require-complete` additionally includes the broader dependency API and optional
extension audit. The JSON report contains every remaining record and identifies
which gate includes it. Test execution, platform
interoperability and visual comparisons are separate evidence described in
[`../../winui-inking.md`](../../winui-inking.md).

## External service scope

The 181 WinUI declarations reference external OS objects; they do not declare
the whole Windows Ink namespace. The separate dependency audit makes those
boundaries visible without silently adding all adjacent Windows interfaces to
the required control port. For example, WinUI invokes the stroke container's
save/load/selection methods and drawing-attribute setters. It does not invoke
`InkModelerAttributes`, recognition-state APIs or `InkStroke.PointerId`.

The external records use `control_role`:

- `required_by_controls`: an actual call in the pinned control implementation,
  with the calling file recorded in `upstream_references`.
- `returned_service_api`: additional public API on an object exposed by the
  control, or a nested property. Missing members limit broader service parity.
- `portable_engine_support`: constructors, samples and stroke-building APIs
  that support the independent portable renderer.
- `optional_extension`: advanced/nested or adjacent OS services that the pinned
  controls do not require, including the optional native recognition extension.

Neither gate represents a percentage of functionality. Source, type, member,
test and service records intentionally overlap, and passing the structural
check does not establish the behavior of any of them.

## Updating a record

1. Read the pinned upstream implementation and matching tests.
2. Add the implementation and meaningful regression tests using FerroUI APIs.
3. Record the target file, exact API symbol, behavior change and remaining gap.
4. Link a test only if it checks the upstream scenario; construction alone is
   insufficient for a test about layout, clipping or accessibility.
5. Run the ledger checker and the relevant Rust tests, then update the human
   report with the actual command and result.

Do not change the pinned inventory or snapshots to make an omitted member
disappear. A new upstream baseline needs a deliberate inventory/API comparison.

## License

The IDL/test snapshots and WinUI-derived declarations are Copyright (c) Microsoft
Corporation. All rights reserved, under the MIT License. The Windows dependency
declaration extract is from Microsoft Rust for Windows, MIT OR Apache-2.0; its
MIT notice is retained with the other Microsoft attribution. The full notice is in
[`../../../../src/FerroUI.Controls.Inking/NOTICE.md`](../../../../src/FerroUI.Controls.Inking/NOTICE.md).
