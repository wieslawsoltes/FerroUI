---
title: "Animate and preview variations in VS Code"
description: "Edit animation tracks, test native state transitions, compare design fixtures, and play a multi-view prototype."
---

Open the source with **FerroUI: Open Designer** first. The animation and stage tools render FerroUI controls with the native layout and animation engines.

## Edit an animation

1. Open [DesignWorkflows.xaml](../samples/DesignWorkflows.xaml) or an XAML file with an authored animation. Choose **FerroUI: Show Animation Timeline**. In the default VS Code layout, the **FerroUI Timeline** tab opens below the source and design. Drag the Panel divider or use **Maximize Panel** for more space. If you previously moved the view, open its title context menu and choose **Reset Location**.
2. Select an animation in the program picker. To create one in XAML, select an authored control and choose **New animation program** from **More Animation Actions**. The same menu manages program names, targets, timing, duplication, and removal. **Timing and Playback** includes delays, repeat, speed, direction, fill, and easing. Duration changes scale the authored key times.
3. Click the time ruler or drag the playhead to preview a pose. Enter an exact time in the seconds field and press Enter. **Previous keyframe** and **Next keyframe** step between authored keys. Use **Play** or **Pause** for the selected program. Open the gear icon, **Timeline view options**, for **Play all animations**, zoom, and keyframe snapping. **Stop** restores the underlying values. A compatible source edit can retain the paused pose and elapsed time.
4. Click a key or focus it and press Enter. The key editor opens beside the tracks on a wide Panel and below them in a narrow dock. Change the time or value and [edit the easing](#shape-an-incoming-easing). **Seek to selected keyframe** previews its exact pose.
5. Open **Animation curves** to see interpolated scalar values, including overshoot. Drag a key horizontally to retime it or vertically to change its value. The curve and keyframe editor use the same checked source transaction. Use **Fit selected keyframes** to zoom into a group of keys in the curve editor.
6. Toggle a target header to collapse its property tracks into a row of summary diamonds. On expanded tracks, round keys have incoming easing and diamonds mark linear keys. Open **Filter tracks** and type a property or target name, or turn on **Selected controls only** to follow the canvas and Outline selection. A filter expands matching groups while it is active. Clear the search and uncheck the selection filter to return to all tracks.
7. Use the lock beside a target to protect its keys from Timeline edits. Locked keys still seek when you select them. Rectangle selection and Select All use displayed, unlocked property tracks. Unlock the tracks before arming **Record animation edits** for the canvas.

Escape dismisses the filter popup when it has focus.

### Scale selected key times

Select at least two keys, then seek to the time that should stay fixed. Open **Keyframe Selection Actions**, choose **Scale from playhead**, and enter a factor. Use `0.5` to halve the spacing from the playhead or `2` to double it. The edit preserves key values and easing. If a destination would collide or leave the clip, adjust the selection, playhead, or factor and try again.

Scaling works with XAML and mapped Rust cues. For a Rust cue with multiple setters, select each of its keys when moving the cue. One Undo restores the complete selection's original times. Agents can pass the `scale_keyframes` action with `keyframes`, `pivot`, and `factor` to the shared `designer_program` authoring tool.

| With the Timeline ruler or a key focused | Action |
| --- | --- |
| `[` and `]` | Seek to the previous or next key among the visible tracks. The ruler pans when the key is outside the current zoom. |
| `,` and `.` | Step one frame backward or forward at the selected rate, or at 30 fps when showing seconds. Hold Shift to step ten frames. |
| Home and End | Seek to the start or end of the authored program. |
| `I` and `O` | Set the preview range In or Out at the current playhead. |
| `+`, `−`, `0` | Zoom in, zoom out, or fit the full duration. Ctrl or Command with the wheel zooms at the pointer. |
| Page Up and Page Down | Pan the zoomed ruler. You can also use the pan slider or scroll horizontally. |
| Shift, Ctrl, or Command with a key click | Select a range or toggle individual keys. Space toggles a focused key. Drag from empty track space to select keys in a rectangle. |
| Left and Right with a key focused | Nudge selected keys. Set the edit grid to 24, 30, or 60 fps from **Timeline view options**. Use **Keyframe Selection Actions** for supported move, duplicate, and delete actions. |

For a native animation resource, choose **Choose preview targets** in **Timeline view options** to mount it on one or more controls. Use its elapsed playhead for payloads whose keys are edited in XAML. [Designer Studio animation authoring](../README.md#animation-authoring)

## Rehearse a section

1. Choose a typed XAML or mapped Rust clip whose targets render in the designer. Open **Preview range** in the toolbar and enable **Limit playback to preview range**.
2. Set **In** and **Out** in seconds, or seek to either point and press `I` or `O` with the ruler focused. A blue line marks the range on the ruler. Keep **Loop range** enabled for repeat playback or clear it to hold the Out pose after one pass.
3. Use **Play preview range**, **Pause preview range**, and **Stop** in the transport. Play starts at the current playhead when it falls inside the range. Otherwise it starts at In. Playback disarms recording and samples the authored timeline. Clear **Limit playback to preview range** to use the clip's authored delay, repeat, and direction again.
4. To bookmark a pose, seek to it, enter a name in **Preview range**, and choose **Add or update marker at playhead**. Click its marker on the ruler or its name in the popup to return to that time. Use **Remove marker** beside the name to delete it. Markers belong to the clip for the current designer session.

The VS Code MCP tool `designer_timeline_control` accepts `action: "play_range"` with a program and a `range` containing `start`, `end`, and `loop`. Use `designer_timeline_markers` to read markers and `designer_timeline_marker_edit` to set or remove them. Use the current designer revision for each action.

## Shape an incoming easing

1. Select a source-backed key. **Incoming easing** shapes the segment that ends at the key. Choose a preset, type four cubic Bézier coordinates, or drag either control point on the curve.
2. Drag horizontally to adjust progress between 0 and 1. Drag vertically above or below that interval to shape overshoot. With a control point focused, use the arrow keys for 0.01 steps or hold Shift for 0.1 steps. Escape during a drag restores the starting curve.
3. Choose **Apply keyframe** to commit the time, value, and easing in one source edit. Use Undo to restore the original key. The same editor appears for supported keys in [App Preview](runtime.md#render-with-your-app-in-app-preview).

## Record a property or geometry change in XAML

Select the target’s program and seek to the time you want. **Add keyframe at playhead** samples a supported effective value, and double-clicking an empty part of a track records at that time. Turn on **Record animation edits** to edit animatable values in Properties or to drag and resize a targeted control in the canvas. The resulting keys preserve base attributes and bindings. One Undo restores the last recorded gesture. Turn recording off to edit the base values again.

## Edit mapped Rust clips and state routes

Open [animation_view.rs](../../../editors/vscode/test/fixtures/animation_view.rs), then select the RSX invocation containing the inline style animation. Timeline can preview that animation and patch its supported literal key times, values, and incoming splines. The source action jumps to the authored declaration. Use the warning action to find computed clips in the original Rust. For custom property accessors, export the [animation getter metadata](../../../editors/vscode/README.md#map-an-application-control-to-rsx) from the owning Cargo package. Use [App Preview](runtime.md#render-with-your-app-in-app-preview) for its registered control and animator.

1. Seek to a time and choose **Add keyframe at playhead**, or double-click an empty part of a property track. The isolated designer samples the track at that time and adds a Rust `.key_frame(...)`. An empty mapped clip starts with Opacity.
2. Select one or more keys and open **Keyframe Selection Actions**. **Duplicate at offset…** copies their values and easing. **Delete** removes the selected setters. With a key focused, use Ctrl+D or Command+D to duplicate at the current snap interval, or press Delete or Backspace.
3. When a cue has several setters, deleting one key preserves the other setters and their easing. Deleting the final key leaves the animation available for another key. One Undo restores the source for each edit.

Agents can preview a draft with `designer_keyframe_draft`. Call `designer_program` through `designer_studio_edit` with the accepted revision. Use `upsert_keyframe` without `previous` to insert an explicit value, `record_keyframe` to sample the accepted preview, `remove_keyframe` for a single property and time, or `duplicate_keyframes` and `remove_keyframes` with the selected key locations.

Author new Rust clip and state group declarations in the source editor. In **Studio → States**, select a group and preview a route. In XAML, use the group’s authoring actions to create states, setters, and transitions. In Rust, edit mapped state literals and applicable route durations and easing. Preview directional and default routes with the transition transport. Close the preview to restore the base view. [Typed Rust state examples](../README.md#rust-visual-states)

## Compare design data and environments

1. Open [PreviewStages.xaml](../samples/PreviewStages.xaml). Run **FerroUI: Show Design Data**. Add a named XAML fixture with typed JSON values, such as a string title and an array of items. Apply it once to store the design-only fixture in the view. Choose **Application values** to return to the regular bindings.
2. Run **Open Preview Stages** and use **Add Stage** to add workspace XAML views or supported RSX invocations. Pick an environment, viewport dimensions or content-fit sizing, a state, and a design fixture as applicable. Use one card for a loading state and another for a populated state, or compare light and dark variants.
3. Use **Select** to inspect a stage or **Interact** to operate its native controls. Pause a stage to hold its current image. Use **Checks** to review accessible names, clipping, and supported contrast warnings. **More Actions** includes PNG export and stage settings.
4. Save the stages as a `.ferrostages` file. Open it again in VS Code or Studio. Each stage retains its chosen view and environment. Source changes update affected stages after a valid render is ready. [Design-time data](../design-time-data.md) · [Preview environments](../xcode-previews.md)

## Play a prototype

Switch the Stages board to **Prototype**. Drag a stage title onto a destination or choose **Add interaction** to add a source-anchored trigger. Choose a native control, set the target stage or overlay, and use **Reveal trigger** to review the authored source. An overlay can set its placement, dimming, and dismissal. Use **Play Prototype** to follow the flow and test the native input. Save the stage set to reopen the same graph in Studio or run a [prototype scenario](runtime.md#run-a-native-ui-scenario). [Prototype authoring](../prototyping.md)
