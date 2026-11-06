//! Public control ownership and synchronous event contracts in a separate consumer.
use ferroboard_controls::{OnScreenKeyboard, input::ScreenKeyboardInput};
use ferroboard_core::{
    editor::{EnterAction, ITextInputTarget, TextBufferTarget},
    keys::{KeyCode, KeyType},
    layouts::TextKeyData,
};
use ferroui_base::threading::Dispatcher;
use ferroui_controls::{AppBuilder, Application, TextBox};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
};
use ferroui_system::threading::CancellationToken;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{Arc, OnceLock},
};

fn run(action: impl Fn() + Send + Sync + 'static) {
    static SESSION: OnceLock<Arc<HeadlessUnitTestSession>> = OnceLock::new();
    SESSION
        .get_or_init(|| {
            HeadlessUnitTestSession::start_new(|| {
                AppBuilder::configure::<Application>()
                    .use_headless(FerroHeadlessPlatformOptions::default())
            })
        })
        .dispatch(
            move || {
                ferroboard_controls::register_types();
                action();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}

fn key(keyboard: &OnScreenKeyboard, code: i32) {
    keyboard
        .engine()
        .input_key(TextKeyData::function(code, KeyType::Function, None).as_ref())
        .unwrap();
}

#[test]
fn releasing_one_view_keeps_another_views_shared_engine_target() {
    run(|| {
        let first = OnScreenKeyboard::new(None).unwrap();
        let engine = first.engine();
        let second = OnScreenKeyboard::new(Some(engine.clone())).unwrap();
        first.settings().set_auto_capitalization(false);
        first.settings().set_haptic_feedback(false);
        let a = TextBox::new();
        let b = TextBox::new();
        first.attach_to(Some(a.clone().upcast())).unwrap();
        second.attach_to(Some(b.clone().upcast())).unwrap();
        let active = engine.target().unwrap();
        first.release_native_subscriptions().unwrap();
        assert!(engine.target().is_some_and(|t| Rc::ptr_eq(&t, &active)));
        assert!(first.target_element().is_none());
        assert_eq!(second.target_element(), Some(b.clone().upcast()));
        engine.input_text(&"second".into()).unwrap();
        assert_eq!(b.text().as_deref(), Some("second"));
        assert_eq!(a.text(), None);
        first.release_native_subscriptions().unwrap();
        engine.input_text(&"!".into()).unwrap();
        assert_eq!(b.text().as_deref(), Some("second!"));
        second.release_native_subscriptions().unwrap();
        assert!(
            engine.target().is_none(),
            "the active adapter belongs to second"
        );
        engine.dispose().unwrap();
    });
}

#[test]
fn explicit_view_release_preserves_borrowed_custom_targets_like_drop() {
    run(|| {
        let keyboard = OnScreenKeyboard::new(None).unwrap();
        let engine = keyboard.engine();
        let target = Rc::new(TextBufferTarget::default());
        let expected: Rc<dyn ITextInputTarget> = target.clone();
        keyboard.attach(target.clone()).unwrap();
        keyboard.release_native_subscriptions().unwrap();
        keyboard.release_native_subscriptions().unwrap();
        assert!(engine.target().is_some_and(|t| Rc::ptr_eq(&t, &expected)));
        assert_eq!(engine.state_changed().handler_count(), 0);
        engine.input_text(&"still owned by caller".into()).unwrap();
        assert_eq!(target.text(), "still owned by caller".into());
        engine.dispose().unwrap();
    });
}

#[test]
fn repeated_release_cannot_retire_a_target_installed_by_a_release_callback() {
    run(|| {
        let first = OnScreenKeyboard::new(None).unwrap();
        let engine = first.engine();
        let second = OnScreenKeyboard::new(Some(engine.clone())).unwrap();
        let a = TextBox::new();
        let b = TextBox::new();
        first.attach_to(Some(a.upcast())).unwrap();
        let replacement = b.clone();
        let other = second.downgrade();
        let token = first.target_changed().add(Rc::new(move |owner| {
            if owner.target_element().is_none() {
                other
                    .upgrade()
                    .unwrap()
                    .attach_to(Some(replacement.clone().upcast()))
                    .unwrap();
            }
        }));
        first.release_native_subscriptions().unwrap();
        first.target_changed().remove(token);
        let active = engine.target().unwrap();
        first.release_native_subscriptions().unwrap();
        assert!(engine.target().is_some_and(|t| Rc::ptr_eq(&t, &active)));
        engine.input_text(&"reentrant".into()).unwrap();
        assert_eq!(b.text().as_deref(), Some("reentrant"));
        second.release_native_subscriptions().unwrap();
        engine.dispose().unwrap();
    });
}

#[test]
fn shared_engine_routes_feedback_before_requests_and_releases_only_one_views_hooks() {
    run(|| {
        let first = OnScreenKeyboard::new(None).unwrap();
        let engine = first.engine();
        engine.settings().set_haptic_feedback(false);
        let second = OnScreenKeyboard::new(Some(engine.clone())).unwrap();
        let trace = Rc::new(RefCell::new(Vec::new()));
        for (view, name) in [(&first, "A"), (&second, "B")] {
            for (event, kind) in [
                (view.settings_requested(), "settings"),
                (view.voice_input_requested(), "voice"),
                (view.hide_requested(), "hide"),
            ] {
                let log = trace.clone();
                event.add(Rc::new(move |_| {
                    log.borrow_mut().push(format!("{name}:{kind}"))
                }));
            }
            let log = trace.clone();
            view.feedback_requested().add(Rc::new(move |_| {
                log.borrow_mut().push(format!("{name}:feedback"))
            }));
        }
        for (code, request) in [
            (KeyCode::Settings, "settings"),
            (KeyCode::VoiceInput, "voice"),
            (KeyCode::ImeHideUi, "hide"),
        ] {
            trace.borrow_mut().clear();
            key(&first, code);
            assert_eq!(
                *trace.borrow(),
                [
                    "A:feedback",
                    "B:feedback",
                    &format!("A:{request}"),
                    &format!("B:{request}")
                ]
            );
        }
        first.release_native_subscriptions().unwrap();
        trace.borrow_mut().clear();
        key(&second, KeyCode::Settings);
        assert_eq!(*trace.borrow(), ["B:feedback", "B:settings"]);
        second.release_native_subscriptions().unwrap();
        trace.borrow_mut().clear();
        key(&second, KeyCode::VoiceInput);
        assert!(trace.borrow().is_empty());
        engine.dispose().unwrap();
    });
}

#[test]
fn enter_callbacks_share_handled_state_and_done_uses_the_final_value() {
    run(|| {
        let keyboard = OnScreenKeyboard::new(None).unwrap();
        let engine = keyboard.engine();
        engine.settings().set_haptic_feedback(false);
        let field = TextBox::new();
        ScreenKeyboardInput::set_enter_action(&field, Some(Rc::new(EnterAction::Done)));
        keyboard.attach_to(Some(field.clone().upcast())).unwrap();
        let trace = Rc::new(RefCell::new(Vec::new()));
        let last_handles = Rc::new(Cell::new(false));
        let expected = field.clone().upcast::<ferroui_base::Visual>();
        let log = trace.clone();
        let one = keyboard.enter_action_requested().add(Rc::new(move |args| {
            assert_eq!(args.element(), Some(expected.clone()));
            assert_eq!(args.action(), EnterAction::Done);
            assert!(!args.handled());
            args.set_handled(true);
            log.borrow_mut().push("first");
        }));
        let handles = last_handles.clone();
        let log = trace.clone();
        let two = keyboard.enter_action_requested().add(Rc::new(move |args| {
            assert!(args.handled());
            args.set_handled(handles.get());
            log.borrow_mut().push("second");
        }));
        let log = trace.clone();
        let hide = keyboard
            .hide_requested()
            .add(Rc::new(move |_| log.borrow_mut().push("hide")));
        key(&keyboard, KeyCode::Enter);
        assert_eq!(*trace.borrow(), ["first", "second", "hide"]);
        trace.borrow_mut().clear();
        last_handles.set(true);
        key(&keyboard, KeyCode::Enter);
        assert_eq!(*trace.borrow(), ["first", "second"]);
        assert_eq!(field.text(), None, "Done must not insert a newline");
        keyboard.enter_action_requested().remove(one);
        keyboard.enter_action_requested().remove(two);
        keyboard.hide_requested().remove(hide);
        keyboard.release_native_subscriptions().unwrap();
        Dispatcher::current_dispatcher().run_jobs(None);
        engine.dispose().unwrap();
    });
}
