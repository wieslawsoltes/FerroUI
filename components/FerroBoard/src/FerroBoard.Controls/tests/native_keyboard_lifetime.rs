#[path = "support/headless.rs"]
mod headless;
use ferroboard_controls::OnScreenKeyboard;
use ferroboard_core::editor::{ITextInputTarget, TextBufferTarget};
use ferroui_base::threading::Dispatcher;
use std::rc::Rc;
#[test]
fn dropping_keyboard_removes_engine_hooks_and_keeps_external_target_usable() {
    headless::run(|| {
        let keyboard = OnScreenKeyboard::new(None).unwrap();
        let engine = keyboard.engine();
        let target = Rc::new(TextBufferTarget::default());
        keyboard.attach(target.clone()).unwrap();
        let weak = keyboard.downgrade();
        drop(keyboard);
        assert!(
            weak.upgrade().is_none(),
            "keyboard callbacks/theme registry must not retain its owner"
        );
        assert_eq!(engine.state_changed().handler_count(), 0);
        assert_eq!(engine.keyboard_changed().handler_count(), 0);
        assert_eq!(engine.suggestions_changed().handler_count(), 0);
        assert!(engine.target().is_some_and(|actual| Rc::ptr_eq(
            &actual,
            &(target.clone() as Rc<dyn ITextInputTarget>)
        )));
        engine.input_text(&"retained engine".into()).unwrap();
        assert_eq!(target.text(), "retained engine".into());
        Dispatcher::current_dispatcher().run_jobs(None);
        engine.dispose().unwrap();
    });
}

use ferroboard_core::editor::EnterAction;
use ferroui_controls::TextBox;
use std::cell::Cell;
#[test]
fn dropping_keyboard_detaches_its_native_adapter_and_expires_enter_callback() {
    headless::run(|| {
        let keyboard = OnScreenKeyboard::new(None).unwrap();
        let engine = keyboard.engine();
        let field = TextBox::new();
        assert!(keyboard.attach_to(Some(field.clone().upcast())).unwrap());
        let adapter = engine.target().unwrap();
        let weak = keyboard.downgrade();
        drop(keyboard);
        assert!(weak.upgrade().is_none());
        assert!(engine.target().is_none());
        assert!(!adapter.perform_enter_action(EnterAction::Search));
        let changed = Rc::new(Cell::new(0));
        let count = changed.clone();
        adapter
            .changed()
            .add(Rc::new(move |_| count.set(count.get() + 1)));
        field.set_text(Some("external"));
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(
            changed.get(),
            0,
            "disposed adapter must stop native text/selection notifications"
        );
        engine.dispose().unwrap();
    });
}
#[test]
fn dropping_keyboard_does_not_detach_a_shared_engine_retargeted_elsewhere() {
    headless::run(|| {
        let keyboard = OnScreenKeyboard::new(None).unwrap();
        let engine = keyboard.engine();
        let field = TextBox::new();
        keyboard.attach_to(Some(field.upcast())).unwrap();
        let other = Rc::new(TextBufferTarget::default());
        engine.attach(other.clone()).unwrap();
        let weak = keyboard.downgrade();
        drop(keyboard);
        assert!(weak.upgrade().is_none());
        assert!(engine.target().is_some_and(|actual| Rc::ptr_eq(
            &actual,
            &(other.clone() as Rc<dyn ITextInputTarget>)
        )));
        engine.input_text(&"other owner".into()).unwrap();
        assert_eq!(other.text(), "other owner".into());
        engine.dispose().unwrap();
    });
}

#[test]
fn dropping_keyboard_stops_pending_private_view_fade_timer() {
    headless::run(|| {
        let keyboard = OnScreenKeyboard::new(None).unwrap();
        let engine = keyboard.engine();
        let touch = keyboard.touch_processor();
        let before = Dispatcher::timers_for_unit_tests();
        touch.glide_ended().raise(touch.as_ref());
        let timers = Dispatcher::timers_for_unit_tests()
            .into_iter()
            .filter(|timer| !before.iter().any(|old| Rc::ptr_eq(timer, old)))
            .collect::<Vec<_>>();
        assert!(!timers.is_empty());
        assert!(timers.iter().all(|timer| timer.is_enabled()));
        let weak = keyboard.downgrade();
        drop(keyboard);
        assert!(weak.upgrade().is_none());
        assert!(timers.iter().all(|timer| !timer.is_enabled()));
        assert_eq!(touch.glide_ended().handler_count(), 0);
        for timer in timers {
            Dispatcher::force_fire_timer_for_unit_tests(&timer);
        }
        engine.dispose().unwrap();
    });
}

#[test]
fn dropping_default_host_releases_its_keyboard_and_core_engine_without_explicit_shutdown() {
    headless::run(|| {
        let host = ferroboard_controls::ScreenKeyboardHost::new().unwrap();
        let keyboard = host.keyboard();
        let engine = keyboard.engine();
        let weak_host = host.downgrade();
        let weak_keyboard = keyboard.downgrade();
        let weak_engine = Rc::downgrade(&engine);
        let weak_settings = Rc::downgrade(&engine.settings());
        let weak_resources = Rc::downgrade(&engine.resources());
        drop(engine);
        drop(keyboard);
        drop(host);
        assert!(weak_host.upgrade().is_none());
        assert!(weak_keyboard.upgrade().is_none());
        assert!(
            weak_engine.upgrade().is_none(),
            "default engine must not survive through event cycles"
        );
        assert!(weak_settings.upgrade().is_none());
        assert!(weak_resources.upgrade().is_none());
        Dispatcher::current_dispatcher().run_jobs(None);
    });
}
