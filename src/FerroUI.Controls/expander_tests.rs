//! The reference implementation has no unit tests for `Expander`; these
//! tests are specific to this port.

use crate::test_support::test_scope;
use crate::{ExpandDirection, Expander};
use ferroui_base::animation::{CrossFade, IPageTransition};
use ferroui_base::threading::Dispatcher;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn expander_raises_events_and_sets_pseudo_classes() {
    let _scope = test_scope();
    let target = Expander::new();
    assert!(target.classes().contains(":down"));
    assert!(!target.classes().contains(":expanded"));

    let events: Rc<RefCell<Vec<&'static str>>> = Rc::new(RefCell::new(Vec::new()));
    let recorded = events.clone();
    target.expanding(move |_, _| recorded.borrow_mut().push("expanding"));
    let recorded = events.clone();
    target.expanded(move |_, _| recorded.borrow_mut().push("expanded"));
    let recorded = events.clone();
    target.collapsing(move |_, _| recorded.borrow_mut().push("collapsing"));
    let recorded = events.clone();
    target.collapsed(move |_, _| recorded.borrow_mut().push("collapsed"));

    target.set_is_expanded(true);
    assert!(target.classes().contains(":expanded"));
    // Expanded is raised asynchronously.
    assert_eq!(vec!["expanding"], *events.borrow());
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(vec!["expanding", "expanded"], *events.borrow());

    target.set_is_expanded(false);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(!target.classes().contains(":expanded"));
    assert_eq!(vec!["expanding", "expanded", "collapsing", "collapsed"], *events.borrow());

    target.set_expand_direction(ExpandDirection::Left);
    assert!(target.classes().contains(":left"));
    assert!(!target.classes().contains(":down"));
}

#[test]
fn expander_expanding_can_be_canceled() {
    let _scope = test_scope();
    let target = Expander::new();
    target.expanding(|_, e| e.set_cancel(true));

    target.set_is_expanded(true);

    assert!(!target.is_expanded());
    assert!(!target.classes().contains(":expanded"));
}

#[test]
fn expander_canceled_expanding_notifies_reverted_value() {
    let _scope = test_scope();
    let target = Expander::new();
    target.expanding(|_, e| e.set_cancel(true));

    let changes: Rc<RefCell<Vec<(bool, bool)>>> = Rc::new(RefCell::new(Vec::new()));
    let recorded = changes.clone();
    target.property_changed(move |e| {
        if e.property() == Expander::is_expanded_property().as_property() {
            recorded.borrow_mut().push(e.get_old_and_new_value::<bool>());
        }
    });
    let completed: Rc<RefCell<Vec<&'static str>>> = Rc::new(RefCell::new(Vec::new()));
    let recorded = completed.clone();
    target.expanded(move |_, _| recorded.borrow_mut().push("expanded"));
    let recorded = completed.clone();
    target.collapsed(move |_, _| recorded.borrow_mut().push("collapsed"));

    target.set_is_expanded(true);
    Dispatcher::ui_thread().run_jobs(None);

    // Observers are told that the value went back from the requested value
    // to the previous one; the control itself ignores that notification.
    assert_eq!(vec![(true, false)], *changes.borrow());
    assert!(!target.is_expanded());
    assert!(!target.classes().contains(":expanded"));
    assert!(completed.borrow().is_empty());
}

#[test]
fn expander_canceled_collapsing_notifies_reverted_value() {
    let _scope = test_scope();
    let target = Expander::new();
    target.set_is_expanded(true);
    Dispatcher::ui_thread().run_jobs(None);
    target.collapsing(|_, e| e.set_cancel(true));

    let changes: Rc<RefCell<Vec<(bool, bool)>>> = Rc::new(RefCell::new(Vec::new()));
    let recorded = changes.clone();
    target.property_changed(move |e| {
        if e.property() == Expander::is_expanded_property().as_property() {
            recorded.borrow_mut().push(e.get_old_and_new_value::<bool>());
        }
    });

    target.set_is_expanded(false);

    assert_eq!(vec![(false, true)], *changes.borrow());
    assert!(target.is_expanded());
    assert!(target.classes().contains(":expanded"));
}

#[test]
fn expander_content_transition_is_compared_by_identity() {
    let _scope = test_scope();
    let target = Expander::new();
    assert!(target.content_transition().is_none());

    let transition: Rc<dyn IPageTransition> = Rc::new(CrossFade::new());
    let changes = Rc::new(RefCell::new(0));
    let recorded = changes.clone();
    target.property_changed(move |e| {
        if e.property() == Expander::content_transition_property().as_property() {
            *recorded.borrow_mut() += 1;
        }
    });
    let initial = *changes.borrow();

    target.set_content_transition(Some(transition.clone()));
    assert_eq!(initial + 1, *changes.borrow());
    assert!(*target.content_transition().unwrap() == *transition);

    // The same instance again is not a change.
    target.set_content_transition(Some(transition.clone()));
    assert_eq!(initial + 1, *changes.borrow());

    // Another instance of the same type is.
    target.set_content_transition(Some(Rc::new(CrossFade::new())));
    assert_eq!(initial + 2, *changes.borrow());
}
