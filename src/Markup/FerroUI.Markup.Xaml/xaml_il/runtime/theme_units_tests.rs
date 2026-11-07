//! Tests of the units of a compiled theme ([`super`]). Not tests of upstream: a table of units is
//! applied the way the functions of generated code apply it.

use super::*;
use crate::xaml_il::runtime::compiled::{populate_context, to_value, DocumentInfo, XmlNamespaceTable};
use ferroui_base::controls::ResourceKey;
use ferroui_base::BoxedValue;

const NAMESPACES: XmlNamespaceTable = &[("", &[("FerroUI.Controls", "FerroUI.Controls", &[])]), ("x", &[])];

static DOCUMENT: DocumentInfo = DocumentInfo { base_uri: Some("ferres://tests/Theme.xaml"), namespaces: NAMESPACES };

fn added(dictionary: &Ref<ResourceDictionary>, key: &str) {
    dictionary.add(key, Some(Rc::new(key.to_string()) as BoxedValue));
}

fn ambient_run(_context: &Rc<XamlIlContext>, dictionary: &Ref<ResourceDictionary>) -> Result<(), XamlLoadError> {
    added(dictionary, "Ambient");
    Ok(())
}

fn first_run(_context: &Rc<XamlIlContext>, dictionary: &Ref<ResourceDictionary>) -> Result<(), XamlLoadError> {
    added(dictionary, "First");
    Ok(())
}

fn second_run(_context: &Rc<XamlIlContext>, dictionary: &Ref<ResourceDictionary>) -> Result<(), XamlLoadError> {
    added(dictionary, "Second");
    Ok(())
}

static AMBIENT: ThemeUnit = ThemeUnit { name: "ambient", index: 0, runs: &[ambient_run] };
static BUTTON: ThemeUnit = ThemeUnit { name: "Controls/Button.xaml", index: 1, runs: &[first_run, second_run] };
static UNITS: &[&ThemeUnit] = &[&AMBIENT, &BUTTON];

fn context_of(dictionary: &Ref<ResourceDictionary>) -> Rc<XamlIlContext> {
    crate::register_types();
    populate_context(None, &DOCUMENT, to_value(dictionary.clone()))
}

/// Whether `dictionary` has the resource `key`.
fn has(dictionary: &Ref<ResourceDictionary>, key: &str) -> bool {
    dictionary.try_get_value(&ResourceKey::from(key)).flatten().is_some()
}

#[test]
fn every_run_of_a_unit_of_the_table_is_applied() {
    let dictionary = ResourceDictionary::new();
    let context = context_of(&dictionary);
    apply_theme_unit(UNITS, 0, 0, &context, &dictionary).expect("the ambient run");
    apply_theme_unit(UNITS, 1, 0, &context, &dictionary).expect("the first run");
    apply_theme_unit(UNITS, 1, 1, &context, &dictionary).expect("the second run");
    assert_eq!(dictionary.count(), 3);
    assert!(has(&dictionary, "Ambient") && has(&dictionary, "First") && has(&dictionary, "Second"));
}

#[test]
fn a_unit_the_table_leaves_out_adds_nothing() {
    static AMBIENT_ONLY: &[&ThemeUnit] = &[&AMBIENT];
    let dictionary = ResourceDictionary::new();
    let context = context_of(&dictionary);
    apply_theme_unit(AMBIENT_ONLY, 0, 0, &context, &dictionary).expect("the ambient run");
    apply_theme_unit(AMBIENT_ONLY, 1, 0, &context, &dictionary).expect("nothing");
    apply_theme_unit(AMBIENT_ONLY, 1, 1, &context, &dictionary).expect("nothing");
    // A run the unit does not have adds nothing either.
    apply_theme_unit(AMBIENT_ONLY, 0, 3, &context, &dictionary).expect("nothing");
    assert_eq!(dictionary.count(), 1);
    assert!(has(&dictionary, "Ambient"));
}

#[test]
fn units_are_found_by_name() {
    assert_eq!(find_theme_unit(UNITS, "Controls/Button.xaml").map(|unit| unit.index), Some(1));
    assert!(find_theme_unit(UNITS, "Controls/Missing.xaml").is_none());
}
