use super::*;
use ferroui_base::input::KeyEventArgs;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_designer_tools::PropertyValueShape;
use std::cell::RefCell;

fn context(property: &str, type_: &str, value: Option<&str>) -> PropertyEditorContext {
    PropertyEditorContext {
        property: property.into(),
        targets: vec![1],
        value: value.map(str::to_owned),
        mixed: false,
        runtime_type: Some(type_.into()),
        apply: Rc::new(|_| {}),
    }
}
fn key(field: &TextBox, key: Key) {
    let mut event = KeyEventArgs::new();
    event.key = key;
    event.set_routed_event(Some(InputElement::key_down_event()));
    field.raise_event(&event);
}

#[test]
fn runtime_enum_metadata_and_unknown_custom_types_use_distinct_resolvers() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    ferroui_base::register_types();
    let property = ferroui_controls::TextBlock::text_wrapping_property().as_property();
    let context = context(
        "AnyCustomPropertyName",
        property.property_type_name(),
        Some("Wrap"),
    );
    let metadata = resolve_property_editor_metadata(&context, Some(property));
    let PropertyEditorKind::Enumeration { values, flags } = &metadata.kind else {
        panic!("Expected reflected enum metadata, got {:?}", metadata.kind)
    };
    assert!(!flags);
    assert!(values.contains(&"WrapWithOverflow".into()));
    assert!(metadata.validate("Wrap").is_ok());
    assert!(metadata.validate("InventedEnumMember").is_err());
    let custom = PropertyEditorContext {
        runtime_type: Some("application::geometry::Point".into()),
        ..context
    };
    assert_eq!(
        resolve_property_editor_metadata(&custom, None).kind,
        PropertyEditorKind::Text,
        "A similarly named custom type does not inherit framework serialization"
    );
}

#[test]
fn type_identity_only_unwraps_storage_wrappers_and_preserves_application_serializers() {
    for type_ in [
        "application::Envelope<ferroui_base::point::Point>",
        "application::ferroui_base::Point",
        "Option<application::ferroui_base::Point>",
    ] {
        let context = context("RenderTransformOrigin", type_, Some("custom"));
        assert_eq!(
            resolve_property_editor_metadata(&context, None).kind,
            PropertyEditorKind::Text,
            "Application type {type_} must retain its serializer"
        );
    }
    let context = context(
        "Position",
        "core::option::Option<ferroui_base::point::Point>",
        None,
    );
    let metadata = resolve_property_editor_metadata(&context, None);
    assert!(metadata.nullable);
    assert_eq!(
        metadata.kind,
        PropertyEditorKind::Components(PropertyValueShape::Point)
    );
}

#[test]
fn native_editor_dispatch_uses_metadata_for_custom_property_names_and_enum_members() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let brush = context(
        "AccentInk",
        "Option<alloc::rc::Rc<dyn ferroui_base::media::i_brush::IBrush>>",
        Some("Red"),
    );
    let metadata = PropertyEditorMetadata::new(PropertyEditorKind::Brush);
    assert!(
        typed_builtin(&brush, &metadata)
            .classes()
            .contains("property-color-editor")
    );
    let thickness = context(
        "Insets",
        "Option<ferroui_base::thickness::Thickness>",
        Some("3"),
    );
    let metadata = PropertyEditorMetadata::new(PropertyEditorKind::Thickness);
    assert!(
        typed_builtin(&thickness, &metadata)
            .classes()
            .contains("property-components")
    );

    let custom = context("Orientation", "application::Orientation", Some("Clockwise"));
    let metadata = PropertyEditorMetadata::new(PropertyEditorKind::Enumeration {
        values: vec!["Clockwise".into(), "CounterClockwise".into()],
        flags: false,
    });
    let editor = typed_builtin(&custom, &metadata)
        .cast::<ferroui_controls::ComboBox>()
        .unwrap();
    assert_eq!(editor.selected_index(), 1);
    assert_eq!(editor.items().count(), 3);

    let unknown = context("HorizontalAlignment", "", Some("FutureAlignment"));
    let metadata = resolve_property_editor_metadata(&unknown, None);
    let editor = typed_builtin(&unknown, &metadata)
        .cast::<ferroui_controls::ComboBox>()
        .unwrap();
    assert_eq!(
        editor.selected_index(),
        5,
        "Unknown authored source stays visible"
    );
}

#[test]
fn enum_inherited_selection_resets_while_mixed_placeholder_does_not() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let context = context("Choice", "CustomEnum", Some("First"));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let captured = seen.clone();
    let mut metadata = PropertyEditorMetadata::new(PropertyEditorKind::Enumeration {
        values: vec!["First".into(), "Second".into()],
        flags: false,
    });
    metadata.commit = Some(Rc::new(move |edit| {
        captured.borrow_mut().push(edit);
        Ok(())
    }));
    let editor = typed_builtin(&context, &metadata)
        .cast::<ferroui_controls::ComboBox>()
        .unwrap();
    editor.set_selected_index(0);
    assert_eq!(*seen.borrow(), vec![PropertyValueEdit::Reset]);
    let mixed = PropertyEditorContext {
        value: None,
        mixed: true,
        ..context
    };
    let editor = typed_builtin(&mixed, &metadata)
        .cast::<ferroui_controls::ComboBox>()
        .unwrap();
    editor.set_selected_index(-1);
    editor.set_selected_index(0);
    assert_eq!(seen.borrow().len(), 1);
}

#[test]
fn grid_length_escape_discards_draft_and_never_emits_a_source_transaction() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let context = context("Length", "GridLength", Some("2*"));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let captured = seen.clone();
    let mut metadata = PropertyEditorMetadata::new(PropertyEditorKind::GridLength);
    metadata.commit = Some(Rc::new(move |edit| {
        captured.borrow_mut().push(edit);
        Ok(())
    }));
    let editor = typed_builtin(&context, &metadata);
    let value = editor
        .get_logical_descendants()
        .find_map(|control| control.cast::<TextBox>())
        .unwrap();
    value.set_text(Some("15"));
    key(&value, Key::Escape);
    assert_eq!(value.text().as_deref(), Some("2"));
    assert!(seen.borrow().is_empty());
}

#[test]
fn component_fields_stage_mixed_values_validate_units_and_cancel_without_commit() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut context = context("Origin", "RelativePoint", Some("25%, 50%"));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let captured = seen.clone();
    let mut metadata = resolve_property_editor_metadata(&context, None);
    metadata.commit = Some(Rc::new(move |edit| {
        captured.borrow_mut().push(edit);
        Ok(())
    }));
    let editor = PropertyVectorEditor::new(&context, &metadata, PropertyValueShape::RelativePoint);
    editor.fields()[0].set_text(Some("75"));
    assert!(seen.borrow().is_empty());
    key(&editor.fields()[0], Key::Escape);
    assert_eq!(editor.fields()[0].text().as_deref(), Some("25"));
    editor.fields()[0].set_text(Some("75"));
    key(&editor.fields()[0], Key::Enter);
    assert_eq!(
        *seen.borrow(),
        vec![PropertyValueEdit::Literal("75%, 50%".into())]
    );
    context.value = None;
    context.mixed = true;
    let editor = PropertyVectorEditor::new(&context, &metadata, PropertyValueShape::RelativePoint);
    editor.fields()[0].set_text(Some("5"));
    key(&editor.fields()[0], Key::Enter);
    assert_eq!(
        seen.borrow().len(),
        1,
        "A mixed compound value requires complete components"
    );
    editor.fields()[1].set_text(Some("6"));
    key(&editor.fields()[1], Key::Enter);
    assert_eq!(seen.borrow().len(), 2);
}

#[test]
fn numeric_native_range_validation_retains_large_integer_precision() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let context = context("Sequence", "u64", Some("18446744073709551615"));
    let metadata = resolve_property_editor_metadata(&context, None);
    assert!(
        built_in(&context).cast::<TextBox>().is_some(),
        "Values beyond exact f64 precision must not be scrubbed"
    );
    assert!(
        metadata
            .apply(
                &context,
                PropertyValueEdit::Literal("18446744073709551615".into())
            )
            .is_ok()
    );
    assert!(
        metadata
            .apply(
                &context,
                PropertyValueEdit::Literal("18446744073709551616".into())
            )
            .is_err()
    );
    assert!(
        metadata
            .apply(&context, PropertyValueEdit::Literal("-1".into()))
            .is_err()
    );
    assert!(
        metadata
            .apply(&context, PropertyValueEdit::Literal("1.5".into()))
            .is_err()
    );
}

#[test]
fn metadata_factory_precedence_preserves_legacy_factories_and_source_state() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    struct Modern(Rc<RefCell<Vec<PropertyValueSource>>>);
    impl PropertyEditorFactory for Modern {
        fn create_with_metadata(
            &self,
            _: &PropertyEditorContext,
            metadata: &PropertyEditorMetadata,
        ) -> Option<Ref<Control>> {
            self.0.borrow_mut().push(metadata.source);
            Some(Button::new().upcast())
        }
    }
    struct Legacy(Rc<RefCell<usize>>);
    impl PropertyEditorFactory for Legacy {
        fn create(&self, _: &PropertyEditorContext) -> Option<Ref<Control>> {
            *self.0.borrow_mut() += 1;
            Some(TextBox::new().upcast())
        }
    }
    let modern = Rc::new(RefCell::new(Vec::new()));
    let legacy = Rc::new(RefCell::new(0));
    let context = context("Value", "String", Some("{Binding Name}"));
    let metadata = resolve_property_editor_metadata(&context, None);
    let _ = create_property_editor(
        &context,
        &metadata,
        &[
            Rc::new(Modern(modern.clone())),
            Rc::new(Legacy(legacy.clone())),
        ],
    );
    assert_eq!(*modern.borrow(), vec![PropertyValueSource::Expression]);
    assert_eq!(*legacy.borrow(), 0);
    let _ = create_property_editor(&context, &metadata, &[Rc::new(Legacy(legacy.clone()))]);
    assert_eq!(*legacy.borrow(), 1);
}

#[test]
fn structured_flags_validate_members_and_shapes_preserve_authored_expressions() {
    let metadata = PropertyEditorMetadata::new(PropertyEditorKind::Enumeration {
        values: vec!["None".into(), "Read".into(), "Write".into()],
        flags: true,
    });
    assert!(metadata.validate("Read, Write").is_ok());
    assert!(metadata.validate("Read, Missing").is_err());
    let context = context("Position", "Point", Some("{Binding Position}"));
    assert_eq!(
        resolve_property_editor_metadata(&context, None).source,
        PropertyValueSource::Expression
    );
    let literal = PropertyEditorContext {
        value: Some("{}{literal}".into()),
        ..context
    };
    assert_eq!(
        resolve_property_editor_metadata(&literal, None).source,
        PropertyValueSource::Local
    );
}

#[test]
fn resource_filter_uses_registered_types_and_leaves_custom_conversions_to_the_host() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    ferroui_base::register_types();
    ferroui_controls::register_types();
    let property = ferroui_controls::Border::background_property().as_property();
    let document = ferroui_designer_core::Document::new(r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" xmlns:c="urn:application"><SolidColorBrush x:Key="Brush" Color="Blue"/><ControlTheme x:Key="Theme" TargetType="Button"/><c:Brush x:Key="Custom"/></ResourceDictionary>"#).unwrap();
    let resource = |name: &str| {
        document
            .ast()
            .elements()
            .find(|node| node.local_name() == name)
            .unwrap()
    };
    assert_eq!(
        resource_is_compatible(property, resource("SolidColorBrush")),
        Some(true)
    );
    assert_eq!(
        resource_is_compatible(property, resource("ControlTheme")),
        Some(false)
    );
    assert_eq!(resource_is_compatible(property, resource("Brush")), None);
}

#[test]
fn native_property_validation_checks_layout_constraints_before_source_commit() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    ferroui_base::register_types();
    let property = ferroui_base::layout::Layoutable::width_property().as_property();
    let context = context("Width", property.property_type_name(), None);
    let metadata = resolve_property_editor_metadata(&context, Some(property));
    assert!(metadata.validate("180").is_ok());
    assert!(metadata.validate("-1").is_err());
    assert!(metadata.validate("Auto").is_ok());
}

#[test]
fn literal_validation_decodes_xaml_escape_without_changing_authored_spelling() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let context = context("Accent", "Color", None);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let captured = seen.clone();
    let mut metadata = resolve_property_editor_metadata(&context, None);
    metadata.commit = Some(Rc::new(move |edit| {
        captured.borrow_mut().push(edit);
        Ok(())
    }));
    metadata
        .apply(&context, PropertyValueEdit::Literal("{}  red  ".into()))
        .unwrap();
    assert_eq!(
        *seen.borrow(),
        vec![PropertyValueEdit::Literal("{}  red  ".into())]
    );
    let thickness = PropertyEditorMetadata::new(PropertyEditorKind::Thickness);
    assert!(thickness.validate("{}1, 2").is_ok());
    assert!(thickness.validate("1, 2, 3").is_err());
    assert!(thickness.validate("invalid").is_err());
    let transform = PropertyEditorMetadata::new(PropertyEditorKind::Transform);
    assert!(transform.validate("rotate(30deg)").is_ok());
    assert!(transform.validate("not a transform").is_err());
}
