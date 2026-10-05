//! Port of `MarkupExtension.cs`.

use crate::XamlLoadException;
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::{ferro_markup_type, BoxedValue};
use std::rc::Rc;

/// The base of markup extensions: objects that provide the value of a
/// property in markup (`{StaticResource Key}`).
///
/// A type is a markup extension if its metadata declares a `ProvideValue`
/// method; implementing this trait is the equivalent of deriving from the
/// abstract base class.
pub trait MarkupExtension {
    /// Returns the object to set on the target property. Null is `None`.
    fn provide_value(&self, service_provider: &Rc<dyn IServiceProvider>) -> Result<Option<BoxedValue>, XamlLoadException>;
}

/// Markup extension handles compare by identity, so that they can be held
/// in untyped values.
impl PartialEq for dyn MarkupExtension {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}

ferro_markup_type!(class dyn MarkupExtension as "MarkupExtension" {
    this: Rc<dyn MarkupExtension>,
    handles: [Rc<dyn MarkupExtension>, Option<Rc<dyn MarkupExtension>>],
    methods: [
        try fn ProvideValue(Rc<dyn IServiceProvider>) -> Option<BoxedValue> =>
            |this: &Rc<dyn MarkupExtension>, service_provider: Rc<dyn IServiceProvider>| {
                this.provide_value(&service_provider)
            },
    ],
});
