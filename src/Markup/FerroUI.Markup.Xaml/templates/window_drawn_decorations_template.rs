//! Port of `Templates/WindowDrawnDecorationsTemplate.cs`.

use super::TemplateContent;
use ferroui_base::styling::ITemplate;
use ferroui_base::{ferro_markup_type, BoxedValue, Ref};
use ferroui_controls::chrome::{IWindowDrawnDecorationsTemplate, WindowDrawnDecorationsContent};
use ferroui_controls::templates::TemplateResult;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The template of the decorations a window draws itself, as markup
/// declares it.
pub struct WindowDrawnDecorationsTemplate {
    this: Weak<WindowDrawnDecorationsTemplate>,
    content: RefCell<Option<BoxedValue>>,
}

impl WindowDrawnDecorationsTemplate {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), content: RefCell::new(None) })
    }

    /// The template content: deferred content whose result is the
    /// decorations content.
    pub fn content(&self) -> Option<BoxedValue> {
        self.content.borrow().clone()
    }

    pub fn set_content(&self, value: Option<BoxedValue>) {
        *self.content.borrow_mut() = value;
    }

    /// Builds the decorations content.
    ///
    /// # Panics
    /// Panics if the template has no content.
    pub fn build(&self) -> TemplateResult<Ref<WindowDrawnDecorationsContent>> {
        crate::throw(self.try_build())
    }

    /// [`build`](Self::build) without the panic: a template without content
    /// is an error.
    pub fn try_build(&self) -> Result<TemplateResult<Ref<WindowDrawnDecorationsContent>>, crate::XamlLoadException> {
        TemplateContent::try_load_as::<Ref<WindowDrawnDecorationsContent>>(self.content().as_ref())?.ok_or_else(|| {
            crate::XamlLoadException::with_message("Operation is not valid due to the current state of the object.")
        })
    }

    /// The template as the decorations template contract.
    pub fn as_window_drawn_decorations_template(&self) -> Rc<dyn IWindowDrawnDecorationsTemplate> {
        self.this.upgrade().expect("the template is alive while it is used")
    }
}

impl IWindowDrawnDecorationsTemplate for WindowDrawnDecorationsTemplate {
    fn build_typed(&self) -> TemplateResult<Ref<WindowDrawnDecorationsContent>> {
        WindowDrawnDecorationsTemplate::build(self)
    }
}

impl ITemplate for WindowDrawnDecorationsTemplate {
    fn build(&self) -> BoxedValue {
        Rc::new(WindowDrawnDecorationsTemplate::build(self).result().clone())
    }
}

crate::identity_eq!(WindowDrawnDecorationsTemplate);

ferro_markup_type!(class WindowDrawnDecorationsTemplate {
    this: Rc<WindowDrawnDecorationsTemplate>,
    handles: [
        WindowDrawnDecorationsTemplate,
        Rc<WindowDrawnDecorationsTemplate>,
        Option<Rc<WindowDrawnDecorationsTemplate>>,
    ],
    interfaces: [Rc<dyn IWindowDrawnDecorationsTemplate>, Rc<dyn ITemplate>],
    constructors: [() => WindowDrawnDecorationsTemplate::new],
    content: Content,
    properties: [
        Content: Option<BoxedValue> {
            get: WindowDrawnDecorationsTemplate::content,
            set: WindowDrawnDecorationsTemplate::set_content
        } [TemplateContent(TemplateResultType = type(Ref<WindowDrawnDecorationsContent>))],
    ],
    methods: [
        try fn Build() -> Ref<WindowDrawnDecorationsContent> => |this: &Rc<WindowDrawnDecorationsTemplate>| {
            this.try_build().map(|result| result.result().clone())
        },
    ],
    attributes: [ControlTemplateScope],
});
