# Linear-gradient handles

Select one eligible layer, open its **Fill paints** editor, and choose **Edit on
canvas** in the linear-gradient paint card. The canvas then exposes the two
gradient endpoints, a draggable axis, and one marker per color stop. The chosen
paint keeps its position in the layer's fill stack.

This is a bounded linear-gradient interaction. Figma's
[gradient reference](https://help.figma.com/hc/en-us/articles/34208860210199-Use-gradients-as-a-fill-or-stroke)
describes linear, radial, angular and diamond gradients, movable color stops,
and stop insertion/deletion. Its [fill reference](https://help.figma.com/hc/en-us/articles/360041003694-Guide-to-fills)
also describes independently configurable fills in a layer's stack. The
implementation below covers the stated linear-fill subset. It does not establish
full gradient feature or visual parity with Figma.

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
opacity, and already contain a valid linear gradient with **2–256 stops**. A
node carrying vector-network geometry is excluded even if its node kind is a
normally supported box.

These supported geometries have native brush bounds that agree with their local
box. Arbitrary vectors, Boolean results, text, groups, polygons, stars and lines
need different brush-bound treatment and do not expose this interaction.
Radial/angular handles, stroke gradients, vector-region gradients, per-stop
variables and diamond gradients are outside this change. Their absence is not
filled by approximating another gradient type.

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
| `axis` | Start and end in world coordinates. |

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

The [exact-source core CI at `7f813672`](../../../docs/ferrodesign-evidence/2026-10-22-7f813672-core/README.md) passed all 511 tests in 1.65 seconds, including all five gradient core cases.

These are authored assertions. Node syntax checking does not execute a browser
or validate pixels. The newly integrated source still requires its exact-source
native and Software2D/WebGL2 CI gates; results and remaining failures belong in
[VALIDATION.md](VALIDATION.md). An earlier successful editor run cannot validate
this later feature. No matched Figma image comparison has established visual
parity for these handles.
