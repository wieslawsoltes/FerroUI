// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::controls::IRootDock;
use ferrodock_model::core::IDockable;
use ferrodock_model::BoxedValue;
use ferroui_reactiveui::{IRoutableViewModel, IScreen};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

pub(crate) fn identity<T: ?Sized>(reference: &Rc<T>) -> usize {
    Rc::as_ptr(reference) as *const () as usize
}
/// A typed service reference retained in its canonical erased representation.
#[derive(Clone)]
pub struct HostServiceValue {
    reference: BoxedValue,
    identity: usize,
    dockable: Option<Rc<dyn IDockable>>,
}
impl HostServiceValue {
    /// Retains the actual reference and original object identity, without cloning its object.
    pub fn new<T: ?Sized + 'static>(reference: Rc<T>) -> Self {
        Self {
            identity: identity(&reference),
            reference: Rc::new(reference),
            dockable: None,
        }
    }
    /// Queries the exact reference representation supplied by the resolver.
    pub fn get<T: ?Sized + 'static>(&self) -> Option<Rc<T>> {
        self.reference.downcast_ref::<Rc<T>>().cloned()
    }
}
type Query = Rc<dyn Fn() -> Option<HostServiceValue>>;
struct Entry {
    owner: Weak<dyn Any>,
    queries: HashMap<TypeId, Query>,
}
/// Instance-scoped native reference projections for the shared IScreen trait,
/// which has no interface-query hook. Registrations are weak and must describe
/// interfaces of the same Rc object. No global state or reflection is used.
#[derive(Default)]
pub struct HostTypeRegistry {
    entries: RefCell<HashMap<usize, Entry>>,
}
impl HostTypeRegistry {
    /// Creates an empty projection registry for a host service composition root.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
    /// Registers a screen and its exact concrete reference type.
    pub fn register_screen<T: IScreen + Any>(&self, instance: &Rc<T>) {
        self.register(instance);
        self.register_interface(instance, instance.clone() as Rc<dyn IScreen>);
    }
    /// Registers a dockable screen, including its existing root membership hook.
    pub fn register_dockable_screen<T: IScreen + IDockable + Any>(&self, instance: &Rc<T>) {
        self.register_screen(instance);
        let dockable: Rc<dyn IDockable> = instance.clone();
        self.register_interface(instance, dockable.clone());
        if let Some(root) = dockable.as_root_dock() {
            self.register_interface(instance, root);
        }
    }
    /// Registers a view model's actual shared ReactiveUI routing interface.
    pub fn register_routable<T: IRoutableViewModel + Any>(&self, instance: &Rc<T>) {
        self.register(instance);
        self.register_interface(instance, instance.clone() as Rc<dyn IRoutableViewModel>);
    }
    /// Registers the exact concrete reference type.
    pub fn register<T: Any>(&self, instance: &Rc<T>) {
        self.register_interface(instance, instance.clone());
    }
    /// Registers an interface projection of the very same object. Registering an
    /// unrelated service instance as membership is rejected rather than changing identity.
    pub fn register_interface<T: Any, I: ?Sized + 'static>(
        &self,
        instance: &Rc<T>,
        projection: Rc<I>,
    ) {
        let key = identity(instance);
        assert_eq!(
            key,
            identity(&projection),
            "an interface projection must retain the same object identity"
        );
        let owner: Rc<dyn Any> = instance.clone();
        let weak = Rc::downgrade(&projection);
        let query: Query = Rc::new(move || weak.upgrade().map(HostServiceValue::new));
        let mut entries = self.entries.borrow_mut();
        if entries
            .get(&key)
            .is_some_and(|entry| entry.owner.upgrade().is_none())
        {
            entries.remove(&key);
        }
        let entry = entries.entry(key).or_insert_with(|| Entry {
            owner: Rc::downgrade(&owner),
            queries: HashMap::new(),
        });
        entry.queries.insert(TypeId::of::<I>(), query);
    }
    /// Queries an explicitly registered interface of a canonical erased native
    /// reference, retaining the same object. This does not infer unregistered
    /// class/interface membership or introduce a separate runtime registry.
    pub fn reference<T: ?Sized + 'static>(&self, reference: &Rc<dyn Any>) -> Option<Rc<T>> {
        self.query_at(identity(reference), TypeId::of::<T>())?.get()
    }
    pub(crate) fn query_at(&self, key: usize, target: TypeId) -> Option<HostServiceValue> {
        let (alive, query) = {
            let entries = self.entries.borrow();
            let entry = entries.get(&key)?;
            (
                entry.owner.upgrade().is_some(),
                entry.queries.get(&target).cloned(),
            )
        };
        if !alive {
            self.entries.borrow_mut().remove(&key);
            return None;
        }
        query?()
    }
    pub(crate) fn query_value<T: ?Sized + 'static>(
        &self,
        value: &HostServiceValue,
    ) -> Option<Rc<T>> {
        value
            .get()
            .or_else(|| self.query_at(value.identity, TypeId::of::<T>())?.get())
            .or_else(|| {
                self.dockable_value(value.dockable.as_ref()?, TypeId::of::<T>())?
                    .get()
            })
    }
    pub(crate) fn screen_value(
        &self,
        screen: &Rc<dyn IScreen>,
        target: TypeId,
    ) -> Option<HostServiceValue> {
        if target == TypeId::of::<dyn IScreen>() {
            return Some(HostServiceValue::new(screen.clone()));
        }
        self.query_at(identity(screen), target).or_else(|| {
            let dockable = self
                .query_at(identity(screen), TypeId::of::<dyn IDockable>())?
                .get::<dyn IDockable>()?;
            self.dockable_value(&dockable, target)
        })
    }
    pub(crate) fn screen<T: ?Sized + 'static>(&self, screen: &Rc<dyn IScreen>) -> Option<Rc<T>> {
        self.screen_value(screen, TypeId::of::<T>())?.get()
    }
    pub(crate) fn dockable_value(
        &self,
        dockable: &Rc<dyn IDockable>,
        target: TypeId,
    ) -> Option<HostServiceValue> {
        if let Some(mut value) = self.query_at(identity(dockable), target) {
            value.dockable = Some(dockable.clone());
            return Some(value);
        }
        if target == TypeId::of::<dyn IDockable>() {
            return Some(HostServiceValue::new(dockable.clone()));
        }
        if target == TypeId::of::<dyn IRootDock>() {
            return dockable.clone().as_root_dock().map(HostServiceValue::new);
        }
        dockable
            .clone()
            .boxed_interface(target)
            .map(|reference| HostServiceValue {
                reference,
                identity: identity(dockable),
                dockable: Some(dockable.clone()),
            })
    }
    pub(crate) fn dockable<T: ?Sized + 'static>(
        &self,
        dockable: &Rc<dyn IDockable>,
    ) -> Option<Rc<T>> {
        self.dockable_value(dockable, TypeId::of::<T>())?.get()
    }
    pub(crate) fn routed_value(
        &self,
        routed: &Rc<dyn IRoutableViewModel>,
        target: TypeId,
    ) -> Option<HostServiceValue> {
        if target == TypeId::of::<dyn IRoutableViewModel>() {
            return Some(HostServiceValue::new(routed.clone()));
        }
        self.query_at(identity(routed), target).or_else(|| {
            let dockable = self
                .query_at(identity(routed), TypeId::of::<dyn IDockable>())?
                .get::<dyn IDockable>()?;
            self.dockable_value(&dockable, target)
        })
    }
    /// Removes expired native registrations. It retains no objects or service values.
    pub fn prune(&self) {
        self.entries
            .borrow_mut()
            .retain(|_, entry| entry.owner.upgrade().is_some());
    }
}
