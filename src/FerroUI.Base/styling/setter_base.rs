use super::{ISetterInstance, StyleInstance};
use crate::property_store::IValueEntry;
use crate::StyledElement;
use std::rc::Rc;

/// Represents the base class for value setters.
pub trait SetterBase: 'static {
    /// Instances a setter on a control.
    #[doc(hidden)]
    fn instance(self: Rc<Self>, style_instance: &Rc<StyleInstance>, target: &StyledElement) -> SetterInstance;

    /// The implementing value, for casting to its concrete type
    /// (`as_any()?.downcast_ref::<T>()`): the equivalent of `is`/`as` on the
    /// contract. `None` for an implementation that does not expose itself.
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        None
    }
}

/// The result of instancing a setter on a control.
pub struct SetterInstance {
    pub(crate) kind: SetterInstanceKind,
    /// True when the instance is the setter itself, in which case the style
    /// instance holding it can be shared between controls.
    pub(crate) is_setter: bool,
}

pub(crate) enum SetterInstanceKind {
    /// An entry in the value frame of the style instance.
    Entry(Rc<dyn IValueEntry>),
    /// A setter that was applied directly to the control.
    Other(Rc<dyn ISetterInstance>),
}

/// Handles compare by identity (reference equality), so that they can be
/// held in property and untyped values.
impl PartialEq for dyn SetterBase {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
