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

static AMBIENT: ThemeUnit = ThemeUnit { name: "ambient", index: 0, runs: &[ambient_run], entries: &[] };
static BUTTON: ThemeUnit = ThemeUnit { name: "Controls/Button.xaml", index: 1, runs: &[first_run, second_run], entries: &[] };
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

/// Step 3: the builds of a class apply the table the last populate of the class made active,
/// and no unit before the first populate.
#[test]
fn the_active_table_is_the_last_one_set_for_the_class() {
    static AMBIENT_ONLY: &[&ThemeUnit] = &[&AMBIENT];
    assert!(active_theme_units("Tests.ActiveTheme").is_empty());
    set_active_theme_units("Tests.ActiveTheme", UNITS);
    set_active_theme_units("Tests.OtherTheme", AMBIENT_ONLY);
    assert_eq!(active_theme_units("Tests.ActiveTheme").len(), 2);
    set_active_theme_units("Tests.ActiveTheme", AMBIENT_ONLY);
    assert_eq!(active_theme_units("Tests.ActiveTheme").iter().map(|unit| unit.index).collect::<Vec<_>>(), [0]);
    assert_eq!(active_theme_units("Tests.OtherTheme").len(), 1);
}

/// Step 3: an application registers the parts it keeps; a registration replaces an earlier one,
/// and the registration of the full theme does not replace one.
#[test]
fn registered_parts_are_found_by_class() {
    static AMBIENT_ONLY: &[&ThemeUnit] = &[&AMBIENT];
    assert!(registered_theme_parts("Tests.RegisteredTheme").is_none());
    register_theme_parts_if_absent("Tests.RegisteredTheme", UNITS);
    assert_eq!(registered_theme_parts("Tests.RegisteredTheme").map(<[_]>::len), Some(2));
    register_theme_parts("Tests.RegisteredTheme", AMBIENT_ONLY);
    register_theme_parts_if_absent("Tests.RegisteredTheme", UNITS);
    assert_eq!(registered_theme_parts("Tests.RegisteredTheme").map(<[_]>::len), Some(1));
}

/// Step 3: the type keys a table leaves out are those of the units it does not have.
#[test]
fn trimmed_type_keys_are_those_of_the_units_left_out() {
    static AMBIENT_ONLY: &[&ThemeUnit] = &[&AMBIENT];
    static TYPE_KEYS: &[&[&str]] = &[&["Window"], &["Button", "RepeatButton"]];
    assert_eq!(trimmed_type_keys(TYPE_KEYS, AMBIENT_ONLY), ["Button", "RepeatButton"]);
    assert!(trimmed_type_keys(TYPE_KEYS, UNITS).is_empty());
}

/// Step 3: only the type key of a trimmed control theme is reported.
#[test]
fn only_trimmed_type_keys_are_reported() {
    use ferroui_base::StaticType;
    let trimmed = TrimmedKeys { types: vec!["ResourceDictionary"], texts: Vec::new() };
    assert!(report_trimmed_theme("Tests", &ResourceKey::Type(<ResourceDictionary as StaticType>::TYPE), &trimmed));
    // Reported once, and still recognised.
    assert!(report_trimmed_theme("Tests", &ResourceKey::Type(<ResourceDictionary as StaticType>::TYPE), &trimmed));
    assert!(!report_trimmed_theme("Tests", &ResourceKey::from("ResourceDictionary"), &trimmed));
    assert!(!report_trimmed_theme("Tests", &ResourceKey::Type(<ResourceDictionary as StaticType>::TYPE), &TrimmedKeys::default()));
}

/// #852, step 2: a text key is left out when no kept unit still defines it: the keys of a unit the
/// table does not have and the selectable keys whose entries its static does not name, but not a
/// key another kept unit defines, and such a key is reported as a trimmed resource.
#[test]
fn trimmed_text_keys_are_those_no_kept_unit_defines() {
    fn build(_provider: &Rc<dyn ferroui_base::metadata::IServiceProvider>) -> Result<ferroui_base::metadata::MarkupValue, XamlLoadError> {
        Ok(None)
    }
    static LIVE: ThemeEntry = ThemeEntry { run: 0, key: "Live", build, line: 1, position: 1 };
    static SELECTED: ThemeUnit = ThemeUnit { name: "Resources/Base.xaml", index: 1, runs: &[first_run], entries: &[&LIVE] };
    static PARTS: &[&ThemeUnit] = &[&AMBIENT, &SELECTED];
    static TYPE_KEYS: &[&[&str]] = &[&[], &[], &["Button"]];
    static TEXT_KEYS: &[&[&str]] = &[&["Shared"], &["Dead", "Inline", "Live", "Shared"], &["OfALeftOutUnit", "Inline"]];
    static ENTRY_KEYS: &[&[&str]] = &[&[], &["Dead", "Live", "Shared"], &[]];
    let trimmed = trimmed_keys(TYPE_KEYS, TEXT_KEYS, ENTRY_KEYS, PARTS);
    assert_eq!(trimmed.types, ["Button"]);
    // `Shared` is defined by the ambient unit, and `Inline` by a run of the kept unit.
    assert_eq!(trimmed.texts, ["Dead", "OfALeftOutUnit"]);
    assert!(report_trimmed_theme("Tests", &ResourceKey::from("Dead"), &trimmed));
    assert!(report_trimmed_theme("Tests", &ResourceKey::from("Dead"), &trimmed));
    assert!(!report_trimmed_theme("Tests", &ResourceKey::from("Live"), &trimmed));
    assert!(!report_trimmed_theme("Tests", &ResourceKey::from("Unknown"), &trimmed));
}

ferroui_base::ferro_thread_local! {
    /// The runs of the region tests, in the order they were applied.
    static APPLIED: std::cell::RefCell<Vec<&'static str>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn logged(name: &'static str) -> Result<(), XamlLoadError> {
    APPLIED.with(|applied| applied.borrow_mut().push(name));
    Ok(())
}

fn applied() -> Vec<&'static str> {
    APPLIED.with(|applied| std::mem::take(&mut *applied.borrow_mut()))
}

fn base_light(_: &Rc<XamlIlContext>, _: &Ref<ResourceDictionary>) -> Result<(), XamlLoadError> {
    logged("base light")
}

fn base_dark(_: &Rc<XamlIlContext>, _: &Ref<ResourceDictionary>) -> Result<(), XamlLoadError> {
    logged("base dark")
}

fn button_light(_: &Rc<XamlIlContext>, _: &Ref<ResourceDictionary>) -> Result<(), XamlLoadError> {
    logged("button light")
}

fn failing(_: &Rc<XamlIlContext>, _: &Ref<ResourceDictionary>) -> Result<(), XamlLoadError> {
    logged("failing")?;
    Err(XamlLoadError::with_message("the run failed"))
}

fn list_light(_: &Rc<XamlIlContext>, _: &Ref<ResourceDictionary>) -> Result<(), XamlLoadError> {
    logged("list light")
}

static REGION_BASE: ThemeUnit = ThemeUnit { name: "Resources/Base.xaml", index: 1, runs: &[base_light, base_dark], entries: &[] };
static REGION_BUTTON: ThemeUnit = ThemeUnit { name: "Controls/Button.xaml", index: 2, runs: &[button_light], entries: &[] };
static REGION_FAILING: ThemeUnit = ThemeUnit { name: "Controls/Failing.xaml", index: 3, runs: &[failing], entries: &[] };
static REGION_LIST: ThemeUnit = ThemeUnit { name: "Controls/List.xaml", index: 4, runs: &[list_light], entries: &[] };
static REGION_UNITS: &[&ThemeUnit] = &[&AMBIENT, &REGION_BASE, &REGION_BUTTON, &REGION_LIST];

/// #853: a region applies the runs of the units the table has, in the order of the region, and
/// the same runs as one call of `apply_theme_unit` for each pair.
#[test]
fn a_region_applies_the_runs_of_the_table_in_its_order() {
    let dictionary = ResourceDictionary::new();
    let context = context_of(&dictionary);
    applied();
    let region: &[(u16, u16)] = &[(1, 0), (2, 0), (3, 0), (4, 0), (1, 1)];
    apply_theme_region(REGION_UNITS, region, &context, &dictionary).expect("the region");
    let by_region = applied();
    assert_eq!(by_region, ["base light", "button light", "list light", "base dark"]);
    for &(unit, run) in region {
        apply_theme_unit(REGION_UNITS, u32::from(unit), usize::from(run), &context, &dictionary).expect("a run");
    }
    assert_eq!(applied(), by_region);
}

/// #853: a selection leaves out the runs of the units it does not have, and a pair of a run the
/// unit does not have adds nothing; units that do not ascend are found too.
#[test]
fn a_region_with_a_selection_applies_only_the_selected_units() {
    static SELECTION: &[&ThemeUnit] = &[&AMBIENT, &REGION_BUTTON, &REGION_LIST];
    let dictionary = ResourceDictionary::new();
    let context = context_of(&dictionary);
    applied();
    let region: &[(u16, u16)] = &[(1, 0), (2, 0), (2, 5), (4, 0), (2, 0), (1, 1), (9, 0)];
    apply_theme_region(SELECTION, region, &context, &dictionary).expect("the region");
    assert_eq!(applied(), ["button light", "list light", "button light"]);
    apply_theme_region(&[], &[(1, 0), (2, 0)], &context, &dictionary).expect("an empty table");
    assert!(applied().is_empty());
}

/// #853: the first failure of a region is its error, and the runs after it are not applied.
#[test]
fn a_region_stops_at_the_first_failure() {
    static WITH_FAILING: &[&ThemeUnit] = &[&AMBIENT, &REGION_BASE, &REGION_BUTTON, &REGION_FAILING, &REGION_LIST];
    let dictionary = ResourceDictionary::new();
    let context = context_of(&dictionary);
    applied();
    let region: &[(u16, u16)] = &[(1, 0), (3, 0), (4, 0)];
    let error = apply_theme_region(WITH_FAILING, region, &context, &dictionary).expect_err("the failure");
    assert_eq!(error.message(), "the run failed");
    assert_eq!(applied(), ["base light", "failing"]);
}

/// Not from upstream (#846): the wider table registered for a class stays, whichever crate
/// registers first, and a compiled construction applies the registered table when it is wider
/// than the table of its own crate.
#[test]
fn the_wider_table_of_a_class_is_registered_and_applied() {
    static ONLY_AMBIENT: &[&ThemeUnit] = &[&AMBIENT];
    let class = "Tests.WiderTheme";
    assert!(std::ptr::eq(wider_theme_parts(class, ONLY_AMBIENT), ONLY_AMBIENT), "nothing registered: its own table");
    register_wider_theme_parts(class, UNITS);
    register_wider_theme_parts(class, ONLY_AMBIENT);
    assert_eq!(registered_theme_parts(class).map(<[&ThemeUnit]>::len), Some(2), "the narrower table does not replace");
    assert!(std::ptr::eq(wider_theme_parts(class, ONLY_AMBIENT), UNITS));
    // A construction whose own table is as wide keeps it.
    assert!(std::ptr::eq(wider_theme_parts(class, UNITS), UNITS));
}

/// #852, step 2: a run adds its statements and then the entries of that run that the unit names;
/// an entry the table of the application leaves out is not in the dictionary; and of two tables of
/// one theme with the same units, the one with more entries is the wider.
#[test]
fn a_unit_adds_the_entries_it_names_after_its_run() {
    fn build(_provider: &Rc<dyn ferroui_base::metadata::IServiceProvider>) -> Result<ferroui_base::metadata::MarkupValue, XamlLoadError> {
        Ok(Some(Rc::new("built".to_string()) as BoxedValue))
    }
    static IN_FIRST: ThemeEntry = ThemeEntry { run: 0, key: "InFirst", build, line: 1, position: 2 };
    static IN_SECOND: ThemeEntry = ThemeEntry { run: 1, key: "InSecond", build, line: 3, position: 4 };
    static WHOLE: ThemeUnit =
        ThemeUnit { name: "Controls/Button.xaml", index: 1, runs: &[first_run, second_run], entries: &[&IN_FIRST, &IN_SECOND] };
    static SELECTED: ThemeUnit =
        ThemeUnit { name: "Controls/Button.xaml", index: 1, runs: &[first_run, second_run], entries: &[&IN_SECOND] };
    static WHOLE_PARTS: &[&ThemeUnit] = &[&WHOLE];
    static SELECTED_PARTS: &[&ThemeUnit] = &[&SELECTED];

    let dictionary = ResourceDictionary::new();
    let context = context_of(&dictionary);
    apply_theme_unit(WHOLE_PARTS, 1, 0, &context, &dictionary).expect("the first run");
    assert!(has(&dictionary, "First") && has(&dictionary, "InFirst") && !has(&dictionary, "InSecond"));
    apply_theme_unit(WHOLE_PARTS, 1, 1, &context, &dictionary).expect("the second run");
    assert!(has(&dictionary, "Second") && has(&dictionary, "InSecond"));

    let dictionary = ResourceDictionary::new();
    let context = context_of(&dictionary);
    apply_theme_region(SELECTED_PARTS, &[(1, 0), (1, 1)], &context, &dictionary).expect("the region");
    assert!(has(&dictionary, "First") && has(&dictionary, "Second") && has(&dictionary, "InSecond"));
    assert!(!has(&dictionary, "InFirst"));

    assert!(parts_width(WHOLE_PARTS) > parts_width(SELECTED_PARTS));
}
