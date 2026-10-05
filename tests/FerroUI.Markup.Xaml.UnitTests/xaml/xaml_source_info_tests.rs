//! Port of `Xaml/XamlSourceInfoTests.cs`.

use std::rc::Rc;

use ferroui_base::animation::DoubleTransition;
use ferroui_base::animation::Animation;
use ferroui_base::controls::{IDeferredContent, IResourceDictionary, IResourceProvider, ResourceDictionary, ResourceKey};
use ferroui_base::input::gesture_recognizers::{PullGestureRecognizer, ScrollGestureRecognizer};
use ferroui_base::media::{LinearGradientBrush, PathGeometry};
use ferroui_base::styling::{ContainerQuery, Style, ThemeVariant};
use ferroui_base::{BoxedValue, ObjectType, Ref};
use ferroui_controls::shapes::{Ellipse, Line, Path, Polygon, Rectangle};
use ferroui_controls::{Border, Button, Canvas, StackPanel, TextBlock, TextBox, UserControl};
use ferroui_markup_xaml::diagnostics::XamlSourceInfo;
use ferroui_markup_xaml::templates::DataTemplate;
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument};

use crate::support::app::xaml_test_base;
use crate::support::helpers::{boxed, object_of, value_of};
use crate::support::loader::{cast, load_document, uri};

/// The configuration of the tests: source info is created.
fn configuration() -> RuntimeXamlLoaderConfiguration {
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.set_create_source_info(true);
    configuration
}

/// `(UserControl)FerroRuntimeXamlLoader.Load(document, s_configuration)`.
fn load_user_control(document: RuntimeXamlLoaderDocument) -> Ref<UserControl> {
    cast::<Ref<UserControl>>(&load_document(document, Some(configuration())))
}

/// `XamlSourceInfo.GetXamlSourceInfo(obj)` for an object of the object model.
fn source_info_of<T: ObjectType>(obj: &Ref<T>) -> Option<XamlSourceInfo> {
    XamlSourceInfo::get_xaml_source_info(&boxed(obj.clone()))
}

/// The cast of a resource provider to the resource dictionary contract.
fn as_resource_dictionary(provider: &dyn IResourceProvider) -> Rc<dyn IResourceDictionary> {
    let dictionary = provider
        .as_object()
        .and_then(|object| object.to_ref().cast::<ResourceDictionary>())
        .expect("the provider is a resource dictionary");
    dictionary.into()
}

/// `dictionary[key]!`.
fn resource(dictionary: &Rc<dyn IResourceDictionary>, key: &str) -> BoxedValue {
    dictionary
        .try_get_value(&ResourceKey::from(key))
        .flatten()
        .unwrap_or_else(|| panic!("the dictionary has no resource '{key}'"))
}

// The rows of the theory: the document, and the URI a file URI builder makes of it as path.
// (The first row is a Windows-style path, the second a Unix-style path.)
const DOCUMENTS: [(&str, &str); 2] = [
    (r"C:\TestFolder\TestFile.xaml", "file:///C:/TestFolder/TestFile.xaml"),
    ("/TestFolder/TestFile.xaml", "file:///TestFolder/TestFile.xaml"),
];

#[test]
fn root_user_control_with_base_uri_gets_xaml_source_info_source_uri_set() {
    for (document, expected) in DOCUMENTS {
        let _base = xaml_test_base();
        let mut xaml_document = RuntimeXamlLoaderDocument::new(
            "<UserControl xmlns='https://github.com/ferroui'
     xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
</UserControl>",
        );
        xaml_document.document = Some(document.to_string());

        let user_control = load_user_control(xaml_document);

        let source_info = source_info_of(&user_control);

        assert!(source_info.is_some(), "row {document:?}");
        let source_info = source_info.unwrap();
        let source_uri = source_info.source_uri().unwrap_or_else(|| panic!("row {document:?}: no source URI"));
        assert_eq!("file", source_uri.scheme(), "row {document:?}");
        assert!(source_uri.is_absolute_uri(), "row {document:?}");
        assert_eq!(&uri(expected), source_uri, "row {document:?}");
    }
}

#[test]
fn root_user_control_gets_xaml_source_info_set() {
    let _base = xaml_test_base();
    let xaml = RuntimeXamlLoaderDocument::new(
        "
<UserControl xmlns='https://github.com/ferroui'
     xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
</UserControl>",
    );

    let user_control = load_user_control(xaml);

    let source_info = source_info_of(&user_control);

    assert!(source_info.is_some());
}

#[test]
fn nested_controls_all_get_xaml_source_info_set() {
    let _base = xaml_test_base();
    let xaml = RuntimeXamlLoaderDocument::new(
        "
<UserControl xmlns='https://github.com/ferroui'
     xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <StackPanel>
        <Button />
        <TextBlock />
    </StackPanel>
</UserControl>",
    );

    let user_control = load_user_control(xaml);
    let stack_panel = object_of::<StackPanel>(&user_control.content());
    let button = stack_panel.children().get(0).cast::<Button>().expect("the first child is a Button");
    let textblock = stack_panel.children().get(1).cast::<TextBlock>().expect("the second child is a TextBlock");

    let user_control_source_info = source_info_of(&user_control);
    assert!(user_control_source_info.is_some());

    let stack_panel_source_info = source_info_of(&stack_panel);
    assert!(stack_panel_source_info.is_some());

    let button_source_info = source_info_of(&button);
    assert!(button_source_info.is_some());

    let textblock_source_info = source_info_of(&textblock);
    assert!(textblock_source_info.is_some());
}

#[test]
fn property_elements_get_xaml_source_info_set() {
    let _base = xaml_test_base();
    let xaml = RuntimeXamlLoaderDocument::new(
        r#"
<UserControl xmlns='https://github.com/ferroui'
     xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Rectangle Fill="Blue" Width="63" Height="41">
        <Rectangle.OpacityMask>
            <LinearGradientBrush StartPoint="0%,0%" EndPoint="100%,100%">
                <LinearGradientBrush.GradientStops>
                    <GradientStop Offset="0" Color="Black"/>
                    <GradientStop Offset="1" Color="Transparent"/>
                </LinearGradientBrush.GradientStops>
            </LinearGradientBrush>
        </Rectangle.OpacityMask>
    </Rectangle>
</UserControl>"#,
    );

    let user_control = load_user_control(xaml);
    let rect = object_of::<Rectangle>(&user_control.content());
    let gradient = rect
        .opacity_mask()
        .expect("the opacity mask is set")
        .as_object()
        .and_then(|object| object.to_ref().cast::<LinearGradientBrush>())
        .expect("the opacity mask is a LinearGradientBrush");
    let stops = gradient.gradient_stops();
    let stop_one = stops.get(0);
    let stop_two = stops.get(stops.len() - 1);

    let rect_source_info = source_info_of(&rect);
    assert!(rect_source_info.is_some());

    let gradient_source_info = source_info_of(&gradient);
    assert!(gradient_source_info.is_some());

    let stop_one_source_info = source_info_of(&stop_one);
    assert!(stop_one_source_info.is_some());

    let stop_two_source_info = source_info_of(&stop_two);
    assert!(stop_two_source_info.is_some());
}

#[test]
fn shapes_get_xaml_source_info_set() {
    let _base = xaml_test_base();
    let xaml = RuntimeXamlLoaderDocument::new(
        r#"
<UserControl xmlns='https://github.com/ferroui'
     xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Canvas Name="TheCanvas" Background="Yellow" Width="300" Height="400">
        <Ellipse Fill="Green" Width="58" Height="58" Canvas.Left="88" Canvas.Top="100"/>
        <Path Fill="Orange" Canvas.Left="30" Canvas.Top="250"/>
        <Path Fill="OrangeRed" Canvas.Left="180" Canvas.Top="250">
            <Path.Data>
                <PathGeometry>
                    <PathFigure StartPoint="0,0" IsClosed="True">
                        <QuadraticBezierSegment Point1="50,0" Point2="50,-50" />
                        <QuadraticBezierSegment Point1="100,-50" Point2="100,0" />
                        <LineSegment Point="50,0" />
                        <LineSegment Point="50,50" />
                    </PathFigure>
                </PathGeometry>
            </Path.Data>
        </Path>
        <Line StartPoint="120,185" EndPoint="30,115" Stroke="Red" StrokeThickness="2"/>
        <Polygon Points="75,0 120,120 0,45 150,45 30,120" Stroke="DarkBlue" StrokeThickness="1" Fill="Violet" Canvas.Left="150" Canvas.Top="31"/>
    </Canvas>
</UserControl>"#,
    );

    let user_control = load_user_control(xaml);
    let canvas = object_of::<Canvas>(&user_control.content());
    let ellipse = canvas.children().get(0).cast::<Ellipse>().expect("child 0 is an Ellipse");
    let path1 = canvas.children().get(1).cast::<Path>().expect("child 1 is a Path");
    let path2 = canvas.children().get(2).cast::<Path>().expect("child 2 is a Path");
    let geometry = path2
        .data()
        .expect("the data of the second path is set")
        .cast::<PathGeometry>()
        .expect("the data of the second path is a PathGeometry");
    let figure = geometry.figures().expect("the geometry has figures").get(0);
    let segments = figure.segments().expect("the figure has segments");
    let segment1 = segments.get(0);
    let segment2 = segments.get(1);
    let segment3 = segments.get(2);
    let segment4 = segments.get(3);
    let line = canvas.children().get(3).cast::<Line>().expect("child 3 is a Line");
    let polygon = canvas.children().get(4).cast::<Polygon>().expect("child 4 is a Polygon");

    let canvas_source_info = source_info_of(&canvas);
    assert!(canvas_source_info.is_some());

    let ellipse_source_info = source_info_of(&ellipse);
    assert!(ellipse_source_info.is_some());

    let path1_source_info = source_info_of(&path1);
    assert!(path1_source_info.is_some());

    let path2_source_info = source_info_of(&path2);
    assert!(path2_source_info.is_some());

    let geometry_source_info = source_info_of(&geometry);
    assert!(geometry_source_info.is_some());

    let figure_source_info = source_info_of(&figure);
    assert!(figure_source_info.is_some());

    let segment1_source_info = source_info_of(&segment1);
    assert!(segment1_source_info.is_some());

    let segment2_source_info = source_info_of(&segment2);
    assert!(segment2_source_info.is_some());

    let segment3_source_info = source_info_of(&segment3);
    assert!(segment3_source_info.is_some());

    let segment4_source_info = source_info_of(&segment4);
    assert!(segment4_source_info.is_some());

    let line_source_info = source_info_of(&line);
    assert!(line_source_info.is_some());

    let polygon_source_info = source_info_of(&polygon);
    assert!(polygon_source_info.is_some());
}

#[test]
fn styles_get_xaml_source_info_set() {
    let _base = xaml_test_base();
    let xaml = RuntimeXamlLoaderDocument::new(
        "
<UserControl xmlns='https://github.com/ferroui'
     xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
\t<UserControl.Styles>
\t\t<Style Selector=\"Button\">
\t\t\t<Setter Property=\"Margin\" Value=\"5\" />
\t\t</Style>
\t\t<ContainerQuery Name=\"container\"
\t\t\t\t\t\tQuery=\"max-width:400\">
\t\t\t<Style Selector=\"Button\">
\t\t\t\t<Setter Property=\"Background\"
\t\t\t\t\t\tValue=\"Red\"/>
\t\t\t</Style>
\t\t</ContainerQuery>
    </UserControl.Styles>
</UserControl>",
    );

    let user_control = load_user_control(xaml);
    let style = user_control
        .styles()
        .get(0)
        .as_object()
        .and_then(|object| object.to_ref().cast::<Style>())
        .expect("the first style is a Style");
    let query = user_control
        .styles()
        .get(1)
        .as_object()
        .and_then(|object| object.to_ref().cast::<ContainerQuery>())
        .expect("the second style is a ContainerQuery");

    let style_source_info = source_info_of(&style);
    assert!(style_source_info.is_some());

    let query_source_info = source_info_of(&query);
    assert!(query_source_info.is_some());
}

#[test]
fn animations_get_xaml_source_info_set() {
    let _base = xaml_test_base();
    let xaml = RuntimeXamlLoaderDocument::new(
        "
<UserControl xmlns='https://github.com/ferroui'
     xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
\t<UserControl.Styles>
\t\t<Style Selector=\"Rectangle.red\">
\t\t\t<Setter Property=\"Fill\" Value=\"Red\"/>
\t\t\t<Style.Animations>
\t\t\t\t<Animation Duration=\"0:0:3\">
\t\t\t\t\t<KeyFrame Cue=\"0%\">
\t\t\t\t\t\t<Setter Property=\"Opacity\" Value=\"0.0\"/>
\t\t\t\t\t</KeyFrame>
\t\t\t\t\t<KeyFrame Cue=\"100%\">
\t\t\t\t\t\t<Setter Property=\"Opacity\" Value=\"1.0\"/>
\t\t\t\t\t</KeyFrame>
\t\t\t\t</Animation>
\t\t\t</Style.Animations>
\t\t</Style>
    </UserControl.Styles>
</UserControl>",
    );

    let user_control = load_user_control(xaml);
    let style = user_control
        .styles()
        .get(0)
        .as_object()
        .and_then(|object| object.to_ref().cast::<Style>())
        .expect("the first style is a Style");
    let animation = style
        .animations()
        .get(0)
        .as_object()
        .and_then(|object| object.cast::<Animation>())
        .expect("the first animation is an Animation");
    let frame1 = animation.children().get(0);
    let frame2 = animation.children().get(1);

    let style_source_info = source_info_of(&style);
    assert!(style_source_info.is_some());

    let animation_source_info = source_info_of(&animation);
    assert!(animation_source_info.is_some());

    let frame_one_source_info = source_info_of(&frame1);
    assert!(frame_one_source_info.is_some());

    let frame_two_source_info = source_info_of(&frame2);
    assert!(frame_two_source_info.is_some());
}

#[test]
fn data_templates_and_deferred_contents_get_xaml_source_info_set() {
    let _base = xaml_test_base();
    let xaml = RuntimeXamlLoaderDocument::new(
        "
<UserControl xmlns='https://github.com/ferroui'
     xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
     xmlns:local='using:FerroUI.Markup.Xaml.UnitTests.Xaml'>
\t<UserControl.DataTemplates>
\t\t<DataTemplate DataType=\"local:SourceInfoTestViewModel\">
\t\t\t<Border Background=\"Red\" CornerRadius=\"8\">
\t\t\t\t<TextBox Text=\"{Binding Name}\"/>
\t\t\t</Border>
\t\t</DataTemplate>
\t</UserControl.DataTemplates>
</UserControl>",
    );

    let user_control = load_user_control(xaml);
    let datatemplate = DataTemplate::from_data_template(&user_control.data_templates().get(0))
        .expect("the first data template is a DataTemplate");

    // The template and it's content is deferred as not used (yet)
    let content = datatemplate.content();
    let border = match value_of::<Rc<dyn IDeferredContent>>(&content) {
        // (The deferred content of the port builds the result itself, not a template result.)
        Some(deferred_content) => object_of::<Border>(&deferred_content.build(None)),
        None => object_of::<Border>(&content),
    };

    let text_box = border
        .child()
        .and_then(|child| child.cast::<TextBox>())
        .expect("the child of the border is a TextBox");

    let datatemplate: BoxedValue = datatemplate;
    let datatemplate_source_info = XamlSourceInfo::get_xaml_source_info(&datatemplate);
    assert!(datatemplate_source_info.is_some());

    let border_source_info = source_info_of(&border);
    assert!(border_source_info.is_some());

    let text_box_source_info = source_info_of(&text_box);
    assert!(text_box_source_info.is_some());
}

#[test]
fn resources_get_xaml_source_info_set() {
    let _base = xaml_test_base();
    let xaml = RuntimeXamlLoaderDocument::new(
        "
<UserControl xmlns='https://github.com/ferroui'
     xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
\t<UserControl.Resources>
\t\t<ResourceDictionary>
            <ResourceDictionary.ThemeDictionaries>
\t\t        <ResourceDictionary x:Key='Light'>
\t\t            <SolidColorBrush x:Key='BackgroundBrush' Color='White'/>
\t\t            <SolidColorBrush x:Key='ForegroundBrush' Color='Black'/>
\t\t        </ResourceDictionary>
\t\t        <ResourceDictionary x:Key='Dark'>
\t\t            <SolidColorBrush x:Key='BackgroundBrush' Color='Black'/>
\t\t            <SolidColorBrush x:Key='ForegroundBrush' Color='White'/>
\t\t        </ResourceDictionary>
\t\t    </ResourceDictionary.ThemeDictionaries>

\t\t    <SolidColorBrush x:Key=\"Background\" Color=\"Yellow\" />
\t\t    <SolidColorBrush x:Key='OtherBrush'>Black</SolidColorBrush>

\t\t</ResourceDictionary>
\t</UserControl.Resources>
</UserControl>",
    );

    let user_control = load_user_control(xaml);
    let resources: Rc<dyn IResourceDictionary> = user_control.resources().into();
    let background_brush = resource(&resources, "Background");
    let light_dictionary = user_control
        .resources()
        .theme_dictionaries()
        .get(&ThemeVariant::light())
        .as_object()
        .and_then(|object| object.to_ref().cast::<ResourceDictionary>())
        .expect("the light theme dictionary is a ResourceDictionary");
    let dark_dictionary = user_control
        .resources()
        .theme_dictionaries()
        .get(&ThemeVariant::dark())
        .as_object()
        .and_then(|object| object.to_ref().cast::<ResourceDictionary>())
        .expect("the dark theme dictionary is a ResourceDictionary");
    let light_resources: Rc<dyn IResourceDictionary> = light_dictionary.clone().into();
    let light_foreground = resource(&light_resources, "ForegroundBrush");
    let dark_background = resource(&light_resources, "BackgroundBrush");
    let other_brush = resource(&resources, "OtherBrush");

    let background_brush_source_info = XamlSourceInfo::get_xaml_source_info(&background_brush);
    assert!(background_brush_source_info.is_some());

    let light_dictionary_source_info = source_info_of(&light_dictionary);
    assert!(light_dictionary_source_info.is_some());

    let dark_dictionary_source_info = source_info_of(&dark_dictionary);
    assert!(dark_dictionary_source_info.is_some());

    let light_foreground_source_info = XamlSourceInfo::get_xaml_source_info(&light_foreground);
    assert!(light_foreground_source_info.is_some());

    let dark_background_source_info = XamlSourceInfo::get_xaml_source_info(&dark_background);
    assert!(dark_background_source_info.is_some());

    let other_brush_source_info = XamlSourceInfo::get_xaml_source_info(&other_brush);
    assert!(other_brush_source_info.is_some());
}

#[test]
fn resource_dictionary_value_types_do_not_set_xaml_source_info() {
    let _base = xaml_test_base();
    let xaml = RuntimeXamlLoaderDocument::new(
        "
<UserControl xmlns='https://github.com/ferroui'
     xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
\t<UserControl.Resources>
\t\t<x:String x:Key='text'>foobar</x:String>
\t\t<x:Double x:Key=\"A_Double\">123.3</x:Double>
\t\t<x:Int16 x:Key=\"An_Int16\">123</x:Int16>
\t\t<x:Int32 x:Key=\"An_Int32\">37434323</x:Int32>
\t\t<Thickness x:Key=\"PreferredPadding\">10,20,10,0</Thickness>
\t</UserControl.Resources>
</UserControl>",
    );

    let user_control = load_user_control(xaml);
    let resources: Rc<dyn IResourceDictionary> = user_control.resources().into();
    let foobar_string = resource(&resources, "text");
    let a_double = resource(&resources, "A_Double");
    let an_int16 = resource(&resources, "An_Int16");
    let an_int32 = resource(&resources, "An_Int32");
    let padding = resource(&resources, "PreferredPadding");

    // Value types shouldn't get source info
    let foobar_string_source_info = XamlSourceInfo::get_xaml_source_info(&foobar_string);
    assert!(foobar_string_source_info.is_none());

    let a_double_source_info = XamlSourceInfo::get_xaml_source_info(&a_double);
    assert!(a_double_source_info.is_none());

    let an_int16_source_info = XamlSourceInfo::get_xaml_source_info(&an_int16);
    assert!(an_int16_source_info.is_none());

    let an_int32_source_info = XamlSourceInfo::get_xaml_source_info(&an_int32);
    assert!(an_int32_source_info.is_none());

    let padding_source_info = XamlSourceInfo::get_xaml_source_info(&padding);
    assert!(padding_source_info.is_none());
}

#[test]
fn resource_dictionary_set_resource_source_info() {
    let _base = xaml_test_base();
    let xaml = RuntimeXamlLoaderDocument::new(
        "
<UserControl xmlns='https://github.com/ferroui'
     xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
\t<UserControl.Resources>
\t\t<x:String x:Key='text'>foobar</x:String>
\t\t<x:Double x:Key=\"A_Double\">123.3</x:Double>
\t\t<x:Int16 x:Key=\"An_Int16\">123</x:Int16>
\t\t<x:Int32 x:Key=\"An_Int32\">37434323</x:Int32>
\t\t<Thickness x:Key=\"PreferredPadding\">10,20,10,0</Thickness>
        <x:Uri x:Key='homepage'>http://ferroui.net</x:Uri>
        <SolidColorBrush x:Key='MyBrush' Color='Red'/>
\t</UserControl.Resources>
</UserControl>",
    );

    let user_control = load_user_control(xaml);
    let resources: Rc<dyn IResourceDictionary> = user_control.resources().into();

    let foobar_string_source_info = XamlSourceInfo::get_xaml_source_info_for_key(&resources, "text");
    assert!(foobar_string_source_info.is_some());

    let a_double_source_info = XamlSourceInfo::get_xaml_source_info_for_key(&resources, "A_Double");
    assert!(a_double_source_info.is_some());

    let an_int16_source_info = XamlSourceInfo::get_xaml_source_info_for_key(&resources, "An_Int16");
    assert!(an_int16_source_info.is_some());

    let an_int32_source_info = XamlSourceInfo::get_xaml_source_info_for_key(&resources, "An_Int32");
    assert!(an_int32_source_info.is_some());

    let padding_source_info = XamlSourceInfo::get_xaml_source_info_for_key(&resources, "PreferredPadding");
    assert!(padding_source_info.is_some());

    let homepage_source_info = XamlSourceInfo::get_xaml_source_info_for_key(&resources, "homepage");
    assert!(homepage_source_info.is_some());

    let my_brush_source_info = XamlSourceInfo::get_xaml_source_info_for_key(&resources, "MyBrush");
    assert!(my_brush_source_info.is_some());
}

#[test]
fn resource_dictionary_set_resource_source_info_with_nested_dictionaries() {
    let _base = xaml_test_base();
    let xaml = RuntimeXamlLoaderDocument::new(
        "
<UserControl xmlns='https://github.com/ferroui'
     xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
\t<UserControl.Resources>
        <ResourceDictionary>
            <x:String x:Key='text'>foobar</x:String>
            <x:Double x:Key=\"A_Double\">123.3</x:Double>
            <x:Int16 x:Key=\"An_Int16\">123</x:Int16>
            <x:Int32 x:Key=\"An_Int32\">37434323</x:Int32>
            <ResourceDictionary.MergedDictionaries>
                <ResourceDictionary>
                    <Thickness x:Key=\"PreferredPadding\">10,20,10,0</Thickness>
                    <x:Uri x:Key='homepage'>http://ferroui.net</x:Uri>
                </ResourceDictionary>
            </ResourceDictionary.MergedDictionaries>
            <ResourceDictionary.ThemeDictionaries>
                <ResourceDictionary x:Key='Light'>
                    <SolidColorBrush x:Key='MyBrush' Color='Red'/>
                </ResourceDictionary>
            </ResourceDictionary.ThemeDictionaries>
        </ResourceDictionary>
\t</UserControl.Resources>
</UserControl>",
    );

    let user_control = load_user_control(xaml);
    let resources: Rc<dyn IResourceDictionary> = user_control.resources().into();
    let inner_resources = as_resource_dictionary(&*user_control.resources().merged_dictionaries().get(0));
    let theme_resources =
        as_resource_dictionary(&*user_control.resources().theme_dictionaries().get(&ThemeVariant::light()));

    // Outer define source info
    let foobar_string_source_info = XamlSourceInfo::get_xaml_source_info_for_key(&resources, "text");
    assert!(foobar_string_source_info.is_some());

    let a_double_source_info = XamlSourceInfo::get_xaml_source_info_for_key(&resources, "A_Double");
    assert!(a_double_source_info.is_some());

    let an_int16_source_info = XamlSourceInfo::get_xaml_source_info_for_key(&resources, "An_Int16");
    assert!(an_int16_source_info.is_some());

    let an_int32_source_info = XamlSourceInfo::get_xaml_source_info_for_key(&resources, "An_Int32");
    assert!(an_int32_source_info.is_some());

    // Outer one should not have source info for inner resources
    let padding_source_info = XamlSourceInfo::get_xaml_source_info_for_key(&resources, "PreferredPadding");
    assert!(padding_source_info.is_none());

    let homepage_source_info = XamlSourceInfo::get_xaml_source_info_for_key(&resources, "homepage");
    assert!(homepage_source_info.is_none());

    let my_brush_source_info = XamlSourceInfo::get_xaml_source_info_for_key(&resources, "MyBrush");
    assert!(my_brush_source_info.is_none());

    // Inner defined source info
    let homepage_source_info = XamlSourceInfo::get_xaml_source_info_for_key(&inner_resources, "homepage");
    assert!(homepage_source_info.is_some());

    let my_brush_source_info = XamlSourceInfo::get_xaml_source_info_for_key(&theme_resources, "MyBrush");
    assert!(my_brush_source_info.is_some());

    // Non-value types should have source info themselves
    let homepage = XamlSourceInfo::get_xaml_source_info(&resource(&inner_resources, "homepage"));
    assert!(homepage.is_some());

    let my_brush = XamlSourceInfo::get_xaml_source_info(&resource(&theme_resources, "MyBrush"));
    assert!(my_brush.is_some());
}

#[test]
fn gestures_get_xaml_source_info_set() {
    let _base = xaml_test_base();
    let xaml = RuntimeXamlLoaderDocument::new(
        "
<UserControl xmlns='https://github.com/ferroui'
     xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
\t<UserControl.GestureRecognizers>
\t\t<ScrollGestureRecognizer CanHorizontallyScroll=\"True\"
\t\t\t\t\t\t\t\t CanVerticallyScroll=\"True\"/>
\t\t<PullGestureRecognizer PullDirection=\"TopToBottom\"/>
\t</UserControl.GestureRecognizers>
</UserControl>",
    );

    let user_control = load_user_control(xaml);
    let recognizers = user_control.gesture_recognizers().to_vec();
    let scroll = recognizers
        .first()
        .and_then(|recognizer| recognizer.cast::<ScrollGestureRecognizer>())
        .expect("the first recognizer is a ScrollGestureRecognizer");
    let pull = recognizers
        .last()
        .and_then(|recognizer| recognizer.cast::<PullGestureRecognizer>())
        .expect("the last recognizer is a PullGestureRecognizer");

    let scroll_source_info = source_info_of(&scroll);
    assert!(scroll_source_info.is_some());

    let pull_source_info = source_info_of(&pull);
    assert!(pull_source_info.is_some());
}

#[test]
fn transitions_get_xaml_source_info_set() {
    let _base = xaml_test_base();
    let xaml = RuntimeXamlLoaderDocument::new(
        "
<UserControl xmlns='https://github.com/ferroui'
     xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
\t<UserControl.Transitions>
\t\t<Transitions>
\t\t\t<DoubleTransition Property=\"Width\" Duration=\"0:0:1.5\"/>
\t\t\t<DoubleTransition Property=\"Height\" Duration=\"0:0:1.5\"/>
\t\t</Transitions>
\t</UserControl.Transitions>
</UserControl>",
    );

    let user_control = load_user_control(xaml);
    let transitions = user_control.transitions().expect("the transitions are set");
    let width = transitions
        .get(0)
        .as_object()
        .and_then(|object| object.cast::<DoubleTransition>())
        .expect("the first transition is a DoubleTransition");
    let height = transitions
        .get(transitions.count() - 1)
        .as_object()
        .and_then(|object| object.cast::<DoubleTransition>())
        .expect("the last transition is a DoubleTransition");

    let width_source_info = source_info_of(&width);
    assert!(width_source_info.is_some());

    let height_source_info = source_info_of(&height);
    assert!(height_source_info.is_some());
}
