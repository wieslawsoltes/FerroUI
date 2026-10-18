//! `Models/TreeDataGrid/ShowExpanderObservable.cs`: whether the expander of a model's row is
//! shown: the model's "has children" value when the column has a selector of it, otherwise
//! whether the model has child models, followed through the model's `PropertyChanged` and the
//! child collection's `CollectionChanged`.

use super::{model_notifier, AccessorBinding, ICellBinding};
use ferroui_treedatagrid_core::accessor::TreeModel;
use ferroui_treedatagrid_core::model_collection::{ModelsChangedArgs, ModelsChangedHandler};
use ferroui_treedatagrid_core::{Accessor, Models};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct State<M: TreeModel> {
    model: Option<Rc<M>>,
    observer: Option<Rc<dyn Fn(bool)>>,
    subscription: Option<(Rc<AccessorBinding<M, bool>>, u64)>,
    incc: Option<(Models<M>, u64)>,
    inpc: Option<(Rc<M>, u64)>,
}

/// `ShowExpanderObservable<TModel>`: an observable with a single subscriber of whether the
/// expander of a model is shown.
pub struct ShowExpanderObservable<M: TreeModel> {
    weak_self: Weak<Self>,
    child_selector: Rc<dyn Fn(&Rc<M>) -> Option<Models<M>>>,
    has_children_selector: Option<Accessor<M, bool>>,
    state: RefCell<State<M>>,
}

impl<M: TreeModel> ShowExpanderObservable<M> {
    pub fn new(
        child_selector: Rc<dyn Fn(&Rc<M>) -> Option<Models<M>>>,
        has_children_selector: Option<Accessor<M, bool>>,
        model: Rc<M>,
    ) -> Rc<Self> {
        Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            child_selector,
            has_children_selector,
            state: RefCell::new(State { model: Some(model), observer: None, subscription: None, incc: None, inpc: None }),
        })
    }

    /// Subscribes the single observer (`Subscribe`, then `Subscribed`).
    ///
    /// # Panics
    /// When the observable already has a subscriber (`InvalidOperationException`), or was
    /// unsubscribed before (`ObjectDisposedException`).
    pub fn subscribe(&self, observer: Rc<dyn Fn(bool)>) {
        let model = {
            let mut state = self.state.borrow_mut();
            if state.observer.is_some() {
                panic!("The observable can only be subscribed once.");
            }
            let Some(model) = state.model.clone() else {
                panic!("Cannot access a disposed object. Object name: 'ShowExpanderObservable'.");
            };
            state.observer = Some(observer);
            model
        };

        match &self.has_children_selector {
            Some(selector) => {
                let binding = AccessorBinding::new(selector.clone(), Some(model));
                // The binding pushes its current value synchronously: store it before.
                let weak = self.weak_self.clone();
                let token = binding.subscribe(Rc::new(move |value: &bool| {
                    if let Some(this) = weak.upgrade() {
                        this.publish_next(*value);
                    }
                }));
                self.state.borrow_mut().subscription = Some((binding, token));
            }
            None => {
                if let Some(notifier) = model_notifier(&model) {
                    let weak = self.weak_self.clone();
                    let token = notifier.property_changed().add(Rc::new(move |_: &str| {
                        if let Some(this) = weak.upgrade() {
                            this.publish_children();
                        }
                    }));
                    self.state.borrow_mut().inpc = Some((model.clone(), token));
                }

                self.publish_children();
            }
        }
    }

    /// Ends the subscription (`Unsubscribed`).
    pub fn unsubscribe(&self) {
        let (subscription, incc, inpc) = {
            let mut state = self.state.borrow_mut();
            state.observer = None;
            state.model = None;
            (state.subscription.take(), state.incc.take(), state.inpc.take())
        };
        if let Some((binding, token)) = subscription {
            binding.unsubscribe(token);
        }
        if let Some((children, token)) = incc {
            children.remove_collection_changed(token);
        }
        if let Some((model, token)) = inpc {
            if let Some(notifier) = model_notifier(&model) {
                notifier.property_changed().remove(token);
            }
        }
    }

    fn publish_next(&self, value: bool) {
        let observer = self.state.borrow().observer.clone();
        if let Some(observer) = observer {
            observer(value);
        }
    }

    // `OnNext(BindingValue<IEnumerable<TModel>?>)`: the child models changed.
    fn on_children(&self, children: Option<Models<M>>) {
        let old = self.state.borrow_mut().incc.take();
        if let Some((old, token)) = old {
            old.remove_collection_changed(token);
        }

        match children {
            Some(children) => {
                let weak = self.weak_self.clone();
                let observed = Rc::downgrade(&children);
                let handler: Rc<ModelsChangedHandler<M>> = Rc::new(move |_: &ModelsChangedArgs<'_, M>| {
                    if let Some(this) = weak.upgrade() {
                        this.publish_next(observed.upgrade().is_some_and(|children| children.count() > 0));
                    }
                });
                let token = children.add_collection_changed(handler);
                if let Some(token) = token {
                    self.state.borrow_mut().incc = Some((children.clone(), token));
                }

                self.publish_next(children.count() > 0);
            }
            None => self.publish_next(false),
        }
    }

    fn publish_children(&self) {
        let model = self.state.borrow().model.clone();
        if let Some(model) = model {
            let children = (self.child_selector)(&model);
            self.on_children(children);
        }
    }
}

impl<M: TreeModel> Drop for ShowExpanderObservable<M> {
    fn drop(&mut self) {
        self.unsubscribe();
    }
}
