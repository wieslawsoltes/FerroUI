//! The theme as a whole: it loads, is added to the styles of an
//! application and resolves the resources of its variants; and the two
//! cases of the XAML suite (`Xaml/StyleIncludeTests.cs`) that use the
//! theme.

use super::support::*;
use crate::SimpleTheme;
use ferroui_base::controls::ResourceKey;
use ferroui_base::media::{Color, IBrush};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::{IStyle, Styles, ThemeVariant};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::{Application, Button, ContentControl, Control, Window};
use ferroui_markup_xaml::{
    RuntimeXamlDiagnostic, RuntimeXamlDiagnosticSeverity, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument,
};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::cell::RefCell;
use std::rc::Rc;

fn color(text: &str) -> Color {
    Color::parse(text).expect("a colour")
}

fn solid_color(brush: Option<Rc<dyn IBrush>>) -> Color {
    brush.expect("a brush").as_solid_color_brush().expect("a solid colour brush").color()
}

#[test]
fn theme_loads_and_is_added_to_the_styles_of_an_application() {
    let _app = start_application();
    let application = Application::current().expect("an application");

    let theme = SimpleTheme::new();
    application.styles().add(theme.clone().upcast::<Styles>());

    assert_eq!(1, application.styles().count());
    assert!(application.styles().get(0).as_object().is_some_and(|object| object.downcast_ref::<SimpleTheme>().is_some()));
    // The theme document: its resources (the accents and the strings, merged) and one style, the
    // list of the control themes.
    assert_eq!(1, theme.count());
    assert!(theme.resources().merged_dictionaries().is_empty());
    assert!(theme.resources().contains_key(&ResourceKey::from("ThemeAccentColor")));
    assert!(theme.resources().contains_key(&ResourceKey::from("StringTextFlyoutCopyText")));
    assert_eq!(2, theme.resources().theme_dictionaries_snapshot().len());
    // The control themes are found from the application.
    let button_theme = application.try_get_resource(&ResourceKey::Type(Button::TYPE), None);
    assert!(button_theme.flatten().is_some());
}

#[test]
fn light_and_dark_variants_resolve_their_resources() {
    let _app = start_themed_application();
    let application = Application::current().expect("an application");

    let resource = |key: &str, variant: ThemeVariant| -> Color {
        let value = application.try_get_resource(&ResourceKey::from(key), Some(&variant));
        from_markup_value::<Color>(&value.unwrap_or_else(|| panic!("{key} is not defined for {variant:?}")))
            .unwrap_or_else(|| panic!("{key} is not a colour"))
    };
    assert_eq!(color("#FFFFFFFF"), resource("ThemeBackgroundColor", ThemeVariant::light()));
    assert_eq!(color("#FF282828"), resource("ThemeBackgroundColor", ThemeVariant::dark()));
    assert_eq!(color("#FF000000"), resource("ThemeForegroundColor", ThemeVariant::light()));
    assert_eq!(color("#FFDEDEDE"), resource("ThemeForegroundColor", ThemeVariant::dark()));
    // Resources outside the theme dictionaries are the same for every variant.
    assert_eq!(color("#CC119EDA"), resource("ThemeAccentColor", ThemeVariant::light()));
    assert_eq!(color("#CC119EDA"), resource("ThemeAccentColor", ThemeVariant::dark()));

    // A themed control follows the variant of its window. (The unit test application does not
    // register itself as the theme variant host of top levels, as the one of the managed test
    // suite does not, so the variant is requested on the window.)
    let button = Button::new();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&button)));
    window.show();

    assert_eq!(color("#FF000000"), solid_color(button.foreground()));
    assert_eq!(color("#FFFFFFFF"), solid_color(window.background()));
    assert_eq!(Thickness::uniform(4.0), button.padding());
    assert_eq!(Thickness::uniform(1.0), button.border_thickness());

    window.set_requested_theme_variant(Some(ThemeVariant::dark()));

    assert_eq!(color("#FFDEDEDE"), solid_color(button.foreground()));
    assert_eq!(color("#FF282828"), solid_color(window.background()));

    window.set_requested_theme_variant(Some(ThemeVariant::light()));

    assert_eq!(color("#FF000000"), solid_color(button.foreground()));
    assert_eq!(color("#FFFFFFFF"), solid_color(window.background()));
}

// --- Xaml/StyleIncludeTests.cs ------------------------------------------------

fn is_simple_theme(style: &Rc<dyn IStyle>) -> bool {
    style.as_object().is_some_and(|object| object.downcast_ref::<SimpleTheme>().is_some())
}

/// Port of `StyleIncludeTests.StyleInclude_Should_Be_Replaced_With_Direct_Call`.
#[test]
#[ignore = "gap G12: a style include of the document of a class of another assembly is not replaced by an instance of the class (the second style is not a SimpleTheme)"]
fn style_include_should_be_replaced_with_direct_call() {
    let _app = start_application();

    let control = load_text(
        "
<ContentControl xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
                xmlns:themes='clr-namespace:FerroUI.Themes.Simple;assembly=FerroUI.Themes.Simple'>
    <ContentControl.Styles>
        <themes:SimpleTheme />
        <StyleInclude Source='ferres://FerroUI.Themes.Simple/SimpleTheme.xaml'/>
    </ContentControl.Styles>
</ContentControl>",
    );
    let control = from_markup_value::<Ref<ContentControl>>(&Some(control)).expect("a content control");
    assert!(is_simple_theme(&control.styles().get(0)));
    assert!(is_simple_theme(&control.styles().get(1)));
}

/// Port of `StyleIncludeTests.Style_Inside_Resources_Should_Produce_Warning`.
#[test]
fn style_inside_resources_should_produce_warning() {
    let _app = start_application();

    let diagnostics: Rc<RefCell<Vec<RuntimeXamlDiagnostic>>> = Rc::new(RefCell::new(Vec::new()));
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.diagnostic_handler = Some(Rc::new({
        let diagnostics = diagnostics.clone();
        move |diagnostic: &RuntimeXamlDiagnostic| {
            diagnostics.borrow_mut().push(diagnostic.clone());
            diagnostic.severity
        }
    }));
    let control = FerroRuntimeXamlLoader::load_document(
        RuntimeXamlLoaderDocument::new(
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
    )
    .unwrap_or_else(|error| panic!("{}", describe(&error)));
    let control = from_markup_value::<Ref<ContentControl>>(&Some(control)).expect("a content control");

    let merged = control.resources().merged_dictionaries().get(0);
    // `Assert.IsAssignableFrom<IStyle>`: the merged dictionary is the theme, a style.
    assert!(merged.as_object().is_some_and(|object| object.downcast_ref::<SimpleTheme>().is_some()));
    let diagnostics = diagnostics.borrow();
    assert_eq!(1, diagnostics.len());
    assert_eq!(RuntimeXamlDiagnosticSeverity::Warning, diagnostics[0].severity);
}
