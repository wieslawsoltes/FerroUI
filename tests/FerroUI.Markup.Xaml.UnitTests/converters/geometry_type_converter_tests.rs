//! Port of `Converters/GeometryTypeConverterTests.cs`.
//!
//! The rows of the theory `GeometryTypeConverter_Value_Work` are one test
//! each, in the order of the member data.

use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::shapes::Path;
use ferroui_controls::Window;

use crate::support::app::styled_window_application;
use crate::support::converters::geometry_type_converter_tests::{IntDataViewModel, StringDataViewModel};
use crate::support::loader::load_as;

fn geometry_type_converter_value_work(vm: BoxedValue, null_data: bool) {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:c='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Converters;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Path Name='path' Data='{Binding PathData}' Height='10' Width='10'/>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let path = window.get_control::<Path>("path");
    window.set_data_context(Some(vm));
    assert_eq!(null_data, path.data().is_none());
}

#[test]
fn geometry_type_converter_value_work_string_data_view_model_without_path_data() {
    let vm = StringDataViewModel::new();
    geometry_type_converter_value_work(vm, true);
}

#[test]
fn geometry_type_converter_value_work_string_data_view_model_with_path_data() {
    let vm = StringDataViewModel::new();
    vm.set_path_data(Some("M406.39,333.45l205.93,0".to_string()));
    geometry_type_converter_value_work(vm, false);
}

#[test]
fn geometry_type_converter_value_work_int_data_view_model_without_path_data() {
    let vm = IntDataViewModel::new();
    geometry_type_converter_value_work(vm, true);
}

#[test]
fn geometry_type_converter_value_work_int_data_view_model_with_path_data() {
    let vm = IntDataViewModel::new();
    vm.set_path_data(100);
    geometry_type_converter_value_work(vm, true);
}
