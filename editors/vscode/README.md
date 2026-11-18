# FerroUI Designer for VS Code

Open a FerroUI `.xaml` file or a Rust file containing `ferroui_view::rsx!`, then choose **FerroUI: Open Designer** from the editor toolbar or Command Palette. The designer opens beside the text editor and renders its unsaved buffer. When VS Code restores a designer tab, it reopens its source, preview dimensions, theme, and selected RSX invocation.

## Working in the designer

- **Toolbox, Outline, Properties, Events, Resources, and Assets** are independent VS Code views. Open them from the canvas toolbar or the Command Palette. Use **View: Move View** to keep Tools and Outline beside the editor and put Properties in the Secondary Side Bar. **View: Toggle Maximize Editor Group** gives the canvas the space of both editor groups. [VS Code custom layout](https://code.visualstudio.com/docs/configure/custom-layout)
- **Toolbox** shares Designer Studio’s 20 built-in controls, layouts, and shapes. After a schema or application build, **Project controls** and **Dependency controls** show the constructible controls exported by that Cargo target. Add single-root XAML snippets to `.ferroui/toolbox` at the workspace root or in a Cargo package. The active package can replace a workspace snippet with the same filename. Mapped controls and compatible snippets also appear when editing RSX. Search by name, category, or description and switch between lists and tiles. Drag a control onto the canvas or Outline. Press Enter to insert into the selected container or its nearest parent with room. Shift+Enter opens the placements allowed before, inside, and after the selected control.
- **Placement** uses the same layout planners as Studio. The canvas shows the insertion location and resolves occupied content hosts through an eligible ancestor or the layout body inside a wrapper. In Source Outline, the top and bottom of a row insert before and after it. Drop in the middle of an available container to nest. Use `Alt` + `↑` or `↓` for keyboard reordering. Canvas placement uses the measured layout, including grid cells and Canvas coordinates. One drop makes one source edit that VS Code can undo.
- **Outline** offers Source, Visual, Logical, and Combined. The combined hierarchy keeps visual nesting and adds logical-only objects under their logical parent. Select a runtime-only template part to inspect its effective properties and see its bounds. Source mode supports structural edits.
- **Properties** displays measured X, Y, width, and height alongside editable size, layout, content, and appearance fields supported by the selected control. Search the additional property groups for less common settings. Reset removes an authored literal so the default or style can take effect. Select a single XAML control and use **Edit binding** on a supported property to stage a path, source, mode, update trigger, delay, converter, or fallback. Named element choices come from the current namescope. **Apply binding** writes the changed fields in one undo step and keeps the other arguments as authored. Source navigation is available for bindings and Rust expressions.
- Use **Edit brush** on a XAML appearance property to adjust a solid, linear, radial, or conic brush. Set colors, geometry, and opacity, and add, reorder, or remove gradient stops. **Apply Brush** commits the draft and refreshes the canvas in one undo step. Agents can draft the same changes with `designer_brush_draft` through `designer_studio_query`, then apply the returned property source through `designer_studio_edit`.
- **Events** lists routed events and assigned handlers for the selected control. Assign or clear a XAML code-behind name or edit an RSX handler expression. Use the reveal action to jump to its declaration in source. Application callbacks run through the application host.
- **Resources** lists authored XAML definitions with scope and usage counts. Filter to the definitions accessible to the selection, reveal a declaration, or apply a compatible static or dynamic resource reference. Use the **Promote to resource** action on supported authored Properties values to choose a scope and key. Studio creates the resource and replaces the literal in one undo step.
- **Assets** browses workspace images, fonts, media, and XAML files. Preview raster images, open files, and copy relative paths. Drag a raster image to the canvas or Source Outline in XAML or a mapped RSX container. Use **Choose image placement…** or Shift+Enter on an image row to choose an allowed position beside or inside the selected control. XAML uses a document-relative URI. RSX inserts an Image with `ferroui_view::embedded_image` and `include_bytes!`. The image path is relative to the Rust file, and changes to embedded bytes take effect on the next Cargo build.
- **Selection** follows the focused designer or its source editor. Use `Ctrl`/`Cmd` + click to add or remove controls in the canvas or Source Outline. With the canvas focused, press the up and down arrows to move through authored controls. Use `Alt` + click to cycle through overlapping controls. Drag a resize handle or focus it and use the arrow keys. Properties shows common fields and mixed values. Drag a selected item to move the controls together, or use **Arrange Selection** for native alignment, distribution, size matching, and reset. The first selected control is the reference for size matching. Double-click a control or press Enter to reveal its declaration. A successful source action is one VS Code undo step.
- Choose **Focus Order Overlay** from the canvas actions menu to see native group and tab ranks and label connections in Select mode. Use Tab to reach a badge and Enter to reveal its authored control. Edit the order and label targets in **Studio → Focus and Labels**. Agents can show the overlay with `designer_canvas` and request its measured geometry from `designer_focus_order` through `designer_studio_query`.
- **Grid Tracks** in Studio Tools reads the nearest authored Grid. Choose a row or column to set an Auto, pixel, or star length, insert a track, or remove one. A structural change adjusts measured child cells and spans in the same undo step. Use **Edit Grid Tracks** from the Command Palette or the `designer_grid_tracks` and `designer_grid_track_edit` Studio tools over MCP. The native planner validates XAML and fully mapped RSX Grids before applying a patch.
- **Interact** sends clicks and scrolling to the native canvas. Click a focusable control, then use the input bar to send text or a key gesture. Preview interaction is local to the mounted design, so it adds no source edits. Choose **Select** to resume picking, moving, and resizing authored controls.
- **Viewport** opens from the size button. Choose a device preset, type a logical width and height, swap the dimensions, or switch the theme. **Fit to view**, **Fit selection**, and **Actual size** are available from the zoom menu. With the canvas focused, use `0` to fit, `1` for actual size, and `+` or `−` to step the zoom. `Ctrl`/`Cmd` + scroll zooms at the pointer. Pan by dragging with Space held, using the middle button, or switching on the pan tool.
- **Alternate source** is available on the **XAML** and **RSX** tabs. For Rust files with several macros, place the caret inside the view you want or use the RSX picker. Apply the alternate source to update the authored buffer. If the editor changes while a projected draft is open, the draft stays visible and Apply waits for a reload of the projection.
- Alternate RSX literal edits preserve authored XAML design values and fixtures. Use the XAML editor for structural changes to those documents.
- **Studio** opens with **Show Studio Tools**. Browse definitions, create or rename resources, set XAML design values and sample counts, and arrange focus and labels. Add state groups, setters, and transitions. Attach routed-event or data-condition behaviors and configure their actions. Use Timeline to add property tracks and edit a keyframe’s time, value, and easing together, or seek the native preview. Layer visibility, locking, and isolation are local to the preview session.
- **Library** opens with **Show Library**. Browse the current design’s components, templates, themes, styles, and resource dictionaries by kind. Search with **Find Definition**, preview a definition in its authored resource scope, or reveal its declaration and references. Create a XAML data or control template, or duplicate or rename a resource with the same checked transactions available in Studio and MCP.
- Studio layer flags follow the document or selected RSX invocation across native updates. Named or otherwise unambiguous controls retain their flags after source edits. An unchanged buffer also retains flags on unnamed siblings.
- **Timeline** opens with **Show Animation Timeline**. Select a program, scrub the playhead, or play and pause the animation on the native canvas. Drag an XAML keyframe along its track to retime it in one undo step. Use Enter on a keyframe to edit its time, value, and easing by keyboard. Agents can seek, play, and pause through `designer_timeline_control`.
- **Data** opens with **Show Design Data**. Add named JSON fixtures to the XAML view and choose one for the canvas or a Stages card. Fixtures supply a preview data context with strings, booleans, numbers, nested records, and item collections. Apply writes a single design-namespace attribute in one undo step. Pick **Application values** to render the regular bindings. Use `designer_design_fixtures` through `designer_studio_query` or `designer_stage_fixtures` to list choices.
- **Stages** opens with **Open Preview Stages**. Add views from workspace XAML or Rust files with **Add Stage**. Each card shows its dimensions and provides controls for environment, Select or Interact mode, Pause or Resume, and Preview Checks. Open **More Actions** for sizing, visual states, design data, and PNG export. Saved and unsaved changes to local XAML includes refresh dependent cards. The latest accepted image stays visible while a stage refreshes, is paused, or reports an invalid include.
- Switch to **Prototype** on the Stages toolbar to see flows and the authored controls that connect stages. Drag a stage title onto another stage or use **Add interaction** to create a connection. Use **Reveal trigger** to open its XAML or RSX declaration. Configure overlay placement, dimming, and dismissal with the same stage-set model as Studio. Save, open, and play the stage set in either editor. VS Code restores the board when it restores the tab.

## Install or develop from the repository

Use desktop VS Code 1.102 or newer for the stable MCP provider API. Development builds use Node.js 22 or newer and the Rust toolchain used to build FerroUI. Install rust-analyzer for the Rust language features. [rust-analyzer on the Marketplace](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

```sh
cargo build --locked -p ferroui-designer-language -p ferroui-designer-preview
cd editors/vscode
npm ci
npm run build
```

Set `ferroui.languageServer.path` and `ferroui.preview.path` in VS Code's user settings to the absolute paths of `ferroui-xaml-lsp` and `ferroui-designer-preview` in the Cargo output directory. The extension also finds platform-bundled tools and tools installed on `PATH`.

Open a development host from `editors/vscode` with `code --extensionDevelopmentPath="$PWD"`. The standalone test builds can use the same tools and either a downloaded VS Code or an installed executable.

```sh
npm test
# Optional overrides for custom builds and an installed VS Code executable:
# FERROUI_TEST_BIN_DIR=/path/to/target/debug FERROUI_TEST_VSCODE=/path/to/code npm test
```

Build a platform VSIX from the repository root, then install it using **Extensions: Install from VSIX**. Pass the native output directory of your own toolchain when using a custom Cargo target directory.

```sh
cargo build --locked --profile dist-unwind -p ferroui-designer-language -p ferroui-designer-preview
cd editors/vscode
npm ci
npm run package:platform -- ../../target/dist-unwind
```

After building the extension, run `npm run test:vsix -- <package.vsix>` to install the package and smoke-test the minimum supported and current stable releases of VS Code. The Visual designer workflow provides a manual installed-package matrix for macOS, Linux, and Windows.

## Preview and round-trip coverage

- FerroUI XAML uses the designer's runtime loader, design-time projection, WinUI theme, and project-relative file URIs. Run **Build Project XAML Schema** to publish versioned `.xamlmeta` files for a Cargo package. Launch and app-hosted design builds also publish metadata, and `ferroui.types` overrides it. The XAML server supports range formatting, linked tags, namespace imports, spelling fixes, and binding type inlay hints. [XAML language service](../../src/FerroUI.Designer.Language/README.md)
- Static RSX conversion supports a single tree of built-in and explicitly mapped controls, string and numeric literals, boolean values, standard alignment and orientation enums, brushes written as color strings, common attached properties, and grid definitions. Generated RSX expects `use ferroui_view::prelude::*;` and the FerroUI crate dependencies named by any qualified paths in the output.
- In Rust source that includes handlers, computed values, control flow, or custom components, the preview shows the supported controls and literals and lists omitted items. The inspector can edit mapped literals and routed handler expressions. Insertions and moves preserve comments and expressions when the edited container has a fully mapped child list. Dynamic child lists are edited in the Rust editor. The alternate tab is read-only for a partial preview.
- XAML with bindings, styles, resource objects, application directives or property elements remains editable in XAML. The RSX tab explains whichever construct prevents conversion. Value-only counterpart edits preserve unrelated bytes in both formats. A structural conversion regenerates the selected tree in the authored language and requires a tree where it can preserve comments. The Rust file outside the selected tree stays intact.
- FerroUI adds mapped RSX property completion and hover using the macro's own grammar. Enable `ferroui.rsx.inlayHints` to see types beside mapped literal fields.

### Map an application control to RSX

Export `MarkupAssembly` with `ferroui-build-scan` and add a metadata entry whose key uses `FerroUI.Designer.Rsx.v1:` followed by the full XAML type name. The JSON value names the equivalent Rust builder type. Use a `crate::` path for a local builder or a qualified public crate path for a dependency builder.

```rust
(
    "FerroUI.Designer.Rsx.v1:Review.Controls.StatusCard",
    r#"{"path":"crate::views::StatusCard","properties":{"Caption":"heading"}}"#,
)
```

`properties` maps writable XAML properties to the corresponding Rust builder method names. Set `inherit` to `true` for an `El<T>` builder that uses the mapped base control's FerroUI properties, events, and content slot. Open the view with **Open App-Hosted Design** to render its registered native implementation. [Todo Workbench mapping](../../samples/TodoWorkbench/app_preview.rs)

## App-hosted design and Live App

**Open App-Hosted Design** builds and starts the selected Cargo binary in design mode. The application registers its own XAML types, loader, assets, and theme using `RemoteDesignerEntryPoint::run_if_requested` from `ferroui-designer-support`. An authenticated loopback relay sends unsaved XAML or the supported static projection of the selected RSX invocation. Specify `ferroui.hostedPreview.assembly` when the registered markup assembly differs from the Cargo package name.

An application can call `register_preview_fixture` during preview setup. A **DataContext** fixture runs the current source with a typed application model. A **RustView** fixture mounts a fresh Rust control from the built binary. Choose **Source preview** to return to the current XAML or RSX buffer. The fixture selector is scoped to the previewed file. Agents can use `designer_hosted_fixtures` and `designer_hosted_fixture` to read choices and render them through the same session.

The [Todo Workbench sample](../../samples/TodoWorkbench/README.md) demonstrates the preview entry point. Enable its `app-preview` Cargo feature in `ferroui.hotReload.targets` to use it with the hosted design command.

**Open Live App** shows actual managed or attached application windows. The visual, logical, and accessibility hierarchies use the existing authenticated runtime inspector. Inspect properties, styles, resources and diagnostics, or use Interact to send native input where the runtime permits it. Hold `Alt` while clicking in Inspect mode to choose from overlapping native visual hits. Runtime property overrides affect the attached app. **Reveal Source** uses compiler locations inside the workspace. Managed debug builds record the source for mounted RSX controls, and compatible hot reloads update those positions. Set `FERROUI_EDITOR_SOURCE=1` when launching a debug build outside VS Code to enable compiled RSX locations for attached inspection.

**Apply to Source** writes verified XAML values or mapped RSX literals to the editor in one undo step. The application keeps running while you review the buffer. The runtime identifies its original source revision through compiled or runtime-loaded XAML metadata or the RSX macro. For externally launched debug apps, use `FERROUI_EDITOR_SOURCE=1` to record runtime source information. Compiling XAML with source information also records the original file and revision. Agents can use `ferroui_runtime_source` and pass its connection generation and buffer version to `ferroui_runtime_apply_source`. Both tools accept the same `runtime` choice, either `managed` or `attached`.

**Project** groups the open workspace folders into Cargo manifests, XAML and RSX views, assets, and saved stage sets. Expand a manifest to find its runnable binaries. **Find Project View** opens a design by name. **New View** and **New Component** create starter files in the selected workspace. Add the generated Rust file to your app’s module tree when you are ready to use it. MCP agents can browse the same index with `ferroui_project_files` and use `ferroui_project_scaffold` to create the same starter files.

Use **Find Definition in Designer** to search the active design’s component, template, and resource definitions and preview one in its authored scope. **Rename RSX Control** uses a checked edit to rename an authored `name` literal in the selected invocation. The same rename is available to agents through `designer_rename_rsx_control`.

**Diagnostics** shows the outcome and acknowledgement time of observed saves in a managed session. It also reads layout and scene counters from the native inspector, and opens authored source from binding errors that carry an available location. **Preview Checks** opens the source of missing accessible names, measured rectangular clipping, and low contrast for text on supported opaque backgrounds in the current native XAML or RSX preview. Each **Stages** card has its own **Checks** action. Agents can run the checks through `designer_studio_query` with `designer_preview_checks` or through `designer_stage_checks`. MCP exposes managed history through `ferroui_reload_history` and the runtime’s logs and counters through `get_logs` and `get_frame_stats`.

### Design token handoff

Use **Import Design Tokens** or **Export Design Tokens** from the Project view or Command Palette. Review the generated file beside the token-to-resource mapping and skipped entries. You can rename a resource key or token path, change a color between `Color` and `SolidColorBrush`, or export a `Double` as a logical-pixel dimension. Create a new file after review. Include an imported dictionary in a view with `ResourceInclude`. The native canvas renders local workspace includes and refreshes when a saved or unsaved dependency changes.

Import supports DTCG numbers, `px`, `rem` with an explicit scale, sRGB colors, and durations. It resolves typed groups, `$root` tokens, curly-brace aliases, and same-document JSON Pointers, including value components. The report shows resolved aliases and rounded or converted values. Export reads direct literal entries and records the XAML key and representation in the generated token metadata. Group inheritance, non-sRGB colors, composite tokens, and dynamic or included XAML resources appear as skipped entries. VS Code and Studio agents use the same read-only `designer_design_tokens` planner. [DTCG format](https://www.designtokens.org/TR/2025.10/format/)

## Native UI scenarios

Save a `*.ferroui-scenario.json` file inside a workspace. The **FerroUI Scenarios** section in Testing runs the saved source in a fresh native preview. Use FerroUI selectors to target input and property assertions. Accessibility assertions read the actual native automation tree.

```json
{
    "version": 1,
    "name": "Customer search",
    "document": "CustomerSearch.xaml",
    "viewport": { "width": 500, "height": 280 },
    "steps": [
        { "action": "expectAccessible", "name": "Customer query", "role": "Edit" },
        { "action": "typeText", "selector": "#Query", "text": "Łódź" },
        { "action": "expectProperty", "selector": "#Result", "property": "Text", "value": "Łódź" },
        { "action": "expectScreenshot", "baseline": "Baselines/customer-search.png" }
    ]
}
```

The document and baseline paths are relative to the scenario. A Rust scenario can specify a zero-based UTF-16 `caret` with `line` and `character` to choose an RSX invocation. Use **Accept FerroUI Screenshot Baselines** from the Testing run profile picker to write the rendered images after the assertions pass. A mismatch links to its expected image, actual image, and highlighted pixel diff. Specify `channelTolerance` and `maxChangedPixels` on an image assertion when the fixture allows pixel variation.

Run the same scenario in CI with `ferroui-designer-preview --scenario customer-search.ferroui-scenario.json`. Use `--workspace` for scenarios stored below their workspace root and `--artifacts` to choose a directory for visual evidence. The runner prints a JSON report and returns a failing status for a failed assertion. Agents use `ferroui_scenario_run` on the same saved file.

Set `prototype` to run a saved `.ferrostages` file through the native Stages player. Supply its `stageSet` path and `flow` name. The optional `sources` map assigns a saved source URI to a workspace-relative `path` and optional RSX `caret` for a portable scenario. Paths are resolved from the scenario folder. Use `activateLink` with an authored interaction name, `expectStage` with the underlying screen name, and `expectOverlay` with the top overlay name or `null`. Input, accessibility, property checks, and PNG comparisons use the current native screen or overlay. The customer review fixture follows a saved flow through two screens and a notes overlay. [Customer review scenario](test/fixtures/customer-review.ferroui-scenario.json)

## Hot reload a running application

Run **FerroUI: Run with Hot Reload** from a local Cargo workspace and select a development binary. Try `hot-reload-demo` for XAML or `view-gallery` for Rust. The extension uses Cargo metadata and enables a package’s `hot-reload` feature when defined. Save your changes in the editor or save a design edit to update the application. The status bar and **FerroUI: Show Hot Reload Output** report in-place updates, rejected edits, and rebuilds.

- XAML uses the application’s existing `ReloadManifest` and `HotReloadSession` to apply a coherent batch of registered view and resource edits. Application integration calls `ferroui_hot_reload::notify_editor_on_open` after preparing its targets and before showing its main window.
- Rust uses `ferroui-view` with `hot-reload` in a debug build. `launch` registers the watcher and startup acknowledgement. Eligible literals are patched through mounted reactive views. RSX structure, Rust logic, and literals without a live reactive reader use a Cargo rebuild.
- The previous app runs while Cargo builds and the candidate opens its window. A failed build or launch leaves that app running. Rebuilds start a new process, so in-memory state is retained by in-place edits and by any persistence the application implements. Use **FerroUI: Rebuild Hot Reload Application** to request a build and **FerroUI: Stop Hot Reload Application** to end the session.
- Use **Run with Registered Rust Patches** for the owning supervisor workflow. It runs the trusted workspace's `ferroui.hotReload.registeredSupervisor` script through Python, with the patch compiler on PATH or configured in `ferroui.hotReload.patchCompilerPath`. The sample supervisor requires its pinned Rust and `dx` toolchain and uses the configured application's explicit patch slots and compatible state handoff. [Registered Rust patch workflow](../../docs/development/hot-reload.md)
- Configure a Rust attach request in `launch.json` using `${command:ferroui.hotReload.processId}` for the adapter's process ID field. Choose **Run and Debug with Hot Reload** to build and attach after startup, or **Attach Debugger to Managed App** to debug an existing session. The extension reuses the selected configuration when a replacement process acknowledges startup.

Discovery starts at the active file and uses the nearest Cargo manifest inside the workspace. Configured targets can specify `label`, a workspace-relative `manifestPath`, `package`, `binary`, `features`, and application `args` in `ferroui.hotReload.targets`. Cargo discovery and builds both start in the selected manifest's directory, so they use the same Cargo configuration. The extension uses `ferroui.hotReload.cargoPath` or Cargo from PATH. [FerroUI hot-reload engine](../../docs/development/hot-reload.md)

## Drive the designer with MCP

In a trusted workspace, start **FerroUI Designer in VS Code** from the MCP servers list and select its tools in your agent. Set `ferroui.mcp.allowWrites` in user settings to authorize source edits, designer controls, and managed Cargo actions. `ferroui.mcp.enableDesigner` controls the editor server. [VS Code MCP integration](https://code.visualstudio.com/api/extension-guides/ai/mcp)

| Workflow | Tools |
| --- | --- |
| Work with project source | Use `ferroui_workspace_files` and `ferroui_workspace_read` to discover files and read unsaved buffers. Query the active VS Code language providers with `ferroui_workspace_language`. Create files with `ferroui_workspace_create`, apply an atomic text transaction with `ferroui_workspace_edit`, and open, save, format, undo, redo, rename, or delete with `ferroui_workspace_file`. |
| Work on the live design | Use `designer_open`, `designer_wait`, and `designer_block` to choose a design or an RSX invocation. Use `designer_view` and `designer_canvas` to control docks, viewport, zoom, pan, canvas mode, and the alternate-source tab. Send preview interactions through `designer_canvas_input` with the current frame revision. Read the outline, Properties, Events, resources, and project assets, and capture the native frame with `designer_preview_image`. |
| Edit the design | Use `designer_placement_hint` and `designer_place` to plan and place Toolbox items, project images, or existing controls. Use the property, event, and resource tools for mapped source edits. `designer_apply_counterpart` applies changes from a supported XAML or RSX projection. |
| Use Studio authoring | `designer_studio_tools` lists native read tools, source-backed actions and presentation actions. Use `designer_studio_query` to inspect the current preview or request a plan, including `designer_binding` for a property’s binding fields. `designer_studio_edit` applies actions such as `designer_binding_edit` to the buffer. `designer_studio_present` changes local layer, definition, animation or state presentation. Pass the same ordered selection and native model token when applying a selection-wide plan. |
| Compare stages and author a prototype | Use `designer_stages_open` and `designer_stages` to read the board, `designer_stage_image` for an accepted native PNG, and `designer_stage_hotspots` for source-anchored controls. Use `designer_stage_change` to configure stages or author the graph, and `designer_stage_prototype` to play it. Send the current board or stage epoch with each action. |
| Run and inspect the native app | Use `ferroui_reload_targets` to choose an exact Cargo binary and `ferroui_reload_control` to start, rebuild, or stop it. `start_debug` can select an attach configuration by name. Read `ferroui_reload_status` and `ferroui_reload_output` for the PID, current reload state, and recent output. |

Workspace edit tools use opaque buffer revisions and UTF-16 positions. Designer edits use the current frame revision. Read the revision again after a change or an undo. Advanced Studio actions that author XAML use the XAML designer. RSX uses mapped design edits and the Rust workspace tools. Language results come from the installed Rust or XAML provider. [VS Code language provider commands](https://code.visualstudio.com/api/references/commands)

If a managed debug binary starts its existing authenticated HTTP MCP server, VS Code discovers **FerroUI Managed Application** after the process starts. It offers native element queries, input, accessibility and screenshots according to the app’s permissions. An in-place reload retains the MCP connection and a successful rebuild replaces its definition. Try it with the [Todo Workbench sample](../../samples/TodoWorkbench/README.md).

**FerroUI: Connect to Running MCP Server** attaches an existing authenticated loopback endpoint of Designer Studio or a separate app. For stdio, configure the executable and its flags in `ferroui.mcp.stdioServers`. Attach to the native Studio workspace for collaboration, project tasks, timeline playback, definition previews, and prototype-stage tools. [FerroUI runtime MCP](../../docs/mcp.md) · [Designer Studio MCP](../../docs/designer/README.md#mcp-integration)

## Architecture

The built-in XAML language client connects to `ferroui-xaml-lsp` over stdio. The server handles UTF-16 document synchronization, diagnostics, completion, hover, references, rename, formatting, full and delta semantic tokens, workspace indexing, and project resource navigation. Rust files keep the normal `rust` language ID and rust-analyzer owns Cargo, macro expansion, and semantic analysis. [VS Code language server guide](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide), [LSP specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/), [rust-analyzer manual](https://rust-analyzer.github.io/book/installation.html).

The extension uses a side-by-side webview so the normal editors and their language servers stay active. It debounces buffer updates and checks document versions before applying each `WorkspaceEdit`. The canvas receives pixels and picking bounds. Docks follow the same active session and request inspection data when visible. Pointer gestures cross the docked extension views, and the native layout engine computes the current landing area. Changing canvas zoom needs no round trip to the renderer. [VS Code webview guide](https://code.visualstudio.com/api/extension-guides/webview) · [VS Code webview drop behavior](https://github.com/microsoft/vscode/issues/182449)

`ferroui-designer-preview` is a versioned stdio process for native layout, edit planning and comparison stages. It uses `DesignerSession`, `PreviewStageBoard`, Vello CPU, and native PNG capture. Hosted design uses `DesignerSupport` and `Remote.Protocol` with correlated source revisions and window-specific frame acknowledgements. [Avalonia live preview documentation](https://docs.avaloniaui.net/docs/app-development/xaml-preview-and-design-settings)

`ferroui-designer-rsx` translates between the static control trees and keeps byte ranges for authored literals. `ferroui-view-syntax` shares the same grammar with the actual `rsx!` macro. `LineIndex` in the XAML language crate translates native UTF-8 ranges to VS Code's UTF-16 positions.

The editor MCP process reuses `ferroui-mcp` for transport, tool annotations, request limits, and cancellation. An authenticated loopback bridge forwards its editor tools to the owning VS Code window. The preview process invokes Studio authoring on the current native frame and returns UTF-16 patches. VS Code checks the current frame and buffer versions before applying the patches, then renders from the editor. `ferroui-reload-protocol` carries runtime acknowledgements and optional loopback MCP discovery events to the managed Cargo session.

The [workbench design review](https://github.com/wieslawsoltes/FerroUI/blob/main/docs/designer/vscode-workbench.md) compares the interaction patterns in other IDEs and design tools.
