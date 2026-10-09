use ferroui_base::{ferro_class, ferro_properties, Control, FerroProperty, Ref, StyledProperty, StyledPropertyOptions};

/// A control with one child.
#[repr(C)]
pub struct Decorator {
    base: Control,
}

ferro_class!(Decorator: Control);
ferroui_base::ferro_class_info!(Decorator { new: Decorator::new });

ferro_properties! {
    impl Decorator {
        /// Defines the `Child` property.
        pub fn child_property() -> StyledProperty<Option<Ref<Control>>> {
            FerroProperty::register::<Decorator, _>("Child", None)
        }

        /// Defines the `Padding` property. `Thickness` is not imported here.
        pub fn padding_property() -> StyledProperty<Thickness> {
            let property = FerroProperty::register_with::<Self, _>(
                "Padding",
                StyledPropertyOptions::new(Thickness::default()).inherits(true).assign_binding(true),
            );
            property
        }
    }
}

impl Decorator {
    pub fn new() -> Ref<Decorator> {
        ferroui_base::instantiate(Decorator { base: Control::construct() })
    }
}
