use crate::media::effects::{
    BlurEffect, DropShadowDirectionEffect, DropShadowEffect, Effect, IBlurEffect, IDirectionDropShadowEffect,
    IDropShadowEffect,
};
use crate::media::ref_adapter::RefAdapter;
use crate::{FerroObject, ObjectType, Ref, Upcast};
use std::any::Any;
use std::rc::Rc;

/// A bitmap effect.
///
/// Implemented by the immutable effect types and, through an adapter, by
/// every class deriving from [`Effect`]: a handle of such a class converts to
/// `Rc<dyn IEffect>` with `into()`. The `as_*` members replace interface
/// casts (`effect as IBlurEffect`).
pub trait IEffect: 'static {
    /// The implementing value, for downcasts to immutable effect types. For
    /// mutable effects use [`as_object`](Self::as_object).
    fn as_any(&self) -> &dyn Any;

    /// The object behind the effect when it is a mutable [`Effect`].
    fn as_object(&self) -> Option<&FerroObject> {
        None
    }

    /// The effect viewed as [`IBlurEffect`], when it is one.
    fn as_blur_effect(&self) -> Option<&dyn IBlurEffect> {
        None
    }

    /// The effect viewed as [`IDropShadowEffect`], when it is one.
    fn as_drop_shadow_effect(&self) -> Option<&dyn IDropShadowEffect> {
        None
    }

    /// The effect viewed as [`IDirectionDropShadowEffect`], when it is one.
    fn as_direction_drop_shadow_effect(&self) -> Option<&dyn IDirectionDropShadowEffect> {
        None
    }

    /// The effect viewed as [`IMutableEffect`], when it is one.
    fn as_mutable_effect(&self) -> Option<&dyn IMutableEffect> {
        None
    }

    /// The effect viewed as [`IImmutableEffect`], when it is one.
    fn as_immutable_effect(&self) -> Option<&dyn IImmutableEffect> {
        None
    }

    /// The effect as an [`IImmutableEffect`] handle, when it is immutable.
    fn into_immutable_effect(self: Rc<Self>) -> Option<Rc<dyn IImmutableEffect>> {
        None
    }

    /// The identity of the effect, used for reference equality.
    #[doc(hidden)]
    fn reference_id(&self) -> *const () {
        self as *const Self as *const ()
    }
}

/// Effects compare by reference; structural comparison of immutable effects
/// is [`IImmutableEffect::equals`].
impl PartialEq for dyn IEffect {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.reference_id() == other.reference_id()
    }
}

impl std::fmt::Debug for dyn IEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(blur) = self.as_blur_effect() {
            write!(f, "IEffect(blur({}))", blur.radius())
        } else if let Some(shadow) = self.as_drop_shadow_effect() {
            write!(
                f,
                "IEffect(drop-shadow({} {} {} {}))",
                shadow.offset_x(),
                shadow.offset_y(),
                shadow.blur_radius(),
                shadow.color()
            )
        } else {
            f.write_str("IEffect")
        }
    }
}

/// A mutable effect which can return an immutable clone of itself.
pub trait IMutableEffect: IEffect {
    /// Creates an immutable clone of the effect.
    fn to_immutable(&self) -> Rc<dyn IImmutableEffect>;
}

/// An immutable effect, which compares structurally.
pub trait IImmutableEffect: IEffect {
    /// Whether the effect has the same parameters as `other`.
    fn equals(&self, other: Option<&dyn IEffect>) -> bool;
}

impl<T: ObjectType + Upcast<Effect>> IEffect for RefAdapter<T> {
    fn as_any(&self) -> &dyn Any {
        Upcast::<Effect>::upcast(&*self.0)
    }

    fn as_object(&self) -> Option<&FerroObject> {
        Some(self.object())
    }

    fn as_blur_effect(&self) -> Option<&dyn IBlurEffect> {
        if self.object().is::<BlurEffect>() {
            Some(self)
        } else {
            None
        }
    }

    fn as_drop_shadow_effect(&self) -> Option<&dyn IDropShadowEffect> {
        if self.object().is::<DropShadowEffect>() || self.object().is::<DropShadowDirectionEffect>() {
            Some(self)
        } else {
            None
        }
    }

    fn as_direction_drop_shadow_effect(&self) -> Option<&dyn IDirectionDropShadowEffect> {
        if self.object().is::<DropShadowDirectionEffect>() {
            Some(self)
        } else {
            None
        }
    }

    fn as_mutable_effect(&self) -> Option<&dyn IMutableEffect> {
        if self.object().is::<BlurEffect>()
            || self.object().is::<DropShadowEffect>()
            || self.object().is::<DropShadowDirectionEffect>()
        {
            Some(self)
        } else {
            None
        }
    }

    fn reference_id(&self) -> *const () {
        RefAdapter::reference_id(self)
    }
}

impl<T: ObjectType + Upcast<Effect>> IMutableEffect for RefAdapter<T> {
    fn to_immutable(&self) -> Rc<dyn IImmutableEffect> {
        let object = self.object();
        if let Some(blur) = object.downcast_ref::<BlurEffect>() {
            blur.to_immutable()
        } else if let Some(shadow) = object.downcast_ref::<DropShadowEffect>() {
            shadow.to_immutable()
        } else {
            self.class::<DropShadowDirectionEffect>().to_immutable()
        }
    }
}

impl<T: ObjectType + Upcast<Effect>> From<Ref<T>> for Rc<dyn IEffect> {
    #[inline]
    fn from(value: Ref<T>) -> Self {
        Rc::new(RefAdapter(value))
    }
}

impl<T: ObjectType + Upcast<Effect>> From<&Ref<T>> for Rc<dyn IEffect> {
    #[inline]
    fn from(value: &Ref<T>) -> Self {
        Rc::new(RefAdapter(value.clone()))
    }
}
