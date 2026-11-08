//! The units of the compiled markup of the theme (#568, docs/porting/theme-trimming.md), as the
//! tests of the WinUI and Fluent themes check them.
//!
//! The crate has no checked-in reference of its compiled markup: the tests read what the build
//! script wrote, `$OUT_DIR/xaml/compiled_xaml.rs` and the `.xamlmeta` of the crate
//! (`FERROUI_XAMLMETA`).

use ferroui_base::controls::ResourceKey;
use ferroui_base::media::IBrush;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::StaticType;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_markup_xaml::xaml_il::runtime::compiled::ThemeUnit;
use ferroui_markup_xaml_loader::rust_emitter::{ThemeUnitsModel, XamlMetadata};
use std::rc::Rc;

use crate::{MaterialColorRole, MaterialColorScheme, MaterialTheme};

/// The generated file of the build script.
const BUILT: &str = concat!(env!("OUT_DIR"), "/xaml/compiled_xaml.rs");

fn start() -> UnitTestApplicationScope {
    crate::register_types();
    UnitTestApplication::start(TestServices::styled_window())
}

/// The unit graph of the theme in the `.xamlmeta` of the build.
fn unit_graph() -> ThemeUnitsModel {
    let path = env!("FERROUI_XAMLMETA");
    let text = std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{path} cannot be read: {error}"));
    let metadata = XamlMetadata::parse(&text).unwrap_or_else(|error| panic!("{path}: {error}"));
    let [graph] = metadata.theme_units.as_slice() else { panic!("one class with units: {:?}", metadata.theme_units.len()) };
    graph.clone()
}

/// The variants the theme has resources for.
fn variants() -> [ThemeVariant; 3] {
    [ThemeVariant::light(), ThemeVariant::dark(), MaterialTheme::high_contrast()]
}

/// The table of all units without the unit named `name`.
fn without(name: &str) -> &'static [&'static ThemeUnit] {
    let units = crate::compiled_xaml::THEME_UNITS;
    let left_out = units.iter().position(|unit| unit.name == name).unwrap_or_else(|| panic!("a unit {name}"));
    let parts: Vec<&'static ThemeUnit> =
        units.iter().enumerate().filter(|(position, _)| *position != left_out).map(|(_, unit)| *unit).collect();
    Box::leak(parts.into_boxed_slice())
}

/// The compiled markup is partitioned into units: the table of all units is in canonical order,
/// the `.xamlmeta` describes the same units, each control document and each inherited resource
/// document is a unit of its own, and the whole theme, which applies every unit through the
/// table, has every key every unit of a merged document defines.
#[test]
fn the_whole_theme_applies_every_unit() {
    let _app = start();
    let units = crate::compiled_xaml::THEME_UNITS;
    assert!(units.len() > 150, "{} units", units.len());
    for (position, unit) in units.iter().enumerate() {
        assert_eq!(unit.index as usize, position, "{unit:?}");
    }
    assert_eq!(units[0].name, "ambient");

    let graph = unit_graph();
    assert_eq!(graph.class, "FerroUI.Themes.Material.MaterialTheme");
    assert_eq!(graph.table_path, "::ferroui_themes_material::compiled_xaml::THEME_UNITS");
    let names: Vec<&str> = graph.units.iter().map(|unit| unit.name.as_str()).collect();
    assert_eq!(names, units.iter().map(|unit| unit.name).collect::<Vec<_>>());
    let ambient = &graph.units[0];
    for host in ["Controls/Window.xaml", "Controls/PopupRoot.xaml", "Controls/OverlayPopupHost.xaml", "Controls/AdornerLayer.xaml"] {
        assert!(ambient.documents.iter().any(|document| document == host), "{host} is ambient: {:?}", ambient.documents);
    }
    let unit = |name: &str| graph.units.iter().find(|unit| unit.name == name).unwrap_or_else(|| panic!("a unit {name}"));
    let button = unit("Controls/Button.xaml");
    assert!(button.type_keys.iter().any(|type_key| type_key == "Button"), "{button:?}");
    assert!(button.keys.iter().any(|key| key == "AccentButtonStyle"), "{button:?}");
    // The inherited resource documents are units, each of one document (MaterialTheme.xaml
    // merges each into a merged dictionary of its own).
    for name in ["Resources/Common_themeresources.xaml", "Resources/Button_themeresources.xaml", "Resources/RatingControl_themeresources.xaml"] {
        let resources = unit(name);
        assert_eq!(resources.documents, [name], "{resources:?}");
        assert!(!resources.keys.is_empty(), "{resources:?}");
    }
    assert!(unit("Resources/RatingControl_themeresources.xaml").keys.iter().any(|key| key == "RatingControlSelectedForeground"));
    for name in ["Resources/MaterialColors.xaml", "Resources/MaterialCompatibility.xaml", "Controls/MaterialBottomSheet.xaml"] {
        unit(name);
    }

    let theme = MaterialTheme::new();
    let variants = variants();
    for unit in &graph.units[1..] {
        // The theme replaces the theme dictionaries of its own resources with the color schemes
        // when it is constructed (`set_color_scheme`); `compiled_theme_resolves_every_role_in_both_variants`
        // checks the roles they define.
        if unit.name == "Resources/MaterialColors.xaml" {
            continue;
        }
        for key in &unit.keys {
            let key = ResourceKey::from(key.as_str());
            let found = variants.iter().any(|variant| theme.try_get_resource(&key, Some(variant)).is_some());
            assert!(found, "{key:?} of the unit {} is not in the theme", unit.name);
        }
    }
}

/// A theme constructed from some of its units (`with_parts`) leaves out the entries of the units it
/// is not given, and has them with the table of all units. The rating control's unit defines only
/// its control theme, which no other unit references.
#[test]
fn a_theme_with_parts_leaves_out_the_units_it_is_not_given() {
    let _app = start();
    let units = crate::compiled_xaml::THEME_UNITS;
    let key = ResourceKey::Type(<ferroui_controls::RatingControl as StaticType>::TYPE);

    let trimmed = MaterialTheme::with_parts(None, without("Controls/RatingControl.xaml"));
    assert!(trimmed.try_get_resource(&key, None).is_none(), "the control theme of the rating control is left out");
    let full = MaterialTheme::with_parts(None, units);
    assert!(full.try_get_resource(&key, None).is_some(), "the full table has the control theme of the rating control");
    // With the tables of types (the tests link the run-time loader), a load by URI and the type
    // information apply every unit.
    assert_eq!(crate::compiled_xaml::registered_theme_units().len(), units.len());
}

/// An inherited resource document is a unit of its own: a theme without it has none of its keys,
/// in any variant, and the color schemes and the motion resources are set as in the full theme.
#[test]
fn a_theme_with_parts_leaves_out_an_inherited_resource_document() {
    let _app = start();
    let key = ResourceKey::from("RatingControlSelectedForeground");
    // The trimmed theme is checked before the full one is constructed: the populate of the class
    // makes its table the active one (#568, step 3).
    let trimmed = MaterialTheme::with_parts(None, without("Resources/RatingControl_themeresources.xaml"));
    for variant in variants() {
        assert!(trimmed.try_get_resource(&key, Some(&variant)).is_none(), "{key:?} in {variant:?} is left out");
    }
    let primary = trimmed.try_get_resource(&ResourceKey::from("MaterialPrimaryColor"), Some(&ThemeVariant::light())).flatten();
    assert_eq!(
        from_markup_value::<ferroui_base::media::Color>(&primary),
        Some(MaterialColorScheme::expressive_light().color(MaterialColorRole::Primary))
    );
    assert!(trimmed.try_get_resource(&ResourceKey::from("MaterialReducedMotion"), None).is_some());
    let full = MaterialTheme::with_parts(None, crate::compiled_xaml::THEME_UNITS);
    for variant in variants() {
        assert!(full.try_get_resource(&key, Some(&variant)).is_some(), "{key:?} in {variant:?} is in the full theme");
    }
}

/// Each inherited resource document is a dictionary of its own (MaterialTheme.xaml), as before it
/// was a unit: a `{StaticResource}` of one document that names a key of another document, which the
/// color schemes of the theme override, resolves to the theme's value. `RadioButtonsHeaderForeground`
/// of `RadioButtons_themeresources.xaml` is `TextFillColorPrimaryBrush` of
/// `Common_themeresources_any.xaml` (`#E4000000` in the light variant), which the color schemes
/// replace with the on-surface role. A dictionary that held both documents would resolve it to the
/// inherited brush.
#[test]
fn an_inherited_resource_resolves_the_overrides_of_the_theme() {
    let _app = start();
    let theme = MaterialTheme::new();
    let value = theme.try_get_resource(&ResourceKey::from("RadioButtonsHeaderForeground"), Some(&ThemeVariant::light())).flatten();
    let color = from_markup_value::<Rc<dyn IBrush>>(&value)
        .and_then(|brush| brush.as_solid_color_brush().map(|brush| brush.color()))
        .expect("the brush RadioButtonsHeaderForeground");
    assert_ne!(color, ferroui_base::media::Color::from_uint32(0xe400_0000), "the inherited brush");
    let schemes = [MaterialColorScheme::expressive_light(), MaterialColorScheme::dark()];
    assert!(
        schemes.iter().any(|scheme| scheme.color(MaterialColorRole::OnSurface) == color),
        "RadioButtonsHeaderForeground is {color:?}, not the on-surface role of a color scheme"
    );
}

/// #612, root D: a namespace table that names classes for reflection-binding paths is reachable
/// only through the units of the theme, so a unit the application trims takes those classes with
/// it. From the public functions of the module, following the items each item names, without
/// passing through the table of a unit (`THEME_UNIT_<n>`, `THEME_UNITS`), no such table is
/// reached. As the test of the WinUI theme, on the output of the build.
#[test]
fn the_classes_of_reflection_paths_are_reached_only_through_their_units() {
    use std::collections::{BTreeMap, BTreeSet};
    let source = std::fs::read_to_string(BUILT).unwrap_or_else(|error| panic!("{BUILT} cannot be read: {error}"));
    // The top-level items: a line that starts an item at column 0 begins it.
    let mut items: BTreeMap<String, (bool, String)> = BTreeMap::new();
    let mut current: Option<(String, bool)> = None;
    let mut body = String::new();
    let item_name = |line: &str| -> Option<(String, bool)> {
        let rest = line.strip_prefix("pub ").unwrap_or(line);
        let public = line.starts_with("pub ");
        for keyword in ["fn ", "static ", "const "] {
            if let Some(after) = rest.strip_prefix(keyword) {
                let name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                return (!name.is_empty()).then_some((name, public));
            }
        }
        None
    };
    for line in source.lines() {
        if let Some(found) = item_name(line) {
            if let Some((name, public)) = current.take() {
                items.insert(name, (public, std::mem::take(&mut body)));
            }
            current = Some(found);
        }
        body.push_str(line);
        body.push('\n');
    }
    if let Some((name, public)) = current {
        items.insert(name, (public, body));
    }
    let names: BTreeSet<&str> = items.keys().map(String::as_str).collect();
    let references = |text: &str| -> Vec<String> {
        text.split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .filter(|word| names.contains(word))
            .map(str::to_string)
            .collect()
    };
    // The channel of the units: the table of each unit and the table of all of them.
    let selective = |name: &str| {
        name == "THEME_UNITS" || name.strip_prefix("THEME_UNIT_").is_some_and(|index| index.parse::<u32>().is_ok())
    };
    let class_tables: BTreeSet<&str> = items
        .iter()
        .filter(|(_, (_, text))| text.contains("rt::XmlNamespaceTable") && text.contains("CastTarget::Class("))
        .map(|(name, _)| name.as_str())
        .collect();
    assert!(!class_tables.is_empty(), "the theme has reflection-binding paths that name classes");
    let mut reached: BTreeSet<String> = BTreeSet::new();
    let mut pending: Vec<String> = items.iter().filter(|(_, (public, _))| *public).map(|(name, _)| name.clone()).collect();
    pending.retain(|name| !selective(name));
    while let Some(name) = pending.pop() {
        if !reached.insert(name.clone()) {
            continue;
        }
        let own = items[&name].1.clone();
        // The item's own name in its first line is not a reference.
        for referenced in references(&own) {
            if referenced != name && !selective(&referenced) && !reached.contains(&referenced) {
                pending.push(referenced);
            }
        }
    }
    let leaked: Vec<&&str> = class_tables.iter().filter(|table| reached.contains(**table)).collect();
    assert!(leaked.is_empty(), "namespace tables naming classes reached outside their units: {leaked:?}");
    // The loader table answers the document of the class alone: every other document is internal,
    // so that no document of a unit is reachable from `try_load`.
    let try_load = &items.get("try_load").expect("the loader table").1;
    assert_eq!(try_load.matches("rt::uri_equals(").count(), 1, "{try_load}");
}
