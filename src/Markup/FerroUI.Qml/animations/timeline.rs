//! A per-property timeline preserves group timing even for children acting on
//! other properties. Loops use modular time instead of expanding keyframes.
use super::*;
use color::{parse_color, Srgb};

#[derive(Clone)]
pub(super) struct Timeline {
    kind: Kind,
    period: Duration,
    pub duration: Duration,
    pub end: Option<Value>,
}

#[derive(Clone)]
enum Kind {
    Gap,
    Point(Value),
    Interpolate {
        range: Range,
        curve: u8,
        reversed: bool,
    },
    Group(Vec<(Duration, Timeline)>),
}

#[derive(Clone)]
enum Range {
    Number {
        start: f64,
        delta: f64,
        integral: bool,
    },
    Color([u8; 4], [u8; 4]),
    Geometry(Geometry, Geometry),
}

impl Range {
    fn sample(&self, t: f64) -> Value {
        let lerp = |a: f64, b: f64| a + (b - a) * t;
        match *self {
            Self::Number {
                start,
                delta,
                integral,
            } => {
                let v = start + delta * t;
                Value::Number(if integral { v.trunc() } else { v })
            }
            Self::Color(a, b) => {
                let [r, g, b, a] =
                    std::array::from_fn(|i| lerp(f64::from(a[i]), f64::from(b[i])) as u8);
                Value::String(if a == 255 {
                    format!("#{r:02x}{g:02x}{b:02x}")
                } else {
                    format!("#{a:02x}{r:02x}{g:02x}{b:02x}")
                })
            }
            Self::Geometry(a, b) => {
                // The target type coerced both endpoints to the same geometry.
                // Only the named storage fields are interpolated; rectangle edges are derived.
                use Geometry::*;
                Value::Geometry(match (a, b) {
                    (Point { x, y }, Point { x: xx, y: yy }) => Point {
                        x: lerp(x, xx),
                        y: lerp(y, yy),
                    },
                    (
                        Size { width, height },
                        Size {
                            width: w,
                            height: h,
                        },
                    ) => Size {
                        width: lerp(width, w),
                        height: lerp(height, h),
                    },
                    (
                        Rect {
                            x,
                            y,
                            width,
                            height,
                        },
                        Rect {
                            x: xx,
                            y: yy,
                            width: w,
                            height: h,
                        },
                    ) => Rect {
                        x: lerp(x, xx),
                        y: lerp(y, yy),
                        width: lerp(width, w),
                        height: lerp(height, h),
                    },
                    _ => unreachable!("matched target geometry"),
                })
            }
        }
    }
}

fn rotation_delta(delta: f64, direction: i32) -> Result<f64, Error> {
    Ok(match direction {
        0 => delta,
        1 => {
            let turn = delta % 360.;
            if turn > 180. {
                turn - 360.
            } else if turn < -180. {
                turn + 360.
            } else {
                turn
            }
        }
        2 if delta < 0. => delta.rem_euclid(360.),
        3 if delta > 0. => -(-delta).rem_euclid(360.),
        2 | 3 => delta,
        _ => return Err(Error::new("invalid RotationAnimation direction")),
    })
}

impl Timeline {
    fn new(kind: Kind, period: Duration, loops: i32, end: Option<Value>) -> Self {
        if loops == 0 {
            return Self {
                kind: Kind::Gap,
                period: Duration::ZERO,
                duration: Duration::ZERO,
                end: None,
            };
        }
        let duration = if loops < 0 && !period.is_zero() {
            Duration::MAX
        } else {
            period.saturating_mul(loops.max(0) as u32)
        };
        Self {
            kind,
            period,
            duration,
            end,
        }
    }
    pub fn sample(&self, elapsed: Duration) -> Option<Value> {
        if elapsed >= self.duration || self.period.is_zero() {
            return self.end.clone();
        }
        let t = elapsed.as_nanos() % self.period.as_nanos();
        match &self.kind {
            Kind::Gap => None,
            Kind::Point(value) => Some(value.clone()),
            Kind::Group(children) => {
                // A later active child on the same property has precedence.
                children.iter().rev().find_map(|(start, child)| {
                    let elapsed = t.checked_sub(start.as_nanos())?;
                    child.sample(Duration::from_nanos(
                        elapsed.min(u128::from(u64::MAX)) as u64
                    ))
                })
            }
            Kind::Interpolate {
                range,
                curve,
                reversed,
            } => Some(range.sample(progress(*curve, *reversed, t, self.period))),
        }
    }
}

fn progress(curve: u8, reversed: bool, elapsed: u128, duration: Duration) -> f64 {
    let t = elapsed as f64 / duration.as_nanos() as f64;
    if reversed {
        1. - easing(curve, 1. - t)
    } else {
        easing(curve, t)
    }
}

fn color(value: &Value) -> Result<[u8; 4], Error> {
    let Value::String(s) = value else {
        return Err(Error::new("ColorAnimation requires color properties"));
    };
    // Qt uses #AARRGGBB. CSS places the alpha pair after RGB.
    let css = match s.strip_prefix('#') {
        Some(hex) if hex.len() == 8 && hex.is_ascii() => {
            let (alpha, rgb) = hex.split_at(2);
            format!("#{rgb}{alpha}")
        }
        Some(hex) if hex.len() != 3 && hex.len() != 6 => {
            return Err(Error::new(format!("unsupported animation color '{s}'")));
        }
        _ => s.clone(),
    };
    parse_color(&css)
        .map(|c| c.to_alpha_color::<Srgb>().to_rgba8().to_u8_array())
        .map_err(|_| Error::new(format!("invalid animation color '{s}'")))
}

impl Instance {
    pub(crate) fn is_animation(&self, id: ObjectId) -> bool {
        self.node(id).is_some_and(|n| {
            n.module == "QtQuick"
                && matches!(
                    n.name.as_str(),
                    "NumberAnimation"
                        | "RotationAnimation"
                        | "PropertyAnimation"
                        | "ColorAnimation"
                        | "SequentialAnimation"
                        | "ParallelAnimation"
                        | "PauseAnimation"
                        | "PropertyAction"
                )
        })
    }

    fn is_group(&self, id: ObjectId) -> bool {
        self.quick_kind(id, "SequentialAnimation") || self.quick_kind(id, "ParallelAnimation")
    }

    pub(super) fn accepts_channel(&self, id: ObjectId, key: SlotId) -> bool {
        if !self.is_group(id) {
            return self.accepts_leaf(id, key);
        }
        let mut pending = vec![id];
        let mut seen = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if seen.insert(id) {
                if self.is_group(id) {
                    pending.extend(self.state_objects(id, "animations"));
                } else if self.accepts_leaf(id, key) {
                    return true;
                }
            }
        }
        false
    }

    fn accepts_leaf(&self, id: ObjectId, key: SlotId) -> bool {
        let ty = self.properties[key].ty.as_str();
        let numeric = matches!(ty, "int" | "real" | "float" | "double");
        if !self.animation_matches(id, key) {
            return false;
        }
        match self.type_name(id) {
            Some("PropertyAction") => true,
            Some("NumberAnimation") => numeric,
            Some("ColorAnimation") => ty == "color",
            Some("PropertyAnimation") => {
                numeric || matches!(ty, "color" | "point" | "size" | "rect")
            }
            Some("RotationAnimation") => {
                numeric
                    && (matches!(self.properties[key].name.as_str(), "rotation" | "angle")
                        || ["property", "properties"].iter().any(
                            |p| matches!(self.get(id,p),Ok(Value::String(s)) if !s.is_empty()),
                        )
                        || self.inherits_animation_target(id))
            }
            _ => false,
        }
    }

    fn inherits_animation_target(&self, mut id: ObjectId) -> bool {
        while let Some(parent) = self.parent(id) {
            if self.quick_kind(parent, "Behavior") {
                return true;
            }
            if !self.is_group(parent) {
                break;
            }
            if matches!(self.get(parent,"$qml:onProperty"),Ok(Value::String(s)) if !s.is_empty()) {
                return true;
            }
            id = parent;
        }
        false
    }

    pub(super) fn playback_targets(
        &self,
        id: ObjectId,
        keys: &mut BTreeSet<SlotId>,
    ) -> Result<(), Error> {
        let mut pending = vec![id];
        let mut seen = BTreeSet::new();
        while let Some(driver) = pending.pop() {
            if seen.insert(driver) {
                self.collect_playback_targets(driver, keys)?;
                if self.is_group(driver) {
                    pending.extend(self.state_objects(driver, "animations"));
                }
            }
        }
        Ok(())
    }

    fn collect_playback_targets(
        &self,
        id: ObjectId,
        keys: &mut BTreeSet<SlotId>,
    ) -> Result<(), Error> {
        let (target, property) = if self.is_group(id) {
            ("$qml:onTarget", "$qml:onProperty")
        } else {
            ("target", "property")
        };
        let mut targets = self.state_objects(id, "targets");
        if let Ok(k) = self.key(id, target) {
            if let Value::Object(t) = self.slots[k].value {
                targets.push(t);
            }
        }
        let mut properties = BTreeSet::new();
        for name in [property, "properties"] {
            if let Ok(Value::String(names)) =
                self.key(id, name).map(|k| self.slots[k].value.clone())
            {
                properties.extend(
                    names
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned),
                );
            }
        }
        let excluded = self.state_objects(id, "exclude");
        for t in targets {
            if excluded.contains(&t) {
                continue;
            }
            if properties.is_empty() && self.quick_kind(id, "RotationAnimation") {
                for name in ["rotation", "angle"] {
                    if let Ok(slot) = self.key(t, name) {
                        keys.insert(self.write_target(slot)?);
                    }
                }
            }
            for p in &properties {
                keys.insert(self.write_target(self.key(t, p)?)?);
            }
        }
        Ok(())
    }

    pub(super) fn timeline(
        &self,
        driver: ObjectId,
        key: SlotId,
        goal: &Value,
        reversed: bool,
    ) -> Result<Timeline, Error> {
        let mut remaining = self.nodes.len().saturating_mul(4).saturating_add(256);
        self.animation_clip(
            driver,
            key,
            &self.slots[key].value,
            (goal, reversed),
            0,
            &mut remaining,
        )
    }

    fn animation_clip(
        &self,
        driver: ObjectId,
        key: SlotId,
        initial: &Value,
        destination: (&Value, bool),
        depth: usize,
        remaining: &mut usize,
    ) -> Result<Timeline, Error> {
        let (goal, reversed) = destination;
        if depth > 64 || *remaining == 0 {
            return Err(Error::new(
                "animation group exceeds nesting or expansion limit",
            ));
        }
        *remaining -= 1;
        if !self.is_animation(driver) {
            return Err(Error::new("unsupported animation child"));
        }
        let loops = if depth == 0 {
            1
        } else {
            self.numeric(driver, "loops").unwrap_or(1.) as i32
        };
        if self.is_group(driver) {
            let sequential = self.quick_kind(driver, "SequentialAnimation");
            let mut children = self.state_objects(driver, "animations");
            if reversed && sequential {
                children.reverse();
            }
            let mut plan = Vec::with_capacity(children.len());
            let mut duration = Duration::ZERO;
            let mut end = None;
            for child in children {
                let offset = if sequential { duration } else { Duration::ZERO };
                let start = if sequential {
                    end.as_ref().unwrap_or(initial)
                } else {
                    initial
                };
                let clip =
                    self.animation_clip(child, key, start, destination, depth + 1, remaining)?;
                duration = duration.max(offset.saturating_add(clip.duration));
                if let Some(v) = &clip.end {
                    end = Some(v.clone());
                }
                plan.push((offset, clip));
            }
            return Ok(Timeline::new(Kind::Group(plan), duration, loops, end));
        }
        let action = self.quick_kind(driver, "PropertyAction");
        let duration = if action {
            Duration::ZERO
        } else {
            Duration::from_millis(self.numeric(driver, "duration").unwrap_or(250.).max(0.) as u64)
        };
        if loops == 0
            || self.quick_kind(driver, "PauseAnimation")
            || !self.accepts_leaf(driver, key)
        {
            return Ok(Timeline::new(Kind::Gap, duration, loops, None));
        }
        let (from, to) = if reversed {
            ("to", "from")
        } else {
            ("from", "to")
        };
        let supplied = |name: &str, default: &Value| {
            self.get(driver, name)
                .ok()
                .filter(|v| v != &Value::Undefined)
                .unwrap_or_else(|| default.clone())
        };
        let end = self.typed(
            &self.properties[key].ty,
            supplied(if action { "value" } else { to }, goal),
        )?;
        let start = supplied(from, initial);
        let kind = if action {
            Kind::Point(end.clone())
        } else {
            let curve = self.numeric(driver, "easing.type").unwrap_or(0.) as u8;
            let ty = &self.properties[key].ty;
            let generic = self.quick_kind(driver, "PropertyAnimation");
            let start = if generic {
                self.typed(ty, start)?
            } else {
                start
            };
            let range = match (&start, &end) {
                (Value::Number(a), Value::Number(b)) => {
                    let mut delta = b - a;
                    if self.quick_kind(driver, "RotationAnimation") {
                        let direction = self.numeric(driver, "direction").unwrap_or(0.) as i32;
                        delta = if reversed {
                            -rotation_delta(-delta, direction)?
                        } else {
                            rotation_delta(delta, direction)?
                        };
                    }
                    Range::Number {
                        start: *a,
                        delta,
                        integral: generic && ty == "int",
                    }
                }
                (Value::Geometry(a), Value::Geometry(b)) if generic => Range::Geometry(*a, *b),
                _ if ty == "color" => Range::Color(color(&start)?, color(&end)?),
                _ => {
                    return Err(Error::new(format!(
                        "invalid animation endpoints for '{ty}'"
                    )))
                }
            };
            Kind::Interpolate {
                range,
                curve,
                reversed,
            }
        };
        Ok(Timeline::new(kind, duration, loops, Some(end)))
    }
}
