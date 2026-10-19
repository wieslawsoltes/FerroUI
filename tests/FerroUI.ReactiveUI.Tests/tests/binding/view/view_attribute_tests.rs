//! `ViewAttributeTests` (ReactiveUI.Binding.Tests, `View/ViewAttributeTests.cs`).
//!
//! The attributes of a view class are the options of a view registration
//! (`ViewRegistrationOptions`); the last tests check them as [`ViewRegistrations`] applies them,
//! where upstream's source generator reads the attributes.

use super::{initialize, is_view};
use ferroui_reactiveui::*;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct TestViewModel;

#[derive(Default)]
struct TestView {
    view_model: RefCell<Option<Rc<TestViewModel>>>,
}
view_for!(TestView, TestViewModel, view_model);

fn object() -> Option<Rc<dyn Any>> {
    Some(Rc::new(TestViewModel))
}

#[test]
fn view_contract_attribute_stores_contract() {
    let attr = ViewContract::new("compact");

    assert_eq!(attr.contract(), "compact");
}

#[test]
fn exclude_from_view_registration_attribute_can_instantiate() {
    let attr = ExcludeFromViewRegistration;

    assert_eq!(attr, ExcludeFromViewRegistration);
}

#[test]
fn single_instance_view_attribute_can_instantiate() {
    let attr = SingleInstanceView;

    assert_eq!(attr, SingleInstanceView);
}

#[test]
fn view_registration_contract_registers_the_view_under_the_contract() {
    initialize();
    ViewRegistrations::new()
        .register::<TestViewModel, TestView>(ViewRegistrationOptions::new().with_contract(ViewContract::new("compact")))
        .apply();
    let locator = DefaultViewLocator::new();

    assert!(is_view::<TestView>(&locator.resolve_view(object(), Some("compact"))));
    assert!(locator.resolve_view(object(), None).is_none());
}

#[test]
fn view_registration_single_instance_reuses_the_view() {
    initialize();
    ViewRegistrations::new().register::<TestViewModel, TestView>(ViewRegistrationOptions::new().with_single_instance()).apply();
    let locator = DefaultViewLocator::new();

    let first = locator.resolve_view(object(), None).expect("the view is registered");
    let second = locator.resolve_view(object(), None).expect("the view is registered");

    assert!(Rc::ptr_eq(&first, &second));
}

#[test]
fn view_registration_without_single_instance_creates_a_view_each_time() {
    initialize();
    ViewRegistrations::new().register::<TestViewModel, TestView>(ViewRegistrationOptions::new()).apply();
    let locator = DefaultViewLocator::new();

    let first = locator.resolve_view(object(), None).expect("the view is registered");
    let second = locator.resolve_view(object(), None).expect("the view is registered");

    assert!(!Rc::ptr_eq(&first, &second));
}

#[test]
fn view_registration_exclude_leaves_the_view_out() {
    initialize();
    ViewRegistrations::new().register::<TestViewModel, TestView>(ViewRegistrationOptions::new().with_exclude()).apply();

    assert!(DefaultViewLocator::new().resolve_view(object(), None).is_none());
}
