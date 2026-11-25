# FerroUI SVG rendering backend

`ferroui-svg` records FerroUI drawing commands through the existing Skia backend and exports
self-contained SVG or SVGZ. It supplies an `IRenderTarget` for frame capture, an
`IPlatformRenderSurface` integration, and a retained document API for named, independently editable
layers. Its platform interface reuses Skia's resource factories and ordinary window rendering.

The default produces outlined text, deterministic resource IDs, explicit presentation attributes,
embedded images and named groups. Skia operations outside its SVG writer's verified subset are
replayed as a PNG of the complete affected layer. The export report identifies every substitution.
`SvgFidelity::VectorOnly` turns any such substitution into an error.

```rust
use ferroui_base::{Rect, RoundedRect, Size};
use ferroui_base::media::{BoxShadows, Colors};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::platform::IDrawingContextImpl;
use ferroui_svg::{SvgDocument, SvgExportOptions, SvgLayerOptions};

let mut document = SvgDocument::new(Size::new(640.0, 360.0))?;
document.record_layer(SvgLayerOptions::new("surface", "Surface"), |context| {
    context.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(Colors::CORNFLOWER_BLUE)),
        None,
        RoundedRect::from_radius(Rect::new(24.0, 24.0, 592.0, 312.0), 16.0),
        &BoxShadows::default(),
    );
})?;
let output = document.export(&SvgExportOptions::default())?;
output.save("surface.svg")?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Initialize Skia services as the application normally does when recording geometries, glyph runs,
images or a visual tree. `record_visual` accepts an already laid-out `Ref<Visual>` and uses
`ImmediateRenderer`. Hosts that need compositor commands can supply `SvgRenderTarget` to their
rendering pipeline and call `export_last` after the drawing context finishes. Alternatively,
register `SvgApplicationExtensions::use_svg()` or `SvgPlatform`, return an explicit `SvgSurface`
from the host's surface provider, and consume `SvgSurface::take_frame()`. SVG surfaces publish the
latest export result without accumulating a frame queue or writing files. Normal platform
surfaces continue using Skia. Registration alone does not attach a capture to an existing window.

## Export controls

- Normal, x-ray and wireframe treatments, rectangular cutaway/section windows, exploded layers,
  bounds and guide annotations, inclusion/exclusion by ID or tag, hidden-layer selection.
- Exact/default numerical output or explicit scalar precision reduction; compact or indented XML;
  repeated-definition interning; optional relative paths.
- Outlined text or opt-in editable text reconstructed by Skia. Editable text can differ for
  ligatures, contextual scripts and font substitution; outlined text is the portable default.
- SVG/SVGZ, reproducible gzip headers, physical dimensions, responsive sizing, crop/bleed viewBox,
  document title and description, layer names, tags and application metadata.
- Raster fallback density and allocation limits; reusable native SVG/PNG caches with explicit
  eviction through `clear_export_cache`.
- Optional immutable UI snapshots: control identity, visual/logical/template relationships,
  geometry/layout, input state and explicitly selected text/accessibility/source metadata.

## Preserve UI meaning

`record_visual_with_semantics` records a complete visual as one composited layer and attaches an
immutable snapshot of its control tree. Enable `SvgExportOptions::semantics` to publish that
snapshot as versioned, namespaced SVG metadata. `SvgSemanticExportOptions::metadata()` includes
structural information; text, accessibility content and source locations remain explicit opt-ins.
`SvgSemanticExportOptions::all()` includes every captured field and layer `data-ui-*` attributes.

Applications can supply persistent control keys, design tokens, source locations, command names
and test selectors through a customization callback. Geometry remains ordinary designer-editable
SVG, including when its text is outlined. Semantics identify the source UI and its recorded layer;
they do not invent ownership of individual Skia paths or turn a static SVG into interactive UI.
Capture is opt-in, and metadata replacement preserves the prepared drawing caches.

See the [UI semantic schema and API guide](../../../docs/porting/svg-semantics.md) and the
[real-control example](../../../samples/SvgExport/semantics/README.md), including a Python reader
that extracts the manifest as deterministic JSON.

## Compositing and scope

A named layer is an **isolated compositing island**. Destination-dependent effects that interact
with each other belong in the same layer. Record a complete frame in one layer when preserving its
full painter order matters. Layers are finally composed source-over, with their declared transform
and opacity. Their bounds are explicit clips, not inferred effect bounds; include shadow/blur
padding in those bounds.

The cutaway is a **2D layer section**, and an exploded view separates declared layers by depth. It
does not compute a CAD solid intersection, hidden-line removal or a cap mesh for a 3D scene.
The optional `media3d` feature forwards FerroUI's existing CPU 3D rendering into embedded images.

The exporter captures what reaches the Skia drawing contract. Immediate visual export shares
`RenderTargetBitmap` limitations: composition-only objects, native child windows and live platform
surfaces are not traversed; composition brushes use their immediate fallback colors. A custom
Skia lease is captured as recorded canvas commands without a GPU target. External GPU-only
resources and application drawing that depends on an onscreen surface need a CPU-readable
snapshot supplied by the application.

## Example and checks

```sh
cargo run -p ferroui-svg --example layered_export -- samples/SvgExport/output
cargo run -p ferroui-svg --example semantic_export -- samples/SvgExport/semantics
cargo test --locked -p ferroui-svg --lib
cargo test --locked -p ferroui-svg --features media3d --lib
```

The example emits normal, x-ray, wireframe, section and exploded SVGs, plus SVGZ. Add `--reference`
for a Skia raster reference and `--benchmark` for 100 repeat exports; use an optimized Cargo profile
for performance measurements.

See [the backend design and feature matrix](../../../docs/porting/svg-backend.md) in the source
repository for architecture, fidelity policy, integration and validation details.
