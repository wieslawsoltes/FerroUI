// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Native view of the original CLR document callback, including null reference results.
use ferrodock_model::{controls::DocumentFactory, core::IDockable};
use ferroui_base::{data::core::ValueTypes, metadata::MarkupDelegate, BoxedValue};
use std::rc::Rc;

/// A view of the original document callback, with the shared native Func metadata.
#[derive(Clone)]
pub struct DocumentFactoryReference(DocumentFactory);
impl DocumentFactoryReference {
    /// Retains the actual callback without invoking or adapting it.
    pub fn new(model: DocumentFactory) -> Self {
        Self(model)
    }
    /// Gets the same original callback instance.
    pub fn model(&self) -> DocumentFactory {
        self.0.clone()
    }
    /// Invokes the original callback and returns its original model.
    pub fn invoke(&self) -> Option<Rc<dyn IDockable>> {
        (self.0)()
    }
    /// Adapts a native method reference after checking its declared signature.
    pub fn from_delegate(delegate: MarkupDelegate) -> Option<Self> {
        let nullable = crate::DockableFactoryReference::from_delegate(delegate)?;
        Some(Self::new(nullable.model()))
    }
}
impl PartialEq for DocumentFactoryReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
/// Both source representations are handles of the one original System.Func<IDockable>.
pub(crate) fn invoke(value: &BoxedValue) -> Result<Option<Rc<dyn IDockable>>, &'static str> {
    let value = ValueTypes::normalize(value.clone())
        .ok_or("A factory invocation requires a nonnull receiver")?;
    if let Some(factory) = value.downcast_ref::<DocumentFactoryReference>() {
        return Ok(factory.invoke());
    }
    value
        .downcast_ref::<crate::DockableFactoryReference>()
        .map(|factory| factory.invoke())
        .ok_or("The receiver is not a registered source factory handle")
}
pub(crate) fn register() {
    ValueTypes::register_nullable::<DocumentFactoryReference>();
    ValueTypes::register_cast::<crate::DockableFactoryReference, DocumentFactoryReference>(
        |value| DocumentFactoryReference::new(value.model()),
    );
    ValueTypes::register_cast::<DocumentFactoryReference, crate::DockableFactoryReference>(
        |value| crate::DockableFactoryReference::new(value.model()),
    );
    ValueTypes::register_reference_identity::<DocumentFactoryReference>(|value| {
        Rc::as_ptr(&value.0).cast::<()>()
    });
    ValueTypes::register_boxed_cast::<MarkupDelegate, DocumentFactoryReference>(|value| {
        DocumentFactoryReference::from_delegate(value.downcast_ref::<MarkupDelegate>()?.clone())
    });
}
