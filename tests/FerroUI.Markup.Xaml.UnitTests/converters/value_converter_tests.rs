//! Port of `Converters/ValueConverterTests.cs`.

use ferroui_base::Ref;
use ferroui_controls::{TextBlock, Window};

use crate::support::app::styled_window_application;
use crate::support::helpers::boxed;
use crate::support::loader::load_as;

#[test]
fn value_converter_special_values_work() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:c='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Converters;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <TextBlock Name='textBlock' Text='{Binding Converter={x:Static c:TestConverter.Instance}, FallbackValue=bar}'/>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    window.apply_template();

    window.set_data_context(Some(boxed(2i32)));
    assert_eq!(Some("foo".to_string()), text_block.text());

    window.set_data_context(Some(boxed(-3i32)));
    assert_eq!(Some("foo".to_string()), text_block.text());

    window.set_data_context(Some(boxed(0i32)));
    assert_eq!(Some("bar".to_string()), text_block.text());
}
