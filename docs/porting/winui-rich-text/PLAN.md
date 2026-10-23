# Rich text implementation plan

## Architectural boundaries

The public read-only rich document controls and editable rich text share document
content and FerroUI's formatting services. Document mutation does not depend on
a renderer. Layout produces existing `TextLine`/glyph-run data; the control uses
`DrawingContext` and the platform-selected backend. Core code has no DirectWrite,
Direct2D, COM, Win32 message, native RichEdit, filesystem, or blocking-thread
dependency.

| Upstream responsibility | FerroUI destination | Translation and acceptance requirement |
|---|---|---|
| `TextElement`, `Inline`, `Run`, `Span`, emphasis, inline UI | `src/FerroUI.Controls/documents/` | Reuse existing `Ref<T>` classes and inherited text properties. Audit content ownership, changed-event propagation, UTF-16 lengths, embedded controls, and typography before marking exact parity. |
| `Block`, `Paragraph`, `BlockCollection`, rich document | New modules beside existing document inlines | Shared collection handles, logical-tree ownership, property inheritance, stable document positions, versioned snapshots, formatted edits, and source-to-run mapping. |
| `TextPosition`, `TextPointer`, logical direction | Document positions and ranges | Separate text indices from structural positions. Preserve anchor gravity through insertion/deletion and reject invalid ranges without panicking on input. UTF-16 is the public text index unit already used by FerroUI. |
| `ParagraphTextSource`, text runs and paragraph properties | `ITextSource`, `TextRun`, `TextRunProperties`, `TextParagraphProperties` | Adapt document spans to the existing formatter; retain paragraph boundaries and inline-object metrics. Do not flatten away formatting or embedded content. |
| `BlockLayoutEngine`, page and paragraph nodes/breaks | Rich document layout and `RichTextBlock` | Use `TextFormatter`/`TextLine`; retain line/paragraph index mapping, margins, line height, indentation, wrapping, trimming, flow direction, selection geometry, hit testing, and incremental invalidation. |
| Linked `RichTextBlockOverflow` | Rich text overflow control | One document and selection across a weakly linked container chain; detect cycles, pass deterministic break state, invalidate following containers, and verify unlink/reflow. |
| LineServices host and callbacks | Existing FerroUI formatter plus explicit missing algorithms | Audit each callback's semantic role. Existing line breaking, shaping and bidi support is reusable; it does not establish identical LineServices metrics or all callback behavior. |
| Font collections, fallback, script analysis | `FontManager`, platform font contracts, `TextShaper`, Unicode services | Keep backend choice in the existing platform services. Verify font fallback, script splitting, grapheme clusters, bidi, OpenType features, and fallback-error behavior with fixed fonts. |
| `TextSelectionManager`, navigation and grippers | Selection/navigation services, routed input, `TextSelectionHandle` | Use existing input/gesture contracts; preserve anchor/focus direction, logical versus visual navigation, word/grapheme selection, dragging, clipboard commands, and touch handles. |
| `RichEditBox`, `TextBoxBase`, `TextServicesHost` | `RichEditBox`, rich text edit services | A native FerroUI control backed by a rich document, with formatted mutation, undo/redo, selection, keyboard commands, hit testing, focus, scrolling, clipboard, IME client, and event ordering. |
| RichEdit text object model | Rust document/range/character/paragraph format APIs | Implement every tracked documented contract independently of the unavailable external engine; avoid pretending a similarly named method with a narrower contract is complete. |
| RTF and clipboard | Bounded parser/writer and platform clipboard abstraction | Preserve the supported rich content end to end. Malformed input must return errors, not panic. Unsupported destinations/features remain explicit; full RichEdit file-format parity needs dedicated fixtures. |
| Text highlighters and range collections | Shared highlighter/range values and draw-layer integration | Audit merge precedence, overlap splitting, background/foreground painting, inline UI exclusions, selection precedence, and high contrast. |
| Accessibility providers | Existing automation peer architecture plus text/range providers | Text pattern, selection, document range, supported selection, geometry, movement/expansion, attribute queries, and editable read-only behavior require actual provider implementations and platform checks. |
| Windows text input/platform helpers | `TextInputMethodClient`, `InputMethodManager`, platform adapters | Expose preedit/commit/surrounding-text/caret geometry through FerroUI; keep TSF/CoreText/browser/IBus details in backends. Host-only adapters do not implement proofing, candidates, or handwriting. |
| WinRT generated classes and XAML model | `ferro_class!`, `ferro_properties!`, `ferro_class_info!`, type registration | PascalCase markup names, shared collections as content, direct/inherited properties, events, object construction, compiled/runtime XAML, and binding updates. |
| RichEditBox theme | Existing WinUI `ControlTheme` architecture | Audit the real control's template parts and states, light/dark/contrast variants, placeholder/header/description, selection and focus behavior; theme delivery is separate from control semantics. |

## Work batches

Each batch updates the per-file/per-member map and introduces tests for behavior
whose implementation is present. A batch is complete only when its acceptance
criteria pass; the existence of a module is insufficient.

| Batch | Deliverable | Acceptance gate |
|---|---|---|
| R0 | Pinned source, dependency and member inventory, attribution, conventions | All 230 + 62 + 23 primary files present, hashes stable, public member IDs unique, generator check clean, external boundaries documented. |
| R1 | Block/paragraph document model, inline reuse, stable ranges, snapshots | Shared-handle identity, ownership/reparenting, UTF-16/astral/grapheme edges, invalid range failure, inherited style invalidation, formatted edit preservation, anchor gravity, snapshot consistency. |
| R2 | Paragraph/page layout, read-only rich control, overflow | Multi-paragraph mixed formatting, bidi and inline UI, constrained measure/arrange, wrapping/trimming, paragraph spacing, hit test round trips, selection rectangles, linked reflow and cycle safety. |
| R3 | Editable rich document, selection, history, format operations | Insert/delete/replace across spans and paragraphs; collapse/extend; transaction grouping; undo/redo of both text and formatting; read-only checks; change notifications outside mutable borrows. |
| R4 | Format interoperability and clipboard | RTF nested groups/escapes/Unicode/code pages/fonts/colors/paragraphs; deterministic round trips; malformed/truncated/deep input; rich clipboard copy/cut/paste; unsupported constructs inventoried. |
| R5 | Editable control, routed commands, scrolling and IME | Focused key/pointer interaction, caret updates, selection rendering, drag autoscroll, composition update/commit/cancel, surrounding selection, clipboard service, read-only event ordering, lifetime tests. |
| R6 | Full highlighters, typography, proofing and input services | Original highlighter overlap fixtures; typography feature inventory; spelling/candidates/handwriting adapters where contracts exist; explicit platform feature detection where they do not. |
| R7 | Accessibility and XAML/theme integration | Text/range provider tests and screen-reader exercise; runtime and compiled XAML load/binding; dynamic theme and property changes; correct logical/visual tree ownership. |
| R8 | Complete external text-object-model contract | Every public API member disposition reviewed; ranges/formatting/tabs/lists/tables/images/math/link/language/serialization contracts exercised as applicable; remaining external-engine behavior closed or explicitly accepted as a deviation. |
| R9 | Native/browser conformance and performance | Upstream behavioral fixture adaptation, platform screenshots with pinned fonts and DPI, large-document edit/reflow/selection profiling, leak/lifetime checks, reproducible source-map and test reports. |

## Rust-specific decisions

`Ref<T>`/`WeakRef<T>`, inherited styled properties, and observable collection
handles follow the porting guide. C++ ref-counting and pointer graphs are
translated into these existing ownership conventions; they are not copied as a
second object system. Ordinary value-state snapshots can use owned vectors and
`Rc` data where immutable sharing is useful. Calls that can run application code
must not retain a `RefCell` borrow.

The public index contract uses UTF-16 code units. Internal UTF-8 storage is valid
only behind explicit conversion that handles surrogate-pair interiors and
grapheme boundaries according to each operation's contract. Layout positions,
storage byte indices, Unicode scalar counts, grapheme indices, and structural
document positions are distinct quantities.

An upstream header/implementation pair may share a Rust module. A formatting
contract already present in FerroUI stays in its existing module and is adapted
by the rich text layer. All members not ported in the current batch remain
`pending` or `partial`; they do not receive `todo!()`, inert defaults, or success
responses that discard requested behavior.

## Review and completion

The local test report records the exact command and result. UI parity additionally
needs real native/browser interaction and render evidence; unit tests of the
document model cannot establish pixel or input-service parity. A native WinUI
comparison should record OS, reference runtime, fonts, scale, locale, theme,
render backend, and corpus revision. Performance reports separate shaping,
paragraph formatting, whole-document invalidation, visible rendering, and edits.

Before declaring full parity, close all pending member rows, check every reuse
candidate against the upstream behavior, reconcile all include-boundary gaps,
and obtain evidence for the platform-dependent features. Do not reduce the
inventory's scope to make the counts look complete.
