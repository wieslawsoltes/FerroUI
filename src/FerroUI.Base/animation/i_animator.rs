use crate::animation::{Animatable, Animation, AnimatorKeyFrame, IClock};
use crate::reactive::{IDisposable, IObservable};
use crate::{FerroProperty, Ref};
use std::any::TypeId;
use std::rc::Rc;

/// Interface for animator objects: the list of the key frames of one
/// property of an animation, and the logic that plays them.
pub trait IAnimator: 'static {
    /// The target property.
    fn property(&self) -> Option<&'static FerroProperty>;

    fn set_property(&self, value: Option<&'static FerroProperty>);

    /// The number of key frames.
    fn count(&self) -> usize;

    /// The key frame at `index`. Panics when out of range.
    fn get(&self, index: usize) -> Ref<AnimatorKeyFrame>;

    /// Adds a key frame. Key frames are expected in cue order.
    fn add(&self, item: Ref<AnimatorKeyFrame>);

    /// The type of the animator, by which key frames find their animator.
    fn animator_type(&self) -> TypeId;

    /// Applies the animation to a control: the animation runs while
    /// `match_` is true.
    fn apply(
        self: Rc<Self>,
        animation: &Animation,
        control: &Animatable,
        clock: Option<Rc<dyn IClock>>,
        match_: Rc<dyn IObservable<bool>>,
        on_complete: Option<Rc<dyn Fn()>>,
        should_pause_on_invisible: bool,
    ) -> Option<Rc<dyn IDisposable>>;
}
