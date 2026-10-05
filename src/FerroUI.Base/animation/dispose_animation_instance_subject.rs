use crate::animation::animators::{run, Animator};
use crate::animation::{Animatable, Animation, IClock};
use crate::reactive::{IDisposable, IObserver, ObservableError};
use crate::{Ref, Visual, WeakRef};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Manages the lifetime of animation instances as determined by the state
/// of the match observable: runs the animation when the match becomes true
/// and disposes it when the match becomes false.
pub(crate) struct DisposeAnimationInstanceSubject<A: Animator> {
    animator: Rc<A>,
    animation: Ref<Animation>,
    control: WeakRef<Animatable>,
    clock: Option<Rc<dyn IClock>>,
    on_complete: Option<Rc<dyn Fn()>>,
    should_pause_on_invisible: bool,
    visual_target: Option<WeakRef<Visual>>,
    last_instance: RefCell<Option<Rc<dyn IDisposable>>>,
    last_match: Cell<bool>,
}

impl<A: Animator> DisposeAnimationInstanceSubject<A> {
    pub(crate) fn new(
        animator: Rc<A>,
        animation: Ref<Animation>,
        control: Ref<Animatable>,
        clock: Option<Rc<dyn IClock>>,
        on_complete: Option<Rc<dyn Fn()>>,
        should_pause_on_invisible: bool,
        visual_target: Option<Ref<Visual>>,
    ) -> Self {
        Self {
            animator,
            animation,
            // The control owns the style instance or subscription that owns
            // this subject: holding it strongly would keep it alive forever.
            control: control.downgrade(),
            clock,
            on_complete,
            should_pause_on_invisible,
            visual_target: visual_target.as_ref().map(Ref::downgrade),
            last_instance: RefCell::new(None),
            last_match: Cell::new(false),
        }
    }

    fn dispose_last_instance(&self) {
        let last_instance = self.last_instance.borrow_mut().take();
        if let Some(last_instance) = last_instance {
            last_instance.dispose();
        }
    }
}

impl<A: Animator> IDisposable for DisposeAnimationInstanceSubject<A> {
    fn dispose(&self) {
        let last_instance = self.last_instance.borrow().clone();
        if let Some(last_instance) = last_instance {
            last_instance.dispose();
        }
    }
}

impl<A: Animator> IObserver<bool> for DisposeAnimationInstanceSubject<A> {
    fn on_completed(&self) {}

    fn on_error(&self, _error: ObservableError) {
        self.dispose_last_instance();
    }

    fn on_next(&self, match_val: bool) {
        if match_val != self.last_match.get() {
            self.dispose_last_instance();

            if match_val {
                if let Some(control) = self.control.upgrade() {
                    let visual_target = self.visual_target.as_ref().and_then(WeakRef::upgrade);
                    let instance = run(
                        &self.animator,
                        &self.animation,
                        &control,
                        self.clock.clone(),
                        self.on_complete.clone(),
                        self.should_pause_on_invisible,
                        visual_target.as_ref(),
                    );
                    *self.last_instance.borrow_mut() = Some(instance);
                }
            }

            self.last_match.set(match_val);
        }
    }
}
