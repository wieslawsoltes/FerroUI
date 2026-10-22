# AvaloniaEdit and TextMateSharp port

The target is a complete idiomatic Rust port integrated with FerroUI. The working
implementation is in [draft PR #295](https://github.com/wieslawsoltes/FerroUI/pull/295),
on `port/ferroedit-textmate`. **This remains an incomplete port.** The checked
source inventory and explicit limitations below are the acceptance backlog;
passing the current regression suite is not a claim of complete parity.

The implementation follows [PORTING-GUIDE.md](PORTING-GUIDE.md): concrete controls
use the framework class model, properties and routed input; shared service
objects use Rust handles and interior mutability; fallible operations return
`Result`; upstream source and behavior are documented separately.

## Pinned inputs and attribution

| Repository | Revision | Role |
|---|---|---|
| [AvaloniaUI/AvaloniaEdit](https://github.com/AvaloniaUI/AvaloniaEdit) | `7277e36366d754219d7ede16b6613786b32caf4c` | Document/editor services, controls, highlighting definitions and integration contracts |
| [danipen/TextMateSharp](https://github.com/danipen/TextMateSharp) | `e0db45be30278c0f3f80302c31f498c8f8f1c4f2` | TextMate engine, grammar registry, themes, language packages and reference tests |
| FerroUI starting point | `d5405346e680e060c9675572efe02df131428c08` | Existing framework APIs, renderer, input, platform services and port conventions |
| `vscode-textmate` | `9.3.2` | Independent token stream oracle |
| `vscode-oniguruma` | `2.0.1` | Oracle scanner and bundled browser scanner |

Each new crate includes the project license and notices plus its upstream
`NOTICE.md`. Grammar packages retain their component licenses; the catalog has
51 packages, 72 language contributions, 79 grammar contributions and 21 theme
files. The XSHD catalog contains all 21 source definitions.

The [asset manifest](data/editor-assets.json) records the exact upstream path,
revision and SHA-256 of 347 unchanged grammar, theme and XSHD files. The browser
scanner's binary and JavaScript hashes are in
[`resources/versions.json`](../../src/FerroUI.TextMate/resources/versions.json).
`check_assets.py` verifies these without downloading anything. The unchanged XML
language package includes a legacy `.axaml` extension: the source-name check
exempts that one data manifest and the asset check pins its complete bytes.
Rust control/API identifiers follow FerroUI naming.

## Architecture and framework reuse

| Package | Responsibility | Reused framework contracts |
|---|---|---|
| `ferroui-edit-core` | Persistent UTF-16 documents, anchors, versions, tracked intervals, undo, selections, search, folding, completion, snippets and XSHD | UI-independent Rust services; no renderer, platform window, native filesystem or worker-thread requirement |
| `ferroui-textmate` | Grammar compilation, Oniguruma scanning, state stacks, captures, selectors, themes, catalog and incremental tokenization | Framework utility contracts where appropriate; no controls or render backend |
| `ferroui-edit` | `TextEditor`, `TextArea`, viewport, command bindings, clipboard, IME, automation, search/completion popups and snippet input | `Decorator`, `ScrollViewer`, `ILogicalScrollable`, `TextLayout`, `DrawingContext`, styled/direct properties, routed events, `ICommand`, dispatcher, popup/focus infrastructure and platform services |
| `editor-demo` | Runnable Rust/C# editor and public-API integration tests | Runtime XAML, Fluent desktop/headless hosts and existing sample support |

The control does not draw through Skia, browser canvas or a new widget system.
Its text shaping, hit testing, wrapping, glyph drawing and decorations use
FerroUI's text formatter. Its popups are actual FerroUI controls. This keeps
renderer and platform selection in the host application.

### Text and ownership

All public document offsets and lengths count **UTF-16 code units**. Logical
lines and columns are one-based; service-specific rectangle coordinates are
explicitly documented. Regex UTF-8 byte indices and JavaScript UTF-16 indices
are converted at their boundaries. The UTF-16 storage can preserve isolated
surrogates; Rust `String` views use replacement characters when required.

`TextDocument` clones share identity. Immutable snapshots retain persistent
rope storage and can cross threads; the mutable document is a UI-thread handle.
Callbacks are copied out before invocation so handlers can inspect services
without borrowing conflicts. Reentrant document mutation during a changing or
changed notification is rejected; post-update callbacks have a separate
contract. Mutating multiple selections or replacement ranges prevalidates the
whole operation and uses one undo group.

`DocumentLine` remains a cheap revision value. `LiveDocumentLine` supplies stable
identity, neighbors and deletion state when the caller needs live line handles.
This distinction is deliberate in Rust and is documented in the
[document report](editor-document.md), including line-tracker callback timing.

### Storage and viewport

The persistent AVL rope stores UTF-16 lengths and CR/LF/CRLF line metrics.
Anchors and tracked segments use safe arena trees with lazy suffix movement;
their randomized tests check both observable positions and tree invariants.
Snapshots and versions do not require a full string copy on every edit.

The viewport shapes visible logical lines and keeps a small nearby cache.
Unmeasured heights have a default value; sparse measured heights account for
wrapping. Collapsed intervals are represented as ranges, so hiding a million
lines does not allocate a million height entries. Folded text joins a visible
prefix, placeholder and suffix while retaining document/display offset maps.
Tabs and IME composition use the same explicit mapping. Moving the caret into
hidden text unfolds its containing section.

`Text` is a direct property. Its notifications materialize strings only when
object or class-wide property observers exist. Document clients and routed
`TextChanged` observers retain the persistent-document editing path.
The small `PropertyChangedObservable::has_observers` addition to FerroUI.Base
supports that distinction without bypassing the property notification API.

### Input and services

The editor uses the platform's configured hotkeys for standard editing and
navigation. Clipboard operations use the top level's asynchronous clipboard;
cut only deletes after a successful write and checks the captured document,
version and selection before doing so. Read-only/protected-range validation
also applies to completion, replace-all and linked snippet mirrors.

Search/replace uses the core search engine and displays its diagnostics.
Completion keeps the text area's focus while a popup list handles navigation
and insertion. Snippet fields use tracked ranges, synchronize bound text after
a completed edit, and participate in the same undo operation. The input target
owns the keyboard subscriptions, including before template application.

The IME client exposes UTF-16 surrounding text and selection. Composition is
projected into the formatted visual line without editing the document; the
platform cursor index determines the composition caret. The automation peer
provides the framework value-provider contract. Full text-range accessibility
is still an acceptance item.

### Native and browser grammar execution

Native builds use the pinned Rust Oniguruma binding. Browser builds use a
bundled official Oniguruma Wasm scanner through wasm-bindgen, with explicit
async initialization and synchronous scanning afterward. The host must await
`initialize_browser_oniguruma()` before loading/tokenizing grammars. Runtime
initialization does not fetch a remote script or binary.

The separately selected `portable-regex` backend is a compatibility subset. It
must not silently replace full Oniguruma behavior. Pattern compilation, match
budgets, cancellation, include errors and unsupported constructs report errors.
Native and browser scanner versions differ; matching the committed oracle
corpus does not establish equivalence for every possible regex.

The editor's TextMate installation aligns cached line states with persistent
snapshots, invalidates the affected interval and stops propagation when the
unchanged suffix converges. Theme/grammar changes notify the host outside
internal borrows. Work is bounded per line and per dispatcher batch.

## Source map and behavioral evidence

[editor-source-map.md](editor-source-map.md) inventories every tracked C# and
XAML-family source under the pinned source roots. It includes tests, demos and
sample sources instead of hiding them. Each file has an explicit mapped,
adapted or unmapped decision; each declared type has an exact Rust destination
or an explicit unmapped entry. The checker validates destination symbols using
FerroUI's existing Rust scanner. It does not infer method parity from a name.

| Report | Evidence and contract details |
|---|---|
| [Document](editor-document.md) | Rope metrics, anchors, intervals, live lines, tracker timing, version translation, custom/optional undo, event ordering and deterministic model tests |
| [Editor services](editor-services.md) | Selection/edit policy, search, XSHD, folding, completion, snippets, commands and popup behavior |
| [TextMate engine](textmate-engine.md) | Grammar/scanner semantics, registry/catalog, metadata, theme rules, incremental states, native/Wasm oracle and remaining engine differences |
| [Source inventory](editor-source-map.md) | Per-file source hashes, exact aliases, partial contracts and unmapped APIs |
| [Editor demo](../../samples/EditorDemo/README.md) | Run commands, runtime XAML, toolbar, explicit-region provider and headless/desktop limits |

Checkpoint evidence is recorded with the commits and updated as tests land:

- The combined checkpoint passes **185 core tests, 34 control tests and two demo tests**. It includes
  1,600 randomized rope edits, 5,000 appends, dense anchor tests, 2,500 interval
  model edits, 8,000 lazily shifted intervals, 2,500 live-line edits with replay,
  custom undo, C# indentation, protected text transformations, rich text/HTML
  exports, incremental XSHD convergence and all XSHD resources.
- The native TextMate library passes **31 tests** and the explicit portable
  backend passes **30 tests**. Native TextMate and actual compiled Rust/Wasm
  both match all **74 reference
  lines across 11 language families**, comparing token boundaries, scopes and
  continuation depths. The fixture is generated independently with the pinned
  JavaScript implementation, not with the Rust code being checked.
- Control tests exercise framework rendering, visible-line cache bounds over a
  100,000-line document, UTF-16/tab hit testing, folding, routed keys, direct Text
  notifications, runtime XAML, popups, snippets, IME visual composition, wrapped
  visual-row navigation and incremental tokenization. The render notification
  regression checks that extent updates publish outside the render pass.
- The incremental installation tests compare 200 batches of queued CR/LF/comment
  edits with fresh highlighting, plus line insertion/deletion, multiline state,
  bounded work, cancellation and theme-change reentrancy.
- The source-inventory fixture tests cover exact-symbol validation and invalid
  mappings. Asset hashing and release metadata checks run independently.

The dedicated [editor CI workflow](../../.github/workflows/editor.yml) runs the
source/asset checks, native core/control/demo tests, explicit fallback tests,
Wasm compilation and actual Rust/Wasm token-stream conformance. The existing
workspace CI also discovers the new packages.

## Reproducing the checks

```sh
python3 scripts/edit-port/editor_port.py --check
python3 scripts/edit-port/test_editor_port.py
python3 scripts/edit-port/check_assets.py

cargo test --locked --lib --no-default-features \
  -p ferroui-edit-core -p ferroui-textmate -p ferroui-edit -p editor-demo \
  --features ferroui-textmate/native-onig,ferroui-textmate/bundled-grammars
cargo test --locked -p ferroui-textmate --lib --features portable-regex
cargo check --locked -p ferroui-edit-core -p ferroui-edit \
  --target wasm32-unknown-unknown
```

For a constrained runner, set `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`,
`CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_PROFILE_TEST_DEBUG=0`. The initial default
framework debug build exceeded this runner's memory allocation. Desktop sample
validation also encountered a blocked Skia dependency download and disk
pressure; headless/library tests avoid that download. A successful headless
lifetime run is not a pixel screenshot or proof of platform IME/clipboard
behavior. Actual desktop and browser visual/input checks remain required.

The Wasm execution commands and oracle regeneration procedure are in the
[engine report](textmate-engine.md). Regenerating a fixture is a reviewable
change: inspect differences before accepting a new baseline.

## Remaining acceptance work

The generated map is the authoritative detailed backlog. The substantial open
areas include the complete visual-line element/transformer/layer/margin
extension contracts; rectangular selection and virtual-space parity; remaining
command and modern C# lexical edge cases; overload insight and rich completion
content; native rich-text control/clipboard integration and XSHD visitor/writer interfaces;
complete managed service/reader/version overload equivalents; text-range
accessibility; and the full upstream behavior/test corpus.

Ported functionality still needs platform verification for pointer capture,
clipboard formats, IME composition, accessibility and browser host lifecycle.
Long-line shaping and throughput must be measured against the reference;
algorithm choices and smoke tests are not performance equivalence evidence.
Crossing/overlapping fold edge cases, bidirectional editing, wrapping geometry,
protected mixed selections and exotic regex/selector cases require additional
reference comparisons.

The PR remains draft until these acceptance items are resolved or explicitly
re-scoped by the maintainer. Unsupported contracts are documented rather than
represented with placeholder methods or silently advertised as complete.

## Working process

1. Pin upstream inputs and inspect FerroUI's port guide and existing services.
2. Open the draft PR before implementation and record the compatibility contract.
3. Port independent document/editor/TextMate subsystems in separate source areas;
   retain source licenses and avoid unrelated workspace changes.
4. Add behavioral tests and independent reference fixtures as each subsystem lands.
5. Integrate through actual FerroUI APIs, then test rendering, input, markup and
   lifecycle behavior using the framework's own services.
6. Generate the full source/asset inventory, record explicit adaptations and gaps,
   and add repeatable CI gates.
7. Commit and push coherent checkpoints frequently. Update this report and the
   PR description from verified results; do not convert a source mapping count
   into a parity percentage or merge the draft as a completed port.
