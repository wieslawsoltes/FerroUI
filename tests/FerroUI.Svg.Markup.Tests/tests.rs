use ferroui_base::metadata::FERRO_XML_NAMESPACE;
use ferroui_base::platform::register_assets;
use ferroui_base::{Ref, Size};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Control, Image, StackPanel};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

const SQUARE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 12"><rect width="24" height="12" fill="#f00"/></svg>"##;

fn start() -> impl Drop {
    let application = UnitTestApplication::start(TestServices::styled_window());
    register_assets("SvgMarkupTests", &[("/square.svg", SQUARE), ("/broken.svg", b"<svg")]);
    ferroui_svg::register_types();
    application
}

fn assert_images(root: &Ref<StackPanel>) {
    for name in ["Short", "Named"] {
        let image = root.find_control::<Control>(name).and_then(|control| control.cast::<Image>()).expect(name);
        assert_eq!(image.source().expect(name).size(), Size::new(24.0, 12.0), "{name}");
    }
}

#[test]
fn the_crate_maps_its_namespace_to_the_default_xml_namespace() {
    let definitions = ferroui_svg::ASSEMBLY.xmlns_definitions;
    assert!(definitions.iter().all(|definition| definition.xml_namespace == FERRO_XML_NAMESPACE));
    assert!(definitions.iter().any(|definition| definition.namespace == "FerroUI.Svg"));
}

#[test]
fn the_compiled_loader_resolves_the_extension_without_a_prefix() {
    let _app = start();
    let value = crate::compiled_xaml::try_load(None, "ferres://ferroui_svg_markup_tests/DefaultNamespace.xaml")
        .unwrap()
        .expect("the compiled page");
    assert_images(value.downcast_ref::<Ref<StackPanel>>().expect("a stack panel"));
}

#[test]
fn the_run_time_loader_resolves_the_extension_without_a_prefix() {
    let _app = start();
    let root = FerroRuntimeXamlLoader::parse_as::<Ref<Control>>(include_str!("DefaultNamespace.xaml"), None)
        .unwrap_or_else(|error| panic!("{error}"));
    assert_images(&root.cast::<StackPanel>().expect("a stack panel"));
}

#[test]
fn the_gallery_pages_load_their_svg_from_the_default_namespace() {
    let _app = start();
    register_assets(
        "WinUIGallery",
        &[("Assets/SampleMedia/MirrorPCConsent.svg", include_bytes!("../../samples/WinUIGallery/Assets/SampleMedia/MirrorPCConsent.svg"))],
    );
    for (page, name) in [
        (include_str!("../../samples/WinUIGallery/Pages/Image.xaml"), "SvgImage"),
        (include_str!("../../samples/WinUIGallery/Pages/IconElement.xaml"), "ImageExample2Icon"),
    ] {
        let root = FerroRuntimeXamlLoader::parse_as::<Ref<Control>>(page, None).unwrap_or_else(|error| panic!("{error}"));
        let image = root.find_control::<Control>(name).and_then(|control| control.cast::<Image>()).expect(name);
        assert_eq!(image.source().expect(name).size(), Size::new(328.0, 164.0), "{name}");
    }
}

#[test]
fn a_document_cannot_make_the_application_fetch_or_panic() {
    let _app = start();
    for source in ["https://example.invalid/a.svg", "file:///etc/passwd", "ferres://SvgMarkupTests/broken.svg", "ferres://SvgMarkupTests/missing.svg"] {
        let page = format!(r#"<Image xmlns="https://github.com/ferroui" Source="{{SvgImage {source}}}" />"#);
        assert!(FerroRuntimeXamlLoader::parse_as::<Ref<Control>>(&page, None).is_err(), "{source}");
    }
}
