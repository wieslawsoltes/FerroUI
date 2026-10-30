//! Original event argument, multicast and explicit subscription lifetime scenarios.
use super::DockDiagnosticEvents as Diagnostics;
type SelectionHandler = dyn Fn(&Option<Rc<dyn IDockable>>);
use ferrodock_model::core::IDockable;
use ferrodock_model_reactiveui::controls::Document;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[test]
fn selection_event_preserves_original_optional_model_and_duplicate_subscription_order() {
    let event = Diagnostics::select_dockable_requested();
    assert!(Rc::ptr_eq(
        &event,
        &Diagnostics::select_dockable_requested()
    ));
    assert!(!event.has_handlers());
    Diagnostics::raise_select_dockable_requested(None);
    let document: Rc<dyn IDockable> = Document::new();
    let arguments = Rc::new(RefCell::new(Vec::new()));
    let observed = arguments.clone();
    let handler: Rc<SelectionHandler> =
        Rc::new(move |value| observed.borrow_mut().push(value.clone()));
    let first = event.add(handler.clone());
    let second = event.add(handler);
    Diagnostics::raise_select_dockable_requested(Some(document.clone()));
    Diagnostics::raise_select_dockable_requested(None);
    let values = arguments.borrow();
    assert_eq!(values.len(), 4);
    assert!(Rc::ptr_eq(values[0].as_ref().unwrap(), &document));
    assert!(Rc::ptr_eq(values[1].as_ref().unwrap(), &document));
    assert!(values[2].is_none() && values[3].is_none());
    drop(values);
    assert!(event.remove(first));
    assert!(event.remove(second));
    assert!(!event.has_handlers());
}
#[test]
fn mutation_during_raise_affects_the_next_snapshot_and_never_borrows_across_callbacks() {
    let event = Diagnostics::select_dockable_requested();
    assert!(!event.has_handlers());
    let seen = Rc::new(RefCell::new(Vec::new()));
    let tokens = Rc::new(Cell::new(None));
    let added = Rc::new(Cell::new(None));
    let entries = seen.clone();
    let remove = tokens.clone();
    let add = added.clone();
    let first = event.add(Rc::new(move |_| {
        entries.borrow_mut().push("first");
        let event = Diagnostics::select_dockable_requested();
        if let Some(token) = remove.take() {
            event.remove(token);
        }
        if add.get().is_none() {
            let entries = entries.clone();
            add.set(Some(
                event.add(Rc::new(move |_| entries.borrow_mut().push("third"))),
            ));
        }
    }));
    let entries = seen.clone();
    let second = event.add(Rc::new(move |_| entries.borrow_mut().push("second")));
    tokens.set(Some(second));
    Diagnostics::raise_select_dockable_requested(None);
    assert_eq!(&*seen.borrow(), &["first", "second"]);
    Diagnostics::raise_select_dockable_requested(None);
    assert_eq!(&*seen.borrow(), &["first", "second", "first", "third"]);
    event.remove(first);
    event.remove(added.take().unwrap());
    assert!(!event.has_handlers());
}
#[test]
fn subscriptions_retain_captures_until_removal_and_failure_stops_the_invocation_snapshot() {
    let event = Diagnostics::select_dockable_requested();
    assert!(!event.has_handlers());
    let capture = Rc::new(Cell::new(0));
    let weak = Rc::downgrade(&capture);
    let captured = capture.clone();
    let first = event.add(Rc::new(move |_| {
        captured.set(captured.get() + 1);
        panic!("diagnostic subscriber failure")
    }));
    let seen = Rc::new(Cell::new(false));
    let flag = seen.clone();
    let second = event.add(Rc::new(move |_| flag.set(true)));
    drop(capture);
    assert!(weak.upgrade().is_some());
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Diagnostics::raise_select_dockable_requested(None)
    }));
    assert!(result.is_err());
    assert!(!seen.get());
    assert_eq!(weak.upgrade().unwrap().get(), 1);
    event.remove(first);
    assert!(weak.upgrade().is_none());
    event.remove(second);
    assert!(!event.has_handlers());
}
