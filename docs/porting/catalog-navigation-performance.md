# Control catalog page navigation latency

Investigation started on 2026-11-07. Machine: Apple M4 (10 cores), macOS,
Rust 1.99.1.

## State of main (2026-11-10)

The change applies to main `53a76bb21f104a9f051afda0da093d7e2846bbe9`. Before
it, main eagerly builds the alternate Calendar cells and uses the
nonvirtualized Calendar examples panel.

Main now also has constant-time removal from the Loaded queue, linear bulk
child removal, and further property, styling and compiled-template optimizations.
These reduce related costs while the two Calendar changes still avoid realizing
unused cells and offscreen example templates. The November 9 measurements below
compare against `91704388e`, **not this newer main**; their percentages have not
been remeasured for the November 10 tree.

The open PRs were checked by changed-file inventory. The CalendarItem
patch in [#496](https://github.com/wieslawsoltes/FerroUI/pull/496) wraps
existing overrides; the general construction work in
[#690](https://github.com/wieslawsoltes/FerroUI/pull/690) does not change
Calendar realization. None changes the catalog Calendar page.

Validation on that base: **756 targeted tests passed** (112 Calendar/
CalendarDatePicker, 10 catalog Calendar integration/document/interaction, and
634 compositor/transport tests). Existing compiler warnings remain. The
default desktop build passed a native Metal smoke run with
Home → Calendar → Home reaching Loaded and a normal timed shutdown. The
first smoke attempt advanced before initial Home reached Loaded, so the
successful run used a longer dwell.

## Measurement against main (2026-11-09)

The Calendar changes show a benefit against
main `91704388e1e9ab9b721414992e3a795817fe7c87`, with substantial timing
variation under the background load of the machine. The pooled median
improvement is **8.9% opening Calendar and 9.7% returning to Home**. These
are observed values for this run, not stable estimates of the percentage gain.

This compares main plus the five timing-hook files against main plus the
whole change. The production
differences are the platform Calendar alternate-cell deferral and the catalog
Calendar page virtualization, measured together. Main's indexed disposal and
other merged layout improvements are present in both executables.

Median navigation request → Loaded, milliseconds; **30 visits per transition
per executable**:

| Transition | Latest main | Optimized | Change |
|---|---:|---:|---:|
| Home → Buttons | 42.66 | 47.98 | +12.5% |
| Home → Calendar | 89.38 | 81.45 | -8.9% |
| Buttons → Home | 39.62 | 50.23 | +26.8% |
| Calendar → Home | 70.67 | 63.79 | -9.7% |

Calendar improved in four of six process pairs on both entry and return.
Pair changes ranged from -46.4% to +16.9% on entry and -44.5% to +27.8% on
return. The unchanged Buttons control also varied widely, with pooled medians
12.5% and 26.8% slower in the optimized executable. That makes the exact
Calendar percentage uncertain under this workload.

As a descriptive check against common timing drift, subtract each process's
Buttons transition median from its corresponding Calendar transition median,
then compare those gaps within each pair. Calendar's excess latency fell in
**all six pairs**, by a median **16.77 ms on entry** and **13.30 ms on return**.
This supports a remaining Calendar benefit; subtracting the control assumes
an additive common timing effect and does not establish a fixed saving.

| Pair | Calendar entry change | Calendar return change | Entry gap change (ms) | Return gap change (ms) |
|---|---:|---:|---:|---:|
| 1 | -39.5% | -33.9% | -38.53 | -13.76 |
| 2 | -18.0% | -21.6% | -16.36 | -12.84 |
| 3 | +14.5% | +0.7% | -10.01 | -23.61 |
| 4 | +16.9% | +27.8% | -15.98 | -11.02 |
| 5 | -16.0% | -26.4% | -17.17 | -21.86 |
| 6 | -46.4% | -44.5% | -26.45 | -8.86 |

First visits and repeat visits within each measured process, also request →
Loaded medians (first: six observations; repeat: 24 per transition/build):

| Calendar transition | Visit group | Latest main (ms) | Optimized (ms) |
|---|---|---:|---:|
| Home → Calendar | First | 111.93 | 93.65 |
| Home → Calendar | Repeat | 82.24 | 72.13 |
| Calendar → Home | First | 76.57 | 67.10 |
| Calendar → Home | Repeat | 64.53 | 60.91 |

For Calendar entry, the construction / host-return / Loaded medians were
**0.28 / 12.06 / 89.38 ms** on main and **0.32 / 16.13 / 81.45 ms** optimized.
For Calendar → Home, host-return fell from **32.49 to 26.46 ms**. These phases
are cumulative; each column is aggregated separately. Loaded is the UI event
milestone, not GPU presentation or time on glass.

Both builds use current main's `dist` profile: fat LTO, one code-generation
unit, `panic=abort`, with symbols retained via `-C strip=none`. Rust 1.99.1,
Apple M4 (10 cores, 24 GiB), macOS 27.2, AC power. The dependency lockfiles and
the five instrumentation files match. The benchmark fixes WinUI Light and
native Metal. It uses one excluded warmup launch per build, then six
alternating process pairs (main first in odd pairs), five visits to each
page per process, returning to Home after each target, at 800 ms intervals.
These are process-local first/repeat visits with warm OS caches, not cold
startup measurements. The 30 observations come from six processes per build;
they are not 30 independent process samples.

Other development/simulator activity was left running. No
benchmark-owned compiler, test runner or profiler ran during collection.
Recorded one-minute load averages ranged from **2.86 to 4.58**. All **240
measured visits** reached Loaded, and all 12 measured processes exited
normally with exactly the expected navigation sequence. The accepted run
needed no retries. An earlier aborted comparison was excluded in full after
an extra Calendar navigation following an app-reopen event; its logs remain
separate. The runner can now retry a bounded number of entire invalid pairs,
keeping every rejected attempt and excluding both processes in that pair.

Build both source variants with the same command (provide a native Ninja
executable through `SKIA_NINJA_COMMAND` if needed):

```sh
cargo rustc --locked --profile dist -p control-catalog-desktop \
  --bin control-catalog-desktop -- -C strip=none

python3 scripts/measure-catalog-navigation.py \
  --before target/calendar-latest-main/main-timed \
  --after target/calendar-latest-main/calendar-optimized \
  --output target/calendar-latest-main/results \
  --pages Buttons,Calendar --rounds 6 --visits 5 --interval-ms 800 \
  --warmups 1 --max-pair-attempts 3
```

[Committed measurement evidence](measurements/catalog-calendar-2026-11-09.json)
contains source revisions, instrumentation file inventory and patch hash,
binary SHA-256 hashes, build settings, all per-pair medians, first/repeat
summaries, control-gap calculations, process load records, and the observation
file hash. Raw logs, every observation, the baseline instrumentation patch,
and preserved executables are under ignored `target/calendar-latest-main/`.

The measurement validation also checked the executable and instrumentation
hashes, all phase ordering and sample counts, and the runner's rejection,
whole-pair recovery and retry-cap behavior using synthetic process output.

## What main already had (2026-11-09)

Compared with main `91704388e1e9ab9b721414992e3a795817fe7c87`.
The November 7 timings below are historical measurements against an older
base. The comparison above measures the change against that main.

- **Disposal indexing is on main.** Main contains
  [584ca03b9](https://github.com/wieslawsoltes/FerroUI/commit/584ca03b9),
  which replaces the linear disposal scan with direct ID indexing, the
  first candidate of the profile below. This change adds the request
  coalescing, out-of-band disposal, and large-batch/ID-reuse regression
  tests, which exercise that implementation. The scaling benchmark of the
  investigation is historical evidence and is not part of the test module.
- **Calendar cell deferral is additional work.** Main populated
  the alternate cells eagerly; the deferral comes with mode-switch tests.
- **Calendar catalog virtualization is additional work.** Main
  used the wrapping panel for all nine examples. The virtualized
  presentation comes with a state/layout/lifetime regression test.
- Main also brings inherited-value lookup caching, text-width/layout holds,
  unchanged-drawing suppression and measure holds (including merged
  [#651](https://github.com/wieslawsoltes/FerroUI/pull/651),
  [#653](https://github.com/wieslawsoltes/FerroUI/pull/653) and
  [#654](https://github.com/wieslawsoltes/FerroUI/pull/654)). These address
  other costs identified in the initial profile; they do not replace the
  two Calendar changes. The fresh comparison above uses a new matching
  main baseline.

At audit time, all seven open PRs were checked by changed-file inventory, with
patch inspection for direct overlaps:

| Open PR | Relationship to this work |
|---|---|
| [#496](https://github.com/wieslawsoltes/FerroUI/pull/496) | Touches CalendarItem, but the patch only wraps existing overrides in `ferro_overrides!`; it does not defer cells. Its property/theme changes are complementary. |
| [#652](https://github.com/wieslawsoltes/FerroUI/pull/652) | Avoids re-recording drawings on position-only changes; complementary render optimization. |
| [#642](https://github.com/wieslawsoltes/FerroUI/pull/642) | Adds screenshot/window-position diagnostics in the desktop entry point and renderer work; does not replace page-sequence/timing diagnostics or Calendar changes. |
| [#564](https://github.com/wieslawsoltes/FerroUI/pull/564) | ProGPU backend; no direct overlap. |
| [#473](https://github.com/wieslawsoltes/FerroUI/pull/473) | Linux desktop composition; no direct overlap. |
| [#453](https://github.com/wieslawsoltes/FerroUI/pull/453) | QML stack; no direct overlap. |
| [#295](https://github.com/wieslawsoltes/FerroUI/pull/295) | Editor/TextMate port; no direct overlap. |

The diagnostics follow the `ferro_thread_local!` convention.

Validation on that base: **711 targeted tests passed** (589 compositor/transport,
112 Calendar/CalendarDatePicker, 10 catalog Calendar integration/document/
interaction tests). Existing compiler warnings remain. The latency
measurements on that base are above.

## Historical before/after comparison (2026-11-07)

**Opening Calendar is 28% faster; returning from Calendar to Home is 38%
faster** in the controlled comparison. Calendar improved in all three
rounds: about 19–32% on entry and 27–38% on return to Home.

Both executables use the same `dist` optimizations and retained symbols.
The baseline was preserved before the framework/page changes. The runner
fixes WinUI, light variant and native Metal rendering. It alternates process
order over three rounds; each process visits the seven pages three times,
returning to Home between targets. There are **nine observations per
transition per executable**, 252 measured visits in total. No profiler or
compiler ran during this final comparison. All expected visits reached
Loaded, and all six processes exited successfully.

Median navigation request → Loaded, milliseconds:

| Transition | Before | After | Change |
|---|---:|---:|---:|
| Home → Buttons | 54.43 | 55.71 | +2.4% |
| Home → TextBox | 46.69 | 42.94 | -8.0% |
| Home → ComboBox | 52.15 | 28.37 | -45.6% |
| Home → TextBlock | 45.72 | 40.23 | -12.0% |
| Home → ColorPicker | 51.75 | 48.01 | -7.2% |
| Home → Calendar | 111.34 | 80.15 | -28.0% |
| Home → TreeDataGrid | 64.10 | 64.04 | -0.1% |
| Buttons → Home | 58.34 | 44.44 | -23.8% |
| TextBox → Home | 54.54 | 51.23 | -6.1% |
| ComboBox → Home | 57.41 | 51.91 | -9.6% |
| TextBlock → Home | 49.80 | 49.53 | -0.5% |
| ColorPicker → Home | 57.39 | 50.03 | -12.8% |
| Calendar → Home | 108.62 | 67.53 | -37.8% |
| TreeDataGrid → Home | 62.76 | 57.07 | -9.1% |

Calendar's median construction / host-return / Loaded times changed from
**0.61 / 27.38 / 111.34 ms** to **0.52 / 19.18 / 80.15 ms**. For the return
to Home, host-return time fell from **49.11 ms to 33.33 ms**, and Loaded
from **108.62 ms to 67.53 ms**.

Absolute timings varied appreciably across processes, so the strongest
result is the consistent Calendar improvement in every round. Buttons and
TreeDataGrid are effectively flat overall; smaller changes on other pages
should not be treated as established gains without more runs. The full
table reports every tested transition rather than selecting only gains.
The earlier single-process observations below are investigation evidence,
not the baseline for these final percentages.

`Loaded` measures UI readiness, not time on glass or GPU presentation. An
earlier comparison was rejected because timer drift truncated the final
return to Home. The final runner allows an additional ten seconds for
shutdown; it rejects missing, unexpected or incomplete navigation. No
measurements from that rejected comparison are pooled into this result.

Reproduce with the preserved binaries (or separately built matching
baseline/candidate executables):

```sh
python3 scripts/measure-catalog-navigation.py \
  --before target/navigation-diagnostics/catalog-before \
  --after target/navigation-diagnostics/catalog-after-virtualized \
  --output target/navigation-comparison-final \
  --rounds 3 --visits 3 --interval-ms 800
```

Raw logs, all observations, executable SHA-256 hashes, and the median
summary are in `target/navigation-comparison-final/`. These are ignored
local measurement artifacts; this table preserves the results in source.

Final validation:

- 585 compositor/transport tests passed, including disposal lifecycle and
  the new queue regressions.
- 112 Calendar/CalendarDatePicker tests passed, including deferred alternate
  views, repeated mode switching, and initial Year/Decade mode.
- The catalog virtualization test passed: wide/narrow wrapping, scrolling,
  date selection retained on the same control, date bounds, blackouts, week
  numbers, and cleanup.
- Three existing Calendar document/class/compiled-markup parity tests passed.
- Both the framework and compositor 1100×800 native Calendar first-viewport
  screenshots are byte-identical before and after (framework PNG SHA-256
  `edca9c4673a52931e366f0ea2755cab59b242fd8f8e77a671cda9fe65dd3c741`).
- A scaling benchmark of the disposal queue (of the investigation, not in
  the test module): in the debug test build, increasing requests from 8,000
  to 32,000 objects changed the linear scan from 183 ms to 3,023 ms, and the
  index from 0.28 ms to 1.00 ms. These demonstrate scaling, not production
  application timings.

## Initial measurements (before optimization)

The reproducible delay is predominantly UI tree realization and layout, plus
teardown of the outgoing page. It is not XML parsing in the default catalog.

The default WinUI catalog, native macOS/Skia rendering, `dist` optimizations
(fat LTO, one code-generation unit), with symbols retained for sampling.
The timing run below had **no sampling profiler attached**. Seven pages were
each opened three times, 800 ms per page, returning to Home between visits
to keep the outgoing page consistent. These are process-local first visits
with warm OS/file caches, not cold process startup measurements. Times are
in milliseconds, cumulative from the navigation request.

| Page | First construction | First host return | First Loaded | Revisit Loaded range (2 visits) |
|---|---:|---:|---:|---:|
| Buttons | 0.60 | 9.17 | 37.28 | 29.93–31.80 |
| TextBox | 0.08 | 9.84 | 23.26 | 17.68–18.49 |
| ComboBox | 0.44 | 9.07 | 22.80 | 23.39–25.29 |
| TextBlock | 0.28 | 7.95 | 23.45 | 17.90–18.45 |
| ColorPicker | 0.28 | 8.28 | 22.55 | 21.99–24.25 |
| Calendar | 0.18 | 8.20 | 86.07 | 85.65–90.29 |
| TreeDataGrid | 1.46 | 12.70 | 48.12 | 40.96–41.79 |

For Calendar, about 78 ms of the first 86 ms occurs **after** the host call
returns; its markup population itself takes 0.15 ms. Buttons' markup
population takes 0.55 ms. Shared resource/font warmup does not explain the
persistent delay: Calendar still takes 86–90 ms on revisits.

Calendar is a concrete example of the amplification: its page contains nine
Calendar controls. Each `CalendarItem::on_apply_template` calls `populate_grids`,
which creates 42 day buttons and 12 year-view buttons, plus headings and
other controls. That is 486 calendar cell buttons across the page before
counting the visual trees of their templates, and all nine calendars
participate in the nonvirtualized page layout.

The outgoing page matters too. Home reaches Loaded in about 29–38 ms after
most of the measured pages, but **79–86 ms after Calendar**. Its host call
returns after 26–31 ms in that case, versus about 7–12 ms for the other
pages. Opening a light page can therefore still feel slow after leaving a
large one.

## CPU profile

A separate 25-second, 1 ms sampling run of the same optimized executable
covered the mixed page tour. Sampling increased the absolute wall times;
those times are excluded from the table above. On the main thread, 18,656
of 19,848 leaf samples were waiting in `mach_msg2_trap`. Of the remaining
1,192 samples, 651 had a layout-pass frame in their stack (about 55%).
Selected inclusive stack counts follow; **they overlap** and must not be
added together:

| Work appearing in the stack | Main-thread samples |
|---|---:|
| Layout pass | 651 |
| Measurement | 526 |
| Applying templates | 292 |
| Navigation body | 283 |
| Inherited-property propagation/lookups | 220 |
| Style application | 131 |
| Resource lookup/build | 116 |
| Text formatting/layout/shaping | 58 |

`Compositor::queue_disposal` alone had 84 self samples (about 7% of the
non-waiting main-thread samples), the largest named framework leaf. It
linearly scans the pending-disposal list for every object queued. Removing
a large tree within one compositor batch can therefore do quadratic list
search work. This is an observed hot path and a specific optimization
candidate, not a measured fix. Calendar's `populate_grids` also appears
in the template/application stacks. The profile also shows PNG decoding
and ColorSpectrum pixel generation on pages that need them.

The sampling categories use named stack frames; optimized inlining limits
exact attribution. `Loaded` is not a GPU presentation fence, so these
results do not quantify any additional render/presentation delay.

## What opening a page does

1. `MainWindowViewModel::navigate_to_async` calls the page factory synchronously
   on the UI thread. The dispatcher starts the future immediately through its
   first pending await. The desktop host installs no `PageAssets` hook, so it
   does not wait for a download here.
2. `PageItem::create_page` creates a fresh page on every visit. The document
   class constructor calls `initialize_component` / `markup::load_component`.
   The default build calls generated Rust population functions: it does not
   parse XML or compile/JIT page code. `runtime-markup` and `hot-reload` are
   exceptions and must be measured separately.
3. The shell's frame navigation creates a `NavigationPage`, pushes the page,
   attaches it to the tree, and updates the navigation selection. The first
   page of a new `NavigationPage` has no page transition.
4. Tree attachment applies control themes and styles. Theme resource entries
   can be deferred: `ResourceDictionary::try_get_value` builds them on first
   lookup and retains shared resources afterward. Compiled XAML still creates
   objects, property values, and bindings at runtime.
5. The first layout builds the controls' templates, applies their bindings
   and resources, measures and arranges them, and formats text. Reusing a
   theme/template factory does not reuse the visual tree of a previous page.

The catalog amplifies this work with large sample pages: `SamplePage` uses a
`ScrollViewer`, with ordinary `StackPanel` / `SampleGroup` children. The scroll
content presenter measures the content with infinite height; `StackPanel`
measures every child. The viewport does not virtualize these example sections.
`SampleSection` adds its title, description, options, stage and code-snippet
controls. Short code snippets start expanded; `CodeBlock::update_inlines`
creates highlighted runs and dynamic-resource bindings when its template is
applied.

## Diagnostics

`FERROUI_CATALOG_NAV_TIMING=1` enables cumulative wall-clock measurements from
the beginning of each navigation:

- `assets-ready`: optional asset preparation finished; effectively immediate
  on the desktop.
- `constructed`: page factory returned, including markup population and the
  constructor's code.
- `host-returned`: shell/navigator call returned.
- `loaded`: the page's first `Loaded` event. This covers pending UI work through
  the framework's loaded priority; it is not a GPU presentation fence or a
  measurement of time on glass.

`Markup timing` reports individual `load_component` durations, nested within
construction; do not add them to the cumulative navigation times. Logging is
opt-in and does not change navigation, caching, styles, or layout.

An ordered smoke sequence permits revisits without screenshot rendering:

```sh
cargo build --locked --profile dist -p control-catalog-desktop
FERROUI_CATALOG_NAV_TIMING=1 \
FERROUI_SMOKE_PAGE_SEQUENCE=Buttons,TextBox,ComboBox,TextBlock,ColorPicker,Calendar,TreeDataGrid,Buttons,TextBox,ComboBox,TextBlock,ColorPicker,Calendar,TreeDataGrid \
FERROUI_SMOKE_PAGES=1000 \
FERROUI_SMOKE_EXIT_MS=17000 \
target/dist/control-catalog-desktop
```

For sampling on macOS, retain executable symbols while keeping the dist
optimizations:

```sh
cargo rustc --locked --profile dist -p control-catalog-desktop \
  --bin control-catalog-desktop -- -C strip=none
sample <pid> 15 1 -file target/catalog-navigation.sample.txt
```

## Further optimization targets

1. Apply the Calendar page approach to other large sample galleries if
   profiling warrants it. A ScrollViewer around an ordinary StackPanel
   still measures all of its examples.
2. Continue measuring large-tree removal now that the pending-disposal
   scan is a direct index (on main, see above).
3. Continue profiling inherited-property propagation and style/resource
   walks. Calendar's hidden alternate cells are now deferred, and its
   catalog examples are virtualized as described below.
4. Defer snippet highlighting and expensive sample-specific work where
   useful. The profile does not establish snippets as the dominant cost.

Changing the default catalog's XAML parser or adding page-code loading
warmup would not address the measured work: the markup is already compiled.
Caching whole pages would trade memory and page-state behavior for latency;
that policy has not been changed by this investigation.

## Validation and artifacts

- Built the optimized catalog with symbols using the command above.
- Completed two native tours, each with three visits to all seven selected
  pages; all 21 requested page visits emitted Loaded and the smoke timer closed
  the window successfully in each run. One tour was sampled, one unsampled.
- Completed a diagnostics-disabled smoke launch: no timing output, successful
  timed shutdown.
- Existing compiler warnings remain.
- Raw local logs, timings (JSON), sampling output and the temporary analysis
  scripts are in `target/navigation-diagnostics/` (ignored build artifacts).
- The initial investigation added opt-in logging and ordered smoke navigation.
  Subsequent optimizations and their validation are described below.

## The implementation

The `Compositor` keeps its ordered pending-disposal entries with a direct
index by server-object ID (on main, see above). Queuing or upgrading a
request is O(1), rather than scanning the entire list per object.
Serialization preserves request order, coalesces dispose/release flags,
handles out-of-band disposal, and only reuses IDs after release is
committed. Out-of-band removal preserves order and reindexes the remaining
tail; that uncommon operation remains O(n).

`CalendarItem` no longer creates its 12 alternate month/year cells while
showing the day grid. They are created on the first Year/Decade update and
reused when switching modes. Starting in Year or Decade still initializes
the visible view. Day cells remain available because selection and day
lookup helpers use them in alternate modes too. This avoids 108 hidden
cell controls, their handlers, and style/resource attachment across the
catalog's nine calendars. It does not cache pages or remove any examples.

The queue has regression tests for request coalescing, order, out-of-band
disposal, and 10,000 IDs reused across batches. Calendar tests cover deferred cells, repeated mode switching
without duplicate cells, and initial Year/Decade mode.

The comparison runner is `scripts/measure-catalog-navigation.py`. It
alternates process order each round, fixes WinUI/light/Metal, returns to
Home between every target page, rejects incomplete or unexpected
navigation, and retains raw logs, observations, executable SHA-256s and
median summaries.

The Calendar catalog page also replaces its wrapping panel with an
`ItemsRepeater` / `UniformGridLayout` before attachment. Its nine original
example controls are retained as direct control items. The layout measures
the first example for theme-dependent cell dimensions; it does not hard-code
calendar heights. The existing styles, margins, maximum width, alignment,
name-scope registrations and example settings are preserved. Viewport caches
are zero, so only visible examples are realized; scrolling realizes later
examples while retaining their state. Tab navigation continues through the
items. The test covers wide/narrow wrapping, scrolling to the final example,
returning to the same selected-date control, configured date bounds and
blackouts, week numbers, and page cleanup.
