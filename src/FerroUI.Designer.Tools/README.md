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

`grid_track_values` reads authored row/column lengths, including explicit definition elements.
`plan_grid_track_change` accepts `GridTrackChange::{Set, Insert, Remove}` and produces one
validated edit batch. Insertion shifts later cells and expands spans that cross the new track;
removal shifts or clamps cells and shrinks affected spans. The axis always retains at least one
track. Compact attributes remain compact, while definition elements retain their names,
namespaces, min/max/shared-size metadata and surrounding comments. Explicit `.Children`
collections and local namespace changes are supported.

Native hosts use `plan_grid_track_change_with_placements` with a `GridTrackPlacement` map for
effective styled child indices and spans. The host must supply every authored child's placement
and reject active bound coordinates before planning. Without that map the portable engine uses
literal source values. Bound/object-valued placement and unsupported definition expressions are
rejected; the planner never evaluates bindings or guesses style state. The native workspace
also protects definition collections supplied entirely by a style or preview factory.

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

## Extensible layout helpers

`LayoutAdapter` also supplies renderer-independent design-time features. Existing adapters
remain compatible because the new methods have defaults:

- `design_helpers(context, request)` returns a `LayoutHelperScene`: semantic guide primitives,
  measured handles and optional placement. The default provides a container outline.
- `placement(context, panel, position, dragged)` returns an optional `LayoutPlacement` with
  the actual AST content container, source insertion boundary, child attributes, visual bounds
  and an explicit rejection reason. The default returns `None`.
- `plan_helper(context, action, value)` plans one handle edit. Standard intents reuse the
  source planners; applications override `LayoutHandleAction::Custom` for additional tools.

The registry exposes corresponding methods, validates returned scene geometry and transactions,
and publishes `revision()` so hosts can invalidate an in-flight gesture when a provider changes.
Register providers using canonical names such as `{urn:my-controls}OrbitPanel`. `adapter_for`
resolves namespace URI plus local name, so changing `app:` to another XAML prefix retains the
same feature provider. Lexical and unqualified registrations remain supported. An unknown
application type named `Grid` or `StackPanel` does not silently inherit framework semantics.

`LayoutHelperRequest::new(panel)` defaults to no pointer, selection or dragged nodes. Set
`selected_nodes` to show direct-child margins and `dragged_nodes` to exclude moved items from
placement calculations. Layout coordinates use the measured context's space; the native host
owns panel/artboard transforms, zoom, screen-sized handles, hit testing and gesture lifetime.
Labels use a center anchor. Roles distinguish container/content geometry, grid tracks, flow,
spacing, padding, relations, placement, rejection and measurements without prescribing colors.

Built-in features include:

| Layout | Design-time features |
| --- | --- |
| Canvas | Local-axis/origin guides, pointer coordinates and position preview. |
| Grid | Actual track boundaries, lexical units, cell preview, row/column sizing and spacing handles. |
| Stack / ReversibleStack | Direction arrow, measured gaps, spacing handle and orientation/reversal-aware insertion. |
| Wrap / Flex | Measured line gaps, item/line spacing, direction/reversal and visual-order-aware insertion. |
| UniformGrid | Measured cells, first-column reservation, row/column spacing and capacity-aware cell insertion. |
| Dock | Dock-order labels, remaining fill identification, spacing and edge placement previews. |
| RelativePanel | Resolved sibling constraint connectors and relationship labels. |
| Content hosts | Content bounds, supported padding handles and explicit occupied-content rejection. |
| Generated panels | Informational outline and rejection; no direct item-editing handles. |

The native Drop authoring path and built-in placement visualization consume the same placement
calculation. Placement indices refer to the original AST element sequence, including explicit
`Children`/`Content`/`Child` containers; a host moving existing nodes removes their earlier
source ranks when applying that boundary. Previewing does not create a document transaction.

Handle values represent logical measured lengths. Dragging a star Grid track scales its weight
by `new_extent / measured_extent`; dragging Auto deliberately authors pixels. Definition
metadata, comments and namespaces are preserved by the existing Grid transaction planner.
Bound/object-valued properties expose a disabled reason and remain protected at commit.
Signed margins preserve negative values and the full precision of untouched components.
`value_after_delta` applies axis, direction and range constraints without authoring source.

Custom provider, namespace-alias, source-preservation and shared-placement examples are exercised
in `layout_helpers_tests.rs`. Scene validation rejects nonfinite geometry, duplicate handle IDs
and unbounded output. A native host should capture source/AST/URI identity and registry revision
at gesture start, preview immutable plans, and commit one validated transaction on release.

`plan_layout_attributes(document, node, attributes)` is the shared source-safe boundary for
native placement attributes. It retains existing framework namespace aliases, rejects duplicate
aliases and expressions/property elements, and qualifies new attached properties when the
current default namespace belongs to the application. Adapter callers validate semantic values
before calling it. The candidate attribute batch is validated atomically before returning edits.

## Component resources and theme variants

`component_resource_resolution(document, target, key, context)` returns a
`ComponentResourceResolution`: candidate declaration IDs, `may_be_missing`, and `external`
uncertainty. Its `resolved()` method yields an ID only when every applicable lookup path
selects the same declaration. The existing `resolve_component_resource` and
`visible_component_resources` APIs pass no context and conservatively consider all authored
theme variants. They do not guess the host's current light/dark theme.

For a live preview, supply `ComponentResourceContext { theme, inherited_themes }` with the
actual runtime theme key and its full inheritance chain, nearest inherited variant first.
`None` or `Default` as the theme consults the Default dictionary before merged dictionaries;
it does not mean an unknown theme. Lookup follows the framework order: local entries,
selected variant, inherited variants, Default variant, merged dictionaries in reverse order,
then outer resource scopes. Built-in `x:Static ThemeVariant.Light/Dark/Default` keys and
namespace aliases are understood. External providers and other expression keys remain
explicitly uncertain.

`resolve_component_resource_in_context` and `visible_component_resources_in_context` expose
that concrete lookup. Pair the latter with `plan_component_action_in_context` for
`ComponentAction::ApplyResource`; the picker and commit use identical resolution rules.
The host must verify the actual theme and document/selection snapshot again before applying
the returned transaction. Targets with different active themes require their own context.
All other component commands retain the normal conservative checks even through this API.

Catalog usage counts and `component_resource_references` include possible uses across
variants and fallbacks behind external providers. Deletion cannot discard a resource merely
because no single active variant was selected. Renaming one member of a resource family is
rejected when a shared reference can select other declarations; edit the whole family in
source in that case. Template copies compare possible dependencies as well as unique
resolutions. The portable engine never loads external dictionaries or executes arbitrary
key expressions while performing these checks.

Resource deletion is limited to explicit string-key entries. It does not remove an entire
unkeyed dictionary container or infer that an implicit/type-keyed resource is unused; those
structural operations remain source edits until the host supplies their complete references.
