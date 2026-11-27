use super::{ICommand, RibbonCommandAction, RibbonCommandPredicate, RibbonRelayCommand};
use crate::{
    ObservableObject, ObservableObjectImpl, RibbonString, model::RibbonIcon,
    text::ordinal::OrdinalKey,
};
use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc};

/// The source descriptor is sealed. Id is immutable; nullable CLR string values remain lossless.
pub struct RibbonCommandDescriptor {
    base: ObservableObject,
    id: RibbonString,
    label: RefCell<Option<RibbonString>>,
    icon: RefCell<Option<Rc<RibbonIcon>>>,
    description: RefCell<Option<RibbonString>>,
    shortcut: RefCell<Option<RibbonString>>,
    category: RefCell<Option<RibbonString>>,
    keywords: Rc<RefCell<Vec<Option<RibbonString>>>>,
    is_enabled: RefCell<bool>,
    is_checked: RefCell<Option<bool>>,
    command: RefCell<Option<Rc<dyn ICommand>>>,
}
macro_rules! property {
    ($get:ident,$set:ident,$type:ty,$name:literal) => {
        pub fn $get(&self) -> $type {
            self.$get.borrow().to_owned()
        }
        pub fn $set(&self, value: $type) {
            ObservableObject::set_property(self, &self.$get, value, $name);
        }
    };
}
impl RibbonCommandDescriptor {
    pub fn new(
        id: Option<RibbonString>,
        label: Option<RibbonString>,
        command: Option<Rc<dyn ICommand>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: ObservableObject::construct(),
            id: id.expect("ArgumentNull: id"),
            label: RefCell::new(label),
            icon: RefCell::default(),
            description: RefCell::default(),
            shortcut: RefCell::default(),
            category: RefCell::default(),
            keywords: Rc::new(RefCell::new(Vec::new())),
            is_enabled: RefCell::new(true),
            is_checked: RefCell::default(),
            command: RefCell::new(command),
        })
    }
    pub fn id(&self) -> RibbonString {
        self.id.clone()
    }
    pub fn keywords(&self) -> Rc<RefCell<Vec<Option<RibbonString>>>> {
        self.keywords.clone()
    }
    property!(label, set_label, Option<RibbonString>, "Label");
    property!(icon, set_icon, Option<Rc<RibbonIcon>>, "Icon");
    property!(
        description,
        set_description,
        Option<RibbonString>,
        "Description"
    );
    property!(shortcut, set_shortcut, Option<RibbonString>, "Shortcut");
    property!(category, set_category, Option<RibbonString>, "Category");
    property!(is_enabled, set_is_enabled, bool, "IsEnabled");
    property!(is_checked, set_is_checked, Option<bool>, "IsChecked");
    pub fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.command.borrow().clone()
    }
    pub fn set_command(&self, value: Option<Rc<dyn ICommand>>) {
        let equal = match (&*self.command.borrow(), &value) {
            (None, None) => true,
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            _ => false,
        };
        if !equal {
            let old = self.command.replace(value);
            drop(old);
            self.on_property_changed("Command");
        }
    }
}
impl INotifyPropertyChanged for RibbonCommandDescriptor {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonCommandDescriptor {}
impl PartialEq for RibbonCommandDescriptor {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

pub struct RibbonCommandStateChangedEventArgs {
    id: Option<RibbonString>,
    property_name: Option<RibbonString>,
}
impl RibbonCommandStateChangedEventArgs {
    pub fn new(id: Option<RibbonString>, property_name: Option<RibbonString>) -> Rc<Self> {
        Rc::new(Self { id, property_name })
    }
    pub fn id(&self) -> Option<RibbonString> {
        self.id.clone()
    }
    pub fn property_name(&self) -> Option<RibbonString> {
        self.property_name.clone()
    }
}
pub struct RibbonCommandExecutedEventArgs {
    id: Option<RibbonString>,
    parameter: Option<Rc<dyn Any>>,
}
impl RibbonCommandExecutedEventArgs {
    pub fn new(id: Option<RibbonString>, parameter: Option<Rc<dyn Any>>) -> Rc<Self> {
        Rc::new(Self { id, parameter })
    }
    pub fn id(&self) -> Option<RibbonString> {
        self.id.clone()
    }
    pub fn parameter(&self) -> Option<Rc<dyn Any>> {
        self.parameter.clone()
    }
}
#[derive(Default)]
pub struct RibbonCommandMetadata {
    pub icon: Option<Rc<RibbonIcon>>,
    pub shortcut: Option<RibbonString>,
    pub description: Option<RibbonString>,
    pub category: Option<RibbonString>,
}
struct Registration {
    descriptor: Rc<RibbonCommandDescriptor>,
    subscription: Option<u64>,
}
#[derive(Default)]
struct Registry {
    commands: HashMap<OrdinalKey, Registration>,
    order: Vec<OrdinalKey>,
}
/// Source registration/invocation algorithm.
pub struct RibbonCommandCatalog {
    registry: RefCell<Registry>,
    commands_changed: Event<(Rc<Self>, RibbonString)>,
    command_state_changed: Event<(Rc<Self>, Rc<RibbonCommandStateChangedEventArgs>)>,
    executed: Event<(Rc<Self>, Rc<RibbonCommandExecutedEventArgs>)>,
}
// C# catalog equality is object identity, including when used by a native property.
impl PartialEq for RibbonCommandCatalog {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for RibbonCommandCatalog {}
impl RibbonCommandCatalog {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            registry: RefCell::default(),
            commands_changed: Event::new(),
            command_state_changed: Event::new(),
            executed: Event::new(),
        })
    }
    pub fn commands_changed(&self) -> &Event<(Rc<Self>, RibbonString)> {
        &self.commands_changed
    }
    pub fn command_state_changed(
        &self,
    ) -> &Event<(Rc<Self>, Rc<RibbonCommandStateChangedEventArgs>)> {
        &self.command_state_changed
    }
    pub fn executed(&self) -> &Event<(Rc<Self>, Rc<RibbonCommandExecutedEventArgs>)> {
        &self.executed
    }
    pub fn commands(&self) -> Vec<Rc<RibbonCommandDescriptor>> {
        let registry = self.registry.borrow();
        registry
            .order
            .iter()
            .map(|id| registry.commands[id].descriptor.clone())
            .collect()
    }
    pub fn register(
        self: &Rc<Self>,
        descriptor: Option<Rc<RibbonCommandDescriptor>>,
    ) -> Rc<RibbonCommandDescriptor> {
        let descriptor = descriptor.expect("ArgumentNull: descriptor");
        let id = descriptor.id();
        let key = OrdinalKey(id.clone());
        let previous = {
            let mut registry = self.registry.borrow_mut();
            if let Some(previous) = registry.commands.get(&key) {
                if let Some(subscription) = previous.subscription {
                    previous.descriptor.property_changed().remove(subscription);
                }
            } else {
                registry.order.push(key.clone());
            }
            let previous = registry.commands.insert(
                key.clone(),
                Registration {
                    descriptor: descriptor.clone(),
                    subscription: None,
                },
            );
            let owner = Rc::downgrade(self);
            let changed_id = id.clone();
            let subscription = descriptor.property_changed().add(Rc::new(move |property| {
                if let Some(owner) = owner.upgrade() {
                    let args = RibbonCommandStateChangedEventArgs::new(
                        Some(changed_id.clone()),
                        Some(property.into()),
                    );
                    owner.command_state_changed.raise(&(owner.clone(), args));
                }
            }));
            registry
                .commands
                .get_mut(&key)
                .expect("registered descriptor")
                .subscription = Some(subscription);
            previous
        };
        // No registry borrow crosses release of a replaced user-owned command.
        drop(previous);
        self.commands_changed.raise(&(self.clone(), id));
        descriptor
    }
    pub fn register_command(
        self: &Rc<Self>,
        id: Option<RibbonString>,
        label: Option<RibbonString>,
        command: Option<Rc<dyn ICommand>>,
        metadata: RibbonCommandMetadata,
    ) -> Rc<RibbonCommandDescriptor> {
        let descriptor = RibbonCommandDescriptor::new(id, label, command);
        descriptor.set_icon(metadata.icon);
        descriptor.set_shortcut(metadata.shortcut);
        descriptor.set_description(metadata.description);
        descriptor.set_category(metadata.category);
        self.register(Some(descriptor))
    }
    pub fn register_action(
        self: &Rc<Self>,
        id: Option<RibbonString>,
        label: Option<RibbonString>,
        execute: Rc<RibbonCommandAction>,
        can_execute: Option<Rc<RibbonCommandPredicate>>,
        metadata: RibbonCommandMetadata,
    ) -> Rc<RibbonCommandDescriptor> {
        self.register_command(
            id,
            label,
            Some(Rc::new(RibbonRelayCommand::construct(execute, can_execute))),
            metadata,
        )
    }
    pub fn unregister(self: &Rc<Self>, id: Option<&RibbonString>) -> bool {
        let id = id.expect("ArgumentNull: id");
        let key = OrdinalKey(id.clone());
        let removed = {
            let mut registry = self.registry.borrow_mut();
            let Some(removed) = registry.commands.remove(&key) else {
                return false;
            };
            registry.order.retain(|item| item != &key);
            if let Some(subscription) = removed.subscription {
                removed.descriptor.property_changed().remove(subscription);
            }
            removed
        };
        self.commands_changed.raise(&(self.clone(), id.clone()));
        drop(removed);
        true
    }
    pub fn find(&self, id: Option<&RibbonString>) -> Option<Rc<RibbonCommandDescriptor>> {
        id.and_then(|id| {
            self.registry
                .borrow()
                .commands
                .get(&OrdinalKey(id.clone()))
                .map(|entry| entry.descriptor.clone())
        })
    }
    pub fn try_resolve(&self, id: Option<&RibbonString>) -> Option<Rc<dyn ICommand>> {
        self.find(id).and_then(|d| d.command())
    }
    pub fn can_execute(&self, id: Option<&RibbonString>, parameter: Option<&dyn Any>) -> bool {
        let Some(descriptor) = self.find(id) else {
            return false;
        };
        descriptor.is_enabled()
            && descriptor
                .command()
                .is_some_and(|command| command.can_execute(parameter))
    }
    pub fn execute(
        self: &Rc<Self>,
        id: Option<&RibbonString>,
        parameter: Option<Rc<dyn Any>>,
    ) -> bool {
        if !self.can_execute(id, parameter.as_deref()) {
            return false;
        }
        // Intentionally resolve again after user CanExecute code, as in the source.
        let descriptor = self
            .find(id)
            .expect("NullReference: descriptor removed during CanExecute");
        let command = descriptor
            .command()
            .expect("NullReference: command cleared during CanExecute");
        command.execute(parameter.as_deref());
        let args = RibbonCommandExecutedEventArgs::new(
            Some(id.expect("successful id").clone()),
            parameter,
        );
        self.executed.raise(&(self.clone(), args));
        true
    }
    pub fn set_enabled(&self, id: Option<&RibbonString>, enabled: bool) {
        if let Some(d) = self.find(id) {
            d.set_is_enabled(enabled);
        }
    }
    pub fn set_checked(&self, id: Option<&RibbonString>, checked: Option<bool>) {
        if let Some(d) = self.find(id) {
            d.set_is_checked(checked);
        }
    }
}
impl Drop for RibbonCommandCatalog {
    fn drop(&mut self) {
        for entry in self.registry.get_mut().commands.values() {
            if let Some(subscription) = entry.subscription {
                entry.descriptor.property_changed().remove(subscription);
            }
        }
    }
}
