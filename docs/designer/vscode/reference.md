---
title: "Use agents and configure FerroUI in VS Code"
description: "Connect the editor, Studio, and application MCP servers, and look up commands and settings."
---

## Start the right MCP server

1. Open a trusted workspace in desktop VS Code. Run **MCP: List Servers** and start **FerroUI Designer in VS Code**. Choose its tools in your VS Code agent.
2. Ask the agent to read the active unsaved document or open a design and describe its accepted native frame. To permit source edits, designer actions, or managed Cargo commands, enable `ferroui.mcp.allowWrites` in VS Code user settings. Editor write access is off by default.
3. For Studio or an independently running application, use **FerroUI: Connect to Running MCP Server** and supply the authenticated loopback endpoint. **Disconnect Running MCP Server** removes that connection. Configure `ferroui.mcp.stdioServers` for a known executable using its native stdio transport.
4. Start a configured app through **Run with Hot Reload** to discover its **FerroUI Managed Application** server when the app advertises one. Its runtime inspection and input permissions come from the app. An in-place update keeps the connection. A replacement process advertises its own server after startup.

| Server or scope | What to use it for |
| --- | --- |
| **FerroUI Designer in VS Code** | VS Code’s open buffers, project files and language providers, design selection and views, authoring in the designer, Stages, and managing Cargo. |
| **Designer Studio** attached from VS Code | Studio’s open documents, native project actions, stage sets, animation and layout tools, and collaboration features of the Studio session. |
| **FerroUI Managed Application** or attached app | Actual native windows, properties, binding and layout diagnostics, accessibility, screenshots, and permitted input. |

Editor and Studio authoring reuse the same native planning and validation. The editor applies source changes through VS Code’s undo history. Studio applies them through its document history. Use the server owning the unsaved buffer you intend to edit. [VS Code MCP setup](https://code.visualstudio.com/docs/copilot/customization/mcp-servers) · [Designer Studio MCP](../README.md#mcp-integration) · [Application MCP](../../mcp.md)

## Drive an editor workflow

| Task | Tools |
| --- | --- |
| Find source and language information | `ferroui_project_files`, `ferroui_workspace_files`, `ferroui_workspace_read`, and `ferroui_workspace_language`. Edits and file actions use `ferroui_workspace_create`, `ferroui_workspace_edit`, and `ferroui_workspace_file`. |
| Open and inspect a design | `designer_open`, `designer_block`, and `designer_wait` choose the view and wait for an accepted frame. Use `designer_canvas`, `designer_view`, and `designer_preview_image` for its mode, display, workbench views, and pixels. `designer_canvas_input` sends native interactions. |
| Measure and mark the canvas | Read the current scope, ruler settings, coordinates, `revision`, and `epoch` with `designer_canvas_guides`. Pass `session`, `expected_revision`, `expected_epoch`, and `edit` to `designer_canvas_guides_edit`. Use `visibility`, `add`, `move`, `duplicate`, `remove`, `clear`, or `apply` as `edit.action`. For an exact batch, `apply` takes `rulers`, `guides`, and `lines` with `axis` (`Horizontal` or `Vertical`), `position`, and an optional existing `id`. [Canvas guide workflow](authoring.md#measure-and-mark-the-canvas) |
| Snap and place geometry | Use `configure_snap` in `designer_canvas_guides_edit` with `options` containing `grid_visible`, `grid_spacing`, `snap_to_grid`, `snap_to_guides`, `snap_to_objects`, and `snap_tolerance`. For a Canvas placement or resize, pass `snap: { epoch, zoom }` where `zoom` is the canvas scale such as `1` for 100%. Add `bypass: true` for a precise gesture. Preview with `designer_placement_hint` or `designer_resize_hint`, then pass the same unadjusted coordinates and snap context to `designer_place` or `designer_resize`. [Canvas snapping workflow](authoring.md#snap-controls-to-guides-the-grid-or-other-controls) |
| Plan layout and source edits | Use `designer_placement_hint` before `designer_place`, and read the current properties, events, or resources before editing them. `designer_apply_counterpart` applies a supported alternate source. Discover native action schemas with `designer_studio_tools`. Then use `designer_studio_query`, `designer_studio_edit`, or `designer_studio_present` to read, author, or change preview-local state. |
| Inspect or edit motion | Read programs and tracks with `designer_programs` through `designer_studio_query`. Use `designer_timeline_control` or `designer_transition_control` for playback and seeks. Use `designer_animation_curve` to read values and `designer_keyframe_draft` to validate a key. Commit using `designer_program` through `designer_studio_edit`. Its `place_clips` action accepts ordered program IDs and optional start and gap in seconds. Use `scrub_all` for the shared clip ruler. [Sequence clips](motion.md#compare-and-sequence-clips) |
| Compare or play stages | `designer_stages_open`, `designer_stages`, `designer_stage_image`, `designer_stage_checks`, and `designer_stage_hotspots` read the board. `designer_stage_change` and `designer_stage_prototype` configure and play it. |
| Render with the app registry | `designer_hosted_open`, `designer_hosted_fixtures`, `designer_hosted_fixture`, and `designer_hosted_image` operate App Preview. Read its animation frame and generation with `designer_hosted_animations`. Query and edit motion with the animation, curve, and keyframe tools. Use `designer_hosted_keyframe_group_draft` and `designer_hosted_keyframe_group` for a key selection, `designer_hosted_clip_draft` and `designer_hosted_clip_edit` for whole clips, and `designer_hosted_state_group_draft` and `designer_hosted_state_group` for visual states. |
| Sequence app-hosted clips | Use `scrub_all` and `play_all` in `designer_hosted_animation` for the app’s shared elapsed clock. Use `designer_hosted_clip_placement_draft` to check a delay change and `designer_hosted_clip_placement` to apply it to the current buffer in one Undo step. |
| Rehearse an app-hosted range | Use `play_range` in `designer_hosted_animation` with an elapsed `start`, `end`, and `loop` in `range`. Include `program` to rehearse one animation or omit it to rehearse the shared clip clock. |
| Set, remove, or seek to a preview marker | Use `designer_timeline_markers` and `designer_timeline_marker_edit` for a typed clip or the shared Timeline clock. In App Preview, read `markers` from `designer_hosted_animations` and pass `action: "marker"` to `designer_hosted_animation`. [Marker controls and arguments](motion.md#bookmark-preview-positions) |
| Run, inspect, and test | `ferroui_reload_targets`, `ferroui_reload_control`, `ferroui_reload_status`, `ferroui_reload_output`, and `ferroui_reload_history` drive managed Cargo and report outcomes. `ferroui_runtime_source` and `ferroui_runtime_apply_source` work with a mapped runtime value. `ferroui_scenario_run` runs a saved native scenario. |

For example, ask an agent to open the current design, inspect its animation programs, and seek to one keyframe. It should read the active document and RSX invocation, wait for the accepted frame, read the program ID and its keys, then call the native seek with the current frame. Ask it to capture the resulting frame or report the current position. To update a key, validate the time, value, and easing together before applying the edit.

Read the new buffer revision, frame token, stage epoch, or connection generation after the corresponding change. Text locations are UTF-16. A stale source edit is rejected. Re-read the current state before retrying the edit. The advertised tool schemas specify which identity each tool needs.

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

### Read Preview Checks with MCP

Call `designer_studio_query` with `tool: "designer_preview_checks"` and empty `arguments` for the active design. On a stage, call `designer_stage_checks` with its current ID and epoch. `layoutOverflow` and `clippedLayout` include `overflow.boundary` and `left`, `top`, `right`, and `bottom` distances in logical pixels. `ineffectiveLayoutValue` identifies the authored `property` and explains the effective layout. `range` points to the original XAML or mapped Rust in UTF-16 coordinates.

In Designer Studio, `designer_preview_checks` reads the current canvas. `designer_preview_stage_checks` takes the current stage model. Studio reports `source` as a byte span in the authored document. [Preview Checks in VS Code](authoring.md#diagnose-a-design-time-issue)

## Find a command

Commands are under **FerroUI** in the Command Palette. Some actions also appear in the canvas, a dock’s title menu, or the Project view.

| Task | Commands |
| --- | --- |
| Open or position the design | **Open Designer**, **Reveal Selected Control in Source**, **Arrange Selection**, **Wrap Selection**, **Edit Grid Tracks** |
| Set up the source project | **Restart XAML Language Server**, **Build Project XAML Schema**, **Refresh Project**, **Find Project View**, **New View**, **New Component**, **Rename RSX Control** |
| Open editing views | **Show Toolbox**, **Show Outline**, **Show Properties**, **Edit Columns**, **Show Events**, **Show Resources**, **Show Assets**, **Show Studio Tools**, **Show Library**, **Find Definition in Designer** |
| Review variations | **Show Animation Timeline**, **Show Design Data**, **Open Preview Stages** |
| Exchange tokens | **Import Design Tokens**, **Export Design Tokens** |
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
