# WinUI Repeater port

This directory records the port of the WinUI Repeater family to native FerroUI
Rust controls, layouts and services. The implementation lives in
[`ferroui_controls_repeater`](../../../src/FerroUI.Controls.Repeater/lib.rs),
with the existing source-view and inclusive-selection-range implementations
reused rather than duplicated.

The source reference is
[`microsoft/microsoft-ui-xaml@7e4b0c2096be402f51cc8f7d203ebf67cff183d0`](https://github.com/microsoft/microsoft-ui-xaml/tree/7e4b0c2096be402f51cc8f7d203ebf67cff183d0/controls/dev/Repeater).
This is the derivation revision already recorded in the implementation's
[`NOTICE.md`](../../../src/FerroUI.Controls.Repeater/NOTICE.md). Moving WinUI
`main` does not silently change the acceptance target.
The port was made from `1af995b98a02d86b1951c4babe35caa269a6d569`, whose
`controls/dev/Repeater` tree is identical to that of `7e4b0c2`
(`git diff 1af995b9 7e4b0c2 -- controls/dev/Repeater` is empty); the project pins every
WinUI port to `7e4b0c2`, the revision of the WinUI theme (`docs/porting/winui-theme.md`).

On 2026-10-20, rechecked on 2026-10-21, the upstream GitHub Contents API
identified the same Repeater directory tree,
[`1de63a85615582e33d24e19795bd73d374195ac1`](https://api.github.com/repos/microsoft/microsoft-ui-xaml/git/trees/1de63a85615582e33d24e19795bd73d374195ac1),
at both this pinned revision and current `main`. The source subtree therefore
needed no pin update at that checkpoint. This comparison covers Repeater's
directory contents, not every transitive dependency in the WinUI repository.

The working review is [FerroUI PR #269](https://github.com/wieslawsoltes/FerroUI/pull/269),
on `feat/winui-repeater-port`. The journal and validation record distinguish
published implementation checkpoints from source mapping and executed evidence.

## Reading the record

| Document | Purpose |
|---|---|
| [DESIGN.md](DESIGN.md) | Native framework integration, ownership, layout, source mutation, selection, scheduling and extension rules |
| [SOURCES.md](SOURCES.md) | All 302 upstream files, including private helpers, IDL, test support, build inputs and sample assets |
| [API.md](API.md) | Every defined type/member in the five Repeater IDL files, including preview contracts and private test hooks |
| [API-SHAPES.md](API-SHAPES.md) | Exact Rust overload spellings, nullability, error boundaries, callbacks and important default/sentinel distinctions |
| [TESTS.md](TESTS.md) | All 265 upstream test declarations and their Rust scenario mappings |
| [DEPENDENCIES.md](DEPENDENCIES.md) | Every direct C++ include boundary outside the Repeater directory and its native replacement or documented platform omission |
| [DEVIATIONS.md](DEVIATIONS.md) | Deliberate Rust/framework adaptations and pinned-source corrections |
| [TRANSITION-LIFECYCLE.md](TRANSITION-LIFECYCLE.md) | Transition cancellation, control reuse, animation completion and scoped scrolling pins |
| [VALIDATION.md](VALIDATION.md) | Commands, actual execution evidence and the limits of that evidence |
| [REMAINING.md](REMAINING.md) | Acceptance work that a source or test-name inventory cannot establish |
| [JOURNAL.md](JOURNAL.md) | Recoverable checkpoints and implementation decisions |

The methodology follows the existing [FerroUI porting guide](../PORTING-GUIDE.md):
preserve observable behavior, explicitly map consolidated files, use the native
class/property system, retain attribution, and keep unported members visible.
The C++/WinRT source needs a separate extractor from the C# framework inventory;
the generated maps use the same distinction between upstream identity, Rust
location, deliberate replacement and executable evidence.

## Implemented areas

The control implementation contains item realization, automatic and explicitly
suppressed recycling, element preparation/clearing/index events, stable-key
resets, focus pinning, data-order keyboard navigation and an automation peer.
`ItemsRepeaterScrollHost` supplies the compatibility scrolling host; ordinary
FerroUI `ScrollViewer` integration uses effective viewport and anchor-provider
services. `LayoutPanel` hosts the same layout contracts over fixed children.

The layout family contains `Layout`, `VirtualizingLayout`,
`NonVirtualizingLayout`, both context adapters, `StackLayout`,
`UniformGridLayout`, `FlowLayout`, `LinedFlowLayout`, the shared flow algorithm,
element management and per-host state. The lined-flow implementation includes
metadata requests, bounded aspect-ratio history, row distribution, frozen
windows, public item locks and transition-provider creation.

The supporting implementation contains keyed/template recycling, extensible
factories, phased template binding, frame scheduling, transition batching and
completion, reduced-motion queries, diagnostic hooks, hierarchical index paths,
lazy hierarchical selection, read-only selected-item views and observable
selection-model properties. Runtime and compiled XAML use FerroUI metadata
rather than C++/WinRT activation.

These statements describe code present in the branch. The generated inventories
do **not** turn source presence into a claim of complete semantic, native
platform or visual parity. Each remaining acceptance item stays recorded.

The runnable native example is
[`ItemsRepeaterPage.xaml`](../../../samples/ControlCatalog/Pages/ItemsRepeaterPage.xaml)
with its [Rust page implementation](../../../samples/ControlCatalog/Pages/items_repeater_page.rs).
It switches the four layouts over a notifying 10,000-item collection, supplies
lined-flow metadata, and exposes insertion, removal, movement and reset.
The [compiled catalog tests](../../../samples/ControlCatalog/tests/items_repeater_page.rs)
exercise virtualization, scrolling, metadata constraints and handler lifetime;
the [browser regression](../../../scripts/browser/tests/repeater.test.mjs)
checks the rendered page and real wheel input in both browser rendering modes.

## Regenerating the maps

From the repository root, using Python 3.11 or later:

```sh
python3 scripts/winui-repeater/inventory.py
python3 scripts/winui-repeater/inventory.py --check
```

Normal generation and checking require no network, WinUI checkout, Windows SDK or
.NET SDK. The immutable extraction is committed as
[`data/upstream.json`](data/upstream.json). It contains each upstream file's
SHA-256 and size, every IDL declaration, test declaration and direct include edge.
[`data/mapping.json`](data/mapping.json) records explicit consolidation and native
service replacements; [`data/test-aliases.json`](data/test-aliases.json) records
reviewed scenario relationships. The generated machine-readable result is
[`data/status.json`](data/status.json).

To re-extract the pinned source:

```sh
python3 scripts/winui-repeater/inventory.py --extract /path/to/microsoft-ui-xaml
python3 scripts/winui-repeater/inventory.py --check
```

The input must be the pinned commit. A Git checkout's `HEAD` is verified. A
source archive can be obtained from the
[official pinned archive](https://codeload.github.com/microsoft/microsoft-ui-xaml/tar.gz/7e4b0c2096be402f51cc8f7d203ebf67cff183d0);
pass its extracted repository root. Extraction validates the pinned file and
scenario totals. Review the committed source hashes when deliberately changing
the extraction; a count alone is not a source integrity check.

## What the numbers mean

`implementation-mapped` means a C++ implementation has named Rust targets.
`framework-replacement` means an explicit native service replaces a WinRT/Windows
mechanism. `platform-diagnostic-omitted` records Windows-only instrumentation
that is intentionally not reproduced: the dependency map contains 89 native
replacements and one such omission, runtime-profiler class registration.
`symbol-found` means an identifier appears in its mapped source,
including macro declarations; overloads, argument validation and behavior still
need review and tests. `name-matched` means an existing Rust `#[test]` function
matches the upstream test name in the mapped suite. None is a test pass count.

The 265 test declarations include three commented-out and 27 ignored upstream
declarations. Ignored tests remain identified, including known product
concerns. Exact WinUI sample applications and their images are retained in the
inventory as references; the FerroUI catalog is a native demonstration, not a
claim that every WinUI sample page was ported.
