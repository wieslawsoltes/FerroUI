use super::{IRecyclingDataTemplate, ITemplateWithParam, ITreeDataTemplate, ITypedDataTemplate};
use crate::Control;
use ferroui_base::{BoxedValue, Ref};

/// Interface representing a template used to build a control for a piece of
/// data.
pub trait IDataTemplate: ITemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>> {
    /// Checks to see if this data template matches the specified data.
    fn match_(&self, data: Option<&BoxedValue>) -> bool;

    /// The template viewed as a recycling data template, if it is one.
    fn as_recycling_data_template(&self) -> Option<&dyn IRecyclingDataTemplate> {
        None
    }

    /// The template viewed as a typed data template, if it is one.
    fn as_typed_data_template(&self) -> Option<&dyn ITypedDataTemplate> {
        None
    }

    /// The template viewed as a tree data template, if it is one.
    fn as_tree_data_template(&self) -> Option<&dyn ITreeDataTemplate> {
        None
    }

    /// The implementing value, for casting to its concrete type
    /// (`as_any()?.downcast_ref::<T>()`): the equivalent of `is`/`as` on the
    /// contract. `None` for an implementation that does not expose itself.
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        None
    }
}

impl PartialEq for dyn IDataTemplate {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
