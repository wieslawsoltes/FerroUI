# Designer Studio

A native FerroUI visual XAML designer built from reusable editing, preview, animation,
project, language and collaboration components. The sample owns application startup; its
workspace and engines are embeddable.

```sh
cargo run -p designer-studio
cargo run -p designer-studio -- --file path/to/View.xaml
```

For a platform-independent startup smoke run:

```sh
FERROUI_SMOKE_EXIT_MS=1500 cargo run -p designer-studio \
  --no-default-features --features headless,mcp -- --headless
```

## Preview hosts

The artboard runs real FerroUI controls. Source errors preserve the accepted preview and leave
the authored text available for correction. Restoring an incomplete document tab restores that
tab's own last-valid preview.

A framework `Window` root has a content projection inside the artboard. **Native window** opens
the actual platform Window with its title, decorations and window properties. The original
Window syntax and `x:Class` remain in the document. The default artboard instantiates the declared
control type; an embedding application can configure `PreviewHostOptions` to register custom
control metadata and factories that create real application/code-behind instances, populate them
from the current XAML, and connect application behavior. These factories remain active across
source reloads and document switches. A failed factory retains the accepted view.

Resource/style/template documents can use native `Design.PreviewWith` or an application-supplied
resource preview factory. Authored nested and shadowed XML namespaces are preserved and supported
by both the runtime loader and compiled XAML pipeline.

## Visual editing

- Pan with the wheel, middle drag or Space+left drag; Ctrl/Cmd+wheel zooms around the pointer.
  **Pan** selects the persistent hand tool. The zoom **…** menu provides percentages, fit
  selection and the alternate Blend wheel mode. Each tab retains its camera. See the
  [canvas navigation guide](../../docs/designer/canvas-navigation.md) for the complete input contract.
- Drag an asset onto a panel or pick an asset and click an insertion location.
- Select in the artboard or source hierarchy; Shift selects multiple controls. Drag the selection,
  use the eight size handles, or rotate around its actual transform origin. Affine manipulation
  accounts for scaled, skewed and mirrored ancestors. Release commits one history item; Escape or
  lost pointer capture cancels it.
- The layout registry provides panel-specific operations, including Grid cells/tracks, Dock and
  RelativePanel relationships, flow ordering and alignment/distribution. Native measurements and
  resolved style/binding values inform those operations. Registered custom adapters are supported;
  custom native panels also have a transform-based manipulation fallback that retains their own
  layout algorithm. Generated item containers remain owned by their item control/template.
- The property panel exposes registered live properties, authored values and value origins, with
  search and custom editor registration. Enter applies a value. An empty value authors an explicit
  empty string; **Reset** removes the authored value. Bindings and resources remain XAML expressions.
- Source edits update the document immediately and debounce the preview reload. Source, property,
  layout, hierarchy and animation changes share transactional undo/redo.
- Source, visual and logical outline modes inspect the authored tree or the live runtime trees.
  Native template children appear as runtime rows.
- **Interact** disables the design overlay and enables runtime interaction. **Select** returns to
  authoring. The preview theme and workspace theme are independent.

## Projects and source tools

Enter a Cargo workspace/project folder or `Cargo.toml` path and choose **Solution**. The explorer
lists projects and documents; tabs retain each document's source, history and saved baseline.
**Check**, **Build** and **Run** execute supervised Cargo tasks with output and cancellation.
**Save all** checks external file changes before replacing local files.

The native XAML editor uses the shared language service for completion, diagnostics, definitions,
references, rename and formatting. Ctrl/Cmd+Space requests completion; F12 navigates to a definition,
Shift+F12 finds references, and Ctrl/Cmd+Shift+F formats. Refactorings validate every affected buffer
before applying a multi-document transaction. The same service provides schema extension,
semantic tokens, folding and a standalone LSP for external editors.

## Animation, states and behaviors

The animation picker lists independent source programs, including programs with different timing
and selectors that match multiple controls. Create, rename, duplicate or remove a program, edit its
selector/settings/XAML, or select explicit preview targets for a resource animation. Switching the
active program leaves the other programs intact. **Play all** previews independent programs together.

Enable **Record** and edit properties at different scrubber times. Recording writes native
`Style.Animations` to the document. **Add keyframe** records the selected control's live opacity at
the playhead; property recording also handles typed animated values. Drag keyframe diamonds or edit
times, remove keys, and enter a per-key curve (`LinearEasing` or `x1,y1,x2,y2`). **Apply curve** and
Enter commit that curve. Changing duration rescales the program's keys consistently in both the
main timeline and settings panel.

Settings include delay, delay between iterations, speed, repeat, direction, fill, playback behavior,
named/spline easing and native spring easing. Typed authoring covers numeric, color and geometry
values. Application animation objects remain in source, and registered codecs can extend typed
authoring. Playback completion respects fill modes; **Stop** restores the authored base values and
bindings rather than recording preview samples into the source.

**States** edits named groups, states, setters and directional transitions. **Behaviors** edits
native triggers/actions and their source. Both use real runtime styles, animations and
`Interaction.Behaviors`, with source/history/collaboration transactions. Previewing behavior that
requires native attachment uses the actual rooted preview; detached API hosts receive an explicit
diagnostic. Application code owns the side effects of its own actions.

## Sharing

**Share** can host a document, join an invitation or resume a saved session. Choose a private session
directory and display name. A fixed loopback bind address supports reconnecting to a restarted host;
an optional public HTTPS endpoint advertises a host behind the operator's TLS proxy. See the
[deployment guide](../../docs/designer/deployment/README.md).

Owners can create editor/viewer invitations, change roles, revoke members and checkpoint the
synchronized current source. Incoming snapshots require an explicit review: use the remote source,
restore a saved draft, or publish the edited resolution. The controller retains backups and the
outbox before replacing a document. Network and persistence work runs on a worker thread. Local
cursors and selection outlines use artboard coordinates, independent of zoom and scrolling.
Viewers can inspect and play previews while authored edits are rejected.

## MCP

The sample starts no server until requested. HTTP uses the existing authenticated loopback
transport and reports its endpoint and bearer token to stderr. Permissions are explicit:

```sh
cargo run -p designer-studio -- --mcp
cargo run -p designer-studio -- --mcp-port 0 --mcp-write
cargo run -p designer-studio -- --mcp-stdio --mcp-write --mcp-input
```

`--mcp-write` enables authored edits. `--mcp-input` enables input simulation; native behavior
execution and project build/run actions require both permissions. Permission flags require a
server option. `--mcp-stdio` reserves stdout for the protocol before application startup.

The workspace provider exposes the same revision-guarded document, layout, property, animation,
state/behavior, project and language operations as the native UI. The standalone room service is a
separate MCP service endpoint with no application inspector/input/log access. See the
[full designer guide](../../docs/designer/README.md) for the tool catalog and extension contracts.

## Embedding

```rust,ignore
let workspace = ferroui_designer::DesignerWorkspace::new(source)?;
window.set_content(Some(ferroui_controls::Control::boxed(workspace.control())));
// Retain workspace for the view lifetime. Construction does not start a server.
let session = workspace.session();
session.borrow_mut().set_property(&[node_id], "Width", "240")?;
```

Use `workspace.serve_workspace_mcp(options)` when project and language tools should be available;
a session-only provider remains available for hosts without the workspace UI. Retain the returned
handle on the UI thread. Configure previews through `session.configure_preview(options)` and
extend layout/property editors through the registries and editor factories.

The library reuses the existing FerroUI inspector and development hot-reload transactions. Its
runtime-document reload mode accepts structural edits only after all target commits succeed;
compiled/application reload sessions retain their original restart protection. Release hosts stage
a runtime load before replacing the preview. The workspace does not select a rendering backend.
