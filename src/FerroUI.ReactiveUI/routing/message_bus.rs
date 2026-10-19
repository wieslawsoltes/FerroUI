//! `MessageBus` (`ReactiveUI.Shared/Routing/MessageBus.cs`).

use super::{NotAWeakReference, SkipFirstObserver};
use crate::{IMessageBus, MessageKey, ScheduledSubject};
use ferroui_reactive::concurrency::{CurrentThreadScheduler, IScheduler};
use ferroui_reactive::subjects::{BehaviorSubject, SubjectBase};
use ferroui_reactive::Observer;
use ferroui_splat::{AppLocator, IEnableLogger, IFullLogger, ILogManager};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

ferroui_system::ferro_thread_local! {
    /// The bus of the application, if one was set or created (`Current`).
    static CURRENT: RefCell<Option<Rc<dyn IMessageBus>>> = const { RefCell::new(None) };
}

/// A message bus: a simple way for view models and other objects to talk to each other without
/// knowing each other (`MessageBus`).
///
/// Which messages go where is decided by the type of the message and a contract, a string that
/// tells apart the messages of the same type and that the client chooses. The typed operations
/// are the methods of [`IMessageBusExt`]:
///
/// ```
/// use ferroui_reactive::ObservableExt;
/// use ferroui_reactiveui::{IMessageBusExt, MessageBus};
/// use std::cell::RefCell;
/// use std::rc::Rc;
///
/// let bus = MessageBus::new();
/// let received = Rc::new(RefCell::new(Vec::new()));
/// let log = received.clone();
/// let _subscription = bus.listen::<String>().subscribe_next(move |message| log.borrow_mut().push(message));
///
/// bus.send_message("Hello".to_owned());
/// bus.send_message_with_contract("Elsewhere".to_owned(), Some("other"));
/// assert_eq!(*received.borrow(), ["Hello"]);
/// ```
///
/// A channel delivers on the scheduler registered for it, the current-thread scheduler by
/// default. A channel holds the latest message, which
/// [`listen_include_latest`](IMessageBusExt::listen_include_latest) replays (the default value
/// of the type before any message).
// Deviation (DEVIATIONS.md, ReactiveUI): the bus is single-threaded, as the observables are
// (docs/porting/reactive.md, section 5.1), and `Current` is per thread.
pub struct MessageBus {
    /// The channels, by key (`_messageBus`).
    message_bus: RefCell<HashMap<MessageKey, NotAWeakReference>>,
    /// The schedulers registered for the channels (`_schedulerMappings`).
    scheduler_mappings: RefCell<HashMap<MessageKey, Rc<dyn IScheduler>>>,
}

impl fmt::Debug for MessageBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MessageBus")
            .field("messages", &self.message_bus.borrow().len())
            .field("scheduler_mappings", &self.scheduler_mappings.borrow().len())
            .finish()
    }
}

impl MessageBus {
    /// A bus without channels.
    pub fn new() -> Rc<Self> {
        Rc::new(Self { message_bus: RefCell::new(HashMap::new()), scheduler_mappings: RefCell::new(HashMap::new()) })
    }

    /// The bus of the application (`Current`): a [`MessageBus`] unless another one was set.
    pub fn current() -> Rc<dyn IMessageBus> {
        if let Some(current) = CURRENT.with(|current| current.borrow().clone()) {
            return current;
        }
        let bus: Rc<dyn IMessageBus> = Self::new();
        CURRENT.with(|current| *current.borrow_mut() = Some(bus.clone()));
        bus
    }

    /// Replaces the bus of the application (`Current { set; }`), for tests or another
    /// implementation.
    pub fn set_current(bus: Rc<dyn IMessageBus>) {
        CURRENT.with(|current| *current.borrow_mut() = Some(bus));
    }

    /// The scheduler of a channel: the registered one, or the current-thread scheduler
    /// (`GetScheduler`).
    fn get_scheduler(&self, key: &MessageKey) -> Rc<dyn IScheduler> {
        self.scheduler_mappings.borrow().get(key).cloned().unwrap_or_else(CurrentThreadScheduler::instance)
    }
}

impl IEnableLogger for MessageBus {}

impl IMessageBus for MessageBus {
    fn register_scheduler_for(&self, key: MessageKey, scheduler: Rc<dyn IScheduler>) {
        self.scheduler_mappings.borrow_mut().insert(key, scheduler);
    }

    fn is_registered_with_contract(&self, message_type: TypeId, contract: Option<&str>) -> bool {
        let key = MessageKey::new(message_type, "", contract);
        let mut message_bus = self.message_bus.borrow_mut();
        let registered = message_bus.get(&key).is_some_and(NotAWeakReference::is_alive);
        if message_bus.get(&key).is_some_and(|reference| !reference.is_alive()) {
            message_bus.remove(&key);
        }
        registered
    }

    fn message_subject(&self, key: &MessageKey, create: &dyn Fn(Rc<dyn IScheduler>) -> Rc<dyn Any>) -> Rc<dyn Any> {
        if let Some(reference) = self.message_bus.borrow().get(key) {
            if reference.is_alive() {
                return reference.target().clone();
            }
        }
        let subject = create(self.get_scheduler(key));
        self.message_bus.borrow_mut().insert(key.clone(), NotAWeakReference::new(subject.clone()));
        subject
    }
}

/// The typed operations of a message bus (the generic methods of `IMessageBus`), on its shared
/// handle: `Rc<MessageBus>` and `Rc<dyn IMessageBus>` have them.
///
/// The type of the message is a type parameter; a message type has a default value, which
/// [`listen_include_latest`](Self::listen_include_latest) gives before any message was sent
/// (upstream's `default(T)`). A contract of `None` is upstream's `null`.
// Deviation (DEVIATIONS.md, ReactiveUI): the type of a message is `Default`, since a channel starts
// with the default value of its type (upstream's `default(T)`, `null` for a class); a message of a
// shared object is an `Rc<T>` of a `T: Default`, or an `Option<Rc<T>>`.
pub trait IMessageBusExt {
    /// Registers the scheduler that delivers the messages of `T` without a contract, replacing
    /// the one registered before (`RegisterScheduler<T>(IScheduler)`).
    fn register_scheduler<T: 'static>(&self, scheduler: Rc<dyn IScheduler>);

    /// Registers the scheduler that delivers the messages of `T` with `contract`
    /// (`RegisterScheduler<T>(IScheduler, string?)`).
    fn register_scheduler_with_contract<T: 'static>(&self, scheduler: Rc<dyn IScheduler>, contract: Option<&str>);

    /// The messages of `T` sent after the subscription (`Listen<T>()`). The channel is created
    /// when the observable is subscribed.
    fn listen<T: Clone + Default + 'static>(&self) -> Rc<dyn IObservable<T>>;

    /// The messages of `T` with `contract` sent after the subscription (`Listen<T>(string?)`).
    fn listen_with_contract<T: Clone + Default + 'static>(&self, contract: Option<&str>) -> Rc<dyn IObservable<T>>;

    /// The latest message of `T` (or its default value) at once, then the later ones
    /// (`ListenIncludeLatest<T>()`).
    fn listen_include_latest<T: Clone + Default + 'static>(&self) -> Rc<dyn IObservable<T>>;

    /// The latest message of `T` with `contract`, then the later ones
    /// (`ListenIncludeLatest<T>(string?)`).
    fn listen_include_latest_with_contract<T: Clone + Default + 'static>(&self, contract: Option<&str>) -> Rc<dyn IObservable<T>>;

    /// Sends each value of `source` as a message of `T`, until the returned disposable is
    /// disposed; an error or the end of `source` is logged and leaves the channel open
    /// (`RegisterMessageSource<T>(IObservable<T>)`).
    fn register_message_source<T: Clone + Default + 'static>(&self, source: Rc<dyn IObservable<T>>) -> Rc<dyn IDisposable>;

    /// [`register_message_source`](Self::register_message_source) on the channel of `contract`
    /// (`RegisterMessageSource<T>(IObservable<T>, string?)`).
    fn register_message_source_with_contract<T: Clone + Default + 'static>(
        &self,
        source: Rc<dyn IObservable<T>>,
        contract: Option<&str>,
    ) -> Rc<dyn IDisposable>;

    /// Sends one message of `T` (`SendMessage<T>(T)`). For messages that answer other changes,
    /// [`register_message_source`](Self::register_message_source) is the better fit.
    fn send_message<T: Clone + Default + 'static>(&self, message: T);

    /// Sends one message of `T` on the channel of `contract` (`SendMessage<T>(T, string?)`).
    fn send_message_with_contract<T: Clone + Default + 'static>(&self, message: T, contract: Option<&str>);
}

impl<B: IMessageBus + ?Sized + 'static> IMessageBusExt for Rc<B> {
    fn register_scheduler<T: 'static>(&self, scheduler: Rc<dyn IScheduler>) {
        self.register_scheduler_with_contract::<T>(scheduler, None);
    }

    fn register_scheduler_with_contract<T: 'static>(&self, scheduler: Rc<dyn IScheduler>, contract: Option<&str>) {
        self.register_scheduler_for(MessageKey::of::<T>(contract), scheduler);
    }

    fn listen<T: Clone + Default + 'static>(&self) -> Rc<dyn IObservable<T>> {
        self.listen_with_contract(None)
    }

    fn listen_with_contract<T: Clone + Default + 'static>(&self, contract: Option<&str>) -> Rc<dyn IObservable<T>> {
        Rc::new(ListenObservable::<B, T> { bus: self.clone(), contract: contract.map(str::to_owned), skip_first: true, _message: std::marker::PhantomData })
    }

    fn listen_include_latest<T: Clone + Default + 'static>(&self) -> Rc<dyn IObservable<T>> {
        self.listen_include_latest_with_contract(None)
    }

    fn listen_include_latest_with_contract<T: Clone + Default + 'static>(&self, contract: Option<&str>) -> Rc<dyn IObservable<T>> {
        Rc::new(ListenObservable::<B, T> { bus: self.clone(), contract: contract.map(str::to_owned), skip_first: false, _message: std::marker::PhantomData })
    }

    fn register_message_source<T: Clone + Default + 'static>(&self, source: Rc<dyn IObservable<T>>) -> Rc<dyn IDisposable> {
        self.register_message_source_with_contract(source, None)
    }

    fn register_message_source_with_contract<T: Clone + Default + 'static>(
        &self,
        source: Rc<dyn IObservable<T>>,
        contract: Option<&str>,
    ) -> Rc<dyn IDisposable> {
        let subject = setup_subject_if_necessary::<B, T>(self, contract);
        let key = MessageKey::of::<T>(contract);
        let completed_key = key.clone();
        source.subscribe(Observer::create_all(
            move |message: T| subject.on_next(message),
            move |error| {
                if let Some(logger) = logger_of::<B>() {
                    logger.warn_exception(&*error, &format!("MessageBus source for {}:{} terminated with an error.", key.type_name(), key.contract().unwrap_or("")));
                }
            },
            move || {
                if let Some(logger) = logger_of::<B>() {
                    logger.info(&format!("MessageBus source for {}:{} completed.", completed_key.type_name(), completed_key.contract().unwrap_or("")));
                }
            },
        ))
    }

    fn send_message<T: Clone + Default + 'static>(&self, message: T) {
        self.send_message_with_contract(message, None);
    }

    fn send_message_with_contract<T: Clone + Default + 'static>(&self, message: T, contract: Option<&str>) {
        setup_subject_if_necessary::<B, T>(self, contract).on_next(message);
    }
}

/// The logger of the bus's type, when the locator has a log manager (`this.Log()`).
fn logger_of<B: ?Sized>() -> Option<Rc<dyn IFullLogger>> {
    AppLocator::get_service::<dyn ILogManager>().map(|log_manager| log_manager.get_logger(std::any::type_name::<B>()))
}

/// The channel of the messages of `T` with `contract`, created with the default value of `T` as
/// its latest message (`SetupSubjectIfNecessary<T>`).
fn setup_subject_if_necessary<B: IMessageBus + ?Sized, T: Clone + Default + 'static>(bus: &B, contract: Option<&str>) -> Rc<ScheduledSubject<T>> {
    let key = MessageKey::of::<T>(contract);
    let subject = bus.message_subject(&key, &|scheduler| {
        let latest: Rc<dyn SubjectBase<T>> = BehaviorSubject::new(T::default());
        let subject: Rc<dyn Any> = ScheduledSubject::with_default_subject(scheduler, None, Some(latest));
        subject
    });
    match subject.downcast::<ScheduledSubject<T>>() {
        Ok(subject) => subject,
        Err(_) => panic!("The channel {key:?} of the message bus does not hold messages of its type"),
    }
}

/// Resolves the channel when an observer subscribes, and forwards it, without the replayed latest
/// message for `Listen` (`ListenObservable<T>`).
struct ListenObservable<B: ?Sized, T> {
    bus: Rc<B>,
    contract: Option<String>,
    skip_first: bool,
    _message: std::marker::PhantomData<fn() -> T>,
}

impl<B: IMessageBus + ?Sized + 'static, T: Clone + Default + 'static> IObservable<T> for ListenObservable<B, T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        if let Some(logger) = logger_of::<B>() {
            logger.info(&format!("Listening to {}:{}", std::any::type_name::<T>(), self.contract.as_deref().unwrap_or("")));
        }
        let source = setup_subject_if_necessary::<B, T>(&self.bus, self.contract.as_deref());
        if self.skip_first {
            source.subscribe(Rc::new(SkipFirstObserver::new(observer)))
        } else {
            source.subscribe(observer)
        }
    }
}
