//! `ReactiveProperty<T>` (`ReactiveUI.Shared/ReactiveProperty/ReactiveProperty.cs`), with its
//! internal sinks (`ValueChangeRelay`, `ValidationStream`, `AsyncProjectObservable`,
//! `PrependObservable`).

use super::{IReactiveProperty, SingletonDataErrorsChangedEventArgs, SingletonPropertyChangedEventArgs};
use crate::internal::subjects::SchedulingObserver;
use crate::internal::SchedulerExtensions;
use crate::internal::PanicException;
use crate::{IHandleObservableErrors, IReactiveObject, ReactiveObject, RxSchedulers};
use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::disposables::{CompositeDisposable, Disposable, ICancelable, SerialDisposable};
use ferroui_reactive::subjects::{BehaviorSubject, ReplaySubject, Subject};
use ferroui_reactive::{Observer, ObservableExt};
use ferroui_splat::IEnableLogger;
use ferroui_system::component_model::{
    DataErrorsChangedEventArgs, INotifyDataErrorInfo, INotifyPropertyChanged, INotifyPropertyChanging,
};
use ferroui_system::events::Event;
use ferroui_system::threading::tasks::Task;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::any::Any;
use std::cell::{Cell, OnceCell, RefCell};
use std::fmt;
use std::rc::{Rc, Weak};

/// The validation errors of a reactive property, as `GetErrors` and `ObserveErrorChanged` give
/// them (upstream's `IEnumerable` of errors): the messages are `String`s, and the other errors are
/// the values the validators reported.
pub type DataErrors = Vec<Rc<dyn Any>>;

/// What one validator of a reactive property reports (upstream's `IEnumerable?`), when it reports
/// errors: `None` is no error.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream's validators return an `IEnumerable`, which is
/// a string for a message, since a string is an `IEnumerable` of characters, and the aggregate
/// tells the two apart (`is string`). The two cases are the variants of this type, so the
/// aggregate keeps upstream's order: the messages first, then the errors of the other validators.
#[derive(Clone)]
pub enum ValidationErrors {
    /// An error message (a `string` validator result).
    Message(String),
    /// A collection of errors (any other `IEnumerable`).
    Errors(DataErrors),
}

impl fmt::Debug for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message(message) => f.debug_tuple("Message").field(message).finish(),
            Self::Errors(errors) => f.debug_tuple("Errors").field(&errors.len()).finish(),
        }
    }
}

/// A result of a validator that converts to what the property aggregates: an optional message
/// (`Option<String>`, `Option<&str>`), an optional collection of messages (`Option<Vec<String>>`,
/// `Option<Vec<&str>>`), an optional collection of untyped errors (`Option<DataErrors>`) or
/// [`ValidationErrors`] itself.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream overloads `AddValidationError` for the
/// validators that return a `string` and those that return an `IEnumerable`; the overloads here
/// take any result that implements this trait.
pub trait IntoValidationErrors {
    /// The result as the errors the property aggregates; `None` is no error.
    fn into_validation_errors(self) -> Option<ValidationErrors>;
}

impl IntoValidationErrors for Option<ValidationErrors> {
    fn into_validation_errors(self) -> Option<ValidationErrors> {
        self
    }
}

impl IntoValidationErrors for Option<String> {
    fn into_validation_errors(self) -> Option<ValidationErrors> {
        self.map(ValidationErrors::Message)
    }
}

impl IntoValidationErrors for Option<&str> {
    fn into_validation_errors(self) -> Option<ValidationErrors> {
        self.map(|message| ValidationErrors::Message(message.to_owned()))
    }
}

impl IntoValidationErrors for Option<Vec<String>> {
    fn into_validation_errors(self) -> Option<ValidationErrors> {
        self.map(|messages| ValidationErrors::Errors(messages.into_iter().map(|message| Rc::new(message) as Rc<dyn Any>).collect()))
    }
}

impl IntoValidationErrors for Option<Vec<&str>> {
    fn into_validation_errors(self) -> Option<ValidationErrors> {
        self.map(|messages| {
            ValidationErrors::Errors(messages.into_iter().map(|message| Rc::new(message.to_owned()) as Rc<dyn Any>).collect())
        })
    }
}

impl IntoValidationErrors for Option<DataErrors> {
    fn into_validation_errors(self) -> Option<ValidationErrors> {
        self.map(ValidationErrors::Errors)
    }
}

/// A validator in its general form: from the values to validate, the errors.
type Validator<T> = Rc<dyn Fn(Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<Option<ValidationErrors>>>>;

struct Inner<T: Clone + PartialEq + 'static> {
    reactive: ReactiveObject,
    /// `_scheduler`.
    scheduler: Rc<dyn IScheduler>,
    /// `_disposables`.
    disposables: Rc<CompositeDisposable>,
    /// `_checkValidation`.
    check_validation: Rc<Subject<T>>,
    /// `_valueRefereshed`.
    value_refreshed: Rc<Subject<T>>,
    /// `_valueChanged`.
    value_changed: Rc<BehaviorSubject<T>>,
    /// `_validationDisposable`.
    validation_disposable: Rc<SerialDisposable>,
    /// `_errorChanged`, created when first needed.
    error_changed: OnceCell<Rc<BehaviorSubject<Option<DataErrors>>>>,
    /// `_validatorStore`.
    validator_store: RefCell<Vec<Validator<T>>>,
    /// `_skipCurrentValue`.
    skip_current_value: usize,
    /// `_isDistinctUntilChanged`.
    is_distinct_until_changed: bool,
    /// `_observable`.
    observable: OnceCell<Rc<ReplaySubject<T>>>,
    /// `_value`.
    value: RefCell<T>,
    /// `_currentErrors`.
    current_errors: RefCell<Option<DataErrors>>,
    /// `ErrorsChanged`.
    errors_changed: Event<DataErrorsChangedEventArgs>,
}

impl<T: Clone + PartialEq + 'static> Inner<T> {
    fn is_disposed(&self) -> bool {
        self.disposables.is_disposed()
    }

    fn has_errors(&self) -> bool {
        self.current_errors.borrow().is_some()
    }

    fn error_changed(&self) -> &Rc<BehaviorSubject<Option<DataErrors>>> {
        self.error_changed.get_or_init(|| BehaviorSubject::new(self.current_errors.borrow().clone()))
    }
}

/// A property that holds a value, notifies its changes to its observers and through
/// `PropertyChanged`, and validates it (`ReactiveProperty<T>`).
///
/// The handle is shared: clones are the same property, as the object of .NET is a reference.
/// Subscribers receive the current value when they subscribe (unless the property skips it), then
/// each change, on the scheduler of the property. Validators turn the values into errors, which
/// the property aggregates and reports through [`INotifyDataErrorInfo`], `HasErrors` and
/// [`observe_error_changed`](Self::observe_error_changed). Disposing the property ends its
/// subscriptions.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream derives the class from `ReactiveObject`. The
/// handle holds a [`ReactiveObject`] and implements [`IReactiveObject`] itself, so it raises its
/// `PropertyChanged` and `PropertyChanging` events as a reactive object does.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream's constructors and its `Create` factories take
/// the same arguments; both are the constructors here. A value of a type with no "null" is held as
/// it is, and `ReactiveProperty<Option<T>>` holds an optional value.
///
/// ```
/// use ferroui_reactive::concurrency::ImmediateScheduler;
/// use ferroui_reactive::ObservableExt;
/// use ferroui_reactiveui::ReactiveProperty;
/// use std::cell::RefCell;
/// use std::rc::Rc;
///
/// let name = ReactiveProperty::with_scheduler(String::new(), Some(ImmediateScheduler::instance()), false, false)
///     .add_validation_error(|name: String| if name.is_empty() { Some("required") } else { None });
/// let seen = Rc::new(RefCell::new(Vec::new()));
/// let log = seen.clone();
/// let _subscription = name.subscribe_next(move |value| log.borrow_mut().push(value));
/// assert!(name.has_errors());
/// name.set_value("Ada".to_owned());
/// assert!(!name.has_errors());
/// assert_eq!(*seen.borrow(), vec![String::new(), "Ada".to_owned()]);
/// ```
pub struct ReactiveProperty<T: Clone + PartialEq + 'static> {
    inner: Rc<Inner<T>>,
}

impl<T: Clone + PartialEq + 'static> Clone for ReactiveProperty<T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T: Clone + PartialEq + 'static> PartialEq for ReactiveProperty<T> {
    /// The same property (reference equality, as for the object of .NET).
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl<T: Clone + PartialEq + fmt::Debug + 'static> fmt::Debug for ReactiveProperty<T> {
    /// `[DebuggerDisplay("{IsDisposed}, {Value}")]`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReactiveProperty")
            .field("is_disposed", &self.is_disposed())
            .field("value", &*self.inner.value.borrow())
            .finish()
    }
}

impl<T: Clone + PartialEq + Default + 'static> Default for ReactiveProperty<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + PartialEq + Default + 'static> ReactiveProperty<T> {
    /// A property of the default value, which notifies on the task pool scheduler, gives the
    /// current value to subscribers and skips the values equal to the last one
    /// (`ReactiveProperty()`, `Create()`).
    pub fn new() -> Self {
        Self::with_scheduler(T::default(), None, false, false)
    }
}

impl<T: Clone + PartialEq + 'static> ReactiveProperty<T> {
    /// A property of `initial_value`, with the defaults of [`new`](Self::new)
    /// (`ReactiveProperty(T)`, `Create(T)`).
    pub fn with_value(initial_value: T) -> Self {
        Self::with_scheduler(initial_value, None, false, false)
    }

    /// A property of `initial_value` that notifies on the task pool scheduler
    /// (`ReactiveProperty(T, bool, bool)`, `Create(T, bool, bool)`): with
    /// `skip_current_value_on_subscribe`, a subscriber does not receive the current value when it
    /// subscribes; with `allow_duplicate_values`, setting a value equal to the current one notifies
    /// it again.
    pub fn with_options(initial_value: T, skip_current_value_on_subscribe: bool, allow_duplicate_values: bool) -> Self {
        Self::with_scheduler(initial_value, None, skip_current_value_on_subscribe, allow_duplicate_values)
    }

    /// A property of `initial_value` that notifies on `scheduler`, or on the task pool scheduler
    /// when it is `None` (`ReactiveProperty(T, ISequencer?, bool, bool)`,
    /// `Create(T, ISequencer, bool, bool)`).
    pub fn with_scheduler(
        initial_value: T,
        scheduler: Option<Rc<dyn IScheduler>>,
        skip_current_value_on_subscribe: bool,
        allow_duplicate_values: bool,
    ) -> Self {
        let inner = Rc::new(Inner {
            reactive: ReactiveObject::new(),
            scheduler: scheduler.unwrap_or_else(RxSchedulers::task_pool_scheduler),
            disposables: CompositeDisposable::new(),
            check_validation: Subject::new(),
            value_refreshed: Subject::new(),
            value_changed: BehaviorSubject::new(initial_value.clone()),
            validation_disposable: SerialDisposable::new(),
            error_changed: OnceCell::new(),
            validator_store: RefCell::new(Vec::new()),
            skip_current_value: usize::from(skip_current_value_on_subscribe),
            is_distinct_until_changed: !allow_duplicate_values,
            observable: OnceCell::new(),
            value: RefCell::new(initial_value),
            current_errors: RefCell::new(None),
            errors_changed: Event::new(),
        });
        let property = Self { inner };
        property.get_subscription();
        property
    }

    /// Whether the property was disposed (`IsDisposed`).
    pub fn is_disposed(&self) -> bool {
        self.inner.is_disposed()
    }

    /// The value (`Value { get; }`).
    pub fn value(&self) -> T {
        self.inner.value.borrow().clone()
    }

    /// Sets the value (`Value { set; }`): a value equal to the current one notifies it again only
    /// when the property allows duplicate values; another one is stored, validated, notified to
    /// the subscribers and raises `PropertyChanged` for `Value`.
    pub fn set_value(&self, value: T) {
        if *self.inner.value.borrow() == value {
            if !self.inner.is_distinct_until_changed {
                let current = self.value();
                self.inner.value_refreshed.on_next(current);
            }
            return;
        }

        self.set_value_core(value);
        self.raise_property_changed(SingletonPropertyChangedEventArgs::VALUE);
    }

    /// Whether the property has validation errors (`HasErrors`).
    pub fn has_errors(&self) -> bool {
        self.inner.has_errors()
    }

    /// The errors of the property each time they change, starting with the current ones
    /// (`ObserveErrorChanged`); `None` when it has none. It completes when the property is
    /// disposed.
    pub fn observe_error_changed(&self) -> Rc<dyn IObservable<Option<DataErrors>>> {
        self.inner.error_changed().as_observable()
    }

    /// Whether the property has validation errors, each time its errors change, starting with
    /// the current state (`ObserveHasErrors`).
    pub fn observe_has_errors(&self) -> Rc<dyn IObservable<bool>> {
        let inner = Rc::downgrade(&self.inner);
        self.observe_error_changed()
            .select(move |_| inner.upgrade().is_some_and(|inner| inner.has_errors()))
    }

    /// Adds a validator that reads the values as a sequence and reports their errors as a
    /// sequence (`AddValidationError(Func<IObservable<T>, IObservable<IEnumerable?>>)` and its
    /// `string` form); the current value is validated at once. Returns the property, to chain the
    /// calls.
    pub fn add_validation_error_observable<R: IntoValidationErrors + 'static>(
        &self,
        validator: impl Fn(Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<R>> + 'static,
    ) -> Self {
        self.add_validation_error_observable_with(validator, false)
    }

    /// Adds a validator that reads the values as a sequence and reports their errors as a
    /// sequence; with `ignore_initial_error`, the current value is not validated
    /// (`AddValidationError(Func<IObservable<T>, IObservable<IEnumerable?>>, bool)` and its
    /// `string` form). The errors of all the validators are aggregated, and `ErrorsChanged` is
    /// raised each time they are reported.
    pub fn add_validation_error_observable_with<R: IntoValidationErrors + 'static>(
        &self,
        validator: impl Fn(Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<R>> + 'static,
        ignore_initial_error: bool,
    ) -> Self {
        let validator: Validator<T> = Rc::new(move |values: Rc<dyn IObservable<T>>| {
            validator(values).select(IntoValidationErrors::into_validation_errors)
        });
        self.add_validator(validator, ignore_initial_error)
    }

    /// Adds a validator of each value (`AddValidationError(Func<T, IEnumerable?>)` and
    /// `AddValidationError(Func<T, string?>)`); the current value is validated at once.
    pub fn add_validation_error<R: IntoValidationErrors + 'static>(&self, validator: impl Fn(T) -> R + 'static) -> Self {
        self.add_validation_error_with(validator, false)
    }

    /// Adds a validator of each value; with `ignore_initial_error`, the current value is not
    /// validated (`AddValidationError(Func<T, IEnumerable?>, bool)` and
    /// `AddValidationError(Func<T, string?>, bool)`).
    pub fn add_validation_error_with<R: IntoValidationErrors + 'static>(
        &self,
        validator: impl Fn(T) -> R + 'static,
        ignore_initial_error: bool,
    ) -> Self {
        let validator = Rc::new(validator);
        self.add_validation_error_observable_with(
            move |values: Rc<dyn IObservable<T>>| {
                let validator = validator.clone();
                values.select(move |value| validator(value))
            },
            ignore_initial_error,
        )
    }

    /// Adds an asynchronous validator of each value (`AddValidationError(Func<T, Task<IEnumerable?>>)`
    /// and `AddValidationError(Func<T, Task<string?>>)`); the current value is validated at once.
    pub fn add_validation_error_async<R: IntoValidationErrors + Clone + 'static>(
        &self,
        validator: impl Fn(T) -> Task<R> + 'static,
    ) -> Self {
        self.add_validation_error_async_with(validator, false)
    }

    /// Adds an asynchronous validator of each value; with `ignore_initial_error`, the current
    /// value is not validated (`AddValidationError(Func<T, Task<IEnumerable?>>, bool)` and
    /// `AddValidationError(Func<T, Task<string?>>, bool)`). The results of the tasks are reported
    /// as they come; a task that fails, or a validator that panics, ends the validation of the
    /// property, as its error goes to the validation pipeline.
    pub fn add_validation_error_async_with<R: IntoValidationErrors + Clone + 'static>(
        &self,
        validator: impl Fn(T) -> Task<R> + 'static,
        ignore_initial_error: bool,
    ) -> Self {
        let validator: Rc<dyn Fn(T) -> Task<R>> = Rc::new(validator);
        self.add_validation_error_observable_with(
            move |values: Rc<dyn IObservable<T>>| -> Rc<dyn IObservable<R>> {
                Rc::new(AsyncProjectObservable { source: values, selector: validator.clone() })
            },
            ignore_initial_error,
        )
    }

    /// Ends the subscriptions of the property and completes its sequences (`Dispose`). A second
    /// call does nothing.
    pub fn dispose(&self) {
        let inner = &self.inner;
        if inner.is_disposed() {
            return;
        }

        inner.disposables.dispose();

        inner.check_validation.dispose();
        inner.value_refreshed.dispose();
        inner.validation_disposable.dispose();

        inner.value_changed.on_completed();
        inner.value_changed.dispose();

        if let Some(observable) = inner.observable.get() {
            observable.dispose();
        }

        let Some(error_changed) = inner.error_changed.get() else {
            return;
        };
        error_changed.on_completed();
        error_changed.dispose();
    }

    /// Validates the current value again (`CheckValidation`).
    pub fn check_validation(&self) {
        let current = self.value();
        self.inner.check_validation.on_next(current);
    }

    /// Notifies the current value again, to the validators and to the subscribers, even when it
    /// did not change, and raises `PropertyChanged` for `Value` (`Refresh`).
    pub fn refresh(&self) {
        let current = self.value();
        self.set_value_core(current.clone());
        self.inner.value_refreshed.on_next(current);
        self.raise_property_changed(SingletonPropertyChangedEventArgs::VALUE);
    }

    /// The errors of the property, whatever `property_name` is (`GetErrors(string?)`); `None` when
    /// it has none. [`INotifyDataErrorInfo::get_errors`] gives an empty list instead.
    pub fn get_errors(&self, _property_name: Option<&str>) -> Option<DataErrors> {
        self.inner.current_errors.borrow().clone()
    }

    /// The event raised each time the validators report errors (`ErrorsChanged`).
    pub fn errors_changed(&self) -> &Event<DataErrorsChangedEventArgs> {
        &self.inner.errors_changed
    }

    /// `AddValidationError(Func<IObservable<T>, IObservable<IEnumerable?>>, bool)`: stores the
    /// validator and subscribes the aggregate of all the validators again.
    fn add_validator(&self, validator: Validator<T>, ignore_initial_error: bool) -> Self {
        let inner = &self.inner;
        inner.validator_store.borrow_mut().push(validator);
        let validator_funcs: Vec<Validator<T>> = inner.validator_store.borrow().clone();
        let validation_source: Rc<dyn IObservable<T>> = if ignore_initial_error {
            inner.check_validation.as_observable()
        } else {
            Rc::new(PrependObservable { source: inner.check_validation.as_observable(), value: self.value() })
        };
        let validators: Vec<Rc<dyn IObservable<Option<ValidationErrors>>>> =
            validator_funcs.iter().map(|validator| validator(validation_source.clone())).collect();

        let weak: Weak<Inner<T>> = Rc::downgrade(inner);
        let observer = Observer::create_with_error(
            move |errors: Option<DataErrors>| {
                let Some(inner) = weak.upgrade() else { return };
                let last_has_errors = inner.has_errors();
                *inner.current_errors.borrow_mut() = errors.clone();
                let current_has_errors = inner.has_errors();
                if inner.errors_changed.has_handlers() {
                    let raising = inner.clone();
                    let _ = inner.scheduler.schedule_or_inline(move || {
                        raising.errors_changed.raise(&SingletonDataErrorsChangedEventArgs::VALUE);
                    });
                }

                if last_has_errors != current_has_errors {
                    let raising = ReactiveProperty { inner: inner.clone() };
                    let _ = inner.scheduler.schedule_or_inline(move || {
                        raising.raise_property_changed(SingletonPropertyChangedEventArgs::HAS_ERRORS);
                    });
                }

                inner.error_changed().on_next(errors);
            },
            // `DelegateObserver` ignores the errors and the end of the aggregate.
            |_| {},
        );
        let stream = ValidationStream { validators, scheduler: inner.scheduler.clone() };
        inner.validation_disposable.set_disposable(Some(stream.subscribe(observer)));
        self.clone()
    }

    /// `SetValue`: stores the value and, unless the property is disposed, validates and notifies
    /// it.
    fn set_value_core(&self, value: T) {
        *self.inner.value.borrow_mut() = value.clone();

        if self.inner.is_disposed() {
            return;
        }

        self.inner.check_validation.on_next(value.clone());
        self.inner.value_changed.on_next(value);
    }

    /// `GetSubscription`: the shared sequence of the subscribers, a replay of the last value fed
    /// by the changes (through the skip and distinct gates) and by the refreshes.
    fn get_subscription(&self) {
        let inner = &self.inner;
        let relay = ReplaySubject::with_buffer_size(1);

        inner.disposables.add(inner.value_changed.subscribe(Rc::new(ValueChangeRelay {
            relay: relay.as_observer(),
            to_skip: Cell::new(inner.skip_current_value),
            is_distinct: inner.is_distinct_until_changed,
            last: RefCell::new(None),
        })));
        let refreshed = relay.as_observer();
        inner
            .disposables
            .add(inner.value_refreshed.subscribe(Observer::create(move |value: T| refreshed.on_next(value))));

        let _ = inner.observable.set(relay);
    }
}

impl<T: Clone + PartialEq + 'static> IObservable<T> for ReactiveProperty<T> {
    /// Subscribes `observer`, which receives the notifications on the scheduler of the property
    /// (`Subscribe(IObserver<T>)`). On a disposed property it completes at once.
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        if self.is_disposed() {
            observer.on_completed();
            return Disposable::empty();
        }

        let Some(observable) = self.inner.observable.get() else {
            return Disposable::empty();
        };
        let subscription = observable.subscribe(SchedulingObserver::new(observer, self.inner.scheduler.clone()));
        self.inner.disposables.add(subscription.clone());
        subscription
    }
}

impl<T: Clone + PartialEq + 'static> ObservableExt<T> for ReactiveProperty<T> {
    fn observable(&self) -> Rc<dyn IObservable<T>> {
        Rc::new(self.clone())
    }
}

impl<T: Clone + PartialEq + 'static> IDisposable for ReactiveProperty<T> {
    fn dispose(&self) {
        ReactiveProperty::dispose(self);
    }
}

impl<T: Clone + PartialEq + 'static> ICancelable for ReactiveProperty<T> {
    fn is_disposed(&self) -> bool {
        ReactiveProperty::is_disposed(self)
    }
}

impl<T: Clone + PartialEq + 'static> INotifyPropertyChanged for ReactiveProperty<T> {
    fn property_changed(&self) -> &Event<str> {
        self.inner.reactive.property_changed()
    }

    fn as_notify_property_changing(&self) -> Option<&dyn INotifyPropertyChanging> {
        Some(self)
    }
}

impl<T: Clone + PartialEq + 'static> INotifyPropertyChanging for ReactiveProperty<T> {
    fn property_changing(&self) -> &Event<str> {
        self.inner.reactive.property_changing()
    }
}

impl<T: Clone + PartialEq + 'static> IEnableLogger for ReactiveProperty<T> {}

impl<T: Clone + PartialEq + 'static> IReactiveObject for ReactiveProperty<T> {
    fn reactive_object(&self) -> &ReactiveObject {
        &self.inner.reactive
    }
}

impl<T: Clone + PartialEq + 'static> IHandleObservableErrors for ReactiveProperty<T> {
    fn thrown_exceptions(&self) -> Rc<dyn IObservable<ObservableError>> {
        self.inner.reactive.thrown_exceptions()
    }
}

impl<T: Clone + PartialEq + 'static> INotifyDataErrorInfo for ReactiveProperty<T> {
    fn has_errors(&self) -> bool {
        self.inner.has_errors()
    }

    /// The errors of the property, or an empty list when it has none
    /// (`INotifyDataErrorInfo.GetErrors(string?)`).
    fn get_errors(&self, _property_name: Option<&str>) -> Vec<Rc<dyn Any>> {
        self.inner.current_errors.borrow().clone().unwrap_or_default()
    }

    fn errors_changed(&self) -> &Event<DataErrorsChangedEventArgs> {
        &self.inner.errors_changed
    }
}

impl<T: Clone + PartialEq + 'static> IReactiveProperty<T> for ReactiveProperty<T> {
    fn value(&self) -> T {
        ReactiveProperty::value(self)
    }

    fn set_value(&self, value: T) {
        ReactiveProperty::set_value(self, value);
    }

    fn observe_error_changed(&self) -> Rc<dyn IObservable<Option<DataErrors>>> {
        ReactiveProperty::observe_error_changed(self)
    }

    fn observe_has_errors(&self) -> Rc<dyn IObservable<bool>> {
        ReactiveProperty::observe_has_errors(self)
    }

    fn refresh(&self) {
        ReactiveProperty::refresh(self);
    }
}

/// `ValueChangeRelay`: skips the first values, and the values equal to the last one when the
/// property is distinct, before the shared relay.
struct ValueChangeRelay<T> {
    relay: Rc<dyn IObserver<T>>,
    /// `_toSkip`.
    to_skip: Cell<usize>,
    /// `isDistinct`.
    is_distinct: bool,
    /// `_last`, once a value went through (`_hasLast`).
    last: RefCell<Option<T>>,
}

impl<T: Clone + PartialEq + 'static> IObserver<T> for ValueChangeRelay<T> {
    fn on_next(&self, value: T) {
        let to_skip = self.to_skip.get();
        if to_skip > 0 {
            self.to_skip.set(to_skip - 1);
            return;
        }

        if self.is_distinct && self.last.borrow().as_ref() == Some(&value) {
            return;
        }

        *self.last.borrow_mut() = Some(value.clone());
        self.relay.on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.relay.on_error(error);
    }

    fn on_completed(&self) {
        self.relay.on_completed();
    }
}

/// `ValidationStream`: combines the latest errors of every validator once all of them reported,
/// aggregates them, and delivers the aggregate on the scheduler of the property.
struct ValidationStream {
    validators: Vec<Rc<dyn IObservable<Option<ValidationErrors>>>>,
    scheduler: Rc<dyn IScheduler>,
}

impl IObservable<Option<DataErrors>> for ValidationStream {
    fn subscribe(&self, observer: Rc<dyn IObserver<Option<DataErrors>>>) -> Rc<dyn IDisposable> {
        let count = self.validators.len();
        let sink = Rc::new(ValidationSink {
            downstream: SchedulingObserver::new(observer, self.scheduler.clone()),
            latest: RefCell::new(vec![None; count]),
            has: RefCell::new(vec![false; count]),
            subscriptions: RefCell::new(Vec::with_capacity(count)),
            have_count: Cell::new(0),
            done_count: Cell::new(0),
            stopped: Cell::new(false),
        });
        // `Run`.
        for (index, validator) in self.validators.iter().enumerate() {
            let subscription = validator.subscribe(Rc::new(ValidationElement { parent: Rc::downgrade(&sink), index }));
            sink.subscriptions.borrow_mut().push(subscription);
        }
        sink
    }
}

/// `ValidationStream.Sink`.
struct ValidationSink {
    downstream: Rc<SchedulingObserver<Option<DataErrors>>>,
    /// `_latest`.
    latest: RefCell<Vec<Option<ValidationErrors>>>,
    /// `_has`.
    has: RefCell<Vec<bool>>,
    /// `_subscriptions`.
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    /// `_haveCount`.
    have_count: Cell<usize>,
    /// `_doneCount`.
    done_count: Cell<usize>,
    /// `_stopped`.
    stopped: Cell<bool>,
}

impl ValidationSink {
    /// `OnNextAt`.
    fn on_next_at(&self, index: usize, value: Option<ValidationErrors>) {
        if self.stopped.get() {
            return;
        }

        {
            let mut has = self.has.borrow_mut();
            if !has[index] {
                has[index] = true;
                self.have_count.set(self.have_count.get() + 1);
            }
        }

        self.latest.borrow_mut()[index] = value;
        if self.have_count.get() < self.latest.borrow().len() {
            return;
        }

        let aggregated = self.build_aggregate();
        self.downstream.on_next(aggregated);
    }

    /// `BuildAggregate`: the messages first, then the errors of the other validators; `None` when
    /// every validator reported none.
    fn build_aggregate(&self) -> Option<DataErrors> {
        let latest = self.latest.borrow();
        if latest.iter().all(Option::is_none) {
            return None;
        }

        let mut string_values: DataErrors = Vec::with_capacity(latest.len());
        let mut other_values: DataErrors = Vec::new();
        for item in latest.iter().flatten() {
            match item {
                ValidationErrors::Message(message) => string_values.push(Rc::new(message.clone())),
                ValidationErrors::Errors(errors) => other_values.extend(errors.iter().cloned()),
            }
        }

        string_values.extend(other_values);
        Some(string_values)
    }

    /// `OnErrorAt`.
    fn on_error_at(&self, error: ObservableError) {
        if self.stopped.replace(true) {
            return;
        }

        self.downstream.on_error(error);
    }

    /// `OnCompletedAt`: completes once every validator has completed.
    fn on_completed_at(&self) {
        if self.stopped.get() {
            return;
        }

        self.done_count.set(self.done_count.get() + 1);
        if self.done_count.get() < self.latest.borrow().len() {
            return;
        }

        self.stopped.set(true);
        self.downstream.on_completed();
    }
}

impl IDisposable for ValidationSink {
    fn dispose(&self) {
        let subscriptions: Vec<Rc<dyn IDisposable>> = self.subscriptions.borrow().clone();
        for subscription in subscriptions {
            subscription.dispose();
        }
    }
}

/// `ValidationStream.Sink.Element`: routes the notifications of one validator to the sink.
struct ValidationElement {
    parent: Weak<ValidationSink>,
    index: usize,
}

impl IObserver<Option<ValidationErrors>> for ValidationElement {
    fn on_next(&self, value: Option<ValidationErrors>) {
        if let Some(parent) = self.parent.upgrade() {
            parent.on_next_at(self.index, value);
        }
    }

    fn on_error(&self, error: ObservableError) {
        if let Some(parent) = self.parent.upgrade() {
            parent.on_error_at(error);
        }
    }

    fn on_completed(&self) {
        if let Some(parent) = self.parent.upgrade() {
            parent.on_completed_at();
        }
    }
}

/// `AsyncProjectObservable<TIn, TOut>`: runs the asynchronous selector for each value and
/// forwards the results of the tasks as they complete.
struct AsyncProjectObservable<TIn, TOut> {
    source: Rc<dyn IObservable<TIn>>,
    selector: Rc<dyn Fn(TIn) -> Task<TOut>>,
}

impl<TIn: 'static, TOut: Clone + 'static> IObservable<TOut> for AsyncProjectObservable<TIn, TOut> {
    fn subscribe(&self, observer: Rc<dyn IObserver<TOut>>) -> Rc<dyn IDisposable> {
        let sink = Rc::new(AsyncProjectSink {
            downstream: observer,
            selector: self.selector.clone(),
            source_subscription: RefCell::new(None),
            active: Cell::new(1),
            stopped: Cell::new(false),
            this: RefCell::new(Weak::new()),
        });
        *sink.this.borrow_mut() = Rc::downgrade(&sink);
        // `Run`.
        let subscription = self.source.subscribe(sink.clone());
        *sink.source_subscription.borrow_mut() = Some(subscription);
        sink
    }
}

/// `AsyncProjectObservable.Sink`.
struct AsyncProjectSink<TIn, TOut> {
    downstream: Rc<dyn IObserver<TOut>>,
    selector: Rc<dyn Fn(TIn) -> Task<TOut>>,
    /// `_sourceSubscription`.
    source_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    /// `_active`: the tasks running, plus one while the source runs.
    active: Cell<usize>,
    /// `_stopped`.
    stopped: Cell<bool>,
    this: RefCell<Weak<AsyncProjectSink<TIn, TOut>>>,
}

impl<TIn: 'static, TOut: Clone + 'static> AsyncProjectSink<TIn, TOut> {
    /// `ForwardAsync`: the result of the task, or its error.
    fn forward(&self, task: &Task<TOut>) {
        match task.result() {
            Ok(result) => {
                if self.stopped.get() {
                    return;
                }
                self.downstream.on_next(result);
                self.done();
            }
            Err(error) => self.fail(error),
        }
    }

    /// `Done`: completes once the source and every task have ended.
    fn done(&self) {
        let active = self.active.get().saturating_sub(1);
        self.active.set(active);
        if active != 0 {
            return;
        }

        if self.stopped.replace(true) {
            return;
        }
        self.downstream.on_completed();
    }

    /// `Fail`: forwards the first error.
    fn fail(&self, error: ObservableError) {
        if self.stopped.replace(true) {
            return;
        }
        self.downstream.on_error(error);
    }
}

impl<TIn: 'static, TOut: Clone + 'static> IObserver<TIn> for AsyncProjectSink<TIn, TOut> {
    fn on_next(&self, value: TIn) {
        // Upstream catches the exception of the selector; the port catches its panic.
        let task = match PanicException::catch(|| (self.selector)(value)) {
            Ok(task) => task,
            Err((error, _payload)) => {
                self.fail(error);
                return;
            }
        };

        self.active.set(self.active.get() + 1);
        let Some(this) = self.this.borrow().upgrade() else { return };
        let _ = task.continue_with(move |completed: &Task<TOut>| this.forward(completed));
    }

    fn on_error(&self, error: ObservableError) {
        self.fail(error);
    }

    fn on_completed(&self) {
        self.done();
    }
}

impl<TIn: 'static, TOut: Clone + 'static> IDisposable for AsyncProjectSink<TIn, TOut> {
    fn dispose(&self) {
        let subscription = self.source_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }
}

/// `PrependObservable`: the value first, then the source, so that a validator sees the current
/// value first.
struct PrependObservable<T> {
    source: Rc<dyn IObservable<T>>,
    value: T,
}

impl<T: Clone + 'static> IObservable<T> for PrependObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        observer.on_next(self.value.clone());
        self.source.subscribe(observer)
    }
}
