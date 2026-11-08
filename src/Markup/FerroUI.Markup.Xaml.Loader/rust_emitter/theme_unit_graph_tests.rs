//! Tests of the units of a compiled theme and their graph ([`super`]). The cases of the key graph
//! follow the tests of the resource roots of #496, rewritten for units.

use super::*;

const NAMESPACES: &str = r#"xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml""#;

fn dictionary(body: &str) -> String {
    format!("<ResourceDictionary {NAMESPACES}>{body}</ResourceDictionary>")
}

/// A document of the group `Tests`: `(name, xaml, URI)`.
fn document(name: &str, xaml: String) -> (String, String, Option<String>) {
    (name.to_string(), xaml, Some(format!("ferres://tests/{name}")))
}

fn uri(name: &str) -> String {
    format!("ferres://tests/{name}")
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

fn unit<'p>(plan: &'p UnitPlan, name: &str) -> &'p ThemeUnitModel {
    plan.units.iter().find(|unit| unit.name == name).unwrap_or_else(|| panic!("the unit {name}: {:?}", plan.units))
}

/// The class document merges a control document and a resource document and includes the
/// controls otherwise; the theme of a window is ambient.
fn theme() -> Vec<(String, String, Option<String>)> {
    vec![
        document(
            "Theme.xaml",
            format!(
                r#"<Styles {NAMESPACES} x:Class="Tests.Theme"><Styles.Resources><ResourceDictionary>
                <ResourceDictionary.MergedDictionaries>
                  <MergeResourceInclude Source="/Resources/Base.xaml" />
                  <MergeResourceInclude Source="/Controls/Button.xaml" />
                  <MergeResourceInclude Source="/Controls/Window.xaml" />
                </ResourceDictionary.MergedDictionaries>
                <ResourceInclude x:Key="CompactStyles" Source="/Compact.xaml" />
                </ResourceDictionary></Styles.Resources>
                <Style Selector="MenuItem > PathIcon"><Setter Property="Template" Value="{{StaticResource Unscaled}}" /></Style>
                </Styles>"#
            ),
        ),
        document(
            "Resources/Base.xaml",
            dictionary(
                r#"<ResourceDictionary.ThemeDictionaries>
                  <ResourceDictionary x:Key="Light"><Color x:Key="Accent">#FF0000FF</Color><SolidColorBrush x:Key="AccentBrush" Color="{StaticResource Accent}" /></ResourceDictionary>
                  <ResourceDictionary x:Key="Dark"><Color x:Key="Accent">#FF000080</Color></ResourceDictionary>
                </ResourceDictionary.ThemeDictionaries>
                <ControlTemplate x:Key="Unscaled"><Viewbox /></ControlTemplate>
                <Style x:Key="Default"><Setter Property="Foreground" Value="{DynamicResource AccentBrush}" /></Style>"#,
            ),
        ),
        document(
            "Controls/Button.xaml",
            dictionary(
                r#"<Design.PreviewWith><Border><Button Theme="{StaticResource Preview}" /></Border></Design.PreviewWith>
                <ControlTheme x:Key="{x:Type Button}" TargetType="Button" BasedOn="{StaticResource Default}">
                  <Setter Property="Template"><ControlTemplate><ContentPresenter x:Name="PART_ContentPresenter" /></ControlTemplate></Setter>
                  <Style Selector="^:pointerover /template/ ContentPresenter#PART_ContentPresenter"><Setter Property="Background" Value="{DynamicResource AccentBrush}" /></Style>
                </ControlTheme>
                <ControlTheme x:Key="AccentButtonStyle" TargetType="Button" BasedOn="{StaticResource {x:Type Button}}" />"#,
            ),
        ),
        document("Controls/Window.xaml", dictionary(r#"<ControlTheme x:Key="{x:Type Window}" TargetType="Window" />"#)),
        document("Compact.xaml", dictionary(r#"<x:Double x:Key="CompactHeight">24</x:Double>"#)),
    ]
}

fn merged() -> Vec<String> {
    vec![uri("Resources/Base.xaml"), uri("Controls/Button.xaml"), uri("Controls/Window.xaml")]
}

#[test]
fn every_merged_document_is_a_unit_and_the_rest_is_the_ambient_unit() {
    let plan = plan(&theme(), &merged()).expect("units");
    let names: Vec<(&str, u32)> = plan.units.iter().map(|unit| (unit.name.as_str(), unit.index)).collect();
    assert_eq!(names, [(AMBIENT, 0), ("Resources/Base.xaml", 1), ("Controls/Button.xaml", 2)]);
    // The ambient unit: the documents that are not merged, and the theme of a host the
    // framework creates.
    let ambient = unit(&plan, AMBIENT);
    assert_eq!(ambient.documents, strings(&["Theme.xaml", "Controls/Window.xaml", "Compact.xaml"]));
    assert_eq!(ambient.keys, strings(&["CompactHeight", "CompactStyles"]));
    assert_eq!(ambient.type_keys, strings(&["Window"]));
    // The global style names a resource of a unit.
    assert_eq!(ambient.references, strings(&["Unscaled"]));
    assert_eq!(plan.unit_of_uri.get(&uri("Controls/Window.xaml")), Some(&0));
    assert_eq!(plan.unit_of_uri.get(&uri("Controls/Button.xaml")), Some(&2));
    assert_eq!(plan.unit_of_uri.get(&uri("Theme.xaml")), None);
}

/// #633: a document merged into an inline dictionary of the class document (a dictionary of its
/// own among the merged dictionaries of the theme) is a unit as a document merged into the
/// dictionary of the theme is; the inline dictionary is not an entry of the theme, so the class
/// document stays an ambient document with no entry that no root can name.
#[test]
fn a_document_merged_into_an_inline_dictionary_is_a_unit() {
    let documents = vec![
        document(
            "Theme.xaml",
            format!(
                r#"<Styles {NAMESPACES} x:Class="Tests.Theme"><Styles.Resources><ResourceDictionary>
                <ResourceDictionary.MergedDictionaries>
                  <ResourceDictionary><ResourceDictionary.MergedDictionaries><MergeResourceInclude Source="/Resources/Inherited.xaml" /></ResourceDictionary.MergedDictionaries></ResourceDictionary>
                  <MergeResourceInclude Source="/Resources/Base.xaml" />
                </ResourceDictionary.MergedDictionaries>
                </ResourceDictionary></Styles.Resources>
                </Styles>"#
            ),
        ),
        document(
            "Resources/Inherited.xaml",
            dictionary(
                r#"<ResourceDictionary.ThemeDictionaries>
                  <ResourceDictionary x:Key="Light"><StaticResource x:Key="InheritedBrush" ResourceKey="AccentBrush" /></ResourceDictionary>
                </ResourceDictionary.ThemeDictionaries>"#,
            ),
        ),
        document("Resources/Base.xaml", dictionary(r#"<SolidColorBrush x:Key="AccentBrush" Color="Blue" />"#)),
    ];
    let merged = vec![uri("Resources/Inherited.xaml"), uri("Resources/Base.xaml")];
    let plan = plan(&documents, &merged).expect("units");
    let names: Vec<(&str, u32)> = plan.units.iter().map(|unit| (unit.name.as_str(), unit.index)).collect();
    assert_eq!(names, [(AMBIENT, 0), ("Resources/Inherited.xaml", 1), ("Resources/Base.xaml", 2)]);
    let ambient = unit(&plan, AMBIENT);
    assert_eq!(ambient.documents, strings(&["Theme.xaml"]));
    assert!(ambient.keys.is_empty(), "{:?}", ambient.keys);
    let inherited = unit(&plan, "Resources/Inherited.xaml");
    assert_eq!(inherited.keys, strings(&["InheritedBrush"]));
    assert_eq!(inherited.references, strings(&["AccentBrush"]));
    assert_eq!(plan.unit_of_uri.get(&uri("Resources/Inherited.xaml")), Some(&1));
}

#[test]
fn a_unit_defines_its_keys_and_type_keys_and_references_the_keys_it_names() {
    let plan = plan(&theme(), &merged()).expect("units");
    let resources = unit(&plan, "Resources/Base.xaml");
    // Every definition of a key, in every theme dictionary, once.
    assert_eq!(resources.keys, strings(&["Accent", "AccentBrush", "Default", "Unscaled"]));
    assert!(resources.type_keys.is_empty());
    assert_eq!(resources.references, strings(&["Accent", "AccentBrush"]));

    let button = unit(&plan, "Controls/Button.xaml");
    assert_eq!(button.keys, strings(&["AccentButtonStyle"]));
    assert_eq!(button.type_keys, strings(&["Button"]));
    // `BasedOn`, `{DynamicResource}`; not the key of the preview.
    assert_eq!(button.references, strings(&["AccentBrush", "Default"]));
}

#[test]
fn a_unit_constructs_and_selects_the_types_of_its_elements_targets_selectors_and_type_extensions() {
    let plan = plan(&theme(), &merged()).expect("units");
    let button = unit(&plan, "Controls/Button.xaml");
    // The elements (template content included), the targets, the type selectors and the
    // `{x:Type}` of `BasedOn`; nothing of `Design.PreviewWith`, no class, name or pseudo-class
    // of a selector.
    assert_eq!(button.types, strings(&["Button", "ContentPresenter", "ControlTemplate", "ControlTheme", "ResourceDictionary", "Setter", "Style"]));
    assert!(!button.types.iter().any(|name| name == "Border" || name == "PART_ContentPresenter" || name == "pointerover"));
    let ambient = unit(&plan, AMBIENT);
    assert!(ambient.types.contains(&"MenuItem".to_string()) && ambient.types.contains(&"PathIcon".to_string()), "{:?}", ambient.types);
}

#[test]
fn entries_no_root_can_name_make_their_document_ambient() {
    let documents = vec![
        document("Theme.xaml", dictionary("")),
        document("Unkeyed.xaml", dictionary(r#"<ControlTheme TargetType="Button" />"#)),
        document("Static.xaml", dictionary(r#"<Color x:Key="{x:Static Colors.Red}" />"#)),
        document("Named.xaml", dictionary(r#"<Panel x:Key="Named"><Button x:Name="Visible" /></Panel>"#)),
        // A name in a template is local to the template.
        document(
            "Template.xaml",
            dictionary(r#"<ControlTheme x:Key="Local"><Setter Property="Template"><ControlTemplate><Button x:Name="PART_Content" /></ControlTemplate></Setter></ControlTheme>"#),
        ),
        // A literal key that is not an identifier is a key all the same.
        document("Dotted.xaml", dictionary(r#"<Color x:Key="special.key" /><Color x:Key="{}{literal}" />"#)),
    ];
    let merged: Vec<String> = documents.iter().skip(1).filter_map(|(_, _, uri)| uri.clone()).collect();
    let plan = plan(&documents, &merged).expect("units");
    let names: Vec<&str> = plan.units.iter().map(|unit| unit.name.as_str()).collect();
    assert_eq!(names, [AMBIENT, "Template.xaml", "Dotted.xaml"]);
    assert_eq!(unit(&plan, AMBIENT).documents, strings(&["Theme.xaml", "Unkeyed.xaml", "Static.xaml", "Named.xaml"]));
    assert_eq!(unit(&plan, "Dotted.xaml").keys, strings(&["special.key", "{literal}"]));
}

#[test]
fn the_dictionaries_of_an_entry_belong_to_the_entry() {
    let documents = vec![
        document("Theme.xaml", dictionary("")),
        document(
            "Controls/List.xaml",
            dictionary(
                r#"<ControlTheme x:Key="{x:Type ListBox}" TargetType="ListBox">
                  <ControlTheme.Resources><ResourceDictionary><Color x:Key="InnerAccent" /></ResourceDictionary></ControlTheme.Resources>
                  <Setter Property="Background" Value="{StaticResource InnerAccent}" />
                </ControlTheme>
                <StaticResource x:Key="ListAlias" ResourceKey="Outer.Key" />"#,
            ),
        ),
        document("Resources/Outer.xaml", dictionary(r#"<Color x:Key="Outer.Key" />"#)),
    ];
    let plan = plan(&documents, &[uri("Controls/List.xaml"), uri("Resources/Outer.xaml")]).expect("units");
    let list = unit(&plan, "Controls/List.xaml");
    assert_eq!(list.keys, strings(&["ListAlias"]));
    assert_eq!(list.type_keys, strings(&["ListBox"]));
    // A key that is not an identifier is referenced through `ResourceKey`; a local key of an
    // entry is not a key of the group.
    assert_eq!(list.references, strings(&["Outer.Key"]));
}

#[test]
fn without_merged_documents_or_with_a_document_that_is_not_plain_xml_there_are_no_units() {
    let documents = theme();
    assert_eq!(plan(&documents, &[]), None);
    // Only ambient documents are merged.
    assert_eq!(plan(&documents, &[uri("Controls/Window.xaml")]), None);
    let mut with_dtd = theme();
    with_dtd[2].1.insert_str(0, "<!DOCTYPE ResourceDictionary [<!-- <ignored> -->]>");
    assert_eq!(plan(&with_dtd, &merged()), None);
}

#[test]
fn selectors_name_their_type_selectors_only() {
    assert_eq!(selector_types("MenuItem > PathIcon"), ["MenuItem", "PathIcon"]);
    assert_eq!(selector_types("^:pointerover /template/ ContentPresenter#PART_ContentPresenter"), ["ContentPresenter"]);
    assert_eq!(selector_types("local|Item.selected:is(Button) [IsChecked=True] TextBlock"), ["Item", "Button", "TextBlock"]);
    assert_eq!(selector_types(":not(.Accent) Border:nth-child(2n+1)"), ["Border"]);
}

/// Step 3: an application document names its types (elements, `TargetType`, selectors,
/// `{x:Type}`) and every token that may be a key; an include's `Source` and the preview of a
/// design are not read.
#[test]
fn an_application_document_names_its_types_and_keys() {
    let xaml = format!(
        r#"<Window {NAMESPACES} x:Class="App.MainWindow" Title="Hello">
          <Window.Styles>
            <StyleInclude Source="ferres://App/Ignored/Theme.xaml" />
            <Style Selector="Button.accent > TextBlock"><Setter Property="Foreground" Value="{{DynamicResource AccentBrush}}" /></Style>
          </Window.Styles>
          <Design.PreviewWith><CalendarView /></Design.PreviewWith>
          <StackPanel>
            <TextBlock Text="Hello, World!" Theme="{{StaticResource ResourceKey=CaptionTextBlockStyle}}" />
            <ContentControl ContentTemplate="{{x:Type ToggleSwitch}}" />
          </StackPanel>
        </Window>"#
    );
    let references = document_references(&xaml).expect("plain XML");
    for name in ["Window", "Style", "StyleInclude", "StackPanel", "TextBlock", "ContentControl", "Button", "ToggleSwitch"] {
        assert!(references.types.contains(name), "{name}: {:?}", references.types);
    }
    assert!(!references.types.contains("CalendarView"), "{:?}", references.types);
    for key in ["AccentBrush", "CaptionTextBlockStyle", "Hello"] {
        assert!(references.keys.contains(key), "{key}: {:?}", references.keys);
    }
    assert!(!references.keys.contains("Ignored"), "{:?}", references.keys);
    assert_eq!(document_references("<!DOCTYPE Window [<!-- x -->]><Window />"), None);
}
