# FerroUI portable designer tools

`ferroui-designer-tools` turns authoring gestures into transactions for
`ferroui-designer-core`. It has no renderer, UI dispatcher or window dependencies. The native
workspace supplies measured geometry from actual FerroUI controls and commits returned `Edit`
commands through the same history, collaboration and preview boundary used by source edits.

## Layout adapters

Create `LayoutRegistry::built_in()` and `LayoutContext::new(&document, &measured_bounds)`.
Each context uses consistent logical layout units for its measurements and actions. The host
converts artboard points and movement vectors into the actual layout parent's coordinate space;
different transformed parents receive separate contexts. `plan` validates the resulting source
on a private document before returning an edit list. Custom panels register a `LayoutAdapter`
with explicit capabilities and property names.

`plan_many` combines independent actions against one snapshot; `plan_batch` accepts actions
with different measured contexts referencing the same `Document`. Both validate the combined
transaction once. Multiselection move, alignment and distribution use this path so source
is not cloned and reparsed for each control. Conflicting writes to the same property, combined
relative constraint cycles, and multiple inserts into a single-content host are rejected.
Structural insertion indices refer to the snapshot; an adapter should combine dependent
structural changes into one action.

| Panel | Authoring behavior |
|---|---|
| Canvas | Position in panel coordinates; convert right/bottom anchors using measured bounds; resize without destroying expressions. |
| Grid | Row/column/span placement; pixel/star/Auto definitions; edit a definition while retaining its other attributes; hit-test measured tracks on drop. |
| StackPanel / ReversibleStackPanel | Source ordering, orientation/reversal-aware insertion, spacing and margin edits. |
| WrapPanel | Measured line-aware insertion, orientation, item dimensions, item and line spacing, alignment. |
| DockPanel | Dock edge assignment, placement before the last fill child, reorder and spacing. |
| UniformGrid | Row/column/first-column configuration and cell-based insertion. |
| RelativePanel | Named sibling constraints, cycle checking, panel-relative alignment properties and margins. |
| FlexPanel | Direction/wrap/alignment/spacing metadata and child ordering through `Flex.Order`. |
| Panel | Source child ordering and margin/size manipulation. |
| Border / Decorator / ContentControl / UserControl / Window / ScrollViewer / Viewbox / Button / ToggleButton | Drop into an empty content slot; preserve explicit `.Child`/`.Content` syntax; move/resize the child with margins; reject replacing existing content implicitly. |
| VirtualizingPanel / VirtualizingStackPanel / VirtualizingCarouselPanel | Declare generated-container ownership; edit supported panel properties instead of inserting into containers managed by an items owner. |
| DateTimePickerPanel / TextSelectionHandleCanvas | Declare generated-container ownership; expose supported panel settings. |
| TableViewColumnHeadersPresenter / TableViewCellsPresenter / AdornerLayer / OverlayLayer / PopupOverlayLayer / TextSelectorLayer | Recognize framework-owned presentation layers; preserve their generated content. |

Auto Grid tracks, constrained/shared-size tracks, and grids with spacing require actual `GridLines` from the host, supplied through
`LayoutContext::with_grid_lines`. The engine never guesses their measured size. Grid definition
property elements remain property elements, and their min/max or shared-size attributes survive
track edits. The adapter recognizes explicit `.Children` collections and preserves other property
elements when changing child order.

`LayoutContext::with_resolved_layout` accepts effective margins, visibility and optional arrange
slots from the native host. This preserves styled margins, avoids adding margins twice when
converting Canvas anchors, and lets UniformGrid account for bound or styled visibility. Without
live metadata the engine reads literal source values. UniformGrid insertion follows the runtime's
square automatic dimensions, first-column normalization and spacing.

Resize sets explicit dimensions and adjusts opposite margins to preserve the control's occupied
layout slot. The requested border rectangle stays anchored for Start, Center, Stretch and End
alignment, and flow siblings keep their original slots. Growing beyond the occupied slot can
therefore produce negative trailing margins; authors can subsequently edit margins or panel
configuration to reflow siblings. Canvas resize converts opposite anchors even when the leading
edge stays stationary. An unchanged resize is a no-op.

Move, resize and layout commands refuse to overwrite bindings, resource expressions or property
object elements. The property editor exposes an explicit replacement policy for deliberate edits.

## Geometry and selection

`SnapSettings` snaps edges and centers against explicit guides, nearby object bounds and a grid.
Tolerance is in document units; a host converts its desired screen tolerance using the zoom.
For a drag, call `SnapSettings::prepare(&peers)` once and retain its `SnapIndex`. Queries inspect
only the neighboring sorted guides for each anchor. `snap_with` accepts an updated grid size and
tolerance without rebuilding geometry. The one-off `snap` API remains available for callers
that do not reuse a peer snapshot. Both paths have the same tie-breaking and finite-value checks.
Alignment and equal-gap distribution return per-node deltas, allowing each parent adapter to
choose the proper authored property representation. `Affine` provides composition, inversion,
rotation/scaling around a pivot and transformed bounds.

`Selection` provides hit testing in paint order, marquee selection, add/toggle/subtract modes,
locked/hidden exclusions and pruning after document edits. `outermost` removes selected
descendants so moving a group does not move a child twice. `MoveGesture` snapshots the start
revision, immutable syntax identity and pure preview geometry; a stale gesture cannot commit
over a concurrent edit or into a different document with coincidentally equal IDs/revisions.
The selection bounds and peer guide index are computed once. Duplicate runtime bounds cannot
toggle the same authored node twice during marquee selection.

See the [performance evidence](../../docs/designer/performance.md) and run
`cargo bench -p ferroui-designer-tools --bench snapping` to compare one-off and prepared queries.

## Property metadata and editors

`PropertyRegistry` exposes common and panel-specific metadata, attached layout properties,
category, type, animation eligibility and read-only status. Hosts can register custom type
metadata, using `@PanelType` for properties attached to its children. Runtime reflection remains
the authority for the actual properties supported by a loaded custom control.

`EditorRegistry` selects extensible `ValueEditor` validators independently of native widget
factories. Built-ins cover numeric constraints, integers, booleans, enums, thicknesses, colors,
grid lengths/definitions, text, object references and transforms. Multi-selection reports mixed
values and creates one validated command batch. Binding and markup-expression syntax is parsed
through the existing core engine. The workspace can associate an editor ID with a native
FerroUI control without introducing UI dependencies into this crate.
