//! Tests of the runs of the units of a compiled theme ([`super`]): bodies as the emitter writes
//! them, with the markers of the entries of units, and what is taken out of them.

use super::*;

const DICTIONARY: &str = "::ferroui_base::Ref<::ferroui_base::controls::ResourceDictionary>";

/// The lines of a body as the emitter writes them, each indented once.
fn body(statements: &[&str]) -> Vec<String> {
    statements.iter().map(|statement| format!("    {statement}")).collect()
}

fn begin(unit: u32) -> String {
    begin_marker(unit).trim_start().to_string()
}

fn units() -> UnitEmission {
    let names = ["ambient", "Controls/Button.xaml", "Resources/Base.xaml"].iter().map(|name| name.to_string()).collect();
    UnitEmission::new(Rc::new(MergedEntryOrigins::default()), HashMap::new(), names)
}

fn local_types() -> HashMap<String, String> {
    let mut types = HashMap::new();
    types.insert("resource_dictionary_0".to_string(), DICTIONARY.to_string());
    types.insert("resource_dictionary_1".to_string(), DICTIONARY.to_string());
    types.insert("styles_0".to_string(), "::ferroui_base::Ref<::ferroui_base::styling::Styles>".to_string());
    types
}

fn extract(lines: Vec<String>, units: &UnitEmission) -> (Vec<String>, Vec<String>) {
    extract_runs("build_t_xaml", "t.xaml", lines, &local_types(), DICTIONARY, units)
}

#[test]
fn markers_are_recognised() {
    assert_eq!(unit_marker(&begin_marker(7)), Some(Some(7)));
    assert_eq!(unit_marker(&format!("    {END_MARKER}")), Some(None));
    assert_eq!(unit_marker("    // t.xaml(1,2) Content (resource)"), None);
    assert_eq!(unit_marker("    //\u{1}unit x"), None);
}

#[test]
fn the_entries_of_a_unit_become_runs_called_where_they_were() {
    let (button, base) = (begin(1), begin(2));
    let lines = body(&[
        "let resource_dictionary_0 = ::ferroui_base::controls::ResourceDictionary::new();",
        &button,
        "// t.xaml(3,4) Content (resource)",
        "let deferred_0 = rt::defer(::ferroui_base::data::core::ValueType::object(), &context, build_t_xaml_deferred_0, 3, 4)?;",
        "rt::invoked(::ferroui_base::controls::ResourceDictionary::__markup_AddDeferred_1(&resource_dictionary_0, rt::boxed(T), deferred_0), 3, 4)?;",
        END_MARKER,
        &button,
        "// t.xaml(5,4) Content (resource)",
        "rt::add_deferred_resource(&resource_dictionary_0, \"AccentButtonStyle\", ::ferroui_base::data::core::ValueType::object(), &context, build_t_xaml_deferred_1, 5, 4)?;",
        END_MARKER,
        "// t.xaml(1,1) Content (resource)",
        &base,
        "// t.xaml(2,4) Content (resource)",
        "rt::add_double_resources(&resource_dictionary_0, BUILD_T_XAML_ROWS_0)?;",
        END_MARKER,
        "let namespace_scope_0 = rt::push_namespace_scope(&context, TABLE);",
        &base,
        "rt::add_double_resources(&resource_dictionary_0, BUILD_T_XAML_ROWS_1)?;",
        END_MARKER,
        "::core::mem::drop(namespace_scope_0);",
        "::core::result::Result::Ok(styles_0)",
    ]);
    let units = units();
    let (written, functions) = extract(lines, &units);
    assert_eq!(written, body(&[
        "let resource_dictionary_0 = ::ferroui_base::controls::ResourceDictionary::new();",
        "rt::apply_theme_unit(active_theme_units(), 1, 0, &context, &resource_dictionary_0)?;",
        // A comment between the runs belongs to what follows it.
        "// t.xaml(1,1) Content (resource)",
        "rt::apply_theme_unit(active_theme_units(), 2, 0, &context, &resource_dictionary_0)?;",
        "let namespace_scope_0 = rt::push_namespace_scope(&context, TABLE);",
        "rt::apply_theme_unit(active_theme_units(), 2, 1, &context, &resource_dictionary_0)?;",
        "::core::mem::drop(namespace_scope_0);",
        "::core::result::Result::Ok(styles_0)",
    ]));
    assert_eq!(functions.len(), 3);
    assert_eq!(
        functions[0],
        "/// Run 0 of `build_t_xaml` (`t.xaml`): entries of the unit `Controls/Button.xaml` (#568).\n\
         fn build_t_xaml_unit_0(\n    \
         context: &::std::rc::Rc<::ferroui_markup_xaml::xaml_il::runtime::XamlIlContext>,\n    \
         dictionary: &::ferroui_base::Ref<::ferroui_base::controls::ResourceDictionary>,\n\
         ) -> ::core::result::Result<(), ::ferroui_markup_xaml::XamlLoadError> {\n    \
         let context = ::core::clone::Clone::clone(context);\n    \
         let resource_dictionary_0 = ::core::clone::Clone::clone(dictionary);\n    \
         // t.xaml(3,4) Content (resource)\n    \
         let deferred_0 = rt::defer(::ferroui_base::data::core::ValueType::object(), &context, build_t_xaml_deferred_0, 3, 4)?;\n    \
         rt::invoked(::ferroui_base::controls::ResourceDictionary::__markup_AddDeferred_1(&resource_dictionary_0, rt::boxed(T), deferred_0), 3, 4)?;\n    \
         // t.xaml(5,4) Content (resource)\n    \
         rt::add_deferred_resource(&resource_dictionary_0, \"AccentButtonStyle\", ::ferroui_base::data::core::ValueType::object(), &context, build_t_xaml_deferred_1, 5, 4)?;\n    \
         ::core::result::Result::Ok(())\n\
         }\n"
    );
    // A run that does not use the context does not take it.
    assert!(functions[1].contains("    _context: &::std::rc::Rc<"), "{}", functions[1]);
    assert!(!functions[1].contains("let context"), "{}", functions[1]);
    assert_eq!(units.runs(), [vec![], vec!["build_t_xaml_unit_0".to_string()], vec!["build_t_xaml_unit_1".to_string(), "build_t_xaml_unit_2".to_string()]]);
}

#[test]
fn a_run_ends_at_another_dictionary_and_at_another_unit() {
    let (button, base) = (begin(1), begin(2));
    let lines = body(&[
        "let resource_dictionary_0 = ::ferroui_base::controls::ResourceDictionary::new();",
        "let resource_dictionary_1 = ::ferroui_base::controls::ResourceDictionary::new();",
        &button,
        "rt::add_double_resources(&resource_dictionary_0, ROWS_0)?;",
        END_MARKER,
        &button,
        "rt::add_double_resources(&resource_dictionary_1, ROWS_1)?;",
        END_MARKER,
        &base,
        "rt::add_double_resources(&resource_dictionary_1, ROWS_2)?;",
        END_MARKER,
        &base,
        "rt::add_double_resources(&resource_dictionary_1, ROWS_3)?;",
        END_MARKER,
    ]);
    let units = units();
    let (written, functions) = extract(lines, &units);
    assert_eq!(written, body(&[
        "let resource_dictionary_0 = ::ferroui_base::controls::ResourceDictionary::new();",
        "let resource_dictionary_1 = ::ferroui_base::controls::ResourceDictionary::new();",
        "rt::apply_theme_unit(active_theme_units(), 1, 0, &context, &resource_dictionary_0)?;",
        "rt::apply_theme_unit(active_theme_units(), 1, 1, &context, &resource_dictionary_1)?;",
        "rt::apply_theme_unit(active_theme_units(), 2, 0, &context, &resource_dictionary_1)?;",
    ]));
    assert_eq!(functions.len(), 3);
    assert!(functions[2].contains("ROWS_2)?;\n    rt::add_double_resources(&resource_dictionary_1, ROWS_3)?;"), "{}", functions[2]);
}

#[test]
fn an_entry_that_cannot_be_a_run_stays_in_the_body_without_its_markers() {
    let button = begin(1);
    let lines = body(&[
        "let resource_dictionary_0 = ::ferroui_base::controls::ResourceDictionary::new();",
        "let resource_dictionary_1 = ::ferroui_base::controls::ResourceDictionary::new();",
        // It names a local of the body that is not a dictionary.
        &button,
        "rt::register_name(&name_scope, \"Name\", &resource_dictionary_0)?;",
        END_MARKER,
        // It declares a local the body names after it.
        &button,
        "let dictionary_0 = resource_dictionary_0.clone();",
        END_MARKER,
        "use_dictionary(&dictionary_0);",
        // It is inside a block.
        "if ready {",
        &button,
        "rt::add_double_resources(&resource_dictionary_0, ROWS_0)?;",
        END_MARKER,
        "}",
        // It names two dictionaries.
        &button,
        "rt::copy(&resource_dictionary_0, &resource_dictionary_1)?;",
        END_MARKER,
    ]);
    let units = units();
    let (written, functions) = extract(lines, &units);
    assert_eq!(written, body(&[
        "let resource_dictionary_0 = ::ferroui_base::controls::ResourceDictionary::new();",
        "let resource_dictionary_1 = ::ferroui_base::controls::ResourceDictionary::new();",
        "rt::register_name(&name_scope, \"Name\", &resource_dictionary_0)?;",
        "let dictionary_0 = resource_dictionary_0.clone();",
        "use_dictionary(&dictionary_0);",
        "if ready {",
        "rt::add_double_resources(&resource_dictionary_0, ROWS_0)?;",
        "}",
        "rt::copy(&resource_dictionary_0, &resource_dictionary_1)?;",
    ]));
    assert!(functions.is_empty());
    assert!(units.runs().iter().all(Vec::is_empty));
}

#[test]
fn a_body_without_markers_is_unchanged() {
    let lines = body(&["let resource_dictionary_0 = ::ferroui_base::controls::ResourceDictionary::new();", "::core::result::Result::Ok(())"]);
    let units = units();
    let (written, functions) = extract(lines.clone(), &units);
    assert_eq!(written, lines);
    assert!(functions.is_empty());
}

#[test]
fn the_table_lists_every_unit_with_its_runs() {
    let units = units();
    units.push_run(2, "populate_t_xaml_unit_0".to_string());
    units.push_run(2, "populate_t_xaml_unit_3".to_string());
    let type_keys = vec![Vec::new(), vec!["Button".to_string()], Vec::new()];
    let table = units.table("Tests.Theme", &type_keys);
    assert!(
        table.starts_with(
            "/// The unit `ambient` of the theme (#568): its runs, in the order the builds apply them.\n\
             pub static THEME_UNIT_0: rt::ThemeUnit = rt::ThemeUnit { name: \"ambient\", index: 0, runs: &[] };\n\n\
             /// The unit `Controls/Button.xaml` of the theme (#568): its runs, in the order the builds apply them.\n\
             pub static THEME_UNIT_1: rt::ThemeUnit = rt::ThemeUnit { name: \"Controls/Button.xaml\", index: 1, runs: &[] };\n\n\
             /// The unit `Resources/Base.xaml` of the theme (#568): its runs, in the order the builds apply them.\n\
             pub static THEME_UNIT_2: rt::ThemeUnit = rt::ThemeUnit { name: \"Resources/Base.xaml\", index: 2, runs: &[populate_t_xaml_unit_0, populate_t_xaml_unit_3] };\n\n"
        ),
        "{table}"
    );
    assert!(table.contains("pub static THEME_UNITS: &[&rt::ThemeUnit] = &[&THEME_UNIT_0, &THEME_UNIT_1, &THEME_UNIT_2];\n"), "{table}");
    // Step 3: the ambient unit alone, the class, the active table, the registered units (all of
    // them with the tables of types) and the type keys of the diagnostic.
    assert!(table.contains("pub static AMBIENT_THEME_UNITS: &[&rt::ThemeUnit] = &[&THEME_UNIT_0];\n"), "{table}");
    assert!(table.contains("pub const THEME_CLASS: &str = \"Tests.Theme\";\n"), "{table}");
    assert!(table.contains("fn active_theme_units() -> rt::ThemeParts {\n    rt::active_theme_units(THEME_CLASS)\n}\n"), "{table}");
    assert!(table.contains("#[cfg(ferroui_full_themes)]\npub fn registered_theme_units() -> rt::ThemeParts {\n    THEME_UNITS\n}\n"), "{table}");
    assert!(
        table.contains("#[cfg(not(ferroui_full_themes))]\npub fn registered_theme_units() -> rt::ThemeParts {\n    rt::registered_theme_parts(THEME_CLASS).unwrap_or(AMBIENT_THEME_UNITS)\n}\n"),
        "{table}"
    );
    assert!(table.ends_with("#[cfg(debug_assertions)]\npub static THEME_UNIT_TYPE_KEYS: &[&[&str]] = &[&[], &[\"Button\"], &[]];\n"), "{table}");
}

/// #853: the plan is off by default, and the emission of a closure can choose another one.
#[test]
fn the_plan_of_the_runs_is_off_by_default() {
    assert_eq!(DEFAULT_THEME_UNIT_PLAN, ThemeUnitPlan::OFF);
    assert_eq!(ThemeUnitPlan::current(), DEFAULT_THEME_UNIT_PLAN);
    let inner = with_theme_unit_plan(ThemeUnitPlan::ON, || {
        let nested = with_theme_unit_plan(ThemeUnitPlan::OFF.with_regions(true), ThemeUnitPlan::current);
        (ThemeUnitPlan::current(), nested)
    });
    assert_eq!(inner, (ThemeUnitPlan::ON, ThemeUnitPlan::OFF.with_regions(true)));
    assert!(inner.1.regions() && !inner.1.keys_in_runs());
    assert_eq!(ThemeUnitPlan::current(), DEFAULT_THEME_UNIT_PLAN);
}

/// The units of the region tests: the ambient unit and three documents.
fn four_units() -> UnitEmission {
    let names = ["ambient", "Controls/Button.xaml", "Resources/Base.xaml", "Controls/List.xaml"];
    UnitEmission::new(Rc::new(MergedEntryOrigins::default()), HashMap::new(), names.iter().map(|name| name.to_string()).collect())
}

/// The body of the region tests: runs of three units into one dictionary, then one into
/// another, a call alone after an ordinary statement.
fn region_body() -> Vec<String> {
    let (button, base, list) = (begin(1), begin(2), begin(3));
    body(&[
        "let resource_dictionary_0 = ::ferroui_base::controls::ResourceDictionary::new();",
        "let resource_dictionary_1 = ::ferroui_base::controls::ResourceDictionary::new();",
        &base,
        "rt::add_double_resources(&resource_dictionary_0, ROWS_0)?;",
        END_MARKER,
        "// t.xaml(4,4) Content (resource)",
        &button,
        "rt::add_double_resources(&resource_dictionary_0, ROWS_1)?;",
        END_MARKER,
        &list,
        "rt::add_double_resources(&resource_dictionary_0, ROWS_2)?;",
        END_MARKER,
        &button,
        "rt::add_double_resources(&resource_dictionary_1, ROWS_3)?;",
        END_MARKER,
        &list,
        "rt::add_double_resources(&resource_dictionary_1, ROWS_4)?;",
        END_MARKER,
        "let value_0 = ::ferroui_base::styling::ThemeVariant::__markup_static_get_Dark();",
        &base,
        "rt::add_double_resources(&resource_dictionary_1, ROWS_5)?;",
        END_MARKER,
        "::core::result::Result::Ok(())",
    ])
}

/// #853: with regions, consecutive calls of runs into one dictionary are one call of
/// `rt::apply_theme_region` with the pairs in their order, after the comments between them; a
/// call alone stays a call of `rt::apply_theme_unit`. The runs are the same functions.
#[test]
fn consecutive_runs_into_one_dictionary_are_one_region() {
    let units = four_units();
    let plan = ThemeUnitPlan::OFF.with_regions(true);
    let (written, functions) = with_theme_unit_plan(plan, || extract(region_body(), &units));
    assert_eq!(written, body(&[
        "let resource_dictionary_0 = ::ferroui_base::controls::ResourceDictionary::new();",
        "let resource_dictionary_1 = ::ferroui_base::controls::ResourceDictionary::new();",
        "// t.xaml(4,4) Content (resource)",
        "rt::apply_theme_region(active_theme_units(), &[(2, 0), (1, 0), (3, 0)], &context, &resource_dictionary_0)?;",
        "rt::apply_theme_region(active_theme_units(), &[(1, 1), (3, 1)], &context, &resource_dictionary_1)?;",
        "let value_0 = ::ferroui_base::styling::ThemeVariant::__markup_static_get_Dark();",
        "rt::apply_theme_unit(active_theme_units(), 2, 1, &context, &resource_dictionary_1)?;",
        "::core::result::Result::Ok(())",
    ]));
    let units_after = units.runs();
    // The same runs as without regions.
    let plain = four_units();
    let (unchanged, plain_functions) = extract(region_body(), &plain);
    assert_eq!(functions, plain_functions);
    assert_eq!(units_after, plain.runs());
    assert_eq!(unchanged.iter().filter(|line| line.contains("rt::apply_theme_unit(")).count(), 6);
    assert!(!unchanged.iter().any(|line| line.contains("apply_theme_region")), "{unchanged:?}");
}

/// #853: a region needs its indices in 16 bits; a unit beyond them keeps its calls.
#[test]
fn a_region_with_an_index_beyond_sixteen_bits_keeps_its_calls() {
    let names: Vec<String> = (0..=70_000).map(|index| format!("unit {index}")).collect();
    let units = UnitEmission::new(Rc::new(MergedEntryOrigins::default()), HashMap::new(), names);
    let lines = body(&[
        "let resource_dictionary_0 = ::ferroui_base::controls::ResourceDictionary::new();",
        &begin(1),
        "rt::add_double_resources(&resource_dictionary_0, ROWS_0)?;",
        END_MARKER,
        &begin(70_000),
        "rt::add_double_resources(&resource_dictionary_0, ROWS_1)?;",
        END_MARKER,
    ]);
    let (written, _) = with_theme_unit_plan(ThemeUnitPlan::ON, || extract(lines, &units));
    assert_eq!(written, body(&[
        "let resource_dictionary_0 = ::ferroui_base::controls::ResourceDictionary::new();",
        "rt::apply_theme_unit(active_theme_units(), 1, 0, &context, &resource_dictionary_0)?;",
        "rt::apply_theme_unit(active_theme_units(), 70000, 0, &context, &resource_dictionary_0)?;",
    ]));
}

/// #853: with `keys_in_runs`, the emitter writes the key a merged theme dictionary assigns again
/// between the markers of its unit, and the assignment joins the run of the entries of the unit
/// that follow it: the runs of the units then follow each other and are one region.
#[test]
fn a_key_assigned_again_joins_the_run_of_its_unit() {
    let (button, base) = (begin(1), begin(2));
    let set_key = |value: &str| {
        format!(
            "<dyn ::ferroui_base::controls::IThemeVariantProvider>::__markup_set_Key(\
             &rt::cast(resource_dictionary_1.clone(), 14, 6)?, rt::cast(::core::clone::Clone::clone(&{value}), 14, 6)?);"
        )
    };
    let (outside, first, second) = (set_key("value_0"), set_key("value_1"), set_key("value_2"));
    let lines = body(&[
        "let resource_dictionary_1 = ::ferroui_base::controls::ResourceDictionary::new();",
        // The key of the dictionary, outside every unit.
        "let value_0 = ::ferroui_base::styling::ThemeVariant::__markup_static_get_Default();",
        &outside,
        &base,
        "// Base.xaml(14,6) Key",
        "let value_1 = ::ferroui_base::styling::ThemeVariant::__markup_static_get_Default();",
        &first,
        END_MARKER,
        &base,
        "rt::add_double_resources(&resource_dictionary_1, ROWS_0)?;",
        END_MARKER,
        &button,
        "// Button.xaml(14,6) Key",
        "let value_2 = ::ferroui_base::styling::ThemeVariant::__markup_static_get_Default();",
        &second,
        END_MARKER,
        &button,
        "rt::add_double_resources(&resource_dictionary_1, ROWS_1)?;",
        END_MARKER,
    ]);
    let units = units();
    let (written, functions) = with_theme_unit_plan(ThemeUnitPlan::ON, || extract(lines, &units));
    let region = "rt::apply_theme_region(active_theme_units(), &[(2, 0), (1, 0)], &context, &resource_dictionary_1)?;";
    assert_eq!(written[3..], body(&[region]));
    assert_eq!(functions.len(), 2);
    let joined = format!(
        "    let value_1 = ::ferroui_base::styling::ThemeVariant::__markup_static_get_Default();\n    {first}\n    \
         rt::add_double_resources(&resource_dictionary_1, ROWS_0)?;\n"
    );
    assert!(functions[0].contains(&joined), "{}", functions[0]);
    assert!(functions[1].contains(&second), "{}", functions[1]);
}
