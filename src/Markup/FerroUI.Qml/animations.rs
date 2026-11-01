//! Numeric property animation samples the same owner-thread clock as QML Timer.
//! Sample writes leave the target binding intact, allowing mid-flight retargets.
use super::*;
use crate::transaction::Transaction;
use std::time::Duration;

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
        "NumberAnimation",
        object()
            .property("target", "QtObject", Value::Null)
            .property("targets", "list<QtObject>", Value::List(vec![]))
            .property("property", "string", Value::String(String::new()))
            .property("properties", "string", Value::String(String::new()))
            .property("from", "real", Value::Undefined)
            .property("to", "real", Value::Undefined)
            .property("duration", "int", Value::Number(250.))
            .property("loops", "int", Value::Number(1.))
            .property("paused", "bool", Value::Bool(false))
            .property("running", "bool", Value::Bool(false))
            .property("easing.type", "int", Value::Number(0.))
            .signal("finished", vec![]),
    )?;
    registry.register(
        "QtQuick",
        "Animation",
        object().enumeration("Loops", &[("Infinite", -1)])?,
    )?;
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
    easing: u8,
    from: f64,
    to: f64,
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
    transitions: BTreeMap<SlotId, (ObjectId, ObjectId)>,
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
            if let Some(track) = before {
                model.timers.tracks.insert(key, track);
            } else {
                model.timers.tracks.remove(&key);
            }
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
        self.quick_kind(id, "NumberAnimation")
            && !self
                .parent(id)
                .is_some_and(|p| self.quick_kind(p, "Transition") || self.quick_kind(p, "Behavior"))
    }
}

impl Transaction<'_> {
    fn track(&mut self, key: SlotId, next: Option<Track>) {
        self.animations
            .before
            .entry(key)
            .or_insert_with(|| self.model.timers.tracks.get(&key).cloned());
        if let Some(track) = next {
            self.model.timers.tracks.insert(key, track);
        } else {
            self.model.timers.tracks.remove(&key);
        }
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
            if !self
                .model
                .timers
                .tracks
                .values()
                .any(|track| track.driver == id || track.transition == Some(id))
            {
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
        if let Some((_, tr)) = best {
            for driver in self.model.state_objects(tr, "animations") {
                if self.model.quick_kind(driver, "NumberAnimation") {
                    for &target in targets {
                        if self.model.animation_matches(driver, target) {
                            self.animations.transitions.insert(target, (driver, tr));
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
        destination: f64,
    ) -> bool {
        let Value::Number(current) = self.model.slots[key].value else {
            return false;
        };
        let from = self.model.numeric(driver, "from").unwrap_or(current);
        let to = self.model.numeric(driver, "to").unwrap_or(destination);
        let ms = self
            .model
            .numeric(driver, "duration")
            .unwrap_or(250.)
            .clamp(0., i32::MAX as f64) as u64;
        let loops = self.model.numeric(driver, "loops").unwrap_or(1.) as i32;
        if from == to || ms == 0 || loops == 0 {
            self.remove_track(key);
            return false;
        }
        if self
            .model
            .timers
            .tracks
            .get(&key)
            .is_some_and(|t| t.driver == driver && t.transition == transition && t.to == to)
        {
            return true;
        }
        self.remove_track(key);
        let track = Track {
            driver,
            transition,
            start: self.model.timers.now,
            duration: Duration::from_millis(ms),
            paused_at: None,
            loops,
            easing: self.model.numeric(driver, "easing.type").unwrap_or(0.) as u8,
            from,
            to,
        };
        self.track(key, Some(track));
        self.running(driver, true);
        if let Some(tr) = transition {
            self.running(tr, true);
        }
        if current != from {
            self.sample_number(key, from);
        }
        true
    }

    fn sample_number(&mut self, key: SlotId, value: f64) {
        let value = if self.model.properties[key].ty == "int" {
            value.round()
        } else {
            value
        };
        self.set_value_raw(key, Value::Number(value));
    }

    pub(crate) fn intercept_animation(&mut self, key: SlotId, value: &Value) -> bool {
        if self.initializing {
            return false;
        }
        if let Some((transition, _)) = self.model.timers.held.get(&key) {
            self.hold(key, Some((*transition, value.clone())));
            return true;
        }
        let Value::Number(to) = *value else {
            return false;
        };
        if let Some(&(driver, tr)) = self.animations.transitions.get(&key) {
            return self.animate(key, driver, Some(tr), to);
        }
        if let Some(&behavior) = self.model.dynamic.behaviors.get(&key) {
            if self.model.get(behavior, "enabled") == Ok(Value::Bool(true)) {
                if let Ok(Value::Object(driver)) = self.model.get(behavior, "animation") {
                    if self.model.quick_kind(driver, "NumberAnimation") {
                        if let Ok(goal) = self.model.key(behavior, "targetValue") {
                            self.set_value_raw(goal, value.clone());
                        }
                        return self.animate(key, driver, None, to);
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
        let active: Vec<_> = self
            .model
            .timers
            .tracks
            .iter()
            .filter_map(|(&key, t)| (t.driver == id).then_some(key))
            .collect();
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
        let mut targets = self.model.state_objects(id, "targets");
        if let Value::Object(t) = self.model.get(id, "target")? {
            targets.push(t);
        }
        let mut props = BTreeSet::new();
        for name in ["properties", "property"] {
            if let Value::String(list) = self.model.get(id, name)? {
                props.extend(
                    list.split(',')
                        .map(str::trim)
                        .filter(|p| !p.is_empty())
                        .map(str::to_owned),
                );
            }
        }
        for target in targets {
            for prop in &props {
                let key = self.model.write_target(self.model.key(target, prop)?)?;
                let to = self
                    .model
                    .numeric(id, "to")
                    .or_else(|| self.model.numeric(target, prop))
                    .ok_or_else(|| Error::new("NumberAnimation requires a numeric property"))?;
                if !self.animate(key, id, None, to) {
                    self.sample_number(key, to);
                }
            }
        }
        if !self.model.timers.tracks.values().any(|t| t.driver == id) {
            self.running(id, false);
        }
        Ok(true)
    }

    pub(crate) fn sample_animations(&mut self, now: Duration) -> Result<(), Error> {
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
                self.sample_number(key, track.to);
                if self.model.standalone_animation(track.driver)
                    && self.model.get(track.driver, "running")? == Value::Bool(false)
                {
                    self.dispatch(track.driver, "finished", vec![])?;
                }
            } else {
                let t = (elapsed.as_nanos() % track.duration.as_nanos()) as f64
                    / track.duration.as_nanos() as f64;
                self.sample_number(
                    key,
                    track.from + (track.to - track.from) * easing(track.easing, t),
                );
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
                "animation controls require a standalone NumberAnimation and no arguments",
            ));
        }
        let running = self.model.key(driver, "running")?;
        let paused = self.model.key(driver, "paused")?;
        if matches!(name, "restart" | "complete") {
            let tracks: Vec<_> = self
                .model
                .timers
                .tracks
                .iter()
                .filter_map(|(&key, t)| (t.driver == driver).then_some((key, t.to)))
                .collect();
            for (key, goal) in tracks {
                self.remove_track(key);
                if name == "complete" {
                    self.sample_number(key, goal);
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
