# Rounded polygon and star corners

Polygon and Star layers use their existing `corner_radius` property to build a
rounded, editable outline. The native **Corner radius** field accepts fractional
values. Changing it keeps the layer kind, point count, star inner ratio, authored
box, center rotation, reflection and affine transform intact. The procedural
layer remains editable; it does not become an arbitrary vector path.

[Native canvas radius handles](CORNER_RADIUS_HANDLES.md) provide the same
uniform-radius authoring through captured pointer input. They preserve the
primitive and its affine geometry, with one undo step per changed drag.

No document field or schema version is added. Existing JSON already stores
`corner_radius`; a nonzero value on a Polygon or Star now affects its visible
outline and picking. The default zero value preserves the prior sharp geometry
exactly. Independent rectangle radii remain serialized as before.

This implements circular fillets for these two primitives. It does not claim
complete Figma Design parity or an exact match to Figma's undocumented numerical
construction for every angle and radius.

## Public geometry and radius policy

[`polygon_corners.rs`](../crates/ferrodesign-core/src/polygon_corners.rs) exposes:

- `polygon_corner_path(&Node) -> VectorPath`, the closed local outline.
- `supports_corner_radius(NodeKind) -> bool`, the current editable-kind predicate.
- `Editor::set_corner_radius(&[NodeId], f64) -> Result<usize, DocumentError>`, the
  validated command used by the native inspector.

The path starts with the same procedural points as `Node::polygon_points`.
For each nondegenerate corner, its incoming and outgoing edges determine a
signed turn and tangent circular arc. Positive and negative turns use the same
construction, so star tips are cut back and concave notches are rounded outward
into their formerly sharp recesses. Path winding is preserved.

Radius normalization has two stages:

1. Each vertex's effective radius is at most half either adjacent edge length.
   This bound applies even to an obtuse corner whose tangent distances alone
   could accommodate a larger circle.
2. The tangent distances of neighboring fillets must fit their shared edge.
   A common scale reduces effective radii further when needed, preventing a
   reversed straight segment or overlapping adjacent tangent intervals. This
   second restriction matters especially for acute tips.

The stored radius is retained even when these geometry limits reduce its
effective value. Increasing the box size can therefore expose more of the
authored radius. Radius zero returns the exact previous sharp polygon command
stream. Changing radius never resizes the layer's authored box to the smaller
rounded outline.

Arcs are split into pieces of at most 90 degrees and represented by standard
tangent cubic Bézier approximations. They are not mathematically exact circles;
for a quarter arc the tests bound radial approximation error to less than
0.03% of its radius. Geometry stays vector based and uses the public FerroUI
`StreamGeometry` and drawing APIs when rendered.

Zero dimensions, repeated points, straight edges, reversing edges and a zero
star inner ratio retain finite degenerate geometry. Nearly reversing or nearly
straight directions without a stable finite tangent construction remain sharp.
Edge normalization divides components by length directly, avoiding reciprocal
overflow for positive subnormal dimensions. Invalid nonfinite dimensions yield
an empty path; document validation rejects invalid authored appearance values.

## Native editing and history

The command supports Rectangle, Frame, Component, Instance, Section, Polygon and
Star layers. It validates a finite, nonnegative radius and resolves every target
ID before mutation. A missing ID or invalid number leaves every target and the
history stacks unchanged. Duplicate IDs are processed once. Unsupported kinds
and layers locked directly or through an ancestor are skipped.

A successful edit:

- sets the uniform radius and clears the independent rectangle corner array;
- removes only the `corner_radius` variable binding;
- records the `corner_radius` component override;
- commits one scoped history entry for all changed targets.

Entering the currently resolved value explicitly detaches a radius variable
binding in that same history entry. Repeating the value after it is already
unbound is a no-op and preserves redo. Undo and redo restore the complete prior
and subsequent node states, including bindings and overrides.

The inspector's existing numeric input limit remains 10,000,000. The core
command accepts any finite nonnegative radius and bounds its effective path
geometrically. Negative and nonfinite native drafts remain visible for
correction without mutating the document; Escape restores the confirmed value.

Independent per-corner radii and corner smoothing continue to describe box
shapes. They do not change the Polygon or Star outline. A uniform radius edit
clears an imported independent-corner array consistently with box authoring.

## Shared rendering, picking and export

The implementation routes the following operations through the same path:

| Consumer | Integration |
| --- | --- |
| Native fills, uniform stroke geometry and vector masks | [`renderer.rs`](../crates/ferrodesign-app/src/renderer.rs) converts the cubic outline through public FerroUI geometry APIs. |
| Shape picking | [`model.rs`](../crates/ferrodesign-core/src/model.rs) uses rounded containment and rounded edge distances instead of the sharp polygon. |
| Variable-width strokes and width-handle sampling | [`stroke_profile.rs`](../crates/ferrodesign-core/src/stroke_profile.rs) uses the rounded primitive centerline. |
| SVG, flattening and other outline consumers | [`svg.rs`](../crates/ferrodesign-core/src/svg.rs) exposes the rounded path through `outline_path`. |
| Native PNG export | The normal FerroUI offscreen renderer paints the same prepared geometry. |
| FerroUI XAML export | [`vector_export.rs`](../crates/ferrodesign-app/src/shell/vector_export.rs) emits public `StreamGeometry` resources, ordered paint paths and the complete affine matrix for rounded primitives. |

Rounding is performed in local layer coordinates. Subsequent nonuniform scale
or shear transforms the circular geometry with the rest of the layer; it does
not reinterpret the radius in screen pixels. Radius changes invalidate the
retained visual, while placement-only changes reuse its prepared geometry.

The existing XAML export limitations still apply, including image-brush
fallbacks and uniform noncenter stroke alignment in mixed-stack export. This
feature does not add a new bitmap fallback or direct renderer backend calls.

## Work bounds and verification

The existing procedural point limit remains 3 through 128 sides. Stars have
twice that many vertices, for a maximum of 256. Fillet preparation is linear in
the vertex count and emits at most two cubics per vertex and at most
`3 × vertex_count + 1` path commands. There is no subdivision loop whose work
grows with the requested radius. Normal renderer caching controls when the
path is rebuilt.

[`polygon_corners_tests.rs`](../crates/ferrodesign-core/src/polygon_corners_tests.rs)
covers exact zero-radius compatibility, quarter-arc tangents and area, acute
maximum-radius clamping, the separate half-edge radius ceiling, concave and
reversed winding, zero/subnormal dimensions, bounded command counts, affine
area, shared picking/strokes, validation, locks, variable detachment and exact
history. Nine geometry tests passed in an isolated Rust harness using the
actual fillet/path/geometry code with a minimal node carrier. That run is not
an integrated editor or framework build.

[`polygon_corner_tests.rs`](../crates/ferrodesign-app/src/renderer/polygon_corner_tests.rs)
adds five native pixel/cache/mask regressions. They compare procedural layers
with the equivalent public cubic vector path, including affine transforms,
uniform and variable-width stroke stacks, transparent former tips, concave
notches and undo.

[`polygon-corners-browser.mjs`](../tests/polygon-corners-browser.mjs) adds four
browser checks and two native screenshots per rendering mode. It authors the
radius through native controls, checks rejected drafts and exact history,
compares PNG against independently decoded SVG, inspects public XAML geometry
and affine matrices, tests actual canvas picking, and detaches a same-value
variable binding. These native and browser regressions require their normal
integrated gates; their presence alone is not a passing validation result.

## Remaining scope

This feature does not add arbitrary vector-anchor fillets, per-vertex radius
authoring for polygons/stars or smoothing for their nonrectangular corners.
Changing a star's point count or inner ratio still
uses the existing procedural-shape functionality. Figma-native file-format
interchange and pixel-identical parity with all Figma primitive configurations
remain separate work.

## Primary references

- [Figma `cornerRadius` API](https://developers.figma.com/docs/plugins/api/properties/nodes-cornerradius/)
  specifies nonnegative fractional pixel values and the half-edge radius bound.
- [Figma shape tools](https://help.figma.com/hc/en-us/articles/360040450133-Shape-tools)
  documents rounded polygon/star corners and explains why a procedural polygon's
  bounding box remains independent of its visible outline.

The signed circular construction, additional tangent-distance reduction and
cubic approximation described above are explicit implementation choices.
