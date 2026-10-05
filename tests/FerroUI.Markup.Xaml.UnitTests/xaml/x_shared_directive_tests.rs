//! Port of `Xaml/XSharedDirectiveTests.cs`.

use ferroui_base::Ref;
use ferroui_controls::{ColumnDefinitions, Window};

use crate::support::app::styled_window_application;
use crate::support::helpers::value_of;
use crate::support::loader::load_as;

#[test]
fn should_create_new_instance_where_x_share_is_false() {
    let _app = styled_window_application();
    let xaml = r#"<Window xmlns="https://github.com/ferroui"
        xmlns:sys="clr-namespace:System;assembly=netstandard"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
    <Window.Resources>
        <ColumnDefinitions x:Key="ImplicitSharedResource">
            <ColumnDefinition Width="150" />
            <ColumnDefinition Width="10" />
            <ColumnDefinition Width="Auto" />
         </ColumnDefinitions>
         <ColumnDefinitions x:Key="NotSharedResource"
                            x:Shared="false">
            <ColumnDefinition Width="150" />
            <ColumnDefinition Width="10" />
            <ColumnDefinition Width="Auto" />
         </ColumnDefinitions>
    </Window.Resources>
</Window>"#;
    let window = load_as::<Ref<Window>>(xaml);
    window.apply_template();

    let implicit_shared_instance1 = window.find_resource(&"ImplicitSharedResource".into());
    assert!(implicit_shared_instance1.is_some());
    let implicit_shared_instance2 = window.find_resource(&"ImplicitSharedResource".into());
    assert!(implicit_shared_instance2.is_some());

    assert!(implicit_shared_instance1 == implicit_shared_instance2);

    let not_shared_resource1 = window.find_resource(&"NotSharedResource".into());
    assert!(not_shared_resource1.is_some());

    let not_shared_resource2 = window.find_resource(&"NotSharedResource".into());
    assert!(not_shared_resource2.is_some());

    assert!(not_shared_resource1 != not_shared_resource2);

    let not_shared_resource1 = value_of::<ColumnDefinitions>(&not_shared_resource1).expect("column definitions");
    let not_shared_resource2 = value_of::<ColumnDefinitions>(&not_shared_resource2).expect("column definitions");
    assert_eq!(not_shared_resource1.to_string(), not_shared_resource2.to_string());
}
