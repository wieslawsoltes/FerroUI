//! Tests of the row tables of theme resources ([`super`]). Not tests of upstream: each builder
//! is compared with the statements of generated code it replaces (the build functions and the
//! additions the emitter wrote before #568), written out here as the emitter wrote them.

use super::*;
use crate::xaml_il::runtime::compiled::{add_deferred_resource, populate_context, service_provider};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::BoxedValue;

const NAMESPACES: XmlNamespaceTable =
    &[("", &[("FerroUI.Controls", "FerroUI.Controls", &[]), ("FerroUI.Media", "FerroUI.Base", &[])]), ("x", &[])];

/// The namespaces of a declaration that imports another prefix than the document.
const SCOPED: XmlNamespaceTable = &[("", &[("FerroUI.Media", "FerroUI.Base", &[])]), ("local", &[("Tests.Views", "Tests", &[])])];

static DOCUMENT: DocumentInfo = DocumentInfo { base_uri: Some("ferres://tests/Theme.xaml"), namespaces: NAMESPACES };

const ACCENT: u32 = 0xFF33_6699;

/// A dictionary being populated, as generated code populates one: the context of the document
/// with the dictionary as its root and on its parent stack, and the resources the rows refer to.
fn populated() -> (Ref<ResourceDictionary>, Rc<XamlIlContext>) {
    crate::register_types();
    let dictionary = ResourceDictionary::new();
    let context = populate_context(None, &DOCUMENT, to_value(dictionary.clone()));
    context.push_parent(to_value(dictionary.clone()));
    dictionary.add("Accent", Some(Rc::new(Color::from_uint32(ACCENT)) as BoxedValue));
    dictionary.add("Text", Some(Rc::new(String::from("not a colour")) as BoxedValue));
    (dictionary, context)
}

/// The resource `key` of `dictionary`, built if it is deferred.
fn resource(dictionary: &Ref<ResourceDictionary>, key: &str) -> BoxedValue {
    dictionary.try_get_value(&ResourceKey::from(key)).flatten().unwrap_or_else(|| panic!("the resource {key}"))
}

fn deferred(dictionary: &Ref<ResourceDictionary>, key: &str) -> bool {
    dictionary.contains_deferred_key(&ResourceKey::from(key))
}

fn brush_color(value: &BoxedValue) -> Color {
    ValueTypes::as_object(&**value)
        .and_then(|object| object.cast::<SolidColorBrush>())
        .expect("a solid colour brush")
        .color()
}

fn at_position(error: &XamlLoadError, line: i32, position: i32) -> bool {
    error.message().ends_with(&format!("Line {line}, position {position}. (line {line} position {position})"))
}

// --- resource aliases ---------------------------------------------------------------------

static ALIASES: &[ResourceAliasRow] = &[
    ResourceAliasRow { key: "First", target: "Accent", namespaces: None, line: 3, position: 5, value_line: 3, value_position: 40 },
    ResourceAliasRow { key: "Second", target: "Text", namespaces: Some(SCOPED), line: 4, position: 5, value_line: 4, value_position: 40 },
];

#[test]
fn resource_aliases_are_deferred_and_build_the_resource_they_name() {
    let (dictionary, context) = populated();
    add_resource_aliases(&dictionary, &context, &DOCUMENT, ALIASES).expect("the aliases are added");
    assert_eq!(dictionary.count(), 4);
    assert!(deferred(&dictionary, "First") && deferred(&dictionary, "Second"));
    assert_eq!(resource(&dictionary, "First").downcast_ref::<Color>(), Some(&Color::from_uint32(ACCENT)));
    assert_eq!(resource(&dictionary, "Second").downcast_ref::<String>().map(String::as_str), Some("not a colour"));
    // Built on first lookup, then held as the value.
    assert!(!deferred(&dictionary, "First") && !deferred(&dictionary, "Second"));
}

#[test]
fn a_failed_alias_is_the_error_of_the_static_resource_at_the_value_position() {
    static MISSING: ResourceAliasRow =
        ResourceAliasRow { key: "Alias", target: "Missing", namespaces: None, line: 6, position: 5, value_line: 7, value_position: 9 };
    let (_dictionary, context) = populated();
    let provider = service_provider(&context);
    let error = build_resource_alias(&provider, &DOCUMENT, &MISSING).err().expect("the target is missing");
    // The statement of the build function the emitter wrote for the alias.
    let expected = static_resource_content(&provider, &DOCUMENT, "Missing", 7, 9).err().expect("the target is missing");
    assert_eq!(error.message(), expected.message());
    assert!(at_position(&error, 7, 9), "{}", error.message());
}

#[test]
fn rows_are_added_in_order_and_the_first_failure_stops_the_run() {
    static DUPLICATE: &[ResourceAliasRow] = &[
        ResourceAliasRow { key: "One", target: "Accent", namespaces: None, line: 1, position: 2, value_line: 1, value_position: 3 },
        ResourceAliasRow { key: "One", target: "Text", namespaces: None, line: 5, position: 6, value_line: 5, value_position: 7 },
        ResourceAliasRow { key: "Two", target: "Accent", namespaces: None, line: 8, position: 9, value_line: 8, value_position: 10 },
    ];
    let (dictionary, context) = populated();
    let error = add_resource_aliases(&dictionary, &context, &DOCUMENT, DUPLICATE).expect_err("a key is added twice");
    assert!(error.message().contains("An item with the same key has already been added"), "{}", error.message());
    assert!(at_position(&error, 5, 6), "{}", error.message());
    // The first row was added and kept its build; the row after the failure was not added.
    assert_eq!(resource(&dictionary, "One").downcast_ref::<Color>(), Some(&Color::from_uint32(ACCENT)));
    assert!(dictionary.try_get_value(&ResourceKey::from("Two")).is_none());
}

// --- brushes ------------------------------------------------------------------------------

/// The build function the emitter wrote for a brush with a literal colour.
fn generated_literal_brush(service_provider: &Rc<dyn IServiceProvider>) -> Result<MarkupValue, XamlLoadError> {
    let _context = deferred_context(service_provider, &DOCUMENT);
    let solid_color_brush_0 = SolidColorBrush::new();
    let value_0 = Color::__markup_FromUInt32_2(0x8010_2030);
    solid_color_brush_0.set_value(SolidColorBrush::color_property(), value_0);
    Ok(to_value(solid_color_brush_0.clone()))
}

#[test]
fn literal_brushes_build_a_brush_of_their_colour() {
    static BRUSHES: &[LiteralBrushRow] = &[
        LiteralBrushRow { key: "Literal", color: 0x8010_2030, namespaces: None, line: 2, position: 8 },
        LiteralBrushRow { key: "Scoped", color: ACCENT, namespaces: Some(SCOPED), line: 3, position: 8 },
    ];
    let (dictionary, context) = populated();
    add_literal_brushes(&dictionary, &context, &DOCUMENT, BRUSHES).expect("the brushes are added");
    assert!(deferred(&dictionary, "Literal") && deferred(&dictionary, "Scoped"));
    add_deferred_resource(&dictionary, "Reference", ValueType::object(), &context, generated_literal_brush, 2, 8)
        .expect("the reference is added");
    assert_eq!(brush_color(&resource(&dictionary, "Literal")), brush_color(&resource(&dictionary, "Reference")));
    assert_eq!(brush_color(&resource(&dictionary, "Literal")), Color::from_uint32(0x8010_2030));
    assert_eq!(brush_color(&resource(&dictionary, "Scoped")), Color::from_uint32(ACCENT));
}

/// The build function the emitter wrote for a brush bound to the dynamic resource `Accent`.
fn generated_dynamic_brush(service_provider: &Rc<dyn IServiceProvider>) -> Result<MarkupValue, XamlLoadError> {
    let context = deferred_context(service_provider, &DOCUMENT);
    let solid_color_brush_0 = SolidColorBrush::new();
    context.push_parent(to_value(solid_color_brush_0.clone()));
    bind_dynamic_resource(&context, solid_color_brush_0.upcast_ref::<FerroObject>(), SolidColorBrush::color_property(), "Accent", 4, 60)?;
    context.pop_parent();
    Ok(to_value(solid_color_brush_0.clone()))
}

#[test]
fn dynamic_brushes_bind_their_colour_as_the_generated_build_does() {
    static BRUSHES: &[DynamicBrushRow] = &[DynamicBrushRow {
        key: "Dynamic", color_key: "Accent", namespaces: None, line: 4, position: 8, value_line: 4, value_position: 60,
    }];
    let (dictionary, context) = populated();
    add_dynamic_brushes(&dictionary, &context, &DOCUMENT, BRUSHES).expect("the brush is added");
    assert!(deferred(&dictionary, "Dynamic"));
    add_deferred_resource(&dictionary, "Reference", ValueType::object(), &context, generated_dynamic_brush, 4, 8)
        .expect("the reference is added");
    let built = resource(&dictionary, "Dynamic");
    let reference = resource(&dictionary, "Reference");
    assert_eq!(brush_color(&built), brush_color(&reference));
}

/// The build function the emitter wrote for a brush whose colour is `{StaticResource key}`.
fn generated_static_brush(service_provider: &Rc<dyn IServiceProvider>, key: &str) -> Result<MarkupValue, XamlLoadError> {
    let context = deferred_context(service_provider, &DOCUMENT);
    let solid_color_brush_0 = SolidColorBrush::new();
    context.push_parent(to_value(solid_color_brush_0.clone()));
    let static_resource_extension_0 = StaticResourceExtension::__markup_new_1(string_object(key));
    let provided_0 = provide_value_invoked(
        &context,
        property_value(SolidColorBrush::color_property()),
        &static_resource_extension_0,
        StaticResourceExtension::__markup_ProvideValue_0,
        9,
        70,
    )?;
    let value_0: MarkupValue = to_value(provided_0);
    if is_instance(&value_0, markup_handle(<UnsetValueType as MarkupTyped>::MARKUP)) {
        unset_value(solid_color_brush_0.upcast_ref::<FerroObject>(), SolidColorBrush::color_property());
    } else if is_instance(&value_0, markup_handle(<dyn BindingBase as MarkupTyped>::MARKUP)) {
        bind(solid_color_brush_0.upcast_ref::<FerroObject>(), SolidColorBrush::color_property(), value_0.clone(), 9, 70)?;
    } else if is_instance(&value_0, markup_handle(<Color as MarkupTyped>::MARKUP)) {
        solid_color_brush_0
            .set_value(SolidColorBrush::color_property(), exact(value_0.clone(), "FerroUI.Media.SolidColorBrush.set_Color", 0, 9, 70)?);
    } else {
        return Err(no_setter("Color", &value_0, 9, 70));
    }
    context.pop_parent();
    Ok(to_value(solid_color_brush_0.clone()))
}

fn static_row(color_key: &'static str) -> StaticBrushRow {
    StaticBrushRow { key: "Static", color_key, namespaces: None, line: 9, position: 8, value_line: 9, value_position: 70 }
}

#[test]
fn static_brushes_set_the_colour_they_look_up() {
    static BRUSHES: &[StaticBrushRow] = &[StaticBrushRow {
        key: "Static", color_key: "Accent", namespaces: None, line: 9, position: 8, value_line: 9, value_position: 70,
    }];
    let (dictionary, context) = populated();
    add_static_brushes(&dictionary, &context, &DOCUMENT, BRUSHES).expect("the brush is added");
    assert!(deferred(&dictionary, "Static"));
    assert_eq!(brush_color(&resource(&dictionary, "Static")), Color::from_uint32(ACCENT));
}

#[test]
fn a_static_brush_fails_with_the_errors_of_the_generated_build() {
    let (_dictionary, context) = populated();
    let provider = service_provider(&context);
    // A value no setter of the colour takes, then a key that is not found.
    for key in ["Text", "Missing"] {
        let row = static_row(key);
        let error = build_static_brush(&provider, &DOCUMENT, &row).err().expect("the build fails");
        let expected = generated_static_brush(&provider, key).err().expect("the generated build fails");
        assert_eq!(error.message(), expected.message());
        assert!(at_position(&error, 9, 70), "{}", error.message());
    }
    let error = build_static_brush(&provider, &DOCUMENT, &static_row("Text")).err().expect("no setter");
    assert!(error.message().starts_with("No setter of property Color accepts a value of type "), "{}", error.message());
}

// --- plain values -------------------------------------------------------------------------

#[test]
fn plain_values_are_added_with_their_types() {
    let (dictionary, _context) = populated();
    add_double_resources(&dictionary, &[
        ValueRow { key: "Width", value: 24.0, line: 1, position: 1 },
        ValueRow { key: "Opacity", value: 0.5, line: 2, position: 1 },
    ])
    .expect("the numbers are added");
    add_thickness_resources(&dictionary, &[ValueRow { key: "Padding", value: [1.0, 2.0, 3.0, 4.0], line: 3, position: 1 }])
        .expect("the thickness is added");
    add_corner_radius_resources(&dictionary, &[ValueRow { key: "Radius", value: [4.0, 4.0, 0.0, 0.0], line: 4, position: 1 }])
        .expect("the corner radius is added");
    add_color_resources(&dictionary, &[ValueRow { key: "Colour", value: ACCENT, line: 5, position: 1 }]).expect("the colour is added");
    add_string_resources(&dictionary, &[ValueRow { key: "Spline", value: "0,0,0,1", line: 6, position: 1 }])
        .expect("the text is added");
    assert_eq!(resource(&dictionary, "Width").downcast_ref::<f64>(), Some(&24.0));
    assert_eq!(resource(&dictionary, "Opacity").downcast_ref::<f64>(), Some(&0.5));
    assert_eq!(resource(&dictionary, "Padding").downcast_ref::<Thickness>(), Some(&Thickness::new(1.0, 2.0, 3.0, 4.0)));
    assert_eq!(resource(&dictionary, "Radius").downcast_ref::<CornerRadius>(), Some(&CornerRadius::new(4.0, 4.0, 0.0, 0.0)));
    assert_eq!(resource(&dictionary, "Colour").downcast_ref::<Color>(), Some(&Color::from_uint32(ACCENT)));
    assert_eq!(resource(&dictionary, "Spline").downcast_ref::<String>().map(String::as_str), Some("0,0,0,1"));
    assert!(!deferred(&dictionary, "Width"));
}

#[test]
fn a_plain_value_row_is_the_statement_the_emitter_wrote() {
    let (dictionary, _context) = populated();
    let reference = ResourceDictionary::new();
    // The statement of an addition of a number, as the emitter wrote it.
    invoked(
        <dyn IResourceDictionary>::__markup_Add_0(&cast(reference.clone(), 1, 2).unwrap(), string_object("Width"), to_object(24.0_f64)),
        1,
        2,
    )
    .expect("the reference is added");
    add_double_resources(&dictionary, &[ValueRow { key: "Width", value: 24.0, line: 1, position: 2 }]).expect("the row is added");
    let built = resource(&dictionary, "Width");
    let expected = resource(&reference, "Width");
    assert_eq!(built.value_type_id(), expected.value_type_id());
    assert_eq!(built.downcast_ref::<f64>(), expected.downcast_ref::<f64>());
}

#[test]
fn a_plain_value_added_twice_is_the_error_at_its_row() {
    let (dictionary, _context) = populated();
    let error = add_thickness_resources(&dictionary, &[
        ValueRow { key: "Margin", value: [1.0; 4], line: 1, position: 2 },
        ValueRow { key: "Margin", value: [2.0; 4], line: 3, position: 4 },
        ValueRow { key: "After", value: [3.0; 4], line: 5, position: 6 },
    ])
    .expect_err("a key is added twice");
    assert!(error.message().contains("An item with the same key has already been added"), "{}", error.message());
    assert!(at_position(&error, 3, 4), "{}", error.message());
    assert_eq!(resource(&dictionary, "Margin").downcast_ref::<Thickness>(), Some(&Thickness::new(1.0, 1.0, 1.0, 1.0)));
    assert!(dictionary.try_get_value(&ResourceKey::from("After")).is_none());
}
