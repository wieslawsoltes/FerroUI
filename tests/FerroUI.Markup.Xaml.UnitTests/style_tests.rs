//! Port of `StyleTests.cs (project root)`.

use std::rc::Rc;

use ferroui_base::data::{BindingBase, BindingMode, ReflectionBinding};
use ferroui_base::styling::testing::try_attach;
use ferroui_base::styling::{Setter, Style};
use ferroui_base::{BoxedValue, Ref, Thickness};
use ferroui_controls::TextBox;
use ferroui_markup::data::Binding;

use crate::support::app::{unit_test_application, TestServices};
use crate::support::helpers::{as_setter, assert_binding_is_type, assert_value, is_type, setter_binding, setter_plain_value};
use crate::support::loader::load_as;
use crate::support::style_tests::Data;

#[test]
fn binding_as_attribute_should_be_assigned_to_setter_value_instead_of_bound() {
    let _app = unit_test_application(TestServices::mock_platform_wrapper());
    let xaml = "<Style Selector='Button' xmlns='https://github.com/ferroui'><Setter Property='Content' Value='{Binding}'/></Style>";
    let style = load_as::<Ref<Style>>(xaml);
    let setter = style.setters().get(0);
    let setter = as_setter(&setter);

    assert_binding_is_type::<ReflectionBinding>(&setter_binding(setter));
}

/// The body of the theory `Binding_As_Element_Should_Be_Assigned_To_Setter_Value`.
#[track_caller]
fn binding_as_element_should_be_assigned_to_setter_value(property_name: &str) {
    let _app = unit_test_application(TestServices::mock_platform_wrapper());
    let style = load_as::<Ref<Style>>(&format!(
        r#"<Style Selector="Button" xmlns="https://github.com/ferroui">
    <Setter Property="{property_name}">
        <Binding />
    </Setter>
</Style>"#
    ));
    let setter = style.setters().get(0);
    let setter = as_setter(&setter);

    assert_binding_is_type::<ReflectionBinding>(&setter_binding(setter));
}

/// `[InlineData(nameof(ContentControl.Content))]`: standard property.
#[test]
fn binding_as_element_should_be_assigned_to_setter_value_content() {
    binding_as_element_should_be_assigned_to_setter_value("Content");
}

/// `[InlineData(nameof(Layoutable.Margin))]`: primitive property which can
/// be directly parsed.
#[test]
fn binding_as_element_should_be_assigned_to_setter_value_margin() {
    binding_as_element_should_be_assigned_to_setter_value("Margin");
}

#[test]
fn xml_value_should_be_assigned_to_setter_value() {
    let _app = unit_test_application(TestServices::mock_platform_wrapper());
    let style = load_as::<Ref<Style>>(
        "
<Style Selector='Button' xmlns='https://github.com/ferroui'>
    <Setter Property='Margin'>
        10, 4, 0, 4
    </Setter>
</Style>",
    );
    let setter = style.setters().get(0);
    let setter = as_setter(&setter);

    let value = setter_plain_value(setter);
    assert!(is_type::<Thickness>(&value), "the value of the setter is not a Thickness");
    assert_value(Thickness::new(10.0, 4.0, 0.0, 4.0), &value);
}

#[test]
fn setter_with_two_way_binding_should_update_source() {
    let _app = unit_test_application(TestServices::mock_threading_interface());
    let data = Data::new();
    data.set_foo(Some("foo".to_string()));

    let control = TextBox::new();
    let data_context: BoxedValue = data.clone();
    control.set_data_context(Some(data_context));

    let style = Style::new();
    let binding: Rc<dyn BindingBase> = Binding::with_path_and_mode("Foo", BindingMode::TwoWay);
    style.setters().add(Setter::new_binding_base(TextBox::text_property().as_property(), binding));

    try_attach(&style, &control, None);
    assert_eq!(Some("foo".to_string()), control.text());

    control.set_text(Some("bar"));
    assert_eq!(Some("bar".to_string()), data.foo());
}
