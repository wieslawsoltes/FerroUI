# The Vello render backend

Status: **in progress** (started 2026-10-09). Row 25 of `CRITICAL-PATH.md`.

The port has two render backends behind the platform contracts of `src/FerroUI.Base/platform`: Skia (`src/Skia/FerroUI.Skia`, a port of upstream's `Avalonia.Skia`, mature) and Vello (`src/Vello/FerroUI.Vello`, crate `ferroui-vello`). The Vello backend is an **addition**: upstream has none. Its structure, names and logic follow the Skia backend file by file so that the two stay comparable, and its correctness is measured against the Skia backend (section 8).

Marks: **[V]** verified in the sources of the released crate named (path in the cargo registry), **[M]** measured here, **[U]** not verified.

## 1. The facts about Vello

Established on 2026-10-09 with `cargo search`, `cargo info` and the sources of the crates as cargo downloaded them (`~/.cargo/registry/src/index.crates.io-*/<crate>-<version>/`). The workspace builds with `rust-version = "1.89"` and CI with the stable toolchain; the machine this was written on has 1.90.

### 1.1 Crates, versions, licences

| Crate | Version | MSRV | Licence | What it is |
|---|---|---|---|---|
| `vello` | 0.11.0 | 1.89 | Apache-2.0 OR MIT | "classic" Vello: compute shaders on `wgpu` 30 (`Cargo.toml`: `wgpu = "30.0.0"`, `skrifa = "0.44.0"`, `peniko = "0.6.1"`) |
| `vello_cpu` | 0.3.0 | 1.89 | Apache-2.0 OR MIT | sparse strips on the processor; features `std`, `png`, `text` (pulls `glifo`), `u8_pipeline`, `f32_pipeline`, `multithreading` (pulls `rayon`) |
| `vello_gpu` | 0.3.0 | 1.89 | Apache-2.0 OR MIT | sparse strips prepared on the processor, rasterized and composed on the GPU; features `wgpu`, `webgl` (GLSL shaders, `web-sys`), `text` |
| `vello_hybrid` | 0.2.0 | 1.88 | Apache-2.0 OR MIT | the **former name** of `vello_gpu`: "This package was previously named `vello_hybrid`. New dependencies and Rust imports should use `vello_gpu`" (`vello_gpu-0.3.0/README.md`, "Package rename") **[V]**. Both names exist on crates.io; 0.2.0 is the last release under the old one |
| `vello_common` | 0.3.0 | 1.89 | Apache-2.0 OR MIT | what `vello_cpu` and `vello_gpu` share: flattening, strips, paints, pixmaps, masks, filters |
| `vello_api` | 0.0.7 | **1.92** | Apache-2.0 OR MIT | the project's own common API (`PaintScene`); see 1.5 |
| `glifo` | 0.4.0 | 1.89 | Apache-2.0 OR MIT | glyph rendering for `vello_cpu` and `vello_gpu` (outlines, hinting, COLR, bitmap glyphs, atlas cache) |
| `peniko` | 0.6.1 | 1.85 | Apache-2.0 OR MIT | brushes, gradients, images, blend modes, fill rules |
| `kurbo` | 0.13.1 | 1.85 | Apache-2.0 OR MIT | paths, strokes, arcs, curve measures |
| `skrifa` | 0.48.0 (latest; `vello` 0.11 and `glifo` 0.4 link 0.44.0) | 1.85 | MIT OR Apache-2.0 | font tables and glyph outlines |
| `wgpu` | 30.0.1 | 1.87 | MIT OR Apache-2.0 | the graphics API of `vello` and `vello_gpu` |
| `fontique` | 0.12.0 | 1.88 | Apache-2.0 OR MIT | font enumeration and fallback |
| `linesweeper` | 0.5.0 | not stated | MIT OR Apache-2.0 | boolean operations of Bézier paths of kurbo ("early beta" by its README) |
| `png` | 0.18.1 | 1.73 | MIT OR Apache-2.0 | PNG decoding and encoding |
| `zune-jpeg` | 0.5.15 (0.5.16-rc2 is a release candidate) | 1.75 | MIT OR Apache-2.0 OR Zlib | JPEG decoding |
| `zune-bmp` | 0.5.2 | 1.87 | MIT OR Apache-2.0 OR Zlib | BMP decoding |
| `zune-core` | 0.5.3 | 1.75 | MIT OR Apache-2.0 OR Zlib | options and readers of the two zune crates |
| `jpeg-encoder` | 0.7.1 | 1.87 | (MIT OR Apache-2.0) AND IJG | JPEG encoding; the IJG terms ask for a statement in the documentation, which `NOTICE.md` has |
| `gif` | 0.14.2 | 1.62 | MIT OR Apache-2.0 | GIF decoding (with `weezl` 0.1.12) |

Everything the backend links is Apache-2.0 OR MIT (`jpeg-encoder` with the IJG terms beside it) and within the toolchain of the workspace, except `vello_api` (1.92), which it does not link. The notices are in `src/Vello/FerroUI.Vello/NOTICE.md`.

### 1.2 The three renderers

The owner wants all three modes; the backend is designed for all three from the start (`VelloRenderingMode { Cpu, Hybrid, Gpu }`, section 4).

| | CPU: `vello_cpu` | Hybrid: `vello_gpu` (was `vello_hybrid`) | GPU: `vello` |
|---|---|---|---|
| Needs | nothing | `wgpu` without compute shaders, or WebGL2 (`webgl` feature) | `wgpu` with compute shaders (Metal, Vulkan, D3D12, WebGPU) |
| Scene | `RenderContext` (`render.rs`) | `Scene` (`scene.rs`), the same methods as the CPU one, name for name **[V]** | `vello::Scene` (`scene.rs`), another API: every call takes its transform, brush and style |
| Target | `PixmapMut` over premultiplied RGBA8 without padding, 16 bit sizes; `TargetInit::Clear` or `SrcOver` | a `wgpu::TextureView` (`Renderer::render`), `RenderTargetConfig { format, width: u16, height: u16 }` | a `wgpu::TextureView` (`Renderer::render_to_texture`) |
| Maturity (own words) | the most mature of the sparse-strips pair | "slightly less mature than its CPU-only counterpart"; mask layers, complex filter graphs and some blend modes of non-isolated blending "will panic" (`README.md`, known limitations) **[V]** | the original renderer |
| Thread traits | `RenderContext` is `Send` (its dispatcher is `Debug + Send`, `dispatch/mod.rs:18`), not shared | [U] for its `Renderer` | `Scene: Send + Sync` (asserted, `scene.rs:50`), `Renderer: Send` and not on WebAssembly (`lib.rs:348-354`) |

### 1.3 What the drawing context contract needs, per renderer

| Contract | `vello_cpu` 0.3 | `vello_gpu` 0.3 | `vello` 0.11 | Gap and how it is closed |
|---|---|---|---|---|
| Fill and stroke of a path | `fill_path`, `stroke_path` | same | `fill`, `stroke` | none |
| Solid brush | yes | yes | yes | none |
| Linear, radial (two-point), conic gradient; pad, repeat, reflect | `peniko::Gradient` (`GradientKind::{Linear, Radial, Sweep}`, `Extend`) | same | same | radial with an offset origin: Skia composes the gradient over the color of its final stop; here the color is drawn first, under the gradient (built) |
| Image and tile brush | `ImageSource::Pixmap`, extend per axis, quality low/medium/high | images uploaded to an atlas (`Renderer::upload_image`) or external textures | `peniko::ImageBrush` | no "decal" extend (a tile that paints nothing beside itself): the shape is clipped to the tile (built) |
| Brush transform | `set_paint_transform`, applied after the path transform (`transforms.rs:114`) | same | `brush_transform` argument | none |
| Caps, joins, miter limit, dashes with offset | `kurbo::Stroke` | same | same | none; dash lengths are scaled by the pen thickness by the backend |
| Fill rules | `Fill::{NonZero, EvenOdd}` | same | same | none |
| Clips: rectangle, rounded rectangle, geometry | `push_clip_path`, `push_clip_rect` (not isolating), `push_clip_layer` | same | `push_clip_layer`, `push_layer` with a clip | none |
| Layers with opacity | `push_layer(clip, blend, opacity, mask, filter)` | same | `push_layer(blend, alpha, transform, clip)` | none |
| Blend modes | all 16 `Mix` and 14 `Compose` of peniko | some panic when not isolated [V README] | same enum | every `BitmapBlendingMode` of the contract maps (12 Porter-Duff, 15 mix functions); in hybrid mode the ones that panic must be drawn in an isolated layer |
| Opacity masks | a layer composed `DestIn` with the mask drawn into it (built); also `Mask` layers | mask layers panic: the `DestIn` layer is the way | `push_luminance_mask_layer` | none in CPU; to verify in the others |
| Images with interpolation | `ImageQuality::{Low, Medium, High}` = nearest, bilinear, bicubic | same | same | no mipmaps: Skia's medium and high quality when shrinking use them. Built in the drawing context, for every mode: the levels of `SkMipmap` and the two of them Skia samples (section 10.2) |
| Glyph runs by id and position | `glyph_run(font).font_size().hint().normalized_coords().glyph_transform().font_embolden().fill_glyphs()` (`glifo`) | same | `draw_glyphs(font)` with the same options | built for the CPU mode (stage 5; section 8, "Text"). Variable coordinates, synthetic bold (embolden) and italic (a skew as glyph transform) exist in all three; colour fonts: COLR and bitmap glyphs in `glifo` (`GlyphColr`, its `png` feature) and in `vello` (`scene.rs:599`); **no sub-pixel (LCD) text** in any of them: text is grey-scale anti-aliased, not closable short of an own rasterizer. Hinting in `glifo` is vertical only and is not applied under a rotation or a non-uniform scale (`glyph.rs:1674-1700`); it widens an emboldened outline in the units of the font, or in pixels when the glyph is hinted (`glyph.rs:1248`, `2087`), so an emboldened run is drawn without hinting; it reads the em square from `head` and panics without one (`glyph.rs:1736`), so a font without that table is not handed to it |
| Blur and drop shadow effects | `push_filter_layer` with `FilterPrimitive::{GaussianBlur, DropShadow}`; "experimental", panics with the thread pool | filters on the GPU; "complex filter graphs" panic | **none**: only `draw_blurred_rounded_rect` | built for the CPU mode: a filter layer where nothing clips the scene, else the layer drawn by `vello_cpu` into an image, which is also what a mode without filters gets (section 10.2). GPU: that image, or later a render-to-texture pass with an own blur shader |
| Box shadows | `fill_blurred_rounded_rect(rect, radius, std_dev, invert)`: one radius, `invert` for inset shadows | same | `draw_blurred_rounded_rect` | built: the closed form where it agrees with the Gaussian, else the shape blurred as an image (section 10.2). Its `std_dev` is not the Gaussian's but √2 times it [M] |
| Anti-aliasing modes | analytic; `set_aliasing_threshold(Some(n))` for aliased edges | same | `AaConfig::{Area, Msaa8, Msaa16}`, no aliased mode | GPU mode: `EdgeMode::Aliased` is not available (capability flag of the sink): the drawing context then asks for anti-aliased edges; snapping to pixels where the contract uses the mode for crisp lines is open |
| Offscreen render and read back | renders into memory | render to a texture, read back with `wgpu` (copy to a buffer, map) | same | none |
| Perspective transforms | `Affine` only | same | same | not closable in the renderer: the perspective column of a matrix is dropped (`to_affine`). The Skia backend passes a 4x4 matrix. Open; a 3D transform would have to be drawn into a layer and mapped as a mesh |

**Curve flattening [V][M].** `vello_common` flattens curves to a fixed quarter of a pixel (`flatten.rs:15-18`: `SQRT_TOL = 0.5`, `TOL = 0.25`). The lines are chords, so a round shape is drawn up to a quarter of a pixel thinner than it is. Against Skia the edge of an ellipse was visibly lighter: 1.03 % of the pixels of the `ellipse` scene differed by more than the tolerance. The scene of the CPU mode therefore flattens curves itself to a twentieth of a pixel (and expands strokes to their outline the same way) and hands the renderer lines: 0.38 %. The hybrid mode shares `vello_common` and needs the same; classic `vello` flattens on the GPU with its own tolerance [U].

### 1.4 What Vello does not give

| Need | Choice | State |
|---|---|---|
| Bounds, tight bounds, bounds with a pen | `kurbo`: `Shape::bounding_box` (tight, by curve extrema), `kurbo::stroke` for the outline of a pen | built |
| Hit tests of fill and stroke | `kurbo`: `Shape::winding` over the path with its figures closed, the fill rule applied by the backend; the stroke through its outline | built |
| Contour length, point and tangent at a distance, a part of a contour | own code over `kurbo` (`helpers/path_measure.rs`: `ParamCurveArclen::arclen`, `inv_arclen`, `subsegment`); Skia has `SkPathMeasure` | built. More exact than Skia's: an ellipse of 160 by 110 is 427.761 long (427.759 exactly), 427.073 to Skia [M] |
| Arcs of the stream geometry | `kurbo::SvgArc` and `Arc::from_svg_arc` (endpoint parameters of SVG, radii scaled up when too small, a line when degenerate: `svg.rs:405-470`), as cubic Béziers | built; agrees with Skia's `arcTo` to 0.0005 units in the bounds [M] |
| Boolean operations of combined geometries | `linesweeper` 0.5.0 (`binary_op(a, b, fill_rule, op)` on `BezPath`, curves kept as curves, output contours fill alike under both rules). Candidates looked at: `i_overlay` 9.0.1 (polygons only: curves would be flattened), `flo_curves` 0.8.1 (own path types, Apache-2.0 only), `kurbo` (none) | built. Risk: "early beta"; a panic inside it is caught and gives the empty geometry of the contract |
| Regions | own code (`vello_region_impl.rs`): rectangles and their decomposition into bands | built |
| Bitmap decode and encode | a crate a format, not the `image` crate over them (which adds `moxcms` and more to every build): `png`, `zune-jpeg` and `jpeg-encoder`, `gif`, `zune-bmp`; ICO and WBMP are own code | built for what the codecs of Skia decode in this workspace (PNG, JPEG, GIF, BMP, ICO, WBMP) and for the two formats the contract encodes (PNG, JPEG); WebP, which Skia decodes only when built with it, fails to load (section 10.4) |
| Pixel formats | own code (`helpers/pixel_format_helper.rs`): RGBA8888 and BGRA8888, premultiplied, not premultiplied, opaque | built: RGBA8888 and BGRA8888 in the three alpha formats, RGB565 and RGB32, for bitmaps from pixels, writeable bitmaps and framebuffers |
| Writeable bitmaps | own code: pixels under a lock, an image of them until they change | built |
| Font manager: installed fonts, matching, fallback by character | `fontique` 0.12: `Collection::load_system_fonts` (CoreText on macOS through `objc2-core-text`, DirectWrite, fontconfig), `family_names`, `Query` matching by weight, style and width, `fallback_families(key)` with a key of script and locale. Skia's font manager gives the same three things (`FontMgr::family_names`, `match_family_style`, `match_family_style_character` with a culture) | built (`font_manager_impl.rs`), verified on macOS [M]: the families, the match by weight, style and width, the data of a matched font (memory-mapped, by `SourceCache`) and what to synthesize (`FontInfo::synthesis`: the variation settings of a variable font, embolden, skew). Three things it does not give and how each is closed: **(1)** fallback is by script and language (`fallback_families(FallbackKey)`: on macOS a font CoreText returns for a sample text of the script), not by character: the script of the character is looked up with the Unicode data of the base library (`helpers/script_tag.rs` has the ISO 15924 codes, which the base library keeps to itself), the family is checked for the character, and a character without a script (a symbol, an emoji) or one the family lacks is searched in the default family, the generic families and then every installed family, once for a character and a culture; **(2)** `family_names` lists a family under each of its localized names (458 names for 320 families here): the list is made from the family identifiers; **(3)** `Collection::new` reads every font file of the system (a scan): the enumerated collection is shared by the font managers of a process and made when a font of the system is first asked for. It leaves out the PingFang families, whose outlines are in Apple's `hvgl` table that `skrifa` does not read, and falls back to Heiti (`backend/coretext.rs`, under the comment "HACK") |
| Glyph metrics and outlines for the glyph typeface | the base crate's own font tables (`src/FerroUI.Base/media/fonts`), which the glyph typeface of the port already reads from the tables a platform typeface serves: every metric is the base library's, the same number over either backend (section 8, "Text"). `skrifa` 0.44.0 (the version `glifo` links) for what the contract leaves to the backend: the tables by tag, the identity of the font, the axes of a variable font, and the outline of a glyph for the bounds, the intersections and the geometry of a glyph run | built (`vello_typeface.rs`) |

### 1.5 A common API across the three modes

- **`vello_api` 0.0.7 [V]**: `PaintScene` and a reusable `Scene`. Its own documentation: "Vello API is currently only released as a preview, and is not ready for usage beyond short-term experiments" and "TODO: Mention Renderer trait when it exists. Otherwise, this code isn't really usable yet" (`src/lib.rs:11`, `:20`). `vello_cpu` 0.3.0 and `vello_gpu` 0.3.0 do not depend on it (their `Cargo.toml`), it does not cover classic `vello`, and its MSRV is 1.92, above the workspace's 1.89: it cannot be built here. **Not used.**
- **`anyrender` 0.14** (with `anyrender_vello` 0.15, `anyrender_vello_cpu` 0.18, `anyrender_vello_hybrid` 0.11; MIT OR Apache-2.0): a third project's sink over all three. Only its metadata was looked at (`cargo info`); its sources were not read. **Not used**: it is one more moving interface between the contract and the renderers, and the three back ends release at different versions.
- **Decision.** The drawing context contract of FerroUI is already the common API above a backend. The Vello backend has one drawing context, one set of geometry, brush and pen conversions, and a small scene interface of its own, `IVelloSceneSink` (section 4), implemented directly over each renderer. `vello_cpu` and `vello_gpu` have the same methods under the same names, so the second implementation is nearly a copy of the first; `vello` gets its own.

## 2. The crate

`src/Vello/FerroUI.Vello`, package `ferroui-vello`. An application chooses it with `use_vello` on the application builder (`VelloApplicationExtensions`, in the crate, as `use_skia` is in Skia's); the options are `VelloOptions` bound in the locator. An application has one backend; only the comparison harness links both.

Dependencies, exact versions in the workspace manifest: `kurbo = "=0.13.1"`, `peniko = "=0.6.1"`, `vello_cpu = "=0.3.0"` (without default features: `std`, `u8_pipeline`; no thread pool, no PNG, no text), `linesweeper = "=0.5.0"`, `png = "=0.18.1"`. `Cargo.lock` gained 24 third-party packages (among them `glifo`, `skrifa`, `read-fonts` and `font-types`, which are optional dependencies of `vello_cpu` and are not compiled) and changed the version of none. In a build of the whole workspace `arrayvec` gains its `serde` feature (asked for by `linesweeper`) and `hashbrown` `default-hasher` (`vello_cpu`); a build of one application does not, since no crate but the harness depends on the backend. Stage 6 added `zune-core = "=0.5.3"`, `zune-jpeg = "=0.5.15"` (`std` and its SIMD paths, which are its default), `zune-bmp = "=0.5.2"` (`std`), `jpeg-encoder = "=0.7.1"` (`std`; not its `simd` feature) and `gif = "=0.14.2"` (`std`; not its color quantizer, which only its encoder uses): `Cargo.lock` gained these five and `weezl` 0.1.12 and changed the version of none. `wgpu`, `vello` and `vello_gpu` are not dependencies yet: they come with stages 7 and 8, behind features of the crate, so that an application of the CPU mode does not compile a graphics API.

Stage 5 added, at exact versions: the features `text` and `png` of `vello_cpu` (`text` compiles `glifo` 0.4.0, the glyph renderer, with `skrifa` 0.44.0, `read-fonts` 0.41.0 and `font-types` 0.12.6, which were in `Cargo.lock` already as optional dependencies; `png` adds nothing but the decoding of the bitmap glyphs of a colour font, the crate `png` being linked already), `glifo = "=0.4.0"` and `skrifa = "=0.44.0"` as direct dependencies (the embolden settings of a glyph run are a type of `glifo` that `vello_cpu` does not re-export; `skrifa` for the typeface), and `fontique = "=0.12.0"` with its default `system` feature. `Cargo.lock` gained 18 packages and changed the version of none: `fontique`, `parlance`, `memmap2`, the bindings `fontique` reaches the font interface of a platform with, each linked on its platform only (`objc2`, `objc2-encode`, `objc2-foundation` and `objc2-core-text` on Apple platforms, where `objc2-core-foundation` was linked already; `windows` and `windows-core` with six parts on Windows; `yeslogic-fontconfig-sys` and `dlib` on Linux; `roxmltree` 0.21.1 on Android, beside the 0.20 the workspace uses). On macOS a build of the backend now also compiles `glifo`, `skrifa`, `read-fonts`, `font-types`, `fontique`, `parlance`, `memmap2` and the `objc2` bindings; the build time was not measured.

`unsafe`: none in the crate.

## 3. File by file

| Skia backend | Vello backend | Shared logic | What differs |
|---|---|---|---|
| `lib.rs` | `lib.rs` | module list | |
| `skia_platform.rs`, `skia_options.rs`, `skia_application_extensions.rs` | `vello_platform.rs`, `vello_options.rs`, `vello_application_extensions.rs` | initialization, options for the render thread | options: the rendering modes in the order they are tried; no GPU resource limit or stencil option. The font manager is bound as in the Skia backend |
| `platform_render_interface.rs` | `platform_render_interface.rs` | every member | glyph runs and their geometry are built on the outlines of `skrifa`; a platform graphics context fails with stages 7 and 8; default pixel format RGBA8888 |
| `skia_backend_context.rs` (`SkiaContext`) | `vello_backend_context.rs` (`VelloContext`) | render targets of framebuffer surfaces, offscreen targets | software only so far |
| `drawing_context_impl.rs` | `drawing_context_impl.rs` and, for what stage 6 added, `drawing_context_impl/box_shadows.rs`, `effects.rs`, `scene_brushes.rs`, `acrylic.rs` | the state (opacity, render and text options, transforms), brush and pen logic, gradients kind by kind, tile brushes; the geometry of box shadows, the filters of effects, the transforms of scene brushes | records into `IVelloSceneSink`; the state stack is its own (section 5); no lease of a canvas; `PaintWrapper` lists what a paint of Skia does in one; blurs as section 10.2 has them |
| `geometry_impl.rs` and the seven geometry files | same names | everything but the path type | `VelloPath` (a `BezPath` and its fill rule) for `SkPath`; no `unsafe impl Send` |
| `helpers/pen_helper.rs` | `helpers/pen_helper.rs` | identical | |
| `helpers/sk_path_helper.rs` | `helpers/path_helper.rs` | the outline of a pen | `kurbo::stroke`; its figures are closed already |
| `helpers/drawing_context_helper.rs` | `helpers/drawing_context_helper.rs` | dash lengths | `render_async`/`wrap_skia_surface`, which draw onto a surface of Skia that another API made, have no counterpart: there is no such surface before the GPU modes (open) |
| `helpers/image_saving_helper.rs` | `helpers/image_saving_helper.rs`, `helpers/image_decoding_helper.rs` | options | `png` and `jpeg-encoder`; the decoders, which in the Skia backend are the codec of Skia inside `immutable_bitmap.rs` |
| (the mipmaps of Skia) | `helpers/mipmap_helper.rs` | | own code |
| `helpers/pixel_format_helper.rs`, `skia_sharp_extensions.rs` | `helpers/pixel_format_helper.rs`, `vello_extensions.rs` | conversions | to kurbo and peniko types; shape paths (rounded rectangle with elliptical radii) |
| (Skia's `SkPathMeasure`) | `helpers/path_measure.rs` | | own code |
| `skia_region_impl.rs` | `vello_region_impl.rs` | the contract | own bands |
| `immutable_bitmap.rs`, `writeable_bitmap_impl.rs`, `render_target_bitmap_impl.rs`, `i_drawable_bitmap_impl.rs` | same names | structure, locks, versions | a bitmap hands out a `peniko::ImageData` instead of drawing itself on a canvas |
| `framebuffer_render_target.rs` | `framebuffer_render_target.rs` | locking, properties | every frame goes the way of Skia's conversion shim: rendered in memory, written in the format of the framebuffer |
| `surface_render_target.rs` | `surface_render_target.rs` | layer, blit, snapshot | pixels in memory |
| `locked_framebuffer.rs` | not needed yet | | comes with the GPU surfaces |
| `picture_render_target.rs` | none | | only the drawing context of the Skia backend uses it (and a test of it), for scalable scene brushes; here their content is replayed into a surface at the resolution of the target (section 10.2) |
| `skia_typeface.rs` | `vello_typeface.rs` (`VelloTypeface`, `VelloFontFace`) | the contract: family name, weight, style, stretch, simulations, the stream, a table by tag | the bytes of the font file read with `skrifa` where Skia has a typeface object; `VelloFontFace` is what a glyph run keeps (the font data, the position in the variation space, the simulations), `Send + Sync` where the typeface is not; the outline of a glyph with the simulations applied (the rule of Skia's fake bold, the skew of -0.3); the values of the axes of a variable font, which the contract has no member for |
| `font_manager_impl.rs` | `font_manager_impl.rs` | every member, the rule for the simulations (bold from a weight of 600 on a lighter font, oblique for italic on an upright one) | `fontique` where Skia has `SkFontMgr`; the fallback for a character is put together here (1.4); `try_match_family_style` and `legacy_make_typeface` for what tests and callers ask of Skia's font manager directly |
| `glyph_run_impl.rs` | `glyph_run_impl.rs` | positions, the bounds placed at the pen positions, the contract | no text blob and no cache of them: a run is glyph indices and positions and the scene draws it; bounds and intersections are computed from the outlines |
| `sk_text_blob_builder_cache.rs` | none | | a pool of Skia objects |
| (the glyph members of the canvas) | `draw_glyph_run` of `IVelloSceneSink` with `VelloSceneGlyphRun`; `helpers/script_tag.rs` | | section 4 |
| `sk_cache_base.rs`, `sk_paint_cache.rs`, `sk_round_rect_cache.rs`, `two_level_cache.rs` | none | | pools of Skia objects; paints and paths here are plain values |
| `i_skia_api_lease_feature.rs` | none; a lease of the scene if a custom draw operation needs one (open) | | |
| `gpu/` (`ISkiaGpu`, Graphite on Metal, Ganesh on OpenGL), `metal/` | `scene/` (`IVelloSceneSink`, `vello_cpu_scene_sink.rs`); stage 7 `vello_hybrid_scene_sink.rs`, stage 8 `vello_gpu_scene_sink.rs`, `gpu/` for the device, the queue and window surfaces | | |
| `tests.rs`, `unit_tests/` | `tests.rs`, `unit_tests.rs` | the contract tests | section 8 |

**Neutral code that should be shared, not copied** (listed; nothing was moved in this task, and the Skia backend was not touched):

| Code | Today | Would move to |
|---|---|---|
| `helpers/pen_helper.rs` (`get_hash_code`) | identical in both crates | `ferroui_base::media` beside `IPen` |
| `try_create_dash_effect` up to the Skia call (dash lengths times thickness, an odd count doubled) | both `helpers/drawing_context_helper.rs` | a function of `IPen`/`IDashStyle` in the base crate |
| `get_relative_transform`, `get_absolute_transform`, `combine` of the drawing context | both drawing contexts | `ferroui_base::rendering::utilities` beside `TileBrushCalculator` |
| The geometry of gradients (points, radii, the reversal of stops for an offset origin, the rotation of a conic gradient) | both `configure_gradient_brush` | a description of a gradient in the base crate that a backend turns into its shader |
| The tile transform of `configure_tile_brush` and the scene brush set-up | both | the same utilities module |
| Text rendering mode resolution of `draw_glyph_run` | Skia | base crate, before stage 5 copies it |
| `FillPath`, `GeometryImplBase`, `Shared`, `impl_geometry_impl!`, the stream context's stroke/fill bookkeeping | both `geometry_impl.rs`/`stream_geometry_impl.rs`, generic over the path type | a generic in the base crate (`platform/internal`) with the path type as a parameter |
| The contract tests (`tests.rs` up to the bitmaps) | copied with their expectations | a test-support module generic over `IPlatformRenderInterface`, run by each backend |
| **The Metal contracts** `metal/i_metal_device.rs`, `metal/i_metal_external_objects_feature.rs` | in the Skia crate (upstream has them in `Avalonia.Skia`); the native backend depends on `ferroui-skia` for them | a neutral place (`ferroui_base::platform` or a small `ferroui-metal` crate like `ferroui-opengl`), before stage 8: the Vello GPU modes need the same device and surface session |

## 4. The scene interface and the rendering modes

```
IDrawingContextImpl (contract)
  DrawingContextImpl          one, for every mode: state stack, brushes, pens, shapes
    IVelloSceneSink           fill, stroke, push/pop clip, push/pop layer (blend, opacity), render_to_pixels, capabilities
      VelloCpuSceneSink       vello_cpu::RenderContext                 built
      VelloHybridSceneSink    vello_gpu::Scene + Renderer (wgpu)       stage 7
      VelloGpuSceneSink       vello::Scene + Renderer (wgpu, compute)  stage 8
```

- Every call of the sink carries its state (transform, paint with its transform, fill rule, blend mode, anti-aliasing); the sink keeps none for its caller. That is the shape of `vello::Scene`; the two sparse-strips scenes get the state set before each call.
- `VelloSceneCapabilities` answers what a mode lacks (blend layers, aliased edges, image paints, read back); the drawing context asks before it uses such a feature and fails with a message rather than draw something else.
- `VelloRenderingMode { Cpu, Hybrid, Gpu }` and `VelloOptions::rendering_modes`, the order in which they are tried (default: GPU, hybrid, CPU). `scene::try_create_scene_sink` fails for a mode that is not built or not available with its reason; `create_scene_sink` takes the first that works and panics with every reason when none does. `DrawingContextImpl::rendering_mode` tells which one draws.
- Glyph runs are a call of the interface (stage 5): `draw_glyph_run(&VelloSceneGlyphRun, transform, paint, anti_alias)`, the run being the font data, the em size, the normalized variation coordinates, the glyphs with their origins, the widening of the bold simulation, the skew of the oblique one and whether to hint. It is a required member: **the sinks of stages 7 and 8 have to implement it** (`vello_gpu::Scene::glyph_run` has the same builder as `vello_cpu`'s; `vello::Scene::draw_glyphs` takes the same options). The CPU sink draws it with `RenderContext::glyph_run`. Filters (stage 6) join the interface the same way.
- Filters are members with capability flags (stage 6): `filter_capabilities` (`VelloSceneFilterCapabilities { filter_layers, blurred_rounded_rects }`), `push_filter_layer` and `fill_blurred_rounded_rect`, at the end of the trait with default bodies that say "none" and fail when called, so a sink that does not implement them is drawn for as section 10.3 describes.

**Which renderer serves which surface.**

| Surface | Renderer | Why |
|---|---|---|
| Offscreen bitmaps, render target bitmaps, layers, tests, the headless platform | CPU | no device, deterministic, and the mode `vello_cpu`'s authors call their most mature |
| The desktop window (macOS, Metal) | **Hybrid first, classic GPU second** | Both draw into a `wgpu::TextureView`. The hybrid one has what a UI needs today that classic `vello` lacks: filters (blur, drop shadow), aliased edges, and the same code path as the CPU mode (the same `vello_common`, so the same pixels as the tests measure). Classic `vello` wins on scenes dense with vector paths, which a UI rarely is, and needs compute shaders. The option order lets an application prefer either |
| The browser (later) | WebGPU: hybrid or GPU through `wgpu`; WebGL2: hybrid with its `webgl` feature; no GPU: CPU into a 2D canvas | staged with the `wasm32-unknown-unknown` configuration of `browser-platform.md`; not built now. `vello` needs WebGPU; only `vello_gpu` has a WebGL2 path |

The desktop surface: the native backend hands the Skia backend a Metal device and command queue and, per frame, a session with the texture of the drawable (`IMetalDevice`, `IMetalPlatformSurfaceRenderingSession`). The GPU modes make a `wgpu::Device` over that Metal device and wrap the texture of each session (`Adapter::create_device_from_hal` and `Device::create_texture_from_hal` of `wgpu` 30, both `unsafe fn`: the one place the crate will need `unsafe`), or create a `wgpu` surface over the layer (`SurfaceTargetUnsafe::CoreAnimationLayer`) if the native backend exposes it. Decided in stage 7, after the Metal contracts have moved (section 3).

## 5. The scene model

- A drawing context is created by a render target with a sink of the size of the target. The contract's immediate calls are recorded into the scene; `dispose` renders it (`render_to_pixels`) and presents.
- **What the target held.** A scene replaces the pixels of its target. A target that keeps its content (a render target bitmap that is drawn into again, a framebuffer that retains its frame, a layer) starts its scene with that content drawn as an image. `clear` outside every clip and layer starts the scene over; inside a clip it fills with the `Copy` composition, which is what the compositor's "clip the dirty rectangle, clear, redraw" needs.
- **State stack.** `(transform, kind)` per pushed state, `kind` being a clip, a layer or nothing; every pop of the contract (`pop_clip`, `pop_layer`, `pop_opacity`, `pop_opacity_mask`, `pop_geometry_clip`) ends the innermost one and restores the transform of its push, as `SKCanvas.Restore` does. The scene itself keeps no transform stack.
- **Clips** are not isolating (`push_clip_path`). A rectangle clip and a region clip are not anti-aliased, a rounded and a geometry clip are: what the Skia backend asks of its canvas.
- **Opacity** multiplies into the paints (as Skia's default), or is a layer when `use_opacity_save_layer` or `requires_full_opacity_handling` is set.
- **Opacity mask**: a layer; at the pop a second layer composed `DestIn` with the mask painted over the whole target in the transform of the push.
- **Caches.** Skia's pools of paints and round rects have no counterpart: a paint is a small value and a rounded rectangle a path built on the spot. A cache that will matter: the flattened path of a geometry per scale (the sink flattens on every draw today), and the image of a bitmap in the renderer's form (kept per frame by the CPU sink). A text blob of Skia has no counterpart: a glyph run is positioned glyph ids, and the renderer has the caches (`glifo` keeps the outlines of the glyphs of a frame and its hinting instances in the `Resources` of the sink; its glyph atlas, which it calls experimental, is not turned on). The font keeps the bounds of the outlines of its glyphs, which every new glyph run asks for.

## 6. Threads

- The contracts decide what crosses threads: `IGeometryImpl`, `IGlyphRunImpl`, shared bitmaps are `Send + Sync`; drawing contexts, render targets, layers and the backend context stay on the thread that renders (`render-thread.md`, section 3).
- Geometries hold a `BezPath` behind `Arc` and their caches behind `Mutex`: `Send + Sync` without `unsafe` (the Skia backend needs two `unsafe impl Send`). Bitmaps hold their pixels and their image under one lock.
- The sinks, `RenderContext` and the `wgpu` objects of a context are created on the render thread inside the server graph and never leave it; under the compositor lock both threads may render in turn, never at once.
- `wgpu::Device`, `Queue` and `Surface` are `Send + Sync` on native targets (`wgpu-30.0.1/src/api/*.rs`, `static_assertions` under `cfg(send_sync)`) and not on WebAssembly without the `fragile-send-sync-non-atomic-wasm` feature. The device and the queue belong to the backend context (`VelloContext`), as the Graphite context belongs to `SkiaContext`; `vello::Renderer` is `Send`.
- Device loss: `wgpu` reports it through the device-lost callback and failing surface acquisition. The context sets a flag that `IPlatformRenderInterfaceContext::is_lost` returns; the compositor then recreates the context and its targets, the path the Skia backend uses for a lost Metal or OpenGL context. Layers of a lost device report `is_corrupted`.
- The CPU mode renders on the calling thread: `vello_cpu`'s thread pool (`multithreading`) is not compiled in. It is an option for large software frames later; filters panic with it today (1.3).

## 7. Stages

| # | Stage | Check | State |
|---|---|---|---|
| 1 | Crate, dependencies, platform, options, `use_vello`, render interface and context | builds; `cargo check --workspace` | done |
| 2 | Geometries on kurbo | ported geometry tests; the geometry table of section 8 | done |
| 3 | CPU render path: render targets, drawing context (shapes, brushes, pens, clips, layers, opacity, masks), bitmaps | ported contract tests | done |
| 4 | Comparison harness | the scene table of section 8, a bound per scene | done |
| 5 | Text: `glyph_run_impl.rs`, `vello_typeface.rs`, `font_manager_impl.rs` (`fontique`), glyph run geometry, intersections; the `text` feature of `vello_cpu` | the text suites of `unit_tests/media` of the Skia backend (fonts, glyph runs, text formatting, 326 tests) and its `text_tests.rs` (14) ported; 13 text scenes and nine numeric comparisons in the harness; a window of the Fluent theme on the headless platform (section 8, "Text") | done for the CPU mode |
| 6 | Effects, box shadows, scene brushes (visual and drawing brushes), acrylic materials, render options, JPEG and the other codecs, RGB565, mipmapped downscaling | the effect, scene brush, acrylic and bitmap tests of the Skia backend ported; 61 scenes and the codec tests in the harness (section 10) | done, but `render_async` (no surface of another API before the GPU modes) and the `HitTesting` suite (which needs the font services of the tests of stage 5) |
| 7 | Hybrid mode: `VelloHybridSceneSink` offscreen on a headless `wgpu` device, then the desktop window (after the Metal contracts moved) | the harness compares hybrid against Skia and against CPU; ControlCatalog runs with `use_vello` | open |
| 8 | GPU mode: `VelloGpuSceneSink`, blur passes | the same, three modes compared | open |
| 9 | Browser: WebGPU, WebGL2, CPU | `browser-platform.md` | open |

**Members that fail with their stage** (nothing pretends to work): loading WebP and whatever else is not PNG, JPEG, GIF, BMP, ICO or WBMP (a load error, as for data in no format); `create_backend_context` with a platform graphics context (stages 7, 8; a panic); the hybrid and GPU modes (`VelloRenderingModeUnavailable`).

## 8. How correctness is measured

1. **The contract tests of the Skia backend, ported** (`src/Vello/FerroUI.Vello/tests.rs`, `unit_tests.rs`): the same scenes and expectations, pixel by pixel, for everything that tests the contract and not Skia. 83 tests pass: 79 in `tests.rs` (the drawing context, the geometries, the bitmaps, the framebuffer render target and the software context, the image brushes, and tests that what is not built fails with its stage and that a target keeps its content) and 4 of the suites `RenderBoundsTests`, `CombinedGeometryImplTests` and `DrawingContextImplTests`. Stage 5 added the text tests (below, "Text"): 430 tests in the crate, 421 pass and 9 are ignored as they are in the Skia backend. Stage 6 ported the tests of box shadows, effects, scene brushes, acrylic and bitmaps (section 10.4). Left: `HitTesting`.
2. **The comparison harness** (`tests/FerroUI.RenderBackends.Comparison`, crate `ferroui-render-backends-comparison`): the same scenes through the contracts by Skia raster and by every Vello mode that is built, 200 by 200 pixels, and for each scene the share of pixels of which a channel differs by more than 32 of 255. Each scene has a recorded bound (the measured share, half as much again and 0.05 %); `scenes_stay_within_their_bounds` fails when a scene exceeds it. `cargo test -p ferroui-render-backends-comparison -- --nocapture` prints the tables below.

### Scenes: Vello CPU against Skia raster (2026-10-09)

| Scene | Pixels beyond the tolerance | Largest difference | Mean difference | Bound |
|---|---|---|---|---|
| `rectangle` | 0.000 % | 1 | 0.005 | 0.05 % |
| `aliased_rectangle` | 0.037 % | 235 | 0.042 | 0.11 % |
| `rounded_rectangle` | 0.080 % | 62 | 0.066 | 0.17 % |
| `elliptical_corners` | 0.033 % | 70 | 0.054 | 0.10 % |
| `ellipse` | 0.375 % | 66 | 0.211 | 0.62 % |
| `lines` | 0.000 % | 18 | 0.197 | 0.05 % |
| `curved_path` | 0.210 % | 83 | 0.163 | 0.37 % |
| `star_non_zero` | 0.005 % | 36 | 0.031 | 0.06 % |
| `star_even_odd` | 0.005 % | 71 | 0.042 | 0.06 % |
| `stroke_flat_miter` | 0.000 % | 14 | 0.052 | 0.05 % |
| `stroke_round_round` | 0.052 % | 68 | 0.079 | 0.13 % |
| `stroke_square_bevel` | 0.000 % | 23 | 0.073 | 0.05 % |
| `stroke_miter_limit` | 0.000 % | 18 | 0.052 | 0.05 % |
| `dashes` | 0.468 % | 156 | 0.364 | 0.76 % |
| `linear_gradient_pad` | 0.000 % | 1 | 0.108 | 0.05 % |
| `linear_gradient_repeat` | 0.000 % | 1 | 0.246 | 0.05 % |
| `linear_gradient_reflect` | 0.000 % | 1 | 0.248 | 0.05 % |
| `linear_gradient_translucent` | 0.000 % | 2 | 0.240 | 0.05 % |
| `radial_gradient_pad` | 0.000 % | 1 | 0.129 | 0.05 % |
| `radial_gradient_repeat` | 0.000 % | 1 | 0.230 | 0.05 % |
| `radial_gradient_reflect` | 0.000 % | 1 | 0.347 | 0.05 % |
| `radial_gradient_elliptical` | 0.000 % | 1 | 0.074 | 0.05 % |
| `radial_gradient_offset_origin` | 0.000 % | 1 | 0.082 | 0.05 % |
| `conic_gradient` | 0.000 % | 1 | 0.224 | 0.05 % |
| `gradient_with_transform` | 0.058 % | 53 | 0.185 | 0.14 % |
| `nested_clips` | 0.095 % | 55 | 0.064 | 0.20 % |
| `transformed_clip` | 0.007 % | 235 | 0.011 | 0.07 % |
| `opacity_layers` | 0.000 % | 14 | 0.163 | 0.05 % |
| `opacity_mask` | 0.000 % | 24 | 0.169 | 0.05 % |
| `layer` | 0.015 % | 44 | 0.046 | 0.08 % |
| `tile_repeated` | 0.000 % | 0 | 0.000 | 0.05 % |
| `tile_flipped` | 0.000 % | 0 | 0.000 | 0.05 % |
| `tile_transformed` | 0.000 % | 0 | 0.000 | 0.05 % |
| `tile_single_transformed` | 0.133 % | 126 | 0.060 | 0.25 % |
| `bitmap` | 0.100 % | 59 | 0.208 | 0.20 % |
| `transforms` | 0.253 % | 78 | 0.368 | 0.43 % |
| `combined_union` | 0.110 % | 67 | 0.089 | 0.22 % |
| `combined_intersect` | 0.060 % | 60 | 0.042 | 0.15 % |
| `combined_xor` | 0.140 % | 67 | 0.123 | 0.26 % |
| `combined_exclude` | 0.025 % | 66 | 0.035 | 0.09 % |
| `geometry_group` | 0.195 % | 59 | 0.247 | 0.35 % |
| **all 41 scenes** | **0.060 %** | | | |

Reading: straight edges, every gradient kind and spread method, opacity, masks and tiles agree to the last digit of a color or nearly (largest difference 0 to 24). What differs is on curved edges (`ellipse`, `curved_path`, `transforms`, the combined geometries: after the finer flattening the two renderers still weigh the pixels of a curved edge a little differently; which of them is nearer the exact coverage was not measured), on aliased edges (a pixel either way, hence 235), and in `dashes`, where the dashes of a curve sit where each backend measures the curve (see the lengths below). Before the two corrections this harness led to, the mean was 0.146 % (curve flattening, 1.3) and `transformed_clip` 0.755 % (the anti-aliasing of rectangle clips, section 5).

### Geometry queries: Vello against Skia (2026-10-09)

Differences in the units of the shapes (which lie in 200 by 200); hit tests on a grid of 4489 points.

| Shape | Bounds | Render bounds of 4 pens | Contour length | Point and tangent along the contour | Fill hit tests that differ | Stroke hit tests that differ (thin flat miter, thick round round, thick square bevel, dashed) |
|---|---|---|---|---|---|---|
| `rectangle` | 0.00000 | 0.00000 | 0.00000 | 0.00002 | 0 | 0, 0, 0, 0 |
| `ellipse` | 0.00000 | 0.00079 | 0.68769 | 0.76758 | 0 | 10, 11, 8, 14 |
| `line` | 0.00000 | 0.00001 | 0.00001 | 0.00001 | 0 | 0, 0, 0, 0 |
| `curves` | 0.00000 | 0.02736 | 0.35943 | 0.35944 | 0 | 1, 2, 2, 6 |
| `arc_small_clockwise` | 0.00001 | 0.06879 | 0.05653 | 0.13480 | 0 | 2, 0, 0, 0 |
| `arc_large_clockwise` | 0.00021 | 0.17539 | 0.57024 | 0.54561 | 0 | 2, 6, 5, 7 |
| `arc_small_counter_clockwise` | 0.00000 | 0.06879 | 0.05653 | 0.16260 | 0 | 2, 1, 1, 0 |
| `arc_large_rotated` | 0.00049 | 0.23430 | 0.34265 | 0.42188 | 0 | 5, 7, 7, 2 |
| `arc_radii_too_small` | 0.00000 | 0.00066 | 0.22920 | 0.27818 | 0 | 1, 2, 2, 0 |
| `star_even_odd` | 0.00001 | 0.00042 | 0.00003 | 0.00005 | 0 | 0, 0, 0, 0 |
| `transformed` | 0.00001 | 0.11081 | 0.28377 | 0.28376 | 0 | 1, 1, 1, 2 |
| `group` | 0.00000 | 0.00001 | 0.00000 | 0.00002 | 0 | 9, 10, 4, 14 |
| `combined_union` | 0.00050 | 0.02447 | not comparable | not comparable | 0 | 6, 3, 2, 178 |
| `combined_intersect` | 0.00050 | 0.05889 | not comparable | not comparable | 0 | 1, 4, 2, 39 |
| `combined_xor` | 0.00050 | 0.09958 | not comparable | not comparable | 0 | 7, 7, 2, 242 |
| `combined_exclude` | 0.00000 | 0.00000 | not comparable | not comparable | 0 | 1, 4, 0, 182 |
| `widened` | 0.02388 | 0.69231 | not comparable | not comparable | 2 | 7, 1, 9, 106 |

Reading: bounds agree to Skia's single precision; fills agree on every point. Lengths of curves differ by up to 0.7 units because Skia measures along chords: the test `the_length_of_an_ellipse` shows the Vello backend exact to 0.002 and Skia 0.69 short. Strokes of curves disagree on up to 0.3 % of the points, those within about a twentieth of a unit of the edge of the stroke (each backend builds the outline of a stroke to its own tolerance). The contour of a combined geometry is not comparable: the boolean operations of the two backends give the same area (0 fill differences) as figures that begin elsewhere, so lengths, points and the dashes of a pen differ; that the dashed column is the outlier there fits this, and it was not examined further. Bounds of the test: 0.001 (bounds), 0.3 (render bounds), 1.0 (lengths and points), 0 (fill hit tests), 15 and 20 of 4489 (strokes; dashed on comparable contours).

**A behaviour kept from Skia on purpose:** `contour_length` and the queries along the contour measure the first contour of a geometry that has a length, as `SkPathMeasure` does, although the contract's comment speaks of all contours.

### Text: stage 5 (2026-10-09)

Measured on macOS (Darwin 25.6, arm64) with the fonts of the Skia backend's unit tests (`src/Skia/FerroUI.Skia/test_assets`, 28 files, included from there by both test crates and not copied) and the fonts of the system. `cargo test -p ferroui-render-backends-comparison text_tests -- --nocapture` prints every table of this section.

**The font stack.** A platform typeface is the bytes of a font file read with `skrifa` (`VelloTypeface`); the glyph typeface over it is the base library's (`GlyphTypeface`: metrics, character map, advances, glyph metrics from the tables the typeface serves); the shaper is HarfBuzz over the same tables; the fonts of the system come from `fontique`; glyphs are drawn by `glifo` inside `vello_cpu`. Embedded fonts (`resm:` and `fonts:` collections) are the base library's: they reach the backend through `try_create_glyph_typeface_from_stream` only, and nothing in them is specific to a backend (the suites `EmbeddedFontCollectionTests`, `CustomFontCollectionTests` and `FontCollectionTests` pass unchanged).

**Tests ported from the Skia backend.**

| Suite of the Skia backend | Tests | Here | Passing | What changed |
|---|---|---|---|---|
| `text_tests.rs` | 14 | `text_tests.rs`, the same names but `skia_platform_registers_the_font_manager`, which is `vello_platform_registers_the_font_manager` | 14 | where a test looks into Skia (4 of them): the font of Skia (`is_embolden`, `skew_x`) is asked of the outline of a glyph, which is wider by the width of Skia's fake bold and leans by 0.3 of its height; `unichar_to_glyph` of the character map of the font; `is_bold`/`is_italic` of the typeface Skia matched, of the typeface `try_match_family_style` gives; the identity of text blobs per text options, of the pixels (the same options give the same pixels, hinting changes them, light and strong hinting are the same, an unaligned baseline changes them, and the sub-pixel mode gives the pixels of the grey-scale mode) |
| `unit_tests/media` (fonts, glyph runs, `text_formatting`) | 330 | `unit_tests/media`, file by file | 317 of 326, 9 ignored as there (fonts of Windows, a profiling test, one whose values "depend on the Skia platform backend") | the two bitmap suites (4 tests) are not text and were not copied; `CustomFontManagerImpl` and the font manager of `FontManagerTests` fall back to the font manager of the backend where upstream's fall back to `SKFontManager.Default`; `SKTypeface.Equals` is the comparison of the font data, the variation position and the simulations; `Similar_Runs_Have_Same_InkBounds_After_Blob_Creation` asks for the intersections where it built a text blob |
| own | | `text_tests.rs`, below the ported ones | 6 | simulated glyph runs against the fill of their outlines; the axes of a variable font (identity, coordinates, outline, ink); intersections and bounds of "Ho" in numbers; a font without a `head` table; fallback by script, language and character; the list of families |

Every expectation that tests the contract is unchanged: metrics, advances, bounds, hit tests through the text layout, line breaking. No test asserts Skia's pixels, so none had to be compared by tolerance or left out.

**Typefaces and metrics, in numbers** (Vello against Skia, all 28 test fonts):

| Compared | Result |
|---|---|
| Whether the font is read | the same 28 (both read the bitmap font of Apple and the font without a Unicode character map) |
| Family name | the same for all 28 (the typographic family where the font has one: `Inter Variable`, `Source Serif 4 36pt`, `Noto Sans Arabic NoLayout`) |
| Weight, style, stretch | the same for all 28 (`MiSans-Normal` is 305 in both; the two subsets of Noto Sans CJK are 100) |
| Every table of the table directory (9 to 24 a font), byte for byte | the same |
| Font metrics of the glyph typeface: em, ascent, descent, line gap, underline and strikethrough position and thickness, fixed pitch | the same for the 27 fonts that have a glyph typeface (neither backend gets one for `TestFontNoCmap412.ttf`) |
| Horizontal and vertical advance and the glyph metrics of every glyph (2 to 29 758 a font) | 0 differ |
| The glyphs of seven characters | the same |
| Shaping of a Latin, an Arabic and a Hebrew text with each font: glyphs, clusters, advances, offsets | identical |
| A paragraph of 113 characters at 15 in 170 (Inter): the lines, their widths, heights and baselines, the rectangle of every character, 1600 point hit tests | identical (6 lines, 157.649 by 108.920) |

They are the same numbers by construction: the metrics are the base library's, read from the tables, and the tables are the same bytes. Ascent, descent, line gap and underline position do not come from the backend at all.

**Glyph runs.**

| Compared | Result | Reading |
|---|---|---|
| Bounds of 36 552 runs of one glyph (the first 300 glyphs of each font at 9, 12, 16, 24, 40 and 72) | 25 fonts: the same in 32 810 of 32 952, one pixel apart in 142, never more | both take the bounds of the outline at the origin, round them out to whole pixels and add one on each side (Skia: the bounds of the glyph mask of its CoreText scaler; here `glyph_run_impl::pixel_bounds`, written to match). The 142 have an edge on a pixel boundary, where single and double precision round differently; in them Skia's bounds are the larger |
| The same, `NISC18030.ttf` | differ by up to 73 | a bitmap font of Apple (`bhed`, `bdat`): Skia measures the bitmaps, which no renderer of the Vello project draws; the run has no bounds here and draws nothing |
| The same, `TwitterColorEmoji-SVGinOT.ttf` | differ by up to 30 at 72 | Skia measures the SVG documents of the glyphs; the Vello backend measures and draws the plain outlines the font also has |
| Intersections with six bands, a line of Inter at 24 | the same number of intervals (0 to 24); the ends agree to 0.34 pixels at worst, 0.03 for the band of an underline | Skia takes the control points of a curve that lie in the band for points of the outline (`SkGlyph`'s intercepts), so its interval of a round glyph is a little wider; here the curve is intersected and its extrema taken |
| The lines an underline, a strikethrough and an overline draw (text layout, "jumping gypsy") | the same lines: 7, 1 and 1, the ends of the underline within 0.06 | |
| The geometry of a run (Inter plain, oblique, bold; Source Serif italic; Noto Sans Arabic; Cascadia Code) at 64: bounds, and 10 000 fill hit tests each | bounds to four decimals, 0 hit tests differ | also emboldened: `kurbo::expand_path` by half the width of Skia's fake bold is the outline Skia gives |

**The font managers** (the fonts of this machine):

| Compared | Skia | Vello |
|---|---|---|
| Default family | Helvetica | Helvetica (the first sans-serif family of `fontique`) |
| Installed families | 196 | 320: 192 of Skia's and 128 more (families CoreText does not list by name but matches: `Courier`, `Athelas`, the Noto Sans fonts of single scripts, the font of the interface as `System Font`). Not offered: the four PingFang families (1.4) |
| Helvetica, Times New Roman, Courier New, Menlo, Arial in five combinations of style and weight: family, weight, style, simulations | | the same in all 25; a family that is not installed gives none in both |
| Fallback for a character (culture en-US): Latin, Cyrillic, Greek, Hebrew, Arabic, Devanagari, Thai, Hiragana, Hangul, an emoji, infinity, the euro sign, Deseret | Helvetica, Helvetica, Helvetica, Lucida Grande, Geeza Pro, Kohinoor Devanagari, Thonburi, Hiragino Sans, Apple SD Gothic Neo, Apple Color Emoji, Helvetica, Helvetica, Apple Symbols | the same 13 |
| Han (U+4E2D), culture ja-JP | Hiragino Sans | Hiragino Sans |
| Han, cultures en-US and zh-CN | PingFang SC | Heiti SC (PingFang is not offered) |
| A snowman (U+2603), an arrow (U+2192) | Hiragino Sans, Lucida Grande | STIX Two Math, System Font: a character without a script has no fallback of the system here, and the first family that has it in the order of 1.4 is taken |
| The fonts of the fallback scene (Latin, a Hebrew word, two Han characters, two kana and the emoji U+1F600, laid out in Inter) | Inter, Lucida Grande, PingFang SC, Hiragino Sans W3, Apple Color Emoji | Inter, Lucida Grande, Heiti SC, Heiti SC, Apple Color Emoji (the base library asks the platform once for a group of scripts, and Heiti SC has the kana) |

**Text scenes: Vello CPU against Skia raster** (200 by 200, the measure of the scenes above):

| Scene | Pixels beyond the tolerance | Largest difference | Mean difference | Bound | Ink coverage of the scene, Skia | Vello |
|---|---|---|---|---|---|---|
| `text_latin_sizes` (Inter at 9, 12, 16, 24, 40) | 4.235 % | 168 | 2.529 | 6.41 % | 9.456 % | 8.186 % |
| `text_interface_sizes` (11 to 20, origins between pixels) | 3.215 % | 92 | 2.339 | 4.88 % | 8.594 % | 7.478 % |
| `text_mixed_scripts` (Latin, Arabic, Hebrew, a font each) | 3.297 % | 235 | 1.946 | 5.00 % | 6.353 % | 5.473 % |
| `text_mixed_scripts_with_fallback` (the fonts of the row above) | 5.955 % | 255 | 6.153 | 8.99 % | 2.956 % | 2.931 % |
| `text_simulations` (plain, bold, oblique, both) | 5.470 % | 143 | 2.931 | 8.26 % | 10.574 % | 10.581 % |
| `text_variable_axis` (Inter Variable at weights 100, 400, 900) | 5.103 % | 174 | 3.088 | 7.71 % | 11.461 % | 9.873 % |
| `text_decorations` (underline, strikethrough, overline) | 4.263 % | 131 | 2.095 | 6.45 % | 9.085 % | 8.004 % |
| `text_rotated` (a rotation; a rotation and a scale) | 1.448 % | 123 | 0.836 | 2.23 % | 3.393 % | 2.998 % |
| `text_scaled` (12 at 2.5, 48 at one half) | 1.765 % | 125 | 1.117 | 2.70 % | 3.931 % | 3.356 % |
| `text_clipped` (behind a rounded clip) | 2.080 % | 201 | 1.110 | 3.17 % | 6.881 % | 6.327 % |
| `text_gradient` (a linear gradient over the bounds of the run) | 3.408 % | 177 | 1.375 | 5.17 % | 6.634 % | 5.929 % |
| `text_aliased` | 0.190 % | 235 | 0.330 | 0.34 % | 4.499 % | 4.449 % |
| `text_translucent` (a translucent brush, an opacity) | 1.183 % | 92 | 0.652 | 1.83 % | 14.507 % | 14.234 % |
| **all 13 text scenes** | **3.201 %** | | | | | |

The other 41 scenes are unchanged (0.060 % on average).

**Where the difference comes from.** The same lines of Inter drawn four ways: as glyph runs and as the fill of the geometry of the runs (the outlines of the glyphs without hinting), by each backend. Share of pixels beyond the tolerance, the mean difference in parentheses:

| Em size | Skia's glyph run against Skia's fill of the outlines | Vello's glyph run against Skia's glyph run | Vello's glyph run against Skia's fill of the outlines | The same without hinting | Vello's fill of the outlines against Skia's | Coverage: Skia glyph run, Skia outlines, Vello glyph run |
|---|---|---|---|---|---|---|
| 9 | 1.915 % (1.814) | 1.320 % (1.654) | 0.320 % (0.850) | 0.260 % (0.828) | 0.125 % (0.708) | 5.92 %, 5.29 %, 5.27 % |
| 12 | 4.183 % (2.793) | 3.797 % (2.631) | 0.807 % (1.295) | 0.427 % (1.129) | 0.235 % (0.836) | 8.66 %, 7.55 %, 7.60 % |
| 16 | 5.900 % (3.558) | 5.938 % (3.287) | 1.135 % (1.530) | 0.760 % (1.362) | 0.427 % (1.016) | 11.93 %, 10.41 %, 10.38 % |
| 24 | 7.960 % (4.622) | 7.855 % (4.543) | 1.812 % (1.911) | 1.265 % (1.534) | 0.335 % (1.026) | 16.23 %, 14.08 %, 13.95 % |
| 40 | 8.840 % (5.419) | 8.460 % (5.123) | 0.588 % (0.890) | 0.608 % (0.880) | 0.285 % (0.757) | 19.11 %, 16.35 %, 16.45 % |
| 72 | 4.633 % (3.291) | 4.692 % (3.252) | 0.128 % (0.299) | 0.130 % (0.294) | 0.315 % (0.376) | 16.19 %, 14.50 %, 14.50 % |
| 28, bold simulation | 8.387 % (4.122) | 9.205 % (5.001) | 1.538 % (1.593) | 1.538 % (1.593) | 0.645 % (1.356) | 17.56 %, 19.57 %, 20.08 % |
| 28, oblique simulation | 8.357 % (4.896) | 8.090 % (4.732) | 1.325 % (1.365) | 1.343 % (1.350) | 0.378 % (0.904) | 16.23 %, 13.80 %, 13.78 % |

Reading:

- **A glyph run of the Vello backend is the fill of its outlines.** It is 0.1 to 1.8 % from the outlines as Skia fills them, and its coverage is theirs to 1 % (2.6 % for the emboldened outline, whose overlapping contours the two renderers anti-alias differently).
- **Skia on macOS does not draw the outlines.** Its glyph runs are 2 to 9 % from its own fill of the same outlines and cover 12 to 18 % more: Skia draws glyphs with CoreText, which makes stems heavier than the outline is. This, and not an error of the Vello backend, is what the text scenes measure: text in the Vello backend is lighter than text in the Skia backend on macOS by about an eighth, at every size. On a platform where Skia rasterizes outlines itself the scenes will measure something else; the bounds above are this platform's.
- **The bold simulation**: Skia on macOS draws it lighter (17.56 %) than the outline it gives for the same run (19.57 %); the Vello backend draws the outline. The total coverage of `text_simulations` agrees by chance of the two effects.
- **Hinting** (vertical, by `glifo`) changes the share by up to 0.5 % of the pixels; Skia's CoreText scaler does not hint. The contract's `Light` and `Strong` are the one hinting of the renderer, `None` turns it off, `Unspecified` hints as it does in the Skia backend.
- **Curves of glyphs are flattened by the renderer** to its quarter of a pixel (1.3): a glyph run is 0.26 to 1.27 % from Skia's outlines where the backend's own fill of the same outlines, flattened to a twentieth, is 0.13 to 0.43 %. The finer flattening of the scene applies to paths, not to glyphs. Open: draw outline glyphs as paths of the scene when a run is not hinted, and keep `glifo` for hinted, colour and bitmap glyphs.
- `text_aliased` shows that positions agree: without anti-aliasing 0.19 % of the pixels differ.

**What the drawing context does with the options of the contract.**

| Option | Skia backend | Vello backend |
|---|---|---|
| Text rendering mode `Alias` | aliased, whole pixel positions | the same (the aliasing threshold of the renderer; glyph origins rounded to pixels while the text is upright) |
| `Antialias` | grey-scale | grey-scale |
| `SubpixelAntialias`, and `Unspecified` with anti-aliased edges | LCD text where the surface allows it | **grey-scale**: no renderer of the Vello project has LCD text. The pixels are those of `Antialias` (asserted) |
| `Unspecified` | from the render options, else from the edge mode | the same |
| Hinting `None` / `Light` / `Strong` / `Unspecified` | none / slight with the auto-hinter / full / full | none / the vertical hinting of `glifo` for the other three. Not applied under a rotation or a non-uniform scale, nor to an emboldened run |
| Baseline pixel alignment | the baseline on a row of pixels unless `Unaligned` | the same, while the text is upright |
| Sub-pixel positioning | on unless aliased | the same (glyph origins are fractional) |
| Bold simulation | Skia's fake bold | the same width of outline, by `kurbo::expand_path` |
| Oblique simulation | a skew of -0.3 | the same |
| Variation coordinates | of the typeface Skia matched | of the typeface: normalized coordinates handed to the renderer |
| Brush | any, over the bounds of the run | the same: solid, gradient and image brushes through the paint of the scene |

**Colour fonts.** Drawn by `glifo`: COLR glyphs (versions 0 and 1, with the first palette of the font) and bitmap glyphs that are PNG images (`sbix`, `CBDT`): Apple Color Emoji is drawn in colour (the ported test asserts it). Missing: glyphs that are SVG documents (the plain outline of the font is drawn, as in `TwitterColorEmoji-SVGinOT.ttf`); bitmap glyphs that are not PNG (`glifo` reports them as unsupported and the outline is tried); the bitmaps of a bitmap font of Apple (`bdat`); a palette other than the first, and a bitmap strike by pixel size (`GlyphDrawingOptions` of the base library reaches no backend yet).

**An application.** `application_tests.rs` of the harness: the headless platform with the Fluent theme, HarfBuzz and `use_vello` (the headless tests choose a backend on the application builder: `use_harfbuzz().use_skia().use_headless(..)` with `use_headless_drawing: false`) shows a window of 320 by 180 with a text block, a button and a text box and captures the frame; the same with `use_skia`. No member of the backend fails (the test collects the panics of every thread, since the render loop catches the panic of a frame): these three controls reach no box shadow, effect or scene brush. The frames differ in 1.811 % of the pixels beyond the tolerance (mean difference 1.387; 3890 pixels unlike the background in Skia's frame, 3591 in Vello's): the text, as in the scenes. Bound: 2.77 %.

**Open after stage 5.**

| # | Open | Note |
|---|---|---|
| 1 | The hybrid and the GPU sink do not draw glyph runs | `draw_glyph_run` is a required member of the sink (section 4) |
| 2 | Text is lighter than the Skia backend's on macOS by about an eighth | inherent in filling outlines; a darkening of stems would be an own step before the fill (an embolden by a fraction of a pixel at small sizes), to be decided with the owner before the backend is a default anywhere |
| 3 | No LCD text | gap 1 of section 9 |
| 4 | Glyph curves at the renderer's quarter of a pixel | see the reading above |
| 5 | The contract has no member for variation coordinates | a typeface carries them (`VelloTypeface::new`, `from_bytes_with_variations`, and what the font manager sets for a variable font of the system); the glyph typeface and the shaper read the tables of the default instance, so the advances of an instance are those of the default one. The Skia backend is in the same position with a variable font of the system |
| 6 | PingFang, SVG glyphs, `bdat` bitmaps | above |
| 7 | A character without a script falls back to the first family that has it | the system has an answer for a character (`CTFontCreateForString`), which `fontique` does not ask; a direct call would need the CoreText bindings and `unsafe` in the crate |
| 8 | Windows and Linux | `fontique` has DirectWrite and fontconfig backends; nothing here was run on them |
| 9 | The browser | `fontique` without its `system` feature and a collection of registered fonts (stage 9) |

## 9. Gaps and risks

| # | Gap or risk | Plan |
|---|---|---|
| 1 | No sub-pixel (LCD) text in any Vello renderer | grey-scale text; `TextRenderingMode::SubpixelAntialias` maps to anti-aliased, as the Skia backend does where LCD text is disabled (built, asserted). Text quality against Skia was measured in stage 5 (section 8, "Text"): the glyphs are the outlines; Skia's on macOS are heavier |
| 2 | Perspective transforms are dropped | open; needs a decision when a 3D transform of the composition layer meets the backend (draw the layer and map it) |
| 3 | Blur and drop shadow in classic `vello` | stage 8: own passes or the CPU renderer for the layer |
| 4 | `vello_gpu` panics on mask layers, complex filter graphs, some non-isolated blend modes | the sink avoids them (masks as `DestIn` layers, blends in isolated layers) and reports capabilities |
| 5 | Interfaces change between minor releases (`vello_hybrid` became `vello_gpu` between 0.2 and 0.3) | exact versions; one file per renderer behind `IVelloSceneSink` |
| 6 | `linesweeper` is in "early beta" | a panic is caught and gives the empty geometry; the harness compares its areas with Skia's path operations |
| 7 | Every software frame is rendered in memory and converted to the framebuffer's format, and a retained target is drawn back in as an image | correct, not fast. Stage 7 removes it for windows; for the CPU mode: render in place when the framebuffer is premultiplied RGBA without padding, and render only the dirty rectangle (`RasterizerSettings::offset` and a smaller scene) |
| 8 | Curves are flattened on every draw | cache per geometry and scale |
| 9 | No mipmaps for shrunk images; RGB565; codecs beyond PNG | done in stage 6. The levels of a mipmap are built for every draw (section 10.6) |
| 10 | A second backend duplicates neutral logic of the first | the list of section 3; move after the Vello backend draws text, so that the shape of the shared code is known |
| 11 | The Metal contracts live in the Skia crate | move before stage 7 (section 3); this is the one change outside the crate the backend needs. **No platform contract of the base crate needed a change** for stages 1 to 4 |
| 12 | Scene sizes are 16 bit | `max_offscreen_render_target_pixel_size` reports 65535; larger targets fail with a message |

## 10. Stage 6: box shadows, effects, scene brushes, render options, codecs

Done on 2026-10-09 for the CPU mode, with capabilities for the modes that follow. Marks as in section 1.

### 10.1 What was verified about the renderer

| Fact | How |
|---|---|
| `vello_cpu` 0.3 has filter layers: `RenderContext::push_layer(clip, blend, opacity, mask, filter)` and `push_filter_layer`, with `FilterPrimitive::{GaussianBlur, DropShadow, DropShadowOnly, Flood, Offset}`; a graph of more than one primitive is `unimplemented!`. "Incomplete and experimental"; panics with the thread pool, which the backend does not compile in | [V] `vello_cpu-0.3.0/src/render.rs:503-590`, `vello_common-0.3.0/src/filter/mod.rs:42-104` |
| The lengths of a filter (standard deviation, offset) are scaled by the transform that is current when the layer is pushed; a blur by the mean of the two scales of the transform, so it stays round under a non-uniform scale, where the image filter of Skia becomes elliptical | [V] `vello_common-0.3.0/src/filter/gaussian_blur.rs:17-33` |
| The blur is a separable Gaussian of at most 13 taps on a pyramid of halvings, in eight bits. Against the exact blur of an edge (the error function) it is within 4 of 255 for deviations of 0.8 to 12 pixels | [V] `vello_cpu-0.3.0/src/filter/gaussian_blur.rs`; [M] a probe: 3.8, 1.5, 0.9, 0.5, 1.1, 1.6 of 255 at deviations 0.8, 1.5, 2, 3.39, 6.27, 12 |
| Beyond its content a filter layer is transparent to the blur (`EdgeMode::None`), as a layer of Skia is | [V] `filter_effects.rs:332-353` |
| **A clip that is open when a filter layer is pushed cuts the content of the layer before it is blurred and does not cut the blurred layer**: a square blurred under a clip that halves it is the blur of the half, and spreads over the clip. The canvas of Skia does the opposite | [M] a probe; the test `an_effect_under_a_clip_is_cut_by_the_clip_and_its_content_is_not` |
| Content of a filter layer that lies outside of the scene still blurs into it: the renderer draws as much of it as the filter reaches | [V] the comment at `render.rs:526-531`; [M] the tests `a_shadow_that_reaches_beyond_the_target_is_not_cut_short`, `an_effect_is_recorded_into_a_scene_as_large_as_its_clip_rectangle` |
| `fill_blurred_rounded_rect(rect, radius, std_dev, invert)` is a closed form over a distance field (after Raph Levien's `blurrr`), evaluated at the middle of a pixel, for one circular radius. **Its `std_dev` is √2 times the standard deviation of the Gaussian it stands for**: the ramp it draws across an edge for a `std_dev` of s is that of a Gaussian of s/√2. Given the deviation of the Gaussian it is 21 of 255 from the Gaussian blur of a plain rectangle, given √2 times it 5 | [V] the form: `vello_common-0.3.0/src/encode.rs:887-960`, `vello_cpu-0.3.0/src/fine/common/rounded_blurred_rect.rs`; [M] the factor: a probe against the blur filter of the same renderer |
| With the factor the closed form is within 6 of 255 of the Gaussian when the radius is at most a quarter of the shorter side, that side at least five deviations long and the deviation at least a pixel (68 of 126 shapes of a sweep); a capsule is up to 22 off, a rectangle thin against its blur 18 | [M] the test `the_closed_form_of_a_blurred_rounded_rectangle_agrees_with_the_gaussian` |
| With `invert` it paints nothing beyond 2.5 deviations around the rectangle, where an inset shadow has to be opaque: not used | [V] `render.rs:446-451` |
| The bicubic filter of `ImageQuality::High` is Mitchell's (B = C = 1/3), the one the Skia backend uses for high quality | [V] `vello_cpu-0.3.0/src/fine/common/image.rs:506-508`; [M] `interpolation_high_upscaled`: 1 of 255 |
| No mipmaps, in any of the three renderers | [V] |
| `vello_gpu` 0.3 has `push_filter_layer`, `fill_blurred_rounded_rect` and `set_aliasing_threshold` under the same names; classic `vello` 0.11 has `draw_blurred_rounded_rect` and no filter | [V] `vello_gpu-0.3.0/src/scene.rs:445, 550, 726`, `vello-0.11.0/src/scene.rs:244` |
| Skia chooses the level of a mipmap by the smaller of the two scales of the inverse matrix, less half a level, blends the level at and the level below it, and builds a level by adding two pixels (three, the middle one twice, along an odd axis) and shifting the sum, which cuts the quotient off | [V] `skia-bindings-0.153.3/skia/src/core/SkMipmap.cpp:196-219`, `SkMipmapAccessor.cpp:49-106`; [M] the other choices of scale and bias are 2 to 5 times further from Skia's pixels |
| A JPEG that `jpeg-encoder` 0.7.1 writes with Huffman tables made for the image is decoded as black by `zune-jpeg` 0.5.15 and correctly by Skia | [M] the two decodings of the file differ in 95 % of the pixels. Not examined further; the backend writes the standard tables |

### 10.2 How each member is built

| Member | How | Where |
|---|---|---|
| Box shadows, outset | The box grown by the spread as `SkRRect::outset` grows it (a round corner by as much, a square one stays square), drawn with the transform followed by the offset (the offset is not transformed, as in the original), everywhere but in the box: a clip of the target with the box as a hole, anti-aliased for a rounded box and not for a rectangle, as the Skia backend clips. No shadows for a box of more than 8192 units | `drawing_context_impl/box_shadows.rs` |
| Box shadows, inset | Everything around the box (the area of the Skia backend, after Chromium) with the box shrunk by the spread as its hole, moved by the offset, inside the box only | same |
| The blur of a shadow | (a) none: a fill, anti-aliased whatever the edge mode; (b) the closed form of the renderer, when the sink has it and the shape passes the rule of 10.1 (outset shadows of rectangles and of boxes with one moderate radius: the common case); (c) otherwise the shape is drawn through a blur layer by `vello_cpu` into a scene of its own, as large as the blur reaches inside the target, and the picture is composed under the clip. (c) serves every inset shadow, elliptical and uneven corners, capsules and thin shapes, and every shadow of a mode without the closed form | same |
| Effects | Blur and drop shadow with the deviation Skia derives from a radius (`0.288675 r + 0.5`); the alpha of a drop shadow times its opacity and the opacity of the context. A filter layer of the scene when the sink has them **and no clip is open**; otherwise everything drawn inside the effect is recorded into a scene of its own of `vello_cpu`, as large as the clip rectangle of the effect inside the target (the whole target without one), and its picture is composed when the effect is popped, under the clips. The clip rectangle cuts the content before the filter, as Skia cuts a filtered layer to the bounds it is given. An effect that changes nothing (a blur without a radius) is a layer | `drawing_context_impl/effects.rs` |
| Scene brushes | Content that is not scalable: drawn into a surface of its own size, the image of a tile brush. Scalable content: replayed into a surface of one tile at the resolution of the target (at most 2048 by 2048 pixels, the limit of the picture shader of Skia), sampled by the nearest pixel as that shader samples; source and destination rectangle, stretch, alignment, tile modes and the transforms of the content as the Skia backend computes them. A tile that is not repeated clips the shape to itself | `drawing_context_impl/scene_brushes.rs` |
| Acrylic materials | The tint over the color of the material as one color, then the noise texture (the asset of the Skia backend, its alpha scaled to 0.0225), repeated; a material that digs through replaces what is under it | `drawing_context_impl/acrylic.rs` |
| Bitmap interpolation | Nearest; bilinear; bilinear with mipmaps (medium); Mitchell when a bitmap is enlarged and bilinear with mipmaps when not (high). With mipmaps the two levels Skia would sample are built and both drawn, each with its share as its alpha, added in a layer; with a blending mode other than source-over, which composes inside the rectangle of the bitmap only, one image (the lower level blended into the upper) stands for both. Also for resizing a bitmap and decoding to a size | `helpers/mipmap_helper.rs`, `vello_extensions.rs` (`to_sampling`) |
| Blending modes | All 27 of the contract: 12 Porter-Duff operators, 15 mix functions composed source-over | `vello_extensions.rs` (`to_blend_mode`) |
| Edge mode | Aliased: the threshold of the renderer at half coverage. A sink without aliased edges is asked for anti-aliased ones | `drawing_context_impl.rs` (`edge_anti_alias`) |
| Decoding | By the first bytes of the data: PNG (`png`), JPEG (`zune-jpeg`), GIF (`gif`: the first frame on the canvas of the image), BMP (`zune-bmp`), ICO (own: the first of the largest images, a PNG or a bitmap with its mask, or with its alpha at 32 bits), WBMP (own; it has no signature and is tried last). A decoder says whether the image has no alpha, and the bitmap is then opaque, as the codec of Skia reports it. 96 DPI always: the Skia backend reads no resolution either | `helpers/image_decoding_helper.rs` |
| Decoding to a width or height | The image is decoded, a JPEG is reduced to the eighths of its size at which the codec of Skia would decode it (libjpeg's scaled decoding; here the mean of the pixels), then scaled with the interpolation mode | `immutable_bitmap.rs` |
| Encoding | PNG with the compression levels (`png`); JPEG with a quality of 0 to 100 (`jpeg-encoder`): the premultiplied colors (what is translucent over black), the color difference channels at half the resolution by the mean of four pixels, standard Huffman tables | `helpers/image_saving_helper.rs` |
| Pixel formats | RGB565, RGB32, RGBA8888 and BGRA8888, premultiplied, not premultiplied and opaque, read into and written from the premultiplied RGBA the renderers draw. A bitmap from pixels takes a negative stride for rows from the bottom up, keeps the pixels it was made from and is read in their format; the render interface reports RGB565, RGBA8888 and BGRA8888 as supported, as the Skia backend does | `helpers/pixel_format_helper.rs`, `immutable_bitmap.rs`, `writeable_bitmap_impl.rs` |

### 10.3 Capabilities per mode

What the drawing context asks of a sink, and what it does when the sink lacks it. The CPU column is built and tested; the other two are what their renderers have by 10.1 and what the context does until their sinks say so (a sink that does not implement the filter members has none: the tests draw with such a sink).

| | CPU (`vello_cpu`) | Hybrid (`vello_gpu`) | GPU (`vello`) | Without it |
|---|---|---|---|---|
| Filter layers (`filter_layers`) | yes; used when no clip is open | the renderer has them ("complex filter graphs" panic; one primitive is what is asked) | no | the effect is recorded into a scene of `vello_cpu` and composed as an image: correct in every mode, on the processor |
| Blurred rounded rectangle (`blurred_rounded_rects`) | yes, with the factor √2 | the renderer has it; whether its `std_dev` is the same is to be measured by its sink | `draw_blurred_rounded_rect`, the same to be measured | the shadow is blurred as an image by `vello_cpu` |
| Shadows that are not in closed form, inset shadows | an image by `vello_cpu` | the same image | the same image | |
| Aliased edges (`aliased_edges`) | yes | the renderer has the threshold | no | anti-aliased edges |
| Blending modes of a bitmap | all 27 | "certain blend modes for non-isolated blending" panic by its README: its sink has to isolate them | all | |
| Mipmaps, scene brushes, acrylic, pixel formats, codecs | in the drawing context and the bitmaps: the same in every mode | | | |

### 10.4 Measures against the Skia backend

**The measure for blurred scenes.** A blur is a smooth ramp: two renderers that blur a little differently differ a little in every pixel of the ramp and by much in none. The share of pixels beyond a tolerance, the measure of the other scenes, is blind to a blur of the wrong width: a rectangle blurred by a radius of 13 instead of 10 has no pixel beyond the tolerance of 32 (the largest difference is 28). The scenes of stage 6 keep that measure with the same tolerance (it finds a shadow that is missing, moved or clipped wrongly: the same blur moved by two pixels has 3 % of its pixels beyond it) and have a second bound, on the mean difference of all channels of all pixels. Between the two backends the blurred rectangle has a mean of 0.25, which gives it a bound of 0.43; the blur that is 30 % too wide has 1.06 and the one that is moved 1.17 (the test `the_mean_difference_finds_a_blur_of_another_width`). The tolerance was not raised. Each bound is the measured value, half as much again and 0.05.

`cargo test -p ferroui-render-backends-comparison -- --nocapture` prints the tables.

| Scene | Pixels beyond the tolerance | Largest difference | Mean difference | Bound of the share | Bound of the mean |
|---|---|---|---|---|---|
| `shadows_outset_rectangle` | 0.000 % | 6 | 0.321 | 0.05 % | 0.54 |
| `shadows_outset_rounded` | 0.092 % | 78 | 0.347 | 0.19 % | 0.58 |
| `shadows_outset_elliptical` | 0.005 % | 37 | 0.300 | 0.06 % | 0.50 |
| `shadows_outset_capsule` | 0.048 % | 72 | 0.285 | 0.13 % | 0.48 |
| `shadows_inset_rectangle` | 0.000 % | 10 | 0.238 | 0.05 % | 0.41 |
| `shadows_inset_rounded` | 0.113 % | 49 | 0.311 | 0.22 % | 0.52 |
| `shadows_inset_elliptical` | 0.007 % | 36 | 0.257 | 0.07 % | 0.44 |
| `shadows_combined` | 0.000 % | 32 | 0.318 | 0.05 % | 0.53 |
| `effect_blur` | 0.000 % | 6 | 0.329 | 0.05 % | 0.55 |
| `effect_blur_transformed` | 0.000 % | 5 | 0.175 | 0.05 % | 0.32 |
| `effect_blur_bounded` | 0.000 % | 12 | 0.423 | 0.05 % | 0.69 |
| `effect_drop_shadow` | 0.025 % | 43 | 0.221 | 0.09 % | 0.39 |
| `effect_drop_shadow_transformed` | 0.018 % | 42 | 0.191 | 0.08 % | 0.34 |
| `scene_brush_single` | 0.028 % | 60 | 0.021 | 0.10 % | 0.09 |
| `scene_brush_tile` | 0.660 % | 60 | 0.678 | 1.04 % | 1.07 |
| `scene_brush_flip_x` | 0.680 % | 60 | 0.682 | 1.07 % | 1.08 |
| `scene_brush_flip_y` | 0.705 % | 60 | 0.679 | 1.11 % | 1.07 |
| `scene_brush_flip_xy` | 0.720 % | 60 | 0.681 | 1.13 % | 1.08 |
| `scene_brush_surface_single` | 0.000 % | 27 | 0.009 | 0.05 % | 0.07 |
| `scene_brush_surface_tile` | 0.000 % | 32 | 0.267 | 0.05 % | 0.46 |
| `scene_brush_surface_flip_xy` | 0.000 % | 32 | 0.267 | 0.05 % | 0.46 |
| `scene_brush_stretched` | 0.180 % | 67 | 0.236 | 0.32 % | 0.41 |
| `scene_brush_transformed` | 0.320 % | 68 | 0.340 | 0.53 % | 0.56 |
| `acrylic` | 0.025 % | 56 | 0.216 | 0.09 % | 0.38 |
| `interpolation_none_upscaled` | 0.000 % | 0 | 0.000 | 0.05 % | 0.05 |
| `interpolation_low_upscaled` | 0.000 % | 2 | 0.190 | 0.05 % | 0.34 |
| `interpolation_medium_upscaled` | 0.000 % | 2 | 0.193 | 0.05 % | 0.34 |
| `interpolation_high_upscaled` | 0.000 % | 1 | 0.000 | 0.05 % | 0.05 |
| `interpolation_none_downscaled` | 0.000 % | 23 | 0.046 | 0.05 % | 0.12 |
| `interpolation_low_downscaled` | 0.000 % | 23 | 0.182 | 0.05 % | 0.33 |
| `interpolation_medium_downscaled` | 0.000 % | 22 | 0.513 | 0.05 % | 0.82 |
| `interpolation_high_downscaled` | 0.000 % | 22 | 0.513 | 0.05 % | 0.82 |
| `blend_source_over` | 0.095 % | 56 | 0.212 | 0.20 % | 0.37 |
| `blend_source` | 0.085 % | 59 | 0.252 | 0.18 % | 0.43 |
| `blend_destination` | 0.000 % | 21 | 0.044 | 0.05 % | 0.12 |
| `blend_destination_over` | 0.000 % | 30 | 0.141 | 0.05 % | 0.27 |
| `blend_source_in` | 0.055 % | 47 | 0.183 | 0.14 % | 0.33 |
| `blend_destination_in` | 0.055 % | 47 | 0.160 | 0.14 % | 0.29 |
| `blend_source_out` | 0.000 % | 30 | 0.100 | 0.05 % | 0.20 |
| `blend_destination_out` | 0.060 % | 47 | 0.157 | 0.14 % | 0.29 |
| `blend_source_atop` | 0.040 % | 43 | 0.133 | 0.11 % | 0.25 |
| `blend_destination_atop` | 0.085 % | 60 | 0.243 | 0.18 % | 0.42 |
| `blend_xor` | 0.045 % | 47 | 0.213 | 0.12 % | 0.37 |
| `blend_plus` | 0.100 % | 59 | 0.231 | 0.20 % | 0.40 |
| `blend_screen` | 0.095 % | 56 | 0.300 | 0.20 % | 0.50 |
| `blend_overlay` | 0.000 % | 30 | 0.210 | 0.05 % | 0.37 |
| `blend_darken` | 0.000 % | 30 | 0.158 | 0.05 % | 0.29 |
| `blend_lighten` | 0.098 % | 57 | 0.220 | 0.20 % | 0.38 |
| `blend_color_dodge` | 0.125 % | 58 | 0.210 | 0.24 % | 0.37 |
| `blend_color_burn` | 0.000 % | 30 | 0.167 | 0.05 % | 0.31 |
| `blend_hard_light` | 0.077 % | 54 | 0.310 | 0.17 % | 0.52 |
| `blend_soft_light` | 0.000 % | 30 | 0.164 | 0.05 % | 0.30 |
| `blend_difference` | 0.090 % | 54 | 0.242 | 0.19 % | 0.42 |
| `blend_exclusion` | 0.087 % | 53 | 0.385 | 0.19 % | 0.63 |
| `blend_multiply` | 0.000 % | 30 | 0.248 | 0.05 % | 0.43 |
| `blend_hue` | 0.000 % | 31 | 0.201 | 0.05 % | 0.36 |
| `blend_saturation` | 0.000 % | 30 | 0.162 | 0.05 % | 0.30 |
| `blend_color` | 0.000 % | 31 | 0.196 | 0.05 % | 0.35 |
| `blend_luminosity` | 0.030 % | 40 | 0.239 | 0.10 % | 0.41 |
| `edge_mode_aliased` | 0.068 % | 255 | 0.134 | 0.16 % | 0.26 |
| `pixel_formats` | 0.013 % | 36 | 0.064 | 0.07 % | 0.15 |
| **all 61 scenes** | **0.081 %** | | **0.248** | | |

Reading. A blur agrees to 6 of 255 where nothing else is in the scene (`shadows_outset_rectangle`, `effect_blur`, the transformed blur under a rounded clip) and to 12 where the clip rectangle of the effect cuts its content (which confirms that Skia cuts before it filters: cutting after would be off by a hundred). The larger differences of the shadow scenes are on the sharp edges of the shadows without a blur and of the rounded clips, as in the scenes of section 8 (`rounded_rectangle`: 62). Scalable scene brushes differ like the shapes they draw, once a tile (0.03 % a tile, 22 tiles). The nearest pixel, bilinear and bicubic sampling agree to the last digits of a color; with mipmaps the mean is 0.5 with the Vello backend lighter by 0.8 of 255 everywhere (not traced: a rounding in the two alphas of the levels or in Skia's blend). Every blending mode agrees; what is beyond the tolerance is on the edge of the ellipse of the bitmap.

**Codecs** (`codec_tests.rs` of the harness; the worst row of each group):

| Data | Pixels beyond the tolerance | Largest difference | Mean difference |
|---|---|---|---|
| 23 photographs of the sample application (JPEG), decoded by both; 8 of them identical | 0.000 % | 10 | 0.259 |
| PNG, opaque and translucent, encoded by either backend and decoded by the other, against the picture | 0.000 % | 0 | 0.000 |
| JPEG of quality 100, 75, 30 encoded by either backend: the two decodings of one file | 0.000 % | 4 | 0.386 |
| JPEG encoded by Vello against the picture, beside Skia's: quality 100 | 6.934 % (Skia 6.934 %) | 74 (74) | 1.259 (1.259) |
| the same, quality 75 | 7.064 % (7.064 %) | 80 (80) | 2.123 (2.114) |
| the same, quality 30 | 7.975 % (7.943 %) | 102 (98) | 3.657 (3.642) |
| GIF (the first of two frames, a transparent color), BMP (24 bits from the bottom and from the top, 8 bits with a palette), ICO (24 bits with a mask, 32 bits with alpha, an embedded PNG, the largest of three), WBMP (drawn) | 0.000 % | 0 | 0.000 |
| a JPEG of 600 by 400 decoded to a width of 181 and of 901, bilinear, with mipmaps, bicubic | 0.000 % | 5 | 0.403 |
| the same by the nearest pixel, reduced | 0.000 % | 7 | 0.290 |
| the same by the nearest pixel, enlarged (600 rows from 400: the middle of every third row lies on the edge between two rows of the image, see 10.5) | 0.563 % | 144 | 1.107 |
| a bitmap of 64 by 48 resized to 23 by 17 and to 150 by 100, every mode | 0.000 % | 5 | 0.947 |

The two JPEG encoders lose the same (the picture has a block of a saturated color, whose edge the halved color channels blur: hence 7 %); the files of the Vello backend are larger (1814 against 1252 bytes at quality 100, 967 against 546 at 75) because Skia writes Huffman tables made for the image (10.1). Whether both backends report an image as opaque is asserted for every file of the GIF, BMP and ICO group.

**Tests.** The crate has 148 tests (83 before the stage; two of those tested that a member fails with stage 6 and are gone): 67 new ones in `effect_tests.rs`, `helpers/mipmap_helper.rs` and `helpers/pixel_format_helper.rs`. Ported from the Skia backend with their expectations: the three box shadow tests, the five effect tests, the two scene brush tests, the three acrylic tests (`tests.rs`), the three of `BitmapSaveTests`, and the constructor of a bitmap from pixels (`immutable_bitmap.rs`). The harness has 14 tests (5 before).

### 10.5 Where the backend behaves differently from the Skia backend

| Difference | Why |
|---|---|
| A blur under a transform that scales x and y differently is round, of the mean of the two scales; Skia's is elliptical | the filter of `vello_cpu` has one deviation (its TODO); not measured, no scene has such a blur |
| The clip rectangle of an effect under a rotation cuts the content to the rotated rectangle; Skia to its bounds in pixels | a clip of the scene; the compositor gives bounds that hold the content either way |
| The nearest pixel of a bitmap, where the middle of a pixel of the target falls exactly on the edge between two pixels of the bitmap, is the one on one side here and on the other in Skia: at 8.5 times the size every second edge, 4.4 % of the pixels of that scene | each renderer's rounding; at whole factors and positions no middle falls on an edge. The scenes avoid the case |
| A pixel that is declared opaque but whose fourth byte is not 255 is read as opaque; the raster path of Skia copies the byte into the target | such data is outside what Skia defines |
| A bitmap from pixels in a format other than premultiplied RGBA holds its pixels twice: as given, and as the image it is drawn from | the renderers draw one format; a bitmap of Skia draws any |
| A decoded bitmap is RGBA8888; Skia's is the 32 bit format of the platform (BGRA8888 here). A WBMP is decoded to opaque RGBA; Skia's is a bitmap of gray that it draws and cannot lock | |
| JPEG files are about a third larger at the same quality | standard Huffman tables (10.1) |
| Scalable scene brush content is replayed for every paint with the brush, into a surface; Skia records a picture and its shader rasterizes the tile | no recorded scene; the pixels are the same |
| An anti-aliased edge of an acrylic rectangle weighs the tint and the noise by the coverage one after the other | two fills for one composed paint |
| With mipmaps and a blending mode other than source-over, one image stands for the two levels: where the lower level has detail of a pixel, sampling is a little softer (a mean of 1.05 of 255 against Skia where the two levels give 0.51) | the two levels are added in a layer, which such a mode would compose beyond the rectangle of the bitmap |

### 10.6 Open after the stage

| # | What | Plan |
|---|---|---|
| 1 | An effect under a clip, which is most effects of a window (the compositor clips to what it redraws), is drawn into a scene of its own and composed as an image, a layer of pixels for every effect of a frame | when `vello_cpu` cuts a filter layer by the clips around it as Skia does, or through a clip layer of the scene that is measured to do so, the filter layer can be used there too; the hybrid and GPU sinks decide for their renderers |
| 2 | The levels of a mipmap are built for every draw of a reduced bitmap in a mode with mipmaps, and the blurred picture of a shadow that is not in closed form for every draw | keep the levels with the bitmap (by its version) and the picture of a shadow by its shape, blur and scale |
| 3 | `render_async` and `wrap_skia_surface` of the Skia backend's helper have no counterpart | with the GPU modes, when there is a surface another API hands over |
| 4 | The `HitTesting` suite of the Skia backend is not ported | it runs controls through the compositor with the font services of the text tests: after stage 5 |
| 5 | WebP is not decoded | the Skia backend of this workspace does not decode it either; `image-webp` when it does |
| 6 | The JPEG encoder writes standard Huffman tables; a file with optimized tables by `jpeg-encoder` is decoded as black by `zune-jpeg` | find which crate is at fault (Skia reads the file) before a newer version of either is taken; a JPEG from elsewhere that decodes as black would be the same defect |
| 7 | The filter capabilities of the hybrid and GPU sinks | their stages: implement the three members at the end of `IVelloSceneSink` where the renderer has them and measure the deviation of its blurred rounded rectangle; without them everything of this stage is drawn through images of `vello_cpu` |
| 8 | Snapping to pixels in the aliased edge mode in a mode without aliased edges | stage 8 |
