# The WinUI 3 composition API

This document is the inventory and the plan of the port of the WinUI 3 composition API
(`Microsoft.UI.Composition`) and of the XAML pieces that sit on it in microsoft-ui-xaml
(`ConnectedAnimationService`, `ConnectedAnimation`, `ElementCompositionPreview`, the `Translation`
facade of `UIElement`) to FerroUI. It records what FerroUI already has, what the port adds, in which
order, and every place where the result differs from WinUI.

Tracking issue: [#303](https://github.com/wieslawsoltes/FerroUI/issues/303). Pull request: [#304](https://github.com/wieslawsoltes/FerroUI/pull/304).

## 1. Sources

- **The compositor of FerroUI.** `src/FerroUI.Base/rendering/composition` is the port of the
  compositor of the upstream framework: `Compositor`, `CompositionVisual` and its container,
  solid colour, surface, draw list and custom visuals, `CompositionPropertySet`,
  `ExpressionAnimation`, the key frame animations of the schema, implicit animations, animation
  groups, the expression language, the composition brushes, the server/transport split and the
  drawing surfaces. Its object model is generated from `scripts/composition-schema.xml` by
  `scripts/generate_composition.py`. The port builds on it and does not duplicate any of it.
- **The connected animations of the upstream framework.** `src/FerroUI.Controls/animation` is the
  port of the upstream `ConnectedAnimationService`, which flies a control over the overlay layer
  with UI-thread animations. The WinUI port replaces its engine with the composition engine of
  WinUI and keeps its public members (section 5.1).
- **microsoft-ui-xaml**, branch `winui3/main`, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`
  (local checkout `/Users/wieslawsoltes/GitHub/microsoft-ui-xaml`, read-only, MIT). The XAML-side
  code is ported from it:
  - `dxaml/xcp/components/animation/ConnectedAnimation.cpp`, `ConnectedAnimationService.cpp` and
    their headers: the state machine, the rectangles, the snapshot brushes and the sprite
    animations of a connected animation;
  - `dxaml/xcp/dxaml/lib/ConnectedAnimation*_partial.cpp`, `GravityConnectedAnimationConfiguration_partial.cpp`,
    `DirectConnectedAnimationConfiguration_partial.cpp`, `BasicConnectedAnimationConfiguration_partial.h`:
    the public classes and the three configurations;
  - `dxaml/xcp/core/core/elements/ConnectedAnimationRoot.cpp`: the layer the animations fly in;
  - `dxaml/xcp/dxaml/lib/ElementCompositionPreview.cpp`: the `ElementCompositionPreview` statics.
- **`Microsoft.UI.Composition`** is closed source (it ships in the Windows App SDK). It is ported
  from its public API reference (learn.microsoft.com, the `Microsoft.UI.Composition` namespace of
  the Windows App SDK 1.6) and from how microsoft-ui-xaml uses it (`DCompTreeHost`, `HWCompNode`,
  the connected animation code). Where the reference does not say how something behaves, the port
  says what it chose in the deviations (section 7).

The upstream framework name never appears in `src`, `samples` or `tests`; this page and the
`scripts/` folder may name it.

## 2. How the WinUI types map onto FerroUI

| WinUI | FerroUI |
|---|---|
| `Microsoft.UI.Composition` | `ferroui_base::rendering::composition` (the compositor and its objects) |
| `Microsoft.UI.Xaml.Hosting.ElementCompositionPreview` | `ferroui_controls::hosting::ElementCompositionPreview` (it reaches `ScrollViewer`, a control); the visual-tree half stays `ferroui_base::rendering::composition::ElementComposition` |
| `Microsoft.UI.Xaml.Media.Animation.ConnectedAnimation*` | `ferroui_controls::animation` |
| `UIElement` | `Visual` (the element that has a composition visual) |
| handoff visual (`GetElementVisual`) | the `CompositionDrawListVisual` the compositing renderer creates for a visual |
| hand-in visual (`SetElementChildVisual`) | the child composition visual of a visual, shown after its children |
| `ICompositionEasingFunction` | `ferroui_base::animation::easings::Easing` (`Rc<dyn IEasing>` inside); `CompositionEasingFunction` has the WinUI factories |
| `Vector2`, `Vector3`, `Vector4`, `Quaternion`, `Matrix3x2`, `Matrix4x4` | `ferroui_base::numerics` (the expression language types); visual properties keep the types of the compositor (`Vector`, `Vector3D`, `Matrix`) |
| `TimeSpan` | `std::time::Duration` |
| events (`Completed`) | handler lists with a disposable registration, as elsewhere in FerroUI |

Property names used by `StartAnimation` and by expressions are the WinUI names (`Offset`,
`Opacity`, `Scale`, `Translation`, `Offset.X`, ...), so an expression or a target written for
WinUI works unchanged.

## 3. Inventory

Status: **has** (present with the WinUI members and behaviour), **partial** (present, some
members or behaviour missing, said in the notes), **missing** (not in FerroUI before this port).
The last column is the phase of section 4 that brings the type to **has**, or the decision when
it does not.

### 3.1 Objects, compositor, property sets

| Type / member | Status | Notes | Phase |
|---|---|---|---|
| `CompositionObject.Compositor`, `Dispose`, `ImplicitAnimations` | has | `CompositionObject` | - |
| `CompositionObject.StartAnimation`, `StopAnimation`, `StartAnimationGroup`, `StopAnimationGroup` | has | Sub-channel targets of vector properties (`Offset.X`, `Scale.XY`, components X, Y, Z, W in any case) animate those components only; sub-channels of one property run together, an animation of the whole property replaces them. `StopAnimation` applies the stop behavior. | 1 |
| `CompositionObject.Properties` | partial | `CompositionObject::properties()`: a live property set of the object, created on first use. An expression reads its entries through a reference to the set (`set_reference_parameter("props", obj.properties())`, then `props.Name`), not as members of the object (`obj.Name`). | 3 |
| `CompositionObject.Comment` | has | `comment`, `set_comment`; empty by default. | 3 |
| `CompositionObject.TryGetAnimationController`, `StartAnimation(name, animation, controller)`, `Compositor.CreateAnimationController`, `AnimationController` (`Pause`, `Resume`, `Progress`, `PlaybackRate`, `ProgressBehavior`, `MaxPlaybackRate`, `MinPlaybackRate`), `AnimationControllerProgressBehavior` | partial | `try_get_animation_controller`, `start_animation_with_controller`, `create_animation_controller`. The controller is a live property set (`Progress`, `PlaybackRate` animatable, the controller referenceable from expressions) that the server instance reads every frame: a paused animation stands still and does not end, a rate scales (a negative one reverses) its time, a new `Progress` moves it there (with or without the delay). Key frame and spring animations can be controlled; expression animations ignore a controller. `Progress` reads the value last assigned, not the position of the running animation; `TryGetAnimationController` gives a controller for any animation started on the property through the object and not stopped, also one that ended by itself. | 3 |
| `CompositionPropertySet` `Insert*`, `TryGet*` | has | Scalar, vector, matrix, quaternion, colour, boolean. | - |
| `Compositor.Create*` for the types below | partial | The factories of the existing types exist; the new types bring theirs. | 1-6 |
| `Compositor.CreatePropertySet` | has | `create_property_set`. | 1 |
| `Compositor.CreateScopedBatch`, `GetCommitBatch`, `CompositionScopedBatch`, `CompositionCommitBatch` | has | `create_scoped_batch`, `get_commit_batch`, `CompositionBatchTypes`. Key frame animations started while a batch is active take a completion guard that the server releases when the animation ends, is stopped or replaced, or its target is disposed; `Completed` is raised on the thread of the compositor. Expression animations belong to no batch. | 1 |
| `Compositor.RequestCommitAsync` | has | `request_commit_async`. | - |
| `Compositor.GlobalPlaybackRate`, `MinGlobalPlaybackRate`, `MaxGlobalPlaybackRate` | has | `global_playback_rate`, `set_global_playback_rate` (0 to 1000, applied from the next frame), `MIN_GLOBAL_PLAYBACK_RATE`, `MAX_GLOBAL_PLAYBACK_RATE`. The server scales the clock the animations start and are evaluated on. | 3 |

### 3.2 Visuals

| Type / member | Status | Notes | Phase |
|---|---|---|---|
| `Visual.Offset`, `Size`, `Opacity`, `IsVisible`, `AnchorPoint`, `CenterPoint`, `RotationAngle`, `Orientation`, `Scale`, `Parent` | has | `CompositionVisual`; `IsVisible` is `visible`. | - |
| `Visual.Translation` (property set entry in WinUI) | has | A real animatable property of every visual. | - |
| `Visual.TransformMatrix` | partial | A 2D `Matrix`, not a `Matrix4x4`; the 3D part is ignored by a 2D renderer. | deviation |
| `Visual.Clip` (`CompositionClip`) | has | `composition_clip` / `set_composition_clip` on every visual (`clip` is the geometry clip of the upstream compositor); applied by the render walk in the space of the visual, after its transform, and part of the own clip of the visual (bounds, dirty rects). | 1 |
| `Visual.RotationAngleInDegrees`, `RotationAxis` | has | Degrees: a view of `RotationAngle` (not an animation target). `RotationAxis` (animatable, default `(0, 0, 1)`): an axis along Z (or zero) rotates in the plane, a negative Z turns the other way; any other axis is folded into the orientation on the server, of which the 2D renderers show the projection. | 2 |
| `Visual.RelativeOffsetAdjustment`, `RelativeSizeAdjustment` | has | Animatable; the server's size and offset of a visual are `Size + RelativeSizeAdjustment * parent size` and `Offset + RelativeOffsetAdjustment * parent size` (X, Y), used for the transform, the content, the clips and the bounds, and followed when the size of the parent changes or the visual moves to another parent. `effective_size()` on the UI thread (hit testing). | 2 |
| `Visual.IsHitTestVisible` | has | Default `true`; composition hit testing skips the visual and its subtree. | 2 |
| `Visual.IsPixelSnappingEnabled` | partial | The translation of the transform relative to the parent is rounded to device pixels of the target, not the position in the window. | 2 / deviation |
| `Visual.BorderMode`, `BackfaceVisibility`, `CompositeMode` | partial | `CompositionBorderMode`, `CompositionBackfaceVisibility`, `CompositionCompositeMode`; stored only: a 2D renderer antialiases every edge, has no back face, and blends subtrees with source-over only. | deviation |
| `Visual.ParentForTransform` | has | The transform of the visual is taken in the space of that visual: the own transform times the transform from it to the parent (through their common ancestor), computed in the adorner pass and followed through the ancestor transform tracker of the adorners. The two must be in one tree (otherwise the parent is used); clips and opacity still come from the parent chain. | 2 |
| `ContainerVisual.Children` (`InsertAbove`, `InsertBelow`, `InsertAtTop`, `InsertAtBottom`, `Remove`, `RemoveAll`, `Count`) | has | `CompositionVisualCollection`. | - |
| `SpriteVisual` (`Brush`, `Shadow`) | has | `SpriteVisual`, `create_sprite_visual`. The shadow of its rectangle, then the brush over `(0, 0, size)`, then the children; brush and shadow are observed render resources. Does not clip to its bounds when created, as a WinUI visual. | 1 |
| `ShapeVisual` (`Shapes`, `ViewBox`) | has | `ShapeVisual`, `create_shape_visual`. Draws its shapes in order, through the view box, clipped to its size (a shape visual without a size shows nothing), then its children; its content bounds are its size. Does not clip its children when created. The shape collection and the view box are observed render resources: a change of a shape, its geometry or brushes, an animation included, redraws the visual. | 2 |
| `LayerVisual` (`Effect`, `Shadow`) | has | `LayerVisual`, `create_layer_visual`. The children are composed into a layer (`push_layer` over the bounds of the subtree) that the opacity of the visual applies to. `Shadow`: through a drop shadow effect of the drawing context when it has effects (Skia, Vello), so it follows the alpha of the composed children, and the bounds grow by the padding of the effect; otherwise the box shadow of the rectangle of the visual under the layer. `Effect` (`effect`, `set_effect`, a `CompositionEffectBrush`; the schema member is `EffectBrush`, apart from the upstream `Effect` of every visual): the composed children are the first source parameter of the graph in graph order; what the graph draws under them and what wraps them (blurs, opacities, the other side of a cross-fade or composite) is drawn and pushed before the children, inside the layer or shadow, and popped and drawn after them; the bounds grow by the padding of the blurs around the layer. A graph without a source parameter is drawn whole without the children; an effect that did not load shows nothing. No implicit clip. | 2, 6 (`Effect`) |
| `RedirectVisual` (`Source`) | has | `RedirectVisual`, `create_redirect_visual`, `create_redirect_visual_with_source_visual`. Draws the content and children of the source in its own coordinates, under its own children, as a visual surface draws them (the visibility, opacity, transform and clip of the source itself ignored), through a hidden `CompositionVisualSurface` that follows the changes of the source and guards against recursion; its bounds cover the source as the update pass leaves them. A redirect visual inside its own source draws nothing. No implicit clip. | 2 |

### 3.3 Brushes and surfaces

| Type / member | Status | Notes | Phase |
|---|---|---|---|
| `CompositionColorBrush` (`Color`) | has | `CompositionSolidColorBrush`; `create_color_brush` and `create_color_brush_with_color` are the WinUI-named factories. | - |
| `CompositionGradientBrush` (`ColorStops`, `ExtendMode`, `MappingMode`, `InterpolationSpace`, brush transform) | has | `color_stops` / `set_color_stops` (the gradient stops; the list ships when assigned), `create_color_gradient_stop`. `extend_mode`: a view of the spread method (`Clamp` = `Pad`, `Wrap` = `Repeat`, `Mirror` = `Reflect`). `mapping_mode` (default `Relative`): the unit of the points the WinUI members set; changing it keeps the numbers of the points and changes their unit. `interpolation_space`: stored only (the drawing contexts interpolate in RGB). The brush transform (`AnchorPoint`, `CenterPoint`, `Offset`, `RotationAngle`, `Scale`, `TransformMatrix`, animatable; `RotationAngleInDegrees` a view) reaches the drawing context as the `Transform` (after the upstream transform, about the upstream `TransformOrigin`, by default the top left corner of the area) and, for the anchor, the `RelativeTransform` of the server brush. | 2 |
| `CompositionLinearGradientBrush` (`StartPoint`, `EndPoint`) | has | `start_point_vector`, `end_point_vector` (WinUI `Vector2`, in the unit of the mapping mode) over the upstream relative points. Animations target the upstream `StartPoint` and `EndPoint` (relative points). The default `EndPoint` is the upstream `(0, 0)`. | 2 |
| `CompositionRadialGradientBrush` (`EllipseCenter`, `EllipseRadius`, `GradientOriginOffset`) | has | `ellipse_center`, `ellipse_radius`, `gradient_origin_offset` over `Center`, `RadiusX/Y` and `GradientOrigin` (the centre plus the offset; moving the centre keeps the offset). | 2 |
| `CompositionSurfaceBrush` (`Surface`, `Stretch`, alignment ratios, `BitmapInterpolationMode`, transform, `SnapToPixels`) | has | `CompositionSurfaceBrush`, `create_surface_brush`, `create_surface_brush_with_surface`. Painted by the visual that shows it (`paint_composition_brush` on the server), not a brush of the drawing context. `CompositionStretch` gains `Uniform` and `UniformToFill`. The interpolation mode maps to the one quality setting of the drawing context. | 1 |
| `CompositionNineGridBrush` | has | `create_nine_grid_brush`; `Source`, the four insets and inset scales (animatable), `IsCenterHollow`, `set_insets` / `set_insets_with`, `set_inset_scales` / `set_inset_scales_with`. Painted by `paint_composition_brush`: a surface brush source is cut at the natural size of its surface (its stretch, alignment and transform are not applied) and each cell drawn into its cell of the area; a colour source fills the cells; any other brush is painted over the area clipped to the cells. Insets that do not fit are scaled down to meet (`nine_grid_rects`). | 2 |
| `CompositionMaskBrush` (`Source`, `Mask`) | partial | `create_mask_brush`. A colour or gradient mask is the opacity mask of the drawing context; a surface (or other composition) brush mask is painted over the source in a layer with destination-in bitmap blending, so only the bitmaps it draws mask. | 2 |
| `CompositionBackdropBrush` | partial | `create_backdrop_brush`, `CompositionBackdropBrush`. Draws nothing, alone and as a source of an effect: the drawing contexts cannot read back what was drawn under a rectangle (the experimental acrylic material is a tinted fill, not a backdrop). | 6 / deviation |
| `CompositionHostBackdropBrush` | missing | Needs the desktop behind the window. | deviation |
| `CompositionEffectFactory` (`CreateBrush`, `LoadStatus`, `ExtendedError`), `Compositor.CreateEffectFactory` (with and without animatable properties), `CompositionEffectFactoryLoadStatus` | has | `create_effect_factory(effect)`, `create_effect_factory_with_properties(effect, &["Blur.BlurAmount"])`; `load_status` (`Success`, `EffectTooComplex`, `Pending`, `Other`) and `extended_error` (`CompositionEffectFactoryError`: `EffectTypeNotSupported`, `CompositeModeNotSupported`, `MissingSource`, `InvalidAnimatableProperty`, `EffectTooComplex`). The graph is checked when the factory is made; a factory that did not load still makes brushes, which draw nothing. More than 256 nodes or 32 levels is `EffectTooComplex`. Not a composition object. | 6 (scoped) |
| `CompositionEffectBrush` (`GetSourceParameter`, `SetSourceParameter`, `Properties`, `StartAnimation` of `EffectName.Property`) | has | `CompositionEffectBrush`: `get_source_parameter`, `set_source_parameter` (an unknown name panics), `source_parameter_names`, `properties`. Painted by `paint_composition_brush` over the area of the visual: a source parameter paints its bound brush, a Gaussian blur pushes the blur effect of the drawing context around its source (clipped to the area; unblurred on a context without effects), an opacity pushes an opacity, a cross-fade paints `source1` with `1 - weight` and `source2` with `weight` over it, a composite paints its sources in order, a colour source fills. The animatable properties are entries of `Properties`, inserted with the values of the description; a change or an animation of an entry (through `start_animation` on the brush or on `properties()`) redraws the brush (the server brush follows the `changed` notification of its property set). | 6 (scoped) |
| Effect descriptions (Win2D `IGraphicsEffect`, `IGraphicsEffectSource`, `CompositionEffectSourceParameter`) | partial | `effects::GraphicsEffect` and `GraphicsEffectSource`, with `GaussianBlurEffect` (`BlurAmount` as the standard deviation, converted to the blur radius of the drawing contexts with `blur_amount_to_radius`; `BorderMode`, `Optimization` stored only), `OpacityEffect`, `CrossFadeEffect` (`Weight`, also `CrossFade`), `CompositeEffect` (`SourceOver` only), `ColorSourceEffect` (`Color`), and `SaturationEffect`, which loads as `Other` / `EffectTypeNotSupported` (no colour matrix in the drawing contexts). `CompositionEffectSourceParameter` is the leaf bound to a brush. Not ported: the other Win2D effects (tint, colour matrix, blend, shadow, ...). | 6 (scoped) / deviation |
| `ICompositionSurface` | has | `ICompositionSurface` (UI thread), implemented by `CompositionSurface` (and so the drawing surface), `CompositionVisualSurface` and `LoadedImageSurface`; on the server `IServerCompositionSurface` gains `natural_size` and `draw`, which the surface visual uses too. | 1 |
| `CompositionVisualSurface` (`SourceVisual`, `SourceOffset`, `SourceSize`) | has | `create_visual_surface`. Live: the source subtree is rendered where the surface is drawn, ignoring the visibility, opacity, transform and clip of the source itself; changes of the subtree are followed while the source is attached to a target. A surface that shows an ancestor of its sprite is not drawn inside itself. `freeze()` (not public in WinUI) renders the source once into a layer. | 1 |
| `CompositionDrawingSurface` | partial | The upstream class takes imported GPU images; WinUI draws with Direct2D interop, which has no counterpart. | deviation |
| `LoadedImageSurface` (XAML) | partial | `LoadedImageSurface::from_bitmap` and `start_load_from_stream` (decoded at once, `load_status` instead of `LoadCompleted`); `natural_size`, `decoded_size`, `decoded_physical_size`. No URI loading and no decode size. | 1 |

### 3.4 Shapes and geometry

| Type / member | Status | Notes | Phase |
|---|---|---|---|
| `CompositionShape` (`CenterPoint`, `Offset`, `RotationAngle`, `RotationAngleInDegrees`, `Scale`, `TransformMatrix`) | has | `CompositionShape`; `Vector2` members as `Vector`, `Matrix3x2` as `Matrix`; the transform is animatable and composed as the one of a visual (matrix, scale and rotation about the center point, offset), in the space of the parent shape. `RotationAngleInDegrees` is a view of `RotationAngle` (not an animation target). | 2 |
| `CompositionShapeCollection` | has | `CompositionShapeCollection` (`append`, `add`, `insert`, `remove`, `remove_at`, `get`, `set`, `clear`, `count`); a render resource on the server that observes its shapes. | 2 |
| `CompositionSpriteShape` (`Geometry`, `FillBrush`, `StrokeBrush`, `StrokeThickness`, dash array, caps, join, miter limit, `IsStrokeNonScaling`) | partial | `create_sprite_shape`, `create_sprite_shape_with_geometry`. Colour and gradient brushes are brushes of the drawing context (`draw_geometry` with a brush and a pen); a surface brush fill is painted inside a geometry clip of the geometry, a surface brush stroke inside the widened outline. `StrokeThickness`, `StrokeDashOffset`, `StrokeMiterLimit` are animatable. `CompositionStrokeCap` and `CompositionStrokeLineJoin` map to the pen: one cap per pen (the start cap; end and dash caps are stored only), `Triangle` drawn as `Round`, `Miter` drawn as miter-or-bevel. The dash array is a plain `Vec<f32>` in multiples of the thickness. A non-scaling stroke divides the thickness by the scale of the transform (exact for uniform scales). | 2 |
| `CompositionContainerShape` (`Shapes`) | has | `create_container_shape`; its shapes are drawn in order under its transform (nesting is limited to 256 levels). | 2 |
| `CompositionGeometry` (`TrimStart`, `TrimEnd`, `TrimOffset`) | has | Animatable. The outline is cut with `try_get_segment` of the platform geometry (the fractions of `contour_length`), the offset wraps around (a wrapped part is two figures of a geometry group); the trim points are clamped to 0..1 and swapped when the start is after the end. A platform geometry that cannot be cut is drawn whole. Trimming applies to the fill and to a geometric clip too. | 2 |
| `CompositionRectangleGeometry`, `CompositionRoundedRectangleGeometry`, `CompositionEllipseGeometry`, `CompositionLineGeometry`, `CompositionPathGeometry` + `CompositionPath` | has | `create_rectangle_geometry`, `create_rounded_rectangle_geometry`, `create_ellipse_geometry`, `create_line_geometry`, `create_path_geometry`, `create_path_geometry_with_path`. The server builds the platform geometry with the render interface of the compositor (rectangle, ellipse and line geometries; a stream geometry for rounded corners, whose radii are limited to half the size) and caches it until a member changes. `CompositionPath` is made from a FerroUI `Geometry` (`from_geometry`), a platform geometry (`new`) or path data (`parse`), not from a Win2D `IGeometrySource2D`. | 2 |
| `CompositionViewBox` | has | `create_view_box`; `Offset`, `Size` and the alignment ratios are animatable, `Stretch` defaults to `Fill`. Laid out with the stretch and alignment math of the surface brush; what is outside the mapped view box is clipped; a view box with a zero size maps nothing. | 2 |

### 3.5 Clips

| Type / member | Status | Notes | Phase |
|---|---|---|---|
| `CompositionClip` (`AnchorPoint`, `CenterPoint`, `Offset`, `RotationAngle`, `RotationAngleInDegrees`, `Scale`, `TransformMatrix`) | has | `CompositionClip`; the transform is animatable; `AnchorPoint` is relative to the size of the visual; `RotationAngleInDegrees` is a view of `RotationAngle` (not an animation target). | 1 |
| `InsetClip` (`LeftInset`, `TopInset`, `RightInset`, `BottomInset`) | has | `create_inset_clip`, `create_inset_clip_with_insets`; the insets are animatable and their animations re-render the visual. | 1 |
| `RectangleClip` (edges and corner radii) | has | `create_rectangle_clip`, `create_rectangle_clip_with`, `create_rectangle_clip_with_radii`; a rounded clip pushes a rounded rectangle. | 1 |
| `GeometricClip` (`Geometry`, `ViewBox`) | has | `create_geometric_clip`, `create_geometric_clip_with_geometry`; pushes a geometry clip in the render walk, the view box maps the geometry onto the size of the visual before the transform of the clip; part of the own clip (bounds, dirty rects). A change of the geometry (its animations and trimming included) or of the view box changes the clip. A geometric clip without a geometry clips nothing. | 2 |

### 3.6 Lights and shadows

| Type / member | Status | Notes | Phase |
|---|---|---|---|
| `DropShadow` (`BlurRadius`, `Color`, `Offset`, `Opacity`, `Mask`, `SourcePolicy`) | has | `create_drop_shadow`; `BlurRadius`, `Color`, `Offset`, `Opacity` animatable, a change of the mask brush redraws. On a sprite visual: without a mask the box shadow of the rectangle of the visual (the bounds of the visual include it); a colour or gradient `Mask` is the opacity mask of that box shadow; any other mask (surface, nine-grid, mask brush) is painted through a drop shadow effect in a layer and then removed with destination-out bitmap blending, so only its shadow stays; `InheritFromVisualContent` (which wins over the mask) draws the brush through a drop shadow effect. On a layer visual: the shadow of the composed children, or of the mask with the `Default` policy. Without effects in the drawing context the box shadow of the rectangle. `ShapeVisual` has no shadow in WinUI either. The Z of the offset is ignored. | 1 (rectangular), 5 (masks) |
| `CompositionLight` (`Targets`, `ExclusionsFromTargets`, `IsEnabled`), `VisualUnorderedCollection` (`Add`, `Remove`, `RemoveAll`, `Count`) | has | `CompositionLight` with the typed handles of its classes; `targets()` and `exclusions_from_targets()` are `VisualUnorderedCollection`s created with the light (`add` keeps a visual once, `contains`, `items`). A light lights its targets and their subtrees minus the exclusions and their subtrees (an exclusion that is a target is excluded); a disabled or disposed light lights nothing. A change of the light or its collections, an animation included, redraws the subtrees of its old and new targets. | 5 |
| `AmbientLight` (`Color`, `Intensity`), `DistantLight` (`Color`, `CoordinateSpace`, `Direction`, `Intensity`), `PointLight` (`Color`, attenuations, `CoordinateSpace`, `Offset`, `Intensity`, `Min/MaxAttenuationCutoff`), `SpotLight` (attenuations, `CoordinateSpace`, `Direction`, inner and outer cone angle, colour and intensity, `Offset`, cutoffs), `Compositor.Create*Light` | partial | `create_ambient_light`, `create_distant_light`, `create_point_light`, `create_spot_light`; every member but `CoordinateSpace` animatable; `*ConeAngleInDegrees` are views of the radians. Diffuse Lambert lighting of flat visuals facing +Z (`server_composition_light.rs`): the sum of colour times intensity, attenuation `1 / (c + l d + q d^2)` and `N.L` of the lights that reach a visual, evaluated in the coordinate space of each light (the target when none). The content of each lit visual is drawn into a layer and a light map (a bitmap of at most 128 x 128 samples) over it with source-atop blending: exact for white and grey light, a tint for coloured light on coloured content, never brighter than the content; a layer visual itself is not lit (its children are). | 5 |

### 3.7 Animations and easing

| Type / member | Status | Notes | Phase |
|---|---|---|---|
| `ScalarKeyFrameAnimation`, `Vector2/3/4KeyFrameAnimation`, `ColorKeyFrameAnimation`, `QuaternionKeyFrameAnimation`, `BooleanKeyFrameAnimation` | has | From the schema. | - |
| `KeyFrameAnimation` `DelayBehavior`, `DelayTime`, `Direction`, `Duration`, `IterationBehavior`, `IterationCount`, `InsertExpressionKeyFrame` | has | | - |
| `KeyFrameAnimation.StopBehavior` | has | Applied by `StopAnimation` (upstream stores it and does not apply it); an animation that is replaced leaves the current value. | 1 |
| `KeyFrameAnimation.KeyFrameCount` | has | `key_frame_count`. | 1 |
| `ColorKeyFrameAnimation.InterpolationColorSpace` (`CompositionColorSpace`) | has | `interpolation_color_space`: `Auto` and `Rgb` interpolate the stored channels, `Hsl` hue (the short way), saturation and lightness, `RgbLinear` the channels in linear light, `HslLinear` HSL of the linear-light colour. `Auto` is RGB. | 3 |
| `PathKeyFrameAnimation` | missing | Needs `CompositionPath` (phase 2). | 3 |
| `ExpressionAnimation` (`Expression`) | has | | - |
| `CompositionAnimation.Set*Parameter`, `SetReferenceParameter`, `ClearParameter`, `ClearAllParameters`, `Target` | has | | - |
| `CompositionAnimation.SetBooleanParameter`, `SetExpressionReferenceParameter` | has | `set_boolean_parameter`; `set_expression_reference_parameter` takes a composition object (`Rc<dyn AsCompositionObject>`) and is resolved as `set_reference_parameter` (WinUI's `IAnimationObject` is not ported, see the deviations). | 3 |
| `CompositionAnimation.InitialValueExpressions` (`InitialValueExpressionCollection`) | has | `initial_value_expressions()`: the map (`insert`, `lookup`, `has_key`, `remove`, `clear`, `size`, `get_view`). Parsed when the animation starts (an expression that cannot be parsed fails the start); on the server, before the instance is initialized, each expression is evaluated against the target and the parameters (`this.StartingValue` = the property's value, the animated property read as it was before the start) and the named property of the target is set; the animated property starts from its initial value. Key frame, expression and spring animations. | 3 |
| `CompositionAnimationGroup` | has | | - |
| `ImplicitAnimationCollection` | has | | - |
| `NaturalMotionAnimation` (`DelayBehavior`, `DelayTime`, `StopBehavior`), `ScalarNaturalMotionAnimation`, `Vector2NaturalMotionAnimation`, `Vector3NaturalMotionAnimation` (`FinalValue`, `InitialValue`, `InitialVelocity`), `SpringScalarNaturalMotionAnimation`, `SpringVector2NaturalMotionAnimation`, `SpringVector3NaturalMotionAnimation` (`DampingRatio`, `Period`), `Compositor.CreateSpring*Animation` | has | `create_spring_scalar_animation`, `create_spring_vector2_animation`, `create_spring_vector3_animation`. Each component moves on a damped harmonic oscillator (closed form, `spring_state`) from `InitialValue` (or the current value) with `InitialVelocity` to `FinalValue` (or the new value of an implicit start, or the current value); the animation ends when every component is within 1e-4 of the size of the motion and nearly at rest, sets the final value and releases its batches. Defaults chosen: `DampingRatio` 1, `Period` 50 ms. A spring with damping 0 is an infinite animation. Works on visual properties (a `Vector3` spring on `Offset` evaluates to `Vector3D`) and property set entries, explicit or implicit. The `Bounce*NaturalMotionAnimation` classes of newer SDKs are not ported. | 3 |
| `CompositionScopedBatch`, `CompositionCommitBatch` | has | See 3.1. | 1 |
| `CubicBezierEasingFunction` (`ControlPoint1`, `ControlPoint2`) | has | The curve of `SplineEasing`. The easing functions are `IEasing` implementations, not composition objects; `CompositionEasingFunction` has the factories of WinUI 1.4+ and `Compositor` the `Create*EasingFunction` factories. | 1 |
| `LinearEasingFunction` | has | `LinearEasingFunction` (the curve of `LinearEasing`). | 1 |
| `StepEasingFunction` (`StepCount`, `InitialStep`, `FinalStep`, `IsInitialStepSingleFrame`, `IsFinalStepSingleFrame`) | has | The steps share the duration; a single-frame step is shown only at the start or the end. | 1 |
| `BackEasingFunction` (`Mode`, `Amplitude`), `BounceEasingFunction` (`Mode`, `Bounces`, `Bounciness`), `CircleEasingFunction` (`Mode`), `ElasticEasingFunction` (`Mode`, `Oscillations`, `Springiness`), `ExponentialEasingFunction` (`Mode`, `Exponent`), `PowerEasingFunction` (`Mode`, `Power`), `SineEasingFunction` (`Mode`) | has | The parametric curves of the XAML easing functions, with `CompositionEasingFunctionMode`; read-only members as in WinUI. | 1 |

### 3.8 Expression language

| Feature | Status | Notes | Phase |
|---|---|---|---|
| Operators (`+ - * / %`, comparisons, `&& || !`, `?:`), literals, member access, swizzles (`.XY`), `this.StartingValue`, `this.CurrentValue`, `this.FinalValue`, `this.Target` | has | `expressions/` | - |
| Functions `Abs`, `ACos`, `ASin`, `ATan`, `Ceil`, `Clamp`, `ColorLerp`, `ColorLerpRGB`, `ColorRGB`, `Concatenate`, `Cos`, `Distance`, `DistanceSquared`, `Floor`, `Inverse`, `Length`, `LengthSquared`, `Lerp`, `Ln`, `Log10`, `Matrix3x2`, `Matrix4x4`, `Max`, `Min`, `Mod`, `Normalize`, `Pow`, `Quaternion`, `Round`, `Scale`, `Sin`, `Slerp`, `SmoothStep`, `Sqrt`, `Square`, `Tan`, `ToDegrees`, `ToRadians`, `Transform`, `Vector2`, `Vector3`, `Vector4` | has | Every overload of the reference ("Expression Functions per Type" of `ExpressionAnimation`), plus those the WinUI expression engine accepts without listing them (the expression builder of the Windows Community Toolkit emits them): scalar `Length`, `LengthSquared`, `Distance`, `DistanceSquared`, `Scale`; per-component `Mod` of vectors; `Transform(Vector4, Matrix4x4)`. Also `Lerp` of `Matrix3x2` and `Matrix4x4` (not in the reference). `Matrix3x2(...)` and `Matrix4x4(...)` take each component from its own argument (the upstream implementation repeated the fifth). | 3 |
| Function names | has | Case-insensitive (`lerp`, `LERP`, `Asin` and `ASin` are the same function), as in WinUI, whose own expression builder writes them in lower case; also the builder's names `ColorHslLerp` and `ColorRgbLerp`. Keywords (`this.Target`, `Pi`, `True`) were already case-insensitive. | 3 |
| `ColorHsl`, `ColorLerpHSL` | has | `ColorHsl(h, s, l)`: hue in radians (0 to 2 pi, wrapping), saturation and lightness 0..1 (clamped), opaque. `ColorLerpHSL(c1, c2, p)`: `c1` at 0, `c2` at 1 as `ColorLerp`; saturation, lightness, alpha linear, hue the short way round, an achromatic colour takes the other's hue. | 3 |
| `Matrix3x2.CreateFromScale`, `CreateFromTranslation`, `CreateRotation`, `CreateScale`, `CreateSkew`, `CreateTranslation`; `Matrix4x4.CreateFromAxisAngle`, `CreateFromScale`, `CreateFromTranslation`, `CreateScale`, `CreateTranslation`; `Quaternion.CreateFromAxisAngle` | has | The static factories of the reference, with its formulas; a dotted name is parsed as one function name. `CreateTranslation` is `CreateFromTranslation`, as the reference says (upstream created a scale). Also the `System.Numerics` overloads: `Matrix3x2.CreateScale(s)`, `(x, y)`, `(scale, center)`, `CreateTranslation(x, y)`, `CreateRotation(angle, center)`, `CreateSkew(x, y)`; `Matrix4x4.CreateScale(s)`, `(x, y, z)`, `(scale, center)`, `CreateTranslation(x, y, z)`, `CreateRotationX/Y/Z(angle)` and `(angle, center)` (not in the reference). `Matrix4x4.CreateFromQuaternion` and the yaw-pitch-roll factories are not in the reference and not added. | 3 |
| Member, swizzle and parameter names | partial | Case-sensitive here (`Offset.X`, `.XY`, parameter keys). Whether WinUI matches them ignoring case is not documented. | - |
| Reference to an `InteractionTracker`, a visual's property set (`visual.Translation` after `SetIsTranslationEnabled`), a pointer property set | partial | Visuals and property sets can be referenced; the tracker comes with phase 4. | 4 |

### 3.9 Interaction

| Type / member | Status | Notes | Phase |
|---|---|---|---|
| `InteractionTracker` (position, scale, velocities, bounds, resting values, decay rates, `TryUpdate*`, states, owner callbacks) | partial | `interactions::InteractionTracker`: `Create`, `CreateWithOwner`, every value property, `InteractionSources`, `TryUpdatePosition` (with `InteractionTrackerClampingOption`), `TryUpdatePositionBy`, `TryUpdatePositionWithAnimation`, `TryUpdatePositionWithAdditionalVelocity`, `TryUpdateScale`, `TryUpdateScaleWithAnimation`, `TryUpdateScaleWithAdditionalVelocity`, request ids, the four states with a rubber band past the boundaries while interacting. Runs on the UI thread with the animation frames of the media context and publishes its values into its property set (`properties`); a reference to the tracker in an expression is a reference to that set. Custom animations are evaluated on the UI thread. Missing: tracker binding (`SetBindingMode`, `IsFromBinding` is always `false`), `TryUpdatePosition` with `InteractionTrackerPositionUpdateOption`, `AdjustPositionXIfGreaterThanThreshold`/`Y`. | 4 |
| `IInteractionTrackerOwner` | has | A trait with no-op defaults; callbacks are queued and delivered after the change, never nested. | 4 |
| `InteractionTrackerInertiaRestingValue`, `InteractionTrackerInertiaMotion`, `InteractionTrackerInertiaNaturalMotion`, `Configure*InertiaModifiers` | partial | Resting value and motion modifiers for X, Y and scale, conditions evaluated with `this.Target` as the tracker. Missing: `InteractionTrackerInertiaNaturalMotion` and `ConfigureVector2PositionInertiaModifiers` (they need the natural motion animations of phase 3). | 4 |
| `VisualInteractionSource` (`TryRedirectForManipulation`, source modes, rails, chaining, wheel configuration) | partial | `interactions::VisualInteractionSource`: `Create` (plus `create_for_element`), source modes, rails, `ManipulationRedirectionMode` and `PointerWheelConfig` (mouse wheel, Ctrl + wheel scales), the input values (`DeltaPosition`, `Position`, ...). `TryRedirectForManipulation` takes the pressed event args (or a pointer point and the element) and follows the routed pointer events of the element. Chaining modes are stored only; pinch input, touchpad redirection and expression references to a source are missing. | 4 |

### 3.10 XAML pieces

| Type / member | Status | Notes | Phase |
|---|---|---|---|
| `ConnectedAnimationService.GetForCurrentView` | has | `get_for_current_view`: the service of the thread; `get_for_top_level` (upstream) stays. | 1 |
| `ConnectedAnimationService.DefaultDuration`, `DefaultEasingFunction` | has | 300 ms; without a default easing the composition engine uses WinUI's cubic Bezier (0.8, 0), (0.2, 1). | 1 |
| `ConnectedAnimationService.PrepareToAnimate`, `GetAnimation` | has | Upstream members; the engine behind them is WinUI's (`connected_animation_flight.rs`). | 1 |
| `ConnectedAnimation.TryStart(destination)`, `TryStart(destination, coordinatedElements)` | has | The port of `CConnectedAnimation`: a frozen image of the source and a live visual surface of the destination on sprite visuals in the connected animation layer of the window, cross-faded, with offset, scale and inset clip animations in a scoped batch; coordinated elements enter from where they would be relative to the source. The flight of the upstream service over the overlay layer remains for a window without a compositor. | 1 |
| `ConnectedAnimation.Cancel`, `IsScaleAnimationEnabled`, `SetAnimationComponent`, `Configuration`, `Completed` | has | `Completed` carries the upstream `cancelled` flag. | 1 |
| `ConnectedAnimationConfiguration`, `GravityConnectedAnimationConfiguration` (`IsShadowEnabled`), `DirectConnectedAnimationConfiguration`, `BasicConnectedAnimationConfiguration` | has | The gravity dip (`GetEffectPropertySet`: offset and scale key frames combined with the primary animation by expressions) and its shadow; the direct configuration with 200 ms and its decelerating curve; the basic one without effect. | 1 |
| `ConnectedAnimationComponent` (`OffsetX`, `OffsetY`, `CrossFade`, `Scale`) | has | `ferroui_controls::animation::ConnectedAnimationComponent`. | 1 |
| `ElementCompositionPreview.GetElementVisual`, `GetElementChildVisual`, `SetElementChildVisual` | has | `ferroui_controls::hosting::ElementCompositionPreview` over `ElementComposition`. | 1 |
| `ElementCompositionPreview.SetImplicitShowAnimation`, `SetImplicitHideAnimation` | has | Show when the element is attached or becomes visible; hide when it becomes invisible (the element stays visible until the animation completes) or is removed from the tree (on an image of it). | 1 |
| `ElementCompositionPreview.SetIsTranslationEnabled` | has | Recorded; translation is always enabled. | 1 |
| `ElementCompositionPreview.GetScrollViewerManipulationPropertySet` | has | `Translation`, `Pan`, `Scale`, `CenterPoint`, `Matrix`; updated with the offset of the scroll viewer. | 1 |
| `ElementCompositionPreview.GetPointerPositionPropertySet` | has | `Position`, updated by the pointer moves of the window. | 1 |
| `UIElement.Translation`, `TranslationTransition` (`Vector3Transition`) | has | `Visual::translation` (a `Vector3D`) writes the `Translation` of the element's composition visual; `translation_transition` animates it (see the transitions row). | 1 (`Translation`), 3 (transitions) |
| `UIElement.Rotation`, `RotationAxis`, `Scale`, `CenterPoint`, `TransformMatrix` | partial | Plain properties of `Visual` (`rotation`, `rotation_axis`, `scale`, `center_point`, `transform_matrix`, with the `numerics` types), written to the element's composition visual whenever the element is rendered again; a facade never set leaves its composition property to the application. `Rotation` around the Z axis is `RotationAngle` (radians), around another axis `Orientation` (a quaternion), of which the 2D renderers show only the projection; a zero axis is the Z axis. `TransformMatrix`: its 2D affine part (`M11`, `M12`, `M21`, `M22`, `M41`, `M42`) follows the render transform in the composition `TransformMatrix`, which the composition visual applies before `Scale`, the rotation and `Translation` (WinUI applies it after the other facades). | 3 |
| `UIElement.OpacityTransition`, `RotationTransition`, `ScaleTransition`; `ScalarTransition` (`Duration`), `Vector3Transition` (`Duration`, `Components`), `Vector3TransitionComponents` | has | `ferroui_base::{ScalarTransition, Vector3Transition, Vector3TransitionComponents}` (values, copied when assigned; 300 ms and all components by default) and `Visual::set_opacity_transition`, `set_rotation_transition`, `set_scale_transition`, `set_translation_transition`. An implicit animation collection on the element's composition visual (`Opacity`, `RotationAngle` and `Orientation`, `Scale`, `Translation`): a key frame animation to `this.FinalValue` over the duration with the compositor's default easing. A `Vector3Transition` of some components starts with a key frame at 0 that holds the animated components at `this.StartingValue` and sets the others to `this.FinalValue`, so those jump at once; one of no component installs nothing. The collection is installed after the first synchronization of a new composition visual (the initial values do not animate) and before the properties are written afterwards, and replaces implicit animations the application set on the element's visual. | 3 |

## 4. Plan

The phases are ordered by value to an application: what a page of an application uses most
(connected animations between a list and a detail view, the element APIs) comes first, then the
visual building blocks, then the animation and easing breadth, then the rarer pieces. Each phase is
implemented with unit tests next to `compositor_tests.rs` and `render_thread_tests.rs` (or in the
test file of its folder), lands in the pull request of the port, and updates the inventory above.

### Phase 1: connected animations and the element APIs

What a connected animation needs, and the `ElementCompositionPreview` statics.

1. **Completion of animations**: `CompositionScopedBatch` and `CompositionCommitBatch`. An
   animation started while batches are active takes a completion guard of each; the server
   releases the guard when a key frame animation ends, is stopped or is replaced, and the
   batch raises `Completed` on the UI thread once it is ended and all its guards are released.
   `StopBehavior` is applied when an animation is stopped. Sub-channel targets (`Offset.X`,
   `Scale.XY`) for `StartAnimation`.
2. **Easing**: `CompositionEasingFunction` with the WinUI factories (cubic Bezier, linear, step,
   back, bounce, circle, elastic, exponential, power, sine, each with its mode and parameters),
   producing the `Easing` objects the key frames take.
3. **Sprite visual and surfaces**: `SpriteVisual` (`Brush`, `Shadow`), `CompositionSurfaceBrush`,
   `CompositionVisualSurface` (live), `LoadedImageSurface` (from a bitmap), `DropShadow` for
   rectangular content, the clips (`CompositionClip`, `InsetClip`, `RectangleClip`) on every
   visual.
4. **`ElementCompositionPreview`**: the WinUI facade over `ElementComposition`, the implicit
   show and hide animations (also when the element leaves the tree), `SetIsTranslationEnabled`,
   the scroll viewer manipulation and pointer position property sets; the `Translation`
   property of a visual.
5. **Connected animations**: the WinUI engine (states, rectangles, snapshot and live brushes on a
   sprite visual in a connected animation layer at the top of the window, the offset, scale,
   clip and cross-fade animations, the gravity, direct and basic configurations,
   `IsScaleAnimationEnabled`, `SetAnimationComponent`, coordinated elements, the timeout,
   `Cancel`, `Completed`) behind the upstream service's API.
6. **Catalog**: a "Composition (WinUI)" page with a list and a detail view joined by a connected
   animation, implicit show and hide animations, a pointer-driven light-like effect from the
   pointer position property set and a parallax header from the scroll viewer property set.
   Verified on macOS with the Skia and Vello backends.

### Phase 2: visuals, brushes and shapes

`ShapeVisual`, `CompositionSpriteShape`, `CompositionContainerShape`, the geometries with trimming,
`CompositionViewBox`, `GeometricClip`; `LayerVisual`, `RedirectVisual`; `CompositionNineGridBrush`,
`CompositionMaskBrush`; the WinUI members of the gradient brushes (mapping mode, `Vector2` points,
ellipse centre and radius, the brush transform); the remaining `Visual` members
(`RotationAngleInDegrees`, `RotationAxis` around Z, the relative adjustments, `IsHitTestVisible`,
`IsPixelSnappingEnabled`, `ParentForTransform`).

### Phase 3: animation and expression breadth

Spring natural motion animations; `AnimationController` and the global playback rate;
`CompositionObject.Properties` and `Comment`; `SetBooleanParameter`, `SetExpressionReferenceParameter`,
`InitialValueExpressions`; `ColorKeyFrameAnimation.InterpolationColorSpace`;
`PathKeyFrameAnimation`; the missing expression functions (`ColorHsl`, `ColorLerpHSL`, the
`Matrix3x2`, `Matrix4x4` and `Quaternion` factories); the `UIElement` facades and their
transitions (`Rotation`, `Scale`, `CenterPoint`, `TransformMatrix`, `ScalarTransition`,
`Vector3Transition`).

### Phase 4: interaction

`InteractionTracker` with its owner callbacks, states, bounds, inertia (decay rates, resting value,
motion and natural motion modifiers) and its `TryUpdate*` requests, exposed to expressions as a
referenceable object (`tracker.Position`, `tracker.Scale`); `VisualInteractionSource` fed by the
pointer and wheel input of the element whose visual it is created for, after
`TryRedirectForManipulation`.

### Phase 5: shadows and lights

`DropShadow` with a mask and the `SourcePolicy`, shadows of shape and layer visuals; the lights
(`AmbientLight`, `DistantLight`, `PointLight`, `SpotLight`) with targets and exclusions, as diffuse
lighting of flat visuals.

### Phase 6: effects (scoped)

`CompositionEffectBrush` and `CompositionEffectFactory` for the effects the drawing contexts of
FerroUI can express (Gaussian blur, opacity, cross-fade, a drop shadow), and
`CompositionBackdropBrush` on the backdrop support of the acrylic visual. The full Win2D effect
graph (the dozens of `Microsoft.Graphics.Canvas.Effects` classes and arbitrary graphs of them) is
out of scope: FerroUI has no Win2D, its drawing context has a small set of image effects, and a
general effect graph needs a shader pipeline per backend (Skia and Vello). An unsupported effect
makes `CreateEffectFactory` report `CompositionEffectFactoryLoadStatus::Other`, with
`CompositionEffectFactoryError::EffectTypeNotSupported` as its extended error (WinUI's load status
has no such value; WinUI throws for an effect it cannot load).

As implemented: the effects are Gaussian blur, opacity, cross-fade, source-over composite and colour
source (a saturation effect is described but does not load). The effect brush is painted by the
visuals that show it, as the surface brush; its animatable properties are entries of its property
set. `LayerVisual.Effect` binds the composed children to the first source parameter. A drop shadow
effect (Win2D `ShadowEffect`, which outputs the shadow alone) is not ported: the drop shadow effect of
the drawing contexts draws the content with its shadow. `CompositionBackdropBrush` exists and draws
nothing, as the drawing contexts cannot read back the pixels behind a visual (the acrylic visual
draws a tinted fill, not a blurred backdrop); `CompositionHostBackdropBrush` is not ported.

## 5. Design notes

### 5.1 Connected animations

The upstream service and its public members stay: `ConnectedAnimationService::get_for_top_level`,
`prepare_to_animate`, `get_animation`, `default_duration`, `default_easing_function`;
`ConnectedAnimation::try_start`, `try_start_with_coordinated_elements`, `configuration`,
`completed` (with the `cancelled` flag), `is_consumed`, `dispose`. WinUI's members are added
(`get_for_current_view`, `cancel`, `is_scale_animation_enabled`, `set_animation_component`). The
engine is WinUI's:

- `prepare_to_animate` records the clipped and unclipped rectangles of the source in the
  coordinates of the window and freezes an image of the source (WinUI freezes a visual surface
  of the source on the next frame and keeps the source in unloading storage until then; here the
  image is rendered at once, see the deviations).
- `try_start` moves the animation to *started*; the first frame after the destination's layout
  computes its rectangles, builds the host sprite in the connected animation layer of the
  window (a container visual above the content, WinUI's `ConnectedAnimationRoot`), hides the
  source and destination elements, and starts the offset, scale, clip and cross-fade animations
  in a scoped batch whose completion ends the animation.
- The gravity configuration adds its dip (offset and scale key frames through a property set
  combined with the primary animation by expressions, as `GetEffectPropertySet` does) and a
  shadow; the direct configuration uses 200 ms and a decelerating curve; the basic one adds
  nothing.

### 5.2 Element visual properties

The compositing renderer writes `Offset`, `Size`, `Opacity`, `IsVisible`, the clip and
`TransformMatrix` of an element's visual whenever the element is rendered again, as WinUI's layout
writes the handoff visual. `Translation`, `Scale`, `CenterPoint`, `RotationAngle`, `Orientation`
and `AnchorPoint` are left to the application, except those whose facade on the visual
(`set_translation`, `set_scale`, `set_center_point`, `set_rotation`, `set_rotation_axis`) was set:
the renderer then writes them too, as WinUI writes the handoff visual from the facades. An implicit animation on `Offset` therefore animates
every layout move of the element, as in WinUI.

## 6. Status

| Phase | State | Pull request |
|---|---|---|
| 1 | implemented; verified in the catalog on macOS with Skia and Vello (section 8) | #304 |
| 2 | implemented | #304 |
| 3 | implemented (springs, controller, properties, colour spaces, expressions, facades and transitions) | #304 |
| 4 | implemented (the tracker on the UI thread) | #304 |
| 5 | implemented (drop shadow masks and source policy; ambient, distant, point and spot lights as a light map over the content) | #304 |
| 6 | implemented (scoped: blur, opacity, cross-fade, composite and colour source effect brushes, the layer visual effect; the backdrop brush draws nothing) | #304 |

## 7. Deviations

Every difference from WinUI, in the format of `DEVIATIONS.md` (Representation, Behaviour,
Missing). `DEVIATIONS.md` points here for this area.

| WinUI | Port | Kind | Why | Since |
|---|---|---|---|---|
| `Visual.TransformMatrix` is a `Matrix4x4`. | A 2D `Matrix`. | Representation | The renderers of FerroUI are 2D; the property is the one the compositor already had. | phase 1 |
| `ConnectedAnimation` freezes a `CompositionVisualSurface` of the source on the next frame and keeps the source element in unloading storage until then. | The source is rendered to a bitmap when `prepare_to_animate` is called, and the bitmap is shown by a `LoadedImageSurface`. | Behaviour | A detached element's composition visual loses its content in the same batch, and the server applies every queued batch before it runs jobs, so a server-side freeze could see the detached state. The image is the same; it is taken a frame earlier. | phase 1 |
| `SetExpressionReferenceParameter(String, IAnimationObject)`. | Takes `Rc<dyn AsCompositionObject>` and is the same as `set_reference_parameter`; an element facade passes its visual. | Representation | `IAnimationObject` and `PopulatePropertyInfo` are not ported. | phase 3 |
| `InitialValueExpressions` is an `InitialValueExpressionCollection` that is itself a `CompositionObject` (`IMap<String, String>`). | A plain map owned by the animation, with the members of the map. | Representation | It needs no server counterpart or identity of its own. | phase 3 |
| The hue path of `ColorLerpHSL` is not documented. | The hue takes the short way round, and an achromatic colour takes the hue of the other colour. | Behaviour | The port had to choose. | phase 3 |
| The case of member, swizzle and parameter names in expressions is not documented. | Function names are matched ignoring case (as the expression builder of the Windows Community Toolkit writes them in lower case); member, swizzle and parameter names are matched with case. | Behaviour | Function names follow the toolkit; the rest is unverified and left as it was. | phase 3 |
| `Matrix4x4.CreateRotationX/Y/Z` and the `System.Numerics` overloads of `CreateScale`, `CreateTranslation`, `CreateRotation`, `CreateSkew` and `Lerp` of matrices are not in the expression reference. | Accepted. | Addition | They are the factories of the numeric types the expressions use; an expression valid in WinUI is valid here. | phase 3 |
| A `CompositionPropertySet` is a live object of the compositor whose entries can be animated. | The same. The parameters of an animation are a property set without a server counterpart, copied when the animation starts (the upstream behaviour). | Representation | See `DEVIATIONS.md` for the difference from upstream. | phase 1 |
| The `Completed` handler of a batch receives `(sender, args)`. | The args only. | Representation | The event convention of FerroUI; the batch need not be kept alive by the application. | phase 1 |
| One commit batch per frame and batch type. | One per frame, which waits for the union of the requested types. | Behaviour | Animations started before `GetCommitBatch` must be included. | phase 1 |
| Whether expression animations belong to a batch is not documented. | They belong to no batch, not even an `InfiniteAnimation` one: they never end. | Behaviour | Unverified against WinUI. | phase 1 |
| The `Effect` batch type waits for effects to load. | Accepted and never waited on. | Behaviour | Effect factories load at once (phase 6). | phase 1 |
| A single-frame initial or final step of `StepEasingFunction` lasts one frame. | It lasts no time: it shows at the start or the end only. How the duration is split between steps is the port's reading. | Behaviour | An easing function does not know frames. | phase 1 |
| Easing functions are `CompositionObject`s. | Plain `IEasing` values (`Easing`); the `owner` argument of the factories is not used. | Representation | Key frames take `Easing`. | phase 1 |
| Stopping `Offset.X` while `Offset` is animated as a whole is not documented. | Does nothing. Starting `Offset.X` while `Offset.XY` runs replaces the whole `XY` channel. | Behaviour | Simplification of an undocumented case. | phase 1 |
| Animations run on a visual that is not in a tree. | They wait until the server object is active (attached to a composition target), and so does the `Completed` of their batch. | Behaviour | The upstream compositor activates animations with the tree; a property set is always active. | phase 1 |
| `InteractionTracker` runs in the compositor. | It runs on the UI thread and ticks with `MediaContext::request_animation_frame` while in inertia or a custom animation; custom animations and modifier expressions are evaluated there, and a reference parameter that is not a property set reads as missing. | Behaviour | The compositor of FerroUI has no input and no callbacks to the UI thread. Expressions that bind visuals to the tracker run on the compositor as usual. | phase 4 |
| The values of an `InteractionTracker` are its properties; its `Properties` belongs to the application. | The values live in the tracker's property set under the WinUI names. | Representation | A reference to the tracker resolves to that live property set. | phase 4 |
| The inertia decay rate and curve are not documented. | `v(t) = v0 * (1 - rate)^t`, resting at `p + v / ln(1 / (1 - rate))`; inertia motion `target + (start - target) * r^t`; `InteractionTrackerInertiaMotion.Motion` is the value of the axis each frame; overpan is a rubber band capped at 120 px; a custom animation that ends out of bounds returns through inertia; scale custom animations are clamped. | Behaviour | The port had to choose; the literal `rate^t` moves about 19.5 times the velocity. | phase 4 |
| Owner callbacks of the tracker are raised by the compositor; `IsFromBinding`. | Queued and delivered after the change, never nested; input-driven callbacks carry the last request id; `IsFromBinding` is always `false`. | Behaviour | No re-entrancy; tracker binding is not ported. | phase 4 |
| `VisualInteractionSource` receives input from the compositor after `TryRedirectForManipulation`. | It follows the routed pointer events of the element of the visual (handled ones too), captured to the element, for any pointer type; the element is found from the visual or given to `create_for_element`. The mouse wheel scrolls 50 px per notch and Ctrl+wheel scales by 1.1. Chaining modes are stored only; the input values of a source cannot be referenced from expressions. | Behaviour | The compositor of FerroUI has no input. | phase 4 |
| `InteractionTrackerInertiaNaturalMotion`, `ConfigureVector2PositionInertiaModifiers`, tracker binding, `InteractionTrackerPositionUpdateOption`, `AdjustPositionX/YIfGreaterThanThreshold`, pinch and touchpad redirection. | Not ported. | Missing | Natural motion animations come with phase 3; FerroUI has no touchpad input and no two-pointer recognizer. | phase 4 |
| `CConnectedAnimation` waits for a layout transition element of an ancestor of the destination to end its animation before it completes (`ReadyToComplete`), and adjusts the rectangles of a right-to-left element (`GetTightInnerBounds`). | Neither: FerroUI has no layout transition elements; the rectangles are the bounds of the element in the window, and the content of a right-to-left element is placed by its own transform. | Missing | No counterpart. | phase 1 |
| The rectangles of a connected animation include the parent scale and a flip of the element (the brush is mirrored for a negative scale). | The rectangles are the axis-aligned bounds in the window; the images are not mirrored. | Missing | Rare; the images follow the element's own rendering. | phase 1 |
| The cross-fade of a connected animation is a `CrossFadeEffect` brush with the weight `Crossfade.Weight`. | Two sprites in the host whose opacities are bound by expressions to the `Weight` entry of a property set, animated from 0 to 1 (a custom `CrossFade` component animates that entry). | Representation | The engine came before the effect brushes of phase 6; two sprites need no effect support of the drawing context. | phase 1 |
| The shadow of the gravity configuration is a `ThemeShadow` cast by the host, whose depth follows the Z of the dip. | A drop shadow under the images whose blur radius, opacity and offset are bound by expressions to the Z of the dip. | Behaviour | No theme shadows; the shadow grows and fades with the dip. | phase 1 |
| The coordinated elements of a connected animation cast the shadow of the gravity configuration, shaped by the alpha of their images. | Coordinated elements cast none. | Behaviour | The shadow of the port is the rectangle of the host; around a text element it would show as a band. | phase 1 |
| A prepared connected animation times out after 2 s; the upstream service disposes an unconsumed animation after 3 s. | The composition engine times out after 2 s (WinUI); the flight over the overlay layer keeps the 3 s of upstream. | Behaviour | Each engine keeps its source's timing. | phase 1 |
| `PrepareToAnimate` throws for a source that is not in the tree. | The animation is returned and its flight is the upstream one (which does nothing without bounds). | Behaviour | The upstream API returns an animation. | phase 1 |
| Connected animations use 1 ms when animations are disabled in the system settings. | The system setting is not consulted. | Missing | FerroUI has no such platform setting. | phase 1 |
| The pointer position property set of the root scroll viewer is that of the root visual. | The set of any element follows the pointer over the whole window. | Behaviour | The same coverage, without the special case. | phase 1 |
| The manipulation property set of a scroll viewer is updated by the compositor during a manipulation. | It is updated on the UI thread when the offset changes, in the frame in which the content moves. | Behaviour | Scrolling of FerroUI runs on the UI thread. | phase 1 |
| `Visual.Clip` takes a `CompositionClip`. | `composition_clip` / `set_composition_clip`; it may be set together with the geometry clip of the upstream compositor (`clip`), and the two intersect. | Representation | `clip` is the upstream geometry clip already. | phase 1 |
| The reference box of `CompositionClip.AnchorPoint` is not documented. | Relative to the size of the visual. | Behaviour | The port had to choose. | phase 1 |
| `RotationAngleInDegrees` of clips and of the surface brush is an animatable property. | A view of `RotationAngle`, not an animation target. | Missing | Only the radians are stored. | phase 1 |
| `CompositionSurfaceBrush` is a brush like the others. | It is painted by the sprite (and later shape) visual itself, not handed to the drawing context; the upstream `Transform` and `RelativeTransform` of composition brushes are not applied to it; in a draw list it draws nothing. | Representation | The drawing contexts have no brush for a composition surface. | phase 1 |
| `BitmapInterpolationMode` has separate minification, magnification and mipmap filters. | Mapped to the single bitmap quality of the drawing context; `Linear` leaves the quality of the context unchanged. | Behaviour | The drawing context has one setting. | phase 1 |
| `DropShadow` has its own `CompositionShadow` base; its `Offset.Z` matters with lights. | No separate `CompositionShadow` type; the Z of the offset is ignored. | Missing | No 3D lighting of shadows. | phase 1 |
| `CompositionVisualSurface` follows its source wherever it is. | Changes of the source are followed while the source is attached to a composition target; a surface that shows an ancestor of its own sprite skips the inner drawing of itself. `freeze()` (not public in WinUI) renders the source into a layer once, with the bounds of the last frame. | Behaviour | Live updates hook into the update pass of the target. | phase 1 |
| `LoadedImageSurface` loads from a URI or a stream, asynchronously, with a decode size and `LoadCompleted`. | Made from a `Bitmap`, or decoded from a stream at once; `load_status` instead of the event; no URI loading and no decode size. | Missing | The surface is backed by a bitmap. | phase 1 |
| A WinUI clip limits the hit testing of composition visuals. | It does not. | Missing | The hit testing of the compositor knows the geometry clip only. | phase 1 |
| The defaults of `DampingRatio` and `Period` of the spring animations are not documented. | 1 and 50 ms. | Behaviour | Chosen, not verified against WinUI. | phase 3 |
| When a spring comes to rest is not documented. | When every component is within 1e-4 of the size of the motion and the velocity within that amount times the angular frequency; a damping ratio of 0 never rests. | Behaviour | The port had to choose. | phase 3 |
| `AnimationController.Progress` reads the position of the running animation. | It reads the value last assigned. | Behaviour | The UI thread does not read the server. | phase 3 |
| `TryGetAnimationController` returns null once the animation has ended. | It returns the controller of any animation started with one through the object and not stopped, also one that ended by itself. | Behaviour | The UI thread does not know when an animation ends. | phase 3 |
| A paused animation at its end, and a jump of `Progress`. | A paused animation never ends; any assignment of `Progress` moves the animation there, also to the same value. | Behaviour | Scrubbing keeps the animation alive. | phase 3 |
| The bounds of the playback rates are not documented. | ±1000 for a controller; 0 to 1000 for the global rate (global time does not run backwards). | Behaviour | Chosen. | phase 3 |
| `CompositionColorSpace.Auto` is not documented. | Interpolates in RGB. | Behaviour | The behaviour of the animation before the member existed. | phase 3 |
| The entries of `CompositionObject.Properties` are readable as `obj.Name` in expressions. | They are read through a reference to the property set itself (`props.Name`). | Missing | Kept to the property set. | phase 3 |
| An expression animation under an `AnimationController`. | Ignores the controller. | Behaviour | Not documented; an expression has no time. | phase 3 |
| The `Bounce*NaturalMotionAnimation` classes. | Not ported. | Missing | Outside the requested surface. | phase 3 |
| `CompositionPath` is made from an `IGeometrySource2D`. | Made from a FerroUI `Geometry`, a platform geometry or path data; it keeps the platform geometry it was made with. | Representation | There is no Win2D. | phase 2 |
| `CompositionStrokeDashArray` is a `CompositionObject` (`IVector<float>`). | A `Vec<f32>` replaced as a whole; dashes and the offset are in multiples of the thickness. | Representation | It needs no identity; the units are the port's reading. | phase 2 |
| Start, end and dash caps of a stroke are separate; `CompositionStrokeCap.Triangle`. | The pens have one cap: the start cap (the others are stored); a triangle cap is drawn round. | Missing | The pens of FerroUI have one line cap and no triangle. | phase 2 |
| `Miter` and `MiterOrBevel` joins differ. | Both draw a miter that becomes a bevel past the limit. | Behaviour | The pens of the platform. | phase 2 |
| `IsStrokeNonScaling`. | The thickness is divided by the square root of the determinant of the shape and view box transform. | Behaviour | Exact for uniform scales only. | phase 2 |
| The edge cases of trimming are not documented. | Trim points are clamped to 0..1 and swapped when the start is after the end; a wrapped part is two figures; a geometry the platform cannot cut is drawn whole; trimming applies to fills and geometric clips too. | Behaviour | The port had to choose. | phase 2 |
| The clipping of a `ShapeVisual` and a view box of zero size are not documented. | Shapes are clipped to the size of the visual (without a size nothing shows) and to the mapped view box; a view box of zero size maps nothing; its `Stretch` defaults to `Fill`. | Behaviour | The port's reading. | phase 2 |
| `GeometricClip` without a geometry. | Clips nothing. | Behaviour | Not documented. | phase 2 |
| `RotationAngleInDegrees` of shapes, visuals and gradient brushes is animatable. | A view of `RotationAngle`. | Missing | Only the radians are stored. | phase 2 |
| `Visual.RotationAxis` rotates in 3D. | An axis other than Z is folded into `Orientation`, of which the 2D renderers show the projection. | Behaviour | 2D renderers. | phase 2 |
| `IsPixelSnappingEnabled` snaps to device pixels in the window. | Rounds the translation relative to the parent. | Behaviour | No world transform at that point. | phase 2 |
| `BorderMode`, `BackfaceVisibility`, `CompositeMode`. | Stored only. | Missing | 2D drawing contexts with source-over blending. | phase 2 |
| `ParentForTransform` across trees. | Computed through the common ancestor; clips and opacity still come from the parent chain; without a common ancestor the parent is used. | Behaviour | The adorner pass of the compositor. | phase 2 |
| `LayerVisual.Shadow` follows the alpha of the composed content. | Where the drawing context has effects; elsewhere the box shadow of the rectangle of the visual. | Behaviour | The capabilities of the drawing context. | phase 2 |
| `RedirectVisual` shows its source as if it were its child. | It ignores the visibility, opacity, transform and clip of the source; it draws nothing inside its own source; when the source comes after it in the tree, a change of the extent of the source reaches its bounds one frame late. | Behaviour | It reuses the rendering of the visual surface; bounds follow the update pass. | phase 2 |
| A nine-grid brush over a surface brush honours that brush's stretch, alignment and transform. | The surface is cut at its natural size; those members are ignored. | Behaviour | The nine rectangles are cut from the surface itself. | phase 2 |
| A mask brush masks with any brush. | A colour or gradient mask works through an opacity mask; a surface (or other composition) mask masks through the bitmaps it draws, in a layer with destination-in blending. | Missing | The drawing contexts blend bitmaps only. | phase 2 |
| The transform of a gradient brush. | Applied after the upstream `Transform`, about the upstream `TransformOrigin` (the top left corner by default, as in WinUI). | Representation | It goes through the `Transform` and `RelativeTransform` of the drawing context. | phase 2 |
| The `Vector2` members of the gradient brushes (`StartPoint`, `EllipseCenter`, ...) are animatable; `EndPoint` defaults to (1, 0). | Animations target the upstream relative-point properties; `EndPoint` keeps the upstream default (0, 0). | Missing / Behaviour | The upstream properties are relative points. | phase 2 |
| `CompositionGradientBrush.InterpolationSpace`. | Stored; the drawing contexts interpolate in RGB. | Missing | No other interpolation in the backends. | phase 2 |
| Lights multiply the content by the light. | Each lit visual's own content goes into a layer over which a light map (a bitmap of up to 128 x 128 samples, one per 4 units) is drawn with source-atop blending: exact for white or grey light over white content; a coloured light tints coloured content; the light is clamped to 1 per channel (never brightens). | Behaviour | The drawing contexts blend only bitmaps; multiply would fill transparent areas with the light. | phase 5 |
| A light lights the subtree of a target. | The own content of each visual of the subtree is lit in its own layer; the own content of a layer visual is not lit (its children are); the shadow of a sprite is lit with its content. | Behaviour | The layers of the render walk. | phase 5 |
| `CoordinateSpace` null; the alpha of a light colour; attenuation cutoffs; the falloff between the cones of a spot light; the defaults of the max cutoff, the cone angles and the direction. | The targeted visual whose subtree holds the lit visual; the alpha scales the light; closer than the min cutoff as at the min cutoff, beyond the max cutoff no light; colour lerped from the inner to the outer cone with a smoothstep fade to the outer edge; `f32::MAX`, pi/4 and 0, `(0, 0, -1)`. | Behaviour | Not documented; chosen. | phase 5 |
| A light reacts when its coordinate space visual moves. | A move of the coordinate space visual alone does not redraw the lit visuals; the light map is rebuilt each time a lit visual is drawn. | Missing | Only the light, its collections and its targets are observed. | phase 5 |
| `Inner/OuterConeAngleInDegrees` are animatable; specular lighting. | Views of the radians; diffuse lighting only. | Missing | Only the radians are stored; no normals. | phase 5 |
| A colour or gradient `DropShadow.Mask`. | An opacity mask over the box shadow; a gradient is spread over the shadow area, not blurred. | Behaviour | No shadow-only effect mode. | phase 5 |
| The shadow of a bitmap mask or of the content (`SourcePolicy.InheritFromVisualContent`). | With effects in the drawing context: the source is drawn inside a drop shadow effect in a layer, then removed with destination-out blending (only bitmaps are removed); `InheritFromVisualContent` wins over the mask. Without effects: the box shadow of the rectangle. On a layer visual with a mask: the shadow of the mask over the rectangle under a plain layer. | Behaviour | The drop shadow effect of the drawing context draws its source too. | phase 5 |
| `CreateEffectFactory` takes any `IGraphicsEffect` and throws for one it cannot load. | An enum of six descriptions (Gaussian blur, opacity, cross-fade, composite, colour source, saturation); a factory is always made and reports `Other` or `EffectTooComplex` with a typed `extended_error`, and its brushes draw nothing. | Representation | No Win2D; no exceptions. | phase 6 |
| `CompositionEffectFactory` is a composition object. | A plain object. | Representation | Nothing to animate. | phase 6 |
| Saturation, tint, colour matrix, blend, the Win2D `ShadowEffect`, composite modes other than `SourceOver`. | Not loaded (saturation) or not ported. | Missing | The drawing contexts have no colour matrix and no blend modes; their drop shadow effect draws the content too. | phase 6 |
| `GaussianBlurEffect` `BorderMode`, `Optimization`, `BlurAmount` (a standard deviation). | Stored only; the blur is soft-edged and clipped to the area of the brush; the amount is converted to the radius of the drawing context as `(amount - 0.5) / 0.288675`, and 0.5 or less is no blur; without effects in the drawing context the source is drawn unblurred. | Behaviour | One blur in the drawing contexts. | phase 6 |
| `CrossFadeEffect` adds its weighted sources. | Draws `source2` over `source1`, each with its weight. | Behaviour | No additive blend. | phase 6 |
| Animation controllers of effect properties started through the brush; effect properties readable as members of the brush in expressions. | Start them through `properties()`; read them through a reference to that property set. | Missing | The properties live in the brush's property set. | phase 6 |
| `CompositionBackdropBrush`, `CompositionHostBackdropBrush`. | The backdrop brush draws nothing; the host backdrop brush is not ported. | Missing | The drawing contexts cannot read back what is under a rectangle, nor the desktop. | phase 6 |
| `LayerVisual.Effect`. | The composed children are the first source parameter, in graph order and once; a graph without a parameter hides the children; an effect that did not load shows nothing. The member is the schema property `EffectBrush` (every visual already has the upstream `Effect`). | Behaviour | The walk draws the children once. | phase 6 |
| An effect brush in a draw list or as the fill of a shape. | Draws nothing, as a surface brush. | Representation | It is painted by sprite and layer visuals. | phase 6 |
| A sub-channel is a property followed by a component (`Offset.X`). | A name whose part after the dot is not made of X, Y, Z and W is a property name (`Blur.BlurAmount`). | Behaviour | The animatable properties of effect brushes are dotted names. | phase 6 |
| `ScalarTransition` and `Vector3Transition` are classes: a change of `Duration` after the assignment reaches the element. | Copyable values, copied when assigned; a change is made by assigning again. | Representation | No identity or change notification is needed. | phase 3 |
| `UIElement.RotationAxis` rotates in 3D. | An axis other than Z is written to `Orientation`, of which the 2D renderers show the projection; a zero axis counts as Z. | Behaviour | The renderers are 2D. | phase 3 |
| `UIElement.TransformMatrix` is applied after the other facades. | Its 2D affine part (M11, M12, M21, M22, M41, M42) is applied after the render transform and before `Scale`, the rotation and `Translation`; the 3D part is ignored. | Behaviour | The composition visual applies its `TransformMatrix` first; folding `Scale` and the rotation into it would break their animations. | phase 3 |
| A facade transition and an animation of the same property of the handoff visual cannot be combined. | While any transition is set, the implicit animations of the facades replace those the application set on the element visual. | Behaviour | A visual has one implicit animation collection. | phase 3 |
| `Vector3Transition.Components` is not documented in detail; the easing of transitions is not documented. | Excluded components jump to their final value at the start; a transition with no components does nothing. The transitions use the default easing of the compositor. | Behaviour | The port had to choose. | phase 3 |
| `SetImplicitHideAnimation` keeps an element removed from the tree in unloading storage until the animation completes. | The animation runs on an image of the element taken when it is removed. | Behaviour | A detached element loses its composition visual and its content in the same batch. | phase 1 |

## 8. Verification in the catalog

The page "Composition (WinUI)" of the ControlCatalog (`samples/ControlCatalog/Pages/winui_composition_page.rs`) shows a connected animation between a list of cards and a detail view, the implicit show and hide animations, a light that follows the pointer (the pointer position property set) and a parallax header (the manipulation property set of a scroll viewer). A screenshot run of the desktop entry point drives it without a person and saves the frames the compositor draws:

```sh
FERROUI_SMOKE_SCREENSHOTS=<directory> FERROUI_SMOKE_SCREENSHOT_PAGES="Composition (WinUI)" \
FERROUI_SMOKE_PAGES=6000 FERROUI_SMOKE_WINUI_FLIGHT=0 cargo run -p control-catalog-desktop --features vello
```

`FERROUI_SMOKE_WINUI_FLIGHT` chooses the configuration (0 gravity, 1 direct, 2 basic); the run opens the first card (`flight-NN-<ms>.png` during the flight there and back), then hides and shows the box of the implicit animations and scrolls the parallax scroller (`steps-<ms>.png`). `FERROUI_RENDERER=vello` selects the Vello backend; with a debug build `FERROUI_SMOKE_WINUI_FLIGHT_FEW=1` saves fewer frames, because the encoding of a picture blocks the thread of the compositor for long enough to shift the later frames. The flights, the implicit animations and the parallax were checked this way with Skia and Vello on macOS.
