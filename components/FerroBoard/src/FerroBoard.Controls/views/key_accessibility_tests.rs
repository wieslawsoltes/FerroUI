use super::*;
use ferroboard_core::keys::{KeyCode, ShiftState};
use ferroui_controls::automation::{
    peers::{AutomationControlType, AutomationPeer, ControlAutomationPeer},
    provider::IInvokeProvider,
};

fn by_code(f: &Fixture, code: i32) -> Ref<KeyView> {
    f.view
        .keys_layer
        .children()
        .snapshot()
        .iter()
        .filter_map(|child| child.cast::<KeyView>())
        .find(|view| view.key().is_visible() && view.key().code() == code)
        .expect("visible key with source code")
}
fn by_label(f: &Fixture, label: &str) -> Ref<KeyView> {
    f.view
        .keys_layer
        .children()
        .snapshot()
        .iter()
        .filter_map(|child| child.cast::<KeyView>())
        .find(|view| view.key().is_visible() && view.key().label() == Some(label.into()))
        .expect("visible computed key label")
}
fn peer(view: &KeyView) -> Ref<AutomationPeer> {
    ControlAutomationPeer::create_peer_for_element(view)
}
fn provider(view: &KeyView) -> Rc<dyn IInvokeProvider> {
    peer(view)
        .get_provider::<dyn IInvokeProvider>()
        .expect("native key press action")
}

#[test]
fn key_press_uses_live_computed_data_and_preserves_native_editor_focus() {
    crate::test_support::run(|| {
        use crate::input::TextBoxInputTarget;
        use ferroui_controls::{StackPanel, TextBox};
        let f = Fixture::new();
        f.engine.settings().set_show_suggestions(false);
        let field = TextBox::new();
        field.set_height(40.0);
        let target = TextBoxInputTarget::new(field.clone(), None).unwrap();
        f.engine.attach(target.clone()).unwrap();
        let panel = StackPanel::new();
        panel.children().add(field.clone());
        panel.children().add(f.view.clone());
        f.view.set_width(1000.0);
        f.view.set_height(240.0);
        let window = Window::new();
        window.set_content(Some(Control::boxed(panel)));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        f.view.refresh(true).unwrap();
        assert!(field.focus());
        let q = by_label(&f, "q");
        let automation = peer(&q);
        assert_eq!(
            automation.get_automation_control_type(),
            AutomationControlType::Button
        );
        assert_eq!(automation.get_name(), "q");
        assert!(!automation.is_keyboard_focusable());
        let press = provider(&q);
        press.invoke().unwrap();
        assert_eq!(field.text().as_deref(), Some("q"));
        assert!(field.is_focused());
        provider(&by_code(&f, KeyCode::Shift)).invoke().unwrap();
        f.view.refresh(false).unwrap();
        assert_eq!(automation.get_name(), "Q");
        press.invoke().unwrap();
        assert_eq!(field.text().as_deref(), Some("qQ"));
        provider(&by_code(&f, KeyCode::Delete)).invoke().unwrap();
        assert_eq!(field.text().as_deref(), Some("q"));
        assert!(field.is_focused());
        assert_eq!(f.touch.pressed_keys().into_iter().count(), 0);
        assert!(f.clock.entries.borrow().is_empty());
        window.close();
        f.engine.detach().unwrap();
        target.detach();
    });
}

#[test]
fn key_press_rejects_disabled_hidden_detached_and_replaced_views() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        let window = f.show();
        let q = by_label(&f, "q");
        let automation = peer(&q);
        let press = provider(&q);
        f.view.set_is_enabled(false);
        assert!(!automation.is_enabled());
        assert!(press.invoke().is_err());
        f.view.set_is_enabled(true);
        f.view.set_is_visible(false);
        assert!(press.invoke().is_err());
        f.view.set_is_visible(true);
        let row = f
            .engine
            .keyboard()
            .unwrap()
            .rows()
            .get(usize::try_from(q.key().row()).unwrap())
            .unwrap();
        row.remove(usize::try_from(q.key().column()).unwrap())
            .unwrap();
        assert!(
            press.invoke().is_err(),
            "removed model keys cannot use retained visual providers"
        );
        row.insert(usize::try_from(q.key().column()).unwrap(), q.key())
            .unwrap();
        f.engine.set_mode(KeyboardMode::Numeric).unwrap();
        assert!(
            press.invoke().is_err(),
            "a not-yet-refreshed old frame is stale"
        );
        f.view.refresh(false).unwrap();
        f.engine.set_mode(KeyboardMode::Characters).unwrap();
        f.view.refresh(false).unwrap();
        assert!(
            press.invoke().is_err(),
            "cached core keys do not revive an old visual"
        );
        let current = provider(&by_label(&f, "q"));
        current.invoke().unwrap();
        assert_eq!(f.target.text(), "q".into());
        window.set_content(None);
        assert!(current.invoke().is_err());
        assert_eq!(f.target.text(), "q".into());
        window.close();
    });
}

#[test]
fn atomic_key_action_coexists_with_a_real_held_shift_without_owning_pointer_timers() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        f.engine
            .settings()
            .set_double_tap_shift_for_caps_lock(false);
        let window = f.show();
        let shift = by_code(&f, KeyCode::Shift).key();
        let contact = Pointer::new(85, PointerType::Touch, true);
        f.down(&window, contact.clone(), center(&shift));
        let timers = f.clock.entries.borrow().len();
        assert_eq!(f.touch.pressed_keys().into_iter().count(), 1);
        provider(&by_label(&f, "A")).invoke().unwrap();
        assert_eq!(f.target.text(), "A".into());
        assert_eq!(f.touch.pressed_keys().into_iter().count(), 1);
        assert_eq!(f.clock.entries.borrow().len(), timers);
        f.up(&window, contact, center(&shift));
        assert_eq!(f.engine.shift_state(), ShiftState::Unshifted);
        assert_eq!(f.touch.pressed_keys().into_iter().count(), 0);
        window.close();
    });
}

#[test]
fn key_action_uses_replaced_target_and_reports_engine_failures() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        let window = f.show();
        let q = provider(&by_label(&f, "q"));
        let next = Rc::new(TextBufferTarget::default());
        f.engine.attach(next.clone()).unwrap();
        f.view.refresh(false).unwrap();
        q.invoke().unwrap();
        assert!(f.target.text().is_empty());
        assert_eq!(next.text(), "q".into());
        let shift = provider(&by_code(&f, KeyCode::Shift));
        let clock = f.engine.clock();
        f.engine.set_clock_nullable(None);
        let error = shift
            .invoke()
            .expect_err("engine failure cannot become a successful press");
        assert!(
            error
                .message()
                .contains(&ferroboard_core::engine::EngineError::NullReference.to_string()),
            "{error}"
        );
        assert_eq!(next.text(), "q".into());
        f.engine.set_clock_nullable(clock);
        window.close();
    });
}

#[test]
fn retained_key_provider_does_not_keep_the_view_parent_or_engine_alive() {
    crate::test_support::run(|| {
        let f = Fixture::new();
        let window = f.show();
        let q = by_label(&f, "q");
        let press = provider(&q);
        let parent = f.view.downgrade();
        let key = q.downgrade();
        let engine = Rc::downgrade(&f.engine);
        window.set_content(None);
        window.close();
        drop(f);
        assert!(parent.upgrade().is_none());
        assert!(engine.upgrade().is_none());
        assert!(
            press.invoke().is_err(),
            "a live key with no parent cannot edit"
        );
        drop(q);
        assert!(key.upgrade().is_none());
        assert!(
            press.invoke().is_err(),
            "an expired key returns a native error"
        );
    });
}

#[test]
fn enter_action_navigates_native_fields_and_preserves_handled_precedence() {
    crate::test_support::run(|| {
        use crate::input::{ScreenKeyboardInput, TextBoxInputTarget};
        use ferroboard_core::editor::EnterAction;
        use ferroui_controls::{StackPanel, TextBox};
        let f = Fixture::new();
        let first = TextBox::new();
        let second = TextBox::new();
        ScreenKeyboardInput::set_enter_action(&first, Some(Rc::new(EnterAction::Next)));
        let target = TextBoxInputTarget::new(first.clone(), None).unwrap();
        f.engine.attach(target.clone()).unwrap();
        let panel = StackPanel::new();
        panel.children().add(first.clone());
        panel.children().add(second.clone());
        panel.children().add(f.view.clone());
        f.view.set_width(1000.0);
        f.view.set_height(240.0);
        let window = Window::new();
        window.set_content(Some(Control::boxed(panel)));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        f.view.refresh(true).unwrap();
        assert!(first.focus());
        provider(&by_code(&f, KeyCode::Enter)).invoke().unwrap();
        assert!(
            second.is_focused(),
            "Next is intentional editor focus movement"
        );
        let count = Rc::new(Cell::new(0));
        let calls = count.clone();
        let handled = TextBoxInputTarget::new(
            first.clone(),
            Some(Rc::new(move |action| {
                assert_eq!(action, EnterAction::Next);
                calls.set(calls.get() + 1);
                true
            })),
        )
        .unwrap();
        f.engine.attach(handled.clone()).unwrap();
        f.view.refresh(false).unwrap();
        assert!(first.focus());
        provider(&by_code(&f, KeyCode::Enter)).invoke().unwrap();
        assert_eq!(count.get(), 1);
        assert!(
            first.is_focused(),
            "handled Enter must not run native traversal"
        );
        assert!(first.text().as_deref().unwrap_or_default().is_empty());
        window.close();
        f.engine.detach().unwrap();
        handled.detach();
        target.detach();
    });
}
