# Third-party notices

The current native dependency inventory is [docs/dependency-audit.json](docs/dependency-audit.json): selected FerroUI 0.5.0-preview.1 at `b00c16c91bfda60cb4bbfd8ed99a51fa2019120b`, based on reviewed main `8a35e0c5fe2214311e441845fef6e363b049d49e` plus five focused native fixes. Its MIT license is retained with the patch series. Registry versions/checksums and full license bytes are recorded in that inventory. The `jni-sys-macros` 0.4.1 archive omits its repository-root licenses; both MIT and Apache-2.0 texts are retained from packaged VCS commit `64d77b7a5f119d7b55b4e2c169a4668067ff59e6`, with exact Git blob provenance in [supplemental licenses](docs/supplemental-dependency-licenses.json).

Clipboard local-time imports use Jiff0.2.38 (Unlicense OR MIT), with exact transitive
versions and retained license files in the dependency audits. Eight deterministic
TZif fixtures are IANA2026e-rearguard public-domain data; their provenance and
notice are in tests/fixtures/clipboard-time-zones/. Production uses native system
zone data and Jiff's selected platform fallback database, not those test fixtures.

The upstream component table below describes the original Uno implementation. FerroBoard uses FerroUI and native Rust readers/font collections. The old managed Task/Stream font adapter and its `UseNotoColorEmoji` installation API are retired; native applications supply local or already downloaded font bytes. No Noto font is bundled or downloaded by default. Its applicable font license must accompany any application that redistributes it.

The native iOS vibration mapping follows Uno.WinUI 6.7.135 at
`da217df2312e9e39676880f9cb4b8d519cc297e0`. Unchanged platform reference sources,
Apache-2.0 license and original third-party notices are retained in
`reference/uno-haptics/`; native bundle notice archives include those full notices.
Apple AudioToolbox is supplied by the operating system and is not redistributed.


Android native locale comparison/search uses the platform's `android.icu` API
through `jni-sys` and `jni-sys-macros` 0.4.1 (MIT OR Apache-2.0). Original license
texts and checksums are retained under reference/rust-dependencies and in
docs/android-jni-provenance.json. Android ICU code/data is supplied by the OS
and is not redistributed by this backend.

The separate FerroBoard iOS launcher uses the pinned FerroUI iOS platform and
the objc2 family of native Apple bindings. Exact versions, registry checksums,
declared licenses and retained notices are in docs/ios-dependency-audit.json;
docs/ios-license-provenance.json records the original license files at each
published crate's source revision. Xcode SDK files are not redistributed.

ScreenKeyboard is licensed under the [Apache License 2.0](LICENSE). It builds on the following third-party work.

| Component | Used for | License |
|---|---|---|
| [FlorisBoard](https://github.com/florisboard/florisboard) | Layouts (90+), popup mappings (55+), subtype presets (70+), composers, currency sets, English dictionary, emoji data, glide typing classifier (ported), layout algorithms (ported), theme palettes | Apache-2.0 |
| [Unicode CLDR](https://cldr.unicode.org/) | Emoji names, keywords and categories (via FlorisBoard) | Unicode License v3 |
| [Material Symbols](https://github.com/google/material-design-icons) | Key and panel icons (converted to path data) | Apache-2.0 |
| [Noto Color Emoji](https://github.com/googlefonts/noto-emoji) | Optional emoji font downloaded at runtime by `EmojiFontFallback.UseNotoColorEmoji()` (not redistributed) | SIL Open Font License 1.1 |
| [Uno Platform](https://platform.uno) | UI framework | Apache-2.0 |

The FlorisBoard license text is included in the NuGet package under `licenses/florisboard/LICENSE`.

## FerroBoard additions

The additional native `PaddedStackPanel` adapts zero-border StackPanel geometry from
Uno.WinUI 6.7.135 at `da217df2312e9e39676880f9cb4b8d519cc297e0` (Apache-2.0),
including Microsoft UI XAML StackPanel methods marked MIT, reference commit
`9d6fb15c089dadebac401ccd858d2cf0f969248b`.
Copyright (c) Microsoft Corporation. All rights reserved.
The unchanged files, Uno notices and complete Apache/MIT licenses are retained in
[reference/uno-layout](reference/uno-layout); package resolution, source hashes and
explicit algorithm limitations are in [docs/uno-layout-provenance.json](docs/uno-layout-provenance.json).

The sample's Slider/ComboBox/ToggleSwitch header composition also consults the selected Uno
Version2 templates and merged resource dictionary, marked Microsoft MIT with MUX
reference tag `winui3/release/1.5.2`. Supporting control/automation references retain
their original MIT/Apache notices, including MUX tag `winui3/release/1.8.4` where marked.
The exact Uno source files/Git blobs and declarative verification scope are recorded
in the same provenance manifest. Full source runtime/header API parity is not implied.

The UTF-16 grapheme segmentation bridge is adapted from the .NET runtime's
`TextSegmentationUtility.cs` at commit `60629d14374c56f1cb51819049ad1fa529307f8d`.
Copyright (c) .NET Foundation and Contributors. Licensed under the MIT License;
the complete license is preserved in [reference/dotnet-runtime/LICENSE.TXT](reference/dotnet-runtime/LICENSE.TXT).
The original ScreenKeyboard notice remains unchanged in the reference checkout.

Managed character/grapheme properties and conformance inputs are derived from Unicode 16.0.0.
Unicode data carries © 2024 Unicode, Inc. notices. The preserved Unicode License v3 carries
Copyright © 1991–2026 Unicode, Inc.; the complete license is preserved in
[reference/unicode-16.0.0/LICENSE.txt](reference/unicode-16.0.0/LICENSE.txt).
Exact source URLs, hashes, runtime provenance and known algorithm differences are recorded in
[docs/unicode-provenance.json](docs/unicode-provenance.json).

The ordinal casing bridge also consults the .NET runtime's `OrdinalCasing.Icu.cs` and `pal_casing.c`
at the same recorded commit and MIT license. Its generated supplementary mappings use the preserved
Unicode 16.0.0 data; its BMP mappings are measured runtime API outputs.

The dictionary's reentrant monitor uses parking_lot 0.12.5, parking_lot_core 0.9.12, lock_api 0.4.14
and scopeguard 1.2.0 (MIT OR Apache-2.0). The resolved platform graph also includes bitflags 2.13.2
and windows-link 0.2.1 (MIT OR Apache-2.0), and redox_syscall 0.5.18 (MIT). Original license texts and
copyright notices are preserved under [reference/rust-dependencies](reference/rust-dependencies).
Exact registry checksums, license paths and file hashes are in [docs/dependency-audit.json](docs/dependency-audit.json).

The strict persistence JSON bridge consults `JsonDocument.TryGetProperty.cs` at the same recorded
.NET runtime commit and MIT license. Its original source and license are retained in
[reference/dotnet-runtime](reference/dotnet-runtime); the exact hash is in the provenance manifest.

The language registry's lazy-publication bridge consults `System/Lazy.cs` at the recorded .NET runtime
commit under the same MIT license. The original source, license and provenance hash are retained with
the other runtime references.

Clipboard IDs use uuid 1.18.1 (MIT OR Apache-2.0) with OS randomness via getrandom 0.3.4. Exact resolved
platform dependencies, declared licenses and registry checksums are recorded in docs/dependency-audit.json;
original notices are preserved in reference/rust-dependencies. r-efi's MIT license is selected and its
complete AUTHORS notice is preserved. Target-specific dependencies are not asserted platform-tested.

The native FerroBoard sample uses the pinned FerroUI desktop, Fluent and ReactiveUI components
under their retained MIT and component notices, including the native backend's upstream notices.
Copies are under reference/ferroui-licenses. Native rendering links rust-skia/skia-safe and
skia-bindings0.153.3 (Rust bindings MIT; native Skia BSD-style license retained as LICENSE_SKIA),
with HarfBuzz and the locked platform/build graph. Registry license files and exact supplemental
repository-root license URLs/commits/hashes are recorded in dependency-audit.json and
supplemental-dependency-licenses.json. Complete native embedded third-party redistribution auditing
is still pending; the generated .app is a local development bundle. Platform fonts are not bundled.

The explicit macOS collation bridge adapts ICU option/rule handling from dotnet/runtime v10.0.13,
commit6aa72daa24f66c01c3d7b6a32519e9d21396deec, under the MIT license. Original algorithm references
and the complete license are retained in reference/dotnet-runtime-collation. It links the operating
system's ICU library; the system library/data are not redistributed. Upstream ICU78.1 ABI header
references and their complete Unicode/third-party license are retained in reference/icu-78.1.
Exact source hashes, measured native versions and remaining Apple-build/platform provenance limits
are in docs/collation-provenance.json. Managed Unicode16 tables and native ICU Unicode17 data are
distinct resources and are not substituted for one another.

The same tagged runtime/MIT reference set also supplies the default locale bridge
(pal_locale.c, pal_locale.m and CultureData.Icu.cs). NSLocale/Foundation and Objective-C are system
services linked on macOS; those system binaries are not redistributed. ICU uloc.h and its retained
license document the locale buffer capacities and ABI. Exact hashes remain in collation-provenance.json.

The existing .NET/ICU culture collation/default-locale adaptation is now located in
`src/FerroBoard.Core/text/culture_collation`; the SDK retains its public re-export. The complete
retained runtime MIT license is also included in Core NOTICE. Exact runtime/ICU source versions,
system-library use and redistribution limits remain as recorded above and in collation-provenance.json.


The composer ownership fixture derives rule/cached-order data from the unchanged Apache-2.0
ScreenKeyboard Core and its bundled FlorisBoard composer extension0.1.1 (all2866 Telex rules).
No new font/data dependency or resource bytes are introduced. Native storage reuses the already
selected FerroUI.System Dictionary and locked parking_lot under their retained notices. Exact
source, framework and fixture hashes and limits are in docs/composer-ownership.json.

The isolated FerroUI dependency patch also changes the Rust XamlX transformer under
MIT, Copyright (c) 2019 Nikita Tsukanov. Its complete original notice and license are
retained in [reference/ferroui-licenses/XAMLX-NOTICE.md](reference/ferroui-licenses/XAMLX-NOTICE.md).
Exact changed-file hashes and the reviewable patch are in
[docs/ferroui-header-template-patch.json](docs/ferroui-header-template-patch.json).

The retired DefaultPredictions enumeration bridge derived LINQ Take/disposal ordering from
System.Linq Take.cs and SkipTake.SpeedOpt.cs at .NET runtime60629d14374c56f1cb51819049ad1fa529307f8d
(v10.0.0), Copyright (c) .NET Foundation and Contributors, MIT. Sources, full license and exact
provenance are retained in reference/dotnet-runtime-prediction. Observed servicing runtime output
is tested separately; no exact servicing-source identity or complete LINQ parity is claimed.

The font output stream adapter derives selected origin-zero,readonly,non-exposable MemoryStream
operations and Stream buffer-argument validation from dotnet/runtime60629d14374c56f1cb51819049ad1fa529307f8d
(v10.0.0), Copyright (c) .NET Foundation and Contributors, MIT. Source and the complete license are
retained in reference/dotnet-runtime-evaluator/{MemoryStream.cs,Stream.cs,LICENSE.TXT}. Current
.NET10.0.13 behavior is measured separately; exact servicing-source identity and complete Stream
parity are not claimed. Exact provenance/limits are recorded in docs/font-stream.json.

The interaction-focus policy references Uno.WinUI6.7.135 at da217df2312e9e39676880f9cb4b8d519cc297e0,
the default resolved by upstream Uno.Sdk6.7.30. Retained FrameworkElement source is under the Uno
Apache-2.0 license;FocusSelection/FocusManager ported WinUI sources retain Microsoft copyright/MIT
headers. Sources are in reference/uno-focus;full licenses remain in reference/uno-layout/LICENSE
and reference/uno-layout/WinUI-LICENSE. Exact hashes,scope and runtime limits are in docs/focus-interaction.json.

The focus tab-stop hierarchy audit retains Panel, Border, Image, Shape, Skia TextBlock and
UIElement.FocusMixins from the same Uno revision in reference/uno-focus. Their original source
bytes and notices remain intact; full Uno Apache-2.0 and WinUI MIT licenses remain at the paths
above. The test host models selected type relationships and is not a redistributed Uno runtime.
Exact hashes and the limited comparison scope are recorded in docs/focus-tab-policy.json.

The traversal audit retains unchanged Uno UIElement.crossruntime, UIElementCollection shared/
crossruntime partials, and Uno.UWP MaterializableList at the same pinned Uno revision. Full licenses
remain at the paths above. The test host links the actual collection sources; owner/property-store
services are local stubs. Original Git blobs, SHA-256 values and limits are in docs/focus-traversal.json.

The additional focus role audit retains Uno ContentPresenter, ItemsPresenter, ScrollContentPresenter,
Viewbox and generated Popup source at the same pinned revision. Original headers and exact Git blobs
are retained. Viewbox keeps its Microsoft MIT header; complete Apache-2.0/MIT texts remain at the
paths above. The scope, test-host limits and source paths are recorded in docs/focus-framework-roles.json.

The single-cast clock delegate equality/hash audit consults unchanged .NET Delegate.CoreCLR.cs
and MulticastDelegate.CoreCLR.cs at60629d14374c56f1cb51819049ad1fa529307f8d (.NET10.0.0). They retain
Microsoft MIT headers; full license is reference/dotnet-runtime-evaluator/LICENSE.TXT. Observed
10.0.13 reference behavior is checked independently; exact servicing-source identity is not claimed.
Source hashes, formula, native mapping and remaining delegate gaps are in docs/clock-delegate-identity.json.

## Native font test fixture

`tests/fixtures/fonts/Inter-Regular.ttf` is the unmodified Inter 3.019 font from the pinned
FerroUI.Fonts.Inter assets. Copyright 2020 The Inter Project Authors. Licensed under
SIL Open Font License 1.1; full notice/license: `tests/fixtures/fonts/NOTICE.md`.
The font is a local test fixture and is not downloaded or installed system-wide.

## Native locale discovery and display-text services

FerroBoard uses [sys-locale 0.3.2](https://github.com/1Password/sys-locale)
(MIT OR Apache-2.0) for OS-preferred locale discovery. Both complete licenses are
retained in `reference/rust-dependencies/sys-locale-0.3.2/`. Native display-list sorting
and filtering use system ICU on macOS/Linux and Windows NLS on Windows. Linux builds
require ICU development files; its runtime must provide the selected ICU ABI. These
paths do not use the historical .NET PAL rule adaptation. Original compatibility
references and their licenses remain retained separately.

## Browser locale data

Emscripten builds use the browser's locale services, `Intl.Collator` and
`Intl.Segmenter` of the JavaScript engine (`src/FerroBoard.Core/text/locale_text/collation.js`),
as desktop/mobile builds use their system locale services. No ICU code or data is
linked or embedded. The engine's collation data versions can differ from the keyboard
text tables and the original .NET runtime.

The selected Emscripten 6.0.10 runtime license references (Emscripten, LLVM
compiler-rt/libc++/libc++abi/libunwind, musl, LLVM libc and mimalloc) are retained
with exact source paths and hashes in `reference/emscripten-6.0.10/`.
Toolchain binaries are external build prerequisites.

## Browser sample and Inter font

The browser sample uses web-sys 0.3.106 (MIT OR Apache-2.0); both licenses are
retained in reference/rust-dependencies/web-sys-0.3.106/. Its embedded Inter3.019
font is supplied by the pinned FerroUI.Fonts.Inter crate under SIL OFL1.1, with
complete provenance/license in reference/ferroui-licenses/INTER-FONT-NOTICE.md.
The SDK browser JavaScript runtime uses native-file-system-adapter at its locked
Git revision d43ad841581c2cc3ce47bbd1e8f11950ebdff027; its license accompanies
the browser bundle. No new .NET runtime service is introduced.
