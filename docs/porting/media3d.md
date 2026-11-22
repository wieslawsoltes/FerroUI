# WPF-style 3D: Viewport3D and Media3D (#608)

The plan for 3D scenes in FerroUI: WPF's `Viewport3D` and the `System.Windows.Media.Media3D` model
(values, transforms, cameras, lights, materials, meshes, models and visuals), with WPF's names and
idiomatic Rust types, rendered by GPU shaders in every backend with GPU fallbacks, the renderer
ported from ProGPU, and video and other shared GPU textures as 3D materials through the frame
textures the media framework already shares. This page is the design and the order of the work;
each step below is one pull request from main.

The rules that bind it are those of [PORTING-GUIDE.md](PORTING-GUIDE.md), ground rule 6, and
[gpu-rendering.md](gpu-rendering.md): every rendering feature is drawn on the GPU by Skia (Ganesh,
Graphite) and Vello (GPU, hybrid, WebGL2), validated by tests against a CPU reference with a
stated tolerance; the CPU path is the reference and the fallback of surfaces without a GPU; a
program is compiled once per device and backend, and what animates changes uniforms, never
textures. A backend that cannot draw a case records the gap in gpu-rendering.md and does not fall
back silently. The API keeps the .NET names and uses Rust types (PORTING-GUIDE, "API shape: .NET
names, Rust types", #423).

The feature is not part of the reference: WPF is the source of the API and its behaviour, ProGPU
the source of the GPU renderer. [docs/media/PORTABLE-MEDIA-FRAMEWORK-SPECIFICATION.md](../media/PORTABLE-MEDIA-FRAMEWORK-SPECIFICATION.md),
section 8, scopes 3D scenes and video on 3D surfaces to this design.

## 1. Sources

### 1.1 WPF

WPF (<https://github.com/dotnet/wpf>, MIT License, .NET Foundation) defines the API: the types
are in `src/Microsoft.DotNet.Wpf/src/PresentationCore/System/Windows/Media3D/` (the values, with
their generated parts and type converters under `Generated/` and `Converters/Generated/`) and
`src/Microsoft.DotNet.Wpf/src/PresentationFramework/System/Windows/Controls/Viewport3D.cs`. The
behaviour this port follows is WPF's documented behaviour ("3-D Graphics Overview", "Maximize WPF
3D Performance" and the reference pages of the namespace on learn.microsoft.com); points the
documentation leaves open are listed in section 6.2 and checked against WPF before tests freeze
them.

| Area | WPF types |
|---|---|
| Values | `Point3D`, `Vector3D`, `Size3D`, `Rect3D`, `Point4D`, `Matrix3D`, `Quaternion`; the collections `Point3DCollection`, `Vector3DCollection` (and `PointCollection`, `Int32Collection` of `System.Windows.Media`) |
| Transforms | `Transform3D` (`Value`, `Inverse`, `TryTransform`, `TransformBounds`), `TranslateTransform3D`, `ScaleTransform3D`, `RotateTransform3D`, `MatrixTransform3D`, `Transform3DGroup`; `Rotation3D`, `AxisAngleRotation3D`, `QuaternionRotation3D` |
| Cameras | `Camera` (`Transform`), `ProjectionCamera`, `PerspectiveCamera`, `OrthographicCamera`, `MatrixCamera` |
| Models and lights | `Model3D` (`Transform`, `Bounds`), `GeometryModel3D`, `Model3DGroup`; `Light`, `AmbientLight`, `DirectionalLight`, `PointLightBase`, `PointLight`, `SpotLight` |
| Geometry | `Geometry3D` (`Bounds`), `MeshGeometry3D` |
| Materials | `Material`, `DiffuseMaterial`, `SpecularMaterial`, `EmissiveMaterial`, `MaterialGroup` |
| Visuals and controls | `Visual3D`, `ModelVisual3D`, `Viewport3DVisual`, `Viewport2DVisual3D`, `UIElement3D`, `ModelUIElement3D`, `ContainerUIElement3D`; `System.Windows.Controls.Viewport3D` |
| Hit testing | `RayHitTestParameters`, `RayHitTestResult`, `RayMeshGeometry3DHitTestResult`, the `VisualTreeHelper.HitTest` overloads |

### 1.2 ProGPU

ProGPU (`../ProGPU`, read on `origin/main` at `23f686151`) has two 3D
implementations. It is a good source for the GPU half (lighting, material and video sampling,
retained uploads, pipelines created on demand) and a weak source for the API half: its managed
`Viewport3D` is a CAD-oriented subset of WPF.

| Part | Files (paths in ProGPU's `origin/main`) | What FerroUI takes |
|---|---|---|
| Managed control and model | `src/ProGPU.WinUI/Controls/Viewport3D.cs` (namespace `Microsoft.UI.Xaml.Media.Media3D`: `MeshGeometry3D` with `Version`, `DiffuseMaterial`, `GeometryModel3D`, `ModelVisual3D`, the cameras, `ProGpuTextureMaterial`, `ProGpuMediaTextureMaterial`) | The version-driven mesh cache and the generation split (scene, records); not the API, which lacks `Transform3D`, `Model3DGroup`, light objects, `SpecularMaterial`, `EmissiveMaterial`, `MaterialGroup`, `MatrixCamera`, hit testing and `Viewport2DVisual3D` |
| Managed renderer | `src/ProGPU.Scene/Extensions/Mesh3DExtensionPipeline.cs` | The compilation of the tree into mesh and light records, the offscreen color, depth and MSAA targets, the uploads only on a new generation, the frame metrics |
| Shaders | `src/ProGPU.Scene/Shaders/Mesh3DSolid.wgsl` (`ComputeWpfLighting`, `SampleMaterial`, `TransformMaterialCoordinate`, the gradient materials), `Native3D.wgsl`; the CAD shaders `Mesh3DWireframe.wgsl`, `Mesh3DEdges.wgsl`, `Line3D.wgsl` | The WPF light loop and the material and video sampling, rewritten to the uniform-buffer subset of section 3.4; not the CAD shaders |
| Native engine | `src/ProGPU.Native/src/Backend/progpu_native_3d_execution.cpp`, `src/ProGPU.Native/src/Scene/Builder/progpu_native_scene_builder_3d.cpp`, `src/ProGPU.Native/include/progpu_native_mil.hpp` (`set_viewport3d_scene`), `src/ProGPU.Backend.Native/NativeMilChannel.cs`, `NativeMilBatchBuilder.cs`, `NativeMilTypes.cs`, `NativeMilViewport3DSnapshot.cs` | The validation of the whole inventory before buffers are replaced, the allocation-free snapshot match; the record of the full WPF resource set in the MIL protocol as a checklist |
| Video into 3D | `src/ProGPU.Media.Scene/MediaMesh3DPresentation.cs`, `src/ProGPU.Backend/IProGpuTextureSource.cs` | A frame lease bound as a material texture with its crop, rotation, mirroring and YUV conversion, acquired only when the pass draws |
| Design records | `docs/retained-3d-pipeline-demand.md`, `docs/native-mil-viewport3d-face-validation-2026-09-10.md` | Pipelines created on first demand from the records the encoder uses; depth initialization tracked per attachment over the whole slot domain |
| Tests and sample | `src/ProGPU.Tests/Mesh3DPipelineDemandTests.cs`, `Mesh3DRetainedReplayTests.cs` (`CameraReplayRetainsSceneUploadsAndRehydratesOnNewContext`), `Mesh3DTextureMaterialTests.cs`, `NativeMilViewport3DSnapshotTests.cs`; `src/ProGPU.Samples/Pages/Mesh3DViewerPage.cs`, `src/ProGPU.WinUI/Controls/ObjReader.cs` | The shape of the steady-state tests: a camera move uploads only uniforms, exact pipeline counts |

ProGPU renders only through WebGPU (wgpu-native, Dawn, the browser) and has no fallback for 3D: a
missing GPU means no ProGPU, a missing material texture a 1x1 white texture. FerroUI's fallback
chain (section 3.9) is its own.

## 2. How WPF 3D works

- **The host.** `Viewport3D` is a `FrameworkElement` that hosts a `Viewport3DVisual`: a `Camera`, a
  collection of `Visual3D` children and the viewport rectangle (the arranged bounds). It clips to
  its bounds and draws nothing where no model is: a 3D scene composes over the 2D content behind
  it like any element, under the element's transform, clip, opacity and effects.
- **The visual tree of 3D.** `ModelVisual3D` carries a `Model3D` as its `Content`, its own
  `Children` and a `Transform`. `Viewport2DVisual3D` puts a 2D `Visual` on a mesh as a material
  that takes input; `UIElement3D`, `ModelUIElement3D` and `ContainerUIElement3D` are 3D elements
  with input, focus and events.
- **Models.** `GeometryModel3D` is a `Geometry3D` (in practice `MeshGeometry3D`) with a `Material`
  for its front faces and a `BackMaterial` for its back faces; `Model3DGroup` groups models. Lights
  are models: they are placed by the transforms of the tree, and every light of a viewport lights
  every model of it. `AmbientLight` lights uniformly, `DirectionalLight` along a `Direction`,
  `PointLight` from a `Position` with a `Range` and constant, linear and quadratic attenuation,
  `SpotLight` also with a `Direction` and an inner and outer cone angle.
- **Meshes.** `Positions`, `TriangleIndices` (when empty, the positions are taken three at a time),
  `Normals` (generated when absent) and `TextureCoordinates`. A triangle whose vertices are
  counterclockwise to the camera is a front face.
- **Materials.** `DiffuseMaterial` (`Brush`, `Color`, `AmbientColor`) reflects the lights;
  `SpecularMaterial` (`Brush`, `Color`, `SpecularPower`) adds highlights; `EmissiveMaterial`
  (`Brush`, `Color`) adds its brush without light; `MaterialGroup` layers materials in order. Any
  brush is a material; the texture coordinates map it.
- **Cameras.** A `ProjectionCamera` has `Position`, `LookDirection`, `UpDirection`,
  `NearPlaneDistance` and `FarPlaneDistance`. `PerspectiveCamera` adds `FieldOfView` (degrees),
  `OrthographicCamera` `Width`; `MatrixCamera` takes a view and a projection `Matrix3D`. Every
  camera has a `Transform`.
- **Transforms.** A `Transform3D` is a `Matrix3D` (`Value`) in WPF's row-vector convention;
  `RotateTransform3D` takes a `Rotation3D` (axis and angle, or a quaternion) and a center;
  `Transform3DGroup` composes its children in order.
- **Values.** `Point3D`, `Vector3D`, `Size3D`, `Rect3D`, `Point4D`, `Matrix3D` and `Quaternion` are
  double-precision structures; the members that cannot accept an input throw (a negative size, a
  matrix with no inverse, the zero axis of a rotation). Markup writes them in their text forms
  (`Position="0,0,5"`, `Positions="0,0,0 1,0,0 0,1,0"`, `TriangleIndices="0 1 2"`,
  `Transform="Identity"`).
- **Rendering.** The scene is retained: the composition thread draws it again only when it
  changes, with a depth buffer and antialiasing, in tree order without sorting transparent models.
  WPF documents its lighting as per vertex (which is why it advises tessellating meshes under point
  and spot lights).
- **Hit testing.** `VisualTreeHelper.HitTest` takes a point of the viewport or a ray
  (`RayHitTestParameters`) and reports `RayMeshGeometry3DHitTestResult`s, nearest first: the point,
  the distance, the three vertex indices and weights of the triangle, the mesh, the model and the
  visual. It runs on the CPU.

## 3. Design for FerroUI

### 3.1 What exists

FerroUI had no 3D scene support before this work: no `Viewport3D`, no `Media3D`, no depth-tested
triangle path in any backend and no CPU triangle rasterizer. It has most of the plumbing a 3D pass
needs:

- f32 numerics in `src/FerroUI.Base/numerics/` (row vectors, right-handed, as .NET's
  `System.Numerics`): `Matrix4x4::create_perspective_field_of_view`, `create_look_at`,
  `create_from_quaternion`, `invert`; a 2D-plus-perspective `Matrix` and `Rotate3DTransform`;
- the base library's f64 `Vector3D` (`src/FerroUI.Base/vector3d.rs`), the composition vector of the
  reference, which is not WPF-shaped (section 3.12);
- Vello's own wgpu passes with pooled textures painted back into the scene
  (`src/Vello/FerroUI.Vello/gpu/effect_passes.rs`, `drawing_context_impl/effects.rs`) and a pooled
  depth texture pattern (`scene/vello_hybrid_scene_sink.rs`); on macOS Vello's wgpu device is the
  platform's Metal device, the one Skia Graphite uses, with queues ordered by a shared `MTLEvent`
  (`gpu/metal.rs`);
- Skia's import of native textures (`gl_skia_external_objects_feature.rs`,
  `skia_metal_external_objects_feature.rs`, `vulkan_skia_external_objects_feature.rs`) and the lease
  of the native graphics API (`i_skia_api_lease_feature.rs`);
- build-time WGSL to GLSL ES 3.00 translation by naga (`ferroui_build::shader_effects`);
- the CPU-against-GPU comparison harness `tests/FerroUI.RenderBackends.Comparison`;
- video frames as GPU textures on every shipped route (section 3.11).

Vello drops the perspective of a 2D transform today (gap C7 of [vello-backend.md](vello-backend.md));
the 3D executor of this design closes it later (step D13).

### 3.2 Crates and size

- **`ferroui-media3d`** (`src/FerroUI.Media3D`): the public model. The values (step D1, section
  3.12), the scene objects (`ferro_class!` types with styled properties, as `Rotate3DTransform` and
  the brushes), the snapshot compiler that flattens a tree into an immutable `Scene3DSnapshot`, the
  hit testing of section 3.8, and optional mesh builders (box, sphere, cylinder) for samples. The
  snapshot types themselves (`Scene3DSnapshot`, `MeshData`, the layers, lights and camera) are in
  the base library (`ferroui_base::rendering::scene3d`, section 3.13), beside the render-data
  operation that carries them, so the backends read them without depending on this crate.
- **`ferroui-render3d`** (`src/FerroUI.Rendering3D`): `scene3d.wgsl`, the CPU reference rasterizer
  (`cpu`), the `IScene3DExecutor` trait, the uniform layouts (`#[repr(C)]` with std140 asserts), the
  pipeline key and the mesh, material and texture cache logic every executor shares. Its build
  script translates the WGSL with naga. The executors are features: `wgpu` (used by
  `ferroui-vello`), `gl`, `metal` and `vulkan`, whose `unsafe` parts live in or next to the FFI
  crates `ferroui-opengl`, `ferroui-metal` and `ferroui-vulkan` (ground rule 4).
- **Backend glue:** `ferroui-skia` (`gpu/scene3d.rs`) and `ferroui-vello` (`gpu/scene3d_passes.rs`)
  implement one optional capability of the drawing context,
  `IDrawingContextImplWithScene3D::draw_scene_3d(&Scene3DFrame) -> bool`, in
  `src/FerroUI.Base/platform/`; `false` means "not drawn on the GPU", and the caller draws the CPU
  reference and counts it, the pattern of `draw_lighting` and `push_pixel_operation`. As built in
  D3 the backend draws the CPU reference itself, behind its feature `media3d`, and counts it on a
  GPU surface (section 3.14); `false` is left for an image it could not make.
- **Size:** the facade enables the crates through an opt-in feature `media3d`, off by default. An
  application that never names `Viewport3D` links none of this, and Skia builds do not link wgpu
  (section 3.4). Sizes are measured through the inputs of `perf.yml`, not locally.

### 3.3 Public API

Module `ferroui::media3d` of the facade (the crate `ferroui_media3d`, feature `media3d`), namespace
`FerroUI.Media.Media3D` (and `FerroUI.Controls` for `Viewport3D`);
WPF's names, Rust types underneath: `f64` values, `Vec`, `Rc` for resources shared on the UI thread
(a mesh used by many models; the snapshot holds `Arc` copies), `Result` where WPF throws, and
generation counters rather than events on the hot path.

| Area | Types and members |
|---|---|
| Values (D1) | `Point3D`, `Vector3D`, `Size3D`, `Rect3D`, `Point4D`, `Matrix3D`, `Quaternion`; section 3.12 |
| Transforms | `Transform3D` (`value() -> Matrix3D`, `inverse`, `try_transform`, `transform_bounds`), `TranslateTransform3D`, `ScaleTransform3D` (with centers), `RotateTransform3D` (`Rotation`, `CenterX/Y/Z`), `MatrixTransform3D`, `Transform3DGroup`; `Rotation3D`, `AxisAngleRotation3D` (`Axis`, `Angle`), `QuaternionRotation3D` |
| Cameras | `Camera` (`Transform`); `ProjectionCamera` (`Position`, `LookDirection`, `UpDirection`, `NearPlaneDistance`, `FarPlaneDistance`); `PerspectiveCamera` (`FieldOfView`, horizontal degrees), `OrthographicCamera` (`Width`), `MatrixCamera` (`ViewMatrix`, `ProjectionMatrix`) |
| Models | `Model3D` (`Transform`, `Bounds`), `GeometryModel3D` (`Geometry`, `Material`, `BackMaterial`), `Model3DGroup` (`Children`) |
| Lights | `Light` (`Color`), `AmbientLight`, `DirectionalLight` (`Direction`), `PointLightBase` (`Position`, `Range`, `ConstantAttenuation`, `LinearAttenuation`, `QuadraticAttenuation`), `PointLight`, `SpotLight` (`Direction`, `InnerConeAngle`, `OuterConeAngle`). WPF has no fixed light count; the GPU takes 16 per viewport, and section 6.1 states the behaviour beyond |
| Geometry | `Geometry3D` (`Bounds`), `MeshGeometry3D` (`Positions`, `Normals`, `TextureCoordinates`, `TriangleIndices`); `Point3DCollection`, `Vector3DCollection`, `PointCollection`, `Int32Collection` as `Vec`-backed types with change notification and WPF's text forms |
| Materials | `DiffuseMaterial` (`Brush`, `Color`, `AmbientColor`), `SpecularMaterial` (`Brush`, `Color`, `SpecularPower`), `EmissiveMaterial` (`Brush`, `Color`), `MaterialGroup` (`Children`) |
| Visuals | `Visual3D` (`Transform`), `ModelVisual3D` (`Content`, `Children`), `Viewport3DVisual` (`Camera`, `Viewport`, `Children`), `Viewport2DVisual3D` (`Geometry`, `Material`, `Visual`, `CacheInvalidationThresholdMinimum`/`Maximum`, attached `IsVisualHostMaterial`); later `UIElement3D`, `ModelUIElement3D`, `ContainerUIElement3D` |
| Control | `Viewport3D` (`Camera`, `Children` as a `Visual3DCollection`; `controls/viewport_3d.rs` of this crate), clipping to its bounds, transparent where no model is |
| Hit testing | `RayHitTestParameters`, `RayHitTestResult`, `RayMeshGeometry3DHitTestResult` (`PointHit`, `DistanceToRayOrigin`, `VertexIndex1..3`, `VertexWeight1..3`, `MeshHit`, `ModelHit`, `VisualHit`), the `VisualTreeHelper::hit_test` overloads for `Visual3D` and viewport points, callbacks returning `HitTestResultBehavior` |
| FerroUI additions | `VideoBrush` (decision D). ProGPU's CAD features are not part of the port (decision E) |

The values narrow to the f32 `numerics::Matrix4x4` and vectors when the snapshot is built; both
use the row-vector convention, so the narrowing is element by element.

### 3.4 Shaders: one WGSL source, translated

One source, `ferroui-render3d/shaders/scene3d.wgsl`, restricted to what every target runs:
uniform buffers only (no storage buffers, no vertex pulling), interleaved vertex attributes with
`u32` indices, one combined texture and sampler per material slot, `@builtin(front_facing)`, fixed
loop bounds (16 lights), f32 throughout, uniform blocks under 16 KiB in a std140-compatible layout,
and derivatives only in the fragment stage. These are the limits of WebGL2 and GLES 3.0 and of
`wgpu::DownlevelCapabilities` on GL.

| Executor | Translation |
|---|---|
| wgpu (Vello hybrid and GPU) | At run time by wgpu's naga, to MSL, SPIR-V, HLSL or GLSL for the backend |
| GL, GLES, WebGL2 (Skia Ganesh GL everywhere, Skia in the browser, later Vello hybrid-webgl) | At build time by naga to GLSL ES 3.00 (`is_webgl: true`), the mechanism of `ferroui_build::shader_effects` |
| Metal (Skia Graphite) | At build time by naga to MSL, compiled by `newLibraryWithSource` (or to a metallib on Apple build hosts) |
| Vulkan (Skia Vulkan) | At build time by naga to SPIR-V |

One source keeps the port traceable to ProGPU's WGSL, uses the naga already in the workspace
(`naga = "=30.0.1"`, validated by `check_shader_module`), and cannot drift between backends, so the
comparison tests check math, not translations. The CPU reference implements the same functions in
Rust, its constants shared through a generated module or a test that compares the layouts. SkSL is
not an option (no vertex stage with depth), nor are storage-buffer layouts like `Native3D.wgsl`'s
(they exclude WebGL2 and GLES 3.0), nor wgpu everywhere (it would link wgpu into every Skia build
and cannot run on the ANGLE and WebGL2 contexts Skia owns; decision B).

**Lighting** (decision A, recorded in DEVIATIONS.md). ProGPU's `ComputeWpfLighting`, per pixel: the sum of the ambient lights
times the material's `AmbientColor`; Lambert diffuse; `SpecularMaterial` as an additive term with
`SpecularPower` (Blinn-Phong); `EmissiveMaterial` added without light; WPF's attenuation
`1 / max(c + l*d + q*d^2, 1)` cut off at `Range`; the spot falloff between the outer and the inner
cone, given as full angles and converted to half-angle cosines. The colour math is in the space
WPF uses (section 6.2). A per-vertex variant is a bit of the pipeline key if exact parity is wanted.

**Materials.** A draw is one material layer `{kind: diffuse | specular | emissive, color, brush
kind, brush uniforms, texture slot}`. `MaterialGroup` expands into draws of the same mesh in group
order: the first diffuse layer writes depth, later layers test `LessEqual` without writing it;
diffuse blends premultiplied source-over, specular and emissive `One, One` (section 6.2). Brushes:

- `SolidColorBrush`: uniforms.
- `LinearGradientBrush` and `RadialGradientBrush`: analytic in the shader with up to 8 stops inline,
  as ProGPU's `MaterialBrush3D`; more stops through a 256x1 lookup texture made once per brush
  version.
- `ImageBrush`: the backend's own image texture (Skia's image cache, Vello's uploads once per
  device).
- `VisualBrush`, `DrawingBrush` and `Viewport2DVisual3D`: an offscreen 2D render into a pooled
  texture, again only when that visual changes.
- Video: the frame texture (section 3.11).

Texture coordinates follow WPF's brush mapping (relative brushes map onto the bounding box of the
mesh's texture coordinates; section 6.2). The video sampling ports ProGPU's
`TransformMaterialCoordinate` (crop, quarter turns, mirroring) and the Rec.709 transfer of Skia's
`frame_color_filter`.

### 3.5 Pipeline caching

- **The key** is a `Copy` struct: vertex layout, material layer kind, brush kind (none, solid,
  gradient, texture, frame), cull mode (back, front, none), blend, depth write, sample count, color
  format, depth format and the frame-sampling variant (RGBA, BGRA with swizzle, Rec.709 transfer).
- Pipelines are created on first demand, from the same records the encoder draws, and cached per
  device or context, never per frame (the lesson of ProGPU's `docs/retained-3d-pipeline-demand.md`).
- Counters `pipelines_created` and `programs_linked`; the tests assert that steady frames create
  none.
- Prewarm: Skia Graphite registers the common keys through `SkiaGpuRenderTarget::prewarm` after the
  first frame; Vello has no prewarm, as for the effects.

### 3.6 Depth, MSAA and the output target

- **Formats.** Color RGBA8, premultiplied, cleared to transparent (0, 0, 0, 0) (ProGPU clears to an
  opaque grey; WPF's viewport is transparent). Depth private to the pass: `Depth32Float` in the wgpu
  executor (section 3.15: the `f32` depth of the CPU reference, a core format of WebGL2 and OpenGL
  ES 3.0), `DEPTH_COMPONENT24` or `DEPTH_COMPONENT32F` in GL.
- **Pooling.** Targets pooled by size rounded to 32 pixels, like `pooled_size`, with a depth pool
  like `vello_hybrid_scene_sink.rs`'s `depth_views`.
- **Size.** Physical pixels: the arranged size times the render scale; under a rotation or skew of
  the `Viewport3D` the bounds transformed to device pixels, as Skia's save layers are.
- **MSAA** (decision F). 4x where the executor reports it (wgpu's texture format features; GL's
  `MAX_SAMPLES` with `renderbufferStorageMultisample` and a `blitFramebuffer` resolve), resolved on
  the GPU; 1x for `RenderOptions.EdgeMode` `Aliased` (section 6.2). ProGPU's "1x at a scale of 1.5 or
  more" is an option, not the default.
- **Depth range.** `FarPlaneDistance` defaults to infinity and WPF fits the planes to the scene for
  precision (section 6.2): the snapshot compiler computes the camera-space bounds of the scene from
  the `Rect3D` bounds and clamps the planes to them, with an infinite-far projection as the
  fallback. Reverse-Z is not available on WebGL2 (no clip control), so it cannot be the only remedy.
- **Transparency.** Tree order, no sorting, no order-independent transparency (section 6.2).
- **Depth initialization.** Cleared once per render of a target and tracked per attachment, which
  avoids ProGPU's cached-slot bug.

### 3.7 Composition and the retained model

**UI thread.** `Viewport3D` (through its `Viewport3DVisual`) writes one new render-data op,
`DrawScene3D { scene: Arc<Scene3DSnapshot>, bounds }`, into the existing stream
(`render_data_writer.rs`, `render_data_opcode.rs`), so clips, transforms, opacity and layers apply
as to any image. A composition visual like `ServerCompositionCustomVisual` was the alternative; the
render-data op needs no new visual kind.

**`Scene3DSnapshot`**, immutable, made by flattening the tree: world matrices (f32), meshes as
`Arc<MeshData>` with an id and a version, the material layers, the lights in world space, the
camera (view and projection with the fitted planes), the brush and texture references, and three
generations after ProGPU's split: `structure_generation` (meshes and materials),
`record_generation` (transforms, lights, material values) and `camera_generation`.

**Render thread** (the backend's `draw_scene_3d`):

1. When the generations, the target size and every input texture identity equal those of the last
   render, the cached color texture is drawn again: no 3D pass. ProGPU renders every frame;
   FerroUI does not.
2. A new camera or record generation writes only the uniform buffers.
3. A new structure generation uploads only the meshes whose `(id, version)` changed. Mesh buffers
   are cached per device by `(id, version)`, indexed (vertex and index buffers), not de-indexed as
   in ProGPU, which costs three times the memory.
4. A new video frame binds the new texture and renders again, with no upload.
5. The result is drawn like an image: an `sk::Image` in Skia, `paint_device_layer` with
   `VelloSceneBrush::Texture` in Vello.

**Steady-state targets:** a moving camera allocates no texture, uploads only uniforms and creates no
pipeline; a static viewport while other 2D content animates issues no 3D pass.

**Synchronization.** One context (GL, Vello): queue order. Skia Graphite with the Metal executor on
the shared `MTLCommandQueue`: the 3D command buffer is committed before the frame's
`insert_recording` and `submit`; on another queue, the shared-event pattern of
`gpu/metal.rs::order_platform_after_device`. Skia Vulkan: the same `VkQueue`, with the image layout
declared to Skia through the backend texture. **Device loss** rebuilds from the retained snapshot;
the 3D caches are per device and dropped with it. **Threads:** the snapshot is built on the UI
thread and drawn on the render thread, so its data is `Send` (`Arc`) and holds no `Rc`.

### 3.8 Hit testing

On the CPU and the UI thread, as WPF does; rule 6 is about rendering, and a GPU readback would add
latency for nothing.

- **The ray** goes from a point in `Viewport3D` coordinates through the inverse of the camera's
  view and projection, with the same fitted planes as rendering; `MatrixCamera` uses its matrices.
- **Intersection:** Moller-Trumbore in model space (the ray through the inverse world matrix), over a
  bounding-volume hierarchy per mesh built lazily and cached by `(id, version)` (or during the
  snapshot compilation, which keeps large builds off the input path).
- **Results** sorted by distance, with barycentric `VertexWeight1..3`. A front face counts when the
  model has a `Material`, a back face when it has a `BackMaterial` (section 6.2).
- **`Viewport2DVisual3D` input:** the texture coordinate of the hit maps to a point of the hosted
  2D visual, and pointer input is routed to it through a transformed hit-test path in FerroUI's
  input system, with capture and focus in their normal lifecycle (new plumbing, step D11).
- **Accessibility** follows the media specification: a triangle is not an accessible element; the
  application describes meaningful scene actions.

### 3.9 Fallback chain per backend

If an executor fails to compile or create, it reports once, tries MSAA 1x, and then the viewport
draws the CPU reference, counted and recorded in gpu-rendering.md; never silently.

| Backend / mode | 1. GPU | 2. Software GPU | 3. CPU reference (tests, last resort) |
|---|---|---|---|
| Skia Graphite (Metal: macOS, iOS) | Metal executor | none on Apple (the iOS simulator has a GPU) | CPU raster and `draw_bitmap`, reported and counted |
| Skia Ganesh GL: ANGLE on Windows | GL executor | ANGLE on WARP (already in CI) | as above |
| Skia Ganesh GL: EGL on Linux | GL executor | Mesa llvmpipe | as above |
| Skia Ganesh GLES: Android | GL executor | SwiftShader (the emulator) | as above |
| Skia Ganesh GL: CGL on macOS | GL executor | none | as above |
| Skia Ganesh WebGL2 (browser) | GL executor | the browser's SwiftShader or software ANGLE | as above |
| Skia Ganesh Vulkan | Vulkan executor (D9) | lavapipe (already in CI) | until D9: the CPU reference, recorded as a gap |
| Skia raster, headless | none | none | the CPU reference: the surface has no GPU, so this is the defined path |
| Vello hybrid and GPU (wgpu on Metal, Vulkan, DX12, GLES) | wgpu executor | `DeviceType::Cpu` adapters (WARP, lavapipe, llvmpipe) only under `FERROUI_VELLO_ALLOW_SOFTWARE_ADAPTER` | the CPU reference |
| Vello CPU | none | none | the CPU reference (the mode is the reference) |
| Vello hybrid-webgl | GL executor once route step 3 of gpu-rendering.md gives the WebGL sink device textures | the browser's software GL | until then the CPU reference, a gap shared with every effect |
| Browser WebGPU (future) | wgpu executor (webgpu backend) | the browser's fallback adapter | the CPU reference |
| ProGPU backend (#369) | ProGPU's retained 3D with lighting equal to FerroUI's, or FerroUI's wgpu executor on ProGPU's device | ProGPU's fallback adapter or WARP | the CPU reference |

**The CPU reference rasterizer** (`ferroui-render3d::cpu`): deterministic safe Rust; homogeneous
clipping against the near and far planes; an edge-function rasterizer with the top-left fill rule;
perspective-correct interpolation; an f32 depth buffer; the lighting, material and brush math of
the WGSL in the same order; 4x ordered-grid supersampling for MSAA; texture sampling through the
existing CPU image paths, bilinear, with the same addressing. It is used by the comparison tests,
Skia raster, headless and Vello CPU, and on a GPU surface only as the counted last resort.

### 3.10 Browser

WebGL2 is the primary browser path: the GL executor on the Skia WebGL2 renderer now, and on Vello
hybrid-webgl after route step 3. WebGL1 draws the CPU reference (no guaranteed `gl_FrontFacing`,
limited formats), recorded as a gap. Video on 3D in the browser follows the 2D route: the
single-threaded build now, the threaded build after #601. WebGPU comes when FerroUI gains a WebGPU
path ([browser-graphite-webgpu.md](browser-graphite-webgpu.md)); the shader is WGSL, so it then
runs unchanged.

### 3.11 Shared-texture materials

Video frames reach every shipped route as GPU textures in the backend's own device or context
(`FrameLease`, `FrameMailbox`, `GpuCompletion` in `src/FerroUI.Base/rendering/composition/frames/`):
zero-copy on Apple Metal (IOSurface into Graphite and Vello), Windows ANGLE (D3D11 shared textures)
and Linux XRGB (DMA-BUF), one GPU copy on Linux NV12, Android and the browser. A 3D draw issued by
the same backend, on the same device or context and the render thread, binds that texture without a
copy. Three additions make that possible (step D7):

1. **A backend-neutral accessor** on the shared bitmap: a capability such as
   `ISharedGpuTexture::gpu_texture(&self) -> Option<GpuTextureRef<'_>>`, with `Metal { texture,
   format }` (from `LeasedGpuBitmap`), `Gl { id, target, context_id, generation }` (from
   `SharedFrameTexture`), `Wgpu(Arc<VelloDeviceTexture>)` (from `VelloSharedFrameBitmap`, crate-private
   today) and later `Vulkan { image, layout }`. Each carries the sampling transform of the
   `FrameMetadata` (crop, rotation, mirroring, origin, colour), which the 2D path already applies
   (`frame_layout`, `project_frame_point`).
2. **One import per frame, shared by 2D and 3D:** the imported `SharedBitmapImpl` cached on the
   render thread by `FrameIdentity` and device generation, used by `TexturePresenter` and the 3D
   material alike, so a `MediaPlayerElement` shown flat and on a cube imports once. On Vello the
   converted pooled RGBA texture is shared (one counted GPU conversion per frame); on Graphite and the
   GL routes the imported texture is sampled directly, with the Rec.709 transfer in the fragment
   shader (`sample_frame()`). Today `TexturePresenter` imports and disposes every frame.
3. **Lifetime:** the 3D submission retains the lease as 2D does (`ISkiaGrContext::retain_frame`,
   Vello's `release_frames_after_submitted_work`, `IGlContext::schedule_frame_completion`). A video
   material subscribes to the `FrameMailbox` and invalidates only the 3D output, never the scene
   structure: a new frame is new bindings and one render of the target, with no mesh or record
   upload.

The API (decision D): WPF's form, `DiffuseMaterial.Brush = VisualBrush { Visual = MediaPlayerElement }`,
with a recognized fast path that samples the frame lease instead of rendering the element (also
for `CameraPreview`), and a FerroUI `VideoBrush` over a `FrameMailbox`. A general `VisualBrush` or
`Viewport2DVisual3D` renders into a pooled texture through the device-layer route. The 3D path adds
no frame routes of its own: it consumes what the 2D media path admits and reports
`FrameError::MissingImporter` where 2D does (Skia Vulkan, Skia Ganesh on Metal, Vello on DX12,
Vulkan and GLES, browser WebGPU, the threaded browser until #601, AHardwareBuffer, and no direct NV12
sampling).

**Built so far (D7, part 1: Vello).** The wgpu executor takes textures the renderer already has on
the device (`Scene3DDeviceTextures`, `Scene3DWgpuExecutor::render_with_textures`): it samples them,
never writes them, and keeps their owner while it keeps the bind group. An image brush whose bitmap
is a `VelloSharedFrameBitmap` of the device samples the frame's converted texture, the one
conversion the 2D path draws from, so a frame shown flat and on a mesh is converted once. A new
frame is a new render with no mesh or texture upload. Without a texture for it, the layer is drawn
with its colour and counted, as for any bitmap whose pixels cannot be read; the CPU reference does
not sample frames.

Not built yet: `VideoBrush` and the visual-brush fast path (today a frame reaches a material only as
the bitmap of an image brush), the import cache by `FrameIdentity` in `TexturePresenter`, the
Graphite and GL routes with `sample_frame()`, and the sample.

### 3.12 The values as built (D1)

`src/FerroUI.Media3D`, crate `ferroui-media3d`: one file per value with its tests beside it
(`point_3d.rs` and `point_3d_tests.rs`, and so on), `error.rs`, `text.rs` (the text forms) and
`markup_types.rs` (the markup metadata). `NOTICE.md` records that the API and the behaviour follow
WPF (MIT, .NET Foundation).

**Naming.** WPF's members in snake case; an overload gets a suffix (PORTING-GUIDE); a static WPF
method whose name an instance method already has gets a plural or a descriptive suffix; operators
are the `std::ops` traits; WPF's explicit conversions are `From`; WPF's exceptions are `Result`
errors (`Matrix3DError`, `QuaternionError`, `Size3DError`, each `std::error::Error`); the text forms
are `parse`, `FromStr` and `Display`.

| WPF | Rust |
|---|---|
| `Point3D` (`X`, `Y`, `Z`, `Offset`, `+`/`-` with `Vector3D`, `Point3D - Point3D`, `* Matrix3D`, explicit to `Vector3D` and `Point4D`) | `Point3D` (`x`, `y`, `z` fields, `offset`, `Add`/`Sub`/`Mul` impls, `From`) |
| `Vector3D` (`Length`, `LengthSquared`, `Normalize`, `Negate`, `DotProduct`, `CrossProduct`, `AngleBetween`, the arithmetic, explicit to `Point3D` and `Size3D`) | `Vector3D` (`length`, `length_squared`, `normalize(&mut self)`, `negate`, `dot_product`, `cross_product`, `angle_between` in degrees; `Add`, `Sub`, `Neg`, `Mul<f64>`, `f64 * Vector3D`, `Div<f64>`, `Mul<Matrix3D>`, the assignments; `From`; `From` both ways with the base library's `Vector3D`) |
| `Size3D` (`X`, `Y`, `Z` guarded, `Empty`, `IsEmpty`) | `Size3D` (private fields; `x()`, `set_x(..) -> Result<(), Size3DError>`; `new(..) -> Result`; `EMPTY`; `is_empty`) |
| `Rect3D` (`X`..`SizeZ`, `Location`, `Size`, `Empty`, `IsEmpty`, `Contains` x3, `IntersectsWith`, `Intersect`, `Union` with a box and a point, `Offset`, the static forms) | `Rect3D` (private fields and guarded setters; `new(..) -> Result`, `from_location_size`; `EMPTY`; `contains`, `contains_xyz`, `contains_rect`, `intersects_with`, `intersect(&mut self)`, `intersect_rects`, `union(&mut self)`, `union_point`, `union_rects`, `union_rect_point`, `offset`, `offset_xyz`, `offset_rect`, the last three `-> Result`) |
| `Point4D` (`X`, `Y`, `Z`, `W`, `Offset`, `+`, `-`, `* Matrix3D`) | `Point4D` (fields, `offset`, `Add`, `Sub`, `Mul<Matrix3D>`) |
| `Matrix3D` (`M11`..`M44`, `OffsetX/Y/Z`, `Identity`, `IsIdentity`, `IsAffine`, `HasInverse`, `Determinant`, `SetIdentity`, `Invert`, `Append`, `Prepend`, `Rotate`, `RotatePrepend`, `RotateAt`, `RotateAtPrepend`, `Scale`, `ScalePrepend`, `ScaleAt`, `ScaleAtPrepend`, `Translate`, `TranslatePrepend`, `Transform` of points, vectors and `Point4D` and their arrays, `*`) | `Matrix3D` (fields `m11`..`m34`, `offset_x`, `offset_y`, `offset_z`, `m44`; `IDENTITY` and `Default`; `is_identity`, `is_affine`, `has_inverse`, `determinant`, `set_identity`, `invert(&mut self) -> Result<(), Matrix3DError>`, `append`, `prepend`, `rotate`, `rotate_prepend`, `rotate_at`, `rotate_at_prepend`, `scale`, `scale_prepend`, `scale_at`, `scale_at_prepend`, `translate`, `translate_prepend`, `transform`, `transform_vector`, `transform_point4d`, `transform_points`, `transform_vectors`, `transform_points4d`; `Mul`, `MulAssign`) |
| `Quaternion` (`X`, `Y`, `Z`, `W`, `Identity`, `IsIdentity`, `IsNormalized`, `Angle`, `Axis`, `Conjugate`, `Invert`, `Normalize`, `Slerp` with and without `useShortestPath`, the constructor from axis and angle, `+`, `-`, `*`) | `Quaternion` (fields; `IDENTITY` and `Default`; `is_identity`, `is_normalized`, `angle` in degrees, `axis`, `conjugate`, `invert`, `normalize`, `slerp`, `slerp_with_path`, `from_axis_angle(..) -> Result<_, QuaternionError>`; `Add`, `Sub`, `Mul`) |

The methods that change a value take `&mut self`, as WPF's change the structure in place; the
default `Matrix3D` and `Quaternion` are the identity, as WPF's are. WPF's "distinguished identity"
(a flag that makes the default value the identity without storing it) is not needed: the default is
the identity values, and every result equals WPF's. `HasInverse` and `IsNormalized` use WPF's
tolerance of ten machine epsilons. `Normalize` of a vector or quaternion divides by the largest
component first, so the squares do not overflow; `AngleBetween` uses the arc sine of half the
difference of the unit vectors, precise near 0 and 180 degrees; `Slerp` follows WPF's branches
(linear above a cosine of `1 - 1e-6`, a perpendicular for opposite quaternions, the length
interpolated geometrically).

**The f64 `Vector3D` and `Quaternion` of this crate are separate types.** The base library's
`ferroui_base::Vector3D` is the composition vector of the reference, with its own shape (a static
`normalize` that returns a new vector, `dot` instead of `DotProduct`, a debug-style text form); it
stays unchanged and converts with `From`. The f32 `numerics::Quaternion` stays the numerics type;
`scripts/idioms/baseline.json` lists the WPF `Quaternion` of `ferroui-media3d` as a deliberate
declaration (its name is also that of `System.Numerics.Quaternion`).

**Text forms and markup.** Each value parses and writes WPF's text form in the invariant culture:
numbers separated by a comma, white space or both (`"1,2,3"`, `"1 2 3"`), written with commas and
the shortest round-trip digits; `Matrix3D` and `Quaternion` also read and write `"Identity"`,
`Size3D` and `Rect3D` `"Empty"` (compared case-sensitively after trimming, as WPF does). A malformed
text is a `FormatError` ("Invalid Point3D.", ...), a negative size the message of
`Size3DError::NegativeDimension`. `markup_types.rs` declares each value with `ferro_markup_type!` in
the namespace `FerroUI.Media.Media3D`: the parse (the type converter of markup), the constructors,
the components as read-only properties and the static `Identity` and `Empty`;
`ferroui_media3d::register_types()` registers the table, the namespace, the assembly
(`FerroUI.Media3D`) and the nullable forms, and the build script exports the type model for the
markup compiler of dependent crates (`links = "ferroui_media3d_xaml"`). A property of one of these
types therefore takes `Position="0,0,5"` from markup. The two points that waited for the scene
objects (the XML namespace and the collections) are settled by D2 (section 3.13). The constructor of
the 16 elements of `Matrix3D` is not declared for markup, which writes a matrix as text.

### 3.13 The scene model as built (D2)

**Classes.** The scene objects are classes of FerroUI's object model (`ferro_class!`, styled
properties, `ferro_class_info!` for markup), one file per family: `transform_3d.rs`
(`Transform3D` with the virtual `value`, `TranslateTransform3D`, `ScaleTransform3D`,
`RotateTransform3D`, `MatrixTransform3D`, `Transform3DGroup`), `rotation_3d.rs` (`Rotation3D` with
the virtual `internal_quaternion`, `AxisAngleRotation3D`, `QuaternionRotation3D`), `camera.rs`
(`Camera` with the virtuals `get_view_matrix` and `get_projection_matrix`, `ProjectionCamera`,
`PerspectiveCamera`, `OrthographicCamera`, `MatrixCamera`), `model_3d.rs` (`Model3D` with the
virtual `bounds_core`, `GeometryModel3D`, `Model3DGroup`), `light.rs` (`Light`, `AmbientLight`,
`DirectionalLight`, `PointLightBase`, `PointLight`, `SpotLight`), `geometry_3d.rs` (`Geometry3D`
with the virtual `bounds`, `MeshGeometry3D`), `material.rs` (`Material`, `DiffuseMaterial`,
`SpecularMaterial`, `EmissiveMaterial`, `MaterialGroup`), `visual_3d.rs` (`Visual3D`,
`ModelVisual3D`), `viewport_3d_visual.rs` and `controls/viewport_3d.rs`. The freezables of WPF
derive from `Animatable`, `Visual3D` and `Viewport3DVisual` from `FerroObject` (WPF's
`DependencyObject`), `Viewport3D` from `Control` (FerroUI has no `FrameworkElement`; `Control` is
its counterpart). The defaults are WPF's: the cameras at the origin looking along (0, 0, -1) with
(0, 1, 0) up, `NearPlaneDistance` 0.125, `FarPlaneDistance` infinite, `FieldOfView` 45 (horizontal),
`Width` 2; the colours of the materials and lights white, `SpecularPower` 40; `DirectionalLight` and
`SpotLight` along (0, 0, -1); `Range` infinite with the attenuations 1, 0, 0; `InnerConeAngle` 180
and `OuterConeAngle` 90; `AxisAngleRotation3D` about (0, 1, 0) by 0 degrees. A transform, a rotation,
a geometry or a material property that WPF defaults to `Identity` or `null` is `None` (the identity,
or nothing), and the collections of a mesh and the children of the groups are empty collections
created with the object.

**Collections.** `Point3DCollection`, `Vector3DCollection`, `Int32Collection`,
`Model3DCollection`, `Transform3DCollection`, `MaterialCollection` and `Visual3DCollection` are
named shared handles over the base library's observable `MediaCollection` (`collections.rs`),
the form of the collections of the 2D media classes; the texture coordinates are the base
library's `Points` (WPF's `PointCollection`). The value collections parse and write WPF's text
forms in the invariant culture: numbers separated by commas, white space or both, three per item
for the 3D values (`Positions="0,0,0 1,0,0 0,1,0"`, `TriangleIndices="0 1 2"`), the items written
separated by a space; a malformed text is a `FormatError` ("Invalid Point3DCollection.").

**Change notification and generations.** Each scene object has a notifier (`change.rs`). A
change of a property raises a `Scene3DChange` of one of three kinds, and an owner forwards the
changes of the objects it holds (a property value, the items of a collection) to its own notifier,
so a change anywhere reaches the `Viewport3DVisual`, which bumps the generation of that kind and
drops its snapshot. The rules: a mesh (any collection or property of it), a model, light or visual
added, removed or replaced, a geometry or material replaced, a new brush of a material (or none)
and the children of a `MaterialGroup` are the **structure**; a transform (of a model, a visual or a
light, including the children of a `Transform3DGroup`), a rotation, any property of a light, a colour or the specular power of a material and a
change of the values of its brush (the brush's own invalidation) are a **record**; any property of
the camera, its transform and the `Viewport` (the arranged size) are the **camera**. Each change
bumps exactly one generation. A removed object is no longer followed. A mesh has an id for the
process and a version that every change bumps; its `MeshData` is made once per version and shared
by every model and snapshot that draws it.

**The snapshot** (`scene_compiler.rs`) flattens the visuals in tree order: the world matrix of a
model is its transform, then those of its groups, then those of its visuals (row vectors), narrowed
element by element to `numerics::Matrix4x4`. A `GeometryModel3D` is a record when its geometry is a
`MeshGeometry3D` and one of its materials has a layer; a `MaterialGroup` is flattened into its
layers in order; a material without a brush is no layer (WPF draws nothing with it); a brush is kept
in its shared form (`SharedBrush`), or as `None` when it has none yet (a visual or drawing brush,
D6). The lights are in world space (positions transformed, directions transformed and normalized;
`Range` is not scaled). The tree is followed to a depth of 256, so a tree that holds itself ends.
The snapshot keeps, for the 3D hit testing of D10, the world matrices, the meshes in the
coordinates of their models, which materials a model has on each side, and in the
`Viewport3DVisual` (UI thread only) the visual and the model of each record by its index
(`model_source`).

**The camera and the planes.** The view matrix is WPF's look-at matrix with the inverse of the
camera's transform before it (a transform without an inverse gives no camera: nothing is drawn, as
WPF draws nothing). The projections are WPF's, to a clip depth between 0 and `w`, with the aspect
ratio of the viewport. When `FarPlaneDistance` is infinite (the default) the snapshot fits both
planes to the depths of the corners of the world bounds of the drawn models in the coordinates of
the camera, widened by a thousandth of the depth range, the near plane never closer than the
camera's `NearPlaneDistance` (and in front of a perspective camera); a finite far plane keeps the
camera's planes, and an empty scene, or a scene entirely behind the near plane, keeps the infinite
far plane (WPF's infinite-far projection). `MatrixCamera` takes its matrices as they are.

**Rendering.** `Viewport3D` records one operation, `DrawScene3D { scene, bounds }`: the opcode
`RenderDataOpcode::DrawScene3D` (appended, so earlier byte values stay), the payload
`DrawScene3DPayload { scene: i32, bounds: Rect }` and the resource `RenderDataResource::Scene3D`,
sent to the render thread as the shared snapshot (not copied). The replay hands it to the optional
capability `IDrawingContextImplWithScene3D::draw_scene_3d(&Scene3DFrame) -> bool`
(`src/FerroUI.Base/platform/i_drawing_context_impl_with_scene_3d.rs`), reached through
`IDrawingContextImpl::as_drawing_context_impl_with_scene_3d` and forwarded by the compositor's
proxy, as the effect and layer capabilities are. No backend implements it in D2: a scene then draws
nothing, `rendering::scene3d::scenes_not_drawn()` counts it, and the first one is logged (area
`Visual`, warning), the way the gaps of the GPU features are reported; D3 adds the CPU reference
and D4 the first GPU route. The bounds of the operation are the viewport (a scene draws at most
there); the 2D hit test does not hit a scene (D10 hits its models).

**XML namespace.** `FerroUI.Media.Media3D` and `FerroUI.Controls` are mapped into the framework's
XML namespace by the assembly of the crate (`markup_types.rs`), so markup names
`<Viewport3D>`, `<PerspectiveCamera>` or `<MeshGeometry3D>` without a prefix, as WPF's default
namespace does. The name both namespaces of the mapping have, `Vector3D` (the 3D vector here, the
composition vector of the base library in `FerroUI`), is settled by how markup resolves names: a
property declares its type, so `LookDirection="0,0,-1"` or `Axis="0,1,0"` reads the 3D vector
whatever the mapping, and only a type named in markup by its name (an element `<Vector3D>`,
`x:Type`, `x:DataType`) is looked up in the XML namespace, where the first mapped namespace with the
name wins: the base library's, registered first, so existing markup keeps its meaning. The 3D
vector is named there through its namespace (`xmlns:media3d="using:FerroUI.Media.Media3D"`,
`<media3d:Vector3D>`). Neither type is renamed and nothing in the base library changes.

**Facade.** The feature `media3d` of the crate `ferroui` (off by default) re-exports the crate as
`ferroui::media3d`, the naming rule of the facade (docs/release.md, section 2.1).

### 3.14 The CPU reference as built (D3)

**The crate.** `src/FerroUI.Rendering3D`, crate `ferroui-render3d`, depends on the base library and
std only and forbids `unsafe`. Its modules: `uniforms` (the constants of a render), `lighting`
(ProGPU's `ComputeWpfLighting` and the colour of each kind of layer), `cpu` (the rasterizer) and
`output_cache` (the kept output). `NOTICE.md` records the port of the lighting from ProGPU (MIT) and
the behaviour of WPF.

**Uniforms** (`uniforms.rs`), `#[repr(C)]` and std140-compatible (every member a `vec4`, a `mat4x4`
or an array of structures of whole `vec4`s, the offsets asserted by the tests), with the bytes
written by `Std140::write_std140` and the same structures in WGSL in `WGSL_DECLARATIONS`, which the
shader of D4 includes:

| Structure | Members (byte offset) | Size |
|---|---|---|
| `LightUniforms` (ProGPU's `GpuLight3DRecord`) | `metadata` (0; `x` the kind: 0 ambient, 1 directional, 2 point, 3 spot), `color` (16), `position_range` (32; `w` the range, `f32::MAX` for infinite), `direction_inner_cos` (48; `w` the cosine of half the inner cone), `attenuation_outer_cos` (64; `w` the cosine of half the outer cone) | 80 |
| `FrameUniforms` | `view_projection` (0), `camera_position` (64; `w` 1 for a position, 0 for the direction toward an orthographic camera), `light_count` (80, `vec4<u32>`), `lights` (96, 16 `LightUniforms`) | 1376 |
| `DrawUniforms` | `world` (0), `normal_matrix` (64; the inverse transpose of the upper 3x3), `color` (128; the brush's colour times the material's `Color`, the brush's opacity in the alpha, straight), `ambient_color` (144), `material` (160; `x` the layer kind: 0 diffuse, 1 specular, 2 emissive; `y` `SpecularPower`; `z` the sign of the normals, -1 for back faces) | 176, at a stride of 256 for dynamic offsets |

A matrix is stored as its rows, which a column-major `mat4x4<f32>` reads as its columns, so the
shader's `M * v` is the snapshot's row-vector `v * M`; `view_projection` is `view * projection`.
Colours are the gamma-encoded components over 255 (section 6.2).

**The pipeline** (`cpu/mod.rs`, `cpu/clip.rs`), stage for stage that of the executors:

1. The vertex stage once per model: the world position, the normal through the normal matrix at
   unit length, the texture coordinate, and the clip position through `view_projection`.
2. Clipping in homogeneous coordinates (Sutherland-Hodgman) against the near plane `z >= 0`, the
   far plane `z <= w`, `w >= 1e-6` (for matrix cameras) and a guard band `|x|, |y| <= 4w` that keeps
   the fixed-point coordinates small; the attributes interpolated linearly in clip space, the
   polygon drawn as a fan.
3. Setup: the perspective division, the viewport with rows going down, the vertices snapped to
   1/256 of a sample. A triangle counterclockwise on the screen is a front face (WPF); a face whose
   side has no layers is culled there.
4. Rasterization by integer edge functions with the top-left rule (a sample on an edge shared by
   two triangles is covered once), within the bounding box of the triangle and a tile of 64 by 64
   pixels; the triangles are binned to the tiles once per render (a counting sort that keeps their
   order).
5. Per sample, after the depth test: the barycentric weights corrected for the perspective
   (`w`), the interpolated world position and normal, and `lighting::shade`.
6. Depth: `f32`, z over w, cleared to 1. The first diffuse layer of a side writes it with `Less`
   (the first layer when that side has no diffuse layer); the other layers test `LessEqual` and do
   not write it. Back layers are drawn after the front layers of the same model.
7. Blending: a diffuse layer premultiplied source-over, a specular or emissive layer added with
   alpha 0, every channel clamped to 0..1 as an RGBA8 target stores it.
8. Resolve: 4x supersampling on an ordered grid (the samples at a quarter and three quarters of a
   pixel each way; 1x with `CpuRenderOptions { antialiased: false }`, the edge mode `Aliased`), the
   mean of the samples, the alpha raised to the largest colour channel (decision N), rounded to
   premultiplied RGBA8.

**Lighting** (`lighting.rs`). `compute_wpf_lighting` is ProGPU's loop over at most 16 lights: an
ambient light adds its colour; a directional light lights along `-direction`; a point or spot
light lights from its position, not beyond its range or at the light itself, attenuated by
`1 / max(c + l d + q d^2, 1)`; a spot light is scaled by `clamp((rho - cos_outer) /
max(cos_inner - cos_outer, 1e-6), 0, 1)` with the cosines of the half angles; `N . L <= 0`
contributes nothing; the specular sum is Blinn-Phong, `max(N . H, 0)` to `SpecularPower`, with the
attenuation. `shade` returns, premultiplied: for a diffuse layer `saturate(color * (ambient *
ambient_color + diffuse))` with the alpha of the colour; for a specular layer `saturate(color *
specular)` with alpha 0; for an emissive layer the colour with alpha 0. The view direction is toward
the camera's position, or the one direction of an orthographic camera.

**Brushes.** A solid colour brush is its colour and opacity. As built in D3, any other brush was
drawn with the layer's `Color` and the brush's opacity, counted
(`ferroui_render3d::layers_with_unsampled_brushes`) and reported once; D6 samples gradients and
images (section 3.18).

**Performance.** Nothing is allocated per pixel or per triangle: a `CpuRasterizer` keeps its
vertex, triangle, pass, bin and tile buffers from one render to the next; rows are slices of the
tile buffers. The image itself is one allocation per render.

**Output cache** (`output_cache.rs`). `Scene3DOutputCache<T>` keeps the backend's form of the image
(an `sk::Image`, a Vello `ImageData`) by the snapshot (its allocation, held weakly, and its
generations), the size, the scale and the options, eight outputs by default, the least recently
used making room; a new output of a scene replaces its older one, and the outputs of dropped
snapshots go. An unchanged viewport is neither rasterized nor uploaded again.

**The backends.** `ferroui-skia` (`scene3d.rs`) and `ferroui-vello`
(`drawing_context_impl/scene_3d.rs`) implement `IDrawingContextImplWithScene3D` behind their
feature `media3d` (decision M), which the facade's `media3d` turns on. The size of the image is the
bounds of the frame at the resolution of the device (`ferroui_render3d::scene_pixel_size`: the
lengths of the transformed axes, so also under a rotation or skew; at most 8192 pixels a side; the
perspective of a 2D transform is not taken into account). The image is drawn over the bounds with
bilinear sampling under the transform, clips and opacity of the context: on Skia an image of pixels
through `drawable_image` (raster, Ganesh and Graphite alike; Graphite keeps the texture while the
kept image lives), on Vello an image paint of the scene (CPU, hybrid and GPU). Skia without a GPU
context and the CPU mode of Vello draw the reference by definition; Skia with a GPU context and the
hybrid and GPU modes of Vello count every such scene (`ferroui_render3d::scenes_drawn_on_cpu`) and
log the first, the gap D4 and later close (D4 closes it for the hybrid and GPU modes of Vello,
section 3.15). The headless platform's own drawing context draws
nothing at all (also no 2D) and has no 3D capability; the headless platform with Skia draws through
Skia raster.

**Tests.** The unit tests of the crate: clipping at the near and far planes, a triangle beyond the
far plane, the fill rule on the shared diagonal and on the edges of a quad through pixel centres,
the resolve of an edge through a pixel, depth in either order, the layers of a material on the same
depth, culling and `BackMaterial`, perspective-correct interpolation, the lighting against hand
values (Lambert, attenuation and range, the spot cone, Blinn-Phong, each kind of layer), the
layouts and their bytes, the WGSL members, the normal matrix, and the cache. The comparison scenes
(`tests/FerroUI.RenderBackends.Comparison/scene3d_scenes.rs`, `scene3d_tests.rs`), 100 pixels at
1x and 2x: `lit-cube`, `point-light`, `spot-light`, `orthographic-camera`, `matrix-camera`,
`front-back-culling`, `material-group`, `transparent-overlap`, `near-far-clipping`,
`clip-opacity-rotated` and `empty`, made by hand with WPF's camera matrices. The tests hold every
backend and mode to the CPU reference (`Tolerance::Largest(2)` for the scenes drawn one to one),
check the golden properties with pixel probes whose colours are computed from WPF's formulas in
double precision (for example the +z face of the turned cube is red at 64 + 128 cos 30 of 255, the
background transparent), compare Skia raster with each Vello mode (no pixel beyond the tolerance
of the harness for the plain scenes, 3 % for the rotated one), check that a `MatrixCamera` with a
perspective camera's matrices draws the same image, that an empty scene draws nothing, and that an
unchanged viewport is rasterized once.

### 3.15 The wgpu executor as built (D4)

**The shader** (`src/FerroUI.Rendering3D/shaders/scene3d.wgsl`, `shader.rs`). One WGSL module, a
port of ProGPU's `Mesh3DSolid.wgsl` rewritten to the subset of section 3.4: the uniform blocks
`FrameUniforms` (group 0) and `DrawUniforms` (group 1, at a dynamic offset per draw), declared by
the text of `uniforms::WGSL_DECLARATIONS` unchanged; interleaved vertices of 32 bytes (position,
normal, texture coordinate at locations 0 to 2) drawn with `u32` indices; `@builtin(front_facing)`
for the side of a face; the loop over at most 16 lights; f32 throughout. The functions are those of
`lighting.rs` and `cpu/mod.rs` in the same order of operations (`safe_normalize` keeps the zero
vector zero, as the CPU reference does); the clip position is `@invariant`, so the layers of a
material, drawn over the same triangles with `LessEqual`, have the same depth. Four entry points:
`vs_main` and `fs_main` draw a layer, `vs_resolve` and `fs_resolve` are the resolve of decision N
(the resolved colour with its alpha raised to its largest colour channel, read by `textureLoad`).
The tests of `shader.rs` validate the module with naga, translate every entry point to GLSL ES
3.00 with `is_webgl: true` (`invariant gl_Position`, `gl_FrontFacing`, `texelFetch`, std140
blocks), check that the declarations are those of `uniforms.rs` and that naga lays the structures
out at the offsets of the Rust structures, that only uniform blocks under 16 KiB and textures are
bound, and how a mesh is written (`shader::write_mesh`: missing normals and texture coordinates
zero, triangles naming a vertex that does not exist left out, as the CPU reference does).

**Where it lives.** `ferroui-render3d`, module `wgpu_executor`, behind the crate's feature `wgpu`
(the workspace's `wgpu`, without `unsafe`). The executor takes a `wgpu::Device`, a `wgpu::Queue`
and the view of an output texture, nothing of Vello's, so the browser's WebGPU path and the ProGPU
backend (D13) can draw with it too; `ferroui-vello` turns the feature on with its GPU modes and
`media3d` (`wgpu-device` names `ferroui-render3d?/wgpu`) and keeps what is Vello's (the pool of
device textures, the kept outputs, the painting) in `gpu/scene3d_passes.rs`.

**The executor** (`Scene3DWgpuExecutor`, one per device and thread):

- **Draws.** One draw per material layer, the front layers of a model and then its back layers, in
  tree order; a front draw culls the back faces (`FrontFace::Ccw`: counterclockwise on the screen is
  a front face) and a back draw the front faces. The first diffuse layer of a side writes depth with
  `Less`, the others test `LessEqual` without writing; a diffuse layer blends premultiplied
  source-over, a specular or emissive layer `One, One`.
- **Pipelines** by `Scene3DPipelineKey` (`Copy`: the blend, the side, whether depth is written, the
  sample count, the colour and depth formats), made on first demand inside an error scope and kept
  in a `HashMap` of the executor, so at most eight per sample count and device, and one resolve
  pipeline; counted (`pipelines_created`). The shader module, the three bind group layouts and the
  two pipeline layouts are made once with the executor. The brush shader joins the key with D6
  (section 3.18), the frame-sampling variant with D7.
- **Meshes** by `(id, version)`: an interleaved vertex buffer and an index buffer, uploaded with
  `Queue::write_buffer` when the pair is new; a new version of a mesh reuses the buffers of the
  version it replaces when they are large enough. Meshes and targets not drawn for 240 renders go.
- **Uniforms.** A `Scene3DViewport` holds the uniform buffers of one viewport with the bytes they
  hold: the `FrameUniforms` (1376 bytes) in one, the `DrawUniforms` of its draws in another at a
  stride of `min_uniform_buffer_offset_alignment` (176 bytes rounded up: 256 on every current
  device), grown by powers of two, each with its bind group made once with the buffer. A render
  writes the frame block when its bytes changed and the range of the draws whose bytes changed:
  a camera that moves writes the 1376 bytes of the frame block alone.
- **Targets.** The multisampled colour, the depth and the resolved colour of decision N are pooled
  by the size of the output texture and the sample count, made on first need; depth is cleared once
  per render and not stored. 4x MSAA where `Rgba8Unorm` and `Depth32Float` take four samples by
  WebGPU's guarantees, one sample for `EdgeMode` `Aliased`. A scene of diffuse layers alone is
  resolved by the GPU into the output; a scene with specular or emissive layers is resolved into
  the pooled colour, and `fs_resolve` writes the output from it. The scene is drawn at the top left
  of the output (viewport and scissor of its size); the rest of the texture is transparent.
- **Failures.** The executor's creation, each pipeline and each render run in error scopes
  (validation and memory). A failure with four samples is reported once and the executor draws
  with one from then on; a failure with one sample is the error of the render.

**Vello** (`gpu/scene3d_passes.rs`, `drawing_context_impl/scene_3d.rs`). In the hybrid and GPU
modes (a scene of the device that paints with its textures) `draw_scene_3d` asks
`scene3d_passes::scene_texture` for the texture of the scene and paints it over the bounds, the
image's pixels stretched over them and sampled bilinearly, as the image of the CPU reference is
painted, under the transform, clips, opacity and layers of the context. Each device keeps up to
eight outputs, each a texture of the pool (`effect_passes::pooled_texture`, sizes rounded up to 32
pixels) with its `Scene3DViewport`:

1. the snapshot, its generations, the size, the scale and the edge mode of a kept output: the
   output is painted again, no 3D pass (`cached_output_frames`);
2. otherwise the output of the same viewport (the same device position and size), of a snapshot
   that is gone, a new one, or the least recently used is drawn into; one that a scene not yet
   rendered still paints with is swapped for another texture of the pool;
3. a render that wrote uniforms and made nothing is a `uniform_only_frames`.

An executor that cannot be made, or a render that fails, is reported once (area `Visual`, level
error); that device then draws its 3D scenes with the CPU reference, each counted
(`scenes_drawn_on_cpu`). The CPU mode, and a scene of the CPU renderer inside a GPU mode, draw the
CPU reference as before. `ferroui_vello::gpu::scene3d_counters()` returns the counters of the
thread (`VelloScene3DCounters`: pipelines created, mesh uploads, uniform-only frames, cached-output
frames, 3D passes, textures and buffers made, uniform bytes, scenes drawn on the GPU).

**Tests.** `scene3d_tests.rs` holds the hybrid and GPU modes to the CPU reference within 3 levels
inside the faces (a pixel whose 3x3 neighbourhood in the reference differs by at most 8 levels) and
within the mean of `Tolerance::Moves` over the image, since their edges are 4x MSAA (rotated-grid
samples) and the reference's an ordered grid; the golden probes, inside the faces, keep their
levels; the comparison with Skia raster allows the share of those edges (3 %).
`steady_frames_of_a_moving_camera_upload_only_uniforms_and_allocate_no_texture` (eight frames of a
camera turning about the cube after two to warm up: eight 3D passes, eight uniform-only frames, no
pipeline, mesh upload, texture or buffer, the frame block's bytes alone, nothing rasterized),
`a_static_viewport_issues_no_3d_pass` (six frames whose 2D content animates: no pass, six kept
outputs, no uniform written, the same pixels) and `pipelines_are_created_once_per_device` (every
scene in each GPU mode at 1x and 2x makes at most one pipeline per key, and drawing them again, in
the same or the other mode, which draws on the same device, makes none). The unit tests of
`wgpu_executor.rs` check the stride, the key, three pipelines made once per device for a diffuse
and an emissive layer (and three again on a second device), and a moving camera writing only the
frame block.

### 3.16 The hit testing as built (D10)

**Where.** `src/FerroUI.Media3D/hit_testing/`: `mod.rs` (the public types and the walk of the
tree), `ray.rs` (the ray of a viewport point), `bvh.rs` (the hierarchy of a mesh and the
intersection), `tests.rs`. On the CPU and the UI thread, as WPF.

| WPF | Rust |
|---|---|
| `RayHitTestParameters` (`Origin`, `Direction`) | `RayHitTestParameters` (`new`, `origin`, `direction`) |
| `HitTestResult` (`VisualHit`), `RayHitTestResult` (`ModelHit`, `PointHit`, `DistanceToRayOrigin`) | the traits `HitTestResult` and `RayHitTestResult` |
| `RayMeshGeometry3DHitTestResult` (`MeshHit`, `VertexIndex1..3`, `VertexWeight1..3`) | the struct `RayMeshGeometry3DHitTestResult` (`visual_hit`, `model_hit` as the `GeometryModel3D`, `mesh_hit`, `point_hit`, `distance_to_ray_origin`, `vertex_index_1..3`, `vertex_weight_1..3`; and `is_front_face`) |
| `HitTestResultBehavior`, `HitTestFilterBehavior` | enums with WPF's members |
| `HitTestFilterCallback`, `HitTestResultCallback` | `&mut dyn FnMut(&Ref<Visual3D>) -> HitTestFilterBehavior`, `&mut dyn FnMut(&RayMeshGeometry3DHitTestResult) -> HitTestResultBehavior` |
| `VisualTreeHelper.HitTest(Visual, Point)` on a viewport | `VisualTreeHelper::hit_test(viewport, point) -> Option<RayMeshGeometry3DHitTestResult>` |
| `VisualTreeHelper.HitTest(Visual, filter, result, PointHitTestParameters)` | `VisualTreeHelper::hit_test_point(viewport, filter, result, point)` |
| `VisualTreeHelper.HitTest(Visual3D, filter, result, HitTestParameters3D)` | `VisualTreeHelper::hit_test_ray(visual, filter, result, &RayHitTestParameters)` |

A viewport is a `Viewport3DVisual` or a `Viewport3D` (the trait `Viewport3DHitTestTarget`); the
point is in the coordinates of the element (those of `Viewport3DVisual::viewport`).

**The ray of a point.** From the snapshot of the viewport (`Viewport3DVisual::snapshot`): its
camera's view and projection, widened to double precision, with the planes the snapshot fitted
to the scene (section 3.13), so the ray is the one rendering draws with. The point is mapped to
the device coordinates as the rasterizer maps them (x to the right, rows down) and unprojected
at the clip depths 0 and 0.5 (finite also under an infinite far plane). The origin is the
position of a perspective camera; for an orthographic camera the point of the plane through its
position (of the near plane, when that is behind the camera); for a matrix camera the point of
the near plane. A hit counts only between the near and the far plane (`0 <= z <= w` in the clip
space, with a tolerance of a millionth of `w`) and inside the viewport, as only that is drawn.
Without a camera, with an empty viewport or matrices without an inverse nothing is hit.

**The walk** follows the scene objects, not the records of the snapshot (which gives only the
camera): the filter is asked of visuals, a model whose material draws nothing is still hit, and
the positions are those of the mesh in double precision. The visuals in tree order, the filter asked of each before it is tested
(`ContinueSkipSelfAndChildren`, `ContinueSkipChildren`, `ContinueSkipSelf`, `Continue`, `Stop`:
the walk ends, the hits found so far are reported); the content of a `ModelVisual3D` (its models,
groups flattened) and then its children; the depth limit of the snapshot compiler (256). The ray
of `hit_test_ray` is in the coordinates of the parent of the visual (its own transform applies).
Each `GeometryModel3D` whose geometry is a `MeshGeometry3D` is tested in the coordinates of its
mesh: the ray through the inverse of its world matrix (the transforms of the model, its groups and
its visuals); a world matrix without an inverse covers nothing. The hits are sorted by their
distance (stable: tree order at equal distances) and handed to the result callback until it
returns `Stop`. A ray through an edge or a vertex shared by triangles hits each of them.

**The sides** (WPF): a triangle is hit on its front (counterclockwise toward the origin of the
ray) when the model has a `Material`, on its back when it has a `BackMaterial`, also a material
that draws nothing (no brush). A transform that mirrors turns the faces over, as on the screen.

**The hierarchy.** Built when a mesh is first hit and kept by the `MeshGeometry3D` until its next
version (`(id, version)`, shared by every model of the mesh): the positions in double precision
and the triangles of its `MeshData` (a triangle with an index out of range is left out, as
rendering leaves it out), split at the median of the centres along the longest axis, at most
four triangles a leaf, one vector of nodes. The traversal visits every box the ray enters (all
hits are reported), with a slab test that keeps a ray in the plane of a face of a box. The
triangle test is Moller-Trumbore in double precision: a triangle parallel to the ray or without
area (the determinant below 1e-12 of the product of the lengths of its edges and the direction)
or with a coordinate that is not a number is not hit. `PointHit` is the sum of the positions
times the weights (`VertexWeight1` is `1 - u - v`, `VertexWeight2` `u`, `VertexWeight3` `v`), in
the coordinates of the mesh; `DistanceToRayOrigin` is the distance from the origin to the point
transformed back by the world matrix, in the coordinates of the ray.

**Tests** (`hit_testing/tests.rs`): a known triangle with its weights, point and distance (also a
slanted ray and a longer direction); the sides with a `Material`, a `BackMaterial`, both, none,
and a material without a brush; nested transforms of a model, a group and two visuals, a rotation
and a mirror; the nearest first and all hits in order through stacked squares and a cube, and a
stop after the first; misses (beside, behind, in the plane, a zero direction, no geometry, a
flattened model); degenerate triangles (on a line, a repeated vertex, an index out of range, an
incomplete triangle, a position that is not a number); each behaviour of the filter; the rays of
perspective, orthographic and matrix cameras (a matrix camera with a perspective camera's
matrices hits the same points), the planes, the viewport bounds, a `Viewport3D`; the hierarchy
built once per version and shared by two models; a mesh of 101,250 triangles hit after at most 32
triangle tests; and picks held to the CPU reference: a turned cube under a perspective camera in
a wide viewport, and an orthographic scene of overlapping squares, a square turned away and one
drawn on its back, rasterized with one sample at the centre of each pixel. Every pixel whose
neighbours agree is hit when it is drawn and missed when it is not, and its colour names the
model hit.

### 3.17 Interactive 3D as built (D11, the model part)

**Classes** (`src/FerroUI.Media3D/ui_element_3d.rs`): `UIElement3D` (a `Visual3D`, WPF's base of
the 3D elements with input; `IsHitTestVisible`, `is_pointer_over`, `is_pointer_captured`,
`capture_pointer`, `release_pointer_capture` and the pointer events), `ModelUIElement3D` (its
`Model`, the content property of markup) and `ContainerUIElement3D` (its `Children`). Both draw as
a `ModelVisual3D` with the same content: the snapshot compiler, `descendant_bounds` and the hit
testing read what a visual shows through one function of `visual_3d.rs`, and the changes are of
the structure (a model, a child) or of a record (a transform), as for a `ModelVisual3D`. The
three classes are registered for markup (`<ContainerUIElement3D>`, `<ModelUIElement3D>` with its
model as content, `IsHitTestVisible`).

**Events.** `pointer_entered`, `pointer_exited`, `pointer_moved`, `pointer_pressed`,
`pointer_released` and `pointer_capture_lost` take a handler `Fn(&Ref<UIElement3D>,
&Pointer3DEventArgs)` and return a `Subscription`. `Pointer3DEventArgs` carries the source (the
innermost element of the route), the `Viewport3D`, the pointer, the 2D event of the viewport (its
point, buttons and modifiers), the position in the viewport's coordinates, the nearest hit under
the pointer and the handled flag. WPF's 3D elements raise the routed mouse events of 2D; the 3D
elements of the port are not `InputElement`s (a `Visual3D` is not a 2D visual), so their events
are lists of handlers that the viewport raises in WPF's order (recorded in DEVIATIONS.md).

**Routing** (`controls/viewport_3d_input.rs`). The 2D hit testing hits a `Viewport3D` only where
one of its models is (its `custom_hit_test` is the 3D hit test), so the input system gives it the
pointer there, as WPF's `Viewport3DVisual` is hit. For each 2D pointer event of the viewport (its
`InputElementImpl` overrides): the nearest hit at the pointer's position (`IsHitTestVisible`
false lets the pointer through an element and its descendants), the route of the hit (the
`UIElement3D`s from the visual hit to the root of the scene, innermost first); the elements the
pointer is over become that route (exited on the elements it left, innermost first, then entered
on the new ones, outermost first, each on the element alone); a move, press or release is raised
on the route, innermost first, until a handler handles it, and then the 2D event is handled too.
A hit in a plain `ModelVisual3D` outside any element is in front of what is behind it and reaches
no element. The pointer leaving the viewport leaves every element.

**Capture.** `capture_pointer` takes the 2D capture of the pointer for the viewport (keeping an
implicit capture of a press) and routes the pointer's events to the element and the elements
that hold it, wherever the pointer is; the element is then the only one the pointer can be over,
and only while the nearest hit is in it. Another element's capture takes it over (capture lost
on the first); `release_pointer_capture` raises capture lost and releases the 2D capture; the
end of the 2D capture of the viewport (the release of the button ends the implicit capture, as
the input system does) ends the 3D capture with capture lost. An element captures through the
viewport that last routed input to it; before any, `capture_pointer` returns false.

**Not built: `Viewport2DVisual3D`.** Its rendering (a 2D visual as the material of a mesh) needs
the materials from visuals of step D6, which is not done; its input (the texture coordinate of
the hit mapped to a point of the hosted visual, and the input routed into it through a
transformed hit-test path) is left with it. Keyboard focus of 3D elements is not part of this
step either.

**Tests** (`ui_element_3d_tests.rs`, in a window of the unit test application with the 2D
pointer events raised on the viewport as the input system raises them, with the implicit capture
of a press): the 2D hit test over models only; a press reaching the element under the pointer and
bubbling to its container, with the hit and the position; a handled press stopping there and
handling the 2D event; entered and exited following the pointer across two elements, the gap and
the edge of the viewport; a capture keeping the events on the capturing element over the other
one, ended by the release, released explicitly, and taken over by another element; a plain visual
in front taking no input and hiding what is behind, an element that is not hit-test visible
letting the pointer through; the elements' models drawn, their changes of the structure and the
records; and the classes loaded from XAML (`xaml_tests.rs`).

### 3.18 Materials from brushes as built (D6)

**The description** (`src/FerroUI.Rendering3D/brush.rs`). The snapshot keeps the brush of a
layer in its shared form (`SharedBrush`: the values of a solid colour, a gradient with its stops,
an image brush with its tile settings and a counted reference to its bitmap). What a draw samples
is made of it on the render thread, the same for the CPU reference and the executors:
`MaterialBrush::of(brush, texture_bounds)` gives the variant of the shader (`BrushShader`:
`Solid`, `Gradient`, `GradientTexture`, `Image`), the uniforms (`BrushUniforms`) and, where the
brush samples a texture, its source (`BrushTextureSource`) with the key of the texture
(`BrushTextureKey`).

| Brush | Sampled as |
|---|---|
| `SolidColorBrush` | the colour of the draw uniforms, as in D3 (`BrushShader::Solid`) |
| `LinearGradientBrush`, `RadialGradientBrush` with up to 8 stops | the stops inline in the uniforms (`BrushShader::Gradient`) |
| the same with more stops | a lookup texture of 256 by 1 straight RGBA8 texels, texel `i` the colour at `i / 255`, sampled with linear filtering at the centre of texel `t * 255` (`BrushShader::GradientTexture`); made once per set of stops (`BrushTextureKey::Gradient`, a hash of the sorted offsets and colours) |
| `ImageBrush` | the pixels of its bitmap, premultiplied RGBA8 (8-bit RGBA or BGRA, premultiplied, straight or opaque, read through the bitmap's framebuffer), sampled bilinearly with the edges clamped and divided by the alpha (`BrushShader::Image`); read once per `(identity, version)` of the bitmap (`BrushTextureKey::Image`) |
| an `ImageBrush` without an image | nothing (transparent), as in 2D |
| a conic gradient, an image whose pixels cannot be read, a visual or drawing brush (until the third part of D6) | the layer's `Color` with the brush's opacity, counted (`layers_with_unsampled_brushes`) and reported once |

**The mapping** is WPF's: the texture coordinate of a point is a point of the brush's 2D space,
and a brush relative to its bounding box (the default of `RelativePoint`, `RelativeRect` and
`RelativeScalar`) is mapped onto the bounding box of the mesh's texture coordinates
(`texture_coordinate_bounds`; a mesh without texture coordinates has the empty box at the origin);
an absolute brush takes the texture coordinates as they are. The brush's `RelativeTransform`
(conjugated into the box) and `Transform` (about its `TransformOrigin`) apply in that space as
they do over the bounds of a 2D element (the composition of Skia's brushes); the uniforms hold
their inverse, the texture coordinate to the brush's space, as two rows.

**Gradients.** A linear gradient's parameter is the projection onto its axis; a radial gradient's
is, in the unit circle of its ellipse, the distance from the focus (`GradientOrigin`) over the
distance from the focus to the circle along the same ray (the focus is moved inside the circle
when it is outside). The spread method (`Pad`, `Reflect`, `Repeat`) applies to the parameter;
the stops, sorted by offset (stable), are interpolated as straight colours in WPF's space (the
gamma-encoded components).

**Image brushes.** The tile is the brush's `DestinationRect` (WPF's `Viewport`) in the brush's
space; the `SourceRect` (WPF's `Viewbox`) of the bitmap (in device-independent pixels, its pixels
at 96 DPI) is stretched (`Stretch`) and aligned (`AlignmentX`, `AlignmentY`) into it as a 2D tile
brush places it (`MediaExtensions::calculate_scaling`, `TileBrushCalculator::calculate_translate`);
outside the viewbox the tile is transparent, outside the tile of `TileMode::None` the brush is;
the other tile modes repeat the tile, `FlipX`, `FlipY` and `FlipXY` mirroring its odd columns,
rows or both.

**Colour.** A sample is a straight colour that multiplies the colour of the draw (the material's
`Color` and the brush's opacity), which `lighting::shade_color` lights: a diffuse layer lit, a
specular layer times its highlight, an emissive layer added without light. The specular and
emissive layers of a `MaterialGroup` paint with their brushes like the diffuse layers.

**The CPU reference** samples the brush per sample at the perspective-correct texture coordinate
(the attributes of D3) and keeps the texels of each `BrushTextureKey` while it draws them
(`cpu::brush_textures_read` counts the reads), with a weak handle to the bitmap so that its
address names one bitmap while the texels are kept. The output caches of the backends also key on
the identities and versions of the bitmaps of a scene (`brush::scene_texture_inputs`): new pixels
of a bitmap change no generation of the snapshot.

**The shader** (`scene3d.wgsl`) has the same functions (`brush_point`, `spread`, `gradient_t`,
`inline_gradient`, `gradient_texture_coordinate`, `image_coordinate`, `unpremultiply`,
`shade_color`) in the same order of operations, and one fragment entry point per variant of the
brush shader: `fs_main` (solid colours, unchanged), `fs_gradient`, `fs_gradient_texture` and
`fs_image`. Within the limits of section 3.4: the brush uniforms are a second uniform block of
group 1 (`@binding(1)`, 256 bytes, at a dynamic offset of its own), the texture of a brush and one
sampler are group 2, the inline stops a loop of at most 8, and textures are read with
`textureSampleLevel` at level 0 (no derivatives, `textureLod` in GLSL ES 3.00). The tests of
`shader.rs` validate the seven entry points, translate the new ones to GLSL ES 3.00 for WebGL2
(three uniform blocks; one texture for the lookup and the image) and check the layout of
`BrushUniforms` as naga lays it out.

**The wgpu executor** (`wgpu_executor.rs`):

- **Pipelines** are keyed by `Scene3DPipelineKey` and the `BrushShader` of the draw (the fragment
  entry point, and the pipeline layout: with the group of the brush texture for
  `GradientTexture` and `Image`), made on first demand and kept per device: at most 32 per sample
  count, and the resolve.
- **Uniforms.** The `BrushUniforms` of the draws are in a third buffer of the viewport, at the
  stride of the device's offset alignment (256 bytes), bound with the draw uniforms in one bind
  group with two dynamic offsets, and written like them only where they changed: a new stop colour
  writes the 256 bytes of its draw.
- **Textures** of brushes are uploaded once per `BrushTextureKey` and device (RGBA8, no mipmaps,
  one linear sampler with the edges clamped), with the texels the CPU reference reads; a new version
  of a bitmap is written into the texture of the version it replaces when it has the same size (no
  texture made). A texture whose texels cannot be read, or that is larger than the device takes
  (`max_texture_dimension_2d`), is refused, not asked again for 240 renders, and its layer drawn
  with its colour and counted. Textures not drawn for 240 renders, and those of bitmaps that are
  gone, go. `wgpu_executor::texture_counters()` counts the uploads, their bytes and the refusals.
- **Vello** (`gpu/scene3d_passes.rs`) also keys a kept output on `brush::scene_texture_inputs`, so
  new pixels of a bitmap draw the scene again (one 3D pass that uploads the new pixels alone), and
  a render that uploaded a texture is not counted as uniform-only.

**Visual and drawing brushes** (the third part of D6). The model side is built; the renderers'
offscreen 2D renders are hooks the backends fill (below, "Left").

- **The description.** A `VisualBrush` (the rendering of its visual) or a `DrawingBrush` (its
  drawing) of a material is recorded on the UI thread into a `Scene3DBrushContent`
  (`ferroui_base::rendering::scene3d`): the operations of the immediate recording as they are, and
  each resource in a form that crosses threads (brushes and pens by their shared values,
  geometries by their platform geometries, glyph runs, bitmaps, effects, custom operations and 3D
  scenes as their shared handles). It has an identity and a generation, the bounds of the content,
  and `render(context, transform)` replays it into any drawing context, on any thread. The layer
  holds it as `SharedBrush::from_scene_content` (the tile settings, opacity and transforms of the
  brush over the content; `SharedBrush::scene_content` reads it). A nested visual or drawing brush
  inside the content has no shared form and draws without its brush.
- **Dirty tracking.** The material keeps the recording of its brush and records it again only when
  it is dirty: a change of a property of the brush (its invalidation) or of what it draws (the
  drawing of a `DrawingBrush`, followed through `ISceneBrush::subscribe_content_changed`) marks it
  dirty and is a change of a record; the next snapshot records a new generation of the same
  identity, and every other snapshot (a moving camera, another record) takes the kept recording.
  A new brush is a new identity. The generations of the snapshot (section 3.13) are unchanged by
  this: the content's own generation names what changed, as a bitmap's version does.
- **Sampling.** `MaterialBrush::of` samples a content as an image (`BrushShader::Image`) of the size
  of its bounds, with the brush's tile settings (`BrushTextureKey::Content { id, generation }`);
  `brush::scene_texture_inputs` includes the identities and generations of the contents, so a kept
  output is drawn again only for a new generation.
- **The CPU reference** renders a content through a `cpu::ContentRenderer` it is given
  (`CpuRasterizer::set_content_renderer`, `Scene3DOutputCache::set_content_renderer`): the
  backend's 2D renderer paints the content's bounds stretched over premultiplied RGBA8 texels of
  `BrushTextureSource::content_pixel_size(scale)` pixels (the bounds at the render's pixels per
  unit, at most 4096 a side). It is asked once per identity, generation and size. Without a
  renderer the layer is drawn with its colour and counted.
- **The wgpu executor** refuses a content's texture (its texels need the 2D renderer of the
  device, below): the layer is drawn with its colour and counted (`textures_refused`).

Left for the backends: (1) Skia raster and the CPU mode of Vello set a `ContentRenderer` on their
output caches that makes a raster target of the given size, scales the content's bounds onto it
and calls `Scene3DBrushContent::render`, returning its premultiplied pixels; (2) Vello's hybrid and
GPU modes render the content into a texture of the device pool (`effect_passes::pooled_texture`)
with a Vello scene of the device (the route of the effect passes' device layers), keep it by
`BrushTextureKey::Content` with the output's slot, and hand it to the executor as the texture of
the key (an executor entry point that takes a texture view for a key in place of texels); (3) Skia
Graphite and Ganesh draw the content into a GPU surface of their context and import it into their
executors (D5, D8); (4) a change inside the visual of a `VisualBrush` (a property of an element of
its subtree) does not invalidate the brush; following it needs the visual's render invalidation
(the 2D compositor finds it through its dirty visuals).

**Tests.** `brush_tests.rs`: the layout of `BrushUniforms` and its WGSL declaration, relative and
absolute linear gradients, the spread methods, radial gradients with a moved focus, sorted stops
with straight colours, the lookup texture (its ends, within the rounding of its texels, one key per
set of stops), the relative transform and the transform, image brushes (fill, viewbox, uniform
stretch, tiles, `FlipX`), an image brush without an image, the reading of BGRA straight pixels, the
key of a bitmap and the inputs of a scene following its version, the box of the texture
coordinates. `cpu/tests.rs`: a diffuse gradient with 2 and 12 stops over a quad, the specular and
emissive layers of a group painting with a gradient and an image, the texels read once while kept
and again for a new version. The comparison scenes (`scene3d_scenes.rs`): `linear-gradient`,
`radial-gradient`, `gradient-lookup-absolute` (12 stops through the lookup texture, absolute
mapping over half of the gradient), `image-brush-tiles` (`FlipX` tiles of half the box),
`image-brush-viewbox` (the bottom half of the bitmap, `Uniform` into a box of 1 by 2.1, so that no
edge of the viewbox is on the centre of a row of pixels, where the GPU and the CPU reference round
the interpolated texture coordinates apart)
and `brush-material-group` (a grey diffuse layer, a specular layer with a gradient and an emissive
layer with an image), with probes computed from the formulas in double precision, held to the CPU
reference in every backend and mode (the GPU modes within the interior levels of section 3.15).
Visual and drawing brushes: `brush_content.rs` (a drawing brush recorded and replayed on another
thread, its shared form, the changes of its drawing reported), `scene_tests.rs` of
`ferroui-media3d` (a drawing brush recorded again only when its content changes: the same
identity and generation across a camera change, a new generation after a change of its drawing, a
new identity for a new brush) and `cpu/tests.rs` (a content rendered once per generation at the
resolution of the render and sampled as an image; without a renderer, the colour).
The steady state and compile once (`scene3d_tests.rs`):
`steady_frames_of_a_moving_camera_over_brushes_upload_no_texture`,
`a_static_viewport_of_brushes_issues_no_3d_pass_and_uploads_nothing`,
`a_new_version_of_a_bitmap_uploads_only_its_texture` and `pipelines_are_created_once_per_device`
with the four brush shaders; the unit tests of `wgpu_executor.rs`: the brush stride, a texture
uploaded once per key and device and a new version into the same texture, a change of a brush
writing only the uniforms of its draw.

## 4. Steps

Each step is one pull request from main. Each updates the row "3D scenes (Viewport3D)" of
[gpu-rendering.md](gpu-rendering.md), [DEVIATIONS.md](DEVIATIONS.md) where the port differs, and
this page.

| Step | Content | Public API | Risk | Status |
|---|---|---|---|---|
| D0 | #608; this page; section 8 of the media specification; the row of gpu-rendering.md | none | none | in review |
| D1 | The values (section 3.12) and their markup converters, with tests: inversion and determinant against hand values, append and prepend, `RotateAt`, axis and angle round trips, slerp, `Rect3D` union and intersection with `Empty`, points against vectors, the projective `Point4D`, the text forms | additions (a new crate) | low | in review |
| D2 | The scene model (section 3.13): transforms, rotations, cameras, lights, materials, `Geometry3D` and `MeshGeometry3D` with the collections, models, `Visual3D`, `ModelVisual3D`, `Viewport3DVisual`, the control `Viewport3D`; `Scene3DSnapshot` with the fitted planes and the generations; the render-data operation `DrawScene3D` and the capability `IDrawingContextImplWithScene3D`; XAML registration and the namespace mapping; the facade feature `media3d`. Tests: the snapshot of nested `Model3DGroup`s with transforms, a camera change bumping only `camera_generation`, a mesh edit bumping the structure, a light colour bumping the records, the fitted planes, `Bounds`, the text forms of the collections, a cube scene loaded from XAML, the operation recorded, sent and replayed with and without the capability | additions | medium: the namespace clash of `Vector3D`; the shape of the snapshot fixes every executor's input | in review |
| D3 | The CPU reference rasterizer (`src/FerroUI.Rendering3D/cpu`) and 11 comparison scenes in `tests/FerroUI.RenderBackends.Comparison` (`scene3d_scenes.rs`, `scene3d_tests.rs`; 100 pixels, scales 1 and 2): a lit cube (ambient and directional), a point light with attenuation and range, a spot light cone, the orthographic camera, the matrix camera, front and back materials with culling, `MaterialGroup` with specular and emissive, transparent overlap in tree order, near and far clipping, `ClipToBounds` with opacity and a rotated viewport, the empty scene. Every backend draws the CPU reference from here, recorded as a gap | additions | medium | in review |
| D4 | `scene3d.wgsl` and the wgpu executor (section 3.15): `ferroui-render3d`'s `shader` module and feature `wgpu` (the pipeline cache by a `Copy` key, the mesh cache by `(id, version)`, uniform buffers with dynamic offsets written only where they changed, the depth, MSAA and resolve pools, the fallback to one sample), and Vello's hybrid and GPU modes drawing on the device (`gpu/scene3d_passes.rs`: the outputs kept per device, painted as textures of the device, the CPU reference as the reported and counted last resort). Tests: `every_scene_3d_matches_its_cpu_reference_in_every_backend_and_mode` and `the_backends_draw_every_scene_3d_alike` with the GPU modes, `steady_frames_of_a_moving_camera_upload_only_uniforms_and_allocate_no_texture`, `a_static_viewport_issues_no_3d_pass`, `pipelines_are_created_once_per_device`, the naga validation and GLSL ES 3.00 of the shader; run by `vello.yml` on Metal (its paths now include `src/FerroUI.Rendering3D/`), Vulkan with D9 | additions (the module `wgpu_executor`, `ferroui_vello::gpu::scene3d_counters`) | high: the first GPU route; the uniform layout and alignment (dynamic offsets at the device's alignment) | in review |
| D5 | Skia Graphite on Metal: the Metal executor (MSL from naga at build time) in or next to `ferroui-metal`, `ferroui-skia/gpu/scene3d.rs`, the output wrapped by `backend_textures::make_metal` and `wrap_texture`, ordered before `insert_recording`, the prewarm hook. Tests: `graphite_draws_every_scene_3d_as_its_cpu_reference` (`gpu/metal/tests.rs`), the steady state, compile once; iOS simulator and device measurements | none new | high: cross-API ordering on Apple | planned |
| D6 | Materials from brushes (section 3.18): gradients (inline stops and the lookup texture), `ImageBrush` (the backend image caches), `VisualBrush` and `DrawingBrush` (offscreen renders, again only when dirty), WPF's texture-coordinate mapping, the specular and emissive passes. Tests: a comparison scene per brush, an image uploaded once per device, a visual brush rendered again only when its visual changes | additions (the module `brush`, `cpu::brush_textures_read`, `lighting::shade_color`) | medium | in review, in three parts: the description and the CPU reference (part 1); the shader and the wgpu executor (part 2: `brush_uniforms_stride`, `texture_counters`, `Scene3DTextureCounters`, `Scene3DWgpuExecutor::texture_count`, the entry points of `shader`); visual and drawing brushes (part 3: the model side and the CPU reference's hook, `Scene3DBrushContent`, `SharedBrush::from_scene_content` and `scene_content`, `ISceneBrush::subscribe_content_changed`, `cpu::ContentRenderer`, `set_content_renderer`; the backends' offscreen renders left, section 3.18) |
| D7 | Video and shared-texture materials, Metal routes first: `ISharedGpuTexture` and `GpuTextureRef` in `src/FerroUI.Base/platform/`, the accessors of `LeasedGpuBitmap`, `VelloSharedFrameBitmap` and `SharedFrameTexture`, the import cache by `FrameIdentity` that `TexturePresenter` also uses, `VideoBrush` and the visual-brush fast path, `sample_frame()`. Tests: `a_video_material_samples_the_frame_without_a_copy` (`FrameTransferCounters`: `gpu_import` only on Graphite, one shared `gpu_conversion` on Vello), one import for a frame shown flat and on a cube, the lease released after GPU completion, no mesh or record upload for a new frame; a "video on a cube" scene in `samples/PortableMedia` | additions | high: lifetimes across 2D and 3D | in review, part 1 (Vello: frames sampled from the device); the rest planned |
| D8 | The GL executor (Skia Ganesh GL everywhere: ANGLE, EGL, Android GLES, CGL, WebGL2): GLSL ES 3.00 from naga at build time, framebuffer, depth renderbuffer and MSAA resolve, the reset of Skia's GL state after drawing, video through `SharedFrameTexture` ids on the same context. Tests: the comparison scenes under ANGLE on WARP (Windows CI), llvmpipe EGL (Linux CI), the Android emulator, a browser smoke run; browser and Android frame times | none new | high: Skia's GL state | planned |
| D9 | Vello on Vulkan, DX12 and GLES, validated in the Windows and Linux virtual machines (one at a time); the Vulkan executor (SPIR-V) for Skia Vulkan with lavapipe in CI. The video gaps of those routes stay recorded gaps | none new | medium | planned |
| D10 | Hit testing (section 3.16, `src/FerroUI.Media3D/hit_testing/`): the ray from the camera with the fitted planes of the snapshot, the hierarchy per mesh by `(id, version)`, Moller-Trumbore with weights and distance, WPF's front and back material rules, the transforms of the tree, `RayMeshGeometry3DHitTestResult` and its base types, `VisualTreeHelper::hit_test`, `hit_test_point` and `hit_test_ray` with the filter and result callbacks. Tests: hits on known triangles with weights and distance, the front and back material rules, nested transforms, the order of the hits, misses, degenerate triangles, the filters, the cameras, picks that match the rendered pixels of the CPU reference, a 100k-triangle mesh | additions (`VisualTreeHelper`, `RayHitTestParameters`, the result types and behaviours, `Viewport3DHitTestTarget`) | medium | in review |
| D11 | Interactive 3D (section 3.17): `UIElement3D`, `ModelUIElement3D` and `ContainerUIElement3D` with their pointer events (entered, exited, moved, pressed, released, capture lost) routed from a `Viewport3D` through the D10 hit test, pointer over and capture; the 2D hit test of `Viewport3D` over its models. Left for after D6: `Viewport2DVisual3D` rendering (the D6 path) and input routing (pointer events reaching a `Button` on a rotated `Viewport2DVisual3D` plane), and focus | additions (the three classes, `Pointer3DEventArgs`) | high: a transformed hit-test path in the input system (for `Viewport2DVisual3D`) | in review (the model part); `Viewport2DVisual3D` planned after D6 |
| D12 | Catalog pages, one per control, in a "3D" section (a `Viewport3D` page with cameras, lights, materials, transforms and an animated `RotateTransform3D`; a `Viewport2DVisual3D` page) and a "video material" page in the Media section (docs/media/CONTROL-CATALOG.md); a `samples/Media3D` application with procedural meshes, run on device and simulator | none (samples) | low | planned |
| D13 | Later: Vello hybrid-webgl 3D after route step 3; the ProGPU backend mapping (#369); browser WebGPU; Vello's perspective gap C7 through the 3D executor; an inventory of WinUI's `Microsoft.UI.Composition.Scenes` in winui-composition.md | to be stated | medium | planned |

## 5. Not in scope

- ProGPU's CAD extras (decision E): shading modes (`ShadingMode3D`), render modes and wireframe
  (`RenderMode3D`), edges (`Mesh3DEdgeStyle`), the orbit, pan and zoom controller, the coordinate
  compass and the selection gestures. A separate opt-in crate could add them later.
- Model and asset formats (OBJ loading, glTF), a scene editor, physics and an animation system for
  scene assets.
- WinUI's composition scenes (`SceneVisual`, `SceneNode`, `SceneMesh`, `SceneMetallicRoughnessMaterial`):
  they could map onto the same core later; D13 only inventories them.
- Order-independent transparency and transparency sorting, which WPF does not do either.
- New frame routes for video: the 3D path consumes the routes of the media framework (section
  3.11).
- Replacing `RendererHost`: the host of a client-owned renderer (the media specification) stays the
  escape hatch next to `Viewport3D`.

## 6. Decisions

### 6.1 Defaults taken

Taken with #608, open to change by the owner.

| # | Question | Decision | Why |
|---|---|---|---|
| A | Lighting: per pixel (ProGPU) or per vertex (WPF) | **Per pixel**, recorded in DEVIATIONS.md when the GPU route lands (D4); a per-vertex variant as a bit of the pipeline key if exact parity is needed | Better highlights under point and spot lights at the same GPU cost; per vertex stays available for parity tests |
| B | Executors: native ones under Skia and wgpu under Vello, or wgpu around Skia's devices | **Native executors under Skia** (Metal, GL, Vulkan), **wgpu under Vello** | Skia builds keep their size and link no wgpu; wgpu cannot run on the ANGLE and WebGL2 contexts Skia owns; no cross-API synchronization beyond the one Apple already has |
| C | Public numeric types | **f64 at the API**, narrowed to f32 when the snapshot is built; the f64 `Quaternion` and `Vector3D` of `ferroui-media3d` apart from the f32 numerics and the base library's composition `Vector3D` | WPF's values are doubles and its tests and text forms depend on it; the GPU takes f32 anyway |
| D | Video on 3D | **Both**: a FerroUI `VideoBrush` over a `FrameMailbox`, and a fast path for a `VisualBrush` over a `MediaPlayerElement` or `CameraPreview` | WPF code keeps working and gets the zero-copy path; new code names the source directly |
| E | ProGPU's CAD extras | **Out of scope** for the WPF port | They are not WPF; a separate crate can add them |
| F | Default MSAA | **4x where supported**, 1x for `EdgeMode` `Aliased` | Edge quality equal at every scale; ProGPU's scale heuristic stays an option |
| G | Where the snapshot types live (D2) | **The base library** (`ferroui_base::rendering::scene3d`), the compiler in `ferroui-media3d` | The render-data operation is in the base library and the backends depend on it, not on this crate; the snapshot is plain data |
| H | The clash of `Vector3D` in the XML namespace (D2) | **Both namespaces mapped; the first mapping (the base library's) wins for a name written in markup**; properties read their declared type | Attribute values, the common case, need no rule; nothing is renamed and existing markup keeps its meaning; the 3D vector is named with a `using:` prefix where an element is needed |
| I | `Viewport3DVisual` (D2) | **The scene object of the viewport, not a 2D visual**; `Viewport3D` hosts one and records its snapshot | A 2D visual without layout would need its own bounds and arrangement for nothing; the render-data operation is written by the control |
| J | The fitted planes (D2) | **Only for an infinite `FarPlaneDistance`**, both planes fitted to the scene with a margin, never closer than `NearPlaneDistance` | What the ticket asks; a finite far plane is the application's choice; to check against WPF (section 6.2) |
| K | Generations (D2) | **One kind per change**; a structure change does not bump the records | An executor compares all three; a new structure rewrites the records anyway |
| L | More than 16 lights (D3) | **The first 16 lights in tree order** (ambient lights included, as ProGPU's loop), the others left out, counted (`ferroui_render3d::lights_left_out`) and reported once; the GPU executors take the same 16 | One pass, the uniform block of every target; a second pass for the further lights stays possible if WPF parity needs it |
| M | How the backends link the CPU reference (D3) | **A feature `media3d` of `ferroui-skia` and `ferroui-vello`**, turned on by the facade's `media3d` | An application without 3D links no rasterizer (the size rules); without the feature a scene is counted as not drawn, as in D2 |
| N | Added light over a transparent pixel (D3) | **Specular and emissive layers add their colour with alpha 0; the resolve raises the alpha of a pixel to its largest colour channel** | The output stays a valid premultiplied image for the 2D compositor; over the viewport's own opaque geometry (the common case) nothing changes |

What a viewport with more lights than the GPU takes (16) draws was open for D4; D3 settles it
(decision L) so that the CPU reference and the executors agree.

### 6.2 Fidelity points to verify against WPF

To be checked against WPF's documentation and behaviour before the tests freeze them:

- `PerspectiveCamera.FieldOfView` is the horizontal angle (ProGPU treats it as vertical); the
  defaults of the cameras (`Position` (0, 0, 0), `LookDirection` (0, 0, -1), `UpDirection`
  (0, 1, 0), `NearPlaneDistance` 0.125, `FarPlaneDistance` infinity).
- The fitting of the near and far planes to the scene with an infinite far plane.
- Lighting per vertex in WPF, and the colour space of the lighting math (gamma-encoded sRGB without
  linearization).
- The `SpotLight` angle convention (full cone angles) and the falloff between the cones.
- The order and blending of the passes of a `MaterialGroup` (diffuse, specular, emissive).
- The mapping of a relative brush onto the bounding box of the texture coordinates.
- What WPF draws for a non-solid brush on a mesh without texture coordinates (the port takes the
  colour at the brush's origin), and for a radial gradient whose origin is outside its ellipse.
- Whether WPF interpolates the stops of a material's gradient straight or premultiplied (the port:
  straight, as its 2D backends).
- The hit-testing rules for front and back faces (`Material` and `BackMaterial`), also under a
  transform that mirrors (D10 turns the faces over, as the screen does); the origin of the ray of
  an orthographic and a matrix camera, which `DistanceToRayOrigin` is measured from.
- No sorting of transparent models.
- `Viewport3D.ClipToBounds` true by default.
- Whether `RenderOptions.EdgeMode` affects 3D antialiasing.
- The values (D1): the infinity handling of `Rect3D.Union` (WPF's 3D union has none, unlike its 2D
  `Rect`), and `NaN` inputs to `Rect3D` and `Vector3D.Normalize` (Rust's `f64::max` ignores `NaN`
  where .NET's `Math.Max` returns it).
