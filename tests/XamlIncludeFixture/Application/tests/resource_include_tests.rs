//! Port of `MarkupExtensions/ResourceIncludeTests.cs`, against compiled
//! documents: the document a test loads is a document of this crate
//! ([`crate::compiled_xaml`]) whose `ResourceInclude` names a document of the
//! crate `xaml-include-fixture-theme` (the assembly `Tests`), compiled into a
//! call of that document's build function. A document upstream populates into
//! an existing root instance (`new RuntimeXamlLoaderDocument(app, ..)`) is
//! built by its build function.

use std::rc::Rc;

use ferroui_base::controls::{ResourceDictionary, ResourceKey};
use ferroui_base::media::fonts::testing::TestAssetLoader;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Border, UserControl};

use super::support::{assert_is_type, build, describe, try_build, xaml_test_base};
use crate::{compiled_xaml, compiled_xaml_source_info};

fn start_with_resources() -> UnitTestApplicationScope {
    crate::register_types();
    let asset_loader = Rc::new(TestAssetLoader::new());
    let services = TestServices::new().with_asset_loader(asset_loader);
    UnitTestApplication::start(services)
}

#[test]
fn resource_include_loads_resource_dictionary_false() {
    resource_include_loads_resource_dictionary(false);
}

#[test]
fn resource_include_loads_resource_dictionary_true() {
    resource_include_loads_resource_dictionary(true);
}

fn resource_include_loads_resource_dictionary(create_source_info: bool) {
    let _base = xaml_test_base();
    let _app = start_with_resources();
    let user_control = match create_source_info {
        false => build(compiled_xaml::build_resourceinclude_loads_resourcedictionary_xaml),
        true => build(compiled_xaml_source_info::build_resourceinclude_loads_resourcedictionary_xaml),
    };
    assert_is_type::<UserControl>(&user_control);
    let border = user_control.get_control::<Border>("border");

    let background = border.background().expect("the background is null");
    let brush = background.as_solid_color_brush().expect("the brush is not an ISolidColorBrush");
    assert_eq!(0xff506070, brush.color().to_uint32(), "createSourceInfo: {create_source_info}");
}

#[test]
fn missing_resource_key_in_resource_include_does_not_cause_stack_overflow() {
    let _base = xaml_test_base();
    let _app = start_with_resources();
    // Only the failure of the lookup of the missing key is expected (the key-not-found
    // error of the static resource); anything else fails.
    if let Err(error) = try_build(compiled_xaml::build_missing_resourcekey_in_resourceinclude_does_not_cause_stackoverflow_xaml) {
        let description = describe(&error);
        assert!(description.contains("Static resource 'missing' not found."), "the document failed to load: {description}");
    }
}

#[test]
fn resource_include_should_be_allowed_to_have_key_in_custom_container() {
    let _base = xaml_test_base();
    let _app = start_with_resources();
    let res = build(compiled_xaml::build_resourceinclude_should_be_allowed_to_have_key_in_custom_container_xaml);
    assert_is_type::<ResourceDictionary>(&res);

    let val = res.try_get_resource(&ResourceKey::from("OkButton"), None);
    let val = val.expect("the resource is found");
    assert_eq!(val.as_ref().and_then(|value| value.downcast_ref::<String>()).map(String::as_str), Some("OK"));
}
