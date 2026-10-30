//! PieSeriesTest.ShouldPlaceDataLabel at the pin: thirteen pie series of one value each in a
//! 500 by 500 chart with a draw margin of 50, the five label positions for the normal, inner,
//! outer and inner-and-outer radii, with the source's assertions on the label locations. The
//! source reads the label visual of each chart point; the native engine places a text primitive
//! per point, centred on the label location, measured here with fixed native text metrics.

use crate::*;
use ferroui_base::{Point, Rect, Size};

const LABEL: Size = Size::new(20.0, 12.0);

/// (label location, slice start angle, slice sweep angle, slice outer radius) per point.
fn labels(series: &[Series]) -> Vec<(Point, f64, f64, f64)> {
    let options = PieOptions {
        draw_margin: Some(Rect::new(50.0, 50.0, 400.0, 400.0)),
        ..PieOptions::default()
    };
    let scene =
        PieChartEngine::measure_with_labels(Size::new(500.0, 500.0), series, &options, |_, _| {
            LABEL
        })
        .unwrap();
    scene
        .hits
        .iter()
        .map(|hit| {
            let HitShape::AnnularSector {
                outer_radius,
                start_angle,
                sweep_angle,
                ..
            } = hit.shape
            else {
                panic!("a pie slice")
            };
            let origin = scene
                .primitives
                .iter()
                .find_map(|p| match p.geometry {
                    Geometry::Text { origin, .. } if p.id == Some(hit.id) => Some(origin),
                    _ => None,
                })
                .expect("a label for every point");
            let location = Point::new(origin.x, origin.y + LABEL.height * 0.5);
            (location, start_angle, sweep_angle, outer_radius)
        })
        .collect()
}

fn degrees_from_center(p: Point) -> f64 {
    let a = (p.y - 250.0).atan2(p.x - 250.0).to_degrees();
    if a < 0.0 {
        a + 360.0
    } else {
        a
    }
}

fn distance_from_center(p: Point) -> f64 {
    (p.x - 250.0).hypot(p.y - 250.0)
}

#[test]
fn source_should_place_data_label() {
    let mut series: Vec<Series> = (0..13)
        .map(|_| Series::from_values(SeriesKind::Pie, [1.0]))
        .collect();

    // Hidden: the default position is Middle and no label is drawn.
    assert!(series
        .iter()
        .all(|s| s.polar_labels_position == PolarLabelsPosition::Middle));
    let options = PieOptions {
        draw_margin: Some(Rect::new(50.0, 50.0, 400.0, 400.0)),
        ..PieOptions::default()
    };
    let hidden = PieChartEngine::measure(Size::new(500.0, 500.0), &series, &options).unwrap();
    assert!(!hidden
        .primitives
        .iter()
        .any(|p| matches!(p.geometry, Geometry::Text { .. })));

    for s in &mut series {
        s.data_labels = true;
    }
    // normal, inner, outer, inner and outer
    for (inner_radius, outer_radius_offset) in [(0.0, 0.0), (100.0, 0.0), (0.0, 10.0), (50.0, 10.0)]
    {
        for s in &mut series {
            s.inner_radius = inner_radius;
            s.outer_radius_offset = outer_radius_offset;
        }
        for position in [
            PolarLabelsPosition::ChartCenter,
            PolarLabelsPosition::Middle,
            PolarLabelsPosition::Start,
            PolarLabelsPosition::End,
            PolarLabelsPosition::Outer,
        ] {
            for s in &mut series {
                s.polar_labels_position = position;
            }
            let placed = labels(&series);
            assert_eq!(placed.len(), 13);
            for (label, start, sweep, outer) in placed {
                let case =
                    format!("{position:?}, inner {inner_radius}, offset {outer_radius_offset}");
                if position == PolarLabelsPosition::ChartCenter {
                    assert!(
                        (label.x - 250.0).abs() < 0.01 && (label.y - 250.0).abs() < 0.01,
                        "{case}"
                    );
                    continue;
                }
                let h = distance_from_center(label);
                let expected = match position {
                    PolarLabelsPosition::Start => start,
                    PolarLabelsPosition::End => {
                        let r = start + sweep;
                        if (r - 360.0).abs() < 0.001 {
                            0.0
                        } else {
                            r
                        }
                    }
                    _ => start + sweep * 0.5,
                };
                if position == PolarLabelsPosition::Outer {
                    // The source compares with half the width of the slice visual, its outer
                    // radius.
                    assert!(h > outer, "{case}: {h} > {outer}");
                } else {
                    assert!(h < 500.0 * 0.5 * 0.65, "{case}: {h}");
                }
                // Compared on the circle: in f64 a label at the end of the last slice can sit a
                // rounding error below the positive x axis, at 360 rather than 0 degrees.
                let a = degrees_from_center(label);
                let difference = (a - expected).rem_euclid(360.0);
                assert!(
                    difference.min(360.0 - difference) < 0.01,
                    "{case}: {a} != {expected}"
                );
            }
        }
    }
}
