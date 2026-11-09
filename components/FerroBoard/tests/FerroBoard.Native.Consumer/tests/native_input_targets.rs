//! Native adapter ownership through public Rust APIs, without a disposable contract.
use ferroboard_controls::input::{PasswordBoxInputTarget, TextBoxInputTarget};
use ferroboard_core::editor::ITextInputTarget;
use ferroui_base::{Ref, threading::Dispatcher};
use ferroui_controls::{AppBuilder, Application, TextBox};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
};
use ferroui_system::threading::CancellationToken;
use std::{
    cell::Cell,
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

fn adapter(field: Ref<TextBox>, password: bool) -> (Rc<dyn ITextInputTarget>, Box<dyn Fn()>) {
    if password {
        let target = PasswordBoxInputTarget::new(field, None).unwrap();
        (target.clone(), Box::new(move || target.detach()))
    } else {
        let target = TextBoxInputTarget::new(field, None).unwrap();
        (target.clone(), Box::new(move || target.detach()))
    }
}

#[test]
fn native_detach_stops_observation_while_shared_targets_remain_editable() {
    run(|| {
        for password in [false, true] {
            let field = TextBox::new();
            if password {
                field.set_password_char('●');
            }
            let (target, detach) = adapter(field.clone(), password);
            let calls = Rc::new(Cell::new(0));
            let observed = calls.clone();
            target.changed().add(Rc::new(move |_| {
                observed.set(observed.get() + 1);
            }));
            field.set_text(Some("attached"));
            Dispatcher::current_dispatcher().run_jobs(None);
            assert!(calls.get() > 0);
            let before = calls.get();
            detach();
            detach();
            field.set_text(Some("detached"));
            field.set_caret_index(1);
            Dispatcher::current_dispatcher().run_jobs(None);
            assert_eq!(calls.get(), before);
            target.replace(0, 0, Some(&"😀".into())).unwrap();
            assert_eq!(field.text().as_deref(), Some("😀detached"));
            assert_eq!(target.selection_start(), 2);
            Dispatcher::current_dispatcher().run_jobs(None);
            assert_eq!(calls.get(), before);
            assert_eq!(target.changed().handler_count(), 1);
            target.changed().raise(target.as_ref());
            assert_eq!(calls.get(), before + 1);
            let weak = Rc::downgrade(&target);
            drop(detach);
            drop(target);
            assert!(weak.upgrade().is_none());
            field.set_text(Some("field still usable"));
        }
    });
}

#[test]
fn detach_from_a_native_callback_keeps_another_adapter_on_the_field_functional() {
    run(|| {
        let field = TextBox::new();
        let first = TextBoxInputTarget::new(field.clone(), None).unwrap();
        let second = TextBoxInputTarget::new(field.clone(), None).unwrap();
        let first_calls = Rc::new(Cell::new(0));
        let second_calls = Rc::new(Cell::new(0));
        let observed = first_calls.clone();
        let weak = Rc::downgrade(&first);
        first.changed().add(Rc::new(move |_| {
            observed.set(observed.get() + 1);
            weak.upgrade().unwrap().detach();
        }));
        let observed = second_calls.clone();
        second.changed().add(Rc::new(move |_| {
            observed.set(observed.get() + 1);
        }));
        field.set_text(Some("one"));
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(first_calls.get() > 0);
        assert!(second_calls.get() > 0);
        let before = (first_calls.get(), second_calls.get());
        field.set_text(Some("two"));
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(first_calls.get(), before.0);
        assert!(second_calls.get() > before.1);
        assert_eq!(second.text(), "two".into());
        drop(first);
        second.try_replace(0, 3, &"three".into()).unwrap();
        assert_eq!(field.text().as_deref(), Some("three"));
    });
}
