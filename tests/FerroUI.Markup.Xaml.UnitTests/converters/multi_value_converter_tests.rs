//! Port of `Converters/MultiValueConverterTests.cs`.

use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{TextBlock, Window};

use crate::support::app::styled_window_application;
use crate::support::converters::multi_value_converter_tests::TupleOfInt32;
use crate::support::helpers::assert_string;
use crate::support::loader::load_as;

#[test]
fn multi_value_converter_special_values_work() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:c='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Converters;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <TextBlock Name='textBlock'>
        <TextBlock.Tag>
            <MultiBinding Converter='{x:Static c:TestMultiValueConverter.Instance}' FallbackValue='bar'>
                <Binding Path='Item1' />
                <Binding Path='Item2' />
            </MultiBinding>
        </TextBlock.Tag>
    </TextBlock>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    window.apply_template();

    let data_context: BoxedValue = TupleOfInt32::create(2, 2);
    window.set_data_context(Some(data_context));
    assert_string("foo", &text_block.tag());

    let data_context: BoxedValue = TupleOfInt32::create(-3, 3);
    window.set_data_context(Some(data_context));
    assert_string("foo", &text_block.tag());

    let data_context: BoxedValue = TupleOfInt32::create(0, 2);
    window.set_data_context(Some(data_context));
    assert_string("bar", &text_block.tag());
}
