use super::{IServerClockItem, ServerObjectAnimations};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

/// The scaling of the clock of the animations by the global playback rate
/// (WinUI `Compositor.GlobalPlaybackRate`): from the compositor time
/// `base_real` on, the animation time runs at `rate` from `base_scaled`
/// (seconds).
#[derive(Clone, Copy, Debug)]
struct AnimationTimeScale {
    rate: f64,
    base_real: Duration,
    base_scaled: f64,
}

/// The clock items and the dirty animated objects of a server compositor.
#[derive(Default)]
pub struct ServerCompositorAnimations {
    clock_items: RefCell<Vec<Rc<dyn IServerClockItem>>>,
    clock_items_to_update: RefCell<Vec<Rc<dyn IServerClockItem>>>,
    dirty_animated_object_queue: RefCell<VecDeque<Rc<ServerObjectAnimations>>>,
    /// `None` until the global playback rate is set: the animation time is
    /// the compositor time.
    time_scale: Cell<Option<AnimationTimeScale>>,
}

fn same_item(a: &Rc<dyn IServerClockItem>, b: &Rc<dyn IServerClockItem>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

impl ServerCompositorAnimations {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_to_clock(&self, item: Rc<dyn IServerClockItem>) {
        let mut items = self.clock_items.borrow_mut();
        if !items.iter().any(|i| same_item(i, &item)) {
            items.push(item);
        }
    }

    pub fn remove_from_clock(&self, item: &Rc<dyn IServerClockItem>) {
        self.clock_items.borrow_mut().retain(|i| !same_item(i, item));
    }

    pub fn process(&self) {
        let mut to_update = std::mem::take(&mut *self.clock_items_to_update.borrow_mut());
        to_update.extend(self.clock_items.borrow().iter().cloned());
        for animation in &to_update {
            animation.on_tick();
        }
        to_update.clear();
        *self.clock_items_to_update.borrow_mut() = to_update;

        loop {
            let Some(animation) = self.dirty_animated_object_queue.borrow_mut().pop_front() else { break };
            animation.evaluate_animations();
        }
    }

    pub fn need_next_tick(&self) -> bool {
        !self.clock_items.borrow().is_empty()
    }

    /// The time of the animations at the compositor time `real`: the
    /// compositor time, scaled by the global playback rate from the moment
    /// the rate was set. Animations are started and evaluated at this time,
    /// so a rate of 0.5 runs every animation at half speed.
    pub fn animation_time(&self, real: Duration) -> Duration {
        let Some(scale) = self.time_scale.get() else { return real };
        let scaled = scale.base_scaled + (real.as_secs_f64() - scale.base_real.as_secs_f64()) * scale.rate;
        Duration::from_secs_f64(if scaled.is_finite() { scaled.max(0.0) } else { 0.0 })
    }

    /// The global playback rate (1 until it is set).
    pub fn global_playback_rate(&self) -> f64 {
        self.time_scale.get().map_or(1.0, |scale| scale.rate)
    }

    /// Sets the global playback rate from the compositor time `real_now` on:
    /// the animation time goes on from where it is at that moment.
    pub fn set_global_playback_rate(&self, rate: f64, real_now: Duration) {
        let rate = if rate.is_finite() { rate.max(0.0) } else { 1.0 };
        let base_scaled = self.animation_time(real_now).as_secs_f64();
        self.time_scale.set(Some(AnimationTimeScale { rate, base_real: real_now, base_scaled }));
    }

    pub fn add_dirty_animated_object(&self, obj: Rc<ServerObjectAnimations>) {
        let mut queue = self.dirty_animated_object_queue.borrow_mut();
        if !queue.iter().any(|o| Rc::ptr_eq(o, &obj)) {
            queue.push_back(obj);
        }
    }
}
