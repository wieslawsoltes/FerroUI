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
| `Editing/RectangleSelection.cs::PerformRectangularPaste` | `editing/editor_session/rectangular_paste.rs::EditorSession::paste_rectangle` and `paste_rectangle_at` | Clipboard column paste, prepared native positions, atomic preflight, final selection |
| `Editing/CaretNavigationCommandHandler.cs` | `editing/caret_navigation.rs` and `editing/editor_session/virtual_space.rs` | Grapheme navigation, virtual movement, persistent desired column |
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

## Editing behavior

| Operation | Result |
|---|---|
| Type nonempty text beyond EOL | Pad to the selection's starting virtual column, replace its real range, then place the caret after the inserted text |
| Delete a virtual-only range | Collapse to its lower virtual endpoint without inserting spaces, removing text, or adding undo history |
| Backspace beyond EOL | Move one virtual cell left; word Backspace reaches the real line end |
| Delete beyond EOL | Delete the following real newline when one exists; do not add padding |
| Delete at a rectangular row boundary | Preserve line count; a rectangle never joins adjacent logical lines |
| Enter beyond EOL | Insert the configured newline and the installed strategy's normal indentation, without unused virtual padding |
| Tab with space conversion | Reach the next tab stop using the actual visual column plus virtual cells |
| Tab across a multiline rectangle | Indent its selected logical lines, following the source command's multiline rule |
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

Every nonempty row receives its own necessary padding. Empty rows preserve their
virtual positions without inserting padding or creating a spurious undo entry.
With `select_inserted_text = true`, the inserted ranges stay selected in
rectangle mode and exclude the materialized padding. The prepared selection's
primary row is preserved. This makes the selection flag effective for multiline
input as well; the source's implementation only restores an inserted selection
if replacement has left rectangle mode active. With the flag false, one caret
ends on the final target row at the first inserted row's resulting physical
column, clamped to that row. A single empty row retains its virtual caret.

## Protection, change events and undo

Replacement plans use original document coordinates. Before any real mutation,
the session checks every affected range against the current read-only section
provider. Virtual padding belongs to the same insertion, so it cannot bypass a
protected insertion position. The complete plan is checked again after the
document's update-start event in case the host changes its policy there. If that
event edits the document, `EditError::StaleEditPlan` rejects the old plan before
applying any of its replacements. A host's independent event-handler edit is
not rolled back by that rejection.

The clipboard-paste plan also captures its document version before the first
read-only-provider callback. A provider that changes text during its initial
preflight cannot cause the old row coordinates to commit. Protection is checked
for the complete insertion plan before and after update-start; a protected row
rejects the entire paste. This atomic error contract is deliberate: the source's
rectangle replacement can skip individual protected row insertions. New
clipboard selections are installed only after all insertions succeed and before
consolidated text-change notifications. Reentrant notification edits therefore
map the final selection through the existing document subscription without
outstanding selection borrows.

Accepted edits use the document's shared batch operation and one update guard.
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

## Verification and remaining boundaries

The standalone document/editing suite passes **165 tests**, including **21
virtual-space regressions** and **17 rectangular clipboard-paste regressions**,
using the real source modules and the existing
regex and Unicode-segmentation dependencies. The tests cover reversed virtual
ranges, distinct virtual carets, short-line rectangles, CRLF, supplementary and
combining text, navigation, tab-aware padding, no-op history, protected row
preflight, undo/redo, multiline paste, update-start changes and allocation
overflow. They also verify that virtual-only selections at offset zero do not
underflow the line-selection calculations in indentation or transformations.
The integrated gate including this paste service and completion controls passes
**257 core tests, 61 native editor control tests and 2 EditorDemo tests**, with no
failures or filtered tests. The
native regressions exercise formatter round trips, short-line box typing,
folded-line rectangle geometry, wrapped-row affinity and notification order,
page movement with virtual anchors, Shift+Home anchor preservation, routed
Alt pointer press/release, keyboard rectangle anchors, and virtual cells after
bidirectional text. IME composition tests verify the same virtual screen position
before commit, supplementary composition text, cancellation, unchanged document
version/history during preview, and one-step undo after committing with padding
in both Latin and mixed bidirectional lines. These tests use the actual FerroUI
control and layout APIs.

The new clipboard-paste tests cover all three newline forms, trailing empty
rows, unchanged document delimiters and suffixes, EOF fallback, final-row fit,
short intermediate lines, tab padding, supplementary and combining text,
prepared native positions that skip lines, selection excluding padding, all-row
protection, document and session read-only flags, update-start policy changes,
document changes during both preflight stages, no-op history, one-step undo and
redo, allocation overflow, and reentrant consolidated notifications. Native
clipboard transport validation is recorded separately when that adapter lands.

This component does not reproduce the managed selection inheritance hierarchy.
The logical projection does not expose custom visual-line elements, wrapped
line affinity, native glyph geometry, or bidirectional selection hit testing;
those are responsibilities of the FerroUI presentation layer. Full upstream
type and member parity is tracked independently of these regression counts in
the [source inventory](editor-source-map.md).


### Explicit presentation and clipboard follow-ups

- Wrapped box-selection caret movement still advances between visible logical
  lines. The upstream box command first navigates wrapped visual rows and then
  projects its rectangle onto each logical line's first row. Matching that
  distinction requires retaining the box caret's wrapped-row position
  independently of the projected row segments; first-row rectangle projection
  itself follows the source.
- The core now supports atomic rectangular clipboard paste at an ordinary caret
  and explicitly projected native positions. The native transport must preserve
  a separate rectangular-selection marker and invoke ordinary text replacement
  only after `Ok(false)`. Active rectangular selections continue to use the
  bounded replacement operation described above. Native marker interoperability
  and clipboard round-trip validation are tracked with the presentation adapter.
- The core `LineStart` movement goes directly to logical BOL. The source's
  first-nonwhitespace smart-Home toggle is a separate remaining command behavior.

These are specific remaining behaviors, not failures hidden by a source-name
match. The source inventory continues to classify the corresponding command
and rectangle sources as adapted.
