# Editor background geometry and rendering contracts

Reference: AvaloniaEdit `7277e36366d754219d7ede16b6613786b32caf4c`.
The implementation uses FerroUI's native `StreamGeometry`, `IGeometryContext`,
`TextLayout`, `TextLine`, and `DrawingContext`. It introduces no rendering backend
and no duplicate font shaping or bidi implementation. Attribution and the MIT
license are retained in `src/FerroUI.Edit/NOTICE.md`.

## Source map

| Pinned upstream source or behavior | Rust location | Port treatment |
| --- | --- | --- |
| [`Rendering/BackgroundGeometryBuilder.cs`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Rendering/BackgroundGeometryBuilder.cs): rectangle outline, `CloseFigure`, `CreateGeometry` | `src/FerroUI.Edit/background_geometry_builder.rs` | Rounded native stream figures; explicit outline closure; independently owned geometry snapshots |
| Same source: `GetRectsForSegment`, `GetRectsFromVisualSegment`, `ProcessTextLines` | `src/FerroUI.Edit/background_geometry_rects.rs` | Retained visual-line views, native shaped text bounds, wrap and virtual-space projection |
| [`Utils/PixelSnapHelpers.cs`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Utils/PixelSnapHelpers.cs): pixel size and scalar rounding | `BackgroundGeometryBuilder::pixel_size`, `add_rectangle_aligned` | `TopLevel::render_scaling`; independent x/y pixel sizes; midpoint-away-from-zero rounding |
| [`Utils/ExtensionMethods.cs`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Utils/ExtensionMethods.cs): `IsClose` | Geometry join comparison | FerroUI `MathUtilities::are_close_eps` with upstream's fixed `0.01` DIP tolerance |
| [`Rendering/IBackgroundRenderer.cs`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Rendering/IBackgroundRenderer.cs) | `src/FerroUI.Edit/background_renderer.rs` | `IBackgroundRenderer` using native view/context references and a fallible draw callback |
| [`Rendering/LayerPosition.cs`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Rendering/LayerPosition.cs): `KnownLayer` | `KnownLayer` | Ordered `Background`, `Selection`, `Text`, `Caret` values |
| [`Rendering/ITextViewConnect.cs`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Rendering/ITextViewConnect.cs) | Existing `ITextViewConnect` supertrait | The same connection lifetime contract as generators and transformers |

This checkpoint supplies the geometry, projection, and renderer contracts.
Live TextArea renderer collection ownership, layer dispatch, invalidation, and
segment convenience methods are a separate integration step. Adding these
types alone does not claim that the live host invokes custom background
renderers. Arbitrary child-control insertion/replacement through upstream's
`LayerInsertionPosition` and `LayerPosition` is not implemented by these files.

## Geometry construction

`BackgroundGeometryBuilder::add_rectangle` accepts already aligned rectangles.
Consecutive rectangles whose next top is within `0.01` DIP of the previous
bottom join into one outline. Changes to the left and right edges create the
same clockwise outer and counterclockwise inner arcs as the pinned algorithm.
A vertical gap, another rectangle on the same row, or an explicit `close_figure`
starts a separate figure. Input order is significant: this API constructs text
row outlines; it does not sort or compute the general union of arbitrary
overlapping polygons.

The upstream insertion index divides a figure into the downward right edge and
the returning left edge. The Rust port stores these as a `Vec` and `VecDeque`.
Appending another row therefore avoids repeated insertion into the middle of a
growing path. Path construction and emission are linear in the number of added
rectangle edges. Zero-radius arcs are emitted as ordinary native line commands.
The upstream narrow-rectangle rule, which omits the horizontal rounded caps
when the width is at most the requested radius, remains observable.

`create_geometry` closes an outstanding figure and returns
`Option<Ref<StreamGeometry>>`; an empty builder returns `None`. Every geometry is
an independent snapshot, so adding rectangles later does not alter previously
returned native resources. `write_to` emits the same filled, closed figures to
a caller-owned `IGeometryContext`, including a native `PathGeometryContext`. It
does not dispose that context. `clear` discards figures and keeps the builder's
formatting options.

Public options are validated before adding a rectangle. Negative dimensions,
nonfinite coordinates, invalid radii, invalid border thicknesses, and
nonpositive device pixel dimensions are reported as `BackgroundGeometryError`.
No partial outline is appended for a failed rectangle. Segment projection is
fully validated before adding its rectangles to an existing outline. A final
cap uses the last successfully added row's radius, so changing an option after
preparing a path cannot inject invalid coordinates during geometry creation.

## Pixel alignment

`add_rectangle_aligned(rectangle, pixel_size)` honors
`align_to_whole_pixels`. With a border thickness of zero it snaps rectangle
edges to device pixel boundaries. With a nonzero border it snaps the outside
edge of the future stroke: half the stroke width is subtracted before rounding
the leading edges and added before rounding the trailing edges. The half width
is then restored to the path centerline.

Rounding uses midpoints away from zero, including negative coordinates and
fractional DPI scales. `add_rectangle_for_view` obtains pixel dimensions from
the view's native `TopLevel::render_scaling`; an unattached view uses one DIP
per pixel. Like upstream, this does not account for additional visual
transforms. A caller with a transform-aware grid can supply pixel dimensions
directly. When a subpixel rectangle and thick stroke would invert a snapped
centerline, that dimension becomes zero instead of creating a negative native
rectangle.

## Native text rectangle projection

`BackgroundGeometryLine` borrows a matching retained `VisualLine` and native
`TextLayout`, plus their displayed origin. The origin already contains the
gutter translation and both scroll offsets. The projection functions return
owned `Rect` values and never keep those borrows.

`BackgroundGeometryViewport` supplies four metrics: content width, default line
height, the physical width of a virtual space, and the visible width used for an
empty selection. Content width is normally the maximum of content extent and
viewport width, excluding the gutter when the origin includes its translation.
The zero-width default is intended for projections without full-width
extension; hosts requesting wrapped/full-width backgrounds must supply their
actual content width.

`get_rects_for_segment` accepts an `ISegment` and clamps its UTF-16 endpoints to
the retained document snapshot, as upstream does. `get_rects_for_selection`
also retains ordered virtual-space endpoints from a directed
`SelectionSegment`. The input line views must be in document order and come
from the host's current retained layout. Offscreen lines are never constructed
by the helpers.

`get_rects_from_visual_segment` uses visual UTF-16 columns. An end column of
`usize::MAX` requests full content width; ordinary columns after the visual
length represent virtual spaces. Document projection produces that sentinel
when the requested range continues beyond a visual line and full-width
extension is enabled.

The projection preserves these upstream behaviors:

- Native `TextLine::get_text_bounds` supplies each shaped range, so a mixed bidi
  selection can produce multiple rectangles on one row. Their widths are
  normalized without flattening the distinct runs into one bounding box.
- Empty lines and ordinary zero-length ranges have a visible marker rectangle.
  Empty rectangles duplicated at an unspaced soft-wrap boundary are suppressed
  when the range continues across that boundary.
- Wrapped rows exclude hidden trailing whitespace from the ordinary text
  rectangle, then extend continuing ranges to content width. The final row
  accounts for the native end-of-paragraph position separately from visible
  text and an optional end-of-line marker.
- A final virtual-space extension is unioned with the last text rectangle when
  they overlap. A disjoint trailing RTL fragment and the extension remain
  separate rectangles.
- The background height is the larger of the native row height and the host's
  default line height. Vertical positioning follows actual shaped row heights.

FerroUI's selection model stores virtual cells rather than AvaloniaEdit's
optional visual-column override. The adaptation places those cells after the
final formatted row's physical right edge, matching the editor's existing RTL
rectangular selection and virtual caret behavior. Virtual endpoints remain
available independently of full-width extension.

## Host integration contract

`IBackgroundRenderer` extends `ITextViewConnect`. `BackgroundRendererRef`
preserves `Rc` identity for `FerroList` membership, forwards connection hooks,
and compares pointers without invoking mutable application state. Its draw
callback receives `&TextArea` and `&mut DrawingContext`, and returns
`Result<(), VisualLineError>` so the host can use the same error-reporting path
as other visual-line extensions. `BackgroundGeometryError` converts into that
error type.

The live host must invoke each renderer before its chosen known-layer content,
in layer order: Background, Selection, Text, Caret. Renderers in the same layer
retain collection order. The collection and cache must be snapshotted before
application callbacks so callbacks run without a mutable collection borrow.
Connection/disconnection and invalidation should use the same reentrancy and
destructor precautions as visual-line generators.

For segment convenience methods, the host can clone retained cached-line
handles, construct `BackgroundGeometryLine` views over those handles, and then
call these projection functions. `full_width` must be supplied in the same
content coordinates as the line origins. Drawing still needs the host's text
viewport clip; the projection deliberately preserves rectangles outside the
horizontal viewport so scrolling and full-width geometry remain consistent.

## Validation

The shared native gate passed **13/13 background geometry tests** for this
checkpoint. Existing host tests are separately gated as integration progresses.

`src/FerroUI.Edit/background_geometry_tests.rs` contains focused path and native
formatter regressions for rounded caps; expanding/contracting row joins;
vertical tolerance and explicit closure; high-DPI stroke alignment and negative
midpoints; rejected input; narrow rectangles; independent native geometry
snapshots; UTF-16 emoji ranges; bidi fragments; wrapped trailing whitespace and
boundary markers; empty/newline selection; reversed virtual selection after
RTL text; and renderer identity/view hooks.

Run the focused suite through the shared native build:

```sh
cargo test -p ferroui-edit --lib background_geometry_tests
```

The path recorder implements the actual native `IGeometryContext` contract.
Native layout tests use FerroUI's deterministic text formatter test services;
they do not replace the projection with estimated character widths. The native
`StreamGeometry` snapshot test uses the framework's geometry test renderer.
Live drawing order and connection lifecycle require host integration tests in
addition to these independent regressions.
