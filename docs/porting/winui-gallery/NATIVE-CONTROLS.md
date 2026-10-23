# Migrating to native FerroUI controls

The implemented WinUI-derived controls now belong to FerroUI's normal control API.
This PR intentionally removes the earlier `ferroui_controls::winui` imports,
`FerroUI.Controls.WinUI` markup namespace and `WinUI*` control-builder aliases.
Update consumers to the native identities below. The existing WinUI source importer
and `ferroui_themes_winui` theme package retain their names and purposes.

## Concrete control mappings

The source names below are relative to `Microsoft.UI.Xaml.Controls`. Native controls
are available through the default `https://github.com/ferroui` XAML namespace.
Builders are exported by `ferroui_view::prelude` with ordinary native names.

| WinUI source type | Canonical FerroUI type | Builder |
|---|---|---|
| `AutoSuggestBox` | `FerroUI.Controls.AutoSuggestBox` | `auto_suggest_box()` |
| `FontIcon` | `FerroUI.Controls.FontIcon` | `font_icon()` |
| `Frame` | `FerroUI.Controls.Frame` | `frame()` |
| `InfoBadge` | `FerroUI.Controls.InfoBadge` | `info_badge()` |
| `InfoBar` | `FerroUI.Controls.InfoBar` | `info_bar()` |
| `Primitives.InfoBarPanel` | `FerroUI.Controls.Primitives.InfoBarPanel` | `info_bar_panel()` |
| `NavigationView` | `FerroUI.Controls.NavigationView` | `navigation_view()` |
| `NavigationViewItem` | `FerroUI.Controls.NavigationViewItem` | `navigation_view_item()` |
| `NavigationViewItemHeader` | `FerroUI.Controls.NavigationViewItemHeader` | `navigation_view_item_header()` |
| `NavigationViewItemSeparator` | `FerroUI.Controls.NavigationViewItemSeparator` | `navigation_view_item_separator()` |
| `NumberBox` | `FerroUI.Controls.NumberBox` | `number_box()` |
| `Page` | `FerroUI.Controls.FramePage` | `frame_page()` |
| `ProgressRing` | `FerroUI.Controls.ProgressRing` | `progress_ring()` |
| `RadioButtons` | `FerroUI.Controls.RadioButtons` | `radio_buttons()` |
| `RatingControl` | `FerroUI.Controls.RatingControl` | `rating_control()` |
| `SymbolIcon` | `FerroUI.Controls.SymbolIcon` | `symbol_icon()` |

These 16 controls have public Rust exports at `ferroui_controls::<Type>`.
`InfoBarPanel` also has its canonical module path
`ferroui_controls::primitives::InfoBarPanel`; its root re-export is the same type.
The source `Page` mapping requires the spelling `FramePage` in native XAML, Rust
and builders. Existing native `Page` and `PageProps` keep their original meaning.

## Native XAML and Rust

Remove the compatibility namespace declaration and prefix from adapted native
markup, including property elements and `x:Type` references:

```xml
<StackPanel xmlns="https://github.com/ferroui"
            xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
            Spacing="12">
  <NumberBox x:Name="Amount" Header="Amount" Value="42" />
  <AutoSuggestBox PlaceholderText="Search" />
  <InfoBar IsOpen="True" Severity="Success" Title="Ready" />
  <Frame x:Name="ContentFrame" />
</StackPanel>
```

Rust application code imports the controls directly:

```rust
use ferroui_controls::{Frame, FramePage, NumberBox};
use ferroui_controls::navigation::NavigationCacheMode;

let amount = NumberBox::new();
amount.set_numeric_value(42.0);
let page = FramePage::new();
page.set_navigation_cache_mode(NavigationCacheMode::Enabled);
let frame = Frame::new();
```

Custom Frame destinations derive `FramePage` and implement `FramePageImpl` in place
of the removed projected `Page` / `PageImpl` imports. The Gallery's
[frame_sample.rs](../../../samples/WinUIGallery/frame_sample.rs) shows a destination
factory and real navigation wiring. Declarative callers use
`ferroui_view::prelude::{number_box, frame, frame_page}` and the other builders above.

## Related API placement

| API family | Markup namespace | Rust location |
|---|---|---|
| Control enums and event arguments, including `Symbol`, `InfoBarSeverity` and `NavigationViewSelectionChangedEventArgs` | `FerroUI.Controls` | `ferroui_controls` |
| Icon sources and rating-item descriptions | `FerroUI.Controls` | `ferroui_controls` |
| Frame navigation options, modes, arguments, journal and `PageStackEntry` | `FerroUI.Navigation` | `ferroui_controls::navigation` |
| `InfoBadgeTemplateSettings`, `InfoBarTemplateSettings`, `ProgressRingTemplateSettings`, `InfoBarPanel` | `FerroUI.Controls.Primitives` | `ferroui_controls::primitives` |
| Automation peers | `FerroUI.Automation.Peers` | `ferroui_controls::automation::peers` |

The eight promoted peers belong to AutoSuggestBox, InfoBar, NavigationView,
NavigationViewItem, NumberBox, ProgressRing, RadioButtons and RatingControl.
`NavigationError` is the native fallible-operation error in the navigation module.
The Frame-specific `navigation::NavigationEventArgs` remains distinct from the
existing root `ferroui_controls::NavigationEventArgs`.

## Existing contracts and evidence

### Reusing controls already in FerroUI

The next Gallery pages use the existing native implementations below. They do not
introduce a second control identity just to retain the WinUI source spelling.
These are explicit adaptations; the API map keeps their semantic differences.

| Source feature | Existing native API | Adaptation and acceptance boundary |
|---|---|---|
| Inline `ColorPicker` | `ferroui_controls_color_picker::ColorView`, `ColorSlider`, `RadioButtons`, `TextBox` | Register the color-picker package and include its Fluent styles in the page scope. Independent alpha editors and More behavior are composed explicitly. Native `ColorPicker` keeps its dropdown contract. |
| `FlipView` | `ferroui_controls::Carousel`, `PipsPager`, `PageSlide` | Three original compositions use horizontal/vertical transitions and original item order. Source container APIs, button overlays, focus transfer and RTL behavior remain pending. |
| `Image` and decode sizing | `Image`, `Bitmap::decode_to_height` | Original JPEG/GIF bytes are embedded; the decoder controls bitmap size. SVG decoding and animated GIF playback remain pending. |
| Fixed-size `Image.NineGrid` samples | `CroppedBitmap`, `ImageDrawing`, `DrawingGroup`, `DrawingImage` | Nine native drawing patches retain the original corner pixels at the source example's 164 × 164 output size. This does not add a general NineGrid property to Image. |
| Connected animation | `ConnectedAnimationService`, `ConnectedAnimation`, `Frame`, `FramePage` | All four source compositions use the merged native composition services. Manual native item buttons replace source list/repeater containers; virtualization, exact XY focus, entrance transitions and acrylic remain separate work. |

For native inline color markup, register
`ferroui_controls_color_picker::register_types()` before loading the document:

```xml
<StackPanel xmlns="https://github.com/ferroui">
  <StackPanel.Styles>
    <StyleInclude Source="ferres://FerroUI.Controls.ColorPicker/Themes/Fluent/Fluent.xaml" />
  </StackPanel.Styles>
  <ColorView Color="CornflowerBlue" IsColorSpectrumSliderVisible="True" />
</StackPanel>
```

The package dependency enables `document-assets` for that embedded style include.
The source importer maps a WinUI `ColorPicker` to `ColorView` and emits `WXAML2006`.
`IsColorSliderVisible` and `IsColorChannelTextInputVisible` map to the native
`IsColorSpectrumSliderVisible` and `IsComponentTextInputVisible`. The importer
reports `WXAML1012` for source alpha enable/visibility and More members unless the
application supplies an explicit policy. In particular, disabling source alpha
editing preserves the selected alpha, while native `ColorView.IsAlphaEnabled`
coerces it to opaque. The Gallery therefore controls its external alpha editors
without changing the existing native contract.

### Preserved native contracts

The existing `Page`, `ContentPage` and native navigation hierarchy keep their
identities and asynchronous contracts. `FramePage` supplies the separate Frame
destination/cache lifecycle. `NumericUpDown` retains its original API and numeric
semantics; `NumberBox` has its own double/NaN contract. `AutoCompleteBox` retains its
existing completion behavior; `AutoSuggestBox` exposes its separate suggestion and
query events. Existing `IconElement` and `PathIcon` are reused directly.

Original upstream XAML still goes through
`ferroui_markup_xaml_loader::winui::{migrate_winui_xaml, WinUIXamlOptions}` or the
explicit `parse_winui_as` loader. That importer maps source identities and reports
unsupported syntax. Installing `ferroui_themes_winui::WinUITheme` remains a separate
theme choice.

Namespace promotion does not upgrade API or Gallery parity statuses. See the
[reviewed mappings](data/api-mappings.json), [current parity report](data/REPORT.md),
[control contracts](CONTROL-CONTRACTS.md) and
[validation ledger](../../../samples/WinUIGallery/VALIDATION.md) for exact scope and
execution evidence. The promotion's
[local native-control record](data/executions/native-controls-005.json) preserves
182 passing tests across 12 integration targets and 53 source hashes frozen before
and checked after the local run. All 521 current API evidence entries reference
[record 009](data/executions/native-controls-009.json): 182 tests passed across twelve
targets at CI checkout `47bbaf91`, including the Gallery dependency change and main
`934b9ce1`. Its 53 named input hashes come from that exact Git tree after execution.
The [local record 008](data/executions/native-controls-008.json) retains its separate
before/after hash scope and original lockfile; the newer main's additive lockfile
change required fresh execution evidence. Previous CI records
[006](data/executions/native-controls-006.json) and
[007](data/executions/native-controls-007.json) also remain unchanged. The exact
source guard and API behavior classifications remain unchanged.

The [promotion validation record](data/native-promotion-validation.json) separately
records loader, theme, external-path and CI results, including corrected failures
and the scope of source-hash capture.
Historical records retain their original source paths, counts and hashes; loader,
theme and Gallery execution results remain separately scoped.
