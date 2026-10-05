//! Port of `Xaml/TestTemplatedControl.cs`.

use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, FerroObjectImpl,
    FerroProperty, Ref, StyledElementImpl, StyledProperty,
    VisualImpl,
};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_controls::primitives::{TemplatedControl, TemplatedControlImpl};
use ferroui_controls::ControlImpl;

/// A templated control with an untyped property.
#[repr(C)]
pub struct TestTemplatedControl {
    base: TemplatedControl,
}

ferro_class!(TestTemplatedControl: TemplatedControl);
ferro_impl_classes!(
    TestTemplatedControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);
ferro_class_info!(TestTemplatedControl {
    new: TestTemplatedControl::new,
    markup: { namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml" },
});

ferro_properties! {
    impl TestTemplatedControl {
        pub fn test_data_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<TestTemplatedControl, _>("TestData", None)
        }
    }
}

impl TestTemplatedControl {
    pub fn construct() -> Self {
        Self { base: TemplatedControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn test_data(&self) -> Option<BoxedValue> {
        self.get_value(Self::test_data_property())
    }

    pub fn set_test_data(&self, value: Option<BoxedValue>) {
        self.set_value(Self::test_data_property(), value)
    }
}
