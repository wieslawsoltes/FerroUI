# Native control contracts

The controls in this document use `FerroUI.Controls` in markup and
`ferroui_controls` in Rust, including the default FerroUI presentation namespace.
They compose the framework's existing property, layout, text, selection, input,
automation and template systems. Frame destination pages use `FramePage`, while
Frame navigation contracts belong to `FerroUI.Navigation`. Template settings and
InfoBarPanel use `FerroUI.Controls.Primitives`; peers use `FerroUI.Automation.Peers`.
Existing control identities and defaults remain intact. A declaration or successful
construction does not establish that every WinUI contract is complete.

The reference commit is
[`a71c42bd3e352060e5f5ecf3b5e1a98f8a745809`](https://github.com/microsoft/microsoft-ui-xaml/tree/a71c42bd3e352060e5f5ecf3b5e1a98f8a745809).
The implementation is based on its API model, native implementation and the
actual Gallery usages recorded in `data/gallery-usage.json`.

## RadioButtons

`RadioButtons` derives from the native `HeaderedSelectingItemsControl` and uses
existing `RadioButton` containers. It supports inline controls, plain values and
`ItemsSource`; plain values are wrapped in a radio button while supplied radio
buttons remain their own containers. Pointer activation, keyboard activation,
mutual exclusion, effective enabled state and radio accessibility use native
services. Selection synchronizes `SelectedIndex`, `SelectedItem` and the actual
checked state of each realized container.

| WinUI member | FerroUI member | Implemented behavior and boundary |
|---|---|---|
| `Items` | inherited `items()` | Native observable item collection and inline XAML content |
| `ItemsSource` | inherited `items_source()` | Native typed item-source adapter; native collection rules apply |
| `ItemTemplate` | inherited `item_template()` | Native `IDataTemplate` displayed in radio content |
| `SelectedIndex`, `SelectedItem` | inherited properties | Native single selection, selected-item identity and collection updates |
| `SelectionChanged` | inherited routed event | Actual removed/added items; duplicate assignments do not emit changes |
| `ContainerFromIndex` | inherited method | Actual generated or supplied radio container |
| `Header`, `HeaderTemplate` | inherited properties | Live native content presenter and data template |
| `MaxColumns` | `max_columns()` | Validated positive integer; column-major layout constrained by available width |
| `RadioButtonsAutomationPeer` | native selection peer | Group role, class name, selected peers, position/size of each radio set |

The default panel measures the largest item, fits the requested columns into
the available width, and fills columns from top to bottom. Earlier columns get
one extra item when the item count does not divide evenly. Pinned spacing is
7 units between columns and 8 between rows; the header's bottom margin is 8.
The template and item panel remain replaceable native templates.

Down/Up moves through items in collection order, skips unavailable containers
and stops at the ends. Left/Right moves to the adjacent visual column. Keyboard
selection follows focus unless Control is held, and entry by keyboard restores
the selected container. The group consumes directional keys at its boundaries
instead of allowing the underlying radio group to wrap. Native button behavior
handles activation.

### RadioButtons boundaries

- Native `ItemsSource` requires an empty inline `Items` collection before source
  mode is enabled. WinUI keeps its separate inline collection while a source is
  active. The source object and collection API are native Rust adapters.
- Native selection normalizes invalid indices and exposes additional inherited
  selection APIs. Those additional APIs are not WinUI contracts and do not
  establish WinUI multi-selection support.
- `ItemTemplate` accepts `IDataTemplate`, not a WinUI `IElementFactory` or
  `DataTemplateSelector`. A template produces the content of the generated radio
  button. A template that itself creates a radio button requires review.
- WinUI gamepad XY-focus escape, AccessKeyInvoked-specific focus redirection,
  exact Windows UIA event sequences and WinUI theme resources are not certified.
- The supplied native theme controls the radio appearance. The native default
  panel is exercised; custom panels may need their own navigation policy.

The integration target `winui_radio_buttons` exercises container reuse, actual
activation and checked state, selection-event ordering, source replacement,
headers/templates, dynamic collection edits, constrained column layout,
keyboard navigation and automation.

## FontIcon

`FontIcon` derives from the existing `IconElement`. Its default template is a
native `TextBlock` using the framework's text shaping, layout and drawing
services. `Glyph` is a string and supports more than one glyph. The native
typography and foreground properties update the existing presenter, including
changes after the first layout pass. Glyph and font changes invalidate the
icon's measurement as well as the text presenter, including when the icon is
measured as a detached root without a window layout manager.

| WinUI member | FerroUI member | Implemented behavior and boundary |
|---|---|---|
| `Glyph` | `glyph()` / `set_glyph()` | Live string rendered by native text services |
| `FontFamily` | inherited typography property | Default `Segoe Fluent Icons`; applications can supply another native family |
| `FontSize` | inherited typography property | Default 20 units; native text metrics determine measured size |
| `FontStyle`, `FontWeight` | inherited typography properties | Native text formatting and font matching |
| `Foreground` | inherited property | Live inherited/local native brush |
| `MirroredWhenRightToLeft` | same property | Opt-in glyph reflection around its center; public control transform is preserved |
| `IsTextScaleFactorEnabled` | **unsupported** | No OS text-scale service is currently connected; there is no stored no-op property |

Typography inheritance follows the existing native property system. WinUI's
specific inherited-text formatting rules and Windows text-scale integration are
separate contracts; native layout tests do not certify those rules.

The text presenter is excluded from the accessibility content view, so its
private-use font characters are not announced as document text.

## SymbolIcon and Symbol

`SymbolIcon` draws one predefined symbol through the same native text services.
Its default is `Symbol::Emoji`. `SymbolIcon::with_symbol(...)` implements the
constructor taking a symbol. Changing `Symbol` or `Foreground` updates its
existing presenter. Pinned symbol rendering uses Segoe Fluent Icons at 20 units,
normal style and normal weight.

**The Symbol API value is often different from the glyph codepoint.** For
example, `Symbol::Play` retains the public numeric value `0xE102`, while the
native implementation renders `U+E768`. Mapping enum numbers directly to
characters would draw the wrong font entries. All **197** public values and
their modern glyph conversions are included, together with all 197 XAML enum
names. The source table is `data/symbol-glyphs.tsv`.

`scripts/winui-gallery/symbol_map.py --check` verifies the complete enum,
rendering map, markup names and iteration table against a pinned reference.
`data/symbol-sources.lock.json` records SHA-256 values for both the upstream API
model and native `icon.cpp`, as well as the reference-table hash. Refreshing the
reference requires an explicit clean checkout at the main source lock's commit:

```sh
python3 scripts/winui-gallery/symbol_map.py --refresh-source \
    --winui-root ../upstream-winui
```

The `winui_icons` integration target checks every numeric value and glyph, then
passes every symbol through native text layout and records the resulting glyph
draw calls. It also checks live typography/foreground changes, RTL mirroring,
constructor defaults and preservation of existing text defaults. Its fonts are
deterministic test fonts. This proves the native data/rendering path and glyph
selection; it does **not** prove installed-font availability or visual equality
to Windows rendering.

No Microsoft font binary is redistributed. Hosts must provide an appropriately
licensed font with the expected glyphs. `FontIcon` accepts an application-supplied
font family. `SymbolIcon` deliberately uses the pinned family's mapping; a font
with a different private-use encoding cannot be substituted just by using the
same numeric API values.

## Reusable icon sources

`IconSource` is an abstract native object with a `Foreground` property and
`create_icon_element()`. `FontIconSource`, `SymbolIconSource` and `PathIconSource`
create the corresponding real native icons. Reusing one source creates separate
visual elements, so two controls never attempt to share the same visual parent.

| Source | Native element | Live source properties |
|---|---|---|
| `FontIconSource` | native `FontIcon` | Glyph, FontFamily, FontSize, FontStyle, FontWeight, MirroredWhenRightToLeft, Foreground |
| `SymbolIconSource` | native `SymbolIcon` | Symbol, Foreground |
| `PathIconSource` | existing `PathIcon` | Data, Foreground |

The source keeps weak references to the elements it creates. It neither owns
those elements nor subscribes to their changes. Removing an element releases
it; dropping a source leaves surviving elements at their last copied values.
Changing a source property updates only that mapped property, preserving local
values on unrelated element properties. Expired weak references are pruned.
If an element's change handler changes the source again during propagation,
later elements receive the current effective source value. A regression test
covers both same-property redirection and a nested change to a different property;
an older notification cannot overwrite the newer source value on another icon.

An initially unset source foreground leaves a new element's normal inherited
foreground intact. An explicit source foreground propagates to every surviving
element. Changing it back to null writes an explicit null to existing elements;
subsequently created elements again receive no foreground override. This follows
the pinned source's distinction between initial copying and later changes.

Font sources retain the FontIcon limitations above. Bitmap and animated icon
sources, `IsTextScaleFactorEnabled` and Windows font services are separate gaps.
Path sources preserve native geometry identity and rely on the existing native
PathIcon theme and drawing implementation.

## InfoBar

`InfoBar` uses native content, template, property, input, command and resource
services. It derives from `ContentControl` to preserve native content ownership
and data templates. The existing notification controls and theme-specific
InfoBar panel keep their original identities and behavior.

| WinUI API | Native implementation and boundary |
|---|---|
| `IsOpen` | Default false; collapses the template root while preserving the outer control's `IsVisible` |
| `Closing`, `Closed`, `CloseButtonClick` | Real ordered callbacks, typed reasons and cancellable closing |
| `Opened` | Implemented as the pinned **preview** contract, including reopening after cancellation |
| `Title`, `Message`, `Severity` | Live text, severity glyphs, native resources and severity pseudo classes |
| `IsClosable` | Controls close-button visibility; programmatic closing remains allowed |
| `CloseButtonCommand`, parameter | Native button executes the command and observes `CanExecuteChanged` |
| `CloseButtonStyle` | **Explicit adaptation:** a native `ControlTheme`; arbitrary WinUI Style setters are not accepted as equivalent |
| `ActionButton` | Actual native button presented with its existing command/input behavior |
| `Content`, `ContentTemplate` | Native content presenter, ownership and data templates |
| `IconSource`, `IsIconVisible` | Reusable source creates a live icon; source replacement and visibility update native parts |
| `TemplateSettings.IconElement` | Actual current native icon, exposed through a stable template-settings object |
| `InfoBarPanel` | Pinned width/height-sensitive layout with styled padding and attached margins |

On close-button activation, `CloseButtonClick` runs first. The control records
`CloseButton` as the reason and requests `IsOpen = false`. `Closing` can cancel
the operation; cancellation restores `IsOpen = true`, preserves the visible
content and raises the preview `Opened` event. Otherwise the root collapses and
`Closed` runs. Reopening resets the reason to `Programmatic`. Event handlers use
snapshots without held internal borrows, so cancellation, event disposal and
reopening from a handler are supported. Reapplying a template detaches the
previous close button's callback.

The default template contains real native borders, a grid, TextBlocks,
ContentPresenters, a Viewbox, a close button and the responsive InfoBarPanel.
The standard icon uses the pinned `F136` background plus `F13F` informational,
`F13E` success, `F13C` warning or `F13D` error glyph. The close glyph is the exact
`Symbol::Cancel` conversion. The same icon-font availability limitation applies
as for SymbolIcon.

The existing WinUI theme already contains the InfoBar palette and
`InfoBarCloseButtonStyle` as native resources. The control observes native
resource/theme changes and reads the severity brushes, text brushes, icon font,
font sizes, panel padding/margins and close-button theme. Light/dark fallback
severity colors come from the pinned palette. A user background is painted
over the severity background, preserving translucent composition. Explicit
foreground values take precedence over the title/message resource brushes.
Missing localized close-button/icon labels use English fallbacks; hosts can
provide native string resources.

`InfoBarAutomationPeer` reports a native StatusBar role, class name, and a
title/message fallback name, while respecting an explicitly supplied automation
name. Name changes raise native automation property notifications. Severity
sets native polite/assertive live-region metadata. **The Windows
`RaiseNotificationEvent` announcement channel is not implemented.** These
properties do not claim platform-specific announcement or Windows UIA sequence
parity.

Styling remains a reviewed native adaptation. WinUI VisualStateManager execution,
arbitrary WinUI Style setters, Windows localization services, pixel equivalence,
and every inherited WinUI member are outside this implementation's verified
scope. The strict XAML migration layer retains diagnostics for unsupported
syntax. Native templates remain replaceable; replacement templates must supply
the documented parts or bind their own native properties.

## NavigationView

The implementation follows the pinned
[NavigationView declarations](https://github.com/microsoft/microsoft-ui-xaml/blob/a71c42bd3e352060e5f5ecf3b5e1a98f8a745809/controls/dev/NavigationView/NavigationView.idl),
[control behavior](https://github.com/microsoft/microsoft-ui-xaml/blob/a71c42bd3e352060e5f5ecf3b5e1a98f8a745809/controls/dev/NavigationView/NavigationView.cpp),
[item behavior](https://github.com/microsoft/microsoft-ui-xaml/blob/a71c42bd3e352060e5f5ecf3b5e1a98f8a745809/controls/dev/NavigationView/NavigationViewItem.cpp)
and [automation declarations](https://github.com/microsoft/microsoft-ui-xaml/blob/a71c42bd3e352060e5f5ecf3b5e1a98f8a745809/controls/dev/NavigationView/NavigationViewItemAutomationPeer.idl).
Native code is split between
[navigation_view.rs](../../../src/FerroUI.Controls/navigation_view.rs),
[navigation_view_item.rs](../../../src/FerroUI.Controls/navigation_view_item.rs)
and [navigation_view_events.rs](../../../src/FerroUI.Controls/navigation_view_events.rs).
The [parent peer](../../../src/FerroUI.Controls/automation/peers/navigation_view_automation_peer.rs)
and [item peer](../../../src/FerroUI.Controls/automation/peers/navigation_view_item_automation_peer.rs)
use separate modules and the runtime namespace
`FerroUI.Automation.Peers`.
These are ordinary native controls; the existing Avalonia-compatible `SplitView`
and selection APIs retain their defaults and public contracts.

### Collections, containers and selection

| WinUI member or behavior | Native implementation and boundary |
|---|---|
| `MenuItems`, `FooterMenuItems` | Instance-owned native `ItemCollection`s; read-only property handles, writable collection contents |
| `MenuItemsSource`, `FooterMenuItemsSource` | Live native `ItemsSource` adapters; inline collections stay writable while a source is active and return when it is cleared; arbitrary WinRT collection objects need adapters |
| `MenuItemTemplate` | Native `IDataTemplate`; an item/header/separator returned by the template becomes the container, while another control becomes generated item content |
| `SelectedItem` | Original model `BoxedValue`, shared across menu, footer, descendants and the owner-specific Settings item |
| `ContainerFromMenuItem`, `MenuItemFromContainer` | Round trips through the current realized hierarchy; unrelated source edits retain existing containers; removed models and listeners are detached |
| `SelectionChanged` | Original model, realized container and `IsSettingsSelected`; changes synchronize selected rows and ancestor `IsChildSelected` state |
| `ItemInvoked` | Container content, invoked container and Settings flag; repeated invocation need not change selection |
| `SelectsOnInvoked` | Defaults true; a false value permits action/expansion invocation while preserving the prior selection |
| `SettingsItem` | Stable per-owner native `NavigationViewItem`; visibility is controlled separately by `IsSettingsVisible` |
| `Expand`, `Collapse` | Change the expansion state of items with realized or unrealized children and raise the corresponding native event |

Invocation precedes the selected-row flag updates and `SelectionChanged`.
Application code can redirect selection during `ItemInvoked`; the final effective
selection wins without a stale outer notification. Assigning a nonselecting item
restores the prior selection and invokes the action, matching the pinned control
policy. Headers and separators use the common item base but have no interactive
row invocation. Native source models retain normal `BoxedValue` comparison and
ownership semantics; the projection does not stringify model identity.

An assignment whose model has no realized container preserves `SelectedItem`
and schedules one native dispatcher completion. `SelectionChanged` describes
the still-current model and permits a null `SelectedItemContainer`. If the
model is realized first, that realization emits the selected container and
consumes the pending notification. A newer selection or clear suppresses the
older callback; assigning a nonselecting action preserves the prior unresolved
selection and its pending notification. Queued work holds the owner weakly.

Generated wrappers for ordinary native `Control` values preserve the live
inherited application `DataContext`; they do not replace it with the control
itself. This applies to nested inline items and active item sources. When a
`MenuItemTemplate` factory changes the template while building containers, the
new factory forces a rebuild while model identity, container round trips and
selection remain intact.

### Adaptive layout and pane lifetime

| Member | Initial value or supported behavior |
|---|---|
| `PaneDisplayMode` | `Auto`; supports `Auto`, `Left`, `Top`, `LeftCompact`, `LeftMinimal` |
| `DisplayMode` | Read-only; initially `Minimal`, then derives from the actual native width and mode |
| `CompactModeThresholdWidth`, `ExpandedModeThresholdWidth` | 641 and 1008; the exact threshold boundaries determine Minimal, Compact and Expanded |
| `CompactPaneLength`, `OpenPaneLength` | 48 and 320; negative lengths coerce to zero; nonfinite rendering geometry is bounded independently of the public value |
| `IsPaneOpen` | Initially true; first adaptive layout can close it, while an explicit closure is retained across expanded resizing |
| `IsPaneVisible`, `IsPaneToggleButtonVisible`, `IsSettingsVisible`, `AlwaysShowHeader` | Initially true; native presenters/buttons respond to effective state |
| `IsBackEnabled`, `IsBackButtonVisible` | Initially false and Auto; a request is an event, with application Frame navigation supplied separately |
| `SelectionFollowsFocus` | Initially Disabled; native focus/input behavior is separate from raw gamepad integration |
| `Header`, `HeaderTemplate`, `PaneTitle` | Native content/template/text; header visibility follows the selected mode and `AlwaysShowHeader` |
| `PaneHeader`, `PaneCustomContent`, `PaneFooter`, `ContentOverlay`, `AutoSuggestBox` | Actual native visual presenters, with ownership moved on replacement or retemplating |
| `PaneToggleButtonStyle`, `MenuItemContainerStyle` | Audited native `ControlTheme` values; arbitrary WinUI `Style` objects are not interchangeable |

The fallback template has a native `RootSplitView`, a `TopPanePresenter`, main
and footer item hosts, header/search/content presenters and actual toggle/back
buttons. Switching Top/Left moves the same pane controls between the hosts. Top
items use horizontal layout. Native SplitView geometry handles inline, compact
and overlay placement; top overflow partitioning is not implemented.

Pane events preserve a distinction in the pinned source: light dismissal asks
through `PaneClosing.Cancel`, while an explicit or adaptive `IsPaneOpen` change
is a forced transition. A canceled dismissal retains the open state. Reentrant
opening/closing handlers cannot produce a stale terminal event. Native subscriptions
are disposable, old template parts disconnect, and retained controls, providers
or sources do not imply ownership of a destroyed NavigationView. Clearing an
attached `Template` also disconnects retained toggle/back buttons and the old
SplitView. Reapplying the template restores the selected container and native
content, with one active back-request handler.
Old header/content presenters clear `ContentTemplate` before `Content` during
replacement or clearing. A custom data template that accepts null data is not
invoked during that teardown, so it cannot recreate a visual that the new
presenter must adopt. This applies to `HeaderTemplate` and the inherited
`ContentTemplate` contract used by the native NavigationView template.

### Hierarchical items and automation

`NavigationViewItemBase.IsSelected` defaults false. A `NavigationViewItem`
initially has no icon, badge or source, with `SelectsOnInvoked = true`,
`IsExpanded = false`, `HasUnrealizedChildren = false` and
`IsChildSelected = false`. Its `CompactPaneLength` is externally read-only and
starts at 48. Child `MenuItems` use the same independent collection policy as
the parent. `HasUnrealizedChildren` supplies a real expansion affordance and
allows an expanding handler to populate the live collection. An active empty
`MenuItemsSource` takes precedence over that flag for the parent's public
`Expand`/`Collapse` operations, matching the pinned source child check.

The replaceable item template contains real content, icon and badge presenters,
a row button, chevron, selection indicator and child host. Icon/badge replacement
detaches the prior visual. Expanding reveals native child controls; collapsed
inline children consume no layout space. Top and closed compact menus use a
native child Popup, with host interaction acceptance tracked separately.
Keyboard expansion respects right-to-left flow and ignores bubbled child events
at ancestors. Moving into children with the expansion-direction key skips
headings and nonfocusable, hidden or disabled rows; the key remains unhandled
when no child can take focus. An ancestor can show its selected-descendant
indicator while the descendant is hidden. Header and separator types are native nonfocusable controls
with replaceable templates.

Native directional handlers operate on popup items as well as the main tree.
Popup Up/Down traverses the visible expanded subtree; Home/End stays in that
popup list, skipping headings and hidden or disabled rows. Top root Left/Right
navigation follows flow direction, and Down enters an expanded child popup.
`SelectionFollowsFocus` determines whether that focus invokes/selects an item;
disabled following leaves selection unchanged. Direct item subscriptions retain
the owner weakly and detach after source removal or template replacement.

The automatic compact caption tooltip uses nonempty string content on the
native row button; an explicit item tooltip takes precedence. The pinned source
places its automatic tooltip on the item for keyboard-focus presentation, so
that exact location/keyboard behavior remains unverified. Native key events also
do not expose WinRT's `WasKeyDown` value; source-specific repeat-key suppression
is not claimed.

The NavigationView automation peer exposes native single-selection semantics.
The item peer supplies SelectionItem, adds ExpandCollapse for expandable items,
and has no Invoke pattern. Select/AddToSelection still invoke the native item;
RemoveFromSelection only changes selection. The selection container resolves
through inline and popup logical parents. Top root items report TabItem while
nested items report ListItem. Disabled input and nonselecting actions retain their
native restrictions. Owned expansion providers use the parent's child predicate
for every call: an active empty source prevents expansion/collapse even when
inline items or `HasUnrealizedChildren` still advertise an expansion pattern.
A retained provider rechecks child removal and replacement before changing
`IsExpanded`. These provider tests do not establish Windows UIA tree
level/group position, localization, screen-reader announcements or pixel parity.

### Gallery examples and exact source behavior

The opt-in WinUI markup migration accepts source `NavigationViewItem Icon="Play"`
and `AutoSuggestBox QueryIcon="Find"` forms by creating native `SymbolIcon`
property elements. `WXAML2005` records the original literal range and the
adaptation. The native Symbol parser still validates the literal. This applies
only to the recognized WinUI owner/member identities; native FerroUI markup,
resource/binding expressions and explicit application type/member policies keep
their existing contracts. No global IconElement string conversion is added.
The [migration implementation](../../../src/Markup/FerroUI.Markup.Xaml.Loader/winui/migration.rs)
retains source edits and namespace scopes; lexical assertions and the
[interpreted/compiled fixture tests](../../../src/Markup/FerroUI.Markup.Xaml.Loader/tests/winui_compiled.rs)
exercise property ownership, separate icon instances and native selection.
Those tests do not imply an executed result until the corresponding run is recorded.

The port follows the pinned
[running XAML](https://github.com/microsoft/WinUI-Gallery/blob/14061eeac21c38ae99f302edf3d9136d4f652c4c/WinUIGallery/Samples/NavigationView/NavigationViewPage.xaml)
and [code behind](https://github.com/microsoft/WinUI-Gallery/blob/14061eeac21c38ae99f302edf3d9136d4f652c4c/WinUIGallery/Samples/NavigationView/NavigationViewPage.xaml.cs).
Every exported example now has a source-hash/test mapping in
[port-status.json](../../../samples/WinUIGallery/port-status.json).
All eight remain **partial**; native composition and source handlers do not
establish complete XAML, template, host or visual equivalence.

| Source composition | Preserved behavior and explicit adaptation |
|---|---|
| `nvSample5` — default pane | Four original labels/icons/tags, initial first selection, header updates, distinct destination pages and Settings navigation |
| `nvSample6` — Top | Original fourth item deliberately still targets `SamplePage3`; the source header remains unchanged |
| `nvSample2` — adaptive orientation | Rust observes native host width against `CompactModeThresholdWidth`, falling back to sample bounds without a host; this explicitly adapts the VisualStateManager/AdaptiveTrigger expression |
| `nvSample7` — focus/tabs | Top, selection follows focus and collapsed back button; preserves the running handler's normal `Frame.Navigate` journal behavior |
| `nvSample4` — category data | Original category labels/icons/tooltips, separator and heading; live model collection and native template dispatch; all categories target `SamplePage1` as in the running source |
| `nvSample9` — footer | Original main/footer labels/icons/tags and shared selection; source Left/Top checked handlers |
| `nvSample8` — hierarchy | Original Home/Account/Document options tree, Mail/Calendar/Create new/Upload file children, destination types and nonselecting action parent |
| `nvSample` — API options | Original settings/back/header/search/custom-content/footer/focus/selection controls and two-way header/title bindings; no initial selection is invented |

The focus/tabs exported text suggests `NavigateToType` with journal recording
disabled, while the running pinned C# handler uses `Navigate`. The port keeps
the running behavior and records the discrepancy. The category example replaces
its custom C# `DataTemplateSelector` and category `x:Bind` expressions with
explicit Rust data-template dispatch. Source search and back controls have no
query-submission or back-navigation handler; the port retains that scope. The
seven sample destination types are real native FramePage subclasses with registered
Frame factories. Callback navigation failures are shown by the page's error
TextBlock rather than silently discarded.

The new destination pages retain the original resource values, shapes, colors,
tile layout and text; native wrapping adapts `WrapWholeWords` to `Wrap`. The
hierarchy explanation uses a multiline native TextBlock in place of the source
RichTextBlock paragraphs. These explicit presentation adaptations do not certify
general WinUI rich-text or line-breaking equivalence.

The InfoBadge page now also includes its
[original embedded NavigationView example](https://github.com/microsoft/WinUI-Gallery/blob/14061eeac21c38ae99f302edf3d9136d4f652c4c/WinUIGallery/Samples/InfoBadge/InfoBadgePage.xaml),
with Home, Account, Inbox, notification value 5, the Inbox automation name,
opacity toggling and the original LeftExpanded/LeftCompact/Top handlers.
Existing ToggleButton/TextBlock controls and explicit Rust wiring adapt the
ToggleSwitch header/state and page property. Its source Frame has no navigation
handler, so remains empty. The other three badge examples remain present.

**Boundaries:** There is no top overflow redistribution or `OverflowLabelMode`,
`NavigationViewTemplateSettings`, public `NavigationViewItemPresenter` or its
template-settings class, general `MenuItemTemplateSelector` or
`MenuItemContainerStyleSelector`, `IsTitleBarAutoPaddingEnabled`,
`ShoulderNavigationEnabled`, or recommended navigation-transition argument.
Acrylic, composition/selection transitions, sound, OS title-bar integration,
raw gamepad input, localization and full WinUI VisualStateManager execution
remain separate obligations. Native popup/provider tests use framework host
services; full desktop interaction and Windows visual/accessibility comparisons
remain pending. The API ledger attaches evidence only to exact members asserted
by focused tests, without promoting untested properties or all type members.

## InfoBadge

The contract follows the pinned
[InfoBadge IDL](https://github.com/microsoft/microsoft-ui-xaml/blob/a71c42bd3e352060e5f5ecf3b5e1a98f8a745809/controls/dev/InfoBadge/InfoBadge.idl),
[native implementation](https://github.com/microsoft/microsoft-ui-xaml/blob/a71c42bd3e352060e5f5ecf3b5e1a98f8a745809/controls/dev/InfoBadge/InfoBadge.cpp)
and [theme dictionary](https://github.com/microsoft/microsoft-ui-xaml/blob/a71c42bd3e352060e5f5ecf3b5e1a98f8a745809/controls/dev/InfoBadge/InfoBadge.xaml).
The [native implementation](../../../src/FerroUI.Controls/info_badge.rs)
derives from `TemplatedControl` and uses native properties, templates and icon
sources. It does not alter another control's defaults.

| WinUI member or behavior | Native implementation and boundary |
|---|---|
| `Value` | `i32`, default -1; values below -1 are rejected; `set_numeric_value` preserves the inherited generic `set_value(property, value)` method |
| Display priority | Any nonnegative value displays its full decimal integer; otherwise a non-null source displays an icon; otherwise a dot is displayed |
| `IconSource` | Default null; current FontIconSource, SymbolIconSource and PathIconSource create separate, live native elements |
| `TemplateSettings` | Read-only property returning the same per-owner `InfoBadgeTemplateSettings` instance |
| `InfoBadgeTemplateSettings.IconElement` | Mutable native icon reference; settings changes update the current content presenter |
| `InfoBadgeTemplateSettings.InfoBadgeCornerRadius` | Mutable native corner radius; updates the template border |
| Measurement | Desired width is at least desired height; default min width/height 4 and max height 16 |
| `CornerRadius` | A local value opts out of automatic half-height radius; style-only values do not; the local base remains detectable beneath animation |
| Named severity styles | Audited native `ControlTheme`s; a migration explicitly changes `Style` to `Theme` for these keys |

The default template has a Border named `RootGrid`, a native Grid, TextBlock
`ValueTextBlock`, Viewbox `IconPresenter` and ContentPresenter
`IconContentPresenter`. The border adapts the source grid's radius and padding
to native rendering. Font icons use margin `4,0,4,2`, other icons use `4`, and
numeric text uses `4,0,4,2` at size 11. `:value`, `:dot`, `:icon` and `:fonticon`
identify the presentation. Switching to value or dot preserves the last icon
in the settings object, matching the source; visibility determines what is
displayed. Retemplating releases the old presenter's content and binds the new
parts. Reentrant property callbacks finish with the current effective state.

The hand-maintained
[InfoBadge.xaml](../../../src/FerroUI.Themes.WinUI/Controls/InfoBadge.xaml)
dictionary retains `DefaultInfoBadgeStyle` and Attention, Informational, Success,
Caution and Critical Dot/Value/Icon style keys. Its implicit theme targets only
`FerroUI.Controls.InfoBadge`. Default, Light and native HighContrast scopes
provide the source brush aliases; common dimensions and margins are shared
native resources. The additive native `InfoBadge.HighContrastThemeVariant`
field (`high_contrast_theme_variant()` in Rust) supplies a typed `x:Static` key
with Dark inheritance. It equals the WinUI theme's high-contrast variant without
a theme-crate dependency and is not an upstream InfoBadge member. The dictionary
depends on the surrounding WinUI theme's
general brushes, including `TextOnAccentFillColorPrimaryBrush`,
`AccentFillColorDefaultBrush`, `SystemControlHighlightAltChromeWhiteBrush` and
`SystemControlHighlightAccentBrush`. Named severity themes use the corresponding
`SystemFillColor` brushes. Resource scope equivalence and pixel equivalence are
separate from matching a resource key. The control observes resource/theme
changes, and explicit foreground/background values retain native precedence.

**Boundaries:** BitmapIconSource, ImageIconSource and AnimatedIconSource are not
implemented. The native `NavigationViewItem.InfoBadge` presenter now hosts this
control, and the original Gallery Inbox composition has a partial executable port.
Arbitrary WinUI Style/VisualStateManager templates require a separate projection.
There is no InfoBadge-specific automation peer or Windows accessibility/pixel
comparison claim. Private numeric text is excluded from accessibility content;
applications can provide native automation metadata. Icon rendering retains the
font availability and platform limitations described above.

The dedicated `winui_info_badge` target contains assertions for priority and
validation, supported source types and ownership, sizing, local/style/animated
radius precedence, settings-driven parts, resources, retemplating and reentrant
updates. These assertions are distinct from a successful execution record and
from full upstream control or Gallery-page parity.

## ProgressRing

The source contract is the pinned
[ProgressRing IDL](https://github.com/microsoft/microsoft-ui-xaml/blob/a71c42bd3e352060e5f5ecf3b5e1a98f8a745809/controls/dev/ProgressRing/ProgressRing.idl),
[control implementation](https://github.com/microsoft/microsoft-ui-xaml/blob/a71c42bd3e352060e5f5ecf3b5e1a98f8a745809/controls/dev/ProgressRing/ProgressRing.cpp)
and [automation peer](https://github.com/microsoft/microsoft-ui-xaml/blob/a71c42bd3e352060e5f5ecf3b5e1a98f8a745809/controls/dev/ProgressRing/ProgressRingAutomationPeer.cpp).
The [native control](../../../src/FerroUI.Controls/progress_ring.rs)
derives from `TemplatedControl`; it does not reuse or change `ProgressBar`'s
different default state.

| WinUI member or behavior | Native implementation and boundary |
|---|---|
| `IsActive`, `IsIndeterminate` | Both default true; native pseudo classes track active and determinate/indeterminate modes |
| `Minimum`, `Maximum`, `Value` | `f64`, defaults 0, 100 and 0; the most recently set bound moves the other bound if needed, then clamps Value |
| Value assignment | `set_range_value` is the convenience setter; generic property assignment follows the same native coercion |
| NaN/degenerate ranges | NaN can remain public Value; `normalized_value()` returns zero for a nonfinite or collapsed range, keeping native arc geometry finite |
| `TemplateSettings` | Stable per-owner, read-only `ProgressRingTemplateSettings` |
| `EllipseDiameter`, `EllipseOffset`, `MaxSideLength` | Read-only legacy settings derived from arranged width, including the pinned 40-unit diameter threshold |
| Inactive state | Template root opacity becomes zero; outer visibility and layout remain; accessibility content view becomes Raw |
| Default size | Width/height 32, minimum width/height 16, centered, nonfocusable and not hit-testable |

The replaceable template contains native Grid `LayoutRoot` and rounded Arc
parts `PART_Track` and `PART_Indicator`. Explicit owner brushes, WinUI resources
and light/dark fallbacks feed these parts. The native inherited clock drives the
default motion. Indeterminate rotation and sweep follow the default source's
two-second sequence; increasing determinate progress traverses its interval at
that sequence's rate, while decreases apply immediately. Numeric changes do
not restart indeterminate motion. Resource lookup is outside the per-frame
geometry update.

The control disposes clock subscriptions on inactivity, effective hiding,
detachment, retemplating and determinate completion. A changed inherited clock
replaces the old subscription. Callbacks hold the owner weakly, and a
synchronously emitting clock cannot leave an obsolete subscription retained.

The [native automation peer](../../../src/FerroUI.Controls/automation/peers/progress_ring_automation_peer.rs)
uses the ProgressBar role and exposes RangeValue only while determinate.
The range provider reports read-only with NaN small/large changes, but its
setter forwards to Value, as the pinned peer does. Active indeterminate progress
prefixes the accessible name with the `ProgressRingIndeterminateStatus` resource
or the English fallback `Busy`; the localized role uses `ProgressRingName` or
`progress ring`. Effective range changes notify native automation clients.

**Boundaries:** The arcs are a native adaptation of the default visual. Preview
`DeterminateSource` and `IndeterminateSource` are absent; the control does not
provide AnimatedVisualPlayer, arbitrary Lottie/composition sources or exact
WinUI template-part compatibility. Legacy template settings do not imply that
the old dot template is rendered. OS localization, Windows UIA delivery, installed
theme/high-contrast appearance and pixel equality need their own host and visual
evidence. A custom template must supply the native parts or implement its own
presentation of the public properties.

The `winui_progress_ring` target contains assertions for defaults, range
coercion, finite geometry, legacy settings, real arc parts, live resources and
animation precedence, clock lifecycle and accessibility. Test presence and
successful execution are tracked separately.

## RatingControl and RatingItemInfo

The contract follows the pinned
[RatingControl IDL](https://github.com/microsoft/microsoft-ui-xaml/blob/a71c42bd3e352060e5f5ecf3b5e1a98f8a745809/controls/dev/RatingControl/RatingControl.idl),
[control implementation](https://github.com/microsoft/microsoft-ui-xaml/blob/a71c42bd3e352060e5f5ecf3b5e1a98f8a745809/controls/dev/RatingControl/RatingControl.cpp)
and [automation peer](https://github.com/microsoft/microsoft-ui-xaml/blob/a71c42bd3e352060e5f5ecf3b5e1a98f8a745809/controls/dev/RatingControl/RatingControlAutomationPeer.cpp).
The [native control](../../../src/FerroUI.Controls/rating_control.rs)
derives from `TemplatedControl` and uses native input, text/image rendering,
property resources and automation.

| WinUI member | Native implementation and boundary |
|---|---|
| `Value` | Default -1; negative values become -1, nonnegative values at or below 1 become 1, larger values clamp to MaxRating; fractions above 1 remain valid |
| `PlaceholderValue` | Default -1; negatives become -1, but zero and fractions below 1 are preserved; upper bound is MaxRating |
| `MaxRating` | Integer, default 5, coerced to at least 1 before dependent values; raising it later does not restore a previously clamped value |
| NaN assignments | Rejected by native Value/PlaceholderValue validation; this is an explicit native validity boundary |
| `InitialSetValue` | Default 1, used when keyboard/gamepad interaction starts from unset |
| `IsClearEnabled` | Default true; repeated pointer selection clears, including the maximum; keyboard increments at the maximum remain set |
| `IsReadOnly` | Default false; blocks interactive edits while preserving programmatic assignment |
| `Caption` | Default empty; real native text with no reserved caption spacing when empty |
| `ItemInfo` | Registered default null; construction supplies an independent default RatingItemFontInfo with set/unset star glyphs |
| `ValueChanged` | User-selection notification with null event arguments; property assignment and bound coercion do not raise it |

`set_rating_value` preserves the inherited generic property setter. The public
`select_rating` and `change_rating_by` methods route through the same interactive
policy as native input. As in the source, a handled selection can raise
`ValueChanged` even when the effective value remains unchanged, such as a capped
keyboard increment. Handlers use snapshots, so disposal and reentrant value
changes do not hold the handler collection borrowed.

Pointer interaction previews the corresponding item, captures the active
pointer on primary-button press and commits on that pointer's primary release.
Dragging before the first item can clear a rating. Capture cancellation,
read-only changes and unrelated pointer releases do not commit a preview.
Coordinates already transformed by native RTL layout are not reversed again.
Left/Right keys reverse direction in RTL; Up/Down retain their numeric direction.
Home clears when allowed and End selects the maximum. Fractional values step
to adjacent integers, and an unset value begins at InitialSetValue.

Normalized `KeyDeviceType::Gamepad` input uses Enter to engage/commit, Escape to
cancel, and Left/Right to edit while engaged. Cancel and focus loss restore the
pre-engagement value. This provides a native engagement state machine, not the
Windows `IsFocusEngagementEnabled` API or raw controller discovery/mapping.

### Rating item descriptions and rendering

[RatingItemInfo and its subclasses](../../../src/FerroUI.Controls/rating_item_info.rs)
are native FerroObject property objects in `FerroUI.Controls`.

| Type | Source properties | Native value type and fallback |
|---|---|---|
| `RatingItemInfo` | Public constructor | Extensible description base; the default renderer recognizes the font and image subclasses |
| `RatingItemFontInfo` | Glyph, DisabledGlyph, PointerOverGlyph, PointerOverPlaceholderGlyph, PlaceholderGlyph, UnsetGlyph | Strings, initially empty; a missing state uses Glyph, except PointerOverPlaceholder first tries PlaceholderGlyph |
| `RatingItemImageInfo` | Image, DisabledImage, PointerOverImage, PointerOverPlaceholderImage, PlaceholderImage, UnsetImage | Optional native `Rc<dyn IImage>`, initially null; the same fallback chain uses Image/PlaceholderImage |

Each item owns separate background and foreground TextBlock/Image visuals.
Foreground rectangles clip fractional values and placeholders; native scale
transforms apply the source's pointer-distance scaling function. Ordinary value
edits update existing items; changing their number, description kind or sizing
resources rebuilds the appropriate visuals. Item-description property changes
update live content without sharing a visual between parents. Replacing a
description disposes its old subscription, and callbacks hold the owner weakly.
Native `RatingControl` brush/spacing/rendering-size resources control the parts;
zero item spacing is a valid override.

A replacement native template exposes StackPanels `RatingBackgroundStackPanel`
and `RatingForegroundStackPanel`, plus TextBlock `Caption`. WinUI
`BackgroundGlyphDefaultTemplate`, `ForegroundGlyphDefaultTemplate`,
`BackgroundImageDefaultTemplate` and `ForegroundImageDefaultTemplate`
DataTemplates and arbitrary custom RatingItemInfo subclasses are not
automatically projected.
The default font is Segoe Fluent Icons; applications must supply an appropriate
font or use image descriptions. Native `IImage` keeps FerroUI image ownership
and rendering; it is not a WinRT ImageSource ABI adapter.

### Rating automation and remaining boundaries

The native peer reports the Slider role and supplies Value and RangeValue
patterns. Range minimum is zero, maximum is MaxRating, both step sizes are one,
and unset Value projects to zero without substituting PlaceholderValue. Text
descriptions distinguish a user rating, community placeholder and unset state.
An explicit accessible name precedes the caption fallback. Both provider setters
perform programmatic assignment, including when interactive input is read-only,
matching the source's assignment behavior. Numeric and text property-change
notifications use their corresponding native value types.

Descriptions and the localized role currently use English, with Rust formatting
and invariant string parsing. They do not implement Windows DecimalFormatter's
locale-sensitive parsing, rounding or resource lookup. The native transforms
apply the scaling function but do not implement Windows composition-expression
animation timing. The source's text-scale-dependent glyph measurement and
`RatingControlCaptionTopMargin` resource are not projected. Windows
focus-engagement/access-key behavior, raw controller mapping, exact UIA event
sequences, screen-reader output, custom WinUI templates and visual/pixel equality
remain separate obligations.

The `winui_rating_control` target contains assertions for numeric domains,
coercion ordering, user-event behavior, keyboard/gamepad policy, real pointer
capture and RTL, fractional clips, glyph/image fallback and lifetime, caption
layout and native automation. Successful runs are recorded independently of
this description of the implementation.

## Local values beneath animation

The additive native
[`FerroObject::has_local_value`](../../../src/FerroUI.Base/ferro_object.rs)
query reports whether a styled property's current non-animation base has
`BindingPriority::LocalValue`. It remains true when an animation hides that
base. An explicit local assignment equal to the default counts; a style,
template, inherited/default value or an animation without a local base does not.
Attaching a binding before it supplies a value does not count. If a native
binding supplies a local fallback, that effective fallback does count, including
the existing observable-Unset fallback behavior.

This query is not a WinUI `ReadLocalValue` implementation and does not expose
stored binding expressions. It adds no property priority and changes no ordinary
binding mode or Avalonia-compatible control default. The three controls above
use distinct convenience setter names, preserving the inherited
`set_value(property, value)` API for applications and bindings.

The existing [property store](../../../src/FerroUI.Base/property_store/value_store.rs)
and its Avalonia-compatible clearing behavior are unchanged. Clearing an ordinary
local value while an animation is effective retains the hidden local base, as
specified by the existing
[reentrancy regression](../../../src/FerroUI.Base/tests/ferro_object_tests_reentrancy.rs).
Once the animation ends, clearing that local value restores the ordinary
lower-priority value. InfoBadge therefore preserves the animated radius while
the local base exists, restores the local radius after animation disposal, and
returns to half-height radius when the local value is subsequently cleared.
Focused assertions in `ferro_object_tests_get_value` cover default/style/inherited
values, explicit local defaults, binding attachment/fallback and animation-hidden
locals; InfoBadge has a corresponding radius/resize/clear regression. Execution
and upstream acceptance remain evidence ledger concerns.

## Evidence and completion

`data/api-mappings.json` records explicit namespace/member mappings and their
limits. `data/api-evidence.json` records evidence for exact source signatures;
`data/api-map.json` joins this evidence to declaration candidates. Verified
evidence must point at an actual successful execution record tied to the
implementation/test file hashes. Partial evidence keeps its boundaries and
does not turn a whole type or Gallery page into verified parity.

The local native-promotion execution remains in
[`data/executions/native-controls-005.json`](data/executions/native-controls-005.json):
**182 tests passed across all 12 native integration targets**, with **53 source
hashes** frozen before and checked after that local run. All 521 current API evidence
entries now reference the fresh
[`data/executions/native-controls-007.json`](data/executions/native-controls-007.json):
**182 native tests passed** on CI checkout `2041e251`, tree `ab341a63`, including
main `171a9ad8`'s updated `Cargo.lock`. Its 53 source hashes were extracted afterward
from the actual checkout's Git blobs; no execution-time before/after snapshot is
claimed. The corresponding merge `34671e41` has the same tree, but was not the
execution commit. Only `Cargo.lock` changed among those inputs since
[`data/executions/native-controls-006.json`](data/executions/native-controls-006.json);
that earlier execution remains unchanged, and the exact source guard still applies.
Full source identities are in the [validation ledger](../../../samples/WinUIGallery/VALIDATION.md).

Behavior classifications remain unchanged; the newly matching
`NumberBoxValueChangedEventArgs` source type and `OldValue`/`NewValue` member
candidates do not acquire behavior evidence from a name match. The earlier records
below retain their original source scope and test counts.

The first recorded native-control execution is
[`data/executions/native-controls-001.json`](data/executions/native-controls-001.json):
**47 integration tests passed** (`winui_icons`: 7, `winui_radio_buttons`: 9,
`winui_navigation`: 17, `winui_number_box`: 14). The record contains the command,
every passing test name and SHA-256 values for the exact implementation and
test inputs. Native text drawing uses deterministic test fonts; native input,
layout and automation tests use the framework's test services.

The next recorded execution is
[`data/executions/native-controls-002.json`](data/executions/native-controls-002.json):
**30 integration tests passed** (`winui_auto_suggest_box`: 13,
`winui_icon_sources`: 7, `winui_info_bar`: 10). The first two records cover
**77 native control integration tests**. The second record covers native
suggestion text coalescing and event identity, icon-source ownership and reentrant
updates, and InfoBar lifecycle, commands, resources, layout and automation. Its
hash scope lists the actual implementation/test files; a later shared registration
helper relocation is outside that execution record and is validated separately.

The third recorded execution is
[`data/executions/native-controls-003.json`](data/executions/native-controls-003.json):
**50 new focused integration tests passed** (`winui_info_badge`: 13,
`winui_progress_ring`: 16, `winui_rating_control`: 21). This run also reran all
77 earlier native-control tests, for **127 passed and 0 failed** in the combined
suite. The record supports **61 new partial member contracts and 10 partial
types**, using actual native properties, templates, input, layout, clock and
automation objects with mock host services. Its source hashes include the additive
Base local-value query dependency; the separate full Base run passed 4,383 tests
with existing clearing and priority semantics preserved.

The second record's 87 member entries and the third record's 61 member entries
remain **partial**, with focused test anchors for each recorded contract.
For example, AutoSuggestBox's `QueryIcon` entry records its tested initial null
value without claiming that this test run verified arbitrary live icon content.
The third record leaves ProgressRing's four absent animated-source members and
RatingItem state branches without direct test assertions behavior-unverified.
RatingControl's NaN rejection is documented as a native validation exception;
these tests do not certify upstream NaN, localization or Windows UIA semantics.
Unsupported members have no successful evidence entry, and broader inherited,
host-service and visual obligations remain visible in the comparison.

This evidence verifies the complete `Symbol` enum contract and selected narrow
icon member contracts. The control types, native collection/template adapters,
navigation and number formatting remain marked **partial**. Successful tests
are evidence for their stated scope, not a certification of all inherited
members, platform integrations, visuals or Gallery examples.
