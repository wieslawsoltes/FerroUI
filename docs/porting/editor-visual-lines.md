# Visual-line elements and native formatting

This report tracks the rendering extension contracts from AvaloniaEdit revision
`7277e36366d754219d7ede16b6613786b32caf4c`. The derived sources retain the
copyright and MIT license in `src/FerroUI.Edit/NOTICE.md`. The implementation
uses FerroUI's existing text source, text formatter, text layout, drawable runs,
brushes, routed events, cursor service and URI launcher.

## Source map

Upstream paths are relative to `src/AvaloniaEdit/`; Rust paths are relative to
`src/FerroUI.Edit/`.

| Upstream source | Rust source and contract | Adaptation |
| --- | --- | --- |
| `Rendering/VisualLineElement.cs` | `visual_line_element.rs`: `VisualLineElement`, `VisualLineElementData` | Object-safe element trait with independently owned lengths, UTF-16 offsets and mutable construction properties. Checked splitting, caret mapping, whitespace and pointer hooks remain element responsibilities. |
| `Rendering/VisualLineElementTextRunProperties.cs` | `visual_line_element_text_run_properties.rs`: `VisualLineElementTextRunProperties` | Copies native property values and collection handles into independently owned collections. Brush setters and run snapshots use native immutable brushes; native decoration objects retain their shared object semantics. |
| `Rendering/VisualLineText.cs` | `visual_line_text.rs`: `VisualLineText` | Splittable one-to-one UTF-16 text; native `TextCharacters`; document-aware word and grapheme movement across styling boundaries. |
| `Rendering/ITextRunConstructionContext.cs` | `i_text_run_construction_context.rs`: `ITextRunConstructionContext`, `VisualLineBuildOptions` | Stable document snapshot and optional weak native view. Text slices reuse a cached memory region. Mutable builder access is passed separately to transformers. |
| `Rendering/ITextViewConnect.cs` | `i_text_view_connect.rs`: `ITextViewConnect` | Explicit add/remove hooks for native view ownership. |
| `Rendering/VisualLineElementGenerator.cs` | `visual_line_element_generator.rs`: `VisualLineElementGenerator`, `VisualLineElementGeneratorRef` | Ordered generation with a scoped context argument. Reference wrappers compare `Rc` identity for native observable collections. |
| `Rendering/VisualLineConstructionStartEventArgs.cs` | `visual_line_construction_start_event_args.rs`: `VisualLineConstructionStartEventArgs` | First logical line in the viewport construction batch. The host raises the event before snapshotting extension collections. |
| `Rendering/VisualLine.cs` | `visual_line.rs`: `VisualLineBuilder`, `VisualLine` | Construction and immutable retained state are separate Rust types. The builder generates and transforms elements; the retained line provides document/visual mappings and native layout creation. Viewport geometry remains a host responsibility. |
| `Rendering/VisualLineTextSource.cs` | `visual_line.rs`: `VisualLineTextSource` | Native `ITextSource` adapter, observable extension errors, preceding-text validation and native EOL runs. Kept with `VisualLine` because it accesses the same immutable state. |
| `Rendering/IVisualLineTransformer.cs` | `visual_line_transformer.rs`: `IVisualLineTransformer`, `VisualLineTransformerRef` | Trait receives scoped mutable access to the builder before shaping. |
| `Rendering/ColorizingTransformer.cs` | `visual_line_transformer.rs`: `ColorizingTransformer` | Callback-based Rust base behavior, checked recursive use and connection hooks. |
| `Rendering/DocumentColorizingTransformer.cs` | `visual_line_transformer.rs`: `DocumentColorizingTransformer`, `DocumentLineColorizer` | Absolute document offsets restricted to the current logical line; visits visible continuations after generated multiline elements. |
| `Rendering/FormattedTextElement.cs` | `formatted_text_element.rs` | Native `FormattedText`, `TextLayout`, `TextLine`, or custom drawable content with native size, baseline and drawing. |
| `Rendering/SingleCharacterElementGenerator.cs` | `single_character_element_generator.rs`: `SingleCharacterElementGenerator`, `IBuiltinElementGenerator` | Native space glyphs and control-character labels; control label display columns affect following tab stops. |
| `Rendering/TabElementGenerator.cs` | `tab_element_generator.rs`: `TabElementGenerator`, `TabTextElement` | Required generator on long lines; tabs continue from the visual line's display columns. A wide configured marker still reserves one space width. |
| `Rendering/LinkElementGenerator.cs` | `link_element_generator.rs`: `LinkElementGenerator`, `MailLinkElementGenerator`, `LinkMatch` | Reuses the core bounded regex/search service. Custom matching strategies, URI resolution and element factories are supported. |
| `Rendering/VisualLineLinkText.cs` | `visual_line_link_text.rs`: `VisualLineLinkText`, `OpenUriRoutedEventArgs`, `LinkTextStyle` | Splitting preserves URI and click policy. Native routed URI event, modifier checks, hand cursor and top-level launcher. |
| `Rendering/PointerHoverLogic.cs` | `pointer_hover_logic.rs`: `PointerHoverLogic`, four routed `TextArea` hover events | Native input subscriptions and a dispatcher timer retain the original position through movements of at most two units per axis. Detach and drop cancel pending work. |
| `Folding/FoldingElementGenerator.cs` | `folding_element_generator.rs`: `FoldingElementGenerator`, `FoldingLineElement` | Merges overlapping folded ranges into an atomic native run, preserves label display columns, and unfolds on pointer activation. |
| Editor IME projection | `preedit_element_generator.rs`: `PreeditElementGenerator`, `PreeditTextElement` | A native element projects composition over a document range without editing it. Virtual-space padding remains separate from the underlined composition. |

`VisualLineError` replaces construction and extension exceptions with checked
results. Combining the small transformer types, text source, paired formatted
element/run types and paired web/mail generators is intentional. This table does
not claim that every member of upstream `TextView`, `VisualLine`, paragraph
properties, background layers or embedded-control hosting is supplied by these
files alone.

## Construction and ownership

All document offsets, document lengths and formatter columns use UTF-16 code
units. Pixel sizes stay in FerroUI device-independent units. An element may
consume zero document units while occupying one or more visual columns, or
represent a long document range using one atomic visual column.

The builder starts every generator in registration order and guarantees matching
finish calls on normal completion and error exits. Optional generators are
skipped above the source's 3,000-unit line cutoff; required generators such as
tabs, folds and composition continue to run. At each document position, all
zero-document-length elements are accepted in registration order, followed by
the first element that consumes document text. After a zero-length result or no
result, interest queries advance by one document unit to prevent repeated
construction at the same offset.

Elements may cross logical line boundaries. The builder extends its last line
to the containing logical line and rejects an element ending inside a CRLF
delimiter. It validates the generated document length against the complete
represented span. Transformations may replace elements only when their total
document length is preserved. Splits must return both the requested visual
boundary and the original document length. A partial atomic element is left
intact by a styling range.

Finishing consumes the mutable builder and returns `Rc<VisualLine>`. The line
retains its document snapshot and elements, and the native text source retains
the line. A view reference in the construction options is weak, preventing a
view/cache/source cycle. The public element data remains immutable after
finishing; its native reference-valued properties retain the framework's usual
reference semantics.

The native `ITextSource` method cannot return a Rust error. Its adapter records
the first invalid run and ends formatting safely; `VisualLine::create_text_layout`
then returns that recorded error. Callers using `text_source()` directly must
check `VisualLineTextSource::error()` after formatting. Zero or negative run
length, a run beyond its element, non-finite or negative drawable dimensions,
oversized preceding text and nonadvancing custom caret positions are reported
explicitly. No extension failure is silently presented as a successfully
formatted truncated line.

## Live TextArea integration

`TextArea` now owns this pipeline in `text_area_visual_lines.rs`. The previous
`DisplayLine` projection and overlay whitespace drawing have been removed.
A retained cache entry holds the immutable `VisualLine` and its native
`TextLayout`; all arranged geometry uses that pair. An unarranged view computes
only UTF-16 tab columns without building a second display-text model.

The public host exposes `element_generators`, `line_transformers`,
`visual_line_construction_starting`, `visual_lines`, `get_visual_line`,
`get_or_construct_visual_line` and `redraw`. Collections are shared native
`FerroList` values. Reordering invalidates construction while duplicated
identities connect only once; the final removal disconnects the entry.

The stable construction sequence is:

```rust,ignore
let mut builder = VisualLineBuilder::generate(
    document.create_snapshot(), first_line_number, global_properties.clone(),
    &generators, VisualLineBuildOptions {
        text_view: Some(area.downgrade()),
        wide_space_width: measured_space_width,
        end_of_line: marker_options,
    },
)?;
// Resolve syntax styles with NativeHighlightingStyle and apply them to their
// absolute UTF-16 ranges before the application's additional transformers.
builder.apply_text_run_properties(start, end, resolved_properties.as_ref())?;
builder.run_transformers(&transformers)?;
let visual_line = builder.finish()?;
let layout = visual_line.create_text_layout(
    paragraph_properties, available_width, f64::INFINITY, 0, None, None,
)?;
```

The cached entry retains both `Rc<VisualLine>` and `TextLayout`. Native paragraph
properties should use automatic height, or a deliberate minimum-height policy
that also handles every wrapped row. A fixed height below a drawable's natural
height causes native text metrics to overlap subsequent rows.

The live editor paints TextMate and XSHD backgrounds with visual elements
before it draws selection geometry. The native glyph runs retain their
foreground colors and decorations. A follow-up redraw leaves the line number
and folding margins visible in the current valid frame. When an extension
invalidates that frame, the affected editor layers wait for a fresh snapshot.

`get_visual_column`/`visual_offset` map a document offset relative to the first
logical line into formatter coordinates. `get_relative_offset` provides exact
element mapping; `document_offset` rounds a hit inside an atomic multicolumn
element to the nearest document boundary. `is_document_offset_visible` excludes
strict fold interiors. `source_lines` lists the logical lines with visible
element starts, while `first_document_line` and `last_document_line` delimit the
complete represented document span, including hidden rows.

The host advances after construction updates its height tree and reconciles hidden
rows in its height tree. It must account for every multiline element, including
composition replacements and custom generators, rather than deriving all
collapsed rows from the folding manager alone. Caret, selection, IME cursor,
popup, word wrapping and hit-test geometry use these retained mappings.

The host exposes shared native observable collections of
`VisualLineElementGeneratorRef` and `VisualLineTransformerRef`, connects and
disconnects entries through `ITextViewConnect`, and invalidates cached layouts
when either collection changes. It snapshots both collections once per viewport
construction batch after raising `VisualLineConstructionStarting`. Fully cached
paints do not raise another construction event or rebuild the built-in regexes.
No collection or cache borrow survives an application callback.

Removal while a batch is inside generator/transformer callbacks marks it stale
but defers disconnect hooks until all matching finish callbacks have returned.
The host checks document identity, version and its own layout revision before
publishing a line, then checks again after hidden cached elements are released.
That second check handles custom element destructors which edit the document.
Viewport eviction also releases values outside the cache borrow. Recursive
construction returns an explicit error; collection hooks can change membership
and the synchronization loop reconciles those changes before finishing.

Pointer processing finds the corresponding retained element and calls its
cursor/press/release hooks before starting editor selection. The native
`OpenUri` event is registered against `TextArea` and can be exposed by a host
delegate to `VisualLineLinkText::open_uri_event()`. Applications can handle it
to supply custom navigation; otherwise the native top-level URI launcher runs.

Key modifier changes, scrolling, and layout requery the cursor under a resting
mouse. FerroUI coalesces these updates on its dispatcher, hit-tests the current
viewport, and lets hosted controls and margins keep their own cursors. Link
cursor queries resume on pointer release after selection. The pinned text
view schedules cursor invalidation after key and layout changes, with its
mouse-device update commented out. The Rust control resolves it through the
native cursor property.

Preedit construction uses the current selection's absolute start and end plus
its starting virtual-space padding. `composition_range()` returns only the
underlined text, excluding padding. The existing IME cursor is a UTF-16 offset
within that range. When the composition text stays the same, moving the cursor
keeps the shaped visual lines and updates the cursor rectangle. A clear with
no active composition leaves the line cache intact.

The primary caret follows the composition cursor. Secondary carets keep their
own mapped document positions, and secondary selections on the same visual line
keep their background and foreground styling.
The native drawing regressions check both behaviors on Windows and Linux.

A fold generator backed by a live manager verifies that its
document snapshot belongs to the same document and current version. Its view
connection retains a folding subscription with a weak view reference.

EOL markers are outside the element collection. CRLF, CR and LF have independent
glyphs; an end-of-file marker has an empty glyph. The text source reserves one
formatter position when enabled, while `visual_length_with_end_of_line_marker`
adds an exposed column only when a real line delimiter follows. They consume no
document content. When the host uses these native runs, it removes its previous
overlay EOL painting to avoid double rendering.

The live editor takes its space, tab, CRLF, CR and LF glyphs from
`EditorOptions`. `TextArea::non_printable_character_brush` controls whitespace
and line-ending color independently of line numbers. Matching line endings
reuse the same native glyph layout throughout a view. Changing the font,
document, options or marker brush starts a new cache. Previously retained
visual lines can still use the layouts prepared for their original style.

The pinned editor displayed custom space, tab and three line-ending glyphs with
the requested brush. Repeated LF markers shared a layout. A font change replaced
it, and changing the LF glyph left the document text intact. FerroUI's native
tests exercise these settings through the live editor and check that the
end-of-file marker remains invisible. Both editor crates compiled for bare
WebAssembly.

`TextArea` raises `PreviewPointerHover` before `PointerHover` after a
400 ms dwell. Moving more than two device-independent units on either axis
stops an active hover at its original position and starts a new dwell.
Leaving the area raises the stopped pair for an active hover. Each pair
uses fresh pointer arguments so awaiting hover leaves the original move
event intact. `PointerHoverLogic::attach` also accepts callbacks for
a standalone native control. The built-in attachment checks for routed event
observers before scheduling work. `RouteFinishedObservable::has_observers`
includes listeners that only subscribe to route completion.

## Paragraph metrics and shaping locale

The live editor measures the shaped `x` once with its default run properties
and uses that width for virtual columns, column guides and visible tab glyphs.
Ordinary document tabs still expand with spaces to the next logical column.
If a custom visual-line extension passes a raw tab to the formatter, it has a
fixed advance of indentation size times the `x` width. A zero-length background
rectangle uses the framework editor geometry's one-unit default.

New visual lines capture the thread's FerroUI text locale for shaping and font
fallback. Rebuilding a line after a locale change uses the active locale.
Previously retained lines keep their run properties, and line-ending layouts
are only shared between runs with the same captured locale.

The pinned AvaloniaEdit sample confirmed these rules with Segoe UI at 18 units.
The shaped `x` measured 8.2617 units and the space measured 4.9307. At
indentation sizes 2 and 7, custom raw tabs advanced 16.5234 and 57.8320 units.
The ordinary expanded tabs advanced 4.9307 and 29.5840 units. The formatter
captured `tr-TR` initially and `az-Latn-AZ` after the sample redrew its view.

The native editor regressions verify the fixed advance, the proportional-font
column width, the one-unit empty-line marker and the locale of both text and
line-ending runs after a targeted redraw. Both editor crates compiled for bare
WebAssembly.

## Viewport notifications

`TextArea::visual_lines_valid` and `visible_visual_lines` expose the last valid
visible snapshot. Its entries carry the shaped line and viewport bounds. The
snapshot covers intersecting rows, including partial rows at the scroll edges.
`ensure_visual_lines` refreshes it and synchronously delivers pending observers
outside rendering. Calls during visual-line construction report recursion.
The existing `visual_lines` method continues to expose the retained cache.

`visual_lines_changed` delivers a completed snapshot. Normal layout and
compositor updates dispatch handlers after the active pass, and coalesce
repeated or superseded frames. Status is invalidated by redraw, document edits,
scroll and viewport changes. `scroll_offset_changed` reports previous and
effective offsets synchronously. Replacing a document resets the scroll offset
and uses the existing `TextEditorComponent::subscribe_document_changed`
property observer. Dropping a viewport subscription releases its handler.

The pinned editor publishes `VisualLinesChanged` at the end of every measure
pass. FerroUI delivers changes to the visible snapshot and allows custom
observers to invalidate it safely. An update produced in a handler remains
pending until the current notification returns.

## Targeted redraw

`TextArea::redraw_range` takes a checked UTF-16 range. Each endpoint touches the
corresponding boundary, so an empty range at a logical line start also refreshes
the visual line that ends there. A range inside a folded paragraph refreshes its
visible owner.

`TextArea::redraw_visual_line` takes a retained visual line handle. A stale or
foreign handle returns `false`. An accepted request retires the current
viewport snapshot and reuses other cached lines when publishing the next one.
Requests from construction callbacks also retire the in-flight batch, so no
partially generated line is published as current.

When a changed range touches an offscreen generated collapse, the editor
retires that collapse even if the visual line has left the cache. Built-in
folds and explicit collapse handles remain authoritative. The next viewport
layout recomputes the horizontal extent from retained and rebuilt lines.

The pinned AvaloniaEdit fixture rebuilt row 2 for an interior edit, row 3 for
a handle redraw, rows 1 and 2 for a boundary request, and row 1 for an offset
inside its fold over rows 1–4. Native tests check the same retained identities,
offscreen collapse removal, extent measurement, and callback invalidation.

## Editor component services

`TextEditorComponent` provides the outer editor and text area with the same
view locator. A view registration overrides the built-in view services and
the document's services. The view resolves `Ref<TextEditor>`, `Ref<TextArea>`
and installed folding, XSHD highlighting and TextMate services. Other lookups
delegate to the current document's locator, which resolves `TextDocument`
and `dyn IDocument`. Two views sharing a document share its registrations.
`FerroLocator::unbind` removes an override and exposes any inherited service.

The locator holds a weak reference to the owner. A program can keep it
across a rebind and immediately resolve the new document's registrations.
Document-binding and options callbacks use the native styled properties.
They receive previous and current values and disconnect when their Rust
subscription is dropped. Text edits keep the same document binding. An
unowned text area exposes its view locator and has no bound document or
editor options.

## Embedded controls and remaining differences

`InlineObjectElement::new` gives visual-line generators one formatter column
for a native control. `with_visual_length` accepts a positive formatter length
independent of the document length. The whole span is one atomic caret target.
A zero document length inserts the control at a boundary. A positive document
length substitutes for the corresponding text. Its `EmbeddedControlRun`
captures the child's measured size and baseline.

The editor parents and measures the child during layout. It arranges the child
from the formatter's physical runs after wrapping and bidi reordering, in a
layer clipped to the text viewport. A generator that returns the same control
across a rebuild preserves its identity. Scroll and layout changes update
placements, and eviction releases both parents. When an evicted child has
keyboard focus, the editor takes focus before detaching it. Child pointer,
keyboard, and text input stay with the native child. A control already owned by
another container or used at two visible positions produces a diagnostic.

Custom visual-line elements expose the native child through `inline_control`
so the editor can prepare it during layout. They can set a positive length
on the measured run with `EmbeddedControlRun::with_length`. The retained
visual line owns placement, including when a formatter row wraps around the
embedded run. A wider formatter span still uses the child's measured width.

The five-character line had seven formatter columns, and the hosted button
measured 86 × 48 with a baseline of 31. At a narrow width the button occupied
its own row and the caret still moved from column 1 to 4. The native regression
checks the same metrics, caret movement and wrapping. It also checks that
removing the generator releases the button from both parents.

The document text range enumerates hosted controls. A text range within a
replacement span resolves to the native control's automation peer. An
inserted control can share a UTF-16 offset with the caret. Ranges resolved
from that control or a hit on its bounds carry the control's identity, even
when cloned. An ordinary caret at the same offset resolves to the editor.
The range reports the inline control as its enclosing peer, which ends the
document's child lookup at that control. Moving the range clears the insertion
identity. After removal or eviction, the range follows the remaining text's
ancestry.
[UI Automation embedded objects](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-textpattern-and-embedded-objects-overview)

The Linux editor suite checks replacement ranges, inserted ranges, cloning,
caret movement, and removal. `RangeFromChild` and `RangeFromPoint` agreed on
each button and its text bounds. Each clone returned the same button as its
enclosing peer, and the adjacent text resolved to the editor. Both editor crates
compiled for bare WebAssembly.

Win32 pointer and keyboard messages focus the child and edit its text. The UI
Automation client enumerates the child through the editor's document range and
gets the replacement text and its enclosing peer through both `RangeFromChild`
and `RangeFromPoint`. Scrolling away removes the child from the range and
transfers focus to the editor. Returning to the row reattaches the same edited
TextBox. Removing its generator releases both parents.

Other explicit adaptations and boundaries:

* Public generators and transformers are ordinary Rust traits/`Rc` objects.
  Their callbacks are not serializable XAML objects; native controls and routed
  events still use FerroUI's type and property system.
* `LinkElementGenerator` uses the existing bounded core regex dialect. Patterns
  outside that dialect and exhausted match budgets return explicit errors.
  The built-in web and email generators reuse compiled expressions within
  each UI thread. Each generator holds its own current match and settings.
  This is unrelated to TextMate's separate native/browser Oniguruma backends.
* Native `Uri` is narrower than `System.Uri`; web links additionally validate
  nonempty authority and percent escapes. Exhaustive URI normalization and
  canonicalization equivalence is not claimed.
* The editor options expose spaces, tabs, EOL visibility, indentation, control
  boxes, web/mail links and the command-modifier click policy. Custom generator
  fields retain their glyph, brush and matching settings. Built-in composition
  has first priority, followed by folding, application generators, tabs, special
  characters and optional web/mail generators.
* Native paragraph formatting uses automatic row height so styled and drawable
  runs can expand a row. The editor's line-height factor sets a per-row minimum.
  Exact upstream drawable line-gap and pixel appearance need reference comparisons.
* Arbitrary custom multiline spans are discovered when their owning line is
  constructed. Known folds, active multiline composition and explicit collapse
  sections preseed intervals even before painting. Constructing line 100 then
  projected the entire span. FerroUI evicts the old interior layout and resolves
  subsequent queries for line 104 through line 100. [Pinned visual-line
  construction](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Rendering/TextView.cs#L762)

## Editor typeface

`FontStyle`, `FontWeight`, and `FontStretch` use FerroUI's shared inheritable
text properties. They apply to the document's default runs, default row
metrics, virtual-space width, and the built-in line numbers. A font change
rebuilds the visual lines and remeasures the gutter. Highlighting and custom
transformers can still override the typeface of individual runs. Accessibility
reports the effective style and weight and raises a format-change event.

A stretch-only change appeared after an explicit redraw. FerroUI rebuilds
immediately for all three properties. The native tests exercise live changes,
line-number runs, accessibility attributes, and XAML attribute loading. [Pinned
typeface
construction](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Utils/ExtensionMethods.cs#L100)

## Empty-document watermark

XAML and Rust can set `Watermark` on `TextEditor`. The native direct property
forwards to the text area's styled property and observes changes from either
control. The hint uses the editor's typeface and foreground at half opacity.
The text phase clips it to the viewport after the gutter. Its cached layout
leaves document positions, insertion, selection and scroll extent intact.

The pinned editor shows the hint while focused and while read-only. Inserting
a newline hides it. Undo and rebinding to an empty document restore it.
FerroUI also clears the hint during active IME preedit so the composition
stays legible. The peer exposes the hint as placeholder metadata. A Windows
UI Automation client read it as HelpText and confirmed that the empty text
range and value both stayed empty.
[Pinned watermark template](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Editing/TextArea.xaml#L29)

The linked sample checks actual canvas pixels with an empty document, after
Enter inserts a newline, and after undo.

## Control-character boxes

Visible control labels fill a box with 2.5-unit rounded corners before
drawing the glyphs. The native drawing context and text geometry collector
receive the radius through the same text drawing interface. The label still
adds three units to its measured width and occupies one caret interval.
The label's display columns affect the next tab stop.

The Rust formatter regression checks the rounded draw operation, glyph ordering,
tab width, and caret movement for the same input.

## Prepared formatted text

Generators can return `FormattedTextElement::from_formatted_text` with a
shared native `FormattedText`. Each element occupies one visual column. Its
document range remains independent of the prepared label's text. The run uses
the object's height, first baseline, and width including trailing whitespace.
It draws the object's own lines and style ranges through the native text
sink. Request `TextArea::redraw()` after changing the shared object's style
or layout constraints to refresh its geometry.

The pinned generator overload throws while preparing an unset plain-text field.
A two-row label occupied one editor column and its trailing spaces contributed
to the caret span. The native regression checks both rendered label rows, the
following glyph, measured bounds, and a larger span after changing the font size
and redrawing.

The glyph area grew after redraw and disappeared on removal. The document stayed
`a!b` throughout.

## Overlapping folds

An installed folding generator builds its sorted snapshot once for a layout
batch. Each entry records the section with the farthest end through that entry
and the end of the connected range. A visual line uses binary searches to find
the next fold start and resolve the placeholder at that offset.

The containing section with the farthest end supplies the label and receives
the pointer action. Equal ends keep the first section in start order. Sections
with equal starts retain their insertion order when created and their existing
order when reconciled with refreshed folding ranges. Touching or crossing
sections can extend the placeholder without changing which section it opens.

The pinned editor and the native regression cover exact duplicates, a shorter
section at the same start, crossing and touching sections, and a disjoint
placeholder reached through the visible suffix. The first placeholder covers 21
document units and uses the first label's eight UTF-16 columns. Clicking it
opens only that section, so the second label appears over the same range. The
reference also confirmed that same-start reconciliation retains the existing
handles and folded state.

## Folds crossed by custom elements

An installed fold takes priority at its original start. A custom element
that starts earlier can consume that start along with part of the folded
range. The folding generator resumes at the custom element's trailing edge
and projects the rest as a single placeholder. A custom element that covers
the entire range needs no additional placeholder.

The resumption point is a visible caret stop. Left and Right traverse the
visible boundaries of the custom element and the fold marker while the fold
stays closed. Placing the caret strictly inside the hidden portion opens
the section. The text area keeps the suffix of the closing source row on
the projected line. Accessibility line ranges span the complete projected
row, and source offsets inside the fold retain the hidden attribute.

The pinned editor leaves its containing-fold return disabled. Enabling that
return in a diagnostic copy recovered the same projected ranges and caret stops
as the Rust regression. The multiline cases ended the custom element before a
CRLF, at the delimiter boundary, and at the next source line. [Pinned fold
generator](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Folding/FoldingElementGenerator.cs)

## Explicit collapsed lines

`TextArea::collapse_lines` takes two live document lines, including both
endpoints. A generator leaves its owning row visible and collapses the rows
covered by its multiline element. The view owns that collapse until the handle
is restored, its last line is removed, the line index is rebuilt, or the view
changes documents. Overlapping handles restore independently. Edits inside a
range follow the live endpoints. Removing either endpoint moves it to the
closest surviving line inside the range.

Explicit sections share the sparse height index with folds, composition, and
automatically discovered generator spans. Hidden rows contribute zero height.
The view retains measured heights for restoration within the current layout.
Document edits and other layout invalidations can require fresh measurements.
Viewport hit tests can resolve the visible logical row by document-space Y, and
a fully collapsed view returns `None` for that lookup.

The pinned [CollapsingTests](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/test/AvaloniaEdit.Tests/Document/CollapsingTests.cs)
cover all inclusive ranges in a ten-line document, restored heights, inserts,
and deletions. The FerroUI tests replay those cases through `TextArea` and
exercise overlaps, document replacement, and generator callbacks.

Explicitly collapsed source rows keep their text in native accessibility
ranges. Unprojected rows report `IsHidden`, inherit the editor's default text
attributes, and remain addressable as source lines. Their text has no bounds.
Scrolling a mixed range uses the nearest projected endpoint in the requested
direction. A multiline generator can still project its last source row's
suffix. Collapse and restoration queue a text-attributes notification.

## Document-space positions

`TextArea` exposes the document-space position queries from the reference
`TextView`. `get_visual_position` projects a logical UTF-16 location or an
explicit visual column into a shaped row. Soft-wrap affinity distinguishes
the trailing edge of one row from the leading edge of the next. The seven
`VisualYPosition` values select the native full row, default text band, or
baseline. The logical-line top query uses measured heights when available and
the default height for unmeasured rows.

`get_position` resolves the nearest shaped boundary. `get_position_floor`
returns the cluster start and uses the shaped row end past its last glyph.
With virtual space enabled, nearest rounds to the nearest cell and floor
rounds down. Virtual cells start at the physical right edge of the last
wrapped row. Hit tests use the visible line frame and document coordinates,
so scrolling changes which rows can be hit and leaves projected locations
in the same coordinate system. Coordinates use FerroUI's live row metrics
and virtual-cell width. Pixel-level comparisons with the reference remain open.

## Vertical navigation with wrapping and bidirectional text

Up, Down, Page Up, and Page Down retain the preferred horizontal coordinate
in document space and resolve it within the chosen shaped row. The editor uses
the formatter's bidirectional character hit in that row. Rectangular Up and
Down use the same projection, and the editable rectangle continues to project
onto first rows. At the first or last row, ordinary Up or Down preserves the
caret and its affinity when there is no adjacent row.

When a hit lands at the logical end of a soft-wrapped row, the editor chooses
the preceding valid caret stop within that row. It honors grapheme and
generated-element boundaries and keeps row affinity when an atomic element
leaves no interior stop. Virtual cells remain available on the final formatted
row, starting at its physical right edge.

Horizontal arrows in the tested bidirectional run follow logical UTF-16 grapheme
order, including movement whose visual X decreases. A Rust regression wraps a
sequence of supplementary characters and verifies that the caret stops before a
complete grapheme.

The pinned editor calculates continuation indentation from its options and
passes it to the text formatter. The formatter leaves continuation rows at their
original horizontal origin. FerroUI keeps that wrap geometry. [Pinned
editor](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Rendering/TextView.cs)
· [Reference
formatter](https://github.com/AvaloniaUI/Avalonia/blob/12.0.0/src/Avalonia.Base/Media/TextFormatting/TextFormatterImpl.cs)

## Line-height factor

`EditorOptions::line_height_factor` defaults to `1.16`. `LineHeightFactor::new`
accepts positive, finite multipliers. The editor applies the factor to its
default text height as the minimum for every formatted row. The formatter
centers the text when the minimum adds space and keeps the natural height of
taller text or inline controls. Drawing, hit tests, caret and selection
rectangles, and document height use those same formatted rows. The view clamps
the minimum to the largest finite drawing coordinate for extreme values.

The FerroUI tests also check selection, caret and inline-child placement after a
live options change.

## Long lines with tabs

The native text source finds its element with a binary search over sorted visual
range ends. The document-offset lookup starts with the first inclusive range so
an insertion with zero document length keeps its original boundary affinity.
Caret navigation starts its scan at the relevant visual element. A regression
compares both mappings and visibility with the original linear scan on a line
containing 512 tabs and a generated insertion. The test includes positions at
the insertion and at either end of the line.

Each ordinary run can pass its full paragraph context to the native shaper.

### DirectWrite script analysis reuse

FerroUI reuses the script runs for slices of a single immutable UTF-16 owner.
The Windows bridge checks the owner, bidi direction and resolved locale and
uses binary search to select the script runs that intersect each slice. Each
run retains the full shaping context and the existing font, feature, cluster
and placement processing. The Rust cache holds at most four entries per
shaping thread and admits sliced owners between 257 and 131,072 UTF-16 units.
Multiline owners and uncacheable inputs use the existing analysis for the
requested paragraph context. [DirectWrite script analysis](https://learn.microsoft.com/en-us/windows/win32/api/dwrite/nf-dwrite-idwritetextanalyzer-analyzescript)

The DirectWrite regression compares glyph identifiers, clusters, advances and
offsets against uncached calls for tabbed and mixed-script slices, including
different locales and OpenType feature ranges.

### Built-in hyperlink discovery

The built-in expressions cannot consume ASCII whitespace. During construction
the generators index the ranges between separators that contain a required web
or email marker, and reuse the first usable match for each complete range.
If another element moves the search start inside one of those ranges, the
generator searches that exact suffix to preserve the leading word boundary.
A multiline projection rebuilds the index when the source span expands.
The index is scoped to the immutable text buffer and its range. Custom
strategies and element factories continue to receive the original suffix,
including its match-relative offsets and replacement captures.

Regression tests compare both built-in generators with the original suffix
search at each UTF-16 offset in the fixtures. They cover tabs, invalid markers,
adjacent links, surrogate units, reuse across different snapshots and a
custom generator that extends the visual line across CRLF. A custom factory
also verifies the protocol capture and original search text.

## Tests and limits

```sh
cargo test --no-default-features \
  -p ferroui-edit-core -p ferroui-edit -p editor-demo --lib
```

Native unit tests establish only the tested behavior. Browser layout, platform
URI launching, arbitrary custom formatter runs and embedded native-control
hosting need their own integration coverage; the linked browser suite is
described in [editor-browser.md](editor-browser.md).
