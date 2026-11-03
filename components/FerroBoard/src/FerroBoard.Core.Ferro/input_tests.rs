use crate::fixture_clock;
use crate::*;
use ferroboard_core::{
    editor::{EditorController, EnterAction, ITextInputTarget, InputAttributes, TextBufferTarget},
    engine::{KeyboardEngine, LanguageModels},
    layouts::KeyboardResources,
    text::Utf16Text,
};
use ferroui_base::metadata::{
    MarkupDelegate, MarkupInvokeError, MarkupTyped, from_markup_value, into_markup_value,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
fn call(
    t: &'static ferroui_base::metadata::MarkupType,
    name: &str,
    args: &[ferroui_base::metadata::MarkupValue],
) -> ferroui_base::metadata::MarkupValue {
    let m = t
        .find_methods(name)
        .find(|m| m.parameters.len() + usize::from(!m.is_static) == args.len())
        .unwrap();
    (m.invoke)(args).unwrap()
}
#[test]
fn original_interface_identity_notification_channels_and_explicit_metadata_cleanup() {
    register_types();
    let buffer = Rc::new(TextBufferTarget::new("A😀B".into(), None));
    let target: Rc<dyn ITextInputTarget> = buffer.clone();
    let object = into_markup_value(buffer.clone());
    let iface = from_markup_value::<Rc<dyn ITextInputTarget>>(&object).unwrap();
    assert!(Rc::ptr_eq(&iface, &target));
    let log = Rc::new(RefCell::new(Vec::new()));
    for (metadata, event, name) in [
        (TextBufferTargetMetadata::MARKUP, "Edited", "edited"),
        (ITextInputTargetMetadata::MARKUP, "Changed", "changed"),
    ] {
        let log = log.clone();
        let original = target.clone();
        let handler = MarkupDelegate::new(move |args| {
            let actual = from_markup_value::<Rc<dyn ITextInputTarget>>(&args[0]).unwrap();
            assert!(Rc::ptr_eq(&actual, &original));
            log.borrow_mut().push(name);
            None
        });
        (metadata.find_event(event).unwrap().add)(&[object.clone(), into_markup_value(handler)])
            .unwrap();
    }
    call(
        ITextInputTargetMetadata::MARKUP,
        "Select",
        &[
            object.clone(),
            into_markup_value(1i32),
            into_markup_value(2i32),
        ],
    );
    assert_eq!(&*log.borrow(), &["edited"]);
    call(
        ITextInputTargetMetadata::MARKUP,
        "Replace",
        &[
            object.clone(),
            into_markup_value(1i32),
            into_markup_value(2i32),
            into_markup_value("xy".to_owned()),
        ],
    );
    assert_eq!(buffer.text(), Utf16Text::from("AxyB"));
    assert_eq!(buffer.selection_start(), 3);
    let text = TextBufferTargetMetadata::MARKUP
        .find_property("Text")
        .unwrap();
    (text.set.unwrap())(&[object.clone(), None]).unwrap();
    assert_eq!(&*log.borrow(), &["edited", "edited", "changed"]);
    assert!(buffer.text().is_empty());
    release_target_markup_subscriptions(&target);
    buffer.set_text(Some("after".into()));
    assert_eq!(log.borrow().len(), 3);
}
#[test]
fn enter_callback_conversion_preserves_synchronous_calls_handled_state_and_failures() {
    register_types();
    let a = Rc::new(TextBufferTarget::default());
    let b = Rc::new(TextBufferTarget::default());
    let f: ferroboard_core::editor::EnterActionHandler =
        Rc::new(|action| action == EnterAction::Go);
    a.set_enter_action_handler(Some(f.clone()));
    b.set_enter_action_handler(Some(f.clone()));
    let prop = TextBufferTargetMetadata::MARKUP
        .find_property("EnterActionHandler")
        .unwrap();
    let read = |t: Rc<TextBufferTarget>| {
        let v = (prop.get.unwrap())(&[into_markup_value(t)]).unwrap();
        from_markup_value::<Option<MarkupDelegate>>(&v)
            .unwrap()
            .unwrap()
    };
    let x = read(a.clone());
    let y = read(b.clone());
    assert_eq!(
        from_markup_value::<bool>(&x.try_invoke(&[into_markup_value(EnterAction::Go)]).unwrap()),
        Some(true)
    );
    assert_eq!(
        from_markup_value::<bool>(
            &y.try_invoke(&[into_markup_value(EnterAction::Done)])
                .unwrap()
        ),
        Some(false)
    );
    (prop.set.unwrap())(&[
        into_markup_value(a.clone()),
        into_markup_value(Some(x.clone())),
    ])
    .unwrap();
    assert!(a.perform_enter_action(EnterAction::Go));
    assert!(!b.perform_enter_action(EnterAction::Done));
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let native = MarkupDelegate::new(move |args| {
        assert_eq!(
            from_markup_value::<EnterAction>(&args[0]),
            Some(EnterAction::Search)
        );
        count.set(count.get() + 1);
        into_markup_value(true)
    });
    (prop.set.unwrap())(&[
        into_markup_value(a.clone()),
        into_markup_value(Some(native.clone())),
    ])
    .unwrap();
    assert!(a.perform_enter_action(EnterAction::Search));
    assert_eq!(calls.get(), 1);
    (prop.set.unwrap())(&[into_markup_value(a.clone()), None]).unwrap();
    assert!(a.enter_action_handler().is_none());
    assert!(!a.perform_enter_action(EnterAction::Go));
    let bad = MarkupDelegate::new(|_| into_markup_value("wrong".to_owned()));
    (prop.set.unwrap())(&[into_markup_value(a.clone()), into_markup_value(Some(bad))]).unwrap();
    let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        a.perform_enter_action(EnterAction::Go)
    }))
    .unwrap_err();
    assert!(matches!(
        error.downcast_ref::<MarkupInvokeError>(),
        Some(MarkupInvokeError::Failed(_))
    ));
    release_target_markup_subscriptions(&(a as Rc<dyn ITextInputTarget>));
    release_target_markup_subscriptions(&(b as Rc<dyn ITextInputTarget>));
}
#[test]
fn editor_metadata_preserves_utf16_graphemes_undo_groups_duration_and_exact_text() {
    register_types();
    let original = Utf16Text::from("ae\u{301}😀!");
    let buffer = Rc::new(TextBufferTarget::new(original.clone(), None));
    let e = Rc::new(EditorController::new(buffer.clone()));
    e.set_clock(Rc::new(|| fixture_clock::time(10_000_000)));
    let object = into_markup_value(e.clone());
    let metadata = EditorControllerMetadata::MARKUP;
    let timeout = metadata.find_property("UndoGroupTimeout").unwrap();
    (timeout.set.unwrap())(&[
        object.clone(),
        into_markup_value(std::time::Duration::from_nanos(123_456_700)),
    ])
    .unwrap();
    assert_eq!(
        e.undo_group_timeout(),
        std::time::Duration::from_nanos(123_456_700)
    );
    for expected in ["ae\u{301}😀", "ae\u{301}", "a"] {
        let value = call(metadata, "DeleteBackward", std::slice::from_ref(&object));
        assert_eq!(from_markup_value::<bool>(&value), Some(true));
        assert_eq!(buffer.text(), Utf16Text::from(expected));
    }
    assert_eq!(
        from_markup_value::<bool>(&call(metadata, "Undo", std::slice::from_ref(&object))),
        Some(true)
    );
    assert_eq!(buffer.text(), original);
    assert_eq!(
        from_markup_value::<bool>(&call(metadata, "Redo", std::slice::from_ref(&object))),
        Some(true)
    );
    assert_eq!(buffer.text(), Utf16Text::from("a"));
    buffer.set_text(Some(Utf16Text::from_units(vec![0xd800])));
    let text = metadata.find_property("Text").unwrap();
    assert!((text.get.unwrap())(std::slice::from_ref(&object)).is_err());
    assert_eq!(e.text().units(), &[0xd800]);
    assert_eq!(buffer.text().units(), &[0xd800]);
    let target: Rc<dyn ITextInputTarget> = buffer;
    let target_value = into_markup_value(target.clone());
    let text = (ITextInputTargetMetadata::MARKUP
        .find_property("Text")
        .unwrap()
        .get
        .unwrap())(&[target_value]);
    assert!(text.is_err());
    assert_eq!(target.text().units(), &[0xd800]);
    release_target_markup_subscriptions(&target);
}
#[test]
fn native_markup_constructor_omissions_create_independent_owned_dependencies() {
    register_types();
    let ctor = &KeyboardEngineMetadata::MARKUP.constructors[0];
    let construct = || {
        let value = (ctor.invoke)(&[None, None, None]).unwrap();
        from_markup_value::<Rc<KeyboardEngine>>(&value).unwrap()
    };
    let first = construct();
    let second = construct();
    let first_resources = first.resources();
    let second_resources = second.resources();
    assert!(!first_resources.extensions().is_empty());
    assert!(!second_resources.extensions().is_empty());
    assert!(!Rc::ptr_eq(&first_resources, &second_resources));
    assert!(!Rc::ptr_eq(&first.settings(), &second.settings()));
    assert!(!Arc::ptr_eq(
        &first.language_models(),
        &second.language_models()
    ));
    let second_height = second.settings().key_height();
    first.settings().set_key_height(second_height * 2.0);
    assert_eq!(second.settings().key_height(), second_height);
    assert_eq!(first_resources.changed().handler_count(), 1);
    assert_eq!(second_resources.changed().handler_count(), 1);
    first.dispose().unwrap();
    second.dispose().unwrap();
    assert_eq!(first_resources.changed().handler_count(), 0);
    assert_eq!(second_resources.changed().handler_count(), 0);
}

#[test]
fn explicit_source_engine_construction_preserves_defaults_and_dispose_event_semantics() {
    register_types();
    let ctor = &KeyboardEngineMetadata::MARKUP.constructors[0];
    let resources = Rc::new(KeyboardResources::create_default().unwrap());
    let models = LanguageModels::new();
    let result = (ctor.invoke)(&[
        None,
        into_markup_value(Some(resources.clone())),
        into_markup_value(Some(models.clone())),
    ])
    .unwrap();
    let engine = from_markup_value::<Rc<KeyboardEngine>>(&result).unwrap();
    assert!(Rc::ptr_eq(&engine.resources(), &resources));
    assert!(Arc::ptr_eq(&engine.language_models(), &models));
    assert_eq!(
        engine.settings().key_height(),
        ferroboard_core::settings::KeyboardSettings::new().key_height()
    );
    let attrs = engine.attributes();
    assert!(Arc::ptr_eq(&attrs, &InputAttributes::shared_default()));
    let count = Rc::new(Cell::new(0));
    let counter = count.clone();
    let handler = MarkupDelegate::new(move |_| {
        counter.set(counter.get() + 1);
        None
    });
    (KeyboardEngineMetadata::MARKUP
        .find_event("KeyboardChanged")
        .unwrap()
        .add)(&[result.clone(), into_markup_value(handler)])
    .unwrap();
    call(
        KeyboardEngineMetadata::MARKUP,
        "Dispose",
        std::slice::from_ref(&result),
    );
    let before = count.get();
    call(
        KeyboardEngineMetadata::MARKUP,
        "SetMode",
        &[
            result.clone(),
            into_markup_value(ferroboard_core::keys::KeyboardMode::Numeric),
        ],
    );
    assert!(
        count.get() > before,
        "source Dispose does not clear public engine handlers"
    );
    release_engine_markup_subscriptions(&engine);
    let before = count.get();
    engine
        .set_mode(ferroboard_core::keys::KeyboardMode::Characters)
        .unwrap();
    assert_eq!(count.get(), before);
}

#[test]
fn input_model_and_runtime_metadata_have_identical_declared_member_shapes() {
    use ferroui_build_scan::model::{AssemblyModel, TypeKind};
    register_types();
    let model = AssemblyModel::parse(include_str!(concat!(
        env!("OUT_DIR"),
        "/ferroboard_core_ferro.xamlmeta"
    )))
    .unwrap();
    for runtime in [
        ITextInputTargetMetadata::MARKUP,
        TextBufferTargetMetadata::MARKUP,
        InputAttributesMetadata::MARKUP,
        EditorControllerMetadata::MARKUP,
        PunctuationRuleMetadata::MARKUP,
        KeyboardEngineMetadata::MARKUP,
        KeyboardResourcesMetadata::MARKUP,
        LanguageModelsMetadata::MARKUP,
        PerformedActionList::MARKUP,
        SettingsSubtypeList::MARKUP,
    ] {
        let model = model
            .types
            .iter()
            .find(|t| t.name == runtime.name && t.namespace == runtime.namespace())
            .unwrap();
        assert_eq!(
            model.kind == TypeKind::Interface,
            runtime.name == "ITextInputTarget"
        );
        let shape = |p: &ferroui_base::metadata::MarkupProperty| {
            (p.name.to_owned(), p.get.is_some(), p.set.is_some())
        };
        let mut actual = runtime.properties.iter().map(shape).collect::<Vec<_>>();
        let mut expected = model
            .properties
            .iter()
            .map(|p| (p.name.clone(), p.getter.is_some(), p.setter.is_some()))
            .collect::<Vec<_>>();
        actual.sort();
        expected.sort();
        assert_eq!(actual, expected, "{} properties", runtime.name);
        let mut actual = runtime
            .static_properties
            .iter()
            .map(shape)
            .collect::<Vec<_>>();
        let mut expected = model
            .static_properties
            .iter()
            .map(|p| (p.name.clone(), p.getter.is_some(), p.setter.is_some()))
            .collect::<Vec<_>>();
        actual.sort();
        expected.sort();
        assert_eq!(actual, expected, "{} static properties", runtime.name);
        let mut actual = runtime
            .methods
            .iter()
            .map(|m| (m.name.to_owned(), m.parameters.len(), m.is_static))
            .collect::<Vec<_>>();
        let mut expected = model
            .methods
            .iter()
            .map(|m| (m.name.clone(), m.parameters.len(), m.is_static))
            .collect::<Vec<_>>();
        actual.sort();
        expected.sort();
        assert_eq!(actual, expected, "{} methods", runtime.name);
        let mut actual = runtime
            .constructors
            .iter()
            .map(|c| c.parameters.len())
            .collect::<Vec<_>>();
        let mut expected = model
            .constructors
            .iter()
            .map(|c| c.parameters.len())
            .collect::<Vec<_>>();
        actual.sort();
        expected.sort();
        assert_eq!(actual, expected, "{} constructors", runtime.name);
        let mut actual = runtime
            .events
            .iter()
            .map(|e| e.name.to_owned())
            .collect::<Vec<_>>();
        let mut expected = model
            .events
            .iter()
            .map(|e| e.name.clone())
            .collect::<Vec<_>>();
        actual.sort();
        expected.sort();
        assert_eq!(actual, expected, "{} events", runtime.name);
        assert_eq!(
            runtime.indexers.len(),
            model.indexers.len(),
            "{} indexers",
            runtime.name
        );
    }
}

#[test]
fn normalized_original_class_handles_resolve_metadata_for_nested_binding_notifications() {
    register_types();
    use ferroui_base::metadata::MarkupType;
    let settings = Rc::new(ferroboard_core::settings::KeyboardSettings::new());
    let value = into_markup_value(settings.clone()).unwrap();
    let metadata = MarkupType::find_by_handle(value.value_type_id()).unwrap();
    assert!(std::ptr::eq(metadata, KeyboardSettingsMetadata::MARKUP));
    let notify = (metadata.notify_property_changed.unwrap())(&*value).unwrap();
    let count = Rc::new(Cell::new(0));
    let observed = count.clone();
    let token = notify.property_changed().add(Rc::new(move |name| {
        assert_eq!(name, "NumberRow");
        observed.set(observed.get() + 1);
    }));
    settings.set_number_row(true);
    assert_eq!(count.get(), 1);
    notify.property_changed().remove(token);
    for (type_id, expected) in [
        (
            std::any::TypeId::of::<TextBufferTarget>(),
            TextBufferTargetMetadata::MARKUP,
        ),
        (
            std::any::TypeId::of::<EditorController>(),
            EditorControllerMetadata::MARKUP,
        ),
        (
            std::any::TypeId::of::<KeyboardEngine>(),
            KeyboardEngineMetadata::MARKUP,
        ),
        (
            std::any::TypeId::of::<KeyboardResources>(),
            KeyboardResourcesMetadata::MARKUP,
        ),
    ] {
        assert!(std::ptr::eq(
            MarkupType::find_by_handle(type_id).unwrap(),
            expected
        ));
    }
}

#[test]
fn duration_metadata_uses_std_values_and_rejects_invalid_arguments_before_mutation() {
    use ferroui_base::data::core::ValueType;
    use std::time::Duration;
    register_types();
    let editor = Rc::new(EditorController::new(Rc::new(TextBufferTarget::default())));
    let property = EditorControllerMetadata::MARKUP
        .find_property("UndoGroupTimeout")
        .unwrap();
    assert_eq!((property.type_)(), ValueType::of::<Duration>());
    let object = into_markup_value(editor.clone());
    for duration in [Duration::ZERO, Duration::from_nanos(1), Duration::MAX] {
        (property.set.unwrap())(&[object.clone(), into_markup_value(duration)]).unwrap();
        let value = (property.get.unwrap())(std::slice::from_ref(&object)).unwrap();
        assert_eq!(from_markup_value::<Duration>(&value), Some(duration));
        assert_eq!(editor.undo_group_timeout(), duration);
    }
    assert!((property.set.unwrap())(&[object, into_markup_value(-1_i64)]).is_err());
    assert_eq!(editor.undo_group_timeout(), Duration::MAX);
}
