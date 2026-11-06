#[path = "support/headless.rs"]
mod headless;
use ferroboard_controls::{OnScreenKeyboard, ScreenKeyboardHost};
use ferroboard_core::editor::{ITextInputTarget, TextBufferTarget};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
#[test]
fn dropping_host_removes_subscriptions_without_releasing_a_retained_keyboard() {
    headless::run(|| {
        let keyboard = OnScreenKeyboard::new(None).unwrap();
        let engine = keyboard.engine();
        let settings = keyboard.settings();
        let settings_before = settings.property_changed().handler_count();
        let hide_before = keyboard.hide_requested().handler_count();
        let host = ScreenKeyboardHost::with_keyboard(keyboard.clone(), None).unwrap();
        assert_eq!(
            settings.property_changed().handler_count(),
            settings_before + 1
        );
        assert_eq!(keyboard.hide_requested().handler_count(), hide_before + 1);
        let weak = host.downgrade();
        drop(host);
        assert!(
            weak.upgrade().is_none(),
            "keyboard/settings callbacks must not retain the host"
        );
        assert_eq!(settings.property_changed().handler_count(), settings_before);
        assert_eq!(keyboard.hide_requested().handler_count(), hide_before);
        settings.set_floating(true);
        keyboard.hide_requested().raise(&keyboard);
        let target = Rc::new(TextBufferTarget::default());
        engine.attach(target.clone()).unwrap();
        engine.input_text(&"still usable".into()).unwrap();
        assert_eq!(target.text(), "still usable".into());
        keyboard.release_native_subscriptions().unwrap();
        engine.dispose().unwrap();
    });
}

use ferroui_base::{
    animation::{IClock, PlayState, SignedDuration},
    reactive::{IObservable, IObserver, LightweightSubject},
    threading::Dispatcher,
};
use ferroui_controls::{Control, Window};
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
struct Clock {
    ticks: LightweightSubject<SignedDuration>,
    state: Cell<PlayState>,
}
impl IObservable<SignedDuration> for Clock {
    fn subscribe(&self, observer: Rc<dyn IObserver<SignedDuration>>) -> Rc<dyn IDisposable> {
        self.ticks.subscribe(observer)
    }
}
impl IClock for Clock {
    fn play_state(&self) -> PlayState {
        self.state.get()
    }
    fn set_play_state(&self, state: PlayState) {
        self.state.set(state)
    }
}
#[test]
fn dropping_host_cancels_closing_animation_and_queued_callbacks() {
    headless::run(|| {
        let keyboard = OnScreenKeyboard::new(None).unwrap();
        let engine = keyboard.engine();
        let clock = Rc::new(Clock {
            ticks: LightweightSubject::new(),
            state: Cell::new(PlayState::Run),
        });
        let host =
            ScreenKeyboardHost::with_keyboard(keyboard.clone(), Some(clock.clone())).unwrap();
        let window = Window::new();
        window.set_width(800.0);
        window.set_height(600.0);
        window.set_content(Some(Control::boxed(host.clone())));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        host.set_is_keyboard_open(true);
        clock.ticks.on_next(SignedDuration::from_ticks(0));
        clock.ticks.on_next(SignedDuration::from_ticks(1_800_000));
        host.set_is_keyboard_open(false);
        clock.ticks.on_next(SignedDuration::from_ticks(2_000_000));
        assert!(clock.ticks.has_observers());
        window.set_content(None);
        window.close();
        let weak = host.downgrade();
        drop(host);
        assert!(
            weak.upgrade().is_none(),
            "closing animation must not retain the host"
        );
        // Native Clock stops immediately and removes its parent hook on the next parent tick.
        // The host callback must already be unreachable; no completion may change the keyboard.
        let visible = keyboard.is_visible();
        clock.ticks.on_next(SignedDuration::from_ticks(9_000_000));
        assert!(
            !clock.ticks.has_observers(),
            "stopped native clock must detach on its next tick"
        );
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(
            keyboard.is_visible(),
            visible,
            "canceled completion must not change retained keyboard"
        );
        keyboard.release_native_subscriptions().unwrap();
        engine.dispose().unwrap();
    });
}
#[test]
fn hide_callback_allows_last_external_host_owner_to_drop_reentrantly() {
    headless::run(|| {
        let keyboard = OnScreenKeyboard::new(None).unwrap();
        let engine = keyboard.engine();
        let host = ScreenKeyboardHost::with_keyboard(keyboard.clone(), None).unwrap();
        host.set_is_animated(false);
        host.set_is_keyboard_open(true);
        let weak = host.downgrade();
        let slot = Rc::new(RefCell::new(Some(host)));
        let owner = Rc::downgrade(&slot);
        slot.borrow()
            .as_ref()
            .unwrap()
            .keyboard_open_changed()
            .add(Rc::new(move |host| {
                assert!(!host.is_keyboard_open());
                owner.upgrade().unwrap().borrow_mut().take();
            }));
        let before = keyboard.hide_requested().handler_count();
        keyboard.hide_requested().raise(&keyboard);
        assert!(slot.borrow().is_none());
        assert!(weak.upgrade().is_none());
        assert_eq!(keyboard.hide_requested().handler_count(), before - 1);
        keyboard.hide_requested().raise(&keyboard);
        keyboard.release_native_subscriptions().unwrap();
        engine.dispose().unwrap();
    });
}

#[test]
fn removing_host_cancels_pressed_key_and_reattaches_with_a_new_native_adapter() {
    use ferroui_base::input::InputMethod;
    use ferroui_controls::{StackPanel, TextBox};
    headless::run(|| {
        let host = ScreenKeyboardHost::new().unwrap();
        host.set_is_animated(false);
        let keyboard = host.keyboard();
        keyboard.settings().set_auto_capitalization(false);
        keyboard.settings().set_glide_typing(false);
        keyboard.settings().set_clipboard_history_enabled(false);
        let field = TextBox::new();
        field.set_text(Some(""));
        host.set_content(Some(field.clone().upcast()));
        let root = StackPanel::new();
        root.children().add(host.clone());
        let window = Window::new();
        window.set_width(720.0);
        window.set_height(650.0);
        window.set_content(Some(Control::boxed(root.clone())));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(field.focus());
        let engine = keyboard.engine();
        let target = Rc::downgrade(&engine.target().unwrap());
        assert_eq!(
            InputMethod::get_is_input_method_enabled(&field),
            !cfg!(any(target_os = "android", target_os = "ios"))
        );
        let keys = engine.keyboard().unwrap();
        keys.layout(720.0, 240.0, None).unwrap();
        let bounds = keys.character_keys().unwrap()[0].touch_bounds();
        let (x, y) = (
            bounds.x + bounds.width / 2.0,
            bounds.y + bounds.height / 2.0,
        );
        let touch = keyboard.touch_processor();
        touch.pointer_down(81, x, y).unwrap();
        assert!(touch.pressed_keys().into_iter().next().is_some());
        root.children().remove(&host);
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(touch.pressed_keys().into_iter().next().is_none());
        touch.pointer_up(81, x, y).unwrap();
        assert_eq!(field.text().as_deref(), Some(""));
        assert!(engine.target().is_none());
        assert!(target.upgrade().is_none());
        assert!(InputMethod::get_is_input_method_enabled(&field));
        root.children().add(host.clone());
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(field.focus());
        assert!(engine.target().is_some());
        assert_eq!(
            InputMethod::get_is_input_method_enabled(&field),
            !cfg!(any(target_os = "android", target_os = "ios"))
        );
        engine.input_text(&"resumed🙂".into()).unwrap();
        assert_eq!(field.text().as_deref(), Some("resumed🙂"));
        host.release_native_subscriptions().unwrap();
        assert!(InputMethod::get_is_input_method_enabled(&field));
        window.close();
    });
}

#[test]
fn standalone_unload_preserves_target_but_cancels_touch_and_stops_focus_tracking() {
    use ferroui_base::input::InputMethod;
    use ferroui_controls::{StackPanel, TextBox};
    headless::run(|| {
        let keyboard = OnScreenKeyboard::new(None).unwrap();
        keyboard.settings().set_auto_capitalization(false);
        keyboard.settings().set_glide_typing(false);
        keyboard.settings().set_clipboard_history_enabled(false);
        let first = TextBox::new();
        let second = TextBox::new();
        first.set_text(Some(""));
        second.set_text(Some(""));
        let root = StackPanel::new();
        root.children().add(first.clone());
        root.children().add(second.clone());
        root.children().add(keyboard.clone());
        let window = Window::new();
        window.set_width(720.0);
        window.set_height(650.0);
        window.set_content(Some(Control::boxed(root.clone())));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(first.focus());
        let engine = keyboard.engine();
        let target = engine.target().unwrap();
        let keys = engine.keyboard().unwrap();
        keys.layout(720.0, 240.0, None).unwrap();
        let bounds = keys.character_keys().unwrap()[0].touch_bounds();
        let (x, y) = (
            bounds.x + bounds.width / 2.0,
            bounds.y + bounds.height / 2.0,
        );
        let touch = keyboard.touch_processor();
        touch.pointer_down(93, x, y).unwrap();
        assert!(touch.pressed_keys().into_iter().next().is_some());
        root.children().remove(&keyboard);
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(touch.pressed_keys().into_iter().next().is_none());
        touch.pointer_up(93, x, y).unwrap();
        assert_eq!(first.text().as_deref(), Some(""));
        assert!(Rc::ptr_eq(&target, &engine.target().unwrap()));
        assert_eq!(
            InputMethod::get_is_input_method_enabled(&first),
            !cfg!(any(target_os = "android", target_os = "ios"))
        );
        assert!(second.focus());
        assert!(
            Rc::ptr_eq(&target, &engine.target().unwrap()),
            "unloaded focus subscription must be absent"
        );
        root.children().add(keyboard.clone());
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(keyboard.target_element(), Some(second.clone().upcast()));
        assert!(InputMethod::get_is_input_method_enabled(&first));
        assert_eq!(
            InputMethod::get_is_input_method_enabled(&second),
            !cfg!(any(target_os = "android", target_os = "ios"))
        );
        keyboard.release_native_subscriptions().unwrap();
        assert!(InputMethod::get_is_input_method_enabled(&second));
        window.close();
    });
}
