# Code editor and TextMate port

FerroEdit is the code editor of FerroUI: a persistent UTF-16 document model, a
virtualized editor control and a TextMate grammar engine with bundled language
packages. [Issue #438](https://github.com/wieslawsoltes/FerroUI/issues/438)
tracks the integration. The source inventory and the known gaps below record
what the port covers and what it does not.

The implementation follows [PORTING-GUIDE.md](PORTING-GUIDE.md): concrete controls
use the framework class model, properties and routed input; shared service
objects use Rust handles and interior mutability; fallible operations return
`Result`; upstream source and behavior are documented separately.

## Pinned inputs and attribution

| Repository | Revision | Role |
|---|---|---|
| [AvaloniaUI/AvaloniaEdit](https://github.com/AvaloniaUI/AvaloniaEdit) | `7277e36366d754219d7ede16b6613786b32caf4c` | Document/editor services, controls, highlighting definitions and integration contracts |
| TextMateSharp | `e0db45be30278c0f3f80302c31f498c8f8f1c4f2` | TextMate engine, grammar registry, themes, language packages and reference tests |
| `vscode-textmate` | `9.3.2` | Independent token stream oracle |
| `vscode-oniguruma` | `2.0.1` | Oracle scanner and bundled browser scanner |

Each new crate includes the project license and notices plus its upstream
`NOTICE.md`. Grammar packages retain their component licenses; the catalog has
51 packages, 72 language contributions, 79 grammar contributions and 21 theme
files. The XSHD catalog contains all 21 source definitions.

The [asset manifest](data/editor-assets.json) records the upstream path, revision
and SHA-256 of the 347 bundled grammar, theme and XSHD files. 346 are byte-identical
to their source. The XML language manifest drops one legacy file extension of the
upstream markup language; see [DEVIATIONS.md](DEVIATIONS.md#text-editor-and-textmate-port).
The browser scanner's binary and JavaScript hashes are in
[`resources/versions.json`](../../src/FerroUI.TextMate/resources/versions.json).
`check_assets.py` verifies these without downloading anything.
Rust control/API identifiers follow FerroUI naming.

## Architecture and framework reuse

| Package | Responsibility | Reused framework contracts |
|---|---|---|
| `ferroui-edit-core` | Persistent UTF-16 documents, anchors, versions, tracked intervals, undo, selections, search, folding, completion, snippets and XSHD | UI-independent Rust services; no renderer, platform window, native filesystem or worker-thread requirement |
| `ferroui-textmate` | Grammar compilation, Oniguruma scanning, continuations, captures, selectors, themes and incremental tokenization | Framework utility contracts where appropriate; no controls or render backend |
| `ferroui-textmate-grammars` | Optional bundled language packages, grammars and themes | Co-located resource and license inventory. The engine enables the crate through `bundled-grammars`. |
| `ferroui-edit` | `TextEditor`, `TextArea`, viewport, command bindings, clipboard, IME, automation, search/completion popups and snippet input | `Decorator`, `ScrollViewer`, `ILogicalScrollable`, `TextLayout`, `DrawingContext`, styled/direct properties, routed events, `ICommand`, dispatcher, popup/focus infrastructure and platform services |
| `editor-demo` | Runnable editor with Rust and C# syntax examples and public-API integration tests | Runtime XAML, Fluent desktop/headless hosts and existing sample support |
| `editor-demo-browser` | Linked native editor browser host with production scanner initialization | Existing FerroUI browser/Emscripten host, native font collection and real pointer/key/composition paths |

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
a completed edit, and participate in the same undo operation.

`TextArea` owns its primary and temporary input handlers. Applications can
replace or disable the default mode, or keep it below temporary preview
handlers. Temporary handlers receive key-down and key-up events newest first.
Popping one detaches it and all newer handlers. Attached handlers can include
children, scoped native event subscriptions, and live FerroUI `KeyBinding`
objects. Completion and snippet traversal share this stack. Replacing the
primary mode releases pointer selection and ends temporary input sessions.
Child lists are read at attachment time.

Routed committed input, Enter, and `perform_text_input` use the same cancellable
`TextEntering` and post-edit `TextEntered` events. A single newline uses the
line ending at each caret and the document's indentation strategy. A document
without a delimiter uses the configured newline. The area also publishes
`TextCopied` and `TextPasted` when the clipboard operation succeeds.

The IME client exposes the primary selection's logical line span and reports
zero-based UTF-16 offsets relative to that span. A selection spanning multiple
lines keeps the delimiters between its endpoints. A platform selection change
maps back through the same span. Echoing the current offsets preserves
secondary carets and virtual cells. The pinned client exposes the caret line.
FerroUI also projects preedit into the formatted visual line without editing
the document. A cursor-only update reuses that layout, and clearing an already
empty composition leaves retained lines intact.
[IME projection and cursor](editor-visual-lines.md#live-textarea-integration)

The native composition connection keeps the directed selection intact until
the IME supplies committed text. Win32 IMM32 passes preview updates to the
text area and commits through routed input as one protected edit. Cancellation
leaves the selection and history intact. A document or selection change
retires the preview and invalidates the captured session. The browser's
revisioned textarea transport requires a client that exposes exact edit state
and accepts explicit range edits.

The automation peer exposes FerroUI Value, Text and Text2 providers. Ranges use the document's UTF-16
anchors and expire when the editor binds a different document. The providers
expose the current selection and caret, Unicode navigation and search, text
styles and protected spans, clipped geometry for wrapped and bidirectional
rows, folded regions, and the ranges of hosted inline controls. Focus and
hit testing in the text area resolve to the editor's peer. An inline control
keeps its own peer and focus. Continuous
scrolling promotes the Page unit to Document. The editor has no annotation
provider.

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

## Source map and reports

[editor-source-map.md](editor-source-map.md) inventories every tracked C# and
XAML-family source under the pinned source roots. It includes tests, demos and
sample sources instead of hiding them. Each file has an explicit mapped,
adapted or unmapped decision; each declared type has an exact Rust destination
or an explicit unmapped entry. The checker validates destination symbols using
FerroUI's existing Rust scanner. It does not infer method parity from a name.

| Report | Contract details |
|---|---|
| [Document](editor-document.md) | Rope metrics, anchors, intervals, live lines, tracker timing, version translation, custom/optional undo, event ordering and deterministic model tests |
| [Editor services](editor-services.md) | Selection/edit policy, search, XSHD, folding, completion, snippets, commands and popup behavior |
| [Virtual space and rectangles](editor-virtual-space.md) | Logical/visual positions, native geometry, virtual padding, rectangular replacement and protected edit plans |
| [Native completion and insight](editor-completion.md) | Rich completion items, list filtering, shared popup lifecycle, observable overload providers and runtime XAML |
| [Rich text and clipboard](editor-rich-text-clipboard.md) | Native brushes/runs, complete font styles, HTML formats, cancelable events, delayed cut ownership and rectangle paste |
| [Visual lines](editor-visual-lines.md) | Live generators/transformers, native runs, multiline projections, extension ownership, URI routing and measured geometry |
| [Background geometry and live layers](editor-backgrounds.md) | Native StreamGeometry, bidi/wrapped range projection, retained rendering phases, callback invalidation and styled renderer settings |
| [Built-in background renderers](editor-built-in-backgrounds.md) | Column guides, current-line highlight, active/linked snippet fields and weak control lifetime |
| [Snippets](editor-snippets.md) | Extensible insertion contexts, tracked custom elements, lifecycle events, transformed fields and native undo ownership |
| [XSHD runtime](editor-xshd-runtime.md) | Explicit capture, bounded region matching, span-only state scanning, zero-width diagnostics and remaining .NET regex differences |
| [File and stream input](editor-file-io.md) | Bounded encoding detection/replay, BOM decoding, UTF-16 readers and standard Rust/native-storage byte streams |
| [Browser editor](editor-browser.md) | Linked native application, production Oniguruma initialization, renderer/input conformance gates |
| [TextMate engine](textmate-engine.md) | Grammar/scanner semantics, registry/catalog, metadata, theme rules, incremental states, native/Wasm oracle and remaining engine differences |
| [TextMate runtime contracts](textmate-runtime-contracts.md) | Source/member audit of compiler, parser, theme and model helpers; token-type boundary correction and explicit public API gaps |
| [Source inventory](editor-source-map.md) | Per-file source hashes, exact aliases, partial contracts and unmapped APIs |
| [Editor demo](../../samples/EditorDemo/README.md) | Run commands, runtime XAML, toolbar, explicit-region provider and headless/desktop limits |

The dedicated [editor CI workflow](../../.github/workflows/editor.yml) runs the
source/asset checks, native core/control/demo tests, explicit fallback tests,
Wasm compilation, actual Rust/Wasm token-stream conformance and the linked
browser editor renderer/input suite. The existing
workspace CI also discovers the new packages.

## Highlighting contracts

### TextMate theme rule precedence

Theme loading reads legacy `settings` and `tokenColors` together and resolves
redefinitions by entry index. Empty scope collections leave defaults intact.
When two eligible rules target the same scope, the selector with the deeper
target or more specific parent path wins. This covers cases where the more
specific ancestor is further up the token stack.

[Theme rule parsing and specificity](textmate-runtime-contracts.md#theme-rule-parsing-and-specificity)

### TextMate style inheritance

The editor applies explicit TextMate properties over the native visual-line
style. A theme palette configures the sample canvas and leaves missing token
properties inheritable. Rich copy carries explicit token formatting, so
unstyled text can pick up the destination's colors. Binary tokenization uses
root token defaults and includes scoped styles from inherited themes.

[Style defaults in rendering and encoding](textmate-runtime-contracts.md#style-defaults-in-rendering-and-encoding)

### Encoded scope precedence

Binary tokenization resolves one winning rule at each scope and carries
inherited attributes into nested scopes. A compiled index resolves dotted
prefixes and repeated declarations within a theme layer. FerroUI differs from
the pinned runtime in one case: it applies a declared parent selector when its
target appears once.
[Encoded scope inheritance](textmate-runtime-contracts.md#encoded-scope-inheritance)

### XSHD definition resolvers

Hosts can supply compiled XSHD definitions through a closure or the
highlighting manager. Qualified references resolve named colors, main and
named rule sets, and recursive spans. Compiled aliases can be replaced while
existing consumers retain their snapshot and subsequent lookups recompile.

[Compiled definition resolvers](editor-xshd-runtime.md#compiled-definition-resolvers)

### Named XSHD in editor markup

`TextEditor.SyntaxHighlighting` accepts bundled names and application aliases
through native markup metadata. It shares the styled property path with the
typed Rust setter. The editor rebuilds its highlighter on document changes.
Local clearing and style changes use normal FerroUI property precedence.

[Named syntax highlighting in markup](editor-services.md#named-syntax-highlighting-in-markup)

### Further editor contracts

- The overload provider handle exposes its selected index, count, display text,
  signature and documentation to FerroUI markup.
  [Completion and overload contracts](editor-completion.md#differences-and-remaining-validation-boundaries)
  records the host binding contract.
- Completion lists accept native item templates from Rust and XAML.
- TextMate models can start with highlighting disabled, clear an active grammar
  and resume tokenization after edits. Clearing publishes one full-document
  change and drops every continuation state.
- A live nested XSHD span preserves its parent across a line break.
- Programmatic highlighting spans can independently include the opening and
  closing delimiters in their span style.
- TextMate capture scans establish their own anchors, and the catalog's
  standalone JavaScript regex plist loads.
- The binary theme resolver preserves inherited fields when the same condition
  is redeclared at a more specific dotted target.
- Control-character labels use the pinned editor's 2.5-unit corner radius. The
  native drawing interface carries the rounded shape to the renderer and
  geometry collector.
  [Control-character boxes](editor-visual-lines.md#control-character-boxes)
- A fold resumes when an earlier custom element consumes its starting offset.
  The projection keeps the closing row's visible suffix and exposes a caret stop
  at the resumed marker.
  [Folds crossed by custom elements](editor-visual-lines.md#folds-crossed-by-custom-elements)
- Captured character and rectangular drags project through the viewport edge
  when the pointer moves above or below it.
- Pointer entry and exit preserve the measured wrapped layout. Font, wrapping,
  highlighting and gutter changes still refresh it. When the viewport narrows,
  the horizontal extent uses the new viewport width and retains any measured
  width greater than the previous viewport.
  [Pointer selection](editor-virtual-space.md#native-navigation-and-pointer-selection)
- The editor exposes the shared native font style, weight and stretch
  properties. The formatter, virtual-space measurements and line-number gutter
  use the configured typeface, and live font changes rebuild the rows and
  publish the effective style and weight through accessibility.
  [Editor typeface](editor-visual-lines.md#editor-typeface)
- `TextEditor.Watermark` forwards to the text area's styled property. The native
  text layer draws the hint for an empty document and reserves the document
  value and text ranges for actual document content.
  [Empty-document watermark](editor-visual-lines.md#empty-document-watermark)
- With the asynchronous Clipboard API unavailable, the browser adapter scopes
  each read to the paste keystroke and resolves a missing or cancelled event as
  access denied.

### AppKit accessibility

The AppKit bridge exposes the multiline editor as `AXTextArea` and forwards
UTF-16 text, selection, visible range and screen geometry through the portable
text provider. The primary selection follows the active caret when possible.
The bridge also exposes the individual ranges and posts a native selection
notification. AppKit gives these method-based selectors precedence over the
element's key-based attributes. `AXValue` replacement passes counted UTF-8 to
the existing value provider, preserving text following an embedded NUL. AppKit
can also query the line at a UTF-16 offset, the span of a line, and the active
caret's line. These zero-based indices follow the editor's document lines
through wraps, and the ranges include the delimiter.

AppKit style queries reuse the editor's format navigation and batched style
runs. The bridge builds attributed substrings with native font, solid
foreground and background colors, underline, strikethrough, and script
alignment attributes. It converts the document offsets to offsets within
the requested UTF-16 substring.
[AppKit peer wrapper](../../native/FerroUI.Native/src/OSX/automation.mm) ·
[AppKit text accessibility protocol](https://developer.apple.com/documentation/appkit/nsaccessibilityprotocol?language=objc) ·
[Apple accessibility integration guide](https://developer.apple.com/documentation/accessibility/integrating-accessibility-into-your-app) ·
[AppKit accessibility text attributes](https://developer.apple.com/documentation/appkit/text-attributed-string-attributes-and-constants?language=objc)

## Running the checks

```sh
python3 scripts/edit-port/editor_port.py --check
python3 scripts/edit-port/test_editor_port.py
python3 scripts/edit-port/check_assets.py

cargo test --locked --lib --no-default-features \
  -p ferroui-edit-core -p ferroui-textmate -p ferroui-edit -p editor-demo \
  --features ferroui-textmate/native-onig,ferroui-textmate/bundled-grammars
cargo test --locked -p ferroui-textmate --lib --no-default-features --features portable-regex,bundled-grammars
cargo check --locked -p ferroui-edit-core -p ferroui-edit \
  --target wasm32-unknown-unknown
```

Run the native text-input protocol test on macOS with
`cargo test --locked -p ferroui-native --test text_input_options`.

Run the AppKit editor accessibility fixture from a logged-in macOS session.

```sh
cargo test --locked -p editor-demo --test editor_clipboard_macos -- --fixture text-automation
```

For a constrained runner, set `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`,
`CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_PROFILE_TEST_DEBUG=0`. A headless run is
not a pixel screenshot or proof of platform IME or clipboard behavior.

The Wasm execution commands and oracle regeneration procedure are in the
[engine report](textmate-engine.md). Regenerating a fixture is a reviewable
change: inspect differences before accepting a new baseline.

## Known gaps

The generated map is the authoritative detailed backlog. Open areas include the
pointer and clipboard interoperability contracts in the virtual-space report,
remaining command parity, platform popup behavior, regex capture and
culture-sensitive comparisons, platform accessibility, and the full upstream
behavior and test corpus. The XSHD schema is disabled in the pinned upstream
loader; compatibility checks must compare actual loader behavior rather than
introducing new schema restrictions.

- Live assistive technology, including Narrator, Magnifier and VoiceOver
  announcements ([#430](https://github.com/wieslawsoltes/FerroUI/issues/430)).
- Windows IME candidate and result handling with a configured IME, and a live
  macOS IME test with a composing input source.
- Interactive visual review on each desktop backend, and the cursor shape on
  gutter re-entry on macOS.
- Throughput on line types beyond long tabbed lines, full highlighting of
  minified files, and expensive lines within the configurable 3,000-unit UTF-16
  TextMate cutoff. A skipped line and its suffix stay editable as plain text.
  [TextMate line limit](textmate-engine.md#editor-line-length-limit)
- Further custom generator span combinations, bidirectional selection and
  wrapping geometry, and exotic regex or selector cases.
- Clipboard markers on Wayland compositors beyond Sway and Weston.

Unsupported contracts are documented rather than represented with placeholder
methods or silently advertised as complete.
