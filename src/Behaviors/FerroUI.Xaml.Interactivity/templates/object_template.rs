//! Port of `Templates/ObjectTemplate.cs`.

use ferroui_base::styling::ITemplate;
use ferroui_base::{ferro_markup_type, BoxedValue};
use ferroui_markup_xaml::templates::TemplateContent;
use std::cell::RefCell;
use std::rc::Rc;

/// A template for creating custom objects: each [`build`](Self::build) creates a new object
/// from the content.
#[derive(Default)]
pub struct ObjectTemplate {
    content: RefCell<Option<BoxedValue>>,
}

impl ObjectTemplate {
    /// Creates an empty template.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Gets or sets the content of the template: deferred content built on each
    /// [`build`](Self::build).
    pub fn content(&self) -> Option<BoxedValue> {
        self.content.borrow().clone()
    }

    pub fn set_content(&self, value: Option<BoxedValue>) {
        *self.content.borrow_mut() = value;
    }

    /// Builds the object from the template.
    ///
    /// # Panics
    /// Panics if the content is not template content.
    pub fn build(&self) -> Option<BoxedValue> {
        TemplateContent::load_as::<Option<BoxedValue>>(self.content().as_ref()).and_then(|result| result.result().clone())
    }
}

impl ITemplate for ObjectTemplate {
    /// Builds the object from the template, as an untyped value.
    fn build(&self) -> BoxedValue {
        Rc::new(ObjectTemplate::build(self))
    }
}

impl PartialEq for ObjectTemplate {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

ferro_markup_type!(class ObjectTemplate {
    this: Rc<ObjectTemplate>,
    handles: [ObjectTemplate, Rc<ObjectTemplate>, Option<Rc<ObjectTemplate>>],
    interfaces: [Rc<dyn ITemplate>],
    constructors: [() => ObjectTemplate::new],
    content: Content,
    properties: [
        Content: Option<BoxedValue> {
            get: ObjectTemplate::content,
            set: ObjectTemplate::set_content
        } [TemplateContent(TemplateResultType = type(Option<BoxedValue>))],
    ],
});
