# WinUI / Gallery API and port-state scanner

This scanner keeps upstream source accounting, namespace/member candidates, and
behavioral port evidence separate. A same-name type or method **never** counts as
functional parity. The checked-in source snapshots work without network access,
.NET, Rust, Windows or the upstream checkouts.

```sh
# Regenerate candidate maps and port-state reports from current Rust sources.
python3 scripts/winui-gallery/scan.py

# Offline CI: validate pinned snapshot hashes, evidence anchors and exact reports.
python3 -m unittest discover -s scripts/winui-gallery -p 'test_*.py'
python3 scripts/winui-gallery/scan.py --check
python3 scripts/winui-gallery/event_targets.py --check
python3 scripts/winui-gallery/symbol_map.py --check
python3 scripts/winui-gallery/existing_api.py --check
python3 scripts/generate_view_builders.py --check
python3 scripts/generate_markup_types.py --target rust-paths --check

# Deliberately fails while API contracts or Gallery pages lack execution evidence.
python3 scripts/winui-gallery/scan.py --check --require-complete
```

## Source refresh

`docs/porting/winui-gallery/data/sources.lock.json` names exact commits of
`microsoft/microsoft-ui-xaml` and `microsoft/WinUI-Gallery`. Prepare clean checkouts
at those commits, then regenerate all immutable reference snapshots:

```sh
python3 scripts/winui-gallery/scan.py --refresh-reference \
    --winui-root ../upstream-winui --gallery-root ../upstream-gallery
```

A mismatched HEAD or modified tracked upstream file fails the refresh. Updating
the pins is a reviewed change: update both commit fields, refresh, inspect the
source-file and API deltas, then commit the new lock and snapshots together. The
regular offline scan verifies every snapshot against its SHA-256 in the lock.

## Outputs

All generated outputs live in `docs/porting/winui-gallery/data/`.

| File | Meaning |
|---|---|
| `sources.lock.json` | Upstream commit pins and immutable snapshot hashes |
| `ferroui-api-baseline.json` / `ferroui-api-baseline.lock.json` | Independently pinned pre-WinUI FerroUI declaration surface; protects existing namespaced types, bases, member names and source registrations |
| `source-files.json` | SHA-256, byte count and repository for every selected API/Gallery input |
| `upstream-api.json` | Namespaced MIDL 3 and C# API-model types, bases, members, signatures and source locations |
| `gallery-pages.json` | Every upstream catalog page, code-behind, example and page-directory snippet |
| `gallery-usage.json` | Every Gallery XAML file's expanded type/member names and markup features; C# imports and explicitly qualified references |
| `api-mappings.json` | Reviewed adaptation candidates, member renames and semantic cautions; hand-maintained input |
| `symbol-glyphs.tsv` / `symbol-sources.lock.json` | Every pinned Symbol API value and modern rendered glyph, with API-model and native-renderer hashes |
| `api-evidence.json` | Contract-specific implementation/test anchors and exact-source execution records; hand-maintained input |
| `api-map.json` | Namespace-scoped Rust candidates, inherited-member presence and separately evaluated behavioral evidence; candidates alone do not establish behavior |
| `gallery-xaml-map.json` | XAML member requirements joined through both inheritance trees, with declaring owner and every use site |
| `xaml-feature-map.json` | Audited syntax adaptations, source/test anchors, explicit rejected features and every use site |
| `gallery-page-map.json` | Curated sample implementation evidence joined with upstream requirements |
| `summary.json` / `REPORT.md` | Machine/human summaries with limits and unresolved platform namespaces |

The WinUI input covers `controls/dev/**/*.idl` and the API model under
`dxaml/xcp/tools/XCPTypesAutoGen/{XamlOM/Model,Modules}/**/*.cs`. The model contains
preview and private APIs as well as released APIs; private contracts are retained
and counted separately. No SDK-version or ABI completeness claim is inferred.
The Gallery input includes **all** tracked XAML, C#, snippet text, project and
JSON files under `WinUIGallery`, plus its public machine-readable catalog.

Type mappings can include an `overloads` object keyed by the exact source
signature. This distinguishes a constructor such as `SymbolIcon(Symbol)` from
the parameterless constructor when their Rust entry points differ. The mapping
still establishes only a declaration candidate; behavioral evidence is separate.

The icon-specific `symbol_map.py` gate checks all 197 public values, their
modern glyph conversions from WinUI's `icon.cpp`, the Rust iteration table and
the XAML enum names. It runs offline. Refreshing its immutable reference uses
`--refresh-source --winui-root <clean pinned checkout>`. See
`docs/porting/winui-gallery/CONTROL-CONTRACTS.md` for native rendering test scope
and installed-font limitations.

The `event_targets.py` gate generates the XAML loader's exact event-owner table
from the pinned public API snapshot. It follows WinUI inheritance and requires an
unambiguous namespace/type identity. A custom property with the same spelling as
a WinUI event does not become an event by name alone. The table prepares strict
target diagnostics; it does not establish that an event-binding contract is implemented.

## Preserving the existing FerroUI API

`existing_api.py --check` compares current declarations with FerroUI commit
`86b6b4ab41669bf830fba681504a4ee94752a4e6`, the baseline before this WinUI work.
Its lock is separate from the Microsoft source locks. The baseline uses the same
namespace-table/member collector as the parity scanner, retaining explicit public
type declarations, generated virtual traits and source-registered markup types.
Private unregistered helper types are excluded.

The guard rejects a removed or renamed identity, a namespace move, a changed
declared base, a removed member name, reduced public visibility, removed source
registration, or an additional declaration origin colliding with an existing
namespace identity. New native types and additive members are allowed. In particular,
`FramePage` preserves the existing `Page` identity, and Frame navigation arguments
use `FerroUI.Navigation` to preserve the existing control-navigation arguments.
Pre-existing conditional declaration origins remain recorded for review.

This is **source-declaration compatibility only**. It does not prove signatures,
generic bounds, defaults, public module re-exports, ABI, layout, event behavior,
or Avalonia compatibility as a whole. Conditional compilation is not evaluated.
Moving a declared member into a base class is reported conservatively for review.
Reviewed adaptation maps and exact-source behavioral evidence remain independent.

The normal gate requires no Git history or upstream checkout. To reproduce the
baseline when the pinned local Git object is available, run:

```sh
python3 scripts/winui-gallery/existing_api.py --verify-reference
```

`--refresh-baseline` rebuilds from that same fixed commit; it never advances the
baseline to current sources. Extraction reads Git objects into a temporary
directory without changing any checkout or index. Changing the baseline commit
requires an explicit reviewed script/lock/snapshot change.

The exported catalog is an index, not the original snippet text. Its exporter
may remove substitution tokens from the displayed XAML. Every example retains
its `catalog_transformations`, original `.txt` source path and raw-file hash;
sample execution and source display must use those original files. The current
pin has 355 actual `ControlExample` nodes and 350 exported examples, so both
inventories remain visible and both participate in complete-page verification.

## Curated Gallery evidence

The sample owns `samples/WinUIGallery/port-status.json`:

```json
{
  "schema_version": 1,
  "pages": {
    "Button": {
      "status": "partial",
      "implementation": ["samples/WinUIGallery/pages.rs"],
      "tests": [{"path": "samples/WinUIGallery/tests.rs", "name": "button_click_updates_output"}],
      "notes": "The click example is executable; all remaining upstream examples need evidence."
    }
  }
}
```

Omitted pages are `pending`. `partial` means some live implementation exists;
`ported` means the author claims the page is implemented; `verified` requires
implementation files, existing test anchors, every catalog example and actual
`ControlExample` instance accounted for as verified, and a distinct execution
record. Test names alone do not prove that
the tests ran. Generated reports never promote a page automatically. A separate
runtime integration test must establish that the sample actually creates controls
and executes the contracts; browsing original source does not establish that.

An execution reference has the form `{"record": "path/to/execution.json"}`. Its
record contains `schema_version: 1`, `result: "passed"`, the actual `command`,
`passed_tests`, and `source_sha256` for every implementation/test input. The
scanner rejects failed, empty, unrelated or stale records. API evidence uses the
exact `id` from `api-map.json`, including member kind and signature, so overloads
cannot accidentally share one completion flag. A type requires separate evidence
for construction/inheritance and every public member before it can be verified.
Partial API entries can record reviewed implementation without a test execution;
when they do supply an execution record, its hashes and passed tests are checked
as strictly as verified entries. A successful subset run still remains partial.

`summary.json` and `REPORT.md` show separate verified/partial/unverified counts
for public type contracts and public member contracts. Private declarations are
excluded. Enum values are shown separately from other members, and verified
types/enum scopes are named explicitly: verifying all 197 `Symbol` values does
not verify the controls or fonts that consume them. Gallery pending/partial/
ported/verified totals appear above the page-by-page table. Candidate counts
remain a separate source-comparison measure and never promote behavior status.

## Deliberate scanner limits

- Source-level lexical parsers do not run Microsoft's MIDL/C# code generators or
  evaluate SDK/preview feature conditions. Source attributes (including default
  values and projected overload names) remain in the reference for review.
- C# imports and explicitly qualified references are inventoried. Unqualified
  receiver expressions, aliases, extensions and overload resolution require a
  Roslyn/WinMD semantic pass; the scanner does not invent bindings for them.
- Rust comparison uses the crate's namespace table, public member declarations
  and unambiguous base-class lookup. It does not prove Rust signatures, XAML
  registration behavior, property defaults, event ordering or visual equality.
- Alternate control candidates such as `ListView` to `ListBox` retain explicit
  semantic cautions and cannot be treated as interchangeable aliases. New native
  implementations such as `NumberBox` map directly to `FerroUI.Controls`; genuine
  name conflicts use reviewed mappings such as source `Page` to `FramePage`.
- Platform imports remain unresolved adapter requirements until native service
  implementations and platform-specific execution evidence exist.

The strict gate is intentionally stronger than report freshness. It must not be
used as a required green check while this port is partial, or weakened by treating
source-name matches as implementation evidence.
