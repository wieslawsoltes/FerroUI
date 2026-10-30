//! Port of `Templates/BehaviorCollectionTemplate.cs`.

use crate::BehaviorCollection;
use ferroui_base::styling::ITemplate;
use ferroui_base::{ferro_markup_type, BoxedValue};
use ferroui_markup_xaml::templates::TemplateContent;
use std::cell::RefCell;
use std::rc::Rc;

/// A template for creating a collection of [`IBehavior`](crate::IBehavior) objects.
///
/// As the value of a style setter of `(Interaction.Behaviors)` it builds a new collection for
/// each element the style applies to (a collection attaches to one object only).
#[derive(Default)]
pub struct BehaviorCollectionTemplate {
    content: RefCell<Option<BoxedValue>>,
}

impl BehaviorCollectionTemplate {
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

    /// Builds the collection of behaviors from the template.
    ///
    /// # Panics
    /// Panics if the content is not template content, or builds something else than a
    /// collection of behaviors.
    pub fn build(&self) -> Option<BehaviorCollection> {
        TemplateContent::load_as::<BehaviorCollection>(self.content().as_ref()).map(|result| result.result().clone())
    }
}

impl ITemplate for BehaviorCollectionTemplate {
    /// Builds the collection of behaviors from the template, as the value of the attached
    /// property (an optional collection).
    fn build(&self) -> BoxedValue {
        Rc::new(BehaviorCollectionTemplate::build(self))
    }
}

impl PartialEq for BehaviorCollectionTemplate {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

ferro_markup_type!(class BehaviorCollectionTemplate {
    this: Rc<BehaviorCollectionTemplate>,
    handles: [BehaviorCollectionTemplate, Rc<BehaviorCollectionTemplate>, Option<Rc<BehaviorCollectionTemplate>>],
    interfaces: [Rc<dyn ITemplate>],
    constructors: [() => BehaviorCollectionTemplate::new],
    content: Content,
    properties: [
        Content: Option<BoxedValue> {
            get: BehaviorCollectionTemplate::content,
            set: BehaviorCollectionTemplate::set_content
        } [TemplateContent(TemplateResultType = type(BehaviorCollection))],
    ],
});
