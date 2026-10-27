# Native toolbox and timeline controls

The designer workspace composes ordinary FerroUI controls with two reusable components:
`AssetToolbox` and `TimelineLane`. Hosts can use either component without loading the
workspace shell. The components do not own a document, start a window, or create an undo
stack; their callbacks let the host apply changes through its existing transaction boundary.

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
| Drag a diamond | Show a ghost marker; request one move on release |
| Escape or lost pointer capture | Cancel the pending keyframe move |
| Double-click an empty point on a track | Request a keyframe at that time |
| Left / Right | Nudge a selected key by 0.1 seconds, or scrub when no key is selected |
| Ctrl + Left / Right | Nudge by 0.01 seconds |
| Shift + Left / Right | Nudge by 1 second |
| Delete / Backspace | Remove the selected keyframe |
| Home / End | Seek to the timeline boundaries |
| Ctrl + wheel | Zoom around the pointer |
| + / − | Zoom around the playhead |
| Shift + wheel, horizontal wheel | Pan the visible interval |
| Page Up / Page Down | Pan by most of the visible interval |

Pointer retiming snaps to 0.1 seconds, or 0.01 seconds with Ctrl. The interval becomes finer
at close zoom levels; Alt disables snapping. Tick labels adapt to the visible scale,
including millisecond precision. Keys are sorted once during configuration, hit testing uses
binary search, and rendering visits only keys inside the visible interval.

## Workspace authoring

The workspace shows object/property lanes and one contextual editor for the selected key.
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
