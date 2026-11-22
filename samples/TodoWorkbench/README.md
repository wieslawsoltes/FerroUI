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
Open [motion.rs](app_preview/motion.rs) with **Open App-Hosted Design** to preview and edit a
project-registered `CornerRadius` clip. At one second, the two-second clip uses the app's
quadratic animator to reach radius 8. Choose **Edit application keyframe** to retime a literal
or change its four radii. VS Code applies one undoable edit to the Rust buffer.
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

Open [template_review.rs](app_preview/template_review.rs) in App Preview and
choose **Review queue**. From **App Preview Templates**, select **Queue** and
open its shared template in [template_rows.rs](app_preview/template_rows.rs).
Use **Open Template Assignment** to return to the list's Rust expression. The
picker also shows the priority selector and the table's account cell.

Open [HostedTreeTemplates.xaml](HostedTreeTemplates.xaml) or
[tree_templates.rs](app_preview/tree_templates.rs) and choose **Account hierarchy**.
In **App Preview Templates**, select **Accounts / Company** to open the display
or editing template for the expander's inner column. The XAML view uses
[TreeTemplateResources.xaml](TreeTemplateResources.xaml) for Company and an
inline template for Review. The Rust view exposes each template's builder
assignment. Select the **Review** editing template before editing a cell.
The `template!` call in its shared factory records a source location in
[template_rows.rs](app_preview/template_rows.rs). [Trace effective templates in VS Code](../../docs/designer/vscode/runtime.md#inspect-treedatagrid-display-and-editing-templates)

To inspect the Rust grid in its own application window, run
`cargo run -p todo-workbench --features app-preview,mcp -- --tree-templates-live --mcp`.
For an externally launched debug build, set `FERROUI_EDITOR_SOURCE=1` before
starting the process. Connect Live App to its inspector, pick **Accounts**, and
open **Live App Templates**. [Inspect the running app](../../docs/designer/vscode/runtime.md#open-an-effective-template-in-the-running-app)

Open [review_states.rs](app_preview/review_states.rs) in App Preview and choose
**Interactive review**. Select **Review · Queued → Ready** in the animation
picker. Scrub the live card's width and corner radius, or use the pencil to
change the Rust route's duration and easing. Stop the playhead, then click
**Queued**, **Ready**, and **Reset** to use the compiled fixture's state
handlers. [App Preview state-route workflow](../../docs/designer/vscode/motion.md#scrub-a-rust-state-route-in-app-preview)

Use **Visual state actions** in App Preview to author groups, states, setters, and transitions in XAML or mapped Rust. The sample registers the custom XAML interactions library so directional transitions load in both preview and `--xaml-live` mode. [Visual state authoring](../../docs/designer/vscode/motion.md#author-visual-states-in-app-preview)
