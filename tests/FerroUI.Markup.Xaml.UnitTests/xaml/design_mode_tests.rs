//! Port of `Xaml/DesignModeTests.cs`.

use std::rc::Rc;

use ferroui_base::controls::ResourceDictionary;
use ferroui_base::media::Colors;
use ferroui_base::styling::Style;
use ferroui_base::Ref;
use ferroui_controls::{Border, Button, ContentControl, Control, Design, PreviewTarget, UserControl, Window};
use ferroui_markup_xaml::templates::DataTemplate;

use crate::support::app::{mock_windowing_platform_application, xaml_test_base};
use crate::support::helpers::{assert_is_type, assert_string, boxed_str, string_of};
use crate::support::loader::{cast, expect_loaded, test_assembly, try_load_with};
use crate::support::xaml::design_mode_tests::DesignModeTests;

#[test]
fn design_mode_preview_with_should_be_ignored_without_design_mode() {
    let _app = mock_windowing_platform_application();
    let obj = cast::<Ref<Control>>(&expect_loaded(try_load_with(
        "
<Button xmlns='https://github.com/ferroui'>
    <Design.PreviewWith>
        <Template>
            <Border />
        </Template>
    </Design.PreviewWith>
</Button>",
        None,
        None,
        None,
        false,
    )));
    let preview = Design::create_preview_with_control(&PreviewTarget::Object(obj.upcast()))
        .expect("a preview control is created");
    // Should return the original control, not the preview.
    assert_is_type::<Button>(&preview);
}

#[test]
fn design_mode_preview_with_returns_original_control() {
    let _app = mock_windowing_platform_application();
    let obj = cast::<Ref<Control>>(&expect_loaded(try_load_with(
        "
<Button xmlns='https://github.com/ferroui'>
    <Design.PreviewWith>
        <Border />
    </Design.PreviewWith>
</Button>",
        None,
        None,
        None,
        true,
    )));
    let preview = Design::create_preview_with_control(&PreviewTarget::Object(obj.clone().upcast()))
        .expect("a preview control is created");
    // Should return the original control, not the preview, as this is not supported to avoid stack overflows.
    assert!(obj.ptr_eq(&preview));
}

#[test]
fn design_mode_preview_with_works_with_style() {
    let _app = mock_windowing_platform_application();
    let obj = cast::<Ref<Style>>(&expect_loaded(try_load_with(
        "
<Style xmlns='https://github.com/ferroui'
       xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
       Selector='Border.preview-border' >
    <Design.PreviewWith>
        <Border Classes='preview-border' />
    </Design.PreviewWith>
    <Setter Property='Background' Value='Red'/>
</Style>",
        None,
        None,
        None,
        true,
    )));
    let preview = Design::create_preview_with_control(&PreviewTarget::Object(obj.upcast()))
        .expect("a preview control is created");
    assert_is_type::<Border>(&preview);
    let preview_border = preview.cast::<Border>().expect("the preview is a Border");
    preview_border.apply_styling();
    assert_eq!(
        Some(Colors::RED),
        preview_border.background().and_then(|brush| brush.as_solid_color_brush().map(|brush| brush.color()))
    );
}

#[test]
fn design_mode_preview_with_works_with_resource_dictionary() {
    let _app = mock_windowing_platform_application();
    let obj = cast::<Ref<ResourceDictionary>>(&expect_loaded(try_load_with(
        "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Design.PreviewWith>
        <Border Background='{DynamicResource PreviewBackground}' />
    </Design.PreviewWith>
    <SolidColorBrush x:Key='PreviewBackground' Color='Red'/>
</ResourceDictionary>",
        None,
        None,
        None,
        true,
    )));
    let preview = Design::create_preview_with_control(&PreviewTarget::Object(obj.upcast()))
        .expect("a preview control is created");
    assert_is_type::<Border>(&preview);
    let preview_border = preview.cast::<Border>().expect("the preview is a Border");
    assert_eq!(
        Some(Colors::RED),
        preview_border.background().and_then(|brush| brush.as_solid_color_brush().map(|brush| brush.color()))
    );
}

#[test]
fn design_mode_preview_with_works_with_i_data_template() {
    let _app = mock_windowing_platform_application();
    let obj = cast::<Rc<DataTemplate>>(&expect_loaded(try_load_with(
        "
<DataTemplate xmlns='https://github.com/ferroui'
              xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
              x:DataType='SolidColorBrush'>
    <Design.PreviewWith>
        <ContentControl>
            <ContentControl.Content>
                <SolidColorBrush Color='Red'/>
            </ContentControl.Content>
        </ContentControl>
    </Design.PreviewWith>
    <Border Background='{Binding}' />
</DataTemplate>",
        None,
        None,
        None,
        true,
    )));
    let preview = Design::create_preview_with_control(&PreviewTarget::DataTemplate(obj.as_data_template()))
        .expect("a preview control is created");
    assert_is_type::<ContentControl>(&preview);
    let preview_content_control = preview.cast::<ContentControl>().expect("the preview is a ContentControl");
    preview_content_control.apply_template();
    preview_content_control.presenter().expect("the content control has a presenter").update_child();
    let border = preview_content_control.find_descendant_of_type::<Border>(false);
    assert!(border.is_some());
    assert_eq!(
        Some(Colors::RED),
        border.unwrap().background().and_then(|brush| brush.as_solid_color_brush().map(|brush| brush.color()))
    );
}

#[test]
fn design_mode_properties_should_be_ignored_at_runtime_and_set_in_design_mode() {
    let _app = mock_windowing_platform_application();
    for design_mode in [true, false] {
        let obj = cast::<Ref<Window>>(&expect_loaded(try_load_with(
            "
<Window xmlns='https://github.com/ferroui' 
        xmlns:d='http://schemas.microsoft.com/expression/blend/2008'
        xmlns:mc='http://schemas.openxmlformats.org/markup-compatibility/2006'
        mc:Ignorable='d'
        d:DataContext='data-context'
        d:DesignWidth='123'
        d:DesignHeight='321'>
</Window>",
            None,
            None,
            None,
            design_mode,
        )));
        let context = Design::get_data_context(&obj);
        let width = Design::get_width(&obj);
        let height = Design::get_height(&obj);
        if design_mode {
            assert_string("data-context", &context);
            assert_eq!(123.0, width);
            assert_eq!(321.0, height);
        } else {
            assert!(!obj.is_set(Design::data_context_property()));
            assert!(!obj.is_set(Design::width_property()));
            assert!(!obj.is_set(Design::height_property()));
        }
    }
}

// The original issue: number 2570 of the upstream repository.
#[test]
fn design_mode_throws_on_invalid_static_property_reference() {
    let _base = xaml_test_base();
    DesignModeTests::set_some_static_property(boxed_str("123"));
    let ex = try_load_with(
        "
<UserControl 
    xmlns='https://github.com/ferroui'
    xmlns:d='http://schemas.microsoft.com/expression/blend/2008'
    xmlns:tests='using:FerroUI.Markup.Xaml.UnitTests.Xaml'
    d:DataContext='{x:Static tests:DesignModeTests.SomeStaticPropery}'
    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'/>",
        Some(test_assembly()),
        None,
        None,
        true,
    )
    .expect_err("the load must fail");
    assert!(ex.message().contains("Unable to resolve "), "{}", ex.message());
    assert!(ex.message().contains(" as static field, property, constant or enum value"), "{}", ex.message());
}

#[test]
fn design_mode_data_context_should_be_set() {
    let _base = xaml_test_base();
    DesignModeTests::set_some_static_property(boxed_str("123"));

    let loaded = cast::<Ref<UserControl>>(&expect_loaded(try_load_with(
        "
<UserControl 
    xmlns='https://github.com/ferroui'
    xmlns:d='http://schemas.microsoft.com/expression/blend/2008'
    xmlns:tests='using:FerroUI.Markup.Xaml.UnitTests.Xaml'
    d:DataContext='{x:Static tests:DesignModeTests.SomeStaticProperty}'
    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'/>",
        Some(test_assembly()),
        None,
        None,
        true,
    )));
    assert_eq!(string_of(&Design::get_data_context(&loaded)), string_of(&DesignModeTests::some_static_property()));
    assert!(Design::get_data_context(&loaded).is_some());
}
