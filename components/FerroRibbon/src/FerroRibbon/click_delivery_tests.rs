//! Actual native controls, base-class subscriptions and exact-source event traces.
use crate::controls::{
    IRibbonItem, RibbonButton, RibbonCheckBox, RibbonDropDownButton, RibbonToggleButton,
};
use ferroui_base::{
    Ref,
    data::core::{ValueType, ValueTypes},
    interactivity::{Interactive, RoutedEventArgs},
};
use ferroui_controls::{
    Button, Control, StackPanel, Window, automation::peers::ButtonAutomationPeer,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn mount(control: &Control) -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(control.to_ref())));
    window.show();
    window
}
fn item(control: &Control) -> Rc<dyn IRibbonItem> {
    ValueTypes::try_cast(
        &Control::boxed(control.to_ref()),
        ValueType::of::<Rc<dyn IRibbonItem>>(),
    )
    .unwrap()
    .downcast_ref::<Rc<dyn IRibbonItem>>()
    .unwrap()
    .clone()
}
fn invoke(button: &Button) {
    ButtonAutomationPeer::new(button).invoke().unwrap();
}
fn boolean(value: bool) -> &'static str {
    if value { "True" } else { "False" }
}

#[test]
fn actual_click_delivery_matches_eight_source_event_trace_projections() {
    crate::group_tests::run(|| {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/button-click.json")).unwrap();
        let mut checked = 0;
        for case in fixture["cases"].as_array().unwrap() {
            let id = case["id"].as_str().unwrap();
            if ![
                "ordinary",
                "first-handled",
                "all-handled",
                "remove-handler",
                "add-handler",
                "reentrant",
                "first-throws",
                "second-throws",
            ]
            .contains(&id)
            {
                continue;
            }
            checked += 1;
            let button = RibbonButton::new();
            let trace = Rc::new(RefCell::new(Vec::new()));
            let args = Rc::new(RefCell::new(Vec::<RoutedEventArgs>::new()));
            let depth = Rc::new(Cell::new(0));
            let second = Rc::new(Cell::new(None));
            let once = Rc::new(Cell::new(false));
            let t = trace.clone();
            let a = args.clone();
            let d = depth.clone();
            let token = second.clone();
            let id_owned = id.to_owned();
            button.click(move |sender, e| {
                assert!(e.is_source(sender));
                a.borrow_mut().push(e.clone());
                let id = id_owned.as_str();
                t.borrow_mut()
                    .push(format!("first:{}:{}", d.get(), boolean(e.handled())));
                if [
                    "first-handled",
                    "all-handled",
                    "remove-handler",
                    "add-handler",
                    "reentrant",
                ]
                .contains(&id)
                {
                    e.set_handled(true);
                }
                if !once.replace(true) {
                    if id == "remove-handler" {
                        assert!(sender.remove_handler(Button::click_event(), token.get().unwrap()));
                    }
                    if id == "add-handler" {
                        let t = t.clone();
                        sender.add_handler(Button::click_event(), move |_, e| {
                            t.borrow_mut()
                                .push(format!("added:{}", boolean(e.handled())))
                        });
                    }
                }
                if id == "reentrant" && d.get() == 0 {
                    d.set(1);
                    invoke(sender.downcast_ref::<Button>().unwrap());
                    d.set(0);
                }
                assert!(id != "first-throws", "first-handler");
            });
            let t = trace.clone();
            let d = depth.clone();
            let id_owned = id.to_owned();
            second.set(Some(button.click(move |_, e| {
                t.borrow_mut()
                    .push(format!("second:{}:{}", d.get(), boolean(e.handled())));
                if id_owned == "all-handled" {
                    e.set_handled(true);
                }
                assert!(id_owned != "second-throws", "second-handler");
            })));
            let window = mount(&button);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                invoke(&button);
                if ["remove-handler", "add-handler"].contains(&id) {
                    invoke(&button);
                }
            }));
            let failure = result.err().map(|p| {
                p.downcast_ref::<String>()
                    .cloned()
                    .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                    .expect("expected handler panic payload")
            });
            assert_eq!(failure.as_deref(), case["failure"].as_str(), "{id}");
            let expected = case["trace"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|v| v.as_str())
                .filter(|v| {
                    v.starts_with("first:") || v.starts_with("second:") || v.starts_with("added:")
                })
                .map(str::to_owned)
                .collect::<Vec<_>>();
            assert_eq!(*trace.borrow(), expected, "{id}");
            let args = args.borrow();
            assert_eq!(
                args.len(),
                case["argsCount"].as_u64().unwrap() as usize,
                "{id}"
            );
            let distinct = args
                .iter()
                .enumerate()
                .filter(|(i, a)| !args[..*i].contains(a))
                .count();
            assert_eq!(
                distinct,
                case["distinctArgs"].as_u64().unwrap() as usize,
                "{id}"
            );
            assert_eq!(
                args.iter().map(|a| a.handled()).collect::<Vec<_>>(),
                case["finalHandled"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_bool().unwrap())
                    .collect::<Vec<_>>(),
                "{id}"
            );
            window.close();
        }
        assert_eq!(checked, 8);
    });
}

#[test]
fn actual_click_delivery_preserves_base_subscriptions_and_command_order_for_all_families() {
    crate::group_tests::run(|| {
        for source in [
            RibbonButton::new().upcast::<Button>(),
            RibbonDropDownButton::new().upcast(),
            RibbonToggleButton::new().upcast(),
            RibbonCheckBox::new().upcast(),
        ] {
            let parent = StackPanel::new();
            parent.children().add(source.clone().upcast::<Control>());
            let trace = Rc::new(RefCell::new(Vec::new()));
            let t = trace.clone();
            parent.add_handler_with(
                Button::click_event(),
                move |_, _| t.borrow_mut().push("ancestor"),
                Interactive::DEFAULT_ROUTES,
                true,
            );
            let t = trace.clone();
            source.click(move |_, e| {
                t.borrow_mut().push("first");
                e.set_handled(true);
            });
            let t = trace.clone();
            source.click(move |_, e| {
                assert!(e.handled());
                t.borrow_mut().push("second");
            });
            let t = trace.clone();
            source.set_command(Some(crate::commands::adapt_core_command(
                ferroribbon_core::commands::RibbonRelayCommand::new(move || {
                    t.borrow_mut().push("command")
                }),
            )));
            let window = mount(&parent);
            invoke(&source);
            assert_eq!(*trace.borrow(), ["first", "second", "command"]);
            if let Some(toggle) = source.cast::<ferroui_controls::primitives::ToggleButton>() {
                assert_eq!(toggle.is_checked(), Some(true));
            }
            window.close();
        }
    });
}

#[test]
fn actual_click_delivery_keeps_removed_handler_alive_only_through_current_snapshot() {
    crate::group_tests::run(|| {
        let button = RibbonButton::new();
        let token = Rc::new(Cell::new(None));
        let payload = Rc::new(());
        let weak = Rc::downgrade(&payload);
        let weak_in_first = weak.clone();
        let token_in_first = token.clone();
        let once = Cell::new(false);
        button.click(move |sender, e| {
            if !once.replace(true) {
                assert!(
                    sender.remove_handler(Button::click_event(), token_in_first.get().unwrap())
                );
                assert!(
                    weak_in_first.upgrade().is_some(),
                    "current snapshot retains removed callback"
                );
            }
            e.set_handled(true);
        });
        let calls = Rc::new(Cell::new(0));
        let c = calls.clone();
        token.set(Some(button.click(move |_, _| {
            assert_eq!(Rc::strong_count(&payload), 1);
            c.set(c.get() + 1);
        })));
        let window = mount(&button);
        invoke(&button);
        assert_eq!(calls.get(), 1);
        assert!(
            weak.upgrade().is_none(),
            "removed callback released after snapshot"
        );
        invoke(&button);
        assert_eq!(calls.get(), 1);
        window.close();
    });
}

#[test]
fn actual_click_delivery_linked_copy_keeps_source_and_copy_handler_counts_exact() {
    crate::group_tests::run(|| {
        let source = RibbonButton::new();
        let copy = item(&source)
            .create_linked_copy()
            .unwrap()
            .cast::<RibbonButton>()
            .unwrap();
        let trace = Rc::new(RefCell::new(Vec::new()));
        for (button, first, second) in [
            (&source, "source-first", "source-second"),
            (&copy, "copy-first", "copy-second"),
        ] {
            let t = trace.clone();
            button.click(move |_, e| {
                t.borrow_mut().push(first);
                e.set_handled(true);
            });
            let t = trace.clone();
            button.click(move |_, _| t.borrow_mut().push(second));
        }
        let parent = StackPanel::new();
        parent.children().add(source.clone().upcast::<Control>());
        parent.children().add(copy.clone().upcast::<Control>());
        let window = mount(&parent);
        invoke(&copy);
        assert_eq!(
            *trace.borrow(),
            ["source-first", "source-second", "copy-first", "copy-second"]
        );
        crate::controls::RibbonItemHelper::unlink(&copy);
        trace.borrow_mut().clear();
        invoke(&copy);
        assert_eq!(
            *trace.borrow(),
            ["source-first", "source-second", "copy-first", "copy-second"]
        );
        crate::controls::RibbonItemHelper::set_source_item(&copy, None);
        trace.borrow_mut().clear();
        invoke(&copy);
        assert_eq!(*trace.borrow(), ["copy-first", "copy-second"]);
        window.close();
    });
}
