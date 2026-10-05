use crate::animation::{Animatable, Animation, AnimationTask, IClock};
use crate::reactive::{IDisposable, IObservable};
use crate::threading::CancellationToken;
use crate::Ref;
use std::rc::Rc;

/// Interface for animation objects.
pub trait IAnimation: 'static {
    /// Applies the animation to the specified control: it runs while
    /// `match_` is true. `on_complete` runs when it ends by itself.
    /// `is_manually_started` tells an animation started by code from one
    /// started by a style. Disposing the result removes the animation.
    fn apply(
        &self,
        control: &Animatable,
        clock: Option<Rc<dyn IClock>>,
        match_: Rc<dyn IObservable<bool>>,
        on_complete: Option<Rc<dyn Fn()>>,
        is_manually_started: bool,
    ) -> Rc<dyn IDisposable>;

    /// Runs the animation on the specified control. The result completes
    /// when the animation has ended or was cancelled.
    fn run_async(
        &self,
        control: &Animatable,
        clock: Option<Rc<dyn IClock>>,
        cancellation_token: CancellationToken,
    ) -> AnimationTask;

    /// The identity of the animation, used for reference equality.
    #[doc(hidden)]
    fn reference_id(&self) -> *const () {
        self as *const Self as *const ()
    }

    /// The object of the object model that implements the contract, if one
    /// does: the equivalent of `is`/`as` on the interface. Cast the object to
    /// the class looked for (`as_object()?.cast::<T>()`).
    fn as_object(&self) -> Option<crate::Ref<crate::FerroObject>> {
        None
    }
}

/// Handles compare by identity (reference equality): two handles are equal
/// when they refer to the same animation, whichever adapter they were made
/// from.
impl PartialEq for dyn IAnimation {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.reference_id() == other.reference_id()
    }
}

struct AnimationRef(Ref<Animation>);

impl IAnimation for AnimationRef {
    fn apply(
        &self,
        control: &Animatable,
        clock: Option<Rc<dyn IClock>>,
        match_: Rc<dyn IObservable<bool>>,
        on_complete: Option<Rc<dyn Fn()>>,
        is_manually_started: bool,
    ) -> Rc<dyn IDisposable> {
        self.0.apply(control, clock, match_, on_complete, is_manually_started)
    }

    fn run_async(
        &self,
        control: &Animatable,
        clock: Option<Rc<dyn IClock>>,
        cancellation_token: CancellationToken,
    ) -> AnimationTask {
        self.0.run_async_with_clock(control, clock, cancellation_token)
    }

    fn reference_id(&self) -> *const () {
        &*self.0 as *const Animation as *const ()
    }

    fn as_object(&self) -> Option<Ref<crate::FerroObject>> {
        Some(self.0.clone().upcast())
    }
}

impl From<Ref<Animation>> for Rc<dyn IAnimation> {
    fn from(value: Ref<Animation>) -> Self {
        Rc::new(AnimationRef(value))
    }
}

impl From<&Ref<Animation>> for Rc<dyn IAnimation> {
    fn from(value: &Ref<Animation>) -> Self {
        Rc::new(AnimationRef(value.clone()))
    }
}
