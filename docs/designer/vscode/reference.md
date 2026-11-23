---
title: "Use agents and configure FerroUI in VS Code"
description: "Connect the editor, Studio, and application MCP servers, and look up commands and settings."
---

## Start the right MCP server

1. Open a trusted workspace in desktop VS Code. Run **MCP: List Servers** and start **FerroUI Designer in VS Code**. Choose its tools in your VS Code agent.
2. Ask the agent to read the active unsaved document or open a design and describe its accepted native frame. To permit source edits, designer actions, or managed Cargo commands, enable `ferroui.mcp.allowWrites` in VS Code user settings. Editor write access is off by default.
3. For Studio or an independently running application, use **FerroUI: Connect to Running MCP Server** and supply the authenticated loopback endpoint. **Disconnect Running MCP Server** removes that connection. Configure `ferroui.mcp.stdioServers` for a known executable using its native stdio transport.
4. Start a configured app through **Run with Hot Reload** to discover its named FerroUI MCP server when the app advertises one. Its runtime inspection and input permissions come from the app. An in-place update keeps the connection. A replacement process advertises its own server after startup.

| Server or scope | What to use it for |
| --- | --- |
| **FerroUI Designer in VS Code** | VS Code’s open buffers, project files and language providers, design selection and views, authoring in the designer, Stages, and managing Cargo. |
| **Designer Studio** attached from VS Code | Studio’s open documents, native project actions, stage sets, animation and layout tools, and collaboration features of the Studio session. |
| Named managed or attached app | Actual native windows, properties, binding and layout diagnostics, accessibility, screenshots, and permitted input. |

Editor and Studio authoring reuse the same native planning and validation. The editor applies source changes through VS Code’s undo history. Studio applies them through its document history. Use the server owning the unsaved buffer you intend to edit. [VS Code MCP setup](https://code.visualstudio.com/docs/copilot/customization/mcp-servers) · [Designer Studio MCP](../README.md#mcp-integration) · [Application MCP](../../mcp.md)

## Drive an editor workflow

| Task | Tools |
| --- | --- |
| Find source and language information | `ferroui_project_files`, `ferroui_workspace_files`, `ferroui_workspace_read`, and `ferroui_workspace_language`. Edits and file actions use `ferroui_workspace_create`, `ferroui_workspace_edit`, and `ferroui_workspace_file`. |
| Find reusable project definitions | `ferroui_project_library` accepts a workspace `folder`, a text `query`, and `offset` and `limit` for paging. Pass a returned `entry` to `designer_open_project_definition` with `destination` set to `preview` or `source`. Opening a definition requires `ferroui.mcp.allowWrites` because it changes the active editor. |
| Import a workspace asset | Call `ferroui_project_plan_asset_import` with `source` and `destination` paths in the same workspace folder. Pass `folder` when needed to disambiguate. Use the same arguments and the returned `review` with `ferroui_project_import_asset` to create the copy. Prepare a new review if the bytes or path change. |
| Move an asset with references | Call `ferroui_project_plan_asset_move` with an indexed `path` and `destination`. For a published asset, pass its `ferres://` ID from `designer_assets` as `path` and include that design’s `session`. Review `publishedAssets` and the patches in `documents`. Pass the original request and `review` to `ferroui_project_move_asset`. Studio accepts the published ID as `asset` in `designer_plan_asset_move` and `designer_move_asset` with the current document revision. |
| Undo or redo one editor transaction | Use `designer_history` with `session`, `action`, and `expected_version`. For a workspace file, use `ferroui_workspace_file` with `action` and the `expected_revision` from `ferroui_workspace_read`. VS Code focuses the authored tab in its existing editor group, so the adjacent Design tab stays visible. After one operation completes, use the returned buffer version for the next call. A second request with the old version is rejected. |
| Format XAML | In VS Code, pass `action: "format"`, `path`, and the current `expected_revision` to `ferroui_workspace_file`. Set `folder` in a multi-root workspace. Omit `range` for the whole document or supply zero-based UTF-16 start and end positions. Omit `options` to use the folder settings. For one call, options such as `attributeLayout: "onePerLine"`, `tabSize: 2`, and `selfClosingSpace: "space"` override them. In Studio, `designer_language` accepts the same range and options for the `format` action. Review its returned `edit`, then call `designer_language_apply` with `expected_revision`. [Formatter settings and protected regions](authoring.md#format-xaml) |
| Open and inspect a design | `designer_open`, `designer_block`, and `designer_wait` choose the view and wait for an accepted frame. Use `designer_canvas`, `designer_view`, and `designer_preview_image` for its mode, display, workbench views, and pixels. `designer_canvas_input` sends native interactions. |
| Filter the source Outline | Call `designer_outline` with `session` and `filter` to match control types or authored names. The results include the matching controls and their ancestor paths in source order. `offset` and `limit` page through the filtered rows. |
| Inspect and edit layers | Read `designer_layers` with `designer_studio_query`. Use its `layer_epoch` and `scope_epoch` as `expected_layer_epoch` and `expected_scope_epoch` in the `designer_layer` arguments to `designer_studio_present`. Pass the current frame as `expected_revision`. Use `set_hidden`, `set_locked`, `isolate`, `exit_isolation`, or `reset`. A checked `designer_place` keeps the same layer identities on moved and displaced controls. Use `designer_history` with the current source version to undo or redo a move. [Layer workflow](authoring.md#keep-layer-state-during-edits) |
| Measure and mark the canvas | Read the current scope, ruler settings, coordinates, `revision`, and `epoch` with `designer_canvas_guides`. Pass `session`, `expected_revision`, `expected_epoch`, and `edit` to `designer_canvas_guides_edit`. Use `visibility`, `add`, `move`, `duplicate`, `remove`, `clear`, or `apply` as `edit.action`. For an exact batch, `apply` takes `rulers`, `guides`, and `lines` with `axis` (`Horizontal` or `Vertical`), `position`, and an optional existing `id`. [Canvas guide workflow](authoring.md#measure-and-mark-the-canvas) |
| Snap and place geometry | Use `configure_snap` in `designer_canvas_guides_edit` with `options` containing `grid_visible`, `grid_spacing`, `snap_to_grid`, `snap_to_guides`, `snap_to_objects`, and `snap_tolerance`. For a Canvas placement or resize, pass `snap: { epoch, zoom }` where `zoom` is the canvas scale such as `1` for 100%. Add `bypass: true` for a precise gesture. Preview with `designer_placement_hint` or `designer_resize_hint`, then pass the same unadjusted coordinates and snap context to `designer_place` or `designer_resize`. [Canvas snapping workflow](authoring.md#snap-controls-to-guides-the-grid-or-other-controls) |
| Plan layout and source edits | Use `designer_placement_hint` before `designer_place`, and read the current properties, events, or resources before editing them. `designer_apply_counterpart` applies a supported alternate source. Discover native action schemas with `designer_studio_tools`. Then use `designer_studio_query`, `designer_studio_edit`, or `designer_studio_present` to read, author, or change preview-local state. |
| Place controls in a partial RSX preview | Use `designer_block` to choose the outer invocation and `designer_outline` for the current node IDs and revision. Pass a `node`, an ordered `selection` with an optional `moving_anchor`, or a compatible Toolbox `asset`. Pass `anchor` and `position` for Outline placement. The `after` position goes before the next authored Rust child, including a loop, conditional, or computed collection member. Use `designer_placement_hint` and `designer_place` with the same `expected_revision`. Studio MCP uses the same placement arguments with the current document revision. [RSX placement](authoring.md#move-and-insert-controls-in-a-partial-rsx-preview) · [Inline collections](authoring.md#arrange-an-inline-rust-child-collection) |
| Remove mapped RSX children | Read `designer_outline` for the current revision and node IDs. Use `designer_remove` for one child in VS Code or `designer_apply` with `RemoveNode` edits for a batch. Add supported placement or layout edits to the same transaction when needed. Studio MCP accepts the same batch. The transaction removes the selected authored expressions and preserves the neighboring code. [Inline collections](authoring.md#arrange-an-inline-rust-child-collection) |
| Place a published Cargo raster | Build the owning target, call `designer_assets`, and choose the `id` of a published raster. Pass that ID as `project_asset` to `designer_placement_hint` or `designer_place` with the current `session` and `expected_revision`. In Designer Studio, use `designer_toolbox` to find the ID under **Packaged images** and pass it as `asset` with the current document revision. [Assets workflow](authoring.md#place-a-published-cargo-asset) |
| Change an image source | Choose a raster ID from `designer_assets` and an authored control from `designer_outline`. Call `designer_set_image_source` with `project_asset`, `node`, and the current `session` and `expected_revision`. Pass an eligible `property` from the node’s `imageSourceProperties`. The default is `Source` when present, or the only eligible property. Designer Studio accepts a published raster ID from `designer_toolbox` and the current document revision. |
| Inspect or edit motion | Read programs and tracks with `designer_programs` through `designer_studio_query`. Use `designer_timeline_control` or `designer_transition_control` for playback and seeks. Use `designer_animation_curve` to read values and `designer_keyframe_draft` to validate a key. Commit using `designer_program` through `designer_studio_edit`. Its `place_clips` action accepts ordered program IDs and optional start and gap in seconds. Use `scrub_all` for the shared clip ruler. [Sequence clips](motion.md#compare-and-sequence-clips) |
| Set the visible animation graph | Read `designer_timeline_view` and pass the accepted revision to `designer_timeline_set_graph`. Use the `designer_hosted_timeline_view` and `designer_hosted_timeline_set_graph` pair for App Preview. [Graph display and arguments](motion.md#choose-the-graph-display-from-an-agent) |
| Switch between property tracks and clips | Use `designer_timeline_view` and `designer_timeline_set_layout` with `layout` set to `tracks` or `clips`. App Preview uses `designer_hosted_timeline_view` and `designer_hosted_timeline_set_layout`. [Clip comparison](motion.md#compare-and-sequence-clips) |
| Zoom, pan, or fit the Timeline | Read the visible viewport and pass its ruler and program to `designer_timeline_set_viewport`. Use `designer_hosted_timeline_set_viewport` for a visible App Preview curve or clip ruler. [Viewport actions](motion.md#set-the-visible-timeline-range-from-an-agent) |
| Filter, lock, or choose Timeline tracks | Read `track_view` from `designer_timeline_view`. Use `designer_timeline_track_action` for the applicable filter, disclosure, lock, selection, and curve component controls. App Preview uses `designer_hosted_timeline_track_action` for its property and component pickers. [Track workflows](motion.md#control-timeline-tracks-from-an-agent) |
| Compare or play stages | Read the board with `designer_stages_open`, `designer_stages`, `designer_stage_image`, `designer_stage_checks`, and `designer_stage_hotspots`. Use `designer_stage_change` for configuration, `designer_stage_input` for keyboard input, `designer_stage_pointer` for pointer contacts, and `designer_stage_prototype` for playback. |
| Render with the app registry | `designer_hosted_open`, `designer_hosted_fixtures`, `designer_hosted_fixture`, and `designer_hosted_image` operate App Preview. Read its animation frame and generation with `designer_hosted_animations`. Query and edit motion with the animation, curve, and keyframe tools. Pass `sample: true` and authored `time` to the single-key tools to insert the native value on a track with `canSample`. Use `designer_hosted_keyframe_group_draft` and `designer_hosted_keyframe_group` for a key selection, `designer_hosted_clip_draft` and `designer_hosted_clip_edit` for whole clips, and `designer_hosted_state_group_draft` and `designer_hosted_state_group` for visual states. [Sample an app-hosted key](motion.md#add-a-native-key-in-app-preview) |
| Interact with App Preview | Use `designer_hosted_image` to identify the frame for `designer_hosted_input`. Click, scroll, or send text to the focused control. Refresh with `designer_hosted_refresh` using the revision from `designer_hosted_fixtures`. Both actions require `ferroui.mcp.allowWrites`. [App Preview agent workflow](runtime.md#interact-with-app-preview-from-an-agent) |
| Inspect and copy an App Preview control template | `designer_hosted_templates` lists source-backed templates from the accepted XAML load. Use `designer_hosted_template_source` for the definition, `designer_hosted_template_copy_draft` for review, and `designer_hosted_template_copy` to apply the edit. |
| Sequence app-hosted clips | Use `scrub_all` and `play_all` in `designer_hosted_animation` for the app’s shared elapsed clock. Use `designer_hosted_clip_placement_draft` to check a delay change and `designer_hosted_clip_placement` to apply it to the current buffer in one Undo step. |
| Rehearse an app-hosted range | Use `play_range` in `designer_hosted_animation` with an elapsed `start`, `end`, and `loop` in `range`. Include `program` to rehearse one animation or omit it to rehearse the shared clip clock. |
| Set, remove, or seek to a preview marker | Use `designer_timeline_markers` and `designer_timeline_marker_edit` for a typed clip or the shared Timeline clock. In App Preview, read `markers` from `designer_hosted_animations` and pass `action: "marker"` to `designer_hosted_animation`. [Marker controls and arguments](motion.md#bookmark-preview-positions) |
| Run, inspect, and test | Discover sessions with `ferroui_app_sessions` and control their Cargo builds and inspectors. `ferroui_runtime_source` and `ferroui_runtime_apply_source` work with a mapped runtime value. `ferroui_scenario_run`, `ferroui_scenario_review`, and `ferroui_scenario_approve_baseline` run tests, compare images, and approve a checked capture. [Target app sessions with MCP](#target-app-sessions-with-mcp) |
| Open a running app’s template | `ferroui_runtime_templates` reads the selected element’s templates. Pass its generation and a template ID to `ferroui_runtime_template_source`. [Live App template tools](#inspect-live-app-templates-with-mcp) |
| Profile a running window | Use `ferroui_runtime_profile_windows`, `ferroui_runtime_profile_start`, `ferroui_runtime_profile_read`, `ferroui_runtime_profile_stop`, and `ferroui_runtime_profile_source`. [Runtime profiler](#profile-a-runtime-with-mcp) |
| Create a project | Use `ferroui_project_templates`, `ferroui_project_plan_new`, `ferroui_project_create`, and `ferroui_project_check_toolchain`. [Project setup](#create-a-project-with-mcp) |

For example, ask an agent to open the current design, inspect its animation programs, and seek to one keyframe. It should read the active document and RSX invocation, wait for the accepted frame, read the program ID and its keys, then call the native seek with the current frame. Ask it to capture the resulting frame or report the current position. To update a key, validate the time, value, and easing together before applying the edit.

Read the new buffer revision, frame token, stage epoch, or connection generation after the corresponding change. Text locations are UTF-16. A stale source edit is rejected. Re-read the current state before retrying the edit. The advertised tool schemas specify which identity each tool needs.

### Create a project with MCP

1. Call `ferroui_project_templates` to choose `desktopRsx` or `desktopXaml`. The catalog reports the published FerroUI version and minimum Rust version used by the starter.
2. Call `ferroui_project_plan_new` with `template` and `name`. In VS Code, pass the workspace `folder` when several are open. The optional `checkout` is a relative directory inside that folder. The plan includes the destination, opening view, generated source, and review ID.
3. Pass the same inputs and `review` to `ferroui_project_create`. The folder must still be available. Creation uses the existing MCP write permission and returns the project root, view, and manifest paths.
4. Call `ferroui_project_check_toolchain` for the folder or pass `manifest_path`, `package`, and `binary` from the Cargo target list. Pass a Rust target triple to check its installed libraries.

Designer Studio exposes the same tools under the open solution. Its `designer_project_edit` and `designer_project_task` open the resulting project and run its Cargo binary. [Create from the VS Code Project view](authoring.md#start-a-ferroui-project)

In VS Code, call `ferroui_project_run_cargo` with a target’s folder, manifest, package, and binary. The Cargo output appears in the task terminal. Use `ferroui_project_cargo_tasks` and `ferroui_project_stop_cargo` to find and stop a running task. For a binary registered for hot reload, use `ferroui_reload_control`.

### Choose a stage theme with MCP

Open a board with `designer_stages_open` and read its `availableThemes`. In `designer_stage_change`, use the board’s `expected_epoch` and the stage ID. Send `action: "update"` with the stage’s complete `configuration` and set `environment.theme` to an advertised value such as `"high_contrast"`.

In Studio, read `available_themes` from `designer_preview_stages`, then update the same environment through `designer_preview_stage`. The preview uses the host’s native contrast variant and the choices persist in the stage set. [Compare high contrast in VS Code](motion.md#compare-a-high-contrast-stage)

### Set stage locale and text size with MCP

Read the board with `designer_stages`. When it reports `textEnvironment`, update the selected stage with `designer_stage_change` and the current board epoch. Send the complete configuration with `text: { locale: "ja-JP", scale_percent: 150 }`. An omitted locale uses the application locale. An omitted scale uses 100%. Text size accepts whole percentages from 50 through 300.

Studio reports the same support as `text_environment` in `designer_preview_stages`. Use the current Studio model and `designer_preview_stage` to apply the same configuration. Read the new stage epoch before requesting its image or checks. [Compare locale and text size](motion.md#compare-locale-and-text-size)

### Send keyboard input to a stage

1. Read the board with `designer_stages`. Use `designer_stage_change` to set the card's mode to `live`. Read the accepted stage ID and epoch from the returned board.
2. Call `designer_stage_input` with `session`, `stage`, `expected_epoch`, and `input: { kind: "focus" }`. The result gives the focused control's `name` and `role`, or `null` if there is no focusable control.
3. Pass `input: { kind: "key", key: "Tab" }` or `"Shift+Tab"` to navigate, `"Enter"` or `"Space"` to activate a button, or `input: { kind: "text", text: "Acme" }` to enter text. Use `designer_stage_image` to read the resulting native frame.

Stage input requires `ferroui.mcp.allowWrites` and an updated preview helper. It changes the card's temporary interaction state. In Designer Studio use `designer_preview_stage_input` with its current `expected_model` and stage epoch. [Studio stage automation](../xcode-previews.md#automation)

### Send pointer contacts to a stage

1. Read `designer_stages` and check for `pointerInput`. Set a ready card to `mode: "live"` with `designer_stage_change`, then use its current `stage` and `expected_epoch`. Set `pointer_input` to `"touch"` or `"pen"` in the configuration to use that device for the card. `"automatic"` uses the device supplied by each call.
2. Call `designer_stage_pointer` with `session`, `stage`, `expected_epoch`, and `input: { contact: 1, phase: "down", device: "pen", x: 0.5, y: 0.5 }`. Coordinates are fractions of the image width and height, measured from its upper-left corner. Start inside the artboard.
3. Send `"move"` and `"up"` phases with the same contact and device to drag and release. A captured contact can continue beyond the artboard edges. Send `"cancel"` to clear the gesture and release its capture. For pen contacts you can include `pressure` from 0 to 1, `tiltX` and `tiltY` from −90° to 90°, and `twist` from 0° up to 360°.

Read the result with `designer_stage_image`. Contact input requires `ferroui.mcp.allowWrites`. Studio exposes `designer_preview_stage_pointer` with its `expected_model` in place of `session` and requires both `allow_input` and `allow_property_writes`. [Try touch and pen](motion.md#try-touch-and-pen-on-a-stage)

### Drive the native prototype player

1. Read `designer_stages` and use `designer_stage_prototype` with the board’s `expected_epoch` and `arguments: { action: "start", flow: "Account setup" }`. Use the returned `frame` as `expected_frame` for subsequent navigation, input, and capture.
2. Send `input: { kind: "keyboard", input: { kind: "focus" } }` to `designer_stage_prototype_input` to read the active control. Use the same outer `kind` with `input: { kind: "key", key: "Tab" }` or `input: { kind: "text", text: "Acme" }` to operate the view.
3. Send `input: { kind: "pointer", input: { contact: 1, phase: "down", device: "mouse", x: 0.5, y: 0.5 } }` to start a contact. Send its later phases with the same contact and frame. Coordinates are fractions of the entire artboard, including the area outside the top overlay.
4. Call `designer_stage_prototype_image` with `session` and the current `expected_frame` to capture the base and overlays together. Read the frame returned after an interaction opens a new screen or closes an overlay before sending input again.

In Designer Studio, use `designer_prototype_input` and `designer_prototype_image` with `expected_model` and the playback `epoch` as `expected_frame`. [Native player guide](motion.md#operate-a-native-prototype) · [Studio tool reference](../prototyping.md#embedding-and-automation)

### Author and play prototype transitions

Read the graph through `designer_stages`, then pass it with a `transitions` array
to `designer_stage_change` using `action: "setPrototype"` and the current
`expected_epoch`. For example, this entry assigns a 300 ms slide to the first link.

```json
{"link":0,"transition":{"effect":"slide_up","duration_ms":300,"easing":"ease_out"}}
```

The available effects are `dissolve`, `slide_left`, `slide_right`, `slide_up`, and
`slide_down`. The easing defaults to `ease_in_out`. Pass an empty transitions array to
clear all motion, or omit it to preserve the assignments for existing source anchors.

Read `animating` in the playback scene before sending the next interaction.
Use `reducedMotion: true` with `start` to drive settled frames, or use
`setReducedMotion` to change the preference for an active run. In Studio,
`designer_prototype_action` accepts the same entries with `set_graph` and uses
`reduced_motion` for its playback preference. [Transition guide](motion.md#set-a-prototype-transition)

### Wrap controls with MCP

1. Wait for the accepted designer frame with `designer_wait`. Use `designer_select` for one authored control or a consecutive source sibling run.
2. Call `designer_studio_query` with `tool: "designer_wrap_selection"` and empty `arguments`. Each choice has a `container`, a description, and a reason if the choice is unavailable.
3. Read the exact plan with the same query and `arguments: { container: "canvas" }`. The report includes the ordered child IDs, wrapper properties, source patches in UTF-8 byte offsets, and the accepted `model`.
4. Call `designer_studio_edit` with `tool: "designer_wrap_selection_edit"`, the same `container`, the model as `expected_model`, and the accepted frame as `expected_revision`. If you supply a selection explicitly, use the same ordered list for the plan and the edit. VS Code selects the wrapper after applying a single source edit.

Studio's MCP server exposes `designer_wrap_selection` and `designer_wrap_selection_edit` directly. Use Studio's document revision for the edit. [Designer Studio MCP](../README.md#mcp-integration)

### Adjust measured spacing with MCP

1. Wait for the accepted design using `designer_wait` and choose authored nodes using `designer_select`. To display the handles in the canvas, call `designer_canvas` with `spacing_helpers: true` in Select mode.
2. Use `designer_studio_query` with `tool: "designer_spacing_helpers"` and empty `arguments`. The response lists the measured handles and their `model`, current values, owner IDs, artboard positions, and any disabled reasons. Supply the same ordered selection for the draft and edit if you pass it explicitly.
3. Query `designer_spacing_draft` with `{ model, handle, adjustment: { value: 18.5 } }` for exact entry or `{ model, handle, adjustment: { delta: { x: 0, y: 8 } } }` for a drag. The delta is relative to the measured start in artboard units. The draft returns the proposed value and position.
4. Call `designer_studio_edit` with `tool: "designer_spacing_edit"`, the same arguments, and the current editor frame as `expected_revision`. A successful change takes one Undo. Read a fresh model after a change to the frame or selection.

On a Studio MCP connection these names are direct tools. Pass Studio’s document revision as `expected_revision` on `designer_spacing_edit`. [Designer Studio MCP](../README.md#mcp-integration)

### Edit table columns with MCP

1. Call `designer_wait` and locate a `TableView` or `TreeDataGrid` with `designer_outline`. Select the table and use `designer_view` with `view: "columns"` to show its editor.
2. Call `designer_studio_query` with `tool: "designer_columns"` and `arguments: { node }`. Read the table ID, current `model`, ordered columns, available kinds, field metadata, and source ranges. A `reason` explains when the collection is read-only.
3. Call `designer_studio_edit` with `tool: "designer_columns_edit"`. Pass the accepted editor frame as `expected_revision`. Use `arguments: { action: "move", table, column, index: 0, expected_model: model }` to move a column to the first position. For `insert`, supply an index and an available `kind`. For `set`, supply a column ID, `field`, and `value`. The catalog also includes `duplicate`, `remove`, and `clear`.

Indices start at zero. The move index is the column’s place in the resulting list. Read a fresh model after a source or preview change. On Studio’s server call `designer_columns` and `designer_columns_edit` directly, using Studio’s document revision for `expected_revision`. [Authoring guide](authoring.md#edit-table-columns)

For a mapped RSX `TableView`, use the same tools and pass `kind: "table"` to insert. Pass a plain row path such as `Customer.Name` as the `Binding` value. Column and field ranges address the original Rust buffer. Use `sourceRange` to open the collection and each field’s `range` to reveal protected Rust values. [RSX column authoring](authoring.md#edit-an-rsx-tableview)

For an RSX `TreeDataGrid`, the model offers `text`, `checkBox`, `template`, `rowHeader`, and `expander` where the selected collection supports them. Use the child node ID from `children` to edit an expander's inner column. Send a plain row path for `Binding`, `ChildrenBinding`, `HasChildrenBinding`, `IsExpandedBinding`, or `TextSearchBinding`. Move and remove actions use the top-level column ID. For an application-supplied `source` or `model`, the query explains why edits are unavailable and returns its Rust range. [TreeDataGrid workbench](authoring.md#edit-an-rsx-treedatagrid)

For a shared Rust column collection, `factory` names the function that returns the array. If `sourceUri` is present, `sourceRange` and all column and field ranges use that companion module’s UTF-16 coordinates. Send edits with the same table ID, revision, and model. VS Code includes the changed module’s URI and editor version in `edited_source`. Undo in the owning editor, or call `ferroui_workspace_read` and then `ferroui_workspace_file` with `action: "undo"`. In Studio, use `designer_undo` or `designer_redo` from either tab. [Shared Rust columns](authoring.md#edit-shared-rust-column-definitions)

### Edit item templates with MCP

1. Call `designer_wait` and use `designer_outline` to find the authored control or column. Read `designer_item_templates` through `designer_studio_query` with `arguments: { node }`. The result includes the current model, slot names and ranges, the assigned `includedDefinition` when resolved, and `includedResources` available for assignment.
2. Open the native definition preview with `designer_studio_present` and `tool: "designer_item_template"`. Pass the current editor frame as `expected_revision` and use `arguments: { action: "preview", node, property: "ItemTemplate", expected_model: model }`. When the result includes `opened_definition`, edit the dictionary in the session identified by `opened_session`. Use `designer_definition` with `action: "exit"` to return to the open consuming view from an included template. The response includes its URI as `returned_to`.
3. Use `designer_studio_edit` with the same tool and model fields. For local assignment, use `assign_resource` and its `resource` ID. For included assignment, use `assign_included_resource` and a `key` from `includedResources`. Use `copy_inline` when `canCopyInline` is true. It copies the definition and any private dependencies into the consumer in one Undo step. `copyReason` explains why a copy is unavailable. The other edits are `create_inline` and `reset`. Read a fresh model after a source, dependency, or preview change. Use the same explicit `selection` for query and action.

On Studio’s server, call `designer_item_templates` and `designer_item_template` directly. For `expected_revision`, pass the Studio document revision. [Authoring guide](authoring.md#edit-item-and-cell-templates)

For a control template, pass `property: "Template"` in the query and in each action. The query returns the control's authored assignment and the compatible resources. When `value.kind` is `unset`, the current native preview can still supply a `localDefinition` node ID or an `includedDefinition` source range for an inherited template. For a mapped compiled resource, `includedDefinition.uri` points to the verified XAML input. An unkeyed template in a setter has no `includedDefinition.key`. Use the same `preview`, assignment, copy, creation, reset, and stale-model checks. [Control template workflow](authoring.md#edit-a-control-template)

For RSX, the query returns a `rustBody` range when the template opens on the native canvas. The `range` points to its Rust assignment. `rustFactory` names the function that returns a shared template. `rustBodyUri` and `sourceUri` identify the body and assignment files when their ranges address companion modules. Use the column's node ID from `designer_columns` for `property: "CellTemplate"` or a TreeDataGrid template column's `"CellEditingTemplate"`. For `create_inline`, pass `rust_type` with the type of one item. The report's `canCreate` and `canReset` fields identify available edits.

For `preview`, VS Code reports `opened_template` with the original range. An `opened_session` identifies the owner’s preview when the body is in a companion module. Query that session for its `rust_template` and use it for body edits. Studio selects the same invocation. Use `designer_definition` with `action: "exit"` to return to the consuming view. In VS Code, `returned_session` identifies the consumer after a cross-file return. A reset or inline creation in a companion column module reports `edited_source`. Undo in the respective editor for body and assignment changes. [Inline Rust templates](authoring.md#edit-a-rust-item-or-cell-template) · [Shared Rust templates](authoring.md#edit-a-shared-rust-template)

### Edit styles and palettes with MCP

Use these tools directly in Designer Studio. In VS Code, pass the tool name and arguments through `designer_studio_query` for reads or `designer_studio_edit` for writes. Pass the current editor revision to VS Code and the current document revision to a direct Studio write.

| Tool | Arguments and result |
| --- | --- |
| `designer_style_catalog` | Optional `target`. Lists authored styles and themes, eligible hosts, named palette scopes, and `included_sources` with a checked `source` and label. |
| `designer_style_definition` | `node`. Reads the selector or base, typed setter choices, authored values, and exact source ranges. |
| `designer_style_resources` | `owner` and `property`. Lists compatible resources visible from the setter's current native scope, including static aliases with a compatible authored target. |
| `designer_style_edit` | `create` uses `owner` and `selector`. `selector` uses `node` and `selector`. `base` uses `node` and a compatible `base` ID or null. `setter` uses `owner`, `property`, and `value` or null to remove. `resource` uses `owner`, `property`, `resource`, and `dynamic`. `remove` uses an unkeyed style's `node`. Use `designer_component create_theme` to create a keyed control theme. |
| `designer_palette` | `owner`. Returns authored colors by layer and their resolved Light, Dark, and High contrast values when `contrastPreview` is true. A comparison cell includes the terminal declaration's `range`, an optional alias `path` with reference kinds, and an `issue` when a brush reference is unresolved. An included cell has a checked `source` with its file URI. `contrastProviders` lists the registered packages available when creating the layer. |
| `designer_palette_edit` | `set` uses a literal color's `node` and a native `color` string. `add` uses `owner`, `layer` (`shared`, `default`, `light`, `dark`, or `high_contrast`), `key`, `kind` (`color` or `solid_color_brush`), and `color`. Supply a reported `provider` of `winui`, `material`, or `macos` when creating the High contrast dictionary. |
| `designer_open_style_source` | Pass `source` from the catalog or comparison. In VS Code use `designer_studio_present` with the consuming preview's `expected_revision`. The result includes `opened_session`. In Studio use the consuming document revision and read `editor` from the result. |
| `designer_style_source` | Pass the original `source` from the owning session. Returns the checked `editor` location and source `range`. The editor identifies the style node or the palette owner, layer, and selected color. |

Use the owning session for edits and Undo. In VS Code, return with `designer_view`, the original session, and `view: "styles"`. Use `styles_page: "comparison"` to show the comparison, or choose `"styles"` or `"palette"` to return to editing. In Studio, use `designer_project_edit` with `action: "activate"` and the consumer URI. For a VS Code canvas, set `theme` on `designer_view` to a value from `available_themes` and wait with `designer_wait`. The older `dark` flag still selects Light or Dark when `theme` is omitted. Use `designer_definition enter` through `designer_studio_present` to preview a definition. [Styles & Themes guide](styles.md)

For the main Studio canvas, call `designer_preview_appearance` for the environment and supported themes. Call `designer_preview_set_theme` with `theme`, `expected_uri`, and `expected_revision` from that response. The same command switches the native canvas while preserving the document and its Undo history.

### Edit Rust styles with MCP

Use `designer_rsx_styles` through `designer_studio_query` for the selected RSX invocation. Omit arguments for the catalog or pass a style `node` to read its selector, setters, typed getter choices, and source ranges. `color_resources` lists declarations and their layers in the selected rule. `resource_choices` lists compatible color keys from the style host. `resource_comparison` shows the resolved Light, Dark, and High contrast values and the source range and layer supplying each cell. A linked setter reports `resource.key` and the declaration range when the same source supplies all mapped appearances. It includes `resource.sourceAddress` for an initializer in a separate module. Studio exposes the same tool directly. Computed expressions have source ranges.

Call `designer_rsx_style_edit` through `designer_studio_edit` with the current session and `expected_revision`. In Studio, call it directly with the document revision. Use IDs from the current catalog and definition.

| Action | Arguments |
| --- | --- |
| `create` | `owner` from `style_hosts` and `selector`. |
| `create_nested` | `style` and a nested `selector`, such as `^:pointerover`. |
| `selector` | `style` and the new `selector`. |
| `setter` | `style`, the setter's `node` as `setter`, and the typed `value`. Pass `null` to remove it. |
| `add_setter` | `style`, a `getter` from `available`, and the typed `value`. |
| `add_color_resource` | `style`, a `key` unique within the selected layer, a hex `value`, and optional `layer`. Use `shared` (default), `default`, `light`, `dark`, or `high_contrast`. |
| `color_resource` | `style`, the declaration's `node` from `color_resources` as `resource`, and the new hex `value`. Use the default `mode: "literal"` for a direct color. If the resource reports `shared_constant`, use `mode: "declaration"` to change all references to the constant or `mode: "local"` for a literal in this rule. |
| `set_resource` | `style`, a `key` from `resource_choices`, and exactly one of `setter` or `getter`. Use `setter` to link an existing value and `getter` to add a compatible property. |
| `remove` | `style` with `can_remove` enabled. |

For a color supplied by a separate Rust module, pass its reported `sourceAddress` as `address` and its `range` to `designer_rsx_style_source`. In VS Code, include the design `session` and accepted `expected_revision`. Studio uses the active document revision. A `declaration` edit in Design reports the changed module in `edited_source`. Use the editor’s Undo in that module or `ferroui_workspace_read` and `ferroui_workspace_file` with `action: "undo"`. In Studio, `designer_undo` and `designer_redo` link the view and supplying module in one history step.

For App Preview, read the Rust file, catalog, and definition with `designer_hosted_styles`. Use `designer_hosted_style_edit_draft` or `designer_hosted_style_edit` with the accepted generation, revision, and sequence. In `arguments.edit`, pass the selected action as a nested edit, such as `{"action":"rust","edit":{"action":"set_resource","style":220,"setter":289,"key":"ReviewInk"}}`. The target's `node` must match the reported style or owner. Use `designer_hosted_style_resources` to list compatible keys for a property and `designer_hosted_style_source` to open a reported range. [Rust color resources guide](styles.md#edit-rust-style-color-resources)

When an App Preview value comes from a separate Rust module, the resource and its comparison cells provide `sourceAddress` and `range`. Use those fields with `designer_hosted_style_source`. Keep the style’s address and node as the edit target. A `color_resource` draft in `declaration` mode returns the owning module’s `uri` and patches. Apply the action with the same accepted frame, then use Undo in the owning editor if needed. [Shared Rust palettes in App Preview](runtime.md#edit-a-shared-rust-palette-in-app-preview)

The `color_resource` action accepts a hex color for mapped Rust constructors, struct literals, parsed color literals, and named FerroUI colors. If a packed constructor reads a shared `u32`, the resource reports that integer constant and its source range. Use `declaration` to edit the supplying integer or `local` to override this resource only. [Mapped Rust color forms](styles.md#edit-rust-style-color-resources)

Use `designer_view` with `styles_page: "comparison"` to show the selected inline Rust palette in the dock. In App Preview, use `designer_hosted_style_show` with `arguments.compare: true` and an optional `arguments.target` from the current definition. [App Preview style workflow](runtime.md#edit-styles-and-control-themes-in-app-preview)

### Edit an App Preview palette with MCP

1. Open the XAML view in App Preview with `designer_hosted_open`. Use `designer_hosted_palette` with the design session to read the accepted source addresses and scope IDs. Repeat with `arguments.target` containing the `address` and `owner` to read the palette and compare theme resolution.
2. Call `designer_hosted_palette_edit_draft` with `expected_generation`, `expected_revision`, and `expected_sequence` from the palette result. Pass `arguments.target` and `arguments.edit`. A `set` edit takes the color `node` and `color`. An `add` edit takes the scope `owner`, `layer`, `key`, `kind`, and `color`. For a new High contrast layer, select a `provider` from the reported providers.
3. Apply the same arguments with `designer_hosted_palette_edit` after reviewing the returned patches. VS Code applies one undoable edit to the owning source buffer. Read the new frame before making another edit.

Use `designer_hosted_palette_source` with the accepted frame, the comparison cell's `sourceAddress` or the selected scope's `address`, and its `range` to reveal the terminal declaration. `designer_hosted_palette_references` reads uses in the selected source. Use `designer_hosted_palette_show` with the current generation to open the toolbar palette. Pass `target`, `layer`, or `compare` in `arguments` to choose the initial presentation. [App Preview palette guide](runtime.md#edit-a-palette-in-app-preview)

### Edit App Preview styles with MCP

1. Open the XAML App Preview and call `designer_hosted_styles` with the design session. The default `files` action returns accepted `address` values. Use `arguments.action: "catalog"` with an `address` to list style hosts, resource scopes, and definitions. Use `action: "read"` with `target: { address, node }` to read one definition.
2. Use the reported generation, revision, and sequence as `expected_generation`, `expected_revision`, and `expected_sequence`. Use `designer_hosted_style_edit_draft` with `arguments.target` and `arguments.edit` to review a patch, then `designer_hosted_style_edit` to apply it in one Undo step.
3. For selectors use `selector` with `value`. For setters use `setter` with `property` and the new `value`, or pass `null` to reset it. Use `designer_hosted_style_resources` to read resources for a setter, then use `resource` with the `property`, `key`, and `dynamic` choice. For control-theme inheritance use `base` with a reported `key`, or pass `null` to clear it.

`create` takes a `selector` and a target from `style_hosts`. `create_theme` takes a `key`, `target_type`, and a target from `palette_scopes`. `remove` applies to an unkeyed style. Use `designer_hosted_style_source` with a reported `range` and accepted source `address` to open a declaration. `designer_hosted_style_show` opens the toolbar editor and accepts an optional `arguments.target`. [App Preview styles guide](runtime.md#edit-styles-and-control-themes-in-app-preview)

### Inspect App Preview templates with MCP

Call `designer_hosted_templates` with the design `session` after opening XAML or a registered Rust fixture in App Preview. Each result includes its `owner`, the template `member`, a `definition` with file URI and UTF-16 range, and `canCopy`. A mapped Rust template also reports its `assignment`. Pass the `owner` as `node` and the reported `member` to `designer_hosted_template_source`. Pass the catalog's `generation` and `frame` as `expected_generation`, `expected_revision`, and `expected_sequence`.

The source tool opens the definition by default. For Rust, use `template_destination` with `assignment` to open the reference or `design` to open the mapped template body in Design.

For TreeDataGrid, pass the reported `CellTemplate` or `CellEditingTemplate` member for the column. An expander's inner column has its own owner. The catalog can include a Rust factory defined with `template!` while its cell is still in display mode. [TreeDataGrid template workflow](runtime.md#inspect-treedatagrid-display-and-editing-templates)

When `canCopy` is true, call `designer_hosted_template_copy_draft` with the same fields to review the checked patch. Call `designer_hosted_template_copy` with write access to install the copy as one Undo step. Read a fresh catalog after a source or frame change. [App Preview template workflow](runtime.md#open-and-copy-app-preview-control-templates)

### Target app sessions with MCP

1. Use `ferroui_reload_targets` to find a target. Start it with `ferroui_reload_control` and `action: "start"`, or use `start_debug` with a Rust attach configuration. For an independent app or Studio, call `ferroui_runtime_attach` with its loopback `url`, bearer `token`, and optional `label`. Only share the token with an agent authorized to inspect that app.
2. Read `ferroui_app_sessions`. Use the returned `app_session` for `ferroui_reload_control`, `ferroui_reload_status`, `ferroui_reload_output`, and `ferroui_reload_history`. Supply the exact ID for controls such as `select`, `attach_debug`, `rebuild`, or `stop` when several managed apps are running. Use `stop_all` to stop the managed apps together.
3. Use the returned `runtime` to pin a Live App editor with `ferroui_runtime_open`, inspect a mapped property, or disconnect one attachment with `ferroui_runtime_detach`. Pass the same `runtime` and the current connection generation for checked source and profiler operations. A replacement process retains the session ID and issues a new generation. `managed` follows the selected app and `attached` follows the most recent attachment.

[App Sessions in the workbench](runtime.md#run-several-app-sessions)

### Inspect Live App templates with MCP

Read the application’s current tree to get a node ID. Call `ferroui_runtime_templates` with `node` and the app’s `runtime` from `ferroui_app_sessions`. The catalog lists effective templates, column headers where available, and verified source stamps. The native application tool `get_template_sources` exposes the same template evidence.

Call `ferroui_runtime_template_source` with the same node and runtime, the catalog’s `generation` as `expected_generation`, and the entry’s `id` as `template_id`. The tool rechecks the app’s template and the editor’s source revision before opening the file. [Live App workflow](runtime.md#open-an-effective-template-in-the-running-app)

### Profile a runtime with MCP

1. Call `ferroui_runtime_profile_windows` for the managed or attached application. Use the returned `runtime`, `generation`, and chosen window ID as `runtime`, `expected_generation`, and `window` for `ferroui_runtime_profile_start`. Choose `duration_ms` between 100 and 30,000.
2. Reproduce the interaction. Use `ferroui_runtime_profile_read` with the same runtime and generation, and the returned `id` as `profile_id`. Use `ferroui_runtime_profile_stop` to end the capture early. Diagnostics shows the same report.
3. For a row with `has_source`, pass its `row_id` as `row` to `ferroui_runtime_profile_source` with the same runtime, expected generation, and profile ID. Set `reveal_source` to open the verified file. The report returned by the read tool contains the same measurement data as the JSON export.

Start, Stop, and Reveal Profile Source use the extension’s existing `ferroui.mcp.allowWrites` permission. The application inspector exposes `start_ui_profile`, `get_ui_profile`, and `stop_ui_profile` directly, including when Designer Studio hosts the endpoint. [VS Code Diagnostics](runtime.md#profile-a-running-window) · [Application MCP tools](../../mcp.md#record-a-ui-profile)

### Edit compound properties with MCP

Call `designer_inspect` for a selected control. An editable compound property reports `components` with its ordered `labels`, numeric `values`, and the current coordinate unit if applicable. The `relative` flag indicates whether percentages are available. The `percent` flag indicates the accepted value's unit. `linkable` identifies four-sided values that can share one number. The property row still reports the authored and effective text.

Use `designer_edit_property` with `action: "set"` and a source value such as `"12, 8, 12, 8"` for padding or `"25%, 50%"` for a relative point. For a group, use `designer_studio_edit` with `tool: "designer_property"`, `action: "set"`, and the array of authored `nodes` in the arguments. Both use the current designer revision. Studio offers `designer_properties` and `designer_property` for its native session. A grouped edit applies as one Undo step. [Component fields in Properties](authoring.md#edit-compound-property-values)

### Trace property values with MCP

Call `designer_studio_query` with `tool: "designer_property_trace"` and `arguments: { node, property }`. Use a current source node from `designer_outline`. For an object that exists only in the preview, read `designer_tree` through `designer_studio_query` and pass its inspector ID as `runtime_node` in place of `node`. The report includes the effective value, authored value and referenced resource where available, ordered contributors, and UTF-16 source ranges for mapped declarations. In Designer Studio, call `designer_property_trace` directly with the same arguments.

Pass `include_sources: true` to include cross-file locations. When a contributor or resource comes from an included XAML file, its location is under `included`. Use `included.uri` and `included.range` together to open the supplying declaration. `sourceRevision` is the hexadecimal revision of the accepted XAML text, excluding an initial byte-order mark. Read a fresh trace after the owner changes.

On the running app’s MCP connection, call `get_property_trace` with `node_id` and `property`. Contributor locations use the runtime’s one-based source coordinates and revision fingerprint when available. Read the trace again after interacting with the preview or the app. [Authoring guide](authoring.md#trace-an-effective-property-value)

### Create an event handler with MCP

Build the package’s XAML schema. Call `designer_event_handler` with the current `expected_revision`, `node`, and authored event name such as `Click`. In VS Code, also pass the `session` from `designer_open`. The result describes the Rust owner file, event signature, suggested name, and `rustVersion`. Pass a `handler` to preview the two versioned source edits. The `declaration` in the preview points to the new handler name after the patch.

Call `designer_scaffold_handler` with the same target, `handler`, and `expected_rust_version` from the read. The request requires write access. It updates the workspace buffers as one edit and returns the declaration location. Use `designer_history` in VS Code or `designer_undo` in Studio to undo the change. Save and rebuild when the handler is ready to run. [Events workflow](authoring.md#create-a-rust-handler-for-an-xaml-event)

### Review a resource rename with MCP

In VS Code, call `ferroui_workspace_plan_resource_rename` with `path`, a zero-based UTF-16 `position` at the key or resource reference, and `new_name`. Use `folder` to select the project in a multi-root window. Inspect each returned document’s `before`, versioned `edit`, and `after` source, including any unopened consumers. A family review also returns `themeFamily.owner` and the declaration ranges and layers in `themeFamily.members`.

Call `ferroui_workspace_rename_resource` with the same fields and the returned `review`. The write requires `ferroui.mcp.allowWrites` and applies all changes in one editor Undo step. In Studio, use `designer_plan_resource_rename` and `designer_rename_resource` with the active XAML UTF-8 byte `offset`, `new_name`, and `expected_revision`. Pass the `review` on apply. Use Undo from a participating tab to restore all buffers. [Authoring workflow](authoring.md#rename-a-resource-across-xaml-files)

### Review design handoff with MCP

1. Call `designer_design_handoff` with `manifest` relative to the receiving workspace folder. Pass a VS Code `session` from `ferroui_designs`. Studio uses the active design and open solution.
2. Review the resulting `rows`. Each resolved action includes the external `id` and its `review` value, plus the proposed definition, placement, or copy. To display the interactive review, call `designer_apply_handoff` with `action: "openReview"` and the same manifest and session.
3. Pass the manifest, row ID, and review value to `designer_apply_handoff`. Set `action` to `copy` for a pending transfer, `source` or `preview` for a Library definition, or `openAsset` for an available asset. Studio returns the resolved asset URI. VS Code also opens the file.
4. To place an item, pass `action: "place"`, the current `expected_revision`, an `anchor` node from the Outline, and `position` of `before`, `inside`, or `after`. The source edit uses the current native placement plan and one Undo step. The mapping is checked against the current source and catalog before applying it.

Write actions use `ferroui.mcp.allowWrites` in VS Code or Studio’s MCP write permission. MCP reads manifests from the open workspace. A local file picker can authorize an export folder elsewhere on the workspace host. [Handoff format and workflows](authoring.md#review-design-handoff)

### Review design tokens with MCP

Call `designer_design_tokens` with `direction` set to `import` for DTCG JSON or `export` for a standalone XAML dictionary. Supply the selected source text, including unsaved changes. The response contains `source`, `mappings`, and `issues` for review.

- During import, key `options.mappings` by dotted token path. Set `key` to rename an XAML resource or `xamlType` to choose an applicable color or gradient brush. Set `options.pixelsPerRem` to convert relative dimensions inside tokens, shadow layers, and typography. Typography returns a keyed `TextBlock` theme. Standalone family and weight values return `FontFamily` and `FontWeight` resources.
- During export, key `options.mappings` by XAML resource key. Use `token` for a dotted token path and `kind: "dimension"` to export a double as a `px` dimension.
- Review the generated bytes and skipped rows before writing. In VS Code, `ferroui_workspace_create` creates the approved output at a new workspace path. In Studio, call `designer_create_design_token_file` with the same `conversion` arguments, the returned `review`, and a new `destination` inside the open solution. Studio checks the plan again before creating the file. Enable Studio MCP writes for this operation.

[Design token review in VS Code and Studio](authoring.md#review-a-design-token-conversion)

### Move a resource with MCP

Call `ferroui_workspace_resource_move_targets` with the XAML `path` and a zero-based UTF-16 `position` on the resource declaration or one of its references. Pass `folder` to identify the workspace root when needed. Each destination includes its dictionary label, range, and versioned address.

Pass the same source fields and `destination: {"uri": "…", "request": {"version": -1, "position": {"line": 0, "character": 1}}}` to `ferroui_workspace_plan_resource_move`. Copy the actual `uri` and `request` from the selected destination. An unopened dictionary can have a negative version. Review each document’s `before`, versioned `edit`, and `after` text. `linkedDestination` indicates a new include in the original scope. Call `ferroui_workspace_move_resource` with the same arguments and the returned `review` to apply in one Undo step. Writes require `ferroui.mcp.allowWrites`.

In Studio, use `designer_resource_move_targets`, `designer_plan_resource_move`, and `designer_move_resource`. Replace the source path and position with the active XAML UTF-8 byte `offset` and `expected_revision`. Use the same returned `destination` and `review` fields. Discovery or edits that change a reviewed document require a fresh review. [Authoring workflow](authoring.md#move-a-resource-to-another-dictionary)

### Extract a resource dictionary with MCP

Call `designer_dictionary_extraction` with an authored `node` and the current `expected_revision`. Use the `session` from `designer_open` in VS Code. The result includes the source range, entry count, and suggested sibling filename. Pass `file_name` to preview the versioned `edit`, the `file` to create, and the `review` identifier.

Send `designer_extract_dictionary` with the same `node`, `expected_revision`, `file_name`, and `review`. The write creates the dictionary and updates the view in one editor transaction. Use `designer_history` in VS Code or `designer_undo` in Studio from the original view. When a source or destination changes, read the target again and review the new plan. [Resources workflow](authoring.md#extract-a-resource-dictionary)

### Bind a command with MCP

1. Find the authored control using `designer_outline`. Call `designer_command_bindings` with its `node` and current `expected_revision`. VS Code also takes the design `session`. The result includes `uri`, `version`, command `slots`, authored values and ranges, and available typed `members`. Pass a member’s `path` as `prefix` to browse its children.
2. Send `designer_command_binding` with the same node and revision. Copy `uri` to `expected_uri` and `version` to `expected_version`. In Studio, also copy `scopeEpoch` to `expected_scope_epoch`.
3. Put the slot’s `property` and both choices in `change`. Use `{"kind":"bind","path":"Actions.Save"}` for a registered path, `{"kind":"keep"}` to preserve the current value, or `{"kind":"clear"}` to remove a simple binding. For a slot without a parameter, pass `{"kind":"keep"}` as the `parameter` choice.

The edit stays in the current buffer and groups both changes into one Undo step. A value with `editable: false` provides a range for opening its authored expression. [Command binding picker](authoring.md#bind-a-command-and-its-parameter)

### Edit a Rust command with MCP

1. In Studio, find the authored control with `designer_tree` and call `designer_rsx_commands` with its `node`. In VS Code, find the node with `designer_outline` and pass the command query through `designer_studio_query` with the design `session`. The result supplies the `model`, registered property types, and UTF-16 ranges for authored expressions.
2. Call `designer_rsx_command_edit` with the `node`, `expected_model`, and a `change` containing the command `property` and both `command` and `parameter` choices. Use `{"kind":"set","expression":"approve.clone()"}`, `{"kind":"clear"}`, or `{"kind":"keep"}` for each choice. Pass `keep` for a slot with no mapped parameter.
3. In Studio, put `expected_revision` from the command report on the edit. In VS Code, send the edit through `designer_studio_edit` with the `session` and `expected_revision` from the design outline. A changed source, scope, or layer requires a fresh report before applying. Both choices form one Undo step. [Rust command binding picker](authoring.md#edit-a-rust-command-binding)

### Find registered Rust definitions with MCP

After building the owning Cargo schema, call `ferroui_workspace_language` with `action: "definition"`, the XAML file in `path`, and a zero-based UTF-16 `position`. It returns the same URI and range as **Go to Definition**. In Studio, call `designer_language` with `action: "definitions"` and the XAML byte `offset`. [Supported declarations](authoring.md#follow-registered-rust-declarations)

Use the same definition query on a typed XAML `ferres://` asset reference to retrieve its published source file. Studio also accepts `designer_language` with `action: "resource_uri"` and the authored `reference`. [Packaged assets](authoring.md#open-the-source-of-a-packaged-asset)

### Browse Library scopes with MCP

In Studio, call `designer_library_scopes` with an empty argument object. In VS Code, use `designer_studio_query` with the current design `session`, `tool: "designer_library_scopes"`, and empty `arguments`. Each group contains the authored `owner`, its path, and the objects declared there. Source ranges use UTF-16. When an RSX assignment lives in a companion module, its `sourceUri` identifies that file.

Use an XAML row’s `definition` with the existing definition actions. Use a behavior group’s `owner` with `designer_behaviors` to inspect its triggers. The report identifies the current revision and scope epoch. Read it again after changing source or opening a different definition. [Library workflow](authoring.md#browse-objects-by-owning-scope)

### Review a registered property rename with MCP

Use the registered property's position in XAML, a typed binding, or a resolved style selector predicate. An affected RSX builder mapping appears as a Rust document in the same review, with current editor text and a version when its buffer is open. A mapping supplied by a text constant in another Cargo module uses that module's current buffer. The patch edits the assembly tuple, and a change to the imported mapping requires a fresh review.

In VS Code, call `ferroui_workspace_plan_property_rename` with the XAML file in `path`, an optional workspace `folder`, a zero-based UTF-16 `position`, and `new_name`. Inspect each file's `before`, `after`, and `edit` fields, then check any locations listed in `unmapped`. Pass the same arguments and the returned `review` to `ferroui_workspace_rename_property`.

In Studio, use `designer_plan_property_rename` with the active XAML byte `offset`, `new_name`, and `expected_revision`. Pass the same inputs and `review` to `designer_rename_property`. Apply requires the host's existing MCP write permission and rechecks the current source. One Undo rolls back the full transaction. [VS Code workflow](authoring.md#rename-a-registered-xaml-property) · [Studio workflow](../native-controls.md#review-a-registered-property-rename)

### Review a registered type rename with MCP

Use the registered type's position in XAML, including a resolved type operand in a style selector or a typed binding path. The reviewed documents include any mapped RSX type keys and Rust builder paths. Property and type reviews keep unresolved binding uses in `unmapped` for source inspection. [Typed binding examples](authoring.md#follow-registered-rust-declarations)

In VS Code, call `ferroui_workspace_plan_type_rename` with `path`, optional `folder`, a zero-based UTF-16 `position`, and `new_name`. The extension resolves any Rust symbol edits through its active rust-analyzer provider. Review the documents and `unmapped` locations before passing the same inputs and `review` to `ferroui_workspace_rename_type`.

In Studio, use `designer_plan_type_rename` with the active XAML byte `offset`, `new_name`, and `expected_revision`. For a name derived from Rust, start analysis with `allow_input` and poll with the same fields while the result reports `status: "resolving"`. A literal alias returns a plan directly. Use `cancel` to discard the pending review. Apply with `designer_rename_type` and the current `review` under the existing MCP write permission. [VS Code workflow](authoring.md#rename-a-registered-xaml-type) · [Studio workflow](../native-controls.md#review-a-registered-type-rename)

### Rename an RSX control with MCP

In VS Code, read `designer_outline` for the current control ID and frame. Call `designer_rename_rsx_control` with `session`, `node`, `expected_revision`, and the new name in `value`. The editor also exposes the same change as a `designer_property` action through `designer_studio_edit`.

In Studio, call `designer_property` with `action: "set"`, `property: "Name"`, the control ID in `nodes`, the current `expected_revision`, and the new name in `value`. Both paths require authoring permission and apply the declaration and referenced literals in one Undo step. [Rename workflow and supported references](authoring.md#rename-an-rsx-control)

### Move an XAML file with MCP

In VS Code, call `ferroui_workspace_plan_xaml_move` with `path` and `destination`. Use `folder` to disambiguate the root in a multi-root workspace. Inspect the proposed location and the `before`, `after`, and `edit` for each affected XAML file, Rust file, Cargo build, stage set, or scenario. Call `ferroui_workspace_move_xaml` with the same inputs and the returned `review`. The move requires `ferroui.mcp.allowWrites`.

When the Cargo build publishes generated XAML symbols, VS Code uses rust-analyzer to add the affected authored Rust references to the review. The `generatedRust` entries identify each declaration and its old and new names. Review the Rust source changes in `documents`. A stale buffer or an unresolved declaration requires a fresh review. [Build the project schema and move a compiled view](authoring.md#move-an-xaml-file-and-its-references)

Both planners include traceable `std::fs::read_to_string` inputs to `XamlGroup::documents`. Compare the filesystem path and any compiled key in the reviewed Cargo source before applying it. [Supported Cargo file reads](authoring.md#existing-cargo-groups-that-read-files)

For file-backed Cargo groups, both planners report the change to the Rust path literal in `documents`. The compiled key and generated entry points keep their names. Relative XAML includes with a mapped compiled target follow its source through the move. Build the selected Cargo target before asking for the plan. [Computed Cargo group setup](authoring.md#computed-cargo-groups-with-file-backed-registrations)

In Studio, open the source file and call `designer_plan_xaml_move` with `destination` and `expected_revision`. Send the same arguments and `review` to `designer_move_xaml` with write permission. Studio updates the saved stage sets and scenarios on Apply. Save the changed XAML and Rust buffers to publish their new references. Use Undo or Redo in a participating source tab to replay the move, including the saved files. [File move workflow](authoring.md#move-an-xaml-file-and-its-references)

For a generated Rust build function, build the owning Cargo package and make rust-analyzer available to Studio. Start `designer_plan_xaml_move` with `allow_input` so Studio can launch the analyzer. If the result has `status: "resolving"`, repeat the plan call with the same destination and revision until it returns `review` and `documents`. Pass `cancel: true` to discard that review. Apply uses the resolved edits and checks the current sources and revision again. [Studio setup](../native-controls.md#move-an-xaml-file)

Read the open board again after Apply, Undo, or Redo. Use `designer_stages` in VS Code or `designer_preview_stages` in Studio. A moved card keeps its stage ID and returns a new epoch for input, selection, and inspection.

### Read Preview Checks with MCP

Call `designer_studio_query` with `tool: "designer_preview_checks"` and empty `arguments` for the active design. On a stage, call `designer_stage_checks` with its current ID and epoch. `layoutOverflow` and `clippedLayout` include `overflow.boundary` and `left`, `top`, `right`, and `bottom` distances in logical pixels. `ineffectiveLayoutValue` identifies the authored `property` and explains the effective layout. `range` points to the original XAML or mapped Rust in UTF-16 coordinates.

Stage checks include `accessibility.tree`, `accessibility.tab_order`, and `accessibility.truncated`. Each row provides the native `name`, `role`, `enabled` and `offscreen` state, and a `range` when the control maps to authored source. Tab order follows one native traversal from the stage's entry point. Each list reports up to 512 rows. The `clippedText` finding identifies text cut by the available layout. Formatted ellipses keep their authored behavior.

In Designer Studio, `designer_preview_checks` reads the current canvas. `designer_preview_stage_checks` takes the current stage model. Studio reports `source` as a byte span in the authored document. [Preview Checks in VS Code](authoring.md#diagnose-a-design-time-issue)

### Edit one keyframe component with MCP

1. Read `designer_programs` through `designer_studio_query` with `include_keyframe_text: true`. Select the program, track, and an existing key with the channel in its `curve_values`. Designer Studio exposes `designer_programs` directly.
2. Pass the program ID, track `target` and `property`, existing key time as `previous`, and the desired `time` and `easing` to `designer_keyframe_draft`. Supply `component: { "channel": "top_right", "value": 24 }`. Copy the reported `easing_text` and keep `time` equal to `previous` to change just the value component.
3. Use the same draft with `action: "upsert_keyframe"` on `designer_program`. Use the current frame revision with VS Code’s `designer_studio_edit` or the current document revision in Studio. Each accepted edit takes one Undo.

For several fields on the same key, pass `components: [{"channel": "top_right", "value": 24}, {"channel": "bottom_left", "value": 12}]`. Supply one value representation per request. Use distinct channels from the original key. A batch can also change the key’s time and easing. The planner validates the whole draft before the edit.

For App Preview, read the authored key from `designer_hosted_animations`. Use its generation and accepted revision and sequence for `designer_hosted_keyframe_draft` and `designer_hosted_keyframe`. The draft returns a UTF-16 source patch and the complete `resolvedKey` from the running app’s codec. Units match the Value graph. Use logical pixels for absolute distances, percentage points for relative coordinates, degrees for rotation, and values from 0 to 1 for color channels. [Curve graph workflow](motion.md#edit-a-curve-component)

Use `components` in App Preview when the program’s `authoring.componentBatch` is true. The source plan for an existing native XAML key targets its authored setter and any compatible timing attributes. It checks the candidate with the same native codec before Apply. [Compound keyframe workflow](motion.md#edit-compound-keyframe-values)

For a Rust cue, spline coordinate, or typed value that references a mapped `f64` constant, the same keyframe draft edits the selected use. Apply writes a typed literal for the changed argument and keeps the shared declaration and sibling references intact. [Shared numeric key workflow](motion.md#edit-a-key-that-uses-a-shared-rust-number)

Use the same draft to edit a shared Rust ARGB integer, color channel, or typed `Color` at one key. A packed integer edit keeps its initializer's radix, grouping, and explicit suffix. A whole `Color` reference becomes an explicit color constructor at the selected use. [Shared color key workflow](motion.md#edit-a-key-that-uses-a-shared-rust-color)

For a registered shader, copy the exact channel from the key’s `curve_values`, for example `effect_parameter/spotlight/strength/value`. A Boolean handle reports `step: "boolean"` and accepts 0 or 1. An integer handle reports `step: "integer"` and accepts a signed 32-bit integer. App Preview resolves project-registered effects against the loaded application. [Registered shader workflow](motion.md#animate-registered-shader-parameters)

### Edit property transitions with MCP

From the current XAML or Rust canvas, read `designer_property_transitions` through `designer_studio_query` in VS Code. Pass the design `session` and `arguments: {"owner": 1}` using the node ID of the selected control, XAML style, theme, or authored collection. Studio calls `designer_property_transitions` directly. The response contains the transition `id`, authored ranges and values, `editableTiming`, and `canRetarget` for each row. Pass a registered `target_type` when `requiresTarget` is true and you need the compatible `properties` for a shared XAML collection.

Call `designer_property_transition_edit` with `action: "timing"`, the same `owner`, the chosen `transition` ID, and a `timing` object. A single call can pass `{"duration": 0.55, "delay": 0.1, "easing": {"kind": "Named", "value": "CubicEaseOut"}}`. Omitted fields keep their authored values. Studio also takes the current `expected_revision`. In VS Code, pass the fields as `arguments` to `designer_studio_edit`, with the design `session` and `expected_revision` from the current outline at the outer level. Use one Undo to restore all fields from that call.

To add an entry, pass `action: "add"` and the chosen `property` and `type_name` from `properties`, with optional `timing`. For `action: "retarget"`, pass the same choice and the selected `transition` ID. Include `target_type` in either call when the collection requires it. Move an entry with `action: "move"`, a `transition` ID, and a zero-based destination `index`. Use `action: "remove"` and a `transition` ID to delete it. Read the accepted revision and collection again before planning another edit.

For an inline Rust collection, use the exact getter path from the `property` field of the chosen property option. Supply duration in seconds with millisecond precision and choose an applicable easing. Read the entry’s Rust source range from the same accepted view.

[Property transition workflow](motion.md#edit-property-transitions)

### Edit App Preview property transitions with MCP

1. Open the app with `designer_hosted_open`. Read its `generation`, accepted `frame`, and `propertyTransitionOwners` with `designer_hosted_animations`. Use an owner ID from that response.
2. Call `designer_hosted_property_transitions` with the design `session`, `expected_generation`, `expected_revision`, and `expected_sequence`. Set `arguments` to `{ "owner": 1 }` using the current owner ID. Supply `target_type` in `arguments` if a shared XAML collection needs a target to resolve compatible properties.
3. Use `designer_hosted_property_transition_draft` with the same fields and add `command` to `arguments`. For example, `"command": { "action": "add", "property": "Emphasis", "type_name": "DoubleTransition", "timing": { "duration": 0.6 } }` plans the edit against Todo Workbench’s live registry. Drafts return checked UTF-16 source patches.
4. Call `designer_hosted_property_transition` with the same accepted frame and arguments to apply one undoable edit. Use the `timing`, `retarget`, `move`, and `remove` commands with the reported entry IDs. For Rust, take the getter path from the property choice. Read the new frame and collection after each change.

[App Preview transition workflow](motion.md#edit-property-transitions-in-app-preview)

### Edit and preview behaviors with MCP

1. Open an XAML or mapped RSX designer. Use `designer_outline` to find the control and `designer_studio_query` with `tool: "designer_behaviors"` and `arguments: { owner }`. The report includes attached triggers, `actionRows` in source order, UTF-16 ranges, editing capabilities, and the active preview ID.
2. Use `designer_studio_edit` with the current frame revision and `tool: "designer_behavior"`. Use `upsert` to attach a complete trigger and `set_trigger` to change its fields. Use `append_action`, `set_action`, `move_action`, or `remove_action` for a single step. The move index is zero-based in the resulting action order. Read `designer_studio_tools` for the typed values and the `source` and `remove` actions.
3. Call `designer_studio_present` with `tool: "designer_behavior"`, `action: "preview"`, and the behavior ID. Read the new frame before calling `clear_preview`. These operations execute or reset native actions on the isolated Design canvas without editing the buffer. Use `designer_view` with `view: "behaviors"` to open the dock.

On Studio's MCP server, call `designer_behaviors` and `designer_behavior` directly. Supply the document revision for editing and preview. Running the trigger requires the server's `allow_input` permission. Call `designer_state_actions` to build the actions for a named visual state, then pass them when attaching a trigger. [Behavior workflow](authoring.md#edit-a-behavior)

For Rust, pass simple data and command paths and literal names for optional targets. Use the mapped authoring actions from the current Rust catalog. The `source` edit applies to XAML subtrees. Use the returned ranges to navigate a Rust declaration. [Rust behavior workflow](authoring.md#edit-a-behavior-in-rust)

### Link Rust events to states with MCP

1. Find the Rust event source with `designer_outline`. Call `designer_studio_query` with `tool: "designer_states"` and `arguments: { event_source: node }`. Read `stateTriggers.events`, `groups`, and the current `triggers`. Each trigger carries its UTF-16 range in the Rust buffer.
2. Call `designer_studio_edit` with the accepted revision and `tool: "designer_state"`. Pass `action: "set_event_trigger"`, `event_source`, `event`, `group`, and `state`. To remove that event and group link, pass `state: null`. One edit covers the whole link.
3. Switch the canvas to Interact and send input using the current preview frame. Read the result with `designer_inspect` or `designer_preview_image`.

On Studio’s MCP server, call `designer_states` and `designer_state` directly using the current document revision. [Rust state trigger workflow](motion.md#link-a-rust-event-to-a-state)

## Find a command

Commands are under **FerroUI** in the Command Palette. Some actions also appear in the canvas, a dock’s title menu, or the Project view.

| Task | Commands |
| --- | --- |
| Open or position the design | **Open Designer**, **Reveal Selected Control in Source**, **Arrange Selection**, **Wrap Selection**, **Edit Grid Tracks** |
| Set up the source project | **New Project**, **Check Project Toolchain**, **Restart XAML Language Server**, **Build Project XAML Schema**, **Refresh Project**, **Find Project View**, **New View**, **New Component**, **Create Rust Event Handler**, **Rename RSX Control** |
| Open editing views | **Show Toolbox**, **Show Outline**, **Show Properties**, **Edit Columns**, **Edit Item Templates**, **Edit Control Template**, **Show Events**, **Edit Behaviors**, **Edit Command Binding**, **Show Resources**, **Show Styles & Themes**, **Show Assets**, **Show Studio Tools**, **Show Library**, **Find Definition in Designer** |
| Extract an inline XAML dictionary | **Extract Resource Dictionary…**, **Review New Dictionary**, **Apply Extraction**, **Cancel Extraction** |
| Review and move an XAML file | **Move XAML File…**, **Apply File Move**, **Cancel File Move** |
| Review variations | **Show Animation Timeline**, **Show Design Data**, **Open Preview Stages** |
| Exchange tokens | **Import Design Tokens**, **Export Design Tokens** |
| Resolve component mappings and transfer assets | **Review Design Handoff** |
| Preview and inspect | **Open App-Hosted Design**, **Open Live App**, **Refresh Diagnostics** |
| Run or debug | **Run with Hot Reload**, **Run with Registered Rust Patches**, **Run and Debug with Hot Reload**, **Rebuild Hot Reload Application**, **Attach Debugger to Managed App**, **Show Hot Reload Output**, **Stop Hot Reload Application** |
| Manage an external MCP connection | **Connect to Running MCP Server**, **Disconnect Running MCP Server** |

## Configure the workspace

| Setting | Default | When to use it |
| --- | --- | --- |
| `ferroui.preview.path` | Empty | Supply an absolute path to a custom `ferroui-designer-preview` binary. The extension uses the bundled or PATH executable by default. |
| `ferroui.languageServer.path` | Empty | Supply an absolute path to a custom `ferroui-xaml-lsp`. The extension uses the bundled or PATH executable by default. |
| `ferroui.types` | `[]` | Add or override XAML metadata in the selected workspace. |
| `ferroui.rsx.completion` | `true` | Control supplemental FerroUI property completion inside mapped Rust spans. |
| `ferroui.rsx.inlayHints` | `false` | Display types on mapped RSX fields. |
| `ferroui.hostedPreview.assembly` | Empty | Name the application’s markup assembly when its name differs from the selected Cargo package. |
| `ferroui.hotReload.targets` | `[]` | Offer chosen Cargo binaries and set their package, manifest, features, and arguments. An empty list discovers targets from Cargo. |
| `ferroui.hotReload.cargoPath` | Empty | Choose a specific Cargo executable on the workspace host. |
| `ferroui.hotReload.registeredSupervisor` | `scripts/hot-reload/supervisor.py` | Choose a workspace-relative script for registered Rust patching and state handoff. |
| `ferroui.hotReload.pythonPath` | Empty | Select the Python executable for that supervisor. |
| `ferroui.hotReload.patchCompilerPath` | Empty | Select the pinned `dx` compiler for registered Rust patches. |
| `ferroui.mcp.enableDesigner` | `true` | Control discovery of the editor MCP server. |
| `ferroui.mcp.allowWrites` | `false` | Allow agent writes, designer controls, and Cargo actions in the trusted editor. |
| `ferroui.mcp.stdioServers` | `[]` | Configure a known server with `label`, absolute `command`, and optional `args`, such as `--mcp-stdio`. |

This workspace settings example selects the app-hosted Todo Workbench target from the FerroUI repository root. `manifestPath` is relative to the workspace folder.

```json
{
  "ferroui.hotReload.targets": [
    {
      "label": "Todo Workbench with App Preview",
      "manifestPath": "samples/TodoWorkbench/Cargo.toml",
      "package": "todo-workbench",
      "binary": "todo-workbench",
      "features": ["app-preview"]
    }
  ]
}
```

For a missing command or helper, see [installation and troubleshooting](README.md#if-the-designer-does-not-open). For an app that cannot attach or apply an edit, see [Run and test a view](runtime.md).

## Review and approve a scenario with MCP

1. Call `ferroui_scenario_review` with the saved scenario `path`. In VS Code, use `folder` to select a workspace root when the path is ambiguous. Pass `baseline_ref` to compare its Git merge-base images. Set `open` to show the comparison in the editor.
2. Read the assertion results and each item's `workspace` and `comparison` status in `visuals`. Inspect the `expected`, `actual`, and `difference` PNGs for the selected step. An available `approval` identifies the exact capture and workspace baseline from the review.
3. Call `ferroui_scenario_approve_baseline` with the returned `approval` and the same scenario path and Git reference. VS Code requires `ferroui.mcp.allowWrites` and Studio requires write permission. A missing or stale identifier requires a fresh review. Call `ferroui_scenario_run` to verify against the accepted image.

Studio's `ferroui_scenario_list` discovers saved scenarios for the open project. Its run, review, and approval calls start the helper asynchronously and return a `run` number. Pass it to `ferroui_scenario_result` for status and the completed report. `ferroui_scenario_cancel` stops the current owned run. VS Code returns the completed report directly. [Scenario review and CI](testing.md)
