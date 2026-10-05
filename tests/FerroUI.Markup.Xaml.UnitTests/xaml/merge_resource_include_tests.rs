//! Port of `Xaml/MergeResourceIncludeTests.cs`.

use std::rc::Rc;

use ferroui_base::controls::{ResourceDictionary, ResourceValue};
use ferroui_base::media::{Color, Colors, IBrush};
use ferroui_base::styling::ThemeVariant;
use ferroui_controls::UserControl;
use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;

use crate::support::app::xaml_test_base;
use crate::support::helpers::{assert_throws_xaml_diagnostic, assert_value_is_type, value_of};
use crate::support::loader::{describe, document, document_without_uri, load_group, try_load_group};

/// `((ISolidColorBrush)value).Color`.
#[track_caller]
fn solid_color(value: &ResourceValue) -> Color {
    let brush = value_of::<Rc<dyn IBrush>>(value).expect("the resource is a brush");
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
    let documents = vec![
        document(
            "ferres://Tests/Resources.xaml",
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <SolidColorBrush x:Key='brush2'>Red</SolidColorBrush>
</ResourceDictionary>",
        ),
        document_without_uri(
            "
<UserControl xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Resources>
        <ResourceDictionary>
            <SolidColorBrush x:Key='brush1'>Blue</SolidColorBrush>
            <ResourceDictionary.MergedDictionaries>
                <MergeResourceInclude Source='ferres://Tests/Resources.xaml'/>
            </ResourceDictionary.MergedDictionaries>
        </ResourceDictionary>
    </UserControl.Resources>
</UserControl>",
        ),
    ];

    let mut config = RuntimeXamlLoaderConfiguration::new();
    config.set_create_source_info(create_source_info);
    let objects = load_group(documents, Some(config));
    let content_control = assert_value_is_type::<UserControl>(&objects[1]);

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
    let documents = vec![
        document(
            "ferres://Tests/Resources1.xaml",
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <SolidColorBrush x:Key='brush1'>Red</SolidColorBrush>
</ResourceDictionary>",
        ),
        document(
            "ferres://Tests/Resources2.xaml",
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <SolidColorBrush x:Key='brush2'>Blue</SolidColorBrush>
</ResourceDictionary>",
        ),
        document_without_uri(
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

    assert_throws_xaml_diagnostic(
        try_load_group(documents, None),
        "FRN2000",
        "MergeResourceInclude should always be included last when mixing with other dictionaries inside of the ResourceDictionary.MergedDictionaries. Line 6, position 10.",
    );
}

#[test]
fn merge_resource_include_is_allowed_after_resource_include() {
    let _base = xaml_test_base();
    let documents = vec![
        document(
            "ferres://Tests/Resources1.xaml",
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <SolidColorBrush x:Key='brush1'>Red</SolidColorBrush>
</ResourceDictionary>",
        ),
        document(
            "ferres://Tests/Resources2.xaml",
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <SolidColorBrush x:Key='brush2'>Blue</SolidColorBrush>
</ResourceDictionary>",
        ),
        document_without_uri(
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ResourceDictionary.MergedDictionaries>
        <ResourceInclude Source='ferres://Tests/Resources2.xaml'/>
        <MergeResourceInclude Source='ferres://Tests/Resources1.xaml'/>
    </ResourceDictionary.MergedDictionaries>
</ResourceDictionary>",
        ),
    ];

    load_group(documents, None);
}

#[test]
fn merge_resource_include_works_with_multiple_resources() {
    let _base = xaml_test_base();
    let documents = vec![
        document(
            "ferres://Tests/Resources1.xaml",
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <SolidColorBrush x:Key='brush1'>Red</SolidColorBrush>
    <SolidColorBrush x:Key='brush2'>Blue</SolidColorBrush>
</ResourceDictionary>",
        ),
        document(
            "ferres://Tests/Resources2.xaml",
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <SolidColorBrush x:Key='brush4'>Yellow</SolidColorBrush>
    <ResourceDictionary.MergedDictionaries>
        <MergeResourceInclude Source='ferres://Tests/Resources1_2.xaml'/>
    </ResourceDictionary.MergedDictionaries>
</ResourceDictionary>",
        ),
        document_without_uri(
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ResourceDictionary.MergedDictionaries>
        <MergeResourceInclude Source='ferres://Tests/Resources1.xaml'/>
        <MergeResourceInclude Source='ferres://Tests/Resources2.xaml'/>
    </ResourceDictionary.MergedDictionaries>
    <SolidColorBrush x:Key='brush5'>Black</SolidColorBrush>
    <SolidColorBrush x:Key='brush6'>White</SolidColorBrush>
</ResourceDictionary>",
        ),
        document(
            "ferres://Tests/Resources1_2.xaml",
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <SolidColorBrush x:Key='brush3'>Green</SolidColorBrush>
</ResourceDictionary>",
        ),
    ];

    let objects = load_group(documents, None);
    let resources = assert_value_is_type::<ResourceDictionary>(&objects[2]);
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
    let documents = vec![
        document(
            "ferres://Tests/Resources1.xaml",
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ResourceDictionary.ThemeDictionaries>
        <ResourceDictionary x:Key='Light'>
            <SolidColorBrush x:Key='brush1'>White</SolidColorBrush>
            <SolidColorBrush x:Key='brush2'>Black</SolidColorBrush>
        </ResourceDictionary>
        <ResourceDictionary x:Key='Dark'>
            <SolidColorBrush x:Key='brush1'>Black</SolidColorBrush>
            <SolidColorBrush x:Key='brush2'>White</SolidColorBrush>
        </ResourceDictionary>
    </ResourceDictionary.ThemeDictionaries>
</ResourceDictionary>",
        ),
        document(
            "ferres://Tests/Resources2.xaml",
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ResourceDictionary.ThemeDictionaries>
        <ResourceDictionary x:Key='Light'>
            <SolidColorBrush x:Key='brush3'>Red</SolidColorBrush>
            <SolidColorBrush x:Key='brush4'>Blue</SolidColorBrush>
        </ResourceDictionary>
        <ResourceDictionary x:Key='Dark'>
            <SolidColorBrush x:Key='brush3'>Blue</SolidColorBrush>
            <SolidColorBrush x:Key='brush4'>Red</SolidColorBrush>
        </ResourceDictionary>
    </ResourceDictionary.ThemeDictionaries>
</ResourceDictionary>",
        ),
        document_without_uri(
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ResourceDictionary.MergedDictionaries>
        <MergeResourceInclude Source='ferres://Tests/Resources1.xaml'/>
        <MergeResourceInclude Source='ferres://Tests/Resources2.xaml'/>
    </ResourceDictionary.MergedDictionaries>
</ResourceDictionary>",
        ),
    ];

    let objects = load_group(documents, None);
    let resources = assert_value_is_type::<ResourceDictionary>(&objects[2]);
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
    let documents = vec![
        document(
            "ferres://Tests/Resources1.xaml",
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ResourceDictionary.ThemeDictionaries>
        <ResourceDictionary x:Key='Light'>
            <SolidColorBrush x:Key='brush1'>White</SolidColorBrush>
        </ResourceDictionary>
    </ResourceDictionary.ThemeDictionaries>
</ResourceDictionary>",
        ),
        document(
            "ferres://Tests/Resources2.xaml",
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ResourceDictionary.ThemeDictionaries>
        <ResourceDictionary x:Key='Light'>
            <SolidColorBrush x:Key='brush1'>Black</SolidColorBrush>
        </ResourceDictionary>
    </ResourceDictionary.ThemeDictionaries>
</ResourceDictionary>",
        ),
        document_without_uri(
            "
<ResourceDictionary xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ResourceDictionary.MergedDictionaries>
        <MergeResourceInclude Source='ferres://Tests/Resources1.xaml'/>
        <MergeResourceInclude Source='ferres://Tests/Resources2.xaml'/>
    </ResourceDictionary.MergedDictionaries>
</ResourceDictionary>",
        ),
    ];

    let error = match try_load_group(documents, None) {
        Ok(_) => panic!("Expected an argument exception, but the group was loaded"),
        Err(error) => error,
    };
    let description = describe(&error);
    assert!(
        description.contains("An item with the same key has already been added."),
        "Expected an argument exception: {description}"
    );
}
