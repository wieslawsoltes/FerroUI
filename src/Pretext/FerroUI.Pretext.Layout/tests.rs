use std::rc::Rc;

use ferroui_pretext::{
    DelegateTextMeasurerFactory, PrepareOptions, PreparedTextWithSegments, PretextLayout,
    WhiteSpaceMode,
};

use crate::*;

fn prepare(text: &str) -> Rc<PreparedTextWithSegments> {
    let engine = PretextLayout::new(Rc::new(DelegateTextMeasurerFactory::new(
        |text: &str, _: &str| text.chars().count() as f64 * 10.0,
    )));
    engine
        .prepare_with_segments(text, "16px Test", PrepareOptions::default())
        .unwrap()
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-8,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn overlapping_unsorted_obstacles_leave_sorted_nonoverlapping_slots() {
    let slots = ObstacleLayoutHelper::carve_text_line_slots(
        Interval {
            left: 0.0,
            right: 100.0,
        },
        [
            Interval {
                left: 60.0,
                right: 80.0,
            },
            Interval {
                left: 20.0,
                right: 40.0,
            },
            Interval {
                left: 30.0,
                right: 70.0,
            },
        ],
        10.0,
    );
    assert_eq!(
        slots,
        [
            Interval {
                left: 0.0,
                right: 20.0
            },
            Interval {
                left: 80.0,
                right: 100.0
            }
        ]
    );
}

#[test]
fn blocked_slots_and_minimum_width_filtering_return_no_slot() {
    let base = Interval {
        left: 10.0,
        right: 90.0,
    };
    assert!(ObstacleLayoutHelper::carve_text_line_slots(
        base,
        [Interval {
            left: -10.0,
            right: 110.0
        }],
        0.0
    )
    .is_empty());
    assert!(ObstacleLayoutHelper::carve_text_line_slots(base, [], 81.0).is_empty());
    assert!(ObstacleLayoutHelper::pick_slot(&[], false).is_none());
}

#[test]
fn ties_choose_the_requested_edge_without_reordering_input() {
    let slots = [
        Interval {
            left: 80.0,
            right: 100.0,
        },
        Interval {
            left: 0.0,
            right: 20.0,
        },
    ];
    assert_eq!(
        ObstacleLayoutHelper::pick_slot(&slots, false),
        Some(slots[1])
    );
    assert_eq!(
        ObstacleLayoutHelper::pick_slot(&slots, true),
        Some(slots[0])
    );
}

#[test]
fn circle_band_uses_nearest_edge_not_midpoint() {
    let interval =
        ObstacleLayoutHelper::circle_interval_for_band(50.0, 9.0, 10.0, 0.0, 10.0, 2.0, 0.0)
            .unwrap();
    assert_eq!(
        interval,
        Interval {
            left: 38.0,
            right: 62.0
        }
    );
    let upper = ObstacleLayoutHelper::circle_interval_for_band(0.0, 0.0, 5.0, -6.0, -4.0, 0.0, 0.0)
        .unwrap();
    close(upper.width(), 6.0);
    assert!(
        ObstacleLayoutHelper::circle_interval_for_band(0.0, 0.0, 5.0, 5.0, 6.0, 0.0, 0.0).is_none()
    );
}

#[test]
fn ellipse_band_honors_horizontal_and_vertical_padding() {
    let interval =
        ObstacleLayoutHelper::ellipse_interval_for_band(10.0, 20.0, 8.0, 4.0, 24.0, 25.0, 2.0, 2.0)
            .unwrap();
    close(interval.width(), 16.0 * (0.75_f64).sqrt() + 4.0);
    assert!(ObstacleLayoutHelper::ellipse_interval_for_band(
        0.0, 0.0, 10.0, 0.0, 0.0, 10.0, 0.0, 0.0
    )
    .is_none());
}

#[test]
fn rectangle_bands_are_half_open_and_include_padding() {
    let rectangles = [RectObstacle {
        x: 10.0,
        y: 20.0,
        width: 30.0,
        height: 40.0,
    }];
    assert!(
        ObstacleLayoutHelper::get_rect_intervals_for_band(&rectangles, 0.0, 20.0, 0.0, 0.0)
            .is_empty()
    );
    assert!(
        ObstacleLayoutHelper::get_rect_intervals_for_band(&rectangles, 60.0, 70.0, 0.0, 0.0)
            .is_empty()
    );
    assert_eq!(
        ObstacleLayoutHelper::get_rect_intervals_for_band(&rectangles, 0.0, 20.0, 2.0, 1.0),
        [Interval {
            left: 8.0,
            right: 42.0
        }],
    );
}

#[test]
fn invalid_geometry_cannot_produce_nan_intervals() {
    assert!(ObstacleLayoutHelper::circle_interval_for_band(
        0.0,
        0.0,
        f64::NAN,
        0.0,
        10.0,
        0.0,
        0.0
    )
    .is_none());
    assert!(ObstacleLayoutHelper::carve_text_line_slots(
        Interval {
            left: 100.0,
            right: 0.0
        },
        [],
        0.0
    )
    .is_empty());
    assert!(ObstacleLayoutHelper::pick_slot(
        &[Interval {
            left: 0.0,
            right: f64::NAN
        }],
        false
    )
    .is_none());
}

#[test]
fn fully_blocked_first_row_does_not_consume_text() {
    let prepared = prepare("alpha beta");
    let columns = [RectObstacle {
        x: 20.0,
        y: 30.0,
        width: 100.0,
        height: 40.0,
    }];
    let obstacles = [RectObstacle {
        x: 20.0,
        y: 30.0,
        width: 100.0,
        height: 10.0,
    }];
    let lines =
        ColumnFlowLayout::layout_into_columns(&prepared, &columns, &obstacles, &[], 10.0, 1.0)
            .unwrap();
    assert_eq!(lines[0].x, 20.0);
    assert_eq!(lines[0].y, 40.0);
    assert_eq!(lines[0].text, "alpha beta");
}

#[test]
fn text_resumes_in_the_next_column() {
    let prepared = prepare("alpha beta gamma");
    let columns = [
        RectObstacle {
            x: 0.0,
            y: 10.0,
            width: 50.0,
            height: 10.0,
        },
        RectObstacle {
            x: 100.0,
            y: 20.0,
            width: 50.0,
            height: 20.0,
        },
    ];
    let lines =
        ColumnFlowLayout::layout_into_columns(&prepared, &columns, &[], &[], 10.0, 1.0).unwrap();
    assert_eq!(
        lines
            .iter()
            .map(|line| (line.text.as_str(), line.x, line.y))
            .collect::<Vec<_>>(),
        [
            ("alpha ", 0.0, 10.0),
            ("beta ", 100.0, 20.0),
            ("gamma", 100.0, 30.0),
        ]
    );
}

#[test]
fn column_flow_respects_full_circle_band() {
    let prepared = prepare("a b c");
    let columns = [RectObstacle {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 20.0,
    }];
    let circles = [CircleObstacle {
        x: 40.0,
        y: 9.0,
        radius: 20.0,
    }];
    let lines =
        ColumnFlowLayout::layout_into_columns(&prepared, &columns, &[], &circles, 10.0, 1.0)
            .unwrap();
    assert_eq!(lines[0].x, 60.0);
}

#[test]
fn invalid_line_height_and_nonprogressing_coordinates_are_rejected() {
    let prepared = prepare("text");
    let columns = [RectObstacle {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    }];
    for line_height in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            ColumnFlowLayout::layout_into_columns(&prepared, &columns, &[], &[], line_height, 1.0),
            Err(LayoutError::InvalidLineHeight)
        );
    }
    let columns = [RectObstacle {
        x: 0.0,
        y: 1e30,
        width: 100.0,
        height: 1e30,
    }];
    assert_eq!(
        ColumnFlowLayout::layout_into_columns(&prepared, &columns, &[], &[], 1.0, 1.0),
        Err(LayoutError::CoordinateOverflow)
    );
}

#[test]
fn impossible_columns_skip_without_iterating_tiny_line_heights() {
    let prepared = prepare("text");
    for width in [0.0, 10.0] {
        let columns = [RectObstacle {
            x: 0.0,
            y: 0.0,
            width,
            height: 1e300,
        }];
        let lines =
            ColumnFlowLayout::layout_into_columns(&prepared, &columns, &[], &[], 1e-300, 20.0)
                .unwrap();
        assert!(lines.is_empty());
    }
    let columns = [RectObstacle {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 1e300,
    }];
    let lines =
        ColumnFlowLayout::layout_into_columns(&prepared, &columns, &columns, &[], 1e-300, 20.0)
            .unwrap();
    assert!(lines.is_empty());
}

#[test]
fn natural_width_is_not_limited_to_one_hundred_thousand() {
    let prepared = prepare(&"a ".repeat(6000));
    assert_eq!(
        PreparedTextMetrics::measure_max_line_width(&prepared),
        119990.0
    );
}

#[test]
fn tight_wrap_matches_line_count_with_smaller_width() {
    let prepared = prepare("alpha beta gamma delta");
    let initial = PreparedTextMetrics::collect_wrap_metrics(&prepared, 125.0, 17.0);
    let tight = PreparedTextMetrics::find_tight_wrap_metrics(&prepared, 125.0, 17.0).unwrap();
    assert_eq!(tight.line_count, initial.line_count);
    assert_eq!(tight.height, initial.height);
    assert!(tight.max_line_width <= initial.max_line_width);
    assert_eq!(
        PreparedTextMetrics::find_tight_wrap_metrics(&prepared, f64::INFINITY, 17.0),
        Err(LayoutError::InvalidMaximumWidth)
    );
}

#[test]
fn max_line_width_respects_preserved_hard_breaks() {
    let engine = PretextLayout::new(Rc::new(DelegateTextMeasurerFactory::new(
        |text: &str, _: &str| text.chars().count() as f64 * 10.0,
    )));
    let prepared = engine
        .prepare_with_segments(
            "ab\nabcdef\n",
            "16px Test",
            PrepareOptions {
                white_space: WhiteSpaceMode::PreWrap,
                ..PrepareOptions::default()
            },
        )
        .unwrap();
    assert_eq!(PreparedTextMetrics::measure_max_line_width(&prepared), 60.0);
    let metrics = PreparedTextMetrics::collect_wrap_metrics(&prepared, 1000.0, 12.0);
    assert_eq!(
        metrics,
        WrapMetrics {
            line_count: 2,
            height: 24.0,
            max_line_width: 60.0
        }
    );
}
