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
  which adds the sample radius to each side), and the names of further children, reserved for the
  images of E6: a description with children is not supported yet, reported once and drawn
  unchanged. It is made with `content_sampling(sksl, wgsl)` or `overlay(sksl, wgsl, blend_mode)`
  and `with_sample_radius`, `with_padding`, `with_children`.
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
  `content` for a content-sampling program, none for an overlay). A content-sampling program is
  the runtime shader image filter of a save layer (`image_filters::runtime_shader_with_options`,
  the sample radius in device pixels), the layer bound to `content`; an overlay is drawn over the
  content inside the save layer with its blend mode before the layer is composed. Skia evaluates a
  runtime shader image filter with translations only (`SkRuntimeImageFilter` takes the matrix
  capability `kTranslate` and applies the rest of the transform to the result), so the layer is
  saved under a translation by whole device pixels to the top left of the area, clipped to the
  area, and the content drawn into it with its own transform: the layer has the resolution of the
  device (under a rotation, the area is scaled to the lengths of the axes and Skia transforms the
  result). The layer is composed with the opacity in effect. A program that does not compile,
  does not match, or has children is reported once (an error in the log area `Skia`) and the
  content drawn unchanged; nothing panics. This works on raster, Ganesh and Graphite surfaces and
  in pictures.
- *Vello* draws a shader effect as E1 draws an effect without a factory until E5: the content
  unchanged in a layer without a filter, reported once. Its CPU mode does not run the CPU
  reference yet (E5, with the route of E3).
- *The library* (`media::effects::library`, E4): `pixelate` (`PixelateEffect`, content-sampling:
  each cell of `cell-size` device independent pixels, clamped to 0..64 and at least one device
  pixel, shows the pixel at its centre; sample radius 64), `scanline` (`ScanlineShaderEffect`, an
  overlay drawn source over: the lower half of each band of `spacing` black at `strength`) and
  `spotlight` (`SpotlightShaderEffect`, an overlay screened over the content: a light of `color`
  fading from (`center-x`, `center-y`) to `radius`), Effector's pixelation, scan line and spotlight
  shaders with the clamps of its factories, each with its SkSL, its WGSL and its CPU reference, and
  a neutral value that leaves the content unchanged.

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

### 3.6 Secondary image handles

An image a shader samples besides the content (the two pages of a transition) is a parameter kind,
`EffectParameterValue::Image`, holding a handle that is a value: an `Arc` of the decoded image
(the same shared bitmap the image brushes use), compared by identity. The `Arc` is the lease
Effector implements by hand: the image lives while a snapshot or a frame holds it, and is freed
when the last one drops it, on whatever thread. Each backend uploads an image once and keeps the
texture keyed by the image's identity while it is in use (as the image brushes are,
gpu-rendering.md), so a frame that changes only uniforms uploads nothing. Capturing a page into
such an image is `RenderTargetBitmap` (made by the application, not per frame).

### 3.7 Samples

`samples/CustomEffects` (a gallery application) and the control catalog, one page per effect
family (as the catalog has one page per control): colour effects (tint, grayscale, sepia, saturation,
brightness and contrast, invert), image effects (glow, sharpen, edge detect, pixelate), shader
effects (scanline, grid, spotlight), interactive effects (pointer spotlight, reactive grid, water
ripple, burn-away button) and page transitions (the compiz-style set). Each shows code, markup
(`Effect="tint(...)"`) and an animation, as Effector's gallery does. Effector's sample effects are
ported as the reference set.

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
  unchanged, reported once (`tests.rs`, `gpu/metal/tests.rs`); Vello drawing a shader effect
  unchanged until E5 (`effect_tests.rs`).
- Later steps: per effect and backend, the GPU output against the CPU reference
  (`tests/FerroUI.RenderTests`, `tests/FerroUI.RenderBackends.Comparison`); the steady-state test
  (no new texture or program over frames that change only parameters, with the allocation
  counters of the Vello pass pool and the Skia resource cache); the prewarm of registered effect
  types; input routing with a test root.

## 4. Steps

Each step is one pull request from main.

| Step | Content | Public API | Risk |
|---|---|---|---|
| E0 | This plan | none | none |
| E1 | The open effect model in ferroui-base (section 3.2): `CustomEffect`, `EffectParameterValue`, `ImmutableCustomEffect`, `CustomEffectClass` and `Effect::construct_custom`, `IEffect::as_custom_effect`, `EffectExtensions::to_custom_effect`, `CustomEffectRegistry` and `Effect::parse`, `CustomEffectAnimator` and `EffectTransition`; the no-op and the report of Skia and Vello; tests | additions (a provided method of `IEffect`) | low: the built-in effects take the same paths; an `IEffect` of an unknown kind still panics in `get_effect_output_padding` as upstream |
| E2 | Graph effects (section 3.3, kind 1): the description hook of `CustomEffect`, loaded once per type into an `EffectGraph`, painted around the content by the render walk with the parameters as animatable members, registered for the prewarm; the colour effects of the sample set as graphs. **Done**: `graph`, `for_each_graph_value`, `cpu_reference`, the library of section 3.3, the catalog page "Custom effects" | addition | medium: the walk gains a second way to draw `Visual.Effect`; the graph painter is shared with the effect brushes |
| E3 | Convolution and the Vello colour layers: a `ConvolveMatrixEffect` node (Skia: a matrix convolution image filter; Vello: a WGSL pass), and Vello step 2 of gpu-rendering.md ("layers with an effect": the content into a pooled device texture, a pass into another) for the colour matrix nodes, so the graph effects are GPU on Vello's GPU and hybrid modes. **Done**: `ConvolveMatrix`, `push_convolve_matrix_layer`, the node, sharpen and edge detect; the route for the colour matrix, convolution, light multiply, pixel operation and luminance layers; WebGL2 keeps the CPU path (route step 3) | addition | high: the Vello pass reads the content of a layer, a new route in the scene |
| E4 | Runtime shader effects on Skia (section 3.4): the shader description (SkSL source, the uniform convention, children, blend mode, overlay or content-sampling), compiled once per thread by `runtime_effects.rs`, drawn as a runtime-shader image filter of a save layer; the shader samples. **Done**: `shader`, `ShaderEffectDescription`, the uniform convention, the Skia programs, pixelate, scan lines and spotlight in the library and on the catalog page "Custom effects" (section 3.4, "Shader effects as built") | addition | medium: SkSL errors surface at the first draw (logged once, then the no-op) |
| E5 | Runtime shader effects on Vello: the WGSL source of the description, checked with naga, a pass of `effect_passes.rs` compiled once per device on the route of E3; WebGL2 by GLSL ES generated at build time (decision 2); the CPU reference hook (decision 3) | addition | high: depends on E3 |
| E6 | Secondary images (section 3.6): `EffectParameterValue::Image` and its handle, uploaded once per backend, bound as children; the page transitions | addition (a variant of a `#[non_exhaustive]` enum) | medium: the lifetime of textures across threads |
| E7 | Interactive effects (section 3.5): the input trait, the host context, the routing through the host's pointer events; the interactive samples | addition | medium: event ordering and capture |
| E8 | Samples and catalog pages (section 3.7) | none (samples) | low |
| E9 | Validation and documentation: render tests GPU against the CPU reference per backend and mode, comparison scenes, the steady-state and prewarm tests, the rows of gpu-rendering.md, a section in winui-composition.md on how WinUI's effect and render subsystems map to FerroUI | none | low |

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
references, and the render tests hold Skia raster and Graphite to those references. Vello draws a
shader effect's content unchanged, reported once, until E5 draws its WGSL.

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
