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

| Phase | State |
|---|---|
| 1 | planned |
| 2 | planned |
| 3 | planned |
| 4 | planned |
| 5 | planned |
| 6 | planned |

## 8. Deviations from WinUI

Every difference from WinUI, in the format of `DEVIATIONS.md` (Representation, Behaviour, Missing,
Addition). `DEVIATIONS.md` points here for this area.

| WinUI | Port | Kind | Why | Since |
|---|---|---|---|---|

## 9. Windows checks

What a Windows machine (Windows 11 22H2 or later, and Windows 10) must confirm, because it was not
run in this port's development:

- to be filled in with phase 4.
