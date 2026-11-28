//! Tests of the rows of theme resources ([`super`]): the shapes recognised in the statements
//! the emitter writes (taken from the generated theme files before #568), and the runs written
//! as row tables.

use super::*;

const BRUSH: &str = "::ferroui_base::media::SolidColorBrush";
const EXTENSION: &str = "::ferroui_markup_xaml::markup_extensions::StaticResourceExtension";
const ADD: &str = "rt::invoked(<dyn ::ferroui_base::controls::IResourceDictionary>::__markup_Add_0(&rt::cast(";

fn paths() -> RowPaths {
    RowPaths {
        resource_dictionary_contract: "::ferroui_base::controls::IResourceDictionary".to_string(),
        solid_color_brush: BRUSH.to_string(),
        color: "::ferroui_base::media::Color".to_string(),
        thickness: "::ferroui_base::Thickness".to_string(),
        corner_radius: "::ferroui_base::CornerRadius".to_string(),
        static_resource_extension: EXTENSION.to_string(),
    }
}

/// The lines of a body as the emitter writes them, each indented once.
fn body(statements: &[&str]) -> Vec<String> {
    statements.iter().map(|statement| format!("    {statement}")).collect()
}

const BRUSH_RETURNED: &str = "rt::to_value(solid_color_brush_0.clone())";

fn alias_body() -> Vec<String> {
    body(&[
        &format!("let static_resource_extension_0 = {EXTENSION}::__markup_new_0();"),
        "// WinUITheme.xaml(184,75) ResourceKey",
        &format!("{EXTENSION}::__markup_set_ResourceKey(&static_resource_extension_0, rt::string_object(\"SystemControlPageTextBaseMediumBrush\"));"),
        &format!("let provided_0 = rt::invoked({EXTENSION}::__markup_ProvideValue_0(&static_resource_extension_0, rt::service_provider(&context)), 184, 8)?;"),
    ])
}

fn literal_brush_body(argb: &str) -> Vec<String> {
    body(&[
        "// WinUITheme.xaml(118,8) SolidColorBrush",
        &format!("let solid_color_brush_0 = {BRUSH}::new();"),
        "// WinUITheme.xaml(118,8) Color",
        &format!("let value_0 = ::ferroui_base::media::Color::__markup_FromUInt32_2({argb});"),
        &format!("solid_color_brush_0.set_value({BRUSH}::color_property(), ::core::clone::Clone::clone(&value_0));"),
    ])
}

fn dynamic_brush_body(property: &str) -> Vec<String> {
    body(&[
        "// WinUITheme.xaml(90,8) SolidColorBrush",
        &format!("let solid_color_brush_0 = {BRUSH}::new();"),
        "context.push_parent(rt::to_value(solid_color_brush_0.clone()));",
        "// WinUITheme.xaml(90,67) Color",
        &format!("rt::bind_dynamic_resource(&context, solid_color_brush_0.upcast_ref::<::ferroui_base::FerroObject>(), {BRUSH}::{property}(), \"SystemAccentColor\", 90, 67)?;"),
        "context.pop_parent();",
    ])
}

fn static_brush_body(extra: Option<&str>) -> Vec<String> {
    let target = "solid_color_brush_0.upcast_ref::<::ferroui_base::FerroObject>()";
    let property = format!("{BRUSH}::color_property()");
    let mut statements = vec![
        "// WinUITheme.xaml(91,8) SolidColorBrush".to_string(),
        format!("let solid_color_brush_0 = {BRUSH}::new();"),
        "context.push_parent(rt::to_value(solid_color_brush_0.clone()));".to_string(),
        "// WinUITheme.xaml(91,68) Color (setter chosen at run time)".to_string(),
        format!("let static_resource_extension_0 = {EXTENSION}::__markup_new_1(rt::string_object(\"SystemAltHighColor\"));"),
        format!("let provided_0 = rt::provide_value_invoked(&context, rt::property_value({property}), &static_resource_extension_0, {EXTENSION}::__markup_ProvideValue_0, 91, 68)?;"),
        "let value_0: ::ferroui_base::metadata::MarkupValue = rt::to_value(provided_0);".to_string(),
        "if rt::is_instance(&value_0, rt::markup_handle(<::ferroui_base::UnsetValueType as ::ferroui_base::metadata::MarkupTyped>::MARKUP)) {".to_string(),
        format!("    rt::unset_value({target}, {property});"),
        "} else if rt::is_instance(&value_0, rt::markup_handle(<dyn ::ferroui_base::data::BindingBase as ::ferroui_base::metadata::MarkupTyped>::MARKUP)) {".to_string(),
        format!("    rt::bind({target}, {property}, value_0.clone(), 91, 68)?;"),
        "} else if rt::is_instance(&value_0, rt::markup_handle(<::ferroui_base::media::Color as ::ferroui_base::metadata::MarkupTyped>::MARKUP)) {".to_string(),
        format!("    solid_color_brush_0.set_value({property}, rt::exact(value_0.clone(), \"FerroUI.Media.SolidColorBrush.set_Color\", 0, 91, 68)?);"),
        "} else {".to_string(),
        "    return ::core::result::Result::Err(rt::no_setter(\"Color\", &value_0, 91, 68));".to_string(),
        "}".to_string(),
    ];
    statements.extend(extra.map(str::to_string));
    statements.push("context.pop_parent();".to_string());
    statements.iter().map(|statement| format!("    {statement}")).collect()
}

/// The brush whose colour is a static resource as the emitter writes it since #568, step 5: the
/// choice of the setter is one call of `rt::assign_resource_value`.
fn static_brush_assigned_body() -> Vec<String> {
    let target = "solid_color_brush_0.upcast_ref::<::ferroui_base::FerroObject>()";
    let property = format!("{BRUSH}::color_property()");
    body(&[
        "// WinUITheme.xaml(91,8) SolidColorBrush",
        &format!("let solid_color_brush_0 = {BRUSH}::new();"),
        "context.push_parent(rt::to_value(solid_color_brush_0.clone()));",
        "// WinUITheme.xaml(91,68) Color (setter chosen at run time)",
        &format!("let static_resource_extension_0 = {EXTENSION}::__markup_new_1(rt::string_object(\"SystemAltHighColor\"));"),
        &format!("let provided_0 = rt::provide_value_invoked(&context, rt::property_value({property}), &static_resource_extension_0, {EXTENSION}::__markup_ProvideValue_0, 91, 68)?;"),
        "let value_0: ::ferroui_base::metadata::MarkupValue = rt::to_value(provided_0);",
        &format!("rt::assign_resource_value({target}, {property}, value_0, &rt::ResourceChoice {{ name: \"Color\", member: \"FerroUI.Media.SolidColorBrush.set_Color\", index: 0, unset: true, instance: rt::ValueHandle::Markup(<::ferroui_base::media::Color as ::ferroui_base::metadata::MarkupTyped>::MARKUP), null: false, line: 91, position: 68 }}, ::core::option::Option::None)?;"),
        "context.pop_parent();",
    ])
}

#[test]
fn a_static_brush_with_the_shared_setter_choice_is_a_row() {
    let paths = paths();
    let row = deferred_row(&paths, &static_brush_assigned_body(), BRUSH_RETURNED, None).expect("a static brush");
    assert_eq!(row, DeferredRow {
        kind: RowKind::StaticBrush,
        operand: "\"SystemAltHighColor\"".to_string(),
        value_position: (91, 68),
        namespaces: None,
    });
    // The choice of another colour type is not the shape.
    let mut other = static_brush_assigned_body();
    other[7] = other[7].replace("media::Color as", "media::Colors as");
    assert!(deferred_row(&paths, &other, BRUSH_RETURNED, None).is_none());
}

#[test]
fn the_four_deferred_shapes_are_recognised() {
    let paths = paths();
    let alias = deferred_row(&paths, &alias_body(), "rt::to_value(provided_0)", None).expect("an alias");
    assert_eq!(alias, DeferredRow {
        kind: RowKind::ResourceAlias,
        operand: "\"SystemControlPageTextBaseMediumBrush\"".to_string(),
        value_position: (184, 8),
        namespaces: None,
    });
    let literal = deferred_row(&paths, &literal_brush_body("16777215_u32"), BRUSH_RETURNED, Some("TABLE_1")).expect("a literal brush");
    assert_eq!(literal, DeferredRow {
        kind: RowKind::LiteralBrush,
        operand: "16777215_u32".to_string(),
        value_position: (0, 0),
        namespaces: Some("TABLE_1".to_string()),
    });
    let dynamic = deferred_row(&paths, &dynamic_brush_body("color_property"), BRUSH_RETURNED, None).expect("a dynamic brush");
    assert_eq!(dynamic, DeferredRow {
        kind: RowKind::DynamicBrush,
        operand: "\"SystemAccentColor\"".to_string(),
        value_position: (90, 67),
        namespaces: None,
    });
    let static_brush = deferred_row(&paths, &static_brush_body(None), BRUSH_RETURNED, None).expect("a static brush");
    assert_eq!(static_brush, DeferredRow {
        kind: RowKind::StaticBrush,
        operand: "\"SystemAltHighColor\"".to_string(),
        value_position: (91, 68),
        namespaces: None,
    });
}

#[test]
fn a_body_of_another_shape_is_not_a_row() {
    let paths = paths();
    // Another member, another property, another statement, another value returned.
    let opacity = format!("solid_color_brush_0.set_value({BRUSH}::opacity_property(), 0.5_f64);");
    assert!(deferred_row(&paths, &static_brush_body(Some(&opacity)), BRUSH_RETURNED, None).is_none());
    assert!(deferred_row(&paths, &dynamic_brush_body("opacity_property"), BRUSH_RETURNED, None).is_none());
    assert!(deferred_row(&paths, &literal_brush_body("value"), BRUSH_RETURNED, None).is_none());
    assert!(deferred_row(&paths, &literal_brush_body("1_u32"), "::core::option::Option::None", None).is_none());
    assert!(deferred_row(&paths, &alias_body(), "rt::to_value(provided_1)", None).is_none());
    let geometry = body(&["let value_0 = rt::invoked(::ferroui_base::media::StreamGeometry::__markup_parse(::std::string::String::from(\"M0 0\")), 23, 45)?;"]);
    assert!(deferred_row(&paths, &geometry, "rt::to_value(value_0.clone())", None).is_none());
}

#[test]
fn a_fused_addition_is_parsed_and_becomes_a_row() {
    let statement = "rt::add_deferred_resource(&resource_dictionary_1, \"Key, with a comma\", ::ferroui_base::data::core::ValueType::object(), &context, populate_x_deferred_7, 15, 57)?;";
    let addition = deferred_addition(statement).expect("a fused addition");
    assert_eq!(addition, DeferredAddition {
        dictionary: "resource_dictionary_1",
        key: "\"Key, with a comma\"",
        build: "populate_x_deferred_7",
        position: (15, 57),
    });
    let row = DeferredRow { kind: RowKind::StaticBrush, operand: "\"Accent\"".to_string(), value_position: (16, 70), namespaces: Some("TABLE".to_string()) };
    let entry = deferred_row_statement(&addition, &row, 7);
    assert_eq!(entry.kind, RowKind::StaticBrush);
    assert_eq!(entry.dictionary, "resource_dictionary_1");
    assert_eq!(entry.slot, Some(7));
    assert_eq!(
        entry.row,
        "rt::StaticBrushRow { key: \"Key, with a comma\", color_key: \"Accent\", namespaces: ::core::option::Option::Some(TABLE), line: 15, position: 57, value_line: 16, value_position: 70 }"
    );
    // Another result type, a dictionary that is not a local, another context.
    assert!(deferred_addition("rt::add_deferred_resource(&resource_dictionary_1, \"K\", rt::class_handle(T), &context, f, 1, 2)?;").is_none());
    assert!(deferred_addition("rt::add_deferred_resource(dictionary.upcast_ref::<X>(), \"K\", ::ferroui_base::data::core::ValueType::object(), &context, f, 1, 2)?;").is_none());
    assert!(deferred_addition("rt::add_deferred_resource(&resource_dictionary_1, \"K\", ::ferroui_base::data::core::ValueType::object(), &other, f, 1, 2)?;").is_none());
}

fn value_call(dictionary: &str, key: &str, value: &str, line: i32, position: i32) -> String {
    format!("{ADD}{dictionary}.clone(), {line}, {position})?, rt::string_object(\"{key}\"), {value}), {line}, {position})?")
}

#[test]
fn plain_values_of_literal_form_are_recognised() {
    let paths = paths();
    let row = |value: &str, previous: Option<&str>| value_row_statement(&paths, &value_call("resource_dictionary_1", "K", value, 3, 4), previous);
    let double = row("rt::to_object(24.0_f64)", None).expect("a number");
    assert_eq!((double.kind, double.row.as_str()), (RowKind::Double, "rt::ValueRow { key: \"K\", value: 24.0_f64, line: 3, position: 4 }"));
    assert_eq!(double.dictionary, "resource_dictionary_1");
    assert_eq!(row("rt::to_object(::core::primitive::f64::NAN)", None).map(|entry| entry.kind), Some(RowKind::Double));
    let thickness = row("rt::to_object(::ferroui_base::Thickness::__markup_new_3(0.0_f64, 1.5_f64, 0.0_f64, -2.0_f64))", None).expect("a thickness");
    assert_eq!(thickness.kind, RowKind::Thickness);
    assert_eq!(thickness.row, "rt::ValueRow { key: \"K\", value: [0.0_f64, 1.5_f64, 0.0_f64, -2.0_f64], line: 3, position: 4 }");
    let radius = row("rt::to_object(::ferroui_base::CornerRadius::__markup_new_3(4.0_f64, 4.0_f64, 0.0_f64, 0.0_f64))", None).expect("a radius");
    assert_eq!(radius.kind, RowKind::CornerRadius);
    let text = row("rt::string_object(\"0,0,0,1\")", None).expect("a text");
    assert_eq!(text.row, "rt::ValueRow { key: \"K\", value: \"0,0,0,1\", line: 3, position: 4 }");
    let previous = "    let value_2 = ::ferroui_base::media::Color::__markup_FromUInt32_2(4294967295_u32);";
    let color = row("rt::to_object(::core::clone::Clone::clone(&value_2))", Some(previous)).expect("a colour");
    assert_eq!(color.kind, RowKind::Color);
    assert_eq!(color.row, "rt::ValueRow { key: \"K\", value: 4294967295_u32, line: 3, position: 4 }");
    assert_eq!(color.prefix, Some((previous.trim_start().to_string(), "value_2".to_string())));

    // Values without a literal form of the rows, a colour from another local, other positions.
    assert!(row("rt::to_object(::core::clone::Clone::clone(&true))", None).is_none());
    assert!(row("rt::to_object(15_i32)", None).is_none());
    assert!(row("rt::to_object(::ferroui_base::layout::VerticalAlignment::Center)", None).is_none());
    assert!(row("rt::to_object(::ferroui_base::Thickness::__markup_new_3(1.0_f64, 2.0_f64))", None).is_none());
    assert!(row("rt::to_object(::core::clone::Clone::clone(&value_3))", Some(previous)).is_none());
    let moved = format!("{ADD}resource_dictionary_1.clone(), 3, 5)?, rt::string_object(\"K\"), rt::to_object(1.0_f64)), 3, 4)?");
    assert!(value_row_statement(&paths, &moved, None).is_none());
}

/// The row statements of `lines`: the fused additions of the deferred rows `rows` (by the name
/// of the build function) and the additions of plain values, as the emitter records them.
fn recorded(lines: &[String], rows: &[(&str, usize, DeferredRow)]) -> HashMap<String, RowStatement> {
    let paths = paths();
    let mut statements = HashMap::new();
    for (index, line) in lines.iter().enumerate() {
        let text = line.trim_start();
        if let Some(addition) = deferred_addition(text) {
            if let Some((_, slot, row)) = rows.iter().find(|(build, ..)| *build == addition.build) {
                statements.insert(text.to_string(), deferred_row_statement(&addition, row, *slot));
            }
        } else if let Some(call) = text.strip_suffix(';') {
            let previous = index.checked_sub(1).map(|previous| lines[previous].as_str());
            if let Some(entry) = value_row_statement(&paths, call, previous) {
                statements.insert(text.to_string(), entry);
            }
        }
    }
    statements
}

fn alias(target: &str) -> DeferredRow {
    DeferredRow { kind: RowKind::ResourceAlias, operand: format!("\"{target}\""), value_position: (9, 9), namespaces: None }
}

fn deferred_add(dictionary: &str, key: &str, build: &str, line: i32) -> String {
    format!("rt::add_deferred_resource(&{dictionary}, \"{key}\", ::ferroui_base::data::core::ValueType::object(), &context, {build}, {line}, 8)?;")
}

#[test]
fn runs_of_one_shape_and_one_dictionary_become_one_call_over_a_table() {
    let lines = body(&[
        "// t.xaml(1,8) Content (resource)",
        &deferred_add("resource_dictionary_1", "A", "build_t_deferred_0", 1),
        "// t.xaml(2,8) Content (resource)",
        &deferred_add("resource_dictionary_1", "B", "build_t_deferred_1", 2),
        "// t.xaml(3,8) Content (resource)",
        &format!("{};", value_call("resource_dictionary_1", "Width", "rt::to_object(1.0_f64)", 3, 8)),
        "// t.xaml(4,8) Content (resource)",
        "let value_0 = ::ferroui_base::media::Color::__markup_FromUInt32_2(1_u32);",
        &format!("{};", value_call("resource_dictionary_1", "First", "rt::to_object(::core::clone::Clone::clone(&value_0))", 4, 8)),
        "// t.xaml(5,8) Content (resource)",
        "let value_1 = ::ferroui_base::media::Color::__markup_FromUInt32_2(2_u32);",
        &format!("{};", value_call("resource_dictionary_1", "Second", "rt::to_object(::core::clone::Clone::clone(&value_1))", 5, 8)),
        "// t.xaml(6,8) Content (resource)",
        &deferred_add("resource_dictionary_1", "C", "build_t_deferred_2", 6),
        "::core::result::Result::Ok(())",
    ]);
    let statements = recorded(&lines, &[
        ("build_t_deferred_0", 0, alias("X")),
        ("build_t_deferred_1", 1, alias("Y")),
        ("build_t_deferred_2", 2, alias("Z")),
    ]);
    let mut count = 0;
    let (written, tables) = with_packed_theme_rows(false, || row_runs(lines, &statements, "BUILD_T_DOCUMENT", || {
        count += 1;
        format!("BUILD_T_ROWS_{}", count - 1)
    }));
    assert_eq!(written, body(&[
        "// t.xaml(1,8) Content (resource)",
        "rt::add_resource_aliases(&resource_dictionary_1, &context, &BUILD_T_DOCUMENT, BUILD_T_ROWS_0)?;",
        // A single number stays a statement.
        "// t.xaml(3,8) Content (resource)",
        &format!("{};", value_call("resource_dictionary_1", "Width", "rt::to_object(1.0_f64)", 3, 8)),
        "// t.xaml(4,8) Content (resource)",
        "rt::add_color_resources(&resource_dictionary_1, BUILD_T_ROWS_1)?;",
        // A deferred row is a row even alone, after the run of another shape.
        "// t.xaml(6,8) Content (resource)",
        "rt::add_resource_aliases(&resource_dictionary_1, &context, &BUILD_T_DOCUMENT, BUILD_T_ROWS_2)?;",
        "::core::result::Result::Ok(())",
    ]));
    let names: Vec<&str> = tables.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["BUILD_T_ROWS_0", "BUILD_T_ROWS_1", "BUILD_T_ROWS_2"]);
    let (_, aliases) = &tables[0];
    assert_eq!(aliases.kind, RowKind::ResourceAlias);
    assert_eq!(aliases.slots, [0, 1]);
    assert_eq!(aliases.rows, [
        "rt::ResourceAliasRow { key: \"A\", target: \"X\", namespaces: ::core::option::Option::None, line: 1, position: 8, value_line: 9, value_position: 9 }",
        "rt::ResourceAliasRow { key: \"B\", target: \"Y\", namespaces: ::core::option::Option::None, line: 2, position: 8, value_line: 9, value_position: 9 }",
    ]);
    let (_, colors) = &tables[1];
    assert_eq!(colors.rows, [
        "rt::ValueRow { key: \"First\", value: 1_u32, line: 4, position: 8 }",
        "rt::ValueRow { key: \"Second\", value: 2_u32, line: 5, position: 8 }",
    ]);
    assert!(colors.slots.is_empty());
    assert_eq!(tables[2].1.slots, [2]);
}

#[test]
fn a_run_ends_at_another_dictionary_and_at_any_other_statement() {
    let lines = body(&[
        &format!("{};", value_call("resource_dictionary_1", "A", "rt::to_object(1.0_f64)", 1, 1)),
        &format!("{};", value_call("resource_dictionary_1", "B", "rt::to_object(2.0_f64)", 2, 1)),
        &format!("{};", value_call("resource_dictionary_2", "C", "rt::to_object(3.0_f64)", 3, 1)),
        &format!("{};", value_call("resource_dictionary_2", "D", "rt::to_object(4.0_f64)", 4, 1)),
        "let namespace_scope_0 = rt::push_namespace_scope(&context, TABLE);",
        &format!("{};", value_call("resource_dictionary_2", "E", "rt::to_object(5.0_f64)", 5, 1)),
        &format!("{};", value_call("resource_dictionary_2", "F", "rt::to_object(6.0_f64)", 6, 1)),
        "::core::mem::drop(namespace_scope_0);",
    ]);
    let statements = recorded(&lines, &[]);
    let mut count = 0;
    let (written, tables) = with_packed_theme_rows(false, || row_runs(lines, &statements, "D", || {
        count += 1;
        format!("ROWS_{count}")
    }));
    assert_eq!(written, body(&[
        "rt::add_double_resources(&resource_dictionary_1, ROWS_1)?;",
        "rt::add_double_resources(&resource_dictionary_2, ROWS_2)?;",
        "let namespace_scope_0 = rt::push_namespace_scope(&context, TABLE);",
        "rt::add_double_resources(&resource_dictionary_2, ROWS_3)?;",
        "::core::mem::drop(namespace_scope_0);",
    ]));
    assert_eq!(tables.iter().map(|(_, run)| run.rows.len()).collect::<Vec<_>>(), [2, 2, 2]);
}

/// Not from upstream (#568, step 2): the entries of a unit of a theme keep their rows; a run
/// ends where the unit changes and steps over the markers of one unit.
#[test]
fn a_run_ends_where_the_unit_of_the_theme_changes() {
    use super::super::theme_units::{begin_marker, END_MARKER};
    let (first, second) = (begin_marker(1), begin_marker(2));
    let (first, second) = (first.trim_start(), second.trim_start());
    let value = |key: &str, number: f64, line: i32| format!("{};", value_call("resource_dictionary_1", key, &format!("rt::to_object({number:?}_f64)"), line, 1));
    let lines = body(&[
        first,
        &value("A", 1.0, 1),
        &value("B", 2.0, 2),
        END_MARKER,
        first,
        &value("C", 3.0, 3),
        END_MARKER,
        second,
        &value("D", 4.0, 4),
        &value("E", 5.0, 5),
        END_MARKER,
        &value("F", 6.0, 6),
        &value("G", 7.0, 7),
    ]);
    let statements = recorded(&lines, &[]);
    let mut count = 0;
    let (written, tables) = with_packed_theme_rows(false, || row_runs(lines, &statements, "D", || {
        count += 1;
        format!("ROWS_{count}")
    }));
    assert_eq!(written, body(&[
        first,
        "rt::add_double_resources(&resource_dictionary_1, ROWS_1)?;",
        END_MARKER,
        second,
        "rt::add_double_resources(&resource_dictionary_1, ROWS_2)?;",
        END_MARKER,
        "rt::add_double_resources(&resource_dictionary_1, ROWS_3)?;",
    ]));
    assert_eq!(tables.iter().map(|(_, run)| run.rows.len()).collect::<Vec<_>>(), [3, 2, 2]);
}

#[test]
fn a_colour_whose_local_is_named_elsewhere_stays_a_statement() {
    let lines = body(&[
        "let value_0 = ::ferroui_base::media::Color::__markup_FromUInt32_2(1_u32);",
        &format!("{};", value_call("resource_dictionary_1", "A", "rt::to_object(::core::clone::Clone::clone(&value_0))", 1, 1)),
        "let value_1 = ::ferroui_base::media::Color::__markup_FromUInt32_2(2_u32);",
        &format!("{};", value_call("resource_dictionary_1", "B", "rt::to_object(::core::clone::Clone::clone(&value_1))", 2, 1)),
        "use_colour(&value_1);",
    ]);
    let statements = recorded(&lines, &[]);
    let (written, tables) = with_packed_theme_rows(false, || row_runs(lines.clone(), &statements, "D", || "ROWS".to_string()));
    // The first colour is a run of one, the second keeps its local: nothing is written as rows.
    assert_eq!(written, lines);
    assert!(tables.is_empty());
}

#[test]
fn a_table_lists_its_rows_in_order() {
    let run = RowRun {
        kind: RowKind::Thickness,
        rows: vec![
            "rt::ValueRow { key: \"A\", value: [1.0_f64, 2.0_f64, 3.0_f64, 4.0_f64], line: 1, position: 2 }".to_string(),
            "rt::ValueRow { key: \"B\", value: [0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64], line: 3, position: 4 }".to_string(),
        ],
        slots: Vec::new(),
        packed: None,
        fields: None,
        unit: None,
        program: None,
    };
    assert_eq!(
        row_table("POPULATE_X_ROWS_0", "x.xaml", &run),
        "/// The thicknesses of a run of `x.xaml`, added in order by `rt::add_thickness_resources` (#568).\n\
         static POPULATE_X_ROWS_0: &[rt::ValueRow<[f64; 4]>] = &[\n    \
         rt::ValueRow { key: \"A\", value: [1.0_f64, 2.0_f64, 3.0_f64, 4.0_f64], line: 1, position: 2 },\n    \
         rt::ValueRow { key: \"B\", value: [0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64], line: 3, position: 4 },\n\
         ];\n"
    );
}

/// #855: packed tables are on by default (kept by their measurement); the emission of a closure can
/// turn them off.
#[test]
fn packed_tables_are_on_by_default() {
    assert!(DEFAULT_PACKED_THEME_ROWS);
    assert!(packed_theme_rows());
    assert!(!with_packed_theme_rows(false, packed_theme_rows));
    assert!(packed_theme_rows());
}

/// The fused additions of the deferred rows `rows` (key, build, line, row), in a body.
fn packed_body(rows: &[(&str, &str, i32)]) -> Vec<String> {
    let mut statements = Vec::new();
    for (key, build, line) in rows {
        statements.push(format!("// t.xaml({line},8) Content (resource)"));
        statements.push(deferred_add("resource_dictionary_1", key, build, *line));
    }
    statements.iter().map(|statement| format!("    {statement}")).collect()
}

/// #855: with packed tables, a run of a deferred shape is one call of the packed builder over an
/// `rt::PackedRows` whose pool holds each string once (a string inside another is found there),
/// whose namespace tables are numbered from one, and whose records are the fields of the rows.
#[test]
fn a_run_of_a_deferred_shape_is_a_packed_table() {
    let lines = packed_body(&[("AccentBrush", "build_t_deferred_0", 1), ("Accent", "build_t_deferred_1", 2), ("Other", "build_t_deferred_2", 3)]);
    let scoped = DeferredRow { namespaces: Some("T_XML_NAMESPACES_1".to_string()), ..alias("AccentBrush") };
    let statements = recorded(&lines, &[
        ("build_t_deferred_0", 0, alias("Accent")),
        ("build_t_deferred_1", 1, alias("SystemAccent")),
        ("build_t_deferred_2", 2, scoped),
    ]);
    let (written, tables) =
        with_packed_theme_rows(true, || row_runs(lines, &statements, "BUILD_T_DOCUMENT", || "BUILD_T_ROWS_0".to_string()));
    assert_eq!(written, body(&[
        "// t.xaml(1,8) Content (resource)",
        "rt::add_packed_resource_aliases(&resource_dictionary_1, &context, &BUILD_T_DOCUMENT, &BUILD_T_ROWS_0)?;",
    ]));
    let (name, run) = &tables[0];
    assert_eq!(run.slots, [0, 1, 2]);
    // The pool: "AccentBrush", then "SystemAccent"; "Accent" and "Other" are found or added.
    assert_eq!(
        run.packed.as_deref(),
        Some(
            "rt::PackedRows {\n    strings: \"AccentBrushSystemAccentOther\",\n    namespaces: &[T_XML_NAMESPACES_1],\n    records: &[\n        \
             0, 11, 0, 6, 0, 1, 8, 9, 9,\n        \
             0, 6, 11, 12, 0, 2, 8, 9, 9,\n        \
             23, 5, 0, 11, 1, 3, 8, 9, 9,\n    ],\n}"
        )
    );
    let table = row_table(name, "t.xaml", run);
    assert!(table.starts_with(
        "/// The resource aliases of a run of `t.xaml`, packed, added in order by `rt::add_packed_resource_aliases` (#855).\n\
         static BUILD_T_ROWS_0: rt::PackedRows = rt::PackedRows {\n"
    ), "{table}");
    // Without packed tables the run is the slice of rows it always was.
    let lines = packed_body(&[("A", "build_t_deferred_0", 1)]);
    let statements = recorded(&lines, &[("build_t_deferred_0", 0, alias("X"))]);
    let (_, tables) = with_packed_theme_rows(false, || row_runs(lines, &statements, "D", || "ROWS".to_string()));
    assert!(tables[0].1.packed.is_none());
    assert!(row_table("ROWS", "t.xaml", &tables[0].1).contains("static ROWS: &[rt::ResourceAliasRow] = &["));
}

/// #855: with the typed bytecode backend a run is its program, also with packed tables on: a run
/// is encoded by one representation only, and packed tables are of the native backend.
#[test]
fn the_typed_backend_encodes_a_run_without_a_packed_table() {
    let lines = packed_body(&[("AccentBrush", "build_t_deferred_0", 1), ("Accent", "build_t_deferred_1", 2)]);
    let statements = recorded(&lines, &[("build_t_deferred_0", 0, alias("Accent")), ("build_t_deferred_1", 1, alias("SystemAccent"))]);
    let (written, tables) = with_packed_theme_rows(true, || {
        row_runs_with_backend(lines, &statements, "BUILD_T_DOCUMENT", || "BUILD_T_ROWS_0".to_string(), true)
    });
    let (name, run) = &tables[0];
    assert!(run.packed.is_none() && run.program.is_some(), "{run:?}");
    assert!(!written.iter().any(|line| line.contains("rt::add_packed_")), "{written:?}");
    assert!(!row_table(name, "t.xaml", run).contains("rt::PackedRows"));
}

/// #855: the literal brush record splits its colour into halves; a key with escapes is
/// unescaped into the pool and written back as a literal.
#[test]
fn a_literal_brush_and_an_escaped_key_are_packed() {
    let lines = body(&[&deferred_add("resource_dictionary_1", "Quoted\\\"Brush\\u{e9}", "build_t_deferred_0", 4)]);
    let row = DeferredRow { kind: RowKind::LiteralBrush, operand: "4281545523_u32".to_string(), value_position: (0, 0), namespaces: None };
    let statements = recorded(&lines, &[("build_t_deferred_0", 0, row)]);
    let (written, tables) = with_packed_theme_rows(true, || row_runs(lines, &statements, "D", || "ROWS".to_string()));
    assert_eq!(written, body(&["rt::add_packed_literal_brushes(&resource_dictionary_1, &context, &D, &ROWS)?;"]));
    // 4281545523 is 0xFF33_3333: low 0x3333 (13107), high 0xFF33 (65331); the key has 14 bytes.
    assert_eq!(
        tables[0].1.packed.as_deref(),
        Some("rt::PackedRows {\n    strings: \"Quoted\\\"Brush\\u{e9}\",\n    namespaces: &[],\n    records: &[\n        0, 14, 13107, 65331, 0, 4, 8,\n    ],\n}")
    );
}

/// #855: a position beyond 16 bits keeps the slice of rows (the escape of a packed table).
#[test]
fn a_position_beyond_sixteen_bits_keeps_the_slice_of_rows() {
    let lines = packed_body(&[("A", "build_t_deferred_0", 70_000)]);
    let statements = recorded(&lines, &[("build_t_deferred_0", 0, alias("X"))]);
    let (written, tables) = with_packed_theme_rows(true, || row_runs(lines, &statements, "D", || "ROWS".to_string()));
    assert!(written[1].contains("rt::add_resource_aliases(&resource_dictionary_1, &context, &D, ROWS)?;"), "{written:?}");
    assert!(tables[0].1.packed.is_none());
}

/// #855: the strings of rows are read back from the literals the emitter writes.
#[test]
fn string_literals_are_read_back() {
    for text in ["", "Plain", "Quote\"Back\\slash", "Tab\tNew\nLine\r\0", "\u{e9}\u{1f600}"] {
        assert_eq!(string_value(&crate::rust_emitter::rust_string_literal(text)).as_deref(), Some(text));
    }
    assert_eq!(string_value("\"bad\\q\""), None);
    assert_eq!(string_value("not a literal"), None);
}

#[test]
fn typed_resource_runs_keep_units_local_and_encode_singleton_values_and_scopes() {
    use super::super::theme_units::{begin_marker, END_MARKER};
    let lines = body(&[
        begin_marker(1).trim_start(),
        &deferred_add("dictionary", "Alias", "build_alias", 6),
        END_MARKER,
        begin_marker(2).trim_start(),
        &format!("{};", value_call("dictionary", "NegativeZero", "rt::to_object(-0.0_f64)", 3, 4)),
        END_MARKER,
    ]);
    let scoped = DeferredRow {
        kind: RowKind::ResourceAlias, operand: "\"Target\"".into(),
        value_position: (7, 9), namespaces: Some("SCOPED".into()),
    };
    let statements = recorded(&lines, &[("build_alias", 0, scoped)]);
    let mut next = 0;
    let (body, tables) = row_runs_with_backend(lines, &statements, "DOC", || {
        next += 1;
        format!("BLOCK_{next}")
    }, true);
    assert_eq!(tables.len(), 2);
    assert_eq!(tables[0].1.slots, [0]);
    assert!(body.iter().any(|line| line.contains("BLOCK_1.add_resource_aliases(&dictionary, &context, &DOC)?;")));
    assert!(body.iter().any(|line| line.contains("BLOCK_2.add_double_resources(&dictionary)?;")));
    let alias = row_table("BLOCK_1", "test.xaml", &tables[0].1);
    assert!(alias.contains(r#"b"\x01\x01\x01\x00\x0c\x10\x01\x02\x02\x01", &["Alias", "Target"], &[SCOPED]"#), "{alias}");
    assert!(!alias.contains("NegativeZero"));
    let number = row_table("BLOCK_2", "test.xaml", &tables[1].1);
    assert!(number.contains(r#"b"\x01\x05\x01\x00\x06\x08\xff\x00\x00\x00\x00\x00\x00\x00\x80", &["NegativeZero"], &[]"#), "{number}");
    assert!(!number.contains("SCOPED"));
}

/// The additions of the numbers `rows` (key, literal, line) to one dictionary, as a run.
fn value_run(rows: &[(&str, &str, i32)], name: &str, unit: Option<u32>) -> (String, String, RowRun) {
    let lines = body(&rows.iter().map(|(key, value, line)| format!("{};", value_call("resource_dictionary_1", key, &format!("rt::to_object({value})"), *line, 8))).collect::<Vec<_>>().iter().map(String::as_str).collect::<Vec<_>>());
    let statements = recorded(&lines, &[]);
    let (_, mut tables) = with_packed_theme_rows(true, || row_runs(lines, &statements, "D", || name.to_string()));
    let (name, mut run) = tables.remove(0);
    run.unit = unit;
    (name, "t.xaml".to_string(), run)
}

/// #855: a run of plain values is a packed table of its strings and numbers when that is smaller
/// than its rows, and the call names the builder of the table.
#[test]
fn a_run_of_values_is_a_packed_table_when_that_is_smaller() {
    let rows = [("A", "1.0_f64", 1), ("BB", "2.0_f64", 2), ("A2", "1.0_f64", 3)];
    let lines = body(&rows.iter().map(|(key, value, line)| format!("{};", value_call("resource_dictionary_1", key, &format!("rt::to_object({value})"), *line, 8))).collect::<Vec<_>>().iter().map(String::as_str).collect::<Vec<_>>());
    let statements = recorded(&lines, &[]);
    let (written, tables) = with_packed_theme_rows(true, || row_runs(lines, &statements, "D", || "BUILD_T_ROWS_0".to_string()));
    assert_eq!(written, body(&["rt::add_packed_double_resources(&resource_dictionary_1, &BUILD_T_ROWS_0)?;"]));
    assert_eq!(
        tables[0].1.packed.as_deref(),
        Some("rt::PackedValueRows {\n    strings: \"ABBA2\",\n    numbers: &[1.0_f64, 2.0_f64],\n    records: &[\n        0, 1, 1, 8, 0,\n        1, 2, 2, 8, 1,\n        3, 2, 3, 8, 0,\n    ],\n}")
    );
    assert!(row_table("BUILD_T_ROWS_0", "t.xaml", &tables[0].1).contains("static BUILD_T_ROWS_0: rt::PackedValueRows = rt::PackedValueRows {"));
    // Two numbers are smaller as a slice than with the header of a table.
    let short = value_run(&[("A", "1.0_f64", 1), ("B", "2.0_f64", 2)], "ROWS", None);
    assert!(short.2.packed.is_none());
    assert!(row_table("ROWS", "t.xaml", &short.2).contains("&[rt::ValueRow<f64>]"));
}

/// #855: every shape of plain value has its record, and a position beyond 16 bits keeps the slice.
#[test]
fn the_shapes_of_values_have_their_records() {
    let thickness = "::ferroui_base::Thickness::__markup_new_3(1.0_f64, 2.0_f64, 1.0_f64, 0.0_f64)";
    let run = value_run(&[("A", thickness, 1), ("B", thickness, 2)], "ROWS", None).2;
    assert_eq!((run.kind, run.packed.as_deref()), (
        RowKind::Thickness,
        Some("rt::PackedValueRows {\n    strings: \"AB\",\n    numbers: &[1.0_f64, 2.0_f64, 0.0_f64],\n    records: &[\n        0, 1, 1, 8, 0, 1, 0, 2,\n        1, 1, 2, 8, 0, 1, 0, 2,\n    ],\n}"),
    ));
    let lines = body(&[
        &format!("{};", value_call("resource_dictionary_1", "A", "rt::string_object(\"xy\")", 1, 8)),
        &format!("{};", value_call("resource_dictionary_1", "B", "rt::string_object(\"y\")", 2, 8)),
    ]);
    let statements = recorded(&lines, &[]);
    let (written, tables) = with_packed_theme_rows(true, || row_runs(lines, &statements, "D", || "ROWS".to_string()));
    assert_eq!(written, body(&["rt::add_packed_string_resources(&resource_dictionary_1, &ROWS)?;"]));
    assert_eq!(
        tables[0].1.packed.as_deref(),
        Some("rt::PackedValueRows {\n    strings: \"AxyB\",\n    numbers: &[],\n    records: &[\n        0, 1, 1, 8, 1, 2,\n        3, 1, 2, 8, 2, 1,\n    ],\n}")
    );
    let far = value_run(&[("A", "1.0_f64", 1), ("B", "2.0_f64", 70_000), ("C", "3.0_f64", 3), ("D", "4.0_f64", 4)], "ROWS", None).2;
    assert!(far.packed.is_none(), "a line beyond 16 bits keeps the slice");
}

/// #855: the tables of values of a unit share its pools, so that a trimmed unit takes its strings
/// and numbers with it.
#[test]
fn the_value_tables_of_a_unit_share_its_pools() {
    let first = value_run(&[("A", "1.0_f64", 1), ("B", "2.0_f64", 2), ("C", "1.0_f64", 3)], "BUILD_T_ROWS_0", Some(4));
    let second = value_run(&[("A", "2.0_f64", 4), ("D", "5.0_f64", 5), ("E", "2.0_f64", 6)], "BUILD_T_ROWS_1", Some(4));
    let other = value_run(&[("F", "7.0_f64", 7), ("G", "8.0_f64", 8), ("H", "7.0_f64", 9)], "BUILD_T_ROWS_2", Some(5));
    let texts = row_tables_in("BUILD_T", &[first, second, other], &Default::default());
    assert_eq!(texts.len(), 5, "{texts:#?}");
    assert!(texts[0].ends_with("const BUILD_T_UNIT_4_STRINGS: &str = \"ABCDE\";\n"), "{}", texts[0]);
    assert!(texts[1].ends_with("static BUILD_T_UNIT_4_NUMBERS: [f64; 3] = [1.0_f64, 2.0_f64, 5.0_f64];\n"), "{}", texts[1]);
    assert!(texts[2].contains("    strings: BUILD_T_UNIT_4_STRINGS,\n    numbers: &BUILD_T_UNIT_4_NUMBERS,\n"), "{}", texts[2]);
    assert!(texts[3].contains("        0, 1, 4, 8, 1,\n        3, 1, 5, 8, 2,\n        4, 1, 6, 8, 1,\n"), "{}", texts[3]);
    // The only table of another unit keeps its own pools.
    assert!(texts[4].contains("    strings: \"FGH\",\n    numbers: &[7.0_f64, 8.0_f64],\n"), "{}", texts[4]);
}

/// #855: the packed tables of one unit of a theme share one pool of strings, written once before
/// the first of them, while a table of no unit, or the only table of its unit, keeps its own.
#[test]
fn the_packed_tables_of_a_unit_share_one_pool() {
    let packed = |rows: &[(&str, &str, i32)], targets: &[&str], name: &str, unit: Option<u32>| {
        let lines = packed_body(rows);
        let rows: Vec<(&str, usize, DeferredRow)> =
            rows.iter().zip(targets).enumerate().map(|(slot, ((_, build, _), target))| (*build, slot, alias(target))).collect();
        let statements = recorded(&lines, &rows);
        let (_, mut tables) =
            with_packed_theme_rows(true, || row_runs(lines, &statements, "BUILD_T_DOCUMENT", || name.to_string()));
        let (name, mut run) = tables.remove(0);
        run.unit = unit;
        (name, "t.xaml".to_string(), run)
    };
    let light = packed(&[("AccentBrush", "build_t_deferred_0", 1), ("OtherBrush", "build_t_deferred_1", 2)], &["Accent", "Other"], "BUILD_T_ROWS_0", Some(3));
    let dark = packed(&[("AccentBrush", "build_t_deferred_2", 3), ("OtherBrush", "build_t_deferred_3", 4)], &["Accent", "Dark"], "BUILD_T_ROWS_1", Some(3));
    let alone = packed(&[("AccentBrush", "build_t_deferred_4", 5), ("OtherBrush", "build_t_deferred_5", 6)], &["Accent", "Other"], "BUILD_T_ROWS_2", None);
    let texts = row_tables_in("BUILD_T", &[light, dark, alone], &Default::default());
    assert_eq!(texts.len(), 4, "{texts:#?}");
    assert_eq!(
        texts[0],
        "/// The keys and targets of the packed tables of the unit 3 of `t.xaml`, each once (#855).\n\
         const BUILD_T_UNIT_3_STRINGS: &str = \"AccentBrushOtherBrushDark\";\n"
    );
    assert!(texts[1].contains("static BUILD_T_ROWS_0: rt::PackedRows = rt::PackedRows {\n    strings: BUILD_T_UNIT_3_STRINGS,\n"), "{}", texts[1]);
    // The table of the dark theme finds the keys and the shared target in the pool.
    assert!(texts[2].contains("    strings: BUILD_T_UNIT_3_STRINGS,\n    namespaces: &[],\n    records: &[\n        0, 11, 0, 6, 0, 3, 8, 9, 9,\n        11, 10, 21, 4, 0, 4, 8, 9, 9,\n"), "{}", texts[2]);
    // A table of no unit keeps its own pool.
    assert!(texts[3].contains("    strings: \"AccentBrushOtherBrush\",\n"), "{}", texts[3]);
}
