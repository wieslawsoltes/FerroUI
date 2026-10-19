//! `IMessageBus` (`ReactiveUI.Shared/Interfaces/IMessageBus.cs`).

use ferroui_reactive::concurrency::IScheduler;
use ferroui_splat::IEnableLogger;
use std::any::{Any, TypeId};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// The channel of a message bus: the type of the messages and the contract that tells apart the
/// messages of one type (the key `(Type, string? Contract)` of upstream's `MessageBus`).
///
/// No contract (`None`) and the empty contract are two channels, as upstream's `null` and `""`.
#[derive(Clone)]
pub struct MessageKey {
    message_type: TypeId,
    type_name: &'static str,
    contract: Option<String>,
}

impl MessageKey {
    /// The channel of the messages of type `T` with `contract`.
    pub fn of<T: 'static>(contract: Option<&str>) -> Self {
        Self { message_type: TypeId::of::<T>(), type_name: std::any::type_name::<T>(), contract: contract.map(str::to_owned) }
    }

    /// The channel of the messages of the type `message_type` with `contract`; `type_name` is
    /// the name the logs show.
    pub fn new(message_type: TypeId, type_name: &'static str, contract: Option<&str>) -> Self {
        Self { message_type, type_name, contract: contract.map(str::to_owned) }
    }

    /// The type of the messages.
    pub fn message_type(&self) -> TypeId {
        self.message_type
    }

    /// The name of the type of the messages.
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// The contract of the channel.
    pub fn contract(&self) -> Option<&str> {
        self.contract.as_deref()
    }
}

impl PartialEq for MessageKey {
    fn eq(&self, other: &Self) -> bool {
        self.message_type == other.message_type && self.contract == other.contract
    }
}

impl Eq for MessageKey {}

impl Hash for MessageKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.message_type.hash(state);
        self.contract.hash(state);
    }
}

impl fmt::Debug for MessageKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.type_name, self.contract.as_deref().unwrap_or(""))
    }
}

/// A message bus: a simple way for view models and other objects to talk to each other without
/// knowing each other (`IMessageBus`).
///
/// Which messages go where is decided by the type of the message and a contract, a string that
/// tells apart the messages of the same type and that the client chooses.
///
/// The typed operations of upstream (`Listen<T>`, `SendMessage<T>`, `RegisterMessageSource<T>`,
/// `RegisterScheduler<T>`, ...) are the methods of [`IMessageBusExt`](crate::IMessageBusExt),
/// which every shared handle of a bus (`Rc<MessageBus>`, `Rc<dyn IMessageBus>`) has; they go
/// through the methods of this trait.
// Deviation (DEVIATIONS.md, ReactiveUI): upstream's methods are generic on the type of the
// message, which a trait object cannot have; this trait holds their type-erased core (the channel
// of a key and its scheduler), so that `MessageBus::current()` is an `Rc<dyn IMessageBus>` as
// upstream's `Current` is an `IMessageBus`, and `IMessageBusExt` adds the typed methods to it.
pub trait IMessageBus: IEnableLogger {
    /// Registers the scheduler that delivers the messages of a channel, replacing the one
    /// registered before (`RegisterScheduler<T>(IScheduler, string?)`).
    fn register_scheduler_for(&self, key: MessageKey, scheduler: Rc<dyn IScheduler>);

    /// Whether the channel of `message_type` and `contract` is registered: a message was sent on
    /// it, or something listened to it (`IsRegistered(Type, string?)`).
    fn is_registered_with_contract(&self, message_type: TypeId, contract: Option<&str>) -> bool;

    /// Whether the channel of `message_type` without a contract is registered
    /// (`IsRegistered(Type)`).
    fn is_registered(&self, message_type: TypeId) -> bool {
        self.is_registered_with_contract(message_type, None)
    }

    /// The subject of the messages of a channel, created with `create` the first time, given the
    /// scheduler registered for the channel (`SetupSubjectIfNecessary<T>`). The subject is an
    /// `Rc<ScheduledSubject<T>>` of the type of the messages, type-erased.
    fn message_subject(&self, key: &MessageKey, create: &dyn Fn(Rc<dyn IScheduler>) -> Rc<dyn Any>) -> Rc<dyn Any>;
}
