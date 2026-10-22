# Rich text verification

## Layout, input geometry, range Gravity and scope integration

The coordinated Linux build uses Rust 1.99.0, locked/offline dependencies, one
build job, and disabled incremental/debug artifacts. All 14 selected crate
test targets compile in one feature graph. The measured runtime results are:

| Gate | Result |
|---|---:|
| Base, full | 4,437 passed |
| Controls, full, with loopback for remote tests | 5,136 passed |
| XAML, full runtime/emitter corpus | 620 passed; 17 existing ignored |
| XAML runtime helpers, full | 202 passed |
| Browser portable adapter, full | 278 passed |
| WinUI / Win32 portable host suites | 50 / 329 passed |
| View, full | 73 passed |
| IBus on a private session bus | 35 passed |
| Native / Wayland host suites | 6 / 118 passed |
| Android / iOS host suites | 165 / 102 passed |
| HarfBuzz, full | 8 passed |

Controls includes 23 independent-range Gravity tests, nine line-stacking tests,
ten TextLineBounds tests and 13 requested-range geometry tests. The Gravity
families replay 45 literal native affinity fixtures, 190 retention commands and
24 invalid values. The geometry capability adds three Base and eight portable
CoreText cases. Five Windows-only native geometry tests require the subsequent
Windows workflow; the host counts do not execute them. Backend/View counts
precede the final XAML-only scope-chain correction; final full XAML, runtime
helpers and Controls gates cover that correction separately.

Initial failures led to concrete corrections. Browser initially passed 271 and
failed two: accepting empty composition incorrectly took legacy cancellation,
and a tail-format fixture queried an insertion boundary. Controls initially
passed 5,132 and failed four fixtures: font-manager scope, enum metadata access,
already parented overflow and Name property registry lookup. Intermediate runs
also exposed six loopback permission failures. Full XAML initially passed 618
and failed two actual template outer-scope lookups; the weak scope-chain fix
keeps those original assertions. The corrected gates above have zero failures.
Earlier compilation attempts also encountered fixture visibility/import/type
errors and disk/memory limits; they are not counted as test passes.

The actual pinned C++ line-stacking function and Rust helper agree in all
**960 formula cases**, with maximum absolute difference `2.7791000007937328e-6`.
View builder freshness and generated source/API tracking pass. The inventory
remains 640 selected files, 1,362 exact API identifiers, 938 framework dependencies
and 827 include-boundary edges. Public states are 159 complete, 654 partial and
549 pending; no runtime source file is marked complete. These are scoped
implementation and traceability gates, not external WinUI engine or pixel parity.

## Published checkpoint 8508220d

Both jobs in the [rich-text workflow](https://github.com/wieslawsoltes/FerroUI/actions/runs/39825948407)
passed. The [Linux job](https://github.com/wieslawsoltes/FerroUI/actions/runs/39825948407/job/119989156361)
passed Base 4,433, Controls 5,073, Win32 329, WinUI host 42, Fluent 210
(two ignored), Simple 213 (three ignored), WinUI theme 301 (two ignored), and
XAML 617 (17 ignored). All five explicit generated-reference comparisons
passed, including the corrected Typography Rust-path row. The
[Windows job](https://github.com/wieslawsoltes/FerroUI/actions/runs/39825948407/job/119989156686)
passed WinUI 52, exact UTF-16 two, IMM32 scopes 23, native MSFTEDIT seven,
and the selected rich-control family 609. This evidence applies to the published
checkpoint before the next layout, geometry, namescope and range Gravity changes.

The immutable [actual-selection Gravity record](native-msftedit-selection-gravity.json)
adds **250 commands across 18 fixtures** and 18 canonical interface-identity
records. The measured checkout is `038c29ce3cc8491b5b4a6fe8ef22eb8624ecdfec`,
workflow head `8508220d7f2323acfb32293d7d0afeb7abf75bfd`, probe blob
`7d80afb5ee5b62e301a09f574d508a603efb411c`, artifact ID 12448130293.
Its ZIP SHA-256 is
`0b71f796c5c86db34fe23d9f37819f469992c45a8e3adbba365ebc02e83e5793`,
matching GitHub's artifact digest. The engine is MSFTEDIT 6.2.26100.33648 on
Windows 10.0.26100. All previous fixture declarations, counts and six observation
families match the .33438 link/gravity record exactly. The strict extractor and
all **34 Python tests** pass, including reconstruction of the new record.
Native characterization and Rust selection implementation remain separate gates;
system MSFTEDIT is not the external WinUI text engine.

## Source and tracking integrity

The inventory command reads the pinned Git tree. `--check` recomputes source
hashes, selected files, include edges, public member identifiers, and generated
reports, compares them byte for byte, and verifies recorded Rust targets and test
paths. A passing inventory check establishes traceability; it establishes no
runtime parity on its own.

```sh
python3 scripts/porting/winui-rich-text-inventory.py --reference /path/to/microsoft-ui-xaml --check
```

## Upstream test families to adapt

All paths below are relative to the pinned reference and appear in the source
inventory when selected. Preserve each original test's contract and assertions;
rewriting a test to assert the current implementation is not a parity check.

| Reference family | Coverage needed |
|---|---|
| `dxaml/xcp/components/text/unittests/TextHighlightMergeUnitTests.*` | Region precedence, splitting/merging and selection/highlighter overlaps. |
| `dxaml/xcp/components/text/unittests/TextBoxHelpersUnitTests.*` | Text navigation/classification helpers and boundary behavior. |
| `dxaml/test/native/external/foundation/text/RichEditBoxTOMTests.*` | RichEdit document/range/selection/format APIs, loading/saving, links and object-model geometry. |
| `dxaml/test/native/external/foundation/text/RtblHasContentOverflow.*` | Content overflow after formatting/size/content changes. |
| `dxaml/test/native/external/foundation/text/TextBidiTests.*` | Mixed-direction runs, paragraph flow, selection and rich/overflow control behavior. |
| `dxaml/test/native/external/foundation/text/TextBoxSelectionTests.*` | Programmatic and user selection, selection colors and focus effects. |
| `dxaml/test/native/external/foundation/text/TextBoxPointerTests.*`, `TextBoxkeyInput.*`, `TextBoxScrollingTests.*` | Pointer/key commands, caret positioning, scrolling, clipping and input integration. |
| `dxaml/test/native/external/foundation/text/HyperlinkInTextTests.*` | Link selection, navigation, focus, style and pointer behavior. |
| `dxaml/test/native/external/foundation/text/TextControlsRenderingTests.*`, `TextControlsSizingTests.*`, `TextFontFallBack.*` | Font fallback, constrained sizing, scale, glyph/raster and rendering regressions. |
| `dxaml/test/native/external/foundation/graphics/rendering/RichTextBlockTests.*`, `TextHighlighterTests.*` | Multi-paragraph layout, inline UI, formatting invalidation, highlights and visual reference comparisons. |
| `dxaml/test/managed/controls/RichTextBlock/`, `dxaml/test/managed/controls/TextBox/TextEditingTests.cs` | XAML-facing control properties, content, events and end-user editing flows. |

The generated snapshot inventories test source, not the native WinUI test runner
or every external Windows integration prerequisite. Original image masters are
reference evidence whose applicability depends on fonts, scale, OS and rendering
configuration. They are not assumed portable golden images.

## Rust test requirements

Document tests cover UTF-16 offsets around astral characters and combining
sequences; malformed ranges; run/paragraph boundary insertion and deletion;
format preservation; collection ownership/identity; structural positions and
gravity; snapshot consistency; notification reentrancy; and undo/redo.

Layout tests use fixed test fonts/shapers and cover wrapped mixed-format and bidi
paragraphs, zero/finite/infinite constraints, line spacing and indentation,
selection/hit-test geometry, overflow continuation, changes in linked controls,
embedded UI, and invalidation after inherited property changes. Fixed shapers
exercise deterministic geometry; separate real-font tests exercise shaping.

Parser tests include nested destinations, escapes, Unicode/fallback handling,
font/color tables, paragraph state, truncated input, oversized counts, bounded
nesting, and supported-content round trips. Unsupported content must have an
explicit result or preserved representation.

Control tests need a root and dispatcher scope, routed keys/pointers, read-only
commands, focus and selection changes, clipboard providers, template application,
runtime/compiled XAML, IME composition/commit/cancel, and no ownership cycles.
Automation tests need real text/range providers; an `IValueProvider` alone is
insufficient rich-text accessibility coverage.

## Recorded runs

Authoritative command/result records are stored in `mapping.json` under
`validation`, including the checkpoint they cover. Generated [PROGRESS.md](PROGRESS.md)
shows those records. A later test definition or source change is not implicitly
covered by an earlier successful run.

### Main TextLineBounds integration checkpoint

Main `0ce4e90c20d89a03590ccdde555d5daa87e88ee6` adds the shared
TextLineBounds enum, cap-height metrics, formatter support, TextBlock property,
and corresponding WinUI theme changes. Merge resolution keeps the rich tab-stop
configuration and line-spacing copy behavior alongside the new paragraph mode.
The authoritative WinUI reference regeneration passed **one test**, then the
fresh full WinUI theme binary passed **301 tests with two ignored**. This includes
the updated templates, actual text bounds in themed controls, both emitter paths,
and deterministic reference-part storage. The existing View-builder consistency
check passed. Rust-path generation added a missing Typography class entry.

The commands were `cargo test --locked --offline -p ferroui-themes-winui --lib
tests::compiled_xaml_tests::regenerate_compiled_xaml -- --ignored --exact`, followed
by the freshly built theme binary without a filter. These results cover the merge
checkpoint, before the separate rich layout, requested-range geometry, namescope,
and range-gravity tranches. They do not replace those tranches' runtime gates.

### Typography, input backends and snapshot restoration

The fresh core build after Typography and the registration repair passed **27
Typography tests**: one Base, 21 Controls, two HarfBuzz and three XAML cases.
The separate full Base executable passed **4,425 tests**, with no failures or
ignored tests. That gate covers the earlier InputScope list registration and
Capacity failures.

After regenerating the two XAML references and correcting the corpus count to
130, the focused emitter suite passed **36 tests** with six ignored. The full
XAML suite passed **617 tests** with 17 ignored. The initial emitter run had
35 passes and one stale-count failure; all 130 object trees already matched.
Both explicit XAML generators passed. View builder regeneration additionally
adds all 43 Typography attached-property builder methods; its `--check` passes.

The main merge `74fa8217` retains the complete 17,290,127-byte WinUI generated
reference in five deterministic source parts. Concatenating them yields the
original generator blob `97c8d50ce2f15b71dc23d37083f78b143897610a`. The actual
theme gate passed **six tests** with one ignored: both emitter comparisons and
four reference-storage checks. Production still generates one source file.

Full Controls testing exposed two related hyperlink snapshot failures in
`c43a907c`: constructor current values were subsequently restored as local
assignments. The correction restores assigned format exactly, then reinstates
missing intrinsic values at current-value priority. A regression checks all
four semantic containers, fresh/reused nodes, inherited parent formatting,
application styles and explicit overrides. An older case-conversion fixture
now checks exact input-snapshot preservation and effective formatting separately.
The final fresh executable at the correction published as `17298a56` passed
**5,068 tests, zero failures or ignored tests**, with loopback access. Earlier
failed gates and the separate socket-only rerun remain in `mapping.json`.

The [Windows job at c43a907c](https://github.com/wieslawsoltes/FerroUI/actions/runs/39807011198/job/119922819095)
passed native adapter compilation, **52 WinUI tests**, **two exact UTF-16 tests**,
**23 IMM32 input-scope tests** and **six MSFTEDIT probes**. The IMM32 suite includes
21 portable policy/lifecycle/editor cases and two Windows-only SDK/real-HWND
cases. It does not inspect an onscreen keyboard or installed IME. The separate
full Controls and broad repository workflows at this checkpoint were not green.
The Linux bridge step passed 329 Win32 and 42 WinUI-host tests; later theme and
multiple Linux/macOS builds failed to compile an IBus callback whose generated
signal parameter needed an explicit type. The annotation repair has its own
subsequent backend gate; a passing WinUI reference subset does not hide that
broader build failure.

The immutable [link/gravity record](native-msftedit-links-gravity.json) contains
**137 Link commands / 59 fixtures** and **1,577 Gravity commands / 199 fixtures**,
plus the unchanged 316 typing, 300 horizontal, 290 vertical, 60 Home/End and
60 repeat observations. The measured checkout is `dc99ba00`, workflow head
`c43a907c`, probe blob `4accd7fd`, artifact ID 12439885287. Its downloaded archive
SHA-256 matches GitHub's digest. This run measured **MSFTEDIT 6.2.26100.33438** on
Windows 10.0.26100; the earlier typing record measured a different DLL build,
6.2.26100.33648. Every earlier fixture declaration and movement/typing row still
matches. The strict extractor and all **23 Python regression tests** pass,
including round-trip of the new record. Observing Link/Gravity does not itself
implement those APIs or establish external WinUI engine conformance.

The Browser bridge at `b563a5d0` passed 13 TypeScript text-input tests and four
input-option tests, plus typecheck, lint and production build. An isolated
harness compiling the actual projection/edit Rust source passed 15 cases.
Those gates precede the full Browser crate gate and do not exercise a real
browser virtual keyboard. Native/macOS optional-interface generation, old-vtable
identity, imported patch reproduction and public API/availability checks passed
as static checks; portable tests and the actual AppKit executable need their own
runtime results. IBus private-bus, Native, Browser and other backend results are
recorded separately as they complete.

Two independent source audits pass: all 43 Typography defaults and declarations,
86 accessors, 38 enum choices and 69 feature-add sites; all 11 FontWeights
values, typed fields and named parser entries. They compare pinned upstream
objects with Rust source. They do not execute native font selection or shaping;
the two actual Inter-font HarfBuzz cases above provide the limited shaping gate.

### Earlier integration checkpoints

The earlier completed [rich-text workflow at d148193d](https://github.com/wieslawsoltes/FerroUI/actions/runs/39618880985)
passed both jobs. The [Linux job](https://github.com/wieslawsoltes/FerroUI/actions/runs/39618880985/job/119286684017)
passed Base **4,413**, Controls **5,002**, Win32 **308**, WinUI host **39**,
Fluent **210** (2 ignored), Simple **213** (3 ignored), WinUI theme **296**
(2 ignored), and XAML **611** (17 ignored), with zero failures. The separate
doctest had no executed tests and one ignored. All five explicit reference
generators and the exact checked-in output comparison passed.

The matching [Windows job](https://github.com/wieslawsoltes/FerroUI/actions/runs/39618880985/job/119286684228)
passed native adapter compilation, **49 WinUI tests**, **2 exact HSTRING tests**,
**3 native TOM probes** and **561 rich Controls tests**, with zero failures or
ignored tests in those suites. The [new immutable native record](native-msftedit-navigation-horizontal.json)
contains 60 Home/End + 60 repeats + 282 vertical + 220 horizontal commands on
MSFTEDIT 6.2.26100.33648 / Windows 10.0.26100. All earlier Home/End and vertical
observations are unchanged. This successful workflow covers focus handoff and
independent TOM clipboard work, and precedes the current InputScope object,
backend, horizontal movement and extended-column changes.

The separate [broader workflow](https://github.com/wieslawsoltes/FerroUI/actions/runs/39618880971)
had one macOS ControlCatalog failure: its expected page count was still 75
after adding RichText, while the actual collection had 76. That fixture result
does not make the dedicated rich-text workflow fail, and the passing dedicated
workflow does not imply that every repository job passed.

At the [probe/extractor checkpoint 4c0bae3](https://github.com/wieslawsoltes/FerroUI/commit/4c0bae33e9d1d1564cd1bfd0ff2d4c2f13133b75),
`python3 scripts/porting/test_rich_text_tom_observations.py -v` passed **7 tests**.
The historical records round-trip unchanged, including a byte-identical
re-extraction of the actual d148193d log. This gate validates parser behavior,
not the newly added native commands.

In the uncommitted feature working tree after merge `1f79352`,
`cargo test --locked --offline -p ferroui-base --lib input_scope` passed
**11 tests**, with zero failures and 4,413 filtered out. Those are nine
InputScope object/value cases and two immutable snapshot manager cases.
Browser webapp dependency installation, type checking, lint and production
build also exited zero. `npm run test:input-options` passed **4 tests** against
the actual TypeScript attribute bridge with guarded input test doubles. These
cover composing value/selection/type/focus preservation, cleared hints,
sensitive options and unnecessary DOM writes. They do not exercise a real
browser virtual keyboard or OS composition session. These results belong to the
pending feature source, not the clean merge commit; committed provenance will
be added after publication.

The focused Controls gate (`cargo test --locked --offline -p ferroui-controls
--lib rich_edit`) subsequently passed **414 tests**, with zero failures/ignored
and 4,609 filtered out. This includes current rich owner InputScope integration,
216 native horizontal observations and geometry/callback safeguards. It precedes
the final mixed-mode column correction and its additional native replay cases.

The [Windows job at the main merge1f793521](https://github.com/wieslawsoltes/FerroUI/actions/runs/39765696678/job/119780463926)
passed native compilation, **49 WinUI**, **2 exact UTF-16**, **4 native probes**
and **561 rich Controls** tests. Its [immutable record](native-msftedit-navigation-options.json)
contains 300 horizontal, 290 vertical and 140 options/typing commands, plus the
60 Home/End commands and60 repeats. All previous movement rows match. This
completed job covers the published merge; it predates the uncommitted features.
The subsequent [Windows job at 394c11ab](https://github.com/wieslawsoltes/FerroUI/actions/runs/39768437133/job/119789629660)
also completed successfully: 49 WinUI tests, two exact UTF-16 tests, four native
probes and 561 rich Controls tests passed after native adapter compilation.
Its [316-case typing record](native-msftedit-typing.json) adds 176 Unicode and
paragraph typing observations; all earlier options and movement rows match.
The matching Linux job passed Base, Controls and host bridge steps before a
subsequent push cancelled it during theme testing; this is not a full green run.

Native provenance now distinguishes the workflow head from the checkout that
actually ran. The earlier immutable JSON files' `provenance.commit` records the
workflow head; those runs did not independently capture the synthetic pull-request
merge checkout. Their verified probe blobs and observations remain valid.
The new typing artifact captures checkout `dfc982dfdb88de5a02e8f531b639e12f56031eed`,
workflow head `394c11ab1d7767d563c79eff8951adf895c3876b`, probe blob
`059c06f53b18b2c2b7d9242b49ef70720abecd65`, and artifact ID 12421956538. The
downloaded archive SHA-256 matches GitHub's published digest. Future CI uploads
the raw log and measured checkout/probe metadata before the full control suite.

The first full XAML feature gate had **611 passes, 3 failures and17 ignored**.
The failures exposed a stale combined-corpus count, the framework's ignored
empty-attribute behavior, and an unnecessary InputScopeName type-converter hook
that broke root text content. The fixes are published in
[`e02522cb`](https://github.com/wieslawsoltes/FerroUI/commit/e02522cb8b5ba6e4b6412afd7ffdfe6df4dd3273).
After correction, all three InputScope runtime XAML tests, the differential
transform across all 129 corpus inputs, reference regeneration, and the
checked-in freshness test passed. The two View InputScope builder tests and
11 Base InputScope tests also passed. These focused repair gates do not replace
a new full XAML or Base run.

The final movement source published in
[`f10fb838`](https://github.com/wieslawsoltes/FerroUI/commit/f10fb8381e78eb581edc2fe4ba192179f5969a80)
passed `cargo test --locked --offline -p ferroui-controls --lib rich_selection_`:
**70 passed, zero failures/ignored, 4,955 filtered out**. It includes 294 literal
horizontal native observations, six explicit terminal-EOP differences, eight
mixed vertical steps, geometry cost checks and reentrant callback safeguards.

The subsequent [full Base CI gate at 01d06a36](https://github.com/wieslawsoltes/FerroUI/actions/runs/39770623181/job/119797112590)
reported **4,423 passes and one failure**: the new InputScope Names list metadata
does not yet expose the common list `Capacity` property. Full Controls passed
5,007 tests and the host bridge steps passed 308 Win32 and 42 WinUI tests. The
collection metadata correction is published in `182a7e04` using the standard
checked list-capacity helpers; its runtime validation is still pending. Browser Rust and
the other platform policy/runtime gates remain separate.

The broader CI run at the same source passed 164 Android host tests, including
all six new InputScope tests, and failed one legacy Password expectation. The
expected value omitted the privacy hint now retained independently of keyboard
purpose. Its iOS simulator compile separately found an early return inside an
Objective-C retained-return getter. Commit `1be7f087` corrects the expectation
and uses the required tail expression; their next runtime/native compile gates
remain pending. The local combined backend gate reached Skia's locked binary
download before sandbox networking stopped it; no backend test pass is inferred.

The earlier checkpoints below retain their original scope and chronology.

The [completed full workflow at 41ea4f1b](https://github.com/wieslawsoltes/FerroUI/actions/runs/39604251320)
passed both Linux and Windows. Linux totals were Base 4,412; Controls 4,915;
Win32 308; WinUI 31; Fluent 210 with two ignored; Simple 213 with three ignored;
WinUI theme 296 with two ignored; XAML 605 with 17 ignored. All five reference
generators and the exact file comparison passed. Windows passed adapter
compilation, 35 WinUI, two exact HSTRING, two native probe and 474 rich Controls
tests. Those results describe the preceding published checkpoint.

At `92140e7c`, the source-identical public harness passed **203 tests with zero
failures/ignored** against matching rebuilt ordinary libraries. The final scoped
Cargo build completed the Controls artifact, preserved Base's exact hash/mtime,
and was stopped before downstream compilation. It is not a full Cargo no-run
claim. Exactly three existing private startup tests remain excluded whole from
the local harness and retained in repository CI.

The coordinated CoreText gate passed **15 portable tests**. At that checkpoint,
six new Windows-only layout-adapter tests still required a native run. The full XAML suite
passed **611 tests with 17 ignored** before the final runtime geometry changes;
after rebuilding that graph, **18 relevant XAML checks passed**. Rust-path and
View-builder generation checks and Simple/Fluent reference comparisons passed.

Pre-fix regressions independently demonstrated three metadata receiver errors,
two same-view column losses, an RTL structural-marker geometry defect, stale
cursor publication, reflected popup-edge placement, ordinary dismissal session
retirement and redirected story-edge count error. Their corrected repository
fixtures are included in the completed gates. A separate template-replacement
probe reproduced connection retirement after that 203-test checkpoint. It became
a public regression and is repaired in the later 235-case checkpoint below.

The [new full workflow at 92140e7c](https://github.com/wieslawsoltes/FerroUI/actions/runs/39612123175)
is separate. Its Windows job finished with two compile errors: the native
LayoutRequest.IsCanceled getter returns ABI `u8`, and two new predicates expected
Rust `bool`. The narrow fix used explicit nonzero/zero comparisons and was
validated by the later 17a84805 run below. Independent later steps passed two exact HSTRING, two native
probe and **529 rich Controls tests**, with no failures/ignored in those tests.
The Linux job completed successfully: Base 4,413, Controls 4,970, Win32 308,
WinUI host 39, Fluent 210/2 ignored, Simple 213/3, WinUI theme 296/2, XAML
611/17, and the separate ignored doctest. All five reference generators and
exact checked-in-file comparisons passed. Neither this Windows job nor the six
new Windows-only layout tests passed at 92140e7c.

The subsequent [focus handoff](https://github.com/wieslawsoltes/FerroUI/commit/b16a5ce14a7bdebfed0752254f5b5d2eb9c3db42)
and [TOM clipboard](https://github.com/wieslawsoltes/FerroUI/commit/132c7853a17553e475e842d8c7c3327a60a37e6e)
source passed **235 original public tests, zero failures/ignored**, including
four focus and 28 clipboard cases. Both ordinary Base variants and Controls
were rebuilt in the established feature graph; hashes were unchanged during
build and execution. Cargo completed the Controls artifact in 222.97 seconds
and was intentionally stopped before downstream linking. The direct harness
then ran in 2.88 seconds. This is scoped compilation plus source-identical
runtime evidence, with the same three whole private-startup exclusions.

Before the focus repair, the same four public fixtures produced two expected
remount failures and two cancellation passes against the old libraries. After
the 235 gate, five separate native-column fixtures produced three expected
failures (15 versus 12, 14 versus 10, and -3 versus -2) and two passes against
those exact libraries. They establish the next correction's starting point;
they are not included in the 235-test passing claim.

The subsequent [Windows job at 17a84805](https://github.com/wieslawsoltes/FerroUI/actions/runs/39615974131/job/119276997549)
completed successfully: native adapter compilation, **49 WinUI tests**, both
exact HSTRING tests, both native probes and **529 rich Controls tests** passed,
with zero failures/ignored. The six new Windows-only layout ownership fixtures
therefore executed successfully. They are adapter callback fixtures, not real
OS candidate/proofing sessions. This native run precedes the focus/clipboard
source checkpoint and InputScope/extended-column work.

The [matching Linux job](https://github.com/wieslawsoltes/FerroUI/actions/runs/39615974131/job/119276997010)
also passed: Base **4,413**, Controls **4,970**, Win32 **308**, WinUI host **39**,
Fluent **210** (2 ignored), Simple **213** (3 ignored), WinUI theme **296**
(2 ignored), and XAML **611** (17 ignored); zero failures. The separate doctest
had zero executed tests and one ignored. All five explicit generators passed
and the final checked-in reference comparison was clean. This makes the full
17a84805 workflow green without extending its claim to later source changes.

### Earlier checkpoints

The restored core checkpoint
[`38c934480ca20192746a363670c12eac47d67edf`](https://github.com/wieslawsoltes/FerroUI/commit/38c934480ca20192746a363670c12eac47d67edf)
passed `cargo test --locked --offline -p ferroui-controls --lib rich_ --no-fail-fast`:
**97 passed; 0 failed; 0 ignored; 0 measured; 4426 filtered out**. That run includes
21 document, 45 editor, 16 view and 15 portable automation tests. The subsequently
added view/highlighter tests, four portable follow-ups, native bridge tests and
markup/catalog tests need their own later records.

The continuation checkpoints added the following completed runs (the authoritative
records and exact revisions are also in `mapping.json`):

| Checkpoint | Command scope | Result |
|---|---|---|
| `871d566d25aab3e89021d7fc5db6edf32c07267e` | controls `--lib rich_` | 181 passed; 0 failed; 4434 filtered out. |
| `f1a051f8eaddd2d9b651a731206eebd9fa3a252a` | Win32 `--lib text_` | 34 passed; 0 failed; 234 filtered out, including ten text pattern/range bridge cases on the Linux-hosted test harness. |
| `92551b30a05c4766594efb6c112e56cdf3464f1a` | controls `--lib rich_` | 246 passed; 0 failed; 4434 filtered out, including persistent presenter/scrolling, streams, native-valued units, live-pointer batching, language and automation owner/geometry tests. |
| `ed836e831fc8af5114e2ce8ae7632867c7a2332f` | full controls `--lib`; Win32 `--lib text_`; source scanner regression | 4706 controls, 41 Linux-hosted Win32 and one source-scanner test passed. The full controls run had loopback access. Exact scope and the equivalent post-run type-registration annotation are recorded in mapping.json. |

The new paragraph tests cover all numeric spacing rules, point conversions,
actual line/overflow behavior, signed indents and RTF sl/slmult ordering. The
27 focused document-service tests include the adapted UseLfAndUseCrLf fixture;
all other additions are independent contract regressions. The control additions
exercise cancellation/reentrancy, deferred notification lifetime, real content
presenters, navigation and IME state. Accessibility additions exercise caret
focus, stable weak hyperlink peers, TextChild, native marshalling and overflow
geometry. The 92551b30 checkpoint verifies the subsequent presenter/stream additions. Later
composition/TextEdit and context-flyout changes require their own recorded run.

## Implementation test map

### Versioned native TOM observations

The [first system MSFTEDIT run](https://github.com/wieslawsoltes/FerroUI/actions/runs/39600670835/job/119224339597)
at `eff6f53ade273677db8679d2c6089909fcf7a886` passed its Home/End probe:
60 initial commands and 60 repeated no-ops, on Windows 10.0.26100 with
`Msftedit.dll` 6.2.26100.33438. The exact fixture text, native story lengths,
normalized and directed endpoints, flags, signed results and HRESULTs are retained
in [native-msftedit-home-end.json](native-msftedit-home-end.json), together with
the probe's Git blob and CI provenance. This is an observation of that system
RichEdit version; it does not establish external WinUI engine equivalence.

The recorded multiline fixture `abc\rdefg\rhij` demonstrates a concrete distinction:
starting with anchor 1 and active end 6, collapsed Line Home reaches 0 and returns
-6, while extended Line Home retains anchor 1, reaches 4 and returns -2. Collapsed
Line End reaches 8 and returns 2. The native Story End extension can include the
synthetic final paragraph mark; FerroUI's normalized model still excludes that
mark under RT-I01/RT-I08. These facts drove the later implementation correction;
the successful probe itself does not change the Rust behavior at its checkpoint.

To retain a later completed observation, use the strict extractor with the exact
tested commit, probe blob and run/job URLs:

```sh
python3 scripts/porting/rich-text-tom-observations.py \
  --log /path/to/completed-windows-job.log \
  --commit <tested-full-commit-id> --probe-blob <tested-probe-git-blob-id> \
  --run-url <actions-run-url> --job-url <actions-job-url> \
  --output /path/to/native-observations.json
```

The extractor rejects unknown probe rows, incomplete command/repeat totals,
inconsistent engine identities, malformed fixture lengths and contradictory
directed-state fields. It preserves observations without comparing them to the
Rust implementation. The later vertical probe has its own execution record;
adding its source does not make it part of this first native run.

The [second record](native-msftedit-navigation.json) comes from
[Windows at 41ea4f1b](https://github.com/wieslawsoltes/FerroUI/actions/runs/39604251320/job/119236263239):
the same 60 Home/End commands/repeats plus 264 vertical commands, using MSFTEDIT
6.2.26100.33648. Its Home/End rows exactly match the earlier engine version.
Vertical collapse applies the full requested distance after choosing the
directional endpoint; a collapse alone reports one. For example, in
`abcdef\rxy\rabcdef\rpqrs`, selection 2→12 followed by collapsed Line Down1
reaches 19 and returns one, while Paragraph Down1 reaches 17. The sequential
long→short→long collapse sequence retains positions 5→8→14→8→5.

The [third record](native-msftedit-navigation-isolated.json) comes from
[Windows at 92140e7c](https://github.com/wieslawsoltes/FerroUI/actions/runs/39612123175/job/119263416996):
60 Home/End commands, 60 repeats and 282 vertical commands on MSFTEDIT
6.2.26100.33438. The original 264 vertical and all Home/End rows exactly match
the 6.2.26100.33648 record. Eighteen additional commands use fresh controls or
explicit sequences. They confirm extended-column clamping: caret 5, Line Down2
reaches 15 for collapse but 12 for extension; Up(-2) agrees. In the isolated
long/short/long sequence collapse visits 5→8→14→8→5, while extension visits
5→8→10→8→1 with anchor 5 retained. Fresh final-empty-line Up1 and Down(-1)
both reach 4; Up2 and Down(-2) reach 3. Thus the earlier negative-direction
difference was reused state, while extended-column behavior required a Rust
correction now defined in the source and awaiting its runtime gate. The two native probe tests passed; this does not turn the
separate failing CoreText compilation into a passing Windows job.

The [fourth record](native-msftedit-navigation-confirmed.json) is the successful
17a84805 Windows run above, using MSFTEDIT 6.2.26100.33648. All exact text
fixtures and all 60 Home/End + 60 repeat + 282 vertical rows equal the previous
6.2.26100.33438 record, including the 18 isolated observations.

The [fifth record](native-msftedit-navigation-horizontal.json) comes from the
successful d148193d Windows job above. It adds **220 fresh-control horizontal
commands**: 88 ASCII, 20 UTF-16/combining/ZWJ, 24 word-boundary, 40 mixed-bidi
default-window, 40 mixed-bidi RTL-reading-window and 8 empty-story cases. It
records native selection type with both directed states and the actual extended
window style. Slots 60/61/62 and Character/Word constants are checked against the
SDK projection. Its 60 Home/End, 60 repeats and 282 vertical observations all
match the preceding record.

Keyboard Character moves graphemes, including combining and joined-emoji
sequences, independently of scalar range Character. The recorded mixed-bidi
fixtures move in logical source order in both window styles. A Rust regression
now replays 216 horizontal rows; four synthetic terminal-EOP cases remain
explicitly outside the normalized model. These observations establish only the
recorded system MSFTEDIT behavior, not external WinUI engine conformance.

The later versioned records expand the probe to **300 horizontal**, **290
vertical** and **316 SetFlags/TypeText scenarios**, all executed on Windows. The
options matrix preserves requested flags, accepted post-SetFlags flags/type,
post-TypeText flags/type, HRESULTs and exact changed story text. It includes all
32 combinations of the five declared bits across collapsed, forward and reverse
selections, plus Unicode, paragraph and empty-typing fixtures. The Rust selection
implementation replays all 316 literal observations; synthetic terminal EOP and
a focused WinUI control remain separate contracts.

The latest `native-msftedit-links-gravity.json` additionally preserves 137 Link
and 1,577 Gravity commands. Link records distinguish null arguments/results,
counted UTF-16 text, original and mutated range/selection positions, hidden field
story text and repeated getter/setter calls. Gravity records capture exact
mode/flags/character and paragraph formats, duplicates, range setters, collapsed
and nonempty ranges, external edits, extreme positions and invalid native
integers. These are observations of system MSFTEDIT, not a claim that all source
APIs have matching Rust implementations.

The extractor decodes Rust Debug Unicode scalar escapes exactly, including ZWJ,
while preserving literal escaped backslashes. Counted BSTR text rejects malformed
scalar sequences. Every declared family and fixture must finish; an early
successful family cannot hide a later failing probe. All **23 Python tests** pass,
covering historical round trips, phase/length consistency, Link and Gravity
state transitions, malformed records and incomplete declared counts.

### Rust families

| Rust family | Concrete evidence and current limits |
|---|---|
| [Document tests](../../../src/FerroUI.Controls/documents/rich_text_document_tests.rs) | UTF-16 surrogate rejection, grapheme/word boundaries, nested semantic containers, snapshot formatting, paragraph splicing, batched revisions, reentrancy, pointer gravity, weak ownership, hyperlink cancellation/release semantics and embedded controls. Native synthetic element markers and every upstream element type are not represented by this suite. |
| [Rich view tests](../../../src/FerroUI.Controls/rich_text_block_tests.rs) | Measure/wrap, inherited formatting invalidation, selection geometry/direction, linked reflow and ownership, embedded controls, text-pointer insertion geometry, hyperlink host integration, and later real-ellipsis/finite-height/paragraph-indent/visible-range regressions. Check the validation record to determine which additions were executed in a particular run. |
| [Editor tests](../../../src/FerroUI.Controls/rich_edit_box_tests.rs) | Rich undo/redo, typing coalescing, read-only/max-length policy, failed transaction rollback, IME preview/cancel/commit and surrounding text, stale async clipboard guards, native transfer policies, bounded RTF fixtures, live ranges/search, mixed formatting, shared control geometry and caret blinking. These do not constitute execution of a real platform IME or unavailable native TOM engine. |
| [Live pointer batch tests](../../../src/FerroUI.Controls/documents/rich_text_pointer_tests.rs) | Immediate ordered rebasing, pointers created midway through a batch, nested rollback, reentrant notification order, formatting-only changes and weak lifetime. |
| [TOM unit tests](../../../src/FerroUI.Controls/rich_text/rich_edit_text_units_tests.rs) | Native values for eight units, scalar Character, actual shaped Cluster, Word/Sentence distinctions, soft/hard paragraphs, CRLF, native collapse counts, one-based/end-relative indices, saturation and nonmutation, weak measured Line association and overflow/trimmed coverage. Cluster requires complete untrimmed layout and tests ligatures, scripts, bidi, wrapped origins, omitted glyphs and reentrancy. |
| [TOM Delete/Character tests](../../../src/FerroUI.Controls/rich_text/rich_edit_text_range_operations_tests.rs) | Signed deletion, selected-span counts, paragraph joins, shaped reflow after first deletion, inherited formatting, scalar-safe Character replacement, independent directed selection and live range rebasing. Callback tests distinguish ordinary rollback from proven external model writes during layout, document publication and editor publication, including nested groups and held outer document batches. |
| [Selection Home/End tests](../../../src/FerroUI.Controls/rich_text/rich_edit_text_selection_navigation_tests.rs) | Twenty-two cases cover directed UTF-16 movement/extension, shaped and headless lines, CRLF/soft breaks, wrapping/RTL/overflow, layout rejection, read-only/composition/history retention, cancellation and selection/model reentry. Six native-derived cases add normalized collapse edges, real paragraph marks, repeated End affinity, empty paragraphs, cancellation and nested/view ownership. Synthetic terminal EOP and further engine affinity remain separate in RT-I21. |
| [Selection vertical and accepted-movement tests](../../../src/FerroUI.Controls/rich_text/rich_edit_text_selection_movement_tests.rs) and [geometry ownership tests](../../../src/FerroUI.Controls/rich_text/rich_edit_text_selection_geometry_tests.rs) | Shaped Line and headless Paragraph fixtures cover directed collapse/extension, accepted counts, signed/extreme/zero requests, mixed heights, bidi/UTF-16, wrapping, empty lines, overflow and unavailable layout. Five new extension fixtures distinguish retained collapsed columns from stepwise extension clamping, including upward, repeated-facade, empty-line and redirected-edge cases. Two formatter-instrumented definitions compare geometry work for 8/1,024-line stories and protect callback-owned state during redirected-edge queries. Callback cancellation/redirection, nested navigation, view replacement and external publication preserve accepted state. New tests require their own runtime gate; the instrumentation is not an executed performance result. |
| [Selection horizontal tests](../../../src/FerroUI.Controls/rich_text/rich_edit_text_selection_horizontal_tests.rs) | Nine source definitions replay 216 recorded native Character/Word commands and separately cover four unsupported terminal-EOP outcomes, scalar range independence, unsupported/zero units, extreme counts, cancellation/redirection, model mutation and nested navigation affinity. Mixed-bidi offsets follow the observed logical policy. Native-aligned word and grapheme-interior probe additions and Rust runtime execution remain separate gates. |
| [InputScope object and value tests](../../../src/FerroUI.Base/input/input_scope_tests.rs) | Nine object/value tests passed in the filtered Base input_scope gate. They cover every exact native enum value and gaps, object/default/checked-string construction, ordered shared Names and immutable snapshots, duplicate item subscriptions, callback list mutation, disposal/weak lifetime, legacy hint precedence, inherited transport and metadata. Exact constant mappings are complete; full object/native platform equivalence remains partial. |
| [InputScope runtime XAML](../../../tests/FerroUI.Markup.Xaml.UnitTests/xaml/rich_text_tests.rs) and [compiled corpus](../../../tests/FerroUI.Markup.Xaml.UnitTests/emitter/corpus.rs) | The rich_input_scope.xaml fixture and three runtime definitions cover named shortcuts, ordered object content, live NameValue changes, null values, all 43 names, undefined input rejection and typed constructor arguments. Compiled/runtime tree comparisons require their own generated source and execution gate. |
| [CoreText policies](../../../src/Windows/FerroUI.WinUI/core_text_input_options.rs), [Android policies](../../../src/Android/FerroUI.Android/platform/input/android_input_method.rs), [iOS policies](../../../src/iOS/FerroUI.iOS/text_input_responder.rs), [Wayland policies](../../../src/FerroUI.Wayland/text_input_options_converter.rs) and [Fcitx policies](../../../src/FerroUI.FreeDesktop/dbus_ime/fcitx/fcitx_x11_text_input_method.rs) | Thirteen new source-defined cases cover exact 43-value CoreText mapping, first-name/default/empty precedence, same-content-type name replacement, numeric versus telephone/alphanumeric PIN hints, privacy flags, Android editor callback refresh, iOS unchanged secure-entry policy and semantic autofill distinctions. Existing CoreText lazy/nested/retry fixtures remain. Real mobile keyboards, candidate windows, native session ordering and device execution remain open. |
| [Browser policy tests](../../../src/Browser/FerroUI.Browser/browser_text_input_options.rs), [method tests](../../../src/Browser/FerroUI.Browser/browser_text_input_method.rs) and [TypeScript bridge tests](../../../src/Browser/FerroUI.Browser/webapp/tests/input-options.test.mjs) | Five Rust policy definitions cover HTML input modes, first-name/legacy fallback, sensitivity, capitalization/suggestions, return-key and locale hints; three method definitions cover active composition, duplicate hints and callback reentry. Those Rust tests await runtime. Four completed Node tests execute the actual TypeScript bridge with guarded input doubles, preserving value/type/focus/UTF-16 selection, clearing stale attributes, suppressing sensitive suggestions and avoiding repeated writes. Typecheck, lint and production build passed. Real browser keyboards/OS composition remain separate gates. |
| [Input policy manager tests](../../../src/FerroUI.Base/input/text_input/text_input_method_options_tests.rs) and [rich owner tests](../../../src/FerroUI.Controls/rich_edit_input_options_tests.rs) | Existing lifecycle/inheritance/disposal/reentrancy and focused/flyout-owner cases now include immutable scope snapshot reentry and inherited clearing, live ordered names, item mutation, replacement/removal, shared scope lifetime, preserved legacy hints and owned-flyout delivery. Current source preserves connection/composition while updating the semantic owner. The two new Base manager snapshot cases passed in the 11-case input_scope gate; current rich owner additions passed in the 414-case focused Controls gate. |
| [TOM clipboard tests](../../../src/FerroUI.Controls/rich_text/rich_edit_text_clipboard_tests.rs) | Twenty-eight public cases cover copy permission, exact representation policies, metadata-only CanPaste, rich/plain formatting, one-group undo, UTF-16 limits, preview ordering, delayed target ownership, held batches, independent clones, read-only completion, retrieved-transfer disposal and callback publication. Real pending futures exercise cancellation rather than immediately ready test doubles. Defined source and completed execution are tracked separately; native protection, formats and ABI remain partial under RT-I24. |
| [Focus handoff tests](../../../src/FerroUI.Controls/rich_edit_input_focus_tests.rs) | Four public cases cover actual template replacement during native/overlay popup focus return, nested Lost/Got redirection and same-owner return, canceled focus and ordinary post-event client installation. Two remount regressions failed against pre-fix libraries while both cancellation cases passed. All four pass in the 235-case source-identical gate; arbitrary asynchronous platform focus order remains open. |
| [Context-menu tests](../../../src/FerroUI.Controls/rich_edit_context_menu_tests.rs) | Eighteen cases drive real pointer, Apps/Shift+F10 and holding requests, shared arguments, root DIPs at 2x scale, normalized/RTL/collapsed/wrapped selection anchors and actual popup positioner calls. Cancellation/precedence, menu installation/replacement/removal, nested requests, detach/reattach, weak anchor ownership, repeated positioning, reflection-correct point transforms and live IME/focus before/after native/overlay popup dismissal have explicit coverage. A separate focus-redirection fixture verifies retirement of the original composition. Native automatic selection floaties and legacy asynchronous PopupMenu are separate under RT-I22. |
| [Cursor publication tests](../../../src/FerroUI.Controls/rich_edit_input_geometry_tests.rs) | Three public input-manager cases deliberately return an old rectangle after getter callbacks change owner/client, reset, leave/return, or publish newer geometry for the same client. The accepted backend rectangle, connection and history are asserted without exposing private manager APIs. |
| [CoreText geometry tests](../../../src/Windows/FerroUI.WinUI/core_text_layout.rs) and [native adapter tests](../../../src/Windows/FerroUI.WinUI/win_ui_text_input_method_reentrancy_tests.rs) | Seven portable geometry cases cover complete rectangle transforms, reflection/rotation, scaling/origin, caret-only minimum, unavailable geometry, cancellation/reentry and native setter failure. Six Windows-only cases cover semantic ownership, accessor/revision replacement, deferred mutation, nested/focus/reset invalidation and a real attached owner resized by the final revision callback. Arbitrary requested-range and candidate-window geometry remain pending; native execution is recorded separately. |
| [Shared formatter marker regression](../../../src/FerroUI.Base/media/text_formatting/text_line_tests.rs) | The direct structural-terminator case covers LTR/RTL caret-distance/hit inverses, selected fragment bounds, nonzero source starts, one/two-unit line/paragraph markers and trailing marker positions. It failed against the preceding Base library and passed after the shared correction. |
| [System MSFTEDIT probe](../../../src/Windows/FerroUI.Win32/rich_text_tom_oracle_tests.rs) | The latest measured record contains 60 Home/End commands, 60 repeats, 290 vertical, 300 horizontal, 316 options/typing, 137 Link commands across 59 fixtures, and 1,577 Gravity commands across 199 fixtures. Captures counted UTF-16, OS/DLL versions, range/selection states, formats, HRESULTs, signed deltas and complete fixture counts. Strict extraction and 23 Python regressions pass. System MSFTEDIT characterization is distinct from the external WinUI engine and from Rust API implementation. |
| [Native observation extractor tests](../../../scripts/porting/test_rich_text_tom_observations.py) | Seven Python tests passed at 4c0bae3. Exact historical replay, Unicode escaping and strict family/fixture/phase/type/flag/count validation protect immutable native evidence; this pass does not execute the new native commands. |
| [Text stream tests](../../../src/FerroUI.Controls/rich_text/rich_edit_text_stream_tests.rs) | UTF-8/UTF-16 and BOM handling, raw RTF, validation before I/O/mutation, exact byte bounds, interrupted and partial I/O, prefix failure, caller position/ownership, transactional range replacement and reentrant output snapshots. |
| [Language tests](../../../src/FerroUI.Controls/documents/rich_language_tests.rs) | Source validation grammar, inheritance into shaping, local overrides, snapshots/history, RTF default/scope/plain reset, representability errors and owner/presenter identity. |
| [Inherited presentation tests](../../../src/FerroUI.Controls/documents/rich_presentation_invalidation_tests.rs) | Theme/owner presentation preserves semantic revision, mutation identity, undo and active composition; font/language/flow and inherited mutable collections still invalidate layout. Explicit local formatting and clearing to inheritance remain semantic changes. The Base inheritance tests distinguish actual notification origin from the resulting inherited priority and preserve it in owned event arguments. |
| [Composition cause tests](../../../src/FerroUI.Controls/rich_text/rich_text_composition_tests.rs) | Accepted Unicode deltas, cursor filtering/normalization, one finalization/undo unit, explicit cancellation, read-only/reentrant rejection, group event ordering/rollback, stale preview invalidation inside held model batches and weak disposable observers. Routed argument ranges have the separate tests below; actual backend event order remains a platform gate. |
| [Composition event tests](../../../src/FerroUI.Controls/rich_edit_composition_event_tests.rs) and [begin lifecycle tests](../../../src/FerroUI.Controls/rich_edit_composition_start_tests.rs) | Native unavailable defaults and whole UTF-16 ranges; accepted filtered/limited completion payloads; no cursor-only changed events; exact disjoint/empty replacements; explicit begin before text without selected-text deletion/history; empty completion/cancel; legacy implicit start; reentrant caret/model/start/end callbacks and stale acknowledgement rejection. |
| [RTF style tests](../../../src/FerroUI.Controls/rich_text/rtf_styles_tests.rs) and [field tests](../../../src/FerroUI.Controls/rich_text/rtf_field_tests.rs) | Literal grammar fixtures for forward/cyclic inheritance, additive/scoped character formatting, font charset selection, direct overrides, names/metadata isolation, metrics/tabs/empty paragraphs and expansion limits. Multi-paragraph fields preserve targets, surrounding content and formatting, exclude invisible empty links, count separators, bound cumulative metadata expansion and reject before mutation during streamed import. Canonical export retains resolved appearance rather than style/field identity. |
| [Highlighter tests](../../../src/FerroUI.Controls/documents/text_highlighter_tests.rs) | All ten literal upstream overlap fixtures plus an independent per-character layering oracle, signed range value copies, markup boxing, empty/invalid clipping, brush defaults, mutable brush invalidation and weak/reentrant subscriptions. Native raster/high-contrast comparisons remain separate. |
| [Portable text automation tests](../../../src/FerroUI.Controls/automation/provider/rich_text_automation_tests.rs) | Six Text/eighteen TextRange operations, Text2 caret and TextChild contracts over the live model, scalar-safe text, range identity/rebasing/lifetime, selection policies, format and text search, unit normalization, 25 typed attributes/sentinels, stable hyperlink peers, children, events and shaped/clipped overflow geometry. The 92551b30 checkpoint covers scroll alignment and presenter ownership. TextEdit composition, payload and subscription additions require the later corresponding record. |
| [Native text pattern tests](../../../src/Windows/FerroUI.Win32/automation/text_pattern_tests.rs) and [range identity lifetime test](../../../src/Windows/FerroUI.Win32/automation/text_range_node.rs) | Native provider slots, worker-to-UI dispatch, BSTR/SAFEARRAY/VARIANT/HRESULT values, correct reserved sentinels/COLORREFs, geometry/children, document replacement, foreign ranges, text/selection events and delayed COM release. Test definitions and a cross-compile are not evidence of an actual Windows UI Automation client or screen-reader run. |

The source overlap fixtures preserved by
`text_highlight_merge_matches_all_source_overlap_cases` are **NonOverlapped,
OverlapNextPartial, OverlapPrevPartial, OverlapNextFull, OverlapEqualFull,
OverlapEqualPartial, OverlapEqualMultiple, OverlapInner, Overlap1 and
OverlapMultiple**. Their source test files remain `reference` entries with
`adapted_test_cases`; they are not counted as completed runtime source ports.

The `rich_` filter includes document, rich view/editor and portable automation
modules, but does not select the `text_highlighter_tests` module. Use a
`text_highlight` filter or the full controls suite for those tests. Native text
tests live in the Win32 crate and require their own command/result record.

Runtime/compiled XAML, catalog interactions, native Windows/macOS/Linux/browser
input and accessibility, exact font/DPI rendering comparisons, large-document
performance and externally implemented service conformance are additional gates
in [PLAN.md](PLAN.md). They remain unverified until recorded with their actual
commands or platform procedure and result.

## Continuation: shaped units, paragraph tabs, public peer events and casing

The published branch already includes the following focused fixtures. Their
source/member mappings are now reconciled with those actual implementations;
a historical gate is not evidence that every later checkpoint has run them.

- `rich_edit_text_units_tests.rs`: actual ligature/script Cluster boundaries,
  bidi logical order, wrapped buffer origins, embedded objects/soft breaks,
  omitted glyphs, linked pages, reflow and reentrant/malformed layout rejection.
- `rich_paragraph_tabs_tests.rs` and Base `text_tab_layout_tests.rs`: native enum
  values, sorted/replaced stops, points conversion, 63-stop validation, immutable
  cloning, rich snapshots/undo, default interval ownership, RTF/clipboard context,
  shaped fields across runs and objects, RTL decimal alignment, wrapping,
  justification, leaders, bars and cached shaping reuse.
- `automation_peer_text_edit_tests.rs` and Win32 `text_pattern_tests.rs`: the
  public event path without TextEdit patterns, runtime metadata, one-hop weak
  EventsSource, cycles/lifetime, UTF-16/NUL payloads, lazy provider identity and
  reentrant source changes.
- New `rich_edit_text_case_tests.rs`: native LetterCase values, Unicode
  expansions/directed selection, contextual sigma across formatted runs,
  paragraph/link/control preservation, astral/interior-caret positions, no-op
  revision/redo, read-only/limit rejection, nonexpanding over-limit documents,
  committed-content recapture after preedit cancellation and grouped rollback.

The focused case-conversion filter is `rich_text_case_`. Execution results for
this continuation are recorded only after a completed gate in the journal and
`mapping.json`; the fixture list alone does not assert a passed run.

### Exact input and CoreText state fixtures

`rich_edit_exact_input_tests.rs` has targeted `rich_exact_` regressions for
replacement ranges independent of selection, empty deletion, invalid UTF-16 and
policy rejection, active composition span, original-selection cancellation,
selection-only preview updates, intercepted completion, selection cancellation,
reentrant application edits and identical-text redo/format preservation.
`text_input_method_edit.rs` exercises the portable request's scalar boundaries.
`core_text_synchronization.rs` covers failed text/selection notification retry,
last-acknowledged length, reentrant state and replacement-client fences. The
CoreText state harness is compiled on the host; it is not an actual native
CoreText/IME execution or clause-format rendering test.

### Actual theme, command, markup and catalog fixtures

The theme tests use each production theme's compiled resources. Their common
six scenarios verify live template parts and one presenter, complete-story
scrolling and end-caret reveal, theme changes during history/composition,
retemplating, an opened owner command flyout with live enablement, and local
placeholder-brush precedence across pointer/focus/disabled states.

| Fixture | Evidence defined |
|---|---|
| [WinUI theme](../../../src/FerroUI.Themes.WinUI/tests/rich_edit_box_tests.rs) | Six actual-theme tests, including light/dark and high-contrast variants. |
| [Fluent theme](../../../src/FerroUI.Themes.Fluent/tests/rich_edit_box_tests.rs) | The same six scenarios against the Fluent dictionary. |
| [Simple theme](../../../src/FerroUI.Themes.Simple/tests/rich_edit_box_tests.rs) | The same six scenarios against the Simple dictionary. |
| [Formatting commands](../../../src/FerroUI.Controls/rich_edit_format_command_tests.rs) | Seven regressions for independent mixed state, uniform toggle/directed selection/undo, unrelated decorations, collapsed typing format, read-only/accelerator policy, reentrancy and preedit cancellation. |
| [Runtime XAML](../../../tests/FerroUI.Markup.Xaml.UnitTests/xaml/rich_text_tests.rs) | Four tests for nested document/highlighter values, actual-owner command bindings, two-way text binding and linked overflow. |
| [Compiled-emitter fixture](../../../tests/FerroUI.Markup.Xaml.UnitTests/emitter/fixtures/rich_text_document.xaml) | Shared document fixture registered in the existing runtime-versus-compiled differential corpus. |
| [Catalog page](../../../samples/ControlCatalog/Pages/RichTextPage.xaml) | Registered Text navigation page plus the sample's generated document tests and shown-page corpus. The page demonstrates editing, shared content and linked columns. |

These are authored FerroUI regressions and examples tied to the mapped native
contracts. They are not native source test cases copied verbatim. Runtime or
compiled-source fixture registration does not assert a passed run; completed
gates are recorded separately in the journal and validation records.

### RTF encoding fixtures

[rtf_encoding_tests.rs](../../../src/FerroUI.Controls/rich_text/rtf_encoding_tests.rs)
defines fifteen focused regressions. Literal byte fixtures cover all seventeen
supported code pages and canonical-writer text round trips. Other cases cover
multibyte characters split across raw/escaped tokens, escaped delimiter bytes,
font/page group state and `plain`, `cpg` precedence, DEFAULT versus ANSI charset,
font names/field instructions, Unicode fallback, incomplete sequences at semantic
boundaries, invalid UTF-8 without replacement, UTF-16 limits, repeated output
buffers and ignored field destinations. Native code-page/vendor behavior and
unsupported RTF content require separate conformance tests. A source fixture is
not a recorded runtime pass.

### Native counted-string and callback fixtures

Two new `exact_hstring_` tests in
[win_rt_tests.rs](../../../src/Windows/FerroUI.Win32/win_rt/win_rt_tests.rs)
cover astral/NUL preservation, borrowed ownership and strict rejection of isolated
surrogates. The WinUI host fixture
`native_text_request_preserves_exact_code_units_and_snapshot_lifetime` checks
that native queries retain the requested raw UTF-16 slice in an owned snapshot.
Four [native adapter regressions](../../../src/Windows/FerroUI.WinUI/win_ui_text_input_method_reentrancy_tests.rs)
cover client replacement from disposal, connection access, subscription
registration and state queries. Those Windows-only tests exercise the actual
adapter without a COM context or desktop window; their definitions are not
proof of execution or a real platform IME integration pass.
