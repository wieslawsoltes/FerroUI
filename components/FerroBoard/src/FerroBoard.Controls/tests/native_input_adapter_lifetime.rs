#[path = "support/headless.rs"]
mod headless;
use ferroboard_controls::input::{PasswordBoxInputTarget, TextBoxInputTarget};
use ferroboard_core::editor::ITextInputTarget;
use ferroui_base::threading::Dispatcher;
use ferroui_controls::TextBox;
use std::rc::Rc;

#[test]
fn dropping_native_adapters_does_not_require_explicit_shutdown_of_borrowed_fields() {
    headless::run(|| {
        for password in [false, true] {
            let field = TextBox::new();
            if password {
                field.set_password_char('●');
            }
            field.set_text(Some("fixture"));
            Dispatcher::current_dispatcher().run_jobs(None);
            let target: Rc<dyn ITextInputTarget> = if password {
                PasswordBoxInputTarget::new(field.clone(), None).unwrap()
            } else {
                TextBoxInputTarget::new(field.clone(), None).unwrap()
            };
            let weak = Rc::downgrade(&target);
            drop(target);
            assert!(
                weak.upgrade().is_none(),
                "native field callbacks must not retain their adapter"
            );
            assert_eq!(field.text().as_deref(), Some("fixture"));
            field.set_text(Some("field remains usable"));
            Dispatcher::current_dispatcher().run_jobs(None);
            assert_eq!(field.text().as_deref(), Some("field remains usable"));
        }
    });
}

use ferroui_base::Ref;
use std::cell::{Cell, RefCell};
fn adapter(field: Ref<TextBox>, password: bool) -> Rc<dyn ITextInputTarget> {
    if password {
        PasswordBoxInputTarget::new(field, None).unwrap()
    } else {
        TextBoxInputTarget::new(field, None).unwrap()
    }
}
#[test]
fn pending_suppression_work_does_not_retain_an_adapter_or_edit_a_borrowed_field() {
    headless::run(|| {
        let field = TextBox::new();
        let target = TextBoxInputTarget::new(field.clone(), None).unwrap();
        target.try_replace(0, 0, &"internal".into()).unwrap();
        let weak = Rc::downgrade(&target);
        drop(target);
        assert!(
            weak.upgrade().is_none(),
            "queued suppression must hold only a weak receiver"
        );
        field.set_text(Some("external after release"));
        field.set_caret_index(4);
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(field.text().as_deref(), Some("external after release"));
        assert_eq!(field.caret_index(), 4);
    });
}
#[test]
fn earlier_native_event_handler_can_release_adapter_before_its_snapshot_callback() {
    headless::run(|| {
        for password in [false, true] {
            let field = TextBox::new();
            if password {
                field.set_password_char('●');
            }
            let slot = Rc::new(RefCell::new(None::<Rc<dyn ITextInputTarget>>));
            let weak_slot = Rc::downgrade(&slot);
            let release = move || {
                if let Some(slot) = weak_slot.upgrade() {
                    slot.borrow_mut().take();
                }
            };
            let token = if password {
                field.text_changing(move |_, _| release())
            } else {
                field.text_changed(move |_, _| release())
            };
            let target = adapter(field.clone(), password);
            let weak = Rc::downgrade(&target);
            let changes = Rc::new(Cell::new(0));
            let observed = changes.clone();
            target
                .changed()
                .add(Rc::new(move |_| observed.set(observed.get() + 1)));
            *slot.borrow_mut() = Some(target);
            field.set_text(Some("external"));
            Dispatcher::current_dispatcher().run_jobs(None);
            assert!(weak.upgrade().is_none());
            assert_eq!(changes.get(), 0);
            assert_eq!(field.text().as_deref(), Some("external"));
            if password {
                field.remove_handler(TextBox::text_changing_event(), token);
            } else {
                field.remove_handler(TextBox::text_changed_event(), token);
            }
        }
    });
}
#[test]
fn an_inflight_changed_callback_holds_its_adapter_until_reentrant_owner_release_returns() {
    headless::run(|| {
        for password in [false, true] {
            let field = TextBox::new();
            if password {
                field.set_password_char('●');
            }
            let target = adapter(field.clone(), password);
            let weak = Rc::downgrade(&target);
            let slot = Rc::new(RefCell::new(Some(target.clone())));
            let weak_slot = Rc::downgrade(&slot);
            let observed = Rc::new(Cell::new(false));
            let seen = observed.clone();
            let during = weak.clone();
            target.changed().add(Rc::new(move |_| {
                weak_slot.upgrade().unwrap().borrow_mut().take();
                assert!(
                    during.upgrade().is_some(),
                    "native callback must keep its active receiver alive"
                );
                seen.set(true);
            }));
            drop(target);
            field.set_text(Some("external"));
            Dispatcher::current_dispatcher().run_jobs(None);
            assert!(observed.get());
            assert!(weak.upgrade().is_none());
        }
    });
}
#[test]
fn dropping_one_adapter_keeps_another_adapter_on_the_same_field_functional() {
    headless::run(|| {
        let field = TextBox::new();
        let first = TextBoxInputTarget::new(field.clone(), None).unwrap();
        let second = TextBoxInputTarget::new(field.clone(), None).unwrap();
        let weak = Rc::downgrade(&first);
        let changes = Rc::new(Cell::new(0));
        let observed = changes.clone();
        second
            .changed()
            .add(Rc::new(move |_| observed.set(observed.get() + 1)));
        drop(first);
        assert!(weak.upgrade().is_none());
        field.set_text(Some("second"));
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(changes.get() > 0);
        second.try_replace(0, 6, &"usable".into()).unwrap();
        assert_eq!(field.text().as_deref(), Some("usable"));
    });
}
#[test]
fn explicit_detach_stops_native_hooks_without_clearing_public_handlers_or_disabling_methods() {
    headless::run(|| {
        for password in [false, true] {
            let field = TextBox::new();
            if password {
                field.set_password_char('●');
            }
            let (target, detach): (Rc<dyn ITextInputTarget>, Box<dyn Fn()>) = if password {
                let concrete = PasswordBoxInputTarget::new(field.clone(), None).unwrap();
                (concrete.clone(), Box::new(move || concrete.detach()))
            } else {
                let concrete = TextBoxInputTarget::new(field.clone(), None).unwrap();
                (concrete.clone(), Box::new(move || concrete.detach()))
            };
            let changes = Rc::new(Cell::new(0));
            let observed = changes.clone();
            target
                .changed()
                .add(Rc::new(move |_| observed.set(observed.get() + 1)));
            detach();
            detach();
            drop(detach);
            target
                .replace(0, 0, Some(&"still callable".into()))
                .unwrap();
            field.set_text(Some("external after dispose"));
            Dispatcher::current_dispatcher().run_jobs(None);
            assert_eq!(changes.get(), 0);
            assert_eq!(target.changed().handler_count(), 1);
            target.changed().raise(target.as_ref());
            assert_eq!(changes.get(), 1);
            let weak = Rc::downgrade(&target);
            drop(target);
            assert!(weak.upgrade().is_none());
        }
    });
}
#[test]
fn earlier_selection_observer_can_release_adapter_without_moving_the_caret() {
    headless::run(|| {
        let field = TextBox::new();
        field.set_text(Some("abcd"));
        Dispatcher::current_dispatcher().run_jobs(None);
        let slot = Rc::new(RefCell::new(None::<Rc<dyn ITextInputTarget>>));
        let owner = Rc::downgrade(&slot);
        let observer = field.property_changed(move |change| {
            if (change.property() == TextBox::selection_start_property().as_property()
                || change.property() == TextBox::selection_end_property().as_property())
                && let Some(slot) = owner.upgrade()
            {
                slot.borrow_mut().take();
            }
        });
        let target = TextBoxInputTarget::new(field.clone(), None).unwrap();
        let weak = Rc::downgrade(&target);
        let changes = Rc::new(Cell::new(0));
        let observed = changes.clone();
        target
            .changed()
            .add(Rc::new(move |_| observed.set(observed.get() + 1)));
        *slot.borrow_mut() = Some(target);
        field.set_caret_index(2);
        assert!(weak.upgrade().is_none());
        assert_eq!(changes.get(), 0);
        assert_eq!(field.caret_index(), 2);
        observer.dispose();
    });
}
