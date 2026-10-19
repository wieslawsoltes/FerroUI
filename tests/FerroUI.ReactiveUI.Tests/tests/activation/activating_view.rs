//! `ActivatingView` (`ReactiveUI.Tests/Activation/ActivatingView.cs`).

use super::ActivatingViewModel;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::Subject;
use ferroui_reactiveui::binding::interfaces::{IActivatableView, IViewFor, IViewForOf};
use ferroui_reactiveui::*;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::{IDisposable, IObservable};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A view that counts its activations and is activated by its `Loaded` and `Unloaded` subjects
/// (`ActivatingView`). Its activation block is set up when it is created, so the fetcher of
/// [`ActivatingViewFetcher`](super::ActivatingViewFetcher) is registered first.
pub(crate) struct ActivatingView {
    reactive: ReactiveObject,
    loaded: Rc<Subject<()>>,
    unloaded: Rc<Subject<()>>,
    is_active_count: Cell<i32>,
    view_model: RefCell<Option<Rc<ActivatingViewModel>>>,
}
reactive_object!(ActivatingView);

impl ActivatingView {
    /// A view without a view model, whose activation block counts.
    pub(crate) fn new() -> Rc<Self> {
        let view = Rc::new(Self {
            reactive: ReactiveObject::new(),
            loaded: Subject::new(),
            unloaded: Subject::new(),
            is_active_count: Cell::new(0),
            view_model: RefCell::new(None),
        });
        let weak = Rc::downgrade(&view);
        let _ = view.when_activated_register(move |d| {
            if let Some(view) = weak.upgrade() {
                view.is_active_count.set(view.is_active_count.get() + 1);
            }
            let weak = weak.clone();
            d(Disposable::create(move || {
                if let Some(view) = weak.upgrade() {
                    view.is_active_count.set(view.is_active_count.get() - 1);
                }
            }));
        });
        view
    }

    /// `Loaded`, which activates the view.
    pub(crate) fn loaded(&self) -> &Rc<Subject<()>> {
        &self.loaded
    }

    /// `Unloaded`, which deactivates the view.
    pub(crate) fn unloaded(&self) -> &Rc<Subject<()>> {
        &self.unloaded
    }

    /// `IsActiveCount`.
    pub(crate) fn is_active_count(&self) -> i32 {
        self.is_active_count.get()
    }

    /// `ViewModel`.
    pub(crate) fn view_model(&self) -> Option<Rc<ActivatingViewModel>> {
        self.view_model.borrow().clone()
    }

    /// Sets `ViewModel`, raising its change when it is another view model (`RaiseAndSetIfChanged`).
    pub(crate) fn set_view_model(&self, value: Option<Rc<ActivatingViewModel>>) {
        let same = match (&*self.view_model.borrow(), &value) {
            (Some(current), Some(value)) => Rc::ptr_eq(current, value),
            (None, None) => true,
            _ => false,
        };
        if same {
            return;
        }
        self.raise_property_changing("ViewModel");
        *self.view_model.borrow_mut() = value;
        self.raise_property_changed("ViewModel");
    }
}

impl IActivatableView for ActivatingView {
    fn as_view_for(&self) -> Option<&dyn IViewFor> {
        Some(self)
    }

    fn as_notify_property_changed(self: Rc<Self>) -> Option<Rc<dyn INotifyPropertyChanged>> {
        Some(self)
    }

    fn as_can_activate(&self) -> Option<&dyn ICanActivate> {
        Some(self)
    }
}

impl IViewFor for ActivatingView {
    fn view_model(&self) -> Option<Rc<dyn Any>> {
        ActivatingView::view_model(self).map(|view_model| view_model as Rc<dyn Any>)
    }

    fn set_view_model(&self, view_model: Option<Rc<dyn Any>>) {
        let typed = view_model.map(|view_model| view_model.downcast::<ActivatingViewModel>().unwrap_or_else(|_| panic!("InvalidCastException")));
        ActivatingView::set_view_model(self, typed);
    }
}

impl IViewForOf<ActivatingViewModel> for ActivatingView {
    fn view_model(&self) -> Option<Rc<ActivatingViewModel>> {
        ActivatingView::view_model(self)
    }

    fn set_view_model(&self, view_model: Option<Rc<ActivatingViewModel>>) {
        ActivatingView::set_view_model(self, view_model);
    }
}

impl ICanActivate for ActivatingView {
    /// `Activated`, the `Loaded` subject.
    fn activated(&self) -> Rc<dyn IObservable<()>> {
        self.loaded.as_observable()
    }

    /// `Deactivated`, the `Unloaded` subject.
    fn deactivated(&self) -> Rc<dyn IObservable<()>> {
        self.unloaded.as_observable()
    }
}

impl IDisposable for ActivatingView {
    fn dispose(&self) {
        self.loaded.dispose();
        self.unloaded.dispose();
    }
}
