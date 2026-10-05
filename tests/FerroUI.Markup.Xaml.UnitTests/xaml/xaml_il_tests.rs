//! Port of `Xaml/XamlIlTests.cs`.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::media::{Brushes, Colors, IBrush, SolidColorBrush};
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, FerroProperty, Ref, Visual};
use ferroui_controls::{
    Application, Button, Canvas, ContentControl, Control, Grid, ItemsControl, ListBox, ListBoxItem, StackPanel, TabControl, TabItem,
    TextBlock, TextBox, UserControl, Window,
};
use ferroui_markup_xaml::{RuntimeXamlDiagnostic, RuntimeXamlDiagnosticSeverity, RuntimeXamlLoaderConfiguration};

use crate::support::app::{styled_window_application, xaml_test_base};
use crate::support::helpers::{assert_is_type, assert_string, boxed, object_of};
use crate::support::loader::{
    cast, describe, document_without_uri, load_as, load_document, load_with_root, local_configuration, parse,
    parse_local, try_load_document, xaml_error, XamlResult,
};
use crate::support::xaml::style_include_tests::TestServiceProvider;
use crate::support::xaml_il_tests::{
    XamlIlBugTestsEventHandlerCodeBehind, XamlIlBugTestsStaticClassWithAttachedProperty,
    XamlIlClassWithClrPropertyWithValue, XamlIlClassWithCustomProperty,
    XamlIlClassWithPrecompiledXaml, XamlIlClassWithTypeConverterOnFerroProperty,
};

/// The diagnostics a load reported, and the handler that collects them
/// without changing their severity.
fn collect_diagnostics(
    configuration: &mut RuntimeXamlLoaderConfiguration,
) -> Rc<RefCell<Vec<RuntimeXamlDiagnostic>>> {
    let diagnostics = Rc::new(RefCell::new(Vec::new()));
    let sink = diagnostics.clone();
    configuration.diagnostic_handler = Some(Rc::new(move |diagnostic: &RuntimeXamlDiagnostic| {
        sink.borrow_mut().push(diagnostic.clone());
        diagnostic.severity
    }));
    diagnostics
}

/// `Assert.Throws<InvalidOperationException>(..)` for the duplicate setter
/// check of a style instance: the load fails with the error of the check
/// (and not with an error of the compiler).
#[track_caller]
fn assert_throws_duplicate_setter_invalid_operation(result: XamlResult<BoxedValue>) {
    let error = match result {
        Ok(_) => panic!("Expected an invalid operation (duplicate setter), but the document loaded"),
        Err(error) => error,
    };
    assert!(xaml_error(&error).is_none(), "Expected an invalid operation, found: {}", describe(&error));
    let description = describe(&error);
    assert!(
        description.contains("Duplicate setter encountered for property"),
        "Expected an invalid operation (duplicate setter), found: {description}"
    );
}

#[test]
fn transitions_should_be_properly_parsed() {
    let _base = xaml_test_base();
    let parsed = parse::<Ref<Grid>>(
        "
<Grid xmlns='https://github.com/ferroui' >
  <Grid.Transitions>
    <Transitions>
      <DoubleTransition Property='Opacity'
        Easing='CircularEaseIn'
        Duration='0:0:0.5' />
    </Transitions>
  </Grid.Transitions>
</Grid>",
    );
    let transitions = parsed.transitions().expect("the transitions are set");
    assert_eq!(1, transitions.count());
    let expected: &'static FerroProperty = Visual::opacity_property();
    assert_eq!(expected, transitions.get(0).property());
}

#[test]
fn parser_should_override_precompiled_xaml() {
    let _base = xaml_test_base();
    let precompiled = XamlIlClassWithPrecompiledXaml::new();
    let red: Rc<dyn IBrush> = Brushes::red();
    assert!(Some(red) == precompiled.background());
    assert_eq!(1.0, precompiled.opacity());
    let loaded = parse::<Ref<XamlIlClassWithPrecompiledXaml>>(
        "
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
             x:Class='FerroUI.Markup.Xaml.UnitTests.XamlIlClassWithPrecompiledXaml'
             Opacity='0'>
    
</UserControl>",
    );
    assert_eq!(loaded.opacity(), 0.0);
    assert!(loaded.background().is_none());
}

#[test]
fn relative_source_templated_parent_works() {
    let _app = styled_window_application();
    load_with_root(
        "
<Application
  xmlns='https://github.com/ferroui'
  xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests;assembly=FerroUI.Markup.Xaml.UnitTests'
>
<Application.Styles>
    <Style Selector='Button'>
      <Setter Property='Template'>
        <ControlTemplate>
          <Grid><Grid><Grid>
            <Canvas>
              <Canvas.Background>
                <SolidColorBrush>
                  <SolidColorBrush.Color>
                    <MultiBinding>
                      <MultiBinding.Converter>
                          <local:XamlIlBugTestsBrushToColorConverter/>
                      </MultiBinding.Converter>
                      <Binding Path='Background' RelativeSource='{RelativeSource TemplatedParent}'/>
                      <Binding Path='Background' RelativeSource='{RelativeSource TemplatedParent}'/>
                      <Binding Path='Background' RelativeSource='{RelativeSource TemplatedParent}'/>
                    </MultiBinding>
                  </SolidColorBrush.Color>
                </SolidColorBrush>
              </Canvas.Background>
            </Canvas>
          </Grid></Grid></Grid>
        </ControlTemplate>
      </Setter>
    </Style>
  </Application.Styles>
</Application>",
        None,
        boxed(Application::current().expect("the application is running")),
    );
    let parsed = parse::<Ref<Window>>(
        "
<Window
  xmlns='https://github.com/ferroui'
  xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests;assembly=FerroUI.Markup.Xaml.UnitTests'
>
  
  <Button Background='Red' />

</Window>
",
    );
    let btn = object_of::<Button>(&parsed.content());
    btn.apply_template();
    let canvas = btn
        .get_visual_children()
        .first()
        .cloned()
        .expect("the button has a visual child")
        .visual_children()
        .get(0)
        .visual_children()
        .get(0)
        .visual_children()
        .get(0)
        .cast::<Canvas>()
        .expect("a Canvas");
    let background = canvas.background().expect("the canvas has a background");
    assert_eq!(
        Brushes::red().color(),
        background.as_solid_color_brush().expect("the background is a solid color brush").color()
    );
}

#[test]
fn custom_properties_should_work_with_x_class() {
    let _base = xaml_test_base();
    let precompiled = XamlIlClassWithCustomProperty::new();
    assert_eq!(Some("123".to_string()), precompiled.test());
    let loaded = parse::<Ref<XamlIlClassWithCustomProperty>>(
        "
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
             x:Class='FerroUI.Markup.Xaml.UnitTests.XamlIlClassWithCustomProperty'
             Test='321'>

</UserControl>",
    );
    assert_eq!(Some("321".to_string()), loaded.test());
}

#[test]
fn attached_properties_from_static_types_should_work_in_style_setters_bug_2561() {
    let _app = styled_window_application();

    let parsed = parse::<Ref<Window>>(
        "
<Window
  xmlns='https://github.com/ferroui'
  xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests;assembly=FerroUI.Markup.Xaml.UnitTests'
>
  <Window.Styles>
    <Style Selector='TextBox'>
      <Setter Property='local:XamlIlBugTestsStaticClassWithAttachedProperty.TestInt' Value='100'/>
    </Style>
  </Window.Styles>
  <TextBox/>

</Window>
",
    );
    let tb = object_of::<TextBox>(&parsed.content());
    parsed.show();
    tb.apply_template();
    assert_eq!(100, XamlIlBugTestsStaticClassWithAttachedProperty::get_test_int(&tb));
}

#[test]
fn provide_value_target_should_provide_clr_property_info() {
    let _base = xaml_test_base();
    let parsed = parse_local::<Rc<XamlIlClassWithClrPropertyWithValue>>(
        "
<XamlIlClassWithClrPropertyWithValue 
    xmlns='clr-namespace:FerroUI.Markup.Xaml.UnitTests'
    Count='{XamlIlCheckClrPropertyInfo ExpectedPropertyName=Count}'
/>",
    );
    assert_eq!(6, parsed.count());
}

#[test]
fn data_context_type_resolution() {
    let _app = styled_window_application();
    let _parsed = parse::<Ref<UserControl>>(
        "
<UserControl 
    xmlns='https://github.com/ferroui'
    xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests;assembly=FerroUI.Markup.Xaml.UnitTests'
    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' x:DataType='local:XamlIlBugTestsDataContext' />",
    );
}

#[test]
#[ignore = "themes: needs the Simple theme (upstream runs this test under it)"]
fn event_handlers_should_work_for_templates() {
    let _app = styled_window_application();
    let w = XamlIlBugTestsEventHandlerCodeBehind::new();
    w.apply_template();
    w.show();

    Dispatcher::ui_thread().run_jobs(None);

    let items_control = object_of::<ItemsControl>(&w.content());
    let items_presenter = items_control.get_visual_children().first().cloned();
    assert!(items_presenter.is_some());
    let items_presenter = items_presenter.unwrap();

    let first = |visual: &Ref<Visual>| -> Ref<Visual> {
        visual.get_visual_children().first().cloned().expect("Sequence contains no elements")
    };
    let item = first(&first(&first(&items_presenter)));

    let item = item.cast::<Control>().expect("the item is a Control");
    item.set_data_context(Some(boxed("test".to_string())));
    assert_string("test", &w.saved_context());
}

#[test]
fn data_templates_should_resolve_named_controls_from_parent_scope() {
    let _app = styled_window_application();
    let parsed = parse::<Ref<Window>>(
        "
<Window
  xmlns='https://github.com/ferroui'
  xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
>
  <StackPanel>
    <StackPanel.DataTemplates>
      <DataTemplate DataType='{x:Type x:String}'>
       <TextBlock Classes='target' Text='{Binding #txt.Text}'/>
      </DataTemplate>
    </StackPanel.DataTemplates>
    <TextBlock Text='Test' Name='txt'/>
    <ContentControl Content='tst'/>
  </StackPanel>
</Window>
",
    );
    parsed.set_data_context(Some(boxed(vec!["Test".to_string()])));
    parsed.show();
    parsed.apply_template();
    let panel = object_of::<StackPanel>(&parsed.content());
    let cc = panel
        .children()
        .snapshot()
        .last()
        .cloned()
        .expect("the panel has children")
        .cast::<ContentControl>()
        .expect("a ContentControl");
    cc.apply_template();
    let templated = cc
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<TextBlock>())
        .find(|x| x.classes().contains("target"))
        .expect("a TextBlock with the class 'target'");
    assert_eq!(Some("Test".to_string()), templated.text());
}

#[test]
fn should_work_with_base_property() {
    let _base = xaml_test_base();
    let parsed = load_as::<Ref<ListBox>>(
        "
<ListBox
  xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
  xmlns='https://github.com/ferroui'
  xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests;assembly=FerroUI.Markup.Xaml.UnitTests'
>
    <ItemsControl.ItemTemplate>
      <DataTemplate>
        <ContentControl Content='{Binding}' />
      </DataTemplate>
    </ItemsControl.ItemTemplate>
</ListBox>",
    );

    assert!(parsed.item_template().is_some());
}

#[test]
fn runtime_loader_should_pass_parents_from_service_provider() {
    let _base = xaml_test_base();
    let sp = TestServiceProvider::new();
    let user_control = UserControl::new();
    user_control.resources().set("Resource1", Some(boxed(SolidColorBrush::with_color(Colors::BLUE))));
    sp.set_parents_stack(Rc::new(vec![Control::boxed(user_control)]));
    let mut document = document_without_uri(
        "
<Button xmlns='https://github.com/ferroui' Background='{StaticResource Resource1}' />",
    );
    let service_provider: Rc<dyn IServiceProvider> = sp;
    document.service_provider = Some(service_provider);

    let parsed = cast::<Ref<Button>>(&load_document(document, None));
    let background = parsed.background().expect("the button has a background");
    assert_eq!(
        Colors::BLUE,
        background.as_solid_color_brush().expect("the background is a solid color brush").color()
    );
}

#[test]
fn style_parser_throws_for_duplicate_setter() {
    let _app = styled_window_application();

    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Window.Styles>
        <Style Selector='TextBlock'>
            <Setter Property='Width' Value='100'/>
            <Setter Property='Height' Value='20'/>
            <Setter Property='Height' Value='30'/>
        </Style>
    </Window.Styles>
    <TextBlock/>
</Window>";
    let mut configuration = local_configuration();
    let diagnostics = collect_diagnostics(&mut configuration);
    // We still have a runtime check in the StyleInstance class, but in this test we only care about compile warnings.
    assert_throws_duplicate_setter_invalid_operation(try_load_document(document_without_uri(xaml), Some(configuration)));
    let diagnostics = diagnostics.borrow();
    assert_eq!(1, diagnostics.len());
    let warning = &diagnostics[0];
    assert_eq!(RuntimeXamlDiagnosticSeverity::Warning, warning.severity);
    assert!(warning.title.starts_with("Duplicate setter encountered for property 'Height'"));
}

#[test]
fn control_theme_parser_throws_for_duplicate_setter() {
    let _app = styled_window_application();

    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:u='using:FerroUI.Markup.Xaml.UnitTests.Xaml'>
    <Window.Resources>
        <ControlTheme x:Key='MyTheme' TargetType='u:TestTemplatedControl'>
            <Setter Property='Width' Value='100'/>
            <Setter Property='Height' Value='20'/>
            <Setter Property='Height' Value='30'/>
        </ControlTheme>
    </Window.Resources>

    <u:TestTemplatedControl Theme='{StaticResource MyTheme}'/>
</Window>";
    let mut configuration = local_configuration();
    let diagnostics = collect_diagnostics(&mut configuration);
    // We still have a runtime check in the StyleInstance class, but in this test we only care about compile warnings.
    assert_throws_duplicate_setter_invalid_operation(try_load_document(document_without_uri(xaml), Some(configuration)));
    let diagnostics = diagnostics.borrow();
    assert_eq!(1, diagnostics.len());
    let warning = &diagnostics[0];
    assert_eq!(RuntimeXamlDiagnosticSeverity::Warning, warning.severity);
    assert!(warning.title.starts_with("Duplicate setter encountered for property 'Height'"));
}

#[test]
fn item_container_inside_of_item_template_should_be_warned() {
    let _app = styled_window_application();

    let xaml = document_without_uri(
        "
<ListBox xmlns='https://github.com/ferroui'
         xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ListBox.ItemTemplate>
        <DataTemplate>
            <ListBoxItem />
        </DataTemplate>
    </ListBox.ItemTemplate>
</ListBox>",
    );
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    let diagnostics = collect_diagnostics(&mut configuration);
    // We still have a runtime check in the StyleInstance class, but in this test we only care about compile warnings.
    let list_box = cast::<Ref<ListBox>>(&load_document(xaml, Some(configuration)));
    // ItemTemplate should still work as before, creating whatever object user put inside
    let built = list_box.item_template().expect("the item template is set").build(&None);
    assert_is_type::<ListBoxItem>(&built.expect("the template builds a control"));

    // But invalid usage should be warned:
    let diagnostics = diagnostics.borrow();
    assert_eq!(1, diagnostics.len());
    let warning = &diagnostics[0];
    assert_eq!(RuntimeXamlDiagnosticSeverity::Warning, warning.severity);
    assert_eq!("FRN2208", warning.id);
}

#[test]
fn item_container_inside_of_data_templates_should_be_warned() {
    let _app = styled_window_application();

    let xaml = document_without_uri(
        "
<TabControl xmlns='https://github.com/ferroui'
         xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <TabControl.DataTemplates>
        <DataTemplate x:DataType='x:Object'>
            <TabItem />
        </DataTemplate>
    </TabControl.DataTemplates>
</TabControl>",
    );
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    let diagnostics = collect_diagnostics(&mut configuration);
    // We still have a runtime check in the StyleInstance class, but in this test we only care about compile warnings.
    let tab_control = cast::<Ref<TabControl>>(&load_document(xaml, Some(configuration)));
    // ItemTemplate should still work as before, creating whatever object user put inside
    let built = tab_control.data_templates().get(0).build(&None);
    assert_is_type::<TabItem>(&built.expect("the template builds a control"));

    // But invalid usage should be warned:
    let diagnostics = diagnostics.borrow();
    assert_eq!(1, diagnostics.len());
    let warning = &diagnostics[0];
    assert_eq!(RuntimeXamlDiagnosticSeverity::Warning, warning.severity);
    assert_eq!("FRN2208", warning.id);
}

#[test]
fn type_converters_should_work_when_specified_with_attributes_on_ferro_properties() {
    let _app = styled_window_application();

    let parsed = parse_local::<Ref<XamlIlClassWithTypeConverterOnFerroProperty>>(
        "
<XamlIlClassWithTypeConverterOnFerroProperty
    xmlns='clr-namespace:FerroUI.Markup.Xaml.UnitTests;assembly=FerroUI.Markup.Xaml.UnitTests' 
    MyProp='a,b,c'/>",
    );

    let values: Vec<String> =
        parsed.my_prop().expect("the property is set").iter().map(|x| x.value()).collect();
    assert_eq!(vec!["a".to_string(), "b".to_string(), "c".to_string()], values);
}

#[test]
fn compiled_binding_should_resolve_named_root_data_context_in_item_template() {
    let _app = styled_window_application();
    let parsed = parse::<Ref<ListBox>>(
        "
<ListBox Name='ListBoxRoot' ItemsSource='{CompiledBinding Items}'
    xmlns='https://github.com/ferroui'
    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
    xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests;assembly=FerroUI.Markup.Xaml.UnitTests'
    x:DataType='local:CompiledBindingRootMock'>
    <ListBox.ItemTemplate>
        <DataTemplate x:DataType='local:CompiledBindingItemMock'>
            <TextBlock Text='{CompiledBinding #ListBoxRoot.DataContext.RootProperty}' />
        </DataTemplate>
    </ListBox.ItemTemplate>
</ListBox>",
    );
    assert!(parsed.item_template().is_some());
}

#[test]
fn compiled_binding_should_resolve_root_command_from_nested_item_template_namescope() {
    let _app = styled_window_application();
    let parsed = parse::<Ref<ListBox>>(
        "
<ListBox Name='ListBoxRoot' ItemsSource='{CompiledBinding Items}'
    xmlns='https://github.com/ferroui'
    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
    xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests;assembly=FerroUI.Markup.Xaml.UnitTests'
    x:DataType='local:CompiledBindingRootMock'>
    <ListBox.ItemTemplate>
        <DataTemplate x:DataType='local:CompiledBindingItemMock'>
            <ListBox ItemsSource='{CompiledBinding InnerItems}'>
                <ListBox.ItemTemplate>
                    <DataTemplate x:DataType='local:CompiledBindingItemMock'>
                        <TextBlock Text='{CompiledBinding #ListBoxRoot.DataContext.RootProperty}' />
                    </DataTemplate>
                </ListBox.ItemTemplate>
            </ListBox>
        </DataTemplate>
    </ListBox.ItemTemplate>
</ListBox>",
    );
    assert!(parsed.item_template().is_some());
}
