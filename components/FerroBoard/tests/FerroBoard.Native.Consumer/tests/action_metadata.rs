//! Native action references through real FerroUI declarations, outside the library workspace.
use ferroboard_controls::{
    SmartbarAction, SmartbarActionMetadata, SmartbarActionRef, SmartbarActionsMetadata,
    themes::ThemeList,
};
use ferroboard_core::{
    editor::{ITextInputTarget, TextBufferTarget},
    engine::{EngineError, KeyboardEngine},
    keys::KeyIcon,
    text::Utf16Text,
};
use ferroui_base::metadata::{
    MarkupInvokeError, MarkupTyped, MarkupValue, from_markup_value, into_markup_value,
};
use std::rc::Rc;

fn read(name: &str, action: &SmartbarActionRef) -> Result<MarkupValue, MarkupInvokeError> {
    let getter = SmartbarActionMetadata::MARKUP
        .properties
        .iter()
        .find(|p| p.name == name)
        .unwrap()
        .get
        .unwrap();
    getter(&[into_markup_value(action.clone())])
}

#[test]
fn native_action_metadata_keeps_shared_vec_identity_callbacks_and_null_entries() {
    ferroboard_controls::register_types();
    let weak = {
        let action = SmartbarAction::new(
            Some("append".into()),
            KeyIcon::None,
            Some("Append 😀".into()),
            Some(Rc::new(|engine| engine.input_text(&"!".into()))),
            None,
        );
        let weak = Rc::downgrade(&action);
        let handle = SmartbarActionRef::from_original(action.clone());
        let same = SmartbarActionRef::from_original(action.clone());
        assert!(handle == same);
        let other = SmartbarActionRef::from_original(SmartbarAction::new(
            action.id.clone(),
            action.icon,
            action.name.clone(),
            action.execute.clone(),
            None,
        ));
        assert!(
            handle != other,
            "native UI handles compare allocation identity"
        );
        assert!(Rc::ptr_eq(&handle.original(), &action));
        assert_eq!(
            from_markup_value::<Option<String>>(&read("Id", &handle).unwrap()),
            Some(Some("append".into()))
        );
        assert_eq!(
            from_markup_value::<Option<String>>(&read("Name", &handle).unwrap()),
            Some(Some("Append 😀".into()))
        );
        assert_eq!(
            from_markup_value::<KeyIcon>(&read("Icon", &handle).unwrap()),
            Some(KeyIcon::None)
        );
        assert!(
            SmartbarActionMetadata::MARKUP.constructors.is_empty(),
            "callbacks are composed through Rust"
        );
        let actions: ThemeList<Option<Rc<SmartbarAction>>> = ThemeList::default();
        let instance = into_markup_value(actions.clone());
        let add = SmartbarActionsMetadata::MARKUP
            .methods
            .iter()
            .find(|m| m.name == "Add")
            .unwrap();
        (add.invoke)(&[instance.clone(), into_markup_value(Some(handle.clone()))]).unwrap();
        (add.invoke)(&[
            instance.clone(),
            into_markup_value(Option::<SmartbarActionRef>::None),
        ])
        .unwrap();
        let count = SmartbarActionsMetadata::MARKUP
            .properties
            .iter()
            .find(|p| p.name == "Count")
            .unwrap()
            .get
            .unwrap();
        assert_eq!(
            from_markup_value::<i32>(&count(&[instance]).unwrap()),
            Some(2)
        );
        let stored = {
            let items = actions.lock().unwrap();
            assert!(items[1].is_none());
            items[0].clone().unwrap()
        };
        assert!(Rc::ptr_eq(&stored, &action));
        let engine = KeyboardEngine::with_defaults().unwrap();
        engine.settings().set_auto_capitalization(false);
        engine.settings().set_learn_words(false);
        let target = Rc::new(TextBufferTarget::default());
        engine.attach(target.clone()).unwrap();
        stored.execute.as_ref().unwrap()(&engine).unwrap();
        assert_eq!(target.text(), "!".into());
        engine.dispose().unwrap();
        weak
    };
    assert!(
        weak.upgrade().is_none(),
        "metadata must not cache or retain action instances"
    );
}

#[test]
fn native_action_metadata_rejects_invalid_text_and_keeps_rust_callback_errors() {
    ferroboard_controls::register_types();
    let action = SmartbarAction::new(
        None,
        KeyIcon::None,
        Some(Utf16Text::from_units(vec![0xd800])),
        Some(Rc::new(|_| Err(EngineError::NoEditor))),
        None,
    );
    let handle = SmartbarActionRef::from_original(action.clone());
    assert_eq!(
        from_markup_value::<Option<String>>(&read("Id", &handle).unwrap()),
        Some(None)
    );
    assert!(
        read("Name", &handle).is_err(),
        "native strings cannot silently accept an unpaired surrogate"
    );
    let engine = KeyboardEngine::with_defaults().unwrap();
    assert!(matches!(
        handle.original().execute.as_ref().unwrap()(&engine),
        Err(EngineError::NoEditor)
    ));
    engine.dispose().unwrap();
}
