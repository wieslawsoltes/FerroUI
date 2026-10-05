use super::{
    AccessorListener, AsNotifyPropertyChanged, IPropertyAccessor, IPropertyAccessorPlugin, PropertyAccessorBase,
};
use crate::data::core::{ValueType, ValueTypes, WeakValue};
use crate::data::model::{Event, ICommand};
use crate::data::{BindingError, BindingErrorType, BindingNotification, BindingPriority};
use crate::reactive::{Disposable, IDisposable};
use crate::threading::{Dispatcher, DispatcherPriority};
use crate::{AnyValue, BoxedValue};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Reads a member from its owner, both untyped. Null is `None`; an `Err` is
/// the equivalent of the getter throwing.
pub type UntypedGetter = Rc<dyn Fn(&BoxedValue) -> Result<Option<BoxedValue>, BindingError>>;

/// Writes a member on its owner, both untyped. An `Err` is the equivalent of
/// the setter throwing.
pub type UntypedSetter = Rc<dyn Fn(&BoxedValue, Option<&BoxedValue>) -> Result<(), BindingError>>;


/// Executes a method of an untyped owner with a command parameter.
pub type UntypedExecute = Rc<dyn Fn(&BoxedValue, Option<&BoxedValue>)>;

/// Asks an untyped owner whether a method can execute with a command
/// parameter.
pub type UntypedCanExecute = Rc<dyn Fn(&BoxedValue, Option<&BoxedValue>) -> bool>;

/// Creates the delegate value of a method for an untyped owner: a value
/// (usually a [`MarkupDelegate`](crate::metadata::MarkupDelegate)) that
/// invokes the method on the owner.
pub type UntypedCreateDelegate = Rc<dyn Fn(&BoxedValue) -> BoxedValue>;

/// What an [`UntypedAccessorPlugin`] reads from its owner.
#[derive(Clone)]
pub(crate) enum UntypedMember {
    /// A property.
    Property {
        property_type: ValueType,
        get: UntypedGetter,
        set: Option<UntypedSetter>,
        notifier: Option<AsNotifyPropertyChanged>,
    },
    /// A method as a command, re-queried when one of the properties it
    /// depends on changes.
    Command {
        execute: UntypedExecute,
        can_execute: Option<UntypedCanExecute>,
        depends_on_properties: Rc<[Box<str>]>,
        notifier: Option<AsNotifyPropertyChanged>,
    },
    /// A method as a delegate value bound to the owner.
    Method { create_delegate: UntypedCreateDelegate },
}

/// The plugin of a compiled binding path element whose accessors are
/// closures over untyped values: what markup compiles a member access to
/// when the member has no typed description.
pub(crate) struct UntypedAccessorPlugin {
    name: Box<str>,
    member: UntypedMember,
}

impl UntypedAccessorPlugin {
    pub(crate) fn new(name: &str, member: UntypedMember) -> Self {
        Self { name: name.into(), member }
    }
}

impl IPropertyAccessorPlugin for UntypedAccessorPlugin {
    fn match_(&self, _obj: &dyn AnyValue, _property_name: &str) -> bool {
        panic!("The UntypedAccessorPlugin does not support dynamic matching");
    }

    fn start(&self, reference: &WeakValue, property_name: &str) -> Option<Rc<dyn IPropertyAccessor>> {
        debug_assert!(property_name == &*self.name);
        Some(Rc::new_cyclic(|this| UntypedAccessor {
            this: this.clone(),
            base: PropertyAccessorBase::new(),
            reference: reference.clone(),
            name: self.name.clone(),
            member: self.member.clone(),
            command: RefCell::new(None),
            token: Cell::new(None),
            event_raised: Cell::new(false),
        }))
    }
}

struct UntypedAccessor {
    this: Weak<UntypedAccessor>,
    base: PropertyAccessorBase,
    reference: WeakValue,
    name: Box<str>,
    member: UntypedMember,
    command: RefCell<Option<Rc<UntypedCommand>>>,
    token: Cell<Option<u64>>,
    event_raised: Cell<bool>,
}

impl UntypedAccessor {
    fn current_value(&self) -> Option<BoxedValue> {
        match &self.member {
            UntypedMember::Property { get, .. } => {
                let owner = self.reference.upgrade()?;
                match get(&owner) {
                    Ok(value) => value,
                    // A failing getter is published as a binding error.
                    Err(e) => Some(Rc::new(BindingNotification::with_error(e, BindingErrorType::Error)) as BoxedValue),
                }
            }
            UntypedMember::Command { .. } => self.command.borrow().as_ref().map(|c| {
                let command: Rc<dyn ICommand> = c.clone();
                Rc::new(command) as BoxedValue
            }),
            UntypedMember::Method { create_delegate } => {
                // The delegate is bound to the owner.
                let owner = self.reference.upgrade()?;
                Some(create_delegate(&owner))
            }
        }
    }

    fn send_current_value(&self) {
        let value = self.current_value();
        self.base.publish_value(value);
    }

    fn on_property_changed(&self, name: &str) {
        match &self.member {
            UntypedMember::Property { .. } => {
                if name.is_empty() || name == &*self.name {
                    self.event_raised.set(true);
                    self.send_current_value();
                }
            }
            UntypedMember::Command { depends_on_properties, .. } => {
                if name.is_empty() || depends_on_properties.iter().any(|d| &**d == name) {
                    let command = self.command.borrow().clone();
                    if let Some(command) = command {
                        command.raise_can_execute_changed();
                    }
                }
            }
            UntypedMember::Method { .. } => {}
        }
    }

    fn notifier(&self) -> Option<AsNotifyPropertyChanged> {
        match &self.member {
            UntypedMember::Property { notifier, .. } | UntypedMember::Command { notifier, .. } => *notifier,
            UntypedMember::Method { .. } => None,
        }
    }

    fn remove_handler(&self) {
        if let Some(token) = self.token.take() {
            if let (Some(owner), Some(notifier)) = (self.reference.upgrade(), self.notifier()) {
                if let Some(inpc) = notifier(&*owner) {
                    inpc.property_changed().remove(token);
                }
            }
        }
    }
}

impl IDisposable for UntypedAccessor {
    fn dispose(&self) {
        if self.base.is_subscribed() {
            self.unsubscribe();
        }
    }
}

impl IPropertyAccessor for UntypedAccessor {
    fn property_type(&self) -> Option<ValueType> {
        Some(match &self.member {
            UntypedMember::Property { property_type, .. } => *property_type,
            UntypedMember::Command { .. } => ValueType::of::<Rc<dyn ICommand>>(),
            UntypedMember::Method { .. } => ValueType::object(),
        })
    }

    fn value(&self) -> Option<BoxedValue> {
        match self.current_value() {
            // The value read directly is the plain value: an error of the
            // getter reads as null.
            Some(value) if value.is::<BindingNotification>() => None,
            value => value,
        }
    }

    fn set_value(&self, value: Option<&BoxedValue>, _priority: BindingPriority) -> Result<bool, BindingError> {
        let UntypedMember::Property { property_type, set: Some(set), .. } = &self.member else {
            return Ok(false);
        };
        let converted = match ValueTypes::try_convert(value, *property_type) {
            Some(v) => v,
            None if property_type.is_object() => value.cloned(),
            None => {
                return Err(BindingError::message(format!(
                    "Object of type '{}' cannot be converted to type '{}'.",
                    value.map_or("null", |v| (**v).type_name()),
                    property_type
                )))
            }
        };
        self.event_raised.set(false);
        if let Some(owner) = self.reference.upgrade() {
            set(&owner, converted.as_ref())?;
        }
        if !self.event_raised.get() {
            self.send_current_value();
        }
        Ok(true)
    }

    fn subscribe(&self, listener: AccessorListener) {
        self.base.set_listener(listener);

        if let UntypedMember::Command { execute, can_execute, .. } = &self.member {
            *self.command.borrow_mut() =
                Some(UntypedCommand::new(self.reference.clone(), execute.clone(), can_execute.clone()));
        }

        if let (Some(notifier), Some(owner)) = (self.notifier(), self.reference.upgrade()) {
            if let Some(inpc) = notifier(&*owner) {
                let weak = self.this.clone();
                let token = inpc.property_changed().add(Rc::new(move |name: &str| {
                    if let Some(this) = weak.upgrade() {
                        this.on_property_changed(name);
                    }
                }));
                self.token.set(Some(token));
            }
        }

        self.send_current_value();
    }

    fn unsubscribe(&self) {
        self.remove_handler();
        self.command.borrow_mut().take();
        self.base.clear_listener();
    }
}

impl Drop for UntypedAccessor {
    fn drop(&mut self) {
        self.remove_handler();
    }
}

/// The command created for a method of an untyped binding source. It holds
/// the source weakly.
struct UntypedCommand {
    this: Weak<UntypedCommand>,
    target: WeakValue,
    execute: UntypedExecute,
    can_execute: Option<UntypedCanExecute>,
    can_execute_changed: Event<()>,
}

impl UntypedCommand {
    fn new(target: WeakValue, execute: UntypedExecute, can_execute: Option<UntypedCanExecute>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            target,
            execute,
            can_execute,
            can_execute_changed: Event::new(),
        })
    }

    /// Raises the "can execute changed" event, at input priority on the
    /// dispatcher.
    fn raise_can_execute_changed(&self) {
        let weak = self.this.clone();
        Dispatcher::current_dispatcher().post_local(
            move || {
                if let Some(this) = weak.upgrade() {
                    this.can_execute_changed.raise(&());
                }
            },
            DispatcherPriority::INPUT,
        );
    }
}

impl ICommand for UntypedCommand {
    fn can_execute(&self, parameter: Option<&BoxedValue>) -> bool {
        match self.target.upgrade() {
            Some(target) => match &self.can_execute {
                Some(can_execute) => can_execute(&target, parameter),
                None => true,
            },
            None => false,
        }
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        if let Some(target) = self.target.upgrade() {
            (self.execute)(&target, parameter);
        }
    }

    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.can_execute_changed.add(Rc::new(move |_: &()| handler()));
        let weak = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.can_execute_changed.remove(token);
            }
        })
    }
}
