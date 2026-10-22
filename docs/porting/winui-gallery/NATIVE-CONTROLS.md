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
and checked after the local run. All 521 current API evidence entries reference the
fresh [CI native-control record](data/executions/native-controls-006.json), which
records 182 passing tests on actual checkout `63e82a17`, tree `523b4d4e`, with main
`87fb`'s updated `Cargo.lock`. Its source hashes were extracted afterward from that
checkout's Git blobs, without an execution-time before/after snapshot. The published
merge `aae91cbe` has the same tree and was not the execution commit. These records do
not advance any behavior classification.

The [promotion validation record](data/native-promotion-validation.json) separately
records loader, theme, external-path and CI results, including corrected failures
and the scope of source-hash capture.
Historical records retain their original source paths, counts and hashes; loader,
theme and Gallery execution results remain separately scoped.
