# Native designer controls and interaction guide

The designer workspace composes ordinary FerroUI controls with reusable `AssetToolbox`,
`TimelineLane`, `PropertyNumberScrubber`, `PropertyComponentsEditor` and `GridTrackEditor`
components. Hosts can use these without loading the workspace shell. They do not own a
document, start a window, or create an undo
stack; their callbacks let the host apply changes through its existing transaction boundary.

## Compact property inspector

Properties are grouped into collapsible categories. Common geometry stays in Layout; minimum
and maximum dimensions are available under Advanced. Search matches every term and supports
`@modified` and its alias `@local` for authored overrides, including expressions. A value-origin dot and
native tooltip distinguish authored values, expressions and runtime effective values. Changing
the filter does not author source or discard the document selection.

Numeric property names are `PropertyNumberScrubber` controls. Drag a label horizontally to
adjust its draft, then release to commit one transaction. Shift multiplies the configured step
by ten; Ctrl divides it by ten. Arrow keys use the same policy. Escape, lost pointer capture,
or reconfiguration cancels a drag. `PropertyNumberOptions` sets minimum, maximum, step and
integer rounding. Native numeric value fields remain directly editable; Enter commits text.

`PropertyComponentsEditor` presents Margin/Padding/BorderThickness as L/T/R/B and CornerRadius as
TL/TR/BR/BL. The link button switches between a common value and independent components.
Enter validates and commits the four values together; Escape restores the accepted values.
A mixed selection starts with empty components rather than inventing zeros. In unlinked mode,
all four values must be explicit before committing a mixed edit; linked mode applies one explicit
value to all four. Padding, border thickness and corner radii enforce nonnegative values;
margins may be negative. Alignment editors provide four directly accessible choices.

Custom hosts construct a scrubber with `PropertyNumberScrubber::new()`, call `configure`
with an accepted label/value/options, and handle `on_commit`. Construct a compound editor
with `PropertyComponentsEditor::new(PropertyComponentsKind::Thickness, false)`, call
`configure(Some([left, top, right, bottom]))`, then connect `on_commit`. Configuration updates
the view without generating an authoring callback. `control()` embeds a compound editor in
another view; a scrubber is itself a native control. Existing `PropertyEditorFactory` extension
points take precedence over built-ins and are used by the contextual panel editors too.

### Dropping values and references

Drop text or a `property_value_data_format()` string onto a property value to validate and
apply it to the selected authored objects. A valid drop copies one value; it never removes the
source. Multi-selection is one atomic transaction. Invalid values and read-only, runtime-only
or stale destinations reject the drop before modifying source.

In Layers, select the destination object first, then drag a keyed resource to the appropriate
property to create `{StaticResource Key}`. Shift-drag creates `{DynamicResource Key}`.
Alt-drag a named element to create `{Binding ElementName=Name}`. The inspector destination
remains selected while the reference is dragged; an ordinary resource click still selects the
resource for inspection. Internal references retain their document identity and are checked
against destination resource/name scopes, including shadowing and merged dictionary order.
Reference drops require the referenced object to be resolvable from every selected target.

The native TreeDataGrid `RowDragStarted` arguments expose `data_transfer()` and
`key_modifiers()` so other hosts can attach typed payloads while retaining normal row drag
behavior. Designer source hierarchy moves continue to use their own guarded move transaction.

## Contextual panel layout

The property dock's Layout section shows the selected panel, or the common parent panel of
selected children. **Select parent** moves to that panel. Stack, Wrap, Dock, UniformGrid and
Flex panels expose their common settings through the same registered property factories and
layout adapters. Custom panel adapters can expose their own settings through this section.

Grid rows and columns each use `GridTrackEditor`: choose a track, edit its numeric length and
`*`, `px` or `Auto` unit, or insert/remove a track. Inserting adjusts child indices and expands
spans crossing the insertion. Removing shifts indices, shrinks spans and keeps cells inside
the remaining tracks. One transaction updates definitions and all affected children; one undo
restores the original source. An implicit axis becomes explicit only when edited, and at least
one track remains. Authored definition property elements retain their other attributes,
namespaces, comments, min/max constraints and shared-size groups.

`GridTrackEditor::new(axis)` creates the standalone view. `configure(values, selected, writable)`
updates it without authoring; `on_selected` reports navigation and `on_change` receives a
`GridTrackChange` whose callback returns a validation result. Invalid typed lengths remain
available for correction. Hosts can inspect the native selector and value controls using
`track_selector`, `length_editor` and `unit_selector`.

The workspace calls `DesignerSession::edit_grid_tracks`. The portable
`plan_grid_track_change` and `plan_grid_track_change_with_placements` functions are available
to other hosts in `ferroui-designer-tools`. The native path supplies effective child placements,
so cells inherited from styles are preserved by materializing only the changed coordinates.
Bound cell coordinates/spans, object-valued positions, and definition collections supplied
entirely by a style or factory are rejected rather than flattened. Structural Grid edits remain
ordinary source edits even when animation recording is enabled.

## Hierarchy and artboard

Source hierarchy drags preserve source order and selected subtrees. Hovering over a compatible
collapsed container expands it; native TreeDataGrid scrolling and drop adorners stay active.
Before/after/inside drops validate content ownership, cycles and the captured source identity.
Canvas reparenting preserves measured world geometry across affine parent transforms.

| Focus / gesture | Behavior |
| --- | --- |
| Layers: Alt + Up / Down | Move selected siblings earlier / later |
| Layers: Alt + Home / End | Move selected siblings first / last |
| Layers: Alt + Right / Left | Nest in the previous compatible sibling / move out one level |
| Artboard: arrow keys | Move selected controls by one layout unit |
| Artboard: Shift + arrows / Ctrl + arrows | Move by ten units / one tenth of a unit |
| Artboard: Shift while dragging | Lock movement to the dominant axis |
| Artboard: marquee left to right | Select fully enclosed objects |
| Artboard: marquee right to left | Select intersecting objects |
| Artboard: Shift / Ctrl / Alt marquee | Add / toggle / subtract objects |
| Escape or lost capture during a canvas gesture | Cancel and restore the original selection/geometry |

The Arrange menu also exposes source-order and nesting commands alongside alignment and
distribution. `OutlineCommand`, `DesignerWorkspace::edit_outline` and the corresponding
session methods provide the same operations for embedding hosts. Multi-object structural
commands are atomic; an incompatible member prevents a partial move.

## Project assets

`DesignerWorkspace::register_asset` adds a project control to the searchable toolbox, or
replaces the existing entry with the same stable ID. Categories and the visible result count
update immediately while preserving the current category filter.

```rust
use ferroui_designer::{AssetIcon, DesignerAsset};

workspace.register_asset(DesignerAsset {
    id: "Project.RevenueCard".into(),
    title: "Revenue card".into(),
    category: "Project".into(),
    description: "A reusable revenue summary".into(),
    source: r##"<Border Width="240" Height="120" CornerRadius="8" Background="#FFE0E7FF">
        <TextBlock Text="Revenue" Margin="16" />
    </Border>"##.into(),
    icon: AssetIcon::CustomPath(
        "M2 22V12H7V22Z M10 22V7H15V22Z M18 22V2H23V22Z".into(),
    ),
})?;
```

Icons use native path geometry in a 24 × 24 coordinate space. Registration rejects empty
IDs, titles, categories, or snippets and invalid vector geometry before modifying the
catalog. Snippets are interpreted in the destination document's namespace context; custom
controls must be registered with the runtime loader, and any additional XML namespaces must
be declared in the document or snippet. Insertion uses the existing layout tools and source
transaction pipeline.

The standard catalog includes 20 controls, panels, and shapes. Search matches every
whitespace-separated term, ignoring case, across the title, ID, category, and description.
Native buttons supply keyboard focus, theme states, tooltips, and automation names. Dragging
starts a placement gesture; keyboard activation arms placement on the artboard.

For a separate host, construct `AssetToolbox::new(default_designer_assets())`, embed its
`control()`, and connect `on_activate` and `on_begin_drag`. The public `set_query`,
`set_category`, `visible_asset_ids`, `assets`, and `register` methods also support a host's
own catalog UI or integration tests.

## Timeline tracks and ruler

`TimelineLane::new()` creates a keyframe lane. `TimelineLane::ruler()` creates a time ruler.
Configure both with the same duration and visible interval to align ticks, diamonds, and
the playhead:

```rust
use ferroui_designer::TimelineLane;

let ruler = TimelineLane::ruler();
let track = TimelineLane::new();
ruler.configure(5.0, Vec::new());
track.configure(5.0, vec![0.0, 2.5, 5.0]);
for lane in [&ruler, &track] {
    lane.set_view(1.0, 4.0);
    lane.set_playhead(2.5);
}
track.set_selected(Some(2.5));
track.set_highlighted(true);
```

The control reports intent through `on_activate`, `on_move`, `on_remove`, `on_insert`,
`on_zoom`, and `on_pan`. A move callback carries the original and destination times. A zoom
callback carries a scale factor and pivot time; a pan callback carries a delta in seconds.
The host applies the command, then updates the controls from accepted state. `view_range`
returns the current visible interval.

| Interaction | Behavior |
| --- | --- |
| Drag the ruler | Scrub continuously |
| Drag a selected diamond | Show selected ghost markers; request one group move on release |
| Ctrl + click / Shift + click | Toggle a key / select a range |
| Drag empty lane space | Marquee-select keys; Ctrl or Shift adds to the selection |
| Ctrl + A | Select every key in the focused lane |
| Escape or lost pointer capture | Cancel the pending keyframe move |
| Double-click an empty point on a track | Request a keyframe at that time |
| Left / Right | Nudge selected keys by 0.1 seconds, or scrub when no key is selected |
| Ctrl + Left / Right | Nudge by 0.01 seconds |
| Shift + Left / Right | Nudge by 1 second |
| Alt + Left / Right | Seek to the previous / next key in the focused lane |
| Ctrl + D | Duplicate after the selected range, separated by one keyboard step |
| Delete / Backspace | Remove the selected keyframes |
| Home / End | Seek to the timeline boundaries |
| Ctrl + wheel | Zoom around the pointer |
| + / − | Zoom around the playhead |
| Shift + wheel, horizontal wheel | Pan the visible interval |
| Page Up / Page Down | Pan by most of the visible interval |

Pointer retiming snaps to 0.1 seconds, or 0.01 seconds with Ctrl. The interval becomes finer
at close zoom levels; Alt disables snapping. Tick labels adapt to the visible scale,
including millisecond precision. Keys are sorted once during configuration, hit testing uses
binary search, and rendering visits only keys inside the visible interval.

The workspace offers seconds or 24/30/60 fps snapping. A host can use
`set_frame_rate(Some(fps))` for any integer rate from 1 through 1,000, or `None` for seconds.
In frame mode arrows move by one frame, Shift by ten frames, and Ctrl retains whole-frame
precision; the ruler displays frame labels. Pointer group moves preserve relative offsets and
clamp a shared delta at the focused lane's timeline boundaries. Cross-lane destinations are
then validated together before committing. Persisted key times use the native animation clock's
100 ns precision so accepted keys remain selectable after source round trips.

Batch hosts use `set_selected_keys`, `selected_keys`, `on_selection_changed`,
`on_move_selection`, `on_remove_selection` and `on_duplicate_selection`. Existing single-key
callbacks remain supported. The selection callback includes whether other lanes should retain
their selection, allowing a workspace to support groups across multiple objects/properties.

## Workspace authoring

The workspace shows compact object/property lanes and a contextual editor for a single selected
key. Selecting multiple keys reveals the Offset/Move/Duplicate/Delete toolbar.
Time, value, and curve changes are validated together and committed in one source transaction,
so one undo restores the whole edit. Invalid input remains available for correction. An
editor captured against an older document URI, syntax snapshot, revision, or active animation
cannot modify the newly active animation, including replacement sessions with coincident IDs.

Zoom, pan, and Fit update existing lanes without rebuilding keyframe editors. This preserves
native focus and unfinished input while changing the viewport. Shortening an animation
resets an interval that no longer fits its duration.

`+ Keyframe` records the currently selected track's property when it belongs to the selected
object; otherwise it records that object's opacity. Values come from the accepted live
preview at the playhead, including interpolation. Record mode continues to capture supported
property edits through the existing session recording pipeline.

A host can reveal an exact keyframe programmatically:

```rust
workspace.select_keyframe(target_node_id, "Width", 2.5)?;
```

The call validates the authored key, selects its object, seeks the playhead, and opens the
contextual editor. New animation programs derive their default selector from the selected
owner and choose an available name. Duplicate creates an available copy name when the name
field still displays the original program.

`DesignerWorkspace::select_keyframes` accepts `(NodeId, property, time)` tuples across tracks.
`selected_keyframes`, `move_selected_keyframes`, `duplicate_selected_keyframes` and
`remove_selected_keyframes` expose the matching group commands. Each group edit validates
every destination before one source transaction; collisions with unselected keys or an
out-of-range destination reject the entire operation. Moving a selected group onto its own
previous positions is valid. Key values and easing remain attached to their keys, and undo
restores the group together. The toolbar exposes selection count, duplicate and delete; curve
presets populate the single-key editor and commit only when Apply is activated.

`set_timeline_frame_rate` and `timeline_frame_rate` coordinate snapping across the ruler and
all lanes. Selection, zoom and frame-rate changes do not author source. Accepted keyboard
edits restore native focus into the rebuilt lane or editor so repeated nudges continue without
requiring a mouse click. Retired timeline editors and gestures remain guarded against URI,
revision, AST identity and active animation changes. Property/layout callbacks additionally
validate the captured destination selection.
