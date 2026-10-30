//! Port of `Controls/CustomControl.xaml.cs` (the template of the control is in the styles of
//! `Controls/CustomControl.xaml`, which the application includes).

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_controls::primitives::{TemplatedControl, TemplatedControlImpl};
use ferroui_controls::ControlImpl;

/// A templated control with a menu that opens and closes (`IsMenuOpen`): its template changes the
/// columns of its grid with data triggers bound to the property with a template binding.
#[repr(C)]
pub struct CustomControl {
    base: TemplatedControl,
}

ferro_class!(CustomControl: TemplatedControl);
ferro_class_info!(CustomControl {
    new: CustomControl::new
});
ferro_impl_classes!(
    CustomControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

ferro_properties! {
    impl CustomControl {
        /// Defines the `IsMenuOpen` property.
        pub fn is_menu_open_property() -> StyledProperty<bool> {
            FerroProperty::register::<CustomControl, _>("IsMenuOpen", false)
        }
    }
}

impl CustomControl {
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// `IsMenuOpen`.
    pub fn is_menu_open(&self) -> bool {
        self.get_value(Self::is_menu_open_property())
    }

    /// Sets `IsMenuOpen`.
    pub fn set_is_menu_open(&self, value: bool) {
        self.set_value(Self::is_menu_open_property(), value)
    }
}
