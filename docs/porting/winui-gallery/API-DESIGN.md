# Additive WinUI contracts and the Gallery port

## Objective and compatibility boundary

The target is a native FerroUI application that can keep the structure, properties,
events and behavior of the WinUI Gallery's XAML. A port should principally change
namespaces and explicitly mapped types. Its C# code-behind still needs a Rust
implementation and registered application metadata; neither an XML rewrite nor a
source browser executes C#.

FerroUI's existing Avalonia-compatible object model, APIs, markup and default
behavior remain the foundation. The implemented controls are ordinary
`ferroui_controls` exports with `FerroUI.Controls` identities in the default
presentation namespace. They use the same native services as existing controls.
The source importer remains an explicit operation because it must diagnose
unimplemented WinUI syntax and semantic differences.

Two existing names have different contracts. The Frame destination type is
`FramePage`, preserving the existing `Page` hierarchy and its asynchronous
navigation behavior. Frame navigation arguments and options live in
`ferroui_controls::navigation` / `FerroUI.Navigation`, preserving the existing
root `NavigationEventArgs`. `PageStackEntry` also belongs to `FerroUI.Navigation`.
These explicit mappings let the Gallery use native APIs without changing the
meaning of existing applications.

The source contracts are pinned in [sources.lock.json](data/sources.lock.json).
The complete declaration and use-site inventory, rather than the examples in
this document, defines the remaining work. [REPORT.md](data/REPORT.md) states
the measured port state. This design includes required future contracts and
must not be read as a claim that all of them are implemented.

## Projection rules

| Situation | Required implementation |
|---|---|
| Same API and semantics | Reuse the existing native type and registered member. |
| Different spelling, same audited meaning | Add a per-document namespace, type or member mapping; retain a source-range edit record. |
| Additional non-conflicting capability | Add a normal native property, event, method or service with existing conventions. |
| Conflicting default, value domain or lifetime | Add a distinct native control or service with an explicit mapping; keep the original type and its defaults. |
| Platform-dependent capability | Implement the corresponding platform service and expose the actual result/capability. |
| Unsupported requirement | Keep its declaration/use site in the parity ledger and produce an actionable error. |

Every projected control uses the same visual tree, object/property registry,
layout, input, dispatcher, text system, renderer, resource system and automation
infrastructure as other FerroUI controls. There is no second rendering engine
or separate emulation tree. Native templates are replaceable through the
existing template APIs.

The code generator resolves scoped identities before deriving builders and
property types. Existing `PageProps`, `ContentPage` and their inheritance remain
unchanged. New controls use normal native builder names such as `FramePage`,
`NumberBox`, `frame_page()` and `number_box()`. There is no
`ferroui_controls::winui` forwarding module or separate control namespace.

| Surface | Native location |
|---|---|
| Controls, icon sources and control events/enums | `FerroUI.Controls` / `ferroui_controls` |
| Frame navigation contracts | `FerroUI.Navigation` / `ferroui_controls::navigation` |
| `InfoBarPanel` and the three template-settings types | `FerroUI.Controls.Primitives` / `ferroui_controls::primitives` |
| Automation peers | `FerroUI.Automation.Peers` / `ferroui_controls::automation::peers` |

## Implemented control foundations

### FramePage and Frame

`FramePage` maps the source `Microsoft.UI.Xaml.Controls.Page`. It derives from
native `UserControl` and adds a weak `Frame` link,
`NavigationCacheMode` and virtual `OnNavigatingFrom`, `OnNavigatedFrom` and
`OnNavigatedTo` callbacks. `Frame` is a native content host. A page type can be
created through its registered constructor or an explicit `PageFactory`; the
factory must return a page compatible with the requested type.

`Frame` exposes `Navigate`, `NavigateToType`, `GoBack`, `GoForward`,
`CanGoBack`, `CanGoForward`, `BackStackDepth`, `SourcePageType`,
`CurrentSourcePageType`, `CacheSize`, `IsNavigationStackEnabled`, live
`BackStack`/`ForwardStack`, and navigation events. Rust operations that can fail
return `Result`. `PageStackEntry` carries the source type and parameter.

The journal is a controlled observable native collection. Public mutation
methods validate ownership and reject changes while navigation is pending.
Notification delivery observes committed state; event handlers can inspect the
content and stacks without borrowing partially updated storage. Page-to-frame
links do not keep frames alive.

The native event sequence is part of the contract: `Frame.Navigating`, the old
page's cancellation callback, then either `NavigationStopped` or a committed
content/history change followed by `Frame.Navigated`, old-page departure and
new-page arrival. Cancellation returns accepted-request success; a reentrant
navigation attempt returns false. A handled construction failure returns false;
an unhandled failure retains the error. These outcomes are not interchangeable.

Caching follows the pinned implementation, including its less obvious details:
default capacity 10, an LRU for Enabled entries, a persistent bucket for
Required entries, zero-capacity admission rules, and explicit invalidation when
a cached page changes to Disabled. Existing `NavigationPage` behavior remains
independent. Navigation transition effects and serialized journals are separate
requirements, not successful empty methods.

### NavigationView and hierarchical menu items

The native `NavigationView` derives from `ContentControl` and composes
the existing `SplitView`, panels, presenters, buttons and popup services.
`NavigationViewItemBase` preserves the common content/selection inheritance;
`NavigationViewItem`, `NavigationViewItemHeader` and
`NavigationViewItemSeparator` provide selectable rows, headings and separators.
Their fallback templates are property metadata defaults, so a native
`ControlTheme` or application template can replace them. Existing `SplitView`,
selection controls and their Avalonia-compatible defaults remain unchanged.
The public shape follows the pinned
[NavigationView IDL](https://github.com/microsoft/microsoft-ui-xaml/blob/a71c42bd3e352060e5f5ecf3b5e1a98f8a745809/controls/dev/NavigationView/NavigationView.idl).

The native menu and footer collections are instance-owned, read-only properties
whose contents remain writable. Assigning `MenuItemsSource` or
`FooterMenuItemsSource` selects a live native `ItemsSource` without replacing or
locking the corresponding inline collection. Clearing the source restores the
inline items. Generated containers preserve their source `BoxedValue`s and
survive unrelated collection edits; nested items use the same policy.
`ContainerFromMenuItem` and `MenuItemFromContainer` expose that relationship.
`MenuItemTemplate` accepts native `IDataTemplate` and can create a menu item,
heading or separator. A general WinUI `DataTemplateSelector` is still separate
work; the Gallery category example makes that adaptation explicit in Rust.
If a template factory changes `MenuItemTemplate` while containers are being
built, the replacement factory rebuilds the containers and preserves their model
round trips. Ordinary native controls supplied as nested items retain the live
inherited application `DataContext` through their generated wrappers, for both
inline collections and active sources.

Selection is shared across main items, footer items, descendants and Settings.
`SelectedItem` retains the original model; `SelectionChanged` includes that
model, its realized container and the Settings flag. `ItemInvoked` instead
exposes the invoked container's content and can occur without a selection
change. Invocation runs before selected-container flags and the selection
notification; a reentrant application selection takes precedence. An action
with `SelectsOnInvoked = false` can invoke or expand without replacing the
current selection. Ancestors reflect a selected descendant through
`IsChildSelected`. Lazy expansion uses `HasUnrealizedChildren` and real
`Expanding`/`Collapsed` events. Every item can host a live native `Icon` and
`InfoBadge`. The two automation peers use the runtime namespace
`FerroUI.Automation.Peers` and separate source modules, following the existing
native automation organization.

Selecting a model without a realized container schedules its selection
notification on the native dispatcher. The callback completes only the current
selection and can report a null container. Realizing the model first completes
the notification with that container and consumes the queued work. A later
selection or clear invalidates the old callback, and queued work holds the
owner weakly.

`Auto`, `Left`, `LeftCompact`, `LeftMinimal` and `Top` drive real layout. Auto
uses the pinned thresholds, 641 and 1008, while the initial compact/open lengths
are 48 and 320. Explicit user closure survives an expanded resize. Top mode
reparents the same menu controls into a horizontal host; it currently has no
overflow redistribution. Pane open/close events and cancelable light dismissal
are distinct from an explicit property assignment. A back-button request does
not perform Frame navigation by itself. Headers, search, pane custom content,
footer and content overlay use native presenters. Style-valued additions accept
audited native `ControlTheme`s; this does not convert arbitrary WinUI styles.
Replacing or clearing a template removes the old header/content presenters'
data templates before their content. Factories that accept null data therefore
cannot recreate a detached visual during teardown, and replacement presenters
can adopt the same native content safely.

The [Gallery page](../../../samples/WinUIGallery/Pages/NavigationView.xaml)
contains all eight original compositions and their named controls, menu data,
destination pages and source option handlers. The
[Rust handlers](../../../samples/WinUIGallery/navigation_view_samples.rs)
explicitly adapt category template selection and the adaptive visual-state
expression. The InfoBadge page now also contains its original NavigationView
composition, with the Inbox count, automation name, opacity toggle and pane
mode handlers. These are partial executable ports: exact templates/resources,
complete XAML language behavior, native host interaction and visual comparisons
remain separately tracked.

The remaining NavigationView contracts include top overflow and
`OverflowLabelMode`, both template-settings projections, the public
`NavigationViewItemPresenter`, template/style selectors, title-bar auto padding,
acrylic and composition transitions, recommended navigation-transition arguments,
shoulder/gamepad input, OS localization and Windows accessibility comparisons.
The declaration map retains these gaps. [CONTROL-CONTRACTS.md](CONTROL-CONTRACTS.md#navigationview)
states the narrower evidence and the original Gallery behavior being preserved.

### NumberBox

`NumberBox` derives from native `TemplatedControl`. Its `Value` is `f64` with
NaN representing an empty value; its initial bounds are the finite double
extremes. It cannot alias `NumericUpDown`, whose decimal value model must remain
unchanged. The new control has native text, repeat-button and popup parts and
an actual range-value automation peer.

The numeric contract includes small/large steps, minimum/maximum coercion,
wrapping, invalid-input policies, explicit edit commitment, escape restoration,
keyboard and wheel input, header/description presentation, inline/compact/hidden
spinners, and the old/new `ValueChanged` event. Reentrant event changes retain
the value chosen by the handler without issuing an extra outer event. Detached
template parts lose their subscriptions.

Expression evaluation uses the pinned grammar and precedence, including
left-associative exponentiation and NaN for division by zero. It is iterative
and bounded. Numeric tokens use the caller's number parser; formatters implement
both parsing and formatting. Display rounds to ten significant digits with
ties toward positive infinity while preserving the full stored value. The
portable default formatter is invariant. Windows regional number services and
remaining text-input-specific properties still need host adapters.

The rounding policy comes from the pinned `NumberBox.cpp` and the documented
[SignificantDigitsNumberRounder default](https://learn.microsoft.com/en-us/uwp/api/windows.globalization.numberformatting.significantdigitsnumberrounder.roundingalgorithm)
and [rounding algorithms](https://learn.microsoft.com/en-us/uwp/api/windows.globalization.numberformatting.roundingalgorithm).
It does not establish a bit-for-bit comparison against Windows globalization.

The automation peer reports a single coerced value after application handlers.
Its writable pattern follows the pinned native peer, including programmatic
writes when interactive input is disabled. Explicit accessibility names take
precedence over the header fallback. Step-property identifiers also map to the
Windows UIA property identifiers.

### Selection and icons

`RadioButtons` reuses native selection and actual radio containers, with a
column-major layout and directional navigation. `FontIcon` and `SymbolIcon`
render through native text services. The `Symbol` public numeric values and
rendered glyph values have separate, pinned maps; equating them is incorrect.
See [CONTROL-CONTRACTS.md](CONTROL-CONTRACTS.md) for exact supported members,
remaining differences, font requirements and test boundaries.

`IconSource` adds a live, reusable source object that creates separate native icon
elements. `FontIconSource`, `SymbolIconSource` and `PathIconSource` propagate source
property changes through weak references to created elements; an icon does not keep the source or
another icon alive. Local foreground values retain native precedence. This makes
source objects usable in controls such as InfoBar without sharing a visual between
two parents.

### AutoSuggestBox

The native `AutoSuggestBox` uses header/items services, TextBox, ListBox and
Popup parts. Applications supply `Items` or `ItemsSource`; the control does not
apply an implicit prefix filter. The existing `AutoCompleteBox` retains its own
filtering and completion contracts.

The text event preserves the pinned 150 ms coalescing interval and distinguishes
`UserInput`, `ProgrammaticChange` and `SuggestionChosen`. A retained event's
`CheckCurrent()` becomes false on a later text change or owner destruction. Event
arguments and timers hold the owner weakly. Choosing a suggestion updates text
through `TextMemberPath` when `UpdateTextOnSelect` is enabled, then raises
`SuggestionChosen`. Enter submits the selected item; the query button and native
automation invoke submit only the current query text. Closing clears selection.

Keyboard navigation preserves the user's draft when moving beyond a list edge or
pressing Escape/Tab, including the native single-item exception. Header, description,
placeholder, icon, item template and maximum list height update actual native parts.
Retemplating disconnects old part handlers. The automation peer follows the native
Group/Invoke contract rather than the existing autocomplete peer's patterns.

The initial implementation does not claim arbitrary WinUI `TextBoxStyle`,
`HeaderPlacement`, `AutoMaximizeSuggestionArea`, input-pane maximization/occlusion,
`LightDismissOverlayMode`, IME candidate coordination,
gamepad navigation or the Windows above-popup reversed-list behavior. Those remain
explicit API/host obligations, not ignored properties.

### InfoBar

The native `InfoBar` is a content control with live severity, title, message,
action, content and icon-source presentation. Opening and closing have distinct
events; programmatic closing and a close-button click preserve their separate
reasons. Closing can be canceled. Commands use native command execution and enabled
state, and closing collapses the template's content while retaining the control's
own visibility setting.

The panel follows the source's inline-versus-stacked layout policy. Existing WinUI
theme resources provide severity colors, typography, spacing and the close-button
theme. `CloseButtonStyle` accepts an explicitly audited native `ControlTheme`; it
does not make arbitrary WinUI Style objects interchangeable. Accessibility uses
native peer/property notifications and live-region metadata. OS screen-reader
announcements and upstream pixel equivalence need separate host evidence.

### InfoBadge

The native `InfoBadge` derives from `TemplatedControl`. `Value` defaults
to -1; a nonnegative value displays the whole integer, otherwise an `IconSource`
displays an icon, otherwise the control displays a dot. Values below -1 are
rejected. A source creates a separate native icon for each badge and keeps its
live-property behavior. The current font, symbol and path sources work through
the existing source contract; this does not supply the missing bitmap, image or
animated source types.

Each badge owns a stable `InfoBadgeTemplateSettings` object. Its mutable
`IconElement` and `InfoBadgeCornerRadius` properties drive actual template
parts. Measurement keeps the desired width at least the desired height. Without
a local `CornerRadius`, the default radius is half the arranged height; a local
radius retains priority even while an animation supplies the effective value.
Styles alone do not opt out of the automatic-radius policy.

`Controls/InfoBadge.xaml` supplies native `ControlTheme`s with the original
default and Attention, Informational, Success, Caution and Critical style keys
for dot/value/icon usage. An audited migration maps their `Style` use to native
`Theme`. Light, dark and high-contrast dictionaries use the native resource
system and require the surrounding WinUI theme's general brushes. The native
template can also run with its resource fallbacks. The dictionaries do not
implement arbitrary WinUI visual states. `NavigationViewItem.InfoBadge` now
hosts and updates the same native badge through its item presenter; the original
Gallery Inbox composition remains partial pending visual and host acceptance.

The additive native `HighContrastThemeVariant` static field
(`high_contrast_theme_variant()` in Rust) supplies a typed high-contrast key
with Dark inheritance for resource dictionaries. It matches the WinUI theme's
variant without requiring that optional crate; it is an integration API, not a
WinUI InfoBadge member.

### ProgressRing

The native `ProgressRing` preserves `IsActive = true`, `IsIndeterminate = true`,
`Minimum = 0`, `Maximum = 100` and `Value = 0` without changing the existing
`ProgressBar` defaults. Setting either range bound gives that bound precedence,
then clamps the value. NaN can remain the public value; a nonfinite or collapsed
range produces safe zero progress in native geometry. Read-only legacy
`ProgressRingTemplateSettings` properties update from actual width.

The replaceable native template uses two `Arc` shapes, native brushes and the
inherited framework clock. The pinned default motion is expressed through native
arc angles on a two-second timeline. Determinate increases animate over their
fraction of that timeline; decreases apply immediately. The subscription stops
when inactive, effectively hidden, detached, retemplated or finished. Inactive
presentation retains layout while hiding the template content and removing it
from the accessibility content view. Resource/theme changes and explicit brushes
update the same parts.

The native peer reports the ProgressBar role, active-indeterminate busy text and
the RangeValue pattern only in determinate mode. The peer advertises read-only
progress but forwards its provider setter, as the pinned implementation does.
The native arcs are an explicit rendering adaptation. Preview
`DeterminateSource`/`IndeterminateSource`, `AnimatedVisualPlayer`, arbitrary
composition sources and Windows/pixel-comparison evidence remain separate
requirements.

### RatingControl and item descriptions

The native `RatingControl` implements the source's distinct numeric domains:
`Value = -1` is unset and a nonnegative assignment is coerced into
`[1, MaxRating]`; `PlaceholderValue` also defaults to -1 but retains fractions
and zero in `[0, MaxRating]`. `MaxRating` defaults to 5 and is at least 1. Bounds
are corrected before dependent values. Native validation rejects NaN rather
than putting an invalid value into the rendering path.

The control supplies native pointer capture and hover preview, keyboard editing,
RTL direction handling, clear-on-repeat selection and read-only/disabled input
gates. `InitialSetValue` defaults to 1. Programmatic assignments do not raise
`ValueChanged`; user selection paths do. Normalized gamepad Enter/Escape and
horizontal keys support engagement, commit and rollback through native input.
Device discovery, raw controller mapping and Windows focus-engagement events
remain host/framework requirements.

`RatingItemInfo`, `RatingItemFontInfo` and `RatingItemImageInfo` are native property
objects. Font and image descriptions expose the six source states and their
fallback chains. Every control has its own default star description, and every
item has its own native visual. The image contract accepts native `IImage`
objects. Foreground clipping displays fractional ratings; native transforms
apply the pointer-distance scale. A caption uses a real TextBlock, and an empty
caption reserves no caption spacing.

The native automation peer supplies Value and RangeValue patterns with the
unset numeric value projected to zero. Its descriptions currently use English
and Rust number formatting/parsing. Windows DecimalFormatter/localization,
text-scale-derived glyph geometry, the source caption-top-margin resource,
composition-driven animation timing, custom WinUI item templates, device and
screen-reader comparisons, and pixel equivalence are not established by the
native implementation.

### Native property API and compatibility

These controls and their helper types belong to the native namespaces listed
above. Their metadata overrides apply only to their own types. Existing Avalonia-compatible
types, ordinary binding defaults, property priorities and generic assignment
APIs remain available. In particular, convenience setters have distinct names
so they do not hide `FerroObject::set_value(property, value)`:

| Native control | Convenience assignment | Generic assignment remains available |
|---|---|---|
| `InfoBadge` | `set_numeric_value(i32)` | `set_value(InfoBadge::value_property(), value)` |
| `ProgressRing` | `set_range_value(f64)` | `set_value(ProgressRing::value_property(), value)` |
| `RatingControl` | `set_rating_value(f64)` | `set_value(RatingControl::value_property(), value)` |

The additive `FerroObject::has_local_value(&StyledProperty<T>)` query inspects
the current non-animation base priority. It distinguishes a local value,
including an explicit assignment equal to the default, from styles, templates,
inheritance and defaults. It sees a local base under animation, but does not
report a binding attachment before it supplies a local value or fallback. This
is an effective-local-value query, not a complete `ReadLocalValue` projection
that returns WinUI binding expressions.

Existing Avalonia-compatible `clear_value` semantics remain unchanged: an
ordinary local value hidden by an active animation is retained when cleared.
The animation must finish or be disposed before clearing that local value.
InfoBadge consequently keeps its animated local radius until the animation
ends, then uses the local radius; clearing it afterward restores the automatic
half-height radius. The query supplies the distinction the badge needs without
changing base property-store behavior. Detailed source links, member contracts
and test boundaries are in [CONTROL-CONTRACTS.md](CONTROL-CONTRACTS.md).

## XAML API

`migrate_winui_xaml(source, options)` returns a reviewable document, exact
non-overlapping source edits and diagnostics with original byte ranges and
one-based line/column positions. `into_xaml()` rejects unresolved errors. The
optional runtime entry points `parse_winui`, `parse_winui_as` and `load_winui`
use that check before the existing loader validates real type/member metadata.

Options are per document. Applications can provide audited namespace mappings,
exact type aliases, exact member renames, theme policy, known style-resource
keys and an asset URI base. Duplicate/conflicting mappings are errors. A map
does not prove the target implements the source contract.

| Construct | Contract |
|---|---|
| Presentation and `using:` namespaces | Resolve XML scopes and prefix shadowing; migrate only the intended identity. |
| Text, comments, escaped values and handlers | Preserve their contents and original source locations. |
| Ordinary WinUI `Binding` | Make the OneWay default explicit; preserve explicitly selected modes and the existing binding engine. |
| `ThemeResource` | Require explicit dynamic-resource policy and installed ported dictionaries/theme scopes. |
| Named WinUI `Style` resource | Map to `ControlTheme` only for an audited key; arbitrary setter/state semantics require a real projection. |
| `NavigationViewItem.Icon` and `AutoSuggestBox.QueryIcon` symbol literals | Expand plain WinUI literals into instance-owned native `SymbolIcon` property elements; record `WXAML2005` at the original value range. |
| `ms-appx:///` | Convert resource-bearing URI attributes only when the host supplies an absolute asset base. |
| Application `x:Class` | Apply explicit namespace identity mappings; require the application's actual Rust type and code-behind. |
| `mc:Ignorable` | Respect namespace scopes; ignored designer trees do not create runtime compatibility claims. |
| Unsupported directives or semantics | Preserve the source requirement and reject loading; never silently remove it. |

The icon-literal adaptation is limited to those two members in recognized WinUI
control namespaces. For example, `Icon="Mail"` becomes an `Icon` property
element containing a native `SymbolIcon` with `Symbol="Mail"`. The native Symbol
parser validates the value; migration does not invent an enum value or install a
global string-to-icon converter. Each document instance receives its own visual.
Resource and binding expressions keep their icon-object contract, explicit
application type/member mappings take precedence, and existing native FerroUI
markup retains its meaning. The
[migration tests](../../../src/Markup/FerroUI.Markup.Xaml.Loader/tests/winui_migration.rs)
and paired interpreted/compiled
[NavigationView fixture](../../../src/Markup/FerroUI.Markup.Xaml.Loader/tests/fixtures/winui_navigation.xaml)
provide separate assertions for lexical preservation and runtime ownership;
execution evidence is recorded independently.

Deferred resource construction may cross a framework interface that cannot
return `Result`. Fresh-root WinUI loading has a narrow typed-error bridge for
the framework's own XAML exceptions. It preserves inner errors and resumes
unrelated application panics. It does not promise rollback for an existing root
and requires an unwinding Rust build. Ordinary loader behavior remains unchanged.

### Explicit property-path x:Bind

`WinUIXBindPolicy::PropertyPathsWithExplicitLifecycle` enables a separate
`FerroUI.Markup.Xaml.WinUI.WinUIBindExtension`. The default policy continues to
reject `x:Bind`. The migration pass records its original line and column in the
extension, so a delayed initialization failure can identify the original use site.
It does not rename the expression to an ordinary `Binding`.

```rust,ignore
let mut options = WinUIXamlOptions::for_winui_theme();
options.x_bind = WinUIXBindPolicy::PropertyPathsWithExplicitLifecycle;
let root = FerroRuntimeXamlLoader::parse_winui_as::<Ref<Control>>(
    source, None, &options,
)?;
// Assign the document's Rust model/fields here, after loading its controls.
let bindings = WinUIBindings::for_root(&root);
bindings.set_stop_on_unload(true);
bindings.initialize()?;
// After an asynchronous operation, explicitly refresh OneTime paths if needed:
bindings.update()?;
```

| Contract | Implemented property-path behavior |
|---|---|
| Root source | The declaring document object; its implicit DataContext is not the source. |
| Named elements | Resolve after construction; document named fields take precedence over same-named root properties. |
| Template source | Each `DataTemplate` instance uses its own root's DataContext, checked against the declared `x:DataType`. Item properties take precedence over same-named template elements. |
| Default mode | OneTime: one read per Initialize/Update, without source-property subscriptions. |
| OneWay | Follow registered native/model property notifications and intermediate source replacement. |
| TwoWay | Require a declared writable source property. `TextBox.Text` defaults to LostFocus; other registered target properties default to PropertyChanged. Explicit triggers are retained. |
| Initialize | The host calls it after constructing controls and assigning code-behind fields. Repeated successful calls do not reread OneTime paths. |
| Update | Reread all paths; report source/type/conversion errors together. A user getter panic propagates. The operation is not transactional. |
| StopTracking | Release source and template-item listeners, block writes, preserve target values and permit a later Initialize. |
| Unload | The host can explicitly enable stopping. Loaded is not substituted for WinUI Loading. |
| Template recycling | After initialization, replacing the typed item refreshes only that instance, including its OneTime paths. |
| Ownership | Root disposal releases source listeners even if application code separately retains a target and its binding controller. |

Application fields must expose registered Rust property/model metadata. This
adapter uses that metadata at runtime in both interpreted and compiled XAML; it
does not provide generated-code performance, WinMD ABI, or WinUI's complete
expression language. Source-member renames still need an explicit application
contract; a target-member alias does not prove an arbitrary model path equivalent.
Converters, fallback/null-value parameters, functions, casts, indexers, event
dispatch, `x:DefaultBindMode`, `x:Phase` and deferred loading remain diagnosed
requirements. The generated event table follows the pinned owner and base chain
so custom properties with event-like names are not rejected by name alone.

StopTracking resets this explicit controller even for groups containing only
OneTime bindings; WinUI can omit that reset when its generated group has no
tracking. The native binding engine retains weak, dormant target adapters while
stopped. They cannot read or write a source after source accessors are released.
These boundaries are stated separately from the remaining generated binding
compiler work. Public lifecycle tests live in
`src/Markup/FerroUI.Markup.Xaml/tests/winui_bindings.rs`; migration and runtime
loading tests live in the loader's `tests/winui_migration.rs`.
Ordinary Binding and CompiledBinding keep their existing defaults and ownership
semantics.

## Additional contracts required for complete Gallery portability

The following are implementation obligations, not declarations of completed
features. A supported subset must retain diagnostics for the rest.

| Area | Required native contract and acceptance boundary |
|---|---|
| Complete `x:Bind` compiler | Extend the explicit property-path subset with generated source access, full lifecycle integration, converters, functions, casts, indexers, phased updates and event dispatch; keep diagnostics for every unsupported expression. |
| Deferred markup | `x:Load` and deferred strategies must manage construction, name scopes, subscriptions, focus and disposal; visibility alone is insufficient. |
| Visual states | State groups, triggers, setters and transitions must own temporary property values and restore prior bindings/styles without overwriting local state. |
| Template selection and virtualization | DataTemplateSelector, element factories, recycling, ownership, index changes and layout algorithms must use the native item generator/layout services. |
| NavigationView | Extend the native hierarchy, pane lifecycle and selection/invocation contract with top overflow, template settings/presenter types, template/style selectors, title-bar padding, acrylic/transitions, raw gamepad input and full native-host/Windows visual and accessibility acceptance. |
| AutoSuggestBox | Extend the implemented text/query/selection contract to input-pane maximization, overlay/IME coordination, styles, gamepad input and platform-specific popup behavior. |
| InfoBar and icon sources | Complete the remaining source variants, style projection and native accessibility/visual comparisons beyond the implemented close lifecycle and live projections. |
| InfoBadge | Complete missing icon sources, arbitrary custom visual states and Windows visual/accessibility comparisons beyond the implemented NavigationView item composition. |
| ProgressRing | Project the preview custom animated-source APIs through real native composition services, then validate source/template behavior and Windows visuals. |
| RatingControl | Complete WinUI custom item-template projection, localized number/automation services, platform gamepad integration and composition/visual comparisons. |
| ListView, GridView and TabView | Item-click semantics, selection, container lifecycle, drag/drop, tab closing/adding/tear-out, incremental loading and virtualization. Same-shaped basic lists/tabs remain partial adapters. |
| Menus and command bars | Text/icon/accelerator mapping, command execution, radio/toggle state, submenus, flyout ownership, placement, overflow and accessible input. |
| Rich text, ink, charts and specialized controls | Actual merged implementations and their platform/rendering contracts; another unmerged PR is a dependency rather than parity evidence. |
| Composition and animation | Shared compositor primitives, clocks, cancellation, expression inputs, theme transitions and connected-animation ownership, with native/render comparison tests. |
| Desktop/WinRT integration | Pickers, clipboard, storage, launchers, app windows/title bars, notifications, globalization, accessibility settings and WebView adapters through real host services. |

The scanner keeps imported Windows/WinRT/community namespaces distinct from
Microsoft.UI.Xaml. A portable host must report capabilities and fallible
results, preserve cancellation and asynchronous completion semantics, and run
its native integration tests. An in-memory substitute is not evidence for an
OS service.

## Acceptance and evidence

The ledger separates source presence, executable adaptations and behavioral
verification. It retains exact upstream namespaces, signatures, inheritance,
locations, file hashes, Gallery use sites and sample IDs. Overloads cannot
share a single completion flag. Private source contracts are counted separately.

Evidence for a reviewed contract names the implementation, actual tests and a
successful execution record tied to current source hashes. Tests must exercise
values, defaults, ordering, cancellation, ownership and failure behavior as
appropriate. Native construction, interaction, drawing, Windows comparison and
pixel comparison are distinct evidence levels. Font-test draw calls do not prove
that a machine has the required licensed icon font installed.

Every exported Gallery example and every actual `ControlExample` instance must
be accounted for before its page is complete. The exported catalog currently
omits five actual instances, so both inventories participate in the strict gate.
Source availability, navigation to a page and one working example cannot promote
an entire page to verified status.

Run the ordinary offline comparison and tests on every change. Run
`scan.py --check --require-complete` as the separate full acceptance gate. A
fresh report may pass while completeness fails; CI exposes those as distinct
operations. The source scanner is lexical and does not claim WinMD ABI or
Roslyn semantic binding completeness. Those are additional verification layers
required before claiming a complete SDK-compatible projection.
