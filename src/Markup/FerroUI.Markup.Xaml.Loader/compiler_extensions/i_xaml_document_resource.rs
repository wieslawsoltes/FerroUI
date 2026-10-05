//! Port of `CompilerExtensions/IXamlDocumentResource.cs`.

use std::cell::RefCell;
use std::rc::Rc;

use xamlx::ast::XamlDocument;
use xamlx::exceptions::XamlResult;
use xamlx::type_system::{IFileSource, IXamlMethod, IXamlType};

use super::XamlDocumentUsage;

/// A XAML document taking part in a compilation, as seen by the group transformers.
///
/// `build_method` and `populate_method` return a `XamlResult` because the implementation
/// creates the document's output type lazily, which can fail.
pub trait IXamlDocumentResource: 'static {
    fn build_method(&self) -> XamlResult<Option<Rc<dyn IXamlMethod>>>;
    fn class_type(&self) -> Option<Rc<dyn IXamlType>>;
    fn uri(&self) -> Option<String>;
    fn populate_method(&self) -> XamlResult<Rc<dyn IXamlMethod>>;
    fn file_source(&self) -> Option<Rc<dyn IFileSource>>;
    /// The document. It is shared and mutable: group transformers replace its root.
    fn xaml_document(&self) -> Rc<RefCell<XamlDocument>>;
    fn usage(&self) -> XamlDocumentUsage;
    fn set_usage(&self, value: XamlDocumentUsage);
}
