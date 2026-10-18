//! `GroupJoin` (`Linq/Observable/GroupJoin.cs`).
//!
//! Each left value has a window (`Rc<dyn IObservable<TRight>>`, the window observable of `Window`
//! made of a subject) of the right values whose durations overlap its own; the subscriptions to the
//! sources last while the outer subscription or a subscription to a window does (a
//! `RefCountDisposable`), as upstream's.

use crate::disposables::{CompositeDisposable, RefCountDisposable, SingleAssignmentDisposableValue};
use crate::internal::{subscribe_sink, Sink};
use crate::linq::join::DurationSelector;
use crate::subjects::Subject;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::rc::{Rc, Weak};

type ResultSelector<L, R, Res> = Rc<dyn Fn(L, Rc<dyn IObservable<R>>) -> Result<Res, ObservableError>>;

/// The sink of `GroupJoin` (`GroupJoin<TLeft, TRight, TLeftDuration, TRightDuration, TResult>._`, an
/// `IdentitySink<TResult>`).
struct GroupJoinSink<L, R, LD, RD, Res> {
    sink: Sink<Res>,
    group: Rc<CompositeDisposable>,
    ref_count: Rc<RefCountDisposable>,
    left_map: RefCell<BTreeMap<usize, Rc<Subject<R>>>>,
    right_map: RefCell<BTreeMap<usize, R>>,
    left_duration_selector: DurationSelector<L, LD>,
    right_duration_selector: DurationSelector<R, RD>,
    result_selector: ResultSelector<L, R, Res>,
    left_id: Cell<usize>,
    right_id: Cell<usize>,
}

impl<L: 'static, R: Clone + 'static, LD: 'static, RD: 'static, Res: 'static> GroupJoinSink<L, R, LD, RD, Res> {
    fn run(self: &Rc<Self>, left: &Rc<dyn IObservable<L>>, right: &Rc<dyn IObservable<R>>) {
        let left_observer = Rc::new(GroupJoinLeftObserver { parent: self.clone(), resource: SingleAssignmentDisposableValue::new() });
        self.group.add(left_observer.clone());

        let right_observer = Rc::new(GroupJoinRightObserver { parent: self.clone(), resource: SingleAssignmentDisposableValue::new() });
        self.group.add(right_observer.clone());

        let subscription = left.subscribe(left_observer.clone());
        left_observer.resource.set_disposable(Some(subscription));
        let subscription = right.subscribe(right_observer.clone());
        right_observer.resource.set_disposable(Some(subscription));

        self.sink.set_upstream(self.ref_count.clone());
    }

    /// Errors every open window, then the result (`OnError` of either side).
    fn fail(&self, error: ObservableError) {
        let windows: Vec<Rc<Subject<R>>> = self.left_map.borrow().values().cloned().collect();
        for window in windows {
            window.on_error(error.clone());
        }
        self.sink.forward_on_error(error);
    }

    /// The duration of the left value `id` has ended: its window completes
    /// (`LeftObserver.Expire`).
    fn expire_left(&self, id: usize, resource: &Rc<dyn IDisposable>) {
        let window = self.left_map.borrow_mut().remove(&id);
        if let Some(window) = window {
            window.on_completed();
        }
        self.group.remove(resource);
    }

    /// The duration of the right value `id` has ended (`RightObserver.Expire`).
    fn expire_right(&self, id: usize, resource: &Rc<dyn IDisposable>) {
        self.right_map.borrow_mut().remove(&id);
        self.group.remove(resource);
    }

    /// Subscribes to the duration of a value (the `DurationObserver` of each side).
    fn subscribe_duration<D: 'static>(self: &Rc<Self>, duration: Rc<dyn IObservable<D>>, id: usize, left: bool) {
        let observer = Rc::new_cyclic(|this| GroupJoinDurationObserver {
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

impl<L, R, LD, RD, Res: 'static> IDisposable for GroupJoinSink<L, R, LD, RD, Res> {
    fn dispose(&self) {
        self.sink.dispose();
    }
}

/// The observer of the left sequence (`LeftObserver`, a `SafeObserver<TLeft>`).
struct GroupJoinLeftObserver<L, R, LD, RD, Res> {
    parent: Rc<GroupJoinSink<L, R, LD, RD, Res>>,
    resource: SingleAssignmentDisposableValue,
}

impl<L: 'static, R: Clone + 'static, LD: 'static, RD: 'static, Res: 'static> IObserver<L> for GroupJoinLeftObserver<L, R, LD, RD, Res> {
    fn on_next(&self, value: L) {
        let parent = &self.parent;
        let s: Rc<Subject<R>> = Subject::new();

        let id = parent.left_id.get();
        parent.left_id.set(id + 1);
        let right_id = parent.right_id.get();
        parent.left_map.borrow_mut().insert(id, s.clone());

        let window = crate::linq::window::window_observable(&s, &parent.ref_count);

        // As upstream (since its version 2), the duration selector is evaluated and its sequence
        // subscribed to before the result selector is evaluated.
        let duration = match (parent.left_duration_selector)(&value) {
            Ok(duration) => duration,
            Err(error) => {
                parent.fail(error);
                return;
            }
        };

        parent.subscribe_duration(duration, id, true);

        let result = match (parent.result_selector)(value, window) {
            Ok(result) => result,
            Err(error) => {
                parent.fail(error);
                return;
            }
        };

        parent.sink.forward_on_next(result);

        // The values of the other side are read before any is notified, so that the observers of
        // the window may cause notifications of the sources while they handle a value.
        let right_values: Vec<R> = parent.right_map.borrow().iter().filter(|(key, _)| **key < right_id).map(|(_, value)| value.clone()).collect();
        for right_value in right_values {
            s.on_next(right_value);
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.fail(error);
    }

    fn on_completed(&self) {
        self.parent.sink.forward_on_completed();
        self.resource.dispose();
    }
}

impl<L, R, LD, RD, Res> IDisposable for GroupJoinLeftObserver<L, R, LD, RD, Res> {
    fn dispose(&self) {
        self.resource.dispose();
    }
}

/// The observer of the right sequence (`RightObserver`, a `SafeObserver<TRight>`).
struct GroupJoinRightObserver<L, R, LD, RD, Res> {
    parent: Rc<GroupJoinSink<L, R, LD, RD, Res>>,
    resource: SingleAssignmentDisposableValue,
}

impl<L: 'static, R: Clone + 'static, LD: 'static, RD: 'static, Res: 'static> IObserver<R> for GroupJoinRightObserver<L, R, LD, RD, Res> {
    fn on_next(&self, value: R) {
        let parent = &self.parent;

        let id = parent.right_id.get();
        parent.right_id.set(id + 1);
        let left_id = parent.left_id.get();
        parent.right_map.borrow_mut().insert(id, value.clone());

        let duration = match (parent.right_duration_selector)(&value) {
            Ok(duration) => duration,
            Err(error) => {
                parent.fail(error);
                return;
            }
        };

        parent.subscribe_duration(duration, id, false);

        let windows: Vec<Rc<Subject<R>>> = parent.left_map.borrow().iter().filter(|(key, _)| **key < left_id).map(|(_, window)| window.clone()).collect();
        for window in windows {
            window.on_next(value.clone());
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.fail(error);
    }

    fn on_completed(&self) {
        self.resource.dispose();
    }
}

impl<L, R, LD, RD, Res> IDisposable for GroupJoinRightObserver<L, R, LD, RD, Res> {
    fn dispose(&self) {
        self.resource.dispose();
    }
}

/// The observer of the duration of a value (`LeftObserver.DurationObserver` and
/// `RightObserver.DurationObserver`, `SafeObserver`s): its first notification ends the duration;
/// an error ends the windows and the result with it.
struct GroupJoinDurationObserver<L, R, LD, RD, Res, D> {
    this: Weak<GroupJoinDurationObserver<L, R, LD, RD, Res, D>>,
    parent: Rc<GroupJoinSink<L, R, LD, RD, Res>>,
    id: usize,
    left: bool,
    resource: SingleAssignmentDisposableValue,
    _duration: PhantomData<fn(D)>,
}

impl<L: 'static, R: Clone + 'static, LD: 'static, RD: 'static, Res: 'static, D: 'static> GroupJoinDurationObserver<L, R, LD, RD, Res, D> {
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

impl<L: 'static, R: Clone + 'static, LD: 'static, RD: 'static, Res: 'static, D: 'static> IObserver<D> for GroupJoinDurationObserver<L, R, LD, RD, Res, D> {
    fn on_next(&self, _value: D) {
        self.expire();
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.fail(error);
    }

    fn on_completed(&self) {
        self.expire();
    }
}

impl<L, R, LD, RD, Res, D> IDisposable for GroupJoinDurationObserver<L, R, LD, RD, Res, D> {
    fn dispose(&self) {
        self.resource.dispose();
    }
}

/// The observable of `GroupJoin` (`GroupJoin<TLeft, TRight, TLeftDuration, TRightDuration,
/// TResult>`).
struct GroupJoinObservable<L, R, LD, RD, Res> {
    left: Rc<dyn IObservable<L>>,
    right: Rc<dyn IObservable<R>>,
    left_duration_selector: DurationSelector<L, LD>,
    right_duration_selector: DurationSelector<R, RD>,
    result_selector: ResultSelector<L, R, Res>,
}

impl<L: 'static, R: Clone + 'static, LD: 'static, RD: 'static, Res: 'static> IObservable<Res> for GroupJoinObservable<L, R, LD, RD, Res> {
    fn subscribe(&self, observer: Rc<dyn IObserver<Res>>) -> Rc<dyn IDisposable> {
        let (left, right) = (self.left.clone(), self.right.clone());
        let (left_duration_selector, right_duration_selector, result_selector) =
            (self.left_duration_selector.clone(), self.right_duration_selector.clone(), self.result_selector.clone());
        subscribe_sink(
            observer,
            move |observer| {
                let group = CompositeDisposable::new();
                let ref_count = RefCountDisposable::new(group.clone());
                Rc::new(GroupJoinSink {
                    sink: Sink::new(observer),
                    group,
                    ref_count,
                    left_map: RefCell::new(BTreeMap::new()),
                    right_map: RefCell::new(BTreeMap::new()),
                    left_duration_selector,
                    right_duration_selector,
                    result_selector,
                    left_id: Cell::new(0),
                    right_id: Cell::new(0),
                })
            },
            move |sink: &Rc<GroupJoinSink<L, R, LD, RD, Res>>| sink.run(&left, &right),
        )
    }
}

/// `GroupJoin`: gives each value of `left`, with the window of the values of `right` whose
/// durations overlap its own, to `result_selector`.
pub(crate) fn group_join<L: 'static, R: Clone + 'static, LD: 'static, RD: 'static, Res: 'static>(
    left: Rc<dyn IObservable<L>>,
    right: Rc<dyn IObservable<R>>,
    left_duration_selector: impl Fn(&L) -> Result<Rc<dyn IObservable<LD>>, ObservableError> + 'static,
    right_duration_selector: impl Fn(&R) -> Result<Rc<dyn IObservable<RD>>, ObservableError> + 'static,
    result_selector: impl Fn(L, Rc<dyn IObservable<R>>) -> Result<Res, ObservableError> + 'static,
) -> Rc<dyn IObservable<Res>> {
    Rc::new(GroupJoinObservable {
        left,
        right,
        left_duration_selector: Rc::new(left_duration_selector),
        right_duration_selector: Rc::new(right_duration_selector),
        result_selector: Rc::new(result_selector),
    })
}

/// The methods of `GroupJoin` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! group_join_methods {
    () => {
        /// Gives each value of the sequence, with the window of the values of `right` whose
        /// durations overlap its own, to `result_selector`; the duration of a value ends at the
        /// first notification of the sequence its selector returns (`GroupJoin(IObservable<TLeft>,
        /// IObservable<TRight>, Func<TLeft, IObservable<TLeftDuration>>, Func<TRight,
        /// IObservable<TRightDuration>>, Func<TLeft, IObservable<TRight>, TResult>)`).
        fn group_join<R: Clone + 'static, LD: 'static, RD: 'static, Res: 'static>(
            &self,
            right: std::rc::Rc<dyn ferroui_system::IObservable<R>>,
            left_duration_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<LD>> + 'static,
            right_duration_selector: impl Fn(&R) -> std::rc::Rc<dyn ferroui_system::IObservable<RD>> + 'static,
            result_selector: impl Fn(T, std::rc::Rc<dyn ferroui_system::IObservable<R>>) -> Res + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<Res>> {
            crate::linq::group_join::group_join(
                self.observable(),
                right,
                move |value: &T| Ok(left_duration_selector(value)),
                move |value: &R| Ok(right_duration_selector(value)),
                move |left, window| Ok(result_selector(left, window)),
            )
        }

        /// [`group_join`](Self::group_join) with selectors that may fail; a failure ends the windows
        /// and the sequence with its error (`GroupJoin` with selectors that throw).
        fn try_group_join<R: Clone + 'static, LD: 'static, RD: 'static, Res: 'static>(
            &self,
            right: std::rc::Rc<dyn ferroui_system::IObservable<R>>,
            left_duration_selector: impl Fn(&T) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<LD>>, ferroui_system::ObservableError> + 'static,
            right_duration_selector: impl Fn(&R) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<RD>>, ferroui_system::ObservableError> + 'static,
            result_selector: impl Fn(T, std::rc::Rc<dyn ferroui_system::IObservable<R>>) -> Result<Res, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<Res>> {
            crate::linq::group_join::group_join(self.observable(), right, left_duration_selector, right_duration_selector, result_selector)
        }
    };
}
pub(crate) use group_join_methods;
