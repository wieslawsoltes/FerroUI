//! Port of `Xaml/ResourceDictionaryTests.cs`.

use std::rc::Rc;

use ferroui_base::controls::{NameScopeExtensions, ResourceDictionary};
use ferroui_base::media::{Color, Colors, IBrush, SolidColorBrush};
use ferroui_base::styling::{styles_as_style, ControlTheme, IStyle, Selectors, Setter, Style, Styles};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use ferroui_controls::testing::UnitTestApplicationScope;
use ferroui_controls::{Button, ContentControl, Panel, UserControl, Window};

use crate::support::app::{unit_test_application, xaml_test_base, TestServices};
use crate::support::helpers::{assert_is_type, assert_value_is_type, boxed, is_type, object_of, try_object_of, value_of};
use crate::support::loader::{document, document_without_uri, group_item, load_as, load_group};

#[test]
fn static_resource_works_in_resource_dictionary() {
    let _app = styled_window();
    let xaml = "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
  <Color x:Key='Red'>Red</Color>
  <SolidColorBrush x:Key='RedBrush' Color='{StaticResource Red}'/>
</ResourceDictionary>";
    let resources = load_as::<Ref<ResourceDictionary>>(xaml);
    let brush = object_of::<SolidColorBrush>(&resources.get(&"RedBrush".into()));

    assert_eq!(Colors::RED, brush.color());
}

#[test]
fn dynamic_resource_finds_resource_in_parent_dictionary() {
    let _app = styled_window();
    let documents = vec![
        document(
            "ferres://FerroUI.Markup.Xaml.UnitTests/dict.xaml",
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
  <SolidColorBrush x:Key='RedBrush' Color='{DynamicResource Red}'/>
</ResourceDictionary>",
        ),
        document_without_uri(
            "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Resources>
        <ResourceDictionary>
            <ResourceDictionary.MergedDictionaries>
                <ResourceInclude Source='ferres://FerroUI.Markup.Xaml.UnitTests/dict.xaml'/>
            </ResourceDictionary.MergedDictionaries>
        </ResourceDictionary>
        <Color x:Key='Red'>Red</Color>
    </Window.Resources>
    <Button Name='button' Background='{DynamicResource RedBrush}'/>
</Window>",
        ),
    ];

    let loaded = load_group(documents, None);
    let window = group_item::<Ref<Window>>(&loaded, 1);
    assert!(window.get_type() == Window::TYPE);
    let button = window.get_control::<Button>("button");

    let background = button.background().expect("the background is set");
    let background = background.as_object().expect("the background is an object of the class model");
    assert_is_type::<SolidColorBrush>(background);
    let brush = background.downcast_ref::<SolidColorBrush>().expect("the background is a SolidColorBrush");
    assert_eq!(Colors::RED, brush.color());

    window.resources().set("Red", Some(boxed(Colors::GREEN)));

    assert_eq!(Colors::GREEN, brush.color());
}

#[test]
fn item_is_added_to_resource_dictionary_as_deferred() {
    let _app = styled_window();
    let xaml = "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <SolidColorBrush x:Key='Red' Color='Red' />
</ResourceDictionary>";
    let resources = load_as::<Ref<ResourceDictionary>>(xaml);

    assert!(resources.contains_deferred_key(&"Red".into()));
}

#[test]
fn item_added_to_resource_dictionary_is_un_deferred_on_read() {
    let _app = styled_window();
    let xaml = "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <SolidColorBrush x:Key='Red' Color='Red' />
</ResourceDictionary>";
    let resources = load_as::<Ref<ResourceDictionary>>(xaml);

    assert!(resources.contains_deferred_key(&"Red".into()));

    assert_value_is_type::<SolidColorBrush>(&resources.get(&"Red".into()));

    assert!(!resources.contains_deferred_key(&"Red".into()));
}

#[test]
fn item_is_added_to_window_resources_as_deferred() {
    let _app = styled_window();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Resources>
        <SolidColorBrush x:Key='Red' Color='Red' />
    </Window.Resources>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let resources = window.resources();

    assert!(resources.contains_deferred_key(&"Red".into()));
}

#[test]
fn named_item_is_added_to_resources_should_not_be_deferred() {
    // Since Named items can be accessed through the NameScope, we cannot delay their initialization.
    let _app = styled_window();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Resources>
        <Panel x:Name='MyPanel' x:Key='MyPanel' />
    </Window.Resources>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let resources = window.resources();

    assert!(!resources.contains_deferred_key(&"MyPanel".into()));
    assert!(resources.contains_key(&"MyPanel".into()));
    let panel = NameScopeExtensions::find::<Panel>(&window, "MyPanel").expect("the panel is found by name");
    assert!(panel.get_type() == Panel::TYPE);
}

#[test]
fn item_is_added_to_window_merged_dictionaries_as_deferred() {
    let _app = styled_window();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Resources>
        <ResourceDictionary>
            <ResourceDictionary.MergedDictionaries>
                <ResourceDictionary>
                    <SolidColorBrush x:Key='Red' Color='Red' />
                </ResourceDictionary>
            </ResourceDictionary.MergedDictionaries>
        </ResourceDictionary>
    </Window.Resources>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let merged = window.resources().merged_dictionaries().get(0);
    let resources = merged
        .as_object()
        .and_then(|object| object.downcast_ref::<ResourceDictionary>())
        .expect("the merged dictionary is a ResourceDictionary");

    assert!(resources.contains_deferred_key(&"Red".into()));
}

#[test]
fn item_is_added_to_style_resources_as_deferred() {
    let _app = styled_window();
    let xaml = "
<Style xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Style.Resources>
        <SolidColorBrush x:Key='Red' Color='Red' />
    </Style.Resources>
</Style>";
    let style = load_as::<Ref<Style>>(xaml);
    let resources = style.resources();

    assert!(resources.contains_deferred_key(&"Red".into()));
}

#[test]
fn item_is_added_to_styles_resources_as_deferred() {
    let _app = styled_window();
    let xaml = "
<Styles xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Styles.Resources>
        <SolidColorBrush x:Key='Red' Color='Red' />
    </Styles.Resources>
</Styles>";
    let style = load_as::<Ref<Styles>>(xaml);
    let resources = style.resources();

    assert!(resources.contains_deferred_key(&"Red".into()));
}

#[test]
fn item_can_be_static_referenced_as_deferred() {
    let _app = styled_window();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Resources>
        <SolidColorBrush x:Key='Red' Color='Red' />
    </Window.Resources>
    <Button>
        <Button.Resources>
            <StaticResource x:Key='Red2' ResourceKey='Red' />
        </Button.Resources>
    </Button>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let window_resources = window.resources();
    let button_resources = object_of::<Button>(&window.content()).resources();

    assert!(window_resources.contains_deferred_key(&"Red".into()));
    assert!(button_resources.contains_deferred_key(&"Red2".into()));
}

#[test]
fn item_static_referenced_is_un_deferred_on_read() {
    let _app = styled_window();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Resources>
        <SolidColorBrush x:Key='Red' Color='Red' />
    </Window.Resources>
    <Button>
        <Button.Resources>
            <StaticResource x:Key='Red2' ResourceKey='Red' />
        </Button.Resources>
    </Button>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let window_resources = window.resources();
    let button_resources = object_of::<Button>(&window.content()).resources();

    assert_value_is_type::<SolidColorBrush>(&button_resources.get(&"Red2".into()));

    assert!(!window_resources.contains_deferred_key(&"Red".into()));
    assert!(!button_resources.contains_deferred_key(&"Red2".into()));
}

#[test]
fn value_type_with_parse_converter_should_not_be_deferred() {
    let _app = styled_window();
    let xaml = "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Color x:Key='Red'>Red</Color>
</ResourceDictionary>";
    let resources = load_as::<Ref<ResourceDictionary>>(xaml);

    assert!(!resources.contains_deferred_key(&"Red".into()));
    assert!(is_type::<Color>(&resources.get(&"Red".into())));
}

#[test]
fn value_type_with_ctor_converter_should_not_be_deferred() {
    let _app = styled_window();
    let xaml = "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Thickness x:Key='Margin'>1 1 1 1</Thickness>
</ResourceDictionary>";
    let resources = load_as::<Ref<ResourceDictionary>>(xaml);

    assert!(!resources.contains_deferred_key(&"Margin".into()));
    assert!(is_type::<Thickness>(&resources.get(&"Margin".into())));
}

#[test]
fn closest_resource_should_be_referenced() {
    let _app = styled_window();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Resources>
        <SolidColorBrush x:Key='Red' Color='Red' />
        <StaticResource x:Key='Red2' ResourceKey='Red' />
    </Window.Resources>
    <Button>
        <Button.Resources>
            <SolidColorBrush x:Key='Red' Color='Blue' />
        </Button.Resources>
    </Button>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let window_resources = window.resources();
    let button_resources = object_of::<Button>(&window.content()).resources();

    let brush = assert_value_is_type::<SolidColorBrush>(&window_resources.get(&"Red2".into()));
    assert_eq!(Colors::RED, brush.color());

    assert!(!window_resources.contains_deferred_key(&"Red".into()));
    assert!(!window_resources.contains_deferred_key(&"Red2".into()));

    assert!(button_resources.contains_deferred_key(&"Red".into()));
}

#[test]
fn should_be_possible_to_redefine_referenced_resource_control_theme() {
    let _app = styled_window();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Resources>
        <ControlTheme x:Key='{x:Type Button}' TargetType='Button' />
    </Window.Resources>
    <UserControl>
        <UserControl.Resources>
            <ControlTheme x:Key='{x:Type Button}' TargetType='Button' BasedOn='{StaticResource {x:Type Button}}' />
        </UserControl.Resources>
    </UserControl>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let window_resources = window.resources();
    let inner_resources =
        object_of::<UserControl>(&window.content()).resources();

    let win_button_theme = assert_value_is_type::<ControlTheme>(&window_resources.get(&Button::TYPE.into()));
    let inner_button_theme = assert_value_is_type::<ControlTheme>(&inner_resources.get(&Button::TYPE.into()));
    assert!(inner_button_theme.based_on().is_some_and(|based_on| based_on.ptr_eq(&win_button_theme)));
}

#[test]
fn should_be_possible_to_redefine_referenced_resource() {
    let _app = styled_window();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Resources>
        <Color x:Key='SystemAccentColor'>#aaa</Color>
    </Window.Resources>
    <UserControl>
        <UserControl.Resources>
            <StaticResource x:Key='SystemAccentColor' ResourceKey='SystemAccentColor' />
        </UserControl.Resources>
    </UserControl>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let window_resources = window.resources();
    let inner_resources =
        object_of::<UserControl>(&window.content()).resources();

    let win_resource = window_resources.get(&"SystemAccentColor".into());
    assert!(is_type::<Color>(&win_resource));
    let win_button_theme = value_of::<Color>(&win_resource).expect("a Color");
    let inner_resource = inner_resources.get(&"SystemAccentColor".into());
    assert!(is_type::<Color>(&inner_resource));
    let inner_button_theme = value_of::<Color>(&inner_resource).expect("a Color");
    assert_eq!(win_button_theme, inner_button_theme);
}

#[test]
fn dynamically_changing_referenced_resources_works_with_dynamic_resource() {
    let _base = xaml_test_base();
    let xaml = "
<UserControl xmlns='https://github.com/ferroui'
     xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
  <UserControl.Resources>
    <Color x:Key='color'>Red</Color>
    <SolidColorBrush x:Key='brush' Color='{DynamicResource color}' />
  </UserControl.Resources>
</UserControl>";

    let user_control = load_as::<Ref<UserControl>>(xaml);
    let brush_color = || {
        let brush = value_of::<Rc<dyn IBrush>>(&user_control.find_resource(&"brush".into())).expect("a brush");
        brush.as_solid_color_brush().expect("a solid color brush").color()
    };

    assert_eq!(Colors::RED, brush_color());

    user_control.resources().remove(&"color".into());
    assert_eq!(Color::default(), brush_color());

    user_control.resources().add("color", Some(boxed(Colors::BLUE)));
    assert_eq!(Colors::BLUE, brush_color());
}

#[test]
fn resource_dictionary_can_be_put_inside_of_resource_dictionary() {
    let _app = styled_window();
    let xaml = "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ResourceDictionary x:Key='NotAThemeVariantKey' />
</ResourceDictionary>";
    let resources = load_as::<Ref<ResourceDictionary>>(xaml);
    let nested = try_object_of::<ResourceDictionary>(&resources.get(&"NotAThemeVariantKey".into()));

    assert!(nested.is_some());
}

/// The application of the tests: a styled window whose theme is the style
/// of the window alone.
fn styled_window() -> UnitTestApplicationScope {
    let services = TestServices::styled_window().with_theme(|| {
        let styles = Styles::new();
        styles.add(window_style());
        styles_as_style(&styles)
    });

    unit_test_application(services)
}

fn window_style() -> Rc<dyn IStyle> {
    let template: Rc<dyn IControlTemplate> = FuncControlTemplate::for_type::<Window>(|x, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        presenter.bind_indexer(
            &ContentPresenter::content_property().as_property().bind(),
            &x.indexer(&ContentControl::content_property().as_property().bind()),
        );
        presenter.register_in_name_scope(&**scope).upcast()
    });

    let style = Style::with_selector(Selectors::of_type::<Window>());
    style.add(Setter::new(TemplatedControl::template_property(), Some(template)));
    style.into()
}
