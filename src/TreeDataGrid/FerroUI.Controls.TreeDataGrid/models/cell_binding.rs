//! The bindings of cells to the values of their models: the stand-in of the reference's typed
//! binding expressions (`ISubject<BindingValue<T>>`, `TypedBindingExpression<TIn, TOut>` and
//! `IRetargetableTypedBindingExpression` of its vendored `Experimental/` engine), which are not
//! ported.
//!
//! A cell binding reads its value with the getter of an [`Accessor`], writes it with the
//! accessor's setter, and follows the model's `PropertyChanged` for the accessor's member name, or
//! the objects of the accessor's path through the core's [`PropertyPathObserver`].

use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::utilities::HandlerList;
use ferroui_base::BoxedValue;
use ferroui_treedatagrid_core::accessor::{as_notifier, TreeModel};
use ferroui_treedatagrid_core::{Accessor, PathSubscription, PropertyPathObserver};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The binding of a cell to its value (`ISubject<BindingValue<T>>`): an observable of the value
/// that the cell writes back to.
pub trait ICellBinding<T: 'static> {
    /// Subscribes `observer`: it receives the current value now, if there is one, and every later
    /// value (`Subscribe`). Returns the token of [`unsubscribe`](Self::unsubscribe). An observer
    /// receives a value of its own, which it keeps without copying it.
    fn subscribe(&self, observer: Rc<dyn Fn(T)>) -> u64;
    /// Ends a subscription (the `IDisposable` that `Subscribe` returns).
    fn unsubscribe(&self, token: u64);
    /// Writes a value to the source (`OnNext`).
    fn on_next(&self, value: T);
    /// Whether the binding writes its values back (`Write is not null`).
    fn can_write(&self) -> bool;
    /// The binding as one whose root can be replaced (`is IRetargetableTypedBindingExpression`).
    fn as_retargetable(&self) -> Option<&dyn IRetargetableCellBinding> {
        None
    }
}

/// A binding whose root object can be replaced, so that a cell can be retargeted to another row
/// (`IRetargetableTypedBindingExpression`).
pub trait IRetargetableCellBinding {
    /// Points the binding at `root`; false when `root` is not a model of the binding's type.
    fn try_set_root(&self, root: Option<&BoxedValue>) -> bool;
    /// Releases the root and the subscriptions of the binding, keeping what a later
    /// [`try_set_root`](Self::try_set_root) needs.
    fn try_suspend_root(&self) -> bool;
}

/// The notification interface of a model (`model as INotifyPropertyChanged`). A model that is
/// itself an untyped value (the models of the declarative columns, whose model type is
/// `BoxedValue`) is looked up by the value it holds.
pub fn model_notifier<M: TreeModel>(model: &Rc<M>) -> Option<&dyn INotifyPropertyChanged> {
    let any: &dyn Any = &**model;
    if let Some(boxed) = any.downcast_ref::<BoxedValue>() {
        return as_notifier(&**boxed);
    }
    as_notifier(&**model)
}

enum Listening<M: TreeModel> {
    // The model's `PropertyChanged`.
    Model { model: Rc<M>, token: u64 },
    // The objects of the accessor's path.
    Path(PathSubscription<M>),
}

/// A binding of the value an [`Accessor`] reads from a model: the reference's
/// `TypedBindingExpression<TModel, TValue>` instantiated for a cell (`InstanceForCell`).
pub struct AccessorBinding<M: TreeModel, V: Clone + 'static> {
    weak_self: Weak<Self>,
    accessor: Accessor<M, V>,
    observer: Option<Rc<PropertyPathObserver<M>>>,
    root: RefCell<Option<Rc<M>>>,
    observers: HandlerList<dyn Fn(V)>,
    listening: RefCell<Option<Listening<M>>>,
    publish_count: Cell<u32>,
}

impl<M: TreeModel, V: Clone + 'static> AccessorBinding<M, V> {
    /// A binding of `accessor` on `root`.
    pub fn new(accessor: Accessor<M, V>, root: Option<Rc<M>>) -> Rc<Self> {
        let observer = PropertyPathObserver::create(&accessor);
        Self::with_observer(accessor, observer, root)
    }

    /// A binding of `accessor` on `root` that follows the path with `observer`, compiled once
    /// for a column and shared by its cells.
    pub fn with_observer(
        accessor: Accessor<M, V>,
        observer: Option<Rc<PropertyPathObserver<M>>>,
        root: Option<Rc<M>>,
    ) -> Rc<Self> {
        Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            accessor,
            observer,
            root: RefCell::new(root),
            observers: HandlerList::new(),
            listening: RefCell::new(None),
            publish_count: Cell::new(0),
        })
    }

    /// The model the binding reads.
    pub fn root(&self) -> Option<Rc<M>> {
        self.root.borrow().clone()
    }

    /// The accessor of the binding.
    pub fn accessor(&self) -> &Accessor<M, V> {
        &self.accessor
    }

    /// The current value, if the binding has a root (`GetResult`).
    pub fn value(&self) -> Option<V> {
        let root = self.root.borrow().clone()?;
        Some(self.accessor.get(&root))
    }

    fn is_initialized(&self) -> bool {
        !self.observers.is_empty()
    }

    fn matches(&self, name: &str) -> bool {
        match self.accessor.property_name() {
            Some(property) => name.is_empty() || name == property,
            None => true,
        }
    }

    fn publish_value(&self) {
        let Some(value) = self.value() else {
            return;
        };
        self.publish_count.set(self.publish_count.get().wrapping_add(1));
        // Every observer but the last gets a copy; the last takes the value.
        let observers = self.observers.snapshot();
        if let Some(((_, last), others)) = observers.split_last() {
            for (_, observer) in others {
                observer(value.clone());
            }
            last(value);
        }
    }

    fn start_listening(&self) {
        if self.listening.borrow().is_some() {
            return;
        }
        let Some(root) = self.root.borrow().clone() else {
            return;
        };
        let weak = self.weak_self.clone();
        let listening = match &self.observer {
            Some(observer) => Some(Listening::Path(observer.subscribe(
                &root,
                Rc::new(move || {
                    if let Some(this) = weak.upgrade() {
                        this.publish_value();
                    }
                }),
            ))),
            None => model_notifier(&root).map(|notifier| {
                let token = notifier.property_changed().add(Rc::new(move |name: &str| {
                    if let Some(this) = weak.upgrade() {
                        if this.matches(name) {
                            this.publish_value();
                        }
                    }
                }));
                Listening::Model { model: root.clone(), token }
            }),
        };
        *self.listening.borrow_mut() = listening;
    }

    fn stop_listening(&self) {
        let listening = self.listening.borrow_mut().take();
        match listening {
            Some(Listening::Model { model, token }) => {
                if let Some(notifier) = model_notifier(&model) {
                    notifier.property_changed().remove(token);
                }
            }
            Some(Listening::Path(subscription)) => subscription.dispose(),
            None => {}
        }
    }

    fn root_changed(&self, root: Option<Rc<M>>) {
        self.stop_listening();
        *self.root.borrow_mut() = root;
        if self.is_initialized() {
            self.start_listening();
        }
        self.publish_value();
    }
}

impl<M: TreeModel, V: Clone + 'static> ICellBinding<V> for AccessorBinding<M, V> {
    fn subscribe(&self, observer: Rc<dyn Fn(V)>) -> u64 {
        let first = self.observers.is_empty();
        let token = self.observers.add(observer.clone());
        if first {
            // `Initialize`: the first subscriber starts the listening.
            self.start_listening();
        }
        if let Some(value) = self.value() {
            observer(value);
        }
        token
    }

    fn unsubscribe(&self, token: u64) {
        self.observers.remove(token);
        if self.observers.is_empty() {
            // `Deinitialize`.
            self.stop_listening();
        }
    }

    fn on_next(&self, value: V) {
        let Some(root) = self.root.borrow().clone() else {
            return;
        };
        if self.accessor.setter().is_none() {
            return;
        }
        let count = self.publish_count.get();
        if self.is_initialized() {
            self.accessor.set(&root, value);
        }
        // A model that does not notify the change still gets its value published, so that the
        // observers see what the model holds.
        if self.publish_count.get() == count {
            self.publish_value();
        }
    }

    fn can_write(&self) -> bool {
        self.accessor.setter().is_some()
    }

    fn as_retargetable(&self) -> Option<&dyn IRetargetableCellBinding> {
        Some(self)
    }
}

impl<M: TreeModel, V: Clone + 'static> IRetargetableCellBinding for AccessorBinding<M, V> {
    fn try_set_root(&self, root: Option<&BoxedValue>) -> bool {
        match root {
            None => {
                self.root_changed(None);
                true
            }
            Some(root) => {
                let any: Rc<dyn Any> = root.clone();
                match any.downcast::<M>() {
                    Ok(model) => {
                        self.root_changed(Some(model));
                        true
                    }
                    Err(_) => false,
                }
            }
        }
    }

    fn try_suspend_root(&self) -> bool {
        self.stop_listening();
        *self.root.borrow_mut() = None;
        true
    }
}

impl<M: TreeModel, V: Clone + 'static> Drop for AccessorBinding<M, V> {
    fn drop(&mut self) {
        self.stop_listening();
    }
}
