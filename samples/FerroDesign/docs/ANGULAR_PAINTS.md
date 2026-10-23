# Angular gradient paints

FerroDesign stores angular gradients as native paint data and draws them through
FerroUI's public `ImmutableConicGradientBrush`. The same prepared brush is used
by normal artwork, strokes, masks and PNG rendering. The editor and renderer do
not issue backend-specific drawing commands for angular paints.

## Document contract

```json
{
  "type": "angular_gradient",
  "center": { "x": 0.5, "y": 0.5 },
  "angle": 90.0,
  "stops": [
    { "offset": 0.0, "color": { "r": 255, "g": 0, "b": 0, "a": 255 } },
    { "offset": 1.0, "color": { "r": 0, "g": 0, "b": 255, "a": 255 } }
  ]
}
```

The center is relative to the brush's target bounds. The angle is measured in
clockwise degrees: zero points up and 90 points right. Offsets span one complete
turn. Different endpoint colors form a deliberate seam; coincident stop offsets
form hard color transitions. Existing solid, linear, radial and image paint
serialization remains unchanged.

Document validation rejects nonfinite centers/angles, empty stop lists and
nonfinite or out-of-range stop offsets. Rendering sorts a copied stop array and
normalizes the angle without changing the portable document. The native fill
editor retains stable stop indices while fields own focus. Shared style values
use the existing monotonically ordered stop contract, allowing duplicate offsets.

## Authoring and retained rendering

The native **Fill paints** and local fill-style editors expose angular paint
selection, center coordinates, angle, RGBA stops, stop offsets, paint opacity,
visibility and the existing stack operations. Type conversion takes the current
confirmed paint value and preserves the entire gradient stop stack. Re-selecting
an unchanged type does not reset its geometry. Creating an image continues to
require actual imported image data.

The existing transaction, draft-validation, Escape, undo/redo and style ownership
boundaries apply to angular paint edits. Editing a shared definition propagates
to linked layers; manually editing a linked layer detaches its corresponding
style binding. No angular-specific copy of the document or editor is introduced.

The visual fingerprint includes the center, angle and every stop's offset and
color. A changed angular paint rebuilds its immutable brush in normal retained
scene preparation. Unchanged camera frames reuse prepared brushes. Smart Animate
interpolates matching angular centers and stops and crosses the angle seam by
the shortest path; incompatible paint kinds use the existing fallback behavior.

## Interchange

| Format | Angular paint behavior |
| --- | --- |
| Native JSON | Preserves exact center, angle, stops, alpha and paint stack metadata |
| PNG | Renders the native FerroUI conic brush at the requested export resolution |
| FerroUI XAML | Emits `ConicGradientBrush` property elements for `Fill`, `Stroke`, `Background` and `Foreground`, including paint opacity and ARGB gradient stops |
| CSS inspection | Emits `conic-gradient` with center, starting angle, exact authored stop boundaries and alpha combined with fill opacity; a one-stop paint becomes a constant color |
| SVG | Emits an explicitly reported, bounded vector approximation because portable SVG has no conic paint primitive |

CSS uses premultiplied alpha interpolation, while the native brush interpolates
straight RGBA. When both color and alpha vary, CSS inspection adds sampled native
colors and a source comment disclosing the approximation. Ordinary intervals use
16 subdivisions, keeping the interpolation error below one quarter of an 8-bit
composited color channel. At most 1,024 additional stops are inserted across the
whole paint; unusually long stop lists share that budget and have a coarser
approximation. All authored stops, including coincident hard transitions, remain
in their stable order. Constant-alpha ramps need no extra stops.

SVG uses at most 1,024 sectors per paint. For ordinary stop lists, sector cuts
include authored stop boundaries so hard transitions remain at their angle.
Long stop lists use a fixed uniform sector budget. Shared internal edges use
`shape-rendering="crispEdges"`, avoiding translucent overlap or transparent
antialiasing seams. The outer layer geometry retains its normal antialiasing.
Both `SvgExport.warnings` and the native export toast disclose the approximation;
patterns carry `data-ferrodesign-approximation="angular-gradient"` metadata.
Brush reference coordinates and the SVG pattern tile are separate. Tiles use
user-space coordinates and cover the full stroke extent, so centered/outside
strokes do not wrap at the unstroked shape edge and horizontal/vertical lines
retain a nonzero paint tile. Centered brush coordinates use geometry bounds
inflated by half the stroke width; ordinary aligned shapes use their outline
bounds. Complex aligned curve/miter outlines, arrowheads and shaped text can have
native paint bounds that differ from this core-only SVG approximation, as the
export warning also states. This SVG representation is not an editable
angular-gradient round trip.

The existing XAML/CSS export subsets remain: a single native brush property does
not reproduce arbitrary stacked paints, rich-text range effects or every design
layer feature. The XAML angular brush itself retains the supported paint values;
this does not establish complete document export parity.

## Regression coverage

- Core tests cover JSON round trips, malformed imports, transactional edits,
  shared-style propagation/detachment, rejected edits preserving redo, bounded
  SVG output and Smart Animate.
- Native application tests render conic start/end seams, midpoint color, layered
  opacity and strokes; incremental angle/center/stop changes must match a freshly
  prepared scene. Additional tests exercise paint conversion and CSS/XAML output.
- `tests/angular-paints-browser.mjs` uses native controls, validates exact history,
  downloads PNG/SVG, compares native PNG colors against browser-decoded SVG and
  checks that the SVG approximation warning is visible. Exterior center/outside
  stroke pixels and a horizontal line are compared against downloaded native PNG.

Execution results belong in [VALIDATION.md](VALIDATION.md); the existence of these
assertions does not itself establish a successful run.
