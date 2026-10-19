//! `FerroActivationForViewFetcher` (`...ActivationForViewFetcher.cs` of the integration).

use crate::FerroObjectHandles;
use ferroui_base::interactivity::{Interactive, RoutedEventArgs};
use ferroui_base::{FerroObject, FerroPropertyChangedEventArgs, Ref, Visual, VisualTreeAttachmentEventArgs};
use ferroui_controls::{Control, Window};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactiveui::{IActivatableView, IActivationForViewFetcher};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::any::Any;
use std::rc::Rc;

/// The activation of the views of FerroUI (`FerroActivationForViewFetcher`).
///
/// A view whose object is a [`Visual`] has an affinity; its activation follows the `Loaded` and
/// `Unloaded` events of a [`Control`], or the attachment to and the detachment from the visual tree
/// of any other visual. Each observable emits only when the state changes.
///
/// The object of a view is the control behind a view adapter of the crate, or, for another type of
/// view, the object [`FerroObjectHandles::object_of`] finds: such a view registers its type there
/// ([`FerroObjectHandles::register`] or [`FerroObjectHandles::register_class`]).
///
/// Deviation (DEVIATIONS.md, ReactiveUI controls): the affinity is computed on the view, as every
/// fetcher of the port does. The activation of a [`Window`] also follows its visibility: hiding the
/// window deactivates it and showing it again activates it, as the views of a hidden window are not
/// shown.
#[derive(Debug, Default, Clone, Copy)]
pub struct FerroActivationForViewFetcher;

impl FerroActivationForViewFetcher {
    /// The affinity of the views that are visuals (`FerroVisualAffinity`).
    pub const VISUAL_AFFINITY: i32 = 10;

    /// The fetcher (`new FerroActivationForViewFetcher()`).
    pub fn new() -> Self {
        Self
    }

    /// The FerroUI object of `view`, if it has one: the control behind a view adapter of the crate
    /// ([`view_control`](crate::view_control::view_control)), or the object of a type that stands
    /// for one ([`FerroObjectHandles::object_of`]).
    fn object_of(view: &dyn IActivatableView) -> Option<Ref<FerroObject>> {
        if let Some(control) = crate::view_control::view_control(view) {
            return Some(control.upcast());
        }
        let view: &dyn Any = view;
        FerroObjectHandles::object_of(view)
    }

    /// `true` when `control` is loaded and `false` when it is unloaded, emitted when the state
    /// changes (`GetActivationForControl`).
    fn activation_for_control(control: Ref<Control>) -> Rc<dyn IObservable<bool>> {
        let loaded = routed_flag(&control, true);
        let unloaded = routed_flag(&control, false);
        let activation = loaded.merge(unloaded);
        match control.cast::<Window>() {
            Some(window) => activation.merge(visibility_of(&window)).distinct_until_changed(),
            None => activation.distinct_until_changed(),
        }
    }

    /// `true` when `visual` is attached to the visual tree and `false` when it is detached, emitted
    /// when the state changes (`GetActivationForVisual`).
    fn activation_for_visual(visual: Ref<Visual>) -> Rc<dyn IObservable<bool>> {
        let attached = attachment_flag(&visual, true);
        let detached = attachment_flag(&visual, false);
        attached.merge(detached).distinct_until_changed()
    }
}

impl IActivationForViewFetcher for FerroActivationForViewFetcher {
    /// [`VISUAL_AFFINITY`](Self::VISUAL_AFFINITY) for a view whose object is a [`Visual`], zero
    /// otherwise (`GetAffinityForView`).
    fn get_affinity_for_view(&self, view: &dyn IActivatableView) -> i32 {
        match Self::object_of(view) {
            Some(object) if object.is::<Visual>() => Self::VISUAL_AFFINITY,
            _ => 0,
        }
    }

    /// The activation of a control or of another visual; a single `false` for a view that is not a
    /// visual (`GetActivationForView`).
    fn get_activation_for_view(&self, view: &Rc<dyn IActivatableView>) -> Rc<dyn IObservable<bool>> {
        let Some(visual) = Self::object_of(&**view).and_then(|object| object.cast::<Visual>()) else {
            return Observable::return_(false);
        };
        match visual.cast::<Control>() {
            Some(control) => Self::activation_for_control(control),
            None => Self::activation_for_visual(visual),
        }
    }
}

/// `value` each time the `Loaded` event (`value` true) or the `Unloaded` event (`value` false) of
/// `control` is raised (`Observable.FromEventPattern<RoutedEventArgs>(...).Select(...)`). The
/// observable holds the control weakly; the subscription holds it until it is disposed.
fn routed_flag(control: &Ref<Control>, value: bool) -> Rc<dyn IObservable<bool>> {
    let weak = control.downgrade();
    Observable::create(move |observer: Rc<dyn IObserver<bool>>| {
        let Some(control) = weak.upgrade() else {
            return Disposable::empty();
        };
        let routed_event = if value { Control::loaded_event() } else { Control::unloaded_event() };
        let token = control.add_handler(routed_event, move |_: &Interactive, _: &RoutedEventArgs| observer.on_next(value));
        Disposable::create(move || {
            control.remove_handler(routed_event, token);
        })
    })
}

/// `value` each time `visual` is attached to the visual tree (`value` true) or detached from it
/// (`value` false).
fn attachment_flag(visual: &Ref<Visual>, value: bool) -> Rc<dyn IObservable<bool>> {
    let weak = visual.downgrade();
    Observable::create(move |observer: Rc<dyn IObserver<bool>>| {
        let Some(visual) = weak.upgrade() else {
            return Disposable::empty();
        };
        let handler = move |_: &VisualTreeAttachmentEventArgs| observer.on_next(value);
        let subscription: Rc<dyn IDisposable> =
            if value { visual.attached_to_visual_tree(handler) } else { visual.detached_from_visual_tree(handler) };
        subscription
    })
}

/// The visibility of `window` each time it changes.
fn visibility_of(window: &Ref<Window>) -> Rc<dyn IObservable<bool>> {
    let weak = window.downgrade();
    Observable::create(move |observer: Rc<dyn IObserver<bool>>| {
        let Some(window) = weak.upgrade() else {
            return Disposable::empty();
        };
        window.property_changed(move |args: &FerroPropertyChangedEventArgs<'_>| {
            if args.property() == Visual::is_visible_property().as_property() {
                observer.on_next(args.get_new_value::<bool>());
            }
        })
    })
}
