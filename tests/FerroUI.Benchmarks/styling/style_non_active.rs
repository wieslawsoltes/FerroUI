//! Activating and deactivating a style by adding and removing the class its
//! selector names, on a control whose properties all have local values: the
//! value of the style never becomes the value of the property.

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

pub struct StyleNonActive {
    target: Ref<StyleNonActiveTestClass>,
    /// The style attached to the target: what is attached refers to its
    /// style weakly.
    _style: Ref<Style>,
}

impl StyleNonActive {
    pub fn new() -> Self {
        StyleNonActiveTestClass::TYPE.ensure_class_init();

        let target = StyleNonActiveTestClass::new();

        let style = Style::with_setters(
            Selectors::of_type::<StyleNonActiveTestClass>().class("foo"),
            [Setter::new(StyleNonActiveTestClass::string_property(), Some("foo".to_string()))],
        );

        try_attach(&style, &target, None);
        target.set_value(StyleNonActiveTestClass::string_property(), Some("foo".to_string()));
        target.set_value(StyleNonActiveTestClass::struct1_property(), Struct1::new(1));
        target.set_value(StyleNonActiveTestClass::struct2_property(), Struct2::new(1));
        target.set_value(StyleNonActiveTestClass::struct3_property(), Struct3::new(1));
        target.set_value(StyleNonActiveTestClass::struct4_property(), Struct4::new(1));
        target.set_value(StyleNonActiveTestClass::struct5_property(), Struct5::new(1));
        target.set_value(StyleNonActiveTestClass::struct6_property(), Struct6::new(1));
        target.set_value(StyleNonActiveTestClass::struct7_property(), Struct7::new(1));
        target.set_value(StyleNonActiveTestClass::struct8_property(), Struct8::new(1));

        Self { target, _style: style }
    }

    pub fn toggle_non_active_style_activation(&self) {
        for _ in 0..100 {
            self.target.classes().add("foo");
            self.target.classes().remove("foo");
        }
    }
}

/// The control of the benchmark (`TestClass` upstream): a control with a
/// text property and eight properties of value types of growing size.
#[repr(C)]
struct StyleNonActiveTestClass {
    base: Control,
}

ferro_class!(StyleNonActiveTestClass: Control);
ferro_impl_classes!(
    StyleNonActiveTestClass: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

ferro_properties! {
    impl StyleNonActiveTestClass {
        pub fn string_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<StyleNonActiveTestClass, _>("String", None)
        }

        pub fn struct1_property() -> StyledProperty<Struct1> {
            FerroProperty::register::<StyleNonActiveTestClass, _>("Struct1", Struct1::default())
        }

        pub fn struct2_property() -> StyledProperty<Struct2> {
            FerroProperty::register::<StyleNonActiveTestClass, _>("Struct2", Struct2::default())
        }

        pub fn struct3_property() -> StyledProperty<Struct3> {
            FerroProperty::register::<StyleNonActiveTestClass, _>("Struct3", Struct3::default())
        }

        pub fn struct4_property() -> StyledProperty<Struct4> {
            FerroProperty::register::<StyleNonActiveTestClass, _>("Struct4", Struct4::default())
        }

        pub fn struct5_property() -> StyledProperty<Struct5> {
            FerroProperty::register::<StyleNonActiveTestClass, _>("Struct5", Struct5::default())
        }

        pub fn struct6_property() -> StyledProperty<Struct6> {
            FerroProperty::register::<StyleNonActiveTestClass, _>("Struct6", Struct6::default())
        }

        pub fn struct7_property() -> StyledProperty<Struct7> {
            FerroProperty::register::<StyleNonActiveTestClass, _>("Struct7", Struct7::default())
        }

        pub fn struct8_property() -> StyledProperty<Struct8> {
            FerroProperty::register::<StyleNonActiveTestClass, _>("Struct8", Struct8::default())
        }
    }
}

impl StyleNonActiveTestClass {
    fn construct() -> Self {
        Self { base: Control::construct() }
    }

    fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("styling", "StyleNonActive");
    class.benchmark("toggle_non_active_style_activation", "", StyleNonActive::new, |b| {
        b.toggle_non_active_style_activation()
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn style_non_active() {
        crate::harness::smoke_class(super::register, "StyleNonActive");
    }
}
