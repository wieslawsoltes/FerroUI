# WinUI Gallery on FerroUI

This is the native FerroUI Gallery port and source browser for Microsoft WinUI Gallery
at commit `14061eeac21c38ae99f302edf3d9136d4f652c4c`.

The full pinned exported catalog is present: **126 pages and 350 sample definitions**.
Every page is searchable by name, stable ID, category, description and curated tags.
Every catalog definition retains the XAML/code from its original source file, its
path and the raw file's SHA-256 hash. The source sections preserve handlers,
bindings and substitution tokens; only line endings/trailing newlines are normalized.
Source and documentation links are available from the page. Back/forward navigation
and live theme selection use real FerroUI controls.

**This is not a completed port of every Gallery behavior.** `port-status.json`
separates source coverage from executable page coverage. The current native subset
contains 71 explicit markup adaptations and a working ScratchPad editor, for 72 partial
pages. Each remains `partial`; it does not imply that
all examples, events, binding expressions, platform capabilities or exact pixels of
the original page have been reproduced. The generated framework parity reports are
under `docs/porting/winui-gallery/data/`.

The upstream exported catalog is not the entire executable Gallery: the scanner also
inventories **355 actual `ControlExample` instances**. Its source manifest contains
**521 tracked text files, including 355 catalog-page snippets**; the remaining text
files include supporting and legacy/unreferenced inputs. Five example instances are
absent from the exported catalog.
Those differences stay in the parity ledger and completeness gate; embedding the
catalog does not mark the remaining examples as ported.

## Run

```sh
cargo run -p winui-gallery --features desktop
cargo run -p winui-gallery --features desktop -- --page=Button
```

The `desktop` feature uses FerroUI's existing desktop host and the WinUI theme.
The library without this feature contains the entire source catalog, native gallery
view and examples without linking a desktop renderer. It can be hosted as a control
by an existing application. A browser package/launcher is not included in this sample.

```rust
let gallery = winui_gallery::GalleryView::new()?;
window.set_content(Some(std::rc::Rc::new(gallery.root())));
gallery.navigate("Slider");
// Retain `gallery` for the lifetime of the view. Its callbacks hold weak state references.
```

## Implementation

`Gallery.xaml` defines the native shell. `gallery.rs` supplies filtering, history,
page composition, theme changes and source inspection. Only the selected page is
created. The catalog stores a precomputed lowercase search index, so filtering does
not allocate strings for every row. Handlers use weak references; replacing a page
releases its control tree and callback state.

`Pages/*.xaml` contains reviewed adaptations to the existing framework API.
`examples.rs` records each adaptation and supplies Rust event handlers. Examples
include button click output and disabled options, original two/three-state and
select-all check-box handlers, password validation/reveal, text character counts,
sliders, selection, layout, dates, menus, flyouts,
scrolling, resources and element-name binding. The native NumberBox, RadioButtons,
FontIcon, SymbolIcon, AutoSuggestBox, InfoBar, InfoBadge, ProgressRing, RatingControl,
NavigationView and Frame/FramePage contracts are exercised directly by examples
adapted from upstream. They use the framework's property, input, layout, drawing,
popup and automation infrastructure.

These controls are ordinary `ferroui_controls` exports and use the default FerroUI
XAML namespace. The reviewed native pages have no compatibility namespace prefix.
`FramePage` is the explicit destination type for Frame navigation; the existing
`Page` hierarchy keeps its original asynchronous navigation contract. Frame-specific
arguments and options use `ferroui_controls::navigation` / `FerroUI.Navigation`.
The source importer remains explicit so unsupported upstream syntax still produces
diagnostics.

The [native-control migration guide](../../docs/porting/winui-gallery/NATIVE-CONTROLS.md)
lists all 16 promoted control mappings, standard builders, helper namespaces and
examples for updating native application code.

Both AutoSuggestBox examples keep the original event-reason checks, word matching,
chosen-suggestion precedence and no-results behavior. The detail view uses the
original title, subtitle and 108 embedded control PNGs through native resource URIs.
`assets-manifest.json` records exact bytes and the two explicit Windows path-case
aliases. Unicode lowercase comparison is an explicit approximation to the source's
CurrentCulture search. Mock-renderer tests verify asset paths/bytes and application
selection behavior; the capture host performs actual bitmap decoding.

`feedback_samples.rs` ports both ProgressRing examples, both RatingControl examples
and all four InfoBadge examples. Activation/background options, cleared
numeric input, rating captions, fractional placeholders, source badge styles and
integer badge-value guards run through the new native controls. The InfoBadge
NavigationView example preserves the Inbox badge, notification name, opacity toggle
and pane-mode handlers. Named badge themes retain the upstream
resource keys and use the explicit Style-to-ControlTheme migration policy.

`navigation_view_samples.rs` ports all eight original NavigationView compositions:
adaptive left navigation, top navigation, orientation switching, focus-following tabs,
data-bound categories, footer items, hierarchy and the API-options form. Distinct
native Frame/FramePage factories preserve the original destinations. Category template
selection, adaptive triggers and C# handlers are explicit Rust adaptations; source
quirks and the remaining overflow, selector, transition and platform requirements
are documented in the example ledger and control contracts.

`sizing_sample.rs` preserves the CompactSizing page's standard/compact navigation
and all four text/password values plus its nullable selected date. Compact resources
are local to each compact page. PasswordBox and Header remain explicit native
TextBox/composition adaptations; changing the example does not change application
density.

`platform_samples.rs` ports the Popup open/close handlers, asynchronous clipboard
text/format/clear operations and the RefreshContainer request/deferral example.
Clipboard access comes from the containing window's existing platform service;
missing services and operation errors produce visible output. Refresh work runs on
the UI dispatcher and balances the native deferral even if the page is removed.
The Popup's placement and the refresh sample's ListBox replacement are explicit
adaptations. Bitmap/file clipboard operations, clipboard history and roaming,
custom refresh visualizers and full operating-system/input acceptance remain pending.

The color-contrast page ports the original relative-luminance calculation, contrast
ratio and pass thresholds to Rust. Its native RGB text inputs and reactive preview
are an explicit replacement for the upstream custom color-picker controls.

Original source sections are also available for inspection. **Load XAML preview**
uses the public opt-in `FerroRuntimeXamlLoader::parse_winui_as` API; it does not delete
unknown handlers, substitution tokens, `x:Bind` expressions or unimplemented elements.
Unsupported constructs produce visible migration diagnostics. Successful construction
is labeled as an XAML preview; it is not automatically promoted to parity evidence.
Original C# remains visible as source and is not executed.

The desktop host explicitly selects `WinUIXamlOptions::for_winui_theme()` after
installing that theme. Library hosts use the strict default unless they pass their
own policy to `GalleryView::with_preview_options`. The UI shows the migration's
source-positioned adaptation diagnostics on a successful preview as well as errors.

The ScratchPad page provides an editable WinUI XAML document using the same strict
loader. A failed migration clears the preview and displays the error.

## Reproduce and validate

```sh
python3 samples/WinUIGallery/generate_catalog.py /path/to/pinned/WinUI-Gallery --check
python3 samples/WinUIGallery/generate_assets.py /path/to/pinned/WinUI-Gallery --check
cargo test -p winui-gallery --lib
cargo check -p winui-gallery --features desktop
```

The catalog generator refuses a different upstream commit or tracked local source
changes. To refresh
the snapshot, first review and update the pinned commit together with the framework
source pins and parity data, then regenerate the catalog. Source text is emitted as
Rust raw strings without interpreting XAML or C# escapes.

The tests cover catalog completeness/identity, token-preserving source wrapping,
navigation failure and forward-history preservation, category/token search,
construction of every native adaptation, the native gallery shell, strict WinUI
preview behavior and actual native event/property interactions. Tests that construct
controls do not claim an interactive desktop run, pixel equality, keyboard traversal,
screen-reader interoperability or verification on every host.

The **native control promotion** has a new
[execution record](../../docs/porting/winui-gallery/data/executions/native-controls-005.json):
**182 native control tests passed across 12 targets**, with **53 source hashes**
checked before and after execution. All 521 current API evidence entries reference
this record. The current maps report **315 type candidates, 1,537 member candidates
and 205 additive types**; the new source matches are `NumberBoxValueChangedEventArgs`
and its `OldValue`/`NewValue` members. Behavior classifications and page statuses are
unchanged. Loader, theme and Gallery results below remain tied to their stated
historical checkpoints.

The **main integration checkpoint `7c679ceb`** incorporates main's TextLineBounds
work at `0ce4e90c` and regenerates the complete combined WinUI theme reference.
Local validation passed **295 WinUI theme tests** (two ignored), the explicit
reference-regeneration test, and **19 focused tests** across Base, Controls, enum
registration, generated public paths and Skia (one ignored regeneration task).
The six focused commands checked source equality before and after execution.
The checkpoint maps identify **1,535 member candidates** and **204 additive types**;
these lexical counts do not promote a behavior or Gallery page to complete parity.
The [main integration record](../../docs/porting/winui-gallery/data/navigation-main-integration.json)
preserves the exact commands and separates this evidence from earlier native and
rendering results. All three jobs in the [merged-source parity workflow](https://github.com/wieslawsoltes/FerroUI/actions/runs/39824609248)
passed; CI observations remain bound to their stated commit and time.

The **NavigationView checkpoint** publishes native contracts at `9ce7530c`, XAML
migration at `fb2fee44`, Gallery compositions at `31eba1b6` and named-control captures
at `8365ed0b`. Local Rust 1.99.0 validation passed **54 Gallery tests**, **182 native
control tests** (including 55 NavigationView/item tests), **44 XAML migration tests**
and **7 compiled/interpreted tests**, with one explicit fixture-regeneration task
ignored. Earlier local checks in this increment passed the **294-test WinUI theme
suite** (two explicit ignored tasks), existing Fluent/Simple template fallbacks,
public Rust paths and view-builder compilation. The
[checkpoint record](../../docs/porting/winui-gallery/data/navigation-validation.json)
separates those runs and preserves their commands, coverage and hashes.

Real Skia/HarfBuzz rendering produced **151 decoded, opaque, nonblank 1280 × 900
PNGs**: all 126 default light pages, nine default dark pages and all eight
NavigationView compositions in both themes. The
[new capture manifest](../../docs/porting/winui-gallery/captures/navigation-capture-manifest.json)
records every output and retains 18 reviewed frames for NavigationView and InfoBadge.
These viewport captures keep page status partial; missing Segoe glyphs, interactive
input and upstream Windows visual/accessibility comparison remain separate work.

The **prior 0.4.0 integration checkpoint `83623379`** passed **42 Gallery library tests**,
**127 native control tests**, **41 XAML migration/runtime tests**, **13 binding-lifecycle
tests** and **three deferred-resource error-isolation tests** in GitHub CI on Rust
1.99.1. Five compiled/interpreted XAML tests passed, with one explicitly ignored
fixture-regeneration task; the WinUI theme passed **294 tests**, with two explicitly
ignored regeneration/timing tasks. Its split reference preserves the exact bytes
used by both compiler comparisons. The view-builder check passed in CI. Local Rust
1.99.0 runs passed **4,383 Base tests** and repeated the 127 native tests, 13 binding
tests and 13-test Base subset used by CI. The local desktop compilation check also passed.
Source gates passed with **44 scanner/comparison regressions** and **two theme
path-case tests**. [VALIDATION.md](VALIDATION.md) records each execution source.

The [initial Actions run](https://github.com/wieslawsoltes/FerroUI/actions/runs/39771098146)
found a stale Gallery lock version after main advanced to 0.4.0; the native and
rendering jobs stopped before compilation. Integration `83623379` aligns that entry
with the workspace. All three jobs in the
[replacement parity run](https://github.com/wieslawsoltes/FerroUI/actions/runs/39771549215)
passed. Exact commands, versions and hashes are in the
[integration record](../../docs/porting/winui-gallery/data/integration-validation.json).

The current
[theme resource/style ledger](../../docs/porting/winui-theme/TRACKING.md) records
6,912 ported, 1,974 waived and 87 pending entries: InfoBadge 27, ProgressRing 8,
RatingControl 35, NavigationView styles 5 and RadioButtons 12 remain pending in the
strict source-definition inventory. The twelve RadioButtons entries moved from
obsolete no-control waivers to pending after native promotion. Native metadata
templates do not establish ports of the corresponding source definitions.

Integration CI rendered **all 126 pages in light** and **eight pages in dark** with
real Skia/HarfBuzz. All **134 PNGs** decoded as nonblank, opaque 1280 × 900 frames,
with 126/8 distinct PNG and pixel hashes. Eight CI frames for InfoBadge, ProgressRing,
RatingControl and CompactSizing were reviewed. CompactSizing shows the initial
Standard form; static ProgressRing frames do not establish motion. Rating stars and
some icons visibly lack Segoe glyphs on the Linux host. Complete visuals, real
platform input and accessibility remain pending.

The [checked-in frames](../../docs/porting/winui-gallery/captures/README.md) and
[capture manifest](../../docs/porting/winui-gallery/captures/capture-manifest.json)
remain unchanged evidence for the earlier native source checkpoint
`3c7ceb81dcc4948f374c05c4757c6cb72ed5de5d`: 126 light, nine dark and four separately
captured light frames. Five `d008845` frames remain separately identified as historical.

### Capture real rendered frames

The optional `capture` feature uses the existing headless platform with real Skia
drawing, HarfBuzz text shaping and the WinUI theme. It does not require a display
server. The following command captures actual native sample content in the Gallery
shell:

```sh
cargo run -p winui-gallery --features capture --bin winui-gallery-capture -- \
  --pages=InfoBadge,ProgressRing,RatingControl,CompactSizing \
  --output-dir=gallery-captures
```

Use `--dark` for the dark theme or `--all` to visit every catalog page. Capture is a
rendering smoke check: it does not compare against upstream Windows pixels and does
not turn a pending page into an implemented page. See `VALIDATION.md` for the tested
checkpoint, exact commands, source hashes and remaining acceptance boundaries.
Use `--pages=AutoSuggestBox --query=Button` to submit the source search example and
capture its native text and image details; the command fails if the selected image
cannot be decoded by the real renderer.

For a specific NavigationView composition, select exactly one page and name its
actual native control:

```sh
cargo run -p winui-gallery --features capture --bin winui-gallery-capture -- \
  --pages=NavigationView --control=nvSample8 --dark \
  --output-dir=gallery-captures/dark
```

The capture host uses native `BringIntoView` requests through the ancestor scroll
viewers, checks the target's visible viewport intersection, and writes
`NavigationView--nvSample8.png`. It records actual target bounds and scroll offsets.
The eight original compositions are `nvSample5`, `nvSample6`, `nvSample2`,
`nvSample7`, `nvSample4`, `nvSample9`, `nvSample8` and `nvSample`. A named-target
capture requires one explicit page; it cannot be combined with `--all`.

## Remaining acceptance

Each partial page still requires review of every original `ControlExample`, Rust
ports of its C# behavior and its complete source/API dependency map. Pages for native
windowing, notifications, islands, storage, media, WebView, composition, advanced text,
inking and controls still in separate PRs need their platform/control implementation
and integration. The catalog entry stays available while those contracts are pending.

New control implementations belong to FerroUI's ordinary namespaces. A source
contract that conflicts with an existing native type receives an explicit mapping,
such as `Page` to `FramePage`, preserving the original API and behavior. Adaptations
such as `ListView` to `ListBox` remain documented and do not establish full WinUI API
parity. Moving a control to the native namespace does not change its recorded
behavioral, visual or platform acceptance state.
