// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original SplitterHeadless scenarios, plus the three private-method MinMax
//! scenarios. Module privacy replaces the source tests' reflection calls.
use super::*;
use crate::tests::{layout, run, scene};
use ferroui_controls::Border;

fn dimension(control: &Control, orientation: Orientation) -> f64 {
    if orientation == Orientation::Horizontal {
        control.bounds().width
    } else {
        control.bounds().height
    }
}

fn constrain(control: &Control, orientation: Orientation, minimum: f64, maximum: f64) {
    if orientation == Orientation::Horizontal {
        control.set_min_width(minimum);
        control.set_max_width(maximum);
    } else {
        control.set_min_height(minimum);
        control.set_max_height(maximum);
    }
}

#[test]
fn splitter_respects_target_and_neighbor_minimum_and_maximum_in_both_orientations() {
    run(|| {
        // Eight original source cases; each iteration has a fresh native tree.
        for orientation in [Orientation::Horizontal, Orientation::Vertical] {
            for (length, proportion, target_limits, neighbor_limits, delta, change) in [
                (
                    600.0,
                    0.5,
                    (200.0, f64::INFINITY),
                    (0.0, f64::INFINITY),
                    -150.0,
                    -1,
                ),
                (800.0, 0.4, (0.0, 350.0), (0.0, f64::INFINITY), 100.0, 1),
                (
                    600.0,
                    0.5,
                    (0.0, f64::INFINITY),
                    (150.0, f64::INFINITY),
                    200.0,
                    -2,
                ),
                (800.0, 0.6, (0.0, f64::INFINITY), (0.0, 250.0), -100.0, 0),
            ] {
                let (panel, a, splitter, b) =
                    scene(orientation, length, proportion, 1.0 - proportion);
                constrain(&a, orientation, target_limits.0, target_limits.1);
                constrain(&b, orientation, neighbor_limits.0, neighbor_limits.1);
                layout(&panel);
                let before_target = dimension(&a, orientation);
                let before_neighbor = dimension(&b, orientation);
                splitter.set_target_proportion(delta);
                layout(&panel);
                let target = dimension(&a, orientation);
                let neighbor = dimension(&b, orientation);
                assert!(target >= target_limits.0 && target <= target_limits.1);
                assert!(neighbor >= neighbor_limits.0 && neighbor <= neighbor_limits.1);
                match change {
                    -1 if orientation == Orientation::Horizontal => assert!(target < before_target),
                    1 => assert!(target > before_target),
                    -2 => assert!(neighbor < before_neighbor),
                    _ => (),
                }
            }
        }
    });
}

#[test]
fn splitter_handles_both_target_and_neighbor_minimum_width_constraints() {
    run(|| {
        let (panel, a, splitter, b) = scene(Orientation::Horizontal, 500.0, 0.5, 0.5);
        a.set_min_width(150.0);
        b.set_min_width(120.0);
        layout(&panel);
        splitter.set_target_proportion(-200.0);
        layout(&panel);
        assert!(a.bounds().width >= 150.0);
        assert!(b.bounds().width >= 120.0);
    });
}

#[test]
fn splitter_handles_both_target_and_neighbor_maximum_width_constraints() {
    run(|| {
        let (panel, a, splitter, b) = scene(Orientation::Horizontal, 600.0, 0.5, 0.5);
        a.set_max_width(250.0);
        b.set_max_width(200.0);
        layout(&panel);
        splitter.set_target_proportion(150.0);
        layout(&panel);
        assert!(a.bounds().width <= 250.0);
        assert!(b.bounds().width <= 200.0);
    });
}

#[test]
fn splitter_handles_mixed_minimum_and_maximum_constraints() {
    run(|| {
        let (panel, a, splitter, b) = scene(Orientation::Horizontal, 800.0, 0.4, 0.6);
        a.set_min_width(200.0);
        a.set_max_width(350.0);
        b.set_min_width(150.0);
        b.set_max_width(300.0);
        layout(&panel);
        for delta in [-150.0, 200.0] {
            splitter.set_target_proportion(delta);
            layout(&panel);
            assert!((200.0..=350.0).contains(&a.bounds().width));
            assert!((150.0..=300.0).contains(&b.bounds().width));
        }
    });
}

#[test]
fn splitter_handles_conflicting_constraints_gracefully_for_both_directions() {
    run(|| {
        let (panel, a, splitter, b) = scene(Orientation::Horizontal, 300.0, 0.5, 0.5);
        a.set_min_width(200.0);
        b.set_min_width(150.0);
        layout(&panel);
        for delta in [50.0, -50.0] {
            splitter.set_target_proportion(delta);
            layout(&panel);
        }
        assert!(a.bounds().width >= 200.0);
        assert!(b.bounds().width >= 150.0);
    });
}

#[test]
fn splitter_works_with_multiple_splitters_and_constraints() {
    run(|| {
        let (panel, a, first_splitter, b) = scene(Orientation::Horizontal, 1000.0, 0.25, 0.35);
        a.set_min_width(100.0);
        a.set_max_width(300.0);
        b.set_min_width(150.0);
        b.set_max_width(400.0);
        let second_splitter = ProportionalStackPanelSplitter::new();
        let c = Border::new();
        c.set_min_width(120.0);
        ProportionalStackPanel::set_proportion(&c, 0.4);
        panel.children().add(second_splitter.clone());
        panel.children().add(c.clone());
        layout(&panel);
        first_splitter.set_target_proportion(-100.0);
        layout(&panel);
        assert!((100.0..=300.0).contains(&a.bounds().width));
        assert!((150.0..=400.0).contains(&b.bounds().width));
        second_splitter.set_target_proportion(80.0);
        layout(&panel);
        assert!((150.0..=400.0).contains(&b.bounds().width));
        assert!(c.bounds().width >= 120.0);
    });
}

#[test]
fn splitter_handles_zero_delta_without_changing_widths() {
    run(|| {
        let (panel, a, splitter, b) = scene(Orientation::Horizontal, 600.0, 0.5, 0.5);
        a.set_min_width(100.0);
        a.set_max_width(400.0);
        b.set_min_width(80.0);
        layout(&panel);
        let initial = (a.bounds().width, b.bounds().width);
        splitter.set_target_proportion(0.0);
        layout(&panel);
        assert_eq!(initial, (a.bounds().width, b.bounds().width));
        assert!((100.0..=400.0).contains(&a.bounds().width));
        assert!(b.bounds().width >= 80.0);
    });
}

#[test]
fn min_max_source_splitter_dragging_cases() {
    run(|| {
        // The three private-method calls in ProportionalStackPanelMinMaxTests.
        for (proportion, target_limits, neighbor_minimum, delta) in [
            (0.5, (100.0, f64::INFINITY), 0.0, -150.0),
            (0.3, (0.0, 200.0), 0.0, 200.0),
            (0.5, (0.0, f64::INFINITY), 100.0, 150.0),
        ] {
            let (panel, a, splitter, b) =
                scene(Orientation::Horizontal, 500.0, proportion, 1.0 - proportion);
            a.set_min_width(target_limits.0);
            a.set_max_width(target_limits.1);
            b.set_min_width(neighbor_minimum);
            layout(&panel);
            splitter.set_target_proportion(delta);
            layout(&panel);
            assert!(a.bounds().width >= target_limits.0 && a.bounds().width <= target_limits.1);
            assert!(b.bounds().width >= neighbor_minimum);
        }
    });
}
