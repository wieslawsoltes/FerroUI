use crate::{require, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurveWindow {
    pub start: f64,
    pub end: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct CurvePoint {
    pub time: f64,
    pub value: f64,
}

impl CurveWindow {
    /// Add authored keys and sample each short segment so that a brief eased
    /// movement is visible even when it falls between the regular samples.
    /// Dense windows use a bounded uniform grid until the user zooms in.
    pub fn sample_times(
        self,
        duration: f64,
        keys: impl IntoIterator<Item = f64>,
    ) -> Result<Vec<f64>> {
        require(
            self.start.is_finite()
                && self.end.is_finite()
                && self.start >= 0.0
                && self.start < self.end
                && self.end <= duration,
            "Choose a curve range within the animation duration",
        )?;
        const SAMPLES: usize = 128;
        let mut points = vec![self.start, self.end];
        subdivide(self.start, self.end, SAMPLES, &mut points);
        let mut edges = keys
            .into_iter()
            .filter(|&time| self.start < time && time < self.end)
            .take(SAMPLES + 1)
            .collect::<Vec<_>>();
        if edges.len() <= SAMPLES {
            edges.extend([self.start, self.end]);
            edges.sort_unstable_by(f64::total_cmp);
            edges.dedup();
            for pair in edges.windows(2) {
                subdivide(pair[0], pair[1], 16, &mut points);
            }
            points.extend(edges);
        }
        points.sort_unstable_by(f64::total_cmp);
        points.dedup();
        Ok(points)
    }
}

fn subdivide(start: f64, end: f64, segments: usize, times: &mut Vec<f64>) {
    times.extend(
        (1..segments).map(|index| start + (end - start) * (index as f64 / segments as f64)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AnimationValue::Number, Easing, Timeline};

    #[test]
    fn geometry_components_follow_the_authored_value_order() {
        use crate::{AnimationValue::CornerRadius, CurveChannel, ValueKind};
        let mut timeline = Timeline::new(2.0).unwrap();
        for (time, top_left, bottom_left) in [(0.0, 4.0, 20.0), (2.0, 20.0, 4.0)] {
            timeline
                .record(
                    12,
                    "CornerRadius",
                    time,
                    CornerRadius {
                        top_left,
                        top_right: 8.0,
                        bottom_right: 12.0,
                        bottom_left,
                    },
                )
                .unwrap();
        }
        let compiled = timeline.compile().unwrap();
        let window = CurveWindow {
            start: 0.0,
            end: 2.0,
        };
        let top = compiled
            .curve(12, "CornerRadius", CurveChannel::TopLeft, window)
            .unwrap();
        let bottom = compiled
            .curve(12, "CornerRadius", CurveChannel::BottomLeft, window)
            .unwrap();
        assert_eq!((top[0].value, top.last().unwrap().value), (4.0, 20.0));
        assert_eq!((bottom[0].value, bottom.last().unwrap().value), (20.0, 4.0));
        assert!(compiled.scalar_curve(12, "CornerRadius", window).is_err());
        assert_eq!(
            CurveChannel::for_authored_kind("CornerRadius"),
            CurveChannel::for_kind(ValueKind::CornerRadius)
        );
    }

    #[test]
    fn graph_samples_implicit_endpoints_and_easing_like_the_playhead() {
        let mut timeline = Timeline::new(3.0).unwrap();
        for (time, value) in [(0.3, 0.25), (0.301, 0.85), (2.0, 0.4)] {
            timeline.record(17, "Opacity", time, Number(value)).unwrap();
        }
        let track = timeline.track_mut(17, "Opacity").unwrap();
        track.base_value = Some(Number(0.1));
        track
            .set_easing(0.301, Easing::parse("0.25,0.3,0.75,1.4").unwrap())
            .unwrap();
        let compiled = timeline.compile().unwrap();
        let graph = compiled
            .scalar_curve(
                17,
                "Opacity",
                CurveWindow {
                    start: 0.0,
                    end: 3.0,
                },
            )
            .unwrap();
        assert_eq!(
            graph[0],
            CurvePoint {
                time: 0.0,
                value: 0.1
            }
        );
        assert!(graph
            .iter()
            .any(|point| point.time == 0.301 && point.value == 0.85));
        assert!(graph
            .iter()
            .any(|point| (0.3..0.301).contains(&point.time) && point.value > 0.85));
        assert!(graph
            .iter()
            .all(|point| compiled.scrub(point.time).unwrap()[0].value == Number(point.value)));
    }

    #[test]
    fn graph_range_is_bounded_and_keeps_exact_window_edges() {
        let window = CurveWindow {
            start: 0.1,
            end: 0.9,
        };
        let samples = window
            .sample_times(1.0, (0..1000).map(|value| value as f64 / 1000.0))
            .unwrap();
        assert_eq!(samples.len(), 129);
        assert_eq!(samples.first(), Some(&window.start));
        assert_eq!(samples.last(), Some(&window.end));
        assert!(window.sample_times(0.5, []).is_err());
        assert!(CurveWindow {
            start: 0.0,
            end: f64::NAN
        }
        .sample_times(1.0, [])
        .is_err());
    }
}
