//! Port of `Xaml/StyleIncludeTests.cs`.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::metadata::IServiceProvider;
use ferroui_base::platform::{AssetAssembly, IAssetLoader, StandardAssetLoader};
use ferroui_base::styling::{styles_as_style, IStyle, Style, Styles};
use ferroui_base::{FerroLocator, Ref};
use ferroui_controls::ContentControl;
use ferroui_markup_xaml::styling::StyleInclude;
use ferroui_markup_xaml::xaml_il::runtime::IFerroXamlIlParentStackProvider;
use ferroui_markup_xaml::{
    RuntimeXamlDiagnostic, RuntimeXamlDiagnosticSeverity, RuntimeXamlLoaderConfiguration, ServiceProviderExtensions,
};

use crate::support::app::{styled_window_application, unit_test_application, xaml_test_base, TestServices};
use crate::support::helpers::{assert_is_type, assert_value_is_type, boxed, value_of};
use crate::support::loader::{
    cast, describe, document, document_without_uri, load_document, load_group, parse, test_assembly, try_load_group,
    uri,
};
use crate::support::xaml::style_include_tests::{StyleWithServiceProvider, TestServiceProvider};

#[test]
fn style_include_is_built() {
    let _app = unit_test_application(TestServices::styled_window().with_theme(|| styles_as_style(&Styles::new())));
    let xaml = "
<ContentControl xmlns='https://github.com/ferroui'>
    <ContentControl.Styles>
        <StyleInclude Source='ferres://FerroUI.Markup.Xaml.UnitTests/Xaml/Style1.xaml'/>
    </ContentControl.Styles>
</ContentControl>";

    let window = parse::<Ref<ContentControl>>(xaml);

    let style = window.styles().get(0);
    assert_is_type::<Style>(style.as_object().expect("the style is an object of the class model"));
}

#[test]
fn style_include_is_built_resources() {
    let _app = unit_test_application(TestServices::styled_window().with_theme(|| styles_as_style(&Styles::new())));
    let xaml = "
<ContentControl xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ContentControl.Resources>
        <StyleInclude x:Key='Include' Source='ferres://FerroUI.Markup.Xaml.UnitTests/Xaml/Style1.xaml'/>
    </ContentControl.Resources>
</ContentControl>";

    let content_control = parse::<Ref<ContentControl>>(xaml);

    assert_value_is_type::<Style>(&content_control.resources().get(&"Include".into()));
}

#[test]
fn style_include_is_resolved_with_two_files() {
    let _base = xaml_test_base();
    let documents = vec![
        document(
            "ferres://Tests/Style.xaml",
            "
<Style xmlns='https://github.com/ferroui'
       xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Style.Resources>
        <Color x:Key='Red'>Red</Color>
    </Style.Resources>
</Style>",
        ),
        document_without_uri(
            "
<ContentControl xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ContentControl.Resources>
        <StyleInclude x:Key='Include' Source='ferres://Tests/Style.xaml'/>
    </ContentControl.Resources>
</ContentControl>",
        ),
    ];

    let objects = load_group(documents, None);
    let _style = assert_value_is_type::<Style>(&objects[0]);
    let content_control = assert_value_is_type::<ContentControl>(&objects[1]);

    assert_value_is_type::<Style>(&content_control.resources().get(&"Include".into()));
}

#[test]
fn relative_back_style_include_is_resolved_with_two_files() {
    let _base = xaml_test_base();
    let documents = vec![
        document(
            "ferres://Tests/Subfolder/Style.xaml",
            "
<Style xmlns='https://github.com/ferroui'
       xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Style.Resources>
        <Color x:Key='Red'>Red</Color>
    </Style.Resources>
</Style>",
        ),
        document(
            "ferres://Tests/Subfolder/Folder/Root.xaml",
            "
<ContentControl xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ContentControl.Resources>
        <StyleInclude x:Key='Include' Source='../Style.xaml'/>
    </ContentControl.Resources>
</ContentControl>",
        ),
    ];

    let objects = load_group(documents, None);
    let _style = assert_value_is_type::<Style>(&objects[0]);
    let content_control = assert_value_is_type::<ContentControl>(&objects[1]);

    assert_value_is_type::<Style>(&content_control.resources().get(&"Include".into()));
}

#[test]
fn relative_root_style_include_is_resolved_with_two_files() {
    let _base = xaml_test_base();
    let documents = vec![
        document(
            "ferres://Tests/Style.xaml",
            "
<Style xmlns='https://github.com/ferroui'
       xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Style.Resources>
        <Color x:Key='Red'>Red</Color>
    </Style.Resources>
</Style>",
        ),
        document(
            "ferres://Tests/Folder/Root.xaml",
            "
<ContentControl xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ContentControl.Resources>
        <StyleInclude x:Key='Include' Source='/Style.xaml'/>
    </ContentControl.Resources>
</ContentControl>",
        ),
    ];

    let objects = load_group(documents, None);
    let _style = assert_value_is_type::<Style>(&objects[0]);
    let content_control = assert_value_is_type::<ContentControl>(&objects[1]);

    assert_value_is_type::<Style>(&content_control.resources().get(&"Include".into()));
}

#[test]
fn relative_style_include_is_resolved_with_two_files() {
    let _base = xaml_test_base();
    let documents = vec![
        document(
            "ferres://Tests/Folder/Style.xaml",
            "
<Style xmlns='https://github.com/ferroui'
       xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Style.Resources>
        <Color x:Key='Red'>Red</Color>
    </Style.Resources>
</Style>",
        ),
        document(
            "ferres://Tests/Folder/Root.xaml",
            "
<ContentControl xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ContentControl.Resources>
        <StyleInclude x:Key='Include' Source='Style.xaml'/>
    </ContentControl.Resources>
</ContentControl>",
        ),
    ];

    let objects = load_group(documents, None);
    let _style = assert_value_is_type::<Style>(&objects[0]);
    let content_control = assert_value_is_type::<ContentControl>(&objects[1]);

    assert_value_is_type::<Style>(&content_control.resources().get(&"Include".into()));
}

#[test]
fn relative_dot_syntax_style_include_is_resolved_with_two_files() {
    let _base = xaml_test_base();
    let documents = vec![
        document(
            "ferres://Tests/Folder/Style.xaml",
            "
<Style xmlns='https://github.com/ferroui'
       xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Style.Resources>
        <Color x:Key='Red'>Red</Color>
    </Style.Resources>
</Style>",
        ),
        document(
            "ferres://Tests/Folder/Root.xaml",
            "
<ContentControl xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ContentControl.Resources>
        <StyleInclude x:Key='Include' Source='./Style.xaml'/>
    </ContentControl.Resources>
</ContentControl>",
        ),
    ];

    let objects = load_group(documents, None);
    let _style = assert_value_is_type::<Style>(&objects[0]);
    let content_control = assert_value_is_type::<ContentControl>(&objects[1]);

    assert_value_is_type::<Style>(&content_control.resources().get(&"Include".into()));
}

#[test]
fn non_latin_style_include_is_resolved_with_two_files() {
    let _base = xaml_test_base();
    let documents = vec![
        document(
            "ferres://アセンブリ/スタイル.xaml",
            "
<Style xmlns='https://github.com/ferroui'
       xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Style.Resources>
        <Color x:Key='Red'>Red</Color>
    </Style.Resources>
</Style>",
        ),
        document_without_uri(
            "
<ContentControl xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ContentControl.Resources>
        <StyleInclude x:Key='Include' Source='ferres://アセンブリ/スタイル.xaml'/>
    </ContentControl.Resources>
</ContentControl>",
        ),
    ];

    let objects = load_group(documents, None);
    let _style = assert_value_is_type::<Style>(&objects[0]);
    let content_control = assert_value_is_type::<ContentControl>(&objects[1]);

    assert_value_is_type::<Style>(&content_control.resources().get(&"Include".into()));
}

#[test]
fn missing_resource_key_in_style_include_does_not_cause_stack_overflow() {
    let _base = xaml_test_base();
    let documents = vec![
        document(
            "ferres://Tests/Style.xaml",
            "
<Style xmlns='https://github.com/ferroui'
       xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Style.Resources>
        <StaticResource x:Key='brush' ResourceKey='missing' />
    </Style.Resources>
</Style>",
        ),
        document_without_uri(
            "
<ContentControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ContentControl.Styles>
        <StyleInclude Source='ferres://Tests/Style.xaml'/>
    </ContentControl.Styles>
</ContentControl>",
        ),
    ];

    // The only failure allowed is the one of the missing key (the "key not
    // found" exception of a static resource).
    if let Err(error) = try_load_group(documents, None) {
        let description = describe(&error);
        assert!(
            description.contains("Static resource 'missing' not found."),
            "the group failed to load: {description}"
        );
    }
}

#[test]
#[ignore = "themes: needs the Simple theme (upstream runs this test under it)"]
fn style_inside_resources_should_produce_warning() {
    let _app = styled_window_application();

    let diagnostics: Rc<RefCell<Vec<RuntimeXamlDiagnostic>>> = Rc::new(RefCell::new(Vec::new()));
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.diagnostic_handler = Some(Rc::new({
        let diagnostics = diagnostics.clone();
        move |diagnostic: &RuntimeXamlDiagnostic| {
            diagnostics.borrow_mut().push(diagnostic.clone());
            diagnostic.severity
        }
    }));
    let control: Ref<ContentControl> = cast(&load_document(
        document_without_uri(
            "
<ContentControl xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
                xmlns:themes='clr-namespace:FerroUI.Themes.Simple;assembly=FerroUI.Themes.Simple'>
    <ContentControl.Resources>
        <ResourceDictionary>
            <ResourceDictionary.MergedDictionaries>
                <themes:SimpleTheme />
            </ResourceDictionary.MergedDictionaries>
        </ResourceDictionary>
    </ContentControl.Resources>
</ContentControl>",
        ),
        Some(configuration),
    ));
    let merged = control.resources().merged_dictionaries().get(0);
    let merged_object = merged.as_object().expect("the merged dictionary is an object of the class model").to_ref();
    assert!(value_of::<Rc<dyn IStyle>>(&Some(boxed(merged_object))).is_some());
    let diagnostics = diagnostics.borrow();
    assert_eq!(1, diagnostics.len());
    let warning = &diagnostics[0];
    assert_eq!(RuntimeXamlDiagnosticSeverity::Warning, warning.severity);
}

#[test]
fn style_include_from_code_behind_resolves_compiled() {
    let _base = xaml_test_base();
    let locator_scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable()
        .bind::<dyn IAssetLoader>()
        .to_constant(Rc::new(StandardAssetLoader::new(Some(&AssetAssembly::new(test_assembly().name)))));

    let sp = TestServiceProvider::new();
    let service_provider: Rc<dyn IServiceProvider> = sp.clone();
    let style_include = StyleInclude::with_service_provider(service_provider.clone());
    style_include.set_source(Some(uri("ferres://FerroUI.Markup.Xaml.UnitTests/Xaml/StyleWithServiceProvider.xaml")));

    let loaded = style_include.loaded();
    let loaded = loaded.as_object().expect("the loaded style is an object of the class model");
    assert_is_type::<StyleWithServiceProvider>(loaded);
    let loaded = loaded.to_ref().cast::<StyleWithServiceProvider>().expect("a StyleWithServiceProvider");
    let loaded_service_provider = loaded.service_provider().expect("the style has a service provider");

    assert!(
        service_provider.get_required_service::<Rc<dyn IFerroXamlIlParentStackProvider>>().parents()
            == loaded_service_provider.get_required_service::<Rc<dyn IFerroXamlIlParentStackProvider>>().parents()
    );

    locator_scope.dispose();
}

// NOT PORTED (the Rust port lacks a type or member the test body needs):
// - style_include_should_be_replaced_with_direct_call -> themes: `SimpleTheme` (the simple theme is not ported)
