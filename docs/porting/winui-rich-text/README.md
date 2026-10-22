# WinUI rich text port

This is the source map, implementation record, and acceptance plan for bringing the
WinUI rich text document, layout, selection, and editing contracts to idiomatic Rust
and FerroUI. The implementation uses FerroUI's object/property system, document
inlines, text formatter, input services, and drawing API. The port is in progress;
an inventory entry or an existing similarly named FerroUI type is not a parity
claim.

## Reference

| Item | Pinned reference |
|---|---|
| Repository | [microsoft/microsoft-ui-xaml](https://github.com/microsoft/microsoft-ui-xaml) |
| Commit | [`fe244ca6ac4d482712b234a9de3cf771cc6481de`](https://github.com/microsoft/microsoft-ui-xaml/tree/fe244ca6ac4d482712b234a9de3cf771cc6481de) |
| Requested source | [`dxaml/xcp/core/text`](https://github.com/microsoft/microsoft-ui-xaml/tree/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text), all 230 tracked files |
| Editable controls and native services | [`dxaml/xcp/core/native/text`](https://github.com/microsoft/microsoft-ui-xaml/tree/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/native/text), all 62 tracked files |
| Highlighters and focused helpers | [`dxaml/xcp/components/text`](https://github.com/microsoft/microsoft-ui-xaml/tree/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/components/text), all 23 tracked files, including its isolated tests |
| License | Microsoft Corporation, MIT; the reference repository's `LICENSE` applies to source derived from it |

The general WinUI theme inventory has its own older reference commit. New
RichEditBox resources in WinUI, Fluent and Simple and the native text-command
source/header are tracked here at the rich-text reference. Completed general
theme-resource inventory does not establish rich text behavior or appearance
parity at this reference.

## Reading the record

- [PLAN.md](PLAN.md): dependency decisions, Rust/FerroUI mapping, work batches,
  and the acceptance requirements for each subsystem.
- [IMPLEMENTATION.md](IMPLEMENTATION.md): document, overflow, editor, RTF/clipboard,
  input-method and accessibility architecture with Rust and XAML examples.
- [TYPOGRAPHY.md](TYPOGRAPHY.md): all named typography properties, enum/feature
  projection, inherited snapshots, formatter integration and verification limits.
- [LAYOUT.md](LAYOUT.md): source-derived line stacking, shared TextLineBounds,
  linked-page ownership, allocation and draw-origin contracts.
- [INPUT-GEOMETRY.md](INPUT-GEOMETRY.md): exact requested UTF-16 range geometry,
  shaped fragments, native transforms and callback ownership.
- [RANGE-GRAVITY.md](RANGE-GRAVITY.md): independently measured range and live-selection
  formatting affinity, insertion/clipboard behavior and tracking boundaries.
- [LINK-FIELDS.md](LINK-FIELDS.md): common cross-paragraph field metadata, typed
  projected Link ranges, RTF/transfer preservation and measured limits.
- [API-MAP.md](API-MAP.md): a destination/review plan for every public API owner,
  with exact member, Rust target, test and deviation evidence for reviewed entries.
- [JOURNAL.md](JOURNAL.md): implementation/checkpoint history and the distinction
  between recovered published code and newly reconstructed work.
- [PROGRESS.md](PROGRESS.md): generated counts of the file and public API inventory.
- [TRACKING.md](TRACKING.md): every selected source file and its disposition.
- [DEPENDENCIES.md](DEPENDENCIES.md): available source, external implementation
  boundaries, and the interpretation of include-graph evidence.
- [DEVIATIONS.md](DEVIATIONS.md): deliberate translations and remaining behavior
  differences, identified separately.
- [TESTS.md](TESTS.md): upstream test families and recorded local verification.
- [inventory.json](inventory.json): pinned source hashes, direct includes,
  declaration candidates, dependency edges, and the complete source-file map.
- [api-members.json](api-members.json): exact documentation member identifiers
  for the selected public controls, document types, and all `Microsoft.UI.Text`
  members exposed by the reference's IntelliSense documentation. It also retains
  Text/TextRange, Text2, TextEdit, TextChild, text attributes, annotations, and the
  text-specific automation events/pattern dispatch members.
- [mapping.json](mapping.json): reviewed implementation, deviation, and test
  evidence used by the generator. This is the editable tracking input.

## Meaning of full port

Every owned source file and every documented public API member must have an
explicit disposition. A `complete` entry requires mapped Rust members, behavior
tests, and any remaining deliberate deviation to be recorded. An entire file
marked `pending` means that all of its members remain unverified. Existing
FerroUI APIs are candidates for reuse until their behavior is checked against the
corresponding WinUI contract. Generated wrappers, build files, metadata, and
upstream test harness files are tracked as reference inputs; counting them does
not count them as executable behavior delivered by this port.

The source repository contains the host of editable RichEdit and the adapters
around LineServices. It does not contain the implementation of `WinUIEdit.dll`,
the `Microsoft.UI.Text.winmd` producer, or the implementation behind
`Ptls-min.h`. Those boundaries require a semantic implementation using FerroUI
and separately verified contracts. Copying the host cannot supply their missing
behavior. The exact source evidence and remaining contracts are in
[DEPENDENCIES.md](DEPENDENCIES.md).

## Updating the record

Follow [../PORTING-GUIDE.md](../PORTING-GUIDE.md), including same-member behavior,
paired tests, safe Rust, weak parent references, shared collection handles,
declarative markup registration, and honest reporting. An upstream C++ header
and its implementation can map to one Rust module; each input file keeps its own
tracking entry. Reuse of a pre-existing FerroUI module is recorded explicitly.

```sh
python3 scripts/porting/winui-rich-text-inventory.py --reference /path/to/microsoft-ui-xaml
python3 scripts/porting/winui-rich-text-inventory.py --reference /path/to/microsoft-ui-xaml --check
```

The generator reads Git objects at the pinned commit, not the mutable checkout.
Hydrate required blobs before offline use; Git may fetch missing promised blobs
in a partial clone. The generator does not check out another revision or alter
the reference working tree. Regeneration preserves
reviewed entries in `mapping.json`; additions to the reference cannot silently
become `complete`. Commit the implementation, tests, mapping, deviations, and
regenerated record together.
