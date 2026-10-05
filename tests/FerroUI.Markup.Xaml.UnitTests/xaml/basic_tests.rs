//! Port of `Xaml/BasicTests.cs`.

use ferroui_base::controls::ResourceKey;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::data::converters::BoolConverters;
use ferroui_base::data::{MultiBinding, ReflectionBinding};
use ferroui_base::media::{Brushes, Colors, IBrush};
use ferroui_base::styling::{Setter, Style, Styles};
use ferroui_base::{FerroProperty, IntoRef, Ref};
use ferroui_controls::documents::TextElement;
use ferroui_base::layout::Layoutable;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_markup_xaml::templates::ControlTemplate;
use ferroui_controls::testing::{MockWindowingPlatform, UnitTestApplicationScope};
use ferroui_controls::{
    Button, ContentControl, Flyout, Grid, GridLength, GridUnitType, ItemsControl, ItemsSource, ListBox, Panel, Slider,
    TextBlock, ToolTip, UserControl, Window,
};
use std::rc::Rc;

use crate::support::app::{
    mock_windowing_platform_application, styled_window_application, unit_test_application, xaml_test_base, TestServices,
};
use crate::support::helpers::{
    as_setter, assert_binding_is_type, assert_control_template_is_type, assert_string, assert_throws_xaml_diagnostic,
    assert_value, assert_value_is, setter_control_template,
    boxed, boxed_str, object_of, setter_plain_value, try_object_of, value_of,
};
use crate::support::loader::{cast, load_as, load_with_root, parse, try_load, try_parse};
use crate::support::xaml::basic_tests::{
    BasicTestsAttachedPropertyHolder, ObjectWithAddChild, ObjectWithAddChildOfT, ObjectWithoutPublicCtor,
    SelectedItemsViewModel,
};
use crate::support::xaml::{InitializationOrderTracker, NonControl, TestControl};

/// `(Style)styles[0]`.
#[track_caller]
fn style_at(styles: &Ref<Styles>, index: usize) -> Ref<Style> {
    let style = styles.get(index);
    let object = style.as_object().expect("the style is an object of the object model");
    object.to_ref().cast::<Style>().expect("the style is a Style")
}

/// `UnitTestApplication.Start(TestServices.MockPlatformWrapper.With(windowingPlatform: new MockWindowingPlatform()))`.
fn mock_platform_wrapper_with_windowing_platform_application() -> UnitTestApplicationScope {
    unit_test_application(TestServices::mock_platform_wrapper().with_windowing_platform(MockWindowingPlatform::new()))
}

#[test]
fn simple_property_is_set() {
    let _base = xaml_test_base();
    let xaml = "<ContentControl xmlns='https://github.com/ferroui' Content='Foo'/>";

    let target = parse::<Ref<ContentControl>>(xaml);

    assert_string("Foo", &target.content());
}

#[test]
fn default_content_property_is_set() {
    let _base = xaml_test_base();
    let xaml = "<ContentControl xmlns='https://github.com/ferroui'>Foo</ContentControl>";

    let target = parse::<Ref<ContentControl>>(xaml);

    assert_string("Foo", &target.content());
}

#[test]
fn content_presenter_default_content_property_is_set() {
    let _base = xaml_test_base();
    let xaml = "<ContentPresenter xmlns='https://github.com/ferroui'>Foo</ContentPresenter>";

    let target = parse::<Ref<ContentPresenter>>(xaml);

    assert_string("Foo", &target.content());
}

#[test]
fn attached_property_is_set() {
    let _base = xaml_test_base();
    let xaml = "<ContentControl xmlns='https://github.com/ferroui' TextElement.FontSize='21'/>";

    let target = parse::<Ref<ContentControl>>(xaml);

    assert_eq!(21.0, TextElement::get_font_size(&target));
}

#[test]
fn attached_property_is_set_on_control_outside_ferro_namespace() {
    let _base = xaml_test_base();
    // Test for issue #1548
    let xaml = "<UserControl xmlns='https://github.com/ferroui'
    xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
  <local:TestControl Grid.Column='2' />
</UserControl>";

    let target = parse::<Ref<UserControl>>(xaml);

    let content = object_of::<TestControl>(&target.content());
    assert_eq!(2, Grid::get_column(&content));
}

#[test]
fn attached_property_with_namespace_is_set() {
    let _base = xaml_test_base();
    let xaml = "<ContentControl xmlns='https://github.com/ferroui' 
                    xmlns:test='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'
                    test:BasicTestsAttachedPropertyHolder.Foo='Bar'/>";

    let target = parse::<Ref<ContentControl>>(xaml);

    assert_eq!(Some("Bar".to_string()), BasicTestsAttachedPropertyHolder::get_foo(&target));
}

#[test]
fn attached_property_supports_binding() {
    let _app = mock_windowing_platform_application();
    let xaml = "<Window xmlns='https://github.com/ferroui' TextElement.FontSize='{Binding}'/>";

    let target = parse::<Ref<ContentControl>>(xaml);

    target.set_data_context(Some(boxed(21.0f64)));

    assert_eq!(21.0, TextElement::get_font_size(&target));
}

#[test]
fn attached_property_in_panel_is_set() {
    let _base = xaml_test_base();
    let xaml = "
<Panel xmlns='https://github.com/ferroui'>
    <ToolTip.Tip>Foo</ToolTip.Tip>
</Panel>";

    let target = parse::<Ref<Panel>>(xaml);

    assert_eq!(0, target.children().count());

    assert_string("Foo", &ToolTip::get_tip(&target));
}

#[test]
fn non_existent_property_throws() {
    let _base = xaml_test_base();
    let xaml = "<ContentControl xmlns='https://github.com/ferroui' DoesntExist='foo'/>";

    assert_throws_xaml_diagnostic(
        try_parse::<Ref<ContentControl>>(xaml),
        "FRN2000",
        "Unable to resolve suitable regular or attached property DoesntExist on type FerroUI.Controls:FerroUI.Controls.ContentControl Line 1, position 2.",
    );
}

#[test]
fn content_control_content_template_is_functional() {
    let _base = xaml_test_base();
    let xaml = "<ContentControl xmlns='https://github.com/ferroui'>
    <ContentControl.ContentTemplate>
        <DataTemplate>
            <TextBlock Text='Foo' />
        </DataTemplate>
    </ContentControl.ContentTemplate>
</ContentControl>";

    let content_control = parse::<Ref<ContentControl>>(xaml);
    let target = content_control.content_template().expect("the content template is set");

    let txt = target.build(&None).expect("the template builds a control").cast::<TextBlock>().expect("a TextBlock");

    assert_eq!(Some("Foo".to_string()), txt.text());
}

#[test]
fn named_control_is_added_to_name_scope_simple() {
    let _base = xaml_test_base();
    let xaml = "
<UserControl xmlns='https://github.com/ferroui'>
    <Button Name='button'>Foo</Button>
</UserControl>";

    let control = parse::<Ref<UserControl>>(xaml);
    let button = control.get_control::<Button>("button");

    assert_string("Foo", &button.content());
}

#[test]
fn direct_content_in_items_control_is_operational() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'>
     <ItemsControl Name='items'>
         <ContentControl>Foo</ContentControl>
         <ContentControl>Bar</ContentControl>
      </ItemsControl>
</Window>";

    let control = parse::<Ref<Window>>(xaml);

    let items_control = control.get_control::<ItemsControl>("items");

    let items: Vec<Ref<ContentControl>> = items_control
        .items()
        .to_vec()
        .iter()
        .map(object_of::<ContentControl>)
        .collect();

    assert_string("Foo", &items[0].content());
    assert_string("Bar", &items[1].content());
}

#[test]
fn panel_children_are_added() {
    let _base = xaml_test_base();
    let xaml = "
<UserControl xmlns='https://github.com/ferroui'>
    <Panel Name='panel'>
        <ContentControl Name='Foo' />
        <ContentControl Name='Bar' />
    </Panel>
</UserControl>";

    let control = parse::<Ref<UserControl>>(xaml);

    let panel = control.get_control::<Panel>("panel");

    assert_eq!(2, panel.children().count());

    let foo = control.get_control::<ContentControl>("Foo");
    let bar = control.get_control::<ContentControl>("Bar");

    assert!(panel.children().contains(&foo.into_ref()));
    assert!(panel.children().contains(&bar.into_ref()));
}

#[test]
fn grid_row_col_definitions_are_built() {
    let _base = xaml_test_base();
    let xaml = "
<Grid xmlns='https://github.com/ferroui'>
    <Grid.ColumnDefinitions>
        <ColumnDefinition Width='100' />
        <ColumnDefinition Width='Auto' />
        <ColumnDefinition Width='*' />
        <ColumnDefinition Width='100*' />
    </Grid.ColumnDefinitions>
    <Grid.RowDefinitions>
        <RowDefinition Height='100' />
        <RowDefinition Height='Auto' />
        <RowDefinition Height='*' />
        <RowDefinition Height='100*' />
    </Grid.RowDefinitions>
</Grid>";

    let grid = parse::<Ref<Grid>>(xaml);

    assert_eq!(4, grid.column_definitions().count());
    assert_eq!(4, grid.row_definitions().count());

    let expected1 = GridLength::from_pixels(100.0);
    let expected2 = GridLength::AUTO;
    let expected3 = GridLength::new(1.0, GridUnitType::Star);
    let expected4 = GridLength::new(100.0, GridUnitType::Star);

    assert_eq!(expected1, grid.column_definitions().get(0).width());
    assert_eq!(expected2, grid.column_definitions().get(1).width());
    assert_eq!(expected3, grid.column_definitions().get(2).width());
    assert_eq!(expected4, grid.column_definitions().get(3).width());

    assert_eq!(expected1, grid.row_definitions().get(0).height());
    assert_eq!(expected2, grid.row_definitions().get(1).height());
    assert_eq!(expected3, grid.row_definitions().get(2).height());
    assert_eq!(expected4, grid.row_definitions().get(3).height());
}

#[test]
fn grid_row_col_definitions_are_parsed() {
    let _base = xaml_test_base();
    let xaml = "
<Grid xmlns='https://github.com/ferroui'
        ColumnDefinitions='100,Auto,*,100*'
        RowDefinitions='100,Auto,*,100*'>
</Grid>";

    let grid = parse::<Ref<Grid>>(xaml);

    assert_eq!(4, grid.column_definitions().count());
    assert_eq!(4, grid.row_definitions().count());

    let expected1 = GridLength::from_pixels(100.0);
    let expected2 = GridLength::AUTO;
    let expected3 = GridLength::new(1.0, GridUnitType::Star);
    let expected4 = GridLength::new(100.0, GridUnitType::Star);

    assert_eq!(expected1, grid.column_definitions().get(0).width());
    assert_eq!(expected2, grid.column_definitions().get(1).width());
    assert_eq!(expected3, grid.column_definitions().get(2).width());
    assert_eq!(expected4, grid.column_definitions().get(3).width());

    assert_eq!(expected1, grid.row_definitions().get(0).height());
    assert_eq!(expected2, grid.row_definitions().get(1).height());
    assert_eq!(expected3, grid.row_definitions().get(2).height());
    assert_eq!(expected4, grid.row_definitions().get(3).height());
}

#[test]
fn grid_row_col_definitions_are_parsed_space_delimiter() {
    let _base = xaml_test_base();
    let xaml = "
<Grid xmlns='https://github.com/ferroui'
        ColumnDefinitions='100 Auto * 100*'
        RowDefinitions='100 Auto * 100*'>
</Grid>";

    let grid = parse::<Ref<Grid>>(xaml);

    assert_eq!(4, grid.column_definitions().count());
    assert_eq!(4, grid.row_definitions().count());

    let expected1 = GridLength::from_pixels(100.0);
    let expected2 = GridLength::AUTO;
    let expected3 = GridLength::new(1.0, GridUnitType::Star);
    let expected4 = GridLength::new(100.0, GridUnitType::Star);

    assert_eq!(expected1, grid.column_definitions().get(0).width());
    assert_eq!(expected2, grid.column_definitions().get(1).width());
    assert_eq!(expected3, grid.column_definitions().get(2).width());
    assert_eq!(expected4, grid.column_definitions().get(3).width());

    assert_eq!(expected1, grid.row_definitions().get(0).height());
    assert_eq!(expected2, grid.row_definitions().get(1).height());
    assert_eq!(expected3, grid.row_definitions().get(2).height());
    assert_eq!(expected4, grid.row_definitions().get(3).height());
}

#[test]
fn named_x_control_is_added_to_name_scope_simple() {
    let _base = xaml_test_base();
    let xaml = "
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Button x:Name='button'>Foo</Button>
</UserControl>";

    let control = parse::<Ref<UserControl>>(xaml);
    let button = control.get_control::<Button>("button");

    assert_string("Foo", &button.content());
}

#[test]
fn standard_type_converter_is_used() {
    let _base = xaml_test_base();
    let xaml = "<UserControl xmlns='https://github.com/ferroui' Width='200.5' />";

    let control = parse::<Ref<UserControl>>(xaml);
    assert_eq!(200.5, control.width());
}

#[test]
fn ferro_type_converter_is_used() {
    let _base = xaml_test_base();
    let xaml = "<UserControl xmlns='https://github.com/ferroui' Background='White' />";

    let control = parse::<Ref<UserControl>>(xaml);
    let background = control.background().expect("the background is set");
    let brush =
        background.as_any().downcast_ref::<ImmutableSolidColorBrush>().expect("an ImmutableSolidColorBrush");
    assert_eq!(Colors::WHITE, brush.color());
}

#[test]
fn simple_style_is_parsed() {
    let _base = xaml_test_base();
    let xaml = "
<Styles xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Style Selector='TextBlock'>
        <Setter Property='Background' Value='White'/>
        <Setter Property='Width' Value='100'/>
    </Style>
</Styles>";

    let styles = parse::<Ref<Styles>>(xaml);

    assert_eq!(1, styles.count());

    let style = style_at(&styles, 0);

    let setters = style.setters().to_vec();
    let setters: Vec<&Setter> = setters.iter().map(as_setter).collect();

    assert_eq!(2, setters.len());

    assert!(setters[0].property() == Some(TextBlock::background_property().as_property()));
    // The plain value of a setter holds the value type of its property.
    let background = assert_value_is::<Option<Rc<dyn IBrush>>>(&setter_plain_value(setters[0]));
    assert_eq!(
        Brushes::white().color(),
        background
            .expect("the value is null")
            .as_solid_color_brush()
            .expect("the value is not a solid color brush")
            .color()
    );

    assert!(setters[1].property() == Some(Layoutable::width_property().as_property()));
    assert_value(100.0_f64, &setter_plain_value(setters[1]));
}

#[test]
fn style_setter_with_attached_property_is_parsed() {
    let _base = xaml_test_base();
    let xaml = "
<Styles xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Style Selector='ContentControl'>
        <Setter Property='TextBlock.FontSize' Value='21'/>
    </Style>
</Styles>";

    let styles = parse::<Ref<Styles>>(xaml);

    assert_eq!(1, styles.count());

    let style = style_at(&styles, 0);

    let setters = style.setters().to_vec();
    let setters: Vec<&Setter> = setters.iter().map(as_setter).collect();

    assert_eq!(1, setters.len());

    assert!(setters[0].property() == Some(TextBlock::font_size_property().as_property()));
    assert_value(21.0_f64, &setter_plain_value(setters[0]));
}

#[test]
fn complex_style_is_parsed() {
    let _app = styled_window_application();
    let xaml = "
<Styles xmlns='https://github.com/ferroui'>
  <Style Selector='CheckBox'>
    <Setter Property='BorderBrush' Value='{DynamicResource ThemeBorderMidBrush}'/>
    <Setter Property='BorderThickness' Value='{DynamicResource ThemeBorderThickness}'/>
    <Setter Property='Template'>
      <ControlTemplate>
        <Grid ColumnDefinitions='Auto,*'>
          <Border Name='border'
                  BorderBrush='{TemplateBinding BorderBrush}'
                  BorderThickness='{TemplateBinding BorderThickness}'
                  Width='18'
                  Height='18'
                  VerticalAlignment='Center'>
            <Path Name='checkMark'
                  Fill='{StaticResource HighlightBrush}'
                  Width='11'
                  Height='10'
                  Stretch='Uniform'
                  HorizontalAlignment='Center'
                  VerticalAlignment='Center'
                  Data='M 1145.607177734375,430 C1145.607177734375,430 1141.449951171875,435.0772705078125 1141.449951171875,435.0772705078125 1141.449951171875,435.0772705078125 1139.232177734375,433.0999755859375 1139.232177734375,433.0999755859375 1139.232177734375,433.0999755859375 1138,434.5538330078125 1138,434.5538330078125 1138,434.5538330078125 1141.482177734375,438 1141.482177734375,438 1141.482177734375,438 1141.96875,437.9375 1141.96875,437.9375 1141.96875,437.9375 1147,431.34619140625 1147,431.34619140625 1147,431.34619140625 1145.607177734375,430 1145.607177734375,430 z'/>
          </Border>
          <ContentPresenter Name='PART_ContentPresenter'
                            Content='{TemplateBinding Content}'
                            ContentTemplate='{TemplateBinding ContentTemplate}'
                            Margin='4,0,0,0'
                            VerticalAlignment='Center'
                            Grid.Column='1'/>
        </Grid>
      </ControlTemplate>
    </Setter>
  </Style>
</Styles>
";

    let styles = parse::<Ref<Styles>>(xaml);

    assert_eq!(1, styles.count());

    let style = style_at(&styles, 0);

    let setters = style.setters().to_vec();
    let setters: Vec<&Setter> = setters.iter().map(as_setter).collect();

    assert_eq!(3, setters.len());

    assert!(setters[0].property() == Some(TemplatedControl::border_brush_property().as_property()));
    assert!(setters[1].property() == Some(TemplatedControl::border_thickness_property().as_property()));
    assert!(setters[2].property() == Some(TemplatedControl::template_property().as_property()));

    let template = setter_control_template(setters[2]);
    assert_control_template_is_type::<ControlTemplate>(&template);
}

#[test]
fn style_resources_are_built() {
    let _base = xaml_test_base();
    let xaml = "
<Style xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:sys='clr-namespace:System;assembly=netstandard'>
    <Style.Resources>
        <SolidColorBrush x:Key='Brush'>White</SolidColorBrush>
        <sys:Double x:Key='Double'>10</sys:Double>
    </Style.Resources>
</Style>";

    let style = parse::<Ref<Style>>(xaml);

    assert!(style.resources().count() > 0);

    let brush = style.try_get_resource(&ResourceKey::from("Brush"), None).flatten();

    assert!(brush.is_some());
    let brush = value_of::<Rc<dyn IBrush>>(&brush).expect("the resource is a brush");
    let brush = brush.as_solid_color_brush().expect("the resource is a solid color brush");
    assert_eq!(Colors::WHITE, brush.color());

    let d = style.try_get_resource(&ResourceKey::from("Double"), None).flatten();

    assert_value(10.0f64, &d);
}

#[test]
fn simple_xaml_binding_is_operational() {
    let _app = mock_platform_wrapper_with_windowing_platform_application();
    let xaml = "<Window xmlns='https://github.com/ferroui' Content='{Binding}'/>";

    let target = parse::<Ref<ContentControl>>(xaml);

    assert!(target.content().is_none());

    target.set_data_context(boxed_str("Foo"));

    assert_string("Foo", &target.content());
}

#[test]
fn double_xaml_binding_is_operational() {
    let _app = mock_platform_wrapper_with_windowing_platform_application();
    let xaml = "<Window xmlns='https://github.com/ferroui' Width='{Binding}'/>";

    let target = parse::<Ref<ContentControl>>(xaml);

    assert!(target.content().is_none());

    target.set_data_context(Some(boxed(55.0f64)));

    assert_eq!(55.0, target.width());
}

#[test]
fn collection_xaml_binding_is_operational() {
    let _app = mock_platform_wrapper_with_windowing_platform_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'>
    <ItemsControl Name='itemsControl' ItemsSource='{Binding}'>
    </ItemsControl>
</Window>
";

    let target = parse::<Ref<Window>>(xaml);

    assert!(target.content().is_some());

    let items_control = target.get_control::<ItemsControl>("itemsControl");

    let items = ItemsSource::from_strs(["Foo", "Bar"]);

    target.set_data_context(Some(boxed(items.clone())));

    assert_eq!(Some(items), items_control.items_source());
}

#[test]
fn multi_xaml_binding_is_parsed() {
    let _base = xaml_test_base();
    let xaml = "<MultiBinding xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
    Converter ='{x:Static BoolConverters.And}'>
     <Binding Path='Foo' />
     <Binding Path='Bar' />
</MultiBinding>";

    let target = parse::<Rc<MultiBinding>>(xaml);

    assert_eq!(2, target.bindings().count());

    assert!(Some(BoolConverters::and()) == target.converter());

    let bindings = target.bindings().to_vec();
    let bindings: Vec<&ReflectionBinding> =
        bindings.iter().map(|binding| assert_binding_is_type::<ReflectionBinding>(binding)).collect();

    assert_eq!("Foo", bindings[0].path());
    assert_eq!("Bar", bindings[1].path());
}

#[test]
fn control_template_is_operational() {
    let _app = mock_platform_wrapper_with_windowing_platform_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Template>
        <ControlTemplate TargetType='Window'>
            <ContentPresenter Name='PART_ContentPresenter'
                        Content='{TemplateBinding Content}'/>
        </ControlTemplate>
    </Window.Template>
</Window>";

    let target = parse::<Ref<ContentControl>>(xaml);

    assert!(target.template().is_some());

    assert!(target.presenter().is_none());

    target.apply_template();

    assert!(target.presenter().is_some());

    target.set_content(boxed_str("Foo"));

    assert_string("Foo", &target.presenter().expect("the presenter is set").content());
}

#[test]
fn style_control_template_is_built() {
    let _base = xaml_test_base();
    let xaml = "
<Style xmlns='https://github.com/ferroui' Selector='ContentControl'>
  <Setter Property='Template'>
     <ControlTemplate>
        <ContentPresenter Name='PART_ContentPresenter'
                       Content='{TemplateBinding Content}'
                       ContentTemplate='{TemplateBinding ContentTemplate}' />
      </ControlTemplate>
  </Setter>
</Style> ";

    let style = parse::<Ref<Style>>(xaml);

    assert_eq!(1, style.setters().count());

    let setter = style.setters().get(0);
    let setter = as_setter(&setter);

    assert!(setter.property() == Some(TemplatedControl::template_property().as_property()));
    let value = setter_control_template(setter);
    let template = assert_control_template_is_type::<ControlTemplate>(&value);

    let control = ContentControl::new();

    let result = template.build(&control.clone().upcast()).expect("the template builds a control");
    let result = result.result().cast::<ContentPresenter>();

    assert!(result.is_some());
}

#[test]
fn named_control_is_added_to_name_scope() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Button Name='button'>Foo</Button>
</Window>";

    let window = parse::<Ref<Window>>(xaml);
    let button = window.get_control::<Button>("button");

    assert_string("Foo", &button.content());
}

#[test]
fn control_is_added_to_parent_before_properties_are_set() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
             xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <local:InitializationOrderTracker Width='100'/>
</Window>";

    let window = parse::<Ref<Window>>(xaml);
    let tracker = object_of::<InitializationOrderTracker>(&window.content());

    let attached = tracker.index_of("AttachedToLogicalTree");
    let width_changed = tracker.index_of("Property Width Changed");

    assert_ne!(-1, attached);
    assert_ne!(-1, width_changed);
    assert!(attached < width_changed);
}

#[test]
fn control_is_added_to_parent_before_final_end_init() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
             xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <local:InitializationOrderTracker Width='100'/>
</Window>";

    let window = parse::<Ref<Window>>(xaml);
    let tracker = object_of::<InitializationOrderTracker>(&window.content());

    let attached = tracker.index_of("AttachedToLogicalTree");
    let end_init = tracker.index_of("EndInit 0");

    assert_ne!(-1, attached);
    assert_ne!(-1, end_init);
    assert!(attached < end_init);
}

#[test]
#[ignore = "themes: needs the Simple theme (upstream runs this test under it)"]
fn all_properties_are_set_before_final_end_init() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
                xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <local:InitializationOrderTracker Width='100' Height='100'
        Tag='{Binding Height, RelativeSource={RelativeSource Self}}' />
</Window>";

    let window = parse::<Ref<Window>>(xaml);
    let tracker = object_of::<InitializationOrderTracker>(&window.content());

    //ensure binding is set and operational first
    assert_value(100.0f64, &tracker.tag());

    let order = tracker.order();

    // EndInit should be second-to-last operation, as last operation will be
    // caused by styling being applied on EndInit.
    assert_eq!("EndInit 0", order[order.len() - 3]);

    // Caused by styling.
    assert_eq!("Property FontFamily Changed", order[order.len() - 2]);
    assert_eq!("Property Foreground Changed", order[order.len() - 1]);
}

#[test]
fn begin_init_matches_end_init() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
             xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <local:InitializationOrderTracker />
</Window>";

    let window = parse::<Ref<Window>>(xaml);
    let tracker = object_of::<InitializationOrderTracker>(&window.content());

    assert_eq!(0, tracker.init_state());
}

#[test]
fn deferred_xaml_loader_should_preserve_namespaces_context() {
    let _base = xaml_test_base();
    let xaml = "<ContentControl xmlns='https://github.com/ferroui'
            xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
            xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <ContentControl.ContentTemplate>
        <DataTemplate>
            <TextBlock  Tag='{x:Static local:NonControl.StringProperty}'/>
        </DataTemplate>
    </ContentControl.ContentTemplate>
</ContentControl>";

    let content_control = parse::<Ref<ContentControl>>(xaml);
    let template = content_control.content_template();

    let template = template.expect("the content template is set");

    let txt = template.build(&None).expect("the template builds a control").cast::<TextBlock>().expect("a TextBlock");

    let expected: &'static FerroProperty = NonControl::string_property();
    assert_eq!(Some(expected), value_of::<&'static FerroProperty>(&txt.tag()));
}

#[test]
fn binding_to_list_ferro_property_is_operational() {
    let _app = mock_windowing_platform_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'>
    <ListBox ItemsSource='{Binding Items}' SelectedItems='{Binding SelectedItems}'/>
</Window>";

    let window = parse::<Ref<Window>>(xaml);
    let list_box = object_of::<ListBox>(&window.content());

    let vm = SelectedItemsViewModel::new();
    vm.set_items(Some(ItemsSource::from_strs(["foo", "bar", "baz"])));

    window.set_data_context(Some(vm.clone()));

    assert_eq!(vm.items(), list_box.items_source());

    assert!(Some(vm.selected_items()) == list_box.selected_items());
}

#[test]
fn element_whitespace_should_be_trimmed() {
    let _app = mock_windowing_platform_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'>
    <TextBlock>
        Hello World!
    </TextBlock>
</Window>";

    let window = parse::<Ref<Window>>(xaml);
    let text_block = object_of::<TextBlock>(&window.content());

    assert_eq!(Some("Hello World!".to_string()), text_block.text());
}

#[test]
fn slider_properties_can_be_set_in_any_order() {
    let _app = mock_windowing_platform_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'>
    <Slider Width='400' Value='500' Minimum='0' Maximum='1000'/>
</Window>";

    let window = parse::<Ref<Window>>(xaml);
    let slider = object_of::<Slider>(&window.content());

    assert_eq!(0.0, slider.minimum());
    assert_eq!(1000.0, slider.maximum());
    assert_eq!(500.0, slider.value());
}

#[test]
fn should_parse_tip_with_comment() {
    let _base = xaml_test_base();
    let xaml = "
                <TextBlock xmlns='https://github.com/ferroui' Text='TextBlock with tooltip'>
                    <ToolTip.Tip>
                        <!--Comment-->
                        <ToolTip>
                            Foo
                        </ToolTip>
                    </ToolTip.Tip>
                </TextBlock>";

    let text_block = parse::<Ref<TextBlock>>(xaml);

    let tool_tip = try_object_of::<ToolTip>(&ToolTip::get_tip(&text_block));

    let tool_tip = tool_tip.expect("the tip is a ToolTip");

    assert_string("Foo", &tool_tip.content());
}

#[test]
fn add_child_child_is_set() {
    let _base = xaml_test_base();
    let xaml = "<ObjectWithAddChild  xmlns='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml'>Foo</ObjectWithAddChild>";

    let target = parse::<Rc<ObjectWithAddChild>>(xaml);

    assert_string("Foo", &target.child());
}

#[test]
fn add_child_of_t_child_is_set() {
    let _base = xaml_test_base();
    let xaml =
        "<ObjectWithAddChildOfT  xmlns='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml'>Foo</ObjectWithAddChildOfT>";

    let target = parse::<Rc<ObjectWithAddChildOfT>>(xaml);

    assert!(target.child().is_none());
    assert_eq!(Some("Foo".to_string()), target.text());
}

#[test]
fn should_parse_and_populate_type_without_public_ctor() {
    let _base = xaml_test_base();
    let xaml = "<ObjectWithoutPublicCtor xmlns='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml' Test2='World' />";
    let target =
        cast::<Rc<ObjectWithoutPublicCtor>>(&load_with_root(xaml, None, ObjectWithoutPublicCtor::new("Hello")));

    assert_eq!(Some("World".to_string()), target.test2());
    assert_eq!(Some("Hello".to_string()), target.test1());
}

#[test]
fn can_specify_button_classes() {
    let _base = xaml_test_base();
    let xaml = "<Button xmlns='https://github.com/ferroui' Classes='foo bar'/>";
    let target = load_as::<Ref<Button>>(xaml);

    assert!(target.classes().contains("foo"));
    assert!(target.classes().contains("bar"));
}

#[test]
fn can_specify_button_classes_longform() {
    let _base = xaml_test_base();
    let xaml = "
<Button xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
  <Button.Classes>
    <x:String>foo</x:String>
    <x:String>bar</x:String>
  </Button.Classes>
</Button>";
    let target = load_as::<Ref<Button>>(xaml);

    assert!(target.classes().contains("foo"));
    assert!(target.classes().contains("bar"));
}

#[test]
fn can_specify_flyout_flyout_presenter_classes() {
    let _base = xaml_test_base();
    let xaml = "<Flyout xmlns='https://github.com/ferroui' FlyoutPresenterClasses='foo bar'/>";
    let target = load_as::<Ref<Flyout>>(xaml);

    assert_eq!(vec!["foo".to_string(), "bar".to_string()], *target.flyout_presenter_classes().snapshot());
}

#[test]
fn trying_to_bind_items_control_items_throws() {
    let _base = xaml_test_base();
    let xaml = "<ItemsControl xmlns='https://github.com/ferroui' Items='{Binding}'/>";

    assert_throws_xaml_diagnostic(
        try_load(xaml),
        "FRN3000",
        "Unable to find suitable setter or adder for property Items of type FerroUI.Controls:FerroUI.Controls.ItemsControl for argument FerroUI.Base:FerroUI.Data.ReflectionBinding, available setter parameter lists are:\nSystem.Object Line 1, position 50.",
    );
}
