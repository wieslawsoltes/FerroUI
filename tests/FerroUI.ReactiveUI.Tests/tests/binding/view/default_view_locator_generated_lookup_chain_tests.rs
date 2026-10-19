//! `DefaultViewLocatorGeneratedLookupChainTests` (ReactiveUI.Binding.Tests,
//! `View/DefaultViewLocatorGeneratedLookupChainTests.cs`).

use super::{dispatch, initialize, is_same_view, is_view};
use ferroui_reactiveui::*;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Default)]
struct FirstViewModel;

#[derive(Default)]
struct SecondViewModel;

/// A view that names no type of view model.
#[derive(Default)]
struct FirstView {
    view_model: RefCell<Option<Rc<dyn Any>>>,
}

impl IActivatableView for FirstView {}

impl IViewFor for FirstView {
    fn view_model(&self) -> Option<Rc<dyn Any>> {
        self.view_model.borrow().clone()
    }

    fn set_view_model(&self, view_model: Option<Rc<dyn Any>>) {
        *self.view_model.borrow_mut() = view_model;
    }
}

/// A view that names no type of view model.
#[derive(Default)]
struct SecondView {
    view_model: RefCell<Option<Rc<dyn Any>>>,
}

impl IActivatableView for SecondView {}

impl IViewFor for SecondView {
    fn view_model(&self) -> Option<Rc<dyn Any>> {
        self.view_model.borrow().clone()
    }

    fn set_view_model(&self, view_model: Option<Rc<dyn Any>>) {
        *self.view_model.borrow_mut() = view_model;
    }
}

/// The two lookups of `TwoLookups_*`: one view per view model type.
fn register_two_lookups() {
    DefaultViewLocator::set_generated_view_dispatch(dispatch(|view_model, _| {
        view_model.is::<FirstViewModel>().then(|| Rc::new(FirstView::default()) as Rc<dyn IViewFor>)
    }));
    DefaultViewLocator::set_generated_view_dispatch(dispatch(|view_model, _| {
        view_model.is::<SecondViewModel>().then(|| Rc::new(SecondView::default()) as Rc<dyn IViewFor>)
    }));
}

/// A lookup that records `name` in `log` and finds nothing.
fn recording_lookup(log: &Rc<RefCell<Vec<String>>>, name: &'static str) -> GeneratedViewDispatch {
    let log = log.clone();
    dispatch(move |_, _| {
        log.borrow_mut().push(name.to_owned());
        None
    })
}

#[test]
fn two_lookups_each_resolves_its_own_view_models() {
    initialize();
    register_two_lookups();
    let locator = DefaultViewLocator::new();

    let first = locator.resolve_view_of(&Rc::new(FirstViewModel), None);
    let second = locator.resolve_view_of(&Rc::new(SecondViewModel), None);

    assert!(is_view::<FirstView>(&first));
    assert!(is_view::<SecondView>(&second));
}

#[test]
fn two_lookups_non_generic_resolve_each_resolves_its_own_view_models() {
    initialize();
    register_two_lookups();
    let locator = DefaultViewLocator::new();

    let first = locator.resolve_view(Some(Rc::new(FirstViewModel) as Rc<dyn Any>), None);
    let second = locator.resolve_view(Some(Rc::new(SecondViewModel) as Rc<dyn Any>), None);

    assert!(is_view::<FirstView>(&first));
    assert!(is_view::<SecondView>(&second));
}

#[test]
fn duplicate_view_model_most_recently_registered_lookup_wins() {
    initialize();
    let earlier = Rc::new(FirstView::default());
    let later = Rc::new(FirstView::default());
    let (earlier_returned, later_returned) = (earlier.clone(), later.clone());
    DefaultViewLocator::set_generated_view_dispatch(dispatch(move |_, _| Some(earlier_returned.clone())));
    DefaultViewLocator::set_generated_view_dispatch(dispatch(move |_, _| Some(later_returned.clone())));

    let result = DefaultViewLocator::new().resolve_view_of(&Rc::new(FirstViewModel), None);

    assert!(is_same_view(&result, &later));
    assert!(!is_same_view(&result, &earlier));
}

#[test]
fn duplicate_view_model_latest_lookup_resolves_null_earlier_lookup_serves() {
    initialize();
    let earlier = Rc::new(FirstView::default());
    let earlier_returned = earlier.clone();
    DefaultViewLocator::set_generated_view_dispatch(dispatch(move |_, _| Some(earlier_returned.clone())));
    DefaultViewLocator::set_generated_view_dispatch(dispatch(|_, _| None));

    let result = DefaultViewLocator::new().resolve_view_of(&Rc::new(FirstViewModel), None);

    assert!(is_same_view(&result, &earlier));
}

#[test]
fn lookups_receive_the_normalized_contract() {
    initialize();
    let contracts = Rc::new(RefCell::new(Vec::<String>::new()));
    for _ in 0..2 {
        let contracts = contracts.clone();
        DefaultViewLocator::set_generated_view_dispatch(dispatch(move |_, contract| {
            contracts.borrow_mut().push(contract.to_owned());
            None
        }));
    }

    let _ = DefaultViewLocator::new().resolve_view_of(&Rc::new(FirstViewModel), None);

    assert_eq!(contracts.borrow().join("|"), "|");
}

#[test]
fn lookups_are_consulted_most_recent_first() {
    initialize();
    let order = Rc::new(RefCell::new(Vec::new()));
    DefaultViewLocator::set_generated_view_dispatch(recording_lookup(&order, "first"));
    DefaultViewLocator::set_generated_view_dispatch(recording_lookup(&order, "second"));
    DefaultViewLocator::set_generated_view_dispatch(recording_lookup(&order, "third"));

    let _ = DefaultViewLocator::new().resolve_view_of(&Rc::new(FirstViewModel), None);

    assert_eq!(order.borrow().join(","), "third,second,first");
}

#[test]
fn registering_the_same_lookup_again_consults_it_once() {
    initialize();
    let calls = Rc::new(Cell::new(0));
    let counted = calls.clone();
    let lookup = dispatch(move |_, _| {
        counted.set(counted.get() + 1);
        None
    });
    DefaultViewLocator::set_generated_view_dispatch(lookup.clone());
    DefaultViewLocator::set_generated_view_dispatch(lookup);

    let _ = DefaultViewLocator::new().resolve_view_of(&Rc::new(FirstViewModel), None);

    assert_eq!(calls.get(), 1);
}

// Not ported: ConcurrentRegistrations_AllTakeEffect - real threads

#[test]
fn reset_removes_every_lookup() {
    initialize();
    register_two_lookups();

    DefaultViewLocator::reset_generated_view_dispatch_for_testing();
    let locator = DefaultViewLocator::new();

    assert!(locator.resolve_view_of(&Rc::new(FirstViewModel), None).is_none());
    assert!(locator.resolve_view_of(&Rc::new(SecondViewModel), None).is_none());
}
