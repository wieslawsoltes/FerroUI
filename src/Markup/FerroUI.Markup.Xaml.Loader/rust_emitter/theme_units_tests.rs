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
    extract_runs("build_t_xaml", "t.xaml", lines, &local_types(), DICTIONARY, units, None)
}

#[test]
fn markers_are_recognised() {
    assert_eq!(unit_marker(&begin_marker(7)), Some(Some(7)));
    assert_eq!(unit_marker(&format!("    {END_MARKER}")), Some(None));
    assert_eq!(unit_marker("    // t.xaml(1,2) Content (resource)"), None);
    assert_eq!(unit_marker("    //\u{1}unit x"), None);
}

#[test]
fn a_complete_run_encodes_rows_then_deferred_keys_with_their_original_positions() {
    use ferroui_markup::theme_run_opcodes as op;
    let image = super::super::BytecodeImage::packed();
    let button = begin(1);
    let lines = body(&[
        "let resource_dictionary_0 = ::ferroui_base::controls::ResourceDictionary::new();",
        &button,
        "ROWS_0.add_thickness_resources(&resource_dictionary_0)?;",
        r#"rt::add_deferred_resource(&resource_dictionary_0, "Q\".add_\\x", ::ferroui_base::data::core::ValueType::object(), &context, factory, 12, 49)?;"#,
        "let deferred = rt::defer(::ferroui_base::data::core::ValueType::object(), &context, factory, 2, 3)?;",
        "rt::invoked(::ferroui_base::controls::ResourceDictionary::__markup_AddDeferred_1(&resource_dictionary_0, rt::boxed(<::ferroui_controls::Button as ::ferroui_base::StaticType>::TYPE), ::core::clone::Clone::clone(&deferred) as ::std::rc::Rc<dyn ::ferroui_base::controls::IDeferredContent>), 6, 7)?;",
        END_MARKER,
    ]);
    let (_, native) = extract(lines.clone(), &units());
    assert!(native[0].contains("ROWS_0.add_thickness_resources("));
    let rows = HashMap::from([("ROWS_0".to_owned(), super::super::theme_rows::RowKind::Thickness)]);
    let config = Schedule { image: &image, document: "DOCUMENT", rows: &rows };
    let (_, runs) = extract_runs("build_t", "t.xaml", lines, &local_types(), DICTIONARY, &units(), Some(config));
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].matches("rt::apply_theme_run(").count(), 1);
    assert!(runs[0].contains(r#"ThemeRunImport::Text("Q\".add_\\x")"#));
    assert_eq!(runs[0].matches("ThemeRunImport::Factory(factory)").count(), 1);
    assert_eq!(image.artifact("t.fxc").unwrap().0,
        [op::VERSION, op::THICKNESS, 0, op::DEFER_TEXT, 1, 2, 24, 98, op::DEFER_TYPE, 3, 2, 4, 6, 12, 14]);
}

#[test]
fn a_changed_call_keeps_the_entire_native_run_and_emits_no_partial_program() {
    let image = super::super::BytecodeImage::packed();
    let button = begin(1);
    let lines = body(&[
        "let resource_dictionary_0 = ::ferroui_base::controls::ResourceDictionary::new();",
        &button,
        "ROWS_0.add_double_resources(&resource_dictionary_0)?;",
        "ROWS_1.add_double_resources(&resource_dictionary_0, &context)?;",
        END_MARKER,
    ]);
    let native = extract(lines.clone(), &units());
    let rows = HashMap::from([("ROWS_0".to_owned(), super::super::theme_rows::RowKind::Double)]);
    let config = Schedule { image: &image, document: "DOCUMENT", rows: &rows };
    for changed in ["UNRESOLVED.add_double_resources(&resource_dictionary_0)?;",
                    "ROWS_0.add_color_resources(&resource_dictionary_0)?;",
                    "ROWS_0.add_double_resources(&resource_dictionary_0, &context)?;"] {
        assert!(bytecode::compile(&body(&[changed]), "resource_dictionary_0", &config).is_none());
    }
    let compiled = extract_runs("build_t_xaml", "t.xaml", lines, &local_types(), DICTIONARY, &units(), Some(config));
    assert_eq!(compiled, native);
    assert!(image.artifact("t.fxc").is_none());
}

#[test]
fn grid_length_operands_keep_scientific_literals_and_reject_an_unresolved_value() {
    use ferroui_markup::{compiled_operands as wire, theme_run_opcodes as op};
    let image = super::super::BytecodeImage::packed();
    let addition = |amount| format!("rt::invoked(<dyn ::ferroui_base::controls::IResourceDictionary>::__markup_Add_0(&rt::cast(dict.clone(), 4, 9)?, rt::string_object(\"Width\"), rt::to_object(::ferroui_controls::GridLength::__markup_new_2({amount}, ::ferroui_controls::GridUnitType::Pixel))), 4, 9)?;");
    let config = Schedule { image: &image, document: "DOC", rows: &HashMap::new() };
    assert!(bytecode::compile(&[addition("amount")], "dict", &config).is_none());
    assert!(bytecode::compile(&[addition("-1.25e-9_f64")], "dict", &config).is_some());
    let (bytes, _) = image.artifact("t.fxc").unwrap();
    assert_eq!(&bytes[..6], &[op::VERSION, op::FROM_DOUBLE, 0, 8, 18, 1]);
    let mut operand = &bytes[6..];
    assert_eq!(wire::read_number(&mut operand).unwrap(), -1.25e-9);
    assert!(operand.is_empty());
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
             pub static THEME_UNIT_0: rt::ThemeUnit = rt::ThemeUnit { name: \"ambient\", index: 0, runs: &[], entries: &[] };\n\n\
             /// The unit `Controls/Button.xaml` of the theme (#568): its runs, in the order the builds apply them.\n\
             pub static THEME_UNIT_1: rt::ThemeUnit = rt::ThemeUnit { name: \"Controls/Button.xaml\", index: 1, runs: &[], entries: &[] };\n\n\
             /// The unit `Resources/Base.xaml` of the theme (#568): its runs, in the order the builds apply them.\n\
             pub static THEME_UNIT_2: rt::ThemeUnit = rt::ThemeUnit { name: \"Resources/Base.xaml\", index: 2, runs: &[populate_t_xaml_unit_0, populate_t_xaml_unit_3], entries: &[] };\n\n"
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
    assert!(!table.contains("THEME_UNIT_TEXT_KEYS"), "{table}");
    assert!(table.contains("#[cfg(debug_assertions)]\npub static THEME_UNIT_ENTRY_KEYS: &[&[&str]] = &[&[], &[], &[]];\n"), "{table}");
    assert!(table.contains("pub fn trimmed_theme_keys(parts: rt::ThemeParts) -> rt::TrimmedKeys {\n"), "{table}");
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

/// #852, step 2: with entries selected by key, a deferred resource that is the only statement of
/// its run naming its key leaves the run for a static the unit names, with the runs in a static
/// an application can name; a key another statement names stays in the run. The graph of the unit
/// gets the number of each static.
#[test]
fn the_deferred_resources_of_a_run_are_entries_selected_by_key() {
    let add = |key: &str, build: &str, line: i32| {
        format!("rt::add_deferred_resource(&resource_dictionary_1, \"{key}\", ::ferroui_base::data::core::ValueType::object(), &context, {build}, {line}, 8)?;")
    };
    let lines = body(&[
        &begin(2),
        "// t.xaml(3,8) Content (resource)",
        &add("One", "build_t_xaml_deferred_0", 3),
        "// t.xaml(4,8) Content (resource)",
        &add("Two", "build_t_xaml_deferred_1", 4),
        "// t.xaml(5,8) Content (resource)",
        "rt::replace_value(&resource_dictionary_1, \"Two\")?;",
        END_MARKER,
    ]);
    let declared = body(&["let resource_dictionary_1 = make();"]);
    let lines: Vec<String> = declared.into_iter().chain(lines).collect();
    let units = units();
    let (_, functions) = with_theme_unit_plan(ThemeUnitPlan::OFF.with_entries(true), || extract(lines, &units));
    assert_eq!(functions.len(), 1);
    assert!(!functions[0].contains("\"One\"") && !functions[0].contains("t.xaml(3,8)"), "{}", functions[0]);
    assert!(functions[0].contains("\"Two\", ::ferroui_base") && functions[0].contains("t.xaml(4,8)"), "{}", functions[0]);
    let none = [Vec::new(), Vec::new(), Vec::new()];
    let table = units.table("Tests.Theme", &none);
    assert!(table.contains("THEME_UNIT_ENTRY_KEYS: &[&[&str]] = &[&[], &[], &[\"One\"]];"), "{table}");
    assert!(
        table.contains(
            "pub static THEME_ENTRY_0: rt::ThemeEntry = rt::ThemeEntry { run: 0, key: \"One\", build: build_t_xaml_deferred_0, line: 3, position: 8 };"
        ),
        "{table}"
    );
    assert!(table.contains("pub static THEME_UNIT_2_RUNS: [rt::ThemeUnitRun; 1] = [build_t_xaml_unit_0];"), "{table}");
    assert!(table.contains("index: 2, runs: &THEME_UNIT_2_RUNS, entries: &[&THEME_ENTRY_0] };"), "{table}");
    let mut graph = vec![ThemeUnitModel::default(), ThemeUnitModel::default(), ThemeUnitModel {
        references: vec!["Other".to_string()],
        entries: vec![ThemeEntryModel { key: "Two".to_string(), ..ThemeEntryModel::default() }],
        ..ThemeUnitModel::default()
    }];
    units.record_slots(&mut graph);
    let keys: Vec<(&str, &[u32])> = graph[2].entries.iter().map(|entry| (entry.key.as_str(), entry.slots.as_slice())).collect();
    assert_eq!(keys, [("One", &[0u32][..]), ("Two", &[][..])]);
    // A key the graph did not list keeps what the unit names.
    assert_eq!(graph[2].entries[0].references, ["Other"]);
}
