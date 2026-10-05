//! Port of `Xaml/TestControl.cs` and `Xaml/AttachedPropertyOwner.cs`.

use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_type, ferro_properties, ferro_static_type,
    instantiate, AttachedProperty, FerroObjectImpl, FerroProperty,
    Ref, StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_controls::{Control, ControlImpl};

/// The owner of the attached property `Double`.
pub struct AttachedPropertyOwner;

ferro_static_type!(AttachedPropertyOwner);

ferro_properties! {
    impl AttachedPropertyOwner {
        pub fn double_property() -> AttachedProperty<f64> {
            FerroProperty::register_attached::<AttachedPropertyOwner, Control, _>("Double", 0.0)
        }
    }
}

impl AttachedPropertyOwner {
    pub fn get_double(control: &Control) -> f64 {
        control.get_value(Self::double_property())
    }

    pub fn set_double(control: &Control, value: f64) {
        control.set_value(Self::double_property(), value)
    }
}

ferro_markup_type!(static AttachedPropertyOwner {
    type_info: AttachedPropertyOwner,
    namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
    methods: [
        static fn GetDouble(Ref<Control>) -> f64 => |control: Ref<Control>| AttachedPropertyOwner::get_double(&control),
        static fn SetDouble(Ref<Control>, f64) =>
            |control: Ref<Control>, value: f64| AttachedPropertyOwner::set_double(&control, value),
    ],
});

/// A control that adds itself as an owner of the attached property.
#[repr(C)]
pub struct TestControl {
    base: Control,
}

ferro_class!(TestControl: Control);
ferro_impl_classes!(
    TestControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);
ferro_class_info!(TestControl {
    new: TestControl::new,
    markup: { namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml" },
});

ferro_properties! {
    impl TestControl {
        pub fn double_property() -> StyledProperty<f64> {
            AttachedPropertyOwner::double_property().add_owner::<TestControl>()
        }
    }
}

impl TestControl {
    pub fn construct() -> Self {
        Self { base: Control::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn double(&self) -> f64 {
        self.get_value(Self::double_property())
    }

    pub fn set_double(&self, value: f64) {
        self.set_value(Self::double_property(), value)
    }
}

