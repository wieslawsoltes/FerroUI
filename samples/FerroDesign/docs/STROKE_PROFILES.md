# Variable-width stroke profiles

FerroDesign stores a width profile on each stroke-stack entry and draws supported
nonuniform strokes as filled public FerroUI geometry. The original primitive or
vector path, stroke paint and layer transform remain editable. Numeric presets,
ordered custom-point fields and [on-canvas width handles](STROKE_WIDTH_HANDLES.md)
author the same serialized profile.

## Presets and stored points

`Stroke.width_profile` defaults to `Uniform`; that default is omitted from JSON
so existing uniform-stroke documents retain their representation. Other variants
use a `type` tag. A Custom profile retains its authored points even when every
width is one.

Position is normalized contour length from zero to one. Width is a nonnegative
multiplier of the base stroke width, so a multiplier of `1.75` means 175% of that
width. One profile repeats independently along each contour or nonbranched
network trail. These are the exact interpolation anchors in the implementation:

| Preset | `(position, width)` anchors |
| --- | --- |
| Uniform | `(0, 1)`, `(1, 1)` |
| Wedge | `(0, 1)`, `(1, 0)` |
| Taper | `(0, 1)`, `(1, 0.25)` |
| Quarter taper | `(0, 0.25)`, `(0.25, 1)`, `(1, 0.25)` |
| Eye | `(0, 0)`, `(0.5, 1)`, `(1, 0)` |
| Mirrored taper | `(0, 0.25)`, `(0.5, 1)`, `(1, 0.25)` |

The anchors follow the published
[Figma VariableWidthStrokeProperties API](https://developers.figma.com/docs/plugins/api/VariableWidthStrokeProperties/).
FerroDesign interpolates linearly between them. The Figma API does not specify
its complete interpolation equation; these presets do not establish pixel or
interaction parity with Figma.

A Custom profile contains one to 256 points. Positions must be finite, strictly
increasing and within `[0, 1]`; widths must be finite and nonnegative. Values
above one are permitted. A single point produces a constant multiplier. Before
the first point and after the last, the nearest supplied width continues to the
contour endpoint. Points are neither sorted nor silently repaired on import.

```json
{
  "type": "custom",
  "points": [
    { "position": 0, "width": 0.25 },
    { "position": 0.8, "width": 1.75 }
  ]
}
```

The example grows linearly from 25% to 175% of the base width over the first 80%
of each contour, then retains 175% through its endpoint. Closed contours may
have different starting and ending widths; the outline joins those radii at the
seam instead of rewriting the stored points.

## Native editing and history

The stroke inspector's preset choice edits stroke entry zero on the selected
editable layers. **Custom points…** opens the native **Stroke width** dialog.
Its stroke selector exposes the selected layer's complete stroke stack. Opening
the dialog copies the current preset anchors into its controls without changing
the document or converting the preset to Custom.

Each row contains **Position %**, **Width %** and **Remove**. Enter or focus loss
validates and commits a changed field. Escape restores that field's last
committed text and clears its error. Add Point inserts at the midpoint of the
largest available normalized interval, using the current interpolated width.
Remove retains at least one custom point. The preset choice can restore Uniform.

Invalid fields remain visible with their validation message. Add, Remove,
stroke-stack changes, preset choices and dialog rebuilding flush pending drafts
before proceeding. An invalid draft blocks the action; a rejected ComboBox
choice returns to the actual current stroke or profile without discarding the
text. The existing inspector draft mechanism also participates in save and
lifecycle boundaries. This differs from a held canvas gesture, whose single
transaction and capture rules are documented in
[Stroke width handles](STROKE_WIDTH_HANDLES.md).

`Editor::set_stroke_width_profile` validates every editable target before
publishing changes. A changed profile is one undoable operation and marks the
stroke property as overridden. Repeating the current profile produces no
history entry. Other stack entries, source commands, paints, opacity and layer
transforms are preserved. Choosing a nonuniform profile converts an Arrow cap
to Butt in that same operation; choosing an Arrow cap restores Uniform. Width
and profile validation reject unsupported combinations before a partial
multi-layer edit can be committed.

## Filled geometry and retained rendering

[stroke_profile.rs](../crates/ferrodesign-core/src/stroke_profile.rs) provides:

- `StrokeWidthProfile::validate`, `sample`, `maximum_multiplier` and
  `control_points` for validated data and preset-to-custom conversion.
- `validate_variable_width_support` for inexpensive document preflight without
  constructing an outline.
- `variable_width_outline` for supported nodes and
  `variable_width_path_outline` for an already prepared local vector path.
- `stroke_width_contours`, `StrokeWidthContour` and `StrokeWidthLocation` for
  the same bounded contour preparation used by native handles.

Quadratic and cubic commands are subdivided adaptively with de Casteljau.
Control-hull distance, derivative direction and control-polygon/chord length
checks bound the accepted approximation. Every authored width knot is inserted
in the prepared centerline, including features narrower than the geometry's
normal sampling interval. Unrepresentable knots return an error instead of
being dropped. This builds an approximate filled outline; it does not flatten
or replace the document's source commands.

Butt, Round and Square caps use the endpoint width. Miter, Bevel and Round joins
retain their separate geometry; the miter limit matches the editor's existing
native pen limit of ten. Smooth subdivision vertices receive round joins.
Output pieces have consistent nonzero winding and form one filled path, so
overlapping pieces of one stroke use one paint operation rather than repeatedly
applying its alpha.

For a closed Inside or Outside stroke, the core returns a doubled centered band.
The renderer clips that band once against the source fill using the existing
Intersect/Exclude semantics. Open contours use centered placement. A path-level
request for aligned bands with open contours is rejected. Paint coordinates are
assigned from the uncut band so clipping does not resize the brush.

The retained renderer stores the resulting public FerroUI `Geometry` alongside
the stroke brush and clip. Profile type and custom points participate in its
visual cache key. Placement and camera work reuse retained geometry; pointer
picking has its own retained outline data. Native width handles share the
centerline and normalized-length semantics. Their transformed nearest-point
query projects transformed chords before choosing a segment, preserving correct
distance comparisons under nonuniform scale, reflection and shear.

Native PNG export uses the public FerroUI drawing path. SVG and FerroUI XAML
exports emit filled variable-width paths, including their closed-path alignment
clip. They retain the existing exporters' limits for effects, blend modes and
brush types. The stroke algorithm, native artwork, handles and export paths do
not use Skia or a separate HTML drawing surface.

## Supported geometry and explicit bounds

Supported inputs include lines, ordinary vector paths, nonbranched stroked
network trails, ellipses, polygons, stars and the shared corner paths of
rectangles and frame-like nodes. Branch checks count only edges that participate
in the stroke; fill-only network edges do not create a stroke branch.

Nonuniform dashed strokes, Arrow caps at the outline API, and branched stroked
networks are rejected. Text, Boolean and group nodes require a separately
prepared outline and are not accepted by the current node-level nonuniform
authoring path. Dynamic strokes, independent left/right widths and per-contour
profiles are not implemented. Selecting Uniform keeps the existing native pen
rendering and its supported cap/dash combinations.

| Guard | Current bound |
| --- | --- |
| Custom profile | 1–256 ordered points |
| Source path | At most 32,768 commands, or 32,768 network edges in support preflight |
| Prepared centerline and inserted knots | At most 65,536 counted points |
| Filled output | At most 262,144 path commands |
| Subdivision | At most 1,048,576 counted work units and 32 levels |
| Local approximation tolerance | `min(0.05, 0.001 × max(path span, maximum band radius))` document units |
| Numeric precision | Finite coordinates, widths and width products; the coordinate magnitude must fit the tolerance's `f64` precision check |

Singular curves, unrepresentable coordinates or exhausted work limits can return
an explicit outline error. Explicit profile edits preflight the complete outline
before mutation. If a later document or rendering operation encounters an
outline failure, the renderer increments its existing unsupported-effects
diagnostic and declines that stroke; it does not substitute a uniform stroke.
There is no claim that these bounds guarantee a frame rate or cover every
pathological curve arrangement.

## Validation sources

[Core profile tests](../crates/ferrodesign-core/src/stroke_profile_tests.rs) cover
preset anchors, linear interpolation, JSON defaults and custom authorship,
caps/joins, holes and repeated contours, curve preparation, narrow knots,
alignment, branches, resource/precision rejection and transformed handle
projection. [Editor tests](../crates/ferrodesign-core/src/stroke_edit.rs) cover
atomic profile and base-width edits, Arrow conversion, untouched targets,
serialization and history. Renderer and picking tests exercise the integrated
ink and retained data.

[Native browser regressions](../tests/width-profiles-browser.mjs) exercise all six
presets, invalid-draft action barriers, field Escape, ordered Add/Remove edits,
both entries of a stroke stack, exact intermediate undo/redo, native JSON
round-trip, and exact interior/exterior samples from a native PNG and an
independently decoded SVG. The independent SVG decoder is a test of the exported
file, not an application renderer. The separate
[width-handle browser module](../tests/width-handles-browser.mjs) covers captured
gestures and lifecycle persistence.

These descriptions identify test source and intended coverage. Exact executed
source/build identities and results belong in [VALIDATION.md](VALIDATION.md);
the presence of a test file alone does not establish a browser pass or full
Figma parity.
