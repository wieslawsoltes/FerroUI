//! Port of `CompilerExtensions/XamlDocumentResource.cs`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use xamlx::ast::XamlDocument;
use xamlx::exceptions::XamlResult;
use xamlx::type_system::{IFileSource, IXamlMethod, IXamlType};

use super::{IXamlDocumentResource, XamlDocumentUsage};

/// The part of the upstream `XamlDocumentTypeBuilderProvider` this file uses: the methods a
/// back end generates for a document.
///
/// Upstream the provider also carries the IL type builders the methods are defined on
/// (`PopulateDeclaringType`, `BuildDeclaringType`); those are specific to a back end, so the
/// back end's provider type implements this trait and keeps them to itself (reachable through
/// [`IXamlDocumentTypeBuilderProvider::as_any`]).
pub trait IXamlDocumentTypeBuilderProvider: 'static {
    /// The method that populates an existing instance of the document's root object.
    fn populate_method(&self) -> Rc<dyn IXamlMethod>;
    /// The method that creates and populates the document's root object, when the document can
    /// be instantiated on its own.
    fn build_method(&self) -> Option<Rc<dyn IXamlMethod>>;
    fn as_any(&self) -> &dyn std::any::Any;
}

/// `Func<XamlDocumentTypeBuilderProvider>`.
pub type XamlDocumentTypeBuilderProviderFactory =
    Box<dyn Fn() -> XamlResult<Rc<dyn IXamlDocumentTypeBuilderProvider>>>;

pub struct XamlDocumentResource {
    create_type_builder_provider: XamlDocumentTypeBuilderProviderFactory,
    type_builder_provider: RefCell<Option<Rc<dyn IXamlDocumentTypeBuilderProvider>>>,
    xaml_document: Rc<RefCell<XamlDocument>>,
    uri: Option<String>,
    file_source: Option<Rc<dyn IFileSource>>,
    class_type: Option<Rc<dyn IXamlType>>,
    is_public: bool,
    usage: Cell<XamlDocumentUsage>,
}

impl XamlDocumentResource {
    pub fn new(
        xaml_document: Rc<RefCell<XamlDocument>>,
        uri: Option<String>,
        file_source: Option<Rc<dyn IFileSource>>,
        class_type: Option<Rc<dyn IXamlType>>,
        is_public: bool,
        create_type_builder_provider: XamlDocumentTypeBuilderProviderFactory,
    ) -> Rc<Self> {
        Rc::new(Self {
            create_type_builder_provider,
            type_builder_provider: RefCell::new(None),
            xaml_document,
            uri,
            file_source,
            class_type,
            is_public,
            usage: Cell::new(XamlDocumentUsage::Unknown),
        })
    }

    pub fn is_public(&self) -> bool {
        self.is_public
    }

    /// `TypeBuilderProvider`: created on first use, which also marks the document as used.
    pub fn type_builder_provider(&self) -> XamlResult<Rc<dyn IXamlDocumentTypeBuilderProvider>> {
        let existing = self.type_builder_provider.borrow().clone();
        if let Some(provider) = existing {
            return Ok(provider);
        }

        let provider = (self.create_type_builder_provider)()?;
        *self.type_builder_provider.borrow_mut() = Some(provider.clone());
        self.usage.set(XamlDocumentUsage::Used);
        Ok(provider)
    }
}

impl IXamlDocumentResource for XamlDocumentResource {
    fn build_method(&self) -> XamlResult<Option<Rc<dyn IXamlMethod>>> {
        Ok(self.type_builder_provider()?.build_method())
    }
    fn class_type(&self) -> Option<Rc<dyn IXamlType>> {
        self.class_type.clone()
    }
    fn uri(&self) -> Option<String> {
        self.uri.clone()
    }
    fn populate_method(&self) -> XamlResult<Rc<dyn IXamlMethod>> {
        Ok(self.type_builder_provider()?.populate_method())
    }
    fn file_source(&self) -> Option<Rc<dyn IFileSource>> {
        self.file_source.clone()
    }
    fn xaml_document(&self) -> Rc<RefCell<XamlDocument>> {
        self.xaml_document.clone()
    }
    fn usage(&self) -> XamlDocumentUsage {
        self.usage.get()
    }
    fn set_usage(&self, value: XamlDocumentUsage) {
        self.usage.set(value)
    }
}
