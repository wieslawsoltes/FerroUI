//! Port of `Templates/NotificationTemplate.cs`.

use ferroui_base::styling::ITemplate;
use ferroui_base::{ferro_markup_type, BoxedValue};
use ferroui_controls::notifications::INotification;
use ferroui_markup_xaml::templates::TemplateContent;
use std::cell::RefCell;
use std::rc::Rc;

/// A template for creating notifications: each [`build`](Self::build) creates a new
/// notification from the content.
#[derive(Default)]
pub struct NotificationTemplate {
    content: RefCell<Option<BoxedValue>>,
}

impl NotificationTemplate {
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

    /// Builds the notification from the template.
    ///
    /// # Panics
    /// Panics if the content is not template content, or builds something else than a
    /// notification.
    pub fn build(&self) -> Option<Rc<dyn INotification>> {
        TemplateContent::load_as::<Rc<dyn INotification>>(self.content().as_ref()).map(|result| result.result().clone())
    }
}

impl ITemplate for NotificationTemplate {
    /// Builds the notification from the template, as an optional notification.
    fn build(&self) -> BoxedValue {
        Rc::new(NotificationTemplate::build(self))
    }
}

impl PartialEq for NotificationTemplate {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

ferro_markup_type!(class NotificationTemplate {
    this: Rc<NotificationTemplate>,
    handles: [NotificationTemplate, Rc<NotificationTemplate>, Option<Rc<NotificationTemplate>>],
    interfaces: [Rc<dyn ITemplate>],
    constructors: [() => NotificationTemplate::new],
    content: Content,
    properties: [
        Content: Option<BoxedValue> {
            get: NotificationTemplate::content,
            set: NotificationTemplate::set_content
        } [TemplateContent(TemplateResultType = type(Rc<dyn INotification>))],
    ],
});
