//! Exercises the actual C# foreach/finally oracle, including reentrant list mutation.
use super::*;
use serde_json::{Value, json};
type Action = RefreshAction;
fn mark(trace: &RefCell<Vec<String>>, context: &SettingsPanelContext, name: &str) {
    trace.borrow_mut().push(format!(
        "{name}:{}",
        if context.is_refreshing() {
            "True"
        } else {
            "False"
        }
    ));
}
#[test]
fn ordered_versioned_refresh_and_boolean_finally_match_unchanged_source() {
    for line in include_str!("../../tests/fixtures/settings-panel-refresh.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let scenario = row["scenario"].as_str().unwrap().to_owned();
        let refreshers = Rc::new(SettingsRefreshers::new());
        let context = Rc::new(SettingsPanelContext::detached());
        let trace = Rc::new(RefCell::new(Vec::new()));
        let first_slot = Rc::new(RefCell::<Option<std::rc::Weak<RefreshCallback>>>::new(None));
        let depth = Cell::new(0);
        let first: Action = {
            let refreshers = Rc::downgrade(&refreshers);
            let context = context.clone();
            let trace = trace.clone();
            let first_slot = first_slot.clone();
            let scenario = scenario.clone();
            Rc::new(move || {
                let refreshers = refreshers.upgrade().unwrap();
                mark(&trace, &context, "first");
                match scenario.as_str() {
                    "clear-mid" => refreshers.clear(),
                    "append-last" => {
                        let trace = trace.clone();
                        let context = context.clone();
                        refreshers.add(Rc::new(move || {
                            mark(&trace, &context, "added");
                            Ok(())
                        }));
                    }
                    "replace-identical" => {
                        // Reflection changes an existing List slot in the source harness.
                        refreshers.actions.borrow_mut()[0] =
                            first_slot.borrow().as_ref().unwrap().upgrade().unwrap();
                        refreshers
                            .version
                            .set(refreshers.version.get().wrapping_add(1));
                    }
                    "throw" => return Err(FieldError::WrongValue),
                    "modify-then-throw" => {
                        refreshers.clear();
                        return Err(FieldError::WrongValue);
                    }
                    "nested" => {
                        let previous = depth.get();
                        depth.set(previous + 1);
                        if previous == 0 {
                            refreshers.refresh(&context)?;
                            mark(&trace, &context, "after-nested");
                        }
                    }
                    "nested-throw-caught" => {
                        let previous = depth.get();
                        depth.set(previous + 1);
                        if previous == 0 {
                            assert_eq!(refreshers.refresh(&context), Err(FieldError::WrongValue));
                            mark(&trace, &context, "caught");
                        } else {
                            return Err(FieldError::WrongValue);
                        }
                    }
                    "empty" | "order" => (),
                    _ => panic!("unknown source scenario {scenario}"),
                }
                Ok(())
            })
        };
        *first_slot.borrow_mut() = Some(Rc::downgrade(&first));
        if scenario != "empty" {
            refreshers.add(first);
        }
        if [
            "order",
            "clear-mid",
            "throw",
            "modify-then-throw",
            "nested",
            "nested-throw-caught",
        ]
        .contains(&scenario.as_str())
        {
            let trace = trace.clone();
            let context = context.clone();
            refreshers.add(Rc::new(move || {
                mark(&trace, &context, "second");
                Ok(())
            }));
        }
        let error = match refreshers.refresh(&context) {
            Ok(()) => None,
            Err(FieldError::ModifiedRefreshers) => Some("InvalidOperationException"),
            // The arbitrary injected source callback throws ArgumentException. Its Rust
            // counterpart deliberately returns an error; both must beat a version failure.
            Err(FieldError::WrongValue) => Some("ArgumentException"),
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(json!(error), row["error"], "{scenario}");
        assert_eq!(json!(*trace.borrow()), row["trace"], "{scenario}");
        assert_eq!(
            json!(context.is_refreshing()),
            row["refreshing"],
            "{scenario}"
        );
        assert_eq!(json!(refreshers.len()), row["count"], "{scenario}");
    }
}
#[test]
fn unwinding_callback_releases_refresh_flag_and_does_not_poison_actions() {
    let refreshers = SettingsRefreshers::new();
    let context = SettingsPanelContext::detached();
    refreshers.add(Rc::new(|| panic!("injected callback unwind")));
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| refreshers.refresh(&context)))
            .is_err()
    );
    assert!(!context.is_refreshing());
    refreshers.clear();
    assert_eq!(refreshers.refresh(&context), Ok(()));
}
#[test]
fn detached_scalar_callbacks_preserve_guard_and_index_before_target_order() {
    let context = Rc::new(SettingsPanelContext::detached());
    let boolean =
        SettingsFieldViewModel::new(context.clone(), FieldKind::Boolean(BooleanField::NumberRow));
    assert_eq!(
        boolean.dispatch(FieldValue::Bool(true)),
        Err(FieldError::MissingTarget)
    );
    for choice in EnumChoice::ALL {
        let field = SettingsFieldViewModel::new(context.clone(), FieldKind::Enum(choice));
        assert_eq!(field.dispatch(FieldValue::Index(-1)), Ok(()));
        assert_eq!(
            field.dispatch(FieldValue::Index(0)),
            Err(FieldError::MissingTarget)
        );
        assert_eq!(
            field.dispatch(FieldValue::Index(i32::MAX)),
            Err(FieldError::Index {
                index: i32::MAX,
                count: choice.items().len()
            })
        );
        let _guard = context.begin_refresh();
        assert_eq!(field.dispatch(FieldValue::Index(i32::MAX)), Ok(()));
        assert_eq!(boolean.dispatch(FieldValue::Bool(true)), Ok(()));
    }
}
