use crate::styling::{Setter, SetterValue};
use crate::FerroProperty;

/// A setter that can be used in the key frames of an animation.
pub trait IAnimationSetter: 'static {
    /// The property the setter targets.
    fn property(&self) -> Option<&'static FerroProperty>;

    fn set_property(&self, value: Option<&'static FerroProperty>);

    /// The value of the setter: a plain value holding exactly the value type
    /// of the property, or a source of such values.
    fn value(&self) -> Option<SetterValue>;

    fn set_value(&self, value: Option<SetterValue>);

    /// The implementing value, for casting to its concrete type
    /// (`as_any()?.downcast_ref::<T>()`): the equivalent of `is`/`as` on the
    /// contract. `None` for an implementation that does not expose itself.
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        None
    }
}

impl IAnimationSetter for Setter {
    fn property(&self) -> Option<&'static FerroProperty> {
        Setter::property(self)
    }

    fn set_property(&self, value: Option<&'static FerroProperty>) {
        Setter::set_property(self, value)
    }

    fn value(&self) -> Option<SetterValue> {
        Setter::value(self)
    }

    fn set_value(&self, value: Option<SetterValue>) {
        Setter::set_value(self, value)
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

/// Handles compare by identity (reference equality), so that they can be
/// held in property and untyped values.
impl PartialEq for dyn IAnimationSetter {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
