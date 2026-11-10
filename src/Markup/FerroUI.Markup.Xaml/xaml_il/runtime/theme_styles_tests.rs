//! Tests of the tables of control themes ([`super`]). Not tests of upstream: each interpreter is
//! compared with the statements of generated code it replaces (the factories, selector chains,
//! setter choices and template bindings the emitter wrote before #568, step 5), written out here
//! as the emitter wrote them.

use super::*;
use crate::xaml_il::runtime::compiled::{populate_context, DocumentInfo, XmlNamespaceTable};
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::media::{Color, SolidColorBrush};
use ferroui_base::metadata::IServiceProvider;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::{Border, Button};
use std::cell::RefCell;

const NAMESPACES: XmlNamespaceTable =
    &[("", &[("FerroUI.Controls", "FerroUI.Controls", &[]), ("FerroUI.Media", "FerroUI.Base", &[])]), ("x", &[])];

static DOCUMENT: DocumentInfo = DocumentInfo { base_uri: Some("ferres://tests/Controls.xaml"), namespaces: NAMESPACES };

/// The context of a build of a control theme: the context of the document with a dictionary as
/// its root and on its parent stack, which holds the resources the tables name.
fn context() -> Rc<XamlIlContext> {
    crate::register_types();
    let dictionary = ResourceDictionary::new();
    let context = populate_context(None, &DOCUMENT, to_value(dictionary.clone()));
    context.push_parent(to_value(dictionary.clone()));
    dictionary.add("Accent", Some(Rc::new(Color::from_uint32(0xFF33_6699)) as BoxedValue));
    context
}

fn at_position(error: &XamlLoadError, line: i32, position: i32) -> bool {
    error.message().ends_with(&format!("Line {line}, position {position}. (line {line} position {position})"))
}

fn background() -> &'static FerroProperty {
    TemplatedControl::background_property()
}

fn padding() -> &'static FerroProperty {
    TemplatedControl::padding_property()
}

fn template_property() -> &'static FerroProperty {
    TemplatedControl::template_property()
}

fn border_background() -> &'static FerroProperty {
    Border::background_property()
}

// --- selectors ----------------------------------------------------------------------------

fn button_type() -> &'static TypeInfo {
    <Button as StaticType>::TYPE
}

static CHAIN: &[SelectorStep] = &[
    SelectorStep::Nesting,
    SelectorStep::Class(":pointerover"),
    SelectorStep::Template,
    SelectorStep::OfType(<Border as StaticType>::TYPE),
    SelectorStep::Name("PART_Border"),
];

#[test]
fn a_chain_builds_the_selector_of_the_statements_it_replaces() {
    let selector = build_selector(CHAIN).expect("a selector");
    // The statements the emitter wrote for the chain.
    let selector_0 = Selectors::nesting(None);
    let selector_1 = Selectors::class(Some(selector_0), ":pointerover");
    let selector_2 = Selectors::template(Some(selector_1));
    let selector_3 = Selectors::of_type_info(Some(selector_2), <Border as StaticType>::TYPE);
    let selector_4 = Selectors::name(Some(selector_3), "PART_Border");
    assert_eq!(selector.to_string(), selector_4.to_string());
    assert!(build_selector(&[]).is_none());
}

fn enabled() -> BoxedValue {
    Rc::new(true)
}

fn is_enabled() -> &'static FerroProperty {
    ferroui_base::input::InputElement::is_enabled_property()
}

static NOT: &[SelectorStep] = &[SelectorStep::Nesting, SelectorStep::Not { argument: &[SelectorStep::Class(":pressed")] }];

static OR: &[SelectorStep] = &[
    SelectorStep::Or {
        alternatives: &[&[SelectorStep::Nesting, SelectorStep::Class(":pressed")], &[SelectorStep::Nesting, SelectorStep::Class(":checked")]],
    },
    SelectorStep::Descendant,
    SelectorStep::Is(<Border as StaticType>::TYPE),
];

static EQUALS: &[SelectorStep] = &[
    SelectorStep::Nesting,
    SelectorStep::PropertyEquals { property: is_enabled, value: enabled },
    SelectorStep::Child,
    SelectorStep::NthChild(2, 1),
    SelectorStep::NthLastChild(-1, 3),
];

#[test]
fn not_or_and_property_steps_build_their_arguments_in_order() {
    let not = Selectors::not(Some(Selectors::nesting(None)), Selectors::class(None, ":pressed"));
    assert_eq!(build_selector(NOT).expect("a selector").to_string(), not.to_string());
    let or = Selectors::or([
        Selectors::class(Some(Selectors::nesting(None)), ":pressed"),
        Selectors::class(Some(Selectors::nesting(None)), ":checked"),
    ]);
    let or = Selectors::is_type_info(Some(Selectors::descendant(Some(or))), <Border as StaticType>::TYPE);
    assert_eq!(build_selector(OR).expect("a selector").to_string(), or.to_string());
    let equals = Selectors::property_equals_untyped(
        Some(Selectors::nesting(None)),
        ferroui_base::input::InputElement::is_enabled_property(),
        Rc::new(true),
    );
    let equals = Selectors::nth_last_child(Some(Selectors::nth_child(Some(Selectors::child(Some(equals))), 2, 1)), -1, 3);
    assert_eq!(build_selector(EQUALS).expect("a selector").to_string(), equals.to_string());
}

// --- control themes -----------------------------------------------------------------------

fn alignment() -> MarkupValue {
    to_object(ferroui_base::layout::HorizontalAlignment::Left)
}

fn computed(context: &Rc<XamlIlContext>) -> Result<MarkupValue, XamlLoadError> {
    let _ = context;
    Ok(to_object(Thickness::__markup_new_3(1.0, 2.0, 3.0, 4.0)))
}

fn template_content(_service_provider: &Rc<dyn IServiceProvider>) -> Result<MarkupValue, XamlLoadError> {
    Ok(None)
}

fn horizontal_alignment() -> &'static FerroProperty {
    ferroui_base::layout::Layoutable::horizontal_alignment_property()
}

static THEME: ControlThemeTable = ControlThemeTable {
    parent: true,
    members: &[
        StyleMember::TargetType(<Button as StaticType>::TYPE),
        StyleMember::Setter(ThemeSetter { property: background, form: SetterForm::Plain, value: ThemeValue::Dynamic("Accent") }),
        StyleMember::Setter(ThemeSetter { property: horizontal_alignment, form: SetterForm::Plain, value: ThemeValue::Expression(alignment) }),
        StyleMember::Setter(ThemeSetter { property: padding, form: SetterForm::Created, value: ThemeValue::Computed(computed) }),
        StyleMember::Setter(ThemeSetter {
            property: template_property,
            form: SetterForm::Parented,
            value: ThemeValue::Template(ThemeTemplate {
                target_type: Some(<Button as StaticType>::TYPE),
                content_type: <ferroui_controls::Control as StaticType>::TYPE,
                build: template_content,
                parent: true,
                line: 9,
                position: 10,
            }),
        }),
        StyleMember::Style {
            parent: true,
            selector: Some(CHAIN),
            members: &[
                StyleMember::Setter(ThemeSetter { property: border_background, form: SetterForm::Parented, value: ThemeValue::Static {
                    key: "Accent",
                    line: 12,
                    position: 20,
                } }),
                StyleMember::Style {
                    parent: false,
                    selector: Some(NOT),
                    members: &[StyleMember::Setter(ThemeSetter { property: padding, form: SetterForm::Plain, value: ThemeValue::Double(2.0) })],
                    line: 13,
                    position: 8,
                },
            ],
            line: 11,
            position: 6,
        },
    ],
};

fn theme_of(value: &MarkupValue) -> Ref<ControlTheme> {
    let value = value.as_ref().expect("a control theme");
    ValueTypes::as_object(&**value).and_then(|object| object.cast::<ControlTheme>()).expect("a control theme")
}

fn setter_properties(style: &Ref<StyleBase>) -> Vec<String> {
    style
        .setters()
        .to_vec()
        .iter()
        .map(|setter| {
            let setter = setter.as_any().and_then(|any| any.downcast_ref::<Setter>()).expect("a setter");
            setter.property().map(|property| property.name().to_string()).unwrap_or_default()
        })
        .collect()
}

fn nested_style(style: &Ref<StyleBase>, index: usize) -> Ref<Style> {
    let child = style.children().get(index);
    child.as_object().and_then(|object| object.to_ref().cast::<Style>()).expect("a nested style")
}

#[test]
fn a_table_builds_the_theme_its_factory_built() {
    let context = context();
    let value = build_control_theme(&context, &THEME).expect("the theme is built");
    let theme = theme_of(&value);
    assert_eq!(theme.target_type().map(TypeInfo::name), Some(button_type().name()));
    assert_eq!(setter_properties(theme.upcast_ref::<StyleBase>()), ["Background", "HorizontalAlignment", "Padding", "Template"]);
    assert_eq!(theme.children().count(), 1);
    let style = nested_style(theme.upcast_ref::<StyleBase>(), 0);
    assert_eq!(style.selector().map(|selector| selector.to_string()), build_selector(CHAIN).map(|selector| selector.to_string()));
    assert_eq!(setter_properties(style.upcast_ref::<StyleBase>()), ["Background"]);
    let inner = nested_style(style.upcast_ref::<StyleBase>(), 0);
    assert_eq!(setter_properties(inner.upcast_ref::<StyleBase>()), ["Padding"]);
    // The theme, the template and the setters were popped: the dictionary is the parent again.
    assert_eq!(context.parents_stack().len(), 1);
}

/// The statements the emitter wrote for `BasedOn="{StaticResource Missing}"` at (3, 40).
fn generated_based_on(context: &Rc<XamlIlContext>, theme: &Ref<ControlTheme>) -> Result<(), XamlLoadError> {
    let static_resource_extension_0 = StaticResourceExtension::__markup_new_1(string_object("Missing"));
    let provided_0 = provide_value_invoked(
        context,
        boxed(clr_property_info(
            class_markup(<ControlTheme as StaticType>::TYPE),
            "BasedOn",
            false,
            class_handle(<ControlTheme as StaticType>::TYPE),
            false,
        )),
        &static_resource_extension_0,
        StaticResourceExtension::__markup_ProvideValue_0,
        3,
        40,
    )?;
    ControlTheme::__markup_set_BasedOn(
        theme,
        exact(
            cast_checked(provided_0, class_handle(<ControlTheme as StaticType>::TYPE), "FerroUI.Styling.ControlTheme", 3, 40)?,
            "FerroUI.Styling.ControlTheme.set_BasedOn",
            0,
            3,
            40,
        )?,
    );
    Ok(())
}

#[test]
fn a_failed_member_is_the_error_of_its_statements() {
    static MISSING: ControlThemeTable = ControlThemeTable {
        parent: true,
        members: &[
            StyleMember::TargetType(<Button as StaticType>::TYPE),
            StyleMember::BasedOn { key: ThemeKey::Text("Missing"), line: 3, position: 40 },
        ],
    };
    let error = build_control_theme(&context(), &MISSING).err().expect("the resource is missing");
    let expected = generated_based_on(&context(), &ControlTheme::new()).err().expect("the resource is missing");
    assert_eq!(error.message(), expected.message());
    assert!(at_position(&error, 3, 40), "{}", error.message());
}

#[test]
fn a_missing_static_resource_of_a_setter_is_the_error_at_its_position() {
    static STATIC: ControlThemeTable = ControlThemeTable {
        parent: false,
        members: &[StyleMember::Setter(ThemeSetter {
            property: background,
            form: SetterForm::Created,
            value: ThemeValue::Static { key: "Missing", line: 5, position: 7 },
        })],
    };
    let error = build_control_theme(&context(), &STATIC).err().expect("the resource is missing");
    assert!(at_position(&error, 5, 7), "{}", error.message());
}

// --- the setter of a resource value -------------------------------------------------------

const COLOR_CHOICE: ResourceChoice = ResourceChoice {
    name: "Color",
    member: "FerroUI.Media.SolidColorBrush.set_Color",
    index: 0,
    unset: true,
    instance: ValueHandle::Markup(<Color as MarkupTyped>::MARKUP),
    null: false,
    line: 21,
    position: 60,
};

#[test]
fn a_resource_value_is_assigned_through_the_setter_generated_code_chose() {
    crate::register_types();
    let brush = SolidColorBrush::new();
    let target = brush.upcast_ref::<FerroObject>();
    let color = Color::from_uint32(0xFF10_2030);
    assign_resource_value(target, SolidColorBrush::color_property(), to_object(color), &COLOR_CHOICE, None).expect("a colour");
    assert_eq!(brush.color(), color);
    // The unset marker clears the colour.
    assign_resource_value(target, SolidColorBrush::color_property(), to_object(UnsetValueType), &COLOR_CHOICE, None)
        .expect("the unset marker");
    assert!(!brush.is_set(SolidColorBrush::color_property()));
}

#[test]
fn a_value_no_setter_takes_is_the_error_of_no_setter() {
    crate::register_types();
    let brush = SolidColorBrush::new();
    let target = brush.upcast_ref::<FerroObject>();
    let text = string_object("not a colour");
    let error = assign_resource_value(target, SolidColorBrush::color_property(), text.clone(), &COLOR_CHOICE, None)
        .expect_err("a text is no colour");
    assert_eq!(error.message(), no_setter("Color", &text, 21, 60).message());
    // Null is taken only by a setter that allows it.
    let error = assign_resource_value(target, SolidColorBrush::color_property(), None, &COLOR_CHOICE, None).expect_err("null");
    assert_eq!(error.message(), no_setter("Color", &None, 21, 60).message());
}

// --- template bindings --------------------------------------------------------------------

ferroui_base::ferro_thread_local! {
    static READ: RefCell<Vec<&'static str>> = RefCell::new(Vec::new());
}

fn read(name: &'static str) {
    READ.with(|read| read.borrow_mut().push(name));
}

fn target_background() -> &'static FerroProperty {
    read("target background");
    Border::background_property()
}

fn source_background() -> &'static FerroProperty {
    read("source background");
    TemplatedControl::background_property()
}

fn target_padding() -> &'static FerroProperty {
    read("target padding");
    ferroui_controls::Decorator::padding_property()
}

fn source_padding() -> &'static FerroProperty {
    read("source padding");
    TemplatedControl::padding_property()
}

#[test]
fn template_bindings_are_bound_in_order() {
    crate::register_types();
    static ROWS: &[TemplateBindingRow] = &[
        TemplateBindingRow { target: target_background, source: source_background, line: 3, position: 9 },
        TemplateBindingRow { target: target_padding, source: source_padding, line: 4, position: 9 },
    ];
    READ.with(|read| read.borrow_mut().clear());
    let border = Border::new();
    bind_template_bindings(border.upcast_ref::<FerroObject>(), ROWS).expect("the bindings are bound");
    assert_eq!(
        READ.with(|read| read.borrow().clone()),
        ["target background", "source background", "target padding", "source padding"]
    );
}

fn candidate_width() -> &'static FerroProperty {
    ferroui_base::layout::Layoutable::width_property()
}

#[test]
fn generated_type_candidates_keep_template_activators_and_selector_replacement() {
    use ferroui_base::styling::IStyle;
    static MISS: &[SelectorStep] = &[SelectorStep::Nesting, SelectorStep::Template, SelectorStep::OfType(Button::TYPE)];
    static HIT: &[SelectorStep] = &[SelectorStep::Nesting, SelectorStep::Template, SelectorStep::OfType(Border::TYPE), SelectorStep::Class(":active")];
    let rule = |selector| StyleMember::Style { parent: false, selector: Some(selector), members: &[
        StyleMember::Setter(ThemeSetter { property: candidate_width, form: SetterForm::Plain, value: ThemeValue::Double(42.0) }),
    ], line: 1, position: 1 };
    let mut members = vec![StyleMember::TargetType(Button::TYPE)];
    members.extend((0..8).map(|_| rule(MISS)));
    members.push(rule(HIT));
    let table = ControlThemeTable { parent: false, members: Box::leak(members.into_boxed_slice()) };
    let theme = theme_of(&build_control_theme(&context(), &table).unwrap());
    let handle: Rc<dyn IStyle> = theme.clone().into();
    let target = Border::new();
    assert_eq!(handle.children_for_target(&target).count(), 1);
    let last = nested_style(theme.upcast_ref::<StyleBase>(), 8);
    let original = last.selector();
    Style::__markup_set_Selector(&last, build_selector(MISS));
    // Replacing a generated selector deoptimizes its cached exclusions.
    let button = Button::new();
    assert_eq!(handle.children_for_target(&button).count(), 9);
    Style::__markup_set_Selector(&last, original);
    last.enable_selector_index();
    assert_eq!(handle.children_for_target(&button).count(), 8);
    assert_eq!(handle.children_for_target(&target).count(), 1);
    let parent = Button::new();
    parent.set_theme(theme);
    target.set_templated_parent(parent.clone());
    target.apply_styling();
    // An absent pseudo-class still installs an activator; both state changes
    // must be observed by the original selector after candidate filtering.
    assert!(target.width().is_nan());
    target.pseudo_classes().add_pseudo(":active");
    assert_eq!(target.width(), 42.0);
    assert!(last.selector().unwrap().match_(&target, last.parent().as_deref(), false).is_match());
    target.pseudo_classes().remove_pseudo(":active");
    assert!(target.width().is_nan());
    assert!(!last.selector().unwrap().match_(&target, last.parent().as_deref(), false).is_match());
}
