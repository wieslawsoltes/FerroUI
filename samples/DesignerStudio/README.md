# Designer Studio

A native FerroUI visual XAML designer built from reusable crates. The sample owns application startup; its workspace and editing engines are embeddable.

```sh
cargo run -p designer-studio
cargo run -p designer-studio -- --file path/to/View.xaml
```

The source document must have a control root that the runtime XAML loader can resolve. The preview retains root `x:Class` declarations in source without instantiating code-behind. A Window root previews its content through a Border projection; native window chrome settings remain in source. Custom controls can be registered by an embedding application before constructing the workspace. Runtime load errors preserve the last successful preview and the authored source remains available for correction.

## Editing

- Drag an asset onto the artboard, or pick an asset and click a location to insert it.
- Select in the artboard or source hierarchy; Shift selects multiple controls. Drag the selection, use its eight size handles, or its rotation handle. Release commits one history item. Escape cancels the active gesture.
- Edit a property and press Enter to apply it. An empty value authors an explicit empty string; **Reset** removes the authored attribute. Bindings and resources can be entered as normal XAML expressions.
- Source edits update the document and preview after a short debounce. Invalid source stays in the editor while the last accepted preview stays visible.
- Undo/redo share the same history across source, properties, layout and keyframe authoring.
- Source, visual and logical outline modes inspect the authoring tree or the real runtime trees. Native template children appear as runtime-only rows.
- Interact disables the designer overlay and unfreezes runtime animation clocks for exercising controls. Select returns to authoring.

## Animation

Enable Record and edit properties at different scrubber times. Recording writes native per-control `Style.Animations` to the document. The timeline also offers keyframe insertion, time editing, removal and easing. The sample's Add keyframe button starts an opacity track; the reusable session accepts all supported typed properties. Playback and scrubbing layer animation values over the live property store and dispose those layers on Stop, preserving base values and bindings.

The active workspace timeline imports the first matching literal local Animation targeting each owning named control and combines targets with matching timing. Additional animations, other selectors, independent timing, unresolved bindings and custom animation objects remain preserved in source. Natural playback completion respects fill modes; Stop restores base values. The reusable animation engine provides strict typed import/export, repeat/reverse, spline and named easing, colors, points, thicknesses and discrete values.

## MCP

The sample starts no server until requested. HTTP uses the existing authenticated loopback transport
and reports its endpoint and bearer token to stderr. Writes require `--mcp-write`:

```sh
cargo run -p designer-studio -- --mcp
cargo run -p designer-studio -- --mcp-port 0 --mcp-write
cargo run -p designer-studio -- --mcp-stdio --mcp-write
```

`--mcp-stdio` reserves stdout for the protocol before startup. The designer tools share the same
document transactions and revision guards as the native UI. See the [full designer guide](../../docs/designer/README.md)
for tool schemas, collaboration semantics and extension contracts.

## Embedding

```rust,ignore
let workspace = ferroui_designer::DesignerWorkspace::new(source)?;
window.set_content(Some(std::rc::Rc::new(workspace.control())));
// Retain `workspace` for the view lifetime. No server starts just by constructing it.
let session = workspace.session();
session.borrow_mut().set_property(&[node_id], "Width", "240")?;
```

The library uses the existing FerroUI inspector and debug hot reload transactions. A release build uses a staged runtime load before replacing the preview; the upstream hot reload package is development-only. The library does not choose a rendering backend.

For a platform-independent startup smoke run:

```sh
FERROUI_SMOKE_EXIT_MS=1500 cargo run -p designer-studio \
  --no-default-features --features headless,mcp -- --headless
```
