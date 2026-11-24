use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition};
use ferroui_base::Ref;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Control, StackPanel, Window};
use ferroui_markup_property_alias_library::AliasedProperties;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

fn fade(root: &Ref<StackPanel>, name: &str) -> f64 {
    AliasedProperties::get_fade(&root.find_control::<Control>(name).expect(name))
}

fn assert_page(root: Ref<StackPanel>) {
    assert_eq!(fade(&root, "Attribute"), 0.5, "an attribute");
    assert_eq!(fade(&root, "Element"), 0.75, "a property element");
    assert_eq!(fade(&root, "OtherClass"), 0.125, "another class derived from the class of the alias");

    // The real property wins over the alias of the same name.
    let real = root.find_control::<Control>("Real").unwrap();
    assert_eq!(real.width(), 42.0);
    assert_eq!(AliasedProperties::get_fade(&real), 0.0);

    let window = Window::new();
    window.set_content(Some(Control::boxed(&root)));
    window.show();
    assert_eq!(fade(&root, "StyledByName"), 0.25, "a setter by the name");
    assert_eq!(fade(&root, "StyledByOwner"), 0.375, "a setter by the owner and the name");
    window.close();
}

fn start() -> impl Drop {
    let application = UnitTestApplication::start(TestServices::styled_window());
    ferroui_markup_xaml::register_types();
    ferroui_markup_property_alias_library::register_types();
    application
}

#[test]
fn the_compiled_loader_resolves_an_alias() {
    let _app = start();
    let value = crate::compiled_xaml::try_load(None, "ferres://ferroui_markup_property_alias_tests/Aliases.xaml")
        .unwrap()
        .expect("the compiled page");
    assert_page(value.downcast_ref::<Ref<StackPanel>>().expect("a stack panel").clone());
}

#[test]
fn the_run_time_loader_resolves_an_alias() {
    let _app = start();
    let root = FerroRuntimeXamlLoader::parse_as::<Ref<Control>>(include_str!("Aliases.xaml"), None)
        .unwrap_or_else(|error| panic!("{error}"));
    assert_page(root.cast::<StackPanel>().expect("a stack panel"));
}

#[test]
fn a_name_without_an_alias_fails_as_it_does_without_the_crate() {
    let _app = start();
    let page = r#"<Border xmlns="https://github.com/ferroui" NoSuchProperty="1" />"#;
    assert!(FerroRuntimeXamlLoader::parse_as::<Ref<Control>>(page, None).is_err());
}

static SECOND: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Markup.PropertyAlias.Second",
    crate_name: "ferroui_markup_property_alias_second",
    xmlns_definitions: &[XmlnsDefinition { xml_namespace: "urn:property-alias-second", namespace: "FerroUI.Markup.PropertyAlias.Second" }],
    xmlns_prefixes: &[],
    metadata: &[("FerroXamlPropertyAlias", "FerroUI.Controls.Control.Twice=FerroUI.Markup.PropertyAlias.AliasedProperties.Fade")],
};

#[test]
fn an_alias_two_crates_declare_is_an_error() {
    let _app = start();
    MarkupAssembly::register_table(&SECOND);
    let page = r#"<Border xmlns="https://github.com/ferroui" Twice="1" />"#;
    let error = match FerroRuntimeXamlLoader::parse_as::<Ref<Control>>(page, None) {
        Ok(_) => panic!("a duplicate alias must not load"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("declared more than once"), "{error}");
}
