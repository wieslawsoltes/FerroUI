//! `WhenPropertyChangedFixture.IntegrationTests` (`DynamicData.Tests/Binding/WhenPropertyChangedFixture.IntegrationTests.Shallow.cs`).

use super::when_property_changed_fixture::Item;
use ferroui_dynamic_data::binding::{NotifyPropertyChangedEx, PropertyValue};
use ferroui_reactive::ObservableExt;
use ferroui_system::prop;
use std::cell::RefCell;
use std::rc::Rc;

/// Ported on one thread: upstream's second thread changes the value while the observer handles
/// the initial one; here the observer changes it while it handles the initial value, a change
/// during the initial emission all the same.
#[test]
fn shallow_concurrent_mutation_during_initial_emit_not_dropped() {
    let item = Item::new(1, 10);

    let source = item.when_property_changed_with(prop!(Item => value), true);

    let observed_values = Rc::new(RefCell::new(Vec::new()));
    let (sink, target) = (observed_values.clone(), item.clone());
    let subscription = source.subscribe_next(move |property_value: PropertyValue<Rc<Item>, i32>| {
        sink.borrow_mut().push(property_value.value().copied().unwrap_or_default());

        // whenSubscribing.Set(): the other thread changes the value once.
        if sink.borrow().len() == 1 {
            target.set_value(20);
        }
    });
    subscription.dispose();

    assert_eq!(
        *observed_values.borrow(),
        vec![10, 20],
        "All change events occurring after publication of the initial value should be captured and forwarded."
    );
}
