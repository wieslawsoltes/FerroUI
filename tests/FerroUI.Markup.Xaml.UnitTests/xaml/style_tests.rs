//! Port of `Xaml/StyleTests.cs`.

use std::rc::Rc;

use ferroui_base::collections::FerroList;
use ferroui_base::controls::NameScopeExtensions;
use ferroui_base::layout::Layoutable;
use ferroui_base::media::{Brushes, Color, Colors, IBrush};
use ferroui_base::styling::{Style, Styles};
use ferroui_base::Ref;
use ferroui_controls::{
    Border, Button, Carousel, ContentControl, Dock, DockPanel, ItemsSource, ListBox, ListBoxItem, StackPanel,
    TextBlock, UserControl, Window,
};
use ferroui_controls::testing::UnitTestApplicationScope;
use ferroui_markup_xaml::templates::{ControlTemplate, DataTemplate};
use xamlx::exceptions::XamlError;

use crate::support::app::{styled_window_application, unit_test_application, TestServices};
use crate::support::helpers::{
    as_setter, assert_throws_xml_exception, assert_value_is_type, object_of, setter_template_binding_property,
    try_object_of, value_of,
};
use crate::support::xaml::{TestSelectorControl, TestSelectorControlExtension, TestTemplatedControl};
use crate::support::loader::{describe, load_as, try_load, xaml_error};
use crate::support::TestViewModel;

/// `((ISolidColorBrush)brush!).Color`.
#[track_caller]
fn solid_color(brush: &Option<Rc<dyn IBrush>>) -> Color {
    let brush = brush.as_ref().expect("the brush is set");
    brush.as_solid_color_brush().expect("the brush is a solid color brush").color()
}

/// `UnitTestApplication.Start(TestServices.MockPlatformWrapper)`.
fn mock_platform_wrapper_application() -> UnitTestApplicationScope {
    unit_test_application(TestServices::mock_platform_wrapper())
}

/// `(Style)styles[0]`.
#[track_caller]
fn style_at(styles: &Ref<Styles>, index: usize) -> Ref<Style> {
    let style = styles.get(index);
    let object = style.as_object().expect("the style is an object of the object model");
    object.to_ref().cast::<Style>().expect("the style is a Style")
}

#[test]
fn color_can_be_added_to_style_resources() {
    let _app = mock_platform_wrapper_application();
    let xaml = "
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Styles>
        <Style>
            <Style.Resources>
                <Color x:Key='color'>#ff506070</Color>
            </Style.Resources>
        </Style>
    </UserControl.Styles>
</UserControl>";
    let user_control = load_as::<Ref<UserControl>>(xaml);
    let color = value_of::<Color>(&style_at(&user_control.styles(), 0).resources().get(&"color".into()))
        .expect("the resource is a Color");

    assert_eq!(0xff506070, color.to_uint32());
}

#[test]
fn data_template_can_be_added_to_style_resources() {
    let _app = mock_platform_wrapper_application();
    let xaml = "
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Styles>
        <Style>
            <Style.Resources>
                <DataTemplate x:Key='dataTemplate'><TextBlock/></DataTemplate>
            </Style.Resources>
        </Style>
    </UserControl.Styles>
</UserControl>";
    let user_control = load_as::<Ref<UserControl>>(xaml);
    let data_template =
        value_of::<Rc<DataTemplate>>(&style_at(&user_control.styles(), 0).resources().get(&"dataTemplate".into()));

    assert!(data_template.is_some());
}

#[test]
fn control_template_can_be_added_to_style_resources() {
    let _app = mock_platform_wrapper_application();
    let xaml = "
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Styles>
        <Style>
            <Style.Resources>
                 <ControlTemplate x:Key='controlTemplate' TargetType='{x:Type Button}'>
                    <ContentPresenter Content='{TemplateBinding Content}'/>
                 </ControlTemplate>
            </Style.Resources>
        </Style>
    </UserControl.Styles>
</UserControl>";
    let user_control = load_as::<Ref<UserControl>>(xaml);
    let control_template = value_of::<Rc<ControlTemplate>>(
        &style_at(&user_control.styles(), 0).resources().get(&"controlTemplate".into()),
    );

    let control_template = control_template.expect("the resource is a ControlTemplate");
    assert_eq!(Some(Button::TYPE), control_template.target_type());
}

#[test]
fn solid_color_brush_can_be_added_to_style_resources() {
    let _app = mock_platform_wrapper_application();
    let xaml = "
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Styles>
        <Style>
            <Style.Resources>
                <SolidColorBrush x:Key='brush'>#ff506070</SolidColorBrush>
            </Style.Resources>
        </Style>
    </UserControl.Styles>
</UserControl>";
    let user_control = load_as::<Ref<UserControl>>(xaml);
    let brush = value_of::<Rc<dyn IBrush>>(&style_at(&user_control.styles(), 0).resources().get(&"brush".into()));

    assert_eq!(0xff506070, solid_color(&brush).to_uint32());
}

#[test]
fn setter_can_contain_template() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Styles>
        <Style Selector='ContentControl'>
            <Setter Property='Content'>
                <Template>
                    <TextBlock>Hello World!</TextBlock>
                </Template>
            </Setter>
        </Style>
    </Window.Styles>

    <ContentControl Name='target'/>
</Window>";

    let window = load_as::<Ref<Window>>(xaml);
    let target = NameScopeExtensions::get::<ContentControl>(&window, "target");

    let text_block = assert_value_is_type::<TextBlock>(&target.content());
    assert_eq!(Some("Hello World!".to_string()), text_block.text());
}

#[test]
fn setter_value_is_bound_directly_if_the_target_type_derives_from_i_template() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Styles>
        <Style Selector=':is(Control)'>
		  <Setter Property='FocusAdorner'>
			<FocusAdornerTemplate>
			  <Rectangle Stroke='Black'
						 StrokeThickness='1'
						 StrokeDashArray='1,2'/>
			</FocusAdornerTemplate>
		  </Setter>
		</Style>
	</Window.Styles>

    <TextBlock Name='target'/>
</Window>";

    let window = load_as::<Ref<Window>>(xaml);
    let target = NameScopeExtensions::get::<TextBlock>(&window, "target");

    assert!(target.focus_adorner().is_some());
}

#[test]
fn setter_can_set_attached_property() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Window.Styles>
        <Style Selector='TextBlock'>
            <Setter Property='DockPanel.Dock' Value='Right'/>
        </Style>
    </Window.Styles>
    <TextBlock/>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let text_block = object_of::<TextBlock>(&window.content());

    window.apply_template();

    assert_eq!(Dock::Right, DockPanel::get_dock(&text_block));
}

#[test]
#[ignore = "The animation system currently needs to be able to set any property on any object"]
fn disallows_setting_non_registered_property() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Window.Styles>
        <Style Selector='TextBlock'>
            <Setter Property='Button.IsDefault' Value='True'/>
        </Style>
    </Window.Styles>
    <TextBlock/>
</Window>";
    let error = match try_load(xaml) {
        Ok(_) => panic!("Expected an XmlException"),
        Err(error) => error,
    };
    let Some(ex @ XamlError::Xml(_)) = xaml_error(&error) else {
        panic!("Expected an XmlException: {}", describe(&error));
    };

    assert_eq!(
        Some("Property 'Button.IsDefault' is not registered on 'FerroUI.Controls.TextBlock'.".to_string()),
        ex.inner_exception().map(|inner| inner.message())
    );
}

#[test]
fn style_can_use_not_selector() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Styles>
        <Style Selector='Border:not(.foo)'>
            <Setter Property='Background' Value='Red'/>
        </Style>
    </Window.Styles>
    <StackPanel>
        <Border Name='foo' Classes='foo bar'/>
        <Border Name='notFoo' Classes='bar'/>
    </StackPanel>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let foo = window.get_control::<Border>("foo");
    let not_foo = window.get_control::<Border>("notFoo");

    assert!(foo.background().is_none());
    assert_eq!(Colors::RED, solid_color(&not_foo.background()));
}

#[test]
fn style_can_use_nth_child_selector() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Styles>
        <Style Selector='Border.foo:nth-child(2n+1)'>
            <Setter Property='Background' Value='Red'/>
        </Style>
    </Window.Styles>
    <StackPanel>
        <Border x:Name='b1' Classes='foo'/>
        <Border x:Name='b2' />
    </StackPanel>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let b1 = window.get_control::<Border>("b1");
    let b2 = window.get_control::<Border>("b2");

    let red: Rc<dyn IBrush> = Brushes::red();

    assert_eq!(Some(red), b1.background());
    assert!(b2.background().is_none());
}

#[test]
fn style_can_use_nth_child_selector_after_reorder() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Styles>
        <Style Selector='Border:nth-child(2n)'>
            <Setter Property='Background' Value='Red'/>
        </Style>
    </Window.Styles>
    <StackPanel x:Name='parent'>
        <Border x:Name='b1' />
        <Border x:Name='b2' />
    </StackPanel>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);

    let parent = window.get_control::<StackPanel>("parent");
    let b1 = window.get_control::<Border>("b1");
    let b2 = window.get_control::<Border>("b2");

    let red: Rc<dyn IBrush> = Brushes::red();

    assert!(b1.background().is_none());
    assert_eq!(Some(red.clone()), b2.background());

    parent.children().remove(&b1);

    assert!(b1.background().is_none());
    assert!(b2.background().is_none());

    parent.children().add(&b1);

    assert_eq!(Some(red), b1.background());
    assert!(b2.background().is_none());
}

#[test]
fn style_can_use_nth_last_child_selector_after_reorder() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Styles>
        <Style Selector='Border:nth-last-child(2n)'>
            <Setter Property='Background' Value='Red'/>
        </Style>
    </Window.Styles>
    <StackPanel x:Name='parent'>
        <Border x:Name='b1' />
        <Border x:Name='b2' />
    </StackPanel>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);

    let parent = window.get_control::<StackPanel>("parent");
    let b1 = window.get_control::<Border>("b1");
    let b2 = window.get_control::<Border>("b2");

    let red: Rc<dyn IBrush> = Brushes::red();

    assert_eq!(Some(red.clone()), b1.background());
    assert!(b2.background().is_none());

    parent.children().remove(&b1);

    assert!(b1.background().is_none());
    assert!(b2.background().is_none());

    parent.children().add(&b1);

    assert!(b1.background().is_none());
    assert_eq!(Some(red), b2.background());
}

#[test]
#[ignore = "themes: needs the Simple theme (upstream runs this test under it)"]
fn style_can_use_nth_child_selector_with_list_box() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Styles>
        <Style Selector='ListBoxItem:nth-child(2n)'>
            <Setter Property='Background' Value='{Binding}'/>
        </Style>
    </Window.Styles>
    <ListBox x:Name='list' />
</Window>";
    let window = load_as::<Ref<Window>>(xaml);

    let red: Rc<dyn IBrush> = Brushes::red();
    let green: Rc<dyn IBrush> = Brushes::green();
    let blue: Rc<dyn IBrush> = Brushes::blue();
    let violet: Rc<dyn IBrush> = Brushes::violet();
    let black: Rc<dyn IBrush> = Brushes::black();
    let transparent: Rc<dyn IBrush> = Brushes::transparent();

    let collection: Rc<FerroList<Rc<dyn IBrush>>> =
        Rc::new(FerroList::from_items([red.clone(), green.clone(), blue.clone()]));

    let list = window.get_control::<ListBox>("list");
    list.set_items_source(Some(ItemsSource::from(collection.clone())));

    window.show();

    let get_colors = || -> Vec<Option<Rc<dyn IBrush>>> {
        list.get_realized_containers()
            .iter()
            .map(|container| container.cast::<ListBoxItem>().expect("the container is a ListBoxItem").background())
            .collect()
    };

    assert_eq!(vec![Some(transparent.clone()), Some(green.clone()), Some(transparent.clone())], get_colors());

    collection.remove(&green);
    window.update_layout();

    assert_eq!(vec![Some(transparent.clone()), Some(blue.clone())], get_colors());

    collection.add(violet);
    collection.add(black.clone());
    window.update_layout();

    assert_eq!(
        vec![Some(transparent.clone()), Some(blue), Some(transparent), Some(black)],
        get_colors()
    );
}

#[test]
fn style_can_use_or_selector_1() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Styles>
        <Style Selector='Border.foo, Border.bar'>
            <Setter Property='Background' Value='Red'/>
        </Style>
    </Window.Styles>
    <StackPanel>
        <Border Name='foo' Classes='foo'/>
        <Border Name='bar' Classes='bar'/>
        <Border Name='baz' Classes='baz'/>
    </StackPanel>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let foo = window.get_control::<Border>("foo");
    let bar = window.get_control::<Border>("bar");
    let baz = window.get_control::<Border>("baz");

    let red: Rc<dyn IBrush> = Brushes::red();

    assert_eq!(Some(red.clone()), foo.background());
    assert_eq!(Some(red), bar.background());
    assert!(baz.background().is_none());
}

#[test]
fn style_can_use_or_selector_2() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Styles>
        <Style Selector='Button,Carousel,ListBox'>
            <Setter Property='Background' Value='Red'/>
        </Style>
    </Window.Styles>
    <StackPanel>
        <Button Name='button'/>
        <Carousel Name='carousel'/>
        <ListBox Name='listBox'/>
    </StackPanel>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let button = window.get_control::<Button>("button");
    let carousel = window.get_control::<Carousel>("carousel");
    let list_box = window.get_control::<ListBox>("listBox");

    let red: Rc<dyn IBrush> = Brushes::red();

    assert_eq!(Some(red.clone()), button.background());
    assert_eq!(Some(red.clone()), carousel.background());
    assert_eq!(Some(red), list_box.background());
}

#[test]
fn transitions_can_be_styled() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Styles>
        <Style Selector='Border'>
            <Setter Property='Transitions'>
                <Transitions>
                    <DoubleTransition Property='Width' Duration='0:0:1'/>
                </Transitions>
            </Setter>
        </Style>
        <Style Selector='Border.foo'>
            <Setter Property='Transitions'>
                <Transitions>
                    <DoubleTransition Property='Height' Duration='0:0:1'/>
                </Transitions>
            </Setter>
        </Style>
    </Window.Styles>
    <Border/>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let border = object_of::<Border>(&window.content());

    let transitions = border.transitions().expect("the transitions are set");
    assert_eq!(1, transitions.count());
    assert_eq!(Layoutable::width_property().as_property(), transitions.get(0).property());

    border.classes().add("foo");

    let transitions = border.transitions().expect("the transitions are set");
    assert_eq!(1, transitions.count());
    assert_eq!(Layoutable::height_property().as_property(), transitions.get(0).property());

    border.classes().remove("foo");

    let transitions = border.transitions().expect("the transitions are set");
    assert_eq!(1, transitions.count());
    assert_eq!(Layoutable::width_property().as_property(), transitions.get(0).property());
}

#[test]
fn style_can_use_class_selector_with_dash() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Styles>
        <Style Selector='Border.foo-bar'>
            <Setter Property='Background' Value='Red'/>
        </Style>
    </Window.Styles>
    <StackPanel>
        <Border Name='foo' Classes='foo-bar'/>
    </StackPanel>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let foo = window.get_control::<Border>("foo");

    assert_eq!(Colors::RED, solid_color(&foo.background()));
}

#[test]
fn style_can_use_pseudolass_selector_with_dash() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Styles>
        <Style Selector='Border:foo-bar'>
            <Setter Property='Background' Value='Red'/>
        </Style>
    </Window.Styles>
    <StackPanel>
        <Border Name='foo'/>
    </StackPanel>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let foo = window.get_control::<Border>("foo");

    assert!(foo.background().is_none());

    foo.pseudo_classes().add_pseudo(":foo-bar");

    assert_eq!(Colors::RED, solid_color(&foo.background()));
}

#[test]
fn can_use_nested_styles() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Styles>
        <Style Selector='Border'>
            <Style Selector='^.foo'>
                <Setter Property='Background' Value='Red'/>
            </Style>
        </Style>
    </Window.Styles>
    <StackPanel>
        <Border Name='foo'/>
    </StackPanel>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let foo = window.get_control::<Border>("foo");

    assert!(foo.background().is_none());

    foo.classes().add("foo");

    assert_eq!(Colors::RED, solid_color(&foo.background()));
}

#[test]
fn multiple_errors_are_reported() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Styles>
        <Style Selector='5' />
        <Style Selector='NonExistentType' />
        <Style Selector='Border:normal' />
        <Style Selector='Border+invalid' />
    </Window.Styles>
</Window>";
    let error = match try_load(xaml) {
        Ok(_) => panic!("Expected an AggregateException"),
        Err(error) => error,
    };
    let Some(XamlError::Aggregate(inner_exceptions)) = xaml_error(&error) else {
        panic!("Expected an AggregateException: {}", describe(&error));
    };
    assert_eq!(3, inner_exceptions.len(), "the errors: {}", describe(&error));
    for inner in inner_exceptions {
        assert!(inner.is_xml_exception(), "not an XmlException: {inner}");
    }
}

#[test]
fn correctly_resolve_template_binding_in_style_with_template_selector() {
    let _app = styled_window_application();
    let xaml = "
<Style xmlns='https://github.com/ferroui'
       xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
       xmlns:u='using:FerroUI.Markup.Xaml.UnitTests.Xaml'
       Selector='u|TestTemplatedControl /template/ Border'>
    <Setter Property='Tag' Value='{TemplateBinding TestData}'/>
</Style>";

    let style = load_as::<Ref<Style>>(xaml);
    assert_eq!(1, style.setters().count(), "the collection does not contain exactly one setter");
    let setter = style.setters().get(0);
    let setter = as_setter(&setter);

    assert!(
        Some(TestTemplatedControl::test_data_property().as_property()) == setter_template_binding_property(setter)
    );
}

#[test]
fn fails_to_resolve_template_binding_in_style_without_template_metadata() {
    let _app = styled_window_application();
    let xaml = "
<Style xmlns='https://github.com/ferroui'
       Selector='Border'>
    <Setter Property='Tag' Value='{TemplateBinding TestData}'/>
</Style>";

    let exception = assert_throws_xml_exception(try_load(xaml));
    let message = xaml_error(&exception).expect("the error of the compiler").message();
    assert!(message.contains("ControlTemplate"), "the message: {message}");
}

#[test]
fn can_use_classes_in_setter() {
    let _app = styled_window_application();
    let xaml = r#"<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
             xmlns:u='using:FerroUI.Markup.Xaml.UnitTests.Xaml'>
    <Window.Styles>
        <Style Selector="Border">
            <Setter Property="(Classes.Banned)" Value='true'/>

            <Style Selector="^.Banned">
               <Setter Property='Background' Value='Red'/>
            </Style>
        </Style>
    </Window.Styles>
    <Border/>
</Window>"#;

    let window = load_as::<Ref<Window>>(xaml);
    let border = try_object_of::<Border>(&window.content());
    let border = border.expect("the content is a Border");
    let red: Rc<dyn IBrush> = Brushes::red();
    assert_eq!(Some(red), border.background());
}

#[test]
fn can_binding_classes_in_setter() {
    let _app = styled_window_application();
    let xaml = r#"<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
             xmlns:u='using:FerroUI.Markup.Xaml.UnitTests.Xaml'
             xmlns:vm='using:FerroUI.Markup.Xaml.UnitTests'
             >
    <Window.Styles>
        <Style Selector="Border" x:DataType='vm:TestViewModel'>
            <Setter Property="(Classes.Banned)" Value='{Binding Boolean}'/>

            <Style Selector="^.Banned">
               <Setter Property='Background' Value='Red'/>
            </Style>
        </Style>
    </Window.Styles>
    <Window.DataContext>
       <vm:TestViewModel/>
    </Window.DataContext>
    <Border/>
</Window>"#;

    let window = load_as::<Ref<Window>>(xaml);
    window.apply_template();
    let vm = value_of::<Rc<TestViewModel>>(&window.data_context());
    let vm = vm.expect("the data context is a TestViewModel");

    let border = try_object_of::<Border>(&window.content());
    let border = border.expect("the content is a Border");
    assert!(border.background().is_none());
    vm.set_boolean(true);
    let red: Rc<dyn IBrush> = Brushes::red();
    assert_eq!(Some(red), border.background());
}

#[test]
fn fails_use_classes_in_setter_when_selector_is_complex() {
    let _app = styled_window_application();
    let xaml = r#"<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
             xmlns:u='using:FerroUI.Markup.Xaml.UnitTests.Xaml'>
    <Window.Styles>
        <Style Selector="Border:pointover">
            <Setter Property="(Classes.Banned)" Value='true'/>

            <Style Selector="^.Banned">
               <Setter Property='Background' Value='Red'/>
            </Style>
        </Style>
    </Window.Styles>
    <Border/>
</Window>"#;

    let exception = assert_throws_xml_exception(try_load(xaml));
    assert_eq!(
        "Cannot set Classes Binding property '(Classes.Banned)' because the style has an activator. Line 6, position 14.",
        xaml_error(&exception).expect("the error of the compiler").message()
    );
}

/// The body of the theory `No_Selector_Should_Target_Parent_Type`.
#[track_caller]
fn no_selector_should_target_parent_type(style_start: &str, style_end: &str) {
    let _app = styled_window_application();

    let window = load_as::<Ref<Window>>(&format!(
        r#"<Window xmlns="https://github.com/ferroui">
    <Window.Styles>
        {style_start}
            <Setter Property="Title" Value="title set via style!" />
        {style_end}
    </Window.Styles>
</Window>"#
    ));

    assert_eq!(Some("title set via style!".to_string()), window.title());
}

#[test]
fn no_selector_should_target_parent_type_row_1() {
    no_selector_should_target_parent_type("<Style>", "</Style>");
}

#[test]
fn no_selector_should_target_parent_type_row_2() {
    no_selector_should_target_parent_type("<Style Selector=''>", "</Style>");
}

#[test]
fn no_selector_should_target_parent_type_row_3() {
    no_selector_should_target_parent_type("<Styles><Style>", "</Style></Styles>");
}

#[test]
fn no_selector_should_target_parent_type_row_4() {
    no_selector_should_target_parent_type("<Styles><Style Selector=''>", "</Style></Styles>");
}

/// The body of the theory `No_Selector_Should_Fail_In_Control_Theme`.
#[track_caller]
fn no_selector_should_fail_in_control_theme(style_start: &str, style_end: &str) {
    let _app = styled_window_application();

    let exception = assert_throws_xml_exception(try_load(&format!(
        r#"<Window xmlns="https://github.com/ferroui"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
   <Window.Resources>
        <ControlTheme x:Key="{{x:Type Window}}" TargetType="Window">
            {style_start}
                <Setter Property="Title" Value="title set via style!" />
            {style_end}
        </ControlTheme>
    </Window.Resources>
</Window>"#
    )));

    assert_eq!(
        "Cannot add a Style without selector to a ControlTheme. Line 5, position 14.",
        xaml_error(&exception).expect("the error of the compiler").message()
    );
}

#[test]
fn no_selector_should_fail_in_control_theme_row_1() {
    no_selector_should_fail_in_control_theme("<Style>", "</Style>");
}

#[test]
fn no_selector_should_fail_in_control_theme_row_2() {
    no_selector_should_fail_in_control_theme("<Style Selector=''>", "</Style>");
}

#[test]
fn selector_should_not_resolve_to_markup_extension_type() {
    let _app = styled_window_application();

    let style = load_as::<Ref<Style>>(
        "<Style xmlns='https://github.com/ferroui'
        xmlns:u='using:FerroUI.Markup.Xaml.UnitTests.Xaml'
        Selector='u|TestSelectorControl'>
</Style>",
    );

    let selector = style.selector().expect("the style has a selector");

    let target_type = selector.target_type();

    assert!(target_type.is_some_and(|type_| std::ptr::eq(type_, TestSelectorControl::TYPE)));
    assert!(!target_type.is_some_and(|type_| std::ptr::eq(type_, TestSelectorControlExtension::TYPE)));
}
