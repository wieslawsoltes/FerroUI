//! Port of `Templates/DataTemplateTests.cs`.

use ferroui_base::data::core::ValueType;
use ferroui_base::BoxedValue;
use ferroui_markup_xaml::templates::DataTemplate;

use crate::support::app::xaml_test_base;
use crate::support::templates::data_template_tests::{Class1, Class2};

#[test]
fn data_template_should_match_data_of_type() {
    let _base = xaml_test_base();
    let target = DataTemplate::new();
    target.set_data_type(Some(ValueType::of::<Class1>()));
    let data: BoxedValue = Class1::new();

    assert!(target.match_(Some(&data)));
}

#[test]
fn data_template_should_match_data_of_derived_type() {
    let _base = xaml_test_base();
    let target = DataTemplate::new();
    target.set_data_type(Some(ValueType::of::<Class1>()));
    let data: BoxedValue = Class2::new();

    assert!(target.match_(Some(&data)));
}
