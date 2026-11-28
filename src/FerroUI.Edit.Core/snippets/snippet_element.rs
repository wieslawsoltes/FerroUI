use super::{SnippetError, SnippetInsertionContext};
use std::fmt;
use std::rc::Rc;

/// An application-defined expansion node. Preparation does not mutate the
/// document, so a rejected extension cannot partially delete the selection.
pub trait ISnippetElement {
    fn insert(&self, context: &mut SnippetInsertionContext) -> Result<(), SnippetError>;
}

/// Shared identity of a custom snippet node, used to find its active element.
#[derive(Clone)]
pub struct SnippetElementRef(pub(crate) Rc<dyn ISnippetElement>);

impl SnippetElementRef {
    pub fn new(element: impl ISnippetElement + 'static) -> Self {
        Self(Rc::new(element))
    }
    pub fn from_rc(element: Rc<dyn ISnippetElement>) -> Self {
        Self(element)
    }
}
impl fmt::Debug for SnippetElementRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SnippetElementRef")
            .field(&Rc::as_ptr(&self.0))
            .finish()
    }
}
impl PartialEq for SnippetElementRef {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for SnippetElementRef {}

type TextConverter = dyn Fn(&str) -> Result<String, SnippetError>;

/// Rust equivalent of overriding `SnippetBoundElement.ConvertText`.
/// Converters run outside document and session collection borrows.
#[derive(Clone)]
pub struct SnippetTextTransform(Rc<TextConverter>);

impl SnippetTextTransform {
    pub fn new(convert: impl Fn(&str) -> String + 'static) -> Self {
        Self::try_new(move |text| Ok(convert(text)))
    }
    pub fn try_new(convert: impl Fn(&str) -> Result<String, SnippetError> + 'static) -> Self {
        Self(Rc::new(convert))
    }
    pub fn convert(&self, text: &str) -> Result<String, SnippetError> {
        (self.0)(text)
    }
}
impl fmt::Debug for SnippetTextTransform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SnippetTextTransform")
            .field(&Rc::as_ptr(&self.0))
            .finish()
    }
}
impl PartialEq for SnippetTextTransform {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for SnippetTextTransform {}
