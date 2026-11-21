//! Reflection-free source binding scopes and collection mirror on native services.
//! Scopes own subscriptions through Rust RAII; model callbacks are thread-affine.
use crate::controls::RibbonItemHelper;
use ferroribbon_core::{RibbonString, collections::RibbonList, model::RibbonItemsSource};
use ferroui_base::{FerroObject, FerroProperty, ObjectType, Ref, threading::Dispatcher};
use ferroui_system::{
    collections::{object_model::ObservableCollection, specialized::INotifyCollectionChanged},
    component_model::INotifyPropertyChanged,
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
};
pub type RibbonAction = Rc<dyn Fn()>;
pub type RibbonItemCreator<M, E> = Rc<dyn Fn(M, Rc<RibbonModelBindings>) -> E>;
static APP_DISPATCHER: Mutex<Option<Arc<Dispatcher>>> = Mutex::new(None);
struct Reset<'a>(&'a Cell<bool>);
impl Drop for Reset<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
struct Entry {
    subscribe: Option<RibbonAction>,
    unsubscribe: Option<RibbonAction>,
    refresh: Option<RibbonAction>,
    dispose: Option<RibbonAction>,
    attached: Cell<bool>,
}
impl Entry {
    fn subscribe(&self) {
        if !self.attached.replace(true)
            && let Some(f) = &self.subscribe
        {
            f();
        }
    }
    fn attach(&self) {
        if self.attached.get() && self.subscribe.is_some() {
            return;
        }
        self.attached.set(true);
        if let Some(f) = &self.subscribe {
            f();
        }
        if let Some(f) = &self.refresh {
            f();
        }
    }
    fn detach(&self) {
        if let Some(f) = &self.unsubscribe
            && self.attached.replace(false)
        {
            f();
        }
    }
    fn dispose(&self) {
        self.detach();
        if let Some(f) = &self.dispose {
            f();
        }
    }
}
/// Native binding owner. Explicit close and dropping the last owner both
/// release subscriptions. Weak callback endpoints/parent links avoid cycles;
/// callers keep this scope for the lifetime of the generated view bindings.
///
/// This owner does not implement a general disposal interface. Keep the `Rc`
/// for the lifetime of the view, or call [`Self::close`] for ordered teardown.
/// The source `Dispose` markup member invokes that same operation.
///
/// ```compile_fail,E0277
/// use ferroribbon::mvvm::RibbonModelBindings;
/// use ferroui_system::IDisposable;
/// use std::rc::Rc;
/// let _: Rc<dyn IDisposable> = RibbonModelBindings::new();
/// ```
pub struct RibbonModelBindings {
    entries: RefCell<Vec<Rc<Entry>>>,
    parent: RefCell<Option<Weak<Self>>>,
    dispatcher: RefCell<Option<Arc<Dispatcher>>>,
    disposed: Cell<bool>,
    suspended: Cell<bool>,
    recreate: RefCell<Option<RibbonAction>>,
    detach_from_parent: RefCell<Option<RibbonAction>>,
}
impl Drop for RibbonModelBindings {
    fn drop(&mut self) {
        self.close();
        // Initial application code may dispose before OneWay installs its entry.
        // Keep that source ordering while the scope is alive, then retire every
        // remaining registration when its last Rust owner goes away.
        let remaining = std::mem::take(self.entries.get_mut());
        for entry in remaining {
            entry.dispose();
        }
    }
}
impl PartialEq for RibbonModelBindings {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl RibbonModelBindings {
    pub fn new() -> Rc<Self> {
        Self::with_dispatcher(None)
    }
    pub fn with_dispatcher(dispatcher: Option<Arc<Dispatcher>>) -> Rc<Self> {
        Rc::new(Self {
            entries: RefCell::new(vec![]),
            parent: RefCell::new(None),
            dispatcher: RefCell::new(dispatcher),
            disposed: Cell::new(false),
            suspended: Cell::new(false),
            recreate: RefCell::new(None),
            detach_from_parent: RefCell::new(None),
        })
    }
    fn parent(&self) -> Option<Rc<Self>> {
        self.parent.borrow().as_ref().and_then(Weak::upgrade)
    }
    pub fn dispatcher(&self) -> Option<Arc<Dispatcher>> {
        self.dispatcher
            .borrow()
            .clone()
            .or_else(|| self.parent().and_then(|p| p.dispatcher()))
    }
    pub fn set_dispatcher(&self, value: Option<Arc<Dispatcher>>) {
        self.dispatcher.replace(value);
    }
    pub fn set_fallback_dispatcher(value: Option<Arc<Dispatcher>>) {
        *APP_DISPATCHER.lock().unwrap() = value;
    }
    pub fn is_disposed(&self) -> bool {
        self.disposed.get()
    }
    pub fn is_suspended(&self) -> bool {
        self.suspended.get() || self.parent().is_some_and(|p| p.is_suspended())
    }
    fn run(&self, action: RibbonAction) {
        if self.is_disposed() {
            return;
        }
        let dispatcher = self.dispatcher().or_else(|| {
            if Dispatcher::from_thread(std::thread::current().id()).is_none() {
                APP_DISPATCHER.lock().unwrap().clone()
            } else {
                None
            }
        });
        if dispatcher.is_some_and(|d| !d.check_access()) {
            panic!(
                "RibbonModelBindings cross-thread callbacks require a Send model/event bridge; selected FerroUI model events and actions are Rc/thread-affine"
            );
        }
        action();
    }
    fn add_entry(&self, entry: Rc<Entry>) {
        self.entries.borrow_mut().push(entry.clone());
        if !self.is_suspended() {
            entry.subscribe();
        }
    }
    pub fn add(&self, dispose: Option<RibbonAction>) {
        let dispose = dispose.expect("ArgumentNullException: dispose");
        if self.is_disposed() {
            dispose();
            return;
        }
        self.entries.borrow_mut().push(Rc::new(Entry {
            subscribe: None,
            unsubscribe: None,
            refresh: None,
            dispose: Some(dispose),
            attached: Cell::new(true),
        }));
    }
    pub fn one_way(
        self: &Rc<Self>,
        model: Option<Rc<dyn INotifyPropertyChanged>>,
        apply: Option<RibbonAction>,
        properties: Option<Vec<Option<RibbonString>>>,
    ) {
        self.property_binding(model, apply, properties, true);
    }
    pub fn watch(
        self: &Rc<Self>,
        model: Option<Rc<dyn INotifyPropertyChanged>>,
        on_changed: Option<RibbonAction>,
        properties: Option<Vec<Option<RibbonString>>>,
    ) {
        self.property_binding(model, on_changed, properties, false);
    }
    fn property_binding(
        self: &Rc<Self>,
        model: Option<Rc<dyn INotifyPropertyChanged>>,
        action: Option<RibbonAction>,
        properties: Option<Vec<Option<RibbonString>>>,
        immediate: bool,
    ) {
        let model = model.expect("ArgumentNullException: model");
        let action = action.expect(if immediate {
            "ArgumentNullException: apply"
        } else {
            "ArgumentNullException: onChanged"
        });
        if self.is_disposed() {
            return;
        }
        if immediate {
            action();
        }
        let owner = Rc::downgrade(self);
        let weak_action = Rc::downgrade(&action);
        let handler: Rc<dyn Fn(&str)> = Rc::new(move |name| {
            let properties = properties.as_ref().expect("NullReference: properties");
            if (properties.is_empty() || name.is_empty() || properties.contains(&Some(name.into())))
                && let (Some(owner), Some(action)) = (owner.upgrade(), weak_action.upgrade())
            {
                owner.run(action);
            }
        });
        let token = Rc::new(Cell::new(None));
        let t = token.clone();
        let m = model.clone();
        let subscribe = Rc::new(move || t.set(Some(m.property_changed().add(handler.clone()))));
        let unsubscribe = Rc::new(move || {
            if let Some(t) = token.take() {
                model.property_changed().remove(t);
            }
        });
        self.add_entry(Rc::new(Entry {
            subscribe: Some(subscribe),
            unsubscribe: Some(unsubscribe),
            refresh: Some(action),
            dispose: None,
            attached: Cell::new(false),
        }));
    }
    pub fn collection(
        self: &Rc<Self>,
        collection: Option<Rc<dyn INotifyCollectionChanged>>,
        on_changed: Option<RibbonAction>,
    ) {
        self.collection_binding(collection, on_changed, true);
    }
    pub fn watch_collection(
        self: &Rc<Self>,
        collection: Option<Rc<dyn INotifyCollectionChanged>>,
        on_changed: Option<RibbonAction>,
    ) {
        self.collection_binding(collection, on_changed, false);
    }
    fn collection_binding(
        self: &Rc<Self>,
        collection: Option<Rc<dyn INotifyCollectionChanged>>,
        action: Option<RibbonAction>,
        immediate: bool,
    ) {
        let collection = collection.expect("ArgumentNullException: collection");
        let action = action.expect("ArgumentNullException: onChanged");
        if self.is_disposed() {
            return;
        }
        if immediate {
            action();
        }
        let owner = Rc::downgrade(self);
        let weak_action = Rc::downgrade(&action);
        let handler = Rc::new(
            move |_: &ferroui_system::collections::specialized::CollectionChange| {
                if let (Some(o), Some(a)) = (owner.upgrade(), weak_action.upgrade()) {
                    o.run(a);
                }
            },
        );
        let token = Rc::new(Cell::new(None));
        let t = token.clone();
        let c = collection.clone();
        let subscribe = Rc::new(move || t.set(Some(c.collection_changed().add(handler.clone()))));
        let unsubscribe = Rc::new(move || {
            if let Some(t) = token.take() {
                collection.collection_changed().remove(t);
            }
        });
        self.add_entry(Rc::new(Entry {
            subscribe: Some(subscribe),
            unsubscribe: Some(unsubscribe),
            refresh: Some(action),
            dispose: None,
            attached: Cell::new(false),
        }));
    }
    pub fn two_way(
        self: &Rc<Self>,
        model: Option<Rc<dyn INotifyPropertyChanged>>,
        property: Option<RibbonString>,
        to_element: Option<RibbonAction>,
        element: Option<Ref<FerroObject>>,
        dp: Option<&'static FerroProperty>,
        to_model: Option<RibbonAction>,
    ) {
        let element = element.expect("ArgumentNullException: element");
        if self.is_disposed() {
            return;
        }
        let syncing = Rc::new(Cell::new(false));
        let s = syncing.clone();
        let action: RibbonAction = Rc::new(move || {
            if s.replace(true) {
                return;
            }
            let _finally = Reset(&s);
            to_element.as_ref().expect("NullReference: toElement")();
        });
        self.one_way(model, Some(action), Some(vec![property]));
        let dp = dp.expect("ArgumentNullException: dp");
        let owner = Rc::downgrade(self);
        let weak_to_model = to_model.as_ref().map(Rc::downgrade);
        let token = element.add_property_changed_handler(Rc::new(move |e| {
            if e.property().id() != dp.id() || !e.is_effective_value_change() {
                return;
            }
            let Some(owner) = owner.upgrade() else {
                return;
            };
            if syncing.get() || owner.is_suspended() || owner.is_disposed() {
                return;
            }
            syncing.set(true);
            let _finally = Reset(&syncing);
            weak_to_model
                .as_ref()
                .expect("NullReference: toModel")
                .upgrade()
                .expect("binding callback lost while scope is live")();
        }));
        self.add(Some(Rc::new(move || {
            let _keep_action_alive = &to_model;
            element.remove_property_changed_handler(token);
        })))
    }
    pub fn request_recreate(&self) {
        if !self.is_disposed() {
            let f = self.recreate.borrow().clone();
            if let Some(f) = f {
                f();
            }
        }
    }
    pub(super) fn set_recreate_handler(&self, value: Option<RibbonAction>) {
        self.recreate.replace(value);
    }
    pub fn child(self: &Rc<Self>) -> Rc<Self> {
        let child = Self::new();
        child.parent.replace(Some(Rc::downgrade(self)));
        if self.is_disposed() {
            child.disposed.set(true);
            return child;
        }
        let suspended = self.is_suspended();
        child.suspended.set(suspended);
        let c = child.clone();
        let c2 = child.clone();
        let c3 = child.clone();
        let entry = Rc::new(Entry {
            subscribe: Some(Rc::new(move || c.resume())),
            unsubscribe: Some(Rc::new(move || c2.suspend())),
            refresh: None,
            dispose: Some(Rc::new(move || c3.close())),
            attached: Cell::new(!suspended),
        });
        let weak_entry = Rc::downgrade(&entry);
        let parent = Rc::downgrade(self);
        child.detach_from_parent.replace(Some(Rc::new(move || {
            if let (Some(p), Some(e)) = (parent.upgrade(), weak_entry.upgrade()) {
                p.entries.borrow_mut().retain(|v| !Rc::ptr_eq(v, &e));
            }
        })));
        self.entries.borrow_mut().push(entry);
        child
    }
    pub fn suspend(&self) {
        if self.is_disposed() || self.suspended.replace(true) {
            return;
        }
        let entries = self.entries.borrow().clone();
        for entry in entries {
            entry.detach();
        }
    }
    pub fn resume(&self) {
        if self.is_disposed() || !self.suspended.replace(false) {
            return;
        }
        let entries = self.entries.borrow().clone();
        for entry in entries {
            if !self.is_disposed() && self.entries.borrow().iter().any(|e| Rc::ptr_eq(e, &entry)) {
                entry.attach();
            }
        }
    }
    /// Release this scope's bindings and child scopes in registration order.
    /// Repeated calls are harmless. Dropping the final `Rc` also closes the
    /// scope, including registrations installed after reentrant initial cleanup.
    pub fn close(&self) {
        if self.disposed.replace(true) {
            return;
        }
        let entries = std::mem::take(&mut *self.entries.borrow_mut());
        for entry in entries {
            entry.dispose();
        }
        let detach = self.detach_from_parent.borrow().clone();
        if let Some(detach) = detach {
            detach();
        }
        self.detach_from_parent.replace(None);
        self.parent.replace(None);
    }
    /// Source-named explicit cleanup. Prefer [`Self::close`] in new Rust code;
    /// this is the same scope operation, without an `IDisposable` protocol.
    pub fn dispose(&self) {
        self.close();
    }
    pub fn items<M: RibbonReference, E: RibbonReference>(
        self: &Rc<Self>,
        source: ObservableCollection<M>,
        target: Rc<dyn RibbonBindingList<E>>,
        create: RibbonItemCreator<M, E>,
        remove_on_dispose: bool,
        changing: Option<RibbonAction>,
        changed: Option<RibbonAction>,
    ) -> Rc<RibbonCollectionMirror<M, E>> {
        let mirror = Rc::new(RibbonCollectionMirror {
            owner: Rc::downgrade(self),
            source: source.clone(),
            target,
            create,
            remove_on_dispose,
            changing,
            changed,
            entries: RefCell::new(vec![]),
            disposed: Cell::new(false),
            reconciling: Cell::new(false),
        });
        if self.is_disposed() {
            return mirror;
        }
        mirror.reconcile();
        let owner = Rc::downgrade(self);
        let weak = Rc::downgrade(&mirror);
        let handler = Rc::new(
            move |_: &ferroui_system::collections::specialized::CollectionChange| {
                if let (Some(o), Some(m)) = (owner.upgrade(), weak.upgrade()) {
                    o.run(Rc::new(move || m.reconcile()));
                }
            },
        );
        let token = Rc::new(Cell::new(None));
        let t = token.clone();
        let c = source.clone();
        let subscribe = Rc::new(move || t.set(Some(c.collection_changed().add(handler.clone()))));
        let unsubscribe = Rc::new(move || {
            if let Some(t) = token.take() {
                source.collection_changed().remove(t);
            }
        });
        let m = mirror.clone();
        let m2 = mirror.clone();
        self.add_entry(Rc::new(Entry {
            subscribe: Some(subscribe),
            unsubscribe: Some(unsubscribe),
            refresh: Some(Rc::new(move || m.reconcile())),
            dispose: Some(Rc::new(move || m2.dispose())),
            attached: Cell::new(false),
        }));
        mirror
    }
    pub fn arrange_contiguous<E: RibbonReference>(
        target: &dyn RibbonBindingList<E>,
        elements: &[E],
    ) {
        if elements.is_empty() {
            return;
        }
        // DefaultIfEmpty(target.Count) evaluates Count before the lazy Select
        // begins enumeration, even when an existing index will be found.
        let fallback = target.count();
        let start = elements
            .iter()
            .filter_map(|e| target.index_of(e))
            .min()
            .unwrap_or(fallback);
        for (i, element) in elements.iter().enumerate() {
            let desired = (start + i).min(target.count());
            let current = target.index_of(element);
            if current == Some(desired) {
                continue;
            }
            if let Some(current) = current {
                if let Some(observable) = target.as_observable_collection() {
                    let last = target
                        .count()
                        .checked_sub(1)
                        .expect("ArgumentOutOfRange: newIndex");
                    observable.move_item(current, desired.min(last));
                } else {
                    target.remove_at(current);
                    target.insert(desired.min(target.count()), element.clone());
                }
            } else {
                target.insert(desired, element.clone());
            }
        }
    }
}
/// Source reference identity is distinct from downstream Equals used by IList.
pub trait RibbonReference: Clone + PartialEq + Default + 'static {
    fn same_reference(&self, other: &Self) -> bool;
    fn is_null(&self) -> bool;
    fn unlink_tree(&self) {}
}
impl<T: ?Sized + PartialEq + 'static> RibbonReference for Option<Rc<T>> {
    fn same_reference(&self, other: &Self) -> bool {
        match (self, other) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        }
    }
    fn is_null(&self) -> bool {
        self.is_none()
    }
}
impl<T: ObjectType> RibbonReference for Option<Ref<T>> {
    fn same_reference(&self, other: &Self) -> bool {
        match (self, other) {
            (Some(a), Some(b)) => a.ptr_eq(b),
            (None, None) => true,
            _ => false,
        }
    }
    fn is_null(&self) -> bool {
        self.is_none()
    }
    fn unlink_tree(&self) {
        RibbonItemHelper::unlink_tree(self.clone().map(|v| v.upcast()));
    }
}
/// Public source IList seam, including the distinct ObservableCollection Move branch.
pub trait RibbonBindingList<E>: 'static {
    fn count(&self) -> usize;
    fn index_of(&self, item: &E) -> Option<usize>;
    fn insert(&self, index: usize, item: E);
    fn remove_at(&self, index: usize);
    fn remove(&self, item: &E) -> bool;
    fn as_observable_collection(&self) -> Option<ObservableCollection<E>> {
        None
    }
}
impl<E: RibbonReference> RibbonBindingList<E> for ObservableCollection<E> {
    fn count(&self) -> usize {
        self.count()
    }
    fn index_of(&self, item: &E) -> Option<usize> {
        self.index_of(item)
    }
    fn insert(&self, index: usize, item: E) {
        self.insert(index, item);
    }
    fn remove_at(&self, index: usize) {
        self.remove_at(index);
    }
    fn remove(&self, item: &E) -> bool {
        self.remove(item)
    }
    fn as_observable_collection(&self) -> Option<ObservableCollection<E>> {
        Some(self.clone())
    }
}
impl<E: RibbonReference> RibbonBindingList<E> for RibbonList<E> {
    fn count(&self) -> usize {
        self.len()
    }
    fn index_of(&self, item: &E) -> Option<usize> {
        self.iter().position(|v| v == *item)
    }
    fn insert(&self, index: usize, item: E) {
        ferroribbon_core::collections::RibbonItems::insert(self.as_ref(), index, item);
    }
    fn remove_at(&self, index: usize) {
        ferroribbon_core::collections::RibbonItems::remove_at(self.as_ref(), index);
    }
    fn remove(&self, item: &E) -> bool {
        ferroribbon_core::collections::RibbonItems::remove(self.as_ref(), item)
    }
}
// LINQ ToArray uses the same empty array per closed element type.
ferroui_base::ferro_thread_local! {static EMPTY_ELEMENTS:RefCell<std::collections::HashMap<std::any::TypeId,Box<dyn std::any::Any>>>=RefCell::new(std::collections::HashMap::new());}
fn empty_elements<E: RibbonReference>() -> RibbonItemsSource<E> {
    EMPTY_ELEMENTS.with_borrow_mut(|values| {
        values
            .entry(std::any::TypeId::of::<E>())
            .or_insert_with(|| Box::new(RibbonItemsSource::<E>::from_values(vec![])))
            .downcast_ref::<RibbonItemsSource<E>>()
            .unwrap()
            .clone()
    })
}
struct MirrorEntry<M, E> {
    model: M,
    element: E,
    scope: Rc<RibbonModelBindings>,
}
impl<M: Clone, E: Clone> Clone for MirrorEntry<M, E> {
    fn clone(&self) -> Self {
        Self {
            model: self.model.clone(),
            element: self.element.clone(),
            scope: self.scope.clone(),
        }
    }
}
pub struct RibbonCollectionMirror<M: RibbonReference, E: RibbonReference> {
    owner: Weak<RibbonModelBindings>,
    source: ObservableCollection<M>,
    target: Rc<dyn RibbonBindingList<E>>,
    create: RibbonItemCreator<M, E>,
    remove_on_dispose: bool,
    changing: Option<RibbonAction>,
    changed: Option<RibbonAction>,
    entries: RefCell<Vec<MirrorEntry<M, E>>>,
    disposed: Cell<bool>,
    reconciling: Cell<bool>,
}
impl<M: RibbonReference, E: RibbonReference> RibbonCollectionMirror<M, E> {
    pub fn elements(&self) -> RibbonItemsSource<E> {
        let values = self
            .entries
            .borrow()
            .iter()
            .filter(|e| !e.element.is_null())
            .map(|e| e.element.clone())
            .collect::<Vec<_>>();
        if values.is_empty() {
            return empty_elements::<E>();
        }
        RibbonItemsSource::from_values(values)
    }
    pub fn find_element(&self, model: &M) -> E {
        self.entries
            .borrow()
            .iter()
            .find(|e| e.model.same_reference(model))
            .map(|e| e.element.clone())
            .unwrap_or_default()
    }
    pub fn reconcile(self: &Rc<Self>) {
        if self.disposed.get() || self.reconciling.replace(true) {
            return;
        }
        let _finally = Reset(&self.reconciling);
        if let Some(f) = &self.changing {
            f();
        }
        let mut previous = self.entries.borrow().clone();
        let mut next = Vec::with_capacity(self.source.count());
        for model in self.source.to_vec() {
            if let Some(index) = previous.iter().position(|e| e.model.same_reference(&model)) {
                next.push(previous.remove(index));
            } else {
                next.push(self.create(model));
            }
        }
        for old in previous {
            self.discard(old, true);
        }
        self.entries.replace(next);
        self.arrange();
        if let Some(f) = &self.changed {
            f();
        }
    }
    fn create(self: &Rc<Self>, model: M) -> MirrorEntry<M, E> {
        let owner = self.owner.upgrade().expect(
            "RibbonCollectionMirror owner has been released without disposal (CLR retention gap)",
        );
        let scope = owner.child();
        let weak = Rc::downgrade(self);
        let weak_scope = Rc::downgrade(&scope);
        let model2 = model.clone();
        scope.set_recreate_handler(Some(Rc::new(move || {
            if let (Some(m), Some(s)) = (weak.upgrade(), weak_scope.upgrade()) {
                let owner = m.owner.upgrade().expect("binding owner released");
                let model = model2.clone();
                owner.run(Rc::new(move || m.recreate(model.clone(), s.clone())));
            }
        })));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            (self.create)(model.clone(), scope.clone())
        }));
        match result {
            Ok(element) => MirrorEntry {
                model,
                element,
                scope,
            },
            Err(error) => {
                scope.dispose();
                std::panic::resume_unwind(error)
            }
        }
    }
    fn recreate(self: &Rc<Self>, model: M, scope: Rc<RibbonModelBindings>) {
        let index = self
            .entries
            .borrow()
            .iter()
            .position(|e| e.model.same_reference(&model) && Rc::ptr_eq(&e.scope, &scope));
        if self.disposed.get() || self.reconciling.get() || index.is_none() {
            return;
        }
        let index = index.unwrap();
        self.reconciling.set(true);
        let _finally = Reset(&self.reconciling);
        if let Some(f) = &self.changing {
            f();
        }
        let old = self.entries.borrow()[index].clone();
        let position = if old.element.is_null() {
            None
        } else {
            self.target.index_of(&old.element)
        };
        self.discard(old, true);
        let created = self.create(model);
        self.entries.borrow_mut()[index] = created.clone();
        if !created.element.is_null()
            && let Some(position) = position
        {
            self.target
                .insert(position.min(self.target.count()), created.element);
        }
        self.arrange();
        if let Some(f) = &self.changed {
            f();
        }
    }
    fn arrange(&self) {
        let values = self
            .entries
            .borrow()
            .iter()
            .filter(|e| !e.element.is_null())
            .map(|e| e.element.clone())
            .collect::<Vec<_>>();
        RibbonModelBindings::arrange_contiguous(&*self.target, &values);
    }
    fn discard(&self, entry: MirrorEntry<M, E>, remove: bool) {
        entry.scope.dispose();
        if entry.element.is_null() {
            return;
        }
        if remove {
            self.target.remove(&entry.element);
        }
        entry.element.unlink_tree();
    }
    pub fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        let entries = self.entries.borrow().clone();
        for e in entries {
            self.discard(e, self.remove_on_dispose);
        }
        self.entries.borrow_mut().clear();
    }
}
#[cfg(test)]
#[path = "model_bindings_tests.rs"]
mod tests;
