//! Port of `RuntimeXamlLoaderDocument.cs`.

use ferroui_base::metadata::IServiceProvider;
use ferroui_base::utilities::Uri;
use ferroui_base::BoxedValue;
use std::cell::RefCell;
use std::io::{Cursor, Read};
use std::rc::Rc;

/// A markup document for the runtime XAML loader.
pub struct RuntimeXamlLoaderDocument {
    /// The URI of the document: what relative URIs in it are resolved
    /// against.
    pub base_uri: Option<Uri>,

    /// The path of the document, for diagnostics.
    pub document: Option<String>,

    /// An existing instance to populate instead of creating the root object
    /// of the document.
    pub root_instance: Option<BoxedValue>,

    /// A service provider the root of the document is given as its parent:
    /// the context of the place the document is loaded into.
    pub service_provider: Option<Rc<dyn IServiceProvider>>,

    xaml_stream: RefCell<Box<dyn Read>>,
}

impl RuntimeXamlLoaderDocument {
    /// A document from text.
    pub fn new(xaml: &str) -> Self {
        Self::from_stream(Box::new(Cursor::new(xaml.as_bytes().to_vec())))
    }

    /// A document from text, with its URI.
    pub fn with_base_uri(base_uri: Option<Uri>, xaml: &str) -> Self {
        Self { base_uri, ..Self::new(xaml) }
    }

    /// A document from text that populates `root_instance`.
    pub fn with_root_instance(root_instance: Option<BoxedValue>, xaml: &str) -> Self {
        Self { root_instance, ..Self::new(xaml) }
    }

    /// A document from text, with its URI, that populates `root_instance`.
    pub fn with_base_uri_and_root_instance(base_uri: Option<Uri>, root_instance: Option<BoxedValue>, xaml: &str) -> Self {
        Self { root_instance, ..Self::with_base_uri(base_uri, xaml) }
    }

    /// A document from a stream of UTF-8 text.
    pub fn from_stream(stream: Box<dyn Read>) -> Self {
        Self {
            base_uri: None,
            document: None,
            root_instance: None,
            service_provider: None,
            xaml_stream: RefCell::new(stream),
        }
    }

    /// A document from a stream, with its URI.
    pub fn from_stream_with_base_uri(base_uri: Option<Uri>, stream: Box<dyn Read>) -> Self {
        Self { base_uri, ..Self::from_stream(stream) }
    }

    /// A document from a stream that populates `root_instance`.
    pub fn from_stream_with_root_instance(root_instance: Option<BoxedValue>, stream: Box<dyn Read>) -> Self {
        Self { root_instance, ..Self::from_stream(stream) }
    }

    /// A document from a stream, with its URI, that populates
    /// `root_instance`.
    pub fn from_stream_with_base_uri_and_root_instance(
        base_uri: Option<Uri>,
        root_instance: Option<BoxedValue>,
        stream: Box<dyn Read>,
    ) -> Self {
        Self { root_instance, ..Self::from_stream_with_base_uri(base_uri, stream) }
    }

    /// The stream of the document. Reading advances it: a document is read
    /// once.
    pub fn xaml_stream(&self) -> std::cell::RefMut<'_, Box<dyn Read>> {
        self.xaml_stream.borrow_mut()
    }

    /// Reads the rest of the document as text.
    pub fn read_to_string(&self) -> std::io::Result<String> {
        let mut text = String::new();
        self.xaml_stream.borrow_mut().read_to_string(&mut text)?;
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::utilities::UriKind;

    #[test]
    fn text_constructors_set_their_members() {
        let uri = Uri::new("ferres://app/Main.xaml", UriKind::Absolute).unwrap();
        let root: BoxedValue = Rc::new(5i32);

        let document = RuntimeXamlLoaderDocument::new("<a/>");
        assert!(document.base_uri.is_none() && document.root_instance.is_none());
        assert!(document.document.is_none() && document.service_provider.is_none());
        assert_eq!(document.read_to_string().unwrap(), "<a/>");
        // The stream has been consumed.
        assert_eq!(document.read_to_string().unwrap(), "");

        let document = RuntimeXamlLoaderDocument::with_base_uri(Some(uri.clone()), "<b/>");
        assert_eq!(document.base_uri, Some(uri.clone()));
        assert!(document.root_instance.is_none());

        let document = RuntimeXamlLoaderDocument::with_root_instance(Some(root.clone()), "<c/>");
        assert!(document.base_uri.is_none());
        assert!(Rc::ptr_eq(document.root_instance.as_ref().unwrap(), &root));

        let document =
            RuntimeXamlLoaderDocument::with_base_uri_and_root_instance(Some(uri.clone()), Some(root.clone()), "<d/>");
        assert_eq!(document.base_uri, Some(uri));
        assert!(document.root_instance.is_some());
        assert_eq!(document.read_to_string().unwrap(), "<d/>");
    }

    #[test]
    fn stream_constructors_set_their_members() {
        let uri = Uri::new("ferres://app/Main.xaml", UriKind::Absolute).unwrap();
        let root: BoxedValue = Rc::new(5i32);
        let stream = || Box::new(Cursor::new(b"<e/>".to_vec())) as Box<dyn Read>;

        assert_eq!(RuntimeXamlLoaderDocument::from_stream(stream()).read_to_string().unwrap(), "<e/>");
        let document = RuntimeXamlLoaderDocument::from_stream_with_base_uri(Some(uri.clone()), stream());
        assert_eq!(document.base_uri, Some(uri.clone()));
        let document = RuntimeXamlLoaderDocument::from_stream_with_root_instance(Some(root.clone()), stream());
        assert!(document.root_instance.is_some() && document.base_uri.is_none());
        let document =
            RuntimeXamlLoaderDocument::from_stream_with_base_uri_and_root_instance(Some(uri), Some(root), stream());
        assert!(document.root_instance.is_some() && document.base_uri.is_some());
        let mut bytes = Vec::new();
        document.xaml_stream().read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"<e/>");
    }
}
