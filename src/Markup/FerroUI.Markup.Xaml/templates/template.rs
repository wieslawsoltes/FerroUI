//! Port of `Templates/Template.cs`.

use super::TemplateContent;
use ferroui_base::styling::ITemplate;
use ferroui_base::{ferro_markup_type, BoxedValue, Ref};
use ferroui_controls::templates::ITemplateOf;
use ferroui_controls::Control;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A template that builds a control.
pub struct Template {
    this: Weak<Template>,
    content: RefCell<Option<BoxedValue>>,
}

impl Template {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), content: RefCell::new(None) })
    }

    /// The template content: deferred content built on each
    /// [`build`](Self::build).
    pub fn content(&self) -> Option<BoxedValue> {
        self.content.borrow().clone()
    }

    pub fn set_content(&self, value: Option<BoxedValue>) {
        *self.content.borrow_mut() = value;
    }

    /// Builds the control.
    pub fn build(&self) -> Option<Ref<Control>> {
        TemplateContent::load(self.content().as_ref()).map(|result| result.result().clone())
    }

    /// The template as the typed template contract.
    pub fn as_template(&self) -> Rc<dyn ITemplateOf<Option<Ref<Control>>>> {
        self.this.upgrade().expect("the template is alive while it is used")
    }
}

impl ITemplateOf<Option<Ref<Control>>> for Template {
    fn build_typed(&self) -> Option<Ref<Control>> {
        Template::build(self)
    }
}

/// `ITemplate<Control>` and `ITemplate<Control?>` are one contract in the
/// managed original; a property declared with the former takes the template
/// too. Building nothing where a control is required is an error.
impl ITemplateOf<Ref<Control>> for Template {
    fn build_typed(&self) -> Ref<Control> {
        Template::build(self).expect("The template built no control.")
    }
}

impl ITemplate for Template {
    fn build(&self) -> BoxedValue {
        Rc::new(Template::build(self))
    }
}

crate::identity_eq!(Template);

ferro_markup_type!(class Template {
    this: Rc<Template>,
    handles: [Template, Rc<Template>, Option<Rc<Template>>],
    interfaces: [Rc<dyn ITemplateOf<Option<Ref<Control>>>>, Rc<dyn ITemplateOf<Ref<Control>>>, Rc<dyn ITemplate>],
    constructors: [() => Template::new],
    content: Content,
    properties: [
        Content: Option<BoxedValue> { get: Template::content, set: Template::set_content } [TemplateContent],
    ],
    methods: [
        try fn Build() -> Option<Ref<Control>> => |this: &Rc<Template>| {
            TemplateContent::try_load_as::<Ref<Control>>(this.content().as_ref()).map(|result| result.map(|result| result.result().clone()))
        },
    ],
});
