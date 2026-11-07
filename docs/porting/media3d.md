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

ProGPU (`/Users/wieslawsoltes/GitHub/ProGPU`, read on `origin/main` at `23f686151`) has two 3D
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
  reference and counts it, the pattern of `draw_lighting` and `push_pixel_operation`.
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

**Lighting** (decision A). ProGPU's `ComputeWpfLighting`, per pixel: the sum of the ambient lights
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
  opaque grey; WPF's viewport is transparent). Depth `Depth24Plus` (wgpu) or `DEPTH_COMPONENT24`
  (GL), private to the pass.
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

## 4. Steps

Each step is one pull request from main. Each updates the row "3D scenes (Viewport3D)" of
[gpu-rendering.md](gpu-rendering.md), [DEVIATIONS.md](DEVIATIONS.md) where the port differs, and
this page.

| Step | Content | Public API | Risk | Status |
|---|---|---|---|---|
| D0 | #608; this page; section 8 of the media specification; the row of gpu-rendering.md | none | none | in review |
| D1 | The values (section 3.12) and their markup converters, with tests: inversion and determinant against hand values, append and prepend, `RotateAt`, axis and angle round trips, slerp, `Rect3D` union and intersection with `Empty`, points against vectors, the projective `Point4D`, the text forms | additions (a new crate) | low | in review |
| D2 | The scene model (section 3.13): transforms, rotations, cameras, lights, materials, `Geometry3D` and `MeshGeometry3D` with the collections, models, `Visual3D`, `ModelVisual3D`, `Viewport3DVisual`, the control `Viewport3D`; `Scene3DSnapshot` with the fitted planes and the generations; the render-data operation `DrawScene3D` and the capability `IDrawingContextImplWithScene3D`; XAML registration and the namespace mapping; the facade feature `media3d`. Tests: the snapshot of nested `Model3DGroup`s with transforms, a camera change bumping only `camera_generation`, a mesh edit bumping the structure, a light colour bumping the records, the fitted planes, `Bounds`, the text forms of the collections, a cube scene loaded from XAML, the operation recorded, sent and replayed with and without the capability | additions | medium: the namespace clash of `Vector3D`; the shape of the snapshot fixes every executor's input | in review |
| D3 | The CPU reference rasterizer (`src/FerroUI.Rendering3D/cpu`) and 11 comparison scenes in `tests/FerroUI.RenderBackends.Comparison` (`scene3d_scenes.rs`, `scene3d_tests.rs`; 100 pixels, scales 1 and 2): a lit cube (ambient and directional), a point light with attenuation and range, a spot light cone, the orthographic camera, the matrix camera, front and back materials with culling, `MaterialGroup` with specular and emissive, transparent overlap in tree order, near and far clipping, `ClipToBounds` with opacity and a rotated viewport, the empty scene. Every backend draws the CPU reference from here, recorded as a gap | additions | medium | planned |
| D4 | `scene3d.wgsl` and the wgpu executor in Vello (hybrid and GPU, Metal first): the pipeline cache, the mesh cache, the depth and MSAA pools, the composition by `paint_device_layer`, the reuse of the cached output; `IDrawingContextImplWithScene3D`. Tests: `every_scene_3d_matches_its_cpu_reference_in_every_backend_and_mode`, `the_backends_draw_every_scene_3d_alike`, `steady_frames_of_a_moving_camera_upload_only_uniforms_and_allocate_no_texture`, `a_static_viewport_issues_no_3d_pass`, `pipelines_are_created_once_per_device`, the naga validation of the shader; `vello.yml` on Metal, lavapipe in `ci.yml` | addition (a capability trait) | high: the first GPU route; the uniform layout and alignment (256-byte dynamic offsets) | planned |
| D5 | Skia Graphite on Metal: the Metal executor (MSL from naga at build time) in or next to `ferroui-metal`, `ferroui-skia/gpu/scene3d.rs`, the output wrapped by `backend_textures::make_metal` and `wrap_texture`, ordered before `insert_recording`, the prewarm hook. Tests: `graphite_draws_every_scene_3d_as_its_cpu_reference` (`gpu/metal/tests.rs`), the steady state, compile once; iOS simulator and device measurements | none new | high: cross-API ordering on Apple | planned |
| D6 | Materials from brushes: gradients (inline stops and the lookup texture), `ImageBrush` (the backend image caches), `VisualBrush` and `DrawingBrush` (offscreen renders, again only when dirty), WPF's texture-coordinate mapping, the specular and emissive passes. Tests: a comparison scene per brush, an image uploaded once per device, a visual brush rendered again only when its visual changes | none new | medium | planned |
| D7 | Video and shared-texture materials, Metal routes first: `ISharedGpuTexture` and `GpuTextureRef` in `src/FerroUI.Base/platform/`, the accessors of `LeasedGpuBitmap`, `VelloSharedFrameBitmap` and `SharedFrameTexture`, the import cache by `FrameIdentity` that `TexturePresenter` also uses, `VideoBrush` and the visual-brush fast path, `sample_frame()`. Tests: `a_video_material_samples_the_frame_without_a_copy` (`FrameTransferCounters`: `gpu_import` only on Graphite, one shared `gpu_conversion` on Vello), one import for a frame shown flat and on a cube, the lease released after GPU completion, no mesh or record upload for a new frame; a "video on a cube" scene in `samples/PortableMedia` | additions | high: lifetimes across 2D and 3D | planned |
| D8 | The GL executor (Skia Ganesh GL everywhere: ANGLE, EGL, Android GLES, CGL, WebGL2): GLSL ES 3.00 from naga at build time, framebuffer, depth renderbuffer and MSAA resolve, the reset of Skia's GL state after drawing, video through `SharedFrameTexture` ids on the same context. Tests: the comparison scenes under ANGLE on WARP (Windows CI), llvmpipe EGL (Linux CI), the Android emulator, a browser smoke run; browser and Android frame times | none new | high: Skia's GL state | planned |
| D9 | Vello on Vulkan, DX12 and GLES, validated in the Windows and Linux virtual machines (one at a time); the Vulkan executor (SPIR-V) for Skia Vulkan with lavapipe in CI. The video gaps of those routes stay recorded gaps | none new | medium | planned |
| D10 | Hit testing (`src/FerroUI.Media3D/hit_testing/`): the ray from the camera, the hierarchy per mesh, `RayMeshGeometry3DHitTestResult`, the `VisualTreeHelper::hit_test` overloads. Tests: hits on known triangles with weights and distance, the front and back material rules, a pick that matches the rendered pixel of the CPU reference, a 100k-triangle mesh. Can follow D2 in parallel | additions | medium | planned |
| D11 | Interactive 3D: `UIElement3D`, `ModelUIElement3D` and `ContainerUIElement3D` input events; `Viewport2DVisual3D` rendering (the D6 path) and input routing. Tests: pointer events reach a `Button` on a rotated `Viewport2DVisual3D` plane; capture and focus | additions | high: a transformed hit-test path in the input system | planned |
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

Open for D4: what a viewport with more lights than the GPU takes (16) draws. WPF has no fixed
count; whichever rule is chosen (for example a second pass for the further lights, or the lights
beyond the limit left out and reported once) is stated here and applied by the CPU reference too,
so the comparison tests hold.

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
- The hit-testing rules for front and back faces (`Material` and `BackMaterial`).
- No sorting of transparent models.
- `Viewport3D.ClipToBounds` true by default.
- Whether `RenderOptions.EdgeMode` affects 3D antialiasing.
- The values (D1): the infinity handling of `Rect3D.Union` (WPF's 3D union has none, unlike its 2D
  `Rect`), and `NaN` inputs to `Rect3D` and `Vector3D.Normalize` (Rust's `f64::max` ignores `NaN`
  where .NET's `Math.Max` returns it).
