# Stroke width handles

The native **Edit width on canvas** action edits the first stroke of one selected,
visible, unlocked layer. It works on the supported primitive and nonbranched
vector contours without converting the node or flattening its source geometry.
The numeric profile editor and presets remain available in the stroke inspector.

## Interaction

| Input | Result |
| --- | --- |
| Click a centerline between existing points | Insert a point at its current interpolated width |
| Drag a center square | Move the point along the contour, keeping its width |
| Drag either side circle | Change the symmetric width multiplier using local normal displacement |
| Tab / Shift+Tab | Select the next / previous point across contours, including overlapping closed-path endpoints |
| Left / Right | Move the selected point by 1% of normalized contour length |
| Up / Down | Increase / decrease its width multiplier by 0.1 |
| Shift with an arrow key | Use a ten-times larger step |
| Delete / Backspace | Remove the selected width point; removing the last point restores Uniform |
| Escape during a gesture | Restore the complete pre-gesture document and preserve redo |
| Escape while idle | Leave width editing |

A width multiplier of one is the authored base stroke width. Side handles use a
minimum twelve-screen-pixel offset so zero and very thin widths remain usable.
Their drag calculation uses displacement from the original pointer position,
so grabbing a displaced marker does not change width before moving it. For
inside/outside closed strokes the markers describe the prepared centered band;
the existing artwork clip determines which side is painted.

One profile repeats over every independent contour. Editing a point on any
contour updates that shared profile. Center positions remain strictly ordered;
dragging cannot cross a neighboring point. The canvas keeps a minimum normalized
gap of `1e-6` when moving a point. It preserves an imported closer pair rather
than merging, sorting or repairing authored data. Keyboard point selection also
makes coincident endpoints of closed contours accessible; the selected point
wins an exact handle hit-test tie.

Entering the mode, hovering and selecting an unchanged point do not author
document data. A changed drag converts preset anchors into a Custom profile.
Dragging back to the original anchors restores the original preset and metadata
without adding history. Arrow caps become butt caps in the same transaction as
the first actual profile edit, matching the numeric profile command; cancel
restores the original cap as well.

## Geometry and public rendering

[stroke_profile.rs](../crates/ferrodesign-core/src/stroke_profile.rs) exposes
`stroke_width_contours`, `StrokeWidthContour` and `StrokeWidthLocation`. The
contour preparation shares the bounded curve flattening and inserted width
knots used by filled stroke outlines. Sampling returns normalized position,
local point and unit tangent/normal. Transformed nearest-point lookup projects
the actual transformed chords before choosing a segment, so nonuniform scale,
reflection and shear do not substitute a local-distance hit test.

[canvas/vector_width.rs](../crates/ferrodesign-app/src/canvas/vector_width.rs)
retains prepared contours across camera and hover events. A captured drag uses
its immutable starting contour map; the rendered markers prepare current data
after a document revision. Native `DrawingContext` line, rectangle, ellipse and
text calls paint the markers in screen space. They are editor adorners and do
not enter artwork PNG/SVG/XAML export or prototype presentation.

The tool requires an enabled first stroke with positive representable base
width. It reports unsupported dashed or branching paths before beginning an
edit. Canvas editing is limited to 64 contours and 2,048 repeated width points,
as well as the core limit of 256 authored points per profile. Exceeding the UI
budget produces an explicit notice; it does not drop points or silently simplify
the source. Core subdivision and outline limits still apply.

## Transaction and save contract

Pointer capture starts one scoped node transaction. The gesture records the
editor transaction identity, render token, target, page, selection and world
transform. Moves, completion and cancellation validate that ownership before
touching the document. A new selection or page survives cancellation; a replaced
editor transaction is never cancelled or committed by a stale gesture.

Release commits once. Escape, capture loss, tool switching and an undo boundary
cancel the pending gesture. Explicit save and host lifecycle boundaries call
`finish_stroke_width_edit()` to commit before serialization. The active gesture
marks the host draft dirty and defers background serialization; late pointer
events after lifecycle completion cannot produce another history entry.

The public `stroke_width_state()` bridge reports editor-only diagnostic state:
node, selected point/contour, active gesture, hover and prepared contour count.
`stroke_width_active()` distinguishes a pending pointer gesture from merely
having the width editor open. These are observation APIs, not alternate editing
commands.

## Coverage and limits

[Native tests](../crates/ferrodesign-app/src/canvas/vector_width_tests.rs) exercise
real native handle pixels, immutable hover caching, multi-update pointer
transactions, exact undo/redo, transformed repeated contours, position ordering,
Escape, capture loss, replacement-transaction ownership, zero-width access,
selection-only clicks, deletion and overlapping closed endpoints.

[Browser tests](../tests/width-handles-browser.mjs) use the named native
`StrokeWidthHandlesButton` and actual captured mouse/keyboard input. They cover
insertion, normal and position drags, deletion, cancellation, pagehide with
IndexedDB persistence and late release, and reflected/sheared multiple contours.
These source coverage descriptions do not assert that the newly added tests
have executed successfully; exact source/build results belong in
[VALIDATION.md](VALIDATION.md).

The tool currently edits stroke entry zero. It does not expose independent
left/right width values, a per-contour profile, or multi-layer width-point
editing. Its interpolation and handle arrangement are FerroDesign behavior;
complete Figma visual or interaction equivalence has not been established.
