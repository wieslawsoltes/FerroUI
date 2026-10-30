// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::{metadata::IServiceProvider, utilities::Uri, BoxedValue};
use ferroui_markup_xaml::{FerroXamlLoader, IUriContext, MarkupExtension, XamlLoadException};
use std::{cell::RefCell, rc::Rc};

/// Loads a markup object from Source using the calling document's URI context.
pub struct LoadExtension {
    source: RefCell<Option<Uri>>,
}
impl LoadExtension {
    /// Creates the source extension without a URI.
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            source: RefCell::new(None),
        })
    }
    /// Creates the source extension with a URI.
    pub fn with_source(source: Uri) -> Rc<Self> {
        let extension = Self::new();
        extension.set_source(Some(source));
        extension
    }
    /// Gets the optional source URI.
    pub fn source(&self) -> Option<Uri> {
        self.source.borrow().clone()
    }
    /// Sets the optional source URI.
    pub fn set_source(&self, source: Option<Uri>) {
        self.source.replace(source);
    }
    /// Loads with the native loader, or returns null without context or source.
    /// Loader failures propagate as the native markup error.
    ///
    /// # Panics
    /// The native loader rejects a missing asset service or a relative URI with
    /// no base URI as programmer errors, following its public loading contract.
    /// Clearing Source from the external BaseUri getter also reaches the source's
    /// null-URI failure rather than returning the initial no-source result.
    pub fn provide_value(
        &self,
        service_provider: &Rc<dyn IServiceProvider>,
    ) -> Result<Option<BoxedValue>, XamlLoadException> {
        let Some(context) = service_provider.get_service_of::<Rc<dyn IUriContext>>() else {
            return Ok(None);
        };
        if self.source().is_none() {
            return Ok(None);
        }
        let base_uri = context.base_uri();
        // The source reads Source again after the external BaseUri getter.
        let source = self
            .source()
            .expect("Source must not be null when loading the URI");
        FerroXamlLoader::load(&source, base_uri.as_ref()).map(Some)
    }
}
impl PartialEq for LoadExtension {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl MarkupExtension for LoadExtension {
    fn provide_value(
        &self,
        service_provider: &Rc<dyn IServiceProvider>,
    ) -> Result<Option<BoxedValue>, XamlLoadException> {
        Self::provide_value(self, service_provider)
    }
}
ferroui_base::ferro_markup_type!(class LoadExtension {
    namespace: "FerroUI.MarkupExtension",
    this: Rc<LoadExtension>,
    handles: [LoadExtension, Rc<LoadExtension>, Option<Rc<LoadExtension>>],
    constructors: [() => LoadExtension::new, (Uri) => LoadExtension::with_source],
    properties: [Source: Option<Uri> { get: LoadExtension::source, set: LoadExtension::set_source } [ConstructorArgument("source")]],
    methods: [try fn ProvideValue(Rc<dyn IServiceProvider>) -> Option<BoxedValue> =>
        |this: &Rc<LoadExtension>, provider: Rc<dyn IServiceProvider>| { this.provide_value(&provider) }],
});
