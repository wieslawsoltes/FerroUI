//! Keep native control templates, keyboard input and automation peers. The QML
//! transaction chooses the next state so one click updates the entire button group.
use ferroui_base::{
    ferro_class, ferro_impl_classes, input::InputElementImpl, instantiate,
    interactivity::InteractiveImpl, layout::LayoutableImpl, FerroObjectImpl, Ref, StaticType,
    StyledElementImpl, TypeInfo, VisualImpl,
};
use ferroui_controls::{
    primitives::{TemplatedControlImpl, ToggleButton, ToggleButtonImpl},
    ButtonImpl, CheckBox, ContentControlImpl, ControlImpl, RadioButton, ToggleSwitch,
};

macro_rules! quick_toggle {
    ($name:ident,$native:ident) => {
        #[repr(C)]
        pub(super) struct $name {base:$native}
        ferro_class!($name: $native);
        ferro_impl_classes!($name: FerroObjectImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl,
            ControlImpl, TemplatedControlImpl, ContentControlImpl, ButtonImpl);
        impl StyledElementImpl for $name {
            fn style_key_override(_: &Self) -> &'static TypeInfo { <$native as StaticType>::TYPE }
        }
        impl ToggleButtonImpl for $name {
            fn toggle(_: &Self) {
                // The inherited on_click raises Click after calling toggle.
                // Its QML handler performs the check and the public signals atomically.
            }
        }
        impl $name {
            pub(super) fn create() -> Ref<ToggleButton> {
                instantiate(Self { base: $native::construct() }).upcast()
            }
        }
    }
}
quick_toggle!(QuickCheck, CheckBox);
quick_toggle!(QuickRadio, RadioButton);
quick_toggle!(QuickSwitch, ToggleSwitch);
