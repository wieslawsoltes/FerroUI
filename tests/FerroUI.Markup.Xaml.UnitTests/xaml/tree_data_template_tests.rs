//! Port of `Xaml/TreeDataTemplateTests.cs`.

use ferroui_base::data::core::ValueType;
use ferroui_base::data::ReflectionBinding;
use ferroui_controls::templates::DataTemplates;
use ferroui_markup_xaml::templates::TreeDataTemplate;

use crate::support::app::{unit_test_application, TestServices};
use crate::support::helpers::assert_binding_is_type;
use crate::support::loader::load_as;

#[test]
fn binding_should_be_assigned_to_items_source_instead_of_bound() {
    let _app = unit_test_application(TestServices::mock_platform_wrapper());
    let xaml = "<DataTemplates xmlns='https://github.com/ferroui'><TreeDataTemplate DataType='Control' ItemsSource='{Binding}'/></DataTemplates>";
    let templates = load_as::<DataTemplates>(xaml);
    let first = templates.get(0);
    let template = first
        .as_any()
        .and_then(|template| template.downcast_ref::<TreeDataTemplate>())
        .expect("the first data template is not a TreeDataTemplate");

    let items_source = template.items_source().expect("the items source is null");
    assert_binding_is_type::<ReflectionBinding>(&items_source);
}

#[test]
fn xdata_type_should_be_assigned_to_clr_property() {
    let _app = unit_test_application(TestServices::mock_platform_wrapper());
    let xaml = "
<DataTemplates xmlns='https://github.com/ferroui'
               xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <TreeDataTemplate x:DataType='x:String' />
</DataTemplates>";
    let templates = load_as::<DataTemplates>(xaml);
    let first = templates.get(0);
    let template = first
        .as_any()
        .and_then(|template| template.downcast_ref::<TreeDataTemplate>())
        .expect("the first data template is not a TreeDataTemplate");

    assert!(template.data_type() == Some(ValueType::of::<String>()));
}
