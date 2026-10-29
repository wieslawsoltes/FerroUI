# WinUI controls: native ports of WinUI 3 controls

This page is the plan and the register of the WinUI 3 controls that FerroUI ports as native controls: controls of WinUI that have no counterpart among the upstream-ported controls of `ferroui-controls`, or whose contract conflicts with the counterpart. It records the decisions, the inventory, the rules that keep the upstream-ported controls unchanged, the templates and fonts, the two navigation models, and the order in which the work lands.

The work comes from pull request #286 (branch `feat/winui-gallery-compat`), which remains a draft reference and is not merged as it is. It lands as a series of small pull requests from `main` (W0 to W11, section 9); #286 is closed when the last of them is merged.

## 1. Source and pin

- **Reference:** [microsoft/microsoft-ui-xaml](https://github.com/microsoft/microsoft-ui-xaml), branch `winui3/main`, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0` (MIT), the pin of the WinUI theme (`winui-theme.md`) and of every other WinUI port of the repository. #286 named `a71c42bd`; the sources of all its controls are byte-identical at both commits (`git diff 7e4b0c209 a71c42bd3` over the folders of section 3 is empty), so the change of pin changes no code.
- **Sources per control:** the controls of `controls/dev/<Control>/` (C++ and the `<Control>.xaml` and `<Control>_themeresources.xaml` documents), and for the controls of the framework layer (Frame, Page, SymbolIcon, FontIcon, AutoSuggestBox) `dxaml/xcp/dxaml/lib/<Control>_Partial.cpp` with the templates of `dxaml/xcp/dxaml/themes/generic.xaml`.
- **Gallery:** the sample `samples/WinUIGallery` (W10) ports pages of [microsoft/WinUI-Gallery](https://github.com/microsoft/WinUI-Gallery) at its own pin, `14061eea`, recorded in the sample's `sources.lock.json`.
- **Attribution:** the section "WinUI" of `src/FerroUI.Controls/NOTICE.md` (MIT, Copyright (c) Microsoft Corporation) names the files of each control as it lands, at the pin `7e4b0c2`; the converted templates are covered by `src/FerroUI.Themes.WinUI/NOTICE.md`.

## 2. Decisions (owner, 2026-10-24)

| # | Decision |
|---|---|
| O1 | The controls go into the core crate `ferroui-controls`, the navigation controls included, as #286 has them. Their XAML names are `FerroUI.Controls.*` in the default namespace `https://github.com/ferroui`; the types of navigation are in the namespace `FerroUI.Navigation` (module `ferroui_controls::navigation`). Repeater and Inking remain feature crates (decision D1 of 2026-10-23). |
| O2, O2b | Default templates are XAML control themes, converted from the WinUI documents at the pin with `scripts/winui-theme/convert.py` into the WinUI theme crate `ferroui-themes-winui`. No template is built in Rust, and the theme tests get no allow-list. The Fluent and Simple themes have no templates for these controls for now (section 5). |
| O3 | The behaviour code of #286 is kept for the leaf controls, with method names aligned to the C++ where they map one to one, and its differences are recorded. NavigationView lands with recorded deviations and moves onto ItemsRepeater in the follow-up F1. |
| O4 | The WinUI XAML importer and the `x:Bind` compatibility go into a new crate, `ferroui-markup-xaml-winui`. Only `xaml_error_scope` is added to `ferroui-markup-xaml`. |
| O5 | `FerroObject::has_local_value` is not added to `ferroui-base`; InfoBadge tracks the priority of its own `CornerRadius` changes. |
| O6 | UI Automation reports the real `SmallChange` and `LargeChange` of a range value provider (W1). |
| O7 | The checked-in `compiled_xaml.rs` of the themes stays one file; #286's split into parts is not taken. |
| O8 | Only reviewed maps and generated reports are committed; the per-run evidence and the large regenerable inventories of #286 are not. |
| O9 | The public API is guarded repository-wide by `cargo-semver-checks` against the published 0.4.0 and by a snapshot of the default values of properties (W11); #286's name baseline (`existing_api.py`) is not taken. |
| O10 | WinUI members are added to an upstream-ported control only as opt-in members whose default keeps upstream, each with a test that the default matches upstream and a row in `DEVIATIONS.md` (section 6). |
| O11 | The symbol font is the theme resource `SymbolThemeFontFamily`, not a hard-coded font; no font is bundled (section 5.2). |
| O12 | The gallery workflow runs on narrowed paths; its rendering job runs nightly and on demand. |
| O13 | The pin is `7e4b0c2`. |

## 3. Inventory

| WinUI control and types | WinUI source | FerroUI counterpart | Treatment | PR |
|---|---|---|---|---|
| `IconSource`, `FontIconSource`, `SymbolIconSource`, `PathIconSource` | `controls/dev/IconSource` | `IconElement`, `PathIcon` (upstream) | New types; `PathIconSource` creates the existing `PathIcon`. The abstract base creates no element (WinUI's base returns null). | W4 |
| `SymbolIcon`, `Symbol` (197 values) | `dxaml/xcp/dxaml/lib/SymbolIcon_Partial.cpp`, `CSymbolIcon::ConvertSymbolValueToGlyph` | `IconElement` | New, derived from `IconElement`; the font from `SymbolThemeFontFamily`. | W4 |
| `FontIcon` | `dxaml/xcp/dxaml/lib/FontIcon_Partial.cpp` | `IconElement` | New, derived from `IconElement`; the default font from `SymbolThemeFontFamily`. | W4 |
| `InfoBar`, `InfoBarPanel`, `InfoBarTemplateSettings`, the closing and closed arguments | `controls/dev/InfoBar` | `NotificationCard` (a toast, another role) | New; `InfoBarPanel` is a structural port of `InfoBarPanel.cpp`. | W5 |
| `InfoBadge`, `InfoBadgeTemplateSettings` | `controls/dev/InfoBadge` | none | New. | W5 |
| `ProgressRing`, `ProgressRingTemplateSettings` | `controls/dev/ProgressRing` | `ProgressBar` (another control in WinUI too) | New; Value, Minimum and Maximum registered on the control as in WinUI, not through `RangeBase`. The Lottie visual is drawn with `Arc` shapes (a recorded deviation). | W5 |
| `NumberBox`, `INumberBoxNumberFormatter`, the spin button placement, validation mode and value-changed arguments | `controls/dev/NumberBox` | `NumericUpDown` (upstream; `Option<Decimal>`, TwoWay, routed `ValueChanged`) | A separate type: the value domain (`f64` with NaN), binding default, event kind, coercion policy and spinner default conflict. The expression evaluator is shared with `NumericUpDown` (W7). | W6 |
| `AutoSuggestBox`, its text-changed, suggestion-chosen and query-submitted arguments, `AutoSuggestionBoxTextChangeReason` | `dxaml/xcp/dxaml/lib/AutoSuggestBox_Partial.cpp`, `controls/dev/AutoSuggestBox` | `AutoCompleteBox` (upstream; the control filters) | A separate type: in WinUI the application filters on `TextChanged`, with a reason. | W6 |
| `RadioButtons` | `controls/dev/RadioButtons` | `RadioButton` (upstream) | New; `RadioButton` is its container, and the spacing comes from the `RadioButtonsColumnSpacing` and `RadioButtonsRowSpacing` resources. | W6 |
| `RatingControl`, `RatingItemInfo`, `RatingItemFontInfo`, `RatingItemImageInfo` | `controls/dev/RatingControl` | none | New. | W6 |
| `Frame`, `FramePage` (WinUI `Page`), the navigation types (`NavigationMode`, `NavigationCacheMode`, `NavigationEventArgs`, `NavigatingCancelEventArgs`, `NavigationFailedEventArgs`, `PageStackEntry`) | `dxaml/xcp/dxaml/lib/Frame_Partial.cpp`, `Page_Partial.cpp`, `NavigationHistory.cpp` | `Page`, `NavigationPage`, `PageNavigationHost` (upstream) | Separate types (section 7). `FramePage` keeps the name `Page` for upstream; the navigation types are in `ferroui_controls::navigation`, so `navigation::NavigationEventArgs` does not collide with the upstream `NavigationEventArgs`. | W8 |
| `NavigationView`, `NavigationViewItemBase`, `NavigationViewItem`, `NavigationViewItemHeader`, `NavigationViewItemSeparator`, their events and arguments | `controls/dev/NavigationView` | `SplitView` (upstream; NavigationView builds on it), `DrawerPage` (upstream; its WinUI template is drawn after the pane of NavigationView) | New. Items are hosted without ItemsRepeater and `SelectionModel` until F1; the overflow of the top pane, the transitions and `NavigationViewItemPresenter` are recorded as missing. | W8 |
| Automation peers of the controls above | `controls/dev/*/*AutomationPeer.cpp` | | New peers in `ferroui_controls::automation::peers`. | W4 to W8 |
| WinUI XAML importer, `x:Bind` | (FerroUI) | | `ferroui-markup-xaml-winui`. | W9 |

Gallery pages that use existing FerroUI controls (ColorPicker as `ColorView`, FlipView as `Carousel` with `PipsPager`, `ConnectedAnimationService`) add no control.

## 4. Where WinUI compatibility lives

| WinUI surface | Mechanism | Location |
|---|---|---|
| A control with no FerroUI counterpart, or with a conflicting contract | A native type with the WinUI name | `ferroui-controls` (this page) |
| A member with the same meaning and another spelling (`ColorPicker.IsColorSliderVisible` and `ColorView.IsColorSpectrumSliderVisible`) | A mapping of the importer, per document, with a diagnostic; no change of the API | `ferroui-markup-xaml-winui` |
| A capability that does not conflict with an upstream-ported control | An opt-in member of that control (section 6) | `ferroui-controls` |
| Markup semantics of WinUI only (`x:Bind` and its lifecycle, `ThemeResource` policy, `ms-appx:` URIs) | A markup extension or attached property of the compatibility crate, reached through the prefix the importer writes | `ferroui-markup-xaml-winui` |
| The WinUI look of the upstream-ported controls | The WinUI theme | `ferroui-themes-winui` |
| The WinUI look of the controls of this page | Control themes converted with `convert.py` | `ferroui-themes-winui` |

A separate namespace for the controls (`ferroui_controls::winui`, `FerroUI.Controls.WinUI`) is not used: the importer writes WinUI source into the default namespace.

## 5. Templates, themes and fonts

### 5.1 Templates

- Each templated control of this page gets its default template from a control theme of the WinUI theme, `src/FerroUI.Themes.WinUI/Controls/<Control>.xaml`, ported by hand from the WinUI document at the pin (`controls/dev/<Control>/<Control>.xaml`) as every control theme of the WinUI theme is: the visual states become pseudo-class selectors, and each difference is a row of `docs/porting/winui-theme/DEVIATIONS.md`. Its resources, the `<Control>_themeresources.xaml` documents, are converted by `scripts/winui-theme/convert.py` into `Resources/`: generated files, regenerated with the script, never edited by hand. A control WinUI draws in code (FontIcon, SymbolIcon, as PathIcon) gets a control theme with the visual its code builds.
- No template is built in Rust, and no template child is styled from code with resource keys or colour literals. The theme tests require a theme for every templated control registered by the theme's controls; there is no allow-list.
- **The themes derived from the WinUI theme style them too.** The macOS 26 theme (`ferroui-themes-macos`, `docs/porting/macos26-theme.md`) and the Material theme (`ferroui-themes-material`, `docs/porting/material-theme.md`) are forked from the WinUI theme and keep a control theme for every control it styles (`scripts/macos-theme/audit.py` checks it for macOS, and `scripts/themes/resource_keys.py` the resource keys of both): each control of this page gets its document in both, adapted the way the theme adapts the other WinUI templates.
- **Fluent and Simple have no templates for these controls for now.** In an application with the Fluent or the Simple theme a templated control of this page has no template, and so shows nothing: use the WinUI, macOS or Material theme for applications that use them. Fluent and Simple documents may follow later; this section is updated when they do.
- The controls that only the WinUI theme and the themes derived from it style are listed in one place, `ferroui_controls::testing::WINUI_ONLY_CONTROLS` (feature `testing`; the name says where their templates come from), with the table below; the theme tests of Fluent and Simple check that these controls have no control theme there (and every other templated control has one), and a test of `ferroui-controls` checks that the table names the same controls.

<!-- winui-only:begin -->
| Control | Why only the WinUI, macOS and Material themes | Since |
|---|---|---|
| `FontIcon` | WinUI's FontIcon: its text part is in the WinUI-derived themes only | W4 |
| `SymbolIcon` | WinUI's SymbolIcon: its text part is in the WinUI-derived themes only | W4 |
| `InfoBar` | WinUI's InfoBar: its template is converted from InfoBar.xaml into the WinUI theme and its derived themes | W5 |
| `InfoBadge` | WinUI's InfoBadge: its template is converted from InfoBadge_themeresources.xaml into the WinUI theme and its derived themes | W5 |
| `ProgressRing` | WinUI's ProgressRing: its template (arcs for the Lottie player) is in the WinUI-derived themes only | W5 |
| `NumberBox` | WinUI's NumberBox: its template is converted from WinUI's NumberBox.xaml into the WinUI theme and its derived themes | W6 |
| `AutoSuggestBox` | WinUI's AutoSuggestBox: its template is converted from WinUI's DefaultAutoSuggestBoxStyle into the WinUI theme and its derived themes | W6 |
| `RadioButtons` | WinUI's RadioButtons: its template and items panel are converted from WinUI's RadioButtons.xaml into the WinUI theme and its derived themes | W6 |
| `RatingControl` | WinUI's RatingControl: its template is converted from WinUI's RatingControl.xaml into the WinUI theme and its derived themes | W6 |
| `Frame` | WinUI's Frame: its template is converted from the Frame style of WinUI's generic.xaml into the WinUI theme and its derived themes | W8 |
| `NavigationView` | WinUI's NavigationView: its template is converted from WinUI's NavigationView.xaml into the WinUI theme and its derived themes | W8 |
| `NavigationViewItem` | WinUI's NavigationViewItem: its template is converted from NavigationView.xaml with the item presenter styles into the WinUI theme and its derived themes | W8 |
| `NavigationViewItemHeader` | WinUI's NavigationViewItemHeader: its template is converted from WinUI's NavigationView.xaml into the WinUI theme and its derived themes | W8 |
| `NavigationViewItemSeparator` | WinUI's NavigationViewItemSeparator: its template is converted from WinUI's NavigationView.xaml into the WinUI theme and its derived themes | W8 |
<!-- winui-only:end -->
- The tables of `winui-theme.md` that say "no FerroUI control" for these controls (NavigationView and its items, RadioButtons, RatingControl; DRP-1, DRP-2, DRP-11, DRP-13, NVP-1 and G-23 of the theme's DEVIATIONS) are updated in the pull request that lands each control; W5 updated them for InfoBar, InfoBadge and ProgressRing, W6 for RadioButtons and RatingControl, W8 for NavigationView and its items.

### 5.2 The symbol font

WinUI draws `SymbolIcon`, the default `FontIcon` and the glyphs of its templates (the back, pane toggle, settings and chevron glyphs of NavigationView, the stars of RatingControl, the severity icons of InfoBar) with the font of the theme resource `SymbolThemeFontFamily`, `Segoe Fluent Icons,Segoe MDL2 Assets` on Windows 11.

- The controls read the font from the `SymbolThemeFontFamily` resource, which the WinUI theme defines with WinUI's value. No font name is hard-coded in the controls.
- **The font is not redistributable and is not bundled.** On Windows 10 and 11 the system has it. On macOS, Linux, the browser, iOS and Android an application that uses these glyphs supplies a symbol font with the same code points and sets `SymbolThemeFontFamily` to it in its resources; without one the glyphs are drawn with the fallback font, usually as missing-glyph boxes.
- Glyphs the templates themselves need may be drawn from path outlines in the theme, as `Resources/Glyphs.xaml` of the WinUI theme does for a few glyphs; each such case is recorded in the theme's DEVIATIONS.

## 6. WinUI additions to upstream-ported controls

The upstream-ported controls keep their upstream contract, defaults and behaviour. A WinUI capability is added to one of them only when all of these hold:

1. it does not conflict with the upstream contract;
2. it is opt-in: its default reproduces upstream exactly;
3. a test checks that the default reproduces upstream, and another test checks the WinUI behaviour;
4. it has a row in `DEVIATIONS.md`, section "WinUI additions to upstream controls", with the WinUI member, the default that keeps upstream, the kind **Behaviour (opt-in)** and the pull request.

When one of these fails, the WinUI control is a separate type. The additions planned (W7):

| Control | Member | Default (upstream unchanged) | WinUI source |
|---|---|---|---|
| `NumericUpDown` | `AcceptsExpression: bool` | `false`: the upstream parsing | `NumberBox.AcceptsExpression` |
| `NumericUpDown` | `LargeChange: Option<Decimal>` with PageUp and PageDown | `None`: the keys stay unhandled and bubble, as upstream | `NumberBox.LargeChange` |
| `NumericUpDown` | `IsWrapEnabled: bool` | `false`: no wrapping | `NumberBox.IsWrapEnabled` |
| `AutoCompleteBox` | `QuerySubmitted` (routed, bubbling) with `AutoCompleteBoxQuerySubmittedEventArgs` | no handler: nothing changes; Enter is marked handled only when the drop-down is open, as upstream | `AutoSuggestBox.QuerySubmitted` |

Not added: the WinUI text-changed reason of AutoSuggestBox (AutoCompleteBox's routed `TextChanged` keeps its upstream arguments), the NaN semantics and validation mode of NumberBox, and the header and description of NumberBox (they belong to the layout of the upstream templates).

The default values of the properties of the upstream-ported controls are guarded by the snapshot of W11, so a change of a default fails the build.

## 7. Two navigation models

FerroUI has two navigation models, one from each source. They are separate on purpose, and neither replaces the other.

| | Upstream pages | WinUI frame |
|---|---|---|
| Types | `Page` (a `TemplatedControl`), `ContentPage`, `NavigationPage`, `DrawerPage`, `TabbedPage`, `CarouselPage`, `PageNavigationHost` | `Frame` (a `ContentControl`), `FramePage` (a `UserControl`, WinUI's `Page`), `navigation::PageStackEntry` |
| What is navigated | Page instances: `push_async(page)` | Page types: `navigate(type, parameter)`, a page made by a registered `PageFactory` or by the type's markup constructor |
| Timing | Asynchronous: `push_async`, `pop_async` and `pop_to_root_async` return a `DispatcherTask`; `NavigatingFrom` handlers may defer | Synchronous: `navigate`, `go_back` and `go_forward` complete before they return; `Navigating` may cancel |
| History | The navigation stack of `NavigationPage` and the modal stack | A back stack and a forward stack of entries (type, parameter, transition), `CacheSize` and `NavigationCacheMode` (disabled, enabled within the cache size, required) |
| Events | `NavigatedTo`, `NavigatedFrom`, `NavigatingFrom` with `NavigationType` (push, pop, replace, ...) | `Navigating`, `Navigated`, `NavigationStopped`, `NavigationFailed` with `NavigationMode` (new, back, forward, refresh); `on_navigated_to`, `on_navigating_from`, `on_navigated_from` of `FramePage` |
| Shell | `DrawerPage`, `TabbedPage` | `NavigationView` with a `Frame` in its content |
| Source | upstream's page hierarchy | WinUI's `Frame`, `Page` and `NavigationView` |

Use the upstream pages for an application written against upstream's model, and `Frame` with `NavigationView` for one written against WinUI's (and for WinUI source brought in through the importer). A `Frame` accepts only `FramePage`; a bridge that would let it host an upstream `Page` is not planned. `Frame` and `NavigationView` have their templates in the WinUI theme only (section 5.1); `FramePage` takes the theme of `UserControl`, as WinUI's `Page` has no style of its own. The view builders name WinUI's arguments `navigation::NavigationEventArgs` `FrameNavigationEventArgs` (`SCOPED_PATHS` of `scripts/generate_view_builders.py`).

## 8. API stability

- The controls of this page are additions to `ferroui-controls`: no existing member changes. Within 0.4 the public API only grows.
- W11 adds a repository-wide gate to `ci.yml`: `cargo semver-checks check-release` of the published crates against 0.4.0 from crates.io, and a snapshot test of the default values of the registered properties (owner, name, value type, default for the owner, default binding mode, inheritance) compared with a checked-in snapshot.
- The controls of this page override no metadata of other types.

## 9. Landing plan

Each pull request carries its tests, its `DEVIATIONS.md` rows and a line in `CHANGELOG.md` under "Unreleased"; generated files are regenerated with their scripts. W4 to W10 change the WinUI theme's `compiled_xaml.rs` and the registration of the controls, and start after #269 (Repeater), #283 (Inking) and #313 (hot reload) are merged.

| PR | Content | Depends on | Windows VM |
|---|---|---|---|
| W0 | This page; the section "WinUI additions to upstream controls" of `DEVIATIONS.md` | none | no |
| W1 | UI Automation: the `SmallChange` and `LargeChange` identifiers of the range value pattern, their UIA mapping, and Win32 getters that return the provider's values | none | yes: UIA with Inspect and the Appium tests |
| W2 | `xaml_error_scope` in `ferroui-markup-xaml`: a typed error out of deferred resource construction during a scoped load | none | no |
| W3 | Tooling: view builder groups and scoped builder names, the casing of the WinUI source paths shared with #283, the theme script tests in CI | #283 | no |
| W4 | Icons: the IconSource family, SymbolIcon, FontIcon, `SymbolThemeFontFamily` | W3 | yes: the glyphs with Segoe Fluent Icons |
| W5 | Status: InfoBar with InfoBarPanel, InfoBadge, ProgressRing | W4 | yes: side by side with WinUI, the peers |
| W6 | Input: NumberBox, AutoSuggestBox, RadioButtons, RatingControl | W4, W1 | yes: RangeValue of the peers, keyboard, IME |
| W7 | The opt-in additions of section 6 to NumericUpDown and AutoCompleteBox | W6 | no |
| W8 | Navigation: Frame, FramePage, the navigation types, NavigationView and its items | W4, W5, W6 | yes: adaptive widths, focus, UIA tree |
| W9 | `ferroui-markup-xaml-winui`: the importer, `x:Bind` compatibility, `parse_winui*` | W2, W8 | no |
| W10 | `samples/WinUIGallery` and its workflow | W4 to W9 | yes: the gallery, fonts, DPI |
| W11 | The API gate of section 8 | none | no |
| F1 | NavigationView items on ItemsRepeater and the hierarchical `SelectionModel` | W8, #269 | yes |
| F2, F3 | The acrylic pane background (#325); ThemeShadow on the AutoSuggestBox popup and the NavigationView flyouts (#326) | #325, #326 | yes |

## 10. Deviations

- Differences from WinUI of each control are listed in the section of the control below as it lands.
- Changes to upstream-ported controls and to core crates are in `DEVIATIONS.md`: "WinUI additions to upstream controls", "Automation peers" and "Windows platform backend" (W1), and the markup section (W2).
- Differences of the converted templates are in `docs/porting/winui-theme/DEVIATIONS.md`.

### 10.1 Icons (W4)

`src/FerroUI.Controls/font_icon.rs`, `symbol_icon.rs`, `icon_source.rs`; themes `Controls/FontIcon.xaml` and `Controls/SymbolIcon.xaml` of the WinUI theme. Tests: `font_icon_tests`, `symbol_icon_tests`, `icon_source_tests` (`ferroui-controls`), `the_icons_draw_in_the_symbol_font_of_the_theme` (`ferroui-themes-winui`). `Symbol` is checked against WinUI by `scripts/winui-controls/symbol_map.py --check` and the table `docs/porting/data/winui-controls/symbol-glyphs.tsv`.

| WinUI | Port | Kind | Why |
|---|---|---|---|
| `FontIcon` and `SymbolIcon` build a TextBlock in code (`CFontIcon::ApplyTemplate`, `CSymbolIcon::ApplyTemplate`). | The TextBlock is the part `PART_Text` of the control theme of the WinUI theme; the control gives it the glyph, the text properties and the foreground. | Representation | FerroUI's `IconElement` is a templated control (upstream). |
| `SymbolIcon` writes the font `Segoe Fluent Icons` in code; the default font of a `FontIcon` is the system's symbol font. | Both use the theme resource `SymbolThemeFontFamily` (section 5.2). | Behaviour | Owner decision O11: an application can supply a symbol font off Windows. |
| `FontIcon.IsTextScaleFactorEnabled`, `FontIconSource.IsTextScaleFactorEnabled` | Not ported. | Missing | FerroUI has no text scale factor. |
| `FontIconSource.FontFamily` is a nullable font family; null leaves the icon's own. | `Option<FontFamily>`, `None` by default; a change to `None` clears the value it had set on the icons. | Representation | Rust has no null font family. |
| `IconSource::CreateIconElementCore` of the abstract base returns `E_NOTIMPL`, and `CreateIconElement` no element. | `create_icon_element` returns `Option<Ref<IconElement>>`: `None` for the base. | Representation | #286 panicked there; WinUI's base creates no element. |
| `BitmapIconSource`, `ImageIconSource`, `AnimatedIconSource`, `BitmapIcon`, `ImageIcon`, `AnimatedIcon` | Not ported. | Missing | Not in #286; candidates for later. |

### 10.2 Status (W5)

`src/FerroUI.Controls/info_bar.rs`, `info_badge.rs`, `progress_ring.rs`, `primitives/info_bar_panel.rs`, `primitives/info_bar_template_settings.rs`, `primitives/info_badge_template_settings.rs`, `primitives/progress_ring_template_settings.rs`, `automation/peers/info_bar_automation_peer.rs`, `automation/peers/progress_ring_automation_peer.rs`; themes `Controls/InfoBar.xaml`, `Controls/InfoBadge.xaml` and `Controls/ProgressRing.xaml` of the WinUI theme, with the converted resources `Resources/InfoBadge_themeresources.xaml`, `Resources/ProgressRing_themeresources.xaml` and the `Symbol` resource `InfoBarCloseButtonSymbol` (a rule of `convert.py`). Tests: `info_bar_tests`, `info_badge_tests`, `progress_ring_tests`, `primitives::info_bar_panel_tests` (`ferroui-controls`); `info_bar`, `info_badge`, `progress_ring` and `the_status_controls_show_their_states` of `control_templates`, and the document tests (`ferroui-themes-winui`). The differences of the templates are INB-1 to INB-8, IBD-1 to IBD-4 and PRG-1 to PRG-4 of the theme's DEVIATIONS.

The controls set the states of WinUI (`GoToState`) as pseudo-classes, which the control themes style: InfoBar `:informational`, `:success`, `:warning`, `:error`, `:standard-icon-visible`, `:user-icon-visible`, `:close-button-collapsed`, `:open`, `:foreground-set`, `:no-banner-content`; InfoBadge `:dot`, `:icon`, `:font-icon`, `:value`; ProgressRing `:active`, `:determinate-active`, `:inactive`. The template parts the code looks up keep WinUI's names (`CloseButton`, `StandardIcon`, `LayoutRoot`), except the two of the port: `PART_ContentPresenter` of InfoBar (WinUI's `ContentArea`) and the arc `PART_Indicator` of ProgressRing. ProgressRing registers `Value`, `Minimum` and `Maximum` on the control, as WinUI does, not through `RangeBase`, and `IsIndeterminate` is `true` by default. `InfoBarPanel` is a structural port of `InfoBarPanel.cpp` (`MeasureOverride`, `ArrangeOverride`); the WinUI theme keeps its own port of the class (`ferroui_themes_winui::primitives::InfoBarPanel`) for the template of its notifications.

| WinUI | Port | Kind | Why |
|---|---|---|---|
| `InfoBar` is a `Control` with its own `Content` and `ContentTemplate`. | `InfoBar` is a `ContentControl`; its template presents the content in `PART_ContentPresenter`. | Representation | #286's model, kept (O3): FerroUI's content controls host their content through that presenter. |
| `InfoBar::UpdateForeground` and `InfoBadge::OnSizeChanged` ask whether `Foreground` and `CornerRadius` are local (`ReadLocalValue(...) == UnsetValue`). | The controls follow it from the priority of the changes of the property (`FerroPropertyChangedEventArgs::priority`): a change at the local priority makes it local, a change of the effective value at a lower priority makes it not local, a change of an animation leaves it. A local value equal to the value it replaces raises no change and is not seen. | Behaviour | Owner decision O5: no `FerroObject::has_local_value`. |
| `InfoBadge.Value` below -1 throws `hresult_out_of_bounds` (`InfoBadge::OnPropertyChanged`). | The value is coerced to -1 and a warning is logged (area `Control`). | Behaviour | A value from markup or a binding must not end the application. |
| `InfoBarAutomationPeer::RaiseOpenedEvent` and `RaiseClosedEvent`: UI Automation notifications with the severity, the title and the message when the bar opens and closes. | Not raised. The peer reports the title and the message as its name when the application gives none, raises the change of its name, and its live setting is assertive for warnings and errors and polite otherwise when the application sets none. | Missing | FerroUI's automation has no notification events; the name and the live setting carry the message. |
| `SetLocalizedLandmarkType(..., "InfoBar")`; the style sets `AutomationProperties.LandmarkType="Custom"` and `IsDialog="True"`. | Left out. | Missing | FerroUI's automation has no custom landmark and no `IsDialog`. |
| The texts of `ResourceAccessor` (`InfoBarCloseButtonName`, `InfoBarCloseButtonTooltip`, `InfoBarIconSeverity*Name`, `ProgressRingName`, `ProgressRingIndeterminateStatus`), localized. | The English texts of `Strings/en-us/Resources.resw`: "Close", "Informational icon", "Success icon", "Warning icon", "Error icon", "ProgressRing", "Busy". | Behaviour | FerroUI has no localized resources of its framework; no resource key is looked up in code. |
| `InfoBar::OnCloseButtonClick` sets `IsOpen(false)`, a cancelled closing `IsOpen(true)`; `ProgressRing::Coerce*` set `Minimum`, `Maximum` and `Value`. | The current value is set (`set_current_value`). | Behaviour | A binding of the property stays, as with FerroUI's other coercions; #286's choice. |
| `InfoBar.Opened` is raised in prerelease builds only (`MUX_PRERELEASE`). | Always raised, after the bar opens. | Behaviour | #286's contract; the event is in WinUI's API at the pin. |
| `CloseButtonClick`, `Closing`, `Closed`, `Opened` are `TypedEventHandler` events. | Handler lists: `close_button_click`, `closing`, `closed` and `opened` return a disposable that removes the handler; markup subscribes with `CloseButtonClick` and the others. `InfoBarClosingEventArgs` clones share the cancellation. | Representation | The events of FerroUI's plain Rust classes. |
| `CloseButtonStyle` is a `Style`; `ActionButton` is a `ButtonBase`. | `Option<Ref<ControlTheme>>`; `Option<Ref<Button>>`. | Representation | FerroUI's styles of controls are control themes; FerroUI's `Button` is the base of its buttons. |
| The close button gets a `ToolTip` with the text as its content. | The text is the tip (`ToolTip::set_tip`). | Representation | FerroUI's tool tips take any value. |
| `InfoBadge.Value` has the setter `Value(int)`; `ProgressRing.Value` `Value(double)`. | `set_numeric_value` and `set_range_value` (the getters are `value`). | Representation | `set_value` is the setter of properties of every FerroUI object; `set_range_value` is the name `RangeBase` uses. |
| `ProgressRing` plays the Lottie compositions `ProgressRingIndeterminate` and `ProgressRingDeterminate` in the `AnimatedVisualPlayer` of its template (`PlayAsync(0, 1, true)`, `PlayAsync(from, to, false)`, `SetProgress`), coloured with its `Foreground` and `Background`. | The template draws the ring with two `Arc` shapes (PRG-1) and the control moves the angles of `PART_Indicator` as the compositions play: the indeterminate one loops over two seconds (900 degrees of rotation, the arc growing to half the ring and shrinking, linear as its key frames), an increase of the determinate one plays from the old to the new progress at two seconds for the whole ring, a decrease shows at once. The brushes are bound by the template. | Representation | FerroUI has no `AnimatedVisualPlayer` and no Lottie. |
| The compositions are driven by the composition clock of Windows. | The ring's `Clock` or the global clock; the ring subscribes only while active, effectively visible, in the visual tree and with a `PART_Indicator`. | Behaviour | FerroUI's animations take their time from a clock. |
| `ProgressRing.DeterminateSource`, `IndeterminateSource` (preview members). | Not ported. | Missing | They take an `IAnimatedVisualSource`, which FerroUI does not have. |
| `ProgressRing::UpdateLottieProgress` divides by the range as it is. | `normalized_value` is 0 for a value that is not a number and for an empty or infinite range, and clamped to 0..1; the properties keep their values. | Behaviour | The arc must get finite angles. |
### 10.3 Input (W6)

`src/FerroUI.Controls/number_box.rs`, `utils/number_expression.rs`, `auto_suggest_box.rs`, `radio_buttons.rs`, `primitives/radio_buttons_panel.rs`, `rating_control.rs`, `rating_item_info.rs` and the peers `automation/peers/number_box_automation_peer.rs`, `auto_suggest_box_automation_peer.rs`, `radio_buttons_automation_peer.rs`, `rating_control_automation_peer.rs`; themes `Controls/NumberBox.xaml`, `Controls/AutoSuggestBox.xaml`, `Controls/RadioButtons.xaml` and `Controls/RatingControl.xaml` of the WinUI theme, with the converted `Resources/RadioButtons_themeresources.xaml` and `Resources/RatingControl_themeresources.xaml` (the NumberBox and AutoSuggestBox resources were already converted for NumericUpDown and AutoCompleteBox, whose themes do not change). Tests: `number_box_tests`, `number_expression_tests` (the evaluator with `f64` and `Decimal`), `auto_suggest_box_tests`, `radio_buttons_tests`, `rating_control_tests` (`ferroui-controls`, with the parts of the templates in `input_test_support`), `multiplication_adds_the_scales` (`ferroui-base`), `the_input_controls_get_their_parts_and_resources_from_the_theme` (`ferroui-themes-winui`). The differences of the templates are NBX, ASB, RBS and RTG of the theme's DEVIATIONS.

The expression evaluator of NumberBox is `ferroui_controls::utils::number_expression`: one public generic function, `evaluate_number_expression`, over the trait `ExpressionNumber`, implemented for `f64` (WinUI's arithmetic, a division by zero is the error kind NumberBox turns into NaN) and for `Decimal` (exact, with overflow reported; `Decimal` gets `checked_mul` for it), so that `NumericUpDown` can evaluate expressions without a conversion to `f64` (W7). The peers of NumberBox raise the changes of `SmallChange` and `LargeChange` with the identifiers of W1.

Where #286 differed from WinUI, the port follows WinUI: the text of NumberBox's text box is input until it is committed (#286 copied every keystroke into `Text`), a change of `ValidationMode` validates the text without coercing the value, and the caption of RatingControl is always present (#286 hid it while empty). #286's Rust templates, its resource lookups (the brushes and sizes of RatingControl) and its colour literals are gone: the templates and the resources are the control themes of the WinUI theme.

| WinUI | Port | Kind | Why |
|---|---|---|---|
| NumberBox's `NumberFormatter`, an `INumberFormatter2` that is also an `INumberParser`; by default a regional `DecimalFormatter` of the user (one integer digit, no fraction digits). | The trait `INumberBoxNumberFormatter` (`format_double`, `parse_double`); by default `InvariantNumberBoxNumberFormatter` (the shortest text that reads back as the number, Rust's parsing). | Behaviour | FerroUI has no globalization formatters; an application supplies a regional formatter. |
| NumberBox's display rounder, an `IncrementNumberRounder` with ten significant digits and `RoundHalfUp`. | A function that rounds the decimal digits of the value to ten significant digits, a tie toward positive infinity. | Representation | No WinRT rounders; the rounding is the same. |
| NumberBox updates and commits the text only through its text box (`UpdateTextToValue`, `UpdateValueToText` do nothing without one). | Without a template the control formats the value into `Text` and commits `Text` as input. | Behaviour | The Fluent and Simple themes have no template for the control (section 5.1); the value and the text stay consistent without one. |
| NumberBox listens to `PreviewKeyDown` and `KeyUp` of its text box; GamepadA and GamepadB. | A handler of the tunnel route of the key down event of the box while its text box has the focus, and `OnKeyUp` of the box; the gamepad buttons are the Enter and Escape of FerroUI's key events. | Representation | FerroUI's preview of an event is its tunnel route. |
| The value NumberBox sets in code (`Value(...)`, a local value). | A current value (`set_current_value`), which keeps a binding of `Value`. | Behaviour | The FerroUI convention for values a control sets. |
| The localized strings of NumberBox (`NumberBoxUpSpinButtonName`, `NumberBoxMinimumValueStatus`, ...) and of RatingControl's peer (`BasicRatingString`, `RatingUnset`, ...). | The English strings of WinUI's resources, written in the code. | Missing | FerroUI has no localized resources of the controls. |
| NumberBoxAutomationPeer raises the change of `Value` only. | The peer also raises the changes of `Minimum`, `Maximum`, `SmallChange` and `LargeChange` (the identifiers of W1). | Behaviour (addition) | UI Automation clients see the range change, as with the range peers of the framework. |
| `NumberBoxParser`: no bounds; `null` for every error. | `evaluate_number_expression` rejects an input longer than 64 KiB or with more than 4096 tokens and reports the kind and the offset of an error; it is generic over `f64` and `Decimal`. | Behaviour (addition) | Bounded work for input from the user; one evaluator for NumberBox and NumericUpDown. |
| NumberBox's `InputScope`, `SelectionFlyout`, `SelectionHighlightColor`, `TextReadingOrder`, `PreventKeyboardDisplayOnProgrammaticFocus`. | Not ported. | Missing | No counterparts in FerroUI's text box. |
| The events of NumberBox and AutoSuggestBox are `TypedEventHandler<sender, args>`. | Handlers of the arguments (`value_changed`, `text_changed`, `suggestion_chosen`, `query_submitted`), disposable; the markup events pass the sender. | Representation | The convention of the plain events of FerroUI's controls. |
| AutoSuggestBox derives from ItemsControl with a `Header` of its own. | It derives from `HeaderedItemsControl` (`Header` and `HeaderTemplate`). | Representation | The upstream base with a header; `HeaderTemplate` is an addition. |
| AutoSuggestBox finds `QueryButton` in the template of its text box. | `QueryButton` is a part of the AutoSuggestBox template, the inner right content of its text box (theme ASB-2). | Representation | The text box theme is shared with AutoCompleteBox, whose template has no query button. |
| AutoSuggestBox's keys (`OnKeyDown` of the text box) and the click of a suggestion (`ItemClick` of the ListView). | #286's handling on the tunnel route: Up and Down move through the suggestions and back to the typed text, Escape and Tab restore the typed text and close the list, Enter submits the selected suggestion; the release of the left button over a suggestion submits it. | Behaviour | A reimplementation, not a line-by-line port of the 3,000 lines of `AutoSuggestBox_Partial.cpp`. |
| AutoSuggestBox: `AutoMaximizeSuggestionArea`, `LightDismissOverlayMode`, `TextBoxStyle`, `HeaderPlacement`, input validation, the list above the box when there is no room below, a negative `MaxSuggestionListHeight` as the available height, the inking button. | Not ported; a negative height is infinite. | Missing | Candidates for later. |
| RadioButtons is a Control with an `ItemsRepeater` (`InnerRepeater`) and its own `SelectedIndex`, `SelectedItem` and `SelectionChanged`. | An items control (`HeaderedSelectingItemsControl`) whose selection is the selecting items control's; the header part is `PART_HeaderPresenter`. | Representation | FerroUI's RadioButtons reuses the upstream selection and containers (`RadioButton`). |
| `ColumnMajorUniformToLargestGridLayout` (a layout of the repeater, in Primitives). | `primitives::RadioButtonsPanel`, a public panel with `ColumnSpacing` and `RowSpacing`, the items panel the WinUI theme gives the group with `RadioButtonsColumnSpacing` and `RadioButtonsRowSpacing`; it reads `MaxColumns` from the group. | Representation | The theme takes the spacing from the resources (#286 wrote 7 and 8 in a private panel); a theme can only name a public type. |
| `RadioButtons::Select` checks the container of the new selection and unchecks the old one; the framework unchecks the other radio buttons of the parent. | The control checks the container of the new selection and relies on the radio button group manager to uncheck the others (the containers are logical children of the group without a group name); it unchecks the old container itself only when no container is checked instead (a cleared selection, or a container that is not realized), which the group manager never does. A container realized after its selection is checked when it is prepared. | Behaviour | One owner of the exclusive check; #286 rewrote the checked state of every container on each change, besides the group manager. |
| `MaxColumns` less than 1 throws `hresult_invalid_argument`. | Coerced to 1, with a warning in the log. | Behaviour | No panic from markup or input. |
| RadioButtons' keys and focus (`OnChildPreviewKeyDown`, `MoveFocus`, `OnGettingFocus`). | #286's handling: the arrow keys move the focus and the selection to the next enabled button without wrapping, Left and Right to the nearest button of the next column, Ctrl with an arrow key moves the focus only, Tab enters at the selected button. | Behaviour | A reimplementation, not a line-by-line port. |
| RadioButtonsAutomationPeer is a plain group. | The peer of a selecting items control: a group with the selection pattern. | Behaviour (addition) | The upstream peer of the base class. |
| RadioButtons' element factories and `DataTemplateSelector` as `ItemTemplate`. | Not ported. | Missing | FerroUI's data templates select themselves. |
| RatingControl creates its items from the DataTemplates `BackgroundGlyphDefaultTemplate`, ... that it looks up in the resources of the application, and reads `RatingControlFontSizeForRendering` and `RatingControlItemSpacing` in code. | The control creates text blocks or images itself, as large as the font size the template gives its stack panels, which also give the spacing and the unselected foreground (theme RTG-1). | Representation | No template is built and no resource is read by the controls (section 5.1). |
| RatingControl's layout of the items (scaled to half their size with -8 margins, a TranslateTransform to centre them, the caption margin computed in code). | Each item is drawn at half its size in a cell of half its width; no transform; the caption margin is in the template (theme RTG-2). | Representation | #286's layout, without its literals. |
| The default `ItemInfo` of RatingControl's style, the shared resource `MUX_RatingControlDefaultFontInfo`. | Each control creates its default `RatingItemFontInfo` with the same glyphs. | Behaviour | A change of the glyphs of one control changes no other. |
| RatingControl's composition animations of the scale of the items (`ApplyScaleExpressionAnimation`). | The control sets the scale of the items as the pointer moves. | Missing | No composition animations in FerroUI. |
| RatingControl's gamepad focus engagement (`IsFocusEngagementEnabled`, GamepadA, GamepadB, `FocusEngaged`). | Not ported (#286 had an engagement of its own, which is not taken). | Missing | FerroUI has no focus engagement. |
| RatingControl handles the pointer on `RatingBackgroundStackPanel`. | The control handles the pointer events and reads the position over the background stack panel. | Representation | #286's handling. |
| RatingControl's coercion keeps a NaN `Value` or `PlaceholderValue`. | NaN is coerced to -1 (not set), with a warning in the log. | Behaviour | #286 rejected NaN with a panic; NaN draws no state. |
| RatingControlAutomationPeer raises the change of `ValuePatternIdentifiers.ValueProperty` with the numbers of the rating. | The change of the value pattern carries the text of the rating ("Rating, 3 of 5"). | Behaviour | The value of the value pattern is that text. |

### 10.4 Navigation (W8)

`src/FerroUI.Controls/frame.rs`, `frame_page.rs`, `navigation.rs`, `navigation/page_stack_entry.rs` (the module `ferroui_controls::navigation`, markup namespace `FerroUI.Navigation`, not in the default XML namespace), `navigation_view.rs`, `navigation_view_events.rs`, `navigation_view_item.rs`, `primitives/navigation_view_template_settings.rs` and the peers `automation/peers/navigation_view_automation_peer.rs`, `navigation_view_item_automation_peer.rs`; themes `Controls/Frame.xaml` and `Controls/NavigationView.xaml` of the WinUI theme (the NavigationView resources were already converted for DrawerPage and TabbedPage, which keep their themes; `PaneToggleButtonStyle`, the text styles of `Controls/DrawerPage.xaml` and the back button styles of `Controls/NavigationPage.xaml` are shared). Tests: `frame_tests` (with the setters of markup), `navigation_view_tests`, `navigation_view_item_tests` (`ferroui-controls`, with the parts of the templates in `navigation_test_support`); `frame`, `navigation_view`, `navigation_view_item`, `navigation_view_item_header`, `navigation_view_item_separator` and `the_navigation_controls_get_their_parts_from_the_theme` of `control_templates`, and the document tests (`ferroui-themes-winui`); the view builder tests of FramePage and the navigation types (`scripts/test_generate_view_builders.py`). The differences of the templates are FRM, NVW, NVI, NVH and NVS of the theme's DEVIATIONS.

`Frame` is synchronous and type-driven (section 7): `navigate`, `go_back` and `go_forward` raise `Navigating` (which can cancel), call `on_navigating_from` of the old page, take the page from the cache or create it, set the content, commit the journal, raise `Navigated` and call `on_navigated_from` and `on_navigated_to`; a navigation requested during a navigation is ignored. The journal is `navigation::NavigationStack`, the live back and forward stacks, also an items source; an edit during a navigation, out of range or with an entry of another frame is an error. The cache keeps pages of the mode `Enabled` up to `CacheSize`, the least recently shown first out, and pages of the mode `Required` whatever the size once admitted. `FramePage` is WinUI's `Page`; its name keeps `Page` for the upstream page.

`NavigationView` keeps #286's behaviour code (O3) without its Rust templates: #286 built a SplitView template, the rows, presenters, chevron buttons and popups of the items in code and set brushes and sizes on them; the template is now WinUI's `NavigationView.xaml`, and the code does what WinUI's code does with the parts: it sets the display mode and the open pane of `RootSplitView`, swaps the pane header, custom content, footer and auto suggest box between the content controls of the left and the top pane, moves the pane title between the toggle button, `PaneTitlePresenter` and `PaneTitleOnTopPane`, sets the margins the back button needs, the values of `TemplateSettings` and WinUI's visual states as pseudo-classes. `NavigationViewItem` sets the states of WinUI's item presenter as its own pseudo-classes, indents its `ContentGrid` by 31 per level (`c_itemIndentation`; #286 used 16), shows its selection indicator for itself or for a collapsed ancestor of the selected child, and moves its children host into `ChildrenFlyout` in the top pane and in a closed compact pane, as WinUI moves its repeater. Header and separator set their states and indentation the same way.

| WinUI | Port | Kind | Why |
|---|---|---|---|
| `Frame.CacheSize` is read as an unsigned number (`OnPropertyChanged2`): a negative size is an unbounded cache. | A negative size from markup, a binding or a style is coerced to 0 and logged (area `Control`); `set_cache_size` returns `NavigationError::InvalidCacheSize` and changes nothing. | Behaviour | No panic and no unbounded cache from markup; #286 panicked in the setter. |
| Setting `Frame.SourcePageType` navigates; a null or invalid type fails with the error of `Navigate`. | The setter of the property (markup) logs the error and leaves the frame as it is; `set_source_page_type` returns it. | Behaviour | #286 panicked in the setter; a value from markup must not end the application. |
| `Navigate`, `GoBack`, `GoForward` return `bool` or fail with an `HRESULT`; the page is activated from its type name. | `Result<bool, NavigationError>` and `Result<(), NavigationError>`; the page comes from a `PageFactory` registered for its type (`register_page_factory`) or from the default constructor of the type; a factory of another type is `FactoryTypeMismatch`. | Representation | Rust errors; FerroUI types have no activation by name, and a factory lets an application inject its pages. |
| A navigation requested from a handler of a navigation. | Ignored: `navigate` returns `Ok(false)`, `go_back` and `go_forward` `Ok(())`. | Behaviour | #286's guard of the synchronous navigation. |
| `Navigate(Type, Object, NavigationTransitionInfo)`, `FrameNavigationOptions.TransitionInfoOverride`, `PageStackEntry.NavigationTransitionInfo`, `ContentTransitions`, `RecommendedNavigationTransitionInfo` of the arguments of NavigationView. | Not ported. | Missing | FerroUI's frame has no navigation transitions. |
| `Frame.GetNavigationState`, `SetNavigationState`; `Page.TopAppBar`, `BottomAppBar`. | Not ported. | Missing | No serialization of the journal; no app bars of pages. |
| The events of `Frame` and `NavigationView` are `EventHandler` and `TypedEventHandler` events. | Handler lists (`navigating`, `navigated`, `selection_changed`, ...) that return a disposable; markup subscribes with the event names and gets the sender. Clones of the arguments share `Cancel` and `Handled`. | Representation | The plain events of FerroUI's controls. |
| `Frame.BackStack` and `ForwardStack` are `IVector<PageStackEntry>`. | `navigation::NavigationStack`, a live list whose edits return a `Result` (`NavigationPending`, `IndexOutOfRange`, `EntryOwnedByAnotherFrame`); it is also an items source of bindings. | Representation | Rust errors where WinUI fails a collection call. |
| `Page.Frame` and the entries of the journal reference their frame. | Held weakly: a page or a journal kept by the application keeps no frame alive. | Representation | Reference counting without cycles. |
| NavigationView realizes its items with `ItemsRepeater`s and selects with a hierarchical `SelectionModel` (`IndexPath`); `NavigationViewItemsFactory`. | Stack panels whose children the control realizes from `MenuItems`, `MenuItemsSource`, `FooterMenuItems` and the child items; the containers of data items are reused across edits; the selection is `SelectedItem` with `IsSelected` and `IsChildSelected` of the containers. | Missing | Follow-up F1, on the repeater crate (#269). |
| `NavigationViewItemPresenter` (primitives) and `NavigationViewItemPresenterTemplateSettings` (`IconWidth`, `SmallerIconWidth`). | Not ported: the template of the item holds the presenter's template, the item sets the presenter's states; the icon column is bound to `SmallerPaneToggleButtonWidth` of the navigation view, the same value (NVI-1, NVI-4). | Missing | The presenter is a template part of WinUI's item with no use of its own; a public presenter type is a candidate for F1. |
| The overflow of the top pane: `TopNavOverflowButton` and its flyout, `TopNavigationViewDataProvider`, `OverflowLabelMode`, the measure that moves items to the overflow. | Not ported: every item stays in `TopNavMenuItemsHost` (NVW-4). | Missing | A candidate for later. |
| The animations: the selection indicator sliding between items (`AnimateSelectionChanged`), the pane and its shadow, the header text, the `AnimatedIcon`s of the chevron, the settings item and the back button. | Not ported: the indicator shows on the new item at once, the chevron glyph turns by a rotation (NVI-3, NVI-6). | Missing | No composition animations or Lottie in FerroUI. |
| The shadow of the overlaying pane and of the children flyout (`ThemeShadow`, `ShadowCaster`). | Not drawn (NVW-5). | Missing | Follow-up F3 (#326). |
| The acrylic background of the pane (`NavigationViewDefaultPaneBackground`, `NavigationViewTopPaneBackground`). | The fallback colours of the converted resources. | Missing | Follow-up F2 (#325). |
| The title bar padding (`IsTitleBarAutoPaddingEnabled`, `TemplateSettings.TopPadding` from the title bar of an extended window), `ShoulderNavigationEnabled`, the gamepad and the system back key, `MenuItemTemplateSelector`, `MenuItemContainerStyleSelector`. | Not ported; `TopPadding` stays 0. | Missing | No extended title bar, gamepad shoulder navigation or template selectors of this kind in FerroUI. |
| `TemplateSettings` visibilities are `Visibility`. | `bool`s with WinUI's names, bound to `IsVisible`. | Representation | FerroUI's visibility is `IsVisible`. |
| `RootSplitView.IsPaneOpen` is bound two way to `IsPaneOpen`; `PaneOpening`, `PaneOpened`, `PaneClosed` and `PaneClosing` are forwarded from the split view. | The control sets the current value of the split view's `IsPaneOpen` and follows its changes (a light dismissal); it raises `PaneOpening` and `PaneOpened` when `IsPaneOpen` becomes true, `PaneClosing` (unless the split view's closing raised it) and `PaneClosed` when it becomes false (NVW-12). | Behaviour | #286's model: one source of the pane state and of its events, also without a split view. |
| `NavigationViewItem` is invoked by the pointer through its presenter (`OnPresenterPointerPressed`, `OnPresenterPointerReleased`, `OnNavigationViewItemClicked`) and by the keys of `NavigationView::OnNavigationViewItemKeyDown`. | The item handles the pointer itself (pressed and released over it with the primary button; a release over `ExpandCollapseChevron` that was pressed there toggles the expansion), Enter and Space invoke it, the arrows expand, collapse and move as #286 did. | Behaviour | No presenter control; #286's keyboard handling. |
| The settings item has an `AnimatedIcon` (Settings, falling back to the Setting symbol), the localized name and, in the top pane, `MUX_NavigationViewSettingsItemStyleWhenOnTopPane`. | The `SymbolIcon` of `Symbol::Setting`, the name "Settings", and in the top pane no content (the icon-only state of the item, NVI-9). | Representation | No animated icons; one template of the items. |
| The localized texts (`SR_SettingsButtonName`, `SR_NavigationButtonOpenName`, `SR_NavigationButtonClosedName`, the back and close tool tips). | The English texts "Settings", "Close Navigation", "Open Navigation", "Back"; an item without content and name is named "Navigation item" (#286). | Missing | FerroUI has no localized resources of the controls. |
| `IsBackButtonVisible` `Auto` hides the back button on Xbox. | `Auto` shows it. | Behaviour | No Xbox form factor. |
| A NavigationViewItem that contains itself or an ancestor (a cycle of items). | The child is left out and a warning is logged. | Behaviour (addition) | #286 asserted; a cycle from markup or data must not end the application. |
| `NavigationViewItemAutomationPeer` reports `PositionInSet`, `SizeOfSet` and `Level`; `NavigationViewAutomationPeer::RaiseSelectionChangedEvent`. | Not reported; the change of the selection is raised as a change of the `Selection` property of the selection pattern. | Missing | No position-in-set members of FerroUI's peers; the selection pattern carries the change. |
| `PaneToggleButtonStyle` and `MenuItemContainerStyle` are `Style`s. | `Option<Ref<ControlTheme>>`; the container theme is applied to the containers at the style priority. | Representation | FerroUI's styles of controls are control themes. |
