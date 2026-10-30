// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    metadata::IServiceProvider,
    BoxedValue,
};
use ferroui_markup_xaml::{MarkupExtension, ServiceProviderExtensions, XamlLoadException};
use std::{cell::RefCell, rc::Rc};

/// Resolves Name synchronously in the calling document's native name scope.
pub struct ReferenceExtension {
    name: RefCell<Option<String>>,
}
impl ReferenceExtension {
    /// Creates the source extension without a name.
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            name: RefCell::new(None),
        })
    }
    /// Creates the source extension with a name.
    pub fn with_name(name: String) -> Rc<Self> {
        let extension = Self::new();
        extension.set_name(Some(name));
        extension
    }
    /// Gets the optional name.
    pub fn name(&self) -> Option<String> {
        self.name.borrow().clone()
    }
    /// Sets the optional name.
    pub fn set_name(&self, name: Option<String>) {
        self.name.replace(name);
    }
    /// Returns the same named object or model, or null without a scope/name/match.
    /// This is immediate Find, without a deferred forward-reference subscription.
    pub fn provide_value(&self, service_provider: &Rc<dyn IServiceProvider>) -> Option<BoxedValue> {
        let scope = service_provider.get_name_scope()?;
        let name = self.name()?;
        let value = scope.find_value(&name)?;
        let Some(element) = ValueTypes::as_object(&*value) else {
            return Some(value);
        };
        let class = element.get_type();
        let value: BoxedValue = Rc::new(element);
        // Restore the object's concrete native handle, retaining object identity.
        class
            .handle()
            .and_then(|handle| {
                ValueTypes::try_convert(Some(&value), ValueType::new(handle, class.name()))
                    .flatten()
            })
            .or(Some(value))
    }
}
impl PartialEq for ReferenceExtension {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl MarkupExtension for ReferenceExtension {
    fn provide_value(
        &self,
        service_provider: &Rc<dyn IServiceProvider>,
    ) -> Result<Option<BoxedValue>, XamlLoadException> {
        Ok(Self::provide_value(self, service_provider))
    }
}
ferroui_base::ferro_markup_type!(class ReferenceExtension {
    namespace: "FerroUI.MarkupExtension",
    this: Rc<ReferenceExtension>,
    handles: [ReferenceExtension, Rc<ReferenceExtension>, Option<Rc<ReferenceExtension>>],
    constructors: [() => ReferenceExtension::new, (String) => ReferenceExtension::with_name],
    properties: [Name: Option<String> { get: ReferenceExtension::name, set: ReferenceExtension::set_name } [ConstructorArgument("name")]],
    methods: [fn ProvideValue(Rc<dyn IServiceProvider>) -> Option<BoxedValue> =>
        |this: &Rc<ReferenceExtension>, provider: Rc<dyn IServiceProvider>| { this.provide_value(&provider) }],
});
