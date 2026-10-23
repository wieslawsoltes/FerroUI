# WinUI 3 materials and composition lighting

This document is the inventory and the plan of the port of the WinUI 3 materials (`AcrylicBrush`,
`MicaBackdrop`, `DesktopAcrylicBackdrop`, the system backdrops and their controllers) and of the
composition lighting that controls use (Reveal: `RevealBrush`, the Reveal lights and `XamlLight`) to
FerroUI. It records what FerroUI had, what the port adds, in which order, how each backend and
platform draws it, and every place where the result differs from WinUI. It continues
[winui-composition.md](winui-composition.md), whose visuals, brushes, effects and lights it builds on.

## 1. Sources

- **microsoft-ui-xaml**, branch `winui3/main`, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`
  (local checkout `/Users/wieslawsoltes/GitHub/microsoft-ui-xaml`, read-only, MIT):
  - `controls/dev/Materials/Acrylic/AcrylicBrush.{idl,h,cpp}`: the members, the effective tint and
    luminosity colours (`GetEffectiveTintColor`, `GetTintOpacityModifier`, `GetLuminosityColor`), the
    effect graph (the "shadow-friendly luminosity-based recipe"), the fallback rules and the
    167 ms cross-fade between the material and its fallback colour;
  - `controls/dev/Lights/MaterialHelper.{h,cpp}`: the material policy (energy saver, transparency
    effects, fast effects, simulated policy), the shared noise brush, the light attachment;
  - `controls/dev/Materials/Reveal/RevealBrush.{idl,h,cpp}`, `RevealBackgroundBrush.h`,
    `RevealBorderBrush.h`: the hover and border effect graphs, the colour matrices, the attached
    `State` and `IsContainer` properties, the attachment of the lights to the root;
  - `controls/dev/Lights/RevealHoverLight.cpp`, `RevealBorderLight.cpp`, `XamlAmbientLight.cpp`,
    `SpotLightStateHelper.{h,cpp}`, `IsTargetDPHelper.h`: the lights, their state machine, their
    animation tables, their expressions and their constants;
  - `controls/dev/Materials/MicaBackdrop`, `controls/dev/Materials/DesktopAcrylicBackdrop`:
    the XAML backdrops over the controllers;
  - `dxaml/xcp/dxaml/lib/SystemBackdrop_Partial.cpp`, `XamlLight_Partial.cpp`,
    `dxaml/xcp/components/graphics/XamlLight.cpp`, `dxaml/xcp/core/hw/hwwalk.cpp`
    (`AccumulateLightsAndCheckLightsEnteringTree`, `UpdateLightTargets`): the configuration of a
    system backdrop per target (theme, high contrast, input activation) and how XAML lights reach
    their targets;
  - `controls/dev/Materials/Acrylic/AcrylicBrush_themeresources.xaml`,
    `controls/dev/Materials/Reveal/RevealBrush_themeresources.xaml` and the control theme
    resources that refer to them.
- **`Microsoft.UI.Composition.SystemBackdrops`** (`MicaController`, `DesktopAcrylicController`,
  `SystemBackdropConfiguration`) and the Win2D effects are closed source. They are ported from
  their public API reference (learn.microsoft.com, Windows App SDK 1.6) and from how
  microsoft-ui-xaml uses them. Where the reference does not say how something behaves, section 8
  says what the port chose.

Only WinUI and FerroUI are named in the sources, the samples, the tests and this page.

## 2. How the WinUI types map onto FerroUI

| WinUI | FerroUI |
|---|---|
| `Microsoft.UI.Xaml.Media.XamlCompositionBrushBase`, `AcrylicBrush` | `ferroui_base::media::{XamlCompositionBrushBase, AcrylicBrush}`: brushes of the object model, drawn by the compositor |
| `Microsoft.UI.Xaml.Media.XamlLight`, `UIElement.Lights` | `ferroui_base::media::XamlLight`, `Visual::lights()` |
| `Microsoft.UI.Xaml.Media.RevealBrush` and its subclasses, the Reveal lights, `XamlAmbientLight` | `ferroui_controls::media` (they need the window, its pointer and the focus of controls) |
| `Microsoft.UI.Xaml.Media.SystemBackdrop`, `MicaBackdrop`, `DesktopAcrylicBackdrop` | `ferroui_controls::media` |
| `Microsoft.UI.Composition.SystemBackdrops` | `ferroui_base::rendering::composition::system_backdrops` |
| `Microsoft.UI.Composition.ICompositionSupportsSystemBackdrop` | the trait of the same name in `system_backdrops`, implemented by `TopLevel` (windows and popup windows) |
| Win2D effects (`BlendEffect`, `ColorMatrixEffect`, ...) | `ferroui_base::rendering::composition::effects` |
| `MaterialHelper` (internal) | `ferroui_base::media::MaterialPolicy` (public: the policy and the shared noise) |

## 3. What FerroUI had

- **Composition** (`winui-composition.md`): effect brushes with Gaussian blur, opacity, cross-fade,
  source-over composite and colour source; a saturation effect that does not load; a backdrop
  brush that draws nothing; ambient, distant, point and spot lights drawn as a light map over the
  content of each lit visual.
- **Drawing contexts**: layers, opacity, opacity masks, clips, a blur and a drop shadow effect;
  the blend modes of `BitmapBlendingMode` (including `Color` and `Luminosity`) for bitmaps only; no
  colour matrix; no way to draw what is under a rectangle again.
- **`ExperimentalAcrylicMaterial`**, **`ExperimentalAcrylicBorder`**,
  **`CompositionExperimentalAcrylicVisual`**: a tint over a luminosity colour (with WinUI's
  formulas) and the noise texture of the backends, without a backdrop.
- **`WindowTransparencyLevel`** (`None`, `Transparent`, `Blur`, `AcrylicBlur`, `Mica`): on Windows
  the host backdrop brush and the visuals of Windows composition (not used by the DirectComposition
  surface), on macOS a translucent window with an `NSVisualEffectView` (HUD material) for
  `AcrylicBlur`, on X11 the KWin blur.
- **The WinUI theme**: every `AcrylicBrush` of WinUI is a `SolidColorBrush` of its fallback colour,
  every Reveal brush a `SolidColorBrush`, the Reveal styles and the system backdrops of flyouts are
  waived.
- **Platform settings**: the light and dark variants and the contrast preference; no energy saver
  and no transparency effects setting.

## 4. Inventory

Status before this port: **has**, **partial** (said in the notes), **missing**. The last column is
the phase of section 5 that brings the member to WinUI parity, or the decision when it does not.

### 4.1 Rendering primitives

| Type / member | Status | Notes | Phase |
|---|---|---|---|
| `CompositionBackdropBrush` | partial | Draws nothing. The port draws what is behind the visual (section 6.1). | 1 |
| `CompositionHostBackdropBrush`, `Compositor.CreateHostBackdropBrush` | missing | What is behind the window: transparent pixels where the platform composes the window over the desktop. | 1 |
| `BlendEffect` (`Mode`, `Background`, `Foreground`), `BlendEffectMode` | missing | The 25 Win2D modes. | 1 |
| `ColorMatrixEffect` (`ColorMatrix`, `AlphaMode`, `ClampOutput`), `Matrix5x4` | missing | | 1 |
| `SaturationEffect` | partial | Described, does not load. | 1 |
| `CompositeEffect` modes other than `SourceOver` | missing | | 1 |
| `ArithmeticCompositeEffect` | missing | Reveal uses it with a colour on one side. | 1 |
| `AlphaMaskEffect` | missing | Reveal masks its light with the noise. | 1 |
| `BorderEffect` (`ExtendX`, `ExtendY`) | missing | The noise tiles through it. | 1 |
| `TintEffect`, `InvertEffect`, `GrayscaleEffect`, `LuminanceToAlphaEffect` | missing | Colour matrices. | 1 |
| `SceneLightingEffect` (`AmbientAmount`, `DiffuseAmount`, `SpecularAmount`, `SpecularShine`, `NormalMapSource`) | missing | The lighting of Reveal. | 3 |
| Composition brushes as the brush of an element (`Border.Background = compositionBrush`) | partial | A colour or gradient composition brush works; the others end the render thread ("not a brush"). | 1 |
| Blend layers and colour matrix layers of the drawing contexts | missing | Skia and Vello. | 1 |
| The noise texture | partial | In the Skia and Vello crates, for the experimental acrylic only. | 2 |

### 4.2 Acrylic

| Type / member | Status | Notes | Phase |
|---|---|---|---|
| `XamlCompositionBrushBase` (`FallbackColor`, `CompositionBrush`, `OnConnected`, `OnDisconnected`) | missing | | 2 |
| `AcrylicBrush` (constructor, `TintColor`, `TintOpacity`, `TintLuminosityOpacity`, `TintTransitionDuration`, `AlwaysUseFallback`, the dependency properties) | missing | `ExperimentalAcrylicMaterial` has the tint and luminosity formulas without a backdrop. | 2 |
| The effective tint colour (`GetTintOpacityModifier`) and luminosity colour (`GetLuminosityColor`) | partial | In `ExperimentalAcrylicMaterial`. | 2 |
| The effect graph: backdrop over the opaque fallback colour, blur 30, luminosity blend, colour blend, noise at 0.02, the opaque-tint variant | missing | | 2 |
| Animated tint (`TintColor.Color`, `LuminosityColor.Color` over `TintTransitionDuration`) | missing | | 2 |
| The cross-fade between the material and the fallback colour (167 ms, `FadeInOut.Weight`) | missing | | 2 |
| Fallback: `AlwaysUseFallback`, energy saver, transparency effects off, effects not fast, high contrast | missing | `PlatformColorValues::contrast_preference` exists and is not read. | 2 |
| `MaterialHelper.SimulateDisabledByPolicy`, `IgnoreAreEffectsFast` (test hooks) | missing | | 2 |

### 4.3 Reveal and XAML lights

| Type / member | Status | Notes | Phase |
|---|---|---|---|
| `XamlLight` (`CompositionLight`, `GetId`, `OnConnected`, `OnDisconnected`) | missing | | 3 |
| `XamlLight.AddTargetElement`, `RemoveTargetElement`, `AddTargetBrush`, `RemoveTargetBrush` | missing | | 3 |
| `UIElement.Lights` | missing | | 3 |
| Lights on the root reach popups (`hwwalk.cpp`) | missing | | 3 |
| `XamlAmbientLight` (`Color`, attached `IsTarget`) | missing | | 3 |
| `RevealHoverLight` (hover) and its press form (`SetIsPressLight`) | missing | Internal in WinUI. | 3 |
| The hover light state machine (`Off`, `AnimToHover`, `AnimToOff`, `Hover`, `Pressing`, `FastRelease`, `SlowRelease`), its animation tables, offset and outer angle expressions | missing | | 3 |
| `RevealBorderLight` (normal and wide, light and dark theme ids, window pointer events, 250 ms switch) | missing | Internal in WinUI. | 3 |
| `RevealBrush` (`Color`, `TargetTheme`, `AlwaysUseFallback`, `FallbackColor`), `RevealBrushState` | missing | The theme has solid colours. | 3 |
| `RevealBrush.State` (attached), `RevealBrush.IsContainer` (attached, internal) | missing | | 3 |
| `RevealBackgroundBrush`, `RevealBorderBrush` | missing | | 3 |
| The hover effect (noise-masked lighting, luminance to alpha, arithmetic composite with the base colour) and the border effect (luminance to alpha times two, inverted in the light theme) | missing | | 3 |
| Attaching the border and ambient lights to the root when the first Reveal brush connects, removing them after the last | missing | | 3 |

### 4.4 System backdrops

| Type / member | Status | Notes | Phase |
|---|---|---|---|
| `SystemBackdrop` (`OnTargetConnected`, `OnTargetDisconnected`, `OnDefaultSystemBackdropConfigurationChanged`, `GetDefaultSystemBackdropConfiguration`) | missing | | 4 |
| `Window.SystemBackdrop`, `Popup.SystemBackdrop`, `FlyoutBase.SystemBackdrop`, `MenuFlyoutPresenter.SystemBackdrop` | missing | | 4 |
| `MicaBackdrop` (`Kind`), `MicaKind` (`Base`, `BaseAlt`) | missing | `WindowTransparencyLevel::mica()`: Windows only, through Windows composition. | 4 |
| `DesktopAcrylicBackdrop` | missing | `WindowTransparencyLevel::acrylic_blur()`. | 4 |
| `SystemBackdropConfiguration` (`IsInputActive`, `Theme`, `IsHighContrast`, `HighContrastBackgroundColor`), `SystemBackdropTheme` | missing | | 4 |
| `MicaController` (`Kind`, `TintColor`, `TintOpacity`, `LuminosityOpacity`, `FallbackColor`, `IsSupported`, `AddSystemBackdropTarget`, `RemoveSystemBackdropTarget`, `RemoveAllSystemBackdropTargets`, `SetSystemBackdropConfiguration`, `ResetProperties`, `State`, `StateChanged`, `Close`) | missing | | 4 |
| `DesktopAcrylicController` (`Kind`, the same members), `DesktopAcrylicKind` (`Default`, `Base`, `Thin`) | missing | | 4 |
| `SystemBackdropState` (`Active`, `Fallback`, `Hidden`) | missing | | 4 |
| `ICompositionSupportsSystemBackdrop` (`SystemBackdrop`) | missing | | 4 |
| Fallback: high contrast (the high contrast background colour), inactive window, energy saver, transparency effects off, unsupported platform | missing | | 4 |
| Windows: DWM system backdrop types (`DWMWA_SYSTEMBACKDROP_TYPE`) | missing | | 4 |
| macOS: `NSVisualEffectView` materials | partial | One material (HUD) for `AcrylicBlur`. | 4 |

### 4.5 Theme and samples

| Item | Status | Notes | Phase |
|---|---|---|---|
| The `AcrylicBrush` resources of `AcrylicBrush_themeresources.xaml` (and `SystemControlTransientBackgroundBrush`) | partial | Solid fallback colours. | 5 |
| Flyout, tooltip, combo box, command bar, navigation pane, picker, number box, auto-suggest and scroll bar backgrounds through those resources | partial | Solid colours. | 5 |
| `MenuFlyoutPresenter` with `DesktopAcrylicBackdrop` (`MenuFlyoutSystemBackdrop`) and a transparent background | partial | Transparent background, no backdrop. | 5 |
| The Reveal brushes of `RevealBrush_themeresources.xaml` and the Reveal styles (`ButtonRevealStyle`, `RepeatButtonRevealStyle`, `ToggleButtonRevealStyle`, list and grid item Reveal styles, ...) | missing | | 5 |
| A materials page and Reveal sections in the ControlCatalog | missing | | 6 |

## 5. Plan

Each phase lands with unit tests next to the composition tests (`composition_winui_*_tests.rs`),
render tests (`tests/FerroUI.RenderTests`, run with Skia and with Vello) where a picture says
more than a value, and updates the inventory.

### Phase 1: rendering primitives

- `ColorMatrix` (`Matrix5x4` of Win2D: a row per input channel, a row of offsets) with the
  saturation, tint, invert, grayscale and luminance-to-alpha matrices.
- A new optional contract of the drawing contexts, `IDrawingContextImplWithLayerEffects`:
  `push_blend_layer(bounds, mode)` composes what is drawn until the pop onto what is under it with
  a blend mode; `push_color_matrix_layer(bounds, matrix)` composes it through a colour matrix. Skia:
  save layers whose paint has the blend mode or a matrix colour filter. Vello: layers with the
  `Mix` and `Compose` of the mode; the colour matrix is applied to the pixels of a scene of the CPU
  renderer (vello 0.3 has no colour matrix filter). The compositor's drawing context proxy and the
  mock forward it.
- The effects: `BlendEffect`, `ColorMatrixEffect`, `SaturationEffect` (loads now), `TintEffect`,
  `InvertEffect`, `GrayscaleEffect`, `LuminanceToAlphaEffect`, `ArithmeticCompositeEffect` (when
  one of its sources is a colour), `AlphaMaskEffect`, `BorderEffect`, every `CanvasComposite` mode.
- The backdrop: `CompositionBackdropBrush` draws what is behind the visual that paints it (section
  6.1); the dirty area of a frame grows to cover every backdrop it touches.
- `CompositionHostBackdropBrush`.
- Composition brushes (effect, backdrop, surface, nine-grid, mask) as the brushes of elements: the
  render data paints them inside the shape they fill.

### Phase 2: acrylic

`MaterialPolicy` (the policy of `MaterialHelper`: energy saver, transparency effects, effects that
are not fast, a simulated policy, `PolicyChanged`; the shared noise brush), the platform settings
it reads (`IPlatformSettings::get_material_settings` on macOS and Windows), the high contrast
preference; `XamlCompositionBrushBase`; `AcrylicBrush` with WinUI's recipe, its tint animations and
its cross-fade to and from the fallback colour.

### Phase 3: lights and Reveal

`XamlLight` and `Visual::lights()` (the targets of a light are found when it connects and when
elements enter its subtree; brushes are targeted by id and lit inside the subtree of the element
that hosts the light); `SceneLightingEffect`; `XamlAmbientLight`; `RevealHoverLight` (hover and
press) with its state machine and tables; `RevealBorderLight`; `RevealBrush`,
`RevealBackgroundBrush`, `RevealBorderBrush`, `RevealBrushState`, `RevealBrush.State`.

### Phase 4: system backdrops

`SystemBackdropConfiguration`, `MicaController`, `DesktopAcrylicController` with their defaults per
theme and kind, their states and fallback rules; `SystemBackdrop`, `MicaBackdrop`,
`DesktopAcrylicBackdrop`; `SystemBackdrop` on windows, popups, flyouts and the menu flyout
presenter; the platforms: DWM on Windows, `NSVisualEffectView` on macOS, the compositor elsewhere
(section 6.3).

### Phase 5: the WinUI theme

The converter of the theme (`scripts/winui-theme/convert.py`) keeps `AcrylicBrush` and the Reveal
brushes; the resources are generated again; the Reveal styles are ported; the menu flyout
presenter gets its system backdrop.

### Phase 6: catalog and verification

Reveal sections on the "Composition (WinUI)" page and a "Materials (WinUI)" page; screenshots of
the window with Skia and Vello on macOS; this page, `DEVIATIONS.md` and the changelog.

## 6. Design

### 6.1 The backdrop

A backdrop is what was drawn under a visual before it. A drawing context cannot read back its
target in the middle of a frame on every backend: the Vello context records a scene that is drawn
when the frame ends, and in Skia a read-back of a save layer sees that layer only, not the window.
FerroUI draws the backdrop again instead: when a backdrop brush is painted, the compositor renders
the tree of the target from its root into the brush's rectangle, with the transform of the root,
and stops at the visual that paints the brush (everything before it in paint order, nothing of it
and after it). The result is drawn where the brush is, through the effects of the brush, inside
the layers, clips and opacities of the visual's ancestors, so a backdrop under a half-transparent
ancestor composes as WinUI's does. Both backends do it the same way; no read-back is needed.

- A blur reads a margin of the backdrop around the rectangle (the reach of the blur), so its edges
  do not fade (the `Hard` border mode of the acrylic blur).
- A backdrop inside a backdrop is drawn to a depth of two; deeper backdrops are empty.
- A visual outside the tree of the render (a visual surface, a redirect) has an empty backdrop.
- The compositor keeps the device rectangles of the backdrops it drew in a frame. In the next frame
  a dirty rectangle that touches one of them makes the whole of it dirty, so a blur never mixes
  new and old pixels.

### 6.2 Effects on the drawing contexts

The effect graph is painted node by node, as before: a blend paints its background, then its
foreground in a blend layer; a colour matrix, a saturation, a tint, an invert, a grayscale and a
luminance to alpha paint their source in a colour matrix layer; an alpha mask paints its source,
then its mask in a destination-in layer; a composite paints its sources with the mode's blend; an
arithmetic composite of an image and a colour is the colour matrix `k1 c + k2` per channel of the
image (`k1 = Source1Amount + MultiplyAmount * colour`, `k2 = Source2Amount * colour + Offset`); a
border effect with wrap tiles a surface brush. Each brush paints into a layer of its own, so its
blend modes see only what the brush drew.

### 6.3 Platforms

| Platform | `MicaBackdrop` | `DesktopAcrylicBackdrop` | In-app `AcrylicBrush` |
|---|---|---|---|
| Windows 11 22H2 and later | DWM `DWMSBT_MAINWINDOW` (`Base`) or `DWMSBT_TABBEDWINDOW` (`BaseAlt`) | DWM `DWMSBT_TRANSIENTWINDOW` | compositor |
| macOS | `NSVisualEffectView` behind the window: `windowBackground` (`Base`), `underWindowBackground` (`BaseAlt`) | `NSVisualEffectView`: `menu` (`Base`, `Default`), `sidebar` (`Thin`) | compositor |
| Elsewhere (X11, Wayland, Linux framebuffer, browser, Android, iOS) | fallback colour | fallback colour | compositor |

The controller asks the target (the `TopLevel` of the window) to apply the material
(`ITopLevelImpl::set_system_backdrop`). The window becomes transparent, its background is cleared,
and the platform draws the material behind it. When the platform cannot, or the policy says
fallback, the target paints the controller's fallback colour (the high contrast background colour
in high contrast) under the content.

Windows checks are listed in section 9.

## 7. Status

All six phases are implemented. The inventory of section 4 has 52 rows (14 rendering primitives,
8 acrylic, 13 Reveal and XAML lights, 12 system backdrops, 5 theme and samples); before the port 9
were partial and 43 missing. After it:

| Section | Rows | Done | Partial (see section 8) |
|---|---|---|---|
| 4.1 Rendering primitives | 14 | 11 | 3: `ArithmeticCompositeEffect` (an image and a colour, not two images), `CompositeEffect` (every mode but `MaskInvert`), `SceneLightingEffect` (ambient and diffuse; no specular, no normal map) |
| 4.2 Acrylic | 8 | 8 | |
| 4.3 Reveal and XAML lights | 13 | 13 | |
| 4.4 System backdrops | 12 | 11 | 1: `SystemBackdrop` on windows, the menu flyout presenter and (an addition) the context menu; not on `Popup` and `FlyoutBase` |
| 4.5 Theme and samples | 5 | 4 | 1: the Reveal brushes and colours are ported, the Reveal styles stay waived (deprecated in WinUI 3) |

| Phase | State | Where |
|---|---|---|
| 1 | done | `ferroui_base::media::ColorMatrix`, `platform::IDrawingContextImplWithLayerEffects` (Skia, Vello, the compositor's proxy, the mock), `rendering::composition::effects` (the effects and the effect graph), `server::render_scope` (the backdrop), `drawing::composition_painted_brush` (composition brushes as element brushes), `CompositionHostBackdropBrush` |
| 2 | done | `media::{MaterialPolicy, XamlCompositionBrushBase, AcrylicBrush}`, `PlatformMaterialSettings` (macOS: reduce transparency and low power mode; Windows: `EnableTransparency` and the battery saver) |
| 3 | done | `media::XamlLight`, `Visual::lights()`, the scene lighting of `server_composition_light.rs`, `ferroui_controls::media::{RevealBrush, RevealBackgroundBrush, RevealBorderBrush, RevealHoverLight, RevealPressLight, RevealBorderLight, XamlAmbientLight}` |
| 4 | done | `rendering::composition::system_backdrops`, `ferroui_controls::media::{SystemBackdrop, MicaBackdrop, DesktopAcrylicBackdrop}`, `TopLevel::system_backdrop`, `MenuFlyoutPresenter::system_backdrop`, `ContextMenu::system_backdrop`, `ITopLevelImpl::set_system_backdrop` (macOS, Windows) |
| 5 | done | `scripts/winui-theme/convert.py` keeps `AcrylicBrush`, the Reveal brushes and `DesktopAcrylicBackdrop`; the menu flyout presenter and the context menu draw `MenuFlyoutSystemBackdrop`; the popups of `Menu` and `MenuItem` draw `AcrylicBackgroundFillColorDefaultBrush`; a window with a system backdrop has a transparent background |
| 6 | done | the "Materials (WinUI)" page and the "Reveal and XAML lights" section of "Composition (WinUI)" in the ControlCatalog (section 10) |

Tests: `composition_winui_material_tests.rs` (the effects, the backdrop, composition brushes as
element brushes, the acrylic recipe, its policy and cross-fade, the XAML lights, the controllers),
`color_matrix.rs`, `acrylic_brush.rs`, `effect_graph.rs` (unit tests), `ferroui_controls::media`
(`media_tests.rs`: the Reveal states and lights, the border lights of the window, the effects of
Reveal, the system backdrop of a window), the platform settings of macOS and Windows.

## 8. Deviations from WinUI

Every difference from WinUI, in the format of `DEVIATIONS.md` (Representation, Behaviour, Missing,
Addition). `DEVIATIONS.md` points here for this area.

| WinUI | Port | Kind | Why | Since |
|---|---|---|---|---|
| A backdrop brush reads the pixels composed under the visual | The compositor renders the tree again from its root up to the visual that paints the brush, inside the brush's rectangle (section 6.1); backdrops nest two deep | Representation | No backend can read its target back in the middle of a frame (the Vello scene is drawn at the end) | 0.4 |
| `CompositionHostBackdropBrush`: what the system composes behind the window, blurred by the system | Clears its rectangle to transparent when the window is transparent (the platform shows what is behind it), draws nothing when the window is opaque | Representation | The desktop behind a window is not readable by an application | 0.4 |
| The acrylic recipe asks the effect wrapper for `Color` where it means the luminosity blend and for `Luminosity` where it means the colour blend (the names are swapped between Direct2D and its wrapper) | `BlendEffectMode` has the documented meaning of each mode; the acrylic graph asks for the mode that gives WinUI's result (the luminosity of the luminosity colour, then the hue and saturation of the tint) | Representation | One meaning per name | 0.4 |
| `SceneLightingEffect`: ambient, diffuse and specular lighting of a normal map | Ambient and diffuse, a flat surface; `SpecularAmount`, `SpecularShine` and `NormalMapSource` are accepted and not drawn | Missing | Reveal uses ambient 0 and diffuse 0.2 on a flat surface | 0.4 |
| The attenuation of a light lighting a scene lighting effect, in device-independent pixels | In units of 100 DIPs (`SCENE_LIGHTING_DISTANCE_UNIT`) | Representation | The constants of the Reveal lights (linear 0.5, constant 2) are meant for the effect's units; in DIPs the light would not reach past a few pixels. The lighting of content (`winui-composition.md`) keeps DIPs | 0.4 |
| `ArithmeticCompositeEffect` of any two sources | Of an image and a colour (a colour matrix); two images fail to load (`EffectNotSupported`) | Missing | Reveal needs the colour form only | 0.4 |
| `CompositeEffect` with `MaskInvert` | Fails to load | Missing | No blend mode of the backends draws it in one layer | 0.4 |
| The noise of the materials at 1 texel per physical pixel | 1 texel per DIP | Representation | The brush has no scale of its own | 0.4 |
| WinUI 3 always draws the Reveal brushes as their fallback colour (Reveal is deprecated; its policy reports "disabled") | The Reveal brushes draw their lights unless `AlwaysUseFallback` is set or the material policy disables materials | Behaviour | The port of Reveal is asked for; an application that wants WinUI 3's look sets `AlwaysUseFallback` | 0.4 |
| The Reveal styles (`ButtonRevealStyle`, ...) | Waived, as before | Missing | Deprecated in WinUI 3; the default styles they duplicate are ported | 0.4 |
| `UIElement.Lights` in markup | `Visual::lights()` in code | Missing | The markup collection property is not part of this port | 0.4 |
| A XAML composition brush on several compositors | Bound to the compositor of the first element that draws it | Representation | One compositor per process in practice | 0.4 |
| `MicaController`, `DesktopAcrylicController` default colours (closed source) | Chosen from the documented look of each kind and theme: Mica `Base` light `#F3F3F3` tint 0.5, dark `#202020` tint 0.8; `BaseAlt` light `#DADADA` 0.5 (fallback `#E8E8E8`), dark `#0A0A0A` 0 (fallback `#202020`); desktop acrylic `Base` light `#F3F3F3` 0 / luminosity 0.9 (fallback `#F9F9F9`), dark `#202020` 0.5 / 0.96 (fallback `#2C2C2C`); `Thin` light `#D3D3D3` 0 / 0.44, dark `#545454` 0 / 0.64 | Representation | Not published; to be compared on Windows (section 9) | 0.4 |
| The system draws Mica and desktop acrylic | macOS: an `NSVisualEffectView` behind the content (`windowBackground` for Mica `Base`, `underWindowBackground` for `BaseAlt`, `menu` for desktop acrylic `Base`, `sidebar` for `Thin`) in the appearance of the configuration's theme; the controller's tint, luminosity and fallback colours do not change it; elsewhere but Windows the fallback colour | Representation | The closest material of each platform | 0.4 |
| `Popup.SystemBackdrop`, `FlyoutBase.SystemBackdrop` | Not ported | Missing | The menu flyout presenter is where the theme uses a system backdrop | 0.4 |
| The system backdrop of a menu flyout presenter in a windowed popup (the system draws desktop acrylic behind the popup's window) | The fallback colour in a popup window of its own; in-app acrylic of the window in an overlay popup | Behaviour | Only windows get a platform backdrop in the port | 0.4 |
| (no counterpart) `ContextMenu.SystemBackdrop` | The property of the menu flyout presenter, shared by the context menu; the WinUI theme sets `MenuFlyoutSystemBackdrop` | Addition | FerroUI's context menu is a presenter of its own | 0.4 |
| The popups of a menu bar item and of a sub item are menu flyout presenters with the system backdrop | FerroUI's `Menu` and `MenuItem` popups draw `AcrylicBackgroundFillColorDefaultBrush` | Representation | A border of a template has no system backdrop | 0.4 |
| (no counterpart) the window's background under a system backdrop | The WinUI theme's `Window` has a transparent background in `:system-backdrop` | Addition | FerroUI's window theme draws a background WinUI's window does not | 0.4 |
| `MicaKind`, `DesktopAcrylicKind`, `SystemBackdropTheme` in `Microsoft.UI.Composition.SystemBackdrops` | Markup enums of the namespace `FerroUI.Media` | Representation | The XAML namespace of the other materials | 0.4 |
| The material policy reads the energy saver, the transparency effects setting and the speed of effects of Windows | The same on Windows; on macOS low power mode and "reduce transparency"; elsewhere materials are always on | Representation | The settings each platform has | 0.4 |
| The colour matrix of an effect runs on the GPU | Skia: a matrix colour filter (straight alpha) or a runtime colour filter (premultiplied); Vello: applied to the pixels of a scene of the CPU renderer | Representation | vello has no colour matrix filter | 0.4 |

## 9. Windows checks

What a Windows machine (Windows 11 22H2 or later, and Windows 10) must confirm, because it was not
run in this port's development (it is compiled for `x86_64-pc-windows-msvc` only):

1. `MicaBackdrop` (`Base`, `BaseAlt`) and `DesktopAcrylicBackdrop` on a window: DWM draws
   `DWMSBT_MAINWINDOW`, `DWMSBT_TABBEDWINDOW` and `DWMSBT_TRANSIENTWINDOW` behind the client area
   (`DWMWA_SYSTEMBACKDROP_TYPE`, frame extended into the whole client area), and the render
   surface leaves it visible (the window's background is transparent, the clear colour has zero
   alpha).
2. `DWMWA_USE_IMMERSIVE_DARK_MODE` follows the theme of the configuration (dark Mica in the dark
   theme).
3. An inactive window shows the fallback colour (`IsInputActive` false), and the material comes back
   when it is activated.
4. Windows 10 and Windows 11 before 22H2 (no system backdrop types): the controllers report
   `Fallback` and the window shows the fallback colour.
5. The material policy: turning "Transparency effects" off (`EnableTransparency`, read again on
   `WM_SETTINGCHANGE`) and the battery saver (`WM_POWERBROADCAST`) make acrylic and Reveal draw their
   fallback colours and the system backdrops fall back.
6. High contrast: the controllers draw the high contrast background colour.
7. The default colours of the controllers (section 8) against WinUI's own window with the same
   backdrop.
8. Removing the system backdrop gives the window its opaque background back (`DWMSBT_AUTO`).
9. The menu flyout presenter and the context menu with `MenuFlyoutSystemBackdrop` in overlay popups
   and in popup windows.
