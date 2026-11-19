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

Select at least two keys, then seek to the time that should stay fixed. Open **Keyframe Actions**, choose **Scale from Playhead**, and enter a factor. Use `0.5` to halve the spacing from the playhead or `2` to double it. The edit preserves key values and easing. If a destination would collide or leave the clip, adjust the selection, playhead, or factor and try again.

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
| Left and Right with a key focused | Nudge selected keys. Set the edit grid to 24, 30, or 60 fps from **Timeline view options**. Use **Keyframe Actions** for supported move, duplicate, and delete actions. |
| Ctrl+C or Command+C | Copy selected keys. With a key focused and no selection, copy that key. |
| Ctrl+V or Command+V | Paste at the playhead. Hold Shift to reverse the copied sequence. With one copied track, focus a key on a compatible track to paste there directly. |

For a native animation resource, choose **Choose preview targets** in **Timeline view options** to mount it on one or more controls. Use its elapsed playhead for payloads whose keys are edited in XAML. [Designer Studio animation authoring](../README.md#animation-authoring)

### Copy, paste, and mirror keyframes

![The VS Code Keyframe Actions picker above four selected keys on the Opacity and Width tracks](../images/vscode-keyframe-actions.jpg)

1. Select keys on one or more tracks. Open the **Keyframe Actions** icon and choose **Copy Keyframes**.
2. Open the destination animation in VS Code or Designer Studio. Show and unlock its destination tracks, then seek to the paste time. Add a property track first if its property is missing.
3. Choose **Paste at Playhead**. Assign each copied track to an existing destination of the same value type. For discrete values, choose the same property. The first pasted key lands at the playhead and the remaining keys keep their relative timing across tracks.
4. Choose **Paste Reversed at Playhead** to paste the sequence in reverse. To reflect selected keys in place, seek to the pivot and choose **Mirror About Playhead**. Both actions reflect the Bézier handles between selected keys.

For a Timeline using a native elapsed clock, enter an authored time in seconds when prompted. For a mapped Rust cue with several setters, include every affected key when mirroring. The Timeline reports collisions, out-of-range placements, and Rust cues whose setters would require different splines. Each successful paste or mirror takes one Undo step.

For VS Code MCP, call `designer_keyframe_clipboard` through `designer_studio_query` with the program ID and selected `keyframes`. Each location contains `target`, `property`, and `time`. Pass the returned clipboard to the `designer_program` action `paste_keyframes` through `designer_studio_edit`. The `paste` argument contains `clipboard`, `destinations` in copied track order, the authored `time`, and optional `reverse`. Each destination contains `target` and `property`. Use `mirror_keyframes` with the selected `keyframes` and authored `pivot`. Use the destination frame’s current revision for the edit. The edit returns the accepted key locations as `keyframes`. Designer Studio exposes the same tool and actions directly.

## Rehearse a section

1. Choose a typed XAML or mapped Rust clip whose targets render in the designer. Open **Preview range** in the toolbar and enable **Limit playback to preview range**.
2. Set **In** and **Out** in seconds, or seek to either point and press `I` or `O` with the ruler focused. A blue line marks the range on the ruler. Keep **Loop range** enabled for repeat playback or clear it to hold the Out pose after one pass.
3. Use **Play preview range**, **Pause preview range**, and **Stop** in the transport. Play starts at the current playhead when it falls inside the range. Otherwise it starts at In. Playback disarms recording and samples the authored timeline. Clear **Limit playback to preview range** to use the clip's authored delay, repeat, and direction again.
4. To bookmark a pose, seek to it, enter a name in **Preview range**, and choose **Add or update marker at playhead**. Click its marker on the ruler or its name in the popup to return to that time. Use **Remove marker** beside the name to delete it. Markers belong to the clip for the current designer session.

The VS Code MCP tool `designer_timeline_control` accepts `action: "play_range"` with a program and a `range` containing `start`, `end`, and `loop`. Use `designer_timeline_markers` to read markers and `designer_timeline_marker_edit` to set or remove them. Use the current designer revision for each action.

## Shape an incoming easing

1. Select a source-backed key. **Incoming easing** shapes the segment that ends at the key. Choose a preset, type four cubic Bézier coordinates, or drag either control point on the curve.
2. Drag horizontally to adjust progress between 0 and 1. Drag vertically above or below that interval to shape overshoot. With a control point focused, use the arrow keys for 0.01 steps. Hold Shift for 0.1 steps or Ctrl or Command for 0.001 steps. Escape during a drag restores the starting curve.
3. Choose **Apply keyframe** to commit the time, value, and easing in one source edit. Use Undo to restore the original key. The same editor appears for supported keys in [App Preview](runtime.md#render-with-your-app-in-app-preview).

Designer Studio provides the same drag and nudge increments. Focus the native graph and press Space to switch handles. [Designer Studio animation authoring](../README.md#animation-authoring)

## Record a property or geometry change in XAML

Select the target’s program and seek to the time you want. **Add keyframe at playhead** samples a supported effective value, and double-clicking an empty part of a track records at that time. Turn on **Record animation edits** to edit animatable values in Properties or to drag and resize a targeted control in the canvas. The resulting keys preserve base attributes and bindings. One Undo restores the last recorded gesture. Turn recording off to edit the base values again.

## Edit mapped Rust clips and state routes

Open [animation_view.rs](../../../editors/vscode/test/fixtures/animation_view.rs), then select the RSX invocation containing the inline style animation. Timeline can preview that animation and patch its supported literal key times, values, and incoming splines. The source action jumps to the authored declaration. Use the warning action to find computed clips in the original Rust. For custom property accessors, export the [animation getter metadata](../../../editors/vscode/README.md#map-an-application-control-to-rsx) from the owning Cargo package. Use [App Preview](runtime.md#render-with-your-app-in-app-preview) for its registered control and animator.

1. Seek to a time and choose **Add keyframe at playhead**, or double-click an empty part of a property track. The isolated designer samples the track at that time and adds a Rust `.key_frame(...)`. An empty mapped clip starts with Opacity.
2. Select one or more keys and open **Keyframe Actions**. **Duplicate at Offset…** copies their values and easing. Use the same menu to [copy, paste, and mirror keys](#copy-paste-and-mirror-keyframes). **Delete** removes the selected setters. With a key focused, use Ctrl+D or Command+D to duplicate at the current snap interval, or press Delete or Backspace.
3. When a cue has several setters, deleting one key preserves the other setters and their easing. Deleting the final key leaves the animation available for another key. One Undo restores the source for each edit.

Agents can preview a draft with `designer_keyframe_draft`. Call `designer_program` through `designer_studio_edit` with the accepted revision. Use `upsert_keyframe` without `previous` to insert an explicit value, `record_keyframe` to sample the accepted preview, `remove_keyframe` for a single property and time, or `duplicate_keyframes` and `remove_keyframes` with the selected key locations.

### Create and manage Rust clips

1. Select the control that should own the style. In **More Animation Actions**, choose **New animation program…**. Choose an eligible owner if prompted.
2. Enter a name, FerroUI selector, and duration in seconds with millisecond precision. The new animation appears in the active invocation’s inline `styles` and opens in Timeline. Use **Add keyframe at playhead** to start its first track.
3. Choose **Rename animation…**, **Duplicate animation…**, or **Remove animation** from the same menu. Renaming keeps the clip’s source identity. Duplicating copies its literal values, timing, and easing with a new identity. Each operation is one Undo step.

The designer stores the name and identity in a Rust comment beside the builder. It can append a clip alongside computed entries in a literal styles array. For a computed styles field, choose an enclosing control with inline styles to hold the new rule, or author the builder in Rust.

Agents use `designer_program` through `designer_studio_edit` with `create`, `rename`, `duplicate`, and `remove`. For creation, supply the current owner ID, name, selector, and duration. Read `designer_programs` after a source edit for the new clip and target IDs.

### Edit and preview visual states

1. Select the control that owns the state group. Open **Studio → States → Add state group…**, then enter a group name and an initial state name.
2. Open the group and choose **Add state…**. Select a state, then choose **Add or edit a state setter…**. Use a property such as `Opacity` on the owner, or supply the name of a descendant such as `Card` and set its `Width`.
3. Choose **Add transition…** in Rust or **Add or edit transition…** in XAML. Set the starting and destination states, then enter the duration in seconds. Choose **Any state** for a default route. Rust routes use whole milliseconds and accept native easing names or four comma-separated Bézier coordinates.
4. Choose **Preview transition…** to open the route in Timeline. Scrub to an intermediate frame, play the route, or reveal its source. **Clear state preview** restores the base view.
5. Open a state to remove it or one of its setters. Removing a state also removes routes connected to it. Use **Remove transition…** and **Remove state group** for those declarations. Each saved authoring action uses one editor Undo step.

Rust authoring works on mapped inline `visual_states` declarations. When a control has no such field, adding its first group inserts the field in the active `rsx!` invocation. Adding a sibling requires an inline array with identifiable group names. Setters use mapped numeric or geometry literals and built-in or project-registered animatable properties. Comments and surrounding Rust survive the checked source edit. [Typed Rust state examples](../README.md#rust-visual-states)

For MCP, read `designer_states` through `designer_studio_query` to get the groups, source ranges, and eligible Rust owner IDs in `creatableOwners`. Send `designer_state` through `designer_studio_edit` with the current designer revision. Use `upsert` with the complete group model for state, setter, and route changes. Use `remove` with the owner ID and group name to delete a group. Read the accepted frame again before issuing the next edit.

### Scrub a Rust state route in App Preview

1. Open the Rust view and run **FerroUI: Open App-Hosted Design**. After the app accepts the view, choose a route under **State routes** in the animation picker. Routes appear when the app can resolve their controls and registered animators. **Base → Ready** uses the current underlying values as the starting point.
2. Drag the playhead or focus it and use the arrow keys. Use **Play**, **Pause**, or **Stop** to drive the route. Stop removes the sampled layer and returns to the app’s own values.
3. Pause playback and use **Edit state route timing** to change the duration in whole milliseconds and the easing. The change to the Rust buffer uses one Undo step. **Reveal Rust state route in source** selects the declaration. A compatible replacement keeps its elapsed pose through the edit.
4. In Todo Workbench, choose the **Interactive review** Rust fixture to test its compiled button handlers. Stop designer playback before trying **Queued**, **Ready**, and **Reset**. The fixture’s runtime uses the logic compiled into the selected binary. The playhead samples eligible route values from the current Rust buffer. Rebuild the binary to exercise edited logic through its handlers. [App Preview setup](runtime.md#render-with-your-app-in-app-preview)

Agents can read the route details in `designer_hosted_animations`. Use `designer_hosted_animation` for playback and reveal, `designer_hosted_state_route_draft` to review the source patches, or `designer_hosted_state_route` to apply them. Pass the current app generation, frame revision and sequence. Durations in the MCP tools are expressed in seconds.

## Compare design data and environments

1. Open [PreviewStages.xaml](../samples/PreviewStages.xaml). Run **FerroUI: Show Design Data**. Add a named XAML fixture with typed JSON values, such as a string title and an array of items. Apply it once to store the design-only fixture in the view. Choose **Application values** to return to the regular bindings.
2. Run **Open Preview Stages** and use **Add Stage** to add workspace XAML views or supported RSX invocations. Pick an environment, viewport dimensions or content-fit sizing, a state, and a design fixture as applicable. Use one card for a loading state and another for a populated state, or compare light and dark variants.
3. Use **Select** to inspect a stage or **Interact** to operate its native controls. Pause a stage to hold its current image. Use **Checks** to review accessible names, clipping, and supported contrast warnings. **More Actions** includes PNG export and stage settings.
4. Save the stages as a `.ferrostages` file. Open it again in VS Code or Studio. Each stage retains its chosen view, environment, state, and design fixture. Source changes update affected stages after a valid render is ready. [Design-time data](../design-time-data.md) · [Preview environments](../xcode-previews.md)

## Play a prototype

Switch the Stages board to **Prototype**. Drag a stage title onto a destination or choose **Add interaction** to add a source-anchored trigger. Choose a native control, set the target stage or overlay, and use **Reveal trigger** to review the authored source. An overlay can set its placement, dimming, and dismissal. Use **Play Prototype** to follow the flow and test the native input. Save the stage set to reopen the same graph in Studio or run a [prototype scenario](runtime.md#run-a-native-ui-scenario). [Prototype authoring](../prototyping.md)
