//! The animation clock of the media context.

use super::media_context::MediaContext;
use crate::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use crate::reactive::{Disposable, IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

type AnimationFrame = Box<dyn FnOnce(TimeSpan)>;

/// The global animation clock: pulsed once per frame by the media context
/// with the time elapsed since the media context started.
///
/// Subscribing schedules a render pass, so that the clock keeps ticking
/// while anything observes it.
pub struct MediaContextClock {
    this: Weak<MediaContextClock>,
    parent: Weak<MediaContext>,
    observers: RefCell<Vec<Rc<dyn IObserver<TimeSpan>>>>,
    new_observers: RefCell<Vec<Rc<dyn IObserver<TimeSpan>>>>,
    queued_animation_frames: RefCell<VecDeque<AnimationFrame>>,
    current_animation_timestamp: Cell<TimeSpan>,
}

impl MediaContextClock {
    pub(crate) fn new(parent: Weak<MediaContext>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            parent,
            observers: RefCell::new(Vec::new()),
            new_observers: RefCell::new(Vec::new()),
            queued_animation_frames: RefCell::new(VecDeque::new()),
            current_animation_timestamp: Cell::new(TimeSpan::ZERO),
        })
    }

    /// Whether observers subscribed since the last pulse.
    pub fn has_new_subscriptions(&self) -> bool {
        !self.new_observers.borrow().is_empty()
    }

    /// Whether anything is waiting for the next pulse.
    pub fn has_subscriptions(&self) -> bool {
        !self.observers.borrow().is_empty() || !self.queued_animation_frames.borrow().is_empty()
    }

    /// Runs `action` with the frame time on the next pulse.
    pub fn request_animation_frame(&self, action: impl FnOnce(TimeSpan) + 'static) {
        if let Some(parent) = self.parent.upgrade() {
            parent.schedule_render(false);
        }
        self.queued_animation_frames.borrow_mut().push_back(Box::new(action));
    }

    pub(crate) fn pulse(&self, now: TimeSpan) {
        self.new_observers.borrow_mut().clear();
        self.current_animation_timestamp.set(now);

        // The queue is taken before it is run: frames requested by a
        // callback are for the next pulse.
        let animation_frames = std::mem::take(&mut *self.queued_animation_frames.borrow_mut());
        for callback in animation_frames {
            callback(now);
        }

        let observers = self.observers.borrow().clone();
        for observer in observers {
            observer.on_next(self.current_animation_timestamp.get());
        }
    }

    pub(crate) fn pulse_new_subscriptions(&self) {
        let new_observers = self.new_observers.borrow().clone();
        for observer in new_observers {
            observer.on_next(self.current_animation_timestamp.get());
        }
        self.new_observers.borrow_mut().clear();
    }
}

impl IObservable<TimeSpan> for MediaContextClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        if let Some(parent) = self.parent.upgrade() {
            parent.schedule_render(false);
            parent.verify_access();
        }
        self.observers.borrow_mut().push(observer.clone());
        self.new_observers.borrow_mut().push(observer.clone());
        let weak = self.this.clone();
        Disposable::create(move || {
            let Some(this) = weak.upgrade() else { return };
            if let Some(parent) = this.parent.upgrade() {
                parent.verify_access();
            }
            let mut observers = this.observers.borrow_mut();
            if let Some(index) = observers.iter().position(|o| Rc::ptr_eq(o, &observer)) {
                observers.remove(index);
            }
        })
    }
}

impl IClock for MediaContextClock {
    fn play_state(&self) -> PlayState {
        PlayState::Run
    }

    fn set_play_state(&self, _value: PlayState) {
        panic!("The play state of the global clock cannot be changed.");
    }
}

impl IGlobalClock for MediaContextClock {}
