//! Not from the reference: `ObservableTriggerBehavior` reads the observable of the stream value a
//! binding hands it for a property of type `IObservable<T>` (`Observable="{Binding Values}"`).

use super::support::{run_jobs, start_app};
use crate::ObservableTriggerBehavior;
use ferroui_base::data::core::plugins::ObservableValue;
use ferroui_base::reactive::{IObservable, IObserver, LightweightSubject};
use ferroui_base::BoxedValue;
use ferroui_controls::Border;
use std::rc::Rc;

fn value_of(behavior: &ObservableTriggerBehavior) -> Option<i32> {
    behavior.value().and_then(|value| value.downcast_ref::<i32>().copied())
}

#[test]
fn the_stream_value_of_a_binding_triggers_the_actions() {
    let _app = start_app();
    let subject = Rc::new(LightweightSubject::<i32>::new());
    let observable: Rc<dyn IObservable<i32>> = subject.clone();
    let behavior = ObservableTriggerBehavior::new();
    behavior.set_observable(Some(Rc::new(ObservableValue::new(observable)) as BoxedValue));
    let border = Border::new();
    behavior.attach(&border);

    subject.on_next(3);
    run_jobs();
    assert_eq!(value_of(&behavior), Some(3));

    // The nullable form of the stream value, as a property of the view model may hold it.
    let other = Rc::new(LightweightSubject::<i32>::new());
    let observable: Rc<dyn IObservable<i32>> = other.clone();
    behavior.set_observable(Some(Rc::new(Some(ObservableValue::new(observable))) as BoxedValue));
    subject.on_next(4);
    other.on_next(5);
    run_jobs();
    assert_eq!(value_of(&behavior), Some(5));

    behavior.detach();
}
