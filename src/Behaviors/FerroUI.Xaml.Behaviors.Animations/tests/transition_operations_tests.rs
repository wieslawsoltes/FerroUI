//! Ported from `tests/Xaml.Behaviors.Animations.UnitTests/TransitionOperationsTests.cs`.

use super::support::start_app;
use crate::TransitionOperations;
use ferroui_base::animation::{DoubleTransition, TransitionBase, Transitions};
use ferroui_base::layout::Layoutable;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{StyledElement, Visual};
use ferroui_controls::Border;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn add_remove_and_clear_manage_transitions() {
    let _app = start_app();
    let target = Border::new();
    let element: &StyledElement = &target;
    let first = DoubleTransition::new();
    first.set_property(Some(Visual::opacity_property().as_property()));
    let second = DoubleTransition::new();
    second.set_property(Some(Layoutable::width_property().as_property()));

    assert!(TransitionOperations::add(Some(element), Some(first.upcast_ref::<TransitionBase>())));
    assert!(TransitionOperations::add(Some(element), Some(second.upcast_ref::<TransitionBase>())));
    assert_eq!(target.transitions().map(|transitions| transitions.count()), Some(2));
    assert!(TransitionOperations::remove(Some(element), Some(first.upcast_ref::<TransitionBase>())));
    assert_eq!(target.transitions().expect("transitions").count(), 1);
    assert!(TransitionOperations::clear(Some(element)));
    assert_eq!(target.transitions().expect("transitions").count(), 0);
}

#[test]
fn replace_returns_previous_collection() {
    let _app = start_app();
    let target = Border::new();
    let previous = Transitions::new();
    let replacement = Transitions::new();
    target.set_transitions(Some(previous.clone()));

    let result = TransitionOperations::replace(&target, Some(replacement.clone()));

    assert!(result.expect("the previous collection").ptr_eq(&previous));
    assert!(target.transitions().expect("the replacement").ptr_eq(&replacement));
}

#[test]
fn operations_return_false_for_missing_inputs() {
    let _app = start_app();
    assert!(!TransitionOperations::add(None, None));
    assert!(!TransitionOperations::remove(None, None));
    assert!(!TransitionOperations::clear(None));
}

#[test]
fn observe_reports_current_and_replacement_collections_until_disposed() {
    let _app = start_app();
    let target = Border::new();
    let replacement = Transitions::new();
    let after_disposal = Transitions::new();
    let observed: Rc<RefCell<Vec<Option<Transitions>>>> = Rc::new(RefCell::new(Vec::new()));

    {
        let observed = observed.clone();
        let subscription =
            TransitionOperations::observe(&target, move |transitions| observed.borrow_mut().push(transitions));
        target.set_transitions(Some(replacement.clone()));
        subscription.dispose();
    }

    target.set_transitions(Some(after_disposal));

    let observed = observed.borrow();
    assert_eq!(observed.len(), 2);
    assert!(observed[0].is_none());
    assert!(observed[1].as_ref().expect("the replacement").ptr_eq(&replacement));
}

// Not ported: Observe_RejectsMissingArguments - argument-null validation, which Rust's types make impossible
