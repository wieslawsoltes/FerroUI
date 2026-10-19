//! `ViewModelRootedUnsafeBindingTests` (`ReactiveUI.Binding.Tests/Mixins/ViewModelRootedUnsafeBindingTests.cs`).
//!
//! A view typed for its view model binds through its `ViewModel`, as upstream's run-time fallback
//! and generated code do; the `Unsafe` calls are the bindings themselves.

use crate::tests::binding::test_models::DispatchStubViewModel;
use ferroui_reactive::subjects::Subject;
use ferroui_reactiveui::*;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use ferroui_system::{prop, IObservable, IObserver};
use std::cell::RefCell;
use std::rc::Rc;

const FIRST_VALUE: &str = "first";

const SECOND_VALUE: &str = "second";

const EDITED_VALUE: &str = "edited";

/// A view typed for [`DispatchStubViewModel`] that raises `PropertyChanged` for `ViewModel` and
/// `Caption` (`TypedStubView`).
pub struct TypedStubView {
    view_model: RefCell<Option<Rc<DispatchStubViewModel>>>,
    caption: RefCell<String>,
    property_changed: Event<str>,
}

impl TypedStubView {
    const INITIAL_CAPTION: &'static str = "initial";

    fn new() -> Rc<Self> {
        Rc::new(Self { view_model: RefCell::new(None), caption: RefCell::new(Self::INITIAL_CAPTION.to_string()), property_changed: Event::new() })
    }

    pub fn view_model(&self) -> Option<Rc<DispatchStubViewModel>> {
        self.view_model.borrow().clone()
    }

    pub fn set_view_model(&self, view_model: Option<Rc<DispatchStubViewModel>>) {
        *self.view_model.borrow_mut() = view_model;
        self.property_changed.raise("ViewModel");
    }

    pub fn caption(&self) -> String {
        self.caption.borrow().clone()
    }

    pub fn set_caption(&self, value: String) {
        *self.caption.borrow_mut() = value;
        self.property_changed.raise("Caption");
    }
}

impl INotifyPropertyChanged for TypedStubView {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

view_for!(TypedStubView, DispatchStubViewModel);

fn view_model(caption: &str) -> Rc<DispatchStubViewModel> {
    let view_model = DispatchStubViewModel::new();
    view_model.set_caption(caption.to_string());
    view_model
}

#[test]
fn one_way_bind_unsafe_null_view_model_binds_once_the_view_has_one() {
    let view = TypedStubView::new();

    let binding = view
        .one_way_bind(None::<Rc<DispatchStubViewModel>>, prop!(DispatchStubViewModel => caption), settable_prop!(TypedStubView => caption))
        .expect("no hook refuses the binding");
    let before_view_model = view.caption();

    view.set_view_model(Some(view_model(FIRST_VALUE)));

    assert_eq!(before_view_model, TypedStubView::INITIAL_CAPTION);
    assert_eq!(view.caption(), FIRST_VALUE);
    binding.dispose();
}

#[test]
fn bind_unsafe_view_model_replaced_follows_the_new_view_model() {
    let first = view_model(FIRST_VALUE);
    let second = view_model(SECOND_VALUE);
    let view = TypedStubView::new();
    view.set_view_model(Some(first.clone()));

    let binding = view
        .bind(first.clone(), settable_prop!(DispatchStubViewModel => caption), settable_prop!(TypedStubView => caption))
        .expect("no hook refuses the binding");
    view.set_view_model(Some(second.clone()));
    let after_replacement = view.caption();
    view.set_caption(EDITED_VALUE.to_string());

    assert_eq!(after_replacement, SECOND_VALUE);
    assert_eq!(second.caption(), EDITED_VALUE);
    assert_eq!(first.caption(), FIRST_VALUE);
    binding.dispose();
}

#[test]
fn bind_unsafe_with_signal_writes_to_the_current_view_model() {
    let first = view_model(FIRST_VALUE);
    let second = view_model(SECOND_VALUE);
    let view = TypedStubView::new();
    view.set_view_model(Some(first.clone()));
    let signal: Rc<Subject<bool>> = Subject::new();

    let updates: Rc<dyn IObservable<bool>> = signal.as_observable();
    let binding = view
        .bind_with_trigger(
            first.clone(),
            settable_prop!(DispatchStubViewModel => caption),
            settable_prop!(TypedStubView => caption),
            Some(updates),
            TriggerUpdate::ViewToViewModel,
        )
        .expect("no hook refuses the binding");
    view.set_view_model(Some(second.clone()));
    view.set_caption(EDITED_VALUE.to_string());
    signal.on_next(true);

    assert_eq!(second.caption(), EDITED_VALUE);
    assert_eq!(first.caption(), FIRST_VALUE);
    binding.dispose();
}
