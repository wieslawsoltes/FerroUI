//! Native button events/commands checked against unchanged Process* fixtures.
use crate::compat::ContentDialogButton;
use crate::controls::{RibbonColorDialogHost, RibbonDialogButtonDeferral, RibbonDialogHost};
use ferroui_base::{
    BoxedValue, Ref,
    input::ICommand,
    interactivity::RoutedEventArgs,
    reactive::{Disposable, IDisposable},
};
use ferroui_controls::{Application, Button, Control, Window};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
type SubscriptionSlot = Rc<RefCell<Option<Rc<dyn IDisposable>>>>;
struct Command {
    trace: Rc<RefCell<Vec<String>>>,
    label: String,
    allowed: bool,
    parameter: Option<BoxedValue>,
}
impl ICommand for Command {
    fn can_execute(&self, p: Option<&BoxedValue>) -> bool {
        if let Some(expected) = &self.parameter {
            assert!(Rc::ptr_eq(expected, p.unwrap()));
        }
        self.allowed
    }
    fn execute(&self, p: Option<&BoxedValue>) {
        if let Some(expected) = &self.parameter {
            assert!(Rc::ptr_eq(expected, p.unwrap()));
        }
        self.trace
            .borrow_mut()
            .push(format!("command:{}", self.label));
    }
    fn can_execute_changed(&self, _: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}
fn run(test: impl FnOnce() + Send + 'static) {
    crate::tests::run(move || {
        Application::current()
            .unwrap()
            .styles()
            .add(ferroui_themes_simple::SimpleTheme::new().as_style());
        test();
    });
}
fn mount() -> (Ref<RibbonColorDialogHost>, Ref<Window>) {
    let host = RibbonColorDialogHost::new();
    host.set_primary_button_text(Some("Primary".into()));
    host.set_secondary_button_text(Some("Secondary".into()));
    host.set_close_button_text(Some("Close".into()));
    let window = Window::new();
    window.set_content(Some(Control::boxed(host.clone())));
    window.show();
    window.update_layout();
    (host, window)
}
fn invoke(host: &RibbonDialogHost, button: ContentDialogButton) {
    let button = match button {
        ContentDialogButton::Primary => host.primary_button(),
        ContentDialogButton::Secondary => host.secondary_button(),
        _ => host.close_button(),
    }
    .unwrap();
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}
fn command(
    host: &RibbonDialogHost,
    button: ContentDialogButton,
    value: Option<Rc<dyn ICommand>>,
    parameter: Option<BoxedValue>,
) {
    match button {
        ContentDialogButton::Primary => {
            host.set_primary_button_command(value);
            host.set_primary_button_command_parameter(parameter);
        }
        ContentDialogButton::Secondary => {
            host.set_secondary_button_command(value);
            host.set_secondary_button_command_parameter(parameter);
        }
        _ => {
            host.set_close_button_command(value);
            host.set_close_button_command_parameter(parameter);
        }
    }
}
#[test]
fn native_dialog_buttons_match_all_sixty_source_command_cancel_deferral_and_hiding_cases() {
    run(|| {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/dialog-outcomes.json"))
                .unwrap();
        let mut count = 0;
        for case in fixture["cases"].as_array().unwrap().iter() {
            count += 1;
            let (host, window) = mount();
            // Establish the source _hiding=true input with a real pending native
            // Closing deferral, rather than exposing a test-only state setter.
            let pending_close = Rc::new(RefCell::new(None));
            if case["hiding"] == true {
                let pending = pending_close.clone();
                let token = host.dialog_closing(move |args| {
                    args.set_cancel(true);
                    pending.replace(Some(args.get_deferral()));
                });
                assert!(!host.request_close(ContentDialogButton::None));
                assert!(host.is_dialog_closing());
                token.dispose();
            }
            let label = case["button"].as_str().unwrap().to_owned();
            let button = match label.as_str() {
                "Primary" => ContentDialogButton::Primary,
                "Secondary" => ContentDialogButton::Secondary,
                _ => ContentDialogButton::Close,
            };
            let trace = Rc::new(RefCell::new(Vec::new()));
            let results = Rc::new(RefCell::new(Vec::new()));
            let seen = trace.clone();
            let output = results.clone();
            host.on_button_action(Some(Rc::new(move |button| {
                let (raw, name) = match button {
                    ContentDialogButton::Primary => (1, "Primary"),
                    ContentDialogButton::Secondary => (2, "Secondary"),
                    _ => (0, "None"),
                };
                output.borrow_mut().push(raw);
                seen.borrow_mut().push(format!("hide:{name}"));
            })));
            if case["command"] == true {
                command(
                    &host,
                    button,
                    Some(Rc::new(Command {
                        trace: trace.clone(),
                        label: label.clone(),
                        allowed: true,
                        parameter: None,
                    })),
                    None,
                );
            }
            let mode = case["mode"].as_str().unwrap().to_owned();
            let event_mode = mode.clone();
            let seen = trace.clone();
            let deferred: Rc<RefCell<Option<RibbonDialogButtonDeferral>>> =
                Rc::new(RefCell::new(None));
            let pending = deferred.clone();
            let args = Rc::new(RefCell::new(None));
            let captured = args.clone();
            let token = host.button_click(button, move |args| {
                seen.borrow_mut().push(format!("event:{label}"));
                args.set_cancel(matches!(
                    event_mode.as_str(),
                    "cancel_event" | "defer_uncancel"
                ));
                if event_mode.starts_with("defer_") {
                    pending.replace(Some(args.get_deferral()));
                }
                captured.replace(Some(args.clone()));
            });
            invoke(&host, button);
            assert_eq!(
                serde_json::json!(*results.borrow()),
                case["during"],
                "{case}"
            );
            if let Some(deferred) = deferred.take() {
                args.borrow()
                    .as_ref()
                    .unwrap()
                    .set_cancel(mode == "defer_cancel");
                deferred.complete();
            }
            assert_eq!(
                serde_json::json!(*results.borrow()),
                case["after"],
                "{case}"
            );
            assert_eq!(serde_json::json!(*trace.borrow()), case["trace"], "{case}");
            token.dispose();
            if let Some(deferral) = pending_close.take() {
                deferral.complete().unwrap();
            }
            window.close();
        }
        assert_eq!(count, 60);
    });
}
#[test]
fn native_dialog_command_reads_event_replacement_and_uses_current_parameter_identity() {
    run(|| {
        for button in [
            ContentDialogButton::Primary,
            ContentDialogButton::Secondary,
            ContentDialogButton::Close,
        ] {
            for allowed in [false, true] {
                let (host, window) = mount();
                let trace = Rc::new(RefCell::new(Vec::new()));
                command(
                    &host,
                    button,
                    Some(Rc::new(Command {
                        trace: trace.clone(),
                        label: "old".into(),
                        allowed: true,
                        parameter: None,
                    })),
                    None,
                );
                let payload: BoxedValue = Rc::new("new parameter".to_owned());
                let weak = host.downgrade();
                let seen = trace.clone();
                let token = host.button_click(button, move |_| {
                    seen.borrow_mut().push("event".into());
                    command(
                        &weak.upgrade().unwrap(),
                        button,
                        Some(Rc::new(Command {
                            trace: seen.clone(),
                            label: "replacement".into(),
                            allowed,
                            parameter: Some(payload.clone()),
                        })),
                        Some(payload.clone()),
                    );
                });
                let seen = trace.clone();
                host.on_button_action(Some(Rc::new(move |_| {
                    seen.borrow_mut().push("close".into())
                })));
                invoke(&host, button);
                assert_eq!(
                    *trace.borrow(),
                    if allowed {
                        vec!["event", "command:replacement", "close"]
                    } else {
                        vec!["event", "close"]
                    }
                );
                assert!(host.primary_button().unwrap().command().is_none());
                token.dispose();
                window.close();
            }
        }
    });
}
#[test]
fn native_dialog_event_snapshot_disposal_reentrancy_and_pending_deferral_do_not_root_host() {
    run(|| {
        let host = RibbonDialogHost::new();
        let trace = Rc::new(RefCell::new(Vec::new()));
        let second: SubscriptionSlot = Rc::new(RefCell::new(None));
        let remove = second.clone();
        let seen = trace.clone();
        let entered = Rc::new(Cell::new(false));
        let weak = host.downgrade();
        let first = host.button_click(ContentDialogButton::Primary, move |args| {
            seen.borrow_mut().push("first");
            if !entered.replace(true) {
                remove.borrow().as_ref().unwrap().dispose();
                weak.upgrade()
                    .unwrap()
                    .process_button(ContentDialogButton::Primary);
            }
            args.set_cancel(true);
        });
        let seen = trace.clone();
        second.replace(Some(
            host.button_click(ContentDialogButton::Primary, move |_| {
                seen.borrow_mut().push("second")
            }),
        ));
        host.process_button(ContentDialogButton::Primary);
        assert_eq!(*trace.borrow(), ["first", "first", "second"]);
        first.dispose();
        let pending = Rc::new(RefCell::new(None));
        let seen = pending.clone();
        let token = host.button_click(ContentDialogButton::Close, move |args| {
            seen.replace(Some(args.get_deferral()));
        });
        host.process_button(ContentDialogButton::Close);
        let weak = host.downgrade();
        drop(host);
        assert!(weak.upgrade().is_none());
        pending.borrow().as_ref().unwrap().complete();
        token.dispose();
    });
}
