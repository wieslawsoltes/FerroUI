# Designer workflow comparison and implementation

This review compares the XAML-relevant workflows in Visual Studio/Blend, Figma,
Qt Widgets Designer and Xcode previews against the native FerroUI designer. The
[multi-view prototyping review](prototyping.md) also covers Penpot, Android Studio,
Xcode storyboards and explicitly historical Qt Design Studio/SketchFlow references.
The implementation extends the
existing session, source-preserving AST, native preview, property factory chain and layout
adapters. It does not add a second document model or another rendering stack.

## Findings

The [multi-view prototyping review](prototyping.md) adds searchable current-document/all-view
stage browsing, named flow starts, source-anchored Navigate/Back interactions, a reachable-screen
connection overview, detached native editors and a native player with its own retained input.
The next addition presents native dialog, sheet and help stages as a bounded modal stack:
Open, Swap and Close overlay actions, nine placements or manual coordinates, optional
outside dismissal and dimming, retained lower controls and focus restoration. Back closes
the top layer before consuming screen history. This is an explicit FerroUI contract,
rather than a claim of identical Figma or platform-specific sheet navigation.
Stage sets persist overlay graphs as version 3, legacy Navigate/Back graphs as version 2
and empty graphs as version 1. Stage indices remap to fresh runtime IDs on load.
The [prototype sample](samples/PrototypeFlow.xaml) exercises Home, Details and Settings,
including a shared settings screen that returns to the actual previous screen. Prototype
navigation remains presentation state and does not author application event handlers.
The [overlay sample](samples/PrototypeOverlays.xaml) uses a document workspace and three
reusable template stages to exercise nested confirmation, swapping, explicit close and
outside dismissal while retaining live text in the lower views.

The [Xcode preview review](xcode-previews.md) adds a native board of independent named
document and definition stages, size/theme/orientation/direction comparisons, pinning across
source tabs, per-stage Live/Select and pause/refresh, native visual-state scenarios and saved
stage sets. The next set adds native content-fitting policies, direct resizing that retains
Live controls, per-card zoom, bounded comparison matrices and PNG capture from accepted
native views. These extend the existing preview environment and host contracts. Phone sizes
are logical layout constraints; platform simulators and Dynamic Type need separate native
runtime support. The [preview-stage sample](samples/PreviewStages.xaml) exercises four screen
states, theme dictionaries, design-time rows and a reusable template.

The [selection and appearance review](selection-workflows.md) adds compatible authored-property
transfer, grouped layout reset, measured size matching, an explicit overlap picker and exact
native-type selection. It also makes the shared resize planner preserve untouched dimensions,
bindings and aliased Canvas anchors. Existing outline range selection remains provided by the
native TreeDataGrid.

The further [binding, inspector and precision-layout review](authoring-workflows.md) adds
structured binding sources/options, inspector arrangements and value-source filters, source
navigation, native ruler guides with configurable snapping, and reference-aware element naming.
These extend the earlier workflow results below.

| Workflow | Reference behavior | FerroUI before this review | Result of this review |
| --- | --- | --- | --- |
| Source/design synchronization | Visual Studio/Blend retain XAML while providing a visual editor | Lossless AST, atomic source edits, native reload, diagnostics and language service | Reused by every new authored command |
| Layout manipulation | Figma alignment/distribution; Blend and Qt container-aware layout editing | Multi-selection, affine movement/resize, alignment/distribution, source order, nesting and panel-specific helpers | Layer locks/isolation now guard these existing paths, including cached drops and tree drags |
| Shared definitions and instances | Blend template/style scopes; Figma component/instance ownership | Component Library, template/theme scopes, edit-current/edit-copy, resource dependencies | Existing workflows retained; generated template controls are explicitly distinguished in focus editing |
| Typed and extensible property editing | Blend inline/extended editors; Qt property-sheet extensions | Shared native typed fields, staged values/bindings/resources/objects, custom factories | New preview and navigation editors use the same session ownership and snapshot rules |
| Authored appearance transfer | Figma compatible property paste; Qt editing shared properties across selected objects | Typed property editing, resources and reusable definitions | Added a workspace appearance clipboard, selective staged paste, native destination compatibility and one source transaction |
| Selection layout commands | Blend selective layout reset; Figma dimension editing and Qt shared-property editing | Individual property resets and native panel-aware resizing | Added grouped resets and matching to the primary control's logical size, preserving untouched bindings and parent layout semantics |
| Explicit and matching selection | Figma under-cursor layer menus and matching-object/property selection | Native overlap cycling, hierarchy navigation and outline range selection | Added a named overlap menu and exact control-type selection within the active authored scope |
| Animation and interaction | Blend timelines/states/behaviors; Figma prototype transitions; Qt connections | Native animation tracks, states/transitions, behavior event/data triggers and actions | Existing native authoring remains the application-behavior model |
| Design-time values and sample data | Visual Studio's ignorable `d:` properties, controls and generated `SampleData` | Loader recognized selected design directives, but no connected designer projection/editor | Added namespace-aware design projection, staged Design Data editor, bounded anonymous sample records and native source mapping |
| Layer lock/visibility and selection depth | Figma lock, hide and nested layer selection | Portable selection flags existed but were not connected to the native surface | Added native row/menu actions, local visibility masks, isolation, hierarchy navigation and overlap cycling |
| Tab order | Qt numbered tab-order mode | `TabIndex` was editable as a scalar; no navigation model/editor | Added native navigation-group catalog, staged order editor, mode editing and canvas badges |
| Label/input associations | Qt buddy connections | Native `Label.Target` existed without designer authoring | Added namescope-correct target picker, connection overlay and atomic target-name/binding creation |
| Preview environment | Figma device sizes/orientation; Qt form previews; native theme scopes | Artboard-size API and a binary theme toggle | Added staged size presets, custom dimensions, orientation, inherited/light/dark theme and reading direction |
| Multiple named preview views | Xcode named previews, side-by-side variants and pinned preview tabs | One editing preview, environment settings, definition scopes, native states and registered factories | Added independent native document/definition stages with variant sets, state endpoints, source identity, pin/pause controls and versioned persistence |
| Preview layout and resizing | Xcode size-that-fits traits and arbitrarily resizable canvas | Fixed-size stages with an environment editor | Added native finite-proposal fitting on either axis, direct resize drafts, measured size reporting and per-card display zoom while retaining Live control graphs |
| Preview argument combinations | Xcode parameterized argument groups | One-axis environment comparisons and manually selected native states | Added bounded combinations of size, theme, direction, design-data and authored state axes, with complete validation before stage creation |
| Preview screenshot export | Xcode canvas screenshot and save sheet | Native RenderTargetBitmap rendering existed only in test captures | Added source-guarded PNG stage capture, native save picker and read-only automation, with pixel/byte bounds and independent raster density |
| Many-view organization | Figma pages/sections and named flows; Penpot searchable screen destinations | Named stages, document pinning and source tabs | Added stage search, current-document/all-view browsing and a reachable-screen connection overview |
| Interactive screen prototypes | Figma/Penpot flow starts and hotspots; Android graph/back-stack separation; Xcode unwind navigation | Independent native stage graphs and source anchors, without connected journeys | Added named flows, source-anchored Navigate/Back, detached native editors, independent player graphs with retained input and versioned graph persistence |
| Dialog and sheet prototypes | Figma/Penpot overlays; WinUI dismissal/focus; Apple modal presentation guidance | Retained native stages and a separate screen-history cursor | Added Open/Swap/Close overlay, bounded modal stacking, nine placements/manual coordinates, outside-dismissal and dimming options, focus restoration, base-frame clipping and v3 persistence; see the [overlay contract and primary references](prototyping.md#research-and-design-choices) |
| Canvas navigation and framing | Figma pointer-anchored zoom, fit selection and file-local views; Blend wheel zoom and Space panning | Zoom buttons and a centered scroll container; no independent camera or fit-selection command | Added a finite native viewport, wheel-mode choice, middle/Space/hand-tool panning, percentage editing, selection framing and retained tab cameras |
| Compact command icons | Visual Studio's centered, padded and consistently weighted icon frames | Mixed compact paths and symbol-font glyphs under application command templates | Designer-scoped vector normalization fits complete geometry, reserves padding and follows themed foreground |

The comparison maps concepts to FerroUI's native contracts. Figma variables map to existing
resource/theme authoring, and application interaction uses native bindings, behaviors,
states and navigation. Qt's C++ signal/slot syntax and Figma's scene-file format are not
introduced into XAML. Foreign plugin ABIs, operating-system emulation and arbitrary Rust
code generation are separate concerns from this authoring layer.

## Canvas navigation and compact icons

The canvas now uses a single finite render transform with reusable artboard/viewport
coordinate conversion. Wheel input pans by default; Ctrl/Cmd+wheel zooms around the pointer.
The zoom options also offer Blend-style plain-wheel zoom. Middle drag, temporary
Space+left drag and the persistent Pan tool share the same camera. Shift+1 fits the artboard,
Shift+2 fits the native selection, and Ctrl/Cmd+0 restores actual size. The options flyout
accepts custom percentages from 1% to 1600% and exposes both wheel modes.

Document tabs retain independent views, and returning from a definition scope restores the
document camera. Unmodified wheel input reaches application controls in Interact mode.
Camera changes leave source, undo and collaboration edits untouched and retire unfinished
authored canvas gestures before changing their coordinate frame.

Designer icons use a scoped vector template that fits their complete geometry uniformly,
centers it with a logical-pixel gutter, and follows themed foreground. Application preview
icons keep their own theme semantics. See [canvas navigation and designer icons](canvas-navigation.md)
for the input contract, public APIs, icon treatment and primary reference comparison.

## Design-time data

Open **Data** on the canvas tool shelf. The editor can toggle sample visualization, stage
common design properties, author a scalar/expression or an object-property value, reset an
override, and generate sample items. Applying authored values is one source transaction.
Cancel or dismissal leaves the document untouched. Existing application values and
bindings retain their original spelling.

```xml
<StackPanel xmlns="https://github.com/ferroui"
            xmlns:d="http://schemas.microsoft.com/expression/blend/2008"
            xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
            mc:Ignorable="d" d:DesignWidth="480" d:DesignHeight="680">
  <TextBox Text="{Binding Name}" d:Text="Alex Morgan" />
  <ListBox ItemsSource="{Binding Projects}"
           d:ItemsSource="{d:SampleData ItemCount=4}">
    <ListBox.ItemTemplate>
      <DataTemplate>
        <StackPanel>
          <TextBlock Text="{Binding Title}" />
          <TextBlock Text="{Binding Owner.Name}" />
        </StackPanel>
      </DataTemplate>
    </ListBox.ItemTemplate>
  </ListBox>
</StackPanel>
```

The implementation resolves namespace URIs rather than assuming the prefix `d`. It
preserves existing namespace aliases and compatibility declarations. Scalar and property
element overrides are distinct semantic commands. Design-only controls and legacy native
`Design.Width`, `Design.Height` and `Design.DataContext` are projected consistently.

The preview projection has its own syntax-to-source identity map. It composes with
definition projections so inserting or masking a design-only property cannot move a
runtime object onto the wrong authored node. Preview factories continue receiving exact
authored source in `PreviewRequest.source`; `preview_source` supplies the disposable
design projection used by `populate`.

Generated sample data uses immutable anonymous records with a dedicated native binding
accessor. It infers simple and nested binding paths from item templates and creates
deterministic values. Generation is bounded to 200 items. It does not construct an
application view model. Complex paths, custom conversions, image assets and domain-specific
values should use explicit design overrides or registered application data types.

The **Use design size** action in Preview environment explicitly adopts root
`d:DesignWidth` and `d:DesignHeight` hints. Those hints leave the root unconstrained so
responsive presets can resize it; explicit `d:Width` and `d:Height` remain real property
overrides. Switching sample visualization changes the preview without adding an undo entry.

Native constructors, registered preview factories and initial template realization run
inside a scoped design-mode flag, restored on success, failure or unwind. The loader's
explicit design-mode configuration is respected. This allows custom controls to recognize
their preview environment without leaving the surrounding application in design mode.

Reusable entry points include `DesignTimeSnapshot`, `DesignTimeEdit`,
`plan_design_time_edits`, `DesignTimeEditor`, and `DesignerSession` design-data methods.
The snapshot rejects a retained dialog after a source, selection, document or definition
scope change. Normal source edits retain collaboration and read-only role enforcement.

## Layer presentation and selection

Source Layers rows expose lock and visibility controls. The layer action menu also provides
isolation, exit isolation, show all, unlock all and hierarchy selection. Source rows remain
inspectable so a hidden or locked control can be deliberately edited or restored.

| Interaction | Behavior |
| --- | --- |
| Lock / unlock | Prevent canvas and hierarchy movement of the layer and its descendants |
| Hide / show in editor | Suppress painting and hit testing while preserving measured layout and authored values |
| Isolate selection | Keep selected subtrees and necessary ancestor context; hide unrelated visual branches |
| Ctrl/Cmd+Shift+L | Toggle the selected layers' locks |
| Ctrl/Cmd+Shift+H | Toggle selected layers' editor visibility |
| Ctrl+Alt+click | Cycle the front-to-back authored hit stack |
| Right-click → Select layer | Choose an authored layer from the native hit stack without changing selection when the menu opens |
| Layers → … → Select same control type | Select visible, unlocked controls with the primary selection's exact native type in the active authored scope |
| Enter / Shift+Enter on the canvas | Select children / parent |
| Tab / Shift+Tab on the canvas | Select next / previous authored sibling |
| Escape | Cancel an active gesture, or exit isolation |

These are local editor preferences. They never write `IsVisible`, `Opacity`, source
history or collaboration operations. Hiding uses the renderer's counted render
suppression, honored by both live composition and exported bitmaps, plus disposable
hit-test values, so animation and binding updates cannot
accidentally reveal a hidden layer. Releasing the designer's suppression does not release
another subsystem's suppression request.

Retained document tabs store `LayerViewSnapshot` alongside the document. It restores only
into the corresponding source allocation and URI; stale metadata is retired without
preventing the document from opening. Each definition scope retains its own isolation.
A monotonically changing layer epoch invalidates pending gestures even after lock/unlock
returns to the previous visible state.

`LayerState`, `LayerCommand`, `LayerEditSnapshot`, `LayerViewSnapshot` and the session APIs
are reusable by hosts. `DesignSurface::hit_stack` exposes the actual authored overlap stack
for alternative selection UIs. Geometry planners, layout helpers and outline drops enforce
the same presentation restrictions before committing.
The [selection workflow guide](selection-workflows.md#select-an-overlapping-layer) describes
the explicit picker's retained-context guards and the scope of control-type matching.

## Keyboard navigation and label targets

Open **Focus** on the canvas shelf. The editor inspects actual native navigation groups,
shows their effective order and allows changes to be staged together. Apply commits one
undo transaction; Cancel retires the draft. Canvas badges show the group and rank, and
label connection lines show existing targets.

FerroUI's native modes are represented directly:

| Mode | Meaning in the editor |
| --- | --- |
| Continue | Children share the enclosing group's index space |
| Local | This group has a separate index space |
| Cycle | Tab wraps within the group |
| Contained | Tab stops at the group's boundary |
| Once | Entry selects one child; the next Tab leaves the group |
| None | Children are excluded from Tab navigation |

The catalog respects native child-order overrides, effective `TabIndex`, enabled/visible
state, `Focusable` and `IsTabStop`. Excluded entries remain inspectable. Generated template
controls are read-only; a scope containing such entries cannot be flattened into a new
authored order. Template parts are authored inside their definition scope.

Reorder accepts a complete permutation within one native group and validates it before
writing. Bound/object indices are preserved until explicitly reset or edited in source.
Reset removes authored values and lets native styles/defaults determine the resulting
order. The editor does not assume that every reset returns to simple source order.

The label picker uses the real XAML namescope. Associating an unnamed input generates a
collision-free name and `Label.Target` ElementName binding together. Removing a target
removes only the authored target value. The native label/access-key implementation owns
focus behavior.

`FocusOrderCatalog`, `FocusOrderSnapshot`, `FocusOrderEdit`, `FocusOrderEditor` and the
session methods provide the embedding contract. An accepted snapshot includes native
navigation semantics as well as source identity; a runtime-only change can retire a
pending dialog even when the XAML revision has not changed.

## Preview environment

The **…** button beside the canvas dimensions opens a reusable staged environment editor.
Presets cover generic phone, tablet and desktop logical sizes. Width and height are
validated together; swapping orientation and adopting XAML design size are explicit
actions. These sizes test native layout constraints and do not claim device hardware or
DPI emulation.

Theme can be Light, Dark or inherited from the application. Reading direction can be
inherited, left-to-right or right-to-left. Both apply under the preview's own native scope,
so designer chrome is unchanged and explicit document values retain native precedence.
The artboard backing tracks subsequent inherited theme changes.

`PreviewEnvironment` and `PreviewEnvironmentEditor` are public. The workspace's
`set_preview_environment`, `rotate_preview` and existing `set_artboard_size`/
`set_preview_dark` methods share the same state and validation. Environment changes are
available to viewers and preserve pending source text, undo, selection and collaboration.

## Automation

| MCP tool | Contract |
| --- | --- |
| `designer_design_data` | Inspect overrides, enablement and design dimensions |
| `designer_design_data_edit` | Set/reset a scalar or object override, configure sample count, toggle visualization |
| `designer_focus_order` | Read the native groups/labels and an ephemeral model token |
| `designer_focus_edit` | Apply an order, reset, focus/tab-stop/navigation change or label target against the accepted model |
| `designer_layers` | Read local layer flags, isolation and epoch |
| `designer_layer` | Change local presentation or hierarchy selection against the accepted layer and preview-scope epochs |

All actions have strict schemas and the existing tool write permission. Authored commands
also enforce document revision and collaboration roles. Focus edits require the current
native model token; layer edits require the current presentation and preview-scope epochs. The tokens are
concurrency checks, not authentication credentials. Preview-only changes do not create
source history.

## Source editing

The language service recognizes the exact Blend namespace through any in-scope alias.
It completes native control types, scalar and object-property overrides, and
`SampleData`/`ItemCount`, including signature help and bounded item-count diagnostics.
Explicit design data-context objects supply the same typed binding completions as runtime
contexts. `mc:Ignorable` is recognized through its exact compatibility namespace.

Static attached-property owners are now included in the generated framework schema, so
`KeyboardNavigation.TabNavigation`, `Design` properties and other native attached members
are discovered from actual Rust declarations. Unrelated application namespaces keep their
ordinary type and member validation.

## Run and verify

```sh
cargo run -p designer-studio -- --file docs/designer/samples/DesignWorkflows.xaml
```

The sample combines runtime bindings with design values, generated item-template data,
real label targets, navigation groups and responsive layout. Native tests cover namespace
aliases, projection identity, factories, samples, roles, stale models, source preservation,
undo, layer visibility restoration and preview inheritance. The PR validation section
records the exact completed test runs and CI status.

## Native render examples

These captures come from the real WinUI-themed FerroUI workspace, shaped with HarfBuzz
and rendered through Vello's CPU backend. The fixture checks native preview readiness,
source-language diagnostics, generated records and source/history preservation.

![Design-time values alongside the original runtime binding](images/designer-design-data-preview.png)

![Native navigation group, order editor and canvas badges](images/designer-focus-order.png)

Additional views: [Design Data editor](images/designer-design-data-editor.png),
[layer isolation](images/designer-layer-isolation.png),
[preview environment editor](images/designer-preview-environment.png), and
[dark, right-to-left landscape preview](images/designer-responsive-rtl-preview.png).

## Primary references

- [Xcode canvas modes and preview variants](https://developer.apple.com/documentation/xcode/interacting-with-previews-in-the-canvas)
- [Xcode named previews and sample context](https://developer.apple.com/documentation/xcode/adding-previews-to-your-interface-files)
- [Xcode 27 preview groups and independent pinning](https://developer.apple.com/documentation/xcode-release-notes/xcode-27-release-notes)
- [Xcode content-fitting preview traits](https://developer.apple.com/documentation/developertoolssupport/previewtrait/sizethatfitslayout)
- [SwiftUI parameterized preview arguments](https://developer.apple.com/documentation/swiftui/preview(_:traits:arguments:body:))
- [Xcode 15.3 preview screenshot export](https://developer.apple.com/documentation/xcode-release-notes/xcode-15_3-release-notes)
- [Visual Studio design-time data](https://learn.microsoft.com/en-us/visualstudio/xaml-tools/xaml-designtime-data?view=visualstudio)
- [Visual Studio generated sample data](https://learn.microsoft.com/en-us/visualstudio/xaml-tools/xaml-design-time-sample-data?view=visualstudio)
- [Blend styles, templates and resource scopes](https://learn.microsoft.com/en-us/visualstudio/xaml-tools/modify-the-style-of-objects-in-blend?view=visualstudio)
- [Blend artboard navigation shortcuts](https://learn.microsoft.com/en-us/visualstudio/xaml-tools/keyboard-shortcuts-in-blend?view=visualstudio)
- [Visual Studio icon layout and compact-size guidance](https://learn.microsoft.com/en-us/visualstudio/extensibility/ux-guidelines/images-and-icons-for-visual-studio?view=visualstudio)
- [Figma zoom, fit selection and file-tab-local view settings](https://help.figma.com/hc/en-us/articles/360041065034-Adjust-your-zoom-and-view-options)
- [Figma canvas panning](https://help.figma.com/hc/en-us/articles/30925881896727-FD4B-Navigate-Figma-Design-files)
- [Figma lock and unlock layers](https://help.figma.com/hc/en-us/articles/360041596573-Lock-and-unlock-layers)
- [Figma layer visibility](https://help.figma.com/hc/en-us/articles/360041112614-Toggle-visibility-to-hide-layers)
- [Figma device and orientation settings](https://help.figma.com/hc/en-us/articles/21158597546391-Set-prototype-device-and-background-settings)
- [Figma responsive prototype previews](https://help.figma.com/hc/en-us/articles/360040318013-Play-your-prototypes)
- [Qt Designer editing modes](https://doc.qt.io/qt-6/designer-editing-mode.html)
- [Qt tab order](https://doc.qt.io/qt-6/designer-tab-order.html)
- [Qt label buddies](https://doc.qt.io/qt-6/designer-buddy-mode.html)
- [Qt form previews](https://doc.qt.io/qt-6/designer-preview.html)
- [Qt property-sheet extensions and undo](https://doc.qt.io/qt-6/qdesignerpropertysheetextension.html)
