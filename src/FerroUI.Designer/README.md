# FerroUI designer

`ferroui-designer` is the embeddable visual XAML designer of FerroUI Designer Studio: a native workspace with a design surface, a source editor, property and layout tools, a component browser, preview stages and an animation timeline, over one transactional document.

## Embedding the workspace

```rust
use ferroui_controls::Control;
use ferroui_designer::DesignerWorkspace;

let workspace = DesignerWorkspace::new(
    r#"<StackPanel xmlns="https://github.com/ferroui">
  <Button Content="Confirm"/>
</StackPanel>"#,
)?;
// The workspace is a control: put it in a window or any content host.
window.set_content(Some(Control::boxed(&workspace.control())));
# Ok::<(), String>(())
```

`DesignerWorkspace::with_uri` keeps the original URI of the document, so that includes and relative resource references resolve as in the application.

## Sessions

`DesignerSession` is the transaction boundary on the UI thread that the source editor, the visual tools and automation share. It previews the document in a `ContentControl` that the caller gives it, and it can be used without the workspace for a custom designer. Neither the session nor the workspace starts a window, a server or a background transport.

The document engine is `ferroui-designer-core`; the project model, the tools, the language services, the animation timeline and collaboration are the crates `ferroui-designer-project`, `ferroui-designer-tools`, `ferroui-designer-language`, `ferroui-designer-animation` and `ferroui-designer-collaboration`. The `mcp` feature adds the automation of the designer through `ferroui-mcp`.

The `ferroui` facade has this crate as its feature `designer`.

## Validation

```sh
cargo test -p ferroui-designer
```
