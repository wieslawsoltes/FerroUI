# The WinUI theme (`ferroui-themes-winui`)

A second theme of the control set, next to the Fluent theme: the control styles, templates and
theme resources of WinUI 3, ported from microsoft-ui-xaml to the controls of FerroUI. This
document is the design of the port and the conventions every agent that ports a part of it
follows. The state of the port is measured by `scripts/winui-theme/status.py` and written to
[winui-theme/PROGRESS.md](winui-theme/PROGRESS.md) (per control, with the summary) and
[winui-theme/TRACKING.md](winui-theme/TRACKING.md) (per resource key and style of the source).
Every difference from WinUI 3 is recorded in
[winui-theme/DEVIATIONS.md](winui-theme/DEVIATIONS.md); every batch adds the deviations of the
controls it ports there and regenerates the tracking in the same commit (section 7).

The Fluent theme (`src/FerroUI.Themes.Fluent`, `ferroui-themes-fluent`) is not changed or
replaced: it is the theme the upstream project ships (itself an adaptation of the WinUI 2
resources); the WinUI theme is the theme of WinUI 3 itself, ported from its own sources.

## 1. Source

- Repository: <https://github.com/microsoft/microsoft-ui-xaml>, branch `winui3/main`, commit
  `7e4b0c2096be402f51cc8f7d203ebf67cff183d0` (local checkout
  `/Users/wieslawsoltes/GitHub/microsoft-ui-xaml`; the scripts take `--reference` or
  `WINUI_XAML_ROOT` for another place). Read-only.
- License: MIT, Copyright (c) Microsoft Corporation (`LICENSE` at the root of the repository;
  `NOTICE.md` lists the third-party code of the repository, none of which the port uses). The
  notice is in `src/FerroUI.Themes.WinUI/NOTICE.md` and the `NOTICE.md` at the root of FerroUI.

### 1.1 What the theme of a WinUI 3 application is

The theme is one dictionary that the build of microsoft-ui-xaml merges from three parts, a later
definition of a key replacing an earlier one (`controls/tools/CustomTasks/MergedDictionary.cs`,
`controls/tools/GenerateMergedXaml.ps1`, `controls/dev/dll/Microsoft.UI.Xaml.Common.targets`,
target `MergeMuxMuxcGenericResourceFiles`):

1. `dxaml/xcp/dxaml/themes/generic.xaml` (23,820 lines): the theme resources of the platform
   (`SystemControl*` brushes, the `System*Color` palette of WinUI 2, typography, sizes) and the
   styles of the controls of the framework (`Button`, `CheckBox`, `ScrollViewer`, ...);
2. the pages of the controls library of type `DefaultStyle` (its `generic.xaml`: the styles of
   `NavigationView`, `Expander`, `TreeView`, ... in `controls/dev/<Control>/<Control>.xaml`);
3. the pages of type `ThemeResources` (its `themeresources.xaml`, which `XamlControlsResources`
   adds to the resources of the application): `controls/dev/CommonStyles/*_themeresources.xaml`
   (the palette of WinUI 3 in `Common_themeresources_any.xaml`, the corner radii, the type ramp,
   and the resources and default styles of the controls of the framework, which replace the ones
   of `generic.xaml`) and `controls/dev/<Control>/*_themeresources.xaml`, ordered by their
   `Priority` metadata (1 to 8, then the ones without).

Within a type the pages keep the order of the `.vcxitems` files that
`controls/ProjectImports.targets` imports. The merge has three more rules: the theme dictionary
`Dark` is merged into `Default`; a key of a theme dictionary removes the same key from the
dictionary itself (`RemoveAncestorNodesWithKey`); and at run time the entries of a dictionary are
read before its theme dictionaries, so a key outside the theme dictionaries shadows the same key
of a theme dictionary. `scripts/winui-theme/winui_sources.py` implements exactly this merge; its
result is the reference of the port (an item that a later document replaces is "superseded" and
not ported).

One more rule comes from how WinUI applies styles: an implicit style of the application
resources is applied on top of the default style of the control (`DefaultStyleKey`), so an
implicit style without setters and without a base (`<Style TargetType="ProgressBar" />` of
`Deprecated_themeresources.xaml`) changes nothing and is not an item of the theme.

### 1.2 Which variant

The default one. `XamlControlsResources.cpp` loads `themeresources_perf2026.xaml` and
`generic_perf2026.xaml` (the pages of type `ThemeResourcesPerf2026` and `StylePerf2026`, files
`*_perf2026.xaml`) only when `AreDefaultStyleOptimizationsEnabled()` is true, an opt-in of the
application; every application gets the default variant otherwise. The perf2026 documents are
not read.

The compact density (`UseCompactResources`, `compact_themeresources.xaml`) is the theme resources
with `controls/dev/dll/DensityStyles/Compact.xaml` and `CompactDatePickerTimePickerFlyout.xaml`
merged last. It is ported as `DensityStyles/Compact.xaml`, the counterpart of the Fluent theme's
`DensityStyles/Compact.xaml` (section 3.6).

## 2. Summary of the decisions

| Question | Decision |
|---|---|
| Crate, folder | `ferroui-themes-winui`, `src/FerroUI.Themes.WinUI`, published, group "controls" |
| Root class | `FerroUI.Themes.WinUI.WinUITheme` (`WinUITheme.xaml`), a `Styles` with the public surface of `FluentTheme` where it applies |
| Theme dictionaries | `Default` (the dark theme of WinUI) and `Light` keep their keys; `HighContrast` is keyed `{x:Static winui:WinUITheme.HighContrast}`, a theme variant of the crate that inherits `Dark` |
| `{ThemeResource X}` | `{DynamicResource X}` |
| `{StaticResource X}`, `<StaticResource x:Key ResourceKey>` | kept |
| `Style` + `ControlTemplate` + `VisualStateManager` | `ControlTheme`, template setter, nested styles on pseudo-classes |
| Keyed styles | keyed control themes with the same key, based on the default theme of their type; `Style="{StaticResource X}"` is `Theme="{StaticResource X}"` |
| Resources | generated by `scripts/winui-theme/convert.py` into `Resources/`, one document per source document, effective items only |
| Control templates | ported by hand into `Controls/`, one document per FerroUI control |
| Symbol font glyphs | geometries `GlyphGeometry<code point>` in `Resources/Glyphs.xaml` |
| Accent colour | `Accents/SystemAccentColors` (the Fluent theme's class, in this crate) |
| System colours | `Resources/SystemResources.xaml`: fixed standard and High Contrast Black colours |

## 3. The crate

### 3.1 Layout

The crate mirrors `src/FerroUI.Themes.Fluent`:

| File | What it is |
|---|---|
| `Cargo.toml` | `links = "ferroui_themes_winui_xaml"`, `version.workspace = true`, dependencies as the Fluent theme |
| `build.rs` | embeds every `.xaml` of the crate and compiles the group of `WinUITheme.xaml` (as the Fluent theme's build script) |
| `lib.rs`, `assets.rs`, `register_types.rs`, `rust_paths.rs` | as in the Fluent theme; `rust_paths.rs` is written by `scripts/generate_markup_types.py --target rust-paths` |
| `winui_theme.rs` | the class `WinUITheme` and the enum `DensityStyle` |
| `Accents/system_accent_colors.rs` | the accent colour provider (section 3.3) |
| `Converters/corner_radius_to_thickness_converter.rs` | `CornerRadiusToThicknessConverter` of WinUI, which the theme resources name and FerroUI's control set lacks (section 5) |
| `Primitives/info_bar_panel.rs` | `InfoBarPanel` of WinUI (`Microsoft.UI.Xaml.Controls.Primitives`), the panel of InfoBar, with which the template of WindowNotificationManager presents a notification (FerroUI's control set has no counterpart) |
| `Primitives/tab_view_equal_width_panel.rs` | `TabViewEqualWidthPanel`, the sizing of the tabs of TabView in its default `TabWidthMode="Equal"` (`TabView::UpdateTabWidths`) as the items panel of the strip of TabControl: every tab as wide as the others, between `TabViewItemMinWidth` and `TabViewItemMaxWidth`, scrolled when they do not fit (FerroUI's TabControl does not size its tabs; DEVIATIONS.md TBC-9) |
| `WinUITheme.xaml` | the document of the class: the resources and the include of the control themes |
| `Resources/*.xaml` | the resources: one generated document per source document, `SystemResources.xaml`, `ThemeShadows.xaml`, `Glyphs.xaml` |
| `DensityStyles/Compact.xaml` | the compact resources (generated) |
| `Controls/WinUIControls.xaml` | merges the control documents |
| `Controls/<Control>.xaml` | the control themes of one FerroUI control |
| `Controls/excluded.txt` | documents that are embedded but not merged yet (empty) |
| `compiled_xaml.rs`, `compiled_xaml.xamlmeta` | the reference output of the compiler, written by a test (section 8.2) |
| `LICENSE`, `NOTICE-PROJECT.md` | links to the files at the root of the repository |
| `NOTICE.md` | the notice of microsoft-ui-xaml and of the parts patterned on the Fluent theme |
| `tests/` | the tests (section 8.1) |

The crate is a member of the workspace and of `[workspace.dependencies]`, an optional dependency
of the facade crate `ferroui` (feature `winui-theme`, module `themes_winui`, `WinUITheme` in the
prelude), in the crate table of the README and in `docs/release.md`.

The ControlCatalog sample offers it as a third theme (`CatalogTheme::WinUI` on its settings page,
`FERROUI_CATALOG_THEME=winui` for the desktop catalog). Until every batch is done the catalog puts
it over the Fluent theme, so the controls it does not style yet are drawn by the Fluent templates
(`samples/ControlCatalog/GAPS.md`, "The WinUI theme of the port").

### 3.2 The class and the theme variants

```rust
application.styles().add(WinUITheme::new().as_style());
```

`WinUITheme` has the members of `FluentTheme` that apply: `new()`, `with_service_provider(sp)`,
`as_style()`, `as_resource_provider()`, `try_get_resource()`, the property `DensityStyle`
(`density_style()`, `set_density_style()`, `density_style_property()`), `DOCUMENT_URI`. It has
no `Palettes` (WinUI has no palette resources). It adds `WinUITheme::high_contrast()`, also the
markup field `WinUITheme.HighContrast`.

The theme dictionaries of WinUI are `Default`, `Light` and `HighContrast` (`Dark` is merged into
`Default` by the build). They map to FerroUI as follows:

- `Default` keeps its key. In WinUI, `Default` is the dark theme: a lookup for `Dark` finds no
  `Dark` dictionary and falls back to `Default`. FerroUI's resource lookup does the same (a
  variant without its own dictionary falls back to `Default`), so `ThemeVariant.Dark` resolves
  the dark values, and a lookup without a variant (`ThemeVariant.Default`) resolves them as WinUI
  does. Renaming `Default` to `Dark` would leave lookups without a variant without a value.
- `Light` keeps its key.
- `HighContrast` is keyed `{x:Static winui:WinUITheme.HighContrast}`: FerroUI has no high
  contrast variant and parses only `Default`, `Light` and `Dark` from text, so the crate defines
  the variant `HighContrast`, which inherits `Dark` (its platform variant, for the window frame,
  is dark, and its colours are the ones of the "High Contrast Black" theme of Windows, section
  3.4). An application requests it like the others:
  `RequestedThemeVariant="{x:Static winui:WinUITheme.HighContrast}"` or
  `set_requested_theme_variant(Some(WinUITheme::high_contrast()))`.
  Pending (framework): WinUI selects the high contrast theme from the contrast setting of the
  system; FerroUI's platform settings report the contrast preference
  (`PlatformColorValues::contrast_preference`) but the actual theme variant of a top level is
  only light or dark. Following the preference needs a change of `ThemeVariant` in the base
  library, not of the theme.

Every theme dictionary of the port has the same keys in the three variants (a test checks it;
the merged theme of WinUI has that property).

### 3.3 The accent colour

WinUI reads `SystemAccentColor` and its shades `SystemAccentColorDark1..3` and
`SystemAccentColorLight1..3` from the system (`UISettings.GetColorValue`,
`SystemThemingInterop::GetSystemVariantAccentColor`); no document defines them. The crate carries
the class `SystemAccentColors` of the Fluent theme (merged into the dictionary of the theme in
`WinUITheme.xaml`): the accent colour of the platform settings, the default `#0078D7` without
platform settings, and the shades the platform reports (`PlatformColorValues::accent_shades`, a
FerroUI addition the Windows backend fills from `UISettings`). Where the platform reports no
shades (every backend but Windows), they are computed with the steps of lightness of the Fluent
theme; the comparison with native WinUI (section 8.3) showed the computed shades of the default
blue darker than the system's (Dark1 `#00589B` against `#0067C0`, Light2 `#61BAFF` against
`#4CC2FF`), which is why Windows reports its own.

### 3.4 The system colours

The `SystemColor*Color` resources (`SystemColorWindowColor`, `SystemColorWindowTextColor`,
`SystemColorButtonFaceColor`, `SystemColorButtonTextColor`, `SystemColorHighlightColor`,
`SystemColorHighlightTextColor`, `SystemColorHotlightColor`, `SystemColorGrayTextColor`) are
the system colours of Windows, which the `HighContrast` dictionaries name. FerroUI has no system
colours, so `Resources/SystemResources.xaml` carries fixed values: the standard colours of
Windows in `Default` and `Light`, the colours of "High Contrast Black" in `HighContrast`. A
later change can feed them from the platform settings.

### 3.5 Fonts

`XamlAutoFontFamily` (the font family of `ContentControlThemeFontFamily` and of the text styles)
is a name WinUI resolves to Segoe UI Variable. The port writes
`Segoe UI Variable Text, Segoe UI, $Default`: the same font on Windows 11, Segoe UI on Windows 10,
the default family of FerroUI elsewhere. `SymbolThemeFontFamily` (`Segoe Fluent Icons,Segoe MDL2
Assets`) is kept as a resource, but the templates do not use it (section 4.7).

### 3.6 Density

`DensityStyle` is `Normal` or `Compact`. As in the Fluent theme, the compact dictionary
(`DensityStyles/Compact.xaml`, included under the key `CompactStyles` and taken out of the
resources by the constructor) is looked up first when the density is compact. Its only style,
the implicit `TextBlock` style of `Compact.xaml` (font size 14 for every text block), is the
default control theme of `TextBlock` there: as in WinUI it applies to the text blocks of
templates too and replaces their inherited font size; the text of access keys (`AccessText`) is
not a `TextBlock` by its style key (DEVIATIONS.md, RES-9).

## 4. Translating WinUI markup

### 4.1 Documents

- Every document starts with a comment that names its source documents in microsoft-ui-xaml and
  the commit (a test checks it), or says that it is not ported and what it is built from.
- The root is `<ResourceDictionary xmlns="https://github.com/ferroui"
  xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" x:ClassModifier="internal">`, with
  `xmlns:winui="using:FerroUI.Themes.WinUI"` where the high contrast key is used and
  `xmlns:converters="using:FerroUI.Controls.Converters"` for the converters of the controls, and
  `xmlns:winuiconverters="using:FerroUI.Themes.WinUI.Converters"` for the converters of the crate,
  and `xmlns:primitives="using:FerroUI.Themes.WinUI.Primitives"` for its primitives (`InfoBarPanel`, `TabViewEqualWidthPanel`).
- A `*_themeresources.xaml` document of WinUI holds resources and styles. The port splits it:
  the resources go to `Resources/<same name>.xaml` (generated, section 5), the styles to the
  document of the FerroUI control, `Controls/<FerroUI control>.xaml`. `Controls/` documents
  contain control themes and the objects that belong to them (template-local themes, converters,
  geometries of a template), never resources of the theme.
- `Design.PreviewWith` is welcome in control documents (the compiler drops it).

### 4.2 Styles and control themes

| WinUI | FerroUI |
|---|---|
| `<Style x:Key="DefaultButtonStyle" TargetType="Button">` | `<ControlTheme x:Key="DefaultButtonStyle" TargetType="Button">` |
| `<Style TargetType="Button" BasedOn="{StaticResource DefaultButtonStyle}" />` | `<ControlTheme x:Key="{x:Type Button}" TargetType="Button" BasedOn="{StaticResource DefaultButtonStyle}" />` |
| `<Style x:Key="AccentButtonStyle" TargetType="Button">` (no `BasedOn`) | `<ControlTheme x:Key="AccentButtonStyle" TargetType="Button" BasedOn="{StaticResource DefaultButtonStyle}">` |
| `<Setter Property="Template"><Setter.Value><ControlTemplate TargetType="Button">` | `<Setter Property="Template"><ControlTemplate>` |
| `Style="{StaticResource AccentButtonStyle}"` (application) | `Theme="{StaticResource AccentButtonStyle}"` |
| `<Style x:Key="BodyTextBlockStyle" TargetType="TextBlock">` | a keyed control theme of `TextBlock` (`Theme="{StaticResource BodyTextBlockStyle}"`) |

WinUI applies the default style of a control beneath an explicit style, so a keyed style without
`BasedOn` inherits the setters of the default style it does not set. A FerroUI control theme
replaces the default one, so the port bases a keyed theme on the default theme of its type
(`Default<Type>Style`) when the WinUI style has no `BasedOn`.

The keys of the port are the keys of WinUI: the tracking finds a ported style by its key, and an
application ported from WinUI names the same keys. Template-local styles and templates (a
`ControlTemplate x:Key` in `Grid.Resources` of a template, a `Style x:Name`) become keyed control
themes of the control document named `<Control><Name>` (`ScrollBarVerticalThumbTemplate`,
`TextBoxDeleteButtonStyle`); they are not keys of the WinUI theme and the tracking does not list
them.

Setters of properties FerroUI does not have are left out and listed in the header comment of the
document (`UseSystemFocusVisuals` and `FocusVisualMargin` are section 4.6;
`HorizontalScrollMode`, `ZoomMode`, `IsTextScaleFactorEnabled`, `LineStackingStrategy`,
`OpticalMarginAlignment`, `AutomationProperties.AccessibilityView`, `ContentTransitions` have no
counterpart). `TextLineBounds` is kept: it is FerroUI's `TextBlock.TextLineBounds`, an addition
after WinUI's (docs/porting/DEVIATIONS.md, Text block).

### 4.3 Resource references

- `{ThemeResource X}` is `{DynamicResource X}`, in templates, setters and resources. A dynamic
  resource in a theme dictionary resolves in the variant of that dictionary, as a theme resource
  does.
- `{StaticResource X}` stays a static resource, also for theme resources (WinUI freezes the
  value of the theme at load as FerroUI does). A static resource inside a theme dictionary
  resolves in the variant of the dictionary.
- `<StaticResource x:Key="A" ResourceKey="B" />` (an alias) is kept: the compiler defers it like
  any resource object, so it is resolved when it is first used, in the whole theme.
- `x:Double`, `x:String`, `x:Boolean`, `x:Int32` are kept. A string that is a duration
  (`ControlFastAnimationDuration`) is kept as a resource but cannot feed a `Duration` of a
  transition: the template writes the value and names the resource in a comment.
- `{x:Bind}` does not occur in the theme. `{Binding X, RelativeSource={RelativeSource
  TemplatedParent}}` is `{TemplateBinding X}` (with a `Converter` where the source has one) or
  `{Binding $parent[Type].X}` where a two-way or nested binding is needed.
- `TemplateSettings` of WinUI controls (`ProgressBar.TemplateSettings`, ...) map to the
  `TemplateSettings` of the FerroUI control where it has one; otherwise the batch documents the
  replacement.

### 4.4 Visual states

A visual state of WinUI sets values while it is the state of its group. In FerroUI the state is a
pseudo-class of the control and the values are the setters of a nested style:

```xml
<!-- WinUI -->
<VisualState x:Name="PointerOver">
  <Storyboard>
    <ObjectAnimationUsingKeyFrames Storyboard.TargetName="ContentPresenter" Storyboard.TargetProperty="Background">
      <DiscreteObjectKeyFrame KeyTime="0" Value="{ThemeResource ButtonBackgroundPointerOver}" />
    </ObjectAnimationUsingKeyFrames>
  </Storyboard>
</VisualState>
<!-- FerroUI -->
<Style Selector="^:pointerover /template/ ContentPresenter#PART_ContentPresenter">
  <Setter Property="Background" Value="{DynamicResource ButtonBackgroundPointerOver}" />
</Style>
```

What is a value:

- `ObjectAnimationUsingKeyFrames` with one `DiscreteObjectKeyFrame` at `KeyTime="0"`;
- `DoubleAnimation`/`ColorAnimation` with `Duration="0"` (and no `BeginTime`), its `To`;
- a key frame animation with one frame at time 0;
- `<Setter Target="Element.Property" Value="..." />` of `VisualState.Setters`.

The state names map to pseudo-classes:

| WinUI state | FerroUI selector |
|---|---|
| `Normal` | the base values (the template) |
| `PointerOver` | `^:pointerover` |
| `Pressed` | `^:pressed` |
| `Disabled` | `^:disabled` |
| `Focused` (text controls) | `^:focus` |
| `Checked`, `Unchecked`, `Indeterminate` | `^:checked`, base, `^:indeterminate` |
| `CheckedPointerOver`, `IndeterminatePressed`, ... (one group combining two) | `^:checked:pointerover`, `^:indeterminate:pressed`, ... |
| `UncheckedNormal`, `UncheckedPointerOver`, ... (CheckBox) | base, `^:pointerover`, ... |
| `Selected`, `SelectedPointerOver`, ... | `^:selected`, `^:selected:pointerover`, ... |
| `Expanded`/`Collapsed` | `^:expanded` or the property of the control (`^[IsExpanded=true]`) |
| `ButtonVisible`/`ButtonCollapsed` (text box) | a selector of the condition the control computes (section 4.7) |

Several pseudo-classes match at once where WinUI has one state (a pressed button is also under
the pointer). The nested styles are written in the order of the states in the source, the
combined ones after their parts (`^:pointerover`, `^:pressed`, `^:disabled`, then `^:checked`,
`^:checked:pointerover`, `^:checked:pressed`, `^:checked:disabled`): the later style wins, so
the values in effect are those of the WinUI state. Where a combined state does not set a
property its parts set (ToggleButton's `CheckedDisabled` does not set `BackgroundSizing`, which
`Checked` sets), the combined style sets the base value back (`{TemplateBinding
BackgroundSizing}`). Where two groups set the same property, the later group in the source is the
later style.

`scripts/winui-theme/vsm.py` drafts these nested styles from a style of the source (the states,
their selectors and the selectors of the named elements are its arguments) and marks what is not
a value with `TODO(port)`. Its output is reviewed and completed by hand; a committed document has
no `TODO`.

### 4.5 Animations

- An animation of a state to a value (`SplineDoubleKeyFrame Value="14"
  KeyTime="{StaticResource ControlNormalAnimationDuration}"`, `DoubleAnimation To="1"
  Duration="..."`) is a setter of the value in the state's nested style and a transition of the
  property on the element (`DoubleTransition`, `BrushTransition`, `TransformOperationsTransition`,
  `ThicknessTransition`, `CornerRadiusTransition`) with the duration and the easing of the source
  (`KeySpline="0,0,0,1"` is `Easing="0,0,0,1"`, `ControlFastOutSlowInKeySpline`). A transition
  also animates the return to the base value, which WinUI does at once; `BeginTime` has no
  counterpart.
- `ContentPresenter.BackgroundTransition` (`BrushTransition`) is a `BrushTransition` of
  `Background` in `ContentPresenter.Transitions`.
- `RenderTransform` animations of a `ScaleTransform`/`TranslateTransform` are a `RenderTransform`
  setter with transform operations (`scale(0.875)`, `translateX(2px)`); the value of a resource
  (`ScrollBarButtonArrowScalePressed`, `ScrollBarThumbOffset`) is written and named in a comment.
- Animations that run on their own (`RepeatBehavior="Forever"`, the indeterminate progress bar)
  are keyframe `Animation`s in the nested style of the state, as the Fluent theme writes them.
- Theme animations (`FadeInThemeAnimation`, `PopInThemeAnimation`, the `*ThemeTransition`s of
  `ContentTransitions`) are opacity and transform transitions or animations with the timing of the
  WinUI animation; a batch that meets one documents its choice in the header of the document.

### 4.6 Focus visuals

WinUI draws the focus visual of an element with `UseSystemFocusVisuals="True"`: an outer
rectangle of 2 px in `SystemControlFocusVisualPrimaryBrush` and an inner one of 1 px in
`SystemControlFocusVisualSecondaryBrush`, at `FocusVisualMargin` around the element. FerroUI
draws focus visuals in the adorner layer: `Controls/AdornerLayer.xaml` defines the default focus
adorner with these brushes, and a control theme whose style sets a `FocusVisualMargin` sets its
own `FocusAdorner` with that margin (`Margin="-3"` for the buttons, `-7,-3,-7,-3` for the check
box and the radio button). `UseSystemFocusVisuals="False"` (or the always false
`IsApplicationFocusVisualKindReveal`) is `FocusAdorner="{x:Null}"`. The corner radius of the
focus rectangles is `ControlCornerRadius`; WinUI widens it by the margin, which the gallery
comparison (section 8.3) checks.

### 4.7 Elements and properties

| WinUI | FerroUI |
|---|---|
| `x:Name` of a template part the FerroUI control requires | the `PART_` name of the control (`ContentPresenter` of a button is `PART_ContentPresenter`, `ContentElement` of a text box is `PART_ScrollViewer`); the header comment lists the renames |
| other `x:Name`s | kept (styles select them: `/template/ Border#RootGrid`) |
| `Grid`/`StackPanel` with `BorderBrush`, `BorderThickness`, `CornerRadius` or `Padding` | a `Border` of the same name around the panel (FerroUI panels have only `Background`) |
| `Visibility="Collapsed"`, a `Visibility` resource | `IsVisible="False"`, `x:Boolean` (`Visible` is `True`) |
| `FontIcon`/`SymbolIcon`/`FontIconSource` or a `TextBlock` in `SymbolThemeFontFamily` | a `Path` (or `PathIcon`) with `Data="{StaticResource GlyphGeometry<code>}"`, `Stretch="Uniform"`, sized to the ink of the glyph at the font size of the source; the geometry goes to `Resources/Glyphs.xaml` |
| `AnimatedIcon` (Lottie animated visuals) | the fallback glyph as a `Path`; the `AnimatedIcon.State` setters become the visibility (opacity) of the glyph; the animation itself is not ported |
| `Rectangle RadiusX="{Binding CornerRadius, ..., Converter={StaticResource TopLeftCornerRadiusDoubleValueConverter}}"` | `RadiusX="{TemplateBinding CornerRadius, Converter={StaticResource TopLeftCornerRadiusDoubleValueConverter}}"` (the converter is ported, section 5) |
| `IsTabStop` | `IsTabStop` |
| `AllowFocusOnInteraction="False"` | `Focusable="False"` |
| `Control.IsTemplateFocusTarget`, `x:DeferLoadStrategy`, `x:Load`, `AutomationProperties.AccessibilityView` | left out |
| `Header`/`HeaderTemplate`/`Description` presenters of controls whose FerroUI class has none (TextBox) | left out with their rows |
| `ScrollViewer.*` attached properties of a control | `{TemplateBinding (ScrollViewer.HorizontalScrollBarVisibility)}` as in the Fluent theme; the modes FerroUI lacks are left out |
| `ThemeShadow`, `Translation` (elevation) of popups (`ApplyElevationEffect`) | the `BoxShadow` of the outer border of the popup content, from `Resources/ThemeShadows.xaml` (the drop shadow recipe of WinUI per elevation), and the margin of its insets on the child of the popup, which the popup positioner leaves out of the placement (DEVIATIONS.md, G-24) |
| `AcrylicBrush` (resource) | the `FallbackColor` as a `SolidColorBrush` (section 5) |
| `TextCommandBarFlyout` (context menu of the text controls) | a `MenuFlyout` of the same commands (batch B3) |
| `RecognizesAccessKey` | `True` on the content presenters of buttons and toggles, FerroUI's access keys (WinUI's `AccessKey` needs nothing in the template) |
| a property the control computes in code (WinUI's `CTextBox::CanInvokeDeleteButton`) | a selector of the same condition where FerroUI exposes it (`^:focus:not(:empty)[AcceptsReturn=False][IsReadOnly=False][TextWrapping=NoWrap]`), the rest documented |

### 4.8 What FerroUI lacks, and what is done instead

| Missing in FerroUI | Instead |
|---|---|
| a high contrast theme variant | `WinUITheme.HighContrast` (section 3.2) |
| system colours; system accent shades outside Windows | fixed colours; computed shades (sections 3.3, 3.4) |
| the symbol font | glyph geometries |
| `AnimatedIcon`, Lottie | the static fallback glyph |
| visual state transitions with begin times | transitions without delays |
| scroll indicator modes (`IndicatorMode`; NoIndicator, TouchIndicator) | the bar is in the mouse indicator mode, collapsed (the thin thumb) or expanded (`IsExpanded`, with the begin times of WinUI as `ShowDelay` and `HideDelay`); the scroll viewer hides its bars (NoIndicator) while the pointer is not over it; no touch indicator |
| `TextCommandBarFlyout`, `CommandBarFlyout` | menu flyouts |
| `DesktopAcrylicBackdrop`, Mica | the fallback colours; the window background is `SolidBackgroundFillColorBaseBrush` |
| `ListViewItemPresenter` (native chrome of list items) | templates with elements (batch B3) |
| `CornerRadiusToThicknessConverter` | the converter of the crate (`FerroUI.Themes.WinUI.Converters`, section 5) |
| `TextTrimming` resources, scaled double corner converters | the template writes the value or is adapted (MNI-5, SB-8) |

## 5. Resources

`scripts/winui-theme/convert.py` writes every resource document of the port from the merged
theme: one document per source document with resources, named after it
(`Resources/Button_themeresources.xaml`, `Resources/Generic.xaml` for `generic.xaml`), the
compact density `DensityStyles/Compact.xaml`, and the list of includes in `WinUITheme.xaml`
(between the `resources:begin`/`resources:end` markers, in the order of the merge). It writes only
the effective items, so every key is defined once in the port (the tracking checks it) and its
value is the one a WinUI 3 application gets. The documents are not edited by hand: a change is a
change of the converter or of `scripts/winui-theme/waivers.json`, then

```sh
/opt/homebrew/bin/python3 scripts/winui-theme/convert.py           # write
/opt/homebrew/bin/python3 scripts/winui-theme/convert.py --check   # verify
/opt/homebrew/bin/python3 scripts/winui-theme/convert.py --report  # list what no rule converts
```

All resource documents are merged into the dictionary of the theme (`MergeResourceInclude`), so a
static resource of one resolves keys of every other when it is used, and a control theme finds
every key of the theme.

The conversion rules (the docstring of the script has them all): the theme dictionary keys of
section 3.2; `{ThemeResource}` to `{DynamicResource}`; `AcrylicBrush` to a `SolidColorBrush` of
its `FallbackColor` (what WinUI draws when acrylic is not available); `LinearGradientBrush` with
`MappingMode="Absolute"` keeps its points (FerroUI's default unit) and relative points become
percentages, `RelativeTransform` becomes `Transform` with the centre as `TransformOrigin`;
`Visibility` to `x:Boolean`; `CornerRadiusFilterConverter` to FerroUI's
`CornerRadiusFilterConverter` (`Top`, `Bottom`, `Left`, `Right`) or
`CornerRadiusToDoubleConverter` (`TopLeftValue`, `BottomRightValue` without scale);
`CornerRadiusToThicknessConverter` to the converter of the same name of the crate
(`winuiconverters:`, `Converters/corner_radius_to_thickness_converter.rs`, a port of
`controls/dev/Common/CornerRadiusToThicknessConverter.cpp`); `XamlAutoFontFamily` to
`Segoe UI Variable Text, Segoe UI, $Default`; and in the density layer, an implicit style of plain
setters to the default control theme of its type.

Three resource documents are written by hand: `Resources/SystemResources.xaml` (section 3.4),
`Resources/ThemeShadows.xaml` (the shadows WinUI draws in code under elevated popups, section 4.7)
and `Resources/Glyphs.xaml` (the glyph geometries; a batch that needs a glyph adds its geometry
there, with the code point and the name of the glyph in a comment).

### 5.1 Waivers

`scripts/winui-theme/waivers.json` lists what the port does not carry, each entry with a
reason: the Reveal brushes and styles (deprecated; the seven Reveal brushes that other documents
still name are ported as the plain brushes they are), the controls FerroUI does not have
(InkToolbar, MediaTransportControls, Pivot, RatingControl, PersonPicture, MapControl,
TeachingTip, TwoPaneView, SwipeControl, AnnotatedScrollBar, ItemsView/ItemContainer,
BreadcrumbBar, PagerControl, RadioButtons, ProgressRing, ImageIcon, ScrollView, CommandBarFlyout,
RichEditBox, GridView, Hub, SemanticZoom, ListView, ...), the colour picker (styled by
`ferroui-controls-color-picker`), system backdrops, switches of native chrome
(`ListViewItemPresenter`, `ThemeShadow`), and the input validation templates that WinUI 3 removed.
A waiver is the answer "this has no meaning in FerroUI", never "not done yet": what is not done
stays pending.

## 6. Control mapping

Every WinUI style of the merged theme maps to the FerroUI control it styles, and every control of
the Fluent theme (the 81 documents of `src/FerroUI.Themes.Fluent/Controls`, which are the
templated controls of the control set and the dialogs) maps to the WinUI sources its WinUI theme
is built from. Every FerroUI control gets a template in this theme: a theme that leaves a
templated control without a template leaves it invisible. For the controls WinUI has no style
for (window, popup and adorner hosts, ButtonSpinner, NotificationCard, ManagedFileChooser, the
pages, the table view, the caption buttons), the template is the one the FerroUI control
requires, built from the WinUI resources named in the table.

The tables are generated from `scripts/winui-theme/controls.json` by `status.py`; a batch that
ports a control updates its state by porting it, not by editing the table.

<!-- mapping:begin (generated by scripts/winui-theme/status.py from scripts/winui-theme/controls.json; do not edit) -->

#### FerroUI controls

| FerroUI control document | Default control themes | WinUI source | Batch | State |
|---|---|---|---|---|
| `AdornerLayer.xaml` | `{AdornerLayer}` | No WinUI style: the system focus visual of WinUI (FocusVisualPrimaryBrush/Thickness, FocusVisualSecondaryBrush/Thickness; the SystemControlFocusVisual* resources of Common_themeresources.xaml). | F | ported |
| `AutoCompleteBox.xaml` | `{AutoCompleteBox}` | AutoSuggestBox: DefaultAutoSuggestBoxStyle, AutoSuggestBoxTextBoxStyle (AutoSuggestBox_themeresources.xaml). | B3 | ported |
| `Button.xaml` | `{Button}` | Button: DefaultButtonStyle, AccentButtonStyle, SubtleButtonStyle (Button_themeresources.xaml). | F | ported |
| `ButtonSpinner.xaml` | `{ButtonSpinner}` | No WinUI control: the spin buttons of NumberBox (NumberBoxSpinButtonStyle, NumberBox.xaml) and the TextBox resources. | B2 | ported |
| `Calendar.xaml` | `{Calendar}` | CalendarView: DefaultCalendarViewStyle (CalendarView_themeresources.xaml). | B4 | ported |
| `CalendarButton.xaml` | `{CalendarButton}` | The month and year items of CalendarView (CalendarViewBaseItem resources of CalendarView_themeresources.xaml). | B4 | ported |
| `CalendarDatePicker.xaml` | `{CalendarDatePicker}` | CalendarDatePicker: DefaultCalendarDatePicker (CalendarDatePicker_themeresources.xaml). | B4 | ported |
| `CalendarDayButton.xaml` | `{CalendarDayButton}` | CalendarViewDayItem: {CalendarViewDayItem} (CalendarView_themeresources.xaml). | B4 | ported |
| `CalendarItem.xaml` | `{CalendarItem}` | The header, navigation buttons and views of the CalendarView template (DefaultCalendarViewStyle). | B4 | ported |
| `Carousel.xaml` | `{Carousel}` | FlipView: DefaultFlipViewStyle (FlipView_themeresources.xaml) and FlipViewItem. | B5 | ported |
| `CarouselPage.xaml` | `{CarouselPage}` | No WinUI control: built from FlipView (DefaultFlipViewStyle) and the page background (SolidBackgroundFillColorBaseBrush). | B5 | ported |
| `CheckBox.xaml` | `{CheckBox}` | CheckBox: DefaultCheckBoxStyle (CheckBox_themeresources.xaml). | F | ported |
| `ComboBox.xaml` | `{ComboBox}` | ComboBox: DefaultComboBoxStyle (ComboBox_themeresources.xaml). | B3 | ported |
| `ComboBoxItem.xaml` | `{ComboBoxItem}` | ComboBoxItem: DefaultComboBoxItemStyle, ComboBoxItemPill (ComboBox_themeresources.xaml). | B3 | ported |
| `CommandBar.xaml` | `{CommandBarButton}`, `{CommandBarToggleButton}`, `{CommandBarSeparator}`, `{CommandBar}` | CommandBar: DefaultCommandBarStyle, DefaultCommandBarOverflowPresenterStyle (CommandBar_themeresources.xaml); AppBarButton: DefaultAppBarButtonStyle, AppBarButtonOverflowStyle; AppBarToggleButton: DefaultAppBarToggleButtonStyle, AppBarToggleButtonOverflowStyle; AppBarSeparator: DefaultAppBarSeparatorStyle. | B5 | ported |
| `ContentPage.xaml` | `{ContentPage}` | No WinUI style (Page has none): the page background and text of the window (SolidBackgroundFillColorBaseBrush, TextFillColorPrimaryBrush). | B5 | ported |
| `ContextMenu.xaml` | `{ContextMenu}` | MenuFlyoutPresenter: DefaultMenuFlyoutPresenterStyle (MenuFlyout_themeresources.xaml). | B3 | ported |
| `DataValidationErrors.xaml` | `{DataValidationErrors}` | No WinUI style (input validation was removed from WinUI 3): the critical fill of the palette (SystemFillColorCriticalBrush) for the message and the border of the field, and the error icon of InfoBar (InfoBar.xaml, InfoBar_themeresources.xaml) for the tool tip variant. | B1 | ported |
| `DatePicker.xaml` | `{DatePicker}`, `{DatePickerPresenter}` | DatePicker: DefaultDatePickerStyle; DatePickerFlyoutPresenter: DefaultDatePickerFlyoutPresenterStyle (DatePicker_themeresources.xaml). | B4 | ported |
| `DateTimePickerShared.xaml` | (keyed themes only) | LoopingSelector: DefaultLoopingSelectorStyle, LoopingSelectorItem; DateTimePickerFlyoutButtonStyle, DateTimePickerFlyoutLoopingSelectorNavigationButtonStyle (DateTimePickerFlyout_themeresources.xaml). | B4 | ported |
| `DrawerPage.xaml` | `{DrawerPage}` | No WinUI control: the pane of NavigationView (NavigationView_themeresources.xaml) and SplitView (DefaultSplitViewStyle). | B5 | ported |
| `DropDownButton.xaml` | `{DropDownButton}` | DropDownButton: DefaultDropDownButtonStyle (DropDownButton.xaml, DropDownButton_themeresources.xaml). | B2 | ported |
| `EmbeddableControlRoot.xaml` | `{EmbeddableControlRoot}` | No WinUI style: the background and text of the window (SolidBackgroundFillColorBaseBrush, TextFillColorPrimaryBrush, ContentControlThemeFontFamily, ControlContentThemeFontSize). | F | ported |
| `Expander.xaml` | `{Expander}` | Expander: DefaultExpanderStyle (Expander.xaml), ExpanderHeaderDownStyle, ExpanderHeaderUpStyle (Expander_themeresources.xaml). | B2 | ported |
| `FlyoutPresenter.xaml` | `{FlyoutPresenter}` | FlyoutPresenter: DefaultFlyoutPresenterStyle (FlyoutPresenter_themeresources.xaml). | B3 | ported |
| `GridSplitter.xaml` | `{GridSplitter}` | No WinUI control in the merged theme: built from the control fills of the palette (ControlStrongFillColorDefaultBrush, ControlStrongFillColorDisabledBrush) and the accent fill for the preview (AccentFillColorDefaultBrush). | B1 | ported |
| `GroupBox.xaml` | `{GroupBox}` | No WinUI control: an expanded Expander without its chevron (DefaultExpanderStyle, Expander.xaml; the header of ExpanderHeaderDownStyle, Expander_themeresources.xaml). | B1 | ported |
| `HeaderedContentControl.xaml` | `{HeaderedContentControl}` | No WinUI style: the header presenter of the WinUI controls with a Header (TextBox: HeaderContentPresenter, TextBoxTopHeaderMargin, TextControlHeaderForeground). | B1 | ported |
| `HyperlinkButton.xaml` | `{HyperlinkButton}` | HyperlinkButton: DefaultHyperlinkButtonStyle (HyperlinkButton_themeresources.xaml). | F | ported |
| `ItemsControl.xaml` | `{ItemsControl}` | No WinUI style in the merged theme (ItemsControl has a built-in template): an ItemsPresenter of the ItemsPanel. | B1 | ported |
| `Label.xaml` | `{Label}` | No WinUI control: the implicit style of ContentControl ({ContentControl}, generic.xaml), which Controls/Label.xaml ports. | B1 | ported |
| `ListBox.xaml` | `{ListBox}` | ListBox: DefaultListBoxStyle (ListBox_themeresources.xaml). | B3 | ported |
| `ListBoxItem.xaml` | `{ListBoxItem}` | ListBoxItem: DefaultListBoxItemStyle (ListBox_themeresources.xaml). | B3 | ported |
| `ManagedFileChooser.xaml` | `{ManagedFileChooser}`, `{ManagedFileChooserOverwritePrompt}` | No WinUI control: built from the ListViewItem (ListViewItem_themeresources.xaml), TextBox, Button and ContentDialog (DefaultContentDialogStyle) styles. | B6 | ported |
| `Menu.xaml` | `{Menu}` | MenuBar: DefaultMenuBarStyle (MenuBar.xaml); MenuBarItem (MenuBarItem.xaml). | B3 | ported |
| `MenuFlyoutPresenter.xaml` | `{MenuFlyoutPresenter}` | MenuFlyoutPresenter: DefaultMenuFlyoutPresenterStyle (MenuFlyout_themeresources.xaml). | B3 | ported |
| `MenuItem.xaml` | `{MenuItem}` | MenuFlyoutItem: DefaultMenuFlyoutItemStyle; MenuFlyoutSubItem: DefaultMenuFlyoutSubItemStyle; ToggleMenuFlyoutItem: DefaultToggleMenuFlyoutItemStyle (MenuFlyout_themeresources.xaml); RadioMenuFlyoutItem: DefaultRadioMenuFlyoutItemStyle (RadioMenuFlyoutItem_themeresources.xaml). | B3 | ported |
| `MenuScrollViewer.xaml` | (keyed themes only) | The ScrollViewer of the MenuFlyoutPresenter template (MenuFlyout_themeresources.xaml). | B3 | ported |
| `NavigationPage.xaml` | `{NavigationPage}` | No WinUI control: NavigationBackButtonNormalStyle, NavigationBackButtonSmallStyle (NavigationBackButton.xaml) and the page background. | B5 | ported |
| `NotificationCard.xaml` | `{NotificationCard}` | InfoBar: {InfoBar} (InfoBar.xaml, InfoBar_themeresources.xaml). | B6 | ported |
| `NumericUpDown.xaml` | `{NumericUpDown}` | NumberBox: {NumberBox}, NumberBoxTextBoxStyle, NumberBoxSpinButtonStyle (NumberBox.xaml, NumberBox_themeresources.xaml). | B2 | ported |
| `OverlayPopupHost.xaml` | `{OverlayPopupHost}` | No WinUI style: the text of the window (TextFillColorPrimaryBrush, ContentControlThemeFontFamily, ControlContentThemeFontSize). | F | ported |
| `PathIcon.xaml` | `{PathIcon}` | No WinUI style (IconElement has no template): the Path that WinUI's PathIcon builds in code (CPathIcon::ApplyTemplate, icon.cpp), unscaled and filled with the Foreground. | B1 | ported |
| `PipsPager.xaml` | `{PipsPager}` | PipsPager: {PipsPager} (PipsPager.xaml), PipsPagerNavigationButtonBaseStyle, PipsPagerPreviousPageButtonStyle, PipsPagerNextPageButtonStyle, PipsPagerButtonBaseStyle, PipsPagerSelectedPipButtonStyle, PipsPagerNormalPipButtonStyle (PipsPager_themeresources.xaml). | B2 | ported |
| `PopupRoot.xaml` | `{PopupRoot}` | No WinUI style: the text of the window (TextFillColorPrimaryBrush, ContentControlThemeFontFamily, ControlContentThemeFontSize). | F | ported |
| `ProgressBar.xaml` | `{ProgressBar}` | ProgressBar: {ProgressBar} (ProgressBar.xaml, ProgressBar_themeresources.xaml), DefaultProgressBarStyle (Deprecated_themeresources.xaml). | B1 | ported |
| `RadioButton.xaml` | `{RadioButton}` | RadioButton: DefaultRadioButtonStyle (RadioButton_themeresources.xaml). | F | ported |
| `RefreshContainer.xaml` | `{RefreshContainer}` | RefreshContainer: DefaultRefreshContainerStyle (RefreshContainer.xaml). | B2 | ported |
| `RefreshVisualizer.xaml` | `{RefreshVisualizer}` | RefreshVisualizer: {RefreshVisualizer} (RefreshVisualizer.xaml). | B2 | ported |
| `RepeatButton.xaml` | `{RepeatButton}` | RepeatButton: DefaultRepeatButtonStyle (RepeatButton_themeresources.xaml). | F | ported |
| `ScrollBar.xaml` | `{ScrollBar}` | ScrollBar: DefaultScrollBarStyle (ScrollBar_themeresources.xaml); {Thumb} (generic.xaml). | F | ported |
| `ScrollViewer.xaml` | `{ScrollViewer}` | ScrollViewer: DefaultScrollViewerStyle (ScrollViewer_themeresources.xaml). | F | ported |
| `SelectableTextBlock.xaml` | `{SelectableTextBlock}` | No WinUI control (TextBlock.IsTextSelectionEnabled): the selection colours of the text selection manager of WinUI (SystemControlHighlightAccentBrush, SystemColorHighlightTextColor). | B1 | ported |
| `Separator.xaml` | `{Separator}` | MenuFlyoutSeparator: {MenuFlyoutSeparator} (MenuFlyout_themeresources.xaml); AppBarSeparator for a separator in a command bar. | B1 | ported |
| `Slider.xaml` | `{Slider}` | Slider: DefaultSliderStyle (Slider_themeresources.xaml). | B2 | ported |
| `SplitButton.xaml` | `{SplitButton}` | SplitButton: SplitButtonStyle (SplitButton.xaml), SplitButtonCommandBarStyle (SplitButton_themeresources.xaml); ToggleSplitButton. | B2 | ported |
| `SplitView.xaml` | `{SplitView}` | SplitView: DefaultSplitViewStyle (SplitView_themeresources.xaml). | B5 | ported |
| `TabControl.xaml` | `{TabControl}` | TabView: DefaultTabViewStyle (TabView.xaml, TabView_themeresources.xaml). | B5 | ported |
| `TabItem.xaml` | `{TabItem}` | TabViewItem: {TabViewItem} (TabView.xaml). | B5 | ported |
| `TabStrip.xaml` | `{TabStrip}` | SelectorBar: DefaultSelectorBarStyle (SelectorBar.xaml, SelectorBar_themeresources.xaml). | B5 | ported |
| `TabStripItem.xaml` | `{TabStripItem}` | SelectorBarItem: DefaultSelectorBarItemStyle, SelectorBarItemPill (SelectorBar.xaml, SelectorBar_themeresources.xaml). | B5 | ported |
| `TabbedPage.xaml` | `{TabbedPage}` | No WinUI control: NavigationView with its pane at the top (TopNavArea of NavigationView.xaml, MUX_NavigationViewItemPresenterStyleWhenOnTopPane of NavigationView_themeresources.xaml) and the page background. | B5 | ported |
| `TableView.xaml` | `{TableView}` | No control of the merged theme: WinUI's TableView of the tabular library (controls/dev/TableView, TableView_themeresources.xaml, not part of XamlControlsResources) is the closest source; the ListViewItem resources otherwise. | B6 | ported |
| `TableViewCell.xaml` | `{TableViewCell}` | As TableView: the cells of controls/dev/TableView. | B6 | ported |
| `TableViewColumnHeader.xaml` | `{TableViewColumnHeader}` | As TableView: the column headers of controls/dev/TableView and SortIndicator. | B6 | ported |
| `TableViewRow.xaml` | `{TableViewRow}` | As TableView: the rows of controls/dev/TableView; the ListViewItem resources. | B6 | ported |
| `TextBox.xaml` | `{TextBox}` | TextBox: DefaultTextBoxStyle and its DeleteButtonStyle (TextBox_themeresources.xaml); PasswordBox: DefaultPasswordBoxStyle (PasswordBox_themeresources.xaml), a keyed theme with the reveal button; the context flyout of TextCommandBarFlyout as a menu flyout (B3). | F | ported |
| `TextSelectionHandle.xaml` | `{TextSelectionHandle}` | No WinUI style (the selection grippers are drawn in code, Gripper.cpp): the gripper circles in SystemColorWindowColor outlined in TextControlSelectionHighlightColor. | B1 | ported |
| `ThemeVariantScope.xaml` | `{ThemeVariantScope}` | No WinUI style: the text foreground (TextFillColorPrimaryBrush). | F | ported |
| `TimePicker.xaml` | `{TimePicker}`, `{TimePickerPresenter}` | TimePicker: {TimePicker}; TimePickerFlyoutPresenter: {TimePickerFlyoutPresenter} (TimePicker_themeresources.xaml). | B4 | ported |
| `ToggleButton.xaml` | `{ToggleButton}` | ToggleButton: DefaultToggleButtonStyle (ToggleButton_themeresources.xaml). | F | ported |
| `ToggleSwitch.xaml` | `{ToggleSwitch}` | ToggleSwitch: DefaultToggleSwitchStyle (ToggleSwitch_themeresources.xaml). | B2 | ported |
| `ToolTip.xaml` | `{ToolTip}` | ToolTip: DefaultToolTipStyle (ToolTip_themeresources.xaml). | B1 | ported |
| `TransitioningContentControl.xaml` | `{TransitioningContentControl}` | No WinUI style: the template of ContentControl ({ContentControl}, generic.xaml), twice, for the page transition of the control. | B1 | ported |
| `TreeView.xaml` | `{TreeView}` | TreeView: DefaultTreeViewStyle (TreeView.xaml, TreeView_themeresources.xaml). | B3 | ported |
| `TreeViewItem.xaml` | `{TreeViewItem}` | TreeViewItem: {TreeViewItem} (TreeView.xaml), MUX_TreeViewItemStyle (TreeViewItem.xaml). | B3 | ported |
| `Window.xaml` | `{Window}` | No WinUI style (Window is not a control in WinUI 3): the Mica fallback background and text of a window (SolidBackgroundFillColorBaseBrush, TextFillColorPrimaryBrush, ContentControlThemeFontFamily, ControlContentThemeFontSize). | F | ported |
| `WindowDrawnDecorations.xaml` | `{WindowDrawnDecorations}` | TitleBar: DefaultTitleBarStyle (TitleBar.xaml, TitleBar_themeresources.xaml); WindowCaptionButton (generic.xaml) for the caption buttons. | B6 | ported |
| `WindowNotificationManager.xaml` | `{WindowNotificationManager}` | No WinUI control: a host of InfoBar cards (InfoBar_themeresources.xaml). | B6 | ported |

#### WinUI styles

Every target type of a style of the merged WinUI theme, the FerroUI control its implicit style themes, and the batch. A type that is not listed has no FerroUI control; its styles are waived (`waivers.json`).

| WinUI target type | FerroUI control | Batch | Note |
|---|---|---|---|
| `AppBarButton` | `CommandBarButton` | B5 |  |
| `AppBarSeparator` | `CommandBarSeparator` | B5 |  |
| `AppBarToggleButton` | `CommandBarToggleButton` | B5 |  |
| `AutoSuggestBox` | `AutoCompleteBox` | B3 |  |
| `Button` | `Button` | F |  |
| `ButtonBase` | `Button` | B4 | keyed styles of the date and time picker flyouts and TextBlockButtonStyle |
| `CalendarDatePicker` | `CalendarDatePicker` | B4 |  |
| `CalendarView` | `Calendar` | B4 |  |
| `CalendarViewDayItem` | `CalendarDayButton` | B4 |  |
| `CheckBox` | `CheckBox` | F |  |
| `ComboBox` | `ComboBox` | B3 |  |
| `ComboBoxItem` | `ComboBoxItem` | B3 |  |
| `CommandBar` | `CommandBar` | B5 |  |
| `CommandBarOverflowPresenter` | `CommandBar` | B5 | the overflow presenter is part of the CommandBar template in FerroUI |
| `ContentControl` | `ContentControl` | B1 | ContentControl has a built-in template in FerroUI; Label is built from it and claims the implicit style |
| `ContentDialog` | `ManagedFileChooserOverwritePrompt` | B6 |  |
| `ContentPresenter` | no FerroUI control | B1 | FerroUI's ContentPresenter is not templated; the implicit style is waived |
| `DatePicker` | `DatePicker` | B4 |  |
| `DatePickerFlyoutPresenter` | `DatePickerPresenter` | B4 |  |
| `DropDownButton` | `DropDownButton` | B2 |  |
| `Expander` | `Expander` | B2 |  |
| `FlipView` | `Carousel` | B5 |  |
| `FlipViewItem` | `Carousel` | B5 | the items of a Carousel are its content presenters |
| `FlyoutPresenter` | `FlyoutPresenter` | B3 |  |
| `HyperlinkButton` | `HyperlinkButton` | F |  |
| `InfoBar` | `NotificationCard` | B6 |  |
| `ListBox` | `ListBox` | B3 |  |
| `ListBoxItem` | `ListBoxItem` | B3 |  |
| `ListViewItem` | `ListBoxItem` | B3 | the keyed ListViewItem styles; the implicit one has no FerroUI control (no ListView) |
| `LoopingSelector` | no FerroUI control | B4 | the looping lists of the date and time picker presenters; FerroUI's DateTimePickerPanel is a panel, not templated, so the selectors are part of the DatePickerPresenter and TimePickerPresenter themes |
| `LoopingSelectorItem` | `ListBoxItem` | B4 | the items of the date and time picker presenters |
| `MenuBar` | `Menu` | B3 |  |
| `MenuBarItem` | `MenuItem` | B3 | the top-level items of a Menu |
| `MenuFlyoutItem` | `MenuItem` | B3 |  |
| `MenuFlyoutPresenter` | `MenuFlyoutPresenter` | B3 |  |
| `MenuFlyoutSeparator` | `Separator` | B1 |  |
| `MenuFlyoutSubItem` | `MenuItem` | B3 |  |
| `NavigationView` | no FerroUI control | B5 | no FerroUI control; DrawerPage is built from its pane |
| `NavigationViewItem` | no FerroUI control | B5 | keyed styles only; NavigationView has no FerroUI control |
| `NavigationViewItemHeader` | no FerroUI control | B5 |  |
| `NavigationViewItemPresenter` | no FerroUI control | B5 | NavigationView has no FerroUI control |
| `NavigationViewItemSeparator` | no FerroUI control | B5 |  |
| `NumberBox` | `NumericUpDown` | B2 |  |
| `PasswordBox` | `TextBox` | B3 | FerroUI's TextBox with a PasswordChar; ported as the keyed DefaultPasswordBoxStyle |
| `PipsPager` | `PipsPager` | B2 |  |
| `ProgressBar` | `ProgressBar` | B1 |  |
| `RadioButton` | `RadioButton` | F |  |
| `RadioMenuFlyoutItem` | `MenuItem` | B3 |  |
| `Rectangle` | no FerroUI control | B3 | keyed styles of template parts (ComboBoxItemPill, SelectorBarItemPill) |
| `RefreshContainer` | `RefreshContainer` | B2 |  |
| `RefreshVisualizer` | `RefreshVisualizer` | B2 |  |
| `RepeatButton` | `RepeatButton` | F |  |
| `ScrollBar` | `ScrollBar` | F |  |
| `ScrollViewer` | `ScrollViewer` | F |  |
| `SelectorBar` | `TabStrip` | B5 |  |
| `SelectorBarItem` | `TabStripItem` | B5 |  |
| `Shape` | no FerroUI control | B3 | ColorPickerBorderStyle (waived with ColorPicker) |
| `Slider` | `Slider` | B2 |  |
| `SplitButton` | `SplitButton` | B2 |  |
| `SplitMenuFlyoutItem` | no FerroUI control | B3 | no FerroUI counterpart: MenuItem has no split mode (waived) |
| `SplitView` | `SplitView` | B5 |  |
| `TabView` | `TabControl` | B5 |  |
| `TabViewItem` | `TabItem` | B5 |  |
| `TabViewListView` | `TabControl` | B5 | the strip of a TabControl |
| `TextBlock` | `TextBlock` | F | keyed styles (BodyTextBlockStyle, ...) become keyed control themes of TextBlock |
| `TextBox` | `TextBox` | F |  |
| `Thumb` | `Thumb` | F | the default theme of FerroUI's Thumb (the thumbs of ScrollBar and Slider have keyed themes) |
| `TimePicker` | `TimePicker` | B4 |  |
| `TimePickerFlyoutPresenter` | `TimePickerPresenter` | B4 |  |
| `TitleBar` | `WindowDrawnDecorations` | B6 |  |
| `ToggleButton` | `ToggleButton` | F |  |
| `ToggleMenuFlyoutItem` | `MenuItem` | B3 |  |
| `ToggleSplitButton` | `SplitButton` | B2 | FerroUI's ToggleSplitButton overrides its style key to SplitButton |
| `ToggleSwitch` | `ToggleSwitch` | B2 |  |
| `ToolTip` | `ToolTip` | B1 |  |
| `TreeView` | `TreeView` | B3 |  |
| `TreeViewItem` | `TreeViewItem` | B3 |  |

Target types without a FerroUI control (all their styles waived): `AnnotatedScrollBar`, `AppBar`, `AppBarElementContainer`, `BreadcrumbBar`, `BreadcrumbBarItem`, `ColorPicker`, `ColorPickerSlider`, `ColorSpectrum`, `CommandBarFlyoutCommandBar`, `Frame`, `GridView`, `GridViewHeaderItem`, `GridViewItem`, `GroupItem`, `Hub`, `HubSection`, `InfoBadge`, `InkToolbar`, `InkToolbarBallpointPenButton`, `InkToolbarCustomPenButton`, `InkToolbarCustomToggleButton`, `InkToolbarCustomToolButton`, `InkToolbarEraserButton`, `InkToolbarFlyoutItem`, `InkToolbarHighlighterButton`, `InkToolbarPenConfigurationControl`, `InkToolbarPencilButton`, `InkToolbarRulerButton`, `InkToolbarStencilButton`, `ItemContainer`, `ItemsView`, `ListPickerFlyoutPresenter`, `ListView`, `ListViewHeaderItem`, `MapControl`, `MediaPlayerElement`, `MediaTransportControls`, `PagerControl`, `PersonPicture`, `PickerFlyoutPresenter`, `Pivot`, `PivotHeaderItem`, `PivotItem`, `ProgressRing`, `RadioButtons`, `RatingControl`, `RichEditBox`, `RichTextBlock`, `ScrollView`, `SemanticZoom`, `SwipeControl`, `TeachingTip`, `TwoPaneView`.

<!-- mapping:end -->

## 7. Work batches

The remaining controls are split into six batches of similar size, each a set of controls that
share resources and helper themes. The foundation (batch F, this branch) is done: the crate, the
scripts, all resources, the window and popup hosts, the adorner layer, the theme variant scope,
the type ramp (`TextBlock`), Button (default, accent and subtle), RepeatButton, ToggleButton,
HyperlinkButton, CheckBox, RadioButton, Thumb, ScrollBar, ScrollViewer and TextBox.

| Batch | Controls (FerroUI) | Main WinUI sources | Notes |
|---|---|---|---|
| B1 Primitives and simple content | ToolTip, ProgressBar, Separator, PathIcon, Label, ItemsControl, HeaderedContentControl, GroupBox, SelectableTextBlock, TransitioningContentControl, TextSelectionHandle, DataValidationErrors, GridSplitter | ToolTip_themeresources, controls/dev/ProgressBar, MenuFlyoutSeparator, generic `{ContentControl}`/`{ContentPresenter}` | adds the DataValidationErrors wrapper to the TextBox template once DataValidationErrors is themed |
| B2 Buttons, ranges and expanders | DropDownButton, SplitButton (and ToggleSplitButton), ToggleSwitch, Slider, ButtonSpinner, NumericUpDown, Expander, PipsPager, RefreshContainer, RefreshVisualizer | DropDownButton, SplitButton, ToggleSwitch_themeresources, Slider_themeresources, NumberBox, Expander, PipsPager, PullToRefresh | Slider thumb as a keyed Thumb theme |
| B3 Flyouts, menus, lists and selection | FlyoutPresenter, MenuFlyoutPresenter, MenuScrollViewer, ContextMenu, MenuItem, Menu, ListBox, ListBoxItem, ComboBox, ComboBoxItem, AutoCompleteBox, TreeView, TreeViewItem | FlyoutPresenter, MenuFlyout, RadioMenuFlyoutItem, MenuBar, ListBox, ListViewItem, ComboBox, AutoSuggestBox, TreeView, PasswordBox | adds the context flyout of TextBox (the commands of TextCommandBarFlyout) and DefaultPasswordBoxStyle (TextBox with the reveal button) |
| B4 Calendar, date and time | Calendar, CalendarItem, CalendarButton, CalendarDayButton, CalendarDatePicker, DatePicker, TimePicker, DateTimePickerShared (the presenters) | CalendarView, CalendarDatePicker, DatePicker, TimePicker, DateTimePickerFlyout | LoopingSelector maps to the presenters' selectors |
| B5 Navigation, pages, tabs and command bar | TabControl, TabItem, TabStrip, TabStripItem, TabbedPage, CommandBar (buttons, toggle buttons, separators), SplitView, NavigationPage, ContentPage, DrawerPage, CarouselPage, Carousel | TabView, SelectorBar, CommandBar, AppBarButton, AppBarToggleButton, AppBarSeparator, SplitView, NavigationView, NavigationBackButton, FlipView | decides the NavigationView styles (pane of DrawerPage) |
| B6 Window decorations, notifications, tables and dialogs | WindowDrawnDecorations, WindowNotificationManager, NotificationCard, TableView, TableViewCell, TableViewColumnHeader, TableViewRow, ManagedFileChooser, ManagedFileChooserOverwritePrompt | TitleBar, generic `WindowCaptionButton`, InfoBar, ContentDialog, ListViewItem; controls/dev/TableView (outside the merged theme) | caption button glyphs as geometries |

What a batch does, per control:

1. Read the WinUI style(s) of the mapping table and the FerroUI control (its template parts,
   pseudo-classes and properties, `src/FerroUI.Controls`), and the Fluent theme's document for
   what the FerroUI control needs.
2. Write `Controls/<Control>.xaml` by sections 4.1 to 4.8 (`vsm.py` for the states), with the
   header comment naming the sources and the translation notes.
3. Add its include to `Controls/WinUIControls.xaml`, remove the control from `PENDING_CONTROLS`
   (`tests/support.rs`), add its test to `tests/control_templates.rs` and its document to
   `tests/documents.rs`.
4. Add glyph geometries to `Resources/Glyphs.xaml`; extend the converter or the waivers if a
   resource it needs is pending; never edit a generated document.
5. Record every deviation from the WinUI source in `docs/porting/winui-theme/DEVIATIONS.md`, a
   section `## <Control>` with one table row per deviation (an identifier, the WinUI source and
   key or element, what WinUI does, what the port does, why); a general rule goes to "General"
   once.
6. Run `convert.py --check` and `status.py`; the tracking must show no unresolved references and
   no duplicate keys, and the batch's styles as ported or waived. Commit the regenerated
   `PROGRESS.md`, `TRACKING.md`, `tracking.json` and `DEVIATIONS.md` in the same commit as the
   port.
7. Check the naming rule (`grep -rIli -E "avalonia|axaml|avares|\bavn" src` finds nothing new
   outside `NOTICE.md`). Agents do not run cargo; the main session builds and tests the branch.

## 8. Validation

### 8.1 Unit tests of the crate (`tests/`)

- `documents`: every document loads on its own with the run-time loader, and every resource it
  defines resolves in the whole theme in the light, dark and high contrast variants (one test per
  document); every document is tested; every resource document is merged; excluded documents are
  not.
- `control_theme_tests`: the rules of the upstream theme tests: no global styles; every
  templated control has a default control theme or is in `PENDING_CONTROLS` (reported, not
  failed; a pending control with a theme fails, so the list is kept current); the requested
  template parts; every dynamic resource of a setter is defined; no hardcoded brushes.
- `control_templates`: each themed control in a shown window gets its template, its parts and
  the values of its setters (following the base themes); the window follows the three variants.
- `resource_dictionary_tests`: the three theme dictionaries have the same keys.
- `theme_class_tests`, `public_api_tests`: the class is constructible and reachable by URI, the
  other documents are internal and not reachable, every document names its source.
- `winui_theme_tests`: the accent colours, the high contrast variant, the variants of the
  palette, the compact density, the theme in markup.
- `compiled_xaml_tests`: the compiled markup of the build equals the checked-in reference and
  the run-time emitter's output.

### 8.2 Building the crate

The checked-in `compiled_xaml.rs` and `compiled_xaml.xamlmeta` are written by a test, not by
hand. After the first build (and after every change of a document):

```sh
cargo build -p ferroui-themes-winui
cargo test -p ferroui-themes-winui --lib tests::compiled_xaml_tests::regenerate_compiled_xaml -- --ignored
cargo test -p ferroui-themes-winui --lib
```

The first build also adds the crate to `Cargo.lock` (CI builds with `--locked`), and
`scripts/release/readme-crates-table.sh --check` confirms the row of the README.

### 8.3 The gallery and the comparison with native WinUI 3

A gallery application written against the crate (by another agent) shows every control in the
three variants and both densities. It runs in the Windows virtual machine next to the WinUI 3
Gallery (or a small WinUI 3 application with the same controls), and the two are compared
screenshot by screenshot. The comparison calibrates what the port cannot take from the sources:
the sizes of the glyph geometries, the accent shades, the corner radius of the focus visuals,
the fonts outside Windows, and the timing of the transitions that replace visual state
animations. Differences found there are fixed in the port (or recorded here as deviations).
