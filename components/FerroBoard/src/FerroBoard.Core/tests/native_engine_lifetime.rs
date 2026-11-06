use ferroboard_core::{
    editor::{ITextInputTarget, TextBufferTarget},
    engine::{KeyboardEngine, LanguageModels},
    layouts::KeyboardResources,
    settings::KeyboardSettings,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::{cell::Cell, rc::Rc};
#[test]
fn dropping_engine_removes_borrowed_event_hooks_without_editing_target() {
    let settings = Rc::new(KeyboardSettings::new());
    settings.set_show_suggestions(false);
    settings.set_auto_capitalization(false);
    let resources = Rc::new(KeyboardResources::create_default().unwrap());
    let settings_before = settings.property_changed().handler_count();
    let resources_before = resources.changed().handler_count();
    let engine =
        KeyboardEngine::new_explicit(settings.clone(), resources.clone(), LanguageModels::new())
            .unwrap();
    let history = engine.clipboard_history();
    let target = Rc::new(TextBufferTarget::new("text 😀".into(), None));
    target.select(1, 3);
    engine.attach(target.clone()).unwrap();
    let notifications = Rc::new(Cell::new(0));
    let count = notifications.clone();
    engine
        .state_changed()
        .add(Rc::new(move |_| count.set(count.get() + 1)));
    let weak = Rc::downgrade(&engine);
    drop(engine);
    assert!(
        weak.upgrade().is_none(),
        "configuration/target events must not retain an engine"
    );
    assert_eq!(settings.property_changed().handler_count(), settings_before);
    assert_eq!(resources.changed().handler_count(), resources_before);
    assert_eq!(target.changed().handler_count(), 0);
    assert_eq!(history.changed().handler_count(), 0);
    assert_eq!(
        notifications.get(),
        0,
        "Drop must not emit engine state transitions"
    );
    assert_eq!(target.text(), "text 😀".into());
    assert_eq!(
        (target.selection_start(), target.selection_length()),
        (1, 3)
    );
    settings.set_auto_capitalization(true);
    resources.changed().raise(resources.as_ref());
    target.set_text(Some("still usable".into()));
    history.add(Some(&"clipboard".into())).unwrap();
}

use ferroboard_core::engine::EngineError;
use std::cell::RefCell;
fn dependencies() -> (Rc<KeyboardSettings>, Rc<KeyboardResources>) {
    let settings = Rc::new(KeyboardSettings::new());
    settings.set_glide_typing(false);
    settings.set_show_suggestions(false);
    settings.set_auto_capitalization(false);
    (
        settings,
        Rc::new(KeyboardResources::create_default().unwrap()),
    )
}
#[test]
fn failed_construction_removes_hooks_from_retained_dependencies() {
    let (settings, resources) = dependencies();
    settings.subtypes().borrow_mut().push(None);
    let before_settings = settings.property_changed().handler_count();
    let before_resources = resources.changed().handler_count();
    let result =
        KeyboardEngine::new_explicit(settings.clone(), resources.clone(), LanguageModels::new());
    assert!(matches!(result, Err(EngineError::UnrepresentedText)));
    assert_eq!(settings.property_changed().handler_count(), before_settings);
    assert_eq!(resources.changed().handler_count(), before_resources);
}
#[test]
fn earlier_settings_listener_can_drop_last_engine_owner_during_notification() {
    let (settings, resources) = dependencies();
    let slot = Rc::new(RefCell::new(None::<Rc<KeyboardEngine>>));
    let owner = Rc::downgrade(&slot);
    let token = settings.property_changed().add(Rc::new(move |_| {
        owner.upgrade().unwrap().borrow_mut().take();
    }));
    let engine =
        KeyboardEngine::new_explicit(settings.clone(), resources.clone(), LanguageModels::new())
            .unwrap();
    let weak = Rc::downgrade(&engine);
    let history = engine.clipboard_history();
    *slot.borrow_mut() = Some(engine);
    settings.set_number_row(true);
    assert!(weak.upgrade().is_none());
    assert!(slot.borrow().is_none());
    assert_eq!(settings.property_changed().handler_count(), 1);
    assert_eq!(resources.changed().handler_count(), 0);
    assert_eq!(history.changed().handler_count(), 0);
    settings.property_changed().remove(token);
}
#[test]
fn target_replacement_and_drop_remove_only_the_current_registration() {
    let (settings, resources) = dependencies();
    let engine = KeyboardEngine::new_explicit(settings, resources, LanguageModels::new()).unwrap();
    let first = Rc::new(TextBufferTarget::default());
    let second = Rc::new(TextBufferTarget::default());
    engine.attach(first.clone()).unwrap();
    assert_eq!(first.changed().handler_count(), 1);
    engine.attach(second.clone()).unwrap();
    assert_eq!(first.changed().handler_count(), 0);
    assert_eq!(second.changed().handler_count(), 1);
    let weak = Rc::downgrade(&engine);
    drop(engine);
    assert!(weak.upgrade().is_none());
    assert_eq!(second.changed().handler_count(), 0);
    first.set_text(Some("first".into()));
    second.set_text(Some("second".into()));
}
#[test]
fn explicit_dispose_preserves_live_history_subscription_until_drop() {
    let (settings, resources) = dependencies();
    let engine =
        KeyboardEngine::new_explicit(settings.clone(), resources.clone(), LanguageModels::new())
            .unwrap();
    let history = engine.clipboard_history();
    engine.dispose().unwrap();
    engine.dispose().unwrap();
    assert_eq!(settings.property_changed().handler_count(), 0);
    assert_eq!(resources.changed().handler_count(), 0);
    // The original Dispose intentionally leaves ClipboardHistory.Changed subscribed while alive.
    assert_eq!(history.changed().handler_count(), 1);
    history.add(Some(&"alive after dispose".into())).unwrap();
    let weak = Rc::downgrade(&engine);
    drop(engine);
    assert!(weak.upgrade().is_none());
    assert_eq!(history.changed().handler_count(), 0);
}

#[test]
fn dropping_one_engine_keeps_other_shared_configuration_subscribers_active() {
    let (settings, resources) = dependencies();
    let first =
        KeyboardEngine::new_explicit(settings.clone(), resources.clone(), LanguageModels::new())
            .unwrap();
    let second =
        KeyboardEngine::new_explicit(settings.clone(), resources.clone(), LanguageModels::new())
            .unwrap();
    assert_eq!(settings.property_changed().handler_count(), 2);
    assert_eq!(resources.changed().handler_count(), 2);
    let weak = Rc::downgrade(&first);
    drop(first);
    assert!(weak.upgrade().is_none());
    assert_eq!(settings.property_changed().handler_count(), 1);
    assert_eq!(resources.changed().handler_count(), 1);
    let before = second.keyboard().unwrap();
    settings.set_number_row(true);
    let after = second.keyboard().unwrap();
    assert!(!Rc::ptr_eq(&before, &after));
    resources.changed().raise(resources.as_ref());
    assert!(!Rc::ptr_eq(&after, &second.keyboard().unwrap()));
    drop(second);
    assert_eq!(settings.property_changed().handler_count(), 0);
    assert_eq!(resources.changed().handler_count(), 0);
}
