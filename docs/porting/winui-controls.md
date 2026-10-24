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

- Each templated control of this page gets its default template from a control theme of the WinUI theme, converted from the WinUI document at the pin by `scripts/winui-theme/convert.py` into `src/FerroUI.Themes.WinUI/Controls/`, with its resources from the `<Control>_themeresources.xaml` documents. The converted documents are generated files: they are regenerated with the script, never edited by hand, and the differences the conversion makes are recorded in `docs/porting/winui-theme/DEVIATIONS.md`.
- No template is built in Rust, and no template child is styled from code with resource keys or colour literals. The theme tests require a theme for every templated control registered by the theme's controls; there is no allow-list.
- **Fluent and Simple have no templates for these controls for now.** In an application with the Fluent or the Simple theme a templated control of this page has no template, and so shows nothing: use the WinUI theme (`WinUITheme`) for applications that use them. Fluent and Simple documents may follow later; this section is updated when they do.
- The tables of `winui-theme.md` that say "no FerroUI control" for these controls (NavigationView and its items, RadioButtons, ProgressRing, RatingControl, InfoBadge; DRP-1, DRP-2, DRP-11, DRP-13, NVP-1 and G-23 of the theme's DEVIATIONS) are updated in the pull request that lands each control.

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

Use the upstream pages for an application written against upstream's model, and `Frame` with `NavigationView` for one written against WinUI's (and for WinUI source brought in through the importer). A `Frame` accepts only `FramePage`; a bridge that would let it host an upstream `Page` is not planned.

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

- Differences from WinUI of each control are listed in the section of the control below as it lands (none yet).
- Changes to upstream-ported controls and to core crates are in `DEVIATIONS.md`: "WinUI additions to upstream controls", "Automation peers" and "Windows platform backend" (W1), and the markup section (W2).
- Differences of the converted templates are in `docs/porting/winui-theme/DEVIATIONS.md`.
