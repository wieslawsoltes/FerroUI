//! Ported from the upstream `MarkupExtensions/StaticResourceExtensionTests`.

use std::rc::Rc;

use ferroui_base::controls::ResourceKey;
use ferroui_base::media::{Color, Colors, IBrush, SolidColorBrush};
use ferroui_base::styling::{styles_as_style, IStyle, Selectors, Setter, Style, Styles};
use ferroui_base::{ObjectType, Ref, Visual};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use ferroui_controls::testing::UnitTestApplicationScope;
use ferroui_controls::{
    Application, Border, Button, ContentControl, Control, Grid, ListBox, TextBlock, UserControl, Window,
};

use crate::support::app::*;
use crate::support::helpers::*;
use crate::support::loader::*;
use crate::support::TypeModule;

/// The test types of this file.
pub(crate) const MODULE: TypeModule = TypeModule::EMPTY;

// --- helpers ----------------------------------------------------------------

/// `Assert.IsAssignableFrom<ISolidColorBrush>(brush).Color`.
#[track_caller]
fn solid_color_brush_color(brush: &Option<Rc<dyn IBrush>>) -> Color {
    let brush = brush.as_ref().expect("the brush is null");
    brush.as_solid_color_brush().expect("the brush is not an ISolidColorBrush").color()
}

/// `Assert.IsAssignableFrom<SolidColorBrush>(brush).Color`.
#[track_caller]
fn mutable_solid_color_brush_color(brush: &Option<Rc<dyn IBrush>>) -> Color {
    let brush = brush.as_ref().expect("the brush is null");
    let object = brush.as_object().expect("the brush is not a SolidColorBrush");
    object.downcast_ref::<SolidColorBrush>().expect("the brush is not a SolidColorBrush").color()
}

/// `(T)visuals.Single()`.
#[track_caller]
fn single<T: ObjectType>(visuals: impl IntoIterator<Item = Ref<Visual>>) -> Ref<T> {
    let visuals: Vec<Ref<Visual>> = visuals.into_iter().collect();
    assert_eq!(1, visuals.len(), "the sequence does not contain exactly one element");
    visuals[0].cast::<T>().unwrap_or_else(|| panic!("the visual is not a '{}'", std::any::type_name::<T>()))
}

/// `visuals.OfType<T>().Single()`.
#[track_caller]
fn single_of_type<T: ObjectType>(visuals: impl IntoIterator<Item = Ref<Visual>>) -> Ref<T> {
    let matches: Vec<Ref<T>> = visuals.into_iter().filter_map(|visual| visual.cast::<T>()).collect();
    assert_eq!(1, matches.len(), "the sequence does not contain exactly one '{}'", std::any::type_name::<T>());
    matches[0].clone()
}

/// `Assert.IsType<Window>(compiled[index])`.
#[track_caller]
fn window_of(compiled: &[Option<ferroui_base::BoxedValue>], index: usize) -> Ref<Window> {
    let window: Ref<Window> = group_item(compiled, index);
    assert_is_type::<Window>(&window);
    window
}

fn styled_window() -> UnitTestApplicationScope {
    let services = TestServices::styled_window().with_theme(|| -> Rc<dyn IStyle> {
        let styles = Styles::new();
        styles.add(window_style());
        styles_as_style(&styles)
    });

    unit_test_application(services)
}

fn window_style() -> Ref<Style> {
    let template: Rc<dyn IControlTemplate> = FuncControlTemplate::for_type::<Window>(|x, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        presenter.bind_indexer(
            &ContentPresenter::content_property().bind(),
            &x.indexer(&ContentControl::content_property().bind()),
        );
        presenter.register_in_name_scope(&**scope).upcast()
    });

    Style::with_setters(Selectors::of_type::<Window>(), [Setter::new(TemplatedControl::template_property(), Some(template))])
}

// --- tests ------------------------------------------------------------------

#[test]
fn static_resource_can_be_assigned_to_property() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Resources>
        <SolidColorBrush x:Key='brush'>#ff506070</SolidColorBrush>
    </UserControl.Resources>

    <Border Name='border' Background='{StaticResource brush}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    let color = solid_color_brush_color(&border.background());
    assert_eq!(0xff506070, color.to_uint32());
}

#[test]
fn static_resource_can_be_assigned_to_attached_property() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Resources>
        <x:Int32 x:Key='col'>5</x:Int32>
    </UserControl.Resources>

    <Border Name='border' Grid.Column='{StaticResource col}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    assert_eq!(5, Grid::get_column(&border));
}

#[test]
fn static_resource_from_style_can_be_assigned_to_property() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Styles>
        <Style>
            <Style.Resources>
                <SolidColorBrush x:Key='brush'>#ff506070</SolidColorBrush>
            </Style.Resources>
        </Style>
    </UserControl.Styles>

    <Border Name='border' Background='{StaticResource brush}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    let color = solid_color_brush_color(&border.background());
    assert_eq!(0xff506070, color.to_uint32());
}

#[test]
fn static_resource_from_application_can_be_assigned_to_property_in_window() {
    let _app = styled_window();
    Application::current().unwrap().resources().add("brush", Some(boxed(SolidColorBrush::from_uint32(0xff506070))));

    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Border Name='border' Background='{StaticResource brush}'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);
    let border = window.get_control::<Border>("border");

    let color = mutable_solid_color_brush_color(&border.background());
    assert_eq!(0xff506070, color.to_uint32());
}

#[test]
fn static_resource_from_merged_dictionary_can_be_assigned_to_property() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Resources>
        <ResourceDictionary>
            <ResourceDictionary.MergedDictionaries>
                <ResourceDictionary>
                    <SolidColorBrush x:Key='brush'>#ff506070</SolidColorBrush>
                </ResourceDictionary>
            </ResourceDictionary.MergedDictionaries>
        </ResourceDictionary>
    </UserControl.Resources>

    <Border Name='border' Background='{StaticResource brush}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    let color = solid_color_brush_color(&border.background());
    assert_eq!(0xff506070, color.to_uint32());
}

#[test]
fn static_resource_from_merged_dictionary_in_style_can_be_assigned_to_property() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Styles>
        <Style>
            <Style.Resources>
                <ResourceDictionary>
                    <ResourceDictionary.MergedDictionaries>
                        <ResourceDictionary>
                            <SolidColorBrush x:Key='brush'>#ff506070</SolidColorBrush>
                        </ResourceDictionary>
                    </ResourceDictionary.MergedDictionaries>
                </ResourceDictionary>
            </Style.Resources>
        </Style>
    </UserControl.Styles>

    <Border Name='border' Background='{StaticResource brush}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    let color = solid_color_brush_color(&border.background());
    assert_eq!(0xff506070, color.to_uint32());
}

#[test]
fn static_resource_from_application_can_be_assigned_to_property_in_user_control() {
    let _app = styled_window_application();
    Application::current().unwrap().resources().add("brush", Some(boxed(SolidColorBrush::from_uint32(0xff506070))));

    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Border Name='border' Background='{StaticResource brush}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    // We don't actually know where the global styles are until we attach the control
    // to a window, as Window has StylingParent set to Application.
    let window = Window::new();
    window.set_content(Some(Control::boxed(user_control.clone())));
    window.show();

    let color = mutable_solid_color_brush_color(&border.background());
    assert_eq!(0xff506070, color.to_uint32());
}

#[test]
fn static_resource_can_be_assigned_to_setter() {
    let _app = styled_window();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Resources>
        <SolidColorBrush x:Key='brush'>#ff506070</SolidColorBrush>
    </Window.Resources>
    <Window.Styles>
        <Style Selector='Button'>
            <Setter Property='Background' Value='{StaticResource brush}'/>
        </Style>
    </Window.Styles>
    <Button Name='button'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");
    let color = solid_color_brush_color(&button.background());

    assert_eq!(0xff506070, color.to_uint32());
}

#[test]
fn static_resource_from_style_can_be_assigned_to_setter() {
    let _app = styled_window();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Styles>
        <Style>
            <Style.Resources>
                <SolidColorBrush x:Key='brush'>#ff506070</SolidColorBrush>
            </Style.Resources>
        </Style>
        <Style Selector='Button'>
            <Setter Property='Background' Value='{StaticResource brush}'/>
        </Style>
    </Window.Styles>
    <Button Name='button'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");
    let color = solid_color_brush_color(&button.background());

    assert_eq!(0xff506070, color.to_uint32());
}

#[test]
fn static_resource_can_be_assigned_to_setter_in_styles_file() {
    let documents = vec![
        document(
            "ferres://Tests/Style.xaml",
            r#"
<Styles xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Styles.Resources>
        <SolidColorBrush x:Key='brush'>#ff506070</SolidColorBrush>
    </Styles.Resources>

    <Style Selector='Border'>
        <Setter Property='Background' Value='{StaticResource brush}'/>
    </Style>
</Styles>"#,
        ),
        document_without_uri(
            r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Styles>
        <StyleInclude Source='ferres://Tests/Style.xaml'/>
    </Window.Styles>
    <Border Name='border'/>
</Window>"#,
        ),
    ];

    let _app = styled_window();
    let compiled = load_group(documents, None);
    let window = window_of(&compiled, 1);
    let border = window.get_control::<Border>("border");
    let color = solid_color_brush_color(&border.background());

    assert_eq!(0xff506070, color.to_uint32());
}

#[test]
fn static_resource_can_be_assigned_to_resource_property() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Resources>
        <Color x:Key='color'>#ff506070</Color>
        <SolidColorBrush x:Key='brush' Color='{StaticResource color}'/>
    </UserControl.Resources>

    <Border Name='border' Background='{StaticResource brush}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    let color = mutable_solid_color_brush_color(&border.background());
    assert_eq!(0xff506070, color.to_uint32());
}

#[test]
fn static_resource_can_be_assigned_to_resource_property_in_styles_file() {
    let _base = xaml_test_base();
    let xaml = r#"
<Styles xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Styles.Resources>
        <Color x:Key='color'>#ff506070</Color>
        <SolidColorBrush x:Key='brush' Color='{StaticResource color}'/>
    </Styles.Resources>
</Styles>"#;

    let styles: Ref<Styles> = load_as(xaml);
    let brush = object_of::<SolidColorBrush>(&styles.resources().get(&ResourceKey::from("brush")));

    assert_eq!(0xff506070, brush.color().to_uint32());
}

#[test]
fn static_resource_can_be_assigned_to_property_in_control_template_in_styles_file() {
    let documents = vec![
        document(
            "ferres://Tests/Style.xaml",
            r#"
<Styles xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Styles.Resources>
        <SolidColorBrush x:Key='brush'>#ff506070</SolidColorBrush>
    </Styles.Resources>

    <Style Selector='Button'>
        <Setter Property='Template'>
            <ControlTemplate>
                <Border Name='border' Background='{StaticResource brush}'/>
            </ControlTemplate>
        </Setter>
    </Style>
</Styles>"#,
        ),
        document_without_uri(
            r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Styles>
        <StyleInclude Source='ferres://Tests/Style.xaml'/>
    </Window.Styles>
    <Button Name='button'/>
</Window>"#,
        ),
    ];

    let _app = styled_window();
    let compiled = load_group(documents, None);
    let window = window_of(&compiled, 1);
    let button = window.get_control::<Button>("button");

    window.show();

    let border = single::<Border>(button.get_visual_children().iter().cloned());
    let color = solid_color_brush_color(&border.background());

    assert_eq!(0xff506070, color.to_uint32());
}

#[test]
fn static_resource_can_be_assigned_to_item_template_property() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Resources>
        <DataTemplate x:Key='PurpleData'>
          <TextBlock Text='{Binding Name}' Background='Purple'/>
        </DataTemplate>
    </UserControl.Resources>

    <ListBox Name='listBox' ItemTemplate='{StaticResource PurpleData}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let list_box = user_control.get_control::<ListBox>("listBox");

    assert!(list_box.item_template().is_some());
}

#[test]
fn static_resource_can_be_assigned_to_converter() {
    let _app = styled_window();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
             xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Window.Resources>
        <local:TestValueConverter x:Key='converter' Append='bar'/>
    </Window.Resources>

    <TextBlock Name='textBlock' Text='{Binding Converter={StaticResource converter}}'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    window.set_data_context(boxed_str("foo"));
    window.apply_template();

    assert_eq!(Some("foobar"), text_block.text().as_deref());
}

#[test]
fn static_resource_can_be_assigned_to_binding_converter_in_data_template() {
    let _app = styled_window();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
             xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Window.Resources>
        <local:TestValueConverter x:Key='converter' Append='bar'/>
        <DataTemplate x:Key='PurpleData'>
          <TextBlock Name='textBlock' Text='{Binding Converter={StaticResource converter}}'/>
        </DataTemplate>
    </Window.Resources>

    <ContentPresenter Name='presenter' Content='foo' ContentTemplate='{StaticResource PurpleData}'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);

    window.set_data_context(boxed_str("foo"));
    let presenter = window.get_control::<ContentPresenter>("presenter");

    window.show();

    let text_block = single::<TextBlock>(presenter.get_visual_children().iter().cloned());

    assert_eq!(Some("foobar"), text_block.text().as_deref());
}

#[test]
fn static_resource_is_correctly_chosen_from_within_data_template() {
    // this tests if IAmbientProviders in DataTemplate contexts are in correct order
    // if they wouldn't be, Purple brush would be bound to
    let _app = styled_window();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
             xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Window.Resources>
        <local:TestValueConverter x:Key='converter' Append='-bar'/>
        <SolidColorBrush x:Key='brush' Color='Purple'/>
        <DataTemplate x:Key='WhiteData'>
          <Border>
            <Border.Resources>
              <SolidColorBrush x:Key='brush' Color='White'/>
            </Border.Resources>
            <TextBlock Name='textBlock' Text='{Binding Color, Source={StaticResource brush}, Converter={StaticResource converter}}' Foreground='{StaticResource brush}' />
          </Border>
        </DataTemplate>
    </Window.Resources>

    <ContentPresenter Content='foo' ContentTemplate='{StaticResource WhiteData}'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);

    window.show();

    let text_block = single_of_type::<TextBlock>(window.get_visual_descendants());

    assert_eq!(Some("White-bar"), text_block.text().as_deref());
}

#[test]
fn static_resource_is_correctly_chosen_for_deferred_content() {
    let _app = styled_window();
    let window: Ref<Window> = load_as(
        r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'>

  <Window.Resources>
    <Color x:Key='Color'>Purple</Color>
  </Window.Resources>

  <Border>
   <Border.Resources>
      <Color x:Key='Color'>Red</Color>
      <SolidColorBrush x:Key='Brush' Color='{StaticResource Color}' />
    </Border.Resources>
    <TextBlock Foreground='{StaticResource Brush}' />
  </Border>

</Window>"#,
    );

    window.show();

    let text_block = single_of_type::<TextBlock>(window.get_visual_descendants());

    let color = solid_color_brush_color(&text_block.foreground());
    assert_eq!(Colors::RED, color);
}

#[test]
fn control_property_is_not_updated_when_parent_is_changed() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Resources>
        <SolidColorBrush x:Key='brush'>#ff506070</SolidColorBrush>
    </UserControl.Resources>

    <Border Name='border' Background='{StaticResource brush}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    let color = solid_color_brush_color(&border.background());
    assert_eq!(0xff506070, color.to_uint32());

    user_control.set_content(None);

    let color = solid_color_brush_color(&border.background());
    assert_eq!(0xff506070, color.to_uint32());
}

#[test]
fn automatically_converts_color_to_solid_color_brush() {
    let _base = xaml_test_base();
    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Resources>
        <Color x:Key='color'>#ff506070</Color>
    </UserControl.Resources>

    <Border Name='border' Background='{StaticResource color}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = user_control.get_control::<Border>("border");

    let color = solid_color_brush_color(&border.background());
    assert_eq!(0xff506070, color.to_uint32());
}

#[test]
fn automatically_converts_color_to_solid_color_brush_from_setter() {
    let _app = styled_window();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Resources>
        <Color x:Key='color'>#ff506070</Color>
    </Window.Resources>
    <Window.Styles>
        <Style Selector='Button'>
            <Setter Property='Background' Value='{StaticResource color}'/>
        </Style>
    </Window.Styles>
    <Button Name='button'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");
    let color = solid_color_brush_color(&button.background());

    assert_eq!(0xff506070, color.to_uint32());
}
