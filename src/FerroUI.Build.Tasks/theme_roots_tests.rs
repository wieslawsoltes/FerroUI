//! Tests of the units an application keeps ([`super`]). Not tests of upstream: small graphs of
//! fake units, rooted the ways an application roots them.

use super::*;
use ferroui_build_scan::xaml_metadata::ThemeUnitModel;

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|item| item.to_string()).collect()
}

/// A unit: its name, the keys and type keys it defines, the keys it references, the types it
/// constructs.
fn unit(index: u32, name: &str, keys: &[&str], type_keys: &[&str], references: &[&str], types: &[&str]) -> ThemeUnitModel {
    ThemeUnitModel {
        name: name.to_string(),
        index,
        documents: vec![name.to_string()],
        keys: strings(keys),
        type_keys: strings(type_keys),
        references: strings(references),
        types: strings(types),
    }
}

/// A theme of the shapes the WinUI theme has: the ambient unit, control documents with type keys
/// and the resources they reference, a style based on another, and a template that constructs a
/// control of another document.
fn theme() -> ThemeUnitsModel {
    ThemeUnitsModel {
        class: "Tests.Theme".to_string(),
        table_path: "::tests::compiled_xaml::THEME_UNITS".to_string(),
        units: vec![
            unit(0, "ambient", &["SystemAccentColor"], &["Window"], &["TextFillColorPrimaryBrush"], &["Window", "ContentPresenter"]),
            unit(1, "Resources/Common_themeresources.xaml", &["TextFillColorPrimaryBrush", "AccentFillColorDefaultBrush"], &[], &["SystemAccentColor"], &["SolidColorBrush"]),
            unit(2, "Resources/Button_themeresources.xaml", &["ButtonBackground"], &[], &["AccentFillColorDefaultBrush"], &["SolidColorBrush"]),
            unit(3, "Controls/Button.xaml", &["DefaultButtonStyle", "AccentButtonStyle"], &["Button"], &["ButtonBackground", "DefaultButtonStyle"], &["Button", "ContentPresenter", "Border"]),
            unit(4, "Controls/TextBlock.xaml", &["CaptionTextBlockStyle"], &["TextBlock"], &[], &["TextBlock"]),
            unit(5, "Controls/ComboBox.xaml", &["DefaultComboBoxStyle"], &["ComboBox"], &[], &["ComboBox", "Popup", "ToggleButton"]),
            unit(6, "Controls/ToggleButton.xaml", &[], &["ToggleButton"], &["DefaultButtonStyle"], &["ToggleButton"]),
            unit(7, "Controls/Popup.xaml", &[], &["Popup"], &[], &["Popup"]),
            unit(8, "Controls/CalendarView.xaml", &[], &["CalendarView"], &[], &["CalendarView", "Button"]),
        ],
    }
}

fn kept(roots: &Roots) -> Vec<u32> {
    kept_units(&theme(), roots, &BaseClasses::default(), false)
}

#[test]
fn the_ambient_unit_and_what_it_references_are_always_kept() {
    assert_eq!(kept(&Roots::default()), [0, 1]);
}

#[test]
fn a_type_roots_its_type_keyed_unit_and_the_resources_it_references() {
    let roots = Roots { types: set(&["Button"]), ..Roots::default() };
    // Button, its resources, and through them the common resources.
    assert_eq!(kept(&roots), [0, 1, 2, 3]);
}

#[test]
fn a_key_the_documents_name_roots_the_unit_that_defines_it() {
    let roots = Roots { keys: set(&["CaptionTextBlockStyle", "Hello", "World"]), ..Roots::default() };
    assert_eq!(kept(&roots), [0, 1, 4]);
}

#[test]
fn based_on_chains_keep_the_unit_of_the_base_style() {
    // `ToggleButton.xaml` references `DefaultButtonStyle` (its `BasedOn`): the unit of the button
    // is kept for it, and with it what that unit references.
    let roots = Roots { types: set(&["ToggleButton"]), ..Roots::default() };
    assert_eq!(kept(&roots), [0, 1, 2, 3, 6]);
}

#[test]
fn types_templates_construct_root_their_own_control_themes() {
    // The template of the combo box constructs a popup and a toggle button.
    let roots = Roots { types: set(&["ComboBox"]), ..Roots::default() };
    assert_eq!(kept(&roots), [0, 1, 2, 3, 5, 6, 7]);
}

#[test]
fn a_subclass_roots_the_themes_of_its_base_classes() {
    let bases = BaseClasses::from_pairs(&[("MyButton", "Button"), ("Button", "ButtonBase")]);
    assert_eq!(bases.chain("MyButton"), ["MyButton", "Button", "ButtonBase"]);
    let roots = Roots { types: set(&["MyButton"]), ..Roots::default() };
    assert_eq!(kept_units(&theme(), &roots, &bases, false), [0, 1, 2, 3]);
}

#[test]
fn explicit_roots_name_units_types_or_keys() {
    // A unit by the path of its document, a type, a key.
    let roots = Roots { names: set(&["Controls/Popup.xaml", "CalendarView", "CaptionTextBlockStyle"]), ..Roots::default() };
    // The calendar view constructs buttons.
    assert_eq!(kept(&roots), [0, 1, 2, 3, 4, 7, 8]);
}

#[test]
fn every_unit_is_kept_for_all_or_with_the_tables_of_types() {
    let all = Roots { all: true, ..Roots::default() };
    assert_eq!(kept(&all), (0..9).collect::<Vec<u32>>());
    assert_eq!(kept_units(&theme(), &Roots::default(), &BaseClasses::default(), true), (0..9).collect::<Vec<u32>>());
}

#[test]
fn roots_added_by_a_build_script_join_the_roots_of_the_documents() {
    let mut roots = Roots { types: set(&["TextBlock"]), ..Roots::default() };
    roots.extend(Roots { names: set(&["Popup"]), ..Roots::default() });
    assert_eq!(kept(&roots), [0, 1, 4, 7]);
}

#[test]
fn the_static_names_and_paths_follow_the_class_and_the_table() {
    assert_eq!(parts_static_name("FerroUI.Themes.WinUI.WinUITheme"), "THEME_PARTS_FERROUI_THEMES_WINUI_WINUITHEME");
    assert_eq!(unit_static_path("::ferroui_themes_winui::compiled_xaml::THEME_UNITS", 12), "::ferroui_themes_winui::compiled_xaml::THEME_UNIT_12");
}

#[test]
fn the_manifest_states_the_opt_in_and_the_roots() {
    let manifest = r#"
[package]
name = "app"
metadata.other = 1

[dependencies]
ferroui-base = { workspace = true, features = ["markup-functions"] }

[package.metadata.ferroui]
# Trim the themes.
theme-trimming = true # the opt-in
theme-roots = [
    "CalendarView", # created in code
    'AccentButtonStyle',
]

[package.metadata.release]
group = "samples"
"#;
    let trimming = ThemeTrimming::from_manifest(manifest).expect("a manifest");
    assert_eq!(trimming, ThemeTrimming { enabled: true, all: false, roots: strings(&["CalendarView", "AccentButtonStyle"]) });

    let all = ThemeTrimming::from_manifest("[package.metadata.ferroui]\ntheme-trimming = true\ntheme-roots = \"all\"\n").expect("a manifest");
    assert_eq!(all, ThemeTrimming { enabled: true, all: true, roots: Vec::new() });

    // Without the table, nothing is trimmed.
    assert_eq!(ThemeTrimming::from_manifest("[package]\nname = \"app\"\n"), Ok(ThemeTrimming::default()));
    assert_eq!(
        ThemeTrimming::from_manifest("[package.metadata.ferroui]\ntheme-trimming = \"yes\"\n"),
        Err("[package.metadata.ferroui] theme-trimming must be true or false".to_string())
    );
    assert_eq!(
        ThemeTrimming::from_manifest("[package.metadata.ferroui]\ntheme-roots = 3\n"),
        Err("[package.metadata.ferroui] theme-roots must be \"all\" or an array of names".to_string())
    );
}
