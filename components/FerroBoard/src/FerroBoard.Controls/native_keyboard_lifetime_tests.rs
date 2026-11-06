use super::*;
#[test]
fn explicit_release_search_restores_external_target_but_retires_owned_adapter() {
    crate::test_support::run(|| {
        for owned in [false, true] {
            let keyboard = OnScreenKeyboard::new(None).unwrap();
            let engine = keyboard.engine();
            let external = Rc::new(TextBufferTarget::default());
            let field = TextBox::new();
            if owned {
                keyboard.attach_to(Some(field.upcast())).unwrap();
            } else {
                keyboard.attach(external.clone()).unwrap();
            }
            let original = engine.target().unwrap();
            keyboard.begin_emoji_search().unwrap();
            let search = engine.target().unwrap();
            assert!(!Rc::ptr_eq(&original, &search));
            keyboard.release_native_subscriptions().unwrap();
            assert!(keyboard.search_buffer.borrow().is_none());
            assert!(keyboard.search_state.borrow().is_none());
            assert_eq!(search.changed().handler_count(), 0);
            if owned {
                assert!(engine.target().is_none());
            } else {
                assert!(engine.target().is_some_and(|t| Rc::ptr_eq(&t, &original)));
                engine.input_text(&"restored".into()).unwrap();
                assert_eq!(external.text(), "restored".into());
            }
            engine.dispose().unwrap();
        }
    });
}

#[test]
fn explicit_release_inactive_search_does_not_restore_a_retired_target() {
    crate::test_support::run(|| {
        let keyboard = OnScreenKeyboard::new(None).unwrap();
        let engine = keyboard.engine();
        keyboard
            .attach(Rc::new(TextBufferTarget::default()))
            .unwrap();
        keyboard.begin_emoji_search().unwrap();
        let other = Rc::new(TextBufferTarget::default());
        let expected: Rc<dyn ITextInputTarget> = other.clone();
        engine.attach(other.clone()).unwrap();
        keyboard.release_native_subscriptions().unwrap();
        assert!(engine.target().is_some_and(|t| Rc::ptr_eq(&t, &expected)));
        engine.input_text(&"other owner".into()).unwrap();
        assert_eq!(other.text(), "other owner".into());
        engine.dispose().unwrap();
    });
}

#[test]
fn dropping_search_restores_external_target_and_expires_retained_search_callbacks() {
    crate::test_support::run(|| {
        let keyboard = OnScreenKeyboard::new(None).unwrap();
        let engine = keyboard.engine();
        let target = Rc::new(TextBufferTarget::default());
        keyboard.attach(target.clone()).unwrap();
        keyboard.begin_emoji_search().unwrap();
        let buffer = keyboard.search_buffer.borrow().clone().unwrap();
        let search = keyboard.search_state.borrow().clone().unwrap();
        let weak = keyboard.downgrade();
        drop(keyboard);
        assert!(weak.upgrade().is_none());
        assert!(engine.target().is_some_and(|actual| Rc::ptr_eq(
            &actual,
            &(target.clone() as Rc<dyn ITextInputTarget>)
        )));
        assert!(!buffer.perform_enter_action(EnterAction::Search));
        assert_eq!(
            search.select.as_ref().unwrap()(Some("😀".into()))
                .unwrap_err()
                .to_string(),
            "OwnerReleased"
        );
        engine.input_text(&"restored".into()).unwrap();
        assert_eq!(target.text(), "restored".into());
        engine.dispose().unwrap();
    });
}
#[test]
fn queued_theme_registration_does_not_retain_keyboard_and_drop_removes_global_hook() {
    crate::test_support::run(|| {
        let keyboard = OnScreenKeyboard::new(None).unwrap();
        let engine = keyboard.engine();
        let (id, token) = keyboard
            .theme_subscription
            .borrow()
            .as_ref()
            .map(|(id, subscription)| (*id, subscription.test_token()))
            .unwrap();
        let theme = KeyboardTheme::new();
        theme.set_id(Some("native-lifetime-theme".into()));
        std::thread::spawn(move || KeyboardThemes::register(theme).unwrap())
            .join()
            .unwrap();
        let weak = keyboard.downgrade();
        drop(keyboard);
        assert!(weak.upgrade().is_none());
        assert!(!THEME_OWNERS.with_borrow(|owners| owners.contains_key(&id)));
        assert!(
            !KeyboardThemes::remove_changed(token),
            "Drop must already remove the global handler"
        );
        Dispatcher::current_dispatcher().run_jobs(None);
        engine.dispose().unwrap();
    });
}
