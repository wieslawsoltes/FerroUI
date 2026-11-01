//! Timers run on a clock supplied by the native dispatcher or a test. Each
//! advance models one animation tick and coalesces overruns rather than firing
//! missed intervals in a burst. Timer state participates in transaction rollback.
use super::*;
use crate::transaction::Transaction;
use std::time::Duration;

#[derive(Default)]
pub(crate) struct Timers {
    pub(crate) held: BTreeMap<SlotId, (ObjectId, Value)>,
    pub(crate) tracks: BTreeMap<SlotId, animations::Track>,
    pub(super) track_owners: BTreeMap<ObjectId, BTreeSet<SlotId>>,
    pub(super) ticking: usize,
    pub(crate) now: Duration,
    due: BTreeSet<(Duration, ObjectId)>,
    deadlines: BTreeMap<ObjectId, Option<Duration>>,
}

impl Timers {
    fn set_deadline(&mut self, id: ObjectId, next: Option<Duration>) {
        if let Some(Some(old)) = self.deadlines.insert(id, next) {
            self.due.remove(&(old, id));
        }
        if let Some(time) = next {
            self.due.insert((time, id));
        }
    }
    pub(crate) fn remove(&mut self, id: ObjectId) {
        if let Some(Some(old)) = self.deadlines.remove(&id) {
            self.due.remove(&(old, id));
        }
    }
    pub(crate) fn truncate(&mut self, index: usize) {
        self.deadlines.retain(|id, _| id.index < index);
        self.due.retain(|(_, id)| id.index < index);
    }
    pub(crate) fn register(&mut self, object: ObjectId) {
        self.deadlines.insert(object, None);
    }
    pub(crate) fn objects(&self) -> impl Iterator<Item = ObjectId> + '_ {
        self.deadlines.keys().copied()
    }
    pub(crate) fn is_timer(&self, object: ObjectId) -> bool {
        self.deadlines.contains_key(&object)
    }
}

#[derive(Default)]
pub(crate) struct TimerChanges {
    pub(crate) pending: BTreeSet<ObjectId>,
    before: BTreeMap<ObjectId, Option<Duration>>,
    previous_time: Option<Duration>,
}

impl TimerChanges {
    pub(crate) fn rollback(self, timers: &mut Timers) {
        if let Some(now) = self.previous_time {
            timers.now = now;
        }
        for (id, old) in self.before {
            timers.set_deadline(id, old);
        }
    }
}

impl Instance {
    /// Elapsed time until the earliest active timer should be serviced.
    pub fn next_timer_deadline(&self) -> Option<Duration> {
        if !self.dynamic.loading.is_empty() {
            return Some(Duration::ZERO);
        }
        let animation = (self.timers.ticking != 0)
            .then(|| self.timers.now.saturating_add(Duration::from_millis(16)));
        self.timers
            .due
            .first()
            .map(|(deadline, _)| deadline)
            .into_iter()
            .chain(self.dynamic.deadlines.values())
            .chain(animation.iter())
            .min()
            .map(|deadline| deadline.saturating_sub(self.timers.now))
    }

    pub fn has_timers(&self) -> bool {
        self.timers.ticking != 0
            || !self.timers.deadlines.is_empty()
            || !self.dynamic.deadlines.is_empty()
            || !self.dynamic.loading.is_empty()
    }

    /// A deterministic dispatcher tick. A delayed host delivers at most one
    /// timeout per timer per tick. A failed handler restores the clock and timers.
    pub fn advance_time(&mut self, elapsed: Duration) -> Result<(), Error> {
        self.transaction(|tx| tx.tick(elapsed))
    }
}

impl Transaction<'_> {
    fn deadline(&mut self, object: ObjectId, next: Option<Duration>) {
        let old = self.model.timers.deadlines[&object];
        if old != next {
            self.timer_changes.before.entry(object).or_insert(old);
            self.model.timers.set_deadline(object, next);
        }
    }

    fn interval(&self, object: ObjectId) -> Result<Duration, Error> {
        let Value::Number(ms) = self.model.get(object, "interval")? else {
            return Err(Error::new("Timer.interval must be an int"));
        };
        // Zero interval schedules the next frame. Never busy-loop on a stalled clock.
        Ok(Duration::from_millis(if ms <= 0. { 16 } else { ms as u64 }))
    }

    pub(crate) fn schedule_timer(&mut self, object: ObjectId) -> Result<(), Error> {
        self.budget()?;
        if self.model.get(object, "running")? != Value::Bool(true) {
            self.deadline(object, None);
            return Ok(());
        }
        let starting = self.model.timers.deadlines[&object].is_none();
        let next = self.model.timers.now.saturating_add(self.interval(object)?);
        self.deadline(object, Some(next));
        if starting && self.model.get(object, "triggeredOnStart")? == Value::Bool(true) {
            self.signal(object, "triggered", Vec::new())?;
        }
        Ok(())
    }

    pub(crate) fn timer_method(
        &mut self,
        object: ObjectId,
        name: &str,
        args: &[Value],
    ) -> Result<Value, Error> {
        if !args.is_empty() {
            return Err(Error::new(format!("Timer.{name} takes no arguments")));
        }
        if self.in_binding() {
            return Err(Error::new("a binding cannot control a timer"));
        }
        let running = self.model.key(object, "running")?;
        let was_running = self.read(running, &mut BTreeSet::new())? == Value::Bool(true);
        match name {
            "restart" => {
                self.deadline(object, None);
                self.timer_changes.pending.insert(object);
                self.write(running, Value::Bool(true))?;
            }
            "start" if !was_running => self.write(running, Value::Bool(true))?,
            "stop" if was_running => self.write(running, Value::Bool(false))?,
            "start" | "stop" => (),
            _ => return Err(Error::new(format!("unknown Timer method '{name}'"))),
        }
        Ok(Value::Undefined)
    }

    fn tick(&mut self, elapsed: Duration) -> Result<(), Error> {
        self.timer_changes.previous_time = Some(self.model.timers.now);
        let now = self
            .model
            .timers
            .now
            .checked_add(elapsed)
            .ok_or_else(|| Error::new("component clock overflow"))?;
        self.model.timers.now = now;
        self.sample_animations(now)?;
        for id in std::mem::take(&mut self.model.dynamic.loading) {
            self.structure.loaded.insert(id);
            if self.model.node(id).is_some() {
                self.finish_factory(id)?;
            }
        }
        let pending_destroy: Vec<_> = self
            .model
            .dynamic
            .deadlines
            .iter()
            .filter_map(|(&id, &due)| (due <= now).then_some(id))
            .collect();
        let mut due: Vec<_> = self
            .model
            .timers
            .due
            .iter()
            .take_while(|(time, _)| *time <= now)
            .map(|(_, id)| *id)
            .collect();
        // Preserve registration/object order when one tick services several deadlines.
        due.sort_unstable();
        for id in due {
            // A previous handler can stop or restart this timer.
            if !self.model.timers.deadlines[&id].is_some_and(|deadline| deadline <= now) {
                continue;
            }
            if self.model.get(id, "repeat")? == Value::Bool(true) {
                self.deadline(id, Some(now.saturating_add(self.interval(id)?)));
            } else {
                self.deadline(id, None);
                self.write(self.model.key(id, "running")?, Value::Bool(false))?;
            }
            self.signal(id, "triggered", Vec::new())?;
            self.settle()?;
        }
        for id in pending_destroy {
            if self.model.node(id).is_some() {
                self.destroy_object(id)?;
            }
        }
        Ok(())
    }
}
