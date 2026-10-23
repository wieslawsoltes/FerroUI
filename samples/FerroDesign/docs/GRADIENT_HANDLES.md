# Native gradient handles

Select one eligible layer, open its **Fill paints** editor, and choose **Edit on
canvas** in a linear, radial or angular gradient paint card. Linear gradients
expose two endpoints; radial gradients expose their center and relative radius;
angular gradients expose their center, sweep angle and color stops on a ring.
The chosen paint keeps its native representation and its position in the layer's
fill stack.

These are bounded native gradient interactions. Figma's
[gradient reference](https://help.figma.com/hc/en-us/articles/34208860210199-Use-gradients-as-a-fill-or-stroke)
describes linear, radial, angular and diamond gradients, movable color stops,
and stop insertion/deletion. Its [fill reference](https://help.figma.com/hc/en-us/articles/360041003694-Guide-to-fills)
also describes independently configurable fills in a layer's stack. The
implementation below covers linear fills and the existing radial/angular paint
models. It does not establish full gradient feature or visual parity with Figma.

## Native interaction

| Target or key | Result |
| --- | --- |
| Drag the start or end marker | Move that endpoint in the layer's relative local bounds. |
| Drag a bare part of the axis | Translate both endpoints together, retaining the axis direction and length. |
| Drag a color-stop marker | Move that authored stop along the transformed axis; its position is clamped to 0–1. |
| Double-click a bare part of the axis | Append a stop at that position, using the existing straight-RGBA interpolation rule. |
| Tab | Select start, end, then stops in their authored source order, wrapping at the end. |
| Shift+Tab | Traverse the same targets in reverse. |
| Arrow keys with an endpoint or axis selected | Move by one world unit; Shift uses ten and Alt/Option uses 0.1. |
| Arrow keys with a stop selected | Left/Down decrease and Right/Up increase its offset by 0.01; Shift uses 0.1 and Alt/Option uses 0.001. |
| Delete with a stop selected | Remove that stop when more than two remain; keep the layer. |
| Escape during a drag | Restore the complete pre-gesture document and retain the editor. |
| Escape without an active drag | Leave the gradient editor. |
| Enter | Finish the current gesture and leave the gradient editor. |
| Save / Ctrl+S | Finish a held gesture before serializing the portable design file. |

Deleting at the two-stop minimum produces a native validation message and
retains the selected stop, artwork and history. Existing numeric paint controls
remain the place to edit stop colors, paint opacity and gradient type.

The target must be one selected, visible and unlocked rectangle, ellipse,
frame, component, instance or section with positive finite local dimensions and
an invertible world transform. The chosen fill must be visible, have positive
opacity, and already contain a valid linear, radial or angular gradient with **2–256 stops**. A
node carrying vector-network geometry is excluded even if its node kind is a
normally supported box.

These supported geometries have native brush bounds that agree with their local
box. Arbitrary vectors, Boolean results, text, groups, polygons, stars and lines
need different brush-bound treatment and do not expose this interaction.
Independent radial X/Y radii, radial focus/ellipse rotation, stroke gradients,
vector-region gradients, per-stop variables and diamond gradients remain outside
this interaction. The existing single radial scalar is retained exactly;
unsupported ellipse parameters are not approximated.

## Coordinates, tied stops and paint identity

Start and end are relative coordinates in the existing paint model. Values may
extend beyond the box; authored coordinates are finite and bounded to
−1000…1000. Endpoint and whole-axis drags convert world displacement through the
inverse complete layer transform, including center rotation, reflection,
nonuniform scale and shear. A translated axis is clamped as a pair at the
coordinate limit so its extent is retained.

A stop drag projects the pointer's world displacement onto the world gradient
axis. Its original index, color and alpha remain attached to it even when it
crosses another stop. Equal offsets retain their original source ordering;
the renderer's existing stable ordering still determines a hard transition.
Inserting a new stop appends it without renumbering existing stops or changing
their colors. Stop deletion removes only the selected source entry.

The stop markers sit sixteen screen pixels away from the axis. Nearby or tied
stops use separated lanes so distinct authored entries can be selected. Marker
placement is an editing affordance: the actual gradient positions remain on
the axis. Very short or coincident endpoint positions likewise use displaced
markers. Every drag measures displacement from its original press, so pressing
a displaced marker does not jump the paint. A coincident axis must first be
opened by moving an endpoint before a meaningful projection or bare-axis
insertion is possible.

The canvas uses public FerroUI drawing and input APIs for the adorners. Existing
FerroUI brushes and the Vello renderer continue to draw the artwork. There is no
DOM handle layer, Skia path, new gradient interpolation mode or alternate
artwork representation. Adorners are not included in artwork exports.

## Styles and transactions

An actual change updates only the chosen fill's paint, detaches the layer's
shared fill-style link, removes a layer-wide `fill` variable binding if present,
and records the existing `fills` component override. The style definition,
other linked layers, other fills, fill opacity/visibility, strokes, geometry and
unrelated variable bindings remain unchanged. Applying a fill variable normally
replaces the whole fill stack with its resolved solid color; the stale-binding
cleanup rule also handles imported snapshots that carry a gradient and old
binding metadata. It does not introduce variable-backed gradient stops.

Hover and a press without movement leave the document and revision unchanged.
One captured gesture owns one transaction: moves preview the paint, release
commits one changed edit, and Undo/Redo restore the complete before/after
documents. Returning exactly to the original paint restores its original
style/binding metadata and creates no history entry. An unchanged gesture must
also preserve an existing redo entry.

Escape and native capture loss cancel the owned gesture, restoring the full
snapshot and redo history. A later pointer move or release cannot recommit it.
Undo/Redo retain gradient mode while the same target node and fill remain
eligible; an invalid selected stop is cleared rather than silently retargeted.
Other authoring commands finish a held gesture before making their own changes
and reconcile the editor against the resulting target.

## Saving and host lifecycle

A held gradient edit marks the host's native draft as dirty. Background
serialization waits for the gesture to resolve. Explicit Save and lifecycle
commit finish the owned transaction before serializing its preview.

For a previously clean IndexedDB snapshot, the real `pagehide` handler therefore
commits and saves the held edit once. A subsequent pointer release must not add
another history entry, serialization or durable revision. Cancellation and an
unchanged release must also allow any earlier pending save to resume rather
than leaving it stranded behind the gesture guard.

The native state export exposes `gradientEdit` for read-only inspection:

| Field | Meaning |
| --- | --- |
| `node`, `fill` | Current layer and source fill index. |
| `active` | Whether a captured gradient transaction is active. |
| `selected`, `hover` | A start, end, stop or axis target; stop targets include the authored index. |
| `handles` | Local/world/canvas positions, displaced marker positions and stop offsets. |
| `axis` | Start and end in world coordinates; center/radius or center/angle for radial/angular paints. |
| `gradientKind` | `linear`, `radial` or `angular`, from the actual authored paint. |
| `ring` | Bounded native radial outline/angular editing ring in canvas coordinates. |

These diagnostics do not set document values, dispatch commands or synthesize
input. Browser tests use marker coordinates only to aim real pointer events.

## Regression coverage and validation status

The [gradient-handles browser scenario](../tests/gradient-handles-browser.mjs)
imports two isolated starting fixtures and then authors exclusively through
native controls, pointer input and keyboard input. Its ten checks cover:

1. Native paint-card entry, immutable hover and separated tied-stop markers on
   a rotated, reflected and sheared multi-fill layer.
2. Fractional endpoint movement, exact shared-style detachment and preservation
   of another linked layer and an unrelated live radius binding.
3. Escape, late pointer events, exact drag-back and unchanged presses with a
   real redo entry still available.
4. Trusted Chromium touch start/move/cancel reaching the production native
   capture-loss path, followed by execution of the preserved redo.
5. Stop crossing and stop-boundary clamping while retaining source indices and RGBA
   colors.
6. Whole-axis translation without changes to other document fields.
7. Clean-baseline `pagehide`, requiring exactly one history entry,
   serialization and durable IndexedDB revision without a post-event flush.
8. Actual Ctrl+S during capture, with the downloaded document compared exactly
   against the preview and the late release checked for duplicate history.
9. Real double-click insertion, a fine keyboard stop nudge, source-order Tab
   traversal and stop deletion.
10. A fresh two-stop validation error that preserves redo, Enter exit, and
    native PNG color samples proving that moved endpoints affect exported
    artwork without exporting adorners.

The scenario retains two native canvas screenshots, a native PNG export, the
portable design download and bounded read-only touch observations for each
renderer mode. The native/core tests separately cover invalid targets and
coordinates, limits, metadata cleanup and transaction ownership.

Stop swatches retain typed immutable FerroUI brushes with explicit alpha, red,
green and blue channels. They do not pass the document's RGBA hexadecimal text
through FerroUI's ARGB parser. A native regression covers two translucent colors
and zero alpha over the actual white marker backing, plus brush reuse and
unchanged document/history state. It awaits the same native CI execution gate.

The [exact-source core CI at `7f813672`](../../../docs/ferrodesign-evidence/2026-10-22-7f813672-core/README.md) passed all 511 tests in 1.65 seconds, including all five gradient core cases.

These are authored assertions. Node syntax checking does not execute a browser
or validate pixels. The newly integrated source still requires its exact-source
native and Software2D/WebGL2 CI gates; results and remaining failures belong in
[VALIDATION.md](VALIDATION.md). An earlier successful editor run cannot validate
this later feature. No matched Figma image comparison has established visual
parity for these handles.


## Radial geometry and authoring

The existing radial paint has a relative center and one radius. FerroUI's
`ImmutableRadialGradientBrush::with_radius` applies that same relative scalar to
both dimensions of the actual brush box: local radii are `radius × width` and
`radius × height`. A nonsquare layer therefore has an elliptical radial boundary.
The native adorner displays that boundary under the complete layer transform;
it does not use a circle in world space or derive a new paint representation.

The start handle moves the center, and the end handle changes the radius. A
radius gesture measures local displacement from the logical radius endpoint,
then takes the norm in normalized box coordinates. It preserves the native
radius when a press only selects a displaced marker. Radius is finite and
bounded to 0–1000. A zero radius retains separately accessible endpoint markers
and can be reopened by dragging the radius handle. The renderer's existing
minimum effective radius is unchanged.

Color stops sit on the center-to-horizontal-radius axis. Dragging one projects
world displacement onto that axis and retains its authored index. Bare-axis
drag translates the center without changing radius or stops. Double-clicking
the axis inserts a stop using the same interpolation and source-order rules as
linear gradients. Keyboard movement, deletion, transaction ownership, Escape,
capture loss and lifecycle serialization use the shared native editing path.

## Angular geometry and authoring

Angular paints use FerroUI's native conic brush: angles run clockwise from the
upward ray in **local pixels**. The editing ring is a local circle with radius
half the smaller box dimension. It is an editing affordance; conic artwork has
no radius field. The complete affine layer transform maps the ring to the
canvas, so reflection, scale and shear never change the authored angle's
coordinate system.

The start handle moves the center; the end handle rotates the sweep's starting
ray. Stop markers lie on the ray at `angle + 360 × offset`. Offsets zero and one
share the seam, yet remain independently selectable; source-order ties likewise
retain distinct markers. A bounded spatial marker index separates neighboring
markers without reordering or rewriting their stops. The ring contains at most
512 line segments, cached with the prepared handles. These segments draw only
the native adorner; artwork continues through the public conic brush.

Pointer gestures inverse-transform displacement and use the logical direction
before visual marker separation. Successive signed angles accumulate across the
0/360 seam. A stop keeps its original index and RGBA value; its offset clamps at
zero or one. A sweep angle retains signed authored turns and is bounded to
±360,000 degrees. Imported finite angles outside that authoring range remain
available through the existing numeric paint controls. Returning to the exact
press position restores the complete original document and redo state.

Double-click a bare part of the **ring** to append a stop at that angular
position. The center-to-angle spoke translates the center; its interior is not
a linear stop-insertion axis. Stop keyboard increments and deletion retain the
shared rules. Endpoint arrow keys displace the logical handle by world units,
then project that movement back to the native geometric parameter. Numeric
paint fields remain the place for exact center/radius/angle and stop-color
entry.

### Additional regression coverage

Six core regressions cover the real normalized radial ellipse, nonsquare local
conic angles, signed seam crossing and large finite direction magnitudes,
kind-preserving atomic edits/history, invalid radius/angle/stale-kind rejection,
and stable stop order/RGBA interpolation.

Five native regressions cover affine radial radius/center history, conic ring
geometry and separated seam/tied stops, sweep rotation across 360 degrees,
exact drag-back/redo/Escape/once-only finish, conic stop clamping with actual
native touch-capture cancellation, ring insertion/keyboard/deletion, and real
Vello adorner pixels. These extend the existing eight native linear-gradient
cases without replacing their assertions.

Seven additional browser checks enter through the real native paint cards and
use trusted pointer/keyboard input: affine radial radius/center edits and exact
history; drag-back with executable redo; affine angular seam rotation and
separated seam stops; stable-index conic stop movement; ring insertion and
exact RGBA interpolation; and native PNG samples at equal normalized X/Y radii
on a nonsquare rectangle; and conic PNG samples proving the edited start ray and
quarter/half-turn colors. The browser scenario now has seventeen feature checks.

Angular angle and insertion-color expectations use the trusted pointer
coordinates delivered to the production bridge. The independent input oracle
inverts the fixture's authored affine transform, accumulates local ray changes
across the seam, and samples the original source-ordered stops at the actual
second-press offset. This retains the `1e-6` angle/offset assertions and exact
RGBA checks despite Chromium rounding requested fractional coordinates.
Requested and delivered positions, native press evidence and the computed
expectations are saved in `*-angular-gradient-input.json`. Five focused oracle
tests cover quantized affine motion, displaced markers on nonsquare boxes,
winding and exact drag-back, byte interpolation boundaries and tied-stop order.

The existing linear checks, original pixel expectations and lifecycle gates
remain enforced. New source assertions are separate from executed validation;
terminal results and exact source identities are recorded in [VALIDATION.md](VALIDATION.md).
