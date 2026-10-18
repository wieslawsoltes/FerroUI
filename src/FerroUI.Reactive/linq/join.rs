//! `Join` (`Linq/Observable/Join.cs`).

use crate::disposables::{CompositeDisposable, SingleAssignmentDisposableValue};
use crate::internal::{subscribe_sink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::rc::{Rc, Weak};

/// The selector of the sequence whose first notification ends the duration of a value.
pub(crate) type DurationSelector<V, D> = Rc<dyn Fn(&V) -> Result<Rc<dyn IObservable<D>>, ObservableError>>;

type ResultSelector<L, R, Res> = Rc<dyn Fn(L, R) -> Result<Res, ObservableError>>;

/// The sink of `Join` (`Join<TLeft, TRight, TLeftDuration, TRightDuration, TResult>._`, an
/// `IdentitySink<TResult>`): the values of each side live until their duration ends; each value
/// pairs with the living values of the other side that arrived before it.
struct JoinSink<L, R, LD, RD, Res> {
    sink: Sink<Res>,
    group: Rc<CompositeDisposable>,
    left_map: RefCell<BTreeMap<usize, L>>,
    right_map: RefCell<BTreeMap<usize, R>>,
    left_duration_selector: DurationSelector<L, LD>,
    right_duration_selector: DurationSelector<R, RD>,
    result_selector: ResultSelector<L, R, Res>,
    left_done: Cell<bool>,
    left_id: Cell<usize>,
    right_done: Cell<bool>,
    right_id: Cell<usize>,
}

impl<L: Clone + 'static, R: Clone + 'static, LD: 'static, RD: 'static, Res: 'static> JoinSink<L, R, LD, RD, Res> {
    fn run(self: &Rc<Self>, left: &Rc<dyn IObservable<L>>, right: &Rc<dyn IObservable<R>>) {
        let left_observer = Rc::new(JoinLeftObserver { parent: self.clone(), resource: SingleAssignmentDisposableValue::new() });
        let right_observer = Rc::new(JoinRightObserver { parent: self.clone(), resource: SingleAssignmentDisposableValue::new() });

        self.group.add(left_observer.clone());
        self.group.add(right_observer.clone());

        let subscription = left.subscribe(left_observer.clone());
        left_observer.resource.set_disposable(Some(subscription));
        let subscription = right.subscribe(right_observer.clone());
        right_observer.resource.set_disposable(Some(subscription));

        self.sink.set_upstream(self.group.clone());
    }

    /// The duration of the left value `id` has ended (`LeftObserver.Expire`).
    fn expire_left(&self, id: usize, resource: &Rc<dyn IDisposable>) {
        let completed = {
            let mut left_map = self.left_map.borrow_mut();
            left_map.remove(&id).is_some() && left_map.is_empty()
        } && self.left_done.get();
        if completed {
            self.sink.forward_on_completed();
        }
        self.group.remove(resource);
    }

    /// The duration of the right value `id` has ended (`RightObserver.Expire`).
    fn expire_right(&self, id: usize, resource: &Rc<dyn IDisposable>) {
        let completed = {
            let mut right_map = self.right_map.borrow_mut();
            right_map.remove(&id).is_some() && right_map.is_empty()
        } && self.right_done.get();
        if completed {
            self.sink.forward_on_completed();
        }
        self.group.remove(resource);
    }

    /// Subscribes to the duration of a value (the `DurationObserver` of each side).
    fn subscribe_duration<D: 'static>(self: &Rc<Self>, duration: Rc<dyn IObservable<D>>, id: usize, left: bool) {
        let observer = Rc::new_cyclic(|this| JoinDurationObserver {
            this: this.clone(),
            parent: self.clone(),
            id,
            left,
            resource: SingleAssignmentDisposableValue::new(),
            _duration: PhantomData,
        });
        self.group.add(observer.clone());
        let subscription = duration.subscribe(observer.clone());
        observer.resource.set_disposable(Some(subscription));
    }
}

impl<L, R, LD, RD, Res: 'static> IDisposable for JoinSink<L, R, LD, RD, Res> {
    fn dispose(&self) {
        self.sink.dispose();
    }
}

/// The observer of the left sequence (`LeftObserver`, a `SafeObserver<TLeft>`).
struct JoinLeftObserver<L, R, LD, RD, Res> {
    parent: Rc<JoinSink<L, R, LD, RD, Res>>,
    resource: SingleAssignmentDisposableValue,
}

impl<L: Clone + 'static, R: Clone + 'static, LD: 'static, RD: 'static, Res: 'static> IObserver<L> for JoinLeftObserver<L, R, LD, RD, Res> {
    fn on_next(&self, value: L) {
        let parent = &self.parent;

        let id = parent.left_id.get();
        parent.left_id.set(id + 1);
        let right_id = parent.right_id.get();
        parent.left_map.borrow_mut().insert(id, value.clone());

        let duration = match (parent.left_duration_selector)(&value) {
            Ok(duration) => duration,
            Err(error) => {
                parent.sink.forward_on_error(error);
                return;
            }
        };

        parent.subscribe_duration(duration, id, true);

        // The values of the other side are read before any is forwarded, so that the observer
        // downstream may cause notifications of the sources while it handles a result.
        let right_values: Vec<R> = parent.right_map.borrow().iter().filter(|(key, _)| **key < right_id).map(|(_, value)| value.clone()).collect();
        for right_value in right_values {
            match (parent.result_selector)(value.clone(), right_value) {
                Ok(result) => parent.sink.forward_on_next(result),
                Err(error) => {
                    parent.sink.forward_on_error(error);
                    return;
                }
            }
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        let parent = &self.parent;
        parent.left_done.set(true);
        let complete = parent.right_done.get() || parent.left_map.borrow().is_empty();
        if complete {
            parent.sink.forward_on_completed();
        } else {
            self.resource.dispose();
        }
    }
}

impl<L, R, LD, RD, Res> IDisposable for JoinLeftObserver<L, R, LD, RD, Res> {
    fn dispose(&self) {
        self.resource.dispose();
    }
}

/// The observer of the right sequence (`RightObserver`, a `SafeObserver<TRight>`).
struct JoinRightObserver<L, R, LD, RD, Res> {
    parent: Rc<JoinSink<L, R, LD, RD, Res>>,
    resource: SingleAssignmentDisposableValue,
}

impl<L: Clone + 'static, R: Clone + 'static, LD: 'static, RD: 'static, Res: 'static> IObserver<R> for JoinRightObserver<L, R, LD, RD, Res> {
    fn on_next(&self, value: R) {
        let parent = &self.parent;

        let id = parent.right_id.get();
        parent.right_id.set(id + 1);
        let left_id = parent.left_id.get();
        parent.right_map.borrow_mut().insert(id, value.clone());

        let duration = match (parent.right_duration_selector)(&value) {
            Ok(duration) => duration,
            Err(error) => {
                parent.sink.forward_on_error(error);
                return;
            }
        };

        parent.subscribe_duration(duration, id, false);

        let left_values: Vec<L> = parent.left_map.borrow().iter().filter(|(key, _)| **key < left_id).map(|(_, value)| value.clone()).collect();
        for left_value in left_values {
            match (parent.result_selector)(left_value, value.clone()) {
                Ok(result) => parent.sink.forward_on_next(result),
                Err(error) => {
                    parent.sink.forward_on_error(error);
                    return;
                }
            }
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        let parent = &self.parent;
        parent.right_done.set(true);
        let complete = parent.left_done.get() || parent.right_map.borrow().is_empty();
        if complete {
            parent.sink.forward_on_completed();
        } else {
            self.resource.dispose();
        }
    }
}

impl<L, R, LD, RD, Res> IDisposable for JoinRightObserver<L, R, LD, RD, Res> {
    fn dispose(&self) {
        self.resource.dispose();
    }
}

/// The observer of the duration of a value (`LeftObserver.DurationObserver` and
/// `RightObserver.DurationObserver`, `SafeObserver`s): its first notification ends the duration;
/// an error ends the result with it.
struct JoinDurationObserver<L, R, LD, RD, Res, D> {
    this: Weak<JoinDurationObserver<L, R, LD, RD, Res, D>>,
    parent: Rc<JoinSink<L, R, LD, RD, Res>>,
    id: usize,
    left: bool,
    resource: SingleAssignmentDisposableValue,
    _duration: PhantomData<fn(D)>,
}

impl<L: Clone + 'static, R: Clone + 'static, LD: 'static, RD: 'static, Res: 'static, D: 'static> JoinDurationObserver<L, R, LD, RD, Res, D> {
    fn expire(&self) {
        let Some(this) = self.this.upgrade() else {
            return;
        };
        let this: Rc<dyn IDisposable> = this;
        if self.left {
            self.parent.expire_left(self.id, &this);
        } else {
            self.parent.expire_right(self.id, &this);
        }
    }
}

impl<L: Clone + 'static, R: Clone + 'static, LD: 'static, RD: 'static, Res: 'static, D: 'static> IObserver<D> for JoinDurationObserver<L, R, LD, RD, Res, D> {
    fn on_next(&self, _value: D) {
        self.expire();
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.expire();
    }
}

impl<L, R, LD, RD, Res, D> IDisposable for JoinDurationObserver<L, R, LD, RD, Res, D> {
    fn dispose(&self) {
        self.resource.dispose();
    }
}

/// The observable of `Join` (`Join<TLeft, TRight, TLeftDuration, TRightDuration, TResult>`).
struct JoinObservable<L, R, LD, RD, Res> {
    left: Rc<dyn IObservable<L>>,
    right: Rc<dyn IObservable<R>>,
    left_duration_selector: DurationSelector<L, LD>,
    right_duration_selector: DurationSelector<R, RD>,
    result_selector: ResultSelector<L, R, Res>,
}

impl<L: Clone + 'static, R: Clone + 'static, LD: 'static, RD: 'static, Res: 'static> IObservable<Res> for JoinObservable<L, R, LD, RD, Res> {
    fn subscribe(&self, observer: Rc<dyn IObserver<Res>>) -> Rc<dyn IDisposable> {
        let (left, right) = (self.left.clone(), self.right.clone());
        let (left_duration_selector, right_duration_selector, result_selector) =
            (self.left_duration_selector.clone(), self.right_duration_selector.clone(), self.result_selector.clone());
        subscribe_sink(
            observer,
            move |observer| {
                Rc::new(JoinSink {
                    sink: Sink::new(observer),
                    group: CompositeDisposable::new(),
                    left_map: RefCell::new(BTreeMap::new()),
                    right_map: RefCell::new(BTreeMap::new()),
                    left_duration_selector,
                    right_duration_selector,
                    result_selector,
                    left_done: Cell::new(false),
                    left_id: Cell::new(0),
                    right_done: Cell::new(false),
                    right_id: Cell::new(0),
                })
            },
            move |sink: &Rc<JoinSink<L, R, LD, RD, Res>>| sink.run(&left, &right),
        )
    }
}

/// `Join`: pairs the values of `left` and `right` whose durations overlap.
pub(crate) fn join<L: Clone + 'static, R: Clone + 'static, LD: 'static, RD: 'static, Res: 'static>(
    left: Rc<dyn IObservable<L>>,
    right: Rc<dyn IObservable<R>>,
    left_duration_selector: impl Fn(&L) -> Result<Rc<dyn IObservable<LD>>, ObservableError> + 'static,
    right_duration_selector: impl Fn(&R) -> Result<Rc<dyn IObservable<RD>>, ObservableError> + 'static,
    result_selector: impl Fn(L, R) -> Result<Res, ObservableError> + 'static,
) -> Rc<dyn IObservable<Res>> {
    Rc::new(JoinObservable {
        left,
        right,
        left_duration_selector: Rc::new(left_duration_selector),
        right_duration_selector: Rc::new(right_duration_selector),
        result_selector: Rc::new(result_selector),
    })
}

/// The methods of `Join` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! join_methods {
    () => {
        /// Pairs the values of the sequence and of `right` whose durations overlap; the duration of
        /// a value ends at the first notification of the sequence its selector returns
        /// (`Join(IObservable<TLeft>, IObservable<TRight>, Func<TLeft, IObservable<TLeftDuration>>,
        /// Func<TRight, IObservable<TRightDuration>>, Func<TLeft, TRight, TResult>)`).
        fn join<R: Clone + 'static, LD: 'static, RD: 'static, Res: 'static>(
            &self,
            right: std::rc::Rc<dyn ferroui_system::IObservable<R>>,
            left_duration_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<LD>> + 'static,
            right_duration_selector: impl Fn(&R) -> std::rc::Rc<dyn ferroui_system::IObservable<RD>> + 'static,
            result_selector: impl Fn(T, R) -> Res + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<Res>>
        where
            T: Clone,
        {
            crate::linq::join::join(
                self.observable(),
                right,
                move |value: &T| Ok(left_duration_selector(value)),
                move |value: &R| Ok(right_duration_selector(value)),
                move |left, right| Ok(result_selector(left, right)),
            )
        }

        /// [`join`](Self::join) with selectors that may fail; a failure ends the sequence with its
        /// error (`Join` with selectors that throw).
        fn try_join<R: Clone + 'static, LD: 'static, RD: 'static, Res: 'static>(
            &self,
            right: std::rc::Rc<dyn ferroui_system::IObservable<R>>,
            left_duration_selector: impl Fn(&T) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<LD>>, ferroui_system::ObservableError> + 'static,
            right_duration_selector: impl Fn(&R) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<RD>>, ferroui_system::ObservableError> + 'static,
            result_selector: impl Fn(T, R) -> Result<Res, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<Res>>
        where
            T: Clone,
        {
            crate::linq::join::join(self.observable(), right, left_duration_selector, right_duration_selector, result_selector)
        }
    };
}
pub(crate) use join_methods;
