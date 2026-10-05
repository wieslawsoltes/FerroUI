//! Port of `Templates/ControlTemplate.cs`.

use super::TemplateContent;
use ferroui_base::{ferro_markup_type, BoxedValue, Ref, TypeInfo};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::templates::{IControlTemplate, ITemplateWithParam, TemplateResult};
use ferroui_controls::Control;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The template of a templated control, as markup declares it.
pub struct ControlTemplate {
    this: Weak<ControlTemplate>,
    content: RefCell<Option<BoxedValue>>,
    target_type: Cell<Option<&'static TypeInfo>>,
}

impl ControlTemplate {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), content: RefCell::new(None), target_type: Cell::new(None) })
    }

    /// The template content: deferred content built for each control the
    /// template is applied to.
    pub fn content(&self) -> Option<BoxedValue> {
        self.content.borrow().clone()
    }

    pub fn set_content(&self, value: Option<BoxedValue>) {
        *self.content.borrow_mut() = value;
    }

    /// The type of control the template is for.
    pub fn target_type(&self) -> Option<&'static TypeInfo> {
        self.target_type.get()
    }

    pub fn set_target_type(&self, value: Option<&'static TypeInfo>) {
        self.target_type.set(value);
    }

    /// Builds the template for `control`.
    pub fn build(&self, _control: &Ref<TemplatedControl>) -> Option<TemplateResult<Ref<Control>>> {
        TemplateContent::load(self.content().as_ref())
    }

    /// The template behind a control template handle, if it is one of this
    /// class (the cast of the contract back to the class).
    pub fn from_control_template(template: &Rc<dyn IControlTemplate>) -> Option<Rc<ControlTemplate>> {
        template.as_any()?.downcast_ref::<ControlTemplate>()?.this.upgrade()
    }

    /// The template as the control template contract.
    pub fn as_control_template(&self) -> Rc<dyn IControlTemplate> {
        self.this.upgrade().expect("the template is alive while it is used")
    }
}

impl ITemplateWithParam<Ref<TemplatedControl>, Option<TemplateResult<Ref<Control>>>> for ControlTemplate {
    fn build(&self, param: &Ref<TemplatedControl>) -> Option<TemplateResult<Ref<Control>>> {
        ControlTemplate::build(self, param)
    }
}

impl IControlTemplate for ControlTemplate {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

crate::identity_eq!(ControlTemplate);

ferro_markup_type!(class ControlTemplate {
    this: Rc<ControlTemplate>,
    handles: [ControlTemplate, Rc<ControlTemplate>, Option<Rc<ControlTemplate>>],
    interfaces: [Rc<dyn IControlTemplate>],
    constructors: [() => ControlTemplate::new],
    content: Content,
    properties: [
        Content: Option<BoxedValue> { get: ControlTemplate::content, set: ControlTemplate::set_content }
            [TemplateContent],
        TargetType: Option<&'static TypeInfo> {
            get: ControlTemplate::target_type,
            set: ControlTemplate::set_target_type
        },
    ],
    methods: [
        try fn Build(Ref<TemplatedControl>) -> Option<Ref<Control>> =>
            |this: &Rc<ControlTemplate>, _control: Ref<TemplatedControl>| {
                TemplateContent::try_load_as::<Ref<Control>>(this.content().as_ref()).map(|result| result.map(|result| result.result().clone()))
            },
    ],
});
