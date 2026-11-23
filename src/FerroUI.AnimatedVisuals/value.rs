//! Animated values: a static value, or keyframes with cubic Bezier and hold easing, and for a
//! position the spatial tangents of the motion path.

use serde_json::Value;

/// The components of a value: a number, a point, a colour.
pub(crate) type Components = [f64; 4];

/// The easing of one component over a keyframe: the control points of a unit cubic Bezier.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Ease {
    out_x: f64,
    out_y: f64,
    in_x: f64,
    in_y: f64,
}

impl Ease {
    /// The control points that make a cubic Bezier a straight line at constant speed.
    const LINEAR: Ease = Ease { out_x: 1.0 / 3.0, out_y: 1.0 / 3.0, in_x: 2.0 / 3.0, in_y: 2.0 / 3.0 };

    /// The eased progress at `progress` of the keyframe.
    pub(crate) fn at(&self, progress: f64) -> f64 {
        if *self == Self::LINEAR {
            return progress;
        }
        let bezier = |a: f64, b: f64, t: f64| {
            let u = 1.0 - t;
            3.0 * u * u * t * a + 3.0 * u * t * t * b + t * t * t
        };
        let (mut low, mut high, mut t) = (0.0_f64, 1.0_f64, progress);
        for _ in 0..24 {
            let x = bezier(self.out_x, self.in_x, t);
            if (x - progress).abs() < 1e-7 {
                break;
            }
            if x < progress {
                low = t;
            } else {
                high = t;
            }
            let u = 1.0 - t;
            let slope = 3.0 * u * u * self.out_x + 6.0 * u * t * (self.in_x - self.out_x) + 3.0 * t * t * (1.0 - self.in_x);
            let next = if slope.abs() > 1e-6 { t - (x - progress) / slope } else { -1.0 };
            t = if next > low && next < high { next } else { (low + high) * 0.5 };
        }
        bezier(self.out_y, self.in_y, t)
    }
}

fn number(value: Option<&Value>, index: usize) -> Option<f64> {
    match value? {
        Value::Number(number) => number.as_f64(),
        Value::Array(items) => items.get(index).or(items.first()).and_then(Value::as_f64),
        _ => None,
    }
}

/// The eases of each component of a keyframe (`o` and `i`).
pub(crate) fn parse_eases(key: &Value) -> [Ease; 4] {
    let (out, into) = (key.get("o"), key.get("i"));
    let mut eases = [Ease::LINEAR; 4];
    let (Some(out), Some(into)) = (out, into) else {
        return eases;
    };
    for (index, ease) in eases.iter_mut().enumerate() {
        let parts = (
            number(out.get("x"), index),
            number(out.get("y"), index),
            number(into.get("x"), index),
            number(into.get("y"), index),
        );
        if let (Some(out_x), Some(out_y), Some(in_x), Some(in_y)) = parts {
            *ease = Ease {
                out_x: out_x.clamp(0.0, 1.0),
                out_y,
                in_x: in_x.clamp(0.0, 1.0),
                in_y,
            };
        }
    }
    eases
}

/// The keyframes of a property value, when `k` holds keyframes rather than a value.
pub(crate) fn keyframes(property: &Value) -> Option<&Vec<Value>> {
    let items = property.get("k")?.as_array()?;
    items.first()?.get("t")?;
    Some(items)
}

fn components(value: &Value) -> Option<(Components, usize)> {
    let mut result = [0.0; 4];
    match value {
        Value::Number(number) => {
            result[0] = number.as_f64()?;
            Some((result, 1))
        }
        Value::Array(items) => {
            for (slot, item) in result.iter_mut().zip(items) {
                *slot = item.as_f64()?;
            }
            Some((result, items.len().min(4)))
        }
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Keyframe {
    time: f64,
    start: Components,
    end: Components,
    eases: [Ease; 4],
    hold: bool,
    /// The out and in tangents of the motion path, relative to the start and the end.
    spatial: Option<([f64; 2], [f64; 2])>,
}

/// A value that may change over the frames of the composition.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Animated {
    value: Components,
    keys: Vec<Keyframe>,
}

fn point(value: Option<&Value>) -> Option<[f64; 2]> {
    let items = value?.as_array()?;
    Some([items.first()?.as_f64()?, items.get(1)?.as_f64()?])
}

impl Animated {
    pub(crate) fn constant(value: Components) -> Self {
        Self { value, keys: Vec::new() }
    }

    /// The property `name` of `owner`, or `default` when it is absent or malformed.
    pub(crate) fn of(owner: &Value, name: &str, default: Components) -> Self {
        owner.get(name).and_then(Self::parse).unwrap_or_else(|| Self::constant(default))
    }

    pub(crate) fn parse(property: &Value) -> Option<Self> {
        let Some(items) = keyframes(property) else {
            return components(property.get("k")?).map(|(value, _)| Self::constant(value));
        };
        let mut keys = Vec::with_capacity(items.len());
        let mut previous_end = None;
        for (index, key) in items.iter().enumerate() {
            let time = key.get("t")?.as_f64()?;
            // A last keyframe may carry only its time: it holds where the one before ended.
            let start = key.get("s").and_then(components).map(|(value, _)| value).or(previous_end)?;
            let end = key
                .get("e")
                .and_then(components)
                .map(|(value, _)| value)
                .or_else(|| {
                    items.get(index + 1).and_then(|next| next.get("s")).and_then(components).map(|(value, _)| value)
                })
                .unwrap_or(start);
            let hold = key.get("h").and_then(Value::as_i64) == Some(1);
            let spatial = match (point(key.get("to")), point(key.get("ti"))) {
                (Some(to), Some(ti)) if to != [0.0; 2] || ti != [0.0; 2] => Some((to, ti)),
                _ => None,
            };
            keys.push(Keyframe { time, start, end, eases: parse_eases(key), hold, spatial });
            previous_end = Some(end);
        }
        Some(Self { value: keys.first()?.start, keys })
    }

    /// The value at `frame`.
    pub(crate) fn at(&self, frame: f64) -> Components {
        let (Some(first), Some(last)) = (self.keys.first(), self.keys.last()) else {
            return self.value;
        };
        if frame <= first.time {
            return first.start;
        }
        if frame >= last.time {
            return last.start;
        }
        let index = self.keys.partition_point(|key| key.time <= frame) - 1;
        let (key, next) = (&self.keys[index], &self.keys[index + 1]);
        if key.hold || next.time <= key.time {
            return key.start;
        }
        let progress = (frame - key.time) / (next.time - key.time);
        if let Some((to, ti)) = key.spatial {
            return spatial_at(key, to, ti, key.eases[0].at(progress));
        }
        let mut result = [0.0; 4];
        for (component, slot) in result.iter_mut().enumerate() {
            let eased = key.eases[component].at(progress);
            *slot = key.start[component] + (key.end[component] - key.start[component]) * eased;
        }
        result
    }
}

/// The point at `fraction` of the length of the motion path of a keyframe.
fn spatial_at(key: &Keyframe, to: [f64; 2], ti: [f64; 2], fraction: f64) -> Components {
    const STEPS: usize = 24;
    let p0 = [key.start[0], key.start[1]];
    let p3 = [key.end[0], key.end[1]];
    let p1 = [p0[0] + to[0], p0[1] + to[1]];
    let p2 = [p3[0] + ti[0], p3[1] + ti[1]];
    let at = |t: f64| {
        let u = 1.0 - t;
        let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
        [
            w[0] * p0[0] + w[1] * p1[0] + w[2] * p2[0] + w[3] * p3[0],
            w[0] * p0[1] + w[1] * p1[1] + w[2] * p2[1] + w[3] * p3[1],
        ]
    };
    let mut lengths = [0.0; STEPS + 1];
    let mut last = p0;
    for step in 1..=STEPS {
        let point = at(step as f64 / STEPS as f64);
        lengths[step] = lengths[step - 1] + (point[0] - last[0]).hypot(point[1] - last[1]);
        last = point;
    }
    let target = lengths[STEPS] * fraction;
    let step = lengths.partition_point(|length| *length < target).clamp(1, STEPS);
    let span = lengths[step] - lengths[step - 1];
    let within = if span > 0.0 { (target - lengths[step - 1]) / span } else { 0.0 };
    let point = at((step as f64 - 1.0 + within) / STEPS as f64);
    [point[0], point[1], key.start[2], key.start[3]]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_static_value_is_the_same_at_every_frame() {
        let value = Animated::parse(&json!({"a": 0, "k": [10, 20]})).unwrap();
        assert_eq!(value.at(0.0)[..2], [10.0, 20.0]);
        assert_eq!(value.at(99.0)[..2], [10.0, 20.0]);
        assert_eq!(Animated::parse(&json!({"k": 5})).unwrap().at(3.0)[0], 5.0);
    }

    #[test]
    fn linear_keyframes_interpolate_and_hold_outside_their_range() {
        let value = Animated::parse(&json!({"a": 1, "k": [
            {"t": 10, "s": [0], "e": [100]},
            {"t": 20}
        ]}))
        .unwrap();
        assert_eq!(value.at(0.0)[0], 0.0);
        assert!((value.at(15.0)[0] - 50.0).abs() < 1e-9);
        assert_eq!(value.at(30.0)[0], 100.0);
    }

    #[test]
    fn the_end_of_a_keyframe_can_be_the_start_of_the_next() {
        let value = Animated::parse(&json!({"k": [{"t": 0, "s": [0]}, {"t": 10, "s": [40]}]})).unwrap();
        assert!((value.at(5.0)[0] - 20.0).abs() < 1e-9);
        assert_eq!(value.at(10.0)[0], 40.0);
    }

    #[test]
    fn a_hold_keyframe_keeps_its_value_until_the_next() {
        let value = Animated::parse(&json!({"k": [
            {"t": 0, "s": [1], "h": 1},
            {"t": 10, "s": [2]}
        ]}))
        .unwrap();
        assert_eq!(value.at(9.9)[0], 1.0);
        assert_eq!(value.at(10.0)[0], 2.0);
    }

    #[test]
    fn cubic_easing_follows_its_control_points() {
        // An ease-in: slow at the start, so below the linear value half way.
        let value = Animated::parse(&json!({"k": [
            {"t": 0, "s": [0], "e": [100], "o": {"x": [0.8], "y": [0.0]}, "i": {"x": [1.0], "y": [1.0]}},
            {"t": 10}
        ]}))
        .unwrap();
        let half = value.at(5.0)[0];
        assert!(half > 0.0 && half < 40.0, "{half}");
        assert!(value.at(1.0)[0] < value.at(2.0)[0]);
    }

    #[test]
    fn a_motion_path_goes_through_its_tangents_at_constant_speed() {
        let value = Animated::parse(&json!({"k": [
            {"t": 0, "s": [0, 0], "e": [100, 0], "to": [0, 50], "ti": [0, 50]},
            {"t": 10}
        ]}))
        .unwrap();
        let middle = value.at(5.0);
        assert!((middle[0] - 50.0).abs() < 0.5, "{middle:?}");
        assert!((middle[1] - 37.5).abs() < 0.5, "{middle:?}");
    }
}
