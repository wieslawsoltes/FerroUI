//! Port of `Xaml/IgnoredDirectivesTests.cs`.

use ferroui_base::Ref;
use ferroui_controls::{TextBlock, Window};

use crate::support::app::styled_window_application;
use crate::support::loader::load_as;

#[test]
fn ignored_directives_should_compile() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:sys='clr-namespace:System;assembly=netstandard'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <TextBlock x:Name='target' x:FieldModifier='Public' Text='Foo'/>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let target = window.get_control::<TextBlock>("target");

    window.apply_template();
    target.apply_template();

    assert_eq!(Some("Foo".to_string()), target.text());
}
