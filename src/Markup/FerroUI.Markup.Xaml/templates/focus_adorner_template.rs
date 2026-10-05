//! Port of `Templates/FocusAdornerTemplate.cs`.

use super::Template;
use ferroui_base::styling::ITemplate;
use ferroui_base::{ferro_markup_type, BoxedValue, Ref};
use ferroui_controls::templates::ITemplateOf;
use ferroui_controls::Control;
use std::ops::Deref;
use std::rc::{Rc, Weak};

/// The template of a focus adorner.
///
/// The managed class derives from [`Template`] and adds nothing; here it
/// holds one and dereferences to it.
pub struct FocusAdornerTemplate {
    this: Weak<FocusAdornerTemplate>,
    base: Rc<Template>,
}

impl FocusAdornerTemplate {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), base: Template::new() })
    }

    /// The template this one is (the base class part).
    pub fn base(&self) -> &Rc<Template> {
        &self.base
    }

    /// The template as the typed template contract.
    pub fn as_template(&self) -> Rc<dyn ITemplateOf<Option<Ref<Control>>>> {
        self.this.upgrade().expect("the template is alive while it is used")
    }
}

impl Deref for FocusAdornerTemplate {
    type Target = Template;

    fn deref(&self) -> &Template {
        &self.base
    }
}

impl ITemplateOf<Option<Ref<Control>>> for FocusAdornerTemplate {
    fn build_typed(&self) -> Option<Ref<Control>> {
        Template::build(&self.base)
    }
}

impl ITemplateOf<Ref<Control>> for FocusAdornerTemplate {
    fn build_typed(&self) -> Ref<Control> {
        Template::build(&self.base).expect("The template built no control.")
    }
}

impl ITemplate for FocusAdornerTemplate {
    fn build(&self) -> BoxedValue {
        Rc::new(Template::build(&self.base))
    }
}

crate::identity_eq!(FocusAdornerTemplate);

ferro_markup_type!(class FocusAdornerTemplate {
    this: Rc<FocusAdornerTemplate>,
    handles: [FocusAdornerTemplate, Rc<FocusAdornerTemplate>, Option<Rc<FocusAdornerTemplate>>],
    base: Rc<Template>,
    constructors: [() => FocusAdornerTemplate::new],
});
