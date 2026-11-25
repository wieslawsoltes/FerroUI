# UI semantics in SVG

`ferroui-svg` can preserve an immutable description of the UI that produced a recorded layer.
The painted SVG remains ordinary, self-contained artwork. A versioned XML vocabulary inside
`<metadata>` carries control identity, hierarchy, layout, input state and explicitly selected
content. Optional `data-ui-*` attributes on the existing painted group make the manifest easy to
discover from DOM-based inspection tools.

This supports design inspection, visual-test explanations, accessibility analysis, source
navigation and application-specific design tooling without trying to reconstruct controls from
Skia's glyph outlines and paths. Automatic conversion to Figma components, Illustrator objects or
executable FerroUI markup would require a consumer that understands this vocabulary.

## Capture and export are separate decisions

Ordinary recording and compositor capture do not perform an additional semantic walk or retain a
semantic snapshot. Ordinary exports omit semantics, including when a snapshot has been attached.
Enable capture on the UI thread after layout, then choose the fields to publish at export time:

```rust
use ferroui_svg::{
    SvgDocument, SvgExportOptions, SvgLayerOptions,
    SvgSemanticCaptureOptions, SvgSemanticExportOptions,
};

// `visual` is an already laid-out Ref<Visual>; Skia services are initialized by the host.
let mut document = SvgDocument::new(size)?;
document.record_visual_with_semantics(
    SvgLayerOptions::new("settings", "Workspace settings"),
    &visual,
    &SvgSemanticCaptureOptions::default(),
    None,
)?;

let output = document.export(&SvgExportOptions {
    semantics: SvgSemanticExportOptions::metadata(),
    ..SvgExportOptions::compact()
})?;
output.save("settings.svg")?;
```

`record_visual_with_semantics` first captures and validates the source tree, then records the
whole visual through the existing immediate renderer. A failed snapshot or recording publishes
no partial layer. The customization callback runs once per node after source collection; it must
not mutate the UI. Recording can execute application rendering code, so hosts must keep their UI
stable across capture and drawing. This API does not lock arbitrary application state.

`capture_visual_semantics` is also available independently. Attach its result with
`set_layer_semantics(layer_id, tree)` to an existing document layer. A host attaching metadata to a
compositor frame is responsible for synchronizing the snapshot with that frame. Compositor
recordings use device coordinates; `tree.transformed(Matrix::create_scale(scale, scale))` maps an
appropriate logical-coordinate snapshot into that recording space. Account for any additional
root/host transformation as well. `SvgSurface` does not implicitly cross threads to walk controls.

`layer_semantics` borrows the immutable tree. `clear_layer_semantics` removes it. Removing a layer
also removes its snapshot, including when its ID is subsequently reused.

## Captured information

| Category | Version-one representation |
| --- | --- |
| Identity | Per-tree key, runtime type name, optional authored name, classes and pseudoclasses. |
| Structure | Visual parent, logical parent and templated parent as distinct relationships. Only targets inside the snapshot are referenced. |
| Geometry | Arranged bounds in parent layout coordinates and all nine coefficients of the node-local to recorded-layer matrix. |
| Layout | Desired size, margin, explicit/Auto width and height, minimum/maximum constraints, alignment, measure/arrange validity, Grid placement for immediate Grid children. |
| Visual state | Local and effective visibility, render suppression, opacity, z-order and the presence of clipping, opacity masks and effects. |
| Input state | Enabled/effectively enabled, focusable/focused, hit-test visibility, tab-stop state and tab index. |
| Display text | Opt-in TextBlock/AccessText content, independently of whether visible typography is outlined. |
| Accessibility | Opt-in automation ID, name, help text, role, landmark, live setting, heading level, required/set-position descriptors. |
| Authored provenance | Optional source URI and one-based line/column, supplied by the application. |
| Extensions | Sorted, typed application properties: string, finite number, Boolean or string list. |

The node order is visual-collection preorder, not z-sorted painter order. `z-index` retains the
source state. A snapshot may be empty or have multiple roots when assembled manually.

Layout bounds describe the source control's allocated rectangle. They do not claim visible ink,
clipped coverage, exact hit regions or occlusion. The `local-to-layer` matrix follows the immediate
renderer, including render-transform origins, manipulation and mirroring. For the recorded root,
layout translation is omitted; its render transforms still apply. Consumers must also apply the
painted layer's SVG ancestry and any inspection-view transformation when locating a node on the
exported page. Perspective coefficients are preserved even when artwork requires raster fallback.

`None` for width/height is emitted as `auto`; `None` for maximum size is `unbounded`. Grid placement
is typed layout data, so disabling layout also removes Grid information. Margins may be negative;
sizes and Grid indices may not. Input and accessibility state describe the source UI at capture
time. They do not make a static image interactive.

## Stable keys and application metadata

Default keys such as `v0-2-1` identify visual-collection paths. They are deterministic for an
unchanged tree, but insertion/reordering can change them. Supply application keys when continuity
across exports matters. Control names are sufficient only when unique across the entire captured
tree; separate template/name scopes commonly reuse names.

```rust
use ferroui_base::Visual;
use ferroui_svg::{SvgSemanticNode, SvgSemanticSource, SvgSemanticValue};

let mut customize = |visual: &Visual, node: &mut SvgSemanticNode| {
    if visual.name().as_deref() == Some("SaveButton") {
        node.id = "workspace.save".into();
        node.properties.insert(
            "design-token.background".into(),
            SvgSemanticValue::String("AccentPrimary".into()),
        );
        node.properties.insert(
            "command".into(),
            SvgSemanticValue::String("SaveWorkspace".into()),
        );
        node.source = Some(SvgSemanticSource {
            uri: "source:///Views/WorkspaceView.axaml".into(),
            line: Some(42),
            column: Some(9),
        });
    }
};

document.record_visual_with_semantics(
    SvgLayerOptions::new("settings", "Workspace settings"),
    &visual,
    &SvgSemanticCaptureOptions::default(),
    Some(&mut customize),
)?;
```

Source locations, resource/design-token keys, binding descriptions, command names, localization
keys, domain IDs and test selectors are authored provenance. The exporter does not infer them
from raster pixels, enumerate DataContext, serialize arbitrary property graphs or evaluate a
binding/command. A callback can add such information explicitly. Visual, logical and template
relationships are resolved against all final customized keys after callbacks complete. To author
different relationships, construct a validated `SvgSemanticTree` directly.

## Content selection

`SvgSemanticCaptureOptions` controls which source information is read. Names, classes, layout,
input state and hidden branches are enabled by default. Text and accessibility are disabled.
`include_hidden = false` omits branches skipped by the immediate renderer for local visibility,
render suppression or zero opacity; it does not perform clip/occlusion analysis.

Automatic text capture reads display text from TextBlock/AccessText, including inline content.
It excludes TextBox controls and their visual descendants, including separately captured subtrees.
It also suppresses text when the captured node or one of its visual ancestors has a TextBox
templated parent. It does not read editable field values or password properties. An application
customization callback can deliberately supply its own text.

Accessibility has three explicit modes:

| Capture mode | Behavior |
| --- | --- |
| `None` | No accessibility text or peers are consulted. |
| `AttachedProperties` | Read authored automation attached properties without creating peers. |
| `AutomationPeer` | Resolve control peers and their computed role, name and help descriptors. This can create peers and run application overrides; value/action providers are not invoked. |

Computed accessibility names/help and application-supplied properties can contain user content.
Select the intended audience's fields rather than assuming any metadata is inherently public.
There is no automatic source URI collection.

`SvgSemanticExportOptions` independently projects an existing snapshot. Modes are `None`,
`Metadata`, and `MetadataAndAttributes`. Names, classes, layout, input state, nonvisual relations
and custom properties each have an inclusion switch. Text, accessibility and source information
remain excluded in `metadata()`. `all()` explicitly includes every captured/supplied field and
adds layer attributes. IDs, runtime types, visual parentage, geometry and basic visual state form
the structural core whenever semantics are enabled; to omit that core, use mode `None` or supply
an application-defined tree. Turning off an export flag does not erase the in-memory snapshot.

## XML vocabulary and rendering association

The namespace identifier is `https://wieslawsoltes.github.io/FerroUI/svg/semantics/1`, with `schema-version="1"`.
It is an identifier; viewers do not fetch it. Foreign-namespace content is placed inside SVG's
standard `<metadata>` element, following [SVG document metadata rules](https://www.w3.org/TR/SVG2/struct.html#MetadataElement).

An abbreviated example:

```xml
<g id="ferro-settings"
   data-ui-schema="https://wieslawsoltes.github.io/FerroUI/svg/semantics/1"
   data-ui-root="ferro-internal-ui-8-settings-WorkspaceSettings"
   data-ui-type="Border">
  <metadata>
    <ui:tree xmlns:ui="https://wieslawsoltes.github.io/FerroUI/svg/semantics/1"
             schema-version="1" hierarchy="visual"
             coordinate-space="layer-local" association="layer" node-count="2">
      <ui:rendering href="#ferro-settings"/>
      <ui:node id="ferro-internal-ui-8-settings-WorkspaceSettings"
               key="WorkspaceSettings" type="Border">
        <!-- Geometry, state and selected descriptors. -->
      </ui:node>
      <ui:node id="ferro-internal-ui-8-settings-workspace.save"
               key="workspace.save" type="Button">
        <ui:parent href="#ferro-internal-ui-8-settings-WorkspaceSettings"/>
        <ui:properties>
          <ui:property name="command" type="string">SaveWorkspace</ui:property>
        </ui:properties>
      </ui:node>
    </ui:tree>
  </metadata>
  <!-- The original composed Skia artwork. -->
</g>
```

`association="layer"` is intentional. Immediate rendering preserves a whole compositing island;
the compositor can flatten several controls into shared drawing items. A source control therefore
does not necessarily own a unique `<path>` or `<g>`. Dividing the drawing by controls would break
effects, clipping and destination-dependent blending. Version one links the semantic graph to its
painted layer and supplies node geometry for inspection. It does not invent per-path ownership.

Document-global node IDs use a reserved internal prefix and a length-prefixed layer component.
Application keys stay separate in `key`. This prevents collisions across layers and with the
exporter's resource IDs. Every relationship is a local fragment reference, checked during normal
document validation. Layer filtering removes the matching manifest; raster, x-ray, section and
exploded exports preserve its association with the included painted layer. Inspection treatments
change the artwork, not the captured source-UI state.

All free-form content is XML-escaped, including attribute whitespace. Source URIs and properties
remain inert data; no script, event handler, external asset, ARIA role or tab stop is created from
them. The serializer preserves semantic numbers independently of optional artwork quantization.
SVG supports these extension mechanisms, but editor import/re-export may discard them. Keep the
original semantic SVG or extract its manifest before handing artwork to a tool that drops unknown
metadata. No Figma/Illustrator metadata round-trip guarantee is implied.

## Limits and performance

Snapshots validate unique keys, reference resolution, acyclic visual/logical/template ancestry,
XML 1.0 characters, finite geometry/property numbers and positive source positions. Visual parents
must precede children. The snapshot has limits of 100,000 nodes, 4,096 ASCII bytes per key, 16 MiB
of aggregate strings and 500,000 string values. Limits are independent, so another budget can be
reached before the node limit. Capture also limits depth (256 by default), queued work and copied
text. It uses an iterative walk; final ancestry validation is iterative as well.

The resulting tree contains owned values and no retained controls or callbacks. Repeated exports
reuse it without walking controls, measuring layout or shaping text. Snapshot replacement does
not invalidate the native/prepared vector or PNG caches. The map lives on `SvgDocument`; the hot
`SvgLayer` representation is unchanged. Mode `None` skips the semantic lookup and serializer and
preserves the pre-extension output bytes. Enabled metadata adds serialization, validation, memory
and file-size cost proportional to its content. SVGZ compresses the repeated vocabulary normally.

`SvgExportReport.semantic_layers` and `.semantic_nodes` report only snapshots actually included
after filtering. Existing draw-call, raster-fallback and cache-hit counters retain their meaning.

## Runnable example

[`semantic_export.rs`](../../src/Svg/FerroUI.Svg/examples/semantic_export.rs) creates and lays out
actual controls with an embedded Inter font and explicit button templates. It writes a plain SVG,
a semantic SVG and SVGZ from the same recording:

```sh
cargo run --locked -p ferroui-svg --example semantic_export -- samples/SvgExport/semantics
python3 src/Svg/FerroUI.Svg/examples/read_ui_semantics.py \
  samples/SvgExport/semantics/workspace-settings.svg \
  --output samples/SvgExport/semantics/workspace-settings.ui.json
```

The standard-library reader validates the version-one manifest and extracts deterministic JSON
for inspection tooling. It reads the SVG file locally and does not fetch namespaces, source URIs
or other resources. The example's display text is outlined, while its manifest retains the
corresponding readable strings. See the [committed example and validation](../../samples/SvgExport/semantics/README.md).
