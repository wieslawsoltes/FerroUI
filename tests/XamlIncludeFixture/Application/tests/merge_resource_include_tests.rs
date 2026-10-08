//! Port of `Xaml/MergeResourceIncludeTests.cs`, against compiled documents.
//!
//! `MergeResourceInclude` merges documents of one compilation only (upstream's
//! `XamlMergeResourceGroupTransformer` finds the included document among the
//! documents being compiled), so the documents of these tests are documents of
//! the crate `xaml-include-fixture-theme` (the assembly `Tests` of their URIs),
//! compiled as one group there; a test builds the compiled document upstream
//! loads. A test that expects the load to fail at compile time compiles its
//! documents in the test. Not from upstream:
//! `merge_resource_include_of_another_crate_is_not_resolved`.

use std::rc::Rc;

use ferroui_base::controls::{ResourceDictionary, ResourceValue};
use ferroui_base::media::{Color, Colors, IBrush};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::ThemeVariant;
use ferroui_controls::UserControl;
use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use ferroui_markup_xaml_loader::rust_emitter::compile_documents;
use xaml_include_fixture_theme::{compiled_xaml, compiled_xaml_source_info};

use super::compiled_xaml_tests::{dependencies, root_uri};
use super::support::{assert_is_type, build, describe, try_build, xaml_test_base};

/// `((ISolidColorBrush)value).Color`.
#[track_caller]
fn solid_color(value: &ResourceValue) -> Color {
    let brush = from_markup_value::<Rc<dyn IBrush>>(value).expect("the resource is a brush");
    brush.as_solid_color_brush().expect("the resource is a solid color brush").color()
}

#[test]
fn merge_resource_include_works_with_single_resource_false() {
    merge_resource_include_works_with_single_resource(false);
}

#[test]
fn merge_resource_include_works_with_single_resource_true() {
    merge_resource_include_works_with_single_resource(true);
}

fn merge_resource_include_works_with_single_resource(create_source_info: bool) {
    let _base = xaml_test_base();
    let content_control = match create_source_info {
        false => build(compiled_xaml::build_mergeresourceinclude_works_with_single_resource_xaml),
        true => build(compiled_xaml_source_info::build_mergeresourceinclude_works_with_single_resource_xaml),
    };
    assert_is_type::<UserControl>(&content_control);

    let resources = content_control.resources();
    assert!(resources.get_type() == ResourceDictionary::TYPE);
    assert!(resources.merged_dictionaries().is_empty());

    let initial_resource = solid_color(&resources.get(&"brush1".into()));
    assert_eq!(Colors::BLUE, initial_resource);

    let merged_resource = solid_color(&resources.get(&"brush2".into()));
    assert_eq!(Colors::RED, merged_resource);
}

#[test]
fn mixing_merge_resource_include_and_resource_include_is_not_allowed() {
    let _base = xaml_test_base();
    let documents = [
        (
            "Resources1.xaml",
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <SolidColorBrush x:Key='brush1'>Red</SolidColorBrush>
</ResourceDictionary>",
        ),
        (
            "Resources2.xaml",
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <SolidColorBrush x:Key='brush2'>Blue</SolidColorBrush>
</ResourceDictionary>",
        ),
        (
            "Mixing_MergeResourceInclude_And_ResourceInclude_Is_Not_Allowed.xaml",
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ResourceDictionary.MergedDictionaries>
        <MergeResourceInclude Source='ferres://Tests/Resources1.xaml'/>
        <ResourceInclude Source='ferres://Tests/Resources2.xaml'/>
    </ResourceDictionary.MergedDictionaries>
</ResourceDictionary>",
        ),
    ];

    // `Assert.ThrowsAny<XmlException>(() => LoadGroup(documents))`: the group does not compile.
    let compiled = compile_documents(&documents, Some("ferres://Tests/"), &RuntimeXamlLoaderConfiguration::new(), &[]);
    let reason = compiled[2].source.clone().expect_err("the group does not compile");
    assert!(
        reason.contains(
            "MergeResourceInclude should always be included last when mixing with other dictionaries inside of the ResourceDictionary.MergedDictionaries. Line 6, position 10."
        ),
        "{reason}"
    );
}

#[test]
fn merge_resource_include_is_allowed_after_resource_include() {
    let _base = xaml_test_base();
    build(compiled_xaml::build_mergeresourceinclude_is_allowed_after_resourceinclude_xaml);
}

#[test]
fn merge_resource_include_works_with_multiple_resources() {
    let _base = xaml_test_base();
    let resources = build(compiled_xaml::build_mergeresourceinclude_works_with_multiple_resources_xaml);
    assert_is_type::<ResourceDictionary>(&resources);
    assert!(resources.merged_dictionaries().is_empty());

    assert_eq!(Colors::RED, solid_color(&resources.get(&"brush1".into())));
    assert_eq!(Colors::BLUE, solid_color(&resources.get(&"brush2".into())));
    assert_eq!(Colors::GREEN, solid_color(&resources.get(&"brush3".into())));
    assert_eq!(Colors::YELLOW, solid_color(&resources.get(&"brush4".into())));
    assert_eq!(Colors::BLACK, solid_color(&resources.get(&"brush5".into())));
    assert_eq!(Colors::WHITE, solid_color(&resources.get(&"brush6".into())));
}

#[test]
fn merge_resource_include_works_with_theme_dictionaries() {
    let _base = xaml_test_base();
    let resources = build(compiled_xaml::build_mergeresourceinclude_works_with_themedictionaries_xaml);
    assert_is_type::<ResourceDictionary>(&resources);
    assert!(resources.merged_dictionaries().is_empty());

    let get = |key: &str, theme_variant: ThemeVariant| -> Color {
        match resources.try_get_resource(&key.into(), Some(&theme_variant)) {
            Some(res) => solid_color(&res),
            None => panic!("The given key '{key}' was not present in the dictionary."),
        }
    };

    assert_eq!(Colors::WHITE, get("brush1", ThemeVariant::light()));
    assert_eq!(Colors::BLACK, get("brush2", ThemeVariant::light()));
    assert_eq!(Colors::BLACK, get("brush1", ThemeVariant::dark()));
    assert_eq!(Colors::WHITE, get("brush2", ThemeVariant::dark()));

    assert_eq!(Colors::RED, get("brush3", ThemeVariant::light()));
    assert_eq!(Colors::BLUE, get("brush4", ThemeVariant::light()));
    assert_eq!(Colors::BLUE, get("brush3", ThemeVariant::dark()));
    assert_eq!(Colors::RED, get("brush4", ThemeVariant::dark()));
}

#[test]
fn merge_resource_include_fails_with_theme_dictionaries_duplicate_resources() {
    let _base = xaml_test_base();
    let error = match try_build(compiled_xaml::build_mergeresourceinclude_fails_with_themedictionaries_duplicate_resources_xaml) {
        Ok(_) => panic!("Expected an argument exception, but the document was loaded"),
        Err(error) => error,
    };
    let description = describe(&error);
    assert!(
        description.contains("An item with the same key has already been added."),
        "Expected an argument exception: {description}"
    );
}

/// Not from upstream: a `MergeResourceInclude` of a document of another crate is
/// upstream's error, as merging works within one compilation.
#[test]
fn merge_resource_include_of_another_crate_is_not_resolved() {
    let _base = xaml_test_base();
    let documents = [(
        "MergeAcrossCrates.xaml",
        "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ResourceDictionary.MergedDictionaries>
        <MergeResourceInclude Source='ferres://Tests/Resources.xaml'/>
    </ResourceDictionary.MergedDictionaries>
    <SolidColorBrush x:Key='brush1'>Blue</SolidColorBrush>
</ResourceDictionary>",
    )];
    let compiled = compile_documents(&documents, Some(&root_uri()), &RuntimeXamlLoaderConfiguration::new(), &dependencies());
    let reason = compiled[0].source.clone().expect_err("the document does not compile");
    assert!(reason.contains("Node MergeResourceInclude is unable to resolve \"ferres://tests/Resources.xaml\" path."), "{reason}");
}
