# Public signatures: from the port types to std and the chosen crates (C5 of #423)

The plan of phase C5 of [idiomatic-rust.md](idiomatic-rust.md): the public signatures of the
framework take and return the std and chosen-crate types of section 2.1 of that page instead of the
ports of `System.*` types, the old types remain one release as deprecated aliases or conversions
(section 3), and every changed signature has a migration note. C2 added the conversions at the
edges (section 7.1 of that page) and changed no signature; C5 changes them. The asynchronous types
(`Task`, `TaskCompletionSource`, `SynchronizationContext`, `CancellationToken`) are phase C4 and
have their own plan, [idiomatic-rust-async.md](idiomatic-rust-async.md); the disposal handles are
C3b, [idiomatic-rust-disposal.md](idiomatic-rust-disposal.md). This page covers them only by
reference. The measures are of main at 0.5.0-preview.1 (2026-11-02).

## 1. What there is

### 1.1 How it was measured

The occurrences come from `scripts/idioms/inventory.py` (over `src/`, comments and literals
blanked). The public signatures were measured with the same blanking, over the files of `src/`
that are not tests (files under `tests/` or `testing/`, files named `..._tests.rs`, and
`#[cfg(test)]` modules are left out), with these kinds:

- **`pub fn`**: a `pub fn` (also `const`, `async`, `unsafe`) outside a trait whose parameters or
  return type name the type; `pub(crate)` and the other restricted forms are not counted. A
  `pub fn` in a private module counts too, so the column is an upper bound.
- **Trait methods**: the methods of a `pub trait` whose signature names the type.
- **Registered properties**: a `pub fn ..._property()` that returns `StyledProperty<T>`,
  `DirectProperty<O, T>` or `AttachedProperty<T>` whose value type `T` names the type. These are
  the XAML-visible properties: their value type is what markup converts text to and what bindings
  convert values to. Three direct properties of `Animation` (`Duration`, `Delay`,
  `DelayBetweenIterations`) are declared by the local macro `direct_cell_property!`
  (`src/FerroUI.Base/animation/animation.rs:93`) and were added by hand, with their six accessors.
- **`pub` fields**, **`pub const` and `pub static`** items, **`pub type` aliases** and
  **supertraits** of a `pub trait` that name the type.

### 1.2 Counts

| Type | Occurrences (files, crates) | `pub fn` takes | returns | both | Trait methods | Properties (styled, direct, attached) | Fields | Constants | Aliases, supertraits | Public total | In the type's own module | Crates |
|---|---|---:|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|
| `TimeSpan` | 1527 (212, 27) | 73 | 57 | 0 | 2 | 28 (21, 6, 1) | 1 | 4 | 2 | 167 | 6 | 9 |
| `DateTime` | 1019 (54, 14) | 47 | 68 | 7 | 1 | 13 (9, 4, 0) | 4 | 4 | 2 | 146 | 65 | 6 |
| `DateTimeOffset` | 317 (28, 8) | 16 | 30 | 2 | 0 | 8 (8, 0, 0) | 0 | 3 | 0 | 59 | 24 | 4 |
| `CultureInfo` | 1202 (254, 29) | 39 | 26 | 0 | 13 | 5 (4, 0, 1) | 3 | 0 | 0 | 86 | 12 | 7 |
| `NumberFormatInfo` | | 9 | 5 | 1 | 1 | 1 (1, 0, 0) | 0 | 0 | 0 | 17 | 6 | 2 |
| `DateTimeFormatInfo` | | 2 | 4 | 0 | 1 | 0 | 0 | 0 | 0 | 7 | 5 | 2 |
| `Uri` | 977 (178, 34) | 48 | 17 | 5 | 18 | 1 (1, 0, 0) | 7 | 0 | 0 | 96 | 5 | 8 |
| `Guid` | 465 (56, 10) | 6 | 7 | 0 | 3 | 0 | 0 | 52 | 0 | 68 | 9 | 4 |
| `Dictionary` | 239 (41, 8) | 1 | 4 | 0 | 2 | 0 | 0 | 0 | 6 | 13 | 3 | 4 |

"In the type's own module" counts the signatures of the port type itself and of its companions
(`TimeSpan::parse`, the calendar API of `DateTime`, `GregorianCalendar` and `TimeZoneInfo`, the
constructors of `CultureInfo`, `Uri` and the three `Guid` types); they change with the type, not
with its users. The `DateTime` row includes the four signatures of the BSON `DateTime` of
`ferroui-remote-protocol`, and the `Dictionary` row the three of the BSON `Dictionary`: these are
types of their own with the same names (section 1.6).

For comparison, 78 lines of `pub fn` signatures already name `std::time::Duration`.

### 1.3 Per crate

Public signatures (all kinds) per crate:

| Type | Crates |
|---|---|
| `TimeSpan` | `ferroui-base` 58, `ferroui-controls` 33, `ferroui-xaml-interactions-custom` 25, `ferrodock-controls-deferred-content-control` 19, `ferroui-xaml-interactions` 9, `ferroui-xaml-behaviors-animations` 8, `ferroui-themes-material` 8, `ferrodock-ferro` 6, `ferroui-view` 1 |
| `DateTime` | `ferroui-base` 63, `ferroui-controls` 42, `ferroui-themes-material` 33, `ferroui-remote-protocol` 4 (BSON), `ferroui-diagnostics` 2, `ferroui-dialogs` 2 |
| `DateTimeOffset` | `ferroui-base` 27, `ferroui-controls` 21, `ferroui-controls-charts` 10, `ferroui-ios` 1 |
| `CultureInfo` | `ferroui-base` 52, `ferroui-controls` 20, `ferroui-controls-treedatagrid` 6, `ferroui-themes-material` 3, `ferroui-dialogs` 2, `ferroui-markup-xaml` 2, `ferroui-pretext-media` 1 |
| `NumberFormatInfo`, `DateTimeFormatInfo` | `ferroui-base` 20, `ferroui-controls` 3, `ferroui-themes-material` 1 |
| `Uri` | `ferroui-base` 52, `ferroui-markup-xaml` 22, `ferroui-markup-xaml-loader` 8, `ferroui-controls` 6, `ferrodock-markup-extension` 3, `ferroui-designer-support` 3, `ferrodock-ferro` 1, `ferroui-markup-xaml-winui` 1 |
| `Guid` | `ferroui-win32` 53 (48 constants: COM class and interface identifiers), `ferroui-remote-protocol` 9, `ferroui-microcom` 5, `ferroui-splat` 1 |
| `Dictionary` | `ferrodock-model` 5, `ferroui-dynamic-data` 4, `ferroui-remote-protocol` 3 (BSON), `ferrodock-ferro` 1 |

The other crates use the types only inside: of the 27 crates with `TimeSpan`, 18 have no public
signature with it (`ferroui-designer` 51 occurrences, `ferroui-themes-macos` 43, the markup
crates); the 100 occurrences of `Uri` in each of `ferroui-skia` and `ferroui-vello` are font tests.

### 1.4 The types and their signatures

**`TimeSpan`** (`ferroui-base`, `animation/time_span.rs:18`: signed 100 ns ticks, `i64`).

- Animation time: `Animation::duration_property`, `delay_property` and
  `delay_between_iterations_property` (direct, `animation/animation.rs:93`),
  `TransitionBase::duration_property` and `delay_property` (direct, `animation/transition_base.rs:74`,
  `:86`), `KeyFrame::with_key_time` and `set_key_time` (`animation/key_frame.rs:58`, `:82`),
  `ClockBase::pulse` (`animation/clock_base.rs:54`), `CrossFade`, `PageSlide`,
  `Rotate3DTransition`, `TransitionInstance::new`, `MediaContext::request_animation_frame`
  (`media/media_context.rs:304`) and `TopLevel::request_animation_frame`
  (`FerroUI.Controls/top_level.rs:1239`), the animation factory of the behaviors, the
  `Transitionable::transition` trait of `FerroUI.View` (`FerroUI.View/animation.rs:26`), the
  motion and spring transitions of the Material theme, and the properties that set the duration
  of an animation they run: `FadeInBehavior::InitialDelay` and `Duration`,
  `FluidMoveBehavior::Duration` (`ferroui-xaml-interactions-custom`),
  `DelayedUniformTabPanel::ResizeAnimationDuration` (`FerroDock.Ferro/controls/delayed_uniform_tab_panel.rs:54`).
- The clock: `IClock: IObservable<TimeSpan>` (`animation/i_clock.rs:9`), a supertrait implemented
  in 9 files of 4 crates (`ferroui-base`, `ferroui-controls`, `ferroui-designer`,
  `ferroui-themes-material`).
- Timer intervals and delays, 16 registered properties: `TextBox`, `TextPresenter` and
  `RichEditBox` `CaretBlinkInterval` (`FerroUI.Controls/text_box.rs:1274`),
  `ScrollBar::HideDelay` and `ShowDelay` (`primitives/scroll_bar.rs:344`, `:351`),
  `AutoCompleteBox::MinimumPopulateDelay` (`auto_complete_box/auto_complete_box.rs:562`), eight
  properties of the behaviors (`DebounceAction::Delay`, `ThrottleAction::Interval`,
  `DelayAction::Duration` in `ferroui-xaml-interactions`; `DelayedLoadBehavior::Delay`,
  `DelayedLoadTrigger::Delay`, `DelayedShowControlAction::Delay`, `InactivityTrigger::Timeout`,
  `VisualDebugBehavior::Duration` in `ferroui-xaml-interactions-custom`), the attached
  `DeferredContentScheduling::Delay` of the dock (`FerroDock.Controls.DeferredContentControl/scheduling.rs:18`)
  and `DelayedUniformTabPanel::ExpansionDelay` (`delayed_uniform_tab_panel.rs:52`). Besides the
  properties: the settings and timeline of the deferred content control
  (`FerroDock.Controls.DeferredContentControl/settings.rs:33`, `timeline.rs:68`), and the
  durations of `ConnectedAnimationConfiguration`, `ConnectedAnimationService` and
  `SelectionIndicatorAnimation`, which drive composition animations (`Duration`).
- Calendar arithmetic: `DateTime::add` and `subtract_date_time`, `DateTimeOffset::offset`
  (`utilities/date_time_offset.rs:137`) and `to_offset`, `TimeZoneInfo::get_local_utc_offset`
  (`utilities/time_zone_info.rs:37`) and the alias `LocalUtcOffsetProvider` (`:13`),
  `TimePicker::SelectedTime` (`Option<TimeSpan>`, `date_time_pickers/time_picker.rs:273`),
  `TimePickerPresenter::Time`, the Material `TimePicker::SelectedTime` (direct).

Already `Duration`: `DispatcherTimer::interval`, `run` and `run_once`
(`threading/dispatcher_timer.rs:144`, `:204`, `:224`), the composition animations
(`KeyFrameAnimation::set_duration`, `rendering/composition/animations/key_frame_animation.rs:98`),
`AcrylicBrush::TintTransitionDuration` (`StyledProperty<Duration>`, `media/acrylic_brush.rs:479`)
and `Notification::set_expiration`, whose markup setter takes a `TimeSpan` and rejects a negative
one (`FerroUI.Controls/markup_types/plain.rs:164`).

**`DateTime`, `DateTimeOffset`** (`ferroui-base`, `utilities/date_time.rs:60`,
`utilities/date_time_offset.rs:29`). Most public signatures are calendar values: `Calendar`
(`SelectedDate`, `DisplayDate`, `DisplayDateStart`, `DisplayDateEnd`, `calendar/calendar.rs:435`),
`CalendarDatePicker`, `CalendarDateRange`, `SelectedDatesCollection`,
`CalendarBlackoutDatesCollection`, the Material `DatePicker` and its state
(`FerroUI.Themes.Material/date_picker.rs:172`); `DatePicker` and `DatePickerPresenter` with
`DateTimeOffset` (`date_time_pickers/date_picker.rs:286`); the chart axis `DateTimeAxis`
(`Minimum`, `Maximum`, `FerroUI.Controls.Charts/date_time_axis.rs:23`). A few are instants:
`IRuntimePlatform::get_utc_now` (`platform/i_runtime_platform.rs:16`), the alias `UtcNowProvider`
(`utilities/date_time.rs:36`), `StorageItemProperties::date_created` and `date_modified`
(`platform/storage/storage_item_properties.rs:29`), `FiredEvent::trigger_time` of the developer
tools (`FerroUI.Diagnostics/diagnostics/view_models/fired_event.rs:96`),
`ManagedFileChooserItemViewModel::modified` of the dialogs, the iOS storage dates.

**`CultureInfo`, `NumberFormatInfo`, `DateTimeFormatInfo`** (`ferroui-base`,
`utilities/culture_info.rs:77`: a name, an LCID, and optionally attached number and date
conventions).

- Formatting: `IValueConverter::convert` and `convert_back` (`data/converters/i_value_converter.rs:21`,
  implemented in 49 files of 13 crates), `IMultiValueConverter::convert` (14 files, 4 crates),
  `TypeConverter::convert_from` and `convert_to` of the markup (`FerroUI.Markup.Xaml/converters/type_converter.rs:79`),
  the `converter_culture` of bindings (a `pub` field, `data/core/binding_expression.rs:31`),
  `MultiBindingExpression`, `TemplateBindingExpression`, the composite format of `StringFormat`
  (`data/converters/composite_format.rs:147`), `ValueType::try_convert_with_culture`, the
  `to_string_format` of `Color`, `HslColor`, `HsvColor`, `MaskedTextBox::Culture`
  (`Option<CultureInfo>`, `masked_text_box.rs:353`), `MaskedTextProvider`, the Material
  `DatePicker::Culture`, the culture of the TreeDataGrid text columns
  (`ITextCellCultureOptions::culture`), `NumericUpDown::NumberFormat`
  (`Option<Rc<NumberFormatInfo>>`, `numeric_up_down/numeric_up_down.rs:388`), the parse and format
  functions of `Decimal` and `number_format.rs`, `ICultureDataProvider`
  (`utilities/i_culture_data_provider.rs:23`, keyed by the culture name).
- Identity (language of text, font matching, shaping): `TextElement::Language` (attached,
  `documents/text_element.rs:95`), `RichTextBlock::Language` and `RichEditBox::Language`
  (`rich_text_block.rs:848`), `CharacterFormat::language`, `FormattedText`, `TextShaperOptions`,
  `GenericTextRunProperties`, `TextRunProperties::culture_info`
  (`media/text_formatting/text_run_properties.rs:49`), `IFontManagerImpl::try_match_character`
  (`platform/i_font_manager_impl.rs:24`), `IFontFallbackMapper`, `IFontCollection`,
  `GlyphTypeface::family_names` and `face_names` (maps keyed by culture,
  `media/glyph_typeface.rs:326`), the script hints of font fallback, the text measurer of Pretext.
- The ambient culture: `CultureInfo::current_culture`, `set_current_culture`, `current_ui_culture`
  (`utilities/culture_info.rs:98`).

**`Uri`** (`ferroui-base`, `utilities/uri.rs:42`: the original text, and the canonical text of an
absolute URI; `Rc<str>`).

- Markup loading: `FerroXamlLoader::load` (`FerroUI.Markup.Xaml/ferro_xaml_loader.rs:127`),
  `RuntimeXamlLoaderDocument` (`base_uri`, a `pub` field), `XamlIlContext::base_uri`, the
  interpreter of the loader, `load_winui`; `IUriContext::base_uri` and `set_base_uri`, defined
  twice (`FerroUI.Markup.Xaml/xaml_types.rs:34` and
  `FerroUI.Markup.Xaml.Loader/runtime/interpreter/services.rs:44`); `ResourceInclude`,
  `StyleInclude` and `MergeResourceInclude` `Source` (`styling/resource_include.rs:110`), the
  `LoadExtension` of the dock and `DockPresetThemeManagerBase`.
- Assets and fonts: `IAssetLoader::exists`, `open`, `get_assets` (`platform/i_asset_loader.rs:48`),
  `FontFamily::with_base_uri`, `FontFamilyKey` (`media/fonts/font_family_key.rs:18`),
  `FontSourceIdentifier` (a `pub` field), `EmbeddedFontCollection`, `IFontCollection::key`,
  `uri_extensions` (`is_resm`, `is_font_collection`, `ensure_absolute`).
- Platform edges: `ILauncher::launch_uri_async` (`platform/storage/i_launcher.rs:13`, implemented
  in 4 crates), `IStorageItem::path` (6 crates), `IStorageProvider::try_get_file_from_path_async`
  and `try_get_folder_from_path_async` (5 crates), `uri_from_file_path` and
  `try_get_path_from_file_uri`, `ProtocolActivatedEventArgs::uri`,
  `HyperlinkButton::NavigateUri` (`Option<Uri>`, `hyperlink_button.rs:83`), the transports of the
  designer support (`html_method_listen_uri`).

**`Guid`** (three types, built on `uuid` in C1): `ferroui-microcom` (`guid.rs:12`,
`#[repr(C)]`, the COM `GUID`, with the 48 class and interface identifiers of `ferroui-win32` and
the COM functions that take them), `ferroui-remote-protocol` (`guid.rs:22`, with
`IMessageTypeResolver::get_by_guid` and `get_guid`, `i_message_type_resolver.rs:9`, and the
message identifiers), `ferroui-splat` (`application_performance_monitoring/guid.rs:11`, the
reference of a feature usage session).

**`Dictionary`** (`ferroui-system`, `collections/generic/dictionary.rs:16`): the cache of DynamicData
(`ChangeAwareCache::from_dictionary`, `dictionary`, `into_dictionary`,
`cache/change_aware_cache.rs:33`; `IObservableCache::key_values`, `cache/observable_cache.rs:56`);
the maps of the dock factory (`DockableControlMap`, `DockableObjectMap`, `ContextLocatorMap`,
`HostWindowLocatorMap`, `DockableLocatorMap`, `FerroDock.Model/core/i_factory.rs:13`, and
`DockTargetOperations`), three of which are markup types named
``System.Collections.Generic.Dictionary`2`` (`FerroDock.Model.Ferro/factory_locators.rs:128`).

### 1.5 Where markup and bindings depend on the type

The markup compiler finds the XAML type of a property by the Rust text of its value type: the
build scan (`ferroui-build-scan`) records the type of each registration, and the core table of the
compiler maps Rust paths to the core types (`core_handles`,
`FerroUI.Markup.Xaml.Loader/core_table.rs:526`). The runtime loader finds it by `TypeId`, through
the `handles` of the markup type (`FerroUI.Base/metadata/markup_type.rs:829`). A type that has no
markup metadata is opaque to markup (the test `rust_types_without_metadata_are_opaque`,
`runtime/type_system/tests.rs:367`, uses `Duration` as its example).

| Core type | Rust handles today | Text conversion | Compiled markup | Bindings and formatting |
|---|---|---|---|---|
| `System.TimeSpan` | `TimeSpan` (`markup_types/values.rs:540`), `Option<TimeSpan>` as its nullable | `TimeSpan::parse`; `TimeSpanTypeConverter` also reads a number of seconds (`0.25`) | `ferro_xaml_il_language_parse_intrinsics.rs:140`: a literal becomes `TimeSpan.FromTicks(n)`; core table `:526`, `:527`; runtime core types `core_types.rs:214`, `:248` (`Parse`) | ordering (`data/core/value_type.rs:1821`); `StringFormat` with the .NET time span formats (`composite_format.rs:105`) |
| `System.DateTime`, `System.DateTimeOffset` | `DateTime`, `DateTimeOffset` (`values.rs:559`, `:594`) | `DateTime::parse` with the invariant culture | through the markup type; the calendar theme reads `{x:Static sys:DateTime.Today}` and binds its `Day` | ordering and culture formatting (`value_type.rs:1663`, `:1821`); `StringFormat` date formats |
| `System.Globalization.CultureInfo` | `CultureInfo`, `Option<CultureInfo>` (`values.rs:603`) | `CultureInfoIetfLanguageTagConverter` (an IETF tag) | core table `:368`, `:535`; runtime `core_types.rs:228`, `:265` (`InvariantCulture`) | `ConverterCulture` of bindings; the `culture` argument of every converter; `StringFormat` |
| `System.Uri` | `Uri`, `Option<Uri>` (`values.rs:616`) | `Uri::new(.., RelativeOrAbsolute)`; `FerroUriTypeConverter`: a text that starts with `/` is relative | `parse_intrinsics.rs:438`; core table `:347`, `:531`; runtime `core_types.rs:224`, `:261`; `Known::OptionUri` of both emitters (the base URI of a `FontFamily`, `rust_emitter/emitter.rs:893`); the document URIs of compiled markup (`rust_emitter/compiled.rs:659`) | none |
| ``System.Collections.Generic.Dictionary`2`` | the dock's locator map references | none | core table `:429`, `:640` | none |

`Guid` has no markup type. Of the 16 timer properties of section 1.4, whose type changes in
section 3.2, `CaretBlinkInterval` is set in 18 markup files of the themes and samples, `HideDelay`
and `ShowDelay` in 3 each, `MinimumPopulateDelay` and `ExpansionDelay` in one each, and the
properties of the behaviors in 7.

### 1.6 Serialization and wire formats

- **The remote protocol** (`ferroui-remote-protocol`, the designer's transport) has a BSON port
  with its own `DateTime` (ticks of 100 ns since the Unix epoch, `metsys_bson.rs:557`) and
  `Dictionary` (string keys in insertion order, the BSON document, `metsys_bson.rs:634`), and its
  `Guid` is written in the .NET byte order (`Guid::to_byte_array`, which is `Uuid::to_bytes_le`).
- **COM**: the `Guid` of `ferroui-microcom` is the `GUID` structure of COM (`#[repr(C)]`), passed
  to and returned from Win32 and WinRT.
- **Hot reload and the designer** compare document URIs by text: the manifest of hot reload
  combines a base and a relative URI and keys documents by the result
  (`FerroUI.HotReload/manifest.rs:127`), include tracking does the same, and compiled markup finds
  the class of a document by its URI, ignoring case
  (`rust_emitter/compiled_resources.rs:171`).
- No `serde` implementation exists for any of these types; the dock's JSON serializer does not
  write them.

## 2. Findings

What the measurement found that a straight replacement would get wrong; each is decided in the
step named, and the ones marked O are owner decisions (section 3.9).

| # | Finding | Step |
|---|---|---|
| F1 | **Timer intervals are `TimeSpan`, timers are `Duration`.** `DispatcherTimer` and the composition animations take `Duration`; the 16 timer properties of section 1.4 hold a `TimeSpan` and convert it where they start a timer, with `to_duration()` (22 calls in `src/`). Most map a negative span to zero (`unwrap_or_default()`, for example `scroll_bar.rs:635`, `text_presenter.rs:1264`); `DebounceAction` and `DelayAction` do nothing for a negative span (`debounce_action.rs:120`, `delay_action.rs:42`); `Notification.Expiration` rejects it in markup. | T3, O2 |
| F2 | **Negative spans are part of three contracts.** Animation: a negative `Delay`, `Duration` or `DelayBetweenIterations` is representable and rejected when the animation runs (`animation/animation_instance.rs:155` to `:174`), and test clocks step backwards. Calendar: offsets from UTC and the difference of two dates are negative. Neither fits `Duration`; this is the signed newtype of section 2.1. | T1 |
| F3 | **`Duration` is opaque to markup.** `AcrylicBrush.TintTransitionDuration` is a `StyledProperty<Duration>` that markup cannot set today; a timer property moved to `Duration` without markup support would stop loading from markup. | T2 |
| F4 | **The markup compiler reads Rust paths as text.** A type alias is transparent to the run-time loader (`TypeId`) but not to the compiler: while `TimeSpan` remains as a deprecated alias, the core table maps both paths (and their `Option` forms) to `System.TimeSpan`. The same holds for each renamed type. | T1, T2, U1, L1 |
| F5 | **A property has no deprecated form.** Its `..._property()` returns one type; a typed `set_value` with the old type stops compiling. Markup, styles and bindings set values untyped and go through the markup conversion and the casts of `ValueTypes`, which have to know both types (a `{StaticResource}` of a `sys:TimeSpan`, a view model property of the old type). `ValueTypes::register_cast` is infallible (`data/core/value_type.rs:406`), so a cast from a signed to an unsigned span has to choose a value for a negative one. | T2, O2 |
| F6 | **Traits implemented in other crates cannot keep an old signature.** `IClock` (9 files, 4 crates), `IValueConverter` (49 files, 13 crates, and the applications), `IMultiValueConverter` (14 files, 4 crates), `IStorageItem` (6 crates), `IStorageProvider` (5), `ILauncher` (4), `IAssetLoader`, `IUriContext` (two definitions), `IFontManagerImpl` (4 crates: base, headless, Skia, Vello), `IRuntimePlatform` (4 crates: base, controls, Android, browser), `TypeConverter` (7 converters), `IMessageTypeResolver`, `IObservableCache`. A trait method has one signature: only a change that is a rename by alias (`TimeSpan` to `SignedDuration`) passes without a compile error; for the others the old type remains (with its conversions) and the trait changes once, with a migration note. | each step |
| F7 | **`CultureInfo` is more than a locale.** It carries an LCID and optionally attached number and date conventions (`with_number_format`, `with_date_time_format`, `utilities/culture_info.rs:262`, used by `MaskedTextProvider`, `masked_text_provider.rs:143`, and two theme galleries), and its formatting reads `NumberFormatInfo` and `DateTimeFormatInfo` from the culture data provider by name. The .NET format strings of markup (`StringFormat='{}{0:N2}'`, `{0:d}`) are interpreted by the port's engine with those conventions; ICU4X has no engine for them, and is used today only by ProTranslate and the C2 conversions. A `Locale` alone cannot replace the formatting parameter. | L2, O3 |
| F8 | **The current culture is a thread-local ambient value** (`culture_info.rs:68`), invariant by default; outside of a test nothing in `src/` sets it, although the documentation says the platform layer does. 94 reads in 56 files depend on it, and other threads always see the invariant culture. | L1 (kept as it is) |
| F9 | **Relative references are part of the markup contract.** `Source="Values.xaml"`, `Source="/Styles/Base.xaml"` and a relative font source are relative references resolved against the base URI of the document; `url::Url` holds only absolute URLs. | U1 |
| F10 | **`Uri` equality and hashing ignore the fragment** (`utilities/uri.rs:316`), as in .NET; `Url`'s do not. `FontFamilyKey` compares its source and base URI with it (`media/fonts/font_family_key.rs:38`); every other map or comparison keyed by a `Uri` is checked in the step that moves it. | U1, O5 |
| F11 | **Text forms change.** The URL standard normalizes the special schemes (`http://host` is `http://host/`, default ports and dot segments go; C2 records it), and hot reload and the designer key documents by URI text (section 1.6). | U2 |
| F12 | **`Url::to_file_path` and `from_file_path` do not exist on `wasm32-unknown-unknown`** (the browser backend); `Uri::local_path` works there today. | U4 |
| F13 | **Dictionary order.** C1 moved the internal uses that only look up; it kept the port where the order is observable, and did not touch the public signatures (the DynamicData cache and the dock factory maps). .NET's order (an entry added after a removal takes the slot of the entry removed last) is neither `IndexMap`'s `swap_remove` nor its `shift_remove` order. | X1, O6 |
| F14 | **Wire types share the names.** The BSON `DateTime` and `Dictionary` of the remote protocol are not the ports of `ferroui-base` and `ferroui-system`; the remote `Guid` has the .NET byte order, the COM `Guid` a fixed layout. | G1, D1, O6 |
| F15 | **Most dates are calendar values, not instants.** A date of a calendar control has no time zone; a `SystemTime` would need one to become a date. Only the signatures of section 1.4 that hold a moment (the clock, file times, event times) are instants. | D1, O4 |

## 3. The target

### 3.1 Rules for every type

- **Functions and methods** take and return the new type. A caller with the old type converts with
  the conversions of C2 (`.into()`, `.try_into()`); no deprecated twin of a function is added,
  because the old type itself remains one release with its conversions (section 3 of
  [idiomatic-rust.md](idiomatic-rust.md)). The migration note names the conversion.
- **Registered properties** change their value type where section 3.2 to 3.7 says so, after the
  markup and bindings know the new type as a handle of the same core type (`System.TimeSpan`,
  `System.Uri`, `System.Globalization.CultureInfo`): markup text, styles, resources and bindings
  keep working unchanged, and only typed Rust code sees the change (F5). The markup names never
  change: a renamed Rust type is declared with its markup name
  (`ferro_markup_type!(struct SignedDuration as "TimeSpan" { .. })`).
- **Traits** change once, in the step of their area; the step changes every implementation in the
  workspace, and the migration note gives the new signature (F6).
- **Deprecation forms** (section 3 of [idiomatic-rust.md](idiomatic-rust.md)):

| Old | New | Form for 0.5 |
|---|---|---|
| `ferroui_base::animation::TimeSpan` | `SignedDuration`, or `std::time::Duration` (section 3.2) | `#[deprecated]` type alias of `SignedDuration` (the same type); its .NET-named methods deprecated, each naming its replacement |
| `NegativeTimeSpanError` | `NegativeDurationError` | `#[deprecated]` alias |
| `ferroui_base::utilities::Uri`, `UriKind`, `UriFormatError` | `url::Url`, `UriRef`, `url::ParseError` | the old types remain `#[deprecated]`, with the C2 conversions and new ones to and from `UriRef` |
| `ferroui_base::utilities::CultureInfo` (identity) | `icu_locale_core::Locale` | the identity positions take `Locale` (L1); `CultureInfo` stays the formatting parameter, not deprecated (O3), made from and giving a `Locale` (L2) |
| `DateTime`, `DateTimeOffset` at an instant | `std::time::SystemTime` | the types remain with their names (calendar values, O4); the C2 conversions |
| `Guid` of `ferroui-splat` and `ferroui-remote-protocol` | `uuid::Uuid` | remain `#[deprecated]`, with the `From`/`Into` of C1 (their methods differ from `Uuid`'s, so not an alias) |
| `Dictionary` in a public signature | `HashMap` or `IndexMap` | the signature changes; the port becomes `SlotOrderMap` (O6) |

An old type that is replaced by an alias is deprecated in the step that renames it, together with
every use in the workspace (T1, as C3a did for the error types). An old type that remains as a type
of its own is deprecated in the step that moves its last public signature (U4 for `Uri`, G1 for
the two `Guid` types; `CultureInfo` stays, decision O3), when the workspace no longer needs it; the steps
before change signatures only. Each step adds its rows to `docs/porting/data/upstream-names.toml`
and its migration table to `CHANGELOG.md`.

### 3.2 `TimeSpan`: `SignedDuration` and `Duration`

The port type becomes `ferroui_base::animation::SignedDuration` (the name section 5 of
[idiomatic-rust.md](idiomatic-rust.md) already uses), with the shape of `Duration`:

```rust
/// A signed time interval with a resolution of 100 ns: animation time and calendar arithmetic.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignedDuration { ticks: i64 }

impl SignedDuration {
    pub const ZERO: Self; pub const MAX: Self; pub const MIN: Self;
    pub const fn from_ticks(ticks: i64) -> Self;  pub const fn ticks(self) -> i64;
    pub const fn from_secs(secs: i64) -> Self;    pub const fn from_millis(millis: i64) -> Self;
    pub fn from_secs_f64(secs: f64) -> Self;      // rounds to the nearest tick, as from_seconds does
    pub fn from_millis_f64(millis: f64) -> Self;
    pub fn as_secs_f64(self) -> f64;              pub fn as_millis_f64(self) -> f64;
    pub fn is_negative(self) -> bool;             pub fn unsigned_abs(self) -> Duration;
    pub fn checked_add(self, rhs: Self) -> Option<Self>; // and checked_sub, saturating_*
}
impl From<Duration> for SignedDuration { .. }            // C2: truncated to ticks, saturating
impl TryFrom<SignedDuration> for Duration { type Error = NegativeDurationError; .. }
impl FromStr for SignedDuration { .. }                   // the invariant text of TimeSpan.Parse
impl fmt::Display for SignedDuration { .. }              // the text of TimeSpan.ToString
```

The representation stays 100 ns ticks, so the tick rounding of the `f64` constructors, the range
and the text format stay as they are (section 4 of [idiomatic-rust.md](idiomatic-rust.md)); the
.NET custom format strings (`to_string_format`, used by `StringFormat`) stay, because markup writes
them.

Which type each signature takes:

| Kind | Type | Signatures |
|---|---|---|
| Animation time (F2) | `SignedDuration` | `Animation`, `TransitionBase` and the transitions, `KeyFrame`, `IClock` (`IObservable<SignedDuration>`), `ClockBase`, `CrossFade`, `PageSlide`, `Rotate3DTransition`, `TransitionInstance`, `request_animation_frame`, `Transitionable`, the animation factory of the behaviors, the Material motion; the properties that set an animation's duration (`FadeInBehavior`, `FluidMoveBehavior`, `DelayedUniformTabPanel::ResizeAnimationDuration`) |
| Calendar arithmetic (F2) | `SignedDuration` | `DateTime`, `DateTimeOffset::offset`, `TimeZoneInfo`, `LocalUtcOffsetProvider`, `TimePicker::SelectedTime`, `TimePickerPresenter::Time`, the Material `TimePicker` |
| Timer intervals and delays (F1) | `Duration` | the 16 timer properties of section 1.4 and their accessors (controls, behaviors, dock), the settings and timeline of the deferred content control, `ConnectedAnimationConfiguration::duration`, `ConnectedAnimationService::default_duration`, `SelectionIndicatorAnimation::default_duration` (composition) |

This is the split section 2.1 prescribes: the signed type only where a negative span is part of
the contract. Since `TimeSpan` is an alias of `SignedDuration`, the first two kinds compile
unchanged for users (with a deprecation warning); only the third changes a type (O1: decided, they change).

### 3.3 `DateTime`, `DateTimeOffset`

As decided in C2: `std::time::SystemTime` at the edges, the port types underneath for the
calendar, the kinds and the .NET text formats, and no new crate.

- **Instants take and return `SystemTime`**: `IRuntimePlatform::get_utc_now`, `UtcNowProvider`,
  `LocalUtcOffsetProvider` (`Fn(SystemTime) -> SignedDuration`), `StorageItemProperties`, the iOS
  storage dates, `FiredEvent::trigger_time`, `ManagedFileChooserItemViewModel::modified`. On
  `wasm32-unknown-unknown` the browser's runtime platform builds the value from `Date.now()`
  (`UNIX_EPOCH` plus a `Duration`); nothing reads `SystemTime::now()` there.
- **Calendar values keep `DateTime` and `DateTimeOffset`**: the calendar, the date pickers, the
  chart axis, the Material date picker. Their markup (`System.DateTime`, the invariant text, the
  bindings to `Day` and `Month`) does not change. The Rust names stay (O4).
- The BSON `DateTime` of the remote protocol is a wire type (section 3.7) and gains
  `From`/`TryFrom` `SystemTime`.

### 3.4 `CultureInfo`, `NumberFormatInfo`, `DateTimeFormatInfo`

- **Identity takes `icu_locale_core::Locale`**: the `Language` properties, the text formatting
  and font API of section 1.4, `GlyphTypeface::family_names` (keyed by `Locale`), the script hints.
  The invariant culture is the unknown locale `und` (C2). The markup type
  `System.Globalization.CultureInfo` gets `Locale` as a handle, parsed from an IETF tag as today.
- **The ambient culture** stays a thread-local value, as `Locale`: `current_locale()` and
  `set_current_locale()` (in `ferroui_base::utilities`) replace `CultureInfo::current_culture` and
  `set_current_culture`; who sets it does not change (F8).
- **Formatting** (converters, bindings, `StringFormat`, `MaskedTextBox`, the TreeDataGrid columns)
  keeps `CultureInfo` as its parameter (O3), reshaped on ICU4X: a `CultureInfo` is made from a
  `Locale` (`CultureInfo::from_locale`) and gives it (`locale()`), its name and LCID derive from the
  locale, and it carries the `NumberFormatInfo` and `DateTimeFormatInfo` that the .NET format
  strings of markup need, the attached ones of `with_number_format` and `with_date_time_format`
  included. `ICultureDataProvider` is keyed by `Locale`. Filling the conventions from ICU4X data
  (`icu_decimal`, `icu_datetime`) changes formatted texts and is not part of C5.

`IValueConverter`, `IMultiValueConverter` and `TypeConverter` keep `culture: &CultureInfo` (F6:
a trait parameter has no deprecated form, and `&Locale` would break every converter of every
application without a change of behaviour).

### 3.5 `Uri`: `url::Url` and `UriRef`

Two types, by what the position accepts:

- **`url::Url`** where the URI is always absolute: base URIs (`IUriContext`, `XamlIlContext`, the
  loaders, `RuntimeXamlLoaderDocument`), the platform edges (`ILauncher`, `IStorageItem::path`,
  `IStorageProvider`, `ProtocolActivatedEventArgs`, the transports of the designer support), font
  collection keys (`fonts:SystemFonts`).
- **`UriRef`** (new, `ferroui_base::utilities`) where markup can write a relative reference (F9):
  `ResourceInclude`, `StyleInclude` and `MergeResourceInclude` `Source`, the `LoadExtension` of the
  dock, the asset loader (`IAssetLoader::open(&UriRef, Option<&Url>)`), font sources
  (`FontFamilyKey`, `FontSourceIdentifier`), `HyperlinkButton::NavigateUri` (O5).

```rust
/// A URI reference (RFC 3986, section 4.1): an absolute URL, or a reference relative to a base.
#[derive(Clone, Debug)]
pub enum UriRef { Absolute(url::Url), Relative(Box<str>) }

impl UriRef {
    pub fn parse(text: &str) -> Result<Self, url::ParseError>; // the markup rule: a leading `/` is relative
    pub fn resolve(&self, base: &url::Url) -> Result<url::Url, url::ParseError>; // Url::join
    pub fn as_url(&self) -> Option<&url::Url>;
}
impl From<url::Url> for UriRef { .. }
impl fmt::Display for UriRef { .. }  // the URL's text, or the relative text as written
```

The schemes of the framework (`ferres://`, `fonts:`, `resm:`) keep their text (C2). Equality of
`UriRef`, and the keys of the maps that compare URIs, follow O5: `Url` equality, and an explicit key without the fragment where the .NET rule is needed.

Resolution (U1). `UriRef::resolve` gives the URL of `Uri::combine` for the references markup
writes (a relative path, a leading `/`, dot segments, a query, a fragment) against the bases of the
framework (`ferres://`, `file://`, `fonts:`, `resm:`), which the test `uri_ref_resolves_like_uri_combine`
states. `Url::join` cannot resolve against an opaque path (`fonts:SystemFonts`,
`resm:Fonts.Assets?assembly=App`): there `resolve` merges as `Uri::combine` does (`fonts:Other`). A
first segment with a colon and no scheme (`C:x`) is a path for `Uri`, so `resolve` joins it behind a
dot segment. Where the URL standard differs from `Uri::combine`, `resolve` follows the standard,
which is also what .NET does (`uri_ref_resolution_follows_the_url_standard_where_uri_combine_differs`):

| Reference, base | `resolve` (URL standard) | `Uri::combine` |
|---|---|---|
| `#f` against `ferres://app/dir/page.xaml?a=1` | `ferres://app/dir/page.xaml?a=1#f` (the base query stays; so for an empty reference) | `ferres://app/dir/page.xaml#f` |
| `//other/x` | `ferres://other/x` (an authority) | `ferres://app/other/x` |
| `a//b` | `ferres://app/dir/a//b` | `ferres://app/dir/a/b` (empty segments go) |
| `/Styles/Base.xaml` against `file:///C:/dir/page.xaml` | `file:///C:/Styles/Base.xaml` (the drive stays) | `file:///Styles/Base.xaml` |
| `../../x.xaml` against `file:///C:/dir/page.xaml` | `file:///C:/x.xaml` | `file:///x.xaml` |

The text of every URL is normalised by the URL standard as C2 records (F11), and the host of a
scheme that is not special keeps the case of the `Url` it was parsed from; `UriRef::parse` and the
conversions from `Uri` write the scheme and the authority in lower case, as the canonical text of
`Uri` does, so that `UriRef::parse(text)` is the reference of the `Uri` of the text.

### 3.6 `Guid`

- `ferroui-splat` and `ferroui-remote-protocol` take and return `uuid::Uuid`: the session
  references of the feature usage tracking; `IMessageTypeResolver`, the message identifiers and
  their attribute. The remote protocol writes and reads the wire layout with `Uuid::to_bytes_le`
  and `Uuid::from_bytes_le` at the serializer, which is what `Guid::to_byte_array` does today.
- `ferroui-microcom`'s `Guid` stays: it is the COM `GUID` layout, the case section 2.1 names
  ("one shared type where a layout is part of a contract"); `ferroui-win32` keeps its identifiers.
  It is kept in the inventory baseline as a deliberate declaration.

### 3.7 `Dictionary`

- The DynamicData cache (`ChangeAwareCache`, `IObservableCache::key_values`) and the dock maps
  (`DockableControlMap`, `DockableObjectMap`, the three locator maps, `DockTargetOperations`) take
  `HashMap`, or `IndexMap` where a caller enumerates and depends on the order. Which one is found as
  in C1: the tests of every user run with the port changed to insertion order and to reversed
  order. Where the exact .NET order after removals is observable, the signature keeps the port
  type, and the step says why.
- The markup types of the dock's locator maps keep the name ``System.Collections.Generic.Dictionary`2``.
- The BSON `Dictionary` and `DateTime` of the remote protocol are wire types (F14): the document is
  ordered by the wire, which a `HashMap` would lose. They keep their names (O6).

### 3.8 Async types, disposal handles and error types

- `Task`, `TaskCompletionSource`, `SynchronizationContext`, `DispatcherTask` and the cancellation
  types: the public signatures move in steps A4 to A7 of
  [idiomatic-rust-async.md](idiomatic-rust-async.md) (section 4 there has their deprecation).
- `IDisposable`: 601 public signatures still name it (285 functions return it, 224 trait methods);
  C3b decided which stay ([idiomatic-rust-disposal.md](idiomatic-rust-disposal.md), section 3: the
  Rx contract, trait methods implemented in other crates).
- `...Exception`: the three public signatures left (`Decycler::enter`, which returns
  `DecyclerException`; the deprecated `TestException` function of `ferroui-reactive-testing`; the
  closure aliases of `ActionLogger` of `ferroui-splat`) are the kept and deprecated cases of
  section 7.2 of [idiomatic-rust.md](idiomatic-rust.md).

### 3.9 Decisions

Decided on 2026-11-02 (the owner asked for the decisions to be made on the merits). Each row
states the question, the decision and why.

| # | Question | Decision | Why |
|---|---|---|---|
| O1 | Do the timer properties change to `Duration` (section 3.2), or does every `TimeSpan` property keep the signed type? | **They change**, in T3, after T2. The animation and calendar positions keep `SignedDuration` | Section 2.1 allows the signed type only where a negative span is part of the contract (F2); a timer interval has no negative meaning, every timer already takes `Duration` and the properties convert at the timer (F1, 22 `to_duration()` calls). After T2 markup, styles and bindings set them as before (`x:TimeSpan`, `"0:0:0.5"`) |
| O2 | What does a negative span do on a `Duration` property? | **It is rejected, never clamped silently.** A negative literal is a markup error: at compile time, and a load error of the run-time loader, as `Notification.Expiration` already is. A negative `SignedDuration` that reaches the property through a binding, a resource or a style setter fails its conversion: T2 adds a fallible cast to `ValueTypes` (beside the infallible `register_cast`), the binding reports a conversion error as for any value it cannot convert, and the property keeps its value (the fallback rules of the binding apply). Code cannot pass a negative value at all | Upstream rejects a negative interval (`DebounceAction`, `DelayAction`, `TimerTrigger`: an `ArgumentOutOfRangeException` from the setter), and a .NET binding catches that and reports it. A clamp to zero would turn a debounce into an immediate run without a word; a reported conversion error is the nearest Rust form of the upstream contract. No DEVIATIONS row: the behaviour of a negative interval stays "rejected" |
| O3 | What is the formatting parameter of converters, bindings and `StringFormat` (section 3.4)? | **`CultureInfo` stays the formatting parameter in 0.5**, reshaped on ICU4X: it is made from and gives a `Locale` (`CultureInfo::from_locale`, `locale()`; the name and the LCID derive from it), and it carries the number and date conventions that the .NET format strings of markup need (the attached ones of `with_number_format` and `with_date_time_format` included). `IValueConverter`, `IMultiValueConverter` and `TypeConverter` keep `&CultureInfo`. The identity positions take `Locale` (L1). `CultureInfo` is a deliberate declaration of the inventory baseline | `Locale` cannot hold the conventions (F7): a look-up by locale would lose the attached conventions that `MaskedTextProvider` and the theme galleries use, and an extra conventions argument on every converter is a second break for the same function. A trait parameter has no deprecated form (F6), so `&Locale` would break every converter of every application in 0.5 and gain no behaviour. What makes the type idiomatic is its identity (`Locale`) and its construction, which L1 and L2 change; the formatting engine stays the port's until ICU4X formatting replaces it, which changes formatted texts and is not part of C5 |
| O4 | Do the calendar types keep the Rust names `DateTime` and `DateTimeOffset`? | **They keep them**, as deliberate declarations of the baseline. Instants take `SystemTime` (D1) | They are the markup types `System.DateTime` and `System.DateTimeOffset` and the calendar API of the controls; a calendar value is not an instant (F15) and std has no calendar type. A new name would add a migration without a change of meaning |
| O5 | Equality of `UriRef` and of the keys of URI maps (F10); the type of `HyperlinkButton::NavigateUri` | **`UriRef` compares as `Url` does, with the fragment.** A map that needs the .NET rule keys by the URL without its fragment, explicitly (a helper `UriRef::without_fragment`), each one named in the step that moves it; `FontFamilyKey` is the known one. **`NavigateUri` takes `UriRef`** | Equality that agrees with `url::Url` is what a Rust reader expects, and an explicit key states the .NET rule where it matters instead of hiding it in `PartialEq`. A relative `NavigateUri` loads from markup today, and `UriRef` keeps it loading |
| O6 | The names of the `Dictionary` port of `ferroui-system` and of the BSON `Dictionary` and `DateTime` | **The port becomes `SlotOrderMap`** (the order of .NET's slots and free list), once X1 has left no public signature with it; `Dictionary` remains one release as a deprecated alias. **The BSON types keep their names** inside the remote protocol's serializer module, as deliberate declarations | A name that states the order says why the type exists beside `HashMap` and `IndexMap` (F13). The BSON types belong to the wire format of a ported serializer (F14) and are not the infrastructure types of section 2.1 |

## 4. Steps

Each step is one pull request from main, built and tested on its own; T3, U2 to U4 and L2 split by
crate or area where they are large. Every step keeps the tests it names unchanged, or changes one
on purpose with a DEVIATIONS row.

| Step | Content | Public API | Risk | Guard tests |
|---|---|---|---|---|
| S0 | This plan | unchanged | none | |
| T1 | `SignedDuration` (section 3.2); `TimeSpan` and `NegativeTimeSpanError` deprecated aliases, the .NET-named methods deprecated; every use in the workspace renamed; `ferro_markup_type!(struct SignedDuration as "TimeSpan")`; both paths in the core table (F4) | 0.5: old names deprecated, the same type | low: mechanical, about 1500 uses | the tests of `time_span.rs` (`from_seconds_rounds_to_ticks`, `parse_accepts_the_invariant_formats`, `custom_formats_match_net`, `display_round_trips`, the C2 conversion tests); `ferro_xaml_il_language_parse_intrinsics_tests::time_span`; `converters_tests::time_span_type_converter_accepts_seconds_and_time_spans`; the build-scan test of `Animation.Duration` (`FerroUI.Build.Scan/scanner/tests.rs:1288`); the animation tests (`animatable_tests`, `transitions_tests`, `animation_iteration_tests`, `page_transition_tests`); new: `the_deprecated_time_span_path_is_a_system_time_span` |
| T2 (done) | `Duration` in markup and bindings: a second handle of `System.TimeSpan` in the run-time type system, the core table and the compiled literal (`FromTicks` for a `Duration` target, the negative case per O2), `TimeSpanTypeConverter`, the casts between `SignedDuration` and `Duration` (a fallible cast for a negative span, O2), `StringFormat` and the ordering; `AcrylicBrush.TintTransitionDuration` becomes settable from markup (F3) | additions | medium: the markup compiler | new: `a_time_span_literal_sets_a_duration_property` (compiled and run-time loader), `a_negative_time_span_literal_for_a_duration_property_is_an_error`, `a_time_span_resource_sets_a_duration_property`, `a_binding_casts_between_signed_duration_and_duration`, `a_negative_span_from_a_binding_is_a_conversion_error_and_keeps_the_value`, `string_format_formats_a_duration_as_a_time_span`; `rust_types_without_metadata_are_opaque` takes another opaque type |
| T3 (done) | The timer properties and functions to `Duration` (section 3.2), by area: T3a the controls (caret blink, `ScrollBar`, `AutoCompleteBox`, the connected animations), T3b the behaviors (eight properties, `SelectionIndicatorAnimation`), T3c the dock (deferred content, `DelayedUniformTabPanel::ExpansionDelay`) | 0.5: property value types | medium: markup of the themes and samples (section 1.5); typed callers listed by the compiler | `text_presenter_tests`, `rich_edit_box_tests`, `auto_complete_box_tests` (T3a); `action_logical_tree_lifetime_tests` (T3b; the eight behaviors have no timing tests yet, so T3b writes them first, on main, before it changes a type); `delayed_uniform_tab_panel_tests` and the `control_tests` of the deferred content control (T3c); the compiled markup of every theme; new per area: a test of a negative value of each property (O2) |
| U1 (done) | `UriRef` (section 3.5) with its tests; `System.Uri` gets `Url`, `UriRef` and their `Option` forms as handles (run-time core types, core table, `FerroUriTypeConverter`, the compiled intrinsic, `Known::OptionUri`), and the conversions between them and `Uri` | additions | medium | the tests of `uri.rs` (`absolute_and_relative`, `combine_resolves_against_base`, `equality_ignores_scheme_case`, the C2 conversion tests); `ferro_xaml_il_language_parse_intrinsics_tests::uri`; the `FerroUriTypeConverter` tests; new: `uri_ref_resolves_like_uri_combine` (`ferres://`, `fonts:`, `resm:`, `file://`, dot segments, a leading `/`), `uri_ref_equality` (per O5) |
| U2 (done) | Markup loading: base URIs `Url`, sources `UriRef` (section 3.5); the two `IUriContext` traits; hot reload and the designer move with it | 0.5 | medium to high: document identity (F11) | `includes_tests` of `ferroui-markup-xaml`; the include tests of `tests/FerroUI.Markup.Xaml.UnitTests` (`resource_include_tests`, `style_include_tests`, `merge_resource_include_tests`) and of `tests/XamlIncludeFixture`; `ferro_xaml_il_compiler_tests`; the tests of `ferroui-hot-reload` (`tests.rs` and the tests of `include_tracking.rs`); `preview_stages_tests`, `workspace_navigation_file_tests` and `workspace_stage_files_tests` of the designer |
| U3 | Assets and fonts: `IAssetLoader`, `AssetLoader`, `FontFamily`, `FontFamilyKey`, `FontSourceIdentifier`, `CompositeFontFamilyKey`, `EmbeddedFontCollection`, `FontManager`, `IFontCollection::key`, `uri_extensions` | 0.5 | medium: font keys compared on hot paths | `font_collection_tests` of `ferroui-base`, Skia and Vello; `embedded_font_collection_tests` and `font_manager_tests` of Vello, `custom_font_manager_impl` of Skia; `simple_theme_graph_tests` of the dock (assets of a theme) |
| U4 | Platform edges: `ILauncher`, `IStorageItem::path`, `IStorageProvider`, the storage helpers, `ProtocolActivatedEventArgs`, `HyperlinkButton::NavigateUri` (O5), the designer transports; by platform crate (Android, iOS, browser, macOS, Win32) | 0.5: trait methods implemented in 6 crates | medium: file URLs on the browser (F12) | `storage_tests` of `ferroui-base`, `storage_misc_tests` of the controls, `managed_storage_provider_tests` of the dialogs; new: `a_file_uri_converts_to_a_path_on_every_target`, and a `HyperlinkButton` test of a relative and an absolute `NavigateUri` (none exists) |
| L1 (done) | `Locale` in the identity positions (section 3.4): text and fonts, the `Language` properties; `System.Globalization.CultureInfo` gets `Locale` as a handle; `current_locale` and `set_current_locale` | 0.5: traits of the font backends | medium: font fallback by language | the C2 tests of `culture_info.rs`; `glyph_typeface_tests`; the tests of `font_fallback_script_hints.rs`; `font_collection_determinism_tests` and `text_shaper_tests` of Vello; `imm32_rich_text_tests` of Win32 |
| L2 (done) | `CultureInfo` on ICU4X per O3: made from and giving a `Locale`, its name and LCID from the locale, `ICultureDataProvider` keyed by `Locale`, `ConverterCulture` parsed into a `Locale`; the converter traits keep `&CultureInfo` | 0.5: additions and the constructors of `CultureInfo`; no trait change | medium: the text of every formatted value is guarded by the tests | `number_format_net_tests`, `date_time_net_tests`, `composite_format_tests`; the culture tests of `value_type.rs` (`comma_culture`, `:2192`); `binding_tests_converters` of `ferroui-markup`, `binding_tests` and `compiled_binding_extension_tests` of `tests/FerroUI.Markup.Xaml.UnitTests`; `masked_text_box_tests`, `numeric_up_down_tests` |
| D1 (done) | Instants to `SystemTime` (section 3.3); the BSON `DateTime` conversions; the calendar types recorded per O4 | 0.5 | low; `SystemTime` before 1970 on the browser | the tests of `date_time_tests.rs` (C2) and `date_time_net_tests.rs`; `storage_tests`; `metsys_bson_tests`; new: `the_browser_runtime_platform_reports_utc_now_as_a_system_time` |
| G1 (done) | `uuid::Uuid` in the public signatures of `ferroui-splat` and `ferroui-remote-protocol` (section 3.6); their `Guid` deprecated; the microcom `Guid` kept | 0.5 | low | `remote_protocol_tests` and `metsys_bson_tests` of the remote protocol (the byte layout); `guid_tests` of Splat; the GUID tests of `ferroui-microcom` (`guid.rs`) |
| X1 | `HashMap` or `IndexMap` in the DynamicData and dock signatures (section 3.7), chosen by the order test of C1; the port per O6 | 0.5 | medium: the order of DynamicData change sets | the cache suites of `tests/FerroUI.DynamicData.Tests`; `factory_base_tests`, `core_options_tests` and `production_factory_tests` of the dock; `factory_locator_sdk_tests` of `tests/Dock/FerroDock.Markup.Tests` (the locator maps in markup) |
| Z | The inventory baseline (the deliberate declarations of O3, O4 and O6 (`CultureInfo`, `DateTime`, `DateTimeOffset`, the BSON types), the microcom `Guid`), `upstream-names.toml`, the migration tables, PORTING-GUIDE section Types; the samples and templates (C6 checks the application repositories) | | | `python3 scripts/idioms/inventory.py --check` |

Order: T1 first (one rename, no behaviour), then T2 before T3 (markup before any property moves),
U1 before U2 to U4, L1 before L2, D1, G1 and X1 at any time. The decisions of section 3.9 are made, so no step waits for one.

Status: S0 (this page) is merged (#506). T1 is done: the port type is `SignedDuration` and its error `NegativeDurationError`, every use in the workspace renamed (code only; the markup name `System.TimeSpan`, the .NET messages and the comments that name the .NET type stay), `TimeSpan` and `NegativeTimeSpanError` deprecated aliases, both Rust paths in the core table of the markup compiler, the time editor of the designer found by either name. The enum variants named `TimeSpan` (`FormatArg`, `KeyFrameTimingMode`, the editor kinds of the designer) keep their names. The .NET-named methods (`from_seconds`, `total_seconds`, ...) and the `Duration`-shaped ones are not part of T1: they move with T2, where the markup and the bindings learn `Duration`.

T2 is done: `std::time::Duration` is the second Rust handle of `System.TimeSpan` and `Option<Duration>` of the nullable time span, in the core types of the run-time type system and in the core table of the compiler (`::std::time::Duration` and `::core::time::Duration`, with their `Option` forms). The untyped value conversions know the two forms (`register_defaults` of `data/core/value_type.rs`): a duration casts to the time span of the same value, saturating at `SignedDuration::MAX_VALUE`; a time span casts to a duration through the new fallible cast `ValueTypes::register_try_cast`, which refuses a negative one with the error of `TryFrom` (`ValueTypes::cast_error` gives it), and a refused value is a value that does not convert: a binding reports the conversion error and its fallback rules apply (without a `FallbackValue` the property has the value it has without the binding), the run-time loader and the conversions of compiled markup (`to_exact`, `rt::exact`, `rt::cast`) report the error of the cast as the failure of the setter. Text converts to a duration as to a time span; the text, the order and the composite format of `StringFormat` of a duration are the ones of the time span. The emitter checks a time span literal for a member that declares a `Duration`: one that is not negative is passed as the duration of the same value (`rt::duration`, which cannot fail; the local of the literal and the code of a `SignedDuration` member are unchanged, so the checked-in generated files of the themes and of the corpus are too), and it refuses a negative literal with "the time span -00:00:01 is negative, and a `Duration` cannot be negative" (FRN3000 in a build) instead of stating the cast at run time. `TimeSpanTypeConverter` converts to `Duration` (`convert_to`, negative is an error). The scanner reads `register_try_cast` as a cast. `AcrylicBrush.TintTransitionDuration` is settable from markup. The guard tests are in `xaml/ferro_intrinsics_tests.rs` (run-time loader) and `emitter/differential_tests.rs` (compiled) of `tests/FerroUI.Markup.Xaml.UnitTests`, `binding_tests_converters.rs` of `ferroui-markup`, `composite_format_tests.rs` and `value_type.rs` of `ferroui-base`; `rust_types_without_metadata_are_opaque` takes `std::time::Instant`. The `Duration`-shaped methods of `SignedDuration` (section 3.2) and the deprecation of the .NET-named ones are not part of T2 either: no markup or binding depends on them.

T3 is done, in three commits by area, the timing tests of the behaviors in a commit of their own before the change of their type. The 16 timer properties of section 1.4 are `StyledProperty<Duration>` (the dock's `Delay` an `AttachedProperty<Duration>`) with `Duration` accessors and defaults, and the functions of the third row of section 3.2 take and return `Duration`: T3a the caret blink interval of `TextBox`, `TextPresenter` and `RichEditBox`, `ScrollBar::HideDelay` and `ShowDelay`, `AutoCompleteBox::MinimumPopulateDelay` (whose validation callback, the reference's `IsValidMinimumPopulateDelay`, is the type now), `DirectConnectedAnimationConfiguration::duration` and `ConnectedAnimationService::default_duration` (the managed fallback converts to the `SignedDuration` of `Animation`); T3b the eight properties of the behaviors and `SelectionIndicatorAnimation` (`default_duration`, and the duration of `try_start` and `try_start_with_visuals`, which goes to the composition animations); T3c the dock's `DeferredContentScheduling::Delay`, the four intervals of `DeferredContentPresentationTimeline` and `DeferredContentPresentationSettings` (the reveal duration converts at its transition) and `DelayedUniformTabPanel::ExpansionDelay`. The timers take the value as it is: the `to_duration()` conversions, the clamps of a negative span to zero and the panics of `DebounceAction` and `DelayAction` are gone. `DelayAction` waits forever for `Duration::MAX` and the durations as long as `TimeSpan.MaxValue` or longer, since `Timeout.InfiniteTimeSpan` (minus one millisecond) is negative: the one deviation of the step (DEVIATIONS.md, Behaviors). The animation durations keep `SignedDuration`: `FadeInBehavior::InitialDelay` and `Duration`, `FluidMoveBehavior::Duration`, `DelayedUniformTabPanel::ResizeAnimationDuration`. The markup of the themes and samples is unchanged (no literal is negative). The view builders, the designer schema and the property defaults snapshot follow the types. The guard tests: the timing tests of the behaviors (`tests/core/timer_action_tests.rs` of `ferroui-xaml-interactions`, `tests/part_a/timer_behavior_tests.rs` of `ferroui-xaml-interactions-custom`); per area, a time span literal sets each property, a negative literal is an error of the run-time loader (and of the compiler for the controls and the dock), and a negative span from a binding leaves each property at its default (`timer_interval_tests` of the controls, the two behavior files, `deferred_tests` of the dock markup tests).

U1 is done: `ferroui_base::utilities::UriRef` (section 3.5) with `parse` and `FromStr` (the rule of markup: a leading `/` is relative, a scheme has two characters or more, and the URL is the one of the canonical text of the `Uri` of the text), `resolve` (the resolution of section 3.5 and its differences from `Uri::combine`), `as_url`, `is_relative`, `without_fragment` (O5), `Display` (the URL, or the relative text as written), `Hash` and `Eq` of `Url` with the fragment, and the conversions `From<Url>`, `TryFrom<UriRef>` and `TryFrom<&UriRef>` for `Url` (`UriToUrlError::Relative` for a relative reference), `TryFrom<&Uri> for UriRef` (the error of the URL parser), `From<&UriRef>` and `From<UriRef>` for `Uri`, so that U2 to U4 can move positions one at a time. `UriRef` and `url::Url`, and their `Option` forms, are Rust handles of `System.Uri` beside `Uri`, which stays the canonical one: in the core types of the run-time type system, in the core table of the compiler (`::ferroui_base::utilities::UriRef`, `::url::Url`) and as `Known::UriRef` and `Known::Url` of both type systems of the emitter. The untyped value conversions (`register_defaults` of `data/core/value_type.rs`) cast between the three forms: a `Uri` is the reference of its text, an absolute `Uri` or `UriRef` the URL of its text, a reference and a URL the `Uri` of their text; a relative reference is refused by the casts to `Url` and a text the URL parser rejects by the casts to `UriRef` and `Url`, with their errors (`register_try_cast`), and `ValueTypes::cast_error` gives the error of the cast to a type for its nullable form too (`Option<Url>`). Both forms have their nullable forms, their text and the text conversion of `UriRef::parse`. The emitter checks a URI literal (the `new Uri(text, kind)` the compiler makes of the text) for a member that declares a `UriRef` or a `Url`: it refuses a relative one for a `Url` with "the URI Values.xaml is relative, and a `Url` cannot be relative" and one the URL parser rejects with "the URI <text> is not a valid URL (<error>)" (FRN3000 in a build), and passes the others through the cast (`rt::cast`); the compiled intrinsic of `System.Uri` itself is unchanged, and so are the checked-in generated files. The run-time loader reports the error of the cast as the failure of the setter. `FerroUriTypeConverter` converts to the two forms and their nullable forms (`convert_to`). The guard tests: `uri_ref_resolves_like_uri_combine`, `uri_ref_equality` and the tests of parsing, of the round trips and of the conversions (`utilities/uri_ref.rs`), `uris_uri_refs_and_urls_are_three_forms_of_a_uri` (`value_type.rs`), `uri_type_converter_converts_to_a_uri_ref_and_a_url` (`converters_tests.rs`), `a_uri_ref_and_a_url_are_system_uris` (run-time type system) and the build-time resolution of the paths, `a_uri_literal_sets_a_uri_ref_and_a_url_property` and `a_relative_uri_literal_for_a_url_property_is_an_error` (run-time loader in `xaml/ferro_intrinsics_tests.rs`, compiled in `emitter/differential_tests.rs`, on the test control `Linked`), `a_uri_resource_sets_a_uri_ref_and_a_url_property`, and `a_binding_casts_between_uri_uri_ref_and_url` and `a_relative_reference_from_a_binding_is_a_conversion_error_for_a_url` (`binding_tests_converters.rs` of `ferroui-markup`). No public signature changes its type: U2 to U4 move the positions, and U3 keys `FontFamilyKey` by `without_fragment`.

U2 is done, in commits by area. The base URIs of markup loading are `url::Url`: both `IUriContext` traits (`base_uri`, `set_base_uri`, the markup property `BaseUri`; the trait of the runtime library and the interpreter's), `get_context_base_uri`, `XamlIlContext` (and the context of compiled documents, `rt::create_context`), `RuntimeXamlLoaderDocument::base_uri` and its constructors, the interpreter's `RuntimeDocument`, `FerroRuntimeXamlLoader::load` and `load_stream`, `DocumentGroup` (its document and its included documents), `load_winui`, and the preset URIs of the dock's `DockPresetThemeManagerBase`. The include sources are `UriRef`: `ResourceInclude`, `StyleInclude` and `MergeResourceInclude` `Source` (their constructors take an `Option<Url>`), the dock's `LoadExtension` `Source` and `with_source`, and `FerroXamlLoader::load`, `load_with_service_provider` and `try_load_with_service_provider` (a `&UriRef` against an `Option<&Url>`). Markup sets them from text through the handles of U1, so no document changes. A trait method has one signature (F6), so the traits changed once, with every implementation in the workspace (the runtime library, the interpreter, the test service providers of the markup tests, the include fixture and the dock tests); the migration note names the conversions. New: `UriRef::parse_absolute`, the URL of a document with the scheme and the authority in lower case, which every place that reads a base URI or a document key uses, so that two spellings of one document are one URL as they were one canonical `Uri` (`Url::parse` keeps the case of the host of a scheme that is not special).

Resolution is `UriRef::resolve` everywhere a source is resolved: `FerroXamlLoader` (which gives the asset loader the resolved URL, the port `Uri` until U3), the include sources' development source and overlay requests, the documents the run-time loader collects for a group, the build-time `class_document_group` and the compile-time `FerroXamlIncludeTransformer`, hot reload's manifest, watcher and include dependencies. Of the five differences from `Uri::combine` (section 3.5), the one a test meets is the drive of a `file:` URL: a rooted include against `file:///C:/dir/page.xaml` is `file:///C:/Styles/Base.xaml` (`include_sources_resolve_against_a_url_base`, `a_document_reached_by_two_spellings_is_one_document`); the others need a reference no document of the workspace writes (`#f` with a base query, `//host`, `a//b`).

The maps keyed by a URI (F11), each keyed by the text of the URL now: the session of hot reload (accepted, published and pending sources, `document_key`), its manifest (files, sources, dependencies), its watcher's discovered files and the include dependencies (`include_key`, the text a source resolves to); the source overlays of the designer and of hot reload (keyed by the text the loaders request); the visited documents of a run-time group and the build-time class group; the designer's identities of a document (`replace_document`, the preview stages' source keys, the factory reload target, the animation runtime). Every one of them was a map of texts (the canonical text of a `Uri`), so none relied on the fragment-insensitive equality of `Uri`, and none keys by `without_fragment` (a document URI has no fragment); `FontFamilyKey`, the known one, is U3. The compiled loaders find a document by its URI ignoring case, as before. One assertion follows the type: the base URI of a compiled document's context is the URL of its canonical text (`ferres://assembly/Views/Main.xaml`), where the test read the `original_string` of the `Uri` (`the_context_has_every_service_of_the_framework_language`). The test asset loader of `ferroui-markup-xaml` finds the registered assembly of an authority ignoring case, as the standard asset loader does, since the URL of a reference has its authority in lower case.

The font family keeps the port `Uri` (U3): its markup constructor takes the `Option<Url>` of the context and converts at the boundary, so the emitter writes `context.base_uri()` with the `Url` type and the same text, and the compiled theme fixtures do not change; the converters that call the asset loader and the font family (`BitmapTypeConverter`, `IconTypeConverter`, `FontFamilyTypeConverter`) convert the base URI at their boundary too. `XamlSourceInfo::source_uri` and the platform edges are not markup loading and keep `Uri` (U4). The designer schema names the include sources `UriRef`; the view builders and the property defaults snapshot do not change. The guard tests: `includes_tests` with the new `include_sources_resolve_against_a_url_base` (a relative, a `ferres://` and a rooted source against `Url` bases); the include tests of `tests/FerroUI.Markup.Xaml.UnitTests` and `tests/XamlIncludeFixture`; `ferro_xaml_il_compiler_tests`; the tests of `ferroui-hot-reload` with the new `a_document_reached_by_two_spellings_is_one_document`; `preview_stages_tests`, `workspace_navigation_file_tests` and `workspace_stage_files_tests` of the designer; the new `uri_ref_parses_an_absolute_url_as_a_base` of `uri_ref.rs`.

D1 is done, in three commits: the instants of section 3.3 are `std::time::SystemTime`. The trait method `IRuntimePlatform::get_utc_now` returns it (the default is `UNIX_EPOCH`), and its implementations change with it: `StandardRuntimePlatform` of the base library (`SystemTime::now()`, the epoch on `wasm32-unknown-unknown`), Android and the browser; the mock of the controls and the test platforms of the markup take the default. A trait method has one signature (F6), so the old form has no deprecated method: the C2 conversions (`DateTime::try_from`, `SystemTime::try_from`) are its migration. `UtcNowProvider` returns a `SystemTime` and `LocalUtcOffsetProvider` takes one (`TimeZoneInfo` asks it at the instant of a date, at the epoch where the platform's `SystemTime` cannot hold the date). The new `ferroui_base::utilities::system_time_now()` is the current instant (the pinned source, else the registered runtime platform, else the standard one), and `DateTime::utc_now` is its date, saturating at `MIN_VALUE` and `MAX_VALUE`. `StorageItemProperties` holds `Option<SystemTime>`, built as instants by the file system storage of the base library, Android, the browser and iOS (`date_from_time_interval_since_1970` returns a `SystemTime`); `FiredEvent::new` and `trigger_time` of the developer tools take and return a `SystemTime` (read with `system_time_now`), `ManagedFileChooserItemViewModel::modified` and `set_modified` of the dialogs an `Option<SystemTime>`. The markup properties `TriggerTime` and `Modified` stay `System.DateTime` in local time, through private accessors, so the views of the developer tools and of the file chooser show what they showed. On WebAssembly the runtime platform of the browser reads `Date.now()` of the page (`TimerHelper.dateNow` of the script module) and makes `UNIX_EPOCH` plus the milliseconds; nothing on the browser reads `SystemTime::now()`. The BSON `DateTime` of the remote protocol keeps its name (O6) and gains `TryFrom<DateTime> for SystemTime` beside its `From<SystemTime>`; its wire layout is unchanged. The calendar values keep `DateTime` and `DateTimeOffset` (O4). The guard tests: `date_time_tests.rs` and `date_time_net_tests.rs`, whose pins of the clock return the instant of their date (the provider's type), and the new `the_clock_and_the_local_offset_are_instants`; `storage_tests`, and the storage test of the file system, which compares system times; `metsys_bson_tests` with the new `date_time_converts_to_and_from_system_time`; the new `the_browser_runtime_platform_reports_utc_now_as_a_system_time`; the date tests of the Android, browser and iOS storage.

G1 is done, in two commits: `uuid::Uuid` is the identifier type of the public signatures of `ferroui-remote-protocol` and `ferroui-splat` (section 3.6). The remote protocol re-exports it (`ferroui_remote_protocol::Uuid`); the trait methods `IMessageTypeResolver::get_by_guid` and `get_guid` take and return it (with `DefaultMessageTypeResolver`, the one implementation; a trait method has one signature, so there is no deprecated form, and a caller with a `Guid` converts with `Uuid::from`), and so do `FerroRemoteMessageGuidAttribute::guid`, the `GUID` constant of every message class, the BSON values `Value::Guid` and `ValueRef::Guid` and a property of type `Uuid`. The wire layout is unchanged: the serializer and the header of a message write `Uuid::to_bytes_le` and read `Uuid::from_bytes_le` (a length other than sixteen is still the `ArgumentException`), and the identifiers of the message classes are parsed with the forms of `Guid.Parse` as before. In Splat the references of the feature usage tracking are `Uuid` (`IFeatureUsageTrackingSessionOf<Uuid>`). The two `Guid` types remain one release, `#[deprecated]`, with their methods and `From` both ways (their methods differ from those of `Uuid`, so they are not aliases), and a BSON property of the deprecated type still serializes. Every use in the workspace moved: the designer support and its tests, and `ferroui-reactiveui`, whose `Guid` converters (`GuidToStringTypeConverter`, `NullableGuidToStringTypeConverter`, `StringToGuidTypeConverter`, `StringToNullableGuidTypeConverter`) convert `Uuid` and `Option<Uuid>` with the same texts, and whose `EqualityTypeConverter` compares `Uuid` (and the deprecated Splat type until it is removed). `ferroui-microcom`'s `Guid` stays, the COM layout, a deliberate declaration of the inventory baseline (its count is unchanged). The guard tests: `metsys_bson_tests` with `Uuid` and the same bytes, with the new `a_property_of_the_deprecated_guid_has_the_layout_of_a_uuid`; `remote_protocol_tests` and the tests of `assembly.rs`; the tests of the port type in `guid.rs` of the remote protocol and `guid_tests` of Splat, unchanged with the deprecation allowed, and the new `the_identifiers_of_the_protocol_are_uuids_with_the_forms_and_the_layout_of_guid`; the GUID tests of `ferroui-microcom`, untouched; the converter tests of ReactiveUI and the feature usage tests of Splat with `Uuid`.

L1 is done, in commits by area: the ambient locale and the casts, the text and font API, the `Language` properties, markup, the moved uses of the current culture, and the generated files. The identity positions of section 1.4 take and return `icu_locale_core::Locale`, re-exported with its `locale!` macro as `ferroui_base::utilities::Locale`: `TextRunProperties::culture_info`, `GenericTextRunProperties`, `TextShaperOptions`, `FormattedText`, the font matching of `IFontManagerImpl`, `IFontFallbackMapper`, `IFontCollection` and `FontCollectionBaseImpl` (every implementation changed with them: the base library and its test managers, Headless, Skia, Vello, HarfBuzz, the Pretext measurer, the test managers of the backends), `GlyphTypeface::family_names` and `face_names` (keyed by the locale; a name record whose language identifier names no locale, an unknown one or `es-ES_tradnl`, is the invariant culture's, as an unknown one was), `declares_language_coverage`, the script hints of the fallback (`Bcp47ScriptResolver` reads the subtags of the locale, `parent_locale` gives the parent `CultureInfo::parent` gave), and the `Language` properties (`TextElement`, `RichTextBlock`, `RichEditBox`, `CharacterFormat`). The invariant culture is `und`; the backends pass the language text they passed (`iv` for HarfBuzz, an empty tag for Skia and Vello), so the fallback of a locale gives the font the culture gave (the guard tests of the row and the fallback tests of the base library, Skia and Vello pass the same locales, unchanged in their results). The one deviation: an XML language tag that is no BCP-47 tag (`x-private`) is not a language any more (DEVIATIONS.md, Text block). The decimal tab stop of a run reads the decimal separator of the culture data of its locale; a run has no attached conventions.

The ambient culture is one thread-local value with two views: `current_locale()` and `set_current_locale()` (in `ferroui_base::utilities`) hold the locale of the thread, and `CultureInfo::current_formatting()` the culture it formats with, the culture of the locale or the culture with attached conventions that `CultureInfo::set_current_formatting()` sets (the locale is then the locale of that culture, `und` for a name that is no BCP-47 tag). A locale cannot hold the attached conventions the theme galleries set (decision O3), so the culture is kept beside the locale instead of being made from it on every read, which is also one reference count per read where a conversion would parse a name. The user interface locale is `current_ui_locale()` and `set_current_ui_locale()`. `CultureInfo::current_culture`, `set_current_culture`, `current_ui_culture` and `set_current_ui_culture` are deprecated and implemented over them; every use in the workspace moved (formatting to `current_formatting`, identity to the locales). Who sets them did not change: the application.

In markup, `Locale` and `Option<Locale>` are handles of `System.Globalization.CultureInfo` in the run-time core types, the core table (by `::icu_locale_core::Locale` and `::ferroui_base::utilities::Locale`), and the emitter (`Known::Locale`, both type systems). Text converts to the culture of the IETF tag and casts to its locale: `ValueTypes` casts a locale to the culture of its name (`register_cast`) and a culture to its locale (`register_try_cast`, refusing a name that is no BCP-47 tag with the error of the cast), and converts text to a locale and to its nullable form. A tag whose culture is no locale is an error of the run-time loader and of the compiler (which remembers the culture literals of a document, as it remembers the time span literals). The guard tests: `a_language_literal_sets_a_locale_property` and `an_invalid_language_tag_for_a_locale_property_is_an_error` (run-time loader and compiled), `a_binding_casts_between_culture_info_and_locale`, `a_culture_that_is_no_locale_from_a_binding_is_a_conversion_error_and_keeps_the_value`, `cultures_and_locales_are_two_forms_of_a_culture`, `current_locale_round_trip`, `a_formatting_culture_keeps_its_attached_conventions_and_sets_the_locale`, `a_locale_is_a_system_globalization_culture_info`. The view builders and the designer schema follow the types.

L2 is done, in commits by area: the culture and its provider in the base library, the provider of the tests of the controls, and markup. A `CultureInfo` is made from a `Locale` and gives it: `CultureInfo::from_locale` (and `From<Locale>` beside the `From<&Locale>` of C2), `locale()`. The identity of a culture (its locale, its name, its Windows LCID) is one shared value, beside the attached conventions, so a clone is still a reference count per part. The name of a culture of a BCP-47 tag is the text of its locale (`EN-us` is `en-US`, as .NET normalizes it; the variants in the order of ICU4X), its LCID the one of its locale, `und` is the invariant culture, and its parent is the culture of the parent of its locale (`parent_locale`, which L1 made). A name that is no BCP-47 tag (`es-ES_tradnl`, `x-custom`, the test cultures `x-day-first` and `x-year-first`) still gives a culture through `get_culture_info`, with its name as written, the LCID of the name and no locale of its own: its `locale()` is `und`, `TryFrom<&CultureInfo> for Locale` refuses it as before, and its parent is the culture of its name without the last subtag (`es-ES`), as it was. `CultureInfo` is not deprecated (O3; the row of `CultureInfo` in the table of section 3.1 says so). `ICultureDataProvider` is keyed by `Locale` with additions, no trait change: `get_date_time_format_by_locale`, `get_number_format_by_locale`, `get_compare_rules_by_locale`, `get_text_info_by_locale` and `get_locales`, whose defaults ask the methods keyed by name with the text of the locale; those are deprecated until the release after 0.5, so a provider written against the names keeps working. The search for the data of a culture asks the provider for its locale and the parents of the locale; a culture without a locale is asked for by name (the deprecated methods) until a parent is a locale. The provider of the tests (`TestCultureDataProvider`) and the providers of the tests of the base library and of the controls compare locales. `ConverterCulture` is parsed into a locale: the converter of an IETF tag (`CultureInfoIetfLanguageTagConverter::convert_from`, the `parse:` of `System.Globalization.CultureInfo`, and the new `convert_to_locale`) and `get_culture_info_by_ietf_language_tag` parse the tag into a `Locale` and give the culture of the locale; a text that is no BCP-47 tag is refused with the error that refused `zh-CHT` (`x-private` and `en_US` too, which made cultures before: a row of DEVIATIONS.md, Bindings). The compiler refuses such a literal for a `Locale` property with that error (the second error of L1, a culture that is no locale, cannot happen for a parsed tag any more). The converter traits keep `&CultureInfo`. The text of every formatted value is unchanged: the guard tests of the row pass the same cultures, whose conventions the provider gives for their locales as it gave them for their names. The tests: `a_culture_is_made_from_and_gives_its_locale`, `the_name_of_a_language_tag_is_the_text_of_its_locale`, `a_name_that_is_no_language_tag_keeps_its_name_and_has_no_locale`, `an_ietf_language_tag_is_parsed_into_a_locale` (`culture_info.rs`), `the_provider_is_asked_for_the_locale_and_its_parents` and `a_provider_keyed_by_name_keeps_working` (`i_culture_data_provider.rs`), `a_language_tag_is_parsed_into_a_locale` and the refused tags of the converter (`culture_info_ietf_language_tag_converter.rs`), `converter_culture_is_parsed_into_a_locale` (`binding_tests` of `tests/FerroUI.Markup.Xaml.UnitTests`); `an_invalid_language_tag_for_a_locale_property_is_an_error` (run-time loader and compiled) expects the error of the converter for `x-private`.

## 5. Not in scope

- The asynchronous and cancellation types (C4, [idiomatic-rust-async.md](idiomatic-rust-async.md))
  and the disposal handles (C3b, [idiomatic-rust-disposal.md](idiomatic-rust-disposal.md)).
- Formatting with ICU4X data instead of the port's conventions, and who sets the current culture
  (section 3.4): both change texts the user sees, and get their own plan with tests of the texts.
- The calendar arithmetic of `DateTime`, `DateTimeOffset`, `GregorianCalendar` and `TimeZoneInfo`
  (it stays, C2), and a time zone database.
- The markup names (`System.TimeSpan`, `System.Uri`, `System.DateTime`,
  `System.Globalization.CultureInfo`, ``System.Collections.Generic.Dictionary`2``) and the text
  formats of markup.
- The internal uses of the port types that no public signature exposes (C1 moved the internal
  `Dictionary` uses; the others follow their type when it is removed after 0.5).
- The COM `Guid` of `ferroui-microcom` and the identifiers of `ferroui-win32`, and the wire formats
  of the remote protocol, which stay byte for byte.
