//! Port of `Xaml/ControlTemplateTests.cs`.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::data::BindingPriority;
use ferroui_base::diagnostics::FerroObjectDiagnosticExtensions;
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::Ref;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::{Button, ContentControl, Dock, DockPanel, Panel, Window};
use ferroui_markup_xaml::templates::ControlTemplate;
use ferroui_markup_xaml::{RuntimeXamlDiagnostic, RuntimeXamlDiagnosticSeverity, RuntimeXamlLoaderConfiguration};

use crate::support::app::{styled_window_application, xaml_test_base};
use crate::support::helpers::{
    assert_throws_xaml_diagnostic, assert_is_type, assert_string, assert_value_is_type, object_of, try_object_of,
};
use crate::support::loader::{
    document_without_uri, load_as, load_document, local_configuration, parse, try_load_document,
};
use crate::support::xaml::control_template_tests::ListBoxHierarchyLine;

/// The configuration of the diagnostic tests: the local assembly of the
/// tests and a handler that records each diagnostic and keeps its severity.
fn recording_configuration() -> (Rc<RefCell<Vec<RuntimeXamlDiagnostic>>>, RuntimeXamlLoaderConfiguration) {
    let diagnostics = Rc::new(RefCell::new(Vec::new()));
    let mut configuration = local_configuration();
    let recorded = diagnostics.clone();
    configuration.diagnostic_handler = Some(Rc::new(move |diagnostic: &RuntimeXamlDiagnostic| {
        recorded.borrow_mut().push(diagnostic.clone());
        diagnostic.severity
    }));
    (diagnostics, configuration)
}

#[test]
fn styled_properties_should_be_set_in_the_control_template() {
    let _app = styled_window_application();

    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:controls="using:FerroUI.Markup.Xaml.UnitTests.Xaml">
    <Button>
        <Button.Template>
            <ControlTemplate>
                <controls:ListBoxHierarchyLine>
                    <controls:ListBoxHierarchyLine.LineDashStyle>
                        <DashStyle Dashes="2,2" Offset="1" />
                    </controls:ListBoxHierarchyLine.LineDashStyle>
                </controls:ListBoxHierarchyLine>
            </ControlTemplate>
        </Button.Template>
    </Button>
</Window>"#;
    let window = load_as::<Ref<Window>>(xaml);
    let button = object_of::<Button>(&window.content());

    window.apply_template();
    button.apply_template();
    let list_box_hierarchy_line = button.get_visual_children()[0].cast::<ListBoxHierarchyLine>();
    let list_box_hierarchy_line = list_box_hierarchy_line.expect("the visual child is not a ListBoxHierarchyLine");
    let line_dash_style = list_box_hierarchy_line.line_dash_style().expect("the dash style is null");
    let dashes = line_dash_style.dashes().expect("the dashes are null");
    assert_eq!(1.0, line_dash_style.offset());
    assert_eq!(2, dashes.len());
    assert_eq!(2.0, dashes.get(0));
    assert_eq!(2.0, dashes.get(1));
}

#[test]
fn inline_control_template_styled_values_are_set_with_style_priority() {
    let _app = styled_window_application();

    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Button>
        <Button.Template>
            <ControlTemplate>
                <ContentPresenter Name='PART_ContentPresenter'
                                  Background='Red'/>
            </ControlTemplate>
        </Button.Template>
    </Button>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let button = object_of::<Button>(&window.content());

    window.apply_template();
    button.apply_template();

    let presenter = button.presenter().expect("the presenter is null");
    let red: Rc<dyn IBrush> = Brushes::red();
    assert_eq!(Some(red), presenter.background());

    let diagnostic = presenter.get_diagnostic(TemplatedControl::background_property().as_property());
    assert_eq!(BindingPriority::Template, diagnostic.priority());
}

#[test]
fn style_control_template_styled_values_are_set_with_style_priority() {
    let _app = styled_window_application();

    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Styles>
        <Style Selector='Button'>
            <Setter Property='Template'>
                <ControlTemplate>
                    <ContentPresenter Name='PART_ContentPresenter'
                                      Background='Red'/>
                </ControlTemplate>
            </Setter>
        </Style>
    </Window.Styles>
    <Button/>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let button = object_of::<Button>(&window.content());

    window.apply_template();
    button.apply_template();

    let presenter = button.presenter().expect("the presenter is null");
    let red: Rc<dyn IBrush> = Brushes::red();
    assert_eq!(Some(red), presenter.background());

    let diagnostic = presenter.get_diagnostic(TemplatedControl::background_property().as_property());
    assert_eq!(BindingPriority::Template, diagnostic.priority());
}

#[test]
fn control_template_attached_values_are_set_with_style_priority() {
    let _app = styled_window_application();

    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Button>
        <Button.Template>
            <ControlTemplate>
                <ContentPresenter Name='PART_ContentPresenter'
                                  DockPanel.Dock='Top'/>
            </ControlTemplate>
        </Button.Template>
    </Button>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let button = object_of::<Button>(&window.content());

    window.apply_template();
    button.apply_template();

    let presenter = button.presenter().expect("the presenter is null");
    assert_eq!(Dock::Top, DockPanel::get_dock(&presenter));

    let diagnostic = presenter.get_diagnostic(DockPanel::dock_property().as_property());
    assert_eq!(BindingPriority::Template, diagnostic.priority());
}

#[test]
fn control_template_static_resources_are_set_with_style_priority() {
    let _app = styled_window_application();

    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Resources>
        <SolidColorBrush x:Key='red'>Red</SolidColorBrush>
    </Window.Resources>
    <Button Content='Foo'>
        <Button.Template>
            <ControlTemplate>
                <ContentPresenter Name='PART_ContentPresenter'
                                  Background='{StaticResource red}'/>
            </ControlTemplate>
        </Button.Template>
    </Button>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let button = object_of::<Button>(&window.content());

    window.apply_template();
    button.apply_template();

    let presenter = button.presenter().expect("the presenter is null");
    let red: Rc<dyn IBrush> = Brushes::red();
    assert_eq!(Some(red), presenter.background());

    let diagnostic = presenter.get_diagnostic(TemplatedControl::background_property().as_property());
    assert_eq!(BindingPriority::Template, diagnostic.priority());
}

#[test]
fn control_template_dynamic_resources_are_set_with_style_priority() {
    let _app = styled_window_application();

    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Window.Resources>
        <SolidColorBrush x:Key='red'>Red</SolidColorBrush>
    </Window.Resources>
    <Button Content='Foo'>
        <Button.Template>
            <ControlTemplate>
                <ContentPresenter Name='PART_ContentPresenter'
                                  Background='{DynamicResource red}'/>
            </ControlTemplate>
        </Button.Template>
    </Button>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let button = object_of::<Button>(&window.content());

    window.apply_template();
    button.apply_template();

    let presenter = button.presenter().expect("the presenter is null");
    let red: Rc<dyn IBrush> = Brushes::red();
    assert_eq!(Some(red), presenter.background());

    let diagnostic = presenter.get_diagnostic(TemplatedControl::background_property().as_property());
    assert_eq!(BindingPriority::Template, diagnostic.priority());
}

#[test]
fn control_template_template_bindings_are_set_with_templated_parent_priority() {
    let _app = styled_window_application();

    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Button Content='Foo'>
        <Button.Template>
            <ControlTemplate>
                <ContentPresenter Name='PART_ContentPresenter'
                                  Content='{TemplateBinding Content}'/>
            </ControlTemplate>
        </Button.Template>
    </Button>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let button = object_of::<Button>(&window.content());

    window.apply_template();
    button.apply_template();

    let presenter = button.presenter().expect("the presenter is null");
    assert_string("Foo", &presenter.content());

    let diagnostic = presenter.get_diagnostic(ContentPresenter::content_property().as_property());
    assert_eq!(BindingPriority::Template, diagnostic.priority());
}

#[test]
fn control_template_with_nested_child_is_operational() {
    let _base = xaml_test_base();
    let xaml = "
<ControlTemplate xmlns='https://github.com/ferroui'>
    <ContentControl Name='parent'>
        <ContentControl Name='child' />
    </ContentControl>
</ControlTemplate>
";
    let template = parse::<Rc<ControlTemplate>>(xaml);

    let result = template.build(&ContentControl::new().upcast()).expect("the template built nothing");
    let parent = result.result().cast::<ContentControl>().expect("the result is not a ContentControl");

    assert_eq!(Some("parent".to_string()), parent.name());

    let child = try_object_of::<ContentControl>(&parent.content());

    let child = child.expect("the content is not a ContentControl");

    assert_eq!(Some("child".to_string()), child.name());
}

#[test]
fn control_template_with_target_type_is_operational() {
    let _base = xaml_test_base();
    let xaml = "
<ControlTemplate xmlns='https://github.com/ferroui' 
                 xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
                 TargetType='{x:Type ContentControl}'>
    <ContentPresenter x:Name='PART_ContentPresenter' Content='{TemplateBinding Content}' />
</ControlTemplate>
";
    let template = parse::<Rc<ControlTemplate>>(xaml);

    assert!(template.target_type().is_some_and(|target_type| std::ptr::eq(target_type, ContentControl::TYPE)));

    let result = template.build(&ContentControl::new().upcast()).expect("the template built nothing");
    assert_is_type::<ContentPresenter>(result.result());
}

#[test]
fn control_template_with_string_target_type() {
    let _base = xaml_test_base();
    let xaml = "
<ControlTemplate xmlns='https://github.com/ferroui' 
                 xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
                 TargetType='ContentControl'>
    <ContentPresenter x:Name='PART_ContentPresenter' Content='{TemplateBinding Content}' />
</ControlTemplate>
";
    let template = parse::<Rc<ControlTemplate>>(xaml);

    assert!(template.target_type().is_some_and(|target_type| std::ptr::eq(target_type, ContentControl::TYPE)));

    let result = template.build(&ContentControl::new().upcast()).expect("the template built nothing");
    assert_is_type::<ContentPresenter>(result.result());
}

#[test]
fn control_template_with_panel_children_are_added() {
    let _base = xaml_test_base();
    let xaml = "
<ControlTemplate xmlns='https://github.com/ferroui'>
    <Panel Name='panel'>
        <ContentControl Name='Foo' />
        <ContentControl Name='Bar' />
    </Panel>
</ControlTemplate>
";
    let template = parse::<Rc<ControlTemplate>>(xaml);

    let result = template.build(&ContentControl::new().upcast()).expect("the template built nothing");
    let panel = result.result().cast::<Panel>().expect("the result is not a Panel");

    assert_eq!(2, panel.children().count());

    let foo = panel.children().get(0);
    let bar = panel.children().get(1);

    assert_eq!(Some("Foo".to_string()), foo.name());
    assert_eq!(Some("Bar".to_string()), bar.name());
}

#[test]
fn control_template_can_be_empty() {
    let _base = xaml_test_base();
    let xaml = "<ControlTemplate xmlns='https://github.com/ferroui' />";
    let template = parse::<Rc<ControlTemplate>>(xaml);

    let template_result = template.build(&TemplatedControl::new());
    assert!(template_result.is_none());
}

#[test]
fn control_template_outputs_error_when_missing_template_part() {
    let _app = styled_window_application();

    let xaml = "
<ControlTemplate xmlns='https://github.com/ferroui'
                 xmlns:controls='using:FerroUI.Markup.Xaml.UnitTests.Xaml'
                 TargetType='controls:CustomButtonWithParts'>
    <Border Name='PART_Typo_MainContentBorder'>
        <ContentPresenter Name='PART_ContentPresenter'
                          Content='{TemplateBinding Content}'/>
    </Border>
</ControlTemplate>";
    let (diagnostics, configuration) = recording_configuration();
    load_document(document_without_uri(xaml), Some(configuration));
    let diagnostics = diagnostics.borrow();
    assert_eq!(1, diagnostics.len(), "the collection does not contain exactly one diagnostic: {diagnostics:?}");
    let warning = &diagnostics[0];
    assert_eq!(RuntimeXamlDiagnosticSeverity::Info, warning.severity);
    assert!(warning.title.contains("'PART_MainContentBorder'"), "unexpected title: {}", warning.title);
}

#[test]
fn control_template_outputs_error_when_using_wrong_type_with_template_part() {
    let _app = styled_window_application();

    let xaml = "
<ControlTemplate xmlns='https://github.com/ferroui'
                 xmlns:controls='using:FerroUI.Markup.Xaml.UnitTests.Xaml'
                 TargetType='controls:CustomControlWithParts'>
    <Border Name='PART_MainContentBorder'>
        <ContentControl Name='PART_ContentPresenter'
                        Content='{TemplateBinding Content}'/>
    </Border>
</ControlTemplate>";
    let (diagnostics, configuration) = recording_configuration();
    assert_throws_xaml_diagnostic(
        try_load_document(document_without_uri(xaml), Some(configuration)),
        "FRN1000",
        "Template part 'PART_ContentPresenter' is expected to be assignable to 'ContentPresenter', but actual type is ContentControl. Line 6, position 25.",
    );
    let diagnostics = diagnostics.borrow();
    assert_eq!(1, diagnostics.len(), "the collection does not contain exactly one diagnostic: {diagnostics:?}");
    let warning = &diagnostics[0];
    assert_eq!(RuntimeXamlDiagnosticSeverity::Error, warning.severity);
    assert!(warning.title.contains("'ContentPresenter'"), "unexpected title: {}", warning.title);
}

#[test]
fn control_template_outputs_error_when_missing_template_part_nested_item_template_case() {
    let _app = styled_window_application();

    let xaml = "
<ControlTemplate xmlns='https://github.com/ferroui'
                 xmlns:controls='using:FerroUI.Markup.Xaml.UnitTests.Xaml'
                 TargetType='controls:CustomControlWithParts'>
    <Border Name='PART_Typo_MainContentBorder'>
        <StackPanel>
            <ItemsControl>
                <ItemsControl.ItemTemplate>
                    <DataTemplate>
                        <!-- This PART_MainContentBorder shouldn't full parent ControlTemplate, PART_Typo_MainContentBorder still isn't properly named. -->
                        <Border Name='PART_MainContentBorder' />
                    </DataTemplate>
                </ItemsControl.ItemTemplate>
            </ItemsControl>
            <ContentPresenter Name='PART_ContentPresenter'
                              Content='{TemplateBinding Content}'/>
        </StackPanel>
    </Border>
</ControlTemplate>";
    let (diagnostics, configuration) = recording_configuration();
    load_document(document_without_uri(xaml), Some(configuration));
    let diagnostics = diagnostics.borrow();
    assert_eq!(1, diagnostics.len(), "the collection does not contain exactly one diagnostic: {diagnostics:?}");
    let warning = &diagnostics[0];
    assert_eq!(RuntimeXamlDiagnosticSeverity::Info, warning.severity);
    assert!(warning.title.contains("'PART_MainContentBorder'"), "unexpected title: {}", warning.title);
}

#[test]
fn custom_control_template_allows_template_bindings() {
    let _app = styled_window_application();
    let window = load_as::<Ref<Window>>(
        r#"<Window xmlns="https://github.com/ferroui"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
        xmlns:controls="using:FerroUI.Markup.Xaml.UnitTests.Xaml">
    <Button Content="Foo">
        <Button.Template>
            <controls:CustomControlTemplate>
                <ContentPresenter Name="PART_ContentPresenter"
                                  Content="{TemplateBinding Content}"/>
            </controls:CustomControlTemplate>
        </Button.Template>
    </Button>
</Window>"#,
    );
    let button = assert_value_is_type::<Button>(&window.content());

    window.apply_template();
    button.apply_template();

    let presenter = button.presenter();
    let presenter = presenter.expect("the presenter is null");
    assert_string("Foo", &presenter.content());
}
