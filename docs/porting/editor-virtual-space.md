# Virtual space and rectangular editing

This follow-up ports the virtual-space behavior in the pinned AvaloniaEdit
selection and command sources into `FerroUI.Edit.Core`. The source pin and the
complete inventory are recorded in [editor-source-map.md](editor-source-map.md).
It extends the existing Rust editing session; it does not create a second
document, undo stack, input system, or text buffer.

## Source responsibilities

| Upstream source | Rust implementation | Responsibility |
|---|---|---|
| `Editing/SelectionSegment.cs` | `editing/selection.rs::SelectionSegment` | Directed physical and virtual endpoints |
| `Editing/Selection.cs`, `SimpleSelection.cs`, `EmptySelection.cs` | `editing/selection.rs::Selection` | Normalization, primary selection, rectangle mode, change mapping |
| `Editing/RectangleSelection.cs` | `editing/rectangle_selection.rs` | Logical-line rectangle projection and explicit virtual coordinates |
| `Editing/Selection.cs::AddSpacesIfRequired` | `editing/editor_session/virtual_space.rs::EditorSession::with_virtual_padding` | Conditional materialization of virtual space |
| `Editing/RectangleSelection.cs::ReplaceSelectionWithText` | `editing/editor_session/virtual_space.rs::EditorSession::replace_virtual_selections` | Broadcast typing and bounded multiline paste |
| `Editing/RectangleSelection.cs::PerformRectangularPaste` | `editing/editor_session/rectangular_paste.rs::EditorSession::paste_rectangle` and `paste_rectangle_at` | Clipboard column paste, prepared native positions, protected-row filtering and final selection |
| `Editing/CaretNavigationCommandHandler.cs` | `editing/caret_navigation.rs` and `editing/editor_session/virtual_space.rs` | Grapheme navigation, virtual movement, persistent desired column |
| `Editing/Caret.cs::PositionChanged`, `Editing/CaretWeakEventHandler.cs` | `src/FerroUI.Edit/text_editor.rs::TextEditor::caret_position_changed` | Scoped notification of primary caret movement after committed updates |
| `Editing/EditingCommandHandler.cs` | `editing/editor_session.rs` and its `virtual_space` module | Delete, Backspace, Enter, Tab, protection checks, undo grouping |

The framework-independent layer uses logical lines and cell columns. The
native editor projects pointer positions, wrapped rows and proportional fonts
through FerroUI text layout. A cell projection does not claim equivalence to a
pixel projection for bidirectional text or a proportional font.

## Coordinate contract

Physical offsets count UTF-16 code units, consistently with the document,
anchors, completion and highlighting APIs. Virtual space counts additional
cells after a logical line's text, before its delimiter. It never counts UTF-16
units and never becomes part of the document until an insertion needs it.

`SelectionSegment` keeps its public `anchor` and `caret` offsets and adds
`anchor_virtual_space` and `caret_virtual_space`. Existing `new(anchor, caret)`
and `empty(offset)` constructors set both virtual values to zero. The explicit
constructor is:

```rust
SelectionSegment::with_virtual_space(
    anchor_offset,
    anchor_virtual_space,
    caret_offset,
    caret_virtual_space,
)
```

Endpoint ordering compares `(offset, virtual_space)` pairs. The `anchor_tuple`,
`caret_tuple`, `start_tuple`, `end_tuple`, `start_virtual_space`,
`end_virtual_space` and `is_reversed` helpers retain direction even when both
physical offsets are equal. `length()` remains the physical UTF-16 length.
`is_empty()` compares both coordinates, so a selected virtual range can have
zero physical length while remaining a nonempty selection. Text extraction
returns actual document text; it does not manufacture spaces for copying a
virtual range.

`EditorSession::set_selection` validates every physical endpoint and requires
each nonzero virtual endpoint to be exactly at a logical line end. The public
`set_caret_position(offset, virtual_space)` and `caret_virtual_space()` expose
the same representation for a single caret. Explicit positions are valid
independently of the navigation option; rectangle mode always permits virtual
navigation. Turning off ordinary virtual navigation clamps a nonrectangular
selection, while preserving an active rectangle.

Document changes map physical endpoints through the document's offset-change
map. Virtual endpoint affinity follows inserted text at an existing line end.
The session then checks the resulting logical line boundaries. A virtual count
is retained when its mapped offset is still a line end, and cleared when an
edit places that endpoint inside text, such as joining the following line.
The subscription owns a weak session reference and runs outside document
borrows; it does not retain the session through a document-reference cycle.

### Observing the primary caret

`TextEditor::caret_position_changed` returns a subscription guard. A handler can
query `caret_offset` and `caret_position` through a weak reference to the editor.
It runs when the primary caret changes UTF-16 location, virtual cells, or
soft-wrap affinity. Selection updates that keep the primary caret at the same
position leave this subscription quiet.

During an outer document update and its nested updates, movement produces one
notification after the document commits. Windows tests exercise wrap affinity,
virtual cells, document replacement, undo, and dropping the subscription.

## Editing behavior

| Operation | Result |
|---|---|
| Type nonempty text beyond EOL | Pad to the selection's starting virtual column, replace its real range, then place the caret after the inserted text |
| Delete a virtual-only range | Collapse to its lower virtual endpoint without inserting spaces, removing text, or adding undo history |
| Backspace beyond EOL | Move one virtual cell left without changing the document |
| Word Backspace beyond EOL | Materialize the original caret's virtual padding and leave the caret after it |
| Delete or word Delete beyond EOL | Replace the targeted newline and any indentation included in a word deletion with virtual padding, or insert the padding at the end of the document |
| Delete at a rectangular row boundary | Preserve line count; a rectangle never joins adjacent logical lines |
| Enter beyond EOL | Insert the configured newline and the installed strategy's normal indentation, without unused virtual padding |
| Single-line Tab with space conversion | Compute indentation from the active caret's logical UTF-16 column and add any virtual padding separately |
| Tab across multiple lines | Insert indentation at writable line starts. Preserve anchors at inserted line starts and exclude a line when the selection ends at its start |
| Shift+Tab | Remove the first writable fragment of one indentation prefix on each selected line |
| Left/Right beyond EOL | Move by virtual cells; extending a selection retains its anchor's virtual coordinate |
| Up/Down | Preserve the desired cell column across shorter lines, tabs and grapheme clusters |
| IME preedit beyond EOL in the native view | Render a transient virtual gap and composition text without changing document text or history; materialize padding only on committed input |

Padding uses spaces on a line containing ordinary text. When tab conversion is
disabled and the existing line is empty or contains only tabs, it uses complete
tab widths followed by remaining spaces. CR, LF and CRLF alone never request
padding. Allocation overflow or an unrepresentable padding allocation returns
`EditError::VirtualSpaceTooLarge` before the document changes.

Character navigation follows extended grapheme boundaries rather than splitting
surrogate pairs, combining sequences or emoji clusters. Tabs advance to the next
configured stop. Word navigation has no virtual word stops: moving left from
virtual space reaches the real line end, while moving right continues into the
following logical line. Up/Down retains the desired column even when ordinary
virtual space is disabled and an intermediate short line clamps the caret.
Horizontal movement, explicit selection changes and document edits reset that
remembered column. Movement beyond the first or last logical line leaves the
caret in place.

## Rectangular projection and paste

`Selection::from_rectangle_segments` records rectangle mode separately from
the ranges. That mode survives document mapping, virtual-only deletion and
single-line typing. A zero-width rectangle therefore remains a column-editing
selection even when every row has a zero physical range.

`RectangleSelection::to_document_selection(&TextDocument, tab_size)` reads only
the selected logical lines. It counts extended grapheme clusters as cells,
expands tab stops, and represents columns after each short line as virtual
space. Line numbers in this rectangle description are zero-based; document
line APIs remain one-based. Out-of-range rectangle rows clamp to the document.
The older string-based `to_selection` retains its clamped endpoint contract.
`to_selection_with_virtual_space` is the explicit string-based convenience
method for callers that need virtual endpoints.

Typing a string without a newline broadcasts that string to each selected row,
with independent padding and one document undo group. Every resulting row
retains a caret and rectangle mode. A string containing newlines uses the
source's bounded split: it produces at most as many replacement pieces as
selected rows. CRLF is one delimiter; standalone CR and LF are delimiters too.
If the text has fewer pieces, the remaining selected rows are untouched. If it
has more, the final piece retains all remaining text and newline delimiters.
Content is never silently truncated.

For example, pasting `Q\r\nR\nS\rT` into two rectangular rows inserts `Q` into
the first row and `R\nS\rT` into the second. It does not apply additional edits
to rows outside the selected rectangle. This multiline operation clears
rectangle mode and uses the first inserted row's physical column to place the
caret on the rectangle's original bottom logical line, clamped to existing
text, following the source's final-caret rule.

### Rectangular clipboard paste at an ordinary caret

The session also exposes the source's column-paste operation independently of
the native clipboard transport:

```rust
pub fn paste_rectangle(
    &self,
    text: &str,
    select_inserted_text: bool,
) -> Result<bool, EditError>;

pub fn paste_rectangle_at(
    &self,
    positions: &Selection,
    text: &str,
    select_inserted_text: bool,
) -> Result<(), EditError>;
```

`paste_rectangle` accepts one empty caret and projects its cell column onto
successive existing logical lines. It reads only the affected lines. It returns
`Ok(false)` without changing document text, selection, version, or undo history
when the current selection is nonempty or has multiple carets, when too few
existing lines remain, or when virtual space is disabled and the final row
cannot reach the initial physical UTF-16 column. The final-row condition follows
`PerformRectangularPaste`; a short intermediate row can still receive padding.
The caller can perform an ordinary paste after `Ok(false)`, preserving the full
input and allowing that ordinary operation to add lines at EOF. An error such
as `ReadOnly` or `StaleEditPlan` must not trigger that fallback.

The native `TextArea::paste_rectangle` prepares positions at the caret's shaped
horizontal coordinate, reading each existing visible line through FerroUI text
layout and skipping collapsed rows. It does not temporarily replace the live
selection. It verifies document identity, version and selection before handing
the positions to the core batch. A view without an arranged width uses the core
cell projection. The native clipboard adapter invokes this path for
`FerroUI.Edit.RectangularSelection` or a valid AvaloniaEdit-compatible byte marker.
Native formatting, transfer ownership, customization and fallback tests are
documented in [editor-rich-text-clipboard.md](editor-rich-text-clipboard.md).

CR, LF, and CRLF delimit the clipboard's rows. A trailing delimiter represents
an additional empty row and therefore participates in the fit check. Delimiters
in the existing document remain unchanged; the row separators do not replace
them. This explicitly fixes the source method's LF-only row count for standalone
CR text. The operation never silently extends the document to accommodate a
rectangle that cannot fit.

`paste_rectangle_at` accepts positions projected by a native view without first
changing the live selection. Its selection must be in rectangle mode, have one
empty segment per input row, and place those segments on distinct ascending
logical lines. The rows can skip folded lines, and their physical columns can
differ when proportional glyph geometry requires it. Virtual endpoints must be
at real logical line ends. A row count mismatch, a nonempty segment, a position
inside a line delimiter, or two carets on the same logical row returns an error
before mutation. The caller supplies positions from the current document; it
must not reuse an unversioned position plan across unrelated document edits.

Every writable nonempty row receives its own necessary padding. Empty and
protected rows preserve their virtual positions without inserting padding.
With `select_inserted_text = true`, the inserted ranges stay selected in
rectangle mode and exclude the materialized padding. The prepared selection's
primary row is preserved. This makes the selection flag effective for multiline
input as well; the source's implementation only restores an inserted selection
if replacement has left rectangle mode active. A skipped row contributes an
empty range. With the flag false, one caret ends on the final target row at the
first row's intended UTF-16 end column, clamped to that row. A single empty row
retains its virtual caret. A fully protected paste consumes the marker and
updates the caret without creating a document change or undo entry.

## Protection, change events and undo

Replacement plans use original document coordinates. Before any real mutation,
the session checks every affected range against the current read-only section
provider. Virtual padding belongs to the same insertion, so it cannot bypass a
protected insertion position. The complete plan is checked again after the
document's update-start event in case the host changes its policy there. If that
event edits the document, `EditError::StaleEditPlan` rejects the old plan before
applying any of its replacements. A host's independent event-handler edit is
not rolled back by that rejection.

The common prepared-edit path captures its document version before the first
read-only-provider callback. A provider that changes text during its initial
preflight cannot cause old coordinates to commit. Replacement, indentation,
newline insertion, transformations and rectangle paste all use this guard.
Custom indentation callbacks are checked for document changes too. Column
paste checks protection while planning, then revalidates every planned
insertion before and after update-start. A callback can invalidate the
prepared document, read-only policy or view binding before any row commits.
Rectangle drops check every target before inserting, including moves whose
source may be removed by another editor. New clipboard selections are
installed after the planned insertions succeed and before
consolidated text-change notifications. Reentrant notification edits therefore
map the final selection through the existing document subscription without
outstanding selection borrows.

Accepted edits use the document's shared batch operation and one update guard.
`TextDocument::apply_edits_with_mapping` preserves the requested anchor mapping
for every replacement, including character-preserving transformations. The
complete batch is accepted before individual change callbacks run. A callback
changing read-only policy after the first edit cannot leave that accepted batch
half applied; subsequent operations use the new policy. Overlapping batches
are rejected before publishing update-start, and a stale plan does not undo an
independent host callback edit.
Temporary surviving anchors track the replacement starts through edits earlier
in the document, keeping adjacent replacement carets on their own sides of a
boundary. No-op virtual edits do not create a document update or undo entry.
Selection values are copied before mutation; document callbacks update the
session without outstanding selection borrows.

Distinct virtual carets can have different columns at the same physical EOL.
The current session rejects a combined edit of those positions with
`EditError::AmbiguousVirtualCarets` before changing text or selection. Normal
rectangles have one segment per logical line and do not encounter this case.
This explicit limitation is preferable to assigning a silently incorrect order
to multiple inserts at one physical offset. The source does not provide the
Rust session's general multicaret extension.

## Scope

This component does not reproduce the managed selection inheritance hierarchy.
The logical projection does not expose custom visual-line elements, wrapped
line affinity, native glyph geometry, or bidirectional selection hit testing;
those are responsibilities of the FerroUI presentation layer. Full upstream
type and member parity is tracked in the
[source inventory](editor-source-map.md).

### Native navigation and pointer selection

The active box caret follows the formatted rows and retains its desired
horizontal position during vertical movement. The editor projects the
rectangle onto each logical line's first row for drawing the selection and
applying edits. Caret queries, selection notifications, scrolling and caret
painting use the active wrapped position. Home toggles between the content
start and logical line start on the first row. End counts the current
formatter row's source columns and keeps its trailing affinity.

Box pointer selection captures the shaped caret on the clicked row for its
anchor. A drag resolves its active caret on the first formatted row of the
destination logical line. The rectangle uses the horizontal positions of those
snapped carets. At a bidirectional boundary the resulting physical position can
differ from the pointer's X coordinate. The forward and reverse drag cases
preserve their directional anchor and follow the respective caret edges
identified by the pinned editor.

Native rectangular paste and drop use the source caret's wrapped-row
affinity to find its horizontal position. Every insertion projects that
position onto its logical line's first formatted row, including the starting
line. The pinned editor and native Rust regressions agree on the inserted
text and one-step undo for a paste from a continuation. Rectangular selection
still edits the contiguous logical range between each row's two endpoints.

Undo restores the original visual caret after the document has restored the
logical selection. This preserves the trailing side of a soft break and the
active continuation row of a box selection. Each attached view records its own
position. Continued groups restore the oldest position, and text edits replayed
on redo map the caret through the forward changes.

Overstrike is a native text-area mode. `Insert` toggles it when the host enables
the shortcut. Text input replaces the next grapheme at each empty caret, and
line ends and virtual cells remain insertion points. The normal selection,
read-only checks and undo transaction apply. The caret covers the shaped glyph
and spans one space at a line end or in virtual space.

Double-click and triple-click drags extend the original anchored word or line
span. Gutter drags include the line delimiter. Captured selection projects the
pointer's Y coordinate through the nearest viewport edge when it leaves the
view above or below. The view saves the original point and projects it again
after scrolling. Capture loss ends the gesture. Character and rectangle drags
retain their own anchor forms.
Control-click starts a word selection with an anchor at the pressed word.
`extend_selection_on_mouse_up` controls the final extension. A plain click
inside selected text keeps the selection until release so the same gesture can
start a drag once the pointer has moved more than two device-independent units.

[Pinned selection handler](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Editing/SelectionMouseHandler.cs)

The editor now preserves the measured extent across pointer entry and exit.
Formatting inputs still invalidate line layout. When the viewport narrows,
the extent also drops the old viewport's width minimum so horizontal scrolling
stays within the new content bounds. The native regression checks the retained
visual lines, measured extent and scroll offset after a routed pointer exit.
The pinned text view also rebuilds cached elements for font and foreground
changes and updates its drawing decorations in their own handlers.
[Pinned text view property handler](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Rendering/TextView.cs#L1973)

`RightClickMovesCaret` is off by default. When enabled, a right click inside
the selection moves the caret and preserves the selected text for context
commands. In a multicaret selection, the clicked segment becomes the primary
one. A click outside the selection places an empty caret. The pointer event
continues to the native context-menu route.

The editor uses FerroUI's drag source and routed drop events. It advertises
plain text, HTML and the rectangle marker through native data transfer and lets
hosts amend or cancel the transfer. Control requests a copy. Moves require
permission to delete every source range. An editor drop previews the target
caret, applies the destination's newline and tab options, and selects the
inserted text. Same-document moves group the source deletions and destination
insertions into one undo unit, including rectangle rows. Each editing session
records its original selection as an optional action in the document undo
group. An external move deletes the source only while the original document
revision, selection and permissions still match.

### Touch selection

The text viewport accepts ordinary touch swipes through the scroll viewer's
gesture recognizer. A tap places the caret, a double-tap selects the word,
and a hold captures a word selection that can extend by word across lines.
Tap and hold use the same formatted visual lines as mouse hit testing.
The held selection survives release, including when the finger returns to
the original word.



### Remaining presentation and clipboard work

Native rectangular clipboard transport and shaped positions are implemented.
Ordinary text replacement runs only after a non-mutating `Ok(false)`, while
active rectangular selections retain bounded replacement. Further desktop
backends and clipboard formats remain open.
