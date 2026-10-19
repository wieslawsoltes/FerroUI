//! `WhenPropertyChangedFixture.IntegrationTests` (`DynamicData.Tests/Binding/WhenPropertyChangedFixture.IntegrationTests.DeepChain.cs`).

use super::when_property_changed_fixture::{ChildModel, ParentModel};
use ferroui_dynamic_data::binding::{NotifyPropertyChangedEx, PropertyChain, PropertyValue};
use ferroui_reactive::ObservableExt;
use ferroui_system::prop;
use std::cell::RefCell;
use std::rc::Rc;

/// Ported on one thread: upstream's observer blocks inside its `OnNext` for the initial leaf
/// value while a second thread mutates the leaf; here the observer mutates the leaf while it
/// handles the initial value, a change during the initial emission all the same.
#[test]
fn deep_chain_concurrent_leaf_mutation_during_initial_emit_not_dropped() {
    // Deep-chain version of the above. The observer blocks inside its OnNext for the initial
    // leaf value while a second thread mutates the leaf.
    let parent = ParentModel::new(Some(ChildModel::new(10)));

    let emissions = Rc::new(RefCell::new(Vec::new()));
    let (sink, target) = (emissions.clone(), parent.clone());
    let observer = move |pv: PropertyValue<Rc<ParentModel>, i32>| {
        sink.borrow_mut().push(pv.value().copied().unwrap_or_default());

        // whenSubscribing.Set(): the other thread changes the leaf once.
        if sink.borrow().len() == 1 {
            target.child().unwrap().set_age(20);
        }
    };

    // p => p.Child!.Age
    let source = parent.when_property_changed_with(PropertyChain::nullable(prop!(ParentModel => child)).then(prop!(ChildModel => age)), true);

    let subscription = source.subscribe_next(observer);
    subscription.dispose();

    assert_eq!(*emissions.borrow(), vec![10, 20]);
}

// Not ported: DeepChain_ConcurrentParentSwap_LeafEventOnWinnerNotDropped - real threads

// Not ported: DeepChain_FiveLevels_AllLevelsMutatedConcurrently_FinalEmissionMatchesActual - real threads
