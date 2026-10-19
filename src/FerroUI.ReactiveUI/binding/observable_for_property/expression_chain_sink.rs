//! `ExpressionChainSink<TSender, TValue>`.

use super::expression_chain_parameters::{ExpressionChainParameters, ValueComparer};
use super::reactive_notify_property_changed_mixins::notify_for_property;
use crate::binding::interfaces::ObservedChange;
use ferroui_reactive::disposables::SerialDisposable;
use ferroui_reactive::linq::InvalidCastException;
use ferroui_system::component_model::{PathStep, PropertyOwner};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Observes the value at the end of a property path (`x => x.A.B.C`) and produces it as a change
/// each time any step of the path changes (`ExpressionChainSink<TSender, TValue>`).
///
/// Each step is observed on the value the step before produced, through the property observer
/// ([`ICreatesObservableForProperty`]) with the highest affinity for its object; the later steps
/// are observed again when an earlier value changes. The current value is produced on
/// subscription unless the first value is skipped. The sequence never completes; its changes
/// carry the root object as their sender.
///
/// [`ICreatesObservableForProperty`]: crate::binding::interfaces::ICreatesObservableForProperty
// Deviation (DEVIATIONS.md, ReactiveUI): a step whose object is absent is a step of an `Option`
// (`PropertyPath::optional_property`) and produces `None`, where upstream produces nothing while
// an intermediate value is null. The observation holds its root and the intermediate objects
// weakly, and reads the intermediate values again from the root when a step notifies.
pub struct ExpressionChainSink<T: 'static, V: 'static> {
    parameters: ExpressionChainParameters<T, V>,
}

impl<T: 'static, V: Clone + 'static> ExpressionChainSink<T, V> {
    /// The observation described by `parameters`.
    pub fn new(parameters: ExpressionChainParameters<T, V>) -> Self {
        Self { parameters }
    }
}

impl<T: 'static, V: Clone + 'static> IObservable<ObservedChange<Rc<T>, V>> for ExpressionChainSink<T, V> {
    fn subscribe(&self, observer: Rc<dyn IObserver<ObservedChange<Rc<T>, V>>>) -> Rc<dyn IDisposable> {
        let parameters = &self.parameters;
        let steps: Vec<PathStep> = parameters.path.steps().to_vec();
        let levels = steps.iter().map(|_| Level { subscription: SerialDisposable::new(), attaching: Cell::new(false) }).collect();
        let state = Rc::new_cyclic(|this| ChainState {
            downstream: observer,
            source: parameters.source.clone(),
            expression: parameters.path.path(),
            steps,
            before_change: parameters.before_change,
            is_distinct: parameters.is_distinct,
            suppress_warnings: parameters.suppress_warnings,
            comparer: parameters.comparer.clone(),
            keeps_repeat_after_attachment: parameters.keeps_repeat_after_attachment,
            levels,
            skip_next: Cell::new(parameters.skip_initial),
            suppress_next_if_same_as_last: Cell::new(false),
            last: RefCell::new(None),
            disposed: Cell::new(false),
            this: this.clone(),
        });
        state.run();
        Rc::new(ChainSubscription { state })
    }
}

/// The running state of one subscription (`Sink`).
struct ChainState<T: 'static, V: 'static> {
    downstream: Rc<dyn IObserver<ObservedChange<Rc<T>, V>>>,
    source: Weak<T>,
    expression: String,
    steps: Vec<PathStep>,
    before_change: bool,
    is_distinct: bool,
    suppress_warnings: bool,
    comparer: Option<ValueComparer<V>>,
    keeps_repeat_after_attachment: bool,
    levels: Vec<Level>,
    /// Whether the next value is dropped (skip-initial).
    skip_next: Cell<bool>,
    /// Set when the read that follows the attachment of a step produced a value, and cleared by
    /// the next value: a notification that repeats that value is dropped once, whatever the
    /// distinct setting is.
    suppress_next_if_same_as_last: Cell<bool>,
    last: RefCell<Option<V>>,
    disposed: Cell<bool>,
    this: Weak<ChainState<T, V>>,
}

/// The watcher of one step (`Level`).
struct Level {
    /// The subscription to the notifications of the step, replaced on each new object.
    subscription: Rc<SerialDisposable>,
    /// Whether the step is subscribing; a notification raised from inside the subscription is
    /// ignored, since the read that follows reports the same value.
    attaching: Cell<bool>,
}

impl<T: 'static, V: Clone + 'static> ChainState<T, V> {
    /// Establishes the chain from the root (`Run`).
    fn run(&self) {
        if self.steps.is_empty() {
            return;
        }
        if let Some(root) = self.root() {
            self.set_parent(0, root);
        }
    }

    /// The root as the value the first step reads from.
    fn root(&self) -> Option<Rc<dyn Any>> {
        self.source.upgrade().map(|source| Rc::new(source) as Rc<dyn Any>)
    }

    /// The value step `index` reads from now, read again from the root.
    fn parent_of(&self, index: usize) -> Option<Rc<dyn Any>> {
        let mut value = self.root()?;
        for step in &self.steps[..index] {
            value = Rc::from((step.get)(&*value));
        }
        Some(value)
    }

    /// Observes step `index` on `parent` and passes its current value on (`SetParent`).
    fn set_parent(&self, index: usize, parent: Rc<dyn Any>) {
        if self.disposed.get() {
            return;
        }
        let Some(this) = self.this.upgrade() else {
            return;
        };
        let step = &self.steps[index];
        let level = &self.levels[index];

        // Subscribe before reading, so that a change between the two is reported rather than lost.
        let owner: PropertyOwner = (step.owner)(&*parent);
        let notifications = notify_for_property(&owner, &step.name, self.before_change, self.suppress_warnings);
        drop(owner);
        level.attaching.set(true);
        let subscription = notifications.subscribe(Rc::new(LevelObserver { state: this, index }));
        level.attaching.set(false);
        level.subscription.set_disposable(Some(subscription));

        let value = (step.get)(&*parent);
        self.push(index, value, true);
    }

    /// A notification of step `index`: its value is read again and passed on (`OnNotification`).
    fn on_notification(&self, index: usize) {
        if self.disposed.get() || self.levels[index].attaching.get() {
            return;
        }
        let Some(parent) = self.parent_of(index) else {
            return;
        };
        let value = (self.steps[index].get)(&*parent);
        self.push(index, value, false);
    }

    /// Passes the value of step `index` to the next step, or produces it at the end (`Push`).
    fn push(&self, index: usize, value: Box<dyn Any>, from_kicker: bool) {
        if index + 1 == self.steps.len() {
            self.emit(value, from_kicker);
        } else {
            self.set_parent(index + 1, Rc::from(value));
        }
    }

    /// Produces the value at the end of the path, after the skip-initial, the drop of the repeat
    /// that follows an attachment, and the distinct gate (`Emit`).
    fn emit(&self, value: Box<dyn Any>, from_kicker: bool) {
        if self.disposed.get() {
            return;
        }
        if self.skip_next.get() {
            self.skip_next.set(false);
            self.suppress_next_if_same_as_last.set(false);
            return;
        }
        let typed: V = match value.downcast::<V>() {
            Ok(typed) => *typed,
            Err(_) => {
                self.downstream.on_error(Rc::new(InvalidCastException::new::<V>()));
                return;
            }
        };
        if self.should_suppress(&typed, from_kicker) {
            return;
        }
        self.suppress_next_if_same_as_last.set(from_kicker && !self.keeps_repeat_after_attachment);
        *self.last.borrow_mut() = Some(typed.clone());
        let Some(sender) = self.source.upgrade() else {
            return;
        };
        self.downstream.on_next(ObservedChange::new(sender, self.expression.clone(), typed));
    }

    /// Whether a value is dropped, by the drop of the repeat after an attachment or by the distinct
    /// gate; the value read after an attachment is never dropped (`ShouldSuppress`).
    fn should_suppress(&self, typed: &V, from_kicker: bool) -> bool {
        if from_kicker {
            return false;
        }
        let repeats_last = match (&self.comparer, self.last.borrow().as_ref()) {
            (Some(comparer), Some(last)) => comparer(typed, last),
            _ => false,
        };
        if self.suppress_next_if_same_as_last.replace(false) && repeats_last {
            return true;
        }
        self.is_distinct && repeats_last
    }

    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        for level in &self.levels {
            level.subscription.dispose();
        }
    }
}

/// The observer of the notifications of one step (`Level.Observer`).
struct LevelObserver<T: 'static, V: 'static> {
    state: Rc<ChainState<T, V>>,
    index: usize,
}

impl<T: 'static, V: Clone + 'static> IObserver<ObservedChange<PropertyOwner, ()>> for LevelObserver<T, V> {
    fn on_next(&self, _value: ObservedChange<PropertyOwner, ()>) {
        self.state.on_notification(self.index);
    }

    fn on_error(&self, error: ObservableError) {
        self.state.downstream.on_error(error);
    }

    fn on_completed(&self) {}
}

/// The subscription to a chain.
struct ChainSubscription<T: 'static, V: 'static> {
    state: Rc<ChainState<T, V>>,
}

impl<T: 'static, V: Clone + 'static> IDisposable for ChainSubscription<T, V> {
    fn dispose(&self) {
        self.state.dispose();
    }
}

