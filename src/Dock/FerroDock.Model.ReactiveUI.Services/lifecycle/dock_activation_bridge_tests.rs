use super::*;
use crate::test_environment::Environment;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{
    reactive_object, IActivatableView, ReactiveObject, ViewModelActivationMixins,
    ViewModelActivator,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
struct Model {
    reactive: ReactiveObject,
    activator: ViewModelActivator,
}
reactive_object!(Model);
impl IActivatableViewModel for Model {
    fn activator(&self) -> &ViewModelActivator {
        &self.activator
    }
}
impl Model {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            reactive: ReactiveObject::new(),
            activator: ViewModelActivator::new(),
        })
    }
}
struct UninspectedView;
impl IActivatableView for UninspectedView {}
impl IViewFor for UninspectedView {
    fn view_model(&self) -> Option<Rc<dyn Any>> {
        panic!("source bridge must not inspect the view")
    }
    fn set_view_model(&self, _: Option<Rc<dyn Any>>) {
        panic!("source bridge must not mutate the view")
    }
}
#[test]
fn actual_activator_scopes_share_activation_and_release_resources_only_after_last_disposal() {
    let _env = Environment::new();
    let model = Model::new();
    let view = Rc::new(UninspectedView);
    let bridge: Rc<dyn IActivationBridge> = DockActivationBridge::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    let seen = log.clone();
    model.when_activated(move || {
        seen.borrow_mut().push("setup");
        let seen = seen.clone();
        vec![Disposable::create(move || {
            seen.borrow_mut().push("cleanup")
        })]
    });
    let seen = log.clone();
    let _active = model
        .activator
        .activated()
        .subscribe_next(move |_| seen.borrow_mut().push("activated"));
    let seen = log.clone();
    let _inactive = model
        .activator
        .deactivated()
        .subscribe_next(move |_| seen.borrow_mut().push("deactivated"));
    let a = bridge.track(model.clone(), view.clone());
    let b = bridge.track(model.clone(), view.clone());
    assert_eq!(&*log.borrow(), &["setup", "activated"]);
    a.dispose();
    a.dispose();
    assert_eq!(log.borrow().len(), 2);
    b.dispose();
    assert_eq!(
        &*log.borrow(),
        &["setup", "activated", "cleanup", "deactivated"]
    );
    let c = bridge.track(model.clone(), view);
    c.dispose();
    assert_eq!(
        &*log.borrow(),
        &[
            "setup",
            "activated",
            "cleanup",
            "deactivated",
            "setup",
            "activated",
            "cleanup",
            "deactivated"
        ]
    );
}
#[test]
fn ignored_activation_handle_keeps_model_active_and_view_is_not_retained() {
    let _env = Environment::new();
    let model = Model::new();
    let view = Rc::new(UninspectedView);
    let weak = Rc::downgrade(&view);
    let active = Rc::new(Cell::new(false));
    let seen = active.clone();
    model.when_activated(move || {
        seen.set(true);
        let seen = seen.clone();
        vec![Disposable::create(move || seen.set(false))]
    });
    DockActivationBridge::new().track(model.clone(), view.clone());
    drop(view);
    assert!(weak.upgrade().is_none());
    assert!(active.get());
    model.activator.deactivate();
    assert!(!active.get());
}
#[test]
fn activation_failure_propagates_and_distinct_models_do_not_share_activation_state() {
    let _env = Environment::new();
    let first = Model::new();
    let second = Model::new();
    let view = Rc::new(UninspectedView);
    let bridge = DockActivationBridge::new();
    first.when_activated(|| panic!("activation failed"));
    let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        bridge.track(first, view.clone())
    }))
    .err()
    .expect("activation failure");
    assert_eq!(
        error.downcast_ref::<&str>().copied(),
        Some("activation failed")
    );
    let active = Rc::new(Cell::new(false));
    let seen = active.clone();
    let _subscription = second
        .activator
        .activated()
        .subscribe_next(move |_| seen.set(true));
    let scope = bridge.track(second, view);
    assert!(active.get());
    scope.dispose();
}
