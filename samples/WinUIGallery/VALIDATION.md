# Gallery validation checkpoint

This ledger records bounded evidence for the native FerroUI sample. Page and API
completion remains governed by `port-status.json` and the framework parity scanner.
The catalog contains 126 pages: **72 have partial native implementations**, including
the ScratchPad, and **54 remain pending**. There are **44 explicit partial mappings to
original source examples**. No page is marked `ported` or `verified`.

The current [generated API summary](../../docs/porting/winui-gallery/data/summary.json)
records 1 verified type, 54 partial types, 202 verified members and 264 partial members.
The complete `Symbol` enum contributes 197 of those verified members; these counts do
not certify complete control, page, visual or platform parity.
The source candidate map separately identifies 315 types and 1,537 members without
certifying their signatures, defaults, behavior or WinMD ABI compatibility.

## Reproducible source inputs

The source repository is [Microsoft WinUI Gallery](https://github.com/microsoft/WinUI-Gallery),
pinned to `14061eeac21c38ae99f302edf3d9136d4f652c4c`. The catalog generator checks
that exact commit and refuses tracked modifications to its inputs.

| Input or generated evidence | SHA-256 / location |
| --- | --- |
| Original `WinUIGallery/SampleSupport/Data/ControlInfoData.json` | `334896cc85905de1afe64bdac7717218dc6f60607c132443312d1e9d37b0e27b` |
| Generated `catalog_data.rs` | `451aa227d421cab6eae1a76573ed5c32a8ac55bed6965041bbdffaa850d26416` |
| Original sample files for 350 catalog definitions | Raw source hashes are retained in every `SampleDefinition` in `catalog_data.rs`. |
| Explicit mappings for 44 partial source examples | Source paths, raw hashes and named test anchors are in `port-status.json`. |
| Original control images | `assets-manifest.json` records the source path, original filename, byte count, SHA-256 and registered URI paths of every PNG. |

The source browser preserves original handlers, binding expressions and substitution
tokens. Exported catalog transformations are not used to sanitize the displayed
source. The framework scanner separately inventories 355 actual `ControlExample`
instances. Its [source-file manifest](../../docs/porting/winui-gallery/data/source-files.json)
contains **521 tracked text files, including 355 catalog-page snippets**, with
supporting and legacy/unreferenced files counted separately from executable examples.
Source inventory is not evidence that every example executes.

At the prior source-input checkpoint, both generation checks passed; the pinned catalog
and asset inputs retain the hashes listed above:

```sh
python3 samples/WinUIGallery/generate_catalog.py ../upstream-gallery --check
python3 samples/WinUIGallery/generate_assets.py ../upstream-gallery --check
```

Results: 126 pages, 350 source definitions, 97 original cat names and 108 original
PNG files. The images total 525,150 bytes. They are registered under 110 paths:
the original Windows metadata spells `CheckBox.png` and `AnnotatedScrollbar.png`
differently from the tracked filenames `Checkbox.png` and `AnnotatedScrollBar.png`.
Both spellings are explicit aliases; original filenames and image bytes are retained.

## Native control promotion

The reviewed Gallery pages now use ordinary native control identities in the default
FerroUI XAML namespace. Rust samples import the controls directly from
`ferroui_controls`. Frame destinations derive `FramePage` and import navigation
errors from `ferroui_controls::navigation`, preserving the existing `Page` and
control-navigation argument contracts. The embedded InfoBadge dictionary now comes
from `FerroUI.Themes.WinUI/Controls/InfoBadge.xaml` and targets the native InfoBadge.
Original source previews still use the explicit WinUI importer.

All 72 implemented pages remain partial, with 54 pending and no completed or verified
page. API behavior statuses are also unchanged. The promotion changes source identity
and usage; it does not demonstrate additional source-example, rendering or platform
coverage. Original catalog/source/assets and their upstream hashes remain intact.

The promotion's [native-control execution record](../../docs/porting/winui-gallery/data/executions/native-controls-005.json)
records **182 passing tests across all 12 integration targets**, with no failures or
ignored tests. Its **53 frozen source hashes** cover the moved implementations,
extracted peers/settings and updated tests and were checked before and after the
command. All **521 current API evidence entries** now reference this execution.
The record identifies the tested source by its hashes; its parent commit identifies
the base of the changes. This establishes native-control test evidence for the
promotion. Loader, theme and Gallery executions retain their separately stated scopes.

The current comparison reports **315 type candidates and 1,537 member candidates**.
The increase consists only of the newly matching native `NumberBoxValueChangedEventArgs`
type and its `OldValue`/`NewValue` members; their behavior remains unverified. The
existing-API guard reports **204 additive types** against the unchanged protected
baseline. No API behavior or Gallery page status advances from these source matches.

The current [theme ledger](../../docs/porting/winui-theme/TRACKING.md) records
**6,912 ported, 1,974 waived and 87 pending definitions**. Twelve RadioButtons source
definitions are now pending after obsolete no-control waivers were removed. They
join InfoBadge's 27, ProgressRing's 8, RatingControl's 35 and five NavigationView
style definitions. These are source-definition dispositions, separate from runtime
or visual acceptance.

The revision-specific executions and captures below predate this promotion. They
retain their original commands, namespaces, paths, counts and hashes as historical
evidence; they are not relabeled as executions of the promoted source.

## Main integration checkpoint: TextLineBounds

Merge [`7c679ceb0a2e36c6259286ef47f4cc5d5b7abc70`](https://github.com/wieslawsoltes/FerroUI/commit/7c679ceb0a2e36c6259286ef47f4cc5d5b7abc70)
incorporates main `0ce4e90c20d89a03590ccdde555d5daa87e88ee6` and the TextLineBounds
changes from PR #294. The generated theme reference was regenerated from the merged
sources; both compiler-output comparisons continue to compare all reference bytes.
The existing `Full` text-line-bounds default remains in place. Only the source's
right-side command-bar labels explicitly select `Tight`.

The separate [main integration record](../../docs/porting/winui-gallery/data/navigation-main-integration.json)
records source hashes, exact commands, environment, measured results and log hashes.
The theme run used the prepared merge whose source blobs and reference match the
published merge. The six focused commands ran after publication and checked source
equality with `7c679ceb` before and after every command; the runner exited 0.

| Validation | Actual result | Evidence scope |
| --- | --- | --- |
| Complete combined theme | **295 passed, 0 failed, 2 ignored** | Includes both full compiler-reference comparisons; the explicit regenerator also passed once. |
| Base text-line bounds | **8 passed** | Text metrics, bounds modes and clipping defaults. |
| Enum registration and lookup | **2 passed** | Two exact metadata tests, one execution each. |
| TextBlock line bounds | **5 passed** | Native control property and layout propagation. |
| Generated public Rust paths | **3 passed, 1 ignored** | Combined TextLineBounds and NavigationView declarations/accessors; the fixture-regeneration task remains explicitly ignored. |
| Skia cap-height metrics | **1 passed** | The font's cap height agrees with its OS/2 table. |
| Source gates | **44 scanner tests passed; normal gates passed** | Protected snapshots and API baseline remain unchanged. Strict completeness still returns expected exit 2. |

The checkpoint lexical inventory has **314 type candidates, 1,535 member candidates,
562 XAML member candidates and 204 additive types**. Existing API preservation still
accounts for **3,170 types, 28,435 declared member names and 458 registrations**.
The additional TextBlock.TextLineBounds candidate is not promoted to verified
WinUI behavior by a name match. The page state remains **72 partial and 54 pending**,
with no complete page claimed.

The [parity workflow at this merge](https://github.com/wieslawsoltes/FerroUI/actions/runs/39824609248)
completed successfully: all three jobs passed, covering pinned comparisons, native
control/XAML/Gallery contracts and real rendering. Its strict completeness gate
is separate and remains unsatisfied. The integration record preserves timestamped
CI observations, commands and available log provenance. The initial
general CI run at this merge rejected an upstream-name reference in one source
comment. Its subsequent terminology correction changes no executable statement;
all five source-convention steps passed locally after the correction. The original
failure and later checks remain separate evidence.

The fresh parity CI used `rustc 1.99.1 (58cf8f9c3 2026-10-13)` on Linux x86_64.
Its actual checkout, `2bd0915de2ada16ef5c2124afd18c694a6a901e5`, is the GitHub merge
commit; fetching that commit and comparing Git tree IDs confirms that its complete
tree equals the published `7c679ceb` tree. The CI log records **182 native-control,
13 Base local-value, 13 binding-lifetime, 44 migration, 7 compiled/interpreted,
3 deferred-resource, 295 WinUI-theme, 2 existing-theme fallback, 3 public-path and
54 Gallery tests passed**. The compiled, theme and public-path suites respectively
retain one, two and one explicit ignored tasks. Declarative API compilation also
passed. These are actual new CI executions, separate from the earlier local runs.

The [new CI rendering artifact](https://github.com/wieslawsoltes/FerroUI/actions/runs/39824609248/artifacts/12447045059)
was downloaded and checked against its reported ZIP digest. All **151 PNGs** decode
as opaque, nonblank 1280 × 900 images, with **150 distinct images** across the full
set. Of the 151 outputs, **148 are byte-for-byte and decoded-pixel identical** to
the earlier local capture manifest, including all sixteen named NavigationView
frames and both InfoBadge frames. The complete comparisons and per-image hashes
are preserved separately in the main integration record.

The three changed frames were reviewed against their hash-verified historical
images. In CommandBar, only the Add/Edit/Share labels move down one pixel: 442
pixels differ, with no clipping and no change to icons, overflow or page layout.
That result is consistent with the merged source's explicit `Tight` label bounds.
The two ProgressRing differences are confined to the moving arc, consistent with
capture at a different animation phase. These observations do not prove motion
timing, interactive behavior, operating-system interoperability or Windows pixel
parity. Host and toolchain provenance remain recorded separately for each run.

The earlier NavigationView runtime records and 151-image capture manifest below
are unchanged. No new local capture execution is claimed for this merge, and no
historical test, binary or image is reassigned to the newer source revision.

## NavigationView checkpoint

This increment merges main `d5405346e680e060c9675572efe02df131428c08` and preserves
the earlier integration evidence below. Its implementation is published in separate
reviewable commits:

| Source scope | Commit |
| --- | --- |
| Native NavigationView, items, events, peers, registrations and focused tests | `9ce7530c2e4cde9d2d04870dda1d9c0afdef5baa` |
| Opt-in XAML projection and emitted/interpreted fixtures | `fb2fee44afa4325c7097f60b4e3d0af745977e44` |
| Eight NavigationView compositions, seven destination pages and InfoBadge hosting | `31eba1b629159c55a168ecdcad01e39df40a1eb5` |
| Explicit API maps, partial evidence and generated inventories | `fdb10276cf93bdfaef9974cf1db7e230a72866d6` |
| Reopened NavigationView source-style obligations | `e32d933fae7afa20f68d8dd78ba6b56aec061d92` |
| Named-control capture support and expanded parity workflow | `8365ed0b33252c5169f48d56f0a9f3d511e53908` |

The [navigation validation record](../../docs/porting/winui-gallery/data/navigation-validation.json)
retains the exact commands, measured results, log hashes and source coverage for
this increment. It distinguishes the final run with frozen native source hashes
from earlier local theme/path/migration checks. Those earlier result manifests did
not record a full execution-time source hash map; their execution commit is not
inferred retroactively. All results in this section are local Linux x86_64 runs on
`rustc 1.99.0 (b940084d7 2026-09-28)`. Previous GitHub CI results below belong to
their stated historical revision.

### Executed tests and source gates

| Suite | Actual result | Evidence scope |
| --- | --- | --- |
| Native compatibility controls, 12 serialized targets | **182 passed, 0 failed** | 127 earlier native tests plus **32 NavigationView and 23 item/hierarchy tests**. Seven new implementation/test files were hash-checked before and after every target. |
| Gallery library | **54 passed, 0 failed** | The previous 42 tests plus ten NavigationView and two InfoBadge integration tests. |
| XAML migration/runtime | **44 passed, 0 failed** | Opt-in namespace and member policy, native icon literals, diagnostics and construction. |
| Emitted/interpreted XAML | **7 passed, 0 failed, 1 ignored** | Generated Rust and the runtime loader agree on hierarchy, icon ownership, badges, footer and selection. The ignored task explicitly regenerates the fixture. |
| WinUI theme library | **294 passed, 0 failed, 2 ignored** | Both complete compiler-reference comparisons and storage checks; explicit ignored regeneration/timing tasks. |
| Existing Fluent/Simple themes | **1 test passed in each** | Built-in templated-control coverage recognizes native metadata-template fallback without replacing existing theme behavior. |
| Public Rust paths | **3 passed, 0 failed, 1 ignored** | All registered public paths and generated accessors; the explicit fixture regeneration also completed separately. |
| View builders | **Compilation and freshness passed** | Existing and WinUI namespaces compile together. |
| Scanner/comparison tests | **44 passed** | Source snapshots, symbols, namespaces, builders, evidence freshness and compatibility baseline rules. |
| Theme path-case tests | **2 passed** | Separate source-path accounting checks. |
| Exact source gates | **Passed** | API scan, event targets, Symbol map, builders, catalog and assets regenerate exactly. |
| Strict completeness | **Exit 2, expected** | 1,544 public API types, 7,264 members and all 126 pages still lack complete behavioral execution evidence. |

The [native execution record](../../docs/porting/winui-gallery/data/executions/native-controls-004.json)
contains all 55 new passing test names, seven source hashes and the twelve real
per-target command/log records. The combined native log SHA-256 is
`52fc6ba8e9b658544d90d1c372f95b49eebc9f122971b0946b34fa5eaf05c291`.
Its source commit is `9ce7530c`; later mapping/documentation commits do not relabel
these tests as having been executed on a different revision.

NavigationView tests cover exact adaptive thresholds and pane modes, cancellable
and reentrant transitions, selection versus invocation, pending selection completion,
selection suppression, observable source changes, item identity, footer/settings
containers, lazy hierarchy, keyboard/focus behavior, RTL, automation, template
replacement and weak ownership. Regression cases include stale dispatcher callbacks,
reentrant template changes, clearing a factory before its content, nested data
contexts and retained automation providers after child removal. These tests exercise
native property, collection, input, layout and automation objects with mock host
services; they do not certify the Windows automation bridge or every source style.

The Gallery tests preserve source behavior that can look surprising: the fourth Top
menu item navigates to SamplePage3; category items navigate to SamplePage1; the
focus-following tabs use the running C# page's normal journal behavior; and the API
example starts unselected with an empty Frame. Category template selection, adaptive
triggers and C# handlers remain explicit Rust adaptations. All four InfoBadge source
compositions now have native examples, including NavigationView badge hosting.

Existing API preservation still accounts for **3,170 namespaced types, 28,435
declared member names and 458 source registrations** against the fixed starting
revision, with **203 additive types** reported separately. This is declaration
preservation, not complete signature/default/behavior certification. Event-target
generation records 1,509 public target types and 375 direct events; the complete
Symbol map retains 197 numeric values, glyphs, markup names and iteration entries.
The protected upstream snapshots, API baseline and symbol source remain unchanged.

The NavigationView checkpoint's theme ledger contains **6,912 ported, 1,986 waived and 75 pending
entries**, with zero unresolved references and duplicate keys. The five NavigationView
styles move from waived to pending because native target controls now exist. They
join InfoBadge's 27, ProgressRing's 8 and RatingControl's 35 pending entries. A native
metadata template does not establish parity with the corresponding WinUI style.

### Rendering and review

The capture executable built successfully, then produced **151 PNGs** with real
Skia drawing and HarfBuzz shaping: **126 default light pages, nine default dark
pages and eight named NavigationView controls in each theme**. Every output decoded
as opaque, nonblank 1280 × 900 content. The
[capture manifest](../../docs/porting/winui-gallery/captures/navigation-capture-manifest.json)
records all eighteen actual invocations, source/binary/log hashes, per-frame PNG
and decoded-pixel hashes, and the eighteen retained images. No image was resized,
retouched or converted after capture.

Named captures use native `BringIntoView` requests, then verify the actual target's
visible intersection through its clipping ancestors. They log target bounds and
ancestor scroll offsets. This resolves the selected source composition into view;
it does not fabricate scroll offsets or produce a full-document screenshot.

All eight NavigationView compositions were reviewed in light and dark, and the
InfoBadge page was reviewed in both themes. Left/Top layouts, initial selection,
footer/settings rows, collapsed hierarchy, options, navigation destinations and
badge hosting are visibly composed. The API example's Settings row is present in
both complete frames. Compact mode intentionally hides captions. Narrow destination
tiles follow the source page's 250-pixel minimum blue tile and `Auto,*,*` columns;
the source's inner ScrollViewer can contain text below the visible sample viewport.
Static frames do not independently establish scrolling, popup behavior, focus
navigation, animation, resizing, upstream pixel equality or platform accessibility.
Some icons remain missing-glyph rectangles because the Linux host lacks the Segoe
icon fonts. The [capture review](../../docs/porting/winui-gallery/captures/README.md)
links the retained images and records these boundaries.

### Reproduction

The local builds used one Cargo job, disabled incremental compilation and development/
test debug information. The isolated toolchain/cache paths are recorded in the
checkpoint JSON; other hosts can use their own paths. The local Skia archive SHA-256
remains `9ebe4c448cc9f789bad241cbd11739cdf759e27db5719c23a7635e096d5807b3`.
The final native runner enabled the `testing`, `ferroui-base/compiler-metadata` and
`ferroui-base/markup-functions` features together to share the constrained local
build graph. The CI workflow retains its normal `testing` command. The execution
record preserves the exact feature set rather than treating these runs as identical.

```sh
cargo test --locked --offline -p ferroui-controls \
  --features testing,ferroui-base/compiler-metadata,ferroui-base/markup-functions \
  --test winui_navigation_view --test winui_navigation_items
cargo test --locked --offline -p winui-gallery --lib
cargo test --locked --offline -p ferroui-markup-xaml-loader --test winui_migration
cargo test --locked --offline -p ferroui-markup-xaml-loader --features emitter --test winui_compiled
cargo build --locked --offline -p winui-gallery --features capture --bin winui-gallery-capture
target/debug/winui-gallery-capture --all --query=Button --output-dir=gallery-captures/light
target/debug/winui-gallery-capture \
  --pages=NumberBox,RadioButton,AutoSuggestBox,InfoBar,InfoBadge,ProgressRing,RatingControl,CompactSizing,NavigationView \
  --dark --query=Button --output-dir=gallery-captures/dark
target/debug/winui-gallery-capture --pages=NavigationView --control=nvSample8 \
  --dark --output-dir=gallery-captures/dark
python3 scripts/winui-gallery/scan.py --check
python3 scripts/winui-gallery/scan.py --check --require-complete
```

Run each of `nvSample5`, `nvSample6`, `nvSample2`, `nvSample7`, `nvSample4`,
`nvSample9`, `nvSample8` and `nvSample` in both themes to reproduce the sixteen named
frames. The final command above intentionally returns exit 2 while the full port
is incomplete. Passing partial contracts or rendering a pending source-browser
page does not satisfy it.

## Prior integration checkpoint: FerroUI 0.4.0

The validated runtime integration checkpoint is
[`83623379d8aeaa5a87d0417e7ce0823fc6fe9722`](https://github.com/wieslawsoltes/FerroUI/commit/83623379d8aeaa5a87d0417e7ce0823fc6fe9722),
which merges main at `cfa62aeb80f5d28990f3d4e4b9e5d1ad9d7fb640` and uses
workspace version **0.4.0**. In the
[initial Actions run](https://github.com/wieslawsoltes/FerroUI/actions/runs/39771098146),
source comparison passed, but native-contract and rendering jobs stopped before
compilation: `Cargo.lock` still named the Gallery as `0.4.0-preview.1` after main's
workspace version advanced. The integration aligns that entry with 0.4.0, and
`cargo metadata --locked --offline` passed.

All three jobs in the
[replacement parity run](https://github.com/wieslawsoltes/FerroUI/actions/runs/39771549215)
passed: pinned source comparison, native/XAML/Gallery contracts and native rendering.
GitHub checked out merge commit `864083407f12acf9cb845c72b0c7337c52b6141d`
for that PR head. The
[integration record](../../docs/porting/winui-gallery/data/integration-validation.json)
records both run IDs, job outcomes, Cargo input hashes, exact commands, measured
counts and log hashes. These reruns do not change any API or page completion status.

## Prior framework regression checkpoint: `83623379`

The following results apply to integration `83623379` on Linux x86_64. GitHub CI
used `rustc 1.99.1 (58cf8f9c3 2026-10-13)`. Local reruns used
`rustc 1.99.0 (b940084d7 2026-09-28)` and covered native controls, the full Base
library, the focused Base subset and bindings, plus desktop compilation. The remaining contract-suite results
come from CI, as identified below.

| Suite | Measured result | Scope |
| --- | --- | --- |
| Native WinUI control integration | **127 passed, 0 failed**, CI and local | 77 existing tests plus InfoBadge 13, ProgressRing 16 and RatingControl 21. |
| Base library | **4,383 passed, 0 failed**, local | Includes the additive local-value query; existing priority and clearing semantics remain unchanged. |
| Base local-value workflow subset | **13 passed, 0 failed**, CI and local | This filtered subset is included in the full Base total above. |
| XAML migration/runtime | **41 passed, 0 failed**, CI | Includes namespace-isolated feedback controls, actual native construction and audited InfoBadge style-key projection. |
| Compiled/interpreted XAML | **5 passed, 0 failed, 1 ignored**, CI | The ignored task explicitly regenerates a reviewed fixture; emitted and interpreted root/template binding tests pass. |
| WinUI binding lifecycle | **13 passed, 0 failed**, CI and local | Explicit initialization, source/target changes, template recycling, error scope and subscription lifetime. |
| Deferred-resource error isolation | **3 passed, 0 failed**, CI | Typed nested error scopes preserve inner errors; unrelated panics resume and ordinary throwing retains its prior behavior. |
| Native view builders | **`cargo check --locked -p ferroui-view` passed**, CI | Generated typed builders compile against the checkpoint's native controls. |
| Native Gallery library | **42 passed, 0 failed**, CI | Application interactions and mock-service acceptance described below. |
| WinUI theme library | **294 passed, 0 failed, 2 ignored**, CI | The explicit ignored tasks regenerate the reference and measure load time. Both compiler-output comparisons and four reference-storage tests pass. |
| Scanner/comparison regression | **44 passed** | API, glyph, namespace-builder and baseline comparison regressions. |
| Theme path-case regression | **2 passed** | Source dictionary path handling remains explicit and deterministic. |
| Source and inventory gates | **Passed** | API/event/symbol/builder freshness, existing API preservation, full theme inventory, and exact catalog/asset regeneration. |
| Desktop host | **Compilation passed**, local | Existing desktop/X11/Skia/HarfBuzz integration compiles; interactive desktop acceptance remains unverified. |

The native-control [execution record](../../docs/porting/winui-gallery/data/executions/native-controls-003.json)
retains the original command, passing names for all 50 new focused tests,
implementation/test hashes and the run-log hash. It is unchanged; the integration
record separately documents the subsequent 127-test reruns. New API contracts
remain `partial`; absent ProgressRing
animated-source members and RatingItem state branches without direct assertions remain
unverified. These recorded tests use actual native objects with mock host services.

The local runs used `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`,
`CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_PROFILE_TEST_DEBUG=0`; the contract CI job
used the same settings with two Cargo jobs. The exact local Base commands were:

```sh
cargo test --locked --offline -p ferroui-base --features testing --lib ferro_object_tests_get_value
cargo test --locked --offline -p ferroui-base --features testing --lib
```

The remaining exact CI commands include:

```sh
cargo test --locked -p ferroui-markup-xaml --test winui_bindings
cargo test --locked -p ferroui-markup-xaml-loader --test winui_migration
cargo test --locked -p ferroui-markup-xaml-loader --features emitter --test winui_compiled
cargo test --locked -p ferroui-markup-xaml --lib xaml_error_scope::
cargo test --locked -p ferroui-themes-winui --lib
cargo test --locked -p winui-gallery --lib
cargo check --locked -p ferroui-view
```

The local desktop compilation check also passed:

```sh
cargo check --locked --offline -p winui-gallery --features desktop
```

The native-control command, local repeats and their individual log hashes are in
the integration record. No extra compiler-metadata or markup-function features
were added to the workflow's commands for these reruns. The checked-in compiled-theme
reference is stored in five parts, with its former path retained as a short
guide. Tests join the parts byte for byte before both existing build-time/runtime
compiler comparisons. The reference-storage change leaves production emitter
behavior unchanged. Four additional tests
cover reference ordering, chunk boundaries, byte-preserving joining and unexpected
files. The two ignored theme tasks are explicit regeneration and timing operations.

The existing-API gate preserves 3,170 namespaced types, 28,435 declared member names
and 458 source registrations against the starting commit, with **181 additional
types: 82 from the compatibility branch and 99 inherited from merged main**.
This source-declaration result does not establish every signature, default or
behavioral equivalence.

The separate [theme resource/style ledger](../../docs/porting/winui-theme/TRACKING.md)
records **6,912 ported, 1,991 waived and 70 pending entries** across scoped resources
and styles. The 70 pending entries comprise InfoBadge 27, ProgressRing 8 and
RatingControl 35. The checkpoint inventory has zero unresolved references and zero
duplicate keys. Its declaration accounting uses the theme's own source pin and
does not itself certify native behavior or visual equivalence; compiled-theme
execution is reported separately above.

## Prior Native Gallery behavior tests: `83623379`

The library suite passed **42 tests, 0 failures** in integration CI on Linux x86_64
with `rustc 1.99.1 (58cf8f9c3 2026-10-13)`:

```sh
cargo test --locked -p winui-gallery --lib
```

The tests use the framework's existing native property, control, event, dispatcher,
mock-window and service test infrastructure. Test sources at this checkpoint are:

| Source | SHA-256 |
| --- | --- |
| `tests.rs` | `1475fcaadfa9e4745eaefdd9539641556693e06143e9db3899f0b82d1cd9b70a` |
| `feedback_samples_tests.rs` | `523fbcde28227291e93fd490eab592e2025fe91130bbe35c3c1eaff96c72b15d` |

| Area | Evidence exercised by the tests | Remaining acceptance |
| --- | --- | --- |
| Catalog and shell | Unique stable IDs; all 126 pages selectable; filtering and navigation history; native adaptation construction; original source preservation. | Every upstream page's complete example and application behavior. |
| XAML preview | Public opt-in WinUI migration; strict unsupported-binding diagnostics; typed missing-resource error returned without unwinding. | Executing original C# and the full upstream binding/template/state feature set. |
| Buttons, text and selection | Click/disabled output; original two-state, three-state and select-all CheckBox handlers; Unicode scalar text counts; Slider/ComboBox outputs; command-bar events. | All source options, inputs, accessibility and visual states. |
| Password examples | Source rejection/clearing of `Password`, valid/empty transitions and reveal option, using an explicit native TextBox adaptation. | A distinct WinUI PasswordBox API contract and full source visual behavior. |
| NumberBox | Original native values, expression evaluation, NaN/empty state and ValueChanged observation. | All source formatters, localization, input and appearance. |
| Radio buttons and icons | Original group/color selection, Symbol enum and FontIcon glyph values. | Platform font availability, full keyboard/automation acceptance and every icon subtype. |
| Frame/Page | Native page types, navigation, BackStack/ForwardStack and journal behavior. | Page transitions, animation effects and journal persistence. |
| AutoSuggestBox | All 97 source cat names; user-input-only filtering; word matching; selected-item output; title filtering and prefix ordering; chosen suggestion before free query; unchanged details on no result. | Exact CurrentCulture search semantics, full platform input/automation and source appearance. |
| Control detail images | All 126 source image URIs resolve to the original embedded PNG bytes, including the two explicit path aliases. | Real decoding is established separately by the renderer capture command. |
| InfoBar | All three source compositions: severity, short/long message, Button/HyperlinkButton action and open/closable/icon options. | Complete native visual states, icons and live-region acceptance. |
| InfoBadge | Three source examples: selection of the original severity theme keys, Refresh-button composition with the original glyph, and the source integer guard for dynamic values. | NavigationView.InfoBadge and the remaining scoped source-resource definitions are pending; installed-font rendering, exact visuals and Windows accessibility require separate acceptance. |
| ProgressRing | Both source examples use the native control, activation/background options and finite/empty NumberBox handling. | Custom DeterminateSource/IndeterminateSource, exact Lottie motion and platform UIA/pixel acceptance remain pending. |
| RatingControl | Both source examples retain user-only caption changes, live clear/read-only options and fractional placeholder binding in both directions. | Locale formatting, raw gamepad mappings, text-scale/custom-template details, installed glyph fonts and Windows visual/accessibility acceptance. Native Value/PlaceholderValue reject NaN as an explicit validation exception. |
| CompactSizing | Navigation between distinct native page types preserves all four text/password fields and the nullable date; compact resources remain local and pages release after removal. | PasswordBox and Header use explicit native TextBox/composition adapters; full original control contracts and rendered density/input acceptance remain partial. |
| Popup | Real native popup host opens/closes; source option bindings update horizontal/vertical offsets. | Source acrylic/absolute placement and platform input/appearance. |
| Clipboard | Existing host service performs text copy/paste, format listing and clear; missing service/access-denied outcomes are visible. | Real operating-system clipboard integration, bitmaps/files, history, roaming and source accessibility announcements. |
| Pull to refresh | Existing RefreshContainer requests refresh; a UI dispatcher continuation inserts the source item and completes its native deferral. | Custom visualizer, gestures, pixels and the original ListView contract. |
| Color contrast | Original luminance/contrast calculation, thresholds, symmetry/endpoints and invalid RGB input feedback. | The upstream custom color-picker controls and full accessibility acceptance. |
| Lifetime | Dropping a native page releases weak callback targets. | Broader application and platform lifetime stress testing. |

Mock-renderer construction does not establish bitmap decoding, rendered pixels,
keyboard traversal, screen-reader interoperability or real operating-system service
behavior. Each partial mapping retains that distinction.

## Prior integration rendering: `83623379`

The [successful rendering job](https://github.com/wieslawsoltes/FerroUI/actions/runs/39771549215/job/119800058965)
used real Skia drawing, HarfBuzz shaping and the WinUI theme on Linux with Rust
1.99.1. Its `winui-gallery-rendering` artifact, ID **12423085701**, contains
**134 PNGs: 126 light-theme pages and eight dark-theme pages**. Every PNG decoded
as nonblank, opaque 1280 × 900 content. PNG and decoded-pixel hashes were distinct
for all 126 light frames and all eight dark frames, and logged IDs matched the files.

The exact workflow commands were:

```sh
cargo run --locked -p winui-gallery --features capture --bin winui-gallery-capture -- \
  --all --output-dir=gallery-captures/light
cargo run --locked -p winui-gallery --features capture --bin winui-gallery-capture -- \
  --pages=NumberBox,RadioButton,AutoSuggestBox,InfoBar,InfoBadge,ProgressRing,RatingControl,CompactSizing \
  --dark --query=Button --output-dir=gallery-captures/dark
```

The integration record preserves the artifact/archive hash, render-log hash,
validation-report hash, summary and hashes of the eight visually reviewed CI frames
for InfoBadge, ProgressRing, RatingControl and CompactSizing. The dark run submitted
the AutoSuggestBox `Button` search and decoded its original detail PNG. This run
did not include WebView2 in its dark selection.

The reviewed integration frames retain the known limitations: missing Segoe rating
stars and some icons, CompactSizing's initial **Standard** form, and a static
ProgressRing image that does not establish motion. The results establish construction
and rendering, with no promotion to full-page, upstream pixel or platform UIA parity.
The checked-in PNGs and capture manifest remain the earlier evidence described below.

## Retained rendering before the 0.4.0 integration

The retained native Gallery source checkpoint is
`3c7ceb81dcc4948f374c05c4757c6cb72ed5de5d`. The optional capture executable built
and ran using the existing headless window platform, real Skia drawing, HarfBuzz
text shaping and WinUI theme without a display server. The desktop application
also compiled successfully with its existing desktop, X11, Skia and HarfBuzz host.
These results were published at documentation checkpoint `8d6c895e` before the
0.4.0 integration; no interactive desktop session was claimed.

| Gate | Measured result |
| --- | --- |
| Desktop host before integration | `cargo check --locked --offline -p winui-gallery --features desktop` passed at the earlier checkpoint. |
| Dedicated light capture | InfoBadge, ProgressRing, RatingControl and CompactSizing rendered; four decoded, nonblank 1280 × 900 PNGs with four distinct hashes. |
| Full light sweep | All 126 catalog pages rendered; 126 decoded, nonblank 1280 × 900 PNGs with 126 distinct hashes. |
| Selected dark sweep | All nine requested pages rendered; nine decoded, nonblank 1280 × 900 PNGs with nine distinct hashes. |
| Visual review | Eight retained frames were inspected: InfoBadge, ProgressRing, RatingControl and CompactSizing in both themes. |

All **139 output PNGs** were decoded and checked for pixel variation. The
[capture manifest](../../docs/porting/winui-gallery/captures/capture-manifest.json)
records source, binary and log hashes, every sweep page ID/hash and the retained
image hashes. The four retained light frames come from the separate dedicated run;
they are recorded separately from the same pages in the full sweep. Three differ
in bytes between runs, so the hashes identify these observations rather than a
deterministic pixel baseline for future builds.

The exact pre-integration capture commands were:

```sh
cargo run --locked --offline -p winui-gallery --features capture --bin winui-gallery-capture -- \
  --pages=InfoBadge,ProgressRing,RatingControl,CompactSizing \
  --output-dir=/tmp/ferroui-gallery-current-light
target/debug/winui-gallery-capture \
  --all --query=Button --output-dir=/tmp/ferroui-gallery-current-all
target/debug/winui-gallery-capture \
  --pages=NumberBox,RadioButton,AutoSuggestBox,InfoBar,InfoBadge,ProgressRing,RatingControl,CompactSizing,WebView2 \
  --dark --query=Button --output-dir=/tmp/ferroui-gallery-current-dark
```

The [capture README](../../docs/porting/winui-gallery/captures/README.md) records the
full toolchain/build environment and reviewed frames. The capture host logs each
stable page ID before rendering, fails on native adaptation construction errors,
and rejects an AutoSuggestBox detail image that fails actual bitmap decoding.
Both sweeps submitted the source `Button` search through the real renderer.

The recorded Linux host does not provide Segoe Fluent Icons or Segoe MDL2 Assets.
Rating stars and some icon glyphs visibly appear as missing-font boxes. Their
API/glyph values are retained; installed-font presentation remains pending.
CompactSizing frames show its initial **Standard** form, so compact-mode switching
and value retention are established by native behavior tests rather than these
frames. ProgressRing's static frames do not establish animation behavior. These
captures establish native construction and rendering; upstream Windows pixels,
real platform input, UIA and screen-reader acceptance remain unverified. Rendering
a pending page's source browser does not implement its control.

### Historical frames

Five unchanged captures remain from the **prior `d008845` checkpoint**: NumberBox
light, AutoSuggestBox light, RadioButton dark, InfoBar dark and WebView2's pending
source browser in light. Their commands and hashes remain in the capture README's
historical section. They are separate evidence for that earlier checkpoint and do
not depict the newly added controls or CompactSizing implementation.

### Renderer dependency

The pre-integration capture environment reused the following public rust-skia binary
archive through the build script's existing `SKIA_BINARIES_URL` option. Its local
bytes were checked against this SHA-256 for that run:

```text
https://github.com/rust-skia/skia-binaries/releases/download/0.153.3/skia-binaries-b7f043e0b1e2a850e702-x86_64-unknown-linux-gnu-ganesh-gl-jpegd-jpege-pdf-vulkan.tar.gz
SHA-256: 9ebe4c448cc9f789bad241cbd11739cdf759e27db5719c23a7635e096d5807b3
```

That run used `SKIA_BINARIES_URL=file:///tmp/ferroui-skia.tar.gz`.
This selects the same public dependency artifact; no dependency sources or toolchain
checks were patched. The local build used one Cargo job, no incremental compilation
and disabled development/test debug information to bound resource usage.

## Completion boundary

The sample remains a partial native port with a complete pinned source catalog.
Rendering an existing native adaptation or a pending page's source browser does not
promote that page's status. The current strict completeness gate reports 1,544 API
types, 7,264 API members and all 126 Gallery pages without complete behavioral
execution evidence. Partial contract tests do not satisfy that completion requirement.
Full completion still requires the missing framework and
platform APIs, every original example and code-behind behavior, compiled/deferred
XAML features, cross-platform input and accessibility acceptance, and reviewed visual
comparison against the pinned upstream Gallery.
