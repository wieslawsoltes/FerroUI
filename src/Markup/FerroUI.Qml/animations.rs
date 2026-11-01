//! Numeric property animation samples the same owner-thread clock as QML Timer.
//! Sample writes leave the target binding intact, allowing mid-flight retargets.
use super::*;
use crate::transaction::Transaction;
use std::time::Duration;
mod timeline;
use timeline::Timeline;

pub(crate) fn register(registry: &mut TypeRegistry) -> Result<(), Error> {
    let object = || TypeDefinition::default().inherits("QtQml", "QtObject");
    registry.register(
        "QtQuick",
        "Behavior",
        object()
            .property("enabled", "bool", Value::Bool(true))
            .property("animation", "QtObject", Value::Null)
            .default_property("animation")
            .readonly_property("targetProperty.object", "QtObject", Value::Null)
            .readonly_property(
                "targetProperty.name",
                "string",
                Value::String(String::new()),
            )
            .readonly_property("targetValue", "var", Value::Undefined),
    )?;
    registry.register(
        "QtQuick",
        "Animation",
        object()
            .property("loops", "int", Value::Number(1.))
            .property("running", "bool", Value::Bool(false))
            .property("paused", "bool", Value::Bool(false))
            .readonly_property("$qml:clock", "real", Value::Number(0.))
            .signal("finished", vec![])
            .enumeration("Loops", &[("Infinite", -1)])?,
    )?;
    let animation = || TypeDefinition::default().inherits("QtQuick", "Animation");
    let targeted = || {
        animation()
            .property("target", "QtObject", Value::Null)
            .property("targets", "list<QtObject>", Value::List(vec![]))
            .property("property", "string", Value::String(String::new()))
            .property("properties", "string", Value::String(String::new()))
    };
    registry.register(
        "QtQuick",
        "PropertyAnimation",
        targeted()
            .property("exclude", "list<QtObject>", Value::List(vec![]))
            .property("from", "var", Value::Undefined)
            .property("to", "var", Value::Undefined)
            .property("duration", "int", Value::Number(250.))
            .property("easing.type", "int", Value::Number(0.)),
    )?;
    for (name, ty) in [
        ("NumberAnimation", "real"),
        ("ColorAnimation", "color"),
        ("RotationAnimation", "real"),
    ] {
        let mut definition = TypeDefinition::default()
            .inherits("QtQuick", "PropertyAnimation")
            .property("from", ty, Value::Undefined)
            .property("to", ty, Value::Undefined);
        if name == "RotationAnimation" {
            definition = definition
                .property("direction", "int", Value::Number(0.))
                .enumeration(
                    "RotationDirection",
                    &[
                        ("Numerical", 0),
                        ("Shortest", 1),
                        ("Clockwise", 2),
                        ("Counterclockwise", 3),
                    ],
                )?;
        }
        registry.register("QtQuick", name, definition)?;
    }
    registry.register(
        "QtQuick",
        "PauseAnimation",
        animation().property("duration", "int", Value::Number(250.)),
    )?;
    registry.register(
        "QtQuick",
        "PropertyAction",
        targeted().property("value", "var", Value::Undefined),
    )?;
    for group in ["SequentialAnimation", "ParallelAnimation"] {
        registry.register(
            "QtQuick",
            group,
            animation()
                .readonly_property("$qml:onTarget", "QtObject", Value::Null)
                .readonly_property("$qml:onProperty", "string", Value::String(String::new()))
                .property("animations", "list<Animation>", Value::List(vec![]))
                .default_property("animations"),
        )?;
    }
    registry.register(
        "QtQuick",
        "Easing",
        object().enumeration(
            "Type",
            &[
                ("Linear", 0),
                ("InQuad", 1),
                ("OutQuad", 2),
                ("InOutQuad", 3),
                ("OutInQuad", 4),
                ("InCubic", 5),
                ("OutCubic", 6),
                ("InOutCubic", 7),
                ("OutInCubic", 8),
                ("InQuart", 9),
                ("OutQuart", 10),
                ("InOutQuart", 11),
                ("OutInQuart", 12),
                ("InQuint", 13),
                ("OutQuint", 14),
                ("InOutQuint", 15),
                ("OutInQuint", 16),
            ],
        )?,
    )
}

#[derive(Clone)]
pub(crate) struct Track {
    driver: ObjectId,
    transition: Option<ObjectId>,
    start: Duration,
    duration: Duration,
    paused_at: Option<Duration>,
    loops: i32,
    timeline: Rc<Timeline>,
    requested: Value,
}

impl Track {
    pub(crate) fn ticking(&self) -> bool {
        self.paused_at.is_none()
    }
}

#[derive(Default)]
pub(crate) struct Turn {
    before: BTreeMap<SlotId, Option<Track>>,
    held_before: BTreeMap<SlotId, Option<(ObjectId, Value)>>,
    pub pending: BTreeSet<ObjectId>,
    transitions: BTreeMap<SlotId, (ObjectId, ObjectId, bool)>,
}

impl Turn {
    pub(crate) fn rollback(self, model: &mut Instance) {
        for (key, before) in self.held_before {
            if let Some(old) = before {
                model.timers.held.insert(key, old);
            } else {
                model.timers.held.remove(&key);
            }
        }
        for (key, before) in self.before {
            model.timers.set_track(key, before);
        }
    }
}

fn easing(kind: u8, t: f64) -> f64 {
    if kind == 0 || kind > 16 {
        return t;
    }
    let power = 2 + i32::from((kind - 1) / 4);
    let inside = |t: f64| t.powi(power);
    let outside = |t: f64| 1. - inside(1. - t);
    match (kind - 1) % 4 {
        0 => inside(t),
        1 => outside(t),
        2 if t < 0.5 => inside(2. * t) / 2.,
        2 => 0.5 + outside(2. * t - 1.) / 2.,
        _ if t < 0.5 => outside(2. * t) / 2.,
        _ => 0.5 + inside(2. * t - 1.) / 2.,
    }
}

impl Instance {
    fn numeric(&self, id: ObjectId, name: &str) -> Option<f64> {
        match self.get(id, name).ok()? {
            Value::Number(n) if n.is_finite() => Some(n),
            _ => None,
        }
    }
    pub(crate) fn quick_kind(&self, id: ObjectId, kind: &str) -> bool {
        self.node(id)
            .is_some_and(|node| node.module == "QtQuick" && node.name == kind)
    }

    fn animation_matches(&self, driver: ObjectId, target: SlotId) -> bool {
        let property = &self.properties[target];
        if self
            .state_objects(driver, "exclude")
            .contains(&property.owner)
        {
            return false;
        }
        let mut targets = self.state_objects(driver, "targets");
        if let Ok(Value::Object(id)) = self.get(driver, "target") {
            targets.push(id)
        }
        if !targets.is_empty() && !targets.contains(&property.owner) {
            return false;
        }
        ["property", "properties"]
            .iter()
            .all(|name| match self.get(driver, name) {
                Ok(Value::String(s)) => {
                    s.is_empty() || s.split(',').any(|p| p.trim() == property.name)
                }
                _ => true,
            })
    }

    pub(crate) fn standalone_animation(&self, id: ObjectId) -> bool {
        self.is_animation(id)
            && !self.parent(id).is_some_and(|p| {
                self.quick_kind(p, "Transition")
                    || self.quick_kind(p, "Behavior")
                    || self.is_animation(p)
            })
    }
}

impl timers::Timers {
    /// Maintain the owner index on both normal updates and rollback. One driver
    /// or transition may control many properties, and a new driver can retarget
    /// the same slot while the previous animation is still running.
    fn set_track(&mut self, key: SlotId, next: Option<Track>) {
        let previous = match next {
            Some(track) => self.tracks.insert(key, track),
            None => self.tracks.remove(&key),
        };
        let old = previous.as_ref();
        let next = self.tracks.get(&key);
        self.ticking = self.ticking - usize::from(old.is_some_and(Track::ticking))
            + usize::from(next.is_some_and(Track::ticking));
        let owners = |t: Option<&Track>| [t.map(|t| t.driver), t.and_then(|t| t.transition)];
        let before = owners(old);
        let after = owners(next);
        for id in before.into_iter().flatten() {
            if !after.contains(&Some(id)) {
                let slots = self
                    .track_owners
                    .get_mut(&id)
                    .expect("indexed animation owner");
                slots.remove(&key);
                if slots.is_empty() {
                    self.track_owners.remove(&id);
                }
            }
        }
        for id in after.into_iter().flatten() {
            if !before.contains(&Some(id)) {
                self.track_owners.entry(id).or_default().insert(key);
            }
        }
    }
    fn driver_keys(&self, driver: ObjectId) -> impl Iterator<Item = SlotId> + '_ {
        self.track_owners
            .get(&driver)
            .into_iter()
            .flatten()
            .copied()
            .filter(move |key| self.tracks[key].driver == driver)
    }
}

impl Transaction<'_> {
    fn track(&mut self, key: SlotId, next: Option<Track>) {
        self.animations
            .before
            .entry(key)
            .or_insert_with(|| self.model.timers.tracks.get(&key).cloned());
        self.model.timers.set_track(key, next);
    }

    fn running(&mut self, id: ObjectId, flag: bool) {
        if let Ok(slot) = self.model.key(id, "running") {
            self.set_value_raw(slot, Value::Bool(flag));
        }
    }

    fn remove_track(&mut self, key: SlotId) {
        let Some(previous) = self.model.timers.tracks.get(&key).cloned() else {
            return;
        };
        self.track(key, None);
        for id in [Some(previous.driver), previous.transition]
            .into_iter()
            .flatten()
        {
            if !self.model.timers.track_owners.contains_key(&id) {
                self.running(id, false);
            }
        }
    }

    pub(crate) fn observe_animation(&mut self, key: SlotId) {
        let prop = &self.model.properties[key];
        if matches!(prop.name.as_str(), "running" | "paused")
            && self.model.standalone_animation(prop.owner)
        {
            self.animations.pending.insert(prop.owner);
        }
    }

    pub(crate) fn transition(
        &mut self,
        group: ObjectId,
        from: &str,
        to: &str,
        targets: &BTreeSet<SlotId>,
        reactive: &BTreeSet<SlotId>,
    ) {
        if self.initializing {
            return;
        }
        for key in targets {
            self.animations.transitions.remove(key);
            if self.model.timers.held.contains_key(key) {
                self.hold(*key, None);
            }
        }
        let matches = |list: &str, name: &str| {
            if list.split(',').any(|n| n.trim() == name) {
                Some(2)
            } else if list.split(',').any(|n| n.trim() == "*") {
                Some(1)
            } else {
                None
            }
        };
        let mut best = None;
        for tr in self.model.state_objects(group, "transitions") {
            if self.model.get(tr, "enabled") != Ok(Value::Bool(true)) {
                continue;
            }
            let (Ok(Value::String(a)), Ok(Value::String(b))) =
                (self.model.get(tr, "from"), self.model.get(tr, "to"))
            else {
                continue;
            };
            let score = matches(&a, from)
                .zip(matches(&b, to))
                .map(|(a, b)| 2 * (a + b) + 1);
            let reverse = if self.model.get(tr, "reversible") == Ok(Value::Bool(true)) {
                matches(&a, to)
                    .zip(matches(&b, from))
                    .map(|(a, b)| 2 * (a + b))
            } else {
                None
            };
            if let Some(score) = score.max(reverse) {
                if best.is_none_or(|(high, _)| score > high) {
                    best = Some((score, tr));
                }
            }
        }
        if let Some((score, tr)) = best {
            let reversed = score % 2 == 0;
            for driver in self.model.state_objects(tr, "animations") {
                if self.model.is_animation(driver) {
                    for &target in targets {
                        if self.model.accepts_channel(driver, target) {
                            self.animations
                                .transitions
                                .insert(target, (driver, tr, reversed));
                        }
                    }
                }
            }
            for &key in reactive {
                if !self.animations.transitions.contains_key(&key) {
                    self.hold(key, Some((tr, self.model.slots[key].value.clone())));
                }
            }
        }
    }

    fn hold(&mut self, key: SlotId, value: Option<(ObjectId, Value)>) {
        self.animations
            .held_before
            .entry(key)
            .or_insert_with(|| self.model.timers.held.get(&key).cloned());
        if let Some(value) = value {
            self.model.timers.held.insert(key, value);
        } else {
            self.model.timers.held.remove(&key);
        }
    }

    pub(crate) fn release_finished_values(&mut self) -> bool {
        if self.model.timers.held.is_empty() {
            return false;
        }
        let ready: Vec<_> = self
            .model
            .timers
            .held
            .iter()
            .filter(|(_, (transition, _))| {
                self.model.get(*transition, "running") != Ok(Value::Bool(true))
            })
            .map(|(&key, (_, value))| (key, value.clone()))
            .collect();
        let changed = !ready.is_empty();
        for (key, value) in ready {
            self.hold(key, None);
            self.set_value_raw(key, value);
        }
        changed
    }

    fn animate(
        &mut self,
        key: SlotId,
        driver: ObjectId,
        transition: Option<ObjectId>,
        reversed: bool,
        destination: &Value,
    ) -> bool {
        if self.model.timers.tracks.get(&key).is_some_and(|t| {
            t.driver == driver && t.transition == transition && t.requested.unchanged(destination)
        }) {
            return true;
        }
        let timeline = match self.model.timeline(driver, key, destination, reversed) {
            Ok(timeline) => Rc::new(timeline),
            Err(error) => {
                self.failed_host_call.get_or_insert(error);
                return true;
            }
        };
        let duration = timeline.duration;
        let loops = self.model.numeric(driver, "loops").unwrap_or(1.) as i32;
        self.remove_track(key);
        if loops == 0 {
            return true;
        }
        if duration.is_zero() {
            if let Some(value) = timeline.end.as_ref() {
                self.sample_value(key, value.clone());
            }
            return true;
        }
        let initial = timeline.sample(Duration::ZERO);
        let track = Track {
            driver,
            transition,
            start: self.model.timers.now,
            duration,
            loops,
            timeline,
            requested: destination.clone(),
            paused_at: (self.model.standalone_animation(driver)
                && self.model.get(driver, "paused") == Ok(Value::Bool(true)))
            .then_some(self.model.timers.now),
        };
        self.track(key, Some(track));
        self.running(driver, true);
        if let Some(tr) = transition {
            self.running(tr, true);
        }
        if let Some(value) = initial {
            self.sample_value(key, value);
        }
        true
    }

    fn sample_value(&mut self, key: SlotId, value: Value) {
        let value = match value {
            Value::Number(n) if self.model.properties[key].ty == "int" => Value::Number(n.round()),
            value => value,
        };
        self.set_value_raw(key, value);
    }

    pub(crate) fn intercept_animation(&mut self, key: SlotId, value: &Value) -> bool {
        if self.initializing {
            return false;
        }
        if let Some((transition, _)) = self.model.timers.held.get(&key) {
            self.hold(key, Some((*transition, value.clone())));
            return true;
        }
        if let Some(&(driver, tr, reversed)) = self.animations.transitions.get(&key) {
            return self.animate(key, driver, Some(tr), reversed, value);
        }
        if let Some(&behavior) = self.model.dynamic.behaviors.get(&key) {
            if self.model.get(behavior, "enabled") == Ok(Value::Bool(true)) {
                if let Ok(Value::Object(driver)) = self.model.get(behavior, "animation") {
                    if self.model.accepts_channel(driver, key) {
                        if self.model.slots[key].value.unchanged(value)
                            && !self.model.timers.tracks.contains_key(&key)
                        {
                            return false;
                        }
                        if let Ok(goal) = self.model.key(behavior, "targetValue") {
                            self.set_value_raw(goal, value.clone());
                        }
                        return self.animate(key, driver, None, false, value);
                    }
                }
            }
        }
        self.remove_track(key);
        false
    }

    pub(crate) fn stage_animation(&mut self) -> Result<bool, Error> {
        let Some(id) = self.animations.pending.pop_first() else {
            return Ok(false);
        };
        if self.initializing || self.model.node(id).is_none() {
            return Ok(true);
        }
        let running = self.model.get(id, "running")? == Value::Bool(true);
        let active: Vec<_> = self.model.timers.driver_keys(id).collect();
        if !running {
            for key in active {
                self.remove_track(key);
            }
            return Ok(true);
        }
        if !active.is_empty() {
            let now = self.model.timers.now;
            let paused = self.model.get(id, "paused")? == Value::Bool(true);
            for key in active {
                let mut track = self.model.timers.tracks[&key].clone();
                if paused && track.paused_at.is_none() {
                    track.paused_at = Some(now);
                    self.track(key, Some(track));
                } else if !paused {
                    if let Some(since) = track.paused_at.take() {
                        track.start = track.start.saturating_add(now.saturating_sub(since));
                        self.track(key, Some(track));
                    }
                }
            }
            return Ok(true);
        }
        let mut targets = BTreeSet::new();
        self.model.playback_targets(id, &mut targets)?;
        if targets.is_empty() {
            targets.insert(self.model.key(id, "$qml:clock")?);
        }
        for key in targets {
            let goal = self.model.slots[key].value.clone();
            self.animate(key, id, None, false, &goal);
        }
        if self.model.timers.driver_keys(id).next().is_none() {
            self.running(id, false);
        }
        Ok(true)
    }

    pub(crate) fn sample_animations(&mut self, now: Duration) -> Result<(), Error> {
        if self.model.timers.ticking == 0 {
            return Ok(());
        }
        let keys: Vec<_> = self.model.timers.tracks.keys().copied().collect();
        for key in keys {
            let Some(track) = self.model.timers.tracks.get(&key).cloned() else {
                continue;
            };
            if track.paused_at.is_some() {
                continue;
            }
            let elapsed = now.saturating_sub(track.start);
            let cycles = elapsed.as_nanos() / track.duration.as_nanos();
            if track.loops >= 0 && cycles >= track.loops as u128 {
                self.remove_track(key);
                if let Some(value) = track.timeline.end.as_ref() {
                    self.sample_value(key, value.clone());
                }
                if self.model.standalone_animation(track.driver)
                    && self.model.get(track.driver, "running")? == Value::Bool(false)
                {
                    self.dispatch(track.driver, "finished", vec![])?;
                }
            } else {
                let within = elapsed.as_nanos() % track.duration.as_nanos();
                if let Some(value) = track
                    .timeline
                    .sample(Duration::from_nanos(within.min(u128::from(u64::MAX)) as u64))
                {
                    self.sample_value(key, value);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn animation_method(
        &mut self,
        driver: ObjectId,
        name: &str,
        args: &[Value],
    ) -> Result<Value, Error> {
        if self.in_binding() || !args.is_empty() || !self.model.standalone_animation(driver) {
            return Err(Error::new(
                "animation controls require a standalone animation and no arguments",
            ));
        }
        let running = self.model.key(driver, "running")?;
        let paused = self.model.key(driver, "paused")?;
        if matches!(name, "restart" | "complete") {
            let tracks: Vec<_> = self
                .model
                .timers
                .driver_keys(driver)
                .map(|key| (key, self.model.timers.tracks[&key].timeline.end.clone()))
                .collect();
            for (key, goal) in tracks {
                self.remove_track(key);
                if name == "complete" {
                    if let Some(value) = goal {
                        self.sample_value(key, value);
                    }
                }
            }
        }
        match name {
            "start" | "restart" => {
                self.write(paused, Value::Bool(false))?;
                self.write(running, Value::Bool(true))?;
            }
            "stop" | "complete" => {
                self.write(running, Value::Bool(false))?;
                self.write(paused, Value::Bool(false))?;
            }
            "pause" | "resume" if self.model.get(driver, "running")? == Value::Bool(true) => {
                self.write(paused, Value::Bool(name == "pause"))?
            }
            "pause" | "resume" => (),
            _ => return Err(Error::new("unknown animation method")),
        }
        self.settle()?;
        Ok(Value::Undefined)
    }

    pub(crate) fn stop_dead_animations(&mut self, dead: &BTreeSet<ObjectId>) {
        let held: Vec<_> = self
            .model
            .timers
            .held
            .iter()
            .filter_map(|(&key, (tr, value))| {
                if dead.contains(tr) || dead.contains(&self.model.properties[key].owner) {
                    Some((key, None))
                } else {
                    lifetime::release_value(value, dead).map(|v| (key, Some((*tr, v))))
                }
            })
            .collect();
        for (key, next) in held {
            self.hold(key, next);
        }
        let keys: Vec<_> = self
            .model
            .timers
            .tracks
            .iter()
            .filter_map(|(&key, track)| {
                (dead.contains(&self.model.properties[key].owner)
                    || dead.contains(&track.driver)
                    || track.transition.is_some_and(|t| dead.contains(&t)))
                .then_some(key)
            })
            .collect();
        for key in keys {
            self.remove_track(key);
        }
    }
}

pub(crate) const METHODS: [&str; 6] = ["start", "stop", "restart", "pause", "resume", "complete"];
