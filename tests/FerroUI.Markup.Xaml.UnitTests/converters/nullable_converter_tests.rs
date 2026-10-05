//! Port of `Converters/NullableConverterTests.cs`.

use std::rc::Rc;

use ferroui_base::layout::Orientation;
use ferroui_base::Thickness;

use crate::support::app::{unit_test_application, TestServices};
use crate::support::converters::nullable_converter_tests::ClassWithNullableProperties;
use crate::support::loader::load_local_as;

#[test]
fn nullable_types_should_still_be_converted_properly() {
    // The services of the original are a mocked runtime platform only, which is not a service
    // of the test services of the port: the application starts without services.
    let _app = unit_test_application(TestServices::new());
    let xaml = "<ClassWithNullableProperties 
xmlns='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Converters'
    Thickness = '5' Orientation='Vertical'
></ClassWithNullableProperties>";
    let data = load_local_as::<Rc<ClassWithNullableProperties>>(xaml);
    assert_eq!(Some(Thickness::uniform(5.0)), data.thickness());
    assert_eq!(Some(Orientation::Vertical), data.orientation());
}
