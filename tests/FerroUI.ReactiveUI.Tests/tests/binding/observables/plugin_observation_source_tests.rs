//! `PluginObservationSourceTests`
//! (`ReactiveUI.Binding.Tests/Observables/PluginObservationSourceTests.cs`).
//!
//! The locator of Splat is per thread and each test runs on a thread of its own, so the
//! registration of a test does not reach another; the test still clears the registrations it
//! made, as upstream restores them. The member expression has no counterpart: a step is named by
//! its property name. Upstream checks the type of the chosen observation; the port checks that it
//! is not the generated one and that it reads the value through the getter.

use crate::tests::binding::test_models::RecordingObserver;
use ferroui_reactive::Observable;
use ferroui_reactiveui::*;
use ferroui_splat::AppLocator;
use ferroui_system::component_model::PropertyOwner;
use ferroui_system::{IObservable, IObserver};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

const OBSERVED_PROPERTY_NAME: &str = "Name";
const INITIAL_NAME: &str = "initial";
const UNBEATABLE_AFFINITY: i32 = i32::MAX;
const BEATABLE_AFFINITY: i32 = 1;

fn owner(fixture: &Rc<ObservedFixture>) -> PropertyOwner {
    PropertyOwner { object: Some(fixture.clone() as Rc<dyn Any>), notify: None }
}

#[test]
fn choose_with_no_registration_above_the_generated_affinity_keeps_the_generated_observation() {
    let fixture = Rc::new(ObservedFixture { name: RefCell::new(INITIAL_NAME.to_string()) });
    let generated: Rc<dyn IObservable<String>> = Rc::new(UnchangingPropertyObservable::new(INITIAL_NAME.to_string()));

    let reader = fixture.clone();
    let chosen = PluginObservationSource::choose(
        &owner(&fixture),
        OBSERVED_PROPERTY_NAME,
        false,
        UNBEATABLE_AFFINITY,
        move || Some(reader.name()),
        generated.clone(),
    );

    assert!(Rc::ptr_eq(&chosen, &generated));
}

#[test]
fn choose_with_a_registration_above_the_generated_affinity_observes_through_the_registration() {
    AppLocator::unregister_all::<dyn ICreatesObservableForProperty>();
    AppLocator::register_constant::<dyn ICreatesObservableForProperty>(Rc::new(WinningPlugin) as Rc<dyn ICreatesObservableForProperty>);

    let fixture = Rc::new(ObservedFixture { name: RefCell::new(INITIAL_NAME.to_string()) });
    let generated: Rc<dyn IObservable<String>> = Rc::new(UnchangingPropertyObservable::new(INITIAL_NAME.to_string()));
    let reader = fixture.clone();
    let chosen = PluginObservationSource::choose(
        &owner(&fixture),
        OBSERVED_PROPERTY_NAME,
        false,
        BEATABLE_AFFINITY,
        move || Some(reader.name()),
        generated.clone(),
    );

    assert!(!Rc::ptr_eq(&chosen, &generated));
    let observer = RecordingObserver::<String>::new();
    let subscription = chosen.subscribe(observer.clone() as Rc<dyn IObserver<String>>);
    assert_eq!(observer.values(), vec![INITIAL_NAME.to_string()]);
    subscription.dispose();

    AppLocator::unregister_all::<dyn ICreatesObservableForProperty>();
}

/// An object with a name and no change notification (`ObservedFixture`).
struct ObservedFixture {
    name: RefCell<String>,
}

impl ObservedFixture {
    fn name(&self) -> String {
        self.name.borrow().clone()
    }
}

/// A property observer with an affinity above the generated one, whose notifications never come
/// (`WinningPlugin`).
struct WinningPlugin;

impl WinningPlugin {
    const WINNING_AFFINITY: i32 = 50;
}

impl ICreatesObservableForProperty for WinningPlugin {
    fn get_affinity_for_object(&self, _owner: &PropertyOwner, _property_name: &str, _before_changed: bool) -> i32 {
        Self::WINNING_AFFINITY
    }

    fn get_notification_for_property(
        &self,
        _owner: &PropertyOwner,
        _property_name: &str,
        _before_changed: bool,
        _suppress_warnings: bool,
    ) -> Rc<dyn IObservable<ObservedChange<PropertyOwner, ()>>> {
        Observable::never()
    }
}
