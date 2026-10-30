use crate::motion::Animatable;
use std::{fmt, rc::Rc};

/// The imperative side of a state. A definition is shared between targets;
/// per-target values belong on the supplied animatable, not on shared paints.
pub trait IStateBehavior {
    fn on_state_applied(&self, target: &mut Animatable, now_ms: f64);
    fn on_state_removed(&self, target: &mut Animatable, now_ms: f64);
}

pub type StateCallback = Rc<dyn Fn(&mut Animatable, f64)>;

/// Inline state behavior. Omitted actions are no-ops.
#[derive(Clone, Default)]
pub struct StateBehavior {
    pub on_applied: Option<StateCallback>,
    pub on_removed: Option<StateCallback>,
}
impl StateBehavior {
    pub fn new(on_applied: Option<StateCallback>, on_removed: Option<StateCallback>) -> Self {
        Self {
            on_applied,
            on_removed,
        }
    }
    pub fn applied(mut self, callback: impl Fn(&mut Animatable, f64) + 'static) -> Self {
        self.on_applied = Some(Rc::new(callback));
        self
    }
    pub fn removed(mut self, callback: impl Fn(&mut Animatable, f64) + 'static) -> Self {
        self.on_removed = Some(Rc::new(callback));
        self
    }
}
impl fmt::Debug for StateBehavior {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StateBehavior")
            .field("on_applied", &self.on_applied.is_some())
            .field("on_removed", &self.on_removed.is_some())
            .finish()
    }
}
impl IStateBehavior for StateBehavior {
    fn on_state_applied(&self, target: &mut Animatable, now_ms: f64) {
        if let Some(callback) = &self.on_applied {
            callback(target, now_ms);
        }
    }
    fn on_state_removed(&self, target: &mut Animatable, now_ms: f64) {
        if let Some(callback) = &self.on_removed {
            callback(target, now_ms);
        }
    }
}
