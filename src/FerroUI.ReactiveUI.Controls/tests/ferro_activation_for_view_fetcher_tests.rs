//! `...ActivationForViewFetcherTests` (the tests of the integration).
//!
//! Upstream's test views are controls and visuals that implement `IActivatableView`; here a test
//! view is a type that holds its control or visual and registers itself with
//! [`FerroObjectHandles`], as a view of the port does. Upstream passes the type of the view to the
//! affinity; the port passes the view.

use super::binding_support::{run_jobs, start_app};
use crate::{FerroActivationForViewFetcher, FerroObjectHandles};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, Visual, VisualImpl};
use ferroui_controls::{Button, Control, Window};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{IActivatableView, IActivationForViewFetcher};
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

/// A button that is an activatable view (`ActivatableButton`).
struct ActivatableButton(Ref<Button>);

impl IActivatableView for ActivatableButton {}

/// An activatable view that is not a visual (`ActivatableOnly`).
struct ActivatableOnly;

impl IActivatableView for ActivatableOnly {}

/// A visual that is not a control (the visual of `ActivatableVisual`).
#[repr(C)]
struct TestVisual {
    base: Visual,
}

ferro_class!(TestVisual: Visual);
ferro_impl_classes!(TestVisual: FerroObjectImpl, StyledElementImpl, VisualImpl);

impl TestVisual {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Visual::construct() })
    }
}

/// An activatable visual that is not a control (`ActivatableVisual`).
struct ActivatableVisual(Ref<TestVisual>);

impl IActivatableView for ActivatableVisual {}

/// A new activatable button, its type registered as a view of its button.
fn activatable_button() -> Rc<ActivatableButton> {
    FerroObjectHandles::register::<ActivatableButton>(|view| view.0.clone().upcast());
    Rc::new(ActivatableButton(Button::new()))
}

/// A new activatable visual, its type registered as a view of its visual.
fn activatable_visual() -> Rc<ActivatableVisual> {
    FerroObjectHandles::register::<ActivatableVisual>(|view| view.0.clone().upcast());
    Rc::new(ActivatableVisual(TestVisual::new()))
}

/// Subscribes to the activation of `view` and gives the last value it produced.
fn last_activation(view: Rc<dyn IActivatableView>) -> (Rc<Cell<Option<bool>>>, Rc<dyn IDisposable>) {
    let last = Rc::new(Cell::new(None));
    let seen = last.clone();
    let subscription = FerroActivationForViewFetcher::new().get_activation_for_view(&view).subscribe_next(move |value| seen.set(Some(value)));
    (last, subscription)
}

#[test]
fn get_affinity_for_view_control_returns_positive() {
    let _app = start_app();
    let view = activatable_button();

    assert!(FerroActivationForViewFetcher::new().get_affinity_for_view(&*view) > 0);
}

#[test]
fn get_affinity_for_view_non_visual_returns_zero() {
    assert_eq!(FerroActivationForViewFetcher::new().get_affinity_for_view(&ActivatableOnly), 0);
}

#[test]
fn get_activation_for_view_non_visual_returns_inactive() {
    let (last, subscription) = last_activation(Rc::new(ActivatableOnly));

    assert_eq!(last.get(), Some(false));
    subscription.dispose();
}

#[test]
fn get_activation_for_view_control_emits_on_loaded_and_unloaded() {
    let _app = start_app();
    let view = activatable_button();
    let button = view.0.clone();
    let (last, subscription) = last_activation(view);

    button.raise_event(&RoutedEventArgs::with_event(Control::loaded_event()));
    assert_eq!(last.get(), Some(true));

    button.raise_event(&RoutedEventArgs::with_event(Control::unloaded_event()));
    assert_eq!(last.get(), Some(false));

    subscription.dispose();
}

#[test]
fn get_activation_for_view_visual_emits_on_attach_and_detach() {
    let _app = start_app();
    let host = Control::new();
    let view = activatable_visual();
    let visual: Ref<Visual> = view.0.clone().upcast();
    let window = Window::new();
    window.set_content(Some(Control::boxed(host.clone())));
    let (last, subscription) = last_activation(view);

    host.visual_children().add(visual.clone());
    window.show();
    // Running the jobs applies the template of the window, which attaches its content.
    run_jobs();

    assert_eq!(last.get(), Some(true));

    host.visual_children().remove(&visual);
    assert_eq!(last.get(), Some(false));

    subscription.dispose();
    window.close();
}
