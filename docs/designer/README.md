# FerroUI Designer Studio

Designer Studio is a native FerroUI XAML authoring workspace and a set of reusable Rust engines.
The workspace combines an artboard, synchronized source editor, object hierarchy, live visual and
logical trees, properties, layout manipulation, and animation tracks. All authored changes pass
through a document transaction; the artboard renders actual FerroUI controls.

![Native workspace with a recorded Width animation at 1.5 seconds](images/designer-animation.png)

This image is captured from the native Vello/HarfBuzz rendering test. The property inspector shows
the authored Width of 656 and its animated value of 608 while the source retains the base value.
The [initial workspace capture](images/designer-workspace.png) shows the same document before edits.

## Run

From the repository root:

```sh
cargo run -p designer-studio
cargo run -p designer-studio -- --file /absolute/path/to/MainWindow.xaml
```

The desktop host uses the normal FerroUI desktop backend. A portable host can embed
`ferroui-designer` without taking a desktop backend dependency. A headless sample configuration
is available for automated hosts:

```sh
cargo check -p designer-studio --no-default-features --features headless,mcp
```

The workspace supports Window, UserControl and other Control roots. Register application-specific
control types and resource providers before loading their documents, just as an ordinary FerroUI
application does. Use `DesignerWorkspace::with_uri` when relative includes and resources need the
original document location. A raw resource dictionary or style document needs a host that supplies
an appropriate preview control; it is not a standalone control artboard.

Window documents preview their content through a Border projection, because a native Window cannot
be embedded inside another control's visual tree. Window chrome settings and the root `x:Class`
declaration remain in the authored source; the preview masks them and reports projection notices.
It does not instantiate the application's code-behind class.

Open, Save and Save as use the host's existing native storage provider. Opening or closing a dirty
document presents a save/discard/cancel choice. Local saves write a temporary sibling and replace
the destination, checking that an existing file has not changed externally. Virtual storage files
use their provider's stream API. The workspace tracks the exact saved source so subsequent edits
remain dirty.

## Crates and ownership

| Crate | Owns | Runtime dependency |
| --- | --- | --- |
| `ferroui-designer-core` | Lossless XAML syntax, source spans, stable local IDs, atomic edits, revisions and history | None |
| `ferroui-designer-tools` | Layout adapters, property metadata/editor contracts, geometry, selection and manipulation planning | None |
| `ferroui-designer-animation` | Typed keyframes, easing, deterministic sampling, playback and native animation XAML interchange | None |
| `ferroui-designer-collaboration` | Replicated operations, shared identities, conflict reporting, snapshots, version summaries and presence | Document core |
| `ferroui-designer` | UI-thread session, live preview, native controls, property editors, design surface, timeline and optional MCP tools | FerroUI runtime |
| `designer-studio` | Application lifetime, document IO, desktop/headless startup and opt-in MCP hosting | Workspace and selected platform |

These boundaries let a command-line tool or service manipulate documents without loading the
windowing, rendering, inspection or hot reload stack. The UI library starts neither a process nor
a network service when it is linked.

## Embedding the workspace

After ordinary FerroUI application and theme initialization:

```rust
use ferroui_controls::{Control, Window};
use ferroui_designer::DesignerWorkspace;

let workspace = DesignerWorkspace::new(source)?;
let window = Window::new();
window.set_content(Some(Control::boxed(&workspace.control())));
let session = workspace.session(); // Rc<RefCell<DesignerSession>>, owned by this UI thread
```

Keep the workspace alive for the lifetime of its view. Hosts can register native property editor
factories and portable layout adapters. The source buffer is available through
`session.borrow().document().source()` for the host's storage workflow.

The facade also exposes `ferroui::designer` with its `designer` feature; `designer-mcp` adds the
optional MCP adapter. Neither feature selects a windowing or rendering backend.

## Source, syntax and preview are separate states

There are three relevant values:

1. **Authored source** is what the user typed. It can temporarily be invalid.
2. **Last valid syntax** supplies the source AST and document-local node identities.
3. **Accepted runtime preview** is the last revision FerroUI successfully loaded and applied.

A malformed tag does not delete the authored text. A well-formed but unknown control does not
replace a working preview. Visual manipulation requires the accepted preview to match the
current valid document revision; this prevents stale geometry from writing changes into newer
source. Source edits can be accepted immediately while expensive preview work is debounced.

Debug builds reuse `HotReloadSession` and `ContentReloadTarget` for accepted preview changes.
Release builds use a staged runtime XAML load and replace the preview only after loading succeeds;
the existing hot reload package is development-only. Both paths reuse FerroUI's runtime loader and
retain the last accepted preview on failure.

The AST retains comments, CDATA, processing instructions, namespaces, attribute spelling and
quotes, text nodes, property elements, and markup expressions. A parse followed by no edits
preserves every byte. Visual changes use UTF-8 source patches rather than reformatting the whole
file. Namespace-aware reparenting preserves the meaning of moved elements.

The engine validates XML structure. FerroUI's existing XAML loader resolves control types,
properties, resources, bindings, templates and converters. Unknown syntax is preserved by the
editing engine even when the runtime cannot preview it.

### One transaction for every authoring path

```rust
use ferroui_designer_core::{Document, Edit};

let mut document = Document::new(
    r#"<Canvas><Button Name="Action" Width="100" /></Canvas>"#,
)?;
let button = document.ast().find_name("Action").unwrap();
let expected = document.revision();
let change = document.apply_at(expected, "Resize button", vec![
    Edit::SetAttribute { node: button, name: "Width".into(), value: "180".into() },
    Edit::SetAttribute { node: button, name: "Height".into(), value: "40".into() },
])?;
assert_eq!(change.previous_revision, expected);
document.undo()?;
document.redo()?;
```

An invalid edit in a batch rolls back the entire candidate. Revision guards reject stale
commands. Pointer gestures preview transient geometry and commit one history item at completion.
Source history includes invalid drafts, so undo restores what the user actually authored.
Immutable AST/source snapshots share storage through `Arc`; property batches coalesce assignments
before reparsing. The core tests exercise a 10,000-element document.
Source offset mapping uses a prefix-delta table and binary search for batch patches; a separate
4,096-element mixed-property transaction verifies stable syntax identities and undo/redo.

## Property and layout extension points

The native inspector exposes registered writable properties, attached properties and authored
attributes. Editors distinguish an explicitly empty value from an inherited value, support mixed
multi-selection values, and route reset operations through source editing. Runtime-only generated
objects can be inspected without pretending that they have an authored XAML node.

A `PropertyEditorFactory` creates a native control from `PropertyEditorContext`; its callback
receives an optional source value, where `None` means reset. The portable `PropertyRegistry` and
`EditorRegistry` supply typed contracts for hosts that use a different UI. Expressions are kept as
expressions. Layout tools refuse to overwrite a binding or resource reference as a side effect of
a drag.

`LayoutRegistry` associates panel types with `LayoutAdapter` implementations. The built-in set
covers Canvas, Grid, StackPanel, ReversibleStackPanel, WrapPanel, DockPanel, UniformGrid,
RelativePanel, FlexPanel and ordinary Panel, with explicit ownership rules for generated
virtualizing and internal panels. Grid track edits preserve definition property elements and
min/max/shared-size metadata. The host supplies measured tracks for Auto sizing. See
[`FerroUI.Designer.Tools/README.md`](../../src/FerroUI.Designer.Tools/README.md) for the exact
capabilities and extension contracts.

The geometry layer supplies affine transforms, snapping, alignment, distribution, marquee
selection, ordered selection and manipulation revision guards. Coordinate conversions must use
actual parent transforms; the portable engine accepts measured geometry rather than guessing a
second layout system. Batch planning validates multiselection operations as one transaction,
including operations spanning different layout parents. The registry recognizes 30 built-in
panel, content-host and framework-layer names; generated containers expose ownership restrictions
instead of accepting edits that their item generator would overwrite.

Resize and rotation gestures reject transforms they cannot faithfully translate, with a message
directing the user to property/source editing. Custom panel semantics and layout values supplied
implicitly by styles can require a host adapter. Duplicate gives copied controls unique names and
refuses occupied single-content hosts; binding/reference expressions are copied verbatim and may
need review when they refer to another control inside the copied subtree.

## Animation authoring

The animation model uses FerroUI's native `Animation`, `KeyFrame` and `Setter` markup. It does not
introduce a second runtime animation dialect. The workspace imports supported animation markup,
records property changes as keyframes, writes them through the document transaction, and samples
preview values at animation priority. Disposable preview layers restore ordinary source/style
values on Stop, preview reload or switching to Interact. Natural playback completion respects
the animation's fill mode and can retain the final sample.

The active workspace timeline imports the first matching local Animation on each named control.
Tracks from different controls are combined when their timing settings match. Additional local
animations, unmatched selectors and independent timing remain in source; the active timeline does
not combine those independent animation programs. The graphical track lanes support scrubbing,
keyframe selection and dragging, with explicit time, easing and removal controls.

Supported values include numbers, colors, points, thicknesses and discrete values. The portable
engine implements linear-light color interpolation, cubic Bézier inversion, named FerroUI easing
families, delay, speed, finite/infinite repetition, direction, alternate playback and fill modes.
Times are seconds in the editing API and native 100 ns times in exported markup. Import/export
checks native duration grammar, timing range, repeat range and keyframe quantization collisions.

A `CompiledTimeline` validates once and can be sampled repeatedly without rescanning all model
validation. Deterministic seek/scrub is independent of a wall clock. The live runtime's property
system remains authoritative about which property types and converters can actually receive an
animation value.

Unsupported custom animator payloads, binding-driven keyframe values and animation structures
that cannot be represented safely remain in authored source with diagnostics. The workspace
must not erase them to make a simplified timeline look complete. See the
[animation crate documentation](../../src/FerroUI.Designer.Animation/README.md) for interchange
rules and runtime semantic tests.

## Collaboration semantics

`DocumentReplica` is an ordered operation-set replication engine over the same lossless document
model. It has stable shared node aliases, actor/sequence identities, Lamport ordering, duplicate
detection, dependency replay, contiguous version summaries and SHA-256 baseline verification.
Every operation is an atomic source transaction. Delivery permutations are tested to converge to
the same accepted source and conflict set.

The session translates its local AST IDs to shared aliases by source correspondence. Those IDs
must not be confused with the inspector's runtime IDs or another replica's local AST IDs.
Structured property/insert/remove/move commands retain their fine-grained semantics. Arbitrary
source replacement uses compare-and-set against the expected source; concurrent text edits can
produce an explicit conflict requiring a merged draft. This is not a character-level text CRDT.

Property compensation checks the last writer's operation identity during replay, including
same-value writes. The workspace's shared history additionally guards whole-source compensation
against a newer peer version. Remote changes never silently discard an unsubmitted local draft.

A host starts a session with `enable_collaboration(actor, document_id)`, exchanges snapshots when
joining, and sends `collaboration_changes_since(version)` to peers. `merge_collaboration` commits
a successfully projected source through normal preview processing. Presence uses a receiver
clock for expiry; it is ephemeral and separate from source history.

The engine opens no sockets and implements no account service. A host supplies authenticated
transport, durable snapshots/log storage, actor identity, reconnection policy and session access.
The MCP adapter can carry synchronization envelopes through the existing authenticated endpoint.
Operation limits are explicit; long-lived sessions need a host-coordinated new baseline instead
of unbounded log growth. See the [collaboration crate documentation](../../src/FerroUI.Designer.Collaboration/README.md).

## MCP integration

The sample enables its server only when requested. It starts with read-only permissions:

```sh
cargo run -p designer-studio -- --mcp
cargo run -p designer-studio -- --file MainWindow.xaml --mcp-port 0 --mcp-write
cargo run -p designer-studio -- --mcp-stdio --mcp-write
```

`--mcp-port` selects the loopback HTTP port (`0` selects an available port). The existing server
reports its endpoint and generated bearer token to stderr. `--mcp-stdio` reserves stdout for the
protocol before application initialization. `--mcp-write` enables authoring commands and requires
an explicit server option; UI input simulation remains disabled by default.

Enable the `mcp` feature of `ferroui-designer` and retain the returned server handle:

```rust
use ferroui_designer::mcp::{Options, Permissions};

let server = workspace.serve_mcp(Options {
    permissions: Permissions {
        allow_property_writes: true,
        allow_input: false,
    },
    ..Options::default()
})?;
```

The existing `ferroui-mcp` transport still owns HTTP loopback binding, bearer authentication,
stdio framing, session negotiation, cancellation, size limits and UI dispatch. The reusable
`UiToolProvider` extension point registers immutable tool descriptions and a UI-thread-owned
provider. Transport threads receive a numeric provider identity and descriptions; they never
carry `Rc<RefCell<DesignerSession>>` or controls across threads. Providers cannot shadow built-in
inspector tool names, and extension registrations are isolated per server.
The original `serve()` API still returns its `Send` server handle. `serve_with_tools()` returns a
separate `UiServerHandle`, which stops its server before disposing the UI provider registration.

The additional tools are:

| Tool | Purpose |
| --- | --- |
| `designer_document` / `designer_ast` | Source, accepted revision, diagnostics, syntax nodes and spans |
| `designer_apply` / `designer_source` | Atomic structured edits or source editing with a required revision guard |
| `designer_select` / `designer_undo` / `designer_redo` | Selection and shared authoring history |
| `designer_tree` | Live visual/logical tree with runtime and source identities |
| `designer_timeline` / `designer_animation` | Inspect, seek, play, record and edit keyframes |
| `designer_collaboration_state` / `designer_collaboration` | Snapshots, version summaries, operations and explicit draft resolution |
| `designer_presence` | Ephemeral peer selection/cursor updates |

For example, after reading revision `7` and source node `12`:

```json
{
  "name": "designer_apply",
  "arguments": {
    "expected_revision": 7,
    "label": "Resize action button",
    "edits": [
      {"SetAttribute": {"node": 12, "name": "Width", "value": "180"}},
      {"SetAttribute": {"node": 12, "name": "Height", "value": "40"}}
    ]
  }
}
```

The built-in inspector tools remain available. Direct runtime `set_property` is an inspection
operation and does not author XAML; use `designer_apply` when a change must persist in source,
history and collaboration.

## Validation

The dedicated [Visual designer workflow](../../.github/workflows/designer.yml) covers:

- Source roundtrips, Unicode spans, namespace moves, invalid drafts, identity and atomic history.
- Layout metadata, measured panel actions, selection, transforms and snapping.
- Native animation timing/interchange and deterministic interpolation.
- Replication delivery order, duplicates, missing dependencies, identity equivocation, baseline
  mismatch, source conflicts, compensation and presence expiry.
- Native property edits, preview retention, Window/UserControl roots, history and collaboration.
- Existing MCP authentication/cancellation plus provider isolation and designer commands.
- Actual Vello/HarfBuzz workspace and animation PNG rendering.
- Headless sample compilation, application startup/shutdown, and the separate release preview code path.

Local commands:

```sh
cargo test -p ferroui-designer-core -p ferroui-designer-tools \
  -p ferroui-designer-animation -p ferroui-designer-collaboration
cargo test -p ferroui-designer --features mcp --lib -- --test-threads=1
cargo test -p ferroui-mcp --lib -- --test-threads=1
FERROUI_DESIGNER_RENDER_OUTPUT=/tmp/designer-renders \
  cargo test -p ferroui-designer --features mcp,render-smoke --lib render_tests:: -- --test-threads=1 --nocapture
cargo check -p ferroui-designer --features mcp --release
```

## Product scope

This is FerroUI's authoring architecture and native workspace, not a statement of complete
Visual Studio/Blend product parity. The source editor currently uses the framework's TextBox;
project/solution systems, language-server completion, a dedicated visual-state/behavior designer,
custom control design assemblies, hosted multiuser infrastructure and pixel parity with those
products are separate work. Templates/resources and unsupported animations remain editable in
XAML. The extension points allow those workflows to share the same document and runtime services
without replacing the engines delivered here.
