# SVG rendering backend

## 1. Purpose and architecture

`ferroui-svg` is a headless export backend layered on `ferroui-skia`. It records the normal FerroUI
drawing contract, reuses Skia's geometry, brushes, glyph shaping output and compositing, and emits
an editable SVG document where the captured operation set permits it. It does not replace the
application's geometry/font/bitmap factories: those remain Skia resources.

The pipeline is:

1. `SvgDrawingContext` audits calls while forwarding them to `DrawingContextImpl`.
2. `PictureRenderTarget` owns a Skia picture recording. Explicit disposal, or the adapter's drop,
   completes it. Export recordings also capture `ISkiaApiLeaseFeature` canvas commands.
3. `SvgDocument` retains pictures, semantic layer descriptions and capability audits.
4. The exporter replays vector-safe pictures into `skia_safe::svg::Canvas`, or replays a whole
   affected compositing island into a CPU raster surface and embeds its PNG.
5. A namespace-aware XML pass applies inspection treatments, namespaces resource IDs, interns
   equivalent definitions, preserves editable layer structure and serializes the final document.

There is no parallel implementation of FerroUI's geometry tessellation, glyph positioning or
effect engine. No unsafe Rust is added. SVG export does not enable Skia's optional SVG DOM/parser
feature; its SVG writer is already available in the pinned Skia build.

### Integration levels

`SvgDocument` records explicit named layers or already laid-out visuals. A callback is invoked
exactly once; exporting additional views repeats neither UI layout nor text shaping. Recording is
transactional: a callback error or invalid recording adds no partial layer.

`SvgRenderTarget` implements `IRenderTarget`. A host can feed it normal frame/compositor drawing
commands, then export or take the completed document. It advertises direct rendering and no
retained prior frame. Overlapping recordings return `NotReady`; disposal marks the target
corrupted and prevents an in-flight recording from publishing a new snapshot. Serialization is
separate from frame capture, so a frame that is never exported incurs no XML work.
An unwinding frame producer marks the capture as a recording failure. Completion owns a readiness
guard, so failed finalization cannot leave the target permanently busy or publish a partial frame
as successful artwork.

`SvgSurface` implements `IPlatformRenderSurface` and provides the platform integration path.
`SvgPlatformRenderInterface` delegates resource creation, ordinary window surfaces and offscreen
targets to Skia, while recognizing explicit SVG surfaces. A completed SVG surface frame is
serialized on the render thread and published through a bounded, latest-result slot. The
application calls `take_frame()` to receive its sequence, options revision and export result;
failures replace old results so stale artwork cannot appear to be a successful new export.
`SvgSurface` is `Send + Sync`; the retained pictures and drawing contexts stay on their owning
thread. No file is written implicitly.

Use `SvgApplicationExtensions::use_svg()` instead of the application's Skia rendering registration,
or initialize `SvgPlatform` directly in a custom host. Registration is opt-in. The host still
chooses where an SVG surface receives its frame; registration alone does not capture an existing
window.

SVG targets opt into `IRenderTarget::prefers_full_frame_rendering()`. The compositor can therefore
render a complete suitable frame directly without its usual retained bitmap intermediary.
Existing targets retain the previous default behavior. Overlays that need that intermediary still
use it and export as raster content. Every frame is invalidated in full because SVG targets do not
retain previous frame pixels. Device-pixel drawing coordinates are recorded unchanged and mapped
to logical SVG coordinates once, including HiDPI frames and rounded pixel dimensions.

## 2. Fidelity policy

SVG is a vector interchange format with a different feature boundary from a general Skia canvas.
The pinned Skia SVG writer does not faithfully serialize every Skia shader, effect or compositing
operation. The audit therefore selects among three explicit policies:

| Policy | Result |
| --- | --- |
| `Automatic` (default) | Keep supported layers as vectors; embed a PNG of each affected layer. Report the reasons. |
| `VectorOnly` | Return `SvgError::VectorFidelity` with the layer and reasons. Intentional bitmap content is still allowed. |
| `Raster` | Embed every layer as PNG; useful for reference rendering and consumers with limited SVG support. |

A fallback includes **all operations in the layer**, preserving destination-dependent blends,
masks and effects inside that recording. Rendering only the offending primitive on transparency
would produce incorrect results for operations such as destination-out.

Every named layer is an **isolated compositing island**. Its recording starts transparent and its
finished result is composed source-over. Content whose effects depend on a shared backdrop must
be recorded together in one layer; a captured full frame is one such island. The exporter does not
pretend independently recorded layers share a Skia backdrop.

Raster fidelity is relative to the recorded Skia scene and the selected raster density. It does not
promise identical antialiasing across SVG viewers, subpixel LCD text, color-managed application
screens or arbitrary GPU-only effects. Raster fallback is visible in `SvgExportReport` and in each
layer's `data-rendering` attribute.

Invalid coordinates, non-Skia resources, unbalanced state and definite missing renderer
capabilities are errors rather than successful partial exports. Native canvas drawing is captured
but conservatively classified as uninspectable, requiring fallback. Application drawing that
depends on external GPU textures, an actual surface or platform child windows must provide an
export-compatible snapshot.

Direct bitmap, image-brush and native layer-blit capture rejects leased GPU frames, texture-backed Skia images and
surfaces requiring their owning GPU context. The recording fails with a request for an explicit CPU
snapshot; it never attempts an implicit GPU readback. Unit tests exercise the leased-frame
contract without hardware and accept every current CPU bitmap implementation. Actual GPU texture
import, device loss and readback are outside these SVG tests. Opaque application drawing and scene
brush callbacks must likewise supply CPU-compatible resources inside their own drawing code.

## 3. Drawing contract coverage

| Contract feature | Export handling |
| --- | --- |
| Lines, rectangles, rounded rectangles, ellipses | Native vector output for supported brushes and strokes. |
| Paths, fill rules, regions | Native Skia geometry serialization; inverse fill uses fallback. |
| Solid colors and alpha | Native SVG presentation attributes. |
| Canvas clear | Initial zero-origin clears stay vector; destination-dependent and nonzero-origin color clears use composed fallback. |
| Linear gradients and centered radial gradients | Native gradients, including ordinary affine transforms. |
| Repeat/reflect gradient spread | Composed raster fallback; the pinned writer does not emit spread semantics. |
| Conic/sweep and focal radial shaders | Conservative raster fallback. |
| Image brushes and scene brushes | Classified fallback where shader/content fidelity cannot be established. |
| Images, source rectangles and image opacity | CPU-readable images become embedded PNG resources; resource sharing preserves references. GPU-only sources require an application-provided CPU snapshot. |
| Bitmap sampling | Default/low-quality sampling retains editable images. Effective nearest-neighbor, mipmapped or high-quality sampling on a bitmap draw uses composed fallback because the pinned SVG writer drops the native sampling policy. |
| Explicit aliased primitive edges | Non-text primitive draws with effective `EdgeMode::Aliased` use composed fallback, preserving the requested device-pixel edge policy. Unused or restored option scopes do not force fallback. |
| Shaped glyph runs | Outlined vector glyphs by default; color/bitmap/non-outline glyphs use fallback. |
| Rectangular, rounded, path and region clips | Native clips where supported; unequal-corner rounded and inverse clips use fallback. |
| Opacity layers, compositing layers and bitmap blend modes | Forwarded to Skia; nontrivial isolation/blending conservatively triggers fallback. |
| Opacity masks and bitmap masks | Complete-layer raster fallback preserves destination-in semantics. |
| Box shadows, blur/drop shadow and custom effects | Forwarded to Skia and rasterized as a composed layer; a missing or invalid custom shader is an error. |
| Color matrix, multiply, convolution, lighting and pixel operations | Every optional layer-effect method forwards through the audited adapter; fallback is explicit. |
| Acrylic-like materials | Skia material rendering with fallback. |
| Offscreen layers and blitting | A destination-context hook validates CPU readability before native downcasting. Accepted blits retain Skia's device-space semantics and use explicit raster fallback. |
| Native Skia canvas leases | Captured in order with the transform, DPI and opacity at lease acquisition, including a lease that finishes immediately before context disposal; fallback. Retained features cannot outlive their context. |
| Reusable fragments | Capability mirrors the underlying Skia context; no unsupported fragment support is advertised. |
| 3D scenes | Optional `media3d` forwards the CPU scene renderer; missing executor is an error, supported output is raster. |
| Perspective transforms | Captured by Skia; raster fallback because SVG affine transforms cannot express them. |

The adapter implements the full core drawing-context surface and the effects, layer-effects,
acrylic and scene interfaces. Extending the underlying contract creates a compile-time obligation
for new required members. New optional/default methods still require a fidelity audit when added.
Brush validation includes resolved gradient coordinates and stroke-expanded bounds, so otherwise
finite inputs that overflow Skia's scalar range fail before native recording.

Render-option auditing reads the native context's effective state, including initial configuration,
inheritance and restoration. A sampling preference around only vector geometry does not trigger
image fallback. Default editable image export retains crop geometry, intrinsic PNG alpha and the
native draw-opacity multiplier. A producer-specific normalization repairs the pinned Skia writer's
`fill-opacity` encoding on image instances; it does not reinterpret arbitrary SVG input.

### Visual trees versus compositor frames

`record_visual` uses `ImmediateRenderer`, just as FerroUI's bitmap snapshots do. It respects the
visual tree's visibility, z-order, transforms, clips, opacity and supported immediate effects.
Composition-only objects are not part of that traversal, and composition brushes have immediate
fallback colors. Use a host-supplied `SvgRenderTarget` or `SvgSurface` when the actual compositor
commands are needed. Merely creating an SVG target does not attach it to an existing window's
compositor.

## 4. Designer interoperability

Default output uses ordinary SVG elements and presentation attributes, named `<g>` groups,
embedded images and glyph outlines. No script, animation, foreign object or external stylesheet
is emitted. The XML validation pass rejects active or unresolved resource references rather than
blindly copying arbitrary source XML.

Each semantic group has a stable `ferro-<id>` identifier, a title, its name, depth, tags and optional
application metadata. Optional Inkscape group labels improve layer recognition without changing
rendering for consumers that ignore the extension. These groups are ordinary editable SVG
structure; proprietary editor layer/document data is not fabricated.

The designer controls include:

- `text`: exact shaped outlines (default) or opt-in editable text.
- `precision`: retain native numbers (default) or round scalar/coordinate-list attributes to
  0–12 decimals. Paths and matrices stay exact because naïve rounding changes topology and can
  amplify transform error.
- `pretty`, `relative_paths`, `deduplicate`: readability, compact path encoding and conservative
  resource interning. Semantic groups, transforms and painter order are retained.
- `units`: pixels, millimeters, centimeters, inches or points, with 96-DPI logical geometry.
- `responsive`: omit absolute width/height while retaining viewBox.
- `view_box`: explicit crop, bleed or enlarged page, including a nonzero origin.
- `title`, `description`, `designer_layers`: document and editor-facing metadata.

Editable text is intentionally opt-in and always emits a diagnostic. The low-level drawing
contract receives shaped glyph IDs and positions, not the original Unicode string. Skia's reverse
mapping can lose ligatures, contextual glyphs and bidi meaning, and the target computer needs the
font. Outlines preserve the captured glyph shapes. The backend does not claim a proprietary Figma
or Illustrator document round trip, and those applications' import behavior must be verified
against the versions in the intended workflow.

Outlined text preserves the shaped glyph IDs, advances and offsets as scalable geometry. It does
not preserve device-specific font hinting, baseline pixel snapping or LCD coverage. The pinned
Skia writer obtains paths through [`SkFont::getPaths`](https://github.com/rust-skia/skia/blob/m153-0.101.2/src/core/SkFont.cpp), whose `setupForAsPaths` disables font hinting;
the ordinary Skia raster text path can use hinted glyph masks and snapped baselines. Consequently,
small outlined text can differ visibly in stroke coverage from a native pixel snapshot even when
every glyph position and contour is correct. `SvgFidelity::Raster` and `raster_scale` provide the
recorded Skia text appearance at a chosen pixel density. Changing application `TextOptions` to
unhinted, unaligned text is useful for geometry comparisons, but is not required for vector export
and does not make independent rasterizers' antialiasing identical.

## 5. Inspection views

All views operate on retained recordings and do not mutate the source document.

| View | Meaning |
| --- | --- |
| Normal | Original recorded appearance within the selected fidelity policy. |
| X-ray | Lower vector fill opacity and emphasize shape outlines; preserve clips and paint resources. |
| Wireframe | Remove vector fills and emphasize outlines; image geometry becomes a bounds representation where possible. |
| Cutaway / section | Keep or remove a rectangular window from selected layers, in final document coordinates. |
| Exploded | Offset layers by `step × depth`; optional guide lines and labels show separation. |
| Bounds | Overlay declared recording bounds for inspection. |

Layer filters select by IDs and/or tags. Within a set, values are ORed; nonempty ID and tag sets
are ANDed. An empty filter matches all layers. Hidden layers are excluded unless requested.
`inspection_layers` independently selects the layers receiving x-ray/wireframe treatment, so a
technical plate can keep its headings, dimensions and annotations unchanged.

A cutaway applied only to a `shell` tag reveals lower layers through the window. The window is
applied after the layer/explosion transform. The remove-inside case uses a standard even-odd clip
path; the keep-inside case uses a rectangular clip. It is a **2D section of UI/illustration layers**,
not a solid-model Boolean intersection. The API does not invent volumetric information, section
caps, materials, occlusion or 3D hidden-line geometry from a flattened display list.

Raster islands cannot yield editable internal vector edges. Inspection diagnostics state that
boundary explicitly. Exploded guide labels follow the text policy: outlined by default, or
generic-font editable text when requested. These generated technical labels use scalar glyph
positioning and report a diagnostic for non-ASCII content; record application-shaped text inside
a layer when complex-script or bidirectional labels are needed. Annotations are emitted above the
scene so later layers cannot obscure an earlier label.

Wireframe image instances retain `<use>` layout, including both source and instance transforms;
they reference a separate bounds rectangle while the original image remains available to paint
resources. X-ray image opacity composes with existing CSS opacity. Optional numeric rounding
preserves token boundaries and leaves foreign metadata unchanged.

## 6. Performance and resource behavior

Recording uses the existing Skia picture pipeline and existing geometry/text caches. A document
owns its recording and image references until removed/dropped. There is no layout, UI-thread
callback or glyph shaping replay for another export view.

The most recent native SVG and encoded fallback PNG can be cached per layer. Cache keys include
the text/path policy or raster dimensions; a different key replaces the previous value, so each
layer retains at most one representation of each kind. `cache_native_svg` and
`cache_raster_images` disable reuse, and `clear_export_cache` releases both while retaining the
pictures. Cache hits are reported. Raster limits are checked even when a cached image exists.

Resource interning uses hashed structural keys with equality checks. It considers inherited
context and independent child IDs before deduplicating definitions. Hash-map iteration never
determines output order: IDs and serialization follow document order. This removes Skia's
process-global clip IDs and makes repeated exports of the same recordings byte-stable.

Fallback surfaces default to two pixels per recording coordinate and at most 64 million pixels /
32,768 pixels per dimension. Named layers normally use logical coordinates; compositor captures
use device pixels and carry a pixel-to-logical layer transform. Applications can lower these limits
and scale. These are bounds on export
replay allocations, not a global quota for Skia: callbacks can create intermediate surfaces of
their own. Supply tight layer bounds, including required effect padding, for smaller fallback
images. A complete destination-dependent frame may necessarily occupy a full-frame surface.

`SvgOutput::write_to` avoids a second caller-owned string, and `write_svgz` streams compression
with deterministic headers. The current optimizer constructs an XML tree; it is bounded by depth
and node limits and deliberately preserves designer structure instead of applying aggressive,
potentially destructive path merging. No universal speed claim is made across unknown drawings;
the example's benchmark reports workload, profile and measured bytes/time.

## 7. Use

```rust
use ferroui_base::{Size, Vector};
use ferroui_svg::{
    SvgDocument, SvgExplodedView, SvgExportOptions, SvgFidelity,
    SvgLayerOptions, SvgView,
};

let mut document = SvgDocument::new(Size::new(1200.0, 800.0))?;
document.record_layer(
    SvgLayerOptions::new("panel", "Control panel").with_depth(1.0).with_tags(["ui"]),
    |context| {
        // Draw through IDrawingContextImpl, or borrow it through PlatformDrawingContext.
    },
)?;
let options = SvgExportOptions {
    fidelity: SvgFidelity::VectorOnly,
    view: SvgView {
        exploded: Some(SvgExplodedView {
            step: Vector::new(24.0, -20.0),
            show_connectors: true,
            show_labels: true,
        }),
        ..Default::default()
    },
    ..Default::default()
};
let output = document.export(&options)?;
output.save("panel.svg")?;
output.save_svgz("panel.svgz")?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

For a laid-out visual, call `document.record_visual(layer_options, &visual)` on its UI thread.
For a full host frame, obtain `SvgRenderTarget::try_create_drawing_context(&scene_info)`, render,
dispose/drop that context, then call `export_last`. The target follows the normal rendering trait
contracts and does not write files implicitly. Frame contexts use device-pixel coordinates,
matching the compositor's contract; `RenderTargetSceneInfo` supplies the logical size and scale.

To integrate an export surface with a custom top-level host:

```rust
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_svg::{SvgExportOptions, SvgSurface};
use std::sync::Arc;

let capture = SvgSurface::new(SvgExportOptions::default())?;
let surface: Arc<dyn IPlatformRenderSurface> = Arc::new(capture.clone());
// Return `surface` from the host's render-surface provider after registering SvgPlatform.
// The compositor creates the SVG target through SvgPlatformRenderInterface.
// After a completed frame:
if let Some(frame) = capture.take_frame() {
    frame.result?.save("frame.svg")?;
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

`set_options` validates policy changes and returns a revision. Export takes an options snapshot
without holding the shared mutex during replay or XML work. If another thread changes the policy
mid-export, the published frame retains the revision that actually produced it.

## 8. Reproducible examples and validation

The executable example is `src/Svg/FerroUI.Svg/examples/layered_export.rs`. It draws a technical
product plate with distinct chassis, electronics, display and cover layers through FerroUI's
normal primitives, paths, gradients and glyph runs. It requests `VectorOnly` for its primary
examples so a regression cannot silently replace the illustration with an image.

```sh
cargo test --locked -p ferroui-svg --lib
cargo test --locked -p ferroui-svg --features media3d --lib
cargo test --locked -p ferroui-base --lib full_frame_render_tests
cargo test --locked -p ferroui-skia --lib bitmap_cpu_export_tests
cargo test --locked -p ferroui-skia --lib native_lease_tests
cargo clippy --locked -p ferroui-svg --all-targets --all-features --no-deps -- -D warnings
cargo run --locked -p ferroui-svg --example layered_export -- samples/SvgExport/output
cargo run --release --locked -p ferroui-svg --example layered_export -- /tmp/svg-check --reference --benchmark
python3 src/Svg/FerroUI.Svg/examples/validate_exports.py /tmp/svg-check --preview-dir /tmp/svg-preview
```

The unit tests exercise actual Skia recordings, not only string snapshots: clips/transforms,
isolated opacity, masks/effects, destination-dependent blending, custom lease ordering, layer
blit, nonzero bounds, lifecycle, invalid state, deterministic XML, reference integrity, Unicode
and whitespace preservation, resource deduplication, inspection transformations and SVGZ round
trips. The normal example can be rendered by an independent SVG renderer and compared with
`--reference`, which replays the same recordings through Skia's raster path. Small edge differences
between independent rasterizers must be separated from structural missing content.
The validation script checks duplicate IDs and local references, renders the variants with
Inkscape, and reports RGB error metrics when the raster reference is present. It can use
Pillow/NumPy or its standard-library PNG decoder. See `samples/SvgExport/README.md` for the checked
example workload and measured results.

The correctness review was validated on Linux with 89 passing SVG tests in the default build and
89 in the `media3d` build, three passing Base compositor tests, and 603 passing Skia library tests
(nine existing ignored tests) under a controlled Noto font setup. The feature-enabled run checks
actual 3D triangle pixels; the feature-disabled run checks the missing-executor error, accounting
for Cargo feature unification. [The review report](svg-review.md) describes fixes, real compositor
coverage, font-environment requirements and validation boundaries. These local results do not
substitute for the repository's separate cross-platform CI.

## 9. Primary format and implementation references

- [SVG 2 rendering model](https://www.w3.org/TR/SVG2/render.html): painter order, grouping,
  compositing, clipping and masking.
- [SVG painting](https://www.w3.org/TR/SVG2/painting.html): fills, strokes and paint servers.
- [Pinned Skia SVG writer](https://github.com/rust-skia/skia/blob/m153-0.101.2/src/svg/SkSVGDevice.cpp):
  the implementation behind `skia-safe = 0.153.3`; native SVG capabilities and omissions.
- [Pinned Skia glyph path conversion](https://github.com/rust-skia/skia/blob/m153-0.101.2/src/core/SkFont.cpp):
  `getPaths` and `setupForAsPaths` define the unhinted geometry used by outlined text.
- [Pinned Rust SVG canvas wrapper](https://github.com/rust-skia/rust-skia/blob/b7f043e0b1e2a850e702245de811f01df1f6b0ff/skia-safe/src/svg/canvas.rs):
  finalization and text/path encoding flags.

Native SVG support can be expanded by translating further audited operations into explicit SVG
structures, or by verified improvements in the pinned Skia writer. Such changes should carry
renderer comparison fixtures and preserve the strict-vector/fallback contract.
