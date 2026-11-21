use ferroribbon_core::{RibbonString, commands::*};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::{
    any::Any,
    cell::{Cell, RefCell},
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};
fn s(value: &str) -> RibbonString {
    value.into()
}
fn text(value: RibbonString) -> String {
    value.to_utf8().unwrap()
}
fn source() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../tests/fixtures/catalog.json")).unwrap()
}
type Trace = Rc<RefCell<Vec<String>>>;
fn push(t: &Trace, value: impl Into<String>) {
    t.borrow_mut().push(value.into());
}
fn simple(c: &Rc<RibbonCommandCatalog>, id: &str, label: &str) -> Rc<RibbonCommandDescriptor> {
    c.register_action(
        Some(s(id)),
        Some(s(label)),
        Rc::new(|_| {}),
        None,
        RibbonCommandMetadata::default(),
    )
}
#[test]
fn catalog_registers_executes_and_pushes_state_original_model_test() {
    let c = RibbonCommandCatalog::new();
    let count = Rc::new(Cell::new(0));
    let seen = count.clone();
    c.register_action(
        Some(s("bold")),
        Some(s("Bold")),
        Rc::new(move |_| seen.set(seen.get() + 1)),
        None,
        RibbonCommandMetadata {
            shortcut: Some(s("Ctrl+B")),
            ..Default::default()
        },
    );
    let changes = Rc::new(RefCell::new(Vec::new()));
    let observed = changes.clone();
    c.command_state_changed().add(Rc::new(move |(_, args)| {
        observed.borrow_mut().push(args.property_name())
    }));
    assert!(c.execute(Some(&s("BOLD")), None));
    c.set_enabled(Some(&s("bold")), false);
    assert!(!c.execute(Some(&s("bold")), None));
    c.set_checked(Some(&s("bold")), Some(true));
    assert_eq!(count.get(), 1);
    assert_eq!(
        *changes.borrow(),
        vec![Some(s("IsEnabled")), Some(s("IsChecked"))]
    );
}
#[test]
fn catalog_keeps_registration_order_original_regression() {
    let c = RibbonCommandCatalog::new();
    for id in ["z", "a", "m", "b"] {
        simple(&c, id, id);
    }
    simple(&c, "a", "A again");
    assert_eq!(
        c.commands()
            .iter()
            .map(|d| text(d.id()))
            .collect::<Vec<_>>(),
        ["z", "a", "m", "b"]
    );
}
#[test]
fn original_catalog_trace_includes_double_predicate_and_detach_order() {
    let c = RibbonCommandCatalog::new();
    let t = Trace::default();
    let out = t.clone();
    let owner = Rc::downgrade(&c);
    c.commands_changed().add(Rc::new(move |(sender, id)| {
        assert!(Rc::ptr_eq(sender, &owner.upgrade().unwrap()));
        push(&out, format!("changed:{}", text(id.clone())));
    }));
    let out = t.clone();
    c.command_state_changed().add(Rc::new(move |(_, e)| {
        push(
            &out,
            format!(
                "state:{}:{}",
                text(e.id().unwrap()),
                text(e.property_name().unwrap())
            ),
        )
    }));
    let out = t.clone();
    c.executed().add(Rc::new(move |(_, e)| {
        push(&out, format!("executed:{}:null", text(e.id().unwrap())))
    }));
    let action = t.clone();
    let predicate = t.clone();
    let d = c.register_action(
        Some(s("bold")),
        Some(s("Bold")),
        Rc::new(move |_| push(&action, "invoke")),
        Some(Rc::new(move |_| {
            push(&predicate, "can");
            true
        })),
        RibbonCommandMetadata {
            shortcut: Some(s("Ctrl+B")),
            ..Default::default()
        },
    );
    let snapshot = c.commands();
    push(&t, format!("result:{}", c.execute(Some(&s("BOLD")), None)));
    c.set_enabled(Some(&s("bold")), false);
    push(
        &t,
        format!("disabled:{}", c.execute(Some(&s("bold")), None)),
    );
    c.set_checked(Some(&s("bold")), Some(true));
    c.set_checked(Some(&s("bold")), Some(true));
    d.keywords().borrow_mut().extend([Some(s("weight")), None]);
    push(&t, format!("keywords:{}", d.keywords().borrow().len()));
    let replacement = simple(&c, "BOLD", "New");
    d.set_label(Some(s("Old")));
    replacement.set_label(Some(s("Replacement")));
    push(&t, format!("snapshot:{}", Rc::ptr_eq(&snapshot[0], &d)));
    push(
        &t,
        format!(
            "order:{}",
            c.commands()
                .iter()
                .map(|d| text(d.id()))
                .collect::<Vec<_>>()
                .join(",")
        ),
    );
    push(&t, format!("removed:{}", c.unregister(Some(&s("bOlD")))));
    replacement.set_label(Some(s("Detached")));
    push(&t, format!("missing:{}", c.unregister(Some(&s("bold")))));
    push(&t, format!("null-find:{}", c.find(None).is_none()));
    push(
        &t,
        format!("null-resolve:{}", c.try_resolve(None).is_some()),
    );
    assert_eq!(serde_json::json!(&*t.borrow()), source()["basic"]);
}
#[test]
fn command_replacement_removal_clearing_and_predicate_reentry_match_source() {
    for mode in ["replace", "remove", "clear", "disable", "second-false"] {
        let c = RibbonCommandCatalog::new();
        let t = Trace::default();
        let out = t.clone();
        c.executed().add(Rc::new(move |(_, e)| {
            push(&out, format!("executed:{}", text(e.id().unwrap())))
        }));
        let owner = Rc::downgrade(&c);
        let action = t.clone();
        let predicate = t.clone();
        let count = Cell::new(0);
        c.register_action(
            Some(s("a")),
            Some(s("A")),
            Rc::new(move |_| push(&action, "old.execute")),
            Some(Rc::new(move |_| {
                count.set(count.get() + 1);
                push(&predicate, format!("can:{}", count.get()));
                let c = owner.upgrade().unwrap();
                match mode {
                    "replace" => {
                        let out = predicate.clone();
                        c.register_action(
                            Some(s("A")),
                            Some(s("New")),
                            Rc::new(move |_| push(&out, "new.execute")),
                            None,
                            Default::default(),
                        );
                    }
                    "remove" => {
                        c.unregister(Some(&s("a")));
                    }
                    "clear" => c.find(Some(&s("a"))).unwrap().set_command(None),
                    "disable" => c.set_enabled(Some(&s("a")), false),
                    _ => {}
                }
                mode != "second-false" || count.get() == 1
            })),
            Default::default(),
        );
        match catch_unwind(AssertUnwindSafe(|| c.execute(Some(&s("a")), None))) {
            Ok(result) => push(&t, format!("result:{result}")),
            Err(error) => {
                let msg = error
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| error.downcast_ref::<&str>().copied())
                    .unwrap();
                assert!(msg.starts_with("NullReference:"));
                push(&t, "NullReferenceException");
            }
        }
        assert_eq!(
            serde_json::json!(&*t.borrow()),
            source()["mutations"][mode],
            "{mode}"
        );
    }
}
#[test]
fn re_registering_same_descriptor_moves_its_subscription_and_reentrant_register_is_ordered() {
    let c = RibbonCommandCatalog::new();
    let d = RibbonCommandDescriptor::new(Some(s("a")), Some(s("A")), None);
    let t = Trace::default();
    let out = t.clone();
    d.property_changed()
        .add(Rc::new(move |_| push(&out, "before")));
    c.register(Some(d.clone()));
    let out = t.clone();
    d.property_changed()
        .add(Rc::new(move |_| push(&out, "after")));
    let out = t.clone();
    c.command_state_changed()
        .add(Rc::new(move |_| push(&out, "catalog")));
    c.register(Some(d.clone()));
    d.set_label(Some(s("Changed")));
    assert_eq!(serde_json::json!(&*t.borrow()), source()["subscriptions"]);
    let c = RibbonCommandCatalog::new();
    let t = Trace::default();
    let out = t.clone();
    let nested = Cell::new(false);
    c.commands_changed().add(Rc::new(move |(c, id)| {
        push(&out, format!("first:{}", text(id.clone())));
        if !nested.replace(true) {
            simple(c, "b", "B");
        }
    }));
    let out = t.clone();
    c.commands_changed().add(Rc::new(move |(_, id)| {
        push(&out, format!("second:{}", text(id.clone())))
    }));
    simple(&c, "a", "A");
    push(
        &t,
        format!(
            "order:{}",
            c.commands()
                .iter()
                .map(|d| text(d.id()))
                .collect::<Vec<_>>()
                .join(",")
        ),
    );
    assert_eq!(serde_json::json!(&*t.borrow()), source()["reentrant"]);
}
#[test]
fn catalog_lookup_preserves_non_ascii_and_malformed_utf16_comparison() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/ordinal.json")).unwrap();
    let units = |v: &serde_json::Value| {
        RibbonString::from_utf16(
            v.as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
    };
    for case in corpus["pairs"].as_array().unwrap() {
        let c = RibbonCommandCatalog::new();
        let a = units(&case["a"]);
        let b = units(&case["b"]);
        let d = c.register(Some(RibbonCommandDescriptor::new(
            Some(a.clone()),
            None,
            None,
        )));
        assert!(d.id().ptr_eq(&a));
        assert_eq!(
            c.find(Some(&b)).is_some(),
            case["equal"].as_bool().unwrap(),
            "{case}"
        );
    }
    let c = RibbonCommandCatalog::new();
    simple(&c, " x ", "space");
    assert!(c.find(Some(&s("x"))).is_none());
    assert!(c.find(Some(&s(" X "))).is_some());
}
#[test]
fn executed_arguments_share_sender_parameter_and_event_identity() {
    let c = RibbonCommandCatalog::new();
    let parameter: Rc<dyn Any> = Rc::new(String::from("parameter"));
    let expected = parameter.clone();
    c.register_action(
        Some(s("id")),
        None,
        Rc::new(move |actual| {
            assert!(std::ptr::eq(
                actual.unwrap().downcast_ref::<String>().unwrap(),
                expected.downcast_ref::<String>().unwrap()
            ))
        }),
        None,
        Default::default(),
    );
    let stored = Rc::new(RefCell::new(None::<Rc<RibbonCommandExecutedEventArgs>>));
    let out = stored.clone();
    let owner = Rc::downgrade(&c);
    let expected = parameter.clone();
    c.executed().add(Rc::new(move |(sender, args)| {
        assert!(Rc::ptr_eq(sender, &owner.upgrade().unwrap()));
        assert!(Rc::ptr_eq(&args.parameter().unwrap(), &expected));
        *out.borrow_mut() = Some(args.clone());
    }));
    let out = stored.clone();
    c.executed().add(Rc::new(move |(_, args)| {
        assert!(Rc::ptr_eq(out.borrow().as_ref().unwrap(), args))
    }));
    assert!(c.execute(Some(&s("ID")), Some(parameter)));
    assert_eq!(stored.borrow().as_ref().unwrap().id(), Some(s("ID")));
    let args = RibbonCommandStateChangedEventArgs::new(None, None);
    assert!(args.id().is_none());
    assert!(args.property_name().is_none());
}
#[test]
fn descriptor_defaults_keywords_identity_and_command_subscription_boundary() {
    let c = RibbonCommandCatalog::new();
    let command = RibbonRelayCommand::new(|| {});
    let d = RibbonCommandDescriptor::new(Some(s("")), None, Some(command.clone()));
    assert_eq!(d.label(), None);
    assert!(d.is_enabled());
    assert_eq!(d.is_checked(), None);
    assert!(d.icon().is_none());
    assert!(d.description().is_none());
    assert!(d.shortcut().is_none());
    assert!(d.category().is_none());
    assert!(Rc::ptr_eq(&d.keywords(), &d.keywords()));
    c.register(Some(d.clone()));
    let count = Rc::new(Cell::new(0));
    let out = count.clone();
    c.command_state_changed()
        .add(Rc::new(move |_| out.set(out.get() + 1)));
    command.notify_can_execute_changed();
    d.keywords().borrow_mut().push(None);
    d.set_command(Some(command));
    assert_eq!(count.get(), 0);
    d.set_command(None);
    assert_eq!(count.get(), 1);
    assert!(c.try_resolve(Some(&s(""))).is_none());
}
#[test]
fn event_failure_preserves_mutation_and_invalid_null_inputs_fail() {
    let c = RibbonCommandCatalog::new();
    c.commands_changed().add(Rc::new(|_| panic!("handler")));
    assert!(catch_unwind(AssertUnwindSafe(|| simple(&c, "a", "A"))).is_err());
    assert!(c.find(Some(&s("a"))).is_some());
    assert!(catch_unwind(AssertUnwindSafe(|| c.unregister(Some(&s("a"))))).is_err());
    assert!(c.find(Some(&s("a"))).is_none());
    assert!(catch_unwind(AssertUnwindSafe(|| c.unregister(None))).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| c.register(None))).is_err());
    assert!(
        catch_unwind(AssertUnwindSafe(|| RibbonCommandDescriptor::new(
            None, None, None
        )))
        .is_err()
    );
}
#[test]
fn known_gc_rooting_gap_is_explicit_and_native_subscriptions_do_not_form_cycles() {
    assert_eq!(source()["catalogRootedByExternalDescriptor"], true);
    let c = RibbonCommandCatalog::new();
    let weak = Rc::downgrade(&c);
    let d = simple(&c, "held", "Held");
    assert_eq!(d.property_changed().handler_count(), 1);
    drop(c);
    // Deliberate limitation regression, NOT successful source lifetime parity.
    assert!(weak.upgrade().is_none());
    assert_eq!(d.property_changed().handler_count(), 0);
}
