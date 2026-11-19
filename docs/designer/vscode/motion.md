---
title: "Animate and preview variations in VS Code"
description: "Edit animation tracks, test native state transitions, compare design fixtures, and play a multi-view prototype."
---

Open the source with **FerroUI: Open Designer** first. The animation and stage tools render FerroUI controls with the native layout and animation engines.

## Edit an animation

1. Open [DesignWorkflows.xaml](../samples/DesignWorkflows.xaml) or an XAML file with an authored animation. Choose **FerroUI: Show Animation Timeline**. In the default VS Code layout, the **FerroUI Timeline** tab opens below the source and design. Drag the Panel divider or use **Maximize Panel** for more space. If you previously moved the view, open its title context menu and choose **Reset Location**.
2. Select an animation in the program picker. To create one in XAML, select an authored control and choose **New animation program** from **More Animation Actions**. The same menu manages program names, targets, timing, duplication, and removal. **Timing and Playback** includes delays, repeat, speed, direction, fill, and easing. Duration changes scale the authored key times.
3. Click the time ruler or drag the playhead to preview a pose. Enter an exact time in the seconds field and press Enter. **Previous keyframe** and **Next keyframe** step between authored keys. Use **Play**, **Pause**, or **Play All Animations** to preview the native clock. **Stop** restores the underlying values. A compatible source edit can retain the paused pose and elapsed time.
4. Click a key or focus it and press Enter. The key editor opens beside the tracks on a wide Panel and below them in a narrow dock. Change the time or value, choose an incoming easing preset such as **Ease out**, or enter custom cubic Bézier coordinates. The graph previews the staged easing. Choose **Apply keyframe** to commit the validated draft. **Seek to selected keyframe** previews its exact pose.
5. Open **Animation curves** to see interpolated scalar values, including overshoot. Drag a key horizontally to retime it or vertically to change its value. The curve and keyframe editor use the same checked source transaction. Use **Fit selected keyframes** to zoom into a group of keys in the curve editor.
6. Open **Filter tracks** and type a property or target name to narrow the track list. Key navigation, rectangle selection, and Select All use the matching tracks. Press Escape in the search field to clear it and return to the filter button. Filtering is local to the Timeline.

| With the Timeline ruler or a key focused | Action |
| --- | --- |
| `[` and `]` | Seek to the previous or next key among the visible tracks. The ruler pans when the key is outside the current zoom. |
| `,` and `.` | Step one frame backward or forward at the selected rate, or at 30 fps when showing seconds. Hold Shift to step ten frames. |
| Home and End | Seek to the start or end of the authored program. |
| `+`, `−`, `0` | Zoom in, zoom out, or fit the full duration. Ctrl or Command with the wheel zooms at the pointer. |
| Page Up and Page Down | Pan the zoomed ruler. You can also use the pan slider or scroll horizontally. |
| Shift, Ctrl, or Command with a key click | Select a range or toggle individual keys. Space toggles a focused key. Drag from empty track space to select keys in a rectangle. |
| Left and Right with a key focused | Nudge selected keys. The 24, 30, and 60 fps options set the edit grid. Use **Keyframe Selection Actions** for supported move, duplicate, and delete actions. |

For a native animation resource, choose **Preview Targets** to mount it on one or more controls. Use its elapsed playhead for payloads whose keys are edited in XAML. [Designer Studio animation authoring](../README.md#animation-authoring)

## Record a property or geometry change in XAML

Select the target’s program and seek to the time you want. **Add keyframe at playhead** samples a supported effective value, and double-clicking an empty part of a track records at that time. Turn on **Record animation edits** to edit animatable values in Properties or to drag and resize a targeted control in the canvas. The resulting keys preserve base attributes and bindings. One Undo restores the last recorded gesture. Turn recording off to edit the base values again.

## Edit mapped Rust clips and state routes

Open [animation_view.rs](../../../editors/vscode/test/fixtures/animation_view.rs), then select the RSX invocation containing the inline style animation. Timeline can preview that animation and patch its supported literal key times, values, and incoming splines. The source action jumps to the authored declaration. Use the warning action to find computed clips in the original Rust. For custom property accessors, export the [animation getter metadata](../../../editors/vscode/README.md#map-an-application-control-to-rsx) from the owning Cargo package. Use [App Preview](runtime.md#render-with-your-app-in-app-preview) for its registered control and animator.

In **Studio → States**, select a group and preview a route. In XAML, use the group’s authoring actions to create states, setters, and transitions. In Rust, edit mapped state literals and applicable route durations and easing. Preview directional and default routes with the transition transport. Close the preview to restore the base view. Structural Rust animation and state declarations are authored in Rust. [Typed Rust state examples](../README.md#rust-visual-states)

## Compare design data and environments

1. Open [PreviewStages.xaml](../samples/PreviewStages.xaml). Run **FerroUI: Show Design Data**. Add a named XAML fixture with typed JSON values, such as a string title and an array of items. Apply it once to store the design-only fixture in the view. Choose **Application values** to return to the regular bindings.
2. Run **Open Preview Stages** and use **Add Stage** to add workspace XAML views or supported RSX invocations. Pick an environment, viewport dimensions or content-fit sizing, a state, and a design fixture as applicable. Use one card for a loading state and another for a populated state, or compare light and dark variants.
3. Use **Select** to inspect a stage or **Interact** to operate its native controls. Pause a stage to hold its current image. Use **Checks** to review accessible names, clipping, and supported contrast warnings. **More Actions** includes PNG export and stage settings.
4. Save the stages as a `.ferrostages` file. Open it again in VS Code or Studio. Each stage retains its chosen view and environment. Source changes update affected stages after a valid render is ready. [Design-time data](../design-time-data.md) · [Preview environments](../xcode-previews.md)

## Play a prototype

Switch the Stages board to **Prototype**. Drag a stage title onto a destination or choose **Add interaction** to add a source-anchored trigger. Choose a native control, set the target stage or overlay, and use **Reveal trigger** to review the authored source. An overlay can set its placement, dimming, and dismissal. Use **Play Prototype** to follow the flow and test the native input. Save the stage set to reopen the same graph in Studio or run a [prototype scenario](runtime.md#run-a-native-ui-scenario). [Prototype authoring](../prototyping.md)
