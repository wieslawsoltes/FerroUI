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

static IDENTITY_BRUSHES: &[LiteralBrushRow] = &[
    LiteralBrushRow { key: "Identity", color: ACCENT, namespaces: None, line: 2, position: 8 },
];

#[test]
fn compiled_rows_capture_each_declaration_after_reentrant_registration_notifications() {
    use ferroui_controls::Border;
    use std::cell::Cell;
    static ROWS: &[ResourceAliasRow] = &[
        ResourceAliasRow { key: "Before", target: "Accent", namespaces: None, line: 1, position: 2, value_line: 1, value_position: 10 },
        ResourceAliasRow { key: "After", target: "Accent", namespaces: None, line: 2, position: 2, value_line: 2, value_position: 10 },
    ];
    crate::register_types();
    let host = Border::new();
    let dictionary = host.resources();
    dictionary.add("Accent", Some(Rc::new(Color::from_uint32(ACCENT))));
    let context = populate_context(None, &DOCUMENT, to_value(dictionary.clone()));
    context.push_parent(to_value(dictionary.clone()));
    let nearer = ResourceDictionary::new();
    nearer.add("Accent", Some(Rc::new(Color::from_uint32(0xFF00_0000))));
    let done = Rc::new(Cell::new(false));
    let notification_context = context.clone();
    let weak = dictionary.downgrade();
    let _subscription = host.resources_changed(move |_| {
        if !done.replace(true) {
            let dictionary = weak.upgrade().unwrap();
            // An observer can realize the just-inserted row immediately.
            assert_eq!(resource(&dictionary, "Before").downcast_ref::<Color>(), Some(&Color::from_uint32(ACCENT)));
            notification_context.push_parent(to_value(nearer.clone()));
        }
    });
    add_resource_aliases(&dictionary, &context, &DOCUMENT, ROWS).unwrap();
    assert_eq!(resource(&dictionary, "After").downcast_ref::<Color>(), Some(&Color::from_uint32(0xFF00_0000)));
}

#[test]
fn compiled_dynamic_brush_keeps_its_resource_subscription_after_realization() {
    use ferroui_controls::Border;
    static ROWS: &[DynamicBrushRow] = &[
        DynamicBrushRow { key: "Brush", color_key: "Accent", namespaces: None, line: 1, position: 2, value_line: 1, value_position: 10 },
    ];
    crate::register_types();
    let host = Border::new();
    let dictionary = host.resources();
    dictionary.add("Accent", Some(Rc::new(Color::from_uint32(ACCENT))));
    let context = populate_context(None, &DOCUMENT, to_value(host.clone()));
    context.push_parent(to_value(host.clone()));
    context.push_parent(to_value(dictionary.clone()));
    add_dynamic_brushes(&dictionary, &context, &DOCUMENT, ROWS).unwrap();
    let value = resource(&dictionary, "Brush");
    assert_eq!(brush_color(&value), Color::from_uint32(ACCENT));
    drop(context);
    dictionary.set("Accent", Some(Rc::new(Color::from_uint32(0xFF00_0000))));
    assert_eq!(brush_color(&value), Color::from_uint32(0xFF00_0000));
    assert!(Rc::ptr_eq(&value, &resource(&dictionary, "Brush")));
}

#[cfg(not(any(feature = "source-overlays", all(feature = "hot-reload", debug_assertions))))]
#[test]
fn compiled_rows_release_owned_plain_roots_per_resource_in_mixed_groups() {
    use ferroui_controls::Border;
    use std::cell::Cell;
    #[derive(PartialEq)]
    struct Root { id: u8, drops: Rc<Cell<usize>> }
    impl Drop for Root { fn drop(&mut self) { self.drops.set(self.drops.get()+1); } }
    static ROWS: &[LiteralBrushRow] = &[
        LiteralBrushRow { key: "Before", color: ACCENT, namespaces: None, line: 1, position: 2 },
        LiteralBrushRow { key: "Middle", color: 0xFF00_0000, namespaces: None, line: 2, position: 2 },
        LiteralBrushRow { key: "After", color: ACCENT, namespaces: None, line: 3, position: 2 },
    ];
    crate::register_types();
    let host = Border::new();
    let dictionary = host.resources();
    let drops = Rc::new(Cell::new(0));
    let first: BoxedValue = Rc::new(Root { id: 1, drops: drops.clone() });
    let second: BoxedValue = Rc::new(Root { id: 2, drops: drops.clone() });
    let weak = Rc::downgrade(&first);
    let context = populate_context(None, &DOCUMENT, Some(first.clone()));
    context.set_intermediate_root_object(None);
    context.push_parent(to_value(dictionary.clone()));
    let notification_context = context.clone();
    let dictionary_weak = dictionary.downgrade();
    let events = Cell::new(0);
    let _subscription = host.resources_changed(move |_| {
        let event = events.get();
        events.set(event + 1);
        match event {
            0 => notification_context.set_root_object(to_value(dictionary_weak.upgrade().unwrap())),
            1 => notification_context.set_root_object(Some(second.clone())),
            _ => {},
        }
    });
    add_literal_brushes(&dictionary, &context, &DOCUMENT, ROWS).unwrap();
    drop(first);
    assert!(weak.upgrade().is_some());
    assert_eq!(drops.get(), 0);
    assert_eq!(brush_color(&resource(&dictionary, "Before")), Color::from_uint32(ACCENT));
    assert!(weak.upgrade().is_none());
    assert_eq!(drops.get(), 1);
    assert!(deferred(&dictionary, "After"));
    // The compact descriptor's index must still refer to the middle static row.
    assert_eq!(brush_color(&resource(&dictionary, "Middle")), Color::from_uint32(0xFF00_0000));
}

#[cfg(not(any(feature = "source-overlays", all(feature = "hot-reload", debug_assertions))))]
#[test]
fn compiled_rows_release_the_capture_of_a_realized_row_and_keep_the_other_indices() {
    #[derive(PartialEq)]
    struct Model;
    static ROWS: &[LiteralBrushRow] = &[
        LiteralBrushRow { key: "First", color: ACCENT, namespaces: None, line: 1, position: 2 },
        LiteralBrushRow { key: "Second", color: 0xFF00_0000, namespaces: None, line: 2, position: 2 },
        LiteralBrushRow { key: "Third", color: 0xFFFF_FFFF, namespaces: None, line: 3, position: 2 },
    ];
    crate::register_types();
    ValueTypes::register_reference::<Model>();
    let dictionary = ResourceDictionary::new();
    // A model root is captured weakly by each row: its weak count shows the captures.
    let root: BoxedValue = Rc::new(Model);
    let context = populate_context(None, &DOCUMENT, Some(root.clone()));
    context.set_intermediate_root_object(None);
    context.push_parent(to_value(dictionary.clone()));
    let before = Rc::weak_count(&root);
    add_literal_brushes(&dictionary, &context, &DOCUMENT, ROWS).unwrap();
    assert_eq!(Rc::weak_count(&root), before + 3);

    assert_eq!(brush_color(&resource(&dictionary, "Second")), Color::from_uint32(0xFF00_0000));
    assert_eq!(Rc::weak_count(&root), before + 2);
    // A second lookup returns the cached value and releases nothing more.
    assert_eq!(brush_color(&resource(&dictionary, "Second")), Color::from_uint32(0xFF00_0000));
    assert_eq!(Rc::weak_count(&root), before + 2);

    // The rows on both sides of the released one are still deferred and build their own row.
    assert!(deferred(&dictionary, "First") && deferred(&dictionary, "Third"));
    assert_eq!(brush_color(&resource(&dictionary, "Third")), Color::from_uint32(0xFFFF_FFFF));
    assert_eq!(brush_color(&resource(&dictionary, "First")), Color::from_uint32(ACCENT));
    assert_eq!(Rc::weak_count(&root), before);
}

#[test]
fn compiled_rows_are_lazy_shared_per_dictionary_and_do_not_keep_owners_alive() {
    static ROWS: &[LiteralBrushRow] = &[
        LiteralBrushRow { key: "Identity", color: ACCENT, namespaces: None, line: 2, position: 8 },
        LiteralBrushRow { key: "Unbuilt", color: ACCENT, namespaces: None, line: 3, position: 8 },
    ];
    let (first, first_context) = populated();
    let (second, second_context) = populated();
    add_literal_brushes(&first, &first_context, &DOCUMENT, ROWS).unwrap();
    add_literal_brushes(&second, &second_context, &DOCUMENT, ROWS).unwrap();
    assert!(deferred(&first, "Identity") && deferred(&second, "Identity"));
    let value = resource(&first, "Identity");
    assert!(Rc::ptr_eq(&value, &resource(&first, "Identity")));
    let other = resource(&second, "Identity");
    let brush = ValueTypes::as_object(&*value).unwrap().cast::<SolidColorBrush>().unwrap();
    let other_brush = ValueTypes::as_object(&*other).unwrap().cast::<SolidColorBrush>().unwrap();
    assert!(!brush.ptr_eq(&other_brush));
    brush.set_color(Color::from_uint32(0xFF00_0000));
    assert_eq!(other_brush.color(), Color::from_uint32(ACCENT));
    assert!(deferred(&first, "Unbuilt"));
    let weak = first.downgrade();
    drop(first_context);
    drop(first);
    assert!(weak.upgrade().is_none());
}

#[test]
fn compiled_failed_alias_retries_and_removal_replacement_clear_work_without_realization() {
    static ROWS: &[ResourceAliasRow] = &[
        ResourceAliasRow { key: "Retry", target: "Later", namespaces: None, line: 6, position: 2, value_line: 6, value_position: 20 },
    ];
    let (dictionary, context) = populated();
    add_resource_aliases(&dictionary, &context, &DOCUMENT, ROWS).unwrap();
    let failed = crate::xaml_error_scope::with_xaml_errors(|| {
        assert!(dictionary.try_get_value(&ResourceKey::from("Retry")).is_none());
        Ok(())
    });
    assert!(failed.is_err());
    assert!(deferred(&dictionary, "Retry"));
    let brush = SolidColorBrush::new();
    dictionary.add("Later", to_value(brush.clone()));
    assert!(ValueTypes::as_object(&*resource(&dictionary, "Retry")).unwrap().ptr_eq(&brush));
    add_literal_brushes(&dictionary, &context, &DOCUMENT, IDENTITY_BRUSHES).unwrap();
    assert!(dictionary.remove(&ResourceKey::from("Identity")));
    assert!(!dictionary.contains_key(&ResourceKey::from("Identity")));
    add_literal_brushes(&dictionary, &context, &DOCUMENT, IDENTITY_BRUSHES).unwrap();
    dictionary.set("Identity", to_value(brush.clone()));
    assert!(!deferred(&dictionary, "Identity"));
    dictionary.clear();
    assert_eq!(dictionary.count(), 0);
}

#[cfg(all(feature = "hot-reload", debug_assertions))]
#[test]
fn compiled_alias_hot_reload_rebases_to_preserved_dictionary_before_first_lookup() {
    use ferroui_base::controls::ResourceDictionaryOwnership;
    use crate::xaml_il::runtime::DeferredContentReloadScope;
    static ROWS: &[ResourceAliasRow] = &[
        ResourceAliasRow { key: "Alias", target: "ApplicationAccent", namespaces: None, line: 6, position: 2, value_line: 6, value_position: 20 },
    ];
    crate::register_types();
    let live = ResourceDictionary::new();
    let ownership = ResourceDictionaryOwnership::capture(&live);
    let brush = SolidColorBrush::new();
    live.add("ApplicationAccent", to_value(brush.clone()));
    let scope = DeferredContentReloadScope::new();
    let (candidate, context) = populated();
    add_resource_aliases(&candidate, &context, &DOCUMENT, ROWS).unwrap();
    let mut change = ownership.prepare(&candidate).unwrap();
    let mut rebase = scope.prepare_rebase(&[(to_value(candidate.clone()).unwrap(), to_value(live.clone()).unwrap())]);
    rebase.commit();
    change.commit().unwrap();
    drop(change);
    drop(rebase);
    drop(scope);
    let weak = candidate.downgrade();
    drop(context);
    drop(candidate);
    assert!(weak.upgrade().is_none());
    assert!(deferred(&live, "Alias"));
    let resolved = resource(&live, "Alias");
    assert!(ValueTypes::as_object(&*resolved).unwrap().ptr_eq(&brush));
    assert!(Rc::ptr_eq(&resolved, &resource(&live, "Alias")));
}

#[cfg(all(feature = "hot-reload", debug_assertions))]
#[test]
fn compiled_rows_reload_rollback_restores_realized_identity_and_detects_application_writes() {
    use ferroui_base::controls::ResourceDictionaryOwnership;
    let (live, context) = populated();
    add_literal_brushes(&live, &context, &DOCUMENT, IDENTITY_BRUSHES).unwrap();
    let ownership = ResourceDictionaryOwnership::capture(&live);
    let original = resource(&live, "Identity");
    let (candidate, candidate_context) = populated();
    add_literal_brushes(&candidate, &candidate_context, &DOCUMENT, IDENTITY_BRUSHES).unwrap();
    let mut change = ownership.prepare(&candidate).unwrap();
    change.commit().unwrap();
    assert!(!Rc::ptr_eq(&original, &resource(&live, "Identity")));
    change.rollback();
    assert!(Rc::ptr_eq(&original, &resource(&live, "Identity")));
    live.set("Identity", Some(original));
    assert!(ownership.prepare(&candidate).is_err());
}

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
