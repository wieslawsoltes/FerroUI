# WinUI API compatibility and Gallery port

This work adds native FerroUI controls and an explicit WinUI XAML importer while preserving
the existing Avalonia-derived API, property defaults, resource precedence and control behavior.
The acceptance target is the complete pinned Microsoft WinUI Gallery, including its XAML,
code-behind interactions, navigation, accessibility and platform services. A catalog entry,
an API with the same name, or a successful parse alone does not establish behavioral parity.

## Reference revisions

| Project | Revision | Purpose |
| --- | --- | --- |
| [FerroUI](https://github.com/wieslawsoltes/FerroUI) | `86b6b4ab41669bf830fba681504a4ee94752a4e6` | Starting framework API |
| [Microsoft WinUI](https://github.com/microsoft/microsoft-ui-xaml) | `a71c42bd3e352060e5f5ecf3b5e1a98f8a745809` | Public API declarations and implementation |
| [Microsoft WinUI Gallery](https://github.com/microsoft/WinUI-Gallery) | `14061eeac21c38ae99f302edf3d9136d4f652c4c` | Actual application, page catalog and sample requirements |

The source repositories are read-only references. Their commit IDs and source hashes belong
in every generated inventory so that an upstream update cannot silently change the denominator.

## Architecture

1. **Shared framework services.** Reuse FerroUI's object/property system, collections, layout,
   routed input, text, rendering, styles, resources, bindings, automation, dispatcher and
   platform interfaces. There is one visual tree and one renderer.
2. **Native controls.** Use existing controls directly when their contracts match. Add
   missing general-purpose members without changing existing defaults. Implemented additions
   belong to `FerroUI.Controls` / `ferroui_controls` and the normal presentation namespace.
   A conflicting contract receives a distinct native identity: source `Page` maps to
   `FramePage`, and Frame navigation arguments live in `FerroUI.Navigation`.
3. **Explicit XAML migration.** An opt-in, diagnostic-producing migration step converts
   audited namespace/type/member equivalents. Its output is ordinary FerroUI XAML for the
   existing runtime loader or compiler. Unsupported syntax and semantic differences remain
   diagnostics; no missing property, event or child may be silently discarded.
4. **Native Gallery.** The port is a FerroUI application. It preserves canonical upstream
   page IDs, categories, descriptions and source references. Interactive examples use real
   FerroUI controls. Catalog coverage, executable examples and verified behavior are reported
   separately.
5. **Evidence-driven parity.** Generated source/API/usage maps record the exact upstream
   declaration or use, the target API, status, tests and remaining differences. Strict gates
   reject unresolved requirements; inventories do not masquerade as implementation evidence.

## Compatibility rules

| Concern | Rule |
| --- | --- |
| Existing markup | The existing FerroUI namespace and loader behavior remain the default. |
| Existing Rust API | Existing public types, methods and defaults retain their behavior. |
| Same name, different contract | Use a distinct native identity with an explicit source mapping. |
| `Page` / navigation | Preserve existing page/navigation classes; map source `Page` to native `FramePage` and Frame arguments to `FerroUI.Navigation`. |
| Binding | Preserve source, lifetime and default-mode semantics; `x:Bind` is not a text substitution for `Binding`. |
| Resources | `ThemeResource` must re-evaluate through the framework resource system; static lookup is not equivalent. |
| Styles and visual states | Preserve precedence and state ownership; unsupported state transitions remain explicit. |
| Host services | Route through existing platform abstractions and report actual capabilities. No success-returning stand-ins. |
| Upstream dependencies | Track Windows App SDK, WinRT, community controls and Gallery-local types separately from WinUI XAML. |
| Validation | Compiling, loading, executing, rendering and native comparison are distinct evidence levels. |

## Delivery sequence

- [x] Inspect the framework and pin source revisions.
- [x] Establish the nonbreaking architecture and open a working PR.
- [x] Generate pinned WinUI source-declaration and Gallery requirement inventories.
- [x] Implement the opt-in XAML projection and initial conflict-safe controls.
- [x] Promote implemented additions to normal native control, navigation, primitive and automation namespaces; use them directly in the Gallery.
- [x] Port the complete exported catalog, native shell and a documented interactive subset.
- [x] Add deterministic API/usage maps and automated regression/completeness gates.
- [x] Execute the implemented control, binding, migration and Gallery behavior tests.
- [ ] Complete native rendering and platform comparison acceptance for every original example.
- [ ] Reconcile every remaining requirement and publish a full parity report.

## Related framework work

The existing [WinUI theme](../winui-theme.md), [theme comparison gallery](../winui-theme-gallery.md)
and [WinUI native host](../winui.md) are reusable infrastructure. This port targets the actual
Microsoft Gallery and its API requirements; it does not rename the theme test application.
Active rich-text, repeater, charts and inking work must be accounted for at its actual merged
or branch state. Features in another unmerged PR are dependencies, not implemented mainline APIs.

## Current acceptance state

**Implementation in progress. Full API, behavior, visual and platform parity is not yet
established.** The PR remains draft until the complete requirement map has executable evidence
and all strict acceptance gates pass. Subsequent commits update this record with measured
counts, commands, exact results and limitations.

### Native control organization

The [native-control migration guide](NATIVE-CONTROLS.md) lists all 16 concrete
control mappings, standard builders, helper namespaces and application examples.

The Gallery's reviewed XAML pages use the default FerroUI namespace for NumberBox,
RadioButtons, FontIcon, SymbolIcon, AutoSuggestBox, InfoBar, InfoBadge, ProgressRing,
RatingControl, NavigationView and Frame. Rust code imports these controls directly
from `ferroui_controls`; Frame destination classes derive `FramePage`. Template
settings and InfoBarPanel follow `FerroUI.Controls.Primitives`, and the eight
automation peers follow `FerroUI.Automation.Peers`. The source importer maps the
pinned WinUI identities to these native APIs. There is no separate controls shim.

The [current native-control execution](data/executions/native-controls-008.json)
records **182 passing tests across all 12 integration targets** on the working tree
based on main integration `7fa010c9` (main parent `0526a3ca`), including the Gallery's
new color-picker dependency. Its **53 named source inputs** were frozen before
and checked after all twelve serialized commands. All **521 current API evidence
entries** reference 008. The recorded parent is not presented as the complete
working-tree snapshot. The older [local record 005](data/executions/native-controls-005.json)
and [CI records 006](data/executions/native-controls-006.json) and
[007](data/executions/native-controls-007.json) retain their exact original sources,
commands, logs and boundaries. The exact source guard remains active.

The current source map identifies **320 type candidates and 1,554 member candidates**;
the existing-API guard preserves **3,170 types, 28,435 declared member names and 458
registrations**, and reports **487 additive types** after integrating main. Candidate
matches remain separate from behavioral evidence. The API behavior counts are unchanged:
1 verified type, 54 partial types, 202 verified members and 264 partial members. The
complete Symbol enum accounts for the verified type and 197 verified members.

The theme ledger retains **6,912 ported, 1,974 waived and 87 pending definitions**.
It has its own scope and acceptance gates; native control availability does not
supply missing source templates or Windows visual acceptance.

### Existing controls used by additional Gallery pages

Four additional pages use existing FerroUI APIs directly, bringing the Gallery to
**76 partial pages and 50 pending pages**. No page is marked ported or verified.
Their 14 original source compositions have **13 partial mappings and one pending
SVG example**. The complete curated ledger contains 57 partial example mappings
and that pending mapping, separate from all 355 actual source ControlExamples.

| Page | Native API reused | Original compositions and remaining scope |
|---|---|---|
| ColorPicker | ColorView, ColorSlider, RadioButtons, TextBox and ToggleButton | One interactive options example; explicit alpha/More/hex composition preserves the existing native dropdown ColorPicker and ColorView contracts. Native slider units, component editing, layout and host acceptance differ. |
| ConnectedAnimation | ConnectedAnimationService, Frame and FramePage | Four collection/card/configuration/tile compositions with original media and return flows. Item virtualization, exact XY focus, entrance transitions and acrylic remain pending. |
| Image | Image, Bitmap and drawing primitives | Six source compositions; decode sizing, stretching and fixed nine-grid drawing are supplied. SVG decoding is pending and GIF playback remains static previews with visible diagnostics. |
| FlipView | Carousel, PageSlide and PipsPager | All three actual source compositions, including the bound-data example omitted from the exported catalog. WinUI container/chrome/focus/RTL/transition equivalence remains pending. |

The [native reuse guide](NATIVE-CONTROLS.md#reusing-controls-already-in-ferroui)
and [Gallery implementation notes](../../../samples/WinUIGallery/README.md) describe
these adapters. The source importer maps inline WinUI ColorPicker to ColorView with
an adaptation diagnostic; alpha and More members require an explicit host policy.
The asset manifest retains 22 additional original media files and six source-page
hashes, alongside 108 control images and two explicit case aliases. Rendering CI
requests all 14 compositions in both themes and rejects image decode failures;
those requests are not themselves evidence of a completed render or pixel parity.

### Main integration checkpoint at `7c679ceb`

The tested merge is
[`7c679ceb`](https://github.com/wieslawsoltes/FerroUI/commit/7c679ceb0a2e36c6259286ef47f4cc5d5b7abc70),
which merges main parent `0ce4e90c20d89a03590ccdde555d5daa87e88ee6`.
The separate [main-integration record](data/navigation-main-integration.json)
records the merged source, commands, results and CI job status. The NavigationView
and earlier integration records below retain their original revisions and counts;
their passing results are not presented as reruns on this merged tree.

The checkpoint maps report **1,535 source member candidates**, **204 additive
types** and **562 XAML member candidates**. These are source-comparison and mapping
counts, not newly established behavioral parity.

| Validation | Merged-checkpoint result | Boundary |
| --- | --- | --- |
| WinUI theme | **295 passed, 2 ignored**; the explicit reference regenerator also passed **1 test** | Local validation of the merged theme and complete compiler-reference comparisons. |
| Focused integration rerun | **19 passed, 1 ignored**: Base 8, registration 1, enum-member lookup 1, Controls 5, public paths 3 and Skia 1 | All six commands completed with source equality checked before and after each command. The public-path regeneration task is explicitly ignored. |
| Parity CI at `7c679ceb` | **All three jobs passed** | [Run 39824609248](https://github.com/wieslawsoltes/FerroUI/actions/runs/39824609248): pinned comparison, native/XAML/Gallery contracts and real rendering. Strict completeness remains a separate gate. |
| General CI at `7c679ceb` | Source conventions failed | The subsequent comment-only correction passes all five convention steps locally. The original CI failure remains recorded; it is not relabeled as a pass. |

The integration record keeps this validation separate from the earlier NavigationView
native executions and the 151-image capture record. Their source hashes, manifests and
historical acceptance boundaries remain unchanged.

The downloaded CI rendering archive contains **151 opaque, nonblank 1280 × 900 PNGs**.
Of these, **148 match the historical images exactly**, including all sixteen named
NavigationView frames and both InfoBadge frames. The remaining differences are confined
to the ProgressRing arc in each theme and the CommandBar's three right-side labels,
which move down one pixel without clipping; every other CommandBar pixel is unchanged.
The integration record preserves the archive, image and decoded-pixel hashes and the
focused visual review. These checks do not establish upstream Windows pixel parity.

### Earlier integration and NavigationView inventory

The prior integration checkpoint is
[`83623379`](https://github.com/wieslawsoltes/FerroUI/commit/83623379d8aeaa5a87d0417e7ce0823fc6fe9722),
which merges main at `cfa62aeb` and uses workspace version **0.4.0**. The
[initial Actions run](https://github.com/wieslawsoltes/FerroUI/actions/runs/39771098146)
passed source comparison, but its native-contract and rendering jobs stopped before
compilation because the Gallery's lock entry still named `0.4.0-preview.1`. The
integration aligns that entry with the workspace version. All three jobs in the
[replacement parity run](https://github.com/wieslawsoltes/FerroUI/actions/runs/39771549215)
passed. The [integration record](data/integration-validation.json) separates those
CI results from local reruns and retains exact commands, versions and hashes.

The pinned source inventory contains **1,545 public-source types, 7,466 public-source
members and 198 separately counted private-source types**. The Gallery has **126 catalog
pages, 350 exported examples and 355 actual ControlExample instances**. All 126 pages are
searchable and navigable. The port records **76 partial pages**, including the live
ScratchPad editor, and **50 pending pages**. There are **57 explicit partial mappings
to original examples and one pending SVG mapping**. No complete page is claimed as ported or behaviorally
verified. The generated [report](data/REPORT.md) and [summary](data/summary.json) are the
current ledger.

At the NavigationView checkpoint, source comparison identified **314 type candidates
and 1,534 member candidates**.
These are lexical declarations connected through explicit namespace/member maps;
they do not establish signature, default, behavioral or WinMD ABI equivalence.

The checkpoint's execution-backed API evidence covers **1 verified type, 54 partial types,
202 verified members and 264 partial members**. The verified type is the complete `Symbol`
enum. Its 197 numeric values account for 197 of the verified members; the remaining five
are narrowly tested icon members. These figures do not imply that 202 independently
implemented control operations or any complete Gallery page have been verified.

The NavigationView increment adds native `NavigationView`, its hierarchical item family,
pane/selection/invocation events and two automation peers. The control uses the existing
SplitView, property system, collections, layout, focus, popup and automation services.
The Gallery now contains all eight original NavigationView compositions and the first
InfoBadge example's NavigationView composition. Seven distinct Frame/Page destinations,
source navigation quirks, settings/footer behavior and category models are preserved.
Category templates, adaptive triggers and C# handlers use documented Rust adaptations.

The XAML migration recognizes the new opt-in types and their precise automation-peer
namespace. Plain symbol literals on `NavigationViewItem.Icon` and
`AutoSuggestBox.QueryIcon` become instance-owned native SymbolIcon elements, with an
explicit `WXAML2005` source-range diagnostic. Interpreted and emitted Rust fixtures
exercise the same hierarchy, icons, badge, footer, selection and ownership behavior.

These remain bounded native adaptations. NavigationView top overflow, source template
settings/presenters, selectors, title-bar integration, acrylic/transitions, localization
and raw gamepad input remain unfinished. ProgressRing's custom animated-visual sources
are absent; RatingControl's Windows number
formatting, text-scale/template-resource details and raw platform gamepad mapping remain
unsupported or unverified. RatingControl rejects NaN for `Value` and `PlaceholderValue` as
an explicit native validation exception. CompactSizing retains explicit TextBox/composition
adapters for PasswordBox and Header. Installed fonts, exact pixels and Windows accessibility
acceptance remain separate from the passing native tests.

At the NavigationView checkpoint, the separate theme resource/style ledger recorded
**6,912 ported, 1,986 waived and 75 pending entries**. The pending entries belong to
InfoBadge (27), ProgressRing (8), RatingControl (35) and NavigationView styles (5).
The five NavigationView styles move from waived to pending because their native target
controls now exist; native metadata templates do not establish source-style parity.
These counts concern scoped
source definitions, with the theme's own reference revision; resource lookup support
does not supply a missing source dictionary or establish visual parity. The checkpoint's
theme suite passes after the reference-fixture storage update, with both compiler
outputs still compared against the complete reference bytes.

### NavigationView checkpoint

This increment merges main at `d5405346e680e060c9675572efe02df131428c08`.
Its native contracts are published at `9ce7530c`, XAML migration at `fb2fee44`,
Gallery examples at `31eba1b6`, API evidence at `fdb10276`, theme accounting at
`e32d933f`, and named-control capture support at `8365ed0b`. The
[checkpoint record](data/navigation-validation.json) retains exact commands,
source coverage, successful and superseded attempts, measured results and log hashes.
These are local Linux results on Rust 1.99.0; earlier GitHub CI results below retain
their original revision and are not evidence of a new run on this increment.

| Validation | Measured result | Scope |
| --- | --- | --- |
| Native WinUI controls | **182 passed** across 12 serialized targets | Includes **32 NavigationView + 23 item/hierarchy tests** and all 127 earlier native tests. The seven new implementation/test files were hash-checked before and after each target. |
| Gallery library | **54 passed** | All eight NavigationView compositions, seven destination factories, source navigation quirks, hierarchy, category models, options and all four InfoBadge compositions. |
| XAML projection | **44 migration tests; 7 compiled/interpreted tests passed**, with one explicit fixture-regeneration test ignored | Opt-in type/peer namespaces, symbol conversion diagnostics, shared hierarchy, ownership and emitted/runtime agreement. |
| Themes and public paths | **294 WinUI theme tests passed** with two explicit ignored tasks; **two existing-theme fallback tests passed**; **3 Rust-path tests passed** with one explicit ignored regeneration task | Earlier local checks are recorded separately from the final frozen native run. Both complete compiler-reference comparisons remain active. |
| Declarative builders | Compilation and generated-output freshness passed | Existing and opt-in namespaces remain available. |
| Source comparison | **44 scanner tests passed**; all freshness/preservation gates passed | Preserves **3,170 existing types, 28,435 declared member names and 458 registrations**; **203 additive types** are reported separately. |
| Real rendering | **151 PNGs**: 126 default light pages, 9 default dark pages and all 8 NavigationView compositions in both themes | Skia/HarfBuzz rendering with native scrolling; all outputs decode as opaque, nonblank 1280 × 900 images. This is viewport construction/rendering evidence. |
| Strict completeness | **Expected exit 2** | 1,544 types, 7,264 members and all 126 pages still lack complete behavioral evidence. |

The [fourth native execution record](data/executions/native-controls-004.json)
records the 55 new passing test names, exact per-target commands, source hashes and
all 182 native results. The [new capture manifest](captures/navigation-capture-manifest.json)
records every output, actual command, log, binary and source hash. Eighteen reviewed
PNGs are retained: the eight NavigationView compositions and InfoBadge in both
themes. The [capture review](captures/README.md) documents compact versus expanded
panes, the initial collapsed hierarchy and unselected API example, source-constrained
content widths, inner scrolling and missing Segoe glyphs. Static captures do not
establish motion, keyboard interaction, popup behavior, upstream pixels or Windows UIA.

### Prior validation at `83623379`

The following table retains the earlier checkpoint's measured results. The
NavigationView checkpoint is recorded separately in the Gallery validation ledger.

| Validation | Prior evidence | Boundary |
| --- | --- | --- |
| Existing FerroUI API | 3,170 namespaced types, 28,435 declared member names and 458 source registrations preserved against the starting commit; 181 additional types | 82 additions belong to this compatibility branch and 99 came from merged main; source declarations do not establish full signature/default/behavior compatibility |
| Native WinUI controls | 127 tests pass in CI and locally: 77 existing tests plus 13 InfoBadge, 16 ProgressRing and 21 RatingControl tests | Native properties, templates, input, clock lifetime and automation; every new control type remains partial |
| Base property/framework regression | 4,383 tests pass locally; the 13-test local-value subset also passes in CI and locally | Includes the additive local-value query; existing clearing and priority semantics are preserved |
| WinUI binding lifecycle | 13 tests pass in CI and locally | Root/named-element and typed-template property paths with explicit host initialization; unsupported expressions remain diagnosed |
| XAML migration and runtime | 41 integration tests and three deferred-resource error-isolation tests pass in CI | Includes native feedback-control construction, audited InfoBadge style mapping, nested error scopes and unchanged ordinary panic behavior |
| Compiled and interpreted XAML | Five focused tests pass in CI, with one explicitly ignored regeneration task | Real emitted Rust compiles and agrees with interpreted root and typed-template property bindings; standalone DataTemplate document roots are outside the emitter contract |
| Native view builders | `cargo check --locked -p ferroui-view` passes in CI | Generated typed builders compile; this does not establish every builder's runtime behavior |
| Native Gallery | 42 tests pass in CI | Catalog, original-example interactions, local compact resources, assets, lifetime and mocked host-service contracts |
| WinUI theme | 294 tests pass in CI, with two explicitly ignored regeneration/timing tasks | Includes exact joined-reference comparisons and four reference-storage tests; source-resource and visual gaps remain explicit |
| Native rendering | Integration CI renders all 126 light-theme pages and eight dark-theme pages | All 134 PNGs decode as nonblank, opaque 1280 × 900 frames; 126/8 distinct PNG and pixel hashes. Eight CI frames were reviewed; missing Segoe glyphs remain visible |
| Desktop host | `cargo check --locked --offline -p winui-gallery --features desktop` passes locally | Interactive desktop, upstream pixel comparison and Windows accessibility remain unverified |
| Source comparison | 44 scanner/comparison regressions and two theme path-case tests pass; API, event, symbol, builder, theme, catalog and asset freshness gates pass | A source candidate is never automatically promoted to behavioral parity; the strict completeness gate remains closed |

CI used Rust 1.99.1; local reruns used Rust 1.99.0. The
[Gallery validation ledger](../../../samples/WinUIGallery/VALIDATION.md) records exact
commands, source hashes, assets, rendering evidence and platform limits. Durable native
control executions are stored in [the execution records](data/executions/). The
[third record](data/executions/native-controls-003.json) for that prior checkpoint contains the 50 new focused test
names and source hashes from the original passing 127-test run. It remains unchanged;
the integration record documents subsequent reruns. Any supplied execution record is
checked for freshness even when its API entry is only partial.

The earlier retained capture files predate the 0.4.0 integration and use native source
checkpoint `3c7ceb81dcc4948f374c05c4757c6cb72ed5de5d`. The unchanged
[capture manifest](captures/capture-manifest.json) keeps the four dedicated light
captures separate from the 126-page light sweep and nine-page dark sweep. The eight
retained frames show InfoBadge, ProgressRing, RatingControl and CompactSizing in
both themes; CompactSizing is captured in its initial Standard state. Five retained
`d008845` frames remain explicitly historical. That integration's CI artifact `12423085701`
has a separate 126-light/eight-dark scope recorded in the integration record.

## Review and run

- [API design and remaining contract obligations](API-DESIGN.md)
- [Native control contracts and execution boundaries](CONTROL-CONTRACTS.md)
- [Native Gallery, run commands and page boundaries](../../../samples/WinUIGallery/README.md)
- [Source scanner, evidence format and regeneration](../../../scripts/winui-gallery/README.md)
- [Reviewed API mappings](data/api-mappings.json) and [generated use-site mappings](data/gallery-xaml-map.json)

```sh
cargo run -p winui-gallery --features desktop -- --page=NumberBox
cargo test --locked --offline -p winui-gallery --lib
cargo test --locked --offline -p ferroui-base \
  --features testing --lib
cargo test --locked -p ferroui-markup-xaml --test winui_bindings
cargo check --locked -p ferroui-view
cargo test --locked -p ferroui-markup-xaml-loader --test winui_migration
cargo test --locked -p ferroui-markup-xaml-loader --features emitter --test winui_compiled
cargo test --locked --offline -p ferroui-themes-winui --lib
cargo check --locked --offline -p winui-gallery --features desktop
cargo test --locked --offline -p ferroui-controls \
  --features testing \
  --test winui_navigation --test winui_number_box --test winui_icons --test winui_radio_buttons \
  --test winui_auto_suggest_box --test winui_icon_sources --test winui_info_bar \
  --test winui_info_badge --test winui_progress_ring --test winui_rating_control \
  --test winui_navigation_view --test winui_navigation_items
python3 -m unittest discover -s scripts/winui-gallery -p 'test_*.py'
python3 scripts/winui-gallery/scan.py --check
python3 scripts/winui-gallery/existing_api.py --check
python3 scripts/winui-gallery/event_targets.py --check
python3 scripts/winui-gallery/symbol_map.py --check
```

The `WinUI and Gallery parity` workflow runs freshness, existing-API, native contract,
compiled-binding and full-catalog rendering checks. Its
separate `require_complete` workflow-dispatch option runs
`python3 scripts/winui-gallery/scan.py --check --require-complete`; this gate intentionally
fails while API contracts or complete Gallery pages lack current behavioral execution evidence.
The current strict run reports 1,544 types, 7,264 members and all 126 Gallery pages
without complete behavioral execution evidence; partial contract tests do not satisfy
that full-completion requirement.
