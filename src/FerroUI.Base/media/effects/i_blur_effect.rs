use crate::media::effects::{BlurEffect, Effect, IEffect, IImmutableEffect};
use crate::media::ref_adapter::RefAdapter;
use crate::{ObjectType, Upcast};
use std::any::Any;
use std::rc::Rc;

/// An effect that blurs its content.
pub trait IBlurEffect: IEffect {
    /// The blur radius.
    fn radius(&self) -> f64;
}

/// An immutable blur effect.
#[derive(Clone, Copy, Debug)]
pub struct ImmutableBlurEffect {
    radius: f64,
}

impl ImmutableBlurEffect {
    pub fn new(radius: f64) -> Self {
        Self { radius }
    }

    /// The blur radius.
    #[inline]
    pub fn radius(&self) -> f64 {
        self.radius
    }
}

impl IEffect for ImmutableBlurEffect {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_blur_effect(&self) -> Option<&dyn IBlurEffect> {
        Some(self)
    }

    fn as_immutable_effect(&self) -> Option<&dyn IImmutableEffect> {
        Some(self)
    }

    fn into_immutable_effect(self: Rc<Self>) -> Option<std::sync::Arc<dyn IImmutableEffect>> {
        Some(std::sync::Arc::new(*self))
    }
}

impl IBlurEffect for ImmutableBlurEffect {
    #[inline]
    fn radius(&self) -> f64 {
        self.radius
    }
}

impl IImmutableEffect for ImmutableBlurEffect {
    fn equals(&self, other: Option<&dyn IEffect>) -> bool {
        other.and_then(|other| other.as_blur_effect()).is_some_and(|blur| blur.radius() == self.radius)
    }
}

impl<T: ObjectType + Upcast<Effect>> IBlurEffect for RefAdapter<T> {
    #[inline]
    fn radius(&self) -> f64 {
        self.class::<BlurEffect>().radius()
    }
}
