# FerroUI Designer for VS Code

Open a FerroUI `.xaml` file or a Rust file containing `ferroui_view::rsx!`, then choose **FerroUI: Open Designer** from the editor toolbar or Command Palette. The designer opens beside the text editor and renders its unsaved buffer.

## Working in the designer

- **Outline and Properties** are separate VS Code views in the FerroUI sidebar. Open them from the canvas toolbar or with **FerroUI: Show Outline** and **FerroUI: Show Properties**. Use **View: Move View** or drag a view header to another sidebar or the Panel. Collapse either view to give the other more room, or hide its sidebar to give the editors more width. **View: Toggle Maximize Editor Group** gives the canvas the space of both editor groups. [VS Code custom layout](https://code.visualstudio.com/docs/configure/custom-layout)
- **Selection** follows the focused designer or its source editor. Clicking a control in the canvas or Outline selects it and reveals its declaration if the source editor is visible. Double-click the canvas control or choose **Reveal Selected Control in Source** to open the declaration. Edit, insert, and remove controls in Properties. VS Code owns undo, redo, saving, and dirty state.
- **Viewport** opens from the size button. Choose a device preset, type a logical width and height, swap the dimensions, or switch the theme. **Fit to view**, **Fit selection**, and **Actual size** are available from the zoom menu. With the canvas focused, use `0` to fit, `1` for actual size, and `+` or `−` to step the zoom. `Ctrl`/`Cmd` + scroll zooms at the pointer. Pan by dragging with Space held, using the middle button, or switching on the pan tool.
- **Alternate source** is available on the **XAML** and **RSX** tabs. For Rust files with several macros, place the caret inside the view you want or use the RSX picker. Apply the alternate source to update the authored buffer. If the editor changes while a projected draft is open, the draft stays visible and Apply waits for a reload of the projection.

## Install or develop from the repository

Use desktop VS Code 1.95 or newer, Node.js 22 or newer, and the Rust toolchain used to build FerroUI. Install [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) for Rust language features.

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
- In Rust source that includes handlers, computed values, control flow, or custom components, the preview shows the supported controls and literals and lists omitted items. The inspector can still write mapped literals into the Rust editor. The alternate tab is read-only for a partial preview. Chained builders and live application state stay in Rust source.
- XAML with bindings, styles, resource objects, application directives or property elements remains editable in XAML. The RSX tab explains whichever construct prevents conversion. Value-only counterpart edits preserve unrelated bytes in both formats. A structural conversion regenerates the selected tree in the authored language and requires a tree where it can preserve comments. The Rust file outside the selected tree stays intact.

## Architecture

The built-in XAML language client connects to `ferroui-xaml-lsp` over stdio. The server handles UTF-16 document synchronization, diagnostics, completion, hover, references, rename, formatting, semantic tokens, workspace indexing, and project resource navigation. Rust files keep the normal `rust` language ID and rust-analyzer owns Cargo, macro expansion, and semantic analysis. [VS Code language server guide](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide), [LSP specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/), [rust-analyzer manual](https://rust-analyzer.github.io/book/installation.html).

The extension uses a side-by-side webview so the normal editors and their language servers stay active. It debounces buffer updates and checks document versions before applying each `WorkspaceEdit`. The canvas receives pixels and picking bounds. The native Outline and the Properties webview follow the same active session and selection. Edits from either view use the same source transaction, and changing the canvas zoom needs no round trip to the renderer. [VS Code webview guide](https://code.visualstudio.com/api/extension-guides/webview), [VS Code custom editor guide](https://code.visualstudio.com/api/extension-guides/custom-editors).

`ferroui-designer-preview` is a separate, versioned stdio process for native layout and edit planning. It uses the existing `DesignerSession`, property transactions, Vello CPU, and PNG capture. The repo's Avalonia-derived previewer ports handle application-hosted preview and their frame transport. This extension uses the same out-of-process approach with the designer session's source identities and edit plans. [Avalonia live preview documentation](https://docs.avaloniaui.net/docs/app-development/xaml-preview-and-design-settings).

`ferroui-designer-rsx` translates between the static control trees and keeps byte ranges for authored literals. `ferroui-view-syntax` shares the same grammar with the actual `rsx!` macro. `LineIndex` in the XAML language crate translates native UTF-8 ranges to VS Code's UTF-16 positions.

The [workbench design review](https://github.com/wieslawsoltes/FerroUI/blob/main/docs/designer/vscode-workbench.md) compares the interaction patterns in other IDEs and design tools.
