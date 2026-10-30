//! Port of `Templates/ActionCollectionTemplate.cs`.

use crate::ActionCollection;
use ferroui_base::styling::ITemplate;
use ferroui_base::{ferro_markup_type, BoxedValue};
use ferroui_markup_xaml::templates::TemplateContent;
use std::cell::RefCell;
use std::rc::Rc;

/// A template for creating a collection of [`IAction`](crate::IAction) objects.
///
/// As the value of a style setter of the `Actions` of a
/// [`StyledElementTrigger`](crate::StyledElementTrigger) it builds a new collection for each
/// trigger the style applies to.
#[derive(Default)]
pub struct ActionCollectionTemplate {
    content: RefCell<Option<BoxedValue>>,
}

impl ActionCollectionTemplate {
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

    /// Builds the collection of actions from the template.
    ///
    /// # Panics
    /// Panics if the content is not template content, or builds something else than a
    /// collection of actions.
    pub fn build(&self) -> Option<ActionCollection> {
        TemplateContent::load_as::<ActionCollection>(self.content().as_ref()).map(|result| result.result().clone())
    }
}

impl ITemplate for ActionCollectionTemplate {
    /// Builds the collection of actions from the template, as the value of the property (an
    /// optional collection).
    fn build(&self) -> BoxedValue {
        Rc::new(ActionCollectionTemplate::build(self))
    }
}

impl PartialEq for ActionCollectionTemplate {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

ferro_markup_type!(class ActionCollectionTemplate {
    this: Rc<ActionCollectionTemplate>,
    handles: [ActionCollectionTemplate, Rc<ActionCollectionTemplate>, Option<Rc<ActionCollectionTemplate>>],
    interfaces: [Rc<dyn ITemplate>],
    constructors: [() => ActionCollectionTemplate::new],
    content: Content,
    properties: [
        Content: Option<BoxedValue> {
            get: ActionCollectionTemplate::content,
            set: ActionCollectionTemplate::set_content
        } [TemplateContent(TemplateResultType = type(ActionCollection))],
    ],
});
