# LinedFlowLayout source audit

This record describes the LinedFlow corrections reviewed during the
2026-10-20 continuation of PR #269. The acceptance source remains
[`microsoft-ui-xaml@7e4b0c2096be402f51cc8f7d203ebf67cff183d0`](https://github.com/microsoft/microsoft-ui-xaml/blob/7e4b0c2096be402f51cc8f7d203ebf67cff183d0/controls/dev/Repeater/LinedFlowLayout.cpp).
The changes preserve the source's observable contracts while using FerroUI
controls, layout contexts, dispatcher timers and Rust-owned metadata.

The general source inventory is in [SOURCES.md](SOURCES.md). The actual runtime
checks and their revisions are recorded separately in
[VALIDATION.md](VALIDATION.md); the scenarios below are implementation and
test mappings, not a claim that native Windows equivalence has been measured.

## Reviewed contracts

| Pinned WinUI implementation | Rust correction | Behavioral regression |
|---|---|---|
| `RequestItemsInfo`, `UpdateItemsInfo`, `GetMinWidthFromItemsInfo`, `GetMaxWidthFromItemsInfo` | Retain raw per-item limits across an overlapping metadata request. Preserve an absent optional width array instead of manufacturing `-1` entries. Apply the latest global limits to both retained and newly supplied ratios. | `regular_metadata_global_maximum_limits_realized_controls_without_per_item_limits`; `current_global_limits_apply_to_retained_raw_metadata_without_fabricated_width_arrays`; `lowering_global_minimum_applies_to_retained_metadata_overlap` |
| `GetDesiredSizeForAvailableSize`, `UpdateActualLineHeight` | When automatic line height probes an existing first item, restore its last allocated width, falling back to its recorded natural width. If no restoration width is recorded, ask the native context for a separate `FORCE_CREATE` probe. | `automatic_line_height_probe_restores_the_metadata_allocation`; `automatic_line_height_uses_a_separate_probe_when_no_restorable_width_is_recorded` |
| `MeasureUnconstrainedLine` | Remember which controls were realized before an unconstrained measure. Start the deferred measurement timer when a newly realized control still reports its minimum width. | `unconstrained_new_minimum_width_items_start_delayed_measurement` |
| `EnsureItemRangeFastPath`, `GetFirstAndLastDisplayedLineIndexes` | Realize the exact host rectangle for full-source metadata. Compute displayed lines with the source's edge, spacing, negative-offset and stale-offset rules. Regular-path sizing and realization padding remains confined to the regular path. | `lined_flow_fast_path_virtualizes_full_source_metadata_and_reuses_distribution`; `lined_flow_displayed_lines_use_host_boundaries_spacing_and_overscroll` |
| `EnsureItemRangeFastPath`, `MeasureItemRange` | Preserve the existing realized range while the fast-path realization rectangle has zero height. Remeasure every retained control when a width change moves its range boundaries into partial lines. Reuse the source distribution when the viewport becomes visible again. | `fast_path_preserves_realization_while_viewport_is_zero_and_resumes_from_the_host_window`; `fast_path_remeasures_partial_retained_lines_when_resized_at_zero_height` |
| `EnsureItemRangeFastPath`, `MeasureConstrainedLinesRegularPath` | Center a disconnected anchor using the host rectangle's original height. Keep the anchor through the last retained measure, then expire it. Fast-path zero-height passes leave the saved recommendation and countdown untouched. | `fast_path_centers_only_the_host_realization_window_on_a_disconnected_anchor` |
| `ExitRegularPath`, `ResetSizedLines` | Clear the regular-path frozen-range diagnostics on entering the fast path, even though the fast path knows all source line allocations. | `lined_flow_fast_path_virtualizes_full_source_metadata_and_reuses_distribution` |

### Optional width arrays are observable

The pinned `GetMaxWidthFromItemsInfo` takes the minimum of a supplied per-item
value and the global maximum, including the `-1` sentinel. With no per-item
array, it returns the global maximum directly. Those two cases must remain
distinct. The regular metadata merge previously created an array of `-1`
entries even when the application only supplied a global `MaxWidth`, so the
layout lost that constraint. The Rust vectors now remain empty unless retained
or newly received metadata actually provides the optional array.

Global limits are kept outside the raw per-item vectors. Baking an earlier
global minimum into an overlapping item's stored value would prevent a later
request from lowering that minimum. The direct overlap regression checks both
the retained and newly appended portions without creating controls.

The original integration regression used a 40-DIP scroll to require another
metadata request. Both that distance and a later 200-DIP correction remained
inside startup prefetch in integrated execution, so the setup failed before
exercising the minimum change. The regression now reads the actual metadata
range and scrolls in viewport-sized steps until it requests another interval.
It then checks realized controls whose metadata came from the original range
and was never supplied again by any later request. At least one such control
must exist, and all must use the lower global limit. Returning to index zero
and rerequesting its metadata would not establish retained-overlap behavior.

### Native measurement and scheduling

FerroUI `Control::measure` updates the control's measurement state. An
automatic-height probe therefore restores the width with which that control
was actually allocated; metadata and scaled rows can differ from the natural
desired width. The fallback probe uses `LayoutContext` realization options,
leaving its ordinary automatic recycling to the host. No temporary control
tree or separate measuring subsystem is introduced.

Deferred sizing uses the existing `DispatcherTimer` and generation-checked
invalidation callback. The unconstrained path now preserves the source's
new-item/minimum-width trigger even though its Rust range-management helper
realizes all controls before collecting their metrics. Metadata still stops
control-based delayed measurement, and uninitialization still cancels the
timer through the native lifetime path.

### Fast realization and regular estimation

Full-source metadata fixes the entire line distribution. The fast path uses
that distribution to index the first and last displayed lines directly, then
measures only the retained range. A 100-DIP host rectangle containing five
20-DIP lines with five items per line realizes exactly 25 items; merely
touching the next line's edge does not add another line.

The regular path still estimates unknown lines, sizes surrounding items and
freezes a wider interval. Its frozen-window helper reuses the extracted
displayed-line calculation before applying the source's frozen padding.
Direct realization lookup removes the source-length scan from that lookup;
the current extent calculation still reduces the stored fast-path line
widths. No frame-time or memory benchmark is implied by these changes.

## Validation boundaries

The nine pure algorithm tests preceding the metadata tests in
`lined_flow_layout_tests.rs` were also compiled in a standalone `rustc --test`
harness during this audit. All nine passed, including twelve host-boundary,
spacing and overscroll cases plus the existing frozen-window, constraint,
fast-distribution and locked-partition checks. That harness imports the same
`lined_flow_layout_algorithm.rs` used by the crate; it does not compile or
exercise FerroUI control or dispatcher integration.

The native regressions live in `lined_flow_layout_tests.rs`,
`lined_flow_lifecycle_tests.rs` and `lined_flow_upstream_tests.rs`. Run them
through the normal crate to cover the framework integration:

```sh
cargo test -p ferroui-controls --lib repeater::layouts::lined_flow
```

Use [VALIDATION.md](VALIDATION.md) for the recorded execution result, complete
Repeater suite and platform gates. These source-reviewed corrections do not
substitute for the remaining visual and performance comparisons.
