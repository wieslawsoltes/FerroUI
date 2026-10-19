//! `WhenPropertyChangedFixture.UnitTests` (`DynamicData.Tests/Binding/WhenPropertyChangedFixture.UnitTests.DeepChain.cs`):
//! the observation of a path through children that may be null.

use super::when_property_changed_fixture::{ChildModel, Level1, Level2, Level3, Level4, ParentModel};
use ferroui_dynamic_data::binding::{NotifyPropertyChangedEx, PropertyChain, PropertyValue};
use ferroui_reactive::ObservableExt;
use ferroui_system::prop;
use std::cell::RefCell;
use std::rc::Rc;

/// `p => p.Child!.Age`.
fn child_age() -> PropertyChain<Rc<ParentModel>, i32> {
    PropertyChain::nullable(prop!(ParentModel => child)).then(prop!(ChildModel => age))
}

// Not ported: DeepChain_InitialGetterThrows_DefaultErrorHandler_DetachesEveryHandler - a getter cannot fail

// Not ported: DeepChain_InitialGetterThrows_ErrorIsRecordedAndEveryHandlerDetached - a getter cannot fail

// Not ported: DeepChain_InitialObserverThrows_DetachesEveryHandler - an observer cannot throw (a panic unwinds through the subscription; it is not an exception that Subscribe rethrows after detaching)

#[test]
fn deep_chain_mid_chain_swap_deeper_levels_retarget_correctly() {
    // Mid-chain swap on a 4-level chain. When level 3 is reassigned, the leaf subscription
    // must re-attach against the new subtree; events on the old subtree must be ignored
    // (its notifier subscription was disposed).
    let l1 = Level1::new(Some(Level2::new(Some(Level3::new(Some(Level4::new(10)))))));

    let emissions = Rc::new(RefCell::new(Vec::new()));
    let sink = emissions.clone();
    // x => x.Child!.Child!.Child!.Leaf
    let path = PropertyChain::nullable(prop!(Level1 => child))
        .then_nullable(prop!(Level2 => child))
        .then_nullable(prop!(Level3 => child))
        .then(prop!(Level4 => leaf));
    let sub = l1
        .when_property_changed_with(path, true)
        .subscribe_next(move |pv: PropertyValue<Rc<Level1>, i32>| sink.borrow_mut().push(pv.value().copied().unwrap_or_default()));

    assert_eq!(*emissions.borrow(), vec![10], "initial emission");

    let original_leaf = l1.child().unwrap().child().unwrap().child().unwrap();

    let new_l4 = Level4::new(20);
    l1.child().unwrap().child().unwrap().set_child(Some(new_l4.clone()));

    assert_eq!(*emissions.borrow(), vec![10, 20], "mid-chain swap emits the new leaf value");

    new_l4.set_leaf(30);
    assert_eq!(*emissions.borrow(), vec![10, 20, 30], "leaf event on new subtree is captured");

    original_leaf.set_leaf(999);
    assert_eq!(*emissions.borrow(), vec![10, 20, 30], "leaf event on detached subtree is ignored");

    sub.dispose();
}

#[test]
fn deep_chain_notify_initial_false_does_not_dedup_same_valued_events() {
    let parent = ParentModel::new(Some(ChildModel::new(1)));
    let emissions = Rc::new(RefCell::new(Vec::new()));

    let sink = emissions.clone();
    let sub = parent
        .when_property_changed_with(child_age(), false)
        .subscribe_next(move |pv: PropertyValue<Rc<ParentModel>, i32>| sink.borrow_mut().push(pv.value().copied().unwrap_or_default()));

    parent.child().unwrap().set_age(7);
    parent.child().unwrap().set_age(7);

    assert_eq!(*emissions.borrow(), vec![7, 7]);
    sub.dispose();
}

#[test]
fn deep_chain_post_swap_leaf_event_on_new_child_captured() {
    // After parent.Child is reassigned, the leaf-level subscription must be re-attached
    // against the new child. A subsequent leaf mutation on the new child must be captured.
    let parent = ParentModel::new(Some(ChildModel::new(10)));
    let emissions = Rc::new(RefCell::new(Vec::new()));

    let sink = emissions.clone();
    let sub = parent
        .when_property_changed_with(child_age(), true)
        .subscribe_next(move |pv: PropertyValue<Rc<ParentModel>, i32>| sink.borrow_mut().push(pv.value().copied().unwrap_or_default()));

    let new_child = ChildModel::new(20);
    parent.set_child(Some(new_child.clone()));
    new_child.set_age(30);

    assert_eq!(*emissions.borrow(), vec![10, 20, 30]);
    sub.dispose();
}

#[test]
fn deep_chain_notify_initial_true_does_not_dedup_same_valued_events() {
    let parent = ParentModel::new(Some(ChildModel::new(1)));
    let emissions = Rc::new(RefCell::new(Vec::new()));

    let sink = emissions.clone();
    let sub = parent
        .when_property_changed_with(child_age(), true)
        .subscribe_next(move |pv: PropertyValue<Rc<ParentModel>, i32>| sink.borrow_mut().push(pv.value().copied().unwrap_or_default()));

    parent.child().unwrap().set_age(1);
    parent.child().unwrap().set_age(1);
    parent.child().unwrap().set_age(1);

    assert_eq!(*emissions.borrow(), vec![1, 1, 1, 1]);
    sub.dispose();
}
