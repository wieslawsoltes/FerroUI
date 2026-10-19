//! `ViewHostNavigation` (`ViewHostNavigation.cs`): resolves and wires the view a content host
//! displays for a view model.
//!
//! The hosts differ only in where the view model comes from, so view resolution, its logging and
//! the subscription bookkeeping live here rather than once per host.

use crate::subscription_errors::SubscriptionErrors;
use crate::view_control::view_control;
use crate::view_model_value::value_of_view_model;
use crate::NavigationTarget;
use ferroui_base::BoxedValue;
use ferroui_controls::Control;
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactive::Observer;
use ferroui_reactiveui::{IViewFor, IViewLocator, ViewLocator};
use ferroui_splat::{AppLocator, IEnableLogger, ILogManager};
use ferroui_system::{IDisposable, IObservable};
use std::any::Any;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// A view that is not a control, as the content of a host (the view object itself as `Content`
/// upstream): compared by identity.
#[derive(Clone)]
pub struct ViewForValue(pub Rc<dyn IViewFor>);

impl PartialEq for ViewForValue {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(Rc::as_ptr(&self.0), Rc::as_ptr(&other.0))
    }
}

impl fmt::Debug for ViewForValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ViewForValue").field(&(Rc::as_ptr(&self.0) as *const ())).finish()
    }
}

/// Whether the hosts can log: a log manager is registered.
///
/// Deviation (DEVIATIONS.md, ReactiveUI controls): upstream's `this.Log()` throws without a log
/// manager; the hosts work without one, as before the application is built (as the port's
/// `DefaultViewLocator` does).
fn can_log() -> bool {
    AppLocator::get_service::<dyn ILogManager>().is_some()
}

/// Writes an information message of `host` (`this.Log().Info(..)`).
pub(crate) fn log_info<H: IEnableLogger>(host: &H, message: &str) {
    if can_log() {
        host.log().info(message);
    }
}

/// Writes a warning of `host` (`this.Log().Warn(..)`).
pub(crate) fn log_warn<H: IEnableLogger>(host: &H, message: &str) {
    if can_log() {
        host.log().warn(message);
    }
}

/// The address of a view model or view, as the logs name it (upstream prints `ToString()`).
fn address<T: ?Sized>(object: &Rc<T>) -> *const () {
    Rc::as_ptr(object) as *const ()
}

/// Resolves and wires the view a content host displays for a view model (`ViewHostNavigation`).
pub struct ViewHostNavigation;

impl ViewHostNavigation {
    /// Resolves the view for a view model and hands it the view model (`ResolveView`): the view
    /// gets the view model through its `ViewModel` and, when it is a control, its `DataContext`.
    /// `None` when no view is registered for the view model.
    ///
    /// `view_locator` is the locator to use, or `None` for the current one.
    pub fn resolve_view<H: IEnableLogger>(
        host: &H,
        view_model: Rc<dyn Any>,
        contract: Option<&str>,
        view_locator: Option<Rc<dyn IViewLocator>>,
    ) -> Option<Rc<dyn IViewFor>> {
        let locator = view_locator.unwrap_or_else(ViewLocator::current);
        // Deviation (DEVIATIONS.md, ReactiveUI controls): the view is resolved with
        // `resolve_view_unsafe`, which asks the generated and mapped views first, then the service
        // locator. Upstream's source generator writes the dispatch of every view of the
        // application, and its `ViewRegistrar` registers the same views with the service locator;
        // here the views that `ViewRegistrar` registers are found through the service locator.
        let Some(view_instance) = locator.resolve_view_unsafe(Some(view_model.clone()), contract) else {
            Self::log_missing_view(host, &view_model, contract);
            return None;
        };

        let message = match contract {
            None => format!("Ready to show {:?} with autowired {:?}.", address(&view_instance), address(&view_model)),
            Some(contract) => format!(
                "Ready to show {:?} with autowired {:?} and contract '{contract}'.",
                address(&view_instance),
                address(&view_model)
            ),
        };
        log_info(host, &message);

        view_instance.set_view_model(Some(view_model.clone()));
        if let Some(control) = view_control(&*view_instance) {
            control.set_data_context(Some(value_of_view_model(view_model)));
        }

        Some(view_instance)
    }

    /// The content that displays `view`: its control, or the view itself when it is not one.
    pub fn content_of_view(view: &Rc<dyn IViewFor>) -> BoxedValue {
        match view_control(&**view) {
            Some(control) => Control::boxed(control),
            None => Rc::new(ViewForValue(view.clone())),
        }
    }

    /// Subscribes a host's navigation stream and returns the subscription it owns while attached
    /// (`Subscribe`); `navigate` displays the view for a target. An error of the stream is
    /// rethrown ([`SubscriptionErrors::throw`]).
    pub fn subscribe(
        navigation: Rc<dyn IObservable<NavigationTarget>>,
        navigate: impl Fn(Option<Rc<dyn Any>>, Option<String>) + 'static,
    ) -> Rc<dyn IDisposable> {
        let subscription = navigation.subscribe(Observer::create_all(
            move |target: NavigationTarget| navigate(target.view_model, target.contract),
            |error| SubscriptionErrors::throw(error),
            || {},
        ));

        CompositeDisposable::from_disposables([subscription])
    }

    /// Disposes the supplied subscriptions and clears the reference (`Release`).
    ///
    /// The reference is cleared before disposal so a re-entrant detach cannot dispose the same set
    /// twice.
    pub fn release(disposables: &RefCell<Option<Rc<dyn IDisposable>>>) {
        let pending = disposables.borrow_mut().take();
        if let Some(pending) = pending {
            pending.dispose();
        }
    }

    /// Logs that no view is registered for a view model (`LogMissingView`).
    fn log_missing_view<H: IEnableLogger>(host: &H, view_model: &Rc<dyn Any>, contract: Option<&str>) {
        let message = match contract {
            None => format!(
                "Couldn't find view for '{:?}'. Is it registered? Falling back to default content.",
                address(view_model)
            ),
            Some(contract) => format!(
                "Couldn't find view with contract '{contract}' for '{:?}'. Is it registered? Falling back to default content.",
                address(view_model)
            ),
        };
        log_warn(host, &message);
    }
}
