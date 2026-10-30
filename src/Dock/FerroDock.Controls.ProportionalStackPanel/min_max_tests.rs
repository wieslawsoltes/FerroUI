// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Source ProportionalStackPanelMinMaxTests layout scenarios. Private splitter
//! method scenarios live in splitter_headless_tests, below the native class module.
use crate::tests::{layout, run, scene};
use crate::*;
use ferroui_base::layout::Orientation;
use ferroui_controls::Border;

#[test]
fn minimum_and_maximum_dimensions_are_respected_during_layout() {
    run(|| {
        // The first seven source layout/dimension cases (including both vertical
        // dimension cases) keep their original panel sizes and proportions.
        for (orientation, length, proportion, minimum, maximum) in [
            (Orientation::Horizontal, 500.0, 0.2, 200.0, 300.0),
            (Orientation::Horizontal, 500.0, 0.8, 100.0, 150.0),
            (Orientation::Vertical, 500.0, 0.2, 200.0, 300.0),
            (Orientation::Horizontal, 400.0, 0.1, 200.0, f64::INFINITY),
            (Orientation::Horizontal, 600.0, 0.8, 0.0, 150.0),
            (Orientation::Vertical, 400.0, 0.1, 200.0, f64::INFINITY),
            (Orientation::Vertical, 600.0, 0.8, 0.0, 150.0),
        ] {
            let (panel, a, _, _) = scene(orientation, length, proportion, 1.0 - proportion);
            if orientation == Orientation::Horizontal {
                a.set_min_width(minimum);
                a.set_max_width(maximum);
            } else {
                a.set_min_height(minimum);
                a.set_max_height(maximum);
            }
            layout(&panel);
            let actual = if orientation == Orientation::Horizontal {
                a.bounds().width
            } else {
                a.bounds().height
            };
            assert!(
                actual >= minimum && actual <= maximum,
                "{orientation:?}: {actual} outside [{minimum}, {maximum}]"
            );
        }
    });
}

fn assert_widths(a: &Border, b: &Border) {
    assert!((150.0..=400.0).contains(&a.bounds().width));
    assert!(b.bounds().width >= 100.0);
}

#[test]
fn maintains_constraint_stability_when_proportions_change() {
    run(|| {
        let (panel, a, _, b) = scene(Orientation::Horizontal, 800.0, 0.4, 0.6);
        a.set_min_width(150.0);
        a.set_max_width(400.0);
        b.set_min_width(100.0);
        for (first, second) in [(0.4, 0.6), (0.1, 0.9), (0.9, 0.1)] {
            ProportionalStackPanel::set_proportion(&a, first);
            ProportionalStackPanel::set_proportion(&b, second);
            layout(&panel);
            assert_widths(&a, &b);
        }
    });
}

#[test]
fn respects_minimum_and_maximum_width_with_different_panel_sizes() {
    run(|| {
        let (panel, a, _, _) = scene(Orientation::Horizontal, 500.0, 0.3, 0.7);
        a.set_min_width(200.0);
        a.set_max_width(300.0);
        for width in [500.0, 1000.0, 2000.0] {
            panel.set_width(width);
            layout(&panel);
            assert!((200.0..=300.0).contains(&a.bounds().width));
        }
    });
}

#[test]
fn simulates_splitter_dragging_behavior_with_constraints() {
    run(|| {
        let (panel, a, _, b) = scene(Orientation::Horizontal, 800.0, 0.4, 0.6);
        a.set_min_width(150.0);
        a.set_max_width(400.0);
        b.set_min_width(100.0);
        for (first, second) in [(0.4, 0.6), (0.1, 0.9), (0.8, 0.2)] {
            ProportionalStackPanel::set_proportion(&a, first);
            ProportionalStackPanel::set_proportion(&b, second);
            layout(&panel);
            assert_widths(&a, &b);
        }
    });
}

#[test]
fn multiple_children_with_mixed_constraints() {
    run(|| {
        let (panel, a, _, b) = scene(Orientation::Horizontal, 1000.0, 0.3, 0.4);
        a.set_min_width(100.0);
        a.set_max_width(200.0);
        b.set_min_width(150.0);
        let c = Border::new();
        c.set_max_width(250.0);
        ProportionalStackPanel::set_proportion(&c, 0.3);
        panel.children().add(ProportionalStackPanelSplitter::new());
        panel.children().add(c.clone());
        layout(&panel);
        assert!((100.0..=200.0).contains(&a.bounds().width));
        assert!(b.bounds().width >= 150.0);
        assert!(c.bounds().width <= 250.0);
    });
}

#[test]
fn constraints_work_with_nan_proportions() {
    run(|| {
        let (panel, a, _, b) = scene(Orientation::Horizontal, 600.0, f64::NAN, f64::NAN);
        a.set_min_width(150.0);
        a.set_max_width(250.0);
        b.set_min_width(100.0);
        layout(&panel);
        assert!((150.0..=250.0).contains(&a.bounds().width));
        assert!(b.bounds().width >= 100.0);
    });
}

#[test]
fn constraints_work_with_zero_proportions() {
    run(|| {
        let (panel, a, _, _) = scene(Orientation::Horizontal, 600.0, 0.0, 1.0);
        a.set_min_width(150.0);
        layout(&panel);
        assert!(a.bounds().width >= 150.0);
    });
}

#[test]
fn extreme_constraints_do_not_break_layout() {
    run(|| {
        let (panel, a, _, b) = scene(Orientation::Horizontal, 200.0, 0.5, 0.5);
        a.set_min_width(150.0);
        b.set_min_width(100.0);
        layout(&panel);
        assert!(a.bounds().width >= 150.0);
        assert!(b.bounds().width >= 100.0);
    });
}
