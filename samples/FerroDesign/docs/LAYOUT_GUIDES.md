# Layout guides

FerroDesign supports ordered stacks of square, column and row guides on frames,
components and instances. Each guide has its own geometry, color and local
visibility. Native FerroUI controls author both individual frame stacks and
shared grid styles. The editor draws the guides through public FerroUI drawing
and clipping APIs; they are stored in the document but excluded from artwork
exports and prototype presentations.

Guides also provide pointer snap references and stretch-section references for
child constraints when a parent is explicitly resized. Native variable pickers
bind guide fields through the existing typed variable system. Guides remain
separate from the auto-layout grid solver. The behavior and remaining gaps below
are the FerroDesign contract; these features do not establish complete Figma
feature or visual parity.

## Source map

| Source | Responsibility |
| --- | --- |
| [model.rs](../crates/ferrodesign-core/src/model.rs) | Serializable node fields, `LayoutGrid`, style variants and document validation |
| [layout_guides.rs](../crates/ferrodesign-core/src/layout_guides.rs) | Axis geometry, constant-size band resolution, stack accessors, typed edits and scoped history |
| [layout_guides/constraints.rs](../crates/ferrodesign-core/src/layout_guides/constraints.rs) | Nearest stretch-section constraints and transformed child resizing |
| [layout_guides/bindings.rs](../crates/ferrodesign-core/src/layout_guides/bindings.rs) | Typed guide variable targets, range validation and binding remapping |
| [layout_guides/snapping.rs](../crates/ferrodesign-core/src/layout_guides/snapping.rs) | Arithmetic snap candidates, transformed distance and constrained movement |
| [style_edit.rs](../crates/ferrodesign-core/src/style_edit.rs) | Shared definitions, propagation, visibility normalization and legacy single-grid commands |
| [components.rs](../crates/ferrodesign-core/src/components.rs) | Guide-stack and visibility overrides during component refresh |
| [shell/layout_grids.rs](../crates/ferrodesign-app/src/shell/layout_grids.rs) | Native frame/style controls, draft validation and indexed commands |
| [shell/layout_grids/variables.rs](../crates/ferrodesign-app/src/shell/layout_grids/variables.rs) | Native property/variable pickers and inline binding validation |
| [shell/styles.rs](../crates/ferrodesign-app/src/shell/styles.rs) | Local-style lifecycle and access to guide settings |
| [canvas/grids.rs](../crates/ferrodesign-app/src/canvas/grids.rs) | Retained guide candidates, transformed clipping and bounded overlay drawing |
| [canvas/grids/snapping.rs](../crates/ferrodesign-app/src/canvas/grids/snapping.rs) | Retained spatial index, eligible frame selection, clipping and native snap adorners |
| [canvas.rs](../crates/ferrodesign-app/src/canvas.rs) | Integration with movement, shape creation and resize gestures |
| [Editor.xaml](../crates/ferrodesign-app/ui/Editor.xaml) | Compiled editor shell and inspector entry point |

## Portable document and backward compatibility

The original `Node::layout_grid: Option<LayoutGrid>` remains the first guide.
`Node::layout_grid_overlays: Vec<LayoutGrid>` stores the additional guides, in
order. `Node::layout_guides()` presents both fields as one sequence;
`layout_guide(index)`, `layout_guide_count()` and `set_layout_guides()` centralize
access to that representation. Removing the first guide promotes the next guide
into the primary field. The stored entries have positional indices rather than
independent guide IDs.

A legacy square guide remains valid and retains its original serialized shape:

```json
{
  "layout_grid": {
    "size": 8.0,
    "color": { "r": 255, "g": 0, "b": 0, "a": 26 },
    "visible": true
  }
}
```

`pattern` defaults to square and is omitted when serializing a square guide.
An absent primary guide and an empty overlay list are also omitted. Existing
square-only documents therefore need no migration or extra fields. A stack
extends the node with ordinary JSON data:

```json
{
  "layout_grid": {
    "size": 8.0,
    "color": { "r": 255, "g": 0, "b": 0, "a": 26 },
    "visible": false
  },
  "layout_grid_overlays": [
    {
      "size": 20.0,
      "color": { "r": 34, "g": 197, "b": 94, "a": 128 },
      "visible": true,
      "pattern": {
        "type": "columns",
        "alignment": "stretch",
        "count": 3,
        "gutter": 8.0,
        "offset": 12.0
      }
    },
    {
      "size": 16.0,
      "color": { "r": 37, "g": 99, "b": 235, "a": 128 },
      "visible": true,
      "pattern": {
        "type": "rows",
        "alignment": "start",
        "count": null,
        "gutter": 8.0,
        "offset": 10.0
      }
    }
  ]
}
```

These examples are node fragments, not complete documents. `count: null` is Auto;
the portable format does not encode a non-JSON infinity. A missing axis field
uses `LayoutGridAxis::default()`: stretch alignment, count 12, gutter 20 and
offset 0. A new guide defaults to size 8, red with alpha 26/255 and visible.
Rows and columns share the same axis structure.

Document validation rejects guides on unsupported node kinds, overlays without
a primary guide, oversized stacks and invalid numeric values. An empty stack
contains neither a primary guide nor overlays; it can still be linked to an
empty shared definition. The new reader accepts the legacy representation, but
older binaries are not guaranteed to understand the new patterns or style
variant. This is FerroDesign JSON, not Figma file interchange.

Shared styles retain the legacy `StyleValue::Grid { size, color }` representation
for a single square guide. `StyleValue::GridStack { grids }` serializes with
`"type": "grid_stack"` and an ordered array of complete guide values. Capturing
or editing exactly one square guide uses the legacy variant; rows, columns and
multiple guides use `GridStack`. A shared definition can change representation
without changing its style ID or the linked nodes' IDs.

## Geometry contract

Square guides draw horizontal and vertical lines at multiples of `size`,
anchored at the frame's local origin. Columns draw filled vertical bands across
the frame height. Rows draw filled horizontal bands across the frame width.
Guide colors include alpha, and later entries paint above earlier entries
within a frame.

For rows and columns, let `E` be the frame's local height or width respectively,
`s` the stored size, `n` the resolved count, `g` the gutter and `o` the offset.
The following table describes the implementation in `LayoutGrid::bands`:

| Alignment | Column / row label | Band size | First band start |
| --- | --- | --- | --- |
| `start` | Left / Top | `s` | `o` |
| `center` | Center / Center | `s` | `(E - T) / 2` |
| `end` | Right / Bottom | `s` | `E - o - T` |
| `stretch` | Stretch / Stretch | `(E - 2o - (n - 1)g) / n` | `o` |

For fixed guides, `T = ns + (n - 1)g` is the total band-and-gutter extent. The
distance between consecutive band starts is the resolved band size plus `g`.
Center alignment ignores the stored offset; changing alignment can make that
retained value effective again. Stretch treats offset as an equal margin on
both sides. Resizing a stretch frame changes band sizes while preserving the
authored count, margins and gutter. Fixed bands can extend beyond the frame and
are clipped during drawing.

Auto count uses an available extent `A`: `E` for center, `E - 2o` for stretch,
and `E - o` for start/end. The count is:

```text
n = floor((A + g) / (s + g))
```

For Auto stretch, `s` is the preferred size used to choose the count; the actual
band size then follows the stretch formula. For a fixed stretch count, `s` does
not determine band width or height. The native editor labels this field
accordingly. This automatic-count formula is the explicit FerroDesign contract;
the primary references do not establish identical behavior in every Figma edge
case.

Resolution returns no bands when there is no positive representable section
space, the derived count is outside its supported range, or arithmetic produces
nonfinite or indistinguishable positions. An undersized stretch frame keeps its
authored settings and begins drawing again when enough space is available.
No guide edit resizes the frame or its children.

## Native authoring and history

Open **Layout grid** in the inspector, **Layout grid guides…** in quick actions,
or **Selected frame grid settings…** in Local styles. The guide dialog is made
of native FerroUI `StackPanel`, `ComboBox`, `TextBox`, `CheckBox` and button
controls hosted by the compiled XAML shell.

Each frame guide card offers its pattern, relevant axis controls, size, one RGBA
hex color field, local visibility, **Bind variable…**, move up/down, duplicate
and remove actions. Axis controls
include the alignment, a positive count or `Auto`, gutter, and margin/offset.
Center alignment hides the unused offset field. Add square, columns or rows
appends an independent guide. **Add missing grids** preserves the legacy
operation: it adds a default square only to selected eligible frames that lack
a primary guide. **Remove all guides** clears the whole stack and its style
binding. With one selected frame whose linked definition is already empty,
the button is disabled; use **Local styles → Detach selected** to remove that
binding. The core `LayoutGuideEdit::Clear` API also clears empty-stack bindings.

Fields commit on Enter or focus loss and use the shared inspector draft
lifecycle. Invalid drafts keep their inline validation and prevent closing or
executing another action that requires a successful flush. Escape restores the
committed value. An unfinished IME composition is not committed as confirmed
guide data. Structural choices flush pending fields before applying their edit
and rebuilding the controls.

The pattern selector uses `LayoutGuideProperty::Kind`, which reads the current
axis when applying the command. Switching columns to rows or back preserves
recent numeric edits and each target's own axis values, even when the combo was
created before those edits. A full `Pattern` property remains available for
explicit replacement by callers. Switching to square removes axis data;
switching from square to a row or column initializes the default axis.

`Editor::edit_layout_guides` validates the complete batch before mutation,
deduplicates target IDs and skips effectively locked or incompatible nodes.
Missing node IDs and invalid values reject the batch. Indexed edits leave
missing rows unchanged in a mixed selection. The dialog starts with the first
eligible selected frame and indicates differing stacks; the displayed index
maps to the same index on other targets. Changing the selection makes the old
dialog target stale and requires reopening it.

An accepted change creates one scoped history transaction for all changed
targets. Unchanged operations do not add history. Rejected batches preserve the
document and redo state. The legacy `Editor::edit_layout_grids` API remains for
first-guide size/color/visibility and add-missing behavior; its remove operation
clears the complete stack.

### Control names for browser automation

Names retain the existing single-square controls. Guide index 0 has no suffix;
later indices append their zero-based index. For example, the second guide uses
`LayoutGridCount1`. Shared-style controls substitute the `StyleGrid` prefix.

| Purpose | Frame control name |
| --- | --- |
| Pattern and alignment | `LayoutGridPattern`, `LayoutGridAlignment` |
| Axis fields | `LayoutGridCount`, `LayoutGridGutter`, `LayoutGridOffset` |
| Common fields | `LayoutGridSize`, `LayoutGridColor`, `LayoutGridVisible` |
| Per-guide actions | `LayoutGridUp`, `LayoutGridDown`, `LayoutGridDuplicate`, `LayoutGridRemoveGuide`, `LayoutGridVariables` |
| Append guides | `LayoutGridAddSquare`, `LayoutGridAddColumns`, `LayoutGridAddRows` |
| Legacy add-missing / clear-stack actions | `LayoutGridAdd`, `LayoutGridRemove` |
| Dialog actions | `LayoutGridCreateStyle`, `LayoutGridStyles`, `LayoutGridDone` |

Validation labels append `Error` to the corresponding field name. Style cards
omit the visibility toggle because visibility belongs to each frame. Their
outer dialog uses the existing style lifecycle controls, including `StyleDone`.
Shared-style cards do not expose the per-frame variable picker.

The binding dialog uses `GuideVariableProperty`, `GuideVariableValue`,
`GuideVariableError`, `GuideVariableApply`, `GuideVariableLibrary` and
`GuideVariableBack`. These names are unsuffixed because the dialog edits one
selected guide index at a time.

## Typed variable bindings

Open **Bind variable…** on a frame guide card, choose a property and a typed
variable, then choose **Apply**. The dialog filters properties for the current
pattern and variables by kind. It reports the number of eligible, unlocked
selected frames and skips targets without a compatible guide at that index.
The existing variable library manages aliases, mode values and the active
mode. Validation errors remain inline without changing the document or history.

Bindings use the existing node `variable_bindings` map. The canonical key is
`layout_grid.<zero-based index>.<field>`, for example:

```json
{
  "variable_bindings": {
    "layout_grid.0.count": 100,
    "layout_grid.0.color": 102,
    "layout_grid.1.size": 101
  }
}
```

| Field | Variable kind | Accepted resolved value |
| --- | --- | --- |
| `size` | Number | Positive finite number |
| `count` | Number | Integer from 1 to 1,000,000; rows and columns only |
| `gutter`, `offset` | Number | Nonnegative finite number; rows and columns only |
| `color` | Color | Typed RGBA color |
| `visible` | Boolean | True or false |

A guide binding must reference an existing supported guide. Every available
mode is checked, including alias resolution, so an inactive mode cannot hide an
invalid count or spacing. Invalid targets, types and ranges reject the atomic
variable edit. **Unbound** removes a binding and retains its current literal
value. Removing a variable likewise preserves the last resolved guide values.
Auto count is authored as a literal; count variables always resolve to explicit
positive integers.

Committing a literal field removes that field's binding even when its numeric
or color value is unchanged. Explicit Enter confirms that choice. Merely
blurring an untouched field retains its binding. Reordering moves bindings with
their guide, duplicating copies them, removing a guide drops its bindings and
renumbers later indices, and clearing the stack removes every guide binding.
Columns-to-rows switches retain compatible bindings; switching to square drops
count, gutter and offset bindings. The legacy primary-grid API follows the same
literal-override and removal rules.

Binding geometry or color detaches a linked grid style. Binding visibility
keeps the style and establishes a local visibility override. Explicitly applying
a grid style clears the frame's guide bindings, including when the resulting
literal guide values are equal. Shared definitions store literal values;
propagation prunes bindings whose target index or pattern no longer exists.

## Guide-relative constraints

Use the existing horizontal and vertical constraint controls on a child. When
its parent is explicitly resized, the first usable **stretch columns** guide in
stack order supplies horizontal reference sections, and the first usable
**stretch rows** guide supplies vertical sections. Individual guide visibility
and alpha do not disable those references. Square guides, fixed alignments and
unusable or collapsed stretch guides fall back to the existing frame-relative
constraint behavior on that axis.

Section selection uses arithmetic nearest-band lookup. Nearest-center ties
choose the later positive index. The two axes resolve independently:

| Constraint | Section-relative resize behavior |
| --- | --- |
| Start | Preserve the child's start offset from the nearest section's start |
| End | Preserve its end offset from the nearest section's end |
| Center | Preserve its center offset from the nearest section's center |
| Stretch | Preserve start and end offsets against the nearest sections at the two child edges |
| Scale | Scale position and size within the span of the nearest sections at the two child edges |

The editor infers these sections at resize time; it does not serialize a
persistent section assignment. If Auto count changes, old section indices are
clamped to the new count. Editing a guide field alone does not reposition
children. These stack-precedence, tie and Auto-count rules are explicit
FerroDesign behavior rather than a claim of exact Figma behavior in every case.

Normal auto-layout flow children remain under the layout solver. Absolute
children use constraints. A locked child still responds to its parent's resize,
as with existing frame constraints; direct edits continue to respect effective
locks. Axis-aligned children use the existing geometry resize path. Rotated or
sheared children preserve their source geometry and receive an affine mapping
from the old parent-local visual bounds to the new bounds. A parent resize and
all resulting child changes remain one history transaction.

## Pointer snapping

The existing snapping setting enables guide snapping for free movement, shape
creation and resize handles. Movement considers the nine points of the selected
visual bounds. Shape placement uses its start and end points; resizing uses the
actual transformed handle while preserving the opposite anchor. Pixel rounding
and aspect constraints happen before guide correction, so a fractional guide
position remains exact. Pen placement also consults guides. Direct vector
control-point edits, continuous pencil samples and the dedicated auto-layout
reorder gesture keep their separate input paths.

Square line positions and both edges of row/column bands are snap targets.
Band centers and gutter centers are not targets. Individual hidden or
transparent guides remain usable. Hidden owners, other pages, selected owners
and their descendants are excluded. A locked frame can be a passive snap
reference. An eligible ancestor frame is preferred over unrelated nearby
frames.

The tolerance is five screen pixels. Inverse-transform row normals measure
perpendicular world distance correctly through rotation, reflection and shear.
Shift-constrained movement keeps its world axis; aspect-constrained drawing and
resizing keep their direction ray. If a two-axis correction would become a large
jump under near-singular shear, the solver uses the closest valid single-axis
correction. Guide snapping takes precedence over ordinary object snapping;
object snapping remains the fallback when no guide candidate is accepted.

Candidates must lie inside the guide owner's actual boundary and clipping
ancestors. Public FerroUI geometry clips include smoothed corners. At most two
pink native snap lines indicate the accepted references; release and
cancellation clear them. These adorners belong to the editor overlay, so they
are absent from artwork exports and prototype presentations. Each completed
gesture uses the existing single undo transaction.

## Shared styles and component instances

The `grid_style` link owns the complete stack. Editing a shared definition
propagates its guide geometry, colors and order to linked compatible nodes
across pages, including locked linked nodes. Explicit application and detachment
respect effective locks. Deleting or detaching a style keeps each node's
effective guide values in the document.

Visibility is local. Style comparison normalizes guide visibility to true, and
capturing a style from a frame normalizes it the same way. Propagation preserves
the receiving frame's existing visibility **by index**. A newly introduced
index takes the definition's stored visibility value. There are no guide IDs
for matching visibility across a style reorder: after that reorder, the local
visibility at index 1 still belongs to index 1. A local frame reorder moves the
complete guide value, including visibility.

Direct changes to guide geometry, color or stack structure detach the frame's
grid-style link. Changing only visibility retains the link; unchanged values
also retain it. Applying a legacy square style replaces the complete stack
with one square, preserving the receiving primary guide's visibility.

Component refresh uses the existing override names:

- `layout_grid` preserves the instance's primary guide, overlays, style link and
  compatible guide bindings as one independent stack.
- `layout_grid_visible` preserves visibility and its binding at each matching
  index while the refreshed component supplies the other guide properties.

New component slots retain their refreshed defaults when no previous visibility
exists at that index. Removing a master slot removes the corresponding slot
unless the instance owns a complete stack override. Flattening to a vector
clears the guide fields, guide variable bindings and grid-style binding because
vectors cannot own frame guides; see [flatten.rs](../crates/ferrodesign-core/src/flatten.rs).

## Rendering, clipping and resource limits

The guide pass runs outside the artwork renderer. Square lines use
`DrawingContext::draw_line`; row/column bands use `draw_rectangle` and an
`ImmutableSolidColorBrush` constructed from explicit ARGB channels. Public
transform, rounded-clip and geometry-clip scopes apply the frame's local
transform chain, the frame boundary and any clipping ancestors. Smoothed
corners use the shared renderer corner geometry. Rotation, reflection and shear do
not require flattening the guides or issuing backend-specific commands. Square
line thickness and density use the projected perpendicular axis scale.

Only effectively visible, active-page guide owners intersecting the viewport
are painted. Individual hidden or fully transparent guides are skipped. The
frame's own rounded boundary clips its guides even when its children are not
clipped. PNG/SVG/XAML artwork export and prototype rendering do not invoke this
overlay pass; native JSON retains the authored guide data.

`LayoutGridOverlay` retains a set of nodes with guide stacks and consumes the
editor render-change feed. Camera-only frames reuse that set. Scoped edits
update changed node/ancestor membership; a full invalidation rebuilds it.
Painting still visits retained guide candidates, so this is not a claim of
constant work for every document size. `LayoutGridBands::visible_range` finds
the intersecting band indices arithmetically without allocating or iterating
over the full authored count.

| Limit | Behavior |
| --- | --- |
| 64 guides per node or shared definition | The primary guide counts toward this limit; oversized edits/imports are rejected |
| Explicit count from 1 to 1,000,000 | Zero and larger explicit values are rejected; `null` remains Auto |
| Derived Auto count up to `u32::MAX` | A larger, nonfinite or sub-one result produces no bands; it is not silently clamped to the explicit-count limit |
| Positive finite size; nonnegative finite gutter and offset | Invalid values are rejected before document mutation |
| Minimum 4 screen pixels between repeated positions | Denser square axes or band sequences are suppressed without changing their authored spacing |
| At most 4,096 visible primitives per axis | An over-budget axis is suppressed; square horizontal and vertical axes are checked separately |
| At most 32,768 line/band primitives per overlay render call | A shared budget covers all visible owners and guide stacks; reaching it stops further overlay drawing |
| Nine points and 64 guides per snap solver call | Nearest positions are computed arithmetically, without expanding a guide's section count |
| At most 256 owners, 64 spatial cells and 4,096 cell entries per pointer query | Work is bounded after preferred ancestor candidates; excess unrelated candidates are not examined |

The final budget is named `MAX_FRAME_LINES` in the renderer but applies to the
whole rendered overlay frame, not separately to each document frame. Density
checks use square `size` or band `step`, which includes the gutter. Suppression
affects display only; saved values, style ownership and history remain intact.
Snapping also suppresses repeated positions denser than four screen pixels.

The snap index uses 512-unit spatial cells and stores an owner in at most 64
cells. Larger owners live in a separate bounded-query set. Scoped changes update
that index, while camera-only frames reuse it. Full invalidation can still scan
the document to rebuild the index; preparing selected-root exclusions is
proportional to the selection size. These bounds do not imply constant total
cost for arbitrary documents, selections or document mutations.

## Regression coverage

- [Core guide tests](../crates/ferrodesign-core/src/layout_guides_tests.rs) cover
  fixed/stretch/Auto geometry, constant-size visible ranges, legacy and stack
  JSON round trips, mixed selection, limits, atomic rejection, shared-style
  propagation/detachment, component overrides and exact undo/redo. The
  orientation regression edits current fields before switching kind and checks
  distinct settings on multiple targets.
- [Constraint tests](../crates/ferrodesign-core/src/layout_guides/constraints.rs),
  [binding tests](../crates/ferrodesign-core/src/layout_guides/bindings/tests.rs)
  and [snap solver tests](../crates/ferrodesign-core/src/layout_guides/snapping.rs)
  cover all five constraint modes, transformed children, auto-layout exclusions,
  hidden guides, atomic mode validation, exact history, style/component binding
  lifecycle, affine snap distances and bounded dense guides.
- [Snap-index tests](../crates/ferrodesign-app/src/canvas/grids/snapping.rs) cover
  retained membership, inherited transforms and visibility, self/descendant
  exclusions, clipping and bounded overlapping-owner queries.
- [Native overlay tests](../crates/ferrodesign-app/src/canvas/grids_tests.rs)
  cover actual pixels, stacked band order and gutters, stretch resizing,
  transformed rounded clipping, density suppression and retained candidates.
- [Browser guide tests](../tests/grid-styles-browser.mjs), invoked by
  [browser.mjs](../tests/browser.mjs), operate native controls for square and
  stacked guides, pattern changes after field edits, shared-style lifecycle,
  local visibility and history. They compare rendered guide pixels and verify
  that downloaded white artwork contains no guide ink.
- [Guide binding and interaction browser tests](../tests/guide-bindings-browser.mjs)
  author numeric/color bindings, mode changes, equal-value detachment and
  structural remapping through native controls. They also resize constrained
  children and perform real pointer move, resize and draw gestures, checking
  fractional snap positions and exact undo/redo.

From `samples/FerroDesign`, focused native commands are:

```sh
cargo test --locked -p ferrodesign-core layout_guides
cargo test --locked -p ferrodesign-app --lib canvas::grids -- --test-threads=1
```

Browser checks require the actual WebAssembly build and run through the normal
browser suite. Execution results and source revisions belong in
[VALIDATION.md](VALIDATION.md); these coverage descriptions do not assert a
passing browser gate or a pixel-equivalence comparison against Figma.

## Remaining guide-specific work

- **Additional snap input paths:** direct vector control-point edits,
  continuous pencil samples and auto-layout reordering do not use the guide
  solver. Band centers and gutter centers are not snap targets.
- **Persistent constraint anchors:** there is no per-child guide/section
  assignment picker. Sections are inferred from the first usable stretch guide
  at explicit parent resize. Assigning parent dimensions through variables uses
  the existing variable-resolution pipeline rather than simulating that resize
  command. Guide edits alone do not reflow children.
- **Additional variable authoring:** shared guide styles store literal values,
  not shared guide-binding metadata. Pattern, alignment and Auto count remain
  literal choices. Mode activation uses the existing document-wide mode model.
- **Full visual and interaction parity:** the native control arrangement,
  defaults, dense-grid display rules and Auto count edge cases are this
  implementation's contract. Complete equivalence with Figma has not been
  established by these features or tests.

## Primary references

[Figma's LayoutGrid plugin API](https://developers.figma.com/docs/plugins/api/LayoutGrid/)
documents grid/row/column patterns, MIN/CENTER/MAX/STRETCH alignment, Auto counts,
section size and offset semantics. FerroDesign maps MIN/MAX to start/end and
represents Auto with JSON null.

[Figma's Create layout guides guide](https://help.figma.com/hc/en-us/articles/360040450513-Create-layout-guides)
describes multiple guides, fixed and stretch settings, margins, local
visibility, reusable styles and the separate guide/constraint workflow. These
references inform the terminology and geometry; the source and formulas above
define the implemented behavior and its limits.

[Figma's Combine layout guides and constraints guide](https://help.figma.com/hc/en-us/articles/360039957934-Combine-layout-guides-and-constraints)
describes nearest-section constraints for stretch guides, frame-relative
constraints for fixed guides and continued behavior when guides are hidden.
[Apply constraints to define how layers resize](https://help.figma.com/hc/en-us/articles/360039957734-Apply-constraints-to-define-how-layers-resize)
defines the five constraint modes used by the native inspector.
