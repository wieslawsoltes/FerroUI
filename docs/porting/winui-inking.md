# WinUI inking port

This is the source, architecture and compatibility record for the inking work in
`src/FerroUI.Controls.Inking`. The goal is the full WinUI control surface with
FerroUI input, rendering, properties, layout and accessibility. The work includes
portable replacements for services supplied by Windows rather than present in
the WinUI source. Those replacements have explicit compatibility boundaries;
this document does not treat a matching API name as proof of OS engine parity.

The required control/source/member/test ledger is complete, with every platform
adaptation recorded explicitly. Dedicated FerroUI themes, the complete 86-culture
string table, toolbar configuration, accessibility providers and interactive
stencils are implemented. The full controls suite passes all 4,587 tests,
including 161 inking regressions; all 22 toolbar theme tests and ten distinct
Windows adapter fixtures pass. The broader audit still records uncalled Windows
namespace APIs and optional recognition/analysis gaps rather than implying that
the entire OS engine has been ported. Rendering through FerroUI and its existing UI-thread input/rendering
services is an intentional platform adaptation; private Windows pixels and ink
thread latency are not prerequisites for implementing that replacement. A Windows
adapter calls the genuine OS ISF/GIF, clipboard and optional recognition APIs.
Actual execution evidence is recorded separately from compilation and structural
ledger checks.

## 1. Pinned sources and accounting

- WinUI repository: <https://github.com/microsoft/microsoft-ui-xaml>.
- WinUI revision: `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`, committed
  2026-10-21 at 04:22:04 UTC.
- FerroUI starting revision: `86b6b4ab41669bf830fba681504a4ee94752a4e6`.
- Branch: `feat/winui-inking-port`.
- Review: <https://github.com/wieslawsoltes/FerroUI/pull/283>.

The exact source roots are
[`controls/dev/InkCanvas`](https://github.com/microsoft/microsoft-ui-xaml/tree/7e4b0c2096be402f51cc8f7d203ebf67cff183d0/controls/dev/InkCanvas)
and
[`controls/dev/InkToolbar`](https://github.com/microsoft/microsoft-ui-xaml/tree/7e4b0c2096be402f51cc8f7d203ebf67cff183d0/controls/dev/InkToolbar).
They contain 164 files, totaling 2,040,931 bytes at this revision:

| Category | Files | Accounting decision |
|---|---:|---|
| Control implementation/header files | 32 | Mapped individually; small related C++ declarations may share a Rust file. |
| Presenter/service implementation/header files | 2 | Split into presenter, input, events, container and synchronizer modules. |
| Public IDLs | 6 | All 45 types and 181 declared members retained in the member ledger. |
| Automation implementation/header files | 9 | Dedicated or inherited FerroUI peers identified separately from visual controls. |
| Button manager, color names and flyout support | 4 | Mapped to the equivalent control orchestration and helpers. |
| XAML templates/theme resources | 2 | Dedicated FerroUI templates and Default/Light/HighContrast dictionaries; entrance animation uses framework clocks and animations. |
| Localized string resources | 86 | All 59 keys in every culture are retained as 5,074 generated entries, checked against the pinned Git blobs. |
| API-test suites | 2 | All 65 test methods enumerated, with exact Rust test mappings where available. |
| Interaction-test suites | 2 | Kept separate from in-process API tests. |
| Test-application XAML/code-behind | 4 | Samples do not count as implemented controls or tests. |
| Diagnostics header | 1 | Existing FerroUI logging and Rust errors replace Windows ETW/debug/test-hook plumbing. |
| C++/MSBuild project files | 14 | Not applicable to the existing Cargo crate. |

The data is under [`data/winui-inking`](data/winui-inking/README.md):

- `source-map.json` keeps every path, Git blob hash, byte size, category and
  disposition.
- `api-map.json` retains every declared type/member, enum value, target file,
  symbol and behavioral note.
- `tests-map.json` keeps all 65 API and eight interaction scenarios and their
  local evidence. Four exact C# snapshots preserve the original assertions.
- `dependencies.json` distinguishes required OS dependencies from adjacent
  recognition/analysis services.
- `service-api-map.json` audits 136 methods/getters/setters on 12 selected
  external Windows classes. It identifies actual control calls separately from
  additional dependency APIs and optional extensions.
- `upstream/*.idl` are exact snapshots of all six IDLs, verified using their
  original Git blob hashes.
- `upstream/windows-0.62.2-inking-services.txt` retains the audited external
  declarations from `microsoft/windows-rs` revision
  `32c3144490c016fe496a0aed769bce60987a2e9d` with an immutable content digest.

The implementation follows [`PORTING-GUIDE.md`](PORTING-GUIDE.md). The source is
MIT licensed; the complete Microsoft notice is retained beside the implementation
in [`NOTICE.md`](../../src/FerroUI.Controls.Inking/NOTICE.md). No OS implementation
or Windows binary is copied into the portable library.

## 2. What WinUI actually supplies

WinUI's `InkCanvas` hosts an external composition visual. It constructs the COM
`InkDesktopHost`, creates an OS `Windows.UI.Input.Inking.InkPresenter` on that
host's ink thread, and attaches it through DirectComposition/SystemVisualLink.
Its `InkPresenter` is a proxy: UI-thread properties are cached, mutations are
queued, thread-affine service calls are marshaled, and input/stroke events are
posted back to the UI dispatcher.

The source of the stroke renderer, curve-fitting algorithm, native pencil
texture, ISF serializer, handwriting recognizer and OS stencil implementation is
not part of these WinUI directories. `InkStroke` and `InkDrawingAttributes`
remain Windows Runtime types in the pinned IDL. A portable port therefore needs
both the visible WinUI controls and an independently implemented service layer.

FerroUI replaces the host integration with ordinary framework drawing and
pointer routing. The controls can use whichever backend the application already
uses. The portable engine has no COM dependency and does not create a second
windowing, property or layout system. This also means that passing portable tests
does not establish Windows independent-thread latency or identical OS pixels.

The 181 WinUI IDL members do not define every method of their returned Windows
objects. The separate service ledger records the additional members instead of
equating the wrapper API with the whole Windows Ink namespace. For example,
the portable stroke currently has no `Recognized`, `PointerId`, wall-clock
`StrokeStartedTime`, independently editable `StrokeDuration`, or WinRT
`GetRenderingSegments` contract. `InkModelerAttributes` and retained container
recognition results are also absent. The pinned control code does not call those
APIs. They limit broader dependency compatibility and remain visible, while the
required control gate covers the services actually called by the control source.

## 3. Implementation organization

All paths below are relative to `src/FerroUI.Controls.Inking`. Their exact status
and individual missing members are in the ledger.

| Upstream responsibility | Rust organization | Framework services |
|---|---|---|
| InkCanvas visual and input host | `ink_canvas.rs` | `Control`, routed pointer events, capture, layout, visual invalidation and drawing context |
| Presenter proxy and collected/erased args | `ink_presenter.rs` | Shared service identity, event subscriptions and invalidation |
| Processed/unprocessed input and configuration | `ink_input.rs`, `ink_events.rs` | FerroUI pointer types and explicit `IDisposable` subscriptions |
| Wet-to-dry handoff | `ink_synchronizer.rs` | Explicit Rust transaction state instead of an OS thread-affine proxy |
| OS point/drawing-attribute value contracts | `ink_point.rs`, `ink_drawing_attributes.rs` | `Point`, `Size`, `Matrix`, `Color` and validation errors |
| OS strokes and construction | `ink_stroke.rs`, `ink_stroke_builder.rs` | Cached portable geometry and immutable input samples |
| OS stroke collection and persistence | `ink_stroke_container.rs`, `persistence.rs` | Observable changes, selection, transforms and bounded byte parsing |
| Portable clipboard transport | `ink_clipboard.rs` | The owning `TopLevel`'s `IClipboard`, application data formats, asynchronous transfers and disposal |
| Ruler/protractor state and constraints | `ink_presenter_stencil.rs` | Stable service properties, FerroUI geometry/transforms, guide constraints and annotations |
| Stencil manipulation | `ink_stencil_input.rs` | Captured touch/mouse contacts, translation, rotation, resize and cancellation |
| Toolbar orchestration and button manager | `ink_toolbar.rs` | Existing panel/children collection, styled properties and events |
| Tool/pen/toggle/menu subclasses | `ink_toolbar_buttons.rs` | FerroUI radio, toggle and checkbox control bases |
| Public toolbar enums | `ink_toolbar_types.rs` | Rust enums preserving upstream values and XAML names |
| Color labels/palettes | `color_names.rs` and pen-button code | All 32 source color families, localized labels, live brush resources and contrast policy |
| Localized resources | `ink_toolbar_strings.rs`, generated `ink_toolbar_strings_data.rs` | Existing `CultureInfo`, culture fallback and observable resource overrides |
| Live pen configuration and width slider | `ink_toolbar_configuration.rs` | Existing flyout, toggle, panel and slider controls |
| Toolbar accessibility | `ink_toolbar_automation_peer.rs` | Named peers with invoke, selection, toggle, expand/collapse, range and text-value providers |
| Optional native persistence/clipboard | `ink_platform_services.rs`; `windows_ink_services.rs` (Windows only) | Local Rust futures and the Windows Runtime |
| XAML and declarative construction | `markup.rs`, `register_types.rs`; the builders of `ferroui-view` with its feature `inking` | Existing service/type registration, event binding and view builders |
| WinUI styles of the toolbar | `Themes/WinUI.xaml`, `Themes/WinUI/InkToolbar.xaml`, `Themes/WinUI/InkToolbarResources.xaml`, compiled by `build.rs` | The compiled markup of the crate, the WinUI theme's resources and its high-contrast variant |

The shared toolbar module is an intentional file mapping: the small C++ headers
share control inheritance and boilerplate, while the source ledger retains their
individual identities. The host's formerly combined presenter header is split by
service responsibility. Neither organization removes an upstream member from
the completeness accounting.

### Ownership and reentrancy

| Object | Rust representation and lifetime |
|---|---|
| Device samples and drawing defaults | Validated value snapshots. Later default changes do not mutate completed ink. |
| Stroke | Shared `Rc<InkStroke>` identity with immutable raw points, selected state, attributes and transform. |
| Presenter and service objects | Shared `Rc` values; child services hold weak presenter references. Repeated access returns stable service identity. |
| Canvas and toolbar controls | FerroUI `Ref<T>`/`WeakRef<T>`, class macros and registered property metadata. |
| Event subscription | Explicit disposable handle; callbacks run without a live mutable `RefCell` borrow. |

The code reuses existing FerroUI patterns rather than carrying C++ reference
tracking, HRESULT, boxing or COM apartment semantics into Rust. Input validation
and malformed persisted data return `InkError`. Core algorithms avoid filesystem
access, blocking waits and an assumed thread runtime.

### Input processing

The actual upstream default is **pen only**. Mouse and touch are opt-in through
`CoreInputDeviceTypes`; a comment in the upstream test page saying that all three
work by default does not match its presenter initialization.

Mode starts at `Inking`; `RightDragAction` starts at `LeaveUnprocessed`. Barrel
and eraser input are enabled. Processed strokes expose started, continued,
ended and canceled events. Unprocessed input exposes entered, hovered, exited,
pressed, moved, released and lost events. Capture loss, disabled input and
detachment must cancel pending input without committing a stray stroke.

Input samples retain their pressure, tilt, twist and timestamps. The canvas feeds
intermediate/coalesced samples from the framework and converts millisecond input
timestamps to microseconds. Intermediate points currently share their enclosing
event's timestamp because the framework's `PointerPoint` has no separate time
field. Disabled barrel/eraser input is ignored rather than treated as an ordinary
pen, and raw pointer loss is delivered even when no processed stroke is active.
Availability of
actual device measurements depends on the existing FerroUI platform backend;
constructing a sample with these fields is not evidence that every platform
driver supplies them. Coalesced sample delivery and hardware pen testing remain
separate validation requirements.

### Stroke geometry and rendering

The portable renderer constructs pressure-dependent circular or rectangular nib
geometry using FerroUI points and matrices. The nib size is scaled by
`0.25 + 1.5 * pressure`, so the nominal pressure 0.5 of mouse, touch and
`InkStrokeBuilder` points draws the configured size (DEVIATIONS.md, "WinUI inking"). It preserves raw samples, caches
derived smoothing/outline/bounds data and invalidates those caches when geometry
inputs change. Selection alone does not need to rebuild tessellation.

The circular nib is polygonized with 32 vertices, rectangular tips retain their
corners, and full nib bounds are computed analytically. Bounded cubic Hermite
smoothing retains raw telemetry; native DirectInk's curve model is not copied.
Sparse pointer motion is erased by testing the whole movement segment. Lasso
selection requires the entire rendered stroke to be inside the polygon,
including concave-boundary checks; line selection tests intersections.

Highlighter contours are filled as one nonzero compound geometry, so overlapping
pieces of the same stroke are composited once. Separate highlighter strokes may
still overlap. Pencil opacity and tilt response are portable approximations;
native pencil grain and DirectInk's exact fit/rasterization are not claimed.

The geometry and persistence tests must include isolated points, repeated
samples, pressure zero, extreme valid coordinates, transforms, sparse segment
eraser intersections and malformed data. Rendering every captured sample as an
unconnected dot would not preserve the stroke hit-testing or appearance contract.

Geometry admission is bounded as well as raw input. A stroke reserves at most
1,048,576 contour vertices, and a document at most 4,194,304. The immutable
reservation allows for fitting subdivisions even when fitting is currently
disabled, so changing drawing attributes cannot bypass the document budget.
Repeated samples with identical position, pressure, tilt and twist preserve
their raw timestamp history without emitting duplicate geometry. Exceeding a
budget returns `LimitExceeded`; collection adds and loads remain atomic. Active
input reserves the same geometry cost across all fragments and simultaneous
strokes, and explicitly cancels/errors at the budget rather than dropping its
preview. The document bound accounts for at most 64 MiB of contour `Point` payload; raw and
smoothed telemetry, vector allocation overhead and backend geometry caches are
additional memory.

### Ruler and protractor

Both stencils expose visibility, kind, affine transform and foreground/background
colors. The ruler adds length, width, ticks and a compass. The protractor adds
radius, accent color, resizability, graduations, rays, center marker and measured
arc text. Transforms use the stencil's local top-left origin; the protractor
center is `(radius, radius)`. Initial canvas sizing adjusts the transform while
preserving the public size defaults.

The defaults follow the documented service values: ruler length 1,600 DIPs
(minimum 300), width 120 DIPs (minimum 120), protractor radius 280 DIPs and
resizing enabled. The portable implementation additionally bounds dimensions
and rejects invalid/singular transforms. See the Windows
[length](https://learn.microsoft.com/en-us/uwp/api/windows.ui.input.inking.inkpresenterruler.length),
[width](https://learn.microsoft.com/en-us/uwp/api/windows.ui.input.inking.inkpresenterruler.width),
[radius](https://learn.microsoft.com/en-us/uwp/api/windows.ui.input.inking.inkpresenterprotractor.radius)
and [resizability](https://learn.microsoft.com/en-us/uwp/api/windows.ui.input.inking.inkpresenterprotractor.isresizable)
contracts.

One touch translates a stencil; two touches rotate it and can resize a resizable
protractor. Mouse dragging translates, Shift/right dragging rotates, and Ctrl
dragging resizes the protractor. Capture is released when a guide is hidden or
its canvas is disabled/detached. Pen input follows the guide and updates the
protractor's measured arc; the angle-readout property controls actual text.
The independent geometry and rendering algorithms do not establish the private
Windows pixel-coverage or nib-masking behavior at a stencil boundary.

### Custom drying

`activate_custom_drying` returns a stable synchronizer. `begin_dry` snapshots a
batch of committed strokes; `end_dry` releases only that batch from the wet
presentation. Strokes arriving during an open transaction remain pending for the
next one. Invalid nesting/order returns an error instead of silently dropping
ink.

The upstream OS rejects activation after the first nonzero size activates its
input. Any portable timing difference is called out in the API record. Several
upstream comments mention replacing a presenter for runtime activation, but the
pinned executable implementation performs the stricter OS call. Comments alone
are not used as behavioral evidence.

### Persistence and clipboard

The portable **FERROINK version 1** format has an eight-byte magic value,
little-endian records and CRC32 integrity checking. It preserves stroke IDs,
selection, all portable drawing attributes, raw samples and point transforms.
The default budgets are 64 MiB, 100,000 strokes, 65,536 points per stroke and
1,000,000 total points. Counts and record boundaries are checked before Rust
allocation, and failed loads preserve the existing document. Portable
`copy_selected`/`paste` exchange this format under
the documented `application/x-ferroui-ink` MIME type; paste creates fresh identities and positions the
visible bounds at the requested point.

`InkClipboard` connects those bytes to FerroUI's existing `IClipboard` obtained
from the owning `TopLevel`. Its application format is `org.ferroui.ink`, which
each backend maps to its own native clipboard naming scheme. Copy snapshots
only selected ink when invoked; paste appends fresh identities at the requested
visible bounds and keeps existing ink if fetching or decoding fails. Clipboard
read transfers are disposed even if an awaited read is canceled. This supports
portable exchange between FerroUI applications; native Windows Ink exchange
uses the separate OS adapter below.

`save_async` intentionally defaults to `Portable`, unlike the Windows default
GIF-with-ISF output. `save_with_format_async` makes the format explicit.
`load_async` recognizes FERROINK magic and delegates other formats to an attached
`IInkPlatformServices`. Missing native services produce `UnsupportedFormat`;
`can_paste_from_clipboard` is false when no service is attached.
Both portable and native save paths capture the requested strokes' values at
method invocation, so edits made before the returned future is polled do not
silently change the saved document.

The optional `ferroui_controls_inking::WindowsInkServices` adapter
(`windows_ink_services.rs`) is compiled only on Windows. It calls the ink engine
through the MicroCom bindings of the crate, as the WinUI integration calls the
Windows Runtime (owner decision D3 of the core plan): `windows_ink.idl` declares
the interfaces of `Windows.UI.Input.Inking`, the in-memory stream with its reader
and writer, and the collections and asynchronous operations it uses, and the
build script generates the bindings from it on every host, where a test checks
the identifiers of the parameterized instances against their signatures.
`windows_ink_interop.rs` adds activation, the await of an operation and the
`IIterable<InkPoint>` the stroke builder reads. The slot orders and identifiers
were compared with the declarations of the `windows` crate 0.62.2, which the
adapter called before. It uses the Windows 10 1703 contracts for tilt/time and
explicit ISF saving. Construction initializes WinRT through the existing
FerroUI Windows backend and verifies native ink activation. The service stays on
its creating thread and is not `Send`. Persistence works on the framework's
local executor; the synchronous Windows clipboard APIs specifically require an
STA, normally the existing Windows UI thread. The adapter checks this condition
and returns a clear error instead of attempting to change a caller's apartment.
The only `unsafe` calls are the two byte copies of the stream reader and
writer; there is no cross-thread COM marshaling or blocking async wait.

| Request | Native operation |
|---|---|
| `Isf` save | OS `InkStrokeContainer.SaveWithFormatAsync(Isf)` |
| `GifWithEmbeddedIsf` save | OS `InkStrokeContainer.SaveAsync`, including the rendered GIF preview |
| Native load | Memory stream and OS `InkStrokeContainer.LoadAsync` |
| Native copy/paste/availability | OS `CopySelectedToClipboard`, `PasteFromClipboard`, `CanPasteFromClipboard` |
| Optional handwriting recognition | Installed recognizer enumeration and OS `RecognizeAsync`, returning candidates, bounds and original FerroUI stroke IDs |

These are genuine Windows engine calls. Native conversion, ISF/GIF round trips,
malformed-load atomicity and twist policy have passed the Windows CI fixtures
listed in section 6. Native errors are returned as `InkError::Platform`,
including missing classes, unsupported attributes or unavailable recognizers.
All asynchronous stream/ink operations use `.await` on the application's local
executor. The native decoder owns its internal allocations: byte limits apply
before calling it, and Rust stroke/point count limits apply before constructing
the returned vectors. GIF export also limits a side to 16,384 pixels and its
estimated BGRA surface to the configured byte budget.

To attach the service in a Windows application:

```rust,ignore
use std::rc::Rc;
use ferroui_controls_inking::{InkCanvas, InkError, InkPersistenceFormat, WindowsInkServices};

fn enable_windows_ink(canvas: &InkCanvas) -> Result<(), InkError> {
    canvas.ink_presenter().stroke_container()
        .set_platform_services(Some(Rc::new(WindowsInkServices::new()?)));
    Ok(())
}

// Await on the existing UI executor:
// let bytes = container.save_with_format_async(InkPersistenceFormat::Isf).await?;
// container.load_async(bytes).await?;
```

The conversion is deliberately explicit. Windows coordinates/matrices are
`f32`, ISF may quantize packet values, native load may reset timestamps, and
imported strokes receive new FerroUI identities. WinRT `InkPoint` has no twist
member, so native export rejects nonzero twist unless the application opts into
`allow_twist_loss(true)`; native imports set twist to zero. Portable-only
metadata, custom native extended properties, recognized state and stroke-start
wall-clock values do not have a complete two-way mapping. Pencil supports a
restricted set of native attributes; unsupported requested changes return the
OS error rather than silently becoming another tool. The pencil opacity of
Windows ranges from 0.01 to 5.0 and the portable one from 0 to 1, on one scale:
export sends 0 as 0.01, and import reads a value above 1 as 1, so a document
of Windows with a darker pencil loads. Use FERROINK when exact
portable round trips are required. Format operations are recorded as verified
adaptations. The native clipboard adapter preserves OS-returned stroke state;
the public portable container deliberately normalizes selection to the
published contract: newly pasted clones are selected and the old selection is
cleared. The Windows runner instead retained the old selection and left its
new strokes unselected. Both the direct OS result and the framework normalization
are explicit, rather than silently attributing the latter to Windows. The
isolated STA clipboard fixture passes against native getters and sample/matrix
values. Installed-language recognition remains an optional, independently
tracked extension.

References: the official [format enum](https://learn.microsoft.com/en-us/uwp/api/windows.ui.input.inking.inkpersistenceformat),
[save overloads](https://learn.microsoft.com/en-us/uwp/api/windows.ui.input.inking.inkstrokecontainer.saveasync),
[InkPoint members](https://learn.microsoft.com/en-us/uwp/api/windows.ui.input.inking.inkpoint),
[timestamp units](https://learn.microsoft.com/en-us/uwp/api/windows.ui.input.inking.inkpoint.timestamp)
and [native pencil restrictions](https://learn.microsoft.com/en-us/uwp/api/windows.ui.input.inking.inkdrawingattributes.createforpencil).
The
[Microsoft ISF specification](https://learn.microsoft.com/en-us/uwp/specifications/ink-serialized-format)
and [WPF's serializer](https://github.com/dotnet/wpf/tree/main/src/Microsoft.DotNet.Wpf/src/PresentationCore/MS/internal/Ink/InkSerializedFormat)
are possible references for a future portable codec; neither is imported here.

## 4. Toolbar behavior and visual scope

The toolbar has independent per-pen palettes and widths. `All` creates ballpoint,
pencil, highlighter, eraser and stencil controls; `PensOnly`, `AllExceptPens` and
`None` retain their upstream meanings. The first available pen becomes active.
Pen tools select Inking mode, eraser selects Erasing, and custom non-pen tools
select None so the previous pen does not continue drawing.

An explicitly set `TargetInkPresenter` wins over `TargetInkCanvas`. Clearing it
falls back to the canvas presenter. Target changes must remove stale stencil
state from the previous target. Color/width changes update the active presenter's
defaults. `EraseAllClicked` fires before clearing to let an undo handler snapshot
the collection, and the toolbar restores the most recently used non-eraser tool.

The intended highlighter uses the six-color upstream palette, starts at yellow
`#FFE600`, has selected width 16, and uses a rectangle nib `(width / 3, width)`.
The pinned button helper implements this, but its toolbar code builds generic
square attributes without dispatching to the helper. Using the chisel helper
and the custom-pen virtual is recorded as a deliberate correction.

The 130,900-byte upstream `InkToolbar.xaml` maps to dedicated control and
resource dictionaries of the crate (`Themes/WinUI/`), which `Themes/WinUI.xaml`
merges. They are not part of the WinUI theme, which does not depend on the inking
crate: an application on the WinUI theme that shows the toolbar includes them
after the theme,

```xml
<StyleInclude Source="ferres://FerroUI.Controls.Inking/Themes/WinUI.xaml" />
```

as the ControlCatalog does in `App.xaml` (without the include the toolbar
controls have no control themes). Seventeen typed themes and
a shared base use FerroUI template parts, pseudo classes, palette controls and
flyouts. Default, Light and HighContrast brush resources preserve their source
roles. Tool icons are framework vector geometry instead of Windows symbol-font
glyphs. Automatic popup fitting uses the existing FerroUI popup service. The
source configuration's `EntranceThemeTransition` is adapted with the framework's
`Animation`, keyframes and inherited clock: child opacity and a 40-DIP horizontal
offset animate to their base values over 200 ms, with 30 ms staggering. These are
portable timing choices. Detach/drop cancels the effect and restores the existing
transform; no clock leaves normal visible content. Progression/lifetime tests
have passed in the full controls suite. Native rendered comparisons remain a
separate visual-fidelity exercise. The pinned XAML does not define a separate
color-animation storyboard.

All 86 `Strings/*/Resources.resw` files are represented by the generated 5,074
entries, including every one of their 59 keys. The generation script verifies
the original size and Git blob hash before producing Rust. Runtime lookup uses
the existing UI culture, language/script fallback and application resource
overrides. Names, tooltips, configuration headings, slider descriptions and
palette color help use these resources. The source's 32 quantized color families
and RGB fallback format are retained. Changing `current_ui_culture` alone does
not emit a framework resource-change notification; already constructed controls
refresh on resource changes, while new controls use the current culture.

High-contrast adjustment handles original colors, system colors and the source
7:1 contrast requirement. Normal and high-contrast selections are independent;
palette entries and selected brushes update when policy or platform theme
resources change. The configuration preview uses all 103 source sample points
and follows the selected attributes. Keyboard arrow navigation wraps across
enabled, visible toolbar controls without changing the active tool; the color
palette retains its six-column navigation model.

The public `Children` collection retains application additions, while only
accepted controls enter the visual panel and receive toolbar wiring. Builtin
tool kinds, ruler and stencil are singletons; distinct custom pen/tool/toggle
instances are allowed. Unsupported children and repeated identities are ignored
for registration without changing their application-owned visibility. An ignored
duplicate is not silently promoted when the accepted control is removed.
Rejected additions emit the existing framework `Logger` warning once. All four
registration regression cases pass, including ignored duplicates, multiple
custom controls, stencil non-promotion and reordering without lost handlers.

Normal stencil activation first selects the guide and opens its chooser on the
next activation; secondary activation selects and opens it. Configuration is
suppressed during live inking. The source lift leaves its ink-state subscription
disconnected, so using the live presenter state is an intentional repair. The
source hold callback does not inspect lifecycle state; FerroUI acts only on
`Started` to avoid another activation on `Completed` or `Canceled`. The source
also ignores `DoubleTapped`, since ordinary click handling already processes
each click.

The ControlCatalog sample is a combined framework page at
`samples/ControlCatalog/Pages/InkingPage.xaml`. It enables pen/mouse/touch, wires
the toolbar to the canvas and provides clear, portable snapshot save and restore
and portable clipboard actions. The source toolbar test-page selectors, guide
checkboxes and active-tool label are exercised as actual framework controls in
`interaction_page_selectors_and_checkboxes_drive_the_live_toolbar`; this keeps
all eight source interaction scenarios with a shared test fixture. Windows
process/UIA harness plumbing is replaced by FerroUI test services. The source
interaction tests do not inject physical pen input or compare rendered pixels.

## 5. Accessibility and upstream limitations

The canvas peer should report a Pane even when no strokes exist. Its visible
bounds must follow layout, its own clip, ancestor clip transforms, scroll
viewports and root clipping. A fully clipped or hidden canvas is offscreen;
partially visible ink remains discoverable. The upstream tests specifically
exercise these cases because a composition-only canvas otherwise reports no
rendered content.

Toolbar accessibility includes tool selection, toggle state, expand/collapse
for configuration flyouts, accessible numeric width values and localized color
help. Reusing a base peer covers only the patterns that base actually implements.
Dedicated upstream peers and their tests remain explicit ledger records.

Named canvas and toolbar/tool/menu/flyout/slider peers are present. The toolbar
and the canvas report `Pane`, as the source peers do. Pen peers append localized selection and
configuration information, menu peers vary toggle/expand patterns with the
available stencils, and flyout items expose invoke plus the applicable selection
pattern. Width values clamp to their range and raise the framework's value
change notification. These mappings use real framework flyouts and providers.

FerroUI's existing selection-provider result type can represent only
`ElementNotEnabledException`, so a forbidden add/remove selection operation
returns that type with an explicit invalid-operation message. The operation is
rejected, but the native `UIA_E_INVALIDOPERATION` HRESULT is not preserved.
Provider and source-scenario tests are recorded individually in the test ledger;
external accessibility-client behavior still requires platform validation.

The pinned upstream toolbar itself documents lift gaps: `IsCustomDry` returns
false, and target changes omit mid-stroke tracking, an internal high-contrast
notification and independent input inspection. Its RadialController/Surface Dial
and external-actor integration are already dropped or no-ops in the pinned lift;
those are not additional missing features of this port. A FerroUI correction is useful
but must be documented as a correction, not attributed to unavailable upstream
implementation. Native recognizers and `InkAnalyzer` are adjacent Windows Ink
services; neither is called by the two pinned control directories. The optional
Windows adapter adds handwriting candidate recognition explicitly as an
extension; it does not implement an InkAnalyzer.

## 6. Verification and completion gates

The ledger checker runs without Rust or platform services:

```sh
python scripts/inking/check_port.py
python scripts/inking/check_port.py --json
python scripts/inking/check_port.py --require-control-complete
python scripts/inking/check_port.py --require-complete
```

It verifies all 164 source identities are retained, the six IDL and four C# test
snapshots retain their Git blob hashes, every IDL type/member and every API or
interaction method is represented, and mapped files, symbols and tests exist.
A successful ordinary check means the record is
internally consistent. `--require-control-complete` rejects gaps in control
sources, IDLs, tests and required/direct services. `--require-complete` also
includes additional dependency API and optional extension gaps. The JSON report
lists each remaining record and its gate scope. Neither mode executes behavior
or compares pixels.

Rust validation is targeted at the controls crate and inking modules:

```sh
cargo test -p ferroui-controls-inking --lib
cargo check -p ferroui-controls-inking --tests --target x86_64-pc-windows-msvc
# On Windows (does not touch the clipboard):
cargo test -p ferroui-controls-inking --lib windows_ink_services::
```

Execution results belong in the PR work log after the commands complete. A test
listed in `tests-map.json` is a scenario mapping, not an assertion that a test
run happened. Do not count a constructor test as coverage for an upstream
clipping, localization or accessibility test.

Validation recorded on 2026-10-21 uses Rust 1.89. The initial 71-test inking
run was followed by exact source-scenario, localization, theme, clipboard,
geometry-budget and provider regressions. At `dd8abb5`,
[CI run 39603593237](https://github.com/wieslawsoltes/FerroUI/actions/runs/39603593237),
job `119233997583`, completed the controls step with **154 passed, zero failed**
in 1.43 seconds, and the view/XAML step with **two passed, zero failed** in
0.03 seconds. This includes the actual source menu expansion/selection and all
seven palette description cases. The separate theme step initially had **19 passed and
two failed**. Its failures identified an alignment expectation and actual glyph
state/resource issues. Subsequent corrections use the content tree correctly:
content hosted by `PART_ContentPresenter` receives scoped descendant styles and
typed button-ancestor bindings; direct-template children retain template
selectors. The final theme command
`cargo test --locked -p ferroui-themes-winui --lib ink_toolbar` passes
**22 tests, zero failed, zero ignored**, in 0.77 seconds. This verifies nonempty
laid-out ruler/protractor geometry, current foreground and selected brushes,
routed Enter opening/closing and extension arrows for all five placements.

At the subsequent `7fbffd0` checkpoint, the full controls library command
`cargo test --locked -p ferroui-controls --lib` passed **4,587 tests, zero failed,
zero ignored**, in 8.03 seconds. This includes **all 161 inking tests** and the
four registration and three entrance/lifetime regressions. The first sandboxed
run had six unrelated remote-control failures because localhost binding was
unavailable; rerunning with localhost permission produced the full passing
result. No inking assertion failed in either run. The default-feature sample command
`cargo check --locked -p control-catalog` also succeeds, including compiled XAML,
in 6 minutes 43 seconds with three existing sample warnings.

These tests exercise portable logic and real framework control APIs. They do
not establish physical pointer telemetry, external accessibility-client
behavior, native renderer pixels or latency equivalence.

The focused workflow is `.github/workflows/inking.yml`. It runs portable control
and view integration tests, WinUI theme tests, a WebAssembly build check and the
Windows native fixtures. The generated localization data can be verified against
a checkout or downloaded snapshot at the pinned revision with:

```sh
python scripts/winui-inking/generate-strings.py /path/to/pinned/winui --check
```

The checked source snapshot has passed this verification for all 86 blobs and
5,074 entries.

The Windows target cross-check succeeds, including native test code. At
checkpoint `353c692`, [Windows CI run 39600257242](https://github.com/wieslawsoltes/FerroUI/actions/runs/39600257242),
job `119222654079`, executed nine adapter tests: **eight passed, zero failed and
one clipboard fixture was ignored**, in 2.40 seconds. The passing tests cover
conversion/guards, direct native getters, real ISF/GIF save/load, malformed-load
atomicity, explicit twist-loss opt-in and optional recognizer enumeration.
Serialized coordinates/pressure allow bounded native quantization and compare
visible points if Windows bakes a transform into coordinates.

Subsequent [run 39602608194](https://github.com/wieslawsoltes/FerroUI/actions/runs/39602608194)
and the native job `119233997021` of run 39603593237 passed **all nine enabled
fixtures**, including a native `MoveSelected` contract comparison. That fixture
confirms a relative delta, despite the Microsoft Learn parameter wording
suggesting a destination. A test-only MTA worker and 30-second completion deadline
exercise native asynchronous futures without a UI-thread blocking wait.

The `native_clipboard_round_trip` fixture remains ignored in normal developer
runs because it replaces the user's global clipboard. The focused workflow runs
it explicitly on its disposable Windows runner:

```sh
cargo test --locked -p ferroui-controls-inking --lib windows_ink_services::tests::native_clipboard_round_trip -- --ignored --exact --test-threads=1 --nocapture
```

Its first dedicated run found the native clipboard's STA requirement
(`RO_E_UNSUPPORTED_FROM_MTA`); the fixture now creates an STA through the existing
safe backend wrappers, and production calls check the apartment. The next run
successfully copied, detected and pasted native ink, then failed an incorrect
assumption that the native returned strokes were selected. A stronger fixture
now compares the adapter directly with native getters, verifies translated
samples instead of requiring identical portable/native nib bounds, and records
selection with an existing selected sentinel. That corrected reference run is
now verified. At `7fbffd0`, [run 39605086703](https://github.com/wieslawsoltes/FerroUI/actions/runs/39605086703),
job `119239093640`, passed the nine enabled native fixtures in 7.18 seconds and
the explicit STA clipboard fixture in 0.11 seconds: **ten distinct fixtures
passed, zero failed**. Its direct reference reports native bounds at the
requested `(300, 200)` position; with an existing selected stroke, that stroke
remains selected and all three pasted strokes are unselected. The adapter matches
those native getters. The public container intentionally selects pasted clones
and clears the old selection, following the
[published paste contract](https://learn.microsoft.com/en-us/uwp/api/windows.ui.input.inking.inkstrokecontainer.pastefromclipboard)
and the existing portable paste behavior. This normalization is covered by the
portable `system_clipboard_uses_only_selection_and_pastes_new_ids` and reentrancy
regressions; it is not claimed as the runner's native selection behavior.

Optional recognizer enumeration does not establish handwriting candidate
accuracy or installed-language coverage. The exact run results and test names
are also preserved in `dependencies.json`.

The reconciled ledger currently contains:

| Records | Adapted | Implemented | Partial | Not ported | Not applicable | External |
|---|---:|---:|---:|---:|---:|---:|
| 164 source files | 149 | 0 | 0 | 0 | 15 | 0 |
| 45 IDL types | 35 | 10 | 0 | 0 | 0 | 0 |
| 181 IDL members | 149 | 32 | 0 | 0 | 0 | 0 |
| 65 upstream API tests | 65 | 0 | 0 | 0 | 0 | 0 |
| 8 upstream interaction tests | 8 | 0 | 0 | 0 | 0 | 0 |
| 12 dependency records | 10 | 0 | 1 | 0 | 0 | 1 |
| 136 external service members | 104 | 0 | 9 | 23 | 0 | 0 |

Both the ordinary ledger check and **`--require-control-complete` pass**, with
**zero required-scope gaps**. All 164 source files, 45 types, 181 IDL members,
65 source API scenarios and eight interaction scenarios are accounted for.
The 15 not-applicable files are the 14 C++/MSBuild project files and one empty
generated C++ internal declaration. Framework rendering, composition and input
are complete documented adaptations with explicit limits.

The broader **`--require-complete` gate deliberately fails with 34 additional
records**: 23 uncalled OS APIs, nine partial broader-service members and two
adjacent recognition/analysis records. The full service audit is not erased to
make the control gate pass. Record counts overlap and are not a percentage of
functional completeness; native pixel/hardware/latency comparisons remain
independent evidence even for implemented framework adaptations.

### Deliberate compatibility boundaries

| Area | Current implementation and remaining evidence |
|---|---|
| Native clipboard | The STA native reference fixture passes. Native selection state is preserved by the adapter; the public container's selected-new/clear-old normalization follows the published contract. |
| Rendering | FerroUI uses bounded Hermite fitting, polygonized nibs and portable pencil opacity/tilt response without native grain. Native visual/latency comparison remains optional fidelity evidence rather than a claim of private-engine equivalence. |
| Localization/high contrast | All 86 source tables and live resource/contrast handling are implemented. Global culture mutation alone has no existing framework notification; physical platform rendering and external accessibility clients remain platform validation. |
| Stencils | All audited service properties, manipulation and annotations are implemented; centerline masking and edge snapping replace native nib/pixel coverage algorithms. |
| Input | Framework routing, capture, cancellation and coalesced samples are tested. Actual pressure/tilt/twist quality depends on the platform backend; coalesced points currently share their enclosing event timestamp. |
| Broader Windows namespace | The 23 uncalled additional APIs and optional recognition/analysis gaps remain visible in the full dependency gate. They are not silently equated with the required control surface. |

## 7. Porting process and review checkpoints

The work was opened as draft PR 283 before implementation and developed in the
review branch with successive source, implementation and correction checkpoints.
The checkpoints below make the process reproducible; their presence does not
replace the executed test evidence above.

| Checkpoint | Work recorded |
|---|---|
| [`a54994c`](https://github.com/wieslawsoltes/FerroUI/commit/a54994c) | Opened the port record, pinned WinUI and FerroUI revisions, and separated public control source from Windows engine dependencies. |
| [`612a375`](https://github.com/wieslawsoltes/FerroUI/commit/612a375) | Added the exhaustive 164-file inventory, IDL/member and test ledgers, dependency contracts, attribution and structural completion checker. |
| [`4bf7b0d`](https://github.com/wieslawsoltes/FerroUI/commit/4bf7b0d) | Added the portable stroke engine, presenter/input/drying services, canvas, toolbar families and optional safe Windows interoperability adapter. |
| [`9d554ff`](https://github.com/wieslawsoltes/FerroUI/commit/9d554ff) | Connected actual stencils, templates, all localized resources, framework clipboard, XAML/view construction and focused platform fixtures. |
| [`353c692`](https://github.com/wieslawsoltes/FerroUI/commit/353c692) | Corrected workflow quoting and established the first passing real Windows conversion/ISF/GIF test evidence. |
| [`c65bba4`](https://github.com/wieslawsoltes/FerroUI/commit/c65bba4) | Added exact source interaction/provider/lifetime scenarios, geometry limits and reentrancy corrections, plus the native relative-translation contract fixture. |
| [`dd8abb5`](https://github.com/wieslawsoltes/FerroUI/commit/dd8abb5) | Fixed provider expansion ordering, completed palette-description assertions and established the native clipboard STA requirement. The controls/view test steps passed; theme and native clipboard assertions identified specific remaining defects. |
| [`7fbffd0`](https://github.com/wieslawsoltes/FerroUI/commit/7fbffd0) | Implemented source registration and diagnostics, added the portable entrance animation, and replaced clipboard assumptions with direct native references. All 161 inking tests and all ten Windows adapter fixtures passed; template tests exposed the final content-tree selector/binding issue. |
| Final theme and ledger checkpoint | Corrected presenter-hosted glyph selectors, ancestor brush bindings and dynamic geometry resources. All 22 theme tests pass, ControlCatalog compiled XAML passes, and the required-control gate reaches zero gaps. |

The method used throughout was to read the pinned implementation and its matching
assertions, map its public contract to the existing FerroUI property/class/event
model, implement the dependent service behavior, and port the assertions through
real framework controls. Failures drove concrete corrections: scroll clipping,
selection-before-expansion, palette help clearing, conditional template
precedence and clipboard apartment requirements were addressed at their owning
layer. A test was marked adapted only after the corresponding assertion passed;
source scenarios that merely construct a control were not inflated into an
unwritten physical-pen or pixel-parity requirement.

For later upstream updates, first compare the immutable source inventory and
IDL/test snapshots against the new revision. Retain old identities until that
comparison is reviewed, update each affected source/member/test row, regenerate
localized resources from the new pinned blobs, and rerun the structural checker
and relevant platform fixtures. Broader dependency APIs must retain their
explicit scope so that adding a Windows namespace member neither silently
expands nor silently disappears from the required control contract.
