# Custom effects (#455)

The plan for user-defined effects on `Visual.Effect`: the feature set of the owner's Effector,
built natively into FerroUI's effect model, compositor and backends, with a GPU implementation in
every backend. This page is the design and the order of the work; each step below is one pull
request from main.

The rules that bind it are those of [PORTING-GUIDE.md](PORTING-GUIDE.md), ground rule 6, and
[gpu-rendering.md](gpu-rendering.md): every rendering feature is drawn on the GPU by Skia (Ganesh,
Graphite) and Vello (GPU, hybrid, WebGL2), validated by tests; the CPU path is the reference and
the fallback of raster surfaces only; an effect is compiled once per backend into a cached GPU
program, and what animates changes its uniforms, never its pixels or textures.

## 1. What Effector provides

Effector (`/Volumes/1TB-macOS/upstream/Effector`, runtime in `src/Effector`) adds extensible,
Skia-backed effects to the .NET framework FerroUI was ported from. That framework has a closed set
of effects (blur, drop shadow) and no extension point, so Effector weaves IL into the consumer's
assemblies and patches the framework's own binaries after build (`src/Effector.Build.Tasks`,
`src/Effector.SelfWeaver`; the entry points it patches in are listed in
`plan/avalonia-custom-effects.md`, "Final Design", item 5). FerroUI needs none of that: the
extension points are ours to add. The features, with the files that implement them:

| Feature | What it does | Effector sources |
|---|---|---|
| Authoring | An effect is a class with ordinary styled properties, set as `Visual.Effect` (and `Border.Effect`) from markup or code; `AffectsRender` invalidates it | `SkiaEffectBase.cs`, `SkiaEffectAttribute.cs` (the factory type and an optional parse name) |
| Immutable snapshot | Before composition transport the effect is frozen into a generated immutable type that carries its values as an array, with equality, hash and cached padding; the render thread sees only that | `EffectorEffectDescriptor.cs` (`Freeze`, `CreateDefaultImmutable`), the weaver's `__EffectorImmutable` types, `plan/avalonia-custom-effects.md` item 7 |
| Typed filter effects | A factory turns the snapshot's values into an `SKImageFilter`, and gives the padding the effect needs; helpers build colour matrices and filter chains | `ISkiaEffectFactory.cs`, `ISkiaEffectValueFactory.cs` (`GetPadding(object[])`, `CreateFilter(object[], context)`), `SkiaEffectContext.cs` (opacity, blur radius to sigma), `SkiaFilterBuilder.cs`, `ColorMatrixBuilder.cs` |
| Runtime shader effects | SkSL compiled once per source (a cache), typed uniforms, child shaders (the content, images), a blend mode, a destination rectangle; the effect subtree is drawn into an offscreen surface and the shader pass drawn over it; a CPU fallback renderer where runtime effects are unavailable | `ISkiaShaderEffectFactory.cs`, `ISkiaShaderEffectValueFactory.cs`, `SkiaShaderEffect.cs`, `SkiaShaderEffectContext.cs`, `SkiaRuntimeShaderBuilder.cs`, `EffectorShaderEffectFrame.cs`, `EffectorShaderDebugInfo.cs` |
| Secondary images | An image (a page capture for a transition) is registered and referenced by a value handle that can live in a snapshot; the render thread takes a lease while it draws; the image is released when the handle is released and no lease remains | `SkiaShaderImageHandle.cs`, `SkiaShaderImageRegistry.cs`, `SkiaShaderImageLease.cs` |
| Interactive effects | Pointer entered, exited, moved, pressed, released, capture lost and wheel are routed to an effect attached to a visual, with a host context: the visual, its bounds, normalized coordinates, capture and release, invalidation | `ISkiaInputEffectHandler.cs`, `SkiaInteractiveEffectBase.cs`, `SkiaEffectHostContext.cs`, `SkiaEffectInputManager.cs` (observes `Visual.EffectProperty.Changed` once and subscribes to the host's events) |
| Parsing | `Effect.Parse("tint(color=#0F9D8E, strength=0.55)")` and the effect converter of markup: a named invocation, parameters matched ignoring case and `-`, values converted by the property type | `EffectorEffectValueSupport.cs` (`TryParseEffectInvocation`, `SplitTopLevel`, `ConvertFromInvariantString`), `EffectorEffectPropertyDescriptor.cs` |
| Transitions and animation | `EffectTransition` and key frames on `Visual.Effect` interpolate custom effects property by property (numbers, colours, points, vectors, sizes, rectangles, thickness); other types switch at half; a missing side is the default instance | `EffectorEffectValueSupport.cs` (`Interpolate`), `EffectorEffectDescriptor.cs` (`Interpolate`), `EffectorEffectTransitionObservable.cs` |
| Samples | Tint, pixelate, grayscale, sepia, saturation, brightness and contrast, invert, glow, sharpen, edge detect, scanline, grid and spotlight shaders, pointer spotlight, reactive grid, water ripple, burning flame (burn-away), and the compiz-style page transitions | `samples/Effector.Sample.Effects` (`ColorEffects.cs`, `ImageEffects.cs`, `ShaderEffects.cs`, `InteractiveShaderEffects.cs`), `samples/Effector.Compiz.Sample.Effects`, `samples/Effector.Sample.App`, `samples/Effector.Compiz.Sample.App` |

Two notes from Effector's own design that matter here: its runtime shaders are scoped to
procedural overlays drawn over a captured image of the content, because content-sampling shaders
were unstable on the headless CPU path (`plan/avalonia-custom-effects.md`, item 8 and the
paragraph after the list); and its shader pass draws the effect subtree into an offscreen surface
every frame. FerroUI keeps the content on the GPU (a layer, section 3.4) and treats the CPU path as
the reference, not the target.

## 2. How WinUI's effect factory works

WinUI (`/Users/wieslawsoltes/GitHub/microsoft-ui-xaml`) separates the description of an effect
from its compiled program and from the per-element values:

- **The description** is a graph of Direct2D effects known by GUID, each a COM object implementing
  `IGraphicsEffectD2D1Interop`: `GetEffectId`, `GetSourceCount`/`GetSource` (the inputs, effects
  or named `CompositionEffectSourceParameter`s), `GetPropertyCount`/`GetProperty` (the values), and
  `GetNamedPropertyMapping`, which maps a property name (`"Blur.BlurAmount"`) to the index of a
  Direct2D property and a mapping (`GRAPHICS_EFFECT_PROPERTY_MAPPING_DIRECT`, `VECTORX` to
  `VECTORW`, `COLOR_TO_VECTOR4`, `RADIANS_TO_DEGREES`, ...). The descriptions are in
  `controls/dev/Effects/microsoft.ui.private.composition.effects_impl.h` (the Win2D-like effects
  of the controls: `EffectBase`, the `DECLARE_NAMED_PROPERTY_MAPPING` tables) and
  `dxaml/xcp/core/hw/Effects.h` (the few the core renderer uses: `ColorSourceEffect`,
  `CompositeEffect`, `Transform2DEffect`).
- **The factory**, `Compositor::CreateEffectFactory(effect, animatableProperties)`, reads the
  description once and compiles it into a GPU program. The names in `animatableProperties` become
  parameters of that program instead of constants.
- **The brushes**, `CompositionEffectFactory::CreateBrush`, share the program; each has its own
  `Properties` (a property set, so an expression or key frame animation can drive a parameter) and
  its own source parameters (`SetSourceParameter`). Changing a parameter changes a uniform.
- **The users** create one factory per recipe and one brush per element:
  `controls/dev/Materials/Acrylic/AcrylicBrush.cpp` (factories for the material policies, then
  `CreateBrush`, `SetSourceParameter(L"Backdrop", ...)`, `Properties().InsertColor(TintColorColor,
  ...)`), `controls/dev/Materials/Reveal/RevealBrush.cpp` (`CreateEffectFactory(effect,
  animatedProperties)` and the `BaseColorEffect.Color` property),
  `dxaml/xcp/components/animation/ConnectedAnimation.cpp` (the cross-fade factory with the
  animatable `Crossfade.Weight` and two surface sources).
- **The render walk** (`dxaml/xcp/core/hw`) does not run effects itself: the XAML tree is
  mirrored into composition visuals, and the composition engine draws them on its own thread with
  the compiled programs. The UI thread only sends values.

FerroUI ported this for the WinUI composition API (`docs/porting/winui-composition.md`, phase 6 and
section 5.3): `CompositionEffectFactory` loads a `GraphicsEffect` description once into an
`EffectGraph` (`src/FerroUI.Base/rendering/composition/effects/effect_graph.rs`),
`ServerCompositionEffectBrush` paints it per frame with the animatable members read from its
property set, and the Skia backend draws each node with a cached GPU program
(`src/Skia/FerroUI.Skia/runtime_effects.rs`, `composition_effects.rs`). `effect_prewarm.rs` (#437)
draws registered graphs offscreen once, so their pipelines are compiled before a frame needs them.

## 3. Design for FerroUI

### 3.1 What exists

- **The effect model** (`src/FerroUI.Base/media/effects`): `IEffect` (the interface handle,
  `Rc<dyn IEffect>`), the mutable classes `BlurEffect`, `DropShadowEffect`,
  `DropShadowDirectionEffect` deriving from `Effect`, their immutable values
  `ImmutableBlurEffect`, `ImmutableDropShadowEffect`, `ImmutableDropShadowDirectionEffect`,
  `ImmutableLuminanceToAlphaEffect`, and `IImmutableEffect: IEffect + Send + Sync`, which compares
  structurally. `EffectExtensions::to_immutable` freezes an effect; `get_effect_output_padding`
  gives its padding; `Effect::parse` reads `blur(...)` and `drop-shadow(...)` and is the markup
  converter of `IEffect` (`markup_types/contracts.rs`). `EffectTransition` and `EffectAnimator`
  (`animation/transitions/effect_transition.rs`, `animation/animators/effect_animator.rs`)
  interpolate blurs and shadows.
- **The transport**: `Visual` freezes its `Effect` when it updates its composition visual
  (`visual.rs`, `EffectExtensions::to_immutable` when the frozen value is no longer equal), the
  generated property writes the `Arc<dyn IImmutableEffect>` into the object stream of the batch
  (`BatchObject::Value`, `transport/batch_stream.rs`), and the server visual reads it back
  (`generated/composition_visual.rs`). What crosses is an `Arc` of plain values; no object of the UI
  thread does.
- **The render side**: the walk of the server visuals (`server_composition_visual/render.rs`)
  pushes the effect around the content with `IDrawingContextImplWithEffects::push_effect`, inside
  the bounds of the subtree inflated by the padding. Skia makes an image filter for a save layer
  (`drawing_context_impl.rs`, `create_effect`); Vello makes a filter layer of the scene, or records
  the content into a scene of its own for the CPU renderer where the renderer has no filter layers
  or a clip is open (`drawing_context_impl/effects.rs`). An effect neither knows is a layer without
  a filter.
- **The GPU layer**: Skia's runtime effects (SkSL compiled once per thread and source, uniforms
  checked by name and type, children; `runtime_effects.rs`) and Vello's own wgpu passes (WGSL
  compiled once per device, uniforms written per pass, the result a pooled device texture the
  scene paints with; `src/Vello/FerroUI.Vello/gpu/effect_passes.rs`: the scene lighting, and since
  E3 the layers with an effect, whose content is a pooled texture the pass reads).

### 3.2 The open effect model (E1, this step)

A custom effect is a value type. Its snapshot is the effect: there is no generated immutable twin
as in Effector, because a Rust value is immutable once shared.

```rust
pub trait CustomEffect: Clone + PartialEq + Default + fmt::Debug + Send + Sync + 'static {
    const NAME: &'static str;                       // "tint" in effect strings
    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue));
    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool;
    fn padding(&self) -> Thickness { Thickness::default() }
    fn neutral() -> Option<Self> { None }           // the value that leaves the content unchanged
    fn interpolate(&self, to: &Self, progress: f64) -> Self { /* parameter by parameter */ }
}
```

- `EffectParameterValue` is the closed set of parameter kinds (`Bool`, `Int`, `Float`, `Color`,
  `Point`, `Vector`, `Size`, `Rect`, `Text`; `#[non_exhaustive]`, so the image handles of step E6
  are an addition). It is what parsing reads and writes, what the default interpolation
  interpolates (numbers, colours with `ColorAnimator`'s interpolation, points, vectors, sizes,
  rectangles; booleans and text switch at half), and what a backend factory will hand its program
  as uniforms. `for_each_parameter` visits without allocating, so a backend can read the uniforms
  of a frame without a `Vec`.
- `ImmutableCustomEffect` is the snapshot: an `Arc` of the value behind a private, type-erased
  trait. It is `Send + Sync`, implements `IEffect` and `IImmutableEffect`, compares by value
  (`PartialEq`, `IImmutableEffect::equals`), gives the padding (clamped to finite, non-negative
  sides), interpolates (`None` for two different types), knows its neutral value, and writes its
  effect string with `Display`. A backend finds its factory by `value_type_id()` and reads the
  value with `downcast_ref::<T>()` or the parameters.
- `IEffect::as_custom_effect` (a provided method, `None` by default) is how every consumer of
  effects (the padding, the animators, the backends, the mock context of the tests) recognizes one,
  as `as_blur_effect` recognizes a blur.
- **Mutable effect classes.** Effector's authoring model, an effect class with styled properties,
  is a class deriving from `Effect` whose class data is made by `Effect::construct_custom::<T>()`
  and which implements `CustomEffectClass::to_custom_effect` (its snapshot). Its handle converts to
  `Rc<dyn IEffect>` as the built-in classes do; the adapter of the class handles reports it as a
  mutable effect, and `to_immutable` returns its snapshot. Properties declared with
  `Effect::affects_render` (or a call of `raise_invalidated`) invalidate the visual, which takes a
  new snapshot when it is no longer equal to the one sent. `EffectExtensions::to_custom_effect`
  gives the snapshot of either form.
- **Registry and parsing.** `CustomEffectRegistry::register::<T>()` makes `T::NAME` known to
  `Effect::parse`, and so to markup and styles (`Effect="tint(color=#0F9D8E, strength=0.55)"`).
  Names and parameters match ignoring case, `-` and `_` (as Effector's); `blur` and `drop-shadow`
  are reserved; a second type under a taken name is an error
  (`CustomEffectRegistrationError`). Arguments are separated by commas or semicolons outside
  parentheses and quotes; a piece without `=` continues the value before it (`offset=10,20`). A
  value is read as the kind of the parameter's default. `Display` writes the same syntax, so a
  snapshot round-trips. The registry is a process-wide `RwLock`; it is read on the UI thread only.
- **Transitions and animation.** `EffectTransition` interpolates two custom effects of the same
  type, and a custom effect to or from no effect through the type's neutral value (a type without
  one switches at half). Effector interpolates from the default instance instead; a default is not
  in general a value without a visible result (a tint's default strength is 0.55), so FerroUI asks
  the type. Key frames on `Visual.Effect` go to `CustomEffectAnimator` when every frame is a
  custom effect. Neither needs the registry: the value carries its interpolation.
- **Render-thread safety and transport.** Nothing new crosses: the snapshot is an
  `Arc<dyn IImmutableEffect>` like the built-in values, written to the batch by value and read by
  the server visual. A mutable class is never sent; its snapshot is.
- **No factory yet.** A backend that has no factory for a custom effect draws its content
  unchanged (the layer without a filter it draws for an effect it does not know) and reports it
  once per backend and effect name (`ImmutableCustomEffect::report_unhandled`, a warning in the
  log area of the backend). Skia and Vello do this from E1 on; nothing panics. The padding of a
  custom effect is honoured even then, so the bounds of the visual already include it.

### 3.3 Backend factories, compiled once per backend

A factory turns the snapshot of one effect type into what a backend draws. As in WinUI, the
program is made once (per type and, for Skia, per thread; for Vello, per device), and a frame only
sets uniforms from the parameters. Three kinds of effects cover Effector's set:

1. **Graph effects** (Effector's typed filter effects). The type describes itself as a
   `GraphicsEffect` graph (the WinUI vocabulary FerroUI already has: Gaussian blur, colour matrix,
   saturation, tint, invert, grayscale, blends, composites, opacity, arithmetic composite) with the
   content as the source parameter `content` and the effect's parameter names as animatable
   properties: a provided associated function of `CustomEffect` (`graph()`, decision 4) returning the description, loaded once per
   type into an `EffectGraph` (the same loader and checks as `CompositionEffectFactory`) and
   registered for the prewarm. The render walk paints the graph around the content the way
   `LayerVisual.Effect` does (the composed content bound to the source parameter), with the
   parameters of the snapshot as the values of its animatable members. This is portable by
   construction: each node is drawn by each backend as the composition effects are, with the
   status gpu-rendering.md records per node. Tint, grayscale, sepia, saturation, brightness and
   contrast, invert and glow are graphs; sharpen and edge detect are graphs of a convolution node
   (`ConvolveMatrixEffect`, a Win2D effect, E3); pixelate needs a node of its own or a shader.
2. **Runtime shader effects** (section 3.4): a shader program with uniforms from the parameters
   and children (the content, secondary images).
3. **Native factories**: a backend crate may register, per effect type, a factory of its own (a
   Skia `ImageFilter`, a Vello pass) for what neither of the above expresses. It is the escape
   hatch, and the rule applies: an effect shipped in a sample has a GPU factory in every backend.

The registration of factories is per backend (`ferroui-skia` and `ferroui-vello` own them; the
base crate only defines the descriptions), keyed by `value_type_id()`. A miss is the no-op of
E1.

**Graph effects as built (E2).** The hook is three provided members of `CustomEffect`:

```rust
fn graph() -> Option<GraphicsEffect> { None }      // type-level, read once per type
fn for_each_graph_value(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
    CustomEffect::for_each_parameter(self, f)      // the values of the members, by default the parameters
}
fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, scale: f64) -> bool { false }
```

- *The content.* The source parameter `content` (`CUSTOM_EFFECT_CONTENT`) is the composed content
  of the visual. A graph with another source parameter does not load. The content is one layer,
  as the children of a layer visual are for its effect brush: the graph uses it once, along one
  path; the exception is a source-over composite of a shadow of the content and the content itself
  (the Win2D way of writing a glow or a drop shadow), which the painter draws as the drop shadow
  effect of the drawing context around the content, both from the one layer. This also applies
  to `LayerVisual.Effect`, whose padding now includes the shadows around the layer.
- *Naming of the members.* A member is named as an animatable property of
  `Compositor::create_effect_factory_with_properties`: `EffectName.PropertyName`, where
  `EffectName` is the `name` of an effect of the graph and `PropertyName` one of its animatable
  properties (`Tint.Color`, `Transfer.RedSlope`). A value of `for_each_graph_value` whose name has
  a dot sets that member. A value whose name has no dot, as the parameters have, sets the one
  animatable property of its kind of the effect named after it: the parameter `amount` (a number)
  of `sepia` sets `amount.Intensity` of the sepia effect named `amount`. A number sets a scalar
  property, a colour a colour property; an effect with no or several properties of the kind is a
  load error (`InvalidAnimatableProperty`). An effect whose members are computed from its
  parameters (a matrix from a strength, a clamped value) overrides `for_each_graph_value` and names
  them in full.
- *Loading and drawing.* The graph is loaded once per type by `EffectGraph::load_with_members`,
  the loader and checks of the effect factories, cached by `TypeId` (process-wide, the render
  thread loads a type it meets first, `CustomEffectRegistry::register` loads the registered ones
  on the UI thread), and registered for the prewarm under `custom-effect:<NAME>`, which draws it
  around an image as the walk draws it. The render walk asks the snapshot for its graph: with one,
  it paints the graph around the content of the visual (`push_custom_effect_graph`, the content,
  `pop_custom_effect_graph`), over the bounds of the subtree with the padding of the effect, the
  values of the snapshot as the members; without one, it pushes the effect, which the backends
  draw unchanged (E1). The painter is the one of the effect brushes (`EffectGraphPainter`), whose
  host gives the values of the members: the property set of a brush, the snapshot of a custom
  effect, the description in the prewarm.
- *The library* (`ferroui_base::media::effects::library`, registered the first time the registry
  is read): `tint` (a tint effect, `Tint.Color` the colour mixed with white by the strength),
  `grayscale` (a saturation effect at `1 - amount`), `sepia` (Win2D's sepia effect, its intensity
  the parameter), `saturation` (a saturation effect, clamped to 0..2.5), `brightness-contrast` and
  `invert` (Win2D's linear transfer, slopes and offsets of the colour channels), `glow` (the shadow
  of the content under the content, drawn as a drop shadow). Each matches Effector's matrices and
  clamps, and has a CPU reference.

### 3.4 Runtime shaders and portability between Skia and Vello

**Skia** runs SkSL on every surface (raster, Ganesh, Graphite). A shader effect is a save layer
whose image filter is `image_filters::runtime_shader` (skia-safe 0.153 binds it): the content is
the layer, given to the program as a child shader, so the content stays a GPU texture and is never
captured offscreen per frame. The program is compiled once by the runtime-effect cache of
`runtime_effects.rs`, and the uniforms are checked against what it declares, so a program and its
parameters cannot drift apart. An overlay shader (Effector's procedural mode) is the same program
without sampling the child.

**Vello** cannot run a user shader in any of its renderers: `vello` 0.11 (GPU), `vello_hybrid`
0.3 and `vello_cpu` 0.3 compose layers with the blend modes of the Compositing specification and
have the filter primitives `Flood`, `GaussianBlur` and `DropShadow` (`vello_common`,
`filter_effects.rs`), and no hook for a custom program. What runs custom GPU code in the Vello
backend is FerroUI's own wgpu pass (`gpu/effect_passes.rs`), which takes WGSL and writes a device
texture the scene paints with. So the plan, stated as it is:

- **The source language of Vello is WGSL.** A shader effect gives its program as SkSL for Skia and
  WGSL for Vello, with one convention for both: a uniform block made of the parameters in the
  order of `for_each_parameter` (a `Float` is a `float`, a `Point`, `Vector` or `Size` a `float2`,
  a `Color` a premultiplied `float4`, a `Rect` a `float4`, a `Bool` or `Int` an `int`), the content
  as the child `content`, coordinates in device pixels of the layer. The factory checks the WGSL
  against the parameters as `runtime_effects.rs` checks SkSL (naga 30, already in the tree through
  wgpu, parses WGSL and reports the uniform layout).
- **Content-sampling on Vello** takes "layers with an effect", step 2 of the Vello route in
  gpu-rendering.md (done in E3): the content recorded into a scene of the device, rendered into a
  pooled texture, read by the pass into another pooled texture the outer scene paints with. The
  step is shared with the colour matrix, the convolution and the pixel operations of the
  composition effects; a shader effect of E5 is one more program on it.
- **WebGL2** has no wgpu device: the WGSL is translated to GLSL ES 3.0 by naga (`glsl-out`) at
  build time, by a helper of `ferroui-build`, and the description embeds both (decision 2); the
  browser module carries no naga.
- **The CPU renderer** of Vello and Skia's raster surfaces are the reference: Skia raster runs the
  SkSL on the CPU; every effect FerroUI ships provides a CPU reference (decision 3), and Vello's CPU
  mode draws the content unchanged only for an application's effect without one. The CPU
  reference is a function over premultiplied RGBA (as `PixelOperation::apply_to_premultiplied_rgba8`
  is for the pixel operations), which the render tests compare the GPU output with.
- **Not chosen: a shared effect IR** (decision 1). One small language (an expression tree built in Rust,
  emitting SkSL, WGSL and GLSL ES) would make a shader portable by construction, at the cost of
  expressiveness and a code generator to maintain. Two sources with one checked convention is the
  smaller step and keeps every program readable in its own language; the IR stays an option if the
  sample set shows the duplication is the larger cost.

**Shader effects as built (E4).** The hook is the provided, type-level
`CustomEffect::shader() -> Option<ShaderEffectDescription>` (decision 4, `None` by default),
read once per type, cached by `TypeId` and registered for the prewarm under
`custom-effect:<NAME>` (`media/effects/custom_effect_shader.rs`).

- *The description.* `ShaderEffectDescription` holds the SkSL (`sksl`) and the WGSL (`wgsl`), the
  mode (`ShaderEffectMode::ContentSampling` or `Overlay`), the blend mode of an overlay
  (`BitmapBlendingMode`, source over by default), the sample radius (how far from the pixel it
  colours the program samples the content, in device independent pixels) and the padding
  behaviour (`ShaderEffectPadding::Effect`, the padding of the effect alone, or `SampleRadius`,
  which adds the sample radius to each side), and the names of its children, the image parameters
  the program samples (E6, section 3.6, "Secondary images as built"), and whether an overlay is
  masked by the content (`masked_by_content`, #580: its output times the alpha of the content
  before the blend, `blend(S * Da, D)`, as Effector draws its overlays). It is made with
  `content_sampling(sksl, wgsl)` or `overlay(sksl, wgsl, blend_mode)` and `with_sample_radius`,
  `with_padding`, `with_children`, `with_content_mask`.
- *Precedence.* A type with both a graph and a shader is drawn as its graph; its shader is not read
  (`ImmutableCustomEffect::shader` is `None`), and a graph that does not load gives the no-op of
  E1, not the shader. Every backend draws graphs; shaders need a backend that runs programs.
- *The uniform convention, as implemented.* `layer_size` (`float2`, the area of the effect in
  device pixels, rounded out to whole pixels) and `layer_scale` (`float`, device pixels per device
  independent pixel, the mean of the two axes when they differ) come first, then the parameters in
  the order of `for_each_parameter`, named with every `-` written `_` (`cell-size` is
  `cell_size`): `Float` a `float`, `Point`, `Vector` and `Size` a `float2`, `Color` a premultiplied
  `float4`, `Rect` a `float4` (x, y, width, height), `Bool` (0 or 1) and `Int` an `int`; `Text` has
  no uniform. Values pass as they are: a program scales a length by `layer_scale`. The program is
  given the centre of the pixel it colours in device pixels from the top left corner of the area.
  The SkSL declares the uniforms in that order, `uniform shader content;` when it samples the
  content, and `half4 main(float2 coord)`; the WGSL declares them as the members of
  `struct Uniforms` and `fn effect(coord: vec2<f32>) -> vec4<f32>`, the pass of E5 adding the
  bindings (`uniforms`, `fn sample_content`) and the entry points.
  `ImmutableCustomEffect::for_each_shader_uniform` visits the block of a frame without allocating
  and `shader_uniform_layout` gives the declarations a backend checks its program against.
- *Skia* (`ferroui-skia/custom_shader_effects.rs`). The drawing context pushes a shader effect
  (the render walk pushes it as an effect of the drawing context, as it pushes an effect without a
  graph) as its program: compiled once per thread and type by the cache of `runtime_effects.rs`,
  its uniforms checked once (names, order, types, no arrays) and its children (one shader
  `content` for a content-sampling program; for an overlay its images, E6, none before). A content-sampling program is
  the runtime shader image filter of a save layer (`image_filters::runtime_shader_with_options`,
  the sample radius in device pixels), the layer bound to `content`; an overlay is drawn over the
  content inside the save layer with its blend mode before the layer is composed, an overlay
  masked by the content with a runtime blender (`MASKED_OVERLAY`, `overlay_blend.eval(src * dst.a, dst)`,
  the blend mode a child blender, made once per program). Skia evaluates a
  runtime shader image filter with translations only (`SkRuntimeImageFilter` takes the matrix
  capability `kTranslate` and applies the rest of the transform to the result), so the layer is
  saved under a translation by whole device pixels to the top left of the area, clipped to the
  area, and the content drawn into it with its own transform: the layer has the resolution of the
  device (under a rotation, the area is scaled to the lengths of the axes and Skia transforms the
  result). The layer is composed with the opacity in effect. A program that does not compile,
  does not match, or has children that are not images of an overlay (E6) is reported once (an
  error in the log area `Skia`) and the content drawn unchanged; nothing panics. This works on
  raster, Ganesh and Graphite surfaces and in pictures.
- *Vello* draws a shader effect from E5 on (below); until then it drew it as E1 draws an effect
  without a factory.
- *The library* (`media::effects::library`, E4): `pixelate` (`PixelateEffect`, content-sampling:
  each cell of `cell-size` device independent pixels, clamped to 0..64 and at least one device
  pixel, shows the pixel at its centre; sample radius 64), `scanline` (`ScanlineShaderEffect`, an
  overlay drawn source over: the lower half of each band of `spacing` black at `strength`),
  `grid` (`GridShaderEffect`, #580, an overlay drawn source over: lines 6 percent of a cell of
  `cell-size` wide in `color` at `strength`) and `spotlight` (`SpotlightShaderEffect`, an overlay
  screened over the content: a light of `color` fading from (`center-x`, `center-y`) to
  `radius`), Effector's pixelation, scan line, grid and spotlight shaders with the clamps of its
  factories, each with its SkSL, its WGSL and its CPU reference, and a neutral value that leaves
  the content unchanged. The overlays are masked by the content (#580).

**Shader effects on Vello as built (E5).**

- *The module of the pass.* `ShaderEffectDescription::wgsl_module` (in `ferroui-base`, so that
  Vello and `ferroui-build` wrap the program the same way) is the WGSL of the description followed
  by what the pass adds: `@group(0) @binding(0) var<uniform> uniforms: Uniforms;` (the block of the
  convention, whose type is the program's `struct Uniforms`), `@binding(1)` the uniforms of the pass
  (`ferroui_pass.area`: the device pixel of the area at the top left texel of the textures, and the
  width and height of the part of the area they hold), `@binding(2)` the content
  (`ferroui_content`, a `texture_2d<f32>` read with `textureLoad`, no sampler), for a
  content-sampling program `fn sample_content(coord)` (the texel under `coord`, transparent beyond
  the area: Skia samples the child of a runtime shader image filter at texel centres with nearest
  sampling), the vertex stage of the layer passes and a fragment stage that calls `effect` at the
  centre of each pixel of the area. An overlay's output, clamped to 0..1 (times the alpha of the
  content texel when it is masked by the content, #580), is composed over the content in the pass
  with the blend mode of the description: every mode of `BitmapBlendingMode`
  (the Porter-Duff operators and the modes Skia writes in premultiplied form, among them source
  over and `Screen`, which the library uses, written so; the other separable and the non-separable
  modes of the Compositing and Blending specification on colours that are not premultiplied,
  composed source over), so no mode falls back. The names the pass adds start with `ferroui_`
  (`FerroUi`), but `uniforms`, `sample_content`, `vs_main` and `fs_main`; the binding numbers and
  the entry points are the constants `SHADER_WGSL_*`.
- *The check.* The module is checked once per thread and type, at the first push of the type
  (`gpu/effect_passes.rs`, `check_shader_module`): parsed and validated by naga 30 (the one of wgpu,
  through `wgpu::naga`, without optional capabilities, so that any device takes what it takes), its
  block `uniforms` compared with `ImmutableCustomEffect::shader_uniform_layout` (the number of
  members, their names, order and types; no array or struct), and a description whose children
  are not images of an overlay refused (E6 binds the images of an overlay). The offsets of the members, and the size of the block, come from naga's layout of
  the struct, so the bytes of a frame are written where WGSL's layout puts them. A module that fails
  is reported once (an error in the log area `Vello`) and the effect drawn as an effect without a
  factory: its content unchanged, and nothing drawn on the processor.
- *The pass.* On the route of E3 (hybrid and GPU modes): the drawing context records the content
  into a scene of the device over the area of the effect (its bounds under the transform, rounded
  out to whole device pixels as Skia's layer is, cut to the scene; the content outside the area is
  not drawn, as Skia clips the layer), with full opacity, renders it into a texture of the pool, and
  the pass of the type writes another texture of the pool, which the scene the effect was pushed in
  paints over the part of the area on it, composed with the opacity in effect. `layer_size` is the
  size of the whole area and the coordinates of the program are those of the area, also where it
  reaches beyond the target; `layer_scale` is the mean of the lengths of the axes of the transform.
  The pipeline is compiled once per device and type, the block and the uniforms of the pass written
  for each pass into the buffers of the pipeline, the bind groups kept by the identity of the
  content texture; steady frames allocate no texture. Under a rotation the area is the bounding box
  of the rotated bounds, in device pixels, where Skia scales the area to the lengths of the axes
  and rotates the result: a program whose output depends on the direction of the axes differs.
- *The CPU mode* (and a shader effect inside a scene of the CPU renderer, and WebGL2 until step 3
  of the Vello route) draws the CPU reference (decision 3): the content is recorded into a scene of
  the CPU renderer over the whole area (cut to the scene above 2^24 pixels), rendered, the
  reference applied to its pixels at `layer_scale`, and the result drawn as an image with the
  opacity in effect. An effect without a reference (an application's) draws its content unchanged
  and is reported once (`report_unhandled`).
- *WebGL2.* `ferroui-build` translates the module to GLSL ES 3.0 at build time with naga
  (`ferroui_build::shader_effects`: `translate_shader_effect_module`, `shader_effect_glsl::<T>()`,
  `write_shader_effect_glsl::<T>()`, which writes `$OUT_DIR/shader_effects/<name>.vert.glsl` and
  `.frag.glsl`, with the names naga gives the uniform blocks and the texture by their WGSL
  bindings). The browser renderer does not draw these programs yet: its sink has no device textures
  (step 3 of the route in gpu-rendering.md), so WebGL2 draws the CPU reference.

### 3.5 Input routing for interactive effects

A mutable effect class may implement an input trait (the members of `ISkiaInputEffectHandler`:
attached, detached, host bounds changed, pointer entered, exited, moved, pressed, released, capture
lost, wheel changed) with a host context (the visual, its bounds, the pointer position in the
visual and normalized to its bounds, capture and release of the pointer, invalidation). The routing
follows Effector's input manager: one class handler of `Visual::effect_property()` changes
attaches to the host when its effect is interactive and detaches when the effect changes, through
the host's `InputElement` pointer events, holding `Subscription` guards
([idiomatic-rust-disposal.md](../idiomatic-rust-disposal.md)). The state an event changes lives in
the class; `raise_invalidated` makes the next snapshot, so the render thread sees only values (a
pointer position is a `Point` parameter, a press time a `Float`).

**Interactive effects as built (E7).**

- *The input trait.* `IEffectInputHandler` (`media/effects/effect_input.rs`) has the members of
  `ISkiaInputEffectHandler`, each with an empty default as in `SkiaInteractiveEffectBase`:
  `on_attached`, `on_detached`, `on_host_bounds_changed`, `on_pointer_entered`, `on_pointer_exited`,
  `on_pointer_moved`, `on_pointer_pressed`, `on_pointer_released`, `on_pointer_capture_lost` and
  `on_pointer_wheel_changed`, and `handled_events_too` (default `false`). A class implements it
  and constructs its base with `Effect::construct_interactive::<T>()` (a custom effect class whose
  `Effect` also finds its handler: `is_interactive`, `input_handler`).
- *The host context.* `EffectHostContext` is Effector's `SkiaEffectHostContext`: the effect, the
  host (`InputElement`) and its visual, `bounds`, `size`, `render_bounds` (#580: the rectangle the
  renderer drew the effect over in the last frame, the bounds of the subtree of the host with the
  padding of the effect, read back from the compositor), `input_bounds` (that rectangle without the
  padding, the host's own size before the first frame, so the input and the program agree where
  Effector uses the content of the host within its bounds for both), `padding` and `effect_bounds`
  (the input bounds inflated by the padding of the effect: the area the program is drawn over),
  `layer_scale` (the render
  scaling times the mean of the lengths of the axes of the transform from the host to the root, as
  the render thread computes it), `get_position` (in the host, through every render transform),
  `get_normalized_position` and `normalize` (0 to 1 across the input bounds, clamped on request),
  `get_effect_position` and `to_effect_space` (device pixels of the area of the effect: the
  `coord` of the program), `capture_pointer`, `release_pointer_capture` and
  `has_pointer_capture`, `invalidate` (`raise_invalidated`), and `request_animation_frame`, a frame
  of the animation clock that runs only while the effect is still the host's.
- *The routing* (`input/input_element_effects.rs`). One class handler of the changes of
  `Visual::effect_property()`, registered with the class handlers of `InputElement`, attaches the
  effect of an input element when it is an interactive class (`InputElement::interactive_effect`)
  and detaches the one it attached when the effect changes: the old attachment is dropped (its
  handlers removed), then `on_detached`; the new one adds its handlers, then `on_attached` and
  `on_host_bounds_changed`. The attachment is a field of the element holding the effect and the
  `Subscription` guards of seven handlers (`add_disposable_handler_with` on the direct and bubble
  routes, handled events too) and of a subscription to the element's `Bounds`,
  `RenderTransform` and `RenderTransformOrigin`; nothing outlives it. A handler delivers the event
  to the effect after the tunnel route, the class handlers of the host (its `on_pointer_*`) and the
  bubble handlers of its descendants, before its ancestors; a handled event only to an effect whose
  `handled_events_too` is set (the four classes of the library set it, #580). A pointer the host
  (or the effect, through the context) captured is
  routed to the host wherever it is; the loss of the capture of the host or of a descendant (the
  release of the implicit capture, another element capturing the pointer) is
  `on_pointer_capture_lost`. No borrow of the host is held while the effect runs, so the effect
  can capture the pointer or replace the effect of its host. Only an `InputElement` hosts an
  interactive effect: a plain visual has no pointer input.
- *Values only cross.* The class keeps its state (the pointer, the press, the ripples, the phase of
  a flame) in a value of `CustomEffect` and raises `invalidated` when the value changes; the visual
  takes the new snapshot, and the program, compiled once per backend, is given its uniforms. The
  tests count the compilations (`runtime_effects::compilation_count` of Skia,
  `effect_passes::shader_pipelines_created` of Vello): frames of an interaction compile nothing.
- *The library* (`media::effects::library`, E7): `PointerSpotlightShaderEffect`
  (`pointer-spotlight`, an overlay screened over the content) and `ReactiveGridShaderEffect`
  (`reactive-grid`, an overlay drawn source over, lines 5 percent of a cell wide), the light and the
  halo under the normalized pointer, brighter or wider while pressed, both masked by the content
  (#580); `WaterRippleShaderEffect` (`water-ripple`, content-sampling, sample radius 56: three
  `RippleWave`s, each a point and two numbers, rings that move the content outward and tint it);
  `BurningFlameShaderEffect` (`burning-flame`, content-sampling, sample radius 21: a plume from an
  ignition point that moves, colours, lights and smokes the content). The two content-sampling
  programs sample the moved content clamped to the area (Effector's clamp tiling) and compose
  what they compute over the content source atop, so nothing is smeared beyond its coverage
  (#580). Each has SkSL, WGSL, a CPU reference and a neutral value. The classes
  `InteractivePointerSpotlightEffect`, `InteractiveReactiveGridEffect`,
  `InteractiveWaterRippleEffect` and `InteractiveBurningFlameEffect` (with `value`, `set_value`
  and `update`) follow Effector's: the overlays track the pointer and capture it while pressed,
  the water ripple makes a ripple on a press, along a drag and on the release and ages them
  (`advance`), the flame ignites on a press, follows a drag and burns down after the release
  (`advance`); the last two advance on the frames of the animation clock while they move, where
  Effector runs a `DispatcherTimer` of 16 ms. All four take handled events, as Effector's do (the
  host of the flame, the burn-away button, handles the press). The programs differ from Effector's
  in five ways: squares are products (`pow` of a negative number is undefined), the moved content
  is sampled at the centre of the texel under the moved coordinate (the same texel whatever the
  filtering of a backend), colours are mixed in premultiplied form, the moved content is composed
  source atop where Effector masks it `DstIn` and draws it source over (the same on opaque and
  transparent content; translucent content at rest stays unchanged), and the phase of the flame
  wraps at 10 pi (a period of its three waves). Lengths are device independent pixels where
  Effector's are device pixels (DEVIATIONS.md, Animation). The catalog page "Custom effects" shows
  them in a section "Interactive effects".

### 3.6 Secondary image handles

An image a shader samples besides the content (the two pages of a transition) is a parameter kind,
`EffectParameterValue::Image`, holding a handle that is a value: an `Arc` of the decoded image
(the same shared bitmap the image brushes use), compared by identity. The `Arc` is the lease
Effector implements by hand: the image lives while a snapshot or a frame holds it, and is freed
when the last one drops it, on whatever thread. Each backend uploads an image once and keeps the
texture keyed by the image's identity while it is in use (as the image brushes are,
gpu-rendering.md), so a frame that changes only uniforms uploads nothing. Capturing a page into
such an image is `RenderTargetBitmap` (made by the application, not per frame).

**Secondary images as built (E6).**

- *The handle.* `EffectImage` (`media/effects/effect_image.rs`) is an `Option<Arc<..>>` of a
  counted reference to the platform bitmap (`RefCounted<SharedBitmapImpl>`, so the bitmap lives
  while a handle does; `EffectImage::new(&bitmap)` takes another count of a `Bitmap`,
  `from_ref` of a counted reference, `from_impl` owns a platform bitmap), with an identity that is
  never reused (`id`, 0 for the empty handle). It is `Send + Sync`, cheap to clone, compares and
  hashes by identity, and its default is the empty handle, which programs sample as transparent.
  `downgrade` gives an `EffectImageWeak` that tells whether the image still lives;
  `EffectImage::release_generation` counts the handles released (the last clone dropped) in the
  process, so a backend looks for dead entries only when it changed. `pixels` reads the
  premultiplied RGBA of the bitmap once per version (`EffectImagePixels`, `None` for a bitmap
  that is not readable on the processor), what the CPU references sample and Vello uploads.
- *The parameter.* `EffectParameterValue::Image` (an addition to the `#[non_exhaustive]` enum)
  is no uniform; it switches at half when interpolated, and an empty image takes the other side's
  image, so a transition from no effect (through the neutral value, which has no images) keeps the
  images of the effect it goes to. Markup cannot set an image: an effect string is parsed without
  a resource context, so `parse_like` of an image fails (`mask-transition(mask=...)` is a parse
  error), and the effect string of a snapshot leaves its images out (it parses back to the effect
  with empty images). An image is set from code, on the value of the effect or in the snapshot of
  a mutable effect class.
- *The children convention.* The `children` of a `ShaderEffectDescription` name image
  parameters, at most `MAX_SHADER_CHILDREN` (8), in the order the program declares them: in SkSL
  `uniform shader <name>;` after the uniforms (`<name>` with every `-` written `_`), sampled with
  `<name>.eval(coord)`; in WGSL `fn sample_<name>(coord: vec2<f32>) -> vec4<f32>` and the texture
  `ferroui_child_<name>` at `SHADER_WGSL_CHILDREN_BINDING` (3) plus the index, which
  `wgsl_module` adds. Both give the image stretched over the whole area of the effect
  (`layer_size`), the texel under `coord * image_size / layer_size` (nearest, as the content is
  sampled; `EffectImagePixels::sample_stretched` for the CPU references), premultiplied,
  transparent beyond the area. Children are for overlays (the shape of Effector's transitions,
  programs drawn over the content from images): a content-sampling program takes none, because
  Skia draws it as a runtime shader image filter, which takes its other children from the builder
  of the program, and skia-safe 0.153 binds no setter for them; both backends refuse it (once,
  the content unchanged), so a program draws the same everywhere.
  `ImmutableCustomEffect::check_shader_children` is the check both backends run once per type,
  `for_each_shader_child` the visit of a frame (no allocation).
- *Skia.* The program of an overlay is checked for its children (names and order) with its
  uniforms; each frame binds image shaders of the Skia images of the images, stretched by a local
  matrix and sampled by the nearest texel (decal beyond), the empty image a transparent shader.
  An image is made a Skia image once per thread and version of its bitmap (the image of an
  immutable bitmap as it is, a snapshot of a writeable bitmap or a surface, a raster image of the
  pixels of another readable bitmap), kept by the identity of its handle with its weak handle; the
  GPU context makes its texture once (Graphite through `drawable_image`, which keeps the texture
  while the image is held; Ganesh by its cache of uploads by the identity of the image). When the
  last handle is dropped, the next drawing context of the thread drops the Skia image (one atomic
  read when nothing was released) and the context releases the texture with its flush.
- *Vello.* The pass binds the textures of the children after the content; an image is uploaded
  once per device into a texture (`create_image_texture`, its premultiplied pixels), kept in a
  cache of the render thread by the identity of its handle with its weak handle (the empty image
  a transparent texture of one pixel per device); the bind groups are kept by the identities of
  the content and the image textures. The handles are dropped on the UI thread, the textures live
  on the render thread: the next drawing context or pass of the render thread after the
  generation of releases changed releases the textures of dead handles and the bind groups that
  hold them. The CPU mode draws the CPU reference, which samples `EffectImage::pixels`.
- *Graph effects* take no images: the graph of a custom effect has one source, the content
  (section 3.3); its vocabulary has no image source of its own (a `CompositionSurfaceBrush` is a
  source parameter bound by a brush, which a custom effect's graph does not have), so images are
  for shader effects only, and an image parameter is no member of a graph.
- *The library* (`media::effects::library`, E6): `dissolve-transition`
  (`DissolveTransitionEffect`, Effector's compiz-style dissolve: from `from-page` to `to-page`
  along a frontier that spreads from (`center-x`, `center-y`) through the luminance of `pattern`,
  with a glow of `glow-color` along it; Effector hashes the position for the pattern, which does
  not draw the same on every processor, so the pattern is an image) and `mask-transition`
  (`MaskTransitionEffect`: `to-page` revealed over the content where the luminance of `mask` is
  below `progress * (1 + softness)`). Both are overlays drawn source over the content, with SkSL,
  WGSL and CPU references; their neutral values (a progress of 0 without images) leave the content
  unchanged. The catalog page "Custom effects" shows them on photographs of its assets.

### 3.7 Samples

`samples/CustomEffects` (a gallery application) and the control catalog, one page per effect
family (as the catalog has one page per control): colour effects (tint, grayscale, sepia, saturation,
brightness and contrast, invert), image effects (glow, sharpen, edge detect, pixelate), shader
effects (scanline, grid, spotlight), interactive effects (pointer spotlight, reactive grid, water
ripple, burn-away button) and page transitions (the compiz-style set). Each shows code, markup
(`Effect="tint(...)"`) and an animation, as Effector's gallery does. Effector's sample effects are
ported as the reference set.

**Samples as built (E8).**

- *The gallery.* `samples/CustomEffects` (the crate `custom-effects`; `cargo run -p custom-effects`)
  is the counterpart of `Effector.Sample.App` and `Effector.Compiz.Sample.App`: a header, a
  section "Setting an effect" (a value set from code, the effect string
  `tint(color=#0F9D8E, strength=0.55)` read by `Effect::parse`, the converter of `IEffect` that
  markup's `Effect="..."` goes through, and key frames of `Visual.Effect` on a glow) and one
  section per family (`Family`, in `Family::ALL`): colour effects, image effects, shader effects,
  interactive effects and page transitions. Without its default feature `app` (the WinUI 3 theme
  and the desktop entry point) the crate is the sections only, which the catalog depends on.
- *The tiles.* A tile (`EffectTile`) holds a value of a library effect and sets its snapshot as
  the `Effect` of its host: a slider per number and a button per colour (Effector's swatches)
  change the value; the tile shows the code that makes the value, the effect string markup writes
  for the current value (which `Effect::parse` reads back to the same snapshot), and two
  animations of `Visual.Effect`: an `EffectTransition` of 300 ms on the host, so a change fades
  in and turning the effect off fades it out through its neutral value, and key frames that sweep
  the first number from the minimum of its slider to the maximum and back (the progress of a
  transition forward only). The interactive tiles host the interactive classes of E7 and show the
  effect string of the snapshot the pointer changes, rewritten at most every `CAPTION_MS` (200 ms)
  while it changes on every frame (#580). The water ripple bends image content (the picture of
  `TransitionImages`, or a checkerboard the gallery draws), and the burn-away button is opaque, as
  Effector's (`#1E2328`, a radius of 18; its face a border of its own, so the visual states of the
  button do not make it translucent). The shader family has Effector's grid (`GridShaderEffect`,
  #580) and Effector's nested sample, a spotlight over cards with tints of their own.
- *The page transitions.* The images are a parameter of the sections (`TransitionImages`: two
  pages and the pattern of the dissolve, also the mask of the mask transition, and the picture the
  water ripple bends). The gallery captures two pages of its own, a pattern of grey cells and a
  picture once with `RenderTargetBitmap` (`TransitionImages::render`); the catalog gives
  photographs of its assets (to the transitions and the interactive effects). Besides a tile per
  transition, the presenter (Effector's compiz sample) shows one page and, on a press or its
  button, rests on the other page and plays the dissolve to it from the point pressed by key
  frames of its progress. The compiz set of Effector beyond the dissolve (burn, cube, wobbly,
  genie, magnetic) is not in the library: the library's transitions are the dissolve and the mask
  of E6.
- *The catalog.* The section "Custom effects", after "Composition", has a page per family
  (Colour effects, Image effects, Shader effects, Interactive effects, Page transitions), each the
  section of its family in a scroll viewer (`Pages/custom_effects_pages.rs`); it replaces the
  single page "Custom effects" of the section "Composition" of E2 to E7.
- *Tests.* The gallery (`samples/CustomEffects/tests.rs`, on the test application with the Skia
  render interface of `sample-testing` and a global clock the tests tick): the tiles of each
  family and the effects of their hosts, the effect strings parsed back, a slider, a colour, the
  toggle, the key frames there and back and forward, the captured pages, the presenter going to
  the other page and back, an interactive tile following its class, the ways of setting an
  effect and the sections of the gallery. The catalog: the section and its pages
  (`tests/view_models.rs`), each page showing its tiles in a window and the photographs of the
  transitions (`tests/custom_effects_pages.rs`).

### 3.8 Tests

- E1 (this step): the snapshot's equality, padding and interpolation, the parameter kinds and
  their text, parse round trips and errors, the registry's rules, the mutable classes
  (`media/effects/custom_effect_tests.rs`); the transitions and key frames
  (`animation/tests/effect_tests.rs`); the transport of a snapshot and of a class's snapshot
  through the batch stream to the server visual, and the render walk drawing the content unchanged
  inside the pushed effect (`rendering/composition/composition_custom_effect_tests.rs`); the no-op
  of each backend, pixel by pixel (`custom_effect_without_a_factory_draws_unchanged` in
  `ferroui-skia/tests.rs` and `ferroui-vello/effect_tests.rs`).
- E2: the loader of the members (`effect_graph.rs`), the library effects (graphs, members, round
  trips, neutral values, CPU references, registration and prewarm; `media/effects/library_tests.rs`),
  the render walk drawing each library effect as its graph with the matrix of its snapshot, the
  glow as a drop shadow, a parameter bound by the name of an effect and a graph that does not load
  (`composition_custom_effect_tests.rs`), and each library effect drawn on Skia raster and Graphite
  (`tests.rs`, `gpu/metal/tests.rs`) and on Vello CPU, hybrid and GPU (`effect_tests.rs`,
  `gpu/effect_pass_tests.rs`) against its CPU reference.
- E3: `ConvolveMatrix` and its CPU reference (`media/convolve_matrix.rs`), the loader of the
  convolution node (`effect_graph.rs`), sharpen and edge detect in the library tests, the render
  walk drawing them as convolution layers (`composition_custom_effect_tests.rs`), the convolution
  layer against its reference on Skia raster, Graphite and Vello CPU (`tests.rs`,
  `gpu/metal/tests.rs`, `effect_tests.rs`), and in the hybrid and GPU modes of Vello
  (`gpu/effect_pass_tests.rs`) each pass of the route (colour matrix, convolution, luminance mask,
  light multiply, pixel operations) against its CPU reference, the library effects without a
  layer drawn on the processor (`cpu_layer_count`), and nested layers over steady frames without
  a texture allocated by the pool.
- E4: the description, its precedence below a graph, the uniform of each kind of parameter, the
  block and the layout of an effect, the padding of a sample radius, the SkSL and the WGSL of the
  library declaring the uniforms of their parameters, round trips, registration and prewarm,
  neutral values and CPU references (`media/effects/custom_effect_shader_tests.rs`); the SkSL of
  the library compiled and checked, the checks of a program against the convention, the geometry
  of the layer (`ferroui-skia/custom_shader_effects.rs`); the render walk pushing a shader effect
  (`composition_custom_effect_tests.rs`); each library shader effect drawn on Skia raster at 96
  and 192 DPI and on Graphite against its CPU reference, a neutral value and the area of the
  effect, and a program that does not compile, does not match or has children drawing the content
  unchanged, reported once (`tests.rs`, `gpu/metal/tests.rs`).
- E5: the WGSL module of the pass (`custom_effect_shader_tests.rs`); the modules of the library
  checked by naga with the offsets of WGSL's layout, the checks of a module against the convention,
  every blend mode of an overlay valid WGSL (without a device), each library shader effect drawn by
  its pass in the hybrid and GPU modes at scales 1 and 2 and over an area that reaches beyond the
  target against its CPU reference without a layer drawn on the processor, a module that does not
  parse or does not match drawing the content unchanged, reported once, and steady frames
  allocating no texture (`gpu/effect_pass_tests.rs`); the CPU mode drawing each library shader
  effect as its CPU reference at 96 and 192 DPI, inside its area and with the opacity in effect, and
  an application's effect without a reference unchanged, reported once (`effect_tests.rs`); the
  GLSL ES of the library's modules from `ferroui-build` (a smoke check of each stage) and the files
  it writes (`shader_effects.rs`).
- E6: the handle (identity, `Send` and `Sync`, the lease of the bitmap across threads, the weak
  handle and the generation of releases, the pixels read once, the sampling stretched over an
  area), the image parameter (interpolation, text, parsing), the checks and the visit of the
  children, the WGSL the pass adds for them, and the transitions of the library (their programs,
  round trips without their images, registration and prewarm, neutral values, a transition from no
  effect keeping the images, CPU references) (`media/effects/effect_image_tests.rs`); the checks
  of the children of a program (`ferroui-skia/custom_shader_effects.rs`); the transitions on Skia
  raster at 96 and 192 DPI and on Graphite against their CPU references, the Skia images made once
  over steady frames and Graphite's upload count constant, the images and textures of dropped
  transitions released, a content-sampling program with images refused (`tests.rs`,
  `gpu/metal/tests.rs`); the modules of the transitions checked by naga, the transitions in the
  hybrid and GPU modes against their CPU references at scales 1 and 2, no upload and no allocation
  over steady frames, the textures of dropped images released and bounded over frames with new
  images (`gpu/effect_pass_tests.rs`), the CPU mode (`effect_tests.rs`); the GLSL ES of the
  transitions binding their images (`ferroui-build/shader_effects.rs`).
- E7: the routing with the test root of the input tests (`input/effect_input_tests.rs`): the
  effect attached and detached with the effect of its host, its handlers and its subscription
  removed with it, the bounds and the render transform reaching it; the order of the routed events
  (tunnel, the bubble handlers of the descendants, the effect, the handlers of the host added
  after it, the ancestors), entered before moved, the release, the loss of the implicit capture,
  the wheel and the exit; handled events reaching only an effect that asks for them; a captured
  pointer reaching the effect outside its host, the loss of the capture on the release and to
  another element; the positions in the host, normalized and in the coordinates of the program
  with the padding, through a render transform of scale 2 (`layer_scale` 2); the spotlight
  following the pointer and capturing it, the burn-away button taking its button's handled press
  while a spotlight does not, the water ripple making ripples on a press, a drag and a release and
  stopping when detached. The library values and classes (`media/effects/interactive_effect_tests.rs`):
  the programs declare the uniforms of the convention, round trips, registration and prewarm,
  neutral values, the CPU references, the ripples, the invalidation of a class only on a change,
  and the advance of the water ripple and the flame. On Skia: the programs compile and match
  (`custom_shader_effects.rs`), raster at 96 and 192 DPI against the CPU references (the programs
  that move the content with at most 1% of the pixels further than 3 levels), neutral values, and
  frames of an interaction compiling no program (`tests.rs`), Graphite (`gpu/metal/tests.rs`). On
  Vello: the modules checked by naga, the hybrid and GPU modes against the CPU references, frames
  of an interaction compiling no pipeline and allocating no texture (`gpu/effect_pass_tests.rs`),
  the CPU mode (`effect_tests.rs`). The GLSL ES of the modules (`ferroui-build/shader_effects.rs`).
- E8: the tiles of the gallery and the pages of the catalog (section 3.7, "Samples as built",
  *Tests*): `samples/CustomEffects/tests.rs`, `samples/ControlCatalog/tests/view_models.rs` and
  `tests/custom_effects_pages.rs` of the catalog.
- E9: the validation of the whole library (section 3.9). Every effect at 96 and 192 DPI against
  its CPU reference on Skia raster (`tests.rs`: the graph effects at 192 DPI added to the earlier
  steps), Graphite at scales 1 and 2 (`gpu/metal/tests.rs`,
  `graphite_draws_the_library_at_twice_the_scale_as_the_cpu_references`), and the CPU, hybrid and
  GPU modes of Vello (`effect_tests.rs`, `gpu/effect_pass_tests.rs`: the graph effects at scale 2
  added); the comparison scenes (`tests/FerroUI.RenderBackends.Comparison/custom_effect_scenes.rs`
  and `custom_effect_tests.rs`): each effect through the contracts on Skia raster and every mode
  of Vello at 96 and 192 DPI against its CPU reference, Skia against each mode of Vello, and the
  neutral values; the steady state: frames of the whole library that change only parameters
  compile no SkSL program and make no image on Skia raster
  (`frames_of_the_library_compile_no_program_after_the_first`), and compile no pipeline, allocate
  no texture and upload no image in the hybrid and GPU modes of Vello
  (`frames_of_the_whole_library_compile_and_allocate_nothing_in_the_gpu_modes`); the prewarm: the
  composition target draws every type of the library once offscreen after its first frame
  (`a_target_prewarms_the_custom_effects_of_the_library_after_its_first_frame`,
  `compositor_tests.rs`), the prewarm drawn as the target draws it compiles every program the first
  frame of the library needs on Skia raster
  (`the_prewarm_of_the_library_compiles_what_its_first_frame_draws`) and through a Graphite render
  target (`a_graphite_render_target_prewarms_the_library_ahead_of_the_frame_that_shows_it`), and on
  Vello, which does not prewarm, the first frame makes one pipeline per shader type and no frame
  after it makes one (`the_first_frame_of_the_library_makes_one_pipeline_per_shader_type`).

### 3.9 Validation as built (E9)

- *Against the CPU reference, per backend and mode.* Every effect of the library (the nine graph
  effects, the three runtime shaders, the two page transitions, the four interactive effects) is
  drawn around a scene of opaque and translucent squares and compared with its CPU reference
  applied to the same scene drawn plain by the same backend and mode, at one and two device pixels
  per unit, on Skia raster, Graphite (macOS, a Metal device), and the CPU, hybrid and GPU modes of
  Vello. The tolerances are those of the steps that added each effect, at both scales: 2 levels for
  the colour matrices and convolutions, 12 for the glow (a blur against the reference's), 1 to 3
  for the runtime shaders and the transitions, one or two more on Graphite (14 for the glow), and
  for the programs that move the content (the water ripple, the flame) at most 1% of the pixels
  further than 3 levels (4 on Graphite) with a mean within 1.5. Ganesh draws the same programs as
  Graphite and raster; no pixel test of the custom effects runs on it (the GPU pixel tests of
  `ferroui-skia` are Graphite's, on a Metal device). WebGL2 draws the CPU references until step 3
  of the Vello route in gpu-rendering.md.
- *Comparison scenes.* `tests/FerroUI.RenderBackends.Comparison` (the crate that links both
  backends) draws each effect through the contracts only, as the render walk draws it, into render
  target bitmaps of Skia raster and of every mode of Vello that is built, at 96 and 192 DPI, holds
  each to the CPU reference, and compares Skia with each mode of Vello in the table form of the
  other comparison scenes: the share of the pixels beyond the tolerance of the harness and the mean
  difference, bounded by the two tolerances and by where the references of the two plain
  renderings differ. A neutral value of each effect draws the content unchanged everywhere.
- *Steady state.* Effect graphs are loaded once per type, programs compiled once per thread (Skia)
  or device (Vello) and type, and a frame changes only uniforms and the members of graphs: the
  tests draw the whole library over eight frames whose parameters change and hold the counters of
  the programs (`runtime_effects::compilation_count`), the Skia images of effect images
  (`custom_shader_effects::effect_images_made`), the pipelines
  (`effect_passes::shader_pipelines_created`), the textures of the pool
  (`effect_passes::texture_allocations`) and the uploads of images (`effect_passes::image_uploads`)
  constant after the first frames.
- *Prewarm.* The prewarm API is the one of #437 (gpu-rendering.md, "Pipelines made ahead of their
  first draw"): each type registers its graph or its default value under `custom-effect:<NAME>`
  when it is first loaded, and a composition target draws what was registered once offscreen after
  its first frame, through `IRenderTarget::prewarm`. Skia's GPU render targets draw it (Graphite on
  a recorder of its own); the tests show that this compiles every program the first frame of the
  library needs. A graph is drawn at the values of its description, where some colour matrices are
  the identity and are not drawn: a colour matrix layer is one program whatever its matrix, which
  the other graphs prewarm. Vello's render targets do not prewarm (the default of the contract):
  the pipeline of a shader effect type is made by the first frame that draws it, once per device,
  which the tests bound; drawing the prewarm onto a scene of the device offscreen is the step that
  would add it, not taken (its cost has not been measured).

## 4. Steps

Each step is one pull request from main.

| Step | Content | Public API | Risk |
|---|---|---|---|
| E0 | This plan | none | none |
| E1 | The open effect model in ferroui-base (section 3.2): `CustomEffect`, `EffectParameterValue`, `ImmutableCustomEffect`, `CustomEffectClass` and `Effect::construct_custom`, `IEffect::as_custom_effect`, `EffectExtensions::to_custom_effect`, `CustomEffectRegistry` and `Effect::parse`, `CustomEffectAnimator` and `EffectTransition`; the no-op and the report of Skia and Vello; tests | additions (a provided method of `IEffect`) | low: the built-in effects take the same paths; an `IEffect` of an unknown kind still panics in `get_effect_output_padding` as upstream |
| E2 | Graph effects (section 3.3, kind 1): the description hook of `CustomEffect`, loaded once per type into an `EffectGraph`, painted around the content by the render walk with the parameters as animatable members, registered for the prewarm; the colour effects of the sample set as graphs. **Done**: `graph`, `for_each_graph_value`, `cpu_reference`, the library of section 3.3, the catalog page "Custom effects" | addition | medium: the walk gains a second way to draw `Visual.Effect`; the graph painter is shared with the effect brushes |
| E3 | Convolution and the Vello colour layers: a `ConvolveMatrixEffect` node (Skia: a matrix convolution image filter; Vello: a WGSL pass), and Vello step 2 of gpu-rendering.md ("layers with an effect": the content into a pooled device texture, a pass into another) for the colour matrix nodes, so the graph effects are GPU on Vello's GPU and hybrid modes. **Done**: `ConvolveMatrix`, `push_convolve_matrix_layer`, the node, sharpen and edge detect; the route for the colour matrix, convolution, light multiply, pixel operation and luminance layers; WebGL2 keeps the CPU path (route step 3) | addition | high: the Vello pass reads the content of a layer, a new route in the scene |
| E4 | Runtime shader effects on Skia (section 3.4): the shader description (SkSL source, the uniform convention, children, blend mode, overlay or content-sampling), compiled once per thread by `runtime_effects.rs`, drawn as a runtime-shader image filter of a save layer; the shader samples. **Done**: `shader`, `ShaderEffectDescription`, the uniform convention, the Skia programs, pixelate, scan lines and spotlight in the library and on the catalog page "Custom effects" (section 3.4, "Shader effects as built") | addition | medium: SkSL errors surface at the first draw (logged once, then the no-op) |
| E5 | Runtime shader effects on Vello: the WGSL source of the description, checked with naga, a pass of `effect_passes.rs` compiled once per device on the route of E3; WebGL2 by GLSL ES generated at build time (decision 2); the CPU reference hook (decision 3). **Done**: `ShaderEffectDescription::wgsl_module`, the pass of the hybrid and GPU modes, the CPU reference in the CPU mode, `ferroui_build::shader_effects` (the GLSL ES, not drawn by the browser yet) (section 3.4, "Shader effects on Vello as built") | addition | high: depends on E3 |
| E6 | Secondary images (section 3.6): `EffectParameterValue::Image` and its handle, uploaded once per backend, bound as children; the page transitions. **Done**: `EffectImage`, `EffectImageWeak`, `EffectImagePixels`, the children convention (`SHADER_WGSL_CHILDREN_BINDING`, `MAX_SHADER_CHILDREN`, `check_shader_children`, `for_each_shader_child`), Skia's image shaders and Vello's textures uploaded once and released after the last handle, the dissolve and mask transitions in the library and on the catalog page (section 3.6, "Secondary images as built") | addition (a variant of a `#[non_exhaustive]` enum) | medium: the lifetime of textures across threads |
| E7 | Interactive effects (section 3.5): the input trait, the host context, the routing through the host's pointer events; the interactive samples. **Done**: `IEffectInputHandler`, `EffectHostContext`, `Effect::construct_interactive`, `InputElement::interactive_effect`, the routing of `input_element_effects.rs`, the pointer spotlight, the reactive grid, the water ripple and the burning flame (values and interactive classes) in the library and on the catalog page (section 3.5, "Interactive effects as built") | addition | medium: event ordering and capture |
| E8 | Samples and catalog pages (section 3.7). **Done**: `samples/CustomEffects` (the gallery application, crate `custom-effects`), the catalog section "Custom effects" with a page per family (section 3.7, "Samples as built") | none (samples) | low |
| E9 | Validation and documentation: render tests GPU against the CPU reference per backend and mode, comparison scenes, the steady-state and prewarm tests, the rows of gpu-rendering.md, a section in winui-composition.md on how WinUI's effect and render subsystems map to FerroUI. **Done**: every effect of the library at 96 and 192 DPI on Skia raster, Graphite and every mode of Vello against its CPU reference, the comparison scenes of `tests/FerroUI.RenderBackends.Comparison`, the steady state of the whole library on both backends, the prewarm by the composition target and a Graphite render target, the rows of gpu-rendering.md, winui-composition.md section 5.4 (section 3.9, "Validation as built") | none | low |

Status: E0 and E1 are merged (#455): the model and the no-op. E2 is done: a custom effect whose
type has a graph is drawn by the render walk as that graph around the content, through the painter
of the composition effects, on every backend (section 3.3, "Graph effects as built"); the colour
effects and the glow of the sample set ship as such effects with CPU references, and the render
tests hold Skia (raster and Graphite) and Vello (CPU, hybrid and GPU modes) to those references.
What each node costs on each backend is the status of gpu-rendering.md. E3 is done: the Win2D
convolution is a node of the graphs (`ConvolveMatrixEffect`, drawn by Skia with its matrix
convolution image filter and by Vello with a pass of its own), sharpen and edge detect ship as
graphs of it with CPU references, and Vello's hybrid and GPU modes draw the colour matrix and
convolution layers (and the multiply layers of lit visuals, the pixel operations and the luminance
mask of the composition effects) on the device: the content recorded into a scene of the device,
rendered into a pooled texture, passed through the program into another pooled texture the outer
scene paints with. The graph effects of the library are GPU on Skia and on Vello's hybrid and GPU
modes; Vello's CPU mode draws them by the references, and WebGL2 keeps the CPU path until step 3
of the route. Another custom effect keeps the no-op of E1.

E4 is done: a custom effect whose type is a runtime shader is drawn by Skia with its SkSL program
on every surface (a runtime shader image filter of a save layer for a content-sampling program, an
overlay drawn into the layer), compiled once per thread and type, its uniforms checked once against
the convention of section 3.4 and set from the snapshot each frame (section 3.4, "Shader effects as
built"); pixelate, scan lines and spotlight ship as such effects with SkSL, WGSL and CPU
references, and the render tests hold Skia raster and Graphite to those references.

E5 is done: Vello's hybrid and GPU modes draw a shader effect with its WGSL, wrapped into the module
of a pass (`ShaderEffectDescription::wgsl_module`), checked by naga against the convention once per
type and compiled once per device and type, on the route of E3: the content a texture of the pool,
the pass writing another the scene paints with, the uniforms written per frame; a module that fails
is reported once and draws the content unchanged. The CPU mode draws the CPU reference. The render
tests hold the three library shader effects to their references in every mode. `ferroui-build`
translates the module to GLSL ES 3.0 at build time for WebGL2 (decision 2), which keeps the CPU
reference until step 3 of the Vello route gives its renderer device textures.

E6 is done: an image parameter (`EffectParameterValue::Image`, a handle `EffectImage` of a bitmap,
compared by identity, `Send + Sync`) is a child of the program of an overlay, `uniform shader
<name>;` in SkSL and `sample_<name>` in WGSL, the image stretched over the area of the effect and
sampled by the nearest texel (section 3.6, "Secondary images as built"). Skia binds image shaders
of images made once per thread, uploaded once by the GPU context; Vello uploads an image once per
device into a texture bound in the pass; both release what they made after the last handle is
dropped, on the thread that owns it, with the next frame. A content-sampling program takes no
images (skia-safe binds no setter for the children of a runtime shader image filter's builder).
The dissolve and mask transitions ship in the library with CPU references, and the render tests
hold them to those references on Skia raster and Graphite and in every mode of Vello, with no
upload over steady frames. Graph effects take no images.

E7 is done: a mutable effect class that implements `IEffectInputHandler` and is constructed with
`Effect::construct_interactive` receives the pointer input of the input element it is the effect
of (section 3.5, "Interactive effects as built"). One class handler of the changes of
`Visual.Effect` attaches it to its host and detaches it when the effect changes, the attachment
holding the guards of its handlers; the events reach it in the order of the routed events (after
the tunnel route, the host's own handling and its descendants' bubble handlers), handled ones only
when it asks for them, a captured pointer wherever it is, and the loss of the capture. The
`EffectHostContext` gives the position in the host, normalized to it and in the coordinates of the
program at the layer's scale, the capture, the invalidation and the frames of the animation clock.
The class changes its value and raises `invalidated`; only the snapshot crosses, so a frame of an
interaction changes uniforms and compiles nothing, which the tests count on both backends. The
pointer spotlight, the reactive grid, the water ripple and the burning flame of Effector's sample
set ship in the library as shader effects with CPU references and as interactive classes, held to
their references on Skia raster and Graphite and in every mode of Vello, and the catalog page
"Custom effects" shows them with the burn-away button.

E8 is done: the samples of section 3.7. `samples/CustomEffects` is the gallery application of the
custom effects, Effector's gallery and compiz sample in one, with a section per family of the
library: each effect in a tile with its code, its effect string, sliders and colours, the
transition of the effect of its host and key frames of `Visual.Effect`; the page transitions
sample pages the application captures once, and a presenter dissolves from one to the other where
it is pressed (section 3.7, "Samples as built"). The ControlCatalog shows the same sections as one
page per family in a section "Custom effects", in place of its single page; its transitions sample
photographs of its assets. The effects are those of E2 to E7, so every tile draws on the GPU of
Skia and Vello, and a slider, a colour or a frame of an animation changes only uniforms and the
members of graphs.

E9 is done, and with it #455: every effect FerroUI ships is held to its CPU reference on Skia raster,
Graphite and the CPU, hybrid and GPU modes of Vello at one and two device pixels per unit, and
compared between Skia and each mode of Vello in the comparison scenes; frames of the whole library
that change only parameters compile no program, make no image, allocate no texture and upload
nothing on either backend; the prewarm of a composition target draws every type of the library after
its first frame, which on Skia compiles every program its first frame needs, and on Vello, which does
not prewarm, each pipeline is made once per device and type (section 3.9, "Validation as built").
gpu-rendering.md records the paths and their tests, and winui-composition.md (section 5.4) maps
WinUI's effect factory, Win2D's effects and pixel shaders and its render walk onto the custom
effects.

#580 (the review of the library against Effector's samples and runtime, 2026-11-06) is fixed. The
water ripple and the burning flame sample the moved content clamped to the area and compose over
the content masked by it (source atop), in their SkSL, WGSL and CPU forms, so the flame cuts no
notches into the top of the button and the ripple neither eats bands into the edges nor fills
rounded corners. The overlays (scan lines, spotlight, pointer spotlight, reactive grid and the new
grid) are masked by the content (`ShaderEffectDescription::masked_by_content`): a runtime blender
on Skia, the pass on Vello, the CPU reference. The reactive grid's lines are 5 percent of a cell,
Effector's `GridShaderEffect` is ported (`grid`: cell 22, strength 0.22, `#3FC3FF`, lines 6
percent of a cell), the pointer is normalized to the rectangle the renderer drew the effect over
(`EffectHostContext::render_bounds`), the sample radii are 56 (ripple) and 21 (flame), and the
four interactive classes take handled events. The lengths stay device independent pixels (the
owner's decision on the units; DEVIATIONS.md, Animation). The gallery shows the ripple on a
picture, an opaque burn-away button and a caption rewritten at most every 200 ms.

## 5. Not in scope

- Weaving, binary patching and the build tasks of Effector: the extension points are part of the
  framework.
- Effector's runtime toggle of direct runtime shaders (`DirectRuntimeShadersEnabled`): the fallback
  is per backend and surface (the CPU reference), not a global switch.
- Fusing the nodes of a graph into one program per factory (the open point of winui-composition.md,
  section 5.3): a later optimization of the shared graph painter, for the composition effects and
  the graph effects alike.

## 6. Decisions

Decided on 2026-11-02 (the owner asked for the decisions to be made on the merits).

| # | Question | Decision | Why |
|---|---|---|---|
| 1 | The shader source of Vello: two sources or a shared IR | **Two sources**: SkSL for Skia and WGSL for Vello, with the one uniform convention of section 3.4, each checked when it is compiled (SkSL by `runtime_effects.rs`, WGSL by naga) against the parameters of the type. Every shader effect that ships in FerroUI (the library and the samples) has a render test that draws it on Skia and on Vello and compares both with its CPU reference (decision 3), so the two programs cannot drift apart unnoticed | Each program stays readable and debuggable in the language of its backend, and Skia's SkSL and Vello's WGSL are the languages their tools report errors in. An IR would limit what an effect can express, add a code generator to maintain, and still need both backends tested. The test against one reference is what makes two sources safe |
| 2 | WebGL2 (Vello's hybrid renderer in the browser has no wgpu device) | **GLSL ES 3.0 generated from the WGSL at build time**, by naga on the build host: a helper of `ferroui-build` (the build dependency an application with compiled markup already has, and the effect crates of FerroUI) translates the WGSL of an effect into GLSL ES in `OUT_DIR`, and the description embeds both. The author writes no third source, and the browser module carries no naga | One authored program per backend family, and no translator in the browser build, whose size #441 is reducing. A third hand-written source would be a third program to keep in step. Skia in the browser (WebGL2 through Ganesh) compiles SkSL itself, so only Vello's WebGL2 path needs this |
| 3 | Vello's CPU mode | **Every effect that ships in FerroUI has a CPU reference** (a function over premultiplied RGBA, as `PixelOperation::apply_to_premultiplied_rgba8`): it is what Vello's CPU mode draws and what the render tests compare the GPU output of both backends with. An application's own effect may leave it out; then Vello's CPU mode draws the content unchanged and reports it once (the no-op of E1) | The CPU path is the reference and the fallback (gpu-rendering.md): without a reference there is nothing to validate the GPU output against, and the rule that every rendering feature is validated on Skia and Vello applies to what FerroUI ships. Requiring it of every application effect would block effects whose authors only target the GPU |
| 4 | Where graph effects (and shader effects) are declared | **Provided associated functions of `CustomEffect`**: `fn graph() -> Option<GraphicsEffect>` and, in E4, `fn shader() -> Option<ShaderEffectDescription>`, both type-level (no `self`; read once per type) and `None` by default | The registry finds the description through the trait the type already implements, so no second registration per type is needed; a separate trait would need one, because Rust has no specialization to detect it. The composition vocabulary is in `ferroui-base`, the crate of `CustomEffect`, so the provided function adds no dependency |
| 5 | Neutral values | **Keep asking the type** (`CustomEffect::neutral`), as E1 does, and record the difference in DEVIATIONS (Animation) | A default instance is not in general an effect without a visible result (a tint's default strength is 0.55), so a transition from no effect that starts at the default would jump; a type without a neutral value switches at half way, as a value that cannot be interpolated does |

With these, no step waits on a decision: E2 declares graph effects by decision 4, E4 and E5 follow decisions 1 to 3.
