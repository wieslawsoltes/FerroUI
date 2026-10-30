// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.
// Source contracts: LinedFlowLayout::{RequestItemsInfo, ArrangeOverride,
// InvalidateMeasureTimerTick, UninitializeForContextCore} and FerroUI Layoutable.

use super::super::layout_tests::{base, run, TestHost};
use super::*;
use ferroui_base::Thickness;

fn settle(layout: &LinedFlowLayout, context: &Ref<VirtualizingLayoutContext>, available: Size) {
    for _ in 0..5 {
        run(layout, context, available);
    }
}

#[test]
fn maximum_width_keeps_the_control_margin_in_its_allocated_slot() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(3, Size::new(60.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.set_line_height(20.0);
    layout.initialize_for_context(&base(&context));
    settle(&layout, &context, Size::new(500.0, 100.0));
    let first = host.element(0);
    first.set_max_width(40.0);
    first.set_margin(Thickness::new(7.0, 0.0, 13.0, 0.0));
    settle(&layout, &context, Size::new(500.0, 100.0));
    assert_eq!(first.desired_size().width, 60.0);
    assert_eq!(first.bounds(), Rect::new(7.0, 0.0, 40.0, 20.0));
    assert_eq!(host.element(1).bounds().x, 60.0);
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn conflicting_control_constraints_follow_ferroui_minimum_precedence() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(3, Size::new(60.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.set_line_height(20.0);
    layout.initialize_for_context(&base(&context));
    settle(&layout, &context, Size::new(500.0, 100.0));
    let first = host.element(0);
    first.set_min_width(100.0);
    first.set_max_width(20.0);
    first.set_margin(Thickness::new(7.0, 0.0, 13.0, 0.0));
    settle(&layout, &context, Size::new(500.0, 100.0));
    assert_eq!(first.desired_size().width, 120.0);
    assert_eq!(first.bounds(), Rect::new(7.0, 0.0, 100.0, 20.0));
    assert_eq!(host.element(1).bounds().x, 120.0);
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn partial_last_line_arranges_its_retained_controls_after_skipping_the_leading_partial_line() {
    let _scope = crate::test_support::test_scope();
    for metadata in [false, true] {
        let host = TestHost::uniform(10, Size::new(20.0, 20.0));
        let context = host.context();
        let layout = LinedFlowLayout::new();
        layout.set_line_height(20.0);
        let _metadata = metadata.then(|| {
            layout.items_info_requested(|args| {
                args.set_items_range_start_index(0).unwrap();
                args.set_desired_aspect_ratios(&[1.0; 10]).unwrap();
            })
        });
        layout.initialize_for_context(&base(&context));
        settle(&layout, &context, Size::new(100.0, 100.0));
        let expected = (5..8)
            .map(|index| (host.element(index), host.element(index).bounds()))
            .collect::<Vec<_>>();
        layout
            .element_manager
            .discard_elements_outside_window(false, 1);
        layout
            .element_manager
            .discard_elements_outside_window(true, 8);
        let created = host.created.get();
        for (element, _) in &expected {
            element.arrange(Rect::new(999.0, 999.0, 20.0, 20.0));
        }
        layout.arrange(&base(&context), Size::new(100.0, 100.0));
        for (element, bounds) in expected {
            assert_eq!(element.bounds(), bounds);
        }
        assert_eq!(host.element(2).bounds().y, 0.0);
        assert_eq!(layout.realized_range(), (2, 7));
        assert_eq!(host.created.get(), created);
        layout.uninitialize_for_context(&base(&context));
    }
}

#[test]
fn declining_one_metadata_interval_falls_back_for_the_complete_window() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(1000, Size::new(20.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.initialize_for_context(&base(&context));
    *layout.items_info.borrow_mut() = Rc::new(LinedFlowItemsInfo {
        start: 100,
        ratios: vec![2.0; 20],
        ..LinedFlowItemsInfo::default()
    });
    let requests = Rc::new(RefCell::new(Vec::new()));
    let observed = requests.clone();
    let _metadata = layout.items_info_requested(move |args| {
        observed.borrow_mut().push(args.items_range_start_index());
        if args.items_range_start_index() < 100 {
            args.set_desired_aspect_ratios(&vec![
                2.0;
                args.items_range_requested_length() as usize
            ])
            .unwrap();
        }
    });
    assert!(!layout.request_items_info(90, 130));
    assert_eq!(*requests.borrow(), vec![90, 120]);
    assert_eq!(layout.requested_range_start_index(), -1);
    assert_eq!(layout.requested_range_length(), 0);
    assert!(!layout.uses_fast_path_layout());
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn expanded_metadata_range_is_retained_and_reused_without_another_request() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(1000, Size::new(20.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.initialize_for_context(&base(&context));
    let requests = Rc::new(Cell::new(0));
    let observed = requests.clone();
    let _metadata = layout.items_info_requested(move |args| {
        observed.set(observed.get() + 1);
        args.set_items_range_start_index(80).unwrap();
        args.set_desired_aspect_ratios(&[1.0; 100]).unwrap();
    });
    assert!(!layout.request_items_info(100, 120));
    assert_eq!(layout.requested_range_start_index(), 80);
    assert_eq!(layout.requested_range_length(), 100);
    assert!(!layout.request_items_info(90, 160));
    assert_eq!(requests.get(), 1);
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn expanded_metadata_is_clipped_to_the_items_source() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(100, Size::new(20.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.initialize_for_context(&base(&context));
    let _metadata = layout.items_info_requested(|args| {
        args.set_desired_aspect_ratios(&[1.0; 100]).unwrap();
    });
    assert!(!layout.request_items_info(80, 100));
    assert_eq!(layout.requested_range_start_index(), 80);
    assert_eq!(layout.requested_range_length(), 20);
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn regular_metadata_global_maximum_limits_realized_controls_without_per_item_limits() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::new(
        vec![Size::new(60.0, 20.0); 1000],
        Rect::new(0.0, 0.0, 600.0, 100.0),
    );
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.set_line_height(20.0);
    let _metadata = layout.items_info_requested(|args| {
        args.set_max_width(30.0);
        args.set_desired_aspect_ratios(&vec![3.0; args.items_range_requested_length() as usize])
            .unwrap();
    });
    layout.initialize_for_context(&base(&context));
    settle(&layout, &context, Size::new(600.0, 100.0));
    assert!(!layout.uses_fast_path_layout());
    assert_eq!(host.element(0).bounds().width, 30.0);
    assert_eq!(host.element(1).bounds().x, 30.0);
    for (_, element) in host.live.borrow().iter() {
        assert!(element.bounds().width <= 30.0);
    }
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn current_global_limits_apply_to_retained_raw_metadata_without_fabricated_width_arrays() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(1000, Size::new(60.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.initialize_for_context(&base(&context));
    let limits = Rc::new(Cell::new((50.0, 70.0)));
    let observed_limits = limits.clone();
    let requests = Rc::new(RefCell::new(Vec::new()));
    let observed_requests = requests.clone();
    let _metadata = layout.items_info_requested(move |args| {
        observed_requests.borrow_mut().push((
            args.items_range_start_index(),
            args.items_range_requested_length(),
        ));
        let (minimum, maximum) = observed_limits.get();
        args.set_min_width(minimum);
        args.set_max_width(maximum);
        args.set_desired_aspect_ratios(&vec![3.0; args.items_range_requested_length() as usize])
            .unwrap();
    });
    assert!(!layout.request_items_info(100, 120));
    limits.set((10.0, 30.0));
    assert!(!layout.request_items_info(110, 130));
    assert_eq!(*requests.borrow(), vec![(100, 20), (120, 10)]);
    let metrics = layout.collect_metrics(&context, 110, 130, 20.0, true);
    assert!(metrics
        .iter()
        .all(|item| { item.width == 30.0 && item.minimum == 10.0 && item.maximum == 30.0 }));
    assert_eq!(host.created.get(), 0);
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn prefetched_positive_ratios_update_history_before_negative_fallback_widths() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(1000, Size::new(20.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.set_line_height(20.0);
    layout.initialize_for_context(&base(&context));
    layout.average_items_per_line.set((5.0, 5.0));
    layout.current_available_width.set(100.0);
    let mut ratios = vec![2.0; 100];
    ratios[20] = -1.0;
    *layout.items_info.borrow_mut() = Rc::new(LinedFlowItemsInfo {
        start: 80,
        ratios,
        ..LinedFlowItemsInfo::default()
    });
    let metrics = layout.collect_metrics(&context, 100, 120, 20.0, true);
    assert_eq!(metrics[0].width, 40.0);
    let history = layout.aspect_ratios.borrow();
    assert_eq!(history.get_at(80).weight, 16);
    assert_eq!(history.get_at(179).weight, 16);
    assert_eq!(history.get_at(100).weight, 0);
    drop(history);
    assert_eq!(host.created.get(), 0);
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn regular_metadata_stops_delayed_control_measurement() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::new(
        vec![Size::new(20.0, 20.0); 10_000],
        Rect::new(0.0, 0.0, 100.0, 100.0),
    );
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.set_line_height(20.0);
    layout.initialize_for_context(&base(&context));
    settle(&layout, &context, Size::new(100.0, 100.0));
    assert!(layout.timer.borrow().is_some());
    let _metadata = layout.items_info_requested(|args| {
        args.set_desired_aspect_ratios(&vec![1.0; args.items_range_requested_length() as usize])
            .unwrap();
    });
    layout.invalidate_items_info();
    settle(&layout, &context, Size::new(100.0, 100.0));
    assert!(!layout.uses_fast_path_layout());
    assert!(layout.requested_range_length() > 0);
    assert!(layout.timer.borrow().is_none());
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn fast_path_preserves_realization_while_viewport_is_zero_and_resumes_from_the_host_window() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::new(
        vec![Size::new(20.0, 20.0); 1000],
        Rect::new(0.0, 0.0, 100.0, 0.0),
    );
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.set_line_height(20.0);
    let requests = Rc::new(Cell::new(0));
    let observed = requests.clone();
    let _metadata = layout.items_info_requested(move |args| {
        observed.set(observed.get() + 1);
        args.set_items_range_start_index(0).unwrap();
        args.set_desired_aspect_ratios(&[1.0; 1000]).unwrap();
    });
    layout.initialize_for_context(&base(&context));
    host.anchor.set(900);
    run(&layout, &context, Size::new(100.0, 100.0));
    assert!(layout.uses_fast_path_layout());
    assert_eq!(layout.realized_range(), (-1, -1));
    assert_eq!(host.created.get(), 0);
    assert_eq!(layout.anchor_index.get(), -1);
    assert_eq!(layout.anchor_retention.get(), 0);
    host.anchor.set(-1);
    host.window.set(Rect::new(0.0, 2000.0, 100.0, 100.0));
    run(&layout, &context, Size::new(100.0, 100.0));
    assert_eq!(layout.realized_range(), (500, 524));
    let retained = host.element(500);
    let created = host.created.get();
    host.window.set(Rect::new(0.0, 2000.0, 100.0, 0.0));
    run(&layout, &context, Size::new(100.0, 100.0));
    assert_eq!(layout.realized_range(), (500, 524));
    assert!(host.element(500).ptr_eq(&retained));
    assert_eq!(host.created.get(), created);
    assert_eq!(requests.get(), 1);
    host.window.set(Rect::new(0.0, 0.0, 100.0, 100.0));
    run(&layout, &context, Size::new(100.0, 100.0));
    assert_eq!(layout.realized_range(), (0, 24));
    assert_eq!(requests.get(), 1);
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn fast_path_remeasures_partial_retained_lines_when_resized_at_zero_height() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::new(
        vec![Size::new(20.0, 20.0); 1000],
        Rect::new(0.0, 2000.0, 100.0, 100.0),
    );
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.set_line_height(20.0);
    let _metadata = layout.items_info_requested(|args| {
        args.set_items_range_start_index(0).unwrap();
        args.set_desired_aspect_ratios(&[1.0; 1000]).unwrap();
    });
    layout.initialize_for_context(&base(&context));
    run(&layout, &context, Size::new(100.0, 100.0));
    assert_eq!(layout.realized_range(), (500, 524));
    let created = host.created.get();
    host.window.set(Rect::new(0.0, 2000.0, 100.0, 0.0));
    // Three-item lines leave index 500 in a leading partial line; four-item
    // lines leave index 524 in a trailing partial line.
    for (available_width, natural_width, items_per_line) in [(60.0, 10.0, 3), (80.0, 15.0, 4)] {
        for index in 500..525 {
            host.set_size(index, Size::new(natural_width, 20.0));
        }
        layout.measure(&base(&context), Size::new(available_width, 100.0));
        assert_eq!(layout.realized_range(), (500, 524));
        assert_eq!(host.created.get(), created);
        for index in 500..525 {
            let element = host.element(index);
            assert!(
                element.is_measure_valid(),
                "item {index} was not remeasured"
            );
            assert_eq!(element.desired_size().width, natural_width);
            assert_eq!(
                layout.element_manager.layout_bounds_for_data_index(index),
                Rect::new(
                    (index % items_per_line) as f64 * 20.0,
                    (index / items_per_line) as f64 * 20.0,
                    20.0,
                    20.0,
                )
            );
        }
    }
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn fast_path_centers_only_the_host_realization_window_on_a_disconnected_anchor() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::new(
        vec![Size::new(20.0, 20.0); 1000],
        Rect::new(0.0, 0.0, 100.0, 100.0),
    );
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.set_line_height(20.0);
    let _metadata = layout.items_info_requested(|args| {
        args.set_items_range_start_index(0).unwrap();
        args.set_desired_aspect_ratios(&[1.0; 1000]).unwrap();
    });
    layout.initialize_for_context(&base(&context));
    host.anchor.set(900);
    run(&layout, &context, Size::new(100.0, 100.0));
    assert_eq!(layout.realized_range(), (890, 914));
    assert_eq!(host.element(900).bounds().y, 3600.0);
    assert_eq!(host.live.borrow().len(), 25);
    assert_eq!(layout.anchor_retention.get(), 10);
    host.window.set(Rect::new(0.0, 0.0, 100.0, 0.0));
    host.anchor.set(700);
    run(&layout, &context, Size::new(100.0, 100.0));
    assert_eq!(layout.anchor_index.get(), 900);
    host.anchor.set(-1);
    for _ in 0..3 {
        run(&layout, &context, Size::new(100.0, 100.0));
        assert_eq!(layout.anchor_retention.get(), 10);
        assert_eq!(layout.realized_range(), (890, 914));
    }
    host.window.set(Rect::new(0.0, 0.0, 100.0, 100.0));
    for remaining in (0..10).rev() {
        run(&layout, &context, Size::new(100.0, 100.0));
        assert_eq!(layout.anchor_retention.get(), remaining);
        assert_eq!(layout.realized_range(), (890, 914));
    }
    assert_eq!(layout.anchor_index.get(), -1);
    run(&layout, &context, Size::new(100.0, 100.0));
    assert_eq!(layout.realized_range(), (0, 24));
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn automatic_line_height_probe_restores_the_metadata_allocation() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(3, Size::new(100.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    let _metadata = layout.items_info_requested(|args| {
        args.set_items_range_start_index(0).unwrap();
        args.set_desired_aspect_ratios(&[5.0; 3]).unwrap();
    });
    layout.initialize_for_context(&base(&context));
    settle(&layout, &context, Size::new(40.0, 100.0));
    assert!(layout.uses_fast_path_layout());
    let element = layout.element_manager.get_realized_element(0);
    let prior_size = element.desired_size();
    let created = host.created.get();
    assert_eq!(prior_size, Size::new(40.0, 20.0));
    assert!(!layout.update_actual_line_height(&context, Size::new(80.0, 100.0)));
    assert_eq!(element.desired_size(), prior_size);
    assert_eq!(host.created.get(), created);
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn automatic_line_height_uses_a_separate_probe_when_no_restorable_width_is_recorded() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(3, Size::new(100.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.initialize_for_context(&base(&context));
    layout.actual_line_height.set(20.0);
    layout.element_manager.ensure_element_realized(true, 0);
    let element = layout.element_manager.get_realized_element(0);
    element.measure(Size::new(40.0, 20.0));
    let created = host.created.get();
    assert!(!layout.update_actual_line_height(&context, Size::new(80.0, 100.0)));
    assert_eq!(element.desired_size(), Size::new(40.0, 20.0));
    assert_eq!(host.created.get(), created + 1);
    assert!(layout
        .element_manager
        .get_realized_element(0)
        .ptr_eq(&element));
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn unconstrained_new_minimum_width_items_start_delayed_measurement() {
    let _scope = crate::test_support::test_scope();
    for natural_width in [0.0, 20.0] {
        let host = TestHost::uniform(3, Size::new(natural_width, 20.0));
        let context = host.context();
        let layout = LinedFlowLayout::new();
        layout.set_line_height(20.0);
        layout.initialize_for_context(&base(&context));
        let desired = run(&layout, &context, Size::new(f64::INFINITY, 100.0));
        assert_eq!(desired, Size::new(3.0 * natural_width, 20.0));
        assert_eq!(layout.timer.borrow().is_some(), natural_width == 0.0);
        if natural_width == 0.0 {
            host.set_size(0, Size::new(40.0, 20.0));
            let desired = run(&layout, &context, Size::new(f64::INFINITY, 100.0));
            assert_eq!(desired.width, 40.0);
            assert_eq!(host.element(1).bounds().x, 40.0);
        }
        layout.uninitialize_for_context(&base(&context));
        assert!(layout.timer.borrow().is_none());
    }
}

#[test]
fn pending_invalidation_cannot_escape_into_a_replacement_context() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(3, Size::new(20.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.initialize_for_context(&base(&context));
    layout.invalidate_measure_async();
    layout.uninitialize_for_context(&base(&context));
    let replacement = host.context();
    layout.initialize_for_context(&base(&replacement));
    let invalidations = Rc::new(Cell::new(0));
    let observed = invalidations.clone();
    let _invalidated = layout.measure_invalidated(move || observed.set(observed.get() + 1));
    layout.invalidate_measure_async();
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(invalidations.get(), 1);
    assert!(!layout.async_invalidation_pending.get());
    layout.uninitialize_for_context(&base(&replacement));
}

#[test]
fn releasing_layout_with_a_queued_measure_timer_stops_it_and_allows_a_fresh_host() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(30, Size::new(20.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.set_line_height(20.0);
    layout.initialize_for_context(&base(&context));
    run(&layout, &context, Size::new(100.0, 100.0));
    let timer = layout.timer.borrow().as_ref().unwrap().0.clone();
    // Queue an already-due tick without a wall-clock sleep. Release the
    // layout before the dispatcher processes that callback.
    timer.stop();
    timer.set_interval(Duration::ZERO);
    timer.start();
    let weak = layout.downgrade();
    drop(layout);
    assert!(weak.upgrade().is_none());
    assert!(!timer.is_enabled());
    Dispatcher::ui_thread().run_jobs(None);
    let fresh_host = TestHost::uniform(3, Size::new(20.0, 20.0));
    let fresh_context = fresh_host.context();
    let fresh_layout = LinedFlowLayout::new();
    fresh_layout.set_line_height(20.0);
    fresh_layout.initialize_for_context(&base(&fresh_context));
    settle(&fresh_layout, &fresh_context, Size::new(100.0, 100.0));
    assert_eq!(fresh_layout.realized_range(), (0, 2));
    assert_eq!(
        fresh_host.element(0).bounds(),
        Rect::new(0.0, 0.0, 20.0, 20.0)
    );
    fresh_layout.uninitialize_for_context(&base(&fresh_context));
}

#[test]
fn detached_anchor_and_frozen_ranges_do_not_follow_the_layout_to_another_host() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(1000, Size::new(20.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.set_line_height(20.0);
    layout.initialize_for_context(&base(&context));
    host.anchor.set(900);
    run(&layout, &context, Size::new(100.0, 100.0));
    assert_eq!(layout.anchor_index.get(), 900);
    layout.uninitialize_for_context(&base(&context));
    assert_eq!(layout.frozen_range(), (-1, -1));
    let replacement_host = TestHost::uniform(3, Size::new(20.0, 20.0));
    let replacement = replacement_host.context();
    layout.initialize_for_context(&base(&replacement));
    settle(&layout, &replacement, Size::new(100.0, 100.0));
    assert_eq!(layout.anchor_index.get(), -1);
    assert_eq!(layout.realized_range(), (0, 2));
    assert_eq!(replacement_host.element(0).bounds().y, 0.0);
    layout.uninitialize_for_context(&base(&replacement));
}

#[test]
fn cache_shrink_uses_absolute_source_indices_at_a_distant_viewport() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::new(
        vec![Size::new(20.0, 20.0); 5000],
        Rect::new(0.0, 4000.0, 100.0, 1.0),
    );
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.set_line_height(20.0);
    layout.initialize_for_context(&base(&context));
    layout.actual_line_height.set(20.0);
    layout.current_available_width.set(100.0);
    layout.average_items_per_line.set((5.0, 5.0));
    {
        let mut ratios = layout.aspect_ratios.borrow_mut();
        ratios.resize(384, 1060);
        for index in [0, 64, 128, 1024, 1088, 1152] {
            ratios.set_at(
                index,
                ItemAspectRatio {
                    aspect_ratio: 1.0,
                    weight: 16,
                },
            );
        }
    }
    *layout.items_info.borrow_mut() = Rc::new(LinedFlowItemsInfo {
        start: 1060,
        ratios: vec![1.0; 2],
        ..LinedFlowItemsInfo::default()
    });
    layout.collect_metrics(&context, 1060, 1062, 20.0, true);
    let ratios = layout.aspect_ratios.borrow();
    assert_eq!(ratios.block_count(), 4);
    assert_eq!(ratios.get_at(1088).weight, 16);
    assert_eq!(ratios.get_at(1152).weight, 16);
    assert_eq!(ratios.get_at(0).weight, 0);
    drop(ratios);
    layout.uninitialize_for_context(&base(&context));
}
