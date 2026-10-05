use crate::NativeMenu;
use ferroui_base::{ferro_properties, ferro_static_type, AttachedProperty, FerroObject, FerroProperty, Ref};

/// Allows native menu support on platforms where a [`NativeMenu`] can be
/// attached to the dock.
pub struct NativeDock;

ferro_static_type!(NativeDock);

ferro_properties! {
    impl NativeDock {
        /// Defines the Menu attached property.
        pub fn menu_property() -> AttachedProperty<Option<Ref<NativeMenu>>> {
            FerroProperty::register_attached::<NativeDock, FerroObject, _>("Menu", None)
        }
    }
}

impl NativeDock {
    /// Sets the value of the attached [`menu_property`](Self::menu_property).
    pub fn set_menu(o: &FerroObject, menu: Option<Ref<NativeMenu>>) {
        o.set_value(Self::menu_property(), menu)
    }

    /// Gets the value of the attached [`menu_property`](Self::menu_property).
    pub fn get_menu(o: &FerroObject) -> Option<Ref<NativeMenu>> {
        o.get_value(Self::menu_property())
    }
}
