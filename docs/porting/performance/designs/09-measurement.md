# 09. Benchmarks and a regression gate

Status: items 1 and 2 are implemented (the native benchmark of recycling and the counters); see
"What is implemented". Their numbers have not been measured yet: the tables below are to be filled
by the session that builds and validates. Items 3 to 5 are design. `scripts/browser/scroll-profile.mjs`
(pull request 47) is the first piece of item 3.

## Finding

- **[M]** The figures of this folder come from one script run by hand on a shared machine; runs of
  the same module differ by up to 10 %.
- The trace that started the analysis was of the published module, which has no function names;
  the function order of a local build differs from the published one, so the names could not be
  mapped and the scrolling had to be reproduced.
- There is no measurement of the same work on the desktop, so it is not known how much of the cost
  is specific to WebAssembly.

## Design

1. **A native benchmark of recycling**, without a window: a headless application (the test
   services of `ferroui-base` with the Skia and HarfBuzz backends, as the catalog tests use them)
   that builds a table view with the data of the catalog page, lays it out, then scrolls it by a
   fixed sequence of offsets and reports time per recycled row and allocations per row. It runs
   under `cargo bench` or as an ignored test with a release profile, on Linux and macOS. This
   gives the designs a number that does not depend on a browser and can be profiled with native
   tools.
2. **Counters behind a feature.** A feature `perf-counters` of `ferroui-base` that counts property
   changes raised, changes with listeners, virtual calls by member, styles evaluated and matched,
   resource lookups, bindings created, text layouts created and cache hits. Off by default, no
   code when off. The designs 02 to 06 each start by reading these.
3. **Scrolling scenarios for the browser script**: besides the wheel, a drag of the scroll bar
   thumb (the case of the original trace: a viewport per event) and other pages with virtualized
   lists (ListBox, TreeView, the data grid when it is ported).
4. **A named module from CI.** The Pages workflow uploads, next to the published module, the same
   link with `--profiling-funcs` as a workflow artifact, so that a trace of the published site can
   be given names. **[H]** Linked in the same job from the same objects, the two should have the same function
   indices; an earlier comparison found their code sections 172 bytes apart, so the step includes
   checking that the indices line up.
5. **A regression gate.** The performance workflow (`.github/workflows/perf.yml`) runs the native
   benchmark and, in the browser job, `scroll-profile.mjs` against the base ref and the measured
   ref alternately, and reports the change; as with start-up times, a slower median is reported,
   not failed, until the noise of the runners is known.

## Expected gain

None by itself. It makes the estimates of the other designs measurable and keeps what they gain.

## Risks

Low. The counters must compile to nothing without the feature, which a size comparison of the
module checks.

## Verification

- The benchmark reports the same per-row figures on repeated runs within a few percent on an idle
  machine.
- The module with the feature off is byte-identical in size to the module before the counters.

## What is implemented

Written without a build: the session that validates builds it, runs it and fills the tables. No
figure below is measured yet.

### The benchmark of recycling (item 1)

`recycling_benchmark_table_view_scrolling` in `samples/ControlCatalog/tests/frame_benchmark.rs`, an
ignored test next to the frame benchmarks, on their harness (`Bench`: the catalog application under
the Fluent theme with the Skia and HarfBuzz backends, a window of 1280 by 800 over the mock
windowing platform). It shows the table view page of the catalog (the page itself, so the table,
its columns, its cell theme and its data are the ones of the sample) and scrolls the table by
setting the offset of its scroll viewer:

- **20 pixels per step**, 300 steps, 40 down and 40 up in turn (the sequence of the frame
  benchmark): a step recycles a row or none. The wheel case.
- **a viewport per step**, 120 steps, down to the end of the table and back: every step recycles
  the rows of a viewport. The case of the original trace (a drag of the scroll bar thumb).

Each scenario runs once unmeasured first, so that the recycle pool and the caches are filled.

What a step measures is **the layout pass alone**: the offset is set and the layout manager of the
window runs its pass (`ILayoutManager::execute_layout_pass`), in which the rows that left the
viewport are cleared and removed and rows of the pool are prepared for the items that entered it
and added. The frame that follows (the jobs of the dispatcher, the commit of the compositor, the
rendering into the raster surface) runs after the measurement of the step and is timed apart; it
is printed for reference and is not part of the per-row figures. The rows a step recycled are
counted from the outside, as the items that have a realized row after the pass and had none before
it. Should a frame realize a row (the layout pass would then not be the whole of the recycling),
the benchmark prints how many; `table_view_offset_scrolling_recycles_its_rows_in_the_layout_pass`,
which is not ignored, asserts that there are none.

It prints, per scenario: the steps and the rows recycled; the layout pass in milliseconds per step
(median, mean, 95th percentile, maximum); **microseconds of layout per recycled row** (the time of
all the passes over the rows they recycled, so the passes that recycled nothing are in it); with
the allocator feature the **allocations, reallocations and bytes per recycled row**; with the
counter feature the counters per recycled row.

**Allocations** come from a counting allocator in `samples/ControlCatalog/tests/allocations.rs`:
the system allocator behind per-thread counts of the blocks allocated, the blocks resized and the
bytes asked for. It is the global allocator of the test build of the sample only, and only with
the feature `count-allocations` of `control-catalog` (off by default); without it the file
declares no allocator.

### The counters (item 2)

`ferroui_base::diagnostics::perf_counters`
(`src/FerroUI.Base/diagnostics/perf_counters.rs`), with the feature `perf-counters` of
`ferroui-base`. `ferroui-controls`, `ferroui-markup-xaml` and `control-catalog` have a feature of
the same name that forwards to it.

A counting site is `perf_count!(Counter)` or `perf_count_virtual!("Class::member")`. Without the
feature the first expands to a block that names the counter and does nothing, the second to `()`;
the functions of the module have empty bodies; `PerfCountersSnapshot` has no fields, which a
constant assertion checks (`size_of == 0`), and another checks that both macros are constant
expressions of the unit type. No struct of the framework has a field for the counters in either
configuration: they are thread-local statics of the module, per thread, so what is read is what
the thread of the dispatcher did. `snapshot()` reads them, `PerfCountersSnapshot::since` and
`plus` subtract and add snapshots, `report()` and `report_per(n, "row")` format them, `reset()`
zeroes them.

No site changes logic: each is one statement added at the point named below.

| Counter | Site | Read by |
|---|---|---|
| property changes raised | `FerroObject::raise_property_changed`, on entry | 02 |
| property changes of an effective value | the same, inside `if is_effective_value` | 02 |
| property changes with handlers of the property | `PropertyChangedObservable::notify`, after the test for an empty handler list (class handlers and subscriptions to the property) | 02 |
| property changes with listeners of the object | `FerroObject::raise_property_changed`, inside the test for a non-empty listener list | 02 |
| virtual calls, by member | the public function of each virtual member that `ferro_class!` generates (`Class::member`, the class that declares the member), and the four virtual members of `FerroObject` (`constructed`, `on_property_changed_core`, `on_property_changed`, `update_data_validation`) | 01 |
| inheritance ancestor changes | `ValueStore::set_inheritance_parent`, after the test that the ancestor is the same | 03 |
| inherited values compared, and differing | the same, in the loop over the paired old and new values | 03 |
| objects visited by inherited value walks | `ValueStore::inherited_value_changed`, on entry (once per object and property) | 03 |
| elements attached to, and detached from, a logical tree | `StyledElement::on_attached_to_logical_tree_core` and `on_detached_from_logical_tree_core`, where the logical root is set and cleared | 04 |
| implicit theme lookups | `StyledElement::get_effective_theme`, where the implicit theme is unknown and is looked up | 04 |
| style hosts walked | `StyledElement::apply_styles`, once per host | 04 |
| styles evaluated, and matched | `Style::try_attach_checked` and `ContainerQuery::try_attach_checked`: where the selector (the query) is matched, and where the match instances the style | 04 |
| control themes evaluated, and matched | `ControlTheme::try_attach_checked` | 04 |
| style instances attached, and created | `StyleBase::try_attach_instance`: every instance added to a value store, and the ones built instead of shared | 04 |
| resource lookups, and resource hosts probed | `IResourceHost::try_find_resource`: on entry, and before each `try_get_resource` of the walk | 04, 05 |
| bindings instanced | `FerroObject::bind_binding_with_anchor` | 05 |
| binding expressions created | `UntypedBindingExpressionBase::new` and `TypedBindingExpression::new` | 05 |
| template binding expressions created | `TemplateBindingExpression::new` | 05 |
| dynamic resource expressions created | `DynamicResourceExpression::new` (`ferroui-markup-xaml`) | 05 |
| binding values published | `UntypedBindingExpressionBase::publish_value` of a running expression, `TypedBindingExpression::publish_value` | 05 |
| text layouts created | `TextLayout::from_text_source` | 06 |
| text lines formatted | `TextFormatterImpl::format_line_with_cache`, on entry | 06 |
| text run cache hits, and misses | the same: where the cached shaped runs are taken, and where newly shaped runs are added to a cache | 06 |
| text runs shaped | `TextShaper::shape_text` | 06 |
| containers recycled, reused, created | `VirtualizingStackPanel::recycle_element` (the branch that clears the container and pushes it to the pool), `get_recycled_element`, `create_element` (`ferroui-controls`) | 02, 05 |
| content presenter children replaced | `ContentPresenter::update_child_with`, where the new child is not the old one (`ferroui-controls`) | 04, 06 |

What the designs ask for and is not counted yet: the time in each part of a notification (02),
the size of the subtree of a parent change as such (03: the walk visits give it per property), the
selectors evaluated inside one style and the dictionaries probed inside one host (04), bindings
disposed (05), the split of text layout time (06). Times are for a profiler, on the benchmark of
item 1. Design 07 names no site: nothing is counted in the compositor.

The tests of the counters are `src/FerroUI.Base/diagnostics/perf_counters_tests.rs`, compiled only
with the feature: a property change with and without a listener, a handler of the property, the
virtual calls by member, an inheritance ancestor change, a style that matches and one that does
not, a resource lookup, two text layouts over one text run cache (a miss, then a hit), reset and
the report.

### Commands

The benchmark, in the `release` profile and in the `dist` profile (the one that is shipped and
measured); `--test-threads=1` keeps the machine to the one benchmark:

```sh
cargo test -p control-catalog --release --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --profile dist --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
```

With the counting allocator (allocations per recycled row):

```sh
cargo test -p control-catalog --release --features count-allocations --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
cargo test -p control-catalog --profile dist --features count-allocations --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
```

With the counters (counters per recycled row; the profile does not change a count, so one is
enough):

```sh
cargo test -p control-catalog --release --features perf-counters --lib recycling_benchmark -- --ignored --nocapture --test-threads=1
```

Take the times from the runs without a feature: counting costs time, and the table of the virtual
calls allocates, so allocations are taken without the counters as well.

The tests: the counters, the check of the off case (the constant assertions are part of every
build of `ferroui-base` without the feature), the allocator and what the benchmark relies on:

```sh
cargo test -p ferroui-base --features perf-counters --lib perf_counters
cargo test -p ferroui-base --lib
cargo test -p control-catalog --lib allocations
cargo test -p control-catalog --features count-allocations --lib allocations
cargo test -p control-catalog --lib table_view_offset_scrolling_recycles_its_rows_in_the_layout_pass
cargo test -p control-catalog --features perf-counters,count-allocations --lib table_view_offset_scrolling_recycles_its_rows_in_the_layout_pass
```

To profile the benchmark with native tools, build the test executable once
(`cargo test -p control-catalog --profile dist --lib --no-run` prints its path) and run it under
the profiler with `recycling_benchmark --ignored --nocapture --test-threads=1`.

### Results

First figures, one run each on 2026-10-09 on the development machine (Apple silicon, macOS) while other builds were running, so the times are an upper bound and are to be repeated on an idle machine, three runs each. The counts do not depend on the load. The time and the row counts are from the run without features, the allocations from the run with `count-allocations`.

| Scenario | Profile | Rows recycled | Layout pass ms per step, median | Layout us per recycled row | Allocations per row | Bytes per row |
|---|---|---:|---:|---:|---:|---:|
| 20 px per step | `release` | 169 | 0.221 | 411.3 | 1224.0 | 112096 |
| 20 px per step | `dist` | not measured yet | | | | |
| a viewport per step | `release` | 2454 | 4.968 | 245.7 | 1186.3 | 108803 |
| a viewport per step | `dist` | not measured yet | | | | |

Run to run difference of the per-row time (the first point of "Verification"): not measured yet (one run).

Counters per recycled row, from the run with `perf-counters`:

| Counter | 20 px per step | A viewport per step | Design |
|---|---:|---:|---|
| property changes raised | 459.24 | 443.50 | 02 |
| property changes of an effective value | 439.24 | 423.45 | 02 |
| property changes with handlers of the property | 215.51 | 202.81 | 02 |
| property changes with listeners of the object | 421.27 | 405.41 | 02 |
| virtual calls (all members) | 2835.50 | 2682.88 | 01 |
| inheritance ancestor changes | 12.00 | 12.03 | 03 |
| inherited values compared | 84.00 | 84.21 | 03 |
| inherited values differing | 84.00 | 84.21 | 03 |
| objects visited by inherited value walks | 228.00 | 228.56 | 03 |
| elements attached to a logical tree | 18.00 | 18.04 | 04 |
| elements detached from a logical tree | 18.00 | 18.04 | 04 |
| implicit theme lookups | 17.00 | 17.04 | 04 |
| style hosts walked | 261.00 | 261.64 | 04 |
| styles evaluated | 0.00 | 0.00 | 04 |
| styles matched | 0.00 | 0.00 | 04 |
| control themes evaluated | 4.00 | 4.01 | 04 |
| control themes matched | 2.00 | 2.00 | 04 |
| style instances attached | 2.00 | 2.00 | 04 |
| style instances created | 2.00 | 2.00 | 04 |
| resource lookups | 43.00 | 43.11 | 04 |
| resource hosts probed | 467.00 | 468.14 | 04 |
| bindings instanced | 5.00 | 5.01 | 05 |
| binding expressions created | 7.00 | 7.02 | 05 |
| template binding expressions created | 0.00 | 0.00 | 05 |
| dynamic resource expressions created | 1.00 | 1.00 | 05 |
| binding values published | 86.42 | 85.22 | 05 |
| text layouts created | 10.00 | 10.02 | 06 |
| text lines formatted | 10.00 | 10.02 | 06 |
| text run cache hits | 5.00 | 5.01 | 06 |
| text run cache misses | 5.00 | 5.01 | 06 |
| text runs shaped | 5.00 | 5.01 | 06 |
| containers recycled | 1.00 | 1.00 | |
| containers reused | 1.00 | 1.00 | |
| containers created | 0.00 | 0.00 | |
| content presenter children replaced | 10.00 | 10.02 | 04 |

The ten most called virtual members per recycled row (20 px per step): `StyledElement::styling_parent` 764.00; `FerroObject::on_property_changed_core` 459.24; `FerroObject::on_property_changed` 439.24; `ResourceProvider::try_get_resource` 343.00; `StyledElement::is_logical_root` 96.00; `Interactive::interactive_parent` 76.33; `InputElement::is_enabled_core` 56.00; `Visual::bypass_flow_direction_policies` 36.00; `Layoutable::arrange_core` 35.75; `Layoutable::arrange_override` 35.75.

The second point of "Verification" (the size of the browser module with the feature off, before
and after the counters): to be measured, with `scripts/browser/module-sizes.mjs` on a build of
this branch and of the commit before it.

### What is left

- Items 3 to 5: the scenarios of the browser script, the named module from CI, the gate. The
  native benchmark is not yet run by `.github/workflows/perf.yml`.
- The benchmark on Linux has not been run.
