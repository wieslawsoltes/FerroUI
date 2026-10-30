// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.

use super::layout_tests::{base, run, TestHost};
use super::lined_flow_layout_algorithm::{
    compute_fast_path, compute_regular_path, displayed_line_range, frozen_line_range, line_scale,
    ItemMetrics, LineOptions,
};
use super::*;
use ferroui_base::{Rect, Size};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

fn options(available: f64, count: usize) -> LineOptions {
    LineOptions {
        available,
        spacing: 0.0,
        stretch: false,
        first_line: 0,
        begin_line_vector_index: 0,
        line_count: count,
        total_item_count: count as i32,
        average_items_per_line: 1.0,
        lines_per_viewport: 10,
        forward: true,
        wrap_multiplier: 2.0,
    }
}
fn metrics(widths: &[f64]) -> Vec<ItemMetrics> {
    widths
        .iter()
        .enumerate()
        .map(|(index, width)| ItemMetrics {
            index: index as i32,
            width: *width,
            minimum: 0.0,
            maximum: f64::INFINITY,
        })
        .collect()
}
fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-8, "{actual} != {expected}");
}

#[test]
fn lined_flow_displayed_lines_use_host_boundaries_spacing_and_overscroll() {
    let cases = [
        (100.0, 0.0, 0.0, 100, (0, 4)),
        (100.0, 200.0, 0.0, 100, (10, 14)),
        (1.0, 20.0, 5.0, 100, (-1, -1)),
        (5.0, 20.0, 5.0, 100, (-1, -1)),
        (1.0, 24.5, 5.0, 100, (1, 1)),
        (20.0, 25.0, 5.0, 100, (1, 1)),
        (20.0, 1000.0, 5.0, 3, (2, 2)),
        (100.0, -50.0, 0.0, 100, (0, 2)),
        (100.0, -200.0, 0.0, 100, (-1, -1)),
        (100.0, -0.009, 0.0, 100, (0, 4)),
        (0.0, 0.0, 0.0, 100, (-1, -1)),
        (f64::INFINITY, 0.0, 5.0, 3, (0, 2)),
    ];
    for (viewport, offset, spacing, count, expected) in cases {
        assert_eq!(
            displayed_line_range(viewport, offset, 20.0, spacing, count),
            expected,
            "viewport={viewport}, offset={offset}, spacing={spacing}, count={count}"
        );
    }
}

#[test]
fn lined_flow_frozen_lines_cover_large_sized_regions_and_small_cache_boundaries() {
    // Displayed lines are 40..=44. With a large realization cache, 40% of
    // either adjacent sized region is larger than 80% of the viewport.
    assert_eq!(
        frozen_line_range(100.0, 800.0, 20.0, 0.0, 100, (0, 99)),
        (24, 66)
    );
    assert_eq!(
        frozen_line_range(100.0, 800.0, 20.0, 0.0, 100, (36, 48)),
        (36, 48)
    );
    assert_eq!(
        frozen_line_range(f64::INFINITY, 0.0, 20.0, 5.0, 100, (0, 99)),
        (0, 99)
    );
}

#[test]
fn lined_flow_frozen_lines_ignore_spacing_and_clamp_stale_scroll_offsets() {
    assert_eq!(
        frozen_line_range(1.0, 20.0, 20.0, 5.0, 100, (0, 99)),
        (-1, -1)
    );
    assert_eq!(
        frozen_line_range(20.0, 25.0, 20.0, 5.0, 100, (1, 1)),
        (1, 1)
    );
    assert_eq!(
        frozen_line_range(20.0, 1000.0, 20.0, 5.0, 3, (0, 2)),
        (1, 2)
    );
    assert_eq!(
        frozen_line_range(0.0, 0.0, 20.0, 5.0, 100, (0, 99)),
        (-1, -1)
    );
}

#[test]
fn lined_flow_constraint_aware_expansion_and_shrinkage() {
    let mut items = metrics(&[20.0, 20.0]);
    items[0].maximum = 25.0;
    close(line_scale(&items, 100.0, 0.0, true), 3.75);
    let mut items = metrics(&[100.0, 100.0]);
    items[0].minimum = 80.0;
    close(line_scale(&items, 100.0, 0.0, false), 0.2);
    items[1].minimum = 80.0;
    assert_eq!(line_scale(&items, 100.0, 0.0, false), 0.0);
}

#[test]
fn lined_flow_metadata_shrink_preserves_upstream_constraint_ordering() {
    // WinUI ComputeLineShrinkFactor probes MaxWidth before MinWidth, while
    // GetArrangeWidth applies MinWidth before MaxWidth to its initial width.
    // These intentionally differ for contradictory application metadata.
    let mut items = metrics(&[20.0, 100.0]);
    items[0].minimum = 100.0;
    items[0].maximum = 20.0;
    close(line_scale(&items, 110.0, 0.0, false), 0.5);
}

#[test]
fn lined_flow_fast_path_minimizes_distortion_and_shrinks_without_fill() {
    let items = metrics(&[40.0, 40.0, 40.0]);
    let lines = compute_fast_path(&items, options(100.0, items.len()));
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].widths.len(), 3);
    close(lines[0].desired_width, 100.0);
    let items = metrics(&[120.0, 20.0, 30.0]);
    let lines = compute_fast_path(&items, options(100.0, items.len()));
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].widths, vec![100.0]);
    assert_eq!(lines[1].widths, vec![20.0, 30.0]);
}

#[test]
fn lined_flow_fast_path_commits_full_line_before_zero_width_successor() {
    let mut items = metrics(&[100.0, 0.0, 20.0]);
    items[1].maximum = 0.0;
    let lines = compute_fast_path(&items, options(100.0, items.len()));
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].widths, vec![100.0]);
    assert_eq!(lines[1].widths, vec![0.0, 20.0]);
    assert_eq!(lines[1].first, 1);
}

#[test]
fn lined_flow_fast_path_minimums_prevent_shrink_and_final_line_stays_unfilled() {
    let mut items = metrics(&[40.0, 40.0, 40.0]);
    for item in &mut items {
        item.minimum = 40.0;
    }
    let mut configuration = options(100.0, items.len());
    configuration.stretch = true;
    let lines = compute_fast_path(&items, configuration);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].widths, vec![50.0, 50.0]);
    assert_eq!(lines[1].widths, vec![40.0]);
}

#[test]
fn lined_flow_regular_partition_preserves_public_locks_in_both_directions() {
    let widths = [
        42.0, 20.0, 65.0, 12.0, 27.0, 50.0, 18.0, 30.0, 48.0, 18.0, 60.0, 14.0, 29.0, 52.0, 33.0,
        22.0, 38.0, 61.0, 24.0, 34.0,
    ];
    let items = metrics(&widths);
    let locks = BTreeMap::from([(6, 2), (7, 2), (14, 5)]);
    for forward in [true, false] {
        let mut configuration = options(100.0, items.len());
        configuration.line_count = 7;
        configuration.average_items_per_line = 20.0 / 7.0;
        configuration.spacing = 3.0;
        configuration.forward = forward;
        let lines = compute_regular_path(&items, configuration, &locks);
        assert_eq!(lines.len(), 7);
        assert_eq!(
            lines.iter().map(|line| line.widths.len()).sum::<usize>(),
            items.len()
        );
        let mut next = 0;
        for line in &lines {
            assert_eq!(line.first, next);
            assert!(!line.widths.is_empty());
            next = line.end();
        }
        for (index, expected_line) in &locks {
            assert_eq!(
                lines
                    .iter()
                    .find(|line| *index >= line.first && *index < line.end())
                    .unwrap()
                    .index,
                *expected_line
            );
        }
    }
}

#[test]
fn lined_flow_metadata_ranges_array_lengths_and_lifetime() {
    let args = LinedFlowLayoutItemsInfoRequestedEventArgs::new(10, 5);
    assert_eq!(
        args.set_items_range_start_index(11),
        Err(LinedFlowLayoutError::RangeStartCannotIncrease)
    );
    assert_eq!(
        args.set_items_range_start_index(-1),
        Err(LinedFlowLayoutError::RangeStartMustBeNonnegative)
    );
    args.set_items_range_start_index(8).unwrap();
    assert_eq!(
        args.set_desired_aspect_ratios(&[1.0; 6]),
        Err(LinedFlowLayoutError::ArrayDoesNotCoverRequestedRange)
    );
    args.set_desired_aspect_ratios(&[1.0; 7]).unwrap();
    assert_eq!(
        args.set_min_widths(&[2.0; 8]),
        Err(LinedFlowLayoutError::ArrayLengthsMustMatch)
    );
    args.set_min_widths(&[2.0; 7]).unwrap();
    args.set_max_widths(&[20.0; 7]).unwrap();
    args.set_min_width(4.0);
    let result = args.complete();
    assert_eq!(result.start, 8);
    assert_eq!(result.minimum(10), 4.0);
    // The pinned implementation takes min(global, per-item), including -1.
    assert_eq!(result.maximum(10), -1.0);
    args.set_desired_aspect_ratios(&[]).unwrap();
    assert_eq!(result.ratios.len(), 7);
}

#[test]
fn lined_flow_weighted_history_is_bounded_and_ignores_immature_distant_items() {
    let mut ratios = LinedFlowLayoutItemAspectRatios::new();
    ratios.resize(128, 0);
    ratios.set_at(
        0,
        ItemAspectRatio {
            aspect_ratio: 2.0,
            weight: 4,
        },
    );
    ratios.set_at(
        65,
        ItemAspectRatio {
            aspect_ratio: 8.0,
            weight: 1,
        },
    );
    close(ratios.average_aspect_ratio(0, 10, 16), 2.0);
    ratios.set_at(
        65,
        ItemAspectRatio {
            aspect_ratio: 8.0,
            weight: 16,
        },
    );
    close(ratios.average_aspect_ratio(0, 10, 16), 6.8);
    assert!(ratios.has_lower_weight(0, 10, 16));
    ratios.set_at(
        5000,
        ItemAspectRatio {
            aspect_ratio: 3.0,
            weight: 16,
        },
    );
    assert_eq!(ratios.block_count(), 2);
    assert_eq!(ratios.get_at(0).weight, 0);
    assert_eq!(ratios.get_at(65).weight, 16);
    ratios.clear();
    assert!(ratios.is_empty());
}

#[test]
fn lined_flow_fast_path_virtualizes_full_source_metadata_and_reuses_distribution() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::new(
        vec![Size::new(20.0, 20.0); 100_000],
        Rect::new(0.0, 0.0, 100.0, 100.0),
    );
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.set_line_height(20.0);
    let requests = Rc::new(Cell::new(0));
    let observed_requests = requests.clone();
    let _handler = layout.items_info_requested(move |args| {
        observed_requests.set(observed_requests.get() + 1);
        args.set_items_range_start_index(0).unwrap();
        args.set_desired_aspect_ratios(&vec![1.0; 100_000]).unwrap();
    });
    layout.initialize_for_context(&base(&context));
    let size = run(&layout, &context, Size::new(100.0, 100.0));
    assert!(layout.uses_fast_path_layout());
    assert_eq!(requests.get(), 1);
    close(size.height, 400_000.0);
    assert_eq!(layout.realized_range(), (0, 24));
    assert_eq!(host.live.borrow().len(), 25);
    assert_eq!(layout.frozen_range(), (-1, -1));
    host.window.set(Rect::new(0.0, 200_000.0, 100.0, 100.0));
    run(&layout, &context, Size::new(100.0, 100.0));
    assert_eq!(requests.get(), 1);
    assert_eq!(layout.realized_range(), (50_000, 50_024));
    assert_eq!(host.live.borrow().len(), 25);
    assert!(host.element(50_000).bounds().y >= 200_000.0);
    layout.invalidate_items_info();
    run(&layout, &context, Size::new(100.0, 100.0));
    assert_eq!(requests.get(), 2);
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn lined_flow_regular_keeps_sizing_window_and_frozen_lines_on_small_scroll() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::new(
        vec![Size::new(20.0, 20.0); 10_000],
        Rect::new(0.0, 500.0, 100.0, 100.0),
    );
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.set_line_height(20.0);
    layout.initialize_for_context(&base(&context));
    // WinUI deliberately lowers its startup aspect-ratio floor over the first
    // four measures. Let that sequence and its anchor compensation settle
    // before checking the steady-state frozen-window contract.
    for _ in 0..5 {
        run(&layout, &context, Size::new(100.0, 100.0));
    }
    let range = layout.realized_range();
    let frozen = layout.frozen_range();
    assert!(range.1 - range.0 > 60);
    assert!(range.1 - range.0 < 200);
    let index = (frozen.0 + frozen.1) / 2;
    let element = host.element(index);
    let bounds = element.bounds();
    let created = host.created.get();
    run(&layout, &context, Size::new(100.0, 100.0));
    assert_eq!(host.created.get(), created);
    host.window.set(Rect::new(0.0, 510.0, 100.0, 100.0));
    run(&layout, &context, Size::new(100.0, 100.0));
    assert!(element.ptr_eq(&host.element(index)));
    assert_eq!(host.element(index).bounds(), bounds);
    assert!(host.created.get() - created < 30);
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn lined_flow_locks_interior_items_and_unlocks_after_invalidation_changes_average() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(30, Size::new(20.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.set_line_height(20.0);
    layout.initialize_for_context(&base(&context));
    assert_eq!(layout.lock_item_to_line(1).unwrap(), -1);
    assert_eq!(
        layout.lock_item_to_line(31),
        Err(LinedFlowLayoutError::ItemIndexOutOfBounds)
    );
    run(&layout, &context, Size::new(100.0, 100.0));
    let first = layout.lock_item_to_line(1).unwrap();
    assert_eq!(layout.lock_item_to_line(3).unwrap(), first);
    assert_eq!(layout.lock_item_to_line(2).unwrap(), first);
    let unlocked = Rc::new(Cell::new(0));
    let observed = unlocked.clone();
    let _handler = layout.items_unlocked(move || observed.set(observed.get() + 1));
    run(&layout, &context, Size::new(200.0, 100.0));
    assert_eq!(unlocked.get(), 1);
    let second_context = host.context();
    assert_eq!(
        layout.try_initialize_for_context(&second_context),
        Err(LinedFlowLayoutError::CannotShareLayout)
    );
    layout.uninitialize_for_context(&base(&context));
    layout.try_initialize_for_context(&second_context).unwrap();
    layout.uninitialize_for_context(&base(&second_context));
}

#[test]
fn lined_flow_partial_metadata_requests_only_new_ranges() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::new(
        vec![Size::new(20.0, 20.0); 10_000],
        Rect::new(0.0, 1000.0, 100.0, 100.0),
    );
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.set_line_height(20.0);
    let requests = Rc::new(RefCell::new(Vec::new()));
    let observed = requests.clone();
    let _handler = layout.items_info_requested(move |args| {
        observed.borrow_mut().push((
            args.items_range_start_index(),
            args.items_range_requested_length(),
        ));
        args.set_desired_aspect_ratios(&vec![1.0; args.items_range_requested_length() as usize])
            .unwrap();
    });
    layout.initialize_for_context(&base(&context));
    run(&layout, &context, Size::new(100.0, 100.0));
    assert!(!layout.uses_fast_path_layout());
    // The first pass replaces the startup aspect estimate with metadata and
    // compensates the viewport. This host keeps its external offset fixed,
    // so one more pass establishes the final range at that offset.
    run(&layout, &context, Size::new(100.0, 100.0));
    requests.borrow_mut().clear();
    run(&layout, &context, Size::new(100.0, 100.0));
    assert!(
        requests.borrow().is_empty(),
        "unexpected ranges: {:?}",
        requests.borrow()
    );
    host.window.set(Rect::new(0.0, 1010.0, 100.0, 100.0));
    run(&layout, &context, Size::new(100.0, 100.0));
    assert!(requests.borrow().iter().all(|(_, count)| *count < 30));
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn layout_test_hooks_force_orientation_and_report_anchors() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(100, Size::new(20.0, 20.0));
    let context = host.context();
    let layout = StackLayout::new();
    let object = layout.clone().upcast::<ferroui_base::FerroObject>();
    LayoutsTestHooks::set_layout_forced_index_based_layout_orientation(
        &object,
        IndexBasedLayoutOrientation::None,
    );
    assert_eq!(
        layout.index_based_layout_orientation(),
        IndexBasedLayoutOrientation::None
    );
    layout.set_orientation(ferroui_base::layout::Orientation::Horizontal);
    LayoutsTestHooks::reset_layout_forced_index_based_layout_orientation(&object);
    assert_eq!(
        layout.index_based_layout_orientation(),
        IndexBasedLayoutOrientation::LeftToRight
    );
    LayoutsTestHooks::set_layout_log_item_index(&object, 17);
    assert_eq!(LayoutsTestHooks::get_layout_log_item_index(&object), 17);
    let changes = Rc::new(Cell::new(0));
    let observed = changes.clone();
    let expected_object = object.clone();
    let subscription = LayoutsTestHooks::layout_anchor_index_changed(move |sender| {
        if sender.ptr_eq(&expected_object) {
            observed.set(observed.get() + 1);
        }
    });
    layout.initialize_for_context(&base(&context));
    run(&layout, &context, Size::new(100.0, 100.0));
    assert_eq!(changes.get(), 1);
    assert_eq!(LayoutsTestHooks::get_layout_anchor_index(&object), 0);
    close(LayoutsTestHooks::get_layout_anchor_offset(&object), 0.0);
    subscription.dispose();
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn lined_flow_test_hooks_change_real_layout_and_emit_typed_events() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(50, Size::new(20.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    let object = layout.clone().upcast::<ferroui_base::FerroObject>();
    layout.set_line_height(20.0);
    let invalidated = Rc::new(RefCell::new(Vec::new()));
    let observed = invalidated.clone();
    let expected_object = object.clone();
    let _events = LayoutsTestHooks::lined_flow_layout_invalidated(move |sender, args| {
        if sender.ptr_eq(&expected_object) {
            observed.borrow_mut().push(args.invalidation_trigger());
        }
    });
    LayoutsTestHooks::set_lined_flow_layout_forced_average_item_aspect_ratio(&object, 2.0);
    LayoutsTestHooks::set_lined_flow_layout_forced_average_items_per_line_divider(&object, 2.0);
    LayoutsTestHooks::set_lined_flow_layout_forced_wrap_multiplier(&object, 3.0);
    assert_eq!(
        LayoutsTestHooks::get_lined_flow_layout_forced_wrap_multiplier(&object),
        3.0
    );
    assert_eq!(invalidated.borrow().len(), 3);
    layout.initialize_for_context(&base(&context));
    run(&layout, &context, Size::new(100.0, 100.0));
    close(
        LayoutsTestHooks::get_lined_flow_layout_raw_average_items_per_line(&object),
        2.5,
    );
    close(
        LayoutsTestHooks::get_lined_flow_layout_snapped_average_items_per_line(&object),
        2.0,
    );
    let locked = Rc::new(RefCell::new(Vec::new()));
    let observed = locked.clone();
    let expected_object = object.clone();
    let _events = LayoutsTestHooks::lined_flow_layout_item_locked(move |sender, args| {
        if sender.ptr_eq(&expected_object) {
            observed
                .borrow_mut()
                .push((args.item_index(), args.line_index()));
        }
    });
    let line = layout.lock_item_to_line(6).unwrap();
    assert_eq!(locked.borrow().as_slice(), &[(6, line)]);
    assert_eq!(
        LayoutsTestHooks::get_lined_flow_layout_line_index(&object, 6),
        line
    );
    assert_eq!(
        LayoutsTestHooks::get_layout_first_realized_item_index(&object),
        0
    );
    LayoutsTestHooks::unlock_lined_flow_layout_items(&object);
    LayoutsTestHooks::clear_lined_flow_layout_item_aspect_ratios(&object);
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn lined_flow_test_hooks_disable_fast_path_for_complete_metadata() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(20, Size::new(20.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    let object = layout.clone().upcast::<ferroui_base::FerroObject>();
    layout.set_line_height(20.0);
    LayoutsTestHooks::set_lined_flow_layout_is_fast_path_supported(&object, false);
    let _handler = layout.items_info_requested(|args| {
        args.set_items_range_start_index(0).unwrap();
        args.set_desired_aspect_ratios(&[1.0; 20]).unwrap();
    });
    layout.initialize_for_context(&base(&context));
    run(&layout, &context, Size::new(100.0, 100.0));
    assert!(!layout.uses_fast_path_layout());
    LayoutsTestHooks::set_lined_flow_layout_is_fast_path_supported(&object, true);
    layout.invalidate_items_info();
    run(&layout, &context, Size::new(100.0, 100.0));
    assert!(layout.uses_fast_path_layout());
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn lined_flow_edge_locks_notify_each_new_implicit_interior_lock() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(20, Size::new(20.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.set_line_height(20.0);
    let object = layout.clone().upcast::<ferroui_base::FerroObject>();
    let _metadata = layout.items_info_requested(|args| {
        args.set_items_range_start_index(0).unwrap();
        args.set_desired_aspect_ratios(&[1.0; 20]).unwrap();
    });
    layout.initialize_for_context(&base(&context));
    run(&layout, &context, Size::new(100.0, 100.0));
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = events.clone();
    let sender_object = object.clone();
    let _events = LayoutsTestHooks::lined_flow_layout_item_locked(move |sender, args| {
        if sender.ptr_eq(&sender_object) {
            observed
                .borrow_mut()
                .push((args.item_index(), args.line_index()));
            // Querying from inside an event must not overlap a mutable borrow.
            assert_eq!(
                LayoutsTestHooks::get_lined_flow_layout_line_index(sender, args.item_index()),
                args.line_index()
            );
        }
    });
    assert_eq!(layout.lock_item_to_line(3).unwrap(), 0);
    assert_eq!(layout.lock_item_to_line(0).unwrap(), 0);
    assert_eq!(
        events.borrow().as_slice(),
        &[(3, 0), (2, 0), (1, 0), (0, 0)]
    );
    layout.lock_item_to_line(1).unwrap();
    assert_eq!(events.borrow().len(), 4);
    events.borrow_mut().clear();
    assert_eq!(layout.lock_item_to_line(16).unwrap(), 3);
    assert_eq!(layout.lock_item_to_line(19).unwrap(), 3);
    assert_eq!(
        events.borrow().as_slice(),
        &[(16, 3), (17, 3), (18, 3), (19, 3)]
    );
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn lined_flow_startup_average_converges_before_freezing_steady_state() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(100, Size::new(20.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.set_line_height(20.0);
    layout.initialize_for_context(&base(&context));
    let mut averages = Vec::new();
    for _ in 0..5 {
        run(&layout, &context, Size::new(100.0, 100.0));
        averages.push(layout.average_items_per_line());
    }
    close(averages[0].0, 100.0 / 30.0);
    close(averages[1].0, 4.0);
    close(averages[2].0, 5.0);
    assert_eq!(averages[2], averages[3]);
    assert_eq!(averages[3], averages[4]);
    layout.uninitialize_for_context(&base(&context));
}
