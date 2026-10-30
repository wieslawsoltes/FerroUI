use super::{IEffect, IImmutableEffect};
use std::any::Any;
use std::rc::Rc;
use std::sync::Arc;

/// Converts rendered color to a luminance mask. The output is black with alpha
/// `0.2126 R + 0.7152 G + 0.0722 B`, using premultiplied color channels. Thus
/// transparent pixels stay transparent and existing alpha is consumed once.
/// The conversion does not expand the content bounds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImmutableLuminanceToAlphaEffect;

impl IEffect for ImmutableLuminanceToAlphaEffect {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_immutable_effect(&self) -> Option<&dyn IImmutableEffect> {
        Some(self)
    }
    fn into_immutable_effect(self: Rc<Self>) -> Option<Arc<dyn IImmutableEffect>> {
        Some(Arc::new(*self))
    }
}

impl IImmutableEffect for ImmutableLuminanceToAlphaEffect {
    fn equals(&self, other: Option<&dyn IEffect>) -> bool {
        other.is_some_and(|effect| effect.as_any().is::<Self>())
    }
}
