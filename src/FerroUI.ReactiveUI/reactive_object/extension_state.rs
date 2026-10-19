//! `ExtensionState<TSender>` (`ReactiveUI.Shared/ReactiveObject/ExtensionState.cs`).

use super::delayable_notification_signal::DelayableNotificationSignal;
use super::IExtensionState;
use crate::internal::PanicException;
use crate::{RxState, ScheduledSubject};
use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::Observer;
use ferroui_splat::{AppLocator, ILogManager};
use ferroui_system::events::Event;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, OnceCell};
use std::collections::HashSet;
use std::rc::{Rc, Weak};

/// The `PropertyChanging` and `PropertyChanged` events of a reactive object, which its
/// [`ReactiveObject`](crate::ReactiveObject) and its state share.
pub(crate) struct PropertyEvents {
    pub(crate) property_changing: Event<str>,
    pub(crate) property_changed: Event<str>,
}

impl PropertyEvents {
    pub(crate) fn new() -> Self {
        Self { property_changing: Event::new(), property_changed: Event::new() }
    }
}

type NameSignal = Rc<DelayableNotificationSignal<Rc<str>>>;

/// The state of the change notifications of a reactive object (`ExtensionState<TSender>`): the
/// suppression and delay counts, the notifications kept while delayed, and the subjects of the
/// changes and of the errors of their subscribers, each created when first needed.
///
/// The order of the notifications is upstream's: a change raises the event, then the observable;
/// a delay keeps both and sends, when it ends, the last notification of each property in the
/// order of those last notifications, the changing event first, then the changed event, then the
/// changing and changed observables. A panic of a subscriber of the observables goes to
/// `thrown_exceptions` once something asked for it, and propagates otherwise.
pub(crate) struct ExtensionState {
    this: Weak<ExtensionState>,
    /// The events of the object (`_sender.RaisePropertyChanging`/`RaisePropertyChanged`).
    events: Rc<PropertyEvents>,
    /// The type of the object, for the logger of its errors (`rxObj.Log()`).
    sender_type: Cell<&'static str>,
    /// `_thrownExceptions`.
    thrown_exceptions: OnceCell<Rc<ScheduledSubject<ObservableError>>>,
    /// `_changing`: the `Changing` observable.
    changing: OnceCell<NameSignal>,
    /// `_changed`: the `Changed` observable.
    changed: OnceCell<NameSignal>,
    /// `_propertyChanging`: raises the `PropertyChanging` event.
    property_changing: OnceCell<NameSignal>,
    /// `_propertyChanged`: raises the `PropertyChanged` event.
    property_changed: OnceCell<NameSignal>,
    /// `_changeNotificationsSuppressed`.
    change_notifications_suppressed: Rc<Cell<i64>>,
    /// `_changeNotificationsDelayed`.
    change_notifications_delayed: Rc<Cell<i64>>,
}

impl ExtensionState {
    /// The state of an object whose events are `events`.
    pub(crate) fn new(events: Rc<PropertyEvents>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            events,
            sender_type: Cell::new(std::any::type_name::<crate::ReactiveObject>()),
            thrown_exceptions: OnceCell::new(),
            changing: OnceCell::new(),
            changed: OnceCell::new(),
            property_changing: OnceCell::new(),
            property_changed: OnceCell::new(),
            change_notifications_suppressed: Rc::new(Cell::new(0)),
            change_notifications_delayed: Rc::new(Cell::new(0)),
        })
    }

    /// Records the type of the object whose changes are raised, for the logger of its errors.
    pub(crate) fn set_sender_type(&self, sender_type: &'static str) {
        self.sender_type.set(sender_type);
    }

    /// The last notification of each property in the order of those last notifications
    /// (`DistinctEvents`).
    fn distinct_events(events: Vec<Rc<str>>) -> Vec<Rc<str>> {
        if events.len() <= 1 {
            return events;
        }
        let mut seen: HashSet<Rc<str>> = HashSet::with_capacity(events.len());
        let mut unique: Vec<Rc<str>> = Vec::with_capacity(events.len());
        for name in events.into_iter().rev() {
            if seen.insert(name.clone()) {
                unique.push(name);
            }
        }
        unique.reverse();
        unique
    }

    /// The subject of an observable of changes (`CreateLazyDelayableSubjectAndObservable`).
    fn create_delayable_signal(&self) -> NameSignal {
        let delayed = self.change_notifications_delayed.clone();
        DelayableNotificationSignal::new(move || delayed.get() > 0, Self::distinct_events)
    }

    /// The subject that raises an event of the object (`CreateLazyDelayableEventSubject`).
    fn create_delayable_event_signal(&self, raise_event: fn(&PropertyEvents, &str)) -> NameSignal {
        let signal = self.create_delayable_signal();
        let events = self.events.clone();
        let observer: Rc<dyn IObserver<Rc<str>>> = Observer::create(move |name: Rc<str>| raise_event(&events, &name));
        let _ = signal.subscribe(observer);
        signal
    }

    fn changing_signal(&self) -> NameSignal {
        self.changing.get_or_init(|| self.create_delayable_signal()).clone()
    }

    fn changed_signal(&self) -> NameSignal {
        self.changed.get_or_init(|| self.create_delayable_signal()).clone()
    }

    fn thrown_exceptions_subject(&self) -> Rc<ScheduledSubject<ObservableError>> {
        self.thrown_exceptions
            .get_or_init(|| ScheduledSubject::with_default_observer(ImmediateScheduler::instance(), Some(RxState::default_exception_handler())))
            .clone()
    }

    /// Flushes a signal if it has been created (`FlushIfCreated`).
    fn flush_if_created(signal: &OnceCell<NameSignal>) {
        let Some(signal) = signal.get().cloned() else {
            return;
        };
        signal.flush();
    }

    /// Sends what was kept while change notifications were delayed (`FlushDelayed`).
    fn flush_delayed(&self) {
        Self::flush_if_created(&self.property_changing);
        Self::flush_if_created(&self.property_changed);
        Self::flush_if_created(&self.changing);
        Self::flush_if_created(&self.changed);
    }

    /// Sends `item` to a subject of the observables, routing a panic of a subscriber to
    /// `thrown_exceptions` once it exists and propagating it otherwise (`NotifyObservable`).
    fn notify_observable(&self, item: Rc<str>, subject: NameSignal) {
        let Err((error, payload)) = PanicException::catch(move || subject.on_next(item)) else {
            return;
        };
        log_subscriber_exception(self.sender_type.get(), &error);
        let Some(thrown_exceptions) = self.thrown_exceptions.get().cloned() else {
            std::panic::resume_unwind(payload);
        };
        thrown_exceptions.on_next(error);
    }
}

/// Logs the error of a subscriber with the logger of the type of the object
/// (`rxObj.Log().Error(ex, "ReactiveObject Subscriber threw exception")`), when the locator has a
/// log manager.
fn log_subscriber_exception(sender_type: &'static str, error: &ObservableError) {
    let Some(log_manager) = AppLocator::get_service::<dyn ILogManager>() else {
        return;
    };
    log_manager.get_logger(sender_type).error_exception(&**error, "ReactiveObject Subscriber threw exception");
}

impl IExtensionState for ExtensionState {
    fn changing(&self) -> Rc<dyn IObservable<Rc<str>>> {
        self.changing_signal()
    }

    fn changed(&self) -> Rc<dyn IObservable<Rc<str>>> {
        self.changed_signal()
    }

    fn thrown_exceptions(&self) -> Rc<dyn IObservable<ObservableError>> {
        self.thrown_exceptions_subject()
    }

    fn subscribe_changing(&self) {
        let _ = self.property_changing.get_or_init(|| self.create_delayable_event_signal(|events, name| events.property_changing.raise(name)));
    }

    fn raise_changing(&self, property_name: &str) {
        if !self.notifications_enabled() {
            return;
        }
        // Deviation (DEVIATIONS.md, ReactiveUI): `Event` has no hook on adding a handler, so the
        // event subject is created when a change finds a handler, where upstream creates it when
        // the first handler is added.
        if self.events.property_changing.has_handlers() {
            self.subscribe_changing();
        }
        let raises_event = self.property_changing.get().cloned();
        let raises_observable = self.changing.get().cloned();
        if raises_event.is_none() && raises_observable.is_none() {
            // No PropertyChanging handler and no Changing observer: nothing to allocate.
            return;
        }
        let changing: Rc<str> = Rc::from(property_name);
        if let Some(event) = raises_event {
            event.on_next(changing.clone());
        }
        let Some(observable) = raises_observable else {
            return;
        };
        self.notify_observable(changing, observable);
    }

    fn subscribe_changed(&self) {
        let _ = self.property_changed.get_or_init(|| self.create_delayable_event_signal(|events, name| events.property_changed.raise(name)));
    }

    fn raise_changed(&self, property_name: &str) {
        if !self.notifications_enabled() {
            return;
        }
        if self.events.property_changed.has_handlers() {
            self.subscribe_changed();
        }
        let raises_event = self.property_changed.get().cloned();
        let raises_observable = self.changed.get().cloned();
        if raises_event.is_none() && raises_observable.is_none() {
            // No PropertyChanged handler and no Changed observer: nothing to allocate.
            return;
        }
        let changed: Rc<str> = Rc::from(property_name);
        if let Some(event) = raises_event {
            event.on_next(changed.clone());
        }
        let Some(observable) = raises_observable else {
            return;
        };
        self.notify_observable(changed, observable);
    }

    fn notifications_enabled(&self) -> bool {
        self.change_notifications_suppressed.get() == 0
    }

    fn suppress(&self) -> Rc<dyn IDisposable> {
        let suppressed = self.change_notifications_suppressed.clone();
        suppressed.set(suppressed.get() + 1);
        Disposable::create(move || suppressed.set(suppressed.get() - 1))
    }

    fn are_change_notifications_delayed(&self) -> bool {
        self.change_notifications_delayed.get() > 0
    }

    fn delay(&self) -> Rc<dyn IDisposable> {
        let delayed = self.change_notifications_delayed.get() + 1;
        self.change_notifications_delayed.set(delayed);
        if delayed == 1 {
            self.flush_delayed();
        }
        let this = self.this.clone();
        Disposable::create(move || {
            let Some(this) = this.upgrade() else {
                return;
            };
            let delayed = this.change_notifications_delayed.get() - 1;
            this.change_notifications_delayed.set(delayed);
            if delayed != 0 {
                return;
            }
            this.flush_delayed();
        })
    }
}
