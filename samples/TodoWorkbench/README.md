# Todo Workbench

A native Todo app using FerroUI Rust components and `rsx!`. It reuses the TodoMVC
model and supports adding, checking, filtering, removing, and clearing tasks.

```sh
cargo run -p todo-workbench --features hot-reload,mcp -- --demo --mcp
```

`--demo` loads three sample tasks. `--mcp` starts the local runtime inspector when the
`mcp` feature is enabled. Add `--mcp-input` to authorize native clicks and keyboard
input from an MCP client.

## Run it from VS Code

Open the FerroUI checkout in VS Code. Use **FerroUI: Open Designer** on
`samples/TodoWorkbench/view.rs` and select the first RSX invocation. Edit the
heading text in Properties. The source and preview update together.

To run this target through the extension, add a target like this to your VS Code
settings and run **FerroUI: Run with Hot Reload**.

```json
{
  "ferroui.hotReload.targets": [
    {
      "label": "Todo Workbench",
      "package": "todo-workbench",
      "binary": "todo-workbench",
      "features": ["hot-reload", "mcp"],
      "args": ["--demo", "--mcp", "--mcp-input"]
    }
  ]
}
```

Give an agent access to **FerroUI Designer in VS Code** to edit the design and
save source. With **FerroUI Managed Application**, the agent can use
`find_elements` to locate `NewTask` and `AddTask`, then call `type_text` and
`click`. Save a heading text edit to see the live title update alongside the
tasks you already entered. [VS Code extension guide](../../editors/vscode/README.md)

## Attach to a XAML window

The app-preview feature can open a file-backed XAML Window with the same
WinUI theme and local inspector. Enable source metadata to use **Reveal Source**
and **Apply to Source** after attaching from VS Code.

```sh
FERROUI_EDITOR_SOURCE=1 cargo run -p todo-workbench --features app-preview,mcp -- \
  --xaml-live path/to/Window.xaml --mcp --mcp-input
```

## App-hosted design

Add `"app-preview"` to the target's features and open
`samples/TodoWorkbench/HostedPreview.xaml`. Run **FerroUI: Open App-Hosted Design**.
The application registers its own `PreviewBadge` control and markup assembly.
Its build exports the control's XAML schema for completion and type checking.
Edit the XAML, or click the preview's text box and use **Send Text**. The app's
native binding updates the badge. Choose **Loading** or **Ready** in Fixture to
apply the registered data context. [Preview entry point](app_preview.rs)

Choose **Review shadow** in the animation picker, then select the pencil icon to
edit the `BoxShadow` key at 2 seconds. For example, enter `12 18 30 3 #88244B90`
and apply it to the XAML buffer. Seek to 2 seconds to see the new native shadow.
The sample codec validates the authored literal and the app animates it as a
native shadow collection. [Review shadow codec](app_preview/animation_codec.rs)

After a build, **Project controls** in the Toolbox includes `PreviewBadge`.
Open the `rsx!` invocation in `app_preview.rs` to edit its mapped `text` literal
or open the same Rust document in app-hosted design. Both use the control's
native XAML registration. The **Empty queue** and **One task** fixtures mount
Rust views from the built binary. **Source preview** follows the current editor
buffer. A Cargo build updates the Rust fixture factories.
