# Native rich text and editor clipboard

This stage connects the editor's UTF-16 rich-text model to FerroUI's native text
and clipboard services. The pinned reference is recorded in
[avaloniaedit-textmate.md](avaloniaedit-textmate.md). Core text and styles remain
independent of a renderer; native presentation uses the existing `Run`,
`TextBlock`, text formatter, resource tree and `IClipboard` contracts.

## Source responsibilities

Upstream paths below are relative to `src/AvaloniaEdit`. Destination paths are
relative to the FerroUI repository. An adaptation row describes the listed
behavior, not parity for every member in the source file.

| Upstream responsibility | Rust destination | Status and adaptation |
|---|---|---|
| `Highlighting/HighlightingColor.cs` | `src/FerroUI.Edit.Core/highlighting/highlighting_definition.rs` | Runtime foreground/background, family, size, weight, exact slant, underline and strikethrough; plain backend-independent values |
| `Highlighting/RichTextModel.cs` | `src/FerroUI.Edit.Core/highlighting/rich_text.rs` | Independent style setters, effective overlay precedence and clearing |
| `Highlighting/Xshd/V2Loader.cs`, `XmlHighlightingDefinition.cs` | `src/FerroUI.Edit.Core/highlighting/xshd/loader.rs`, `compiler.rs` | Full runtime style retention and validation for XML and programmatic syntax trees |
| `Highlighting/HighlightingColor.ToCss`, `HtmlOptions.cs`, `HtmlRichTextWriter.cs` | `src/FerroUI.Edit.Core/highlighting/html_clipboard.rs` | Full font/decoration CSS, native ARGB to CSS RGBA conversion, safe family escaping |
| `Highlighting/HighlightingBrush.cs` (also declares `SimpleHighlightingBrush`) | `src/FerroUI.Edit/highlighting_brush.rs` | Native immutable brush, color equality/hash, context-aware resolution; adds resource-tree brush resolution |
| Archived `Highlighting/RichText.cs::CreateRuns`, `ApplyColorToTextElement` | `src/FerroUI.Edit/rich_text_adapter.rs` | Working FerroUI `Run`/`TextBlock` adapter and shared native formatter properties; these methods are commented in the pinned source |
| `Highlighting/HighlightingColorizer.cs` style application | `src/FerroUI.Edit/rich_text_adapter.rs`, `text_area.rs` | Native font/brush/decoration translation shared with the editor's formatted lines |
| `Editing/Selection.cs::CreateDataObject`, `EditingCommandHandler.cs` copy/cut/paste | `src/FerroUI.Edit/editor_clipboard.rs`, `text_editor.rs` | Framework data transfer, rich/plain selection, cancellable preparation, async ownership, current-policy cut and paste |
| Archived `Highlighting/HtmlClipboard.cs::SetHtml` and copying customization | `src/FerroUI.Edit/editor_clipboard.rs`, `clipboard_event_args.rs` | Working native HTML publication and routed customization; extension of commented upstream behavior |
| `Editing/RectangleSelection.cs::CreateDataObject`, `PerformRectangularPaste` | `src/FerroUI.Edit/editor_clipboard.rs`, `src/FerroUI.Edit.Core/editing/editor_session/rectangular_paste.rs` | Separate rectangular marker and transactional column paste; Core details in [editor-virtual-space.md](editor-virtual-space.md) |

Native run construction and rich clipboard publication are deliberately
identified as restored/extended behavior. The pinned source includes their
historical implementations as comments; their presence is not evidence that
the pinned application exercised them.

## Runtime styles and native text

`HighlightingColor` now preserves `font_family`, `font_weight`, `font_style`
and `strikethrough` in addition to the existing fields. Font weights are
OpenType values from 1 through 1000. The slant distinguishes normal, italic and
oblique. The existing `bold` and `italic` flags remain available to callers.
An explicit numeric weight or slant wins when both forms appear in the same
overlay. A later legacy boolean overlay clears the inherited semantic field,
so `bold: false` still resets a previously inherited weight. Individual model
setters follow the same rule.

The XSHD loader and compiler retain these properties instead of reducing them
to boolean bold/italic. Named font weights and numeric variable weights map to
the same runtime representation. Programmatic ASTs are checked during
compilation as well as XML input during parsing.

`NativeHighlightingStyle::resolve` translates these plain values into FerroUI
`FontFamily`, `FontWeight`, `FontStyle`, `IBrush` and decorations. Its
`text_run_properties` method keeps the caller's font stretch, baseline,
culture, features and unspecified values. Underline and strikethrough can be
enabled or cleared independently without discarding inherited overline or
other decorations.

`RichTextDisplayExt` adds four presentation operations to `RichText`:

- `create_runs` constructs ordinary native `Run` values.
- `create_runs_with_context` also resolves named brush resources through a
  supplied styled element.
- `to_text_block` creates a native text block with formatted inlines.
- `apply_to_text_block` prepares every run before replacing an existing text
  block, so a bad brush or font does not leave a partially replaced value.

Core text remains lossless UTF-16. Native runs require Rust UTF-8 strings, so
isolated surrogates project to the replacement character. A surrogate pair
crossing a style boundary is emitted as one scalar using the leading unit's
style, matching the existing HTML projection. Unspecified native properties
remain inheritable from the text block containing a run.

`SimpleHighlightingBrush` holds an immutable native solid brush. Construction
from a mutable brush freezes its current color, opacity and transform. Its
equality and hashing use the color, as in the reference. The additional
`ResourceHighlightingBrush` resolves the current styled-element resource tree
and theme on each request. `resolve_highlighting_brush` first accepts native
color syntax and then resource names; `resource:Red` explicitly selects a
resource even though `Red` is a native color name. Returned runs are snapshots;
the adapter does not install new dynamic-resource subscriptions on them.

HTML exports numeric weights, exact slant, family fallback lists and combined
decorations. Family names are quoted and CSS-escaped before the style attribute
is HTML-escaped. Native hexadecimal colors use ARGB, while CSS hexadecimal
colors use RGBA; the exporter reorders alpha-bearing values. A resource brush
is resolved before export, and a solid brush's opacity is folded into alpha.
Non-solid brushes have no single CSS text color, so that property is omitted
from HTML while the native display can retain its brush.

## Clipboard payload and events

`EditorClipboard::prepare_copy` builds one data item with the universal
`DataFormat::text()` payload and a native HTML format. Plain text keeps the
selection's exact text and configured separator between independent selection
segments. It does not normalize newlines or expand tabs. HTML formatting can
expand tabs according to the editor's indentation size.

The pinned command handler normalizes copied newlines for the operating system
and normalizes pasted newlines/tabs for the current document options. This
adapter preserves the Rust editor's established exact-text clipboard behavior.
Marker interpretation restores the standalone rectangular-paste behavior;
the pinned command handler's active paste path reads only plain text.

The framework currently has no universal HTML data format or HTML conversion
layer. Consequently the adapter supplies one appropriate platform format:

| Target | Native format | Data type |
|---|---|---|
| Linux and browser targets | `text/html` | `String` fragment |
| Apple targets | `public.html` | `String` fragment |
| Windows | `HTML Format` | `Rc<[u8]>` CF_HTML payload encoded in UTF-8 |

Windows uses bytes because the generic native String adapter writes UTF-16,
whereas CF_HTML offsets count UTF-8 bytes. The existing Core generator computes
the fragment and document offsets. The adapter does not add redundant native
plain-text formats; FerroUI already converts its universal text format.

The following routed events expose the preparation and success boundaries:

| Event | Handler behavior |
|---|---|
| `CopyingToClipboard` | Inspect or mutate formats, replace the entire `DataTransfer`, cancel; inspect rich-format errors and copy/cut intent |
| `TextCopied` | Observe accepted platform writes and the customized plain-text value, if retained |
| `PastingFromClipboard` | Change text and marker classification or cancel before editing |
| `TextPasted` | Observe text and marker classification after a completed paste transaction |

Mutable event-argument clones share state. Cancellation or handled preparation
prevents the corresponding platform write or document edit. A failed rich
formatter leaves plain text available and exposes the formatting error to the
copying event and editor error state. A platform write failure does not raise
`TextCopied` or delete text.

`selected_rich_text` uses the installed TextMate token styles when present;
otherwise it consumes XSHD highlighted sections. Ranges and tokens are clipped
using UTF-16 offsets. A version check rejects rich construction if a callback
changes or replaces the document while formatting is prepared.

## Async transactions and ownership

Copy preparation is synchronous. The returned native future owns the platform
write and holds no editor or document borrow across `await`. The outgoing
transfer is not disposed by the editor because its lifetime belongs to the
clipboard service.

Cut preflights every selected range before publication. After the write
succeeds and success handlers have run, deletion proceeds only if document
identity, document version and selection still match the prepared state.
The session checks its current read-only provider again and applies deletion
as one undo unit. A changed document or selection leaves the successful copy
in place and returns `CopiedWithoutCut`. A newly protected selection reports
the editing error without deletion. This also covers changes made by a
`TextCopied` handler itself.

Reading obtains text and marker formats from one incoming data-transfer
snapshot. A small RAII guard calls its required `dispose` on success, error,
or dropping a pending read. Paste retains the existing editor behavior of
using the current selection when an asynchronous read finishes, but declines
to edit a replacement document. Preparation handlers run before any edit.

Rectangular selections add the string application format
`FerroUI.Edit.RectangularSelection` with value `"1"`. Ordinary selections do
not emit this marker. It adapts the reference's application byte marker to a
FerroUI-named string marker; it is not a promise of cross-application marker
interoperability. The marker remains separate from exact plain text.

Column paste preflights all rows, supports virtual padding, and commits one
undo unit. Insufficient existing rows or an unsuitable selection return a
non-mutating fallback result; the adapter then performs one ordinary paste.
An error such as a protected target row does not trigger fallback. CR, LF and
CRLF separate rows. The source's lone-CR counting limitation is corrected.

`FerroUI.Edit.WholeLineSelection` is also recognized. With one empty caret it
inserts at the logical line start. Version, document identity and read-only
state are checked after policy callbacks and again after the outer update
starts. Empty-selection whole-line copy is not yet an editor option in this
stage, so ordinary copy/cut still requires selected text and does not emit this
marker automatically.

## Verification and limits

The Core regression gate after semantic-style integration passed **264 tests**
with no failures, ignores or filtered tests. Seven new cases cover semantic and
legacy overlay precedence, individual setters, compiled XSHD metadata and
programmatic validation, alpha channel order, CSS family escaping and
independent decoration clearing.

Native integration regressions cover real formatted `Run`/`TextBlock` values,
native resource changes, exact rich/plain payloads, TextMate Unicode ranges,
CF_HTML byte offsets, customization/cancellation, deferred cut races,
read-only policy changes, rectangular fallback and atomicity, incoming
transfer disposal, and whole-line policy reentry. The combined integration
gate passed **271 Core tests, 91 control tests, and two demo tests**, with no
failures, ignores or filtered tests. This includes 21 native clipboard tests,
six rich-text/brush adapter tests, and three native formatting/height/rectangle
projection regressions. The command used the shared native feature union:

```sh
cargo test --offline --no-default-features \
  -p ferroui-edit-core -p ferroui-edit -p editor-demo --lib
```

These tests use FerroUI's actual native abstractions with deterministic test
render and clipboard services. They do not claim operating-system clipboard
interoperability or pixel parity across font/render backends. HTML class/style
callbacks remain concrete options rather than virtual extension hooks.
Live-document rich writers, automatic runtime-resource subscriptions on
constructed runs, empty-selection whole-line copying, and arbitrary custom
brush objects inside Core `HighlightingColor` remain separate work.
