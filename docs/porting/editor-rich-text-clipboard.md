# Native rich text and editor clipboard

This stage connects the editor's UTF-16 rich-text model to FerroUI's native text
and clipboard services. The pinned reference is recorded in
[editor-textmate.md](editor-textmate.md). Core text and styles remain
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
| `Highlighting/RichTextColorizer.cs` | `src/FerroUI.Edit/rich_text_colorizer.rs` | Applies a shared model to the live visual line and resolves native resources |
| Archived `Highlighting/RichTextModelWriter.cs` | `src/FerroUI.Edit.Core/highlighting/rich_text_writer/document.rs` | Live UTF-16 writes, tracked insertion point, scoped formatting and shared undo with the document |
| `Highlighting/Xshd/V2Loader.cs`, `XmlHighlightingDefinition.cs` | `src/FerroUI.Edit.Core/highlighting/xshd/loader.rs`, `compiler.rs` | Full runtime style retention and validation for XML and programmatic syntax trees |
| `Highlighting/HighlightingColor.ToCss`, `HtmlOptions.cs`, `HtmlRichTextWriter.cs` | `src/FerroUI.Edit.Core/highlighting/html_clipboard.rs` | Inline CSS, custom span classes and styles, native ARGB to CSS RGBA conversion, safe family escaping |
| `Highlighting/HighlightingBrush.cs` (also declares `SimpleHighlightingBrush`) | `src/FerroUI.Edit/highlighting_brush.rs` | Native immutable brush, color equality/hash, context-aware resolution; adds resource-tree brush resolution |
| Archived `Highlighting/RichText.cs::CreateRuns`, `ApplyColorToTextElement` | `src/FerroUI.Edit/rich_text_adapter.rs` | Working FerroUI `Run`/`TextBlock` adapter and shared native formatter properties; these methods are commented in the pinned source |
| `Highlighting/HighlightingColorizer.cs` style application | `src/FerroUI.Edit/rich_text_adapter.rs`, `text_area.rs` | Native font/brush/decoration translation shared with the editor's formatted lines |
| `Editing/Selection.cs::CreateDataObject`, `EditingCommandHandler.cs` copy/cut/paste | `src/FerroUI.Edit/editor_clipboard.rs`, `text_editor.rs` | Framework data transfer, rich/plain selection, cancellable preparation, async ownership, current-policy cut and paste |
| `TextEditorOptions.cs::CutCopyWholeLine`, `Editing/EditingCommandHandler.cs::CanCopy`, `CanCut`, `CopyWholeLine` | `src/FerroUI.Edit.Core/editing/editor_session.rs`, `src/FerroUI.Edit/commands.rs`, `editor_clipboard.rs` | Default-enabled whole-line copy/cut for one empty caret, complete line range, command option notifications and native clipboard publication |
| `Editing/EditingCommandHandler.cs::GetTextToPaste`, archived `LineSelectedType` publication | `src/FerroUI.Edit/editor_clipboard.rs` | Whole-line marker restores line-start insertion; marked line paste normalizes destination delimiters and optional tabs; marker publication is a restoration of commented source behavior |
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

Hosts can implement `HtmlExportOptions` for rich text, highlighted lines,
document fragments, and CF_HTML export. The policy returns an `HtmlSpanStyle`
for each effective style run. It can emit a class, supply custom CSS, combine
the two, or leave the run unwrapped. The exporter escapes both attribute values.
The standard `HtmlOptions` still supplies the inline CSS used by editor Copy.

## Live rich-text colorizer

Add `RichTextColorizer` to `TextArea::line_transformers()` with a shared
`Rc<RefCell<RichTextModel>>`. The transformer applies each displayed logical
line's UTF-16 style ranges over the current formatting. Missing properties
inherit from each element, so a model can add a foreground or font weight
without losing the font or decoration from an earlier transformer.

Metadata-only ranges leave the run boundaries intact. Native brush resources
resolve through the text area's resource tree at layout time. Model backgrounds
use the editor's paint pass before selection. The owner still manages model
offsets when the document changes. Release the model's mutable borrow after
changing the overlay. Call `TextArea::redraw_range()` for the edited offsets or
`TextArea::redraw()` for the whole view. The same model can serve multiple
views, each of which requests its own redraw.

Redraw and extension collection changes queue an accessibility formatting
notification. The editor coalesces requests on the UI dispatcher and waits
until visual-line construction and rendering finish. A notification queued
for a replaced document is discarded. Existing text ranges can read the new
style while retaining their text and offsets.

External Windows UI Automation and Linux AT-SPI clients each received one
event for two redraw requests and read the updated color from the same emoji
range. The Linux client received `TextAttributesChanged` and reported the
emoji at one Unicode character offset. Both clients read the same document
text before and after redraw.

A metadata-only span left two adjacent characters together. Updating the same
model and redrawing made the suffix green. Removing the transformer restored the
base formatting. The native regression also resolves the blue foreground from a
named resource. It applies a size-only overlay as well. The pinned colorizer
inherits the base size in that case.

## Live document writer

`RichTextDocument` binds formatting to an existing `TextDocument`.
`RichTextDocument::writer` creates a `DocumentRichTextWriter` at a UTF-16
offset. Its insertion anchor follows document edits and survives deletion.
It supports the same formatting spans and indentation as the buffered writer.
`write_line` uses the document's detected newline sequence and inserts the text
and delimiter as one edit. Callers can group multiple writes with the standard
document update guard.

Each successful write publishes its text and styles before document change
observers run. Undo removes the inserted formatting with the text. Redo
reapplies the formatting before observers run, using the style data retained
by that edit. An ordinary document edit remaps existing style boundaries through
the document's offset map. `snapshot` returns lossless UTF-16 text and its
current styles. Its result works with `RichTextDisplayExt` for native runs and
text blocks. Editor syntax highlighting remains a separate service.

The live writer restores the behavior described by the archived
[rich-text model writer](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Highlighting/RichTextModelWriter.cs).

## Clipboard payload and events

`EditorClipboard::prepare_copy` builds one data item with the universal
`DataFormat::text()` payload and a native HTML format. Plain text keeps the
explicit selection's exact text and configured separator between independent
selection segments. Selected-text copy does not normalize newlines or expand
tabs. HTML formatting can expand tabs according to the editor's indentation
size. With one empty caret and `cut_copy_whole_line` enabled, the payload instead
contains the current logical line and a whole-line marker; its newlines follow
the operating system, matching `CopyWholeLine` in the pinned source.

The pinned command handler normalizes copied newlines for the operating system
and normalizes pasted newlines/tabs for the current document options. This
adapter preserves the Rust editor's established exact-text behavior for explicit
selections, ordinary paste and rectangular paste. Whole-line copy and insertion
now apply the reference's newline rules; whole-line insertion also applies
`convert_tabs_to_spaces`. Marker interpretation restores richer insertion
behavior; the pinned command handler's active paste path reads only plain text.

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

`selected_rich_text` uses the installed TextMate theme's explicit token styles
when present and otherwise consumes XSHD highlighted sections. The editor's
canvas palette stays in the editor, so a copied unstyled span can inherit its
destination's colors. Ranges and tokens are clipped using UTF-16 offsets. A
version check rejects rich construction if a callback changes or replaces the
document while formatting is prepared.

## Async transactions and ownership

Copy preparation is synchronous. The returned native future owns the platform
write and holds no editor or document borrow across `await`. The outgoing
transfer is not disposed by the editor because its lifetime belongs to the
clipboard service.

Cut checks document and editor read-only state before writing. An ordinary,
rectangular, or implicit whole-line cut copies its full target and then
deletes spans that remain writable after the clipboard write and
`TextCopied` handlers. A wholly protected ordinary selection stays selected.
A wholly protected box collapses to a column. Neither case adds an undo entry.
Explicit accessibility ranges and independent multicaret cuts check the full
deletion target before publication.

Deletion proceeds only while document identity, version and selection still
match the prepared state. Whole-line cut keeps its prepared range separately
from the empty user selection. Selection and whole-line cuts check that the
target is still current after policy callbacks and each document update starts.
A changed document or selection during the asynchronous write leaves the
successful copy in place and returns `CopiedWithoutCut`. Ordinary and
rectangular cuts group their text changes in one undo step. Whole-line cut
commits writable spans from right to left, each with its own undo entry. If a
callback detaches or changes the document between spans, earlier removals
remain undoable and the next removal is rejected.

Reading obtains text and marker formats from one incoming data-transfer
snapshot. A small RAII guard calls its required `dispose` on success, error,
or dropping a pending read. Paste retains the existing editor behavior of
using the current selection when an asynchronous read finishes, but declines
to edit a replacement document. Preparation handlers run before any edit.
Paste into an ordinary selection replaces its writable spans and places the
insertion in the last writable span. Paste into a rectangular selection
applies the same rule to the affected rows. For these selections the Paste
command is enabled when the primary caret admits insertion. A column paste
projected from one empty caret by a clipboard marker skips protected rows.
Even when all destination rows are protected, the marker controls the caret
placement and the document stays unchanged.

Rectangular selections publish `FerroUI.Edit.RectangularSelection` with value
`"1"`. On Windows they also publish the classic AvalonEdit one-byte marker
`AvalonEditRectangularSelection` as a separate format on the same transfer item,
and the clipboard and drop handlers accept a valid marker of that name without
a FerroUI marker. Ordinary selections publish neither marker. Other platforms
carry the rectangle through the FerroUI application format alone. The port does
not publish or read the upstream framework's prefixed marker names, so
rectangular selections from applications built on it arrive as plain text.
The classic Windows name comes from [AvalonEdit](https://github.com/icsharpcode/AvalonEdit/blob/master/ICSharpCode.AvalonEdit/Editing/RectangleSelection.cs).

The browser adapter uses web custom formats for FerroUI application data.
The MIME subtype carries a hex-encoded identifier and a text or bytes tag.
The encoding preserves the identifier's original case when the browser
normalizes the MIME name. Browsers that skip custom formats still receive
their supported standard formats. The web custom format is a browser
transport for the FerroUI marker.
[Chrome's web custom clipboard formats](https://developer.chrome.com/blog/web-custom-formats-for-the-async-clipboard-api)

The map needs a JSON codec, so it sits behind the non-default
`chromium-clipboard-map` feature of `ferroui-base`, which `ferroui-edit` enables.
An application without the editor neither publishes nor reads the map and links
no JSON library.

The Windows clipboard publishes FerroUI application formats to Chromium's
`Web Custom Format Map` and numbered native slots alongside their existing
OLE formats. A browser-written map makes the same typed marker available
to the native editor. The reader checks map size, reads only advertised
slots in Chromium's fixed range, and prefers an existing native
representation. The classic marker above remains available for native
clipboard consumers. OLE drag-and-drop continues to use native
formats. The independent OLE fixtures exercise each direction with
UTF-8 text, binary values containing zero bytes, plain text, and a
reserved slot already in use.
[Chromium's Windows clipboard format registry](https://chromium.googlesource.com/chromium/src/+/main/ui/base/clipboard/clipboard_format_type_win.cc)

The [Windows round-trip test](editor-browser.md) verifies the corresponding
browser and native editor commands on an isolated Windows system clipboard.
Rectangular and whole-line paste both preserve their exact document edits
and one-step undo.

The X11 clipboard uses the same codec with Chromium's X11 target names.
It bounds both direct and incremental reads of the optional map and checks
clipboard ownership before accepting the payload. An owner that refuses a
target returns immediately, allowing the editor to continue with its other
formats. X11 primary selections and drag-and-drop retain their existing
selection handling. A live test verifies the marker, exact pasted text,
and undo for rectangular and whole-line copies in both directions.
[Chromium's X11 clipboard format registry](https://chromium.googlesource.com/chromium/src/+/12a7862a280dbb36a57c5e6f38c4a21f3c77ea6c/ui/base/clipboard/clipboard_format_type_aura.cc)

Wayland windows and popups expose the platform clipboard to native controls.
The clipboard data source publishes the same Chromium map and slots as X11.
The reader resolves only advertised slots, preserves existing native
formats, and checks the offer cookie before and after asynchronous pipe reads.
Chromium metadata has a 64 KiB limit, and metadata and decoded slot reads
have a two-second deadline.
The [Wayland round-trip test](../../scripts/browser/tests/editor_clipboard_wayland.test.mjs)
runs on private Sway and Weston compositors with the browser sample in
Software2D mode. Both editors connect directly to Wayland. Xvfb supplies
input to the compositor and `wl-paste` inspects the marker payload.

```sh
cargo build --locked -p editor-demo --no-default-features --features wayland
node scripts/browser/tests/editor_clipboard_wayland.test.mjs SITE target/debug/editor-demo
node scripts/browser/tests/editor_clipboard_wayland.test.mjs SITE target/debug/editor-demo --weston
```

The macOS clipboard uses Chromium's `org.w3.web-custom-format.map` and
numbered pasteboard types. It publishes compatible aliases from the first
item that provides plain text. A pasteboard without plain text uses the first
item with compatible application data. The reader resolves aliases within
each pasteboard item and verifies the change count on native reads.
Native drag data uses its existing representations. The Rust fixtures
exercise item boundaries, reserved slots, typed payloads and stale
pasteboard reads through the generated interfaces. AppKit compilation and
a live Chromium exchange on macOS remain open.
[Chromium's macOS clipboard format registry](https://chromium.googlesource.com/chromium/src/+/12a7862a280dbb36a57c5e6f38c4a21f3c77ea6c/ui/base/clipboard/clipboard_format_type_mac.mm)

Column paste preflights writable rows, supports virtual padding, and commits
one undo unit. Insufficient existing rows or an unsuitable selection trigger
one ordinary paste. A stale edit plan or a read-only document returns an
error. CR, LF and CRLF separate rows.

`FerroUI.Edit.WholeLineSelection` is published automatically for an implicit
whole-line copy or cut. It has the string value `"1"` on the same native item as
plain text and HTML. Windows additionally publishes and accepts the one-byte
`MSDEVLineSelect` format used by [classic AvalonEdit](https://github.com/icsharpcode/AvalonEdit/blob/master/ICSharpCode.AvalonEdit/Editing/EditingCommandHandler.cs).
A valid rectangle marker takes precedence over a valid whole-line marker.
Readers accept byte `1` and tolerate zero-filled OLE allocation padding on
Windows. Invalid marker values are ignored. An editor-generated native alias is
added after copy customization when the corresponding FerroUI marker remains.
Handlers can remove the FerroUI marker to suppress those aliases or supply a
native alias explicitly.

## Whole-line command behavior

The `EditorOptions::cut_copy_whole_line` field defaults to `true`, matching
[`TextEditorOptions.CutCopyWholeLine`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/TextEditorOptions.cs).
The implementation follows the line-range and empty-line decisions in
[`EditingCommandHandler.CopyWholeLine`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Editing/EditingCommandHandler.cs),
while retaining the Rust editor's cancellable events and awaited clipboard
transaction. The source's whole-line marker and HTML publication remain
commented; their working FerroUI equivalents are intentional restorations.

| Situation | Copy/cut behavior |
|---|---|
| One empty caret, option enabled | Copy the caret's logical line from its UTF-16 offset through its existing delimiter; a successful cut removes that exact original range |
| Blank line with a delimiter | Copy or cut the delimiter as the line's text |
| Nonempty final line without a delimiter | Copy or cut its existing text; do not append a new delimiter |
| Empty final line, including an empty document | Return `ClipboardCopyOutcome::Empty` without a platform write, success event or undo item |
| One empty caret, option disabled | Copy and cut are unavailable and return an empty result |
| Explicit selection, including a rectangle | Keep the existing selected-text behavior and rectangle marker |
| Multiple empty carets, including an empty rectangle | Return an empty result; multicaret whole-line semantics have no counterpart in the pinned single-caret command and are not invented here |

Preparing a line never temporarily selects it. Selection observers and
`selected_rich_text` continue to see the user's actual selection. Rich export
uses the separately captured line range with the same TextMate/XSHD style
resolution as an explicit selection. A customization handler may replace or
remove the outgoing formats without redirecting a pending cut's source range.

`EditorCommand` observes the native `Options` property, so bound copy and cut
commands raise their availability notification when this option changes. With
the option enabled, their basic eligibility includes an empty final line, as
in the reference, although execution produces no payload there. A protected
portion of the line stays in the document and appears in the copied text.
Undo restores each removed span in reverse commit order.

An incoming whole-line marker with one empty caret inserts at the logical line
start, independent of the destination's `cut_copy_whole_line` option. CR, LF and
CRLF in the payload are normalized to that line's delimiter. An unterminated
last line uses the previous line's delimiter; an empty or single unterminated
document uses the operating-system newline. If `convert_tabs_to_spaces` is
enabled, every tab becomes `indentation_size` spaces, following the source's
fixed-size replacement rather than visual tab-stop expansion. The session's
validated indentation size is used. No final delimiter is invented for a
payload that lacks one.

The pasting event sees the incoming payload before normalization and may change
its text or classification. The success event reports the text actually
inserted after normalization. An explicit selection or multiple carets cause
ordinary selected-text insertion instead of line-start insertion; those
fallbacks preserve the exact payload. Whole-line paste checks version, document
identity, selection and read-only state after policy callbacks and again after
the outer update starts, then commits one undo item. A host callback that moves
the caret cannot redirect that prepared insertion.

## Tests and limits

```sh
cargo test --no-default-features \
  -p ferroui-edit-core -p ferroui-edit -p editor-demo --lib
```

These tests use FerroUI's native abstractions with deterministic test render
and clipboard services. They do not claim operating-system clipboard
interoperability or pixel parity across font and render backends. Automatic
runtime-resource subscriptions on constructed runs, and arbitrary custom brush
objects inside Core `HighlightingColor`, remain separate work. Selected-text
newline and tab normalization still differs from the pinned command handler as
described above. Clipboard exchanges with other editors and marker transport on
further desktop backends remain open.
