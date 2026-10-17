//! Accessors: what the reference reads from `Expression<Func<TModel, TValue>>`, without expression
//! trees. A column's value is read by a getter, written by an optional setter, named for change
//! notifications, and observed along the objects of its path (`Utils/PropertyPathObserver.cs`).
//!
//! Not a file of the reference: C# compiles the expression into a getter, synthesizes the setter
//! from it (`TreeDataGridExpressionHelper.TryCreateSetter`, the `Expression.Assign` of
//! `HierarchicalExpanderColumn`), reads the member name (`TryGetMemberName`) and walks the member
//! accesses for the owners to observe. Rust has no expression trees, so an accessor states each of
//! those parts.

use ferroui_base::data::model::{INotifyPropertyChanged, ModelTypes};
use ferroui_base::{AnyValue, BoxedValue};
use std::rc::Rc;

/// A model type of a source: compared and boxed as the framework's values are.
pub trait TreeModel: PartialEq + 'static {}

impl<T: PartialEq + 'static> TreeModel for T {}

/// The notification interface of a value, through the model type metadata of the framework (the
/// stand-in of the reference's `is INotifyPropertyChanged`).
pub fn as_notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    ModelTypes::find(value).and_then(|model_type| model_type.as_notify_property_changed(value))
}

/// An object on the path of an accessor whose changes are observed: the owner of a member the
/// accessor reads below the model (`x => x.Expansion.IsExpanded`: the `Expansion` of the model,
/// member `IsExpanded`).
pub struct PathLink<M: 'static> {
    /// The owner, or `None` when an object on the way to it is null.
    pub owner: Rc<dyn Fn(&Rc<M>) -> Option<BoxedValue>>,
    /// The name of the member read from the owner.
    pub property: &'static str,
}

impl<M: 'static> Clone for PathLink<M> {
    fn clone(&self) -> Self {
        Self { owner: self.owner.clone(), property: self.property }
    }
}

/// The getter, setter, member name and observed path of a value of a model.
pub struct Accessor<M: 'static, V: 'static> {
    getter: Rc<dyn Fn(&Rc<M>) -> V>,
    setter: Option<Rc<dyn Fn(&Rc<M>, V)>>,
    property_name: Option<&'static str>,
    links: Vec<PathLink<M>>,
}

impl<M: 'static, V: 'static> Clone for Accessor<M, V> {
    fn clone(&self) -> Self {
        Self {
            getter: self.getter.clone(),
            setter: self.setter.clone(),
            property_name: self.property_name,
            links: self.links.clone(),
        }
    }
}

impl<M: 'static, V: 'static> Accessor<M, V> {
    /// An accessor that reads with `getter` (a delegate column of the reference: no member name,
    /// only the model itself is observed).
    pub fn new(getter: impl Fn(&Rc<M>) -> V + 'static) -> Self {
        Self { getter: Rc::new(getter), setter: None, property_name: None, links: Vec::new() }
    }

    /// An accessor of the member `property` of the model (`x => x.Name`): the name filters the
    /// model's change notifications.
    pub fn member(property: &'static str, getter: impl Fn(&Rc<M>) -> V + 'static) -> Self {
        Self::new(getter).named(property)
    }

    /// The member name.
    pub fn named(mut self, property: &'static str) -> Self {
        self.property_name = Some(property);
        self
    }

    /// The setter (the assignment the reference synthesizes for a writable member).
    pub fn with_setter(mut self, setter: impl Fn(&Rc<M>, V) + 'static) -> Self {
        self.setter = Some(Rc::new(setter));
        self
    }

    /// An object below the model whose member `property` the accessor reads, observed for changes
    /// (`x => x.Expansion.IsExpanded`: `.through(|x| Some(x.expansion() as BoxedValue), "IsExpanded")`).
    pub fn through(mut self, owner: impl Fn(&Rc<M>) -> Option<BoxedValue> + 'static, property: &'static str) -> Self {
        self.links.push(PathLink { owner: Rc::new(owner), property });
        self
    }

    /// Reads the value.
    pub fn get(&self, model: &Rc<M>) -> V {
        (self.getter)(model)
    }

    /// The getter.
    pub fn getter(&self) -> &Rc<dyn Fn(&Rc<M>) -> V> {
        &self.getter
    }

    /// The setter, if the value can be written.
    pub fn setter(&self) -> Option<&Rc<dyn Fn(&Rc<M>, V)>> {
        self.setter.as_ref()
    }

    /// Writes the value; false when the accessor has no setter.
    pub fn set(&self, model: &Rc<M>, value: V) -> bool {
        match &self.setter {
            Some(setter) => {
                setter(model, value);
                true
            }
            None => false,
        }
    }

    /// The member name (`TryGetMemberName`).
    pub fn property_name(&self) -> Option<&'static str> {
        self.property_name
    }

    /// The objects below the model on the accessor's path.
    pub fn links(&self) -> &[PathLink<M>] {
        &self.links
    }

    /// The accessor with its value mapped (`Expression.Convert`).
    pub fn map<W: 'static>(self, to: impl Fn(V) -> W + 'static, from: impl Fn(W) -> V + 'static) -> Accessor<M, W> {
        let getter = self.getter.clone();
        let setter = self.setter.clone().map(|setter| {
            let from = Rc::new(from);
            Rc::new(move |m: &Rc<M>, w: W| setter(m, from(w))) as Rc<dyn Fn(&Rc<M>, W)>
        });
        Accessor {
            getter: Rc::new(move |m: &Rc<M>| to(getter(m))),
            setter,
            property_name: self.property_name,
            links: self.links,
        }
    }
}

/// `PropertyPathObserver<TModel>`: the owners on the path of an accessor, compiled once per column;
/// each row owns only its subscriptions.
pub struct PropertyPathObserver<M: 'static> {
    // (owner, member); the first is the model itself with no member, so that a computed value
    // whose dependencies cannot be inferred is refreshed by any change of the model.
    paths: Vec<PathLink<M>>,
}

impl<M: TreeModel> PropertyPathObserver<M> {
    /// The observer of `accessor`'s path, or `None` when the accessor reads only members of the
    /// model, which the row's own subscription to the model covers (`Create`).
    pub fn create<V: 'static>(accessor: &Accessor<M, V>) -> Option<Rc<Self>> {
        if accessor.links().is_empty() {
            return None;
        }
        let mut paths = vec![PathLink { owner: Rc::new(|m: &Rc<M>| Some(m.clone() as BoxedValue)), property: "" }];
        paths.extend(accessor.links().iter().cloned());
        Some(Rc::new(Self { paths }))
    }

    /// Observes the path of `model`; `changed` runs when a member on it changes. The subscriptions
    /// end when the returned value is dropped or disposed.
    pub fn subscribe(self: &Rc<Self>, model: &Rc<M>, changed: Rc<dyn Fn()>) -> PathSubscription<M> {
        let state = Rc::new(SubscriptionState {
            path: self.clone(),
            model: model.clone(),
            changed,
            owners: std::cell::RefCell::new(vec![None; self.paths.len()]),
            disposed: std::cell::Cell::new(false),
        });
        SubscriptionState::refresh(&state);
        PathSubscription { state }
    }
}

struct Owner {
    value: BoxedValue,
    token: Option<u64>,
}

impl Clone for Owner {
    fn clone(&self) -> Self {
        Self { value: self.value.clone(), token: self.token }
    }
}

struct SubscriptionState<M: 'static> {
    path: Rc<PropertyPathObserver<M>>,
    model: Rc<M>,
    changed: Rc<dyn Fn()>,
    owners: std::cell::RefCell<Vec<Option<Owner>>>,
    disposed: std::cell::Cell<bool>,
}

fn same(a: &BoxedValue, b: &BoxedValue) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

impl<M: TreeModel> SubscriptionState<M> {
    fn refresh(this: &Rc<Self>) {
        let count = this.path.paths.len();
        for i in 0..count {
            let next = (this.path.paths[i].owner)(&this.model);
            let old = this.owners.borrow()[i].clone();
            match (&old, &next) {
                (Some(old), Some(next)) if same(&old.value, next) => continue,
                (None, None) => continue,
                _ => {}
            }
            this.owners.borrow_mut()[i] = None;
            if let Some(old) = &old {
                if !Self::contains(this, &old.value) {
                    Self::unsubscribe(this, old);
                }
            }
            let next = next.map(|value| {
                // An owner already on the path keeps its one subscription.
                let existing = this.owners.borrow().iter().flatten().find(|o| same(&o.value, &value)).map(|o| o.token);
                match existing {
                    Some(token) => Owner { value, token },
                    None => {
                        let token = Self::subscribe_owner(this, &value);
                        Owner { value, token }
                    }
                }
            });
            this.owners.borrow_mut()[i] = next;
        }
    }

    fn contains(this: &Rc<Self>, owner: &BoxedValue) -> bool {
        this.owners.borrow().iter().flatten().any(|o| same(&o.value, owner))
    }

    fn subscribe_owner(this: &Rc<Self>, owner: &BoxedValue) -> Option<u64> {
        let notifier = as_notifier(&**owner)?;
        let weak = Rc::downgrade(this);
        let key = owner.clone();
        Some(notifier.property_changed().add(Rc::new(move |name: &str| {
            if let Some(this) = weak.upgrade() {
                Self::on_property_changed(&this, &key, name);
            }
        })))
    }

    fn unsubscribe(_this: &Rc<Self>, owner: &Owner) {
        if let (Some(token), Some(notifier)) = (owner.token, as_notifier(&*owner.value)) {
            notifier.property_changed().remove(token);
        }
    }

    fn on_property_changed(this: &Rc<Self>, sender: &BoxedValue, name: &str) {
        if this.disposed.get() {
            return;
        }
        let count = this.path.paths.len();
        for i in 0..count {
            let matches = {
                let owners = this.owners.borrow();
                owners[i].as_ref().is_some_and(|o| same(&o.value, sender))
            };
            let property = this.path.paths[i].property;
            if matches && (name.is_empty() || property.is_empty() || name == property) {
                Self::refresh(this);
                (this.changed)();
                break;
            }
        }
    }

    fn dispose(this: &Rc<Self>) {
        if this.disposed.replace(true) {
            return;
        }
        let count = this.path.paths.len();
        for i in 0..count {
            let old = this.owners.borrow_mut()[i].take();
            if let Some(old) = old {
                if !Self::contains(this, &old.value) {
                    Self::unsubscribe(this, &old);
                }
            }
        }
    }
}

/// The subscriptions of one model to the path of a [`PropertyPathObserver`].
pub struct PathSubscription<M: TreeModel> {
    state: Rc<SubscriptionState<M>>,
}

impl<M: TreeModel> PathSubscription<M> {
    /// Ends the subscriptions.
    pub fn dispose(&self) {
        SubscriptionState::dispose(&self.state);
    }
}

impl<M: TreeModel> Drop for PathSubscription<M> {
    fn drop(&mut self) {
        SubscriptionState::dispose(&self.state);
    }
}
