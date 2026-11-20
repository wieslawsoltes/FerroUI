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
| Undo or redo one editor transaction | Use `designer_history` with `session`, `action`, and `expected_version`. For a workspace file, use `ferroui_workspace_file` with `action` and the `expected_revision` from `ferroui_workspace_read`. VS Code uses the existing source editor group when available, so the adjacent Design tab stays visible. If the source does not change and the file stays open, the tool reports that the requested history step did not apply. |
| Open and inspect a design | `designer_open`, `designer_block`, and `designer_wait` choose the view and wait for an accepted frame. Use `designer_canvas`, `designer_view`, and `designer_preview_image` for its mode, display, workbench views, and pixels. `designer_canvas_input` sends native interactions. |
| Measure and mark the canvas | Read the current scope, ruler settings, coordinates, `revision`, and `epoch` with `designer_canvas_guides`. Pass `session`, `expected_revision`, `expected_epoch`, and `edit` to `designer_canvas_guides_edit`. Use `visibility`, `add`, `move`, `duplicate`, `remove`, `clear`, or `apply` as `edit.action`. For an exact batch, `apply` takes `rulers`, `guides`, and `lines` with `axis` (`Horizontal` or `Vertical`), `position`, and an optional existing `id`. [Canvas guide workflow](authoring.md#measure-and-mark-the-canvas) |
| Snap and place geometry | Use `configure_snap` in `designer_canvas_guides_edit` with `options` containing `grid_visible`, `grid_spacing`, `snap_to_grid`, `snap_to_guides`, `snap_to_objects`, and `snap_tolerance`. For a Canvas placement or resize, pass `snap: { epoch, zoom }` where `zoom` is the canvas scale such as `1` for 100%. Add `bypass: true` for a precise gesture. Preview with `designer_placement_hint` or `designer_resize_hint`, then pass the same unadjusted coordinates and snap context to `designer_place` or `designer_resize`. [Canvas snapping workflow](authoring.md#snap-controls-to-guides-the-grid-or-other-controls) |
| Plan layout and source edits | Use `designer_placement_hint` before `designer_place`, and read the current properties, events, or resources before editing them. `designer_apply_counterpart` applies a supported alternate source. Discover native action schemas with `designer_studio_tools`. Then use `designer_studio_query`, `designer_studio_edit`, or `designer_studio_present` to read, author, or change preview-local state. |
| Inspect or edit motion | Read programs and tracks with `designer_programs` through `designer_studio_query`. Use `designer_timeline_control` or `designer_transition_control` for playback and seeks. Use `designer_animation_curve` to read values and `designer_keyframe_draft` to validate a key. Commit using `designer_program` through `designer_studio_edit`. Its `place_clips` action accepts ordered program IDs and optional start and gap in seconds. Use `scrub_all` for the shared clip ruler. [Sequence clips](motion.md#compare-and-sequence-clips) |
| Compare or play stages | `designer_stages_open`, `designer_stages`, `designer_stage_image`, `designer_stage_checks`, and `designer_stage_hotspots` read the board. `designer_stage_change` configures it. `designer_stage_input` sends keyboard input to a card. `designer_stage_prototype` drives a flow. |
| Render with the app registry | `designer_hosted_open`, `designer_hosted_fixtures`, `designer_hosted_fixture`, and `designer_hosted_image` operate App Preview. Read its animation frame and generation with `designer_hosted_animations`. Query and edit motion with the animation, curve, and keyframe tools. Use `designer_hosted_keyframe_group_draft` and `designer_hosted_keyframe_group` for a key selection, `designer_hosted_clip_draft` and `designer_hosted_clip_edit` for whole clips, and `designer_hosted_state_group_draft` and `designer_hosted_state_group` for visual states. |
| Sequence app-hosted clips | Use `scrub_all` and `play_all` in `designer_hosted_animation` for the app’s shared elapsed clock. Use `designer_hosted_clip_placement_draft` to check a delay change and `designer_hosted_clip_placement` to apply it to the current buffer in one Undo step. |
| Rehearse an app-hosted range | Use `play_range` in `designer_hosted_animation` with an elapsed `start`, `end`, and `loop` in `range`. Include `program` to rehearse one animation or omit it to rehearse the shared clip clock. |
| Set, remove, or seek to a preview marker | Use `designer_timeline_markers` and `designer_timeline_marker_edit` for a typed clip or the shared Timeline clock. In App Preview, read `markers` from `designer_hosted_animations` and pass `action: "marker"` to `designer_hosted_animation`. [Marker controls and arguments](motion.md#bookmark-preview-positions) |
| Run, inspect, and test | `ferroui_reload_targets`, `ferroui_reload_control`, `ferroui_reload_status`, `ferroui_reload_output`, and `ferroui_reload_history` drive managed Cargo and report outcomes. `ferroui_runtime_source` and `ferroui_runtime_apply_source` work with a mapped runtime value. `ferroui_scenario_run` runs a saved native scenario. |

For example, ask an agent to open the current design, inspect its animation programs, and seek to one keyframe. It should read the active document and RSX invocation, wait for the accepted frame, read the program ID and its keys, then call the native seek with the current frame. Ask it to capture the resulting frame or report the current position. To update a key, validate the time, value, and easing together before applying the edit.

Read the new buffer revision, frame token, stage epoch, or connection generation after the corresponding change. Text locations are UTF-16. A stale source edit is rejected. Re-read the current state before retrying the edit. The advertised tool schemas specify which identity each tool needs.

### Send keyboard input to a stage

1. Read the board with `designer_stages`. Use `designer_stage_change` to set the card's mode to `live`. Read the accepted stage ID and epoch from the returned board.
2. Call `designer_stage_input` with `session`, `stage`, `expected_epoch`, and `input: { kind: "focus" }`. The result gives the focused control's `name` and `role`, or `null` if there is no focusable control.
3. Pass `input: { kind: "key", key: "Tab" }` or `"Shift+Tab"` to navigate, `"Enter"` or `"Space"` to activate a button, or `input: { kind: "text", text: "Acme" }` to enter text. Use `designer_stage_image` to read the resulting native frame.

Stage input requires `ferroui.mcp.allowWrites` and an updated preview helper. It changes the card's temporary interaction state. In Designer Studio use `designer_preview_stage_input` with its current `expected_model` and stage epoch. [Studio stage automation](../xcode-previews.md#automation)

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

### Edit item templates with MCP

1. Call `designer_wait` and use `designer_outline` to find the authored control or column. Read `designer_item_templates` through `designer_studio_query` with `arguments: { node }`. The result includes the current model, property names, local definitions, source ranges, and visible template resources.
2. Open the native definition preview with `designer_studio_present` and `tool: "designer_item_template"`. Pass the current editor frame as `expected_revision` and use `arguments: { action: "preview", node, property: "ItemTemplate", expected_model: model }`. Exit with the `designer_definition` presentation action `exit`.
3. Use `designer_studio_edit` with the same tool and model fields for `create_inline`, `copy_inline`, `assign_resource`, or `reset`. For assignment, pass the selected template ID as `resource`. Read a fresh model after a change. Pass the same `selection` to the query and the action when specifying it explicitly.

On Studio’s server, call `designer_item_templates` and `designer_item_template` directly. For `expected_revision`, pass the Studio document revision. [Authoring guide](authoring.md#edit-item-and-cell-templates)

### Edit styles and palettes with MCP

Use these tools directly in Designer Studio. In VS Code, pass the tool name and arguments through `designer_studio_query` for reads or `designer_studio_edit` for writes. Pass the current editor revision to VS Code and the current document revision to a direct Studio write.

| Tool | Arguments and result |
| --- | --- |
| `designer_style_catalog` | Optional `target`. Lists authored styles and themes, eligible hosts, and named palette scopes. |
| `designer_style_definition` | `node`. Reads the selector or base, typed setter choices, authored values, and exact source ranges. |
| `designer_style_resources` | `owner` and `property`. Lists compatible resources visible from the setter's current native scope. |
| `designer_style_edit` | `create` uses `owner` and `selector`. `selector` uses `node` and `selector`. `base` uses `node` and a compatible `base` ID or null. `setter` uses `owner`, `property`, and `value` or null to remove. `resource` uses `owner`, `property`, `resource`, and `dynamic`. `remove` uses an unkeyed style's `node`. Use `designer_component create_theme` to create a keyed control theme. |
| `designer_palette` | `owner`. Returns authored colors by layer and resolution of each key in Light and Dark. |
| `designer_palette_edit` | `set` uses a literal color's `node` and a native `color` string. `add` uses `owner`, `layer` (`shared`, `default`, `light`, or `dark`), `key`, `kind` (`color` or `solid_color_brush`), and `color`. |

Use `designer_view` with `view: "styles"` to focus the dock. Set `dark` to change the canvas environment and wait for the new frame with `designer_wait`. Use `designer_definition enter` through `designer_studio_present` to preview a definition. [Styles & Themes guide](styles.md)

### Trace property values with MCP

Call `designer_studio_query` with `tool: "designer_property_trace"` and `arguments: { node, property }`. Use a current source node from `designer_outline`. For an object that exists only in the preview, read `designer_tree` through `designer_studio_query` and pass its inspector ID as `runtime_node` in place of `node`. The report includes the effective value, authored value and referenced resource where available, ordered contributors, and UTF-16 source ranges for mapped declarations. In Designer Studio, call `designer_property_trace` directly with the same arguments.

On the running app’s MCP connection, call `get_property_trace` with `node_id` and `property`. Contributor locations use the runtime’s one-based source coordinates and revision fingerprint when available. Read the trace again after interacting with the preview or the app. [Authoring guide](authoring.md#trace-an-effective-property-value)

### Create an event handler with MCP

Build the package’s XAML schema. Call `designer_event_handler` with the current `expected_revision`, `node`, and authored event name such as `Click`. In VS Code, also pass the `session` from `designer_open`. The result describes the Rust owner file, event signature, suggested name, and `rustVersion`. Pass a `handler` to preview the two versioned source edits. The `declaration` in the preview points to the new handler name after the patch.

Call `designer_scaffold_handler` with the same target, `handler`, and `expected_rust_version` from the read. The request requires write access. It updates the workspace buffers as one edit and returns the declaration location. Use `designer_history` in VS Code or `designer_undo` in Studio to undo the change. Save and rebuild when the handler is ready to run. [Events workflow](authoring.md#create-a-rust-handler-for-an-xaml-event)

### Extract a resource dictionary with MCP

Call `designer_dictionary_extraction` with an authored `node` and the current `expected_revision`. Use the `session` from `designer_open` in VS Code. The result includes the source range, entry count, and suggested sibling filename. Pass `file_name` to preview the versioned `edit`, the `file` to create, and the `review` identifier.

Send `designer_extract_dictionary` with the same `node`, `expected_revision`, `file_name`, and `review`. The write creates the dictionary and updates the view in one editor transaction. Use `designer_history` in VS Code or `designer_undo` in Studio from the original view. When a source or destination changes, read the target again and review the new plan. [Resources workflow](authoring.md#extract-a-resource-dictionary)

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

In Designer Studio, `designer_preview_checks` reads the current canvas. `designer_preview_stage_checks` takes the current stage model. Studio reports `source` as a byte span in the authored document. [Preview Checks in VS Code](authoring.md#diagnose-a-design-time-issue)

## Find a command

Commands are under **FerroUI** in the Command Palette. Some actions also appear in the canvas, a dock’s title menu, or the Project view.

| Task | Commands |
| --- | --- |
| Open or position the design | **Open Designer**, **Reveal Selected Control in Source**, **Arrange Selection**, **Wrap Selection**, **Edit Grid Tracks** |
| Set up the source project | **Restart XAML Language Server**, **Build Project XAML Schema**, **Refresh Project**, **Find Project View**, **New View**, **New Component**, **Create Rust Event Handler**, **Rename RSX Control** |
| Open editing views | **Show Toolbox**, **Show Outline**, **Show Properties**, **Edit Columns**, **Edit Item Templates**, **Show Events**, **Show Resources**, **Show Styles & Themes**, **Show Assets**, **Show Studio Tools**, **Show Library**, **Find Definition in Designer** |
| Extract an inline XAML dictionary | **Extract Resource Dictionary…**, **Review New Dictionary**, **Apply Extraction**, **Cancel Extraction** |
| Review and move an XAML file | **Move XAML File…**, **Apply File Move**, **Cancel File Move** |
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
