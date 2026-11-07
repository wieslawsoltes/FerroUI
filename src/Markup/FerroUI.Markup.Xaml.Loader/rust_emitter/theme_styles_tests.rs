//! Tests of the control theme tables ([`super`]): the factories, selector chains, setter
//! choices and template bindings recognised in the statements the emitter writes (taken from
//! the generated theme files before #568, step 5), and the tables written for them.

use super::*;

const TEMPLATED: &str = "::ferroui_controls::primitives::TemplatedControl";
const PRESENTER: &str = "::ferroui_controls::presenters::ContentPresenter";
const THEME_RETURNED: &str = "rt::to_value(control_theme_0.clone())";

/// The lines of a body as the emitter writes them, each indented once.
fn body(statements: &[&str]) -> Vec<String> {
    statements.iter().map(|statement| format!("    {statement}")).collect()
}

fn style_of(local: &str) -> String {
    format!("{local}.upcast_ref::<{STYLE_BASE}>()")
}

/// The factory of the Button theme of the Fluent theme, shortened: a dynamic setter, constant
/// setters, an expression, a value with a statement, a template, and nested styles with selector
/// chains (two of them alike).
fn button_factory() -> Vec<String> {
    let theme = style_of("control_theme_0");
    let mut lines = vec![
        "// Controls/FluentControls.xaml(13,4) ControlTheme".to_string(),
        format!("let control_theme_0 = {CONTROL_THEME}::new();"),
        "context.push_parent(rt::to_value(control_theme_0.clone()));".to_string(),
        "// Controls/FluentControls.xaml(13,4) TargetType".to_string(),
        format!("{CONTROL_THEME}::__markup_set_TargetType(&control_theme_0, ::core::option::Option::Some(<::ferroui_controls::Button as ::ferroui_base::StaticType>::TYPE));"),
        format!("rt::add_dynamic_resource_setter_with_parent(&context, {theme}, rt::property({TEMPLATED}::background_property()), \"ButtonBackground\");"),
        format!("rt::add_setter({theme}, rt::property(::ferroui_base::layout::Layoutable::horizontal_alignment_property()), rt::to_object(::ferroui_base::layout::HorizontalAlignment::Left));"),
        format!("rt::add_setter({theme}, rt::property({TEMPLATED}::padding_property()), rt::to_object({THICKNESS}::__markup_new_3(8.0_f64, 5.0_f64, 8.0_f64, 6.0_f64)));"),
        format!("rt::add_setter({theme}, rt::class_property(\"accent\"), rt::to_object(::core::clone::Clone::clone(&false)));"),
        "// Controls/FluentControls.xaml(22,6) Content".to_string(),
        format!("let setter_8 = rt::new_setter(rt::property(::ferroui_base::Visual::render_transform_property()));"),
        "// Controls/FluentControls.xaml(22,6) Value".to_string(),
        "let value_0 = rt::invoked(<dyn ::ferroui_base::media::ITransform>::__markup_parse(::std::string::String::from(\"none\")), 22, 40)?;".to_string(),
        format!("rt::add_setter_value({theme}, &setter_8, rt::to_object(::core::clone::Clone::clone(&value_0)));"),
        format!("let setter_10 = rt::new_setter(rt::property({TEMPLATED}::template_property()));"),
        format!("let control_template_0 = {CONTROL_TEMPLATE}::__markup_new_0();"),
        "let deferred_0 = rt::defer(rt::class_handle(<::ferroui_controls::Control as ::ferroui_base::StaticType>::TYPE), &context, build_controls_fluentcontrols_xaml_deferred_1, 31, 10)?;".to_string(),
        format!("{CONTROL_TEMPLATE}::__markup_set_Content(&control_template_0, rt::to_object(::core::clone::Clone::clone(&deferred_0)));"),
        format!("rt::add_setter_value({theme}, &setter_10, rt::to_object(::core::clone::Clone::clone(&control_template_0)));"),
    ];
    for (style, first) in [("style_0", 0), ("style_1", 4)] {
        let selectors: Vec<String> = (first..first + 4).map(|index| format!("selector_{index}")).collect();
        lines.extend([
            format!("// Controls/FluentControls.xaml(44,6) Style"),
            format!("let {style} = {STYLE}::new();"),
            format!("context.push_parent(rt::to_value({style}.clone()));"),
            format!("let {} = {SELECTORS}::nesting(::core::option::Option::None);", selectors[0]),
            format!("let {} = {SELECTORS}::class(::core::option::Option::Some({}), \":pointerover\");", selectors[1], selectors[0]),
            format!("let {} = {SELECTORS}::template(::core::option::Option::Some({}));", selectors[2], selectors[1]),
            format!(
                "let {} = {SELECTORS}::of_type_info(::core::option::Option::Some({}), <{PRESENTER} as ::ferroui_base::StaticType>::TYPE);",
                selectors[3], selectors[2]
            ),
            format!("{STYLE}::__markup_set_Selector(&{style}, ::core::option::Option::Some({}.clone()));", selectors[3]),
            format!(
                "rt::add_dynamic_resource_setter_with_parent(&context, {}, rt::property({PRESENTER}::background_property()), \"ButtonBackgroundPointerOver\");",
                style_of(style)
            ),
            "context.pop_parent();".to_string(),
            format!("{STYLE_BASE}::__markup_Add_1({theme}, rt::cast({style}.clone(), 44, 6)?);"),
        ]);
    }
    lines.push("context.pop_parent();".to_string());
    lines.iter().map(|line| format!("    {line}")).collect()
}

#[test]
fn a_factory_becomes_a_tree_of_its_members() {
    let theme = parse_theme(&button_factory(), THEME_RETURNED).expect("a control theme");
    assert!(theme.parent);
    assert_eq!(theme.members.len(), 9);
    assert_eq!(theme.members[0], Member::TargetType("<::ferroui_controls::Button as ::ferroui_base::StaticType>::TYPE".to_string()));
    assert_eq!(theme.members[1], Member::Setter {
        property: format!("rt::property({TEMPLATED}::background_property())"),
        form: Form::Plain,
        value: Value::Dynamic("\"ButtonBackground\"".to_string()),
    });
    assert_eq!(theme.members[2], Member::Setter {
        property: "rt::property(::ferroui_base::layout::Layoutable::horizontal_alignment_property())".to_string(),
        form: Form::Plain,
        value: Value::Expression("rt::to_object(::ferroui_base::layout::HorizontalAlignment::Left)".to_string()),
    });
    assert_eq!(theme.members[3], Member::Setter {
        property: format!("rt::property({TEMPLATED}::padding_property())"),
        form: Form::Plain,
        value: Value::Constant("rt::ThemeValue::Thickness([8.0_f64, 5.0_f64, 8.0_f64, 6.0_f64])".to_string()),
    });
    assert_eq!(theme.members[4], Member::Setter {
        property: "rt::class_property(\"accent\")".to_string(),
        form: Form::Plain,
        value: Value::Constant("rt::ThemeValue::Bool(false)".to_string()),
    });
    assert_eq!(theme.members[5], Member::Setter {
        property: "rt::property(::ferroui_base::Visual::render_transform_property())".to_string(),
        form: Form::Created,
        value: Value::Computed {
            statements: vec!["    let value_0 = rt::invoked(<dyn ::ferroui_base::media::ITransform>::__markup_parse(::std::string::String::from(\"none\")), 22, 40)?;".to_string()],
            value: "rt::to_object(::core::clone::Clone::clone(&value_0))".to_string(),
            uses_context: false,
        },
    });
    assert_eq!(theme.members[6], Member::Setter {
        property: format!("rt::property({TEMPLATED}::template_property())"),
        form: Form::Created,
        value: Value::Template(Template {
            target_type: None,
            content_type: "<::ferroui_controls::Control as ::ferroui_base::StaticType>::TYPE".to_string(),
            build: "build_controls_fluentcontrols_xaml_deferred_1".to_string(),
            parent: false,
            position: (31, 10),
        }),
    });
    let Member::Style(style) = &theme.members[7] else { panic!("a nested style") };
    assert!(style.parent);
    assert_eq!(style.position, (44, 6));
    assert_eq!(style.selector, Some(vec![
        Step::Nesting,
        Step::Class("\":pointerover\"".to_string()),
        Step::Template,
        Step::OfType(format!("<{PRESENTER} as ::ferroui_base::StaticType>::TYPE")),
    ]));
    assert_eq!(style.members.len(), 1);
    assert_eq!(theme.members[8], theme.members[7]);
}

#[test]
fn a_table_shares_its_properties_and_selector_chains() {
    let theme = parse_theme(&button_factory(), THEME_RETURNED).expect("a control theme");
    let mut pool = StylePool::new("build_controls_fluentcontrols_xaml", "Controls/FluentControls.xaml");
    let call = control_theme_table(&mut pool, "build_controls_fluentcontrols_xaml_deferred_0", &theme);
    assert_eq!(call, "rt::build_control_theme(&context, &BUILD_CONTROLS_FLUENTCONTROLS_XAML_DEFERRED_0_THEME)");
    let items = pool.items.join("\n");
    // One function per property, one static per distinct chain.
    assert_eq!(items.matches("fn build_controls_fluentcontrols_xaml_property_").count(), 7);
    assert_eq!(items.matches("static BUILD_CONTROLS_FLUENTCONTROLS_XAML_SELECTOR_").count(), 1);
    assert!(items.contains(&format!(
        "static BUILD_CONTROLS_FLUENTCONTROLS_XAML_SELECTOR_0: &[rt::SelectorStep] = &[rt::SelectorStep::Nesting, rt::SelectorStep::Class(\":pointerover\"), rt::SelectorStep::Template, rt::SelectorStep::OfType(<{PRESENTER} as ::ferroui_base::StaticType>::TYPE)];"
    )));
    assert!(items.contains("static BUILD_CONTROLS_FLUENTCONTROLS_XAML_DEFERRED_0_THEME: rt::ControlThemeTable = rt::ControlThemeTable {\n    parent: true,\n    members: &[\n        rt::StyleMember::TargetType(<::ferroui_controls::Button as ::ferroui_base::StaticType>::TYPE),\n"));
    assert!(items.contains("        rt::StyleMember::Setter(rt::ThemeSetter { property: build_controls_fluentcontrols_xaml_property_0, form: rt::SetterForm::Plain, value: rt::ThemeValue::Dynamic(\"ButtonBackground\") }),\n"));
    assert!(items.contains("value: rt::ThemeValue::Expression(build_controls_fluentcontrols_xaml_deferred_0_value_0) }),"));
    assert!(items.contains("value: rt::ThemeValue::Computed(build_controls_fluentcontrols_xaml_deferred_0_value_1) }),"));
    assert!(items.contains("value: rt::ThemeValue::Template(rt::ThemeTemplate { target_type: ::core::option::Option::None, content_type: <::ferroui_controls::Control as ::ferroui_base::StaticType>::TYPE, build: build_controls_fluentcontrols_xaml_deferred_1, parent: false, line: 31, position: 10 }) }),"));
    assert!(items.contains("            selector: ::core::option::Option::Some(BUILD_CONTROLS_FLUENTCONTROLS_XAML_SELECTOR_0),\n"));
    assert!(items.contains("fn build_controls_fluentcontrols_xaml_deferred_0_value_0() -> ::ferroui_base::metadata::MarkupValue {\n    rt::to_object(::ferroui_base::layout::HorizontalAlignment::Left)\n}\n"));
    assert!(items.contains("fn build_controls_fluentcontrols_xaml_deferred_0_value_1(\n    _context: &::std::rc::Rc<::ferroui_markup_xaml::xaml_il::runtime::XamlIlContext>,\n) -> ::core::result::Result<::ferroui_base::metadata::MarkupValue, ::ferroui_markup_xaml::XamlLoadError> {\n    let value_0 = rt::invoked("));
    // A second factory of the same function reuses the shared items.
    let before = pool.items.len();
    control_theme_table(&mut pool, "build_controls_fluentcontrols_xaml_deferred_2", &theme);
    assert_eq!(pool.items.len(), before + 3, "its table and its two value functions");
}

#[test]
fn based_on_and_a_static_resource_setter_are_members() {
    let theme_type = format!("<{CONTROL_THEME} as ::ferroui_base::StaticType>::TYPE");
    let lines = body(&[
        &format!("let control_theme_0 = {CONTROL_THEME}::new();"),
        "context.push_parent(rt::to_value(control_theme_0.clone()));",
        &format!("let static_resource_extension_0 = {STATIC_RESOURCE}::__markup_new_1(rt::boxed(<::ferroui_controls::Button as ::ferroui_base::StaticType>::TYPE));"),
        &format!("let provided_0 = rt::provide_value_invoked(&context, rt::boxed(rt::clr_property_info(rt::class_markup({theme_type}), \"BasedOn\", false, rt::class_handle({theme_type}), false)), &static_resource_extension_0, {STATIC_RESOURCE}::__markup_ProvideValue_0, 85, 17)?;"),
        &format!("{CONTROL_THEME}::__markup_set_BasedOn(&control_theme_0, rt::exact(rt::cast_checked(provided_0, rt::class_handle({theme_type}), \"FerroUI.Styling.ControlTheme\", 85, 17)?, \"FerroUI.Styling.ControlTheme.set_BasedOn\", 0, 85, 17)?);"),
        &format!("let setter_0 = rt::new_setter_with_parent(&context, rt::property({TEMPLATED}::foreground_property()));"),
        &format!("let static_resource_extension_1 = {STATIC_RESOURCE}::__markup_new_1(rt::string_object(\"AccentBrush\"));"),
        &format!("let provided_1 = rt::provide_value_invoked(&context, rt::setter_value_property(), &static_resource_extension_1, {STATIC_RESOURCE}::__markup_ProvideValue_0, 86, 30)?;"),
        &format!("rt::add_setter_value_with_parent(&context, {}, &setter_0, ::core::clone::Clone::clone(&provided_1));", style_of("control_theme_0")),
        "context.pop_parent();",
    ]);
    let theme = parse_theme(&lines, THEME_RETURNED).expect("a control theme");
    assert_eq!(theme.members, vec![
        Member::BasedOn { key: Key::Type("<::ferroui_controls::Button as ::ferroui_base::StaticType>::TYPE".to_string()), position: (85, 17) },
        Member::Setter {
            property: format!("rt::property({TEMPLATED}::foreground_property())"),
            form: Form::Parented,
            value: Value::Static { key: "\"AccentBrush\"".to_string(), position: (86, 30) },
        },
    ]);
}

#[test]
fn selector_chains_with_or_not_and_property_steps_are_trees() {
    let lines = body(&[
        &format!("let control_theme_0 = {CONTROL_THEME}::new();"),
        &format!("let style_0 = {STYLE}::new();"),
        &format!("let selector_0 = {SELECTORS}::nesting(::core::option::Option::None);"),
        &format!("let selector_1 = {SELECTORS}::class(::core::option::Option::Some(selector_0), \":checked\");"),
        &format!("let selector_2 = {SELECTORS}::nesting(::core::option::Option::None);"),
        &format!("let selector_3 = {SELECTORS}::class(::core::option::Option::None, \":pressed\");"),
        &format!("let selector_4 = {SELECTORS}::not(::core::option::Option::Some(selector_2), selector_3);"),
        &format!("let selector_5 = {SELECTORS}::or([selector_1, selector_4]);"),
        &format!("let selector_6 = {SELECTORS}::property_equals_untyped(::core::option::Option::Some(selector_5), rt::property(::ferroui_controls::ComboBox::is_editable_property()), ::std::rc::Rc::new(::core::clone::Clone::clone(&true)));"),
        &format!("let selector_7 = {SELECTORS}::nth_last_child(::core::option::Option::Some(selector_6), -2, 1);"),
        &format!("{STYLE}::__markup_set_Selector(&style_0, ::core::option::Option::Some(selector_7.clone()));"),
        &format!("rt::add_setter({}, rt::property(::ferroui_base::Visual::is_visible_property()), ::core::option::Option::None);", style_of("style_0")),
        &format!("{STYLE_BASE}::__markup_Add_1({}, rt::cast(style_0.clone(), 9, 6)?);", style_of("control_theme_0")),
    ]);
    let theme = parse_theme(&lines, THEME_RETURNED).expect("a control theme");
    assert!(!theme.parent);
    let Member::Style(style) = &theme.members[0] else { panic!("a nested style") };
    assert!(!style.parent);
    assert_eq!(style.selector, Some(vec![
        Step::Or(vec![
            vec![Step::Nesting, Step::Class("\":checked\"".to_string())],
            vec![Step::Nesting, Step::Not(vec![Step::Class("\":pressed\"".to_string())])],
        ]),
        Step::PropertyEquals {
            property: "rt::property(::ferroui_controls::ComboBox::is_editable_property())".to_string(),
            value: "::core::clone::Clone::clone(&true)".to_string(),
        },
        Step::NthLastChild(-2, 1),
    ]));
    let mut pool = StylePool::new("build_x", "x.xaml");
    let name = pool.selector(style.selector.as_deref().expect("a selector"));
    assert_eq!(name, "BUILD_X_SELECTOR_0");
    let items = pool.items.join("\n");
    assert!(items.contains("rt::SelectorStep::Or { alternatives: &[&[rt::SelectorStep::Nesting, rt::SelectorStep::Class(\":checked\")], &[rt::SelectorStep::Nesting, rt::SelectorStep::Not { argument: &[rt::SelectorStep::Class(\":pressed\")] }]] }"));
    assert!(items.contains("rt::SelectorStep::PropertyEquals { property: build_x_property_0, value: build_x_selector_value_0 }, rt::SelectorStep::NthLastChild(-2, 1)]"));
    assert!(items.contains("fn build_x_selector_value_0() -> ::ferroui_base::BoxedValue {\n    ::std::rc::Rc::new(::core::clone::Clone::clone(&true))\n}\n"));
}

#[test]
fn a_factory_with_a_member_the_table_cannot_hold_keeps_its_code() {
    let theme = style_of("control_theme_0");
    // An animation of the theme.
    let animation = body(&[
        &format!("let control_theme_0 = {CONTROL_THEME}::new();"),
        &format!("let animations_collection_0 = {STYLE_BASE}::__markup_get_Animations({theme});"),
    ]);
    assert!(parse_theme(&animation, THEME_RETURNED).is_none());
    // A value that names a local of the factory other than its own.
    let shared = body(&[
        &format!("let control_theme_0 = {CONTROL_THEME}::new();"),
        "let value_0 = rt::invoked(::ferroui_base::media::FontWeight::__markup_parse(::std::string::String::from(\"Bold\")), 3, 4)?;",
        &format!("let setter_0 = rt::new_setter(rt::property({TEMPLATED}::font_weight_property()));"),
        &format!("rt::add_setter_value({theme}, &setter_0, rt::to_object(::core::clone::Clone::clone(&value_0)));"),
    ]);
    assert!(parse_theme(&shared, THEME_RETURNED).is_none());
    // The name scope, another value returned, a missing pop, another first statement.
    let named = body(&[
        &format!("let control_theme_0 = {CONTROL_THEME}::new();"),
        &format!("rt::add_setter({theme}, rt::property({TEMPLATED}::tag_property()), rt::to_object(name_scope.clone()));"),
    ]);
    assert!(parse_theme(&named, THEME_RETURNED).is_none());
    assert!(parse_theme(&button_factory(), "rt::to_value(control_theme_1.clone())").is_none());
    let mut unbalanced = button_factory();
    unbalanced.pop();
    assert!(parse_theme(&unbalanced, THEME_RETURNED).is_none());
    assert!(parse_theme(&body(&["let style_0 = ::ferroui_base::styling::Style::new();"]), THEME_RETURNED).is_none());
}

#[test]
fn constants_are_values_of_the_table() {
    assert_eq!(constant_value("::core::option::Option::None").as_deref(), Some("rt::ThemeValue::Null"));
    assert_eq!(constant_value("rt::to_object(1.5_f64)").as_deref(), Some("rt::ThemeValue::Double(1.5_f64)"));
    assert_eq!(constant_value("rt::to_object(::core::primitive::f64::NAN)").as_deref(), Some("rt::ThemeValue::Double(::core::primitive::f64::NAN)"));
    assert_eq!(constant_value("rt::to_object(-3_i32)").as_deref(), Some("rt::ThemeValue::Int(-3_i32)"));
    assert_eq!(constant_value("rt::string_object(\"a, b\")").as_deref(), Some("rt::ThemeValue::Text(\"a, b\")"));
    assert_eq!(
        constant_value(&format!("rt::to_object({CORNER_RADIUS}::__markup_new_3(4.0_f64, 4.0_f64, 0.0_f64, 0.0_f64))")).as_deref(),
        Some("rt::ThemeValue::CornerRadius([4.0_f64, 4.0_f64, 0.0_f64, 0.0_f64])")
    );
    assert_eq!(constant_value("rt::to_object(2_u32)"), None);
    assert_eq!(constant_value("rt::to_object(::ferroui_base::layout::Orientation::Vertical)"), None);
    assert_eq!(split_property("rt::property(::a::B::c_property()), rest"), Some(("rt::property(::a::B::c_property())", ", rest")));
    assert_eq!(split_property("rt::class_property(\"x)\"), rest"), Some(("rt::class_property(\"x)\")", ", rest")));
    assert_eq!(split_property("rt::property(value_0)"), None);
}

// --- template bindings --------------------------------------------------------------------

fn binding(object: &str, target: &str, source: &str, line: i32) -> String {
    format!(
        "    rt::bind_template_binding({object}.upcast_ref::<::ferroui_base::FerroObject>(), {PRESENTER}::{target}(), rt::property({TEMPLATED}::{source}()), {line}, 27)?;"
    )
}

#[test]
fn a_run_of_template_bindings_on_one_object_becomes_a_table() {
    let lines = vec![
        "    // Controls/Button.xaml(16,27) Padding".to_string(),
        binding("content_presenter_0", "padding_property", "padding_property", 16),
        "    // Controls/Button.xaml(17,27) Background".to_string(),
        binding("content_presenter_0", "background_property", "background_property", 17),
        binding("content_presenter_0", "foreground_property", "foreground_property", 18),
        binding("border_0", "background_property", "background_property", 19),
        "    border_0.set_value(::ferroui_controls::Border::padding_property(), ::ferroui_base::Thickness::__markup_new_3(1.0_f64, 1.0_f64, 1.0_f64, 1.0_f64));".to_string(),
        binding("border_0", "padding_property", "padding_property", 21),
    ];
    let mut pool = StylePool::new("build_x", "x.xaml");
    let body = template_binding_runs(&mut pool, lines.clone());
    assert_eq!(body, vec![
        "    // Controls/Button.xaml(16,27) Padding".to_string(),
        "    rt::bind_template_bindings(content_presenter_0.upcast_ref::<::ferroui_base::FerroObject>(), BUILD_X_BINDINGS_0)?;".to_string(),
        lines[5].clone(),
        lines[6].clone(),
        lines[7].clone(),
    ]);
    let items = pool.items.join("\n");
    assert!(items.contains("static BUILD_X_BINDINGS_0: &[rt::TemplateBindingRow] = &[\n    rt::TemplateBindingRow { target: build_x_property_0, source: build_x_property_1, line: 16, position: 27 },\n"));
    assert!(items.contains(&format!("fn build_x_property_0() -> &'static ::ferroui_base::FerroProperty {{\n    rt::property({PRESENTER}::padding_property())\n}}\n")));
    assert_eq!(items.matches("rt::TemplateBindingRow {").count(), 3);
    // A body without a run is written as it was.
    let single = vec![binding("border_0", "padding_property", "padding_property", 4)];
    assert_eq!(template_binding_runs(&mut pool, single.clone()), single);
}

#[test]
fn a_template_binding_is_parsed() {
    let statement = binding("border_0", "padding_property", "padding_property", 4);
    assert_eq!(template_binding(statement.trim_start()), Some(TemplateBinding {
        object: "border_0",
        target: "::ferroui_controls::presenters::ContentPresenter::padding_property()",
        source: "::ferroui_controls::primitives::TemplatedControl::padding_property()",
        position: (4, 27),
    }));
    assert_eq!(template_binding("rt::bind_template_binding(border_0, x, y, 1, 2)?;"), None);
}

// --- the setter of a resource value -------------------------------------------------------

const UNSET: &str = "rt::markup_handle(<::ferroui_base::UnsetValueType as ::ferroui_base::metadata::MarkupTyped>::MARKUP)";
const BINDING: &str = "rt::markup_handle(<dyn ::ferroui_base::data::BindingBase as ::ferroui_base::metadata::MarkupTyped>::MARKUP)";

fn chain(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| line.to_string()).collect()
}

#[test]
fn the_choice_of_a_static_resource_setter_becomes_one_call() {
    let property = "::ferroui_base::media::GradientStop::color_property()";
    let target = "gradient_stop_0.upcast_ref::<::ferroui_base::FerroObject>()";
    // Outside a template: the unset marker, a binding, a colour, else the error.
    let local = chain(&[
        &format!("if rt::is_instance(&value_0, {UNSET}) {{"),
        &format!("    rt::unset_value({target}, {property});"),
        &format!("}} else if rt::is_instance(&value_0, {BINDING}) {{"),
        &format!("    rt::bind({target}, {property}, value_0.clone(), 198, 39)?;"),
        "} else if rt::is_instance(&value_0, rt::markup_handle(<::ferroui_base::media::Color as ::ferroui_base::metadata::MarkupTyped>::MARKUP)) {",
        &format!("    gradient_stop_0.set_value({property}, rt::exact(value_0.clone(), \"FerroUI.Media.GradientStop.set_Color\", 0, 198, 39)?);"),
        "} else {",
        "    return ::core::result::Result::Err(rt::no_setter(\"Color\", &value_0, 198, 39));",
        "}",
    ]);
    assert_eq!(
        resource_choice("value_0", &local).as_deref(),
        Some(format!("rt::assign_resource_value({target}, {property}, value_0, &rt::ResourceChoice {{ name: \"Color\", member: \"FerroUI.Media.GradientStop.set_Color\", index: 0, unset: true, instance: rt::ValueHandle::Markup(<::ferroui_base::media::Color as ::ferroui_base::metadata::MarkupTyped>::MARKUP), null: false, line: 198, position: 39 }}, ::core::option::Option::None)?;").as_str())
    );
    // In a template: a binding, a text or null with the priority of the template, else the error.
    let property = "::ferroui_controls::TextBlock::text_property()";
    let target = "text_block_0.upcast_ref::<::ferroui_base::FerroObject>()";
    let set = format!("    text_block_0.set_value_with_priority({property}, rt::exact(value_3.clone(), \"TextBlock.Text\", 1, 73, 28)?, priority_8);");
    let template = chain(&[
        &format!("if rt::is_instance(&value_3, {BINDING}) {{"),
        &format!("    rt::bind({target}, {property}, value_3.clone(), 73, 28)?;"),
        "} else if rt::is_instance(&value_3, ::ferroui_base::data::core::ValueType::of::<::std::string::String>()) {",
        &set,
        "} else if value_3.is_none() {",
        &set,
        "} else {",
        "    return ::core::result::Result::Err(rt::no_setter(\"Text\", &value_3, 73, 28));",
        "}",
    ]);
    assert_eq!(
        resource_choice("value_3", &template).as_deref(),
        Some(format!("rt::assign_resource_value({target}, {property}, value_3, &rt::ResourceChoice {{ name: \"Text\", member: \"TextBlock.Text\", index: 1, unset: false, instance: rt::ValueHandle::Of(::ferroui_base::data::core::ValueType::of::<::std::string::String>), null: true, line: 73, position: 28 }}, ::core::option::Option::Some(priority_8))?;").as_str())
    );
}

#[test]
fn another_choice_keeps_its_chain() {
    let property = "::ferroui_controls::ContentPresenter::content_property()";
    // A template binding: no type check.
    let binding = chain(&[
        "if value_0.is_some() {",
        &format!("    rt::bind(content_presenter_0.upcast_ref::<::ferroui_base::FerroObject>(), {property}, value_0.clone(), 28, 33)?;"),
        "} else {",
        &format!("    content_presenter_0.set_value_with_priority({property}, rt::exact(value_0.clone(), \"ContentControl.Content\", 1, 28, 33)?, priority_1);"),
        "}",
    ]);
    assert_eq!(resource_choice("value_0", &binding), None);
    // Two typed setters.
    let target = "path_0.upcast_ref::<::ferroui_base::FerroObject>()";
    let two = chain(&[
        &format!("if rt::is_instance(&value_0, {BINDING}) {{"),
        &format!("    rt::bind({target}, {property}, value_0.clone(), 1, 2)?;"),
        "} else if rt::is_instance(&value_0, rt::class_handle(<::ferroui_base::media::Geometry as ::ferroui_base::StaticType>::TYPE)) {",
        &format!("    path_0.set_value({property}, rt::exact(value_0.clone(), \"A\", 0, 1, 2)?);"),
        "} else if rt::is_instance(&value_0, rt::class_handle(<::ferroui_base::media::Brush as ::ferroui_base::StaticType>::TYPE)) {",
        &format!("    path_0.set_value({property}, rt::exact(value_0.clone(), \"B\", 0, 1, 2)?);"),
        "} else {",
        "    return ::core::result::Result::Err(rt::no_setter(\"Data\", &value_0, 1, 2));",
        "}",
    ]);
    assert_eq!(resource_choice("value_0", &two), None);
    // Positions that differ.
    let moved = chain(&[
        &format!("if rt::is_instance(&value_0, {BINDING}) {{"),
        &format!("    rt::bind({target}, {property}, value_0.clone(), 1, 2)?;"),
        "} else if rt::is_instance(&value_0, rt::class_handle(<::ferroui_base::media::Geometry as ::ferroui_base::StaticType>::TYPE)) {",
        &format!("    path_0.set_value({property}, rt::exact(value_0.clone(), \"A\", 0, 1, 2)?);"),
        "} else {",
        "    return ::core::result::Result::Err(rt::no_setter(\"Data\", &value_0, 3, 4));",
        "}",
    ]);
    assert_eq!(resource_choice("value_0", &moved), None);
}
