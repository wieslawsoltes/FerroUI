use crate::fixture_clock;
use crate::{
    ClockRef, EditorClockMetadata, EditorControllerMetadata, SuggestionProviderFactoryMetadata,
    SuggestionProviderFactoryRef, register_types,
};
use ferroboard_core::{editor::*, engine::*, layouts::KeyboardResources};
use ferroui_base::{
    data::core::ValueType,
    metadata::{MarkupTyped, from_markup_value, into_markup_value},
};
use std::{cell::Cell, rc::Rc};

#[test]
fn metadata_carriers_have_native_identity_and_no_delegate_runtime_surface() {
    register_types();
    for metadata in [
        EditorClockMetadata::MARKUP,
        SuggestionProviderFactoryMetadata::MARKUP,
    ] {
        assert!(metadata.generic.is_none());
        assert!(metadata.methods.is_empty());
        assert!(metadata.constructors.is_empty());
        assert!(metadata.namespace().starts_with("FerroBoard."));
    }
    let property = EditorControllerMetadata::MARKUP
        .find_property("Clock")
        .unwrap();
    assert_eq!((property.type_)(), ValueType::of::<Option<ClockRef>>());
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let callback: EditorClock = Rc::new(move || {
        count.set(count.get() + 1);
        fixture_clock::time(100_000_000)
    });
    let weak = Rc::downgrade(&callback);
    let carried = ClockRef::new(callback.clone());
    let same = ClockRef::new(callback.clone());
    assert!(carried == same);
    let editor = Rc::new(EditorController::new(Rc::new(TextBufferTarget::default())));
    (property.set.unwrap())(&[
        into_markup_value(editor.clone()),
        into_markup_value(Some(carried.clone())),
    ])
    .unwrap();
    let value = (property.get.unwrap())(&[into_markup_value(editor.clone())]).unwrap();
    let restored: ClockRef = from_markup_value(&value).unwrap();
    assert!(Rc::ptr_eq(&restored.callback(), &callback));
    editor.commit_text(&"a".into()).unwrap();
    assert_eq!(calls.get(), 1);
    drop(value);
    drop(restored);
    drop(carried);
    drop(same);
    drop(callback);
    assert!(weak.upgrade().is_some());
    editor.set_clock_nullable(None);
    assert!(weak.upgrade().is_none());
    assert!(matches!(
        editor.commit_text(&"b".into()),
        Err(EditorError::NullReference)
    ));
}

#[test]
fn factory_carrier_preserves_fallible_rust_callback_and_panics() {
    register_types();
    let engine =
        KeyboardEngine::new(None, Some(Rc::new(KeyboardResources::create_empty())), None).unwrap();
    let factory: SuggestionProviderFactory = Rc::new(|_| Err(EngineError::NullReference));
    let carried = SuggestionProviderFactoryRef::new(factory.clone());
    assert!(Rc::ptr_eq(&carried.callback(), &factory));
    engine.set_suggestion_provider_factory(Some(carried.callback()));
    assert!(matches!(
        engine.suggestion_provider(),
        Err(EngineError::NullReference)
    ));
    let payload = std::sync::Arc::new(17u32);
    let marker = payload.clone();
    let factory: SuggestionProviderFactory =
        Rc::new(move |_| std::panic::panic_any(marker.clone()));
    let carried = SuggestionProviderFactoryRef::new(factory);
    let error =
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| carried.callback()(None))) {
            Err(error) => error,
            Ok(_) => panic!("factory panic must propagate"),
        };
    assert!(std::sync::Arc::ptr_eq(
        error.downcast_ref().unwrap(),
        &payload
    ));
    engine.dispose().unwrap();
}
