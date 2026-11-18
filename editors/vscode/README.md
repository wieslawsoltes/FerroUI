# FerroUI Designer for VS Code

Open a FerroUI `.xaml` file or a Rust file containing `ferroui_view::rsx!`, then choose **FerroUI: Open Designer** from the editor toolbar or Command Palette. The designer opens beside the text editor and renders its unsaved buffer.

## Working in the designer

- **Toolbox, Outline, Properties, Events, Resources, and Assets** are independent VS Code views. Open them from the canvas toolbar or the Command Palette. Use **View: Move View** to keep Tools and Outline beside the editor and put Properties in the Secondary Side Bar. **View: Toggle Maximize Editor Group** gives the canvas the space of both editor groups. [VS Code custom layout](https://code.visualstudio.com/docs/configure/custom-layout)
- **Toolbox** shares Designer Studio’s 20 built-in controls, layouts, and shapes. Search by name, category, or description and switch between lists and tiles. Drag an asset onto the canvas or Outline. Press Enter to insert into the selected container or its nearest parent with room.
- **Placement** uses the same layout planners as Studio. The canvas shows the insertion location and resolves occupied content hosts through an eligible ancestor or the layout body inside a wrapper. In Source Outline, the top and bottom of a row insert before and after it. Drop in the middle of an available container to nest. Use `Alt` + `↑` or `↓` for keyboard reordering. Canvas placement uses the measured layout, including grid cells and Canvas coordinates. One drop makes one source edit that VS Code can undo.
- **Outline** offers Source, Visual, Logical, and Combined. The combined hierarchy keeps visual nesting and adds logical-only objects under their logical parent. Select a runtime-only template part to inspect its effective properties and see its bounds. Source mode supports structural edits.
- **Properties** displays measured X, Y, width, and height alongside editable size, layout, content, and appearance fields supported by the selected control. Search the additional property groups for less common settings. Reset removes an authored literal so the default or style can take effect. Bindings and Rust expressions have a source navigation action.
- **Events** lists routed events and assigned handlers for the selected control. Assign or clear a XAML code-behind name or edit an RSX handler expression. Use the reveal action to jump to its declaration in source. Application callbacks run through the application host.
- **Resources** lists authored XAML definitions with scope and usage counts. Filter to the definitions accessible to the selection, reveal a declaration, or apply a compatible static or dynamic resource reference.
- **Assets** browses workspace images, fonts, media, and XAML files. Preview raster images, open files, and copy relative paths. Drag a raster image to the XAML canvas or Source Outline to insert an Image with a document-relative URI. Rust projects can browse assets and copy paths for their image-loading expressions.
- **Selection** follows the focused designer or its source editor. Clicking a control in the canvas or Outline selects it and reveals its declaration if the source editor is visible. Double-click the control or use **Reveal Selected Control in Source** to open the declaration. Properties edits the selected control. VS Code owns undo, redo, saving, and dirty state.
- **Viewport** opens from the size button. Choose a device preset, type a logical width and height, swap the dimensions, or switch the theme. **Fit to view**, **Fit selection**, and **Actual size** are available from the zoom menu. With the canvas focused, use `0` to fit, `1` for actual size, and `+` or `−` to step the zoom. `Ctrl`/`Cmd` + scroll zooms at the pointer. Pan by dragging with Space held, using the middle button, or switching on the pan tool.
- **Alternate source** is available on the **XAML** and **RSX** tabs. For Rust files with several macros, place the caret inside the view you want or use the RSX picker. Apply the alternate source to update the authored buffer. If the editor changes while a projected draft is open, the draft stays visible and Apply waits for a reload of the projection.

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

## Preview and round-trip coverage

- FerroUI XAML uses the designer's runtime loader, design-time projection, WinUI theme, and project-relative file URIs. Application-owned control registrations, code-behind handlers and live data contexts require an application host. XAML language metadata can be supplied through `ferroui.types` using the schema documented by the [XAML language service](https://github.com/wieslawsoltes/FerroUI/tree/main/src/FerroUI.Designer.Language).
- Static RSX conversion supports one built-in control tree with common containers and controls, string and numeric literals, boolean values, standard alignment and orientation enums, brushes written as color strings, common attached properties, and grid definitions. Generated RSX expects `use ferroui_view::prelude::*;` and the FerroUI crate dependencies named by any qualified enum paths in the output.
- In Rust source that includes handlers, computed values, control flow, or custom components, the preview shows the supported controls and literals and lists omitted items. The inspector can edit mapped literals and routed handler expressions. Insertions and moves preserve comments and expressions when the edited container has a fully mapped child list. Dynamic child lists are edited in the Rust editor. The alternate tab is read-only for a partial preview.
- XAML with bindings, styles, resource objects, application directives or property elements remains editable in XAML. The RSX tab explains whichever construct prevents conversion. Value-only counterpart edits preserve unrelated bytes in both formats. A structural conversion regenerates the selected tree in the authored language and requires a tree where it can preserve comments. The Rust file outside the selected tree stays intact.

## Hot reload a running application

Run **FerroUI: Run with Hot Reload** from a local Cargo workspace and select a development binary. Try `hot-reload-demo` for XAML or `view-gallery` for Rust. The extension uses Cargo metadata and enables a package’s `hot-reload` feature when defined. Save your changes in the editor or save a design edit to update the application. The status bar and **FerroUI: Show Hot Reload Output** report in-place updates, rejected edits, and rebuilds.

- XAML uses the application’s existing `ReloadManifest` and `HotReloadSession` to apply a coherent batch of registered view and resource edits. Application integration calls `ferroui_hot_reload::notify_editor_on_open` after preparing its targets and before showing its main window.
- Rust uses `ferroui-view` with `hot-reload` in a debug build. `launch` registers the watcher and startup acknowledgement. Eligible literals are patched through mounted reactive views. RSX structure, Rust logic, and literals without a live reactive reader use a Cargo rebuild.
- The previous app runs while Cargo builds and the candidate opens its window. A failed build or launch leaves that app running. Rebuilds start a new process, so in-memory state is retained by in-place edits and by any persistence the application implements. Use **FerroUI: Rebuild Hot Reload Application** to request a build and **FerroUI: Stop Hot Reload Application** to end the session.

Configured targets can specify `label`, a workspace-relative `manifestPath`, `package`, `binary`, `features`, and application `args` in `ferroui.hotReload.targets`. The extension uses `ferroui.hotReload.cargoPath` or Cargo from PATH. The existing native Rust hot-patch supervisor remains available for applications registered with it. [FerroUI hot-reload engine](../../docs/development/hot-reload.md)

## Drive the designer with MCP

In a trusted workspace, VS Code discovers **FerroUI Designer in VS Code** through its MCP server provider. Start it from the MCP servers list and select its tools in your agent. `ferroui_designs` finds open sessions, `designer_document` reads the unsaved buffer, and `designer_outline` and `designer_preview_image` return the source hierarchy and a native PNG. Agents can inspect Properties and Events with `designer_inspect`, browse `designer_resources` and `designer_assets`, and query Studio’s `designer_tree` in visual, logical, or combined mode. `designer_studio_tools` lists the Studio queries available against the native preview. [VS Code MCP integration](https://code.visualstudio.com/api/extension-guides/ai/mcp)

Set `ferroui.mcp.allowWrites` in user settings to enable source edits and UI or runtime controls. `designer_apply` accepts Studio’s typed source edit format. `designer_place` uses the same Toolbox and layout planner as the canvas. Buffer tools check the current version before replacing, saving, or undoing an edit. `designer_apply_counterpart` applies the XAML or RSX projection. `ferroui.mcp.enableDesigner` controls the editor server.

**FerroUI: Connect to Running MCP Server** attaches the existing authenticated loopback endpoint of Designer Studio or an app. For stdio, configure an absolute executable and its native flags in `ferroui.mcp.stdioServers`. These endpoints expose their original tools and permission settings, including native input and inspection, collaboration, timeline authoring, project and component workflows, and prototype stages in Studio. Each endpoint operates on its own live session. Use the VS Code server for unsaved VS Code buffers. [FerroUI runtime MCP](../../docs/mcp.md) · [Designer Studio MCP](../../docs/designer/README.md#mcp-integration)

## Architecture

The built-in XAML language client connects to `ferroui-xaml-lsp` over stdio. The server handles UTF-16 document synchronization, diagnostics, completion, hover, references, rename, formatting, semantic tokens, workspace indexing, and project resource navigation. Rust files keep the normal `rust` language ID and rust-analyzer owns Cargo, macro expansion, and semantic analysis. [VS Code language server guide](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide), [LSP specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/), [rust-analyzer manual](https://rust-analyzer.github.io/book/installation.html).

The extension uses a side-by-side webview so the normal editors and their language servers stay active. It debounces buffer updates and checks document versions before applying each `WorkspaceEdit`. The canvas receives pixels and picking bounds. Docks follow the same active session and request inspection data when visible. Pointer gestures cross the docked extension views, and the native layout engine computes the current landing area. Changing canvas zoom needs no round trip to the renderer. [VS Code webview guide](https://code.visualstudio.com/api/extension-guides/webview) · [VS Code webview drop behavior](https://github.com/microsoft/vscode/issues/182449)

`ferroui-designer-preview` is a separate, versioned stdio process for native layout and edit planning. It uses the existing `DesignerSession`, property transactions, Vello CPU, and PNG capture. The repo's Avalonia-derived previewer ports handle application-hosted preview and their frame transport. This extension uses the same out-of-process approach with the designer session's source identities and edit plans. [Avalonia live preview documentation](https://docs.avaloniaui.net/docs/app-development/xaml-preview-and-design-settings).

`ferroui-designer-rsx` translates between the static control trees and keeps byte ranges for authored literals. `ferroui-view-syntax` shares the same grammar with the actual `rsx!` macro. `LineIndex` in the XAML language crate translates native UTF-8 ranges to VS Code's UTF-16 positions.

The editor MCP process reuses `ferroui-mcp` for transport, tool annotations, request limits, and cancellation. An authenticated loopback bridge forwards its editor tools to the owning VS Code window. The extension host owns `WorkspaceEdit` and document versions. `ferroui-reload-protocol` carries runtime acknowledgements between the XAML and RSX development runtimes and the editor’s managed Cargo session.

The [workbench design review](https://github.com/wieslawsoltes/FerroUI/blob/main/docs/designer/vscode-workbench.md) compares the interaction patterns in other IDEs and design tools.
