# LiveCharts2 port

This is the work record for the native Rust port of LiveCharts2 to FerroUI. The
target is the behavior of the pinned upstream implementation, including its data,
layout, interaction, animation, drawing and control services. The port uses
FerroUI's property system, collections, drawing context, input, text and markup
infrastructure wherever those services already provide the required behavior.

**Status: implementation in progress.** Source inventory, API discovery and chart
family coverage are separate from verified behavioral equivalence. The
[machine-readable summary](data/livecharts2/summary.json) records the current
source, member and test dispositions. A partial implementation is never promoted
to a complete port because it has the same type name or can draw an example.

## Reference and scope

The reference is [Live-Charts/LiveCharts2 at
`8537b90ffb100a947cd0b1f6be49f896ae6050eb`](https://github.com/Live-Charts/LiveCharts2/tree/8537b90ffb100a947cd0b1f6be49f896ae6050eb).
Its license is MIT, copyright 2021 Alberto Rodriguez Orozco. The
[reference descriptor](data/livecharts2/reference.json) is the authority for the
pin and license. The upstream checkout does not have to remain on that commit:
the inventory tool reads Git objects at the pin without changing the checkout.

The pinned source includes Cartesian, pie, polar, geographic, Sankey and treemap
engines. It also contains range bars and lines, box plots, financial series,
per-element paint animation and visual-state machinery. Those are included in
the scope even when older LiveCharts2 tutorials do not mention them.

The source inventory includes every tracked file under `src`, `generators` and
`build`, including source generators, shared platform wrappers, resources and
project files. Tests and samples have their own inventory sections. Binary image
baselines are recorded by hash without copying them into FerroUI. The source
scope has **698 files, including 644 C# files and 79,040 C# lines**; samples have
2,028 files, including 1,133 C# files. These counts describe the pinned reference,
not implemented Rust code.

## Porting rules

The [FerroUI porting guide](PORTING-GUIDE.md) applies. In particular:

- Preserve observable semantics, edge cases, ownership and event ordering.
- Keep one source file per Rust file where possible; record every deliberate
  split, merge or replacement in the map.
- Prefer safe Rust. Do not introduce a parallel property system, unmanaged
  ownership graph, blocking timer thread or separate graphics runtime.
- Reuse FerroUI collection handles and disposable subscriptions. Reference
  identity, mutation notifications and subscription lifetime are part of the
  contract, including collections shared between charts.
- Record missing work explicitly. Missing members are absent, rather than
  implemented by `todo!()`, unconditional success or a visually similar fallback.
- Preserve upstream copyright and MIT text beside derived code. The renderer
  dependency choices do not remove the attribution requirement.

## Dependency mapping

| Upstream responsibility | FerroUI service or Rust design | Behavior to retain |
| --- | --- | --- |
| Chart, axis, series and default data models | Typed Rust data; native property metadata for bindable objects | Null points, mapping, bounds, notifications, visibility and stable identity |
| `INotifyPropertyChanged` / observable collections | `FerroObject` properties, `FerroList<T>`, native event subscriptions | Mutation through shared handles, replacement, detach and deep changes |
| `LvcPoint`, `LvcSize`, rectangles, colors and transforms | Existing FerroUI geometric and media values where compatible | Precision, coordinate conventions and transform order |
| SkiaSharp painting, geometry and clipping | `DrawingContext`, brushes, pens, geometries and clips | Stroke/fill, ordering, clipping, opacity and geometry hit testing |
| SkiaSharp/HarfBuzz text services | FerroUI text layout and its configured shaping backend | Font selection, label measurement, alignment, wrapping and shaping |
| Frame tickers and update throttling | FerroUI dispatcher/render clock; deterministic caller-supplied time in mathematical code | Coalescing, invalidation, animation completion and disposal |
| Platform pointer controllers | Native routed pointer, wheel, capture and gesture services | Hover, release/capture loss, drag/pan, zoom anchor and pointer coordinates |
| Generated bindable and XAML wrapper properties | `ferro_properties!`, `ferro_class_info!`, type and markup registration | Property names, defaults, binding priority, content and collection behavior |
| Roslyn generators | Native Rust metadata/macros; existing FerroUI build pipeline | Resulting API and behavior, without requiring a .NET runtime |
| `System.Text.Json` in map loading | A Rust parser with explicit typed errors | GeoJSON shape validation, polygon holes, supported geometry and feature metadata |
| UI host packages for individual .NET frameworks | One FerroUI control layer over FerroUI platform backends | Load/unload, layout, input, theme, accessibility and rendering lifecycle |
| `IDisposable`, managed weak references and finalization | Existing disposable subscriptions plus `Rc`/`Weak` or native `Ref`/`WeakRef` | No stale callbacks, leaks or strong ownership cycles |

The reference core project uses `System.Text.Json` and the build-time
`LiveChartsGenerators` project. Its SkiaSharp view project adds SkiaSharp and
SkiaSharp.HarfBuzz. The native Rust port does not require those managed libraries
to render a chart: the active FerroUI drawing/text backend supplies those
services. Existing FerroUI Skia support remains a host choice, alongside the
other drawing backends.

## Behavior map

Each row names a complete upstream responsibility. The source/member/test maps
remain the authoritative implementation status; this table is the review
checklist and does not mark a feature complete.

| Family | Required behavior |
| --- | --- |
| Cartesian engine | Draw margin and layout negotiation; multiple axes; updates; visibility; chart/series lifecycle; clipping; scaling and rendering order |
| Axes | Linear, labels/category, date/time and duration formatting; inversion; limits; ticks and subseparators; sections; zero line; visible bounds and shared axes |
| Line and step line | Null gaps, interpolation, smoothing, geometry markers, fill, error visuals, data labels, hover and transitions |
| Bars | Rows and columns, grouping, stacking and stack groups, positive/negative values, maximum width, baseline/pivot, spacing, tooltips and labels |
| Range series | Range columns/rows, range bands, time-valued dimensions, automatic bounds and tooltip anchors |
| Scatter and heat | Point dimensions/weights, heat interpolation and stops, cell sizing, bounds, hover and legend |
| Financial and box | OHLC/candlesticks, open/close/high/low conventions, box quartiles and whiskers, negative or reversed axes and transitions |
| Pie and gauges | Donut radius, angle origin/extent, radial alignment, push-out, labels, hit sectors, multiple concentric series and gauge composition |
| Polar | Angle/radius axes, scaler, inversion, labels, closed lines, null points and radial layout |
| Geographic | GeoJSON import, map/layer/land identity, projections, longitude seams, heat colors, map hover, pan/zoom/rotation and clipping |
| Sankey | Data validation, directed graph, layout, node/link measures, ribbons, arc/chord layout, labels, colors and interaction |
| Treemap | Hierarchical values, layout, padding, sorting, node identity, labels, interaction and updates |
| Motion | Easing, keyframes, property transitions, interruption, nullable values, paint references, color/gradient/effect transitions, removal and frame scheduling |
| Visual elements | Labels, geometry, lines, needles, angular ticks, relative/stack/table layouts, measurement and interaction events |
| Themes and states | Palette, chart/axis/series rules, post rules, per-element state setters, conditional styling and light/dark transitions |
| Data services | Generic mappings, default observable model types, stable chart entities, bounds/minimum delta, observers, deep changes, series source and cleanup |
| Interaction | Pointer events and capture, nearest-point strategies, hover enter/leave, legends/tooltips, pan/zoom policies, programmatic interaction and coordinate conversion |
| Native controls | Styling, binding, XAML, measurement, invalidation, attach/detach, accessibility, templates and host integration |

## Inventory and evidence

The tracking files serve different purposes:

| File | Meaning |
| --- | --- |
| [inventory.json](data/livecharts2/inventory.json) | Complete tracked source, test and sample file paths; byte counts; SHA-256 hashes; text line counts |
| [upstream-api.json](data/livecharts2/upstream-api.json) | Roslyn syntax extraction of source types and nonprivate members, including signatures, modifiers, attributes and line locations |
| [source-map.json](data/livecharts2/source-map.json) | A disposition and Rust path list for every source/build/resource file |
| [api-map.json](data/livecharts2/api-map.json) | An independent disposition for every extracted nonprivate source member |
| [test-map.json](data/livecharts2/test-map.json) | Every discovered test declaration, including parameter attributes, profile availability and reviewed Rust counterparts |
| [mapping.json](data/livecharts2/mapping.json) | Hand-reviewed source/member/test mappings; all other records default to pending |
| [summary.json](data/livecharts2/summary.json) | Counts by disposition and test suite, calculated from those records |

Roslyn extraction reuses FerroUI's existing
`scripts/api-extract/Program.cs`. The small project under `scripts/livecharts2`
references Roslyn from the installed .NET SDK, so the extraction does not need a
NuGet package or an upstream solution build. It parses a temporary export of the
pinned source tree and unions the declarations found under the named platform
profiles in `scripts/livecharts2/profiles.json`. A declaration's `profile_mask`
refers to the masks recorded in `upstream-api.json`. Hashes of the extractor and
its configuration are stored with the extraction and checked for drift.

The API inventory is a syntax inventory, not a compiled semantic API model.
Roslyn-generated declarations are tracked through their generator/template source
and must be matched to the Rust metadata surface separately. Private helper
members are represented by the complete file inventory, not by the nonprivate
member map. The generated API records retain all selected profile signatures;
they do not establish whether a specific platform builds successfully.

Test counts are **method declarations**, not successful executions. A
parameterized method is counted once and its data attributes are retained.
Core, snapshot, generator, console and shared UI tests are accounted for
separately. Benchmark source and golden images are in the file inventory;
benchmarks are not counted as unit tests. A passing native regression is recorded
as an upstream test counterpart only after its assertions and applicable data
rows have been reviewed.

### Dispositions

| Status | Meaning |
| --- | --- |
| `pending` | No reviewed implementation or test counterpart is recorded |
| `partial` | Rust behavior exists; the entry lists known remaining obligations |
| `ported` | The mapped implementation/test has been reviewed for the stated upstream contract |
| `native-replacement` | Existing native services provide the responsibility; the entry identifies those paths and the reason |
| `not-applicable` | A concrete host/build responsibility does not apply to the Rust/FerroUI deployment; a reason is required |

Every non-pending mapping must supply a reason. An implemented mapping must
identify existing Rust files. Incomplete mappings must state remaining work.
Marking a file as an adaptation does **not** mark all its API members complete.
Likewise, mapping a control to native input does not prove its routed input
behavior or all upstream UI tests.

## Regeneration and checks

From the FerroUI repository root, the checked-in records can be validated without
the reference checkout, network access or .NET:

```sh
python scripts/livecharts2/inventory.py --check
```

To verify all recorded file hashes against the pinned Git tree:

```sh
python scripts/livecharts2/inventory.py --upstream ../LiveCharts2 --check
```

After reviewing entries in `mapping.json`, regenerate the disposition maps and
summary with `--refresh`. To re-extract the API from the pin as well, use .NET 10:

```sh
python scripts/livecharts2/inventory.py \
  --upstream ../LiveCharts2 --refresh --extract-api
```

`--dotnet /path/to/dotnet` selects an SDK executable explicitly. The command does
not modify the upstream checkout or fetch a newer reference. To update the
reference, first review the new source boundary, change `reference.json`, then
regenerate and review source, API and test differences together.

The completion gate is deliberately stronger than metadata consistency:

```sh
python scripts/livecharts2/inventory.py --check --require-complete
```

It fails while any source, member or test entry is pending or partial. Ordinary
`--check` verifies that the records are complete and consistent, so work can be
committed incrementally without hiding incomplete port coverage.

## Validation required before completion

Verification must cover source contracts as well as the integrated controls:

1. Port and run the corresponding upstream mathematical, data and interaction
   tests, including parameterized cases and degenerate/invalid inputs.
2. Exercise native property bindings and collection changes through real
   controls. Test shared ownership, mutation, reentrancy, detach and disposal.
3. Load chart/axis/series markup through runtime and compiled XAML. Verify that
   defaults do not mask bindings or templates.
4. Capture actual rendering through a FerroUI backend, including text, clipping,
   gradients, light/dark themes and live mutation. Record the backend and size.
5. Verify transitions with a deterministic clock, including interrupted updates,
   removal, completion and geometry/paint continuity.
6. Compile for the supported native and browser targets and distinguish
   compilation from interactive execution. Platform input and accessibility
   behavior need host-level evidence.
7. Check the source/member/test maps and document every retained deviation before
   marking the work complete.

Completed test commands and their exact results are added to this record as the
implementation progresses. The inventory check itself supplies no rendering or
runtime evidence.

### Tracking validation, 2026-10-23

| Check | Result |
| --- | --- |
| Roslyn extraction across 13 symbol profiles | 852 C# files parsed per profile; union records 4,859 source API member declarations |
| Source hash verification against the pinned Git tree | Passed: all 698 source files and the recorded test/sample files matched |
| Offline generated-record consistency check | Passed |
| Test declaration discovery | 1,066 declarations: 826 core, 172 snapshot, 33 console, 32 shared UI and 3 generator |
| Completion gate (`--require-complete`) | Correctly fails while unimplemented/unreviewed records remain |

No upstream test execution is implied by these declaration counts.
