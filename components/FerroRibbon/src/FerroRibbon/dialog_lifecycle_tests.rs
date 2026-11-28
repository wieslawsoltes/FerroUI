//! Original Hide/Closing/Deferral cases through real native overlay and color future.
use crate::{
    compat::ContentDialogButton,
    controls::{
        RibbonColorDialogHost, RibbonDialogClosingDeferral, RibbonDialogClosingEventArgs,
        RibbonDialogDeferralError,
    },
};
use ferroui_base::{media::Color, threading::Dispatcher};
use ferroui_controls::{Application, Window};
use futures_util::FutureExt;
use std::{cell::RefCell, rc::Rc};
fn run(test: impl FnOnce() + Send + 'static) {
    crate::tests::run(move || {
        Application::current()
            .unwrap()
            .styles()
            .add(ferroui_themes_simple::SimpleTheme::new().as_style());
        test();
    });
}
fn bool_text(value: bool) -> &'static str {
    if value { "True" } else { "False" }
}
fn result_name(result: crate::controls::RibbonDialogOutcome) -> &'static str {
    match result {
        crate::controls::RibbonDialogOutcome::Primary => "Primary",
        crate::controls::RibbonDialogOutcome::Secondary => "Secondary",
        _ => "None",
    }
}
#[test]
fn native_dialog_closing_matches_twenty_four_source_lifecycle_cases_with_actual_queued_future() {
    run(|| {
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/dialog-closing.json")).unwrap();
        assert_eq!(corpus.as_array().unwrap().len(), 24);
        for case in corpus.as_array().unwrap() {
            let window = Window::new();
            window.set_width(800.);
            window.set_height(650.);
            window.show();
            window.update_layout();
            let initial = Color::from_rgb(1, 2, 3);
            let task = crate::dialog_future_test_support::show_color(
                Some(window.clone().upcast()),
                initial,
                None,
            );
            let host = window
                .get_visual_descendants()
                .find_map(|v| v.cast::<RibbonColorDialogHost>())
                .expect("actual native dialog overlay");
            Dispatcher::ui_thread().run_jobs(None);
            let mode = case["mode"].as_str().unwrap().to_owned();
            let pending: Rc<RefCell<Vec<RibbonDialogClosingDeferral>>> =
                Rc::new(RefCell::new(Vec::new()));
            let captured: Rc<RefCell<Option<Rc<RibbonDialogClosingEventArgs>>>> =
                Rc::new(RefCell::new(None));
            let trace = Rc::new(RefCell::new(Vec::new()));
            let current = task.clone();
            let observed = trace.clone();
            let weak = host.downgrade();
            let handles = pending.clone();
            let args_slot = captured.clone();
            let event_mode = mode.clone();
            let closing = host.dialog_closing(move |args| {
                let host = weak.upgrade().unwrap();
                observed.borrow_mut().push(format!(
                    "closing:{}:{}",
                    result_name(args.result()),
                    bool_text(host.is_dialog_closing())
                ));
                args_slot.replace(Some(args.clone()));
                args.set_cancel(matches!(
                    event_mode.as_str(),
                    "cancel_event" | "defer_uncancel"
                ));
                if matches!(
                    event_mode.as_str(),
                    "defer_accept"
                        | "defer_cancel"
                        | "defer_uncancel"
                        | "double_complete"
                        | "two_reverse"
                        | "inline"
                ) {
                    handles.borrow_mut().push(args.get_deferral());
                }
                if event_mode == "two_reverse" {
                    handles.borrow_mut().push(args.get_deferral());
                }
                if event_mode == "inline" {
                    handles.borrow()[0].complete().unwrap();
                    assert!(
                        host.is_dialog_showing(),
                        "inline completion waits for all subscribers"
                    );
                    assert!(host.visual_parent().is_some());
                    observed.borrow_mut().push(format!(
                        "inline_completed:{}",
                        bool_text(current.clone().now_or_never().is_some())
                    ));
                }
            });
            let observed = trace.clone();
            let current = task.clone();
            let weak = host.downgrade();
            let closed = host.dialog_closed(move |args| {
                let host = weak.upgrade().unwrap();
                assert!(!host.is_dialog_showing());
                assert!(host.visual_parent().is_none());
                observed.borrow_mut().push(format!(
                    "closed:{}:{}:{}",
                    result_name(args.result()),
                    bool_text(host.is_dialog_closing()),
                    bool_text(current.clone().now_or_never().is_some())
                ));
            });
            let button = match case["result"].as_i64().unwrap() {
                1 => ContentDialogButton::Primary,
                2 => ContentDialogButton::Secondary,
                _ => ContentDialogButton::None,
            };
            assert_eq!(
                host.request_close(button),
                case["synchronous"].as_bool().unwrap(),
                "{case}"
            );
            let snapshot = || serde_json::json!({"showing":host.is_dialog_showing(),"hiding":host.is_dialog_closing(),"popupOpen":host.visual_parent().is_some(),"hasChild":host.visual_parent().is_some(),"taskCompleted":task.clone().now_or_never().is_some()});
            let expected = |key: &str| {
                let mut value = case[key].clone();
                value.as_object_mut().unwrap().remove("queued");
                value
            };
            assert_eq!(snapshot(), expected("during"), "{case}");
            if mode != "inline" && !pending.borrow().is_empty() {
                captured
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .set_cancel(mode == "defer_cancel");
                let handles = pending.borrow();
                for (index, deferral) in handles.iter().rev().enumerate() {
                    deferral.complete().unwrap();
                    if mode == "two_reverse" && index == 0 {
                        assert!(host.is_dialog_showing());
                        assert!(host.is_dialog_closing());
                        assert!(task.clone().now_or_never().is_none());
                    }
                }
            }
            if mode == "double_complete" {
                assert_eq!(
                    pending.borrow()[0].complete(),
                    Err(RibbonDialogDeferralError::AlreadyCompleted)
                );
                assert_eq!(case["error"], "Deferral already completed.");
            }
            assert_eq!(snapshot(), expected("beforeDispatch"), "{case}");
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(snapshot(), expected("after"), "{case}");
            let expected_trace: Vec<_> = case["trace"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|s| s.as_str() != Some("visual"))
                .cloned()
                .collect();
            assert_eq!(
                serde_json::json!(*trace.borrow()),
                serde_json::json!(expected_trace),
                "{case}"
            );
            if task.clone().now_or_never().is_some() {
                assert_eq!(
                    task.clone().now_or_never(),
                    Some(Ok(
                        (button == ContentDialogButton::Primary).then_some(initial)
                    ))
                );
            }
            closing.dispose();
            closed.dispose();
            if host.is_dialog_showing() {
                assert!(host.hide());
                Dispatcher::ui_thread().run_jobs(None);
            }
            window.close();
        }
    });
}

#[test]
fn native_dialog_future_drops_its_waker_without_dismissing_and_rejects_same_host_other_owner() {
    run(|| {
        use crate::controls::{RibbonDialogHost, RibbonDialogOpenError};
        use std::{
            future::Future,
            pin::Pin,
            sync::{
                Arc,
                atomic::{AtomicUsize, Ordering},
            },
            task::{Context, Wake, Waker},
        };
        struct Count(AtomicUsize);
        impl Wake for Count {
            fn wake(self: Arc<Self>) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let window = Window::new();
        window.show();
        let other = Window::new();
        other.show();
        let host = RibbonDialogHost::new();
        host.set_close_button_text(Some("Close".into()));
        let mut future = host.show_dialog(window.clone().upcast()).unwrap();
        assert!(matches!(
            host.show_dialog(other.clone().upcast()),
            Err(RibbonDialogOpenError::AlreadyOpen)
        ));
        let count = Arc::new(Count(AtomicUsize::new(0)));
        let weak = Arc::downgrade(&count);
        let waker = Waker::from(count.clone());
        let mut context = Context::from_waker(&waker);
        assert!(Pin::new(&mut future).poll(&mut context).is_pending());
        drop(waker);
        drop(count);
        assert!(weak.upgrade().is_some());
        drop(future);
        assert!(weak.upgrade().is_none());
        assert!(host.is_dialog_showing());
        assert!(host.hide());
        Dispatcher::ui_thread().run_jobs(None);
        assert!(!host.is_dialog_showing());
        other.close();
        window.close();
    });
}
