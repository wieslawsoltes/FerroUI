# Continuous corner smoothing

The native **Corner smoothing %** field accepts 0–100 for rectangles, frames,
components, instances and sections. The document stores `corner_smoothing` from
0 to 1; absent and zero values preserve legacy circular corners. Uniform and
independent radii remain editable. A batch skips locked layers, validates every
target before mutation and records one undo step. Repeating the current value
retains redo. Component instances retain explicit smoothing overrides.

[Native radius handles](CORNER_RADIUS_HANDLES.md) also edit the radius directly
on the selected shape. Uniform drags preserve smoothing; Alt/Option at press
edits one Rectangle corner. Smoothing itself remains a numeric inspector field.

## Geometry and performance

The shared core `corner_path` implements the cubic/arc/cubic construction in
[Figma's published corner-smoothing derivation](https://www.figma.com/blog/desperately-seeking-squircles/):
the smoothing parameter increases consumed edge length from `R` to `(1+s)R`;
the incoming cubic has collinear initial controls with a 2:1 handle allocation.
The central circular segment retains its original center. The outgoing cubic
is its reflected counterpart. This follows the published construction; it is
not evidence of numerical equality to every current Figma corner implementation.

Adjacent radii are normalized proportionally. Available edge space reduces
smoothing before reducing a radius. Each corner has at most three cubic
segments, independent of zoom or canvas size. No raster approximation is stored
in the document. The public FerroUI `StreamGeometry`, vector clip, stroke and
effect APIs consume the outline. Retained geometry is reused while panning;
changing the parameter invalidates only the affected visual and its ancestors.

Artwork, child clips, sibling vector masks, hit testing and SVG export share the
same corner outline. Smoothed box effects follow the existing general-path
effect implementation. Flatten bakes the contour and clears the corner fields.
Smart Animate interpolates smoothing. Appearance matching includes the value.
XAML emits native path geometry and a vector clip for smoothed boxes. CSS inspect
explains that `border-radius` alone cannot encode this contour.

## Validation scope

Core tests cover edge tangency, symmetry, independent radii, finite degenerate
geometry, normalization, validation, legacy serialization, atomic edits, locks,
no-op redo preservation and undo/redo. Native pixel tests compare smoothed
artwork with its equivalent explicit cubic path, child clipping, retained-cache
reuse and history restoration. The browser `corners` scenario edits the native
field, rejects an invalid draft, checks exact history, downloads FerroUI PNG and
SVG, and compares the PNG silhouette with Chromium's independent SVG decoder.
Run results belong in [VALIDATION.md](VALIDATION.md); the existence of a test is
not a claim that a particular source revision has passed it.

This smoothing field operates on box-shaped layers. [Polygon and star corner
rounding](POLYGON_CORNERS.md) uses the native radius field and a shared circular
fillet path. Arbitrary vector-anchor fillets and nonrectangular smoothing remain
separate work. A matched Figma reference capture is still needed to establish
visual parity.
