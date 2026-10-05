use crate::{BoxedValue, FerroObject, StyledElement, TypeInfo};
use std::cell::RefCell;

thread_local! {
    static PROVIDER_TYPES: RefCell<Vec<&'static TypeInfo>> = const { RefCell::new(Vec::new()) };
}

/// Defines an element with a data context that can be used for binding.
pub trait IDataContextProvider {
    /// Gets the element's data context.
    fn data_context(&self) -> Option<BoxedValue>;

    /// Sets the element's data context.
    fn set_data_context(&self, value: Option<BoxedValue>);
}

impl dyn IDataContextProvider {
    /// Declares that the objects of a class that is not a styled element
    /// (the application class) have a data context that can be used for
    /// binding: the class is an owner of
    /// [`StyledElement::data_context_property`]. Styled elements and the
    /// classes deriving from a declared class are providers already.
    pub fn register_type(type_: &'static TypeInfo) {
        PROVIDER_TYPES.with_borrow_mut(|types| {
            if !types.iter().any(|t| std::ptr::eq(*t, type_)) {
                types.push(type_);
            }
        });
    }

    /// Whether `object` has a data context that can be used for binding.
    pub fn is_implemented_by(object: &FerroObject) -> bool {
        if object.is::<StyledElement>() {
            return true;
        }
        let type_ = object.get_type();
        PROVIDER_TYPES.with_borrow(|types| types.iter().any(|t| t.is_assignable_from(type_)))
    }
}

impl IDataContextProvider for StyledElement {
    fn data_context(&self) -> Option<BoxedValue> {
        StyledElement::data_context(self)
    }

    fn set_data_context(&self, value: Option<BoxedValue>) {
        StyledElement::set_data_context(self, value)
    }
}
