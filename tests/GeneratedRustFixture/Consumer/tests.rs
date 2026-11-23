//! The names and the constructions the build of the crate generated (#849). That the crate
//! compiles is most of the check: rustc resolves every path the tests name, and refused the
//! generated Rust before the fix (`App::with_parts`, two parts tables and two build functions of
//! one name).

use ferroui_markup_xaml::xaml_il::runtime::compiled::ThemeParts;

use crate::{compiled_collision_styles, compiled_markup, compiled_xaml};

/// The compiled documents of the group `compiled_xaml`, as the build script wrote them.
const GENERATED: &str = include_str!(concat!(env!("OUT_DIR"), "/xaml/compiled_xaml.rs"));

/// A class whose markup is partitioned into units and that cannot be constructed from selected
/// units is constructed with `new()`: an application class with `new()` only, and a theme whose
/// constructor from units has another name. The themes that can are constructed with `with_parts`.
#[test]
fn only_a_class_constructed_from_units_is_constructed_with_its_parts() {
    let library = "::generated_rust_fixture_library";
    for constructed in [format!("{library}::App::new()"), format!("{library}::RenamedTheme::new()")] {
        assert!(GENERATED.contains(&constructed), "{constructed}: {GENERATED}");
    }
    assert!(!GENERATED.contains("App::with_parts"), "{GENERATED}");
    assert!(!GENERATED.contains("RenamedTheme::with_parts"), "{GENERATED}");
    let themes = ["PartsTheme", "Partstheme", "FluentTheme", "MacOSTheme", "MaterialTheme", "SimpleTheme"];
    for theme in themes.into_iter().chain(["WinUITheme"]) {
        let construction = format!("{theme}::with_parts(::core::option::Option::None, ");
        assert!(GENERATED.contains(&construction), "{theme}: {GENERATED}");
    }
}

/// Every theme of the framework keeps the established name of its parts table, which the code of
/// an application names (`Build::theme`); the two themes whose established names are one get the
/// name disambiguated from their full names. Each table has the ambient unit at least.
#[test]
fn parts_tables_keep_the_established_names_where_they_are_unique() {
    let tables: [ThemeParts; 7] = [
        compiled_markup::THEME_PARTS_FERROUI_THEMES_FLUENT_FLUENTTHEME,
        compiled_markup::THEME_PARTS_FERROUI_THEMES_MACOS_MACOSTHEME,
        compiled_markup::THEME_PARTS_FERROUI_THEMES_MATERIAL_MATERIALTHEME,
        compiled_markup::THEME_PARTS_FERROUI_THEMES_SIMPLE_SIMPLETHEME,
        compiled_markup::THEME_PARTS_FERROUI_THEMES_WINUI_WINUITHEME,
        // `GeneratedRust.Library.A_B.PartsTheme`
        compiled_markup::THEME_PARTS_GENERATEDRUST_LIBRARY_A_B_PARTSTHEME_B1A67636,
        // `GeneratedRust.Library.A.B.Partstheme`
        compiled_markup::THEME_PARTS_GENERATEDRUST_LIBRARY_A_B_PARTSTHEME_D780CBFD,
    ];
    for table in tables {
        assert!(!table.is_empty());
    }
}

/// Two public documents whose established build functions are one name get functions of two
/// names, disambiguated from their paths: in a group of documents and in the group of a class.
#[test]
fn documents_whose_functions_collide_get_two_names() {
    // `Views/A-B.xaml` and `Views/A_B.xaml`.
    let _ = compiled_xaml::build_views_a_b_xaml_b89f822e;
    let _ = compiled_xaml::build_views_a_b_xaml_9a3d4304;
    // `Styles/A-B.xaml` and `Styles/A_B.xaml`.
    let _ = compiled_collision_styles::build_styles_a_b_xaml_9e379e34;
    let _ = compiled_collision_styles::build_styles_a_b_xaml_99c3347e;
    // A name that is unique stays the established one.
    let _ = compiled_xaml::build_application_xaml;
    let entry = "(\"Views/A-B.xaml\", build_views_a_b_xaml_b89f822e_untyped as BuildDocument)";
    assert!(GENERATED.contains(entry), "{GENERATED}");
}
