//! Activating and deactivating a style by adding and removing the class its
//! selector names, on a control whose other values are all unset.

use crate::harness::Registry;
use crate::test_types::{Struct1, Struct2, Struct3, Struct4, Struct5, Struct6, Struct7, Struct8};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::styling::testing::try_attach;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};

pub struct StyleActivation {
    target: Ref<StyleActivationTestClass>,
    /// The style attached to the target: what is attached refers to its
    /// style weakly.
    _style: Ref<Style>,
}

impl StyleActivation {
    pub fn new() -> Self {
        StyleActivationTestClass::TYPE.ensure_class_init();

        let target = StyleActivationTestClass::new();

        let style = Style::with_setters(
            Selectors::of_type::<StyleActivationTestClass>().class("foo"),
            [Setter::new(StyleActivationTestClass::string_property(), Some("foo".to_string()))],
        );
        try_attach(&style, &target, None);

        Self { target, _style: style }
    }

    pub fn toggle_style_activation_via_class(&self) {
        for _ in 0..100 {
            self.target.classes().add("foo");
            self.target.classes().remove("foo");
        }
    }
}

/// The control of the benchmark (`TestClass` upstream): a control with a
/// text property and eight properties of value types of growing size.
#[repr(C)]
struct StyleActivationTestClass {
    base: Control,
}

ferro_class!(StyleActivationTestClass: Control);
ferro_impl_classes!(
    StyleActivationTestClass: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

ferro_properties! {
    impl StyleActivationTestClass {
        pub fn string_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<StyleActivationTestClass, _>("String", None)
        }

        pub fn struct1_property() -> StyledProperty<Struct1> {
            FerroProperty::register::<StyleActivationTestClass, _>("Struct1", Struct1::default())
        }

        pub fn struct2_property() -> StyledProperty<Struct2> {
            FerroProperty::register::<StyleActivationTestClass, _>("Struct2", Struct2::default())
        }

        pub fn struct3_property() -> StyledProperty<Struct3> {
            FerroProperty::register::<StyleActivationTestClass, _>("Struct3", Struct3::default())
        }

        pub fn struct4_property() -> StyledProperty<Struct4> {
            FerroProperty::register::<StyleActivationTestClass, _>("Struct4", Struct4::default())
        }

        pub fn struct5_property() -> StyledProperty<Struct5> {
            FerroProperty::register::<StyleActivationTestClass, _>("Struct5", Struct5::default())
        }

        pub fn struct6_property() -> StyledProperty<Struct6> {
            FerroProperty::register::<StyleActivationTestClass, _>("Struct6", Struct6::default())
        }

        pub fn struct7_property() -> StyledProperty<Struct7> {
            FerroProperty::register::<StyleActivationTestClass, _>("Struct7", Struct7::default())
        }

        pub fn struct8_property() -> StyledProperty<Struct8> {
            FerroProperty::register::<StyleActivationTestClass, _>("Struct8", Struct8::default())
        }
    }
}

impl StyleActivationTestClass {
    fn construct() -> Self {
        Self { base: Control::construct() }
    }

    fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("styling", "StyleActivation");
    class.benchmark("toggle_style_activation_via_class", "", StyleActivation::new, |b| {
        b.toggle_style_activation_via_class()
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn style_activation() {
        crate::harness::smoke_class(super::register, "StyleActivation");
    }
}
