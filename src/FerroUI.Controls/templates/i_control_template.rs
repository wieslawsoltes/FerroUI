use super::{ITemplateWithParam, TemplateResult};
use crate::primitives::TemplatedControl;
use crate::Control;
use ferroui_base::Ref;

/// Interface representing a template used to build a control for a lookless
/// control.
pub trait IControlTemplate:
    ITemplateWithParam<Ref<TemplatedControl>, Option<TemplateResult<Ref<Control>>>>
{
    /// The template as its concrete type
    /// (`as_any()?.downcast_ref::<T>()`): the equivalent of `is`/`as` on the
    /// contract. `None` unless the implementation provides it.
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        None
    }
}

impl PartialEq for dyn IControlTemplate {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
