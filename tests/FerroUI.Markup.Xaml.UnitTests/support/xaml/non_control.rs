//! Port of `Xaml/NonControl.cs`.

use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObject, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledProperty,
};
use ferroui_controls::Control;

/// An object of the object model that is not a control.
#[repr(C)]
pub struct NonControl {
    base: FerroObject,
}

ferro_class!(NonControl: FerroObject);
ferro_impl_classes!(NonControl: FerroObjectImpl);
ferro_class_info!(NonControl {
    new: NonControl::new,
    markup: { namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml" },
});

ferro_properties! {
    impl NonControl {
        pub fn control_property() -> StyledProperty<Option<Ref<Control>>> {
            FerroProperty::register::<NonControl, _>("Control", None)
        }

        pub fn string_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<NonControl, _>("String", None)
        }

        // A registered property without getter or setter.
        pub fn foo_property() -> StyledProperty<i32> {
            FerroProperty::register::<NonControl, _>("Foo", 0)
        }

        // A registered property with a getter only.
        pub fn bar_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<NonControl, _>("Bar", None)
        }
    }
}

impl NonControl {
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn control(&self) -> Option<Ref<Control>> {
        self.get_value(Self::control_property())
    }

    pub fn set_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::control_property(), value.into().0)
    }

    pub fn string(&self) -> Option<String> {
        self.get_value(Self::string_property())
    }

    pub fn set_string(&self, value: Option<String>) {
        self.set_value(Self::string_property(), value)
    }

    pub fn bar(&self) -> Option<String> {
        self.get_value(Self::bar_property())
    }
}
