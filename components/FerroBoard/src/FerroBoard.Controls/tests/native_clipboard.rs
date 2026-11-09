#[path = "support/headless.rs"]
mod headless;
use ferroboard_controls::{
    OnScreenKeyboard,
    services::{ClipboardMonitor, SystemClipboardService},
};
use ferroboard_core::editor::{
    InputAttributes, InputAttributesValues, InputKind, TextBufferTarget,
};
use ferroui_base::{
    input::platform::{ClipboardError, ClipboardErrorKind},
    threading::Dispatcher,
};
use ferroui_controls::{Control, Window};
use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::Arc,
    task::{Context, Poll, Waker},
};
fn flush() {
    Dispatcher::current_dispatcher().run_jobs(None);
}
fn ready<T>(future: impl Future<Output = T>) -> T {
    let mut future = std::pin::pin!(future);
    match future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("local test clipboard must be ready"),
    }
}
#[test]
fn sampled_changes_deduplicate_skip_empty_and_rebaseline_after_privacy_or_denial() {
    headless::run(|| {
        let value = Rc::new(RefCell::new(Ok(Some("initial".to_owned()))));
        let calls = Rc::new(Cell::new(0));
        let seen = Rc::new(RefCell::new(Vec::new()));
        let input = value.clone();
        let read_count = calls.clone();
        let changes = seen.clone();
        let monitor = ClipboardMonitor::new(
            move || {
                read_count.set(read_count.get() + 1);
                Box::pin(std::future::ready(input.borrow().clone()))
            },
            move |text| changes.borrow_mut().push(text),
        );
        monitor.set_enabled(true);
        flush();
        assert!(seen.borrow().is_empty());
        for text in [Some("A"), Some("A"), Some(""), None, Some("A")] {
            *value.borrow_mut() = Ok(text.map(str::to_owned));
            monitor.refresh();
            flush();
        }
        assert_eq!(&*seen.borrow(), &["A", "A"]);
        monitor.set_enabled(false);
        let previous = calls.get();
        *value.borrow_mut() = Ok(Some("private".into()));
        monitor.refresh();
        flush();
        assert_eq!(calls.get(), previous);
        monitor.set_enabled(true);
        flush();
        assert_eq!(&*seen.borrow(), &["A", "A"]);
        *value.borrow_mut() = Err(ClipboardError::access_denied("denied"));
        monitor.refresh();
        flush();
        assert_eq!(
            monitor.last_error().unwrap().kind(),
            ClipboardErrorKind::AccessDenied
        );
        *value.borrow_mut() = Ok(Some("during denial".into()));
        monitor.refresh();
        flush();
        assert!(monitor.last_error().is_none());
        *value.borrow_mut() = Ok(Some("public".into()));
        monitor.refresh();
        flush();
        assert_eq!(&*seen.borrow(), &["A", "A", "public"]);
    });
}
#[derive(Default)]
struct Gate {
    result: RefCell<Option<Result<Option<String>, ClipboardError>>>,
    wake: RefCell<Option<Waker>>,
    dropped: Cell<usize>,
}
struct PendingRead(Rc<Gate>);
impl Future for PendingRead {
    type Output = Result<Option<String>, ClipboardError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(value) = self.0.result.borrow_mut().take() {
            Poll::Ready(value)
        } else {
            *self.0.wake.borrow_mut() = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
impl Drop for PendingRead {
    fn drop(&mut self) {
        self.0.dropped.set(self.0.dropped.get() + 1);
    }
}
#[test]
fn pending_reads_are_single_flight_cancel_immediately_and_do_not_retain_monitor() {
    headless::run(|| {
        let gate = Rc::new(Gate::default());
        let input = gate.clone();
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        let monitor = ClipboardMonitor::new(
            move || {
                count.set(count.get() + 1);
                Box::pin(PendingRead(input.clone()))
            },
            |_| panic!("baseline/cancel must not emit"),
        );
        monitor.set_enabled(true);
        flush();
        monitor.refresh();
        monitor.refresh();
        assert_eq!(calls.get(), 1);
        monitor.set_enabled(false);
        assert_eq!(gate.dropped.get(), 1);
        flush();
        monitor.set_enabled(true);
        flush();
        assert_eq!(calls.get(), 2);
        *gate.result.borrow_mut() = Some(Ok(Some("baseline".into())));
        gate.wake.borrow_mut().take().unwrap().wake();
        flush();
        assert_eq!(gate.dropped.get(), 2);
        monitor.refresh();
        flush();
        assert_eq!(calls.get(), 3);
        drop(monitor);
        assert_eq!(gate.dropped.get(), 3);
        flush();
        let before = calls.get();
        flush();
        assert_eq!(calls.get(), before);
    });
}
#[test]
fn reentrant_reader_can_disable_monitor_and_drop_its_pending_read() {
    headless::run(|| {
        let slot = Rc::new(RefCell::new(None::<std::rc::Weak<ClipboardMonitor>>));
        let reader_slot = slot.clone();
        let gate = Rc::new(Gate::default());
        let input = gate.clone();
        let monitor = Rc::new(ClipboardMonitor::new(
            move || {
                reader_slot
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .upgrade()
                    .unwrap()
                    .set_enabled(false);
                Box::pin(PendingRead(input.clone()))
            },
            |_| panic!("disabled"),
        ));
        *slot.borrow_mut() = Some(Rc::downgrade(&monitor));
        monitor.set_enabled(true);
        flush();
        assert!(!monitor.is_enabled());
        assert_eq!(gate.dropped.get(), 1);
        let weak = Rc::downgrade(&monitor);
        drop(monitor);
        flush();
        assert!(weak.upgrade().is_none());
    });
}
#[test]
fn native_keyboard_records_external_changes_but_not_private_or_unloaded_reads() {
    headless::run(|| {
        let keyboard = OnScreenKeyboard::new(None).unwrap();
        keyboard.set_auto_attach(false);
        let engine = keyboard.engine();
        let window = Window::new();
        window.set_width(800.0);
        window.set_height(500.0);
        window.set_content(Some(Control::boxed(keyboard.clone())));
        let service = SystemClipboardService::for_visual(keyboard.clone().upcast());
        ready(SystemClipboardService::new().write_text("initial")).unwrap();
        window.show();
        flush();
        let sample = |text: &str| {
            ready(service.write_text(text)).unwrap();
            keyboard.refresh_system_clipboard();
            flush();
        };
        assert!(engine.clipboard_history().items().unwrap().is_empty());
        sample("external 😀");
        assert_eq!(
            engine.clipboard_history().primary().unwrap().text,
            "external 😀".into()
        );
        engine.settings().set_clipboard_history_enabled(false);
        sample("disabled");
        engine.settings().set_clipboard_history_enabled(true);
        flush();
        assert_eq!(engine.clipboard_history().items().unwrap().len(), 1);
        engine.settings().set_incognito_mode(true);
        sample("incognito");
        engine.settings().set_incognito_mode(false);
        flush();
        for attrs in [
            InputAttributesValues {
                kind: InputKind::Password,
                ..Default::default()
            },
            InputAttributesValues {
                kind: InputKind::NumericPassword,
                ..Default::default()
            },
            InputAttributesValues {
                is_private: true,
                ..Default::default()
            },
        ] {
            let target = Rc::new(TextBufferTarget::new(
                "".into(),
                Some(Arc::new(InputAttributes::from_values(attrs))),
            ));
            engine.attach(target).unwrap();
            sample("private");
            engine.detach().unwrap();
            flush();
        }
        assert_eq!(engine.clipboard_history().items().unwrap().len(), 1);
        sample("public");
        assert_eq!(engine.clipboard_history().items().unwrap().len(), 2);
        window.set_content(None);
        flush();
        assert_eq!(
            ready(service.read_text()).unwrap_err().kind(),
            ClipboardErrorKind::Platform
        );
        ready(SystemClipboardService::new().write_text("unloaded")).unwrap();
        keyboard.refresh_system_clipboard();
        flush();
        assert_eq!(engine.clipboard_history().items().unwrap().len(), 2);
        window.close();
        keyboard.release_native_subscriptions().unwrap();
        engine.dispose().unwrap();
        let weak = keyboard.downgrade();
        drop(keyboard);
        flush();
        assert!(weak.upgrade().is_none());
    });
}

#[test]
fn native_dispatcher_timer_requests_samples_and_stops_when_disabled() {
    headless::run(|| {
        let before = Dispatcher::timers_for_unit_tests();
        let value = Rc::new(RefCell::new("initial".to_owned()));
        let input = value.clone();
        let seen = Rc::new(Cell::new(false));
        let changed = seen.clone();
        let monitor = ClipboardMonitor::new(
            move || Box::pin(std::future::ready(Ok(Some(input.borrow().clone())))),
            move |_| changed.set(true),
        );
        monitor.set_enabled(true);
        flush();
        let timer = Dispatcher::timers_for_unit_tests()
            .into_iter()
            .find(|timer| !before.iter().any(|old| Rc::ptr_eq(timer, old)))
            .unwrap();
        assert_eq!(timer.interval(), std::time::Duration::from_millis(500));
        *value.borrow_mut() = "timer change".into();
        Dispatcher::force_fire_timer_for_unit_tests(&timer);
        assert!(seen.get(), "native timer callback must request a sample");
        monitor.set_enabled(false);
        assert!(!timer.is_enabled());
    });
}

#[test]
fn history_callback_can_reentrantly_release_monitor() {
    headless::run(|| {
        let slot = Rc::new(RefCell::new(None::<ClipboardMonitor>));
        let owner = Rc::downgrade(&slot);
        let text = Rc::new(RefCell::new("baseline".to_owned()));
        let read = text.clone();
        *slot.borrow_mut() = Some(ClipboardMonitor::new(
            move || Box::pin(std::future::ready(Ok(Some(read.borrow().clone())))),
            move |_| {
                owner.upgrade().unwrap().borrow_mut().take();
            },
        ));
        slot.borrow().as_ref().unwrap().set_enabled(true);
        flush();
        *text.borrow_mut() = "changed".into();
        slot.borrow().as_ref().unwrap().refresh();
        flush();
        assert!(slot.borrow().is_none());
    });
}
