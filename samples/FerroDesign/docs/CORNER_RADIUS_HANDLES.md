# Corner radius handles

One selected, visible and unlocked supported layer exposes native corner-radius
handles in the Select tool. Hover a corner handle and drag to change the radius.
Rectangles, frames, components, instances and sections support uniform radius
drags; polygons and stars retain their editable procedural geometry. Hold
**Alt** on Windows/Linux or **Option** on macOS when pressing a rectangle handle
to change that corner independently. The modifier is captured at press time.

The interaction follows the canvas radius and rectangle modifier workflow in
[Figma's corner-radius reference](https://help.figma.com/hc/en-us/articles/360050986854-Adjust-corner-radius-and-smoothing).
Handle placement and the geometry described here are FerroDesign's explicit
implementation. A matched Figma capture has not established visual or numerical
parity.

## Radius and geometry

Handles edit local design-pixel values, including fractions. A drag preserves
the node kind, box, path, polygon point count, star inner ratio, center rotation,
reflection, affine transform and existing corner smoothing. It does not flatten
or resize the primitive. The same rounded paths continue to drive public
FerroUI artwork, picking, masks and export; see [continuous box corners](CORNERS.md)
and [polygon/star fillets](POLYGON_CORNERS.md).

Each handle has a local radius direction and its transformed world direction.
Pointer displacement is measured from the original press, relative to the
original authored radius. The minimum sixteen-screen-pixel inset keeps small or
zero radii accessible without changing a value merely by pressing its marker.
Marker displacement is also capped at one third of the nearest other vertex's
screen distance, separating handles whose true arc centers coincide at maximum
radius. This changes marker placement only. Rotation, reflection, nonuniform
scale and shear transform the handle with the layer. Geometry limits can reduce
the effective visible radius while retaining the authored value, as documented
for the underlying primitive.

A uniform drag clears an independent rectangle-corner array. An Alt/Option
rectangle drag retains the uniform value, initializes the four independent
values from it if needed, and changes only the pressed corner. An actual change
removes the uniform `corner_radius` variable binding and records the existing
`corner_radius` component override. Other bindings and document fields remain
unchanged. Hover, a press without movement and a drag that returns exactly to
its original value preserve the original metadata and redo history.

The native canvas draws adorners through public FerroUI `DrawingContext`
operations. There is no DOM handle layer or direct Skia/backend drawing path.
Handles are editing adorners and do not enter artwork exports. Resize, rotation
and deep-selection hit behavior retains its own precedence. A marker is omitted
when its minimum inset falls outside the local box or overlaps the protected
screen-space area around an existing resize or rotation target; small shapes
can still use the numeric radius field.

## Gesture and save lifecycle

A captured gesture owns one document transaction. Pointer moves preview the
radius; release commits a changed gesture once. Escape cancels the active drag
before clearing selection. Native capture loss also cancels. Both restore the
complete pre-gesture document, including bindings and overrides, and retain an
existing redo entry. Later pointer events cannot commit a canceled gesture.

The active gesture marks the host draft dirty and delays background document
serialization. Explicit save and lifecycle commits finish it before serializing.
The browser's real `pagehide` handler therefore saves a held preview as one
committed edit; a subsequent pointer release cannot create another history or
storage revision. A stale gesture must not commit or cancel a replacement
transaction owned by another operation.

`cornerRadius` in the native state export is read-only diagnostic information.
It includes the target node, hovered and active corner, independent-edit flag,
and prepared handle positions/directions in local, world and canvas coordinates.
It does not provide an alternate mutation API.

## Browser coverage and limits

The [corner-radius-handles browser scenario](../tests/corner-radius-handles-browser.mjs)
uses isolated starting fixtures followed by real mouse/keyboard input and
native FerroUI screenshots. Its assertions cover immutable hover, fractional
uniform drags, Alt rectangle editing, unchanged clicks, full undo/redo and
Escape rollback with late pointer events. Polygon and star fixtures include
center rotation, reflection and shear. The lifecycle case starts from a clean
IndexedDB snapshot and requires exactly one history entry, serialization and
durable revision after `pagehide`, without an explicit post-event flush.

A separate real Chromium touch gesture uses `touchStart`, `touchMove` and
`touchCancel`. The production browser bridge forwards touch cancellation to the
native pointer-disposal/capture-loss path. Read-only input observations require
trusted touch events and are saved with the report. This case does not claim
that releasing DOM mouse capture invokes the same native path.

These are authored regression assertions, not evidence that the current source
has passed browser validation. Exact source/build results belong in
[VALIDATION.md](VALIDATION.md).

Canvas radius editing currently targets one supported layer at a time.
Independent canvas corners are restricted to Rectangle; frame/container
independent values remain available through their numeric fields. Polygon and
star handles edit their uniform radius, not individual vertices. Arbitrary
vector-anchor fillets, nonrectangular corner smoothing and a canvas smoothing
handle remain outside this feature.
