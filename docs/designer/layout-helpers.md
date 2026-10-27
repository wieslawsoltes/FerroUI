# Canvas layout helpers and panel extensions

The designer presents the current panel's layout rules directly on the artboard. A
selected container exposes its own helpers; a selected child exposes its common
layout parent. During an asset insertion or object move, the deepest eligible
authored container supplies a placement proposal. The proposal describes the
actual content slot, source insertion boundary and attached properties that the
transaction will use.

The surface draws a prospective insertion caret, cell, dock region or content
area before committing. Rejected destinations remain visible with a reason.
Pointer movement changes only this transient presentation; release commits one
validated edit. Escape, capture loss, selection changes and stale source cancel
the draft.

## Reference model

The extension architecture follows the separation documented in Microsoft's
[XAML designer extensibility migration guide](https://github.com/microsoft/xaml-designer-extensibility/blob/main/documents/xaml-designer-extensibility-migration.md):
`AdornerProvider`, `ParentAdapter` and `PlacementAdapter` supply distinct design-time
features, and modern surface-isolated extensions interact with model APIs rather
than holding runtime control objects. Type registration uses string identities.
The [Visual Studio interaction API](https://learn.microsoft.com/en-us/dotnet/api/microsoft.visualstudio.designtools.extensibility.interaction?view=visualstudiosdk-2022)
also separates adorner coordinates, selection, tools, gestures and placement.

FerroUI adapts that division to its existing Rust/XAML architecture. It does not
load `.design.dll`/`.designtools.dll` assemblies or claim Visual Studio binary API
compatibility. Portable providers receive model and measurement snapshots; the
native host owns runtime measurement, coordinate conversion, pointer capture,
rendering and commits. The existing document transaction is the single undo unit.

For interaction presentation, [Figma auto layout](https://help.figma.com/hc/en-us/articles/360040451373-Guide-to-auto-layout-in-Figma)
provides the useful connection between flow, gap, padding and nested containers.
[PowerPoint smart guides](https://support.microsoft.com/en-gb/powerpoint/training/guides-for-arranging-things-on-a-slide)
provide alignment feedback while objects move. FerroUI uses bounded alignment
lines, measured neighboring gaps and selection-size badges, keeping the underlying
panel's layout semantics authoritative.

## Feature architecture

| Layer | Responsibility |
| --- | --- |
| `LayoutRegistry` | Resolve the panel's canonical XAML type identity and capability policy. Re-registering a provider advances the registry revision. |
| `LayoutAdapter::design_helpers` | Describe panel-local guides, labels and editable handles using the immutable model and measured `LayoutContext`. |
| `LayoutAdapter::placement` | Propose the real content owner, insertion boundary, attached values and prospective bounds for a pointer position. |
| `LayoutAdapter::plan_helper` | Convert a semantic handle intent and numeric draft into source `Edit` values. Custom providers can define custom intents. |
| `DesignerSession` native bridge | Measure layout, convert artboard pointers, resolve nested candidates, enforce content ownership and produce a guarded `LayoutEditPreview`. |
| `DesignSurface` | Cache idle measurements, own capture and gesture drafts, expose keyboard equivalents and commit or cancel. |
| Native helper renderer | Paint the portable scene, final placement, handles, labels, finite measurements and optional cached drag images. It never writes source. |

The three new `LayoutAdapter` methods have defaults, so existing third-party
adapters remain source compatible. A provider that adds no helper feature receives
a generic container outline. Unknown native `Panel` subclasses retain their layout
algorithm and can use the host's generic content/transform fallback until a
dedicated provider is registered.

### Coordinate contract

`LayoutHelperScene` uses the same measured coordinate space as its `LayoutContext`.
The native bridge constructs this in the target panel's local coordinates and
returns a `LayoutHelperSnapshot` with `to_artboard: Affine`.

The renderer transforms every line endpoint and rectangle corner explicitly.
Rotated, reflected, scaled and sheared panel helpers therefore retain their actual
quadrilaterals. Font size, pen width, insertion end caps and grip hit targets are
expressed in screen units divided by the canvas zoom; an ancestor scale does not
make an interaction grip disappear or become enormous. Noninvertible transforms
cannot supply editable hit targets.

Labels use center anchors, remain upright, and can be clamped to the visible
artboard rectangle. Overlapping secondary labels are suppressed. The main
placement description remains visible because it explains the pending action or
rejection.

### Portable scene contract

`LayoutGuide` pairs a semantic role with one of three geometries:

- `Line { start, end }` for tracks, flow arrows, dimensions and relations.
- `Rectangle { bounds, filled }` for containers, content, occupied cells and bands.
- `Label { position, text }` for centered, screen-sized descriptions.

Roles distinguish Container, Content, Track, Flow, Spacing, Padding, Relation,
Placement, Rejected and Measurement. A native or alternative renderer owns their
presentation. Providers do not depend on brushes, windows, fonts or controls.

Each `LayoutHandle` has a stable ID, descriptive label, position, drag axis,
accepted value, range, direction and semantic `LayoutHandleAction`. A disabled
reason keeps protected features visible without making them interactive. Standard
actions cover panel properties, individual thickness sides and Grid tracks;
`Custom { key }` delegates planning to the registered provider.

`scene.placement` is authoritative after validation. A transaction planner may
refine its bounds or reject the original proposal. The native renderer uses that
final placement and suppresses stale Placement/Rejected primitives supplied by an
earlier scene. A zero-width or zero-height placement is an insertion caret with
screen-sized end caps.

## Built-in layout presentation

| Layout | On-canvas feedback |
| --- | --- |
| Canvas / generic Panel | Container/content bounds and spatial placement; moving objects also show bounded alignment lines, nearest sibling distances and measured size. |
| StackPanel / ReversibleStackPanel | Effective flow direction, ordered child slots, spacing bands/grips and insertion boundaries. Reversal follows the runtime property. |
| Grid | Measured row/column boundaries and labels, selected cell/placement region and track-size grips. Track resizing preserves star proportions; dragging Auto deliberately materializes a pixel length. |
| WrapPanel | Measured lines, item flow, item/line spacing and insertion within the correct line. |
| UniformGrid | Actual row/column cells, spacing and reserved leading cells from `FirstColumn`. |
| DockPanel | Consumed dock regions, remaining fill area and the proposed dock side. |
| FlexPanel | Measured line/direction presentation, wrapping and row/column spacing. Source placement follows the native adapter's ordering policy. |
| RelativePanel | Named relation lines between participating children and container bounds. |
| Border / Decorator / content holders | Actual child/content area, supported padding edges and explicit rejection of an occupied single-content slot. |
| Framework-generated panels | Inspection hints without writable placement/resize handles; edit the item source/template instead. |

Effective runtime values drive these helpers, including properties inherited from
styles or bindings. A visible effective value does not authorize replacing its
authored expression. Bound or object-valued properties remain protected by the
same source planners used by property and layout editing.

## Native input and transaction flow

The **Layout helpers** canvas command controls only presentation. Hiding helpers
cancels pending helper/drop drafts. Select/Interact modes continue to distinguish
authoring from interaction with the preview application.

Dragging an enabled grip stages its value. Trailing padding edges invert the
pointer direction; minimum and maximum values are enforced before preview. On
release, the session applies the exact validated edit list as one transaction.
`F8` cycles enabled helper grips (`Shift+F8` reverses). Arrow keys edit the focused
grip; Shift uses larger steps and Ctrl uses fractional steps. Escape leaves helper
focus or cancels a captured draft. Alt retains ordinary free manipulation instead
of structural placement.

Each `LayoutEditPreview` privately retains its edits and a guard covering document
URI, revision, AST identity, selection, preview generation and provider-registry
revision. Commit rechecks permissions and that guard. A source edit, replacement
document, different selection or replacement provider cannot turn a stale
preview into an edit against a new target.

Within flow panels, placement is an ordered source move. Crossing a panel boundary
uses the existing content-ownership and namespace-preserving reparent planner.
Grid and Dock placement add the relevant attached values atomically. A Canvas
reparent preserves measured affine geometry and adds the current drag displacement
in the new Canvas coordinate space. The prospective marker and accepted edit share
the same placement provider.

### Actual control ghosts

`DragVisualGhost` optionally captures a native control bitmap once after the drag
threshold is crossed. It paints a translucent copy with the current artboard
adjustment while the accepted native control and source remain unchanged. A bitmap
is bounded to 2048 pixels on its longest edge; the surface also bounds aggregate
capture work for multiselection. Release and cancellation drop the cache.

Ancestor affine transforms are supported. A selected control with its own render
transform or mirror currently uses a geometric ghost: the native bitmap renderer
applies that transform inside its local clipping rectangle, so reusing the image
with the full visual transform would clip or apply the transform twice. Selection
geometry, helpers, placement and accepted edits still use the complete affine
transform in that case.

## Extending a user panel

Register a native control through the existing preview-host metadata/factory API,
then register its portable `LayoutAdapter` with
`session.layout_tools_mut().register(Box::new(provider))`.

Use a namespace-stable identity such as `{urn:acme:controls}RhythmPanel` from
`panel_type()`. `LayoutRegistry::adapter_for(document, node)` resolves the expanded
namespace identity before legacy lexical names. Aliases such as `a:RhythmPanel`,
`b:RhythmPanel`, or a default namespace resolve the same provider. An unregistered
application type called `Grid` or `StackPanel` does not inherit a framework
provider merely because its local name matches.

Implement the hooks according to the panel's actual runtime behavior:

1. `capabilities` states whether source children may move, resize, reorder or be
   inserted. Mark framework-generated content as generated.
2. `plan` remains the existing source planner for standard layout actions.
3. `design_helpers` starts from `LayoutHelperScene::generic(context, request)` and
   adds measured primitives and stable semantic handles. Read effective values
   through `context.property`; do not guess positions from source text.
4. `placement` returns the actual content container, original-snapshot insertion
   index, attached values and preview bounds. Return a rejection when the panel
   cannot accept the proposed content.
5. `plan_helper` handles application-specific intents and returns source edits.
   Validate bindings, ownership, finite values and the panel's property ranges.
   Let the registry/session validate and commit the resulting transaction.

Use the portable `plan_layout_attributes` helper when applying placement
attributes. It retains an existing framework namespace prefix and qualifies
framework attached members under an application-owned default namespace. It also
rejects bindings, property-element values and duplicate aliases instead of
silently replacing or shadowing their authored representation.

The compiled custom-provider regression in
[`layout_helpers_tests.rs`](../../src/FerroUI.Designer.Tools/layout_helpers_tests.rs)
demonstrates canonical namespace registration, custom scene geometry, a custom
handle intent, source edits and alias-preserving resolution. Existing custom
adapters can adopt helper/placement features independently.

### Reusing the native view

An embeddable host can obtain `selection_layout_helpers()` or
`layout_helper_snapshot(panel, pointer, dragged_nodes)` from the session and call
`render_layout_helpers`. `render_layout_helpers_with_state` additionally accepts
a viewport and hovered/active handle indices. `hit_test_layout_handle` uses the
same transformed, screen-sized hit targets as the renderer.

`LayoutHelperSnapshot::new(scene, to_artboard)` creates a render-only snapshot for
an independent host. It cannot be submitted as a guarded authoring snapshot to a
session. `preview_layout_move`, `preview_layout_insert` and
`preview_layout_handle` return session-owned proposals; `commit_layout_preview`
is the mutation boundary. Cached move/insert variants avoid repeating validation
while the pointer remains over the same semantic placement.

`render_manipulation_metrics` draws finite alignment lines, nearest overlapping
neighbor gaps, equal-gap labels and a measured bounds badge. It never changes the
snap result or layout. `DragVisualGhost` can be used independently by a host that
owns capture lifetime and its aggregate bitmap budget.

## Verification

### Native playground

Open the checked-in [LayoutHelpers.xaml](samples/LayoutHelpers.xaml) sample in the
native designer to try the same layout scenarios used by the render fixture:

```sh
cargo run -p designer-studio -- --file docs/designer/samples/LayoutHelpers.xaml
```

The sample has named Grid, Stack, Wrap, Dock, UniformGrid and Flex layouts, a
movable component, and available/occupied content holders. Hide the timeline and
fit the artboard to make all containers visible. Select a layout to expose its
own handles; drag the coral component between containers to inspect placement
and the native visual ghost.

The native workspace test writes eight captures through actual DesignSurface
pointer events. It checks Grid cell attributes, a staged Stack gap, Wrap
reparenting and cached control pixels, Dock edge attachment, UniformGrid/Flex
insertion, and accepted/rejected content targeting. Source remains unchanged
while each draft is visible; accepted releases restore with one undo.

```sh
FERROUI_DESIGNER_RENDER_OUTPUT=/tmp/designer-renders \
  cargo test -p ferroui-designer --features mcp,render-smoke --lib \
  native_workspace_renders_layout_helper_interactions -- --test-threads=1 --nocapture
```

### Coverage

Portable tests cover each layout family, placement/planner agreement, effective
properties, protected expressions, custom provider identity and scene validation.
Native bridge and pointer tests cover accepted/rejected reparenting, helper
transactions, stale previews, cancellation and runtime layout. Renderer tests
cover affine quadrilaterals, screen-space hit targets, finite alignment guides,
nearest-neighbor/equal-gap measurements, authoritative placement and actual cached
control rendering without runtime mutation.

The `render-smoke` fixture `designer-layout-overlay-affine.png` is real native
renderer evidence for extension-provided geometry, transformed panels, insertion
and rejection feedback, and a cached native control ghost. Workspace captures
exercise the same renderer through actual editor gestures.
