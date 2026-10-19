//! `WhenPropertyChangedFixture.UnitTests` (`DynamicData.Tests/Binding/WhenPropertyChangedFixture.UnitTests.Subscription.cs`):
//! the release of the handlers at the end of a subscription.

use super::when_property_changed_fixture::{ObservablePrice, OBSERVED_AMOUNT};
use crate::utilities::ObservableExtensions;
use ferroui_dynamic_data::binding::{NotifyPropertyChangedEx, PropertyChain};
use ferroui_reactive::ObservableExt;
use ferroui_system::{prop, IObservable};
use std::rc::Rc;

/// The models and the observed sequence of a case: the chain `price => price.Child!.Child!.Amount`
/// on the root, or `price => price.Amount` on the leaf.
fn arrange(deep_chain: bool) -> (Vec<Rc<ObservablePrice>>, Rc<dyn IObservable<f64>>) {
    let leaf = ObservablePrice::new(OBSERVED_AMOUNT);
    let child = ObservablePrice::with_child(leaf.clone());
    let root = ObservablePrice::with_child(child.clone());
    let models = if deep_chain { vec![root.clone(), child, leaf.clone()] } else { vec![leaf.clone()] };
    let source = if deep_chain {
        root.when_value_changed(
            PropertyChain::nullable(prop!(ObservablePrice => child))
                .then_nullable(prop!(ObservablePrice => child))
                .then(prop!(ObservablePrice => amount)),
        )
    } else {
        leaf.when_value_changed(prop!(ObservablePrice => amount))
    };
    (models, source)
}

fn subscription_explicit_disposal_releases_handlers(deep_chain: bool) {
    // Arrange
    let (models, source) = arrange(deep_chain);
    let (subscription, results) = source.record_values();
    let attached_handler_counts: Vec<usize> = models.iter().map(|model| model.handler_count()).collect();

    // Act
    subscription.dispose();

    // Assert
    assert!(attached_handler_counts.iter().all(|&count| count == 1), "each visited object must stay subscribed after initialization");
    assert!(
        models.iter().all(|model| model.handler_count() == 0),
        "disposing the returned subscription must release every retained handler"
    );
    assert_eq!(results.recorded_values(), vec![OBSERVED_AMOUNT], "initialization must publish the observed value");
    assert!(results.error().is_none(), "explicit disposal is not an observation failure");
    assert!(!results.has_completed(), "unsubscribing does not publish a completion notification");
}

#[test]
fn subscription_explicit_disposal_releases_handlers_case1() {
    subscription_explicit_disposal_releases_handlers(false);
}

#[test]
fn subscription_explicit_disposal_releases_handlers_case2() {
    subscription_explicit_disposal_releases_handlers(true);
}

fn subscription_synchronous_completion_releases_handlers(deep_chain: bool) {
    // Arrange
    let (models, source) = arrange(deep_chain);

    // Act
    let (subscription, results) = source.take(1).record_values();

    // Assert
    assert_eq!(results.recorded_values(), vec![OBSERVED_AMOUNT], "the requested initial value must be delivered");
    assert!(results.error().is_none(), "taking an initial value is normal completion");
    assert!(results.has_completed(), "Take completes after receiving its requested value");
    assert!(models.iter().all(|model| model.was_subscribed()), "handlers must attach before initial delivery");
    assert!(
        models.iter().all(|model| model.handler_count() == 0),
        "synchronous completion must release handlers before Subscribe returns"
    );

    subscription.dispose();
}

#[test]
fn subscription_synchronous_completion_releases_handlers_case1() {
    subscription_synchronous_completion_releases_handlers(false);
}

#[test]
fn subscription_synchronous_completion_releases_handlers_case2() {
    subscription_synchronous_completion_releases_handlers(true);
}
