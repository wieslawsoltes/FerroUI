//! Port of `Xaml/ControlThemeTests.cs`.

use std::rc::Rc;

use ferroui_base::media::{Brushes, IBrush, TextDecorations};
use ferroui_base::styling::{ControlTheme, Style};
use ferroui_base::{CornerRadius, Ref};
use ferroui_controls::{Border, ContentControl, TextBlock, Window};

use crate::support::app::styled_window_application;
use crate::support::helpers::{
    as_setter, assert_is_type, assert_value_is_type, setter_template_binding_property, value_of,
};
use crate::support::loader::load_as;
use crate::support::xaml::TestTemplatedControl;
use crate::support::TestViewModel;

/// The assertions the first three tests share: the theme gave the control
/// its template and the nested style of the theme its border the brush.
#[track_caller]
fn assert_theme_is_applied(button: &Ref<TestTemplatedControl>) {
    assert!(button.template().is_some());

    let children = button.get_visual_children();
    assert_eq!(1, children.len());
    let child = &children[0];
    assert_is_type::<Border>(child);
    let border = child.cast::<Border>().expect("the child is a Border");

    let red: Rc<dyn IBrush> = Brushes::red();
    assert_eq!(Some(red), border.background());
}

#[test]
fn control_theme_can_be_static_resource() {
    let _app = styled_window_application();
    let xaml = format!(
        "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:u='using:FerroUI.Markup.Xaml.UnitTests.Xaml'>
    <Window.Resources>
        {CONTROL_THEME_XAML}
    </Window.Resources>

    <u:TestTemplatedControl Theme='{{StaticResource MyTheme}}'/>
</Window>"
    );

    let window = load_as::<Ref<Window>>(&xaml);
    let button = assert_value_is_type::<TestTemplatedControl>(&window.content());

    window.show();

    assert_theme_is_applied(&button);
}

#[test]
fn control_theme_can_be_dynamic_resource() {
    let _app = styled_window_application();
    let xaml = format!(
        "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:u='using:FerroUI.Markup.Xaml.UnitTests.Xaml'>
    <Window.Resources>
        {CONTROL_THEME_XAML}
    </Window.Resources>

    <u:TestTemplatedControl Theme='{{DynamicResource MyTheme}}'/>
</Window>"
    );

    let window = load_as::<Ref<Window>>(&xaml);
    let button = assert_value_is_type::<TestTemplatedControl>(&window.content());

    window.show();

    assert_theme_is_applied(&button);
}

#[test]
fn control_theme_can_be_set_in_style() {
    let _app = styled_window_application();
    let xaml = format!(
        "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:u='using:FerroUI.Markup.Xaml.UnitTests.Xaml'>
    <Window.Resources>
        {CONTROL_THEME_XAML}
    </Window.Resources>

    <Window.Styles>
        <Style Selector='u|TestTemplatedControl'>
            <Setter Property='Theme' Value='{{StaticResource MyTheme}}'/>
        </Style>
    </Window.Styles>

    <u:TestTemplatedControl/>
</Window>"
    );

    let window = load_as::<Ref<Window>>(&xaml);
    let button = assert_value_is_type::<TestTemplatedControl>(&window.content());

    window.show();

    assert_theme_is_applied(&button);
}

#[test]
fn correctly_resolve_template_binding_in_nested_style() {
    let _app = styled_window_application();
    let xaml = "
<ControlTheme xmlns='https://github.com/ferroui'
              xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
              xmlns:u='using:FerroUI.Markup.Xaml.UnitTests.Xaml'
              TargetType='u:TestTemplatedControl'>
    <Setter Property='Template'>
        <ControlTemplate>
            <Border/>
        </ControlTemplate>
    </Setter>
    <Style Selector='^ /template/ Border'>
        <Setter Property='Tag' Value='{TemplateBinding TestData}'/>
    </Style>
</ControlTheme>";

    let theme = load_as::<Ref<ControlTheme>>(xaml);
    assert_eq!(1, theme.children().count(), "the collection does not contain exactly one style");
    let style = theme.children().get(0);
    let style = style.as_object().expect("the style is an object of the object model");
    assert_is_type::<Style>(style);
    let style = style.to_ref().cast::<Style>().expect("the style is a Style");
    assert_eq!(1, style.setters().count(), "the collection does not contain exactly one setter");
    let setter = style.setters().get(0);
    let setter = as_setter(&setter);

    assert!(
        Some(TestTemplatedControl::test_data_property().as_property()) == setter_template_binding_property(setter)
    );
}

#[test]
fn correctly_resolve_template_binding_in_theme_detached_template() {
    let _app = styled_window_application();
    let window = load_as::<Ref<Window>>(
        "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:u='using:FerroUI.Markup.Xaml.UnitTests.Xaml'>
    <Window.Resources>
        <ControlTheme x:Key='MyTheme' TargetType='ContentControl'>
            <Setter Property='CornerRadius' Value='10, 0, 0, 10' />
            <Setter Property='Content'>
                <Template>
                    <Border CornerRadius='{TemplateBinding CornerRadius}'/>
                </Template>
            </Setter>
            <Setter Property='Template'>
                <ControlTemplate>
                    <Button Content='{TemplateBinding Content}'
                            ContentTemplate='{TemplateBinding ContentTemplate}' />
                </ControlTemplate>
            </Setter>
        </ControlTheme>
    </Window.Resources>

    <ContentControl Theme='{StaticResource MyTheme}' />
</Window>",
    );
    let control = assert_value_is_type::<ContentControl>(&window.content());

    window.show();

    let border = assert_value_is_type::<Border>(&control.content());

    assert_eq!(CornerRadius::new(10.0, 0.0, 0.0, 10.0), border.corner_radius());
}

#[test]
fn can_use_classes_in_setter() {
    let _app = styled_window_application();
    let xaml = r#"<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:u='using:FerroUI.Markup.Xaml.UnitTests.Xaml'>
    <Window.Resources>
        <ControlTheme x:Key='MyTheme' TargetType='ContentControl'>
            <Setter Property='CornerRadius' Value='10, 0, 0, 10' />
            <Setter Property='(Classes.Banned)' Value='true'/>
            <Setter Property='Content'>
                <Template>
                    <Border CornerRadius='{TemplateBinding CornerRadius}'/>
                </Template>
            </Setter>
            <Setter Property='Template'>
                <ControlTemplate>
                    <Button Content='{TemplateBinding Content}'
                            ContentTemplate='{TemplateBinding ContentTemplate}' />
                </ControlTemplate>
            </Setter>

            <Style Selector='^.Banned'>
                <Setter Property="TextBlock.TextDecorations" Value="Strikethrough"/>
            </Style>
        </ControlTheme>
    </Window.Resources>
    <ContentControl Theme='{StaticResource MyTheme}' />
</Window>"#;

    let window = load_as::<Ref<Window>>(xaml);
    let control = assert_value_is_type::<ContentControl>(&window.content());
    assert!(Some(TextDecorations::strikethrough()) == control.get_value(TextBlock::text_decorations_property()));
}

#[test]
fn can_binding_classes_in_setter() {
    let _app = styled_window_application();
    let xaml = r#"<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:u='using:FerroUI.Markup.Xaml.UnitTests.Xaml'
        xmlns:vm='using:FerroUI.Markup.Xaml.UnitTests'>
    <Window.Resources>
        <ControlTheme x:Key='MyTheme' TargetType='ContentControl' x:DataType='vm:TestViewModel'>
            <Setter Property='CornerRadius' Value='10, 0, 0, 10' />
            <Setter Property='(Classes.Banned)' Value='{Binding Boolean}'/>
            <Setter Property='Content'>
                <Template>
                    <Border CornerRadius='{TemplateBinding CornerRadius}'/>
                </Template>
            </Setter>
            <Setter Property='Template'>
                <ControlTemplate>
                    <Button Content='{TemplateBinding Content}'
                            ContentTemplate='{TemplateBinding ContentTemplate}' />
                </ControlTemplate>
            </Setter>

            <Style Selector='^.Banned'>
                <Setter Property="TextBlock.TextDecorations" Value="Strikethrough"/>
            </Style>
        </ControlTheme>
    </Window.Resources>
    <Window.DataContext>
       <vm:TestViewModel/>
    </Window.DataContext>
    <ContentControl Theme='{StaticResource MyTheme}' />
</Window>"#;

    let window = load_as::<Ref<Window>>(xaml);
    window.apply_template();
    let vm = value_of::<Rc<TestViewModel>>(&window.data_context());
    let vm = vm.expect("the data context is a TestViewModel");
    let control = assert_value_is_type::<ContentControl>(&window.content());
    assert!(control.get_value(TextBlock::text_decorations_property()).is_none());
    vm.set_boolean(true);
    assert!(Some(TextDecorations::strikethrough()) == control.get_value(TextBlock::text_decorations_property()));
}

const CONTROL_THEME_XAML: &str = "
<ControlTheme x:Key='MyTheme' TargetType='u:TestTemplatedControl'>
    <Setter Property='Template'>
        <ControlTemplate>
            <Border/>
        </ControlTemplate>
    </Setter>
    <Style Selector='^ /template/ Border'>
        <Setter Property='Background' Value='Red'/>
    </Style>
</ControlTheme>";
