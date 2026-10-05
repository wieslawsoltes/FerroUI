//! Ported from the upstream `Xaml/AssignBindingTests`.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::data::BindingBase;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, AttachedProperty,
    FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, StyledPropertyOptions, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};

use crate::support::app::*;
use crate::support::loader::*;
use crate::support::TypeModule;

// --- test types -------------------------------------------------------------

/// A control with a plain property and an attached property that are
/// assigned the bindings given for them in markup.
#[repr(C)]
pub struct AssignBindingTestControl {
    base: Control,
    clr_binding: RefCell<Option<Rc<dyn BindingBase>>>,
}

ferro_class!(AssignBindingTestControl: Control);
ferro_impl_classes!(
    AssignBindingTestControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);
ferro_class_info!(AssignBindingTestControl {
    new: AssignBindingTestControl::new,
    markup: {
        properties: [
            ClrBinding: Option<Rc<dyn BindingBase>> {
                get: |this: &Ref<AssignBindingTestControl>| this.clr_binding(),
                set: |this: &Ref<AssignBindingTestControl>, value: Option<Rc<dyn BindingBase>>| this.set_clr_binding(value)
            } [AssignBinding],
        ],
        methods: [
            static fn GetAttachedBinding(Ref<Control>) -> Option<Rc<dyn BindingBase>> =>
                |obj: Ref<Control>| AssignBindingTestControl::get_attached_binding(&obj),
            static fn SetAttachedBinding(Ref<Control>, Option<Rc<dyn BindingBase>>) =>
                |obj: Ref<Control>, value: Option<Rc<dyn BindingBase>>| {
                    AssignBindingTestControl::set_attached_binding(&obj, value)
                },
        ],
    },
});

ferro_properties! {
    impl AssignBindingTestControl {
        // The attribute of the upstream getter (`[AssignBinding] GetAttachedBinding`) is part of
        // the definition of a registered property.
        pub fn attached_binding_property() -> AttachedProperty<Option<Rc<dyn BindingBase>>> {
            FerroProperty::register_attached_with::<AssignBindingTestControl, Control, _>(
                "AttachedBinding",
                StyledPropertyOptions::new(None).assign_binding(true),
            )
        }
    }
}

impl AssignBindingTestControl {
    pub fn construct() -> Self {
        Self { base: Control::construct(), clr_binding: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn clr_binding(&self) -> Option<Rc<dyn BindingBase>> {
        self.clr_binding.borrow().clone()
    }

    pub fn set_clr_binding(&self, value: Option<Rc<dyn BindingBase>>) {
        self.clr_binding.replace(value);
    }

    pub fn get_attached_binding(obj: &Control) -> Option<Rc<dyn BindingBase>> {
        obj.get_value(Self::attached_binding_property())
    }

    pub fn set_attached_binding(obj: &Control, value: Option<Rc<dyn BindingBase>>) {
        obj.set_value(Self::attached_binding_property(), value)
    }
}

/// The test types of this file.
pub(crate) const MODULE: TypeModule =
    TypeModule { types: &[AssignBindingTestControl::TYPE], markup_types: &[], value_types: || {} };

// --- tests ------------------------------------------------------------------

#[test]
fn assign_binding_works_with_clr_property() {
    let _app = styled_window_application();

    let control: Ref<AssignBindingTestControl> = load_as(
        r#"
            <local:AssignBindingTestControl
                xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
                xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'
                ClrBinding='{Binding SomePath}' />
            "#,
    );

    assert!(control.clr_binding().is_some());
}

#[test]
fn assign_binding_works_with_attached_property() {
    let _app = styled_window_application();

    let control: Ref<Control> = load_as(
        r#"
            <Control
                xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
                xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'
                local:AssignBindingTestControl.AttachedBinding='{Binding SomePath}' />
            "#,
    );

    let binding = AssignBindingTestControl::get_attached_binding(&control);
    assert!(binding.is_some());
}
