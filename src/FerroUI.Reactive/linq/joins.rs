//! The join patterns and plans (`Joins/Pattern.cs`, `Joins/Pattern.Generated.cs`, `Joins/Plan.cs`,
//! `Joins/ActivePlan.cs`, `Joins/JoinObserver.cs`), and `And`, `Then` and `When`
//! (`Linq/QueryLanguage.Joins.cs`).
//!
//! A pattern (`xs.and(ys).and(zs)`) names the sequences that must all have a value; `then` turns
//! it into a [`Plan`] with the projection of those values; `Observable::when` subscribes to the
//! sequences of its plans once each (a sequence of several plans is shared, recognized by its
//! handle) and notifies the projections of the plans as their values match up.
//!
//! The types of upstream are generated for one to sixteen sequences; so are those of the port,
//! by a macro. Upstream's `Pattern<TSource1, ..., TSourceN>` is [`PatternN`](Pattern2).

use crate::disposables::{CompositeDisposable, Disposable, SingleAssignmentDisposableValue};
use crate::internal::subscribe_auto_detach;
use crate::linq::{Observable, ObservableExt};
use crate::notification::Notification;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

type OnError = Rc<dyn Fn(ObservableError)>;

/// What a plan does when it is done (`Action<ActivePlan> deactivate` of `Plan<TResult>.Activate`).
type Deactivate = Rc<dyn Fn(&Rc<dyn ActivePlan>)>;

/// A plan of a subscription, matching the values of its join observers (`ActivePlan`).
trait ActivePlan {
    /// Notifies the projection of the first values of its sequences when each has one, or ends the
    /// plan when one of them has completed (`Match`).
    fn match_(&self);
}

/// The observer of one sequence of a subscription, shared by the plans that name the sequence
/// (`IJoinObserver`).
trait IJoinObserver: IDisposable {
    /// Subscribes to the sequence (`Subscribe(object gate)`).
    fn subscribe(&self);

    /// Removes the first value of the queue (`Dequeue`).
    fn dequeue(&self);
}

fn same_plan(a: &Rc<dyn ActivePlan>, b: &Rc<dyn ActivePlan>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

/// `JoinObserver<T>`: it queues the values and the completion of its sequence (materialized), and
/// lets its plans match after each; an error goes to the error handler of the subscription.
struct JoinObserver<T> {
    this: Weak<JoinObserver<T>>,
    source: Rc<dyn IObservable<T>>,
    on_error: RefCell<Option<OnError>>,
    queue: RefCell<VecDeque<Notification<T>>>,
    active_plans: RefCell<Vec<Rc<dyn ActivePlan>>>,
    subscription: SingleAssignmentDisposableValue,
    is_stopped: Cell<bool>,
    is_disposed: Cell<bool>,
}

impl<T: 'static> JoinObserver<T> {
    fn new(source: Rc<dyn IObservable<T>>, on_error: OnError) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            source,
            on_error: RefCell::new(Some(on_error)),
            queue: RefCell::new(VecDeque::new()),
            active_plans: RefCell::new(Vec::new()),
            subscription: SingleAssignmentDisposableValue::new(),
            is_stopped: Cell::new(false),
            is_disposed: Cell::new(false),
        })
    }

    /// `AddActivePlan`.
    fn add_active_plan(&self, active_plan: Rc<dyn ActivePlan>) {
        self.active_plans.borrow_mut().push(active_plan);
    }

    /// `RemoveActivePlan`: the observer is disposed when no plan is left.
    fn remove_active_plan(&self, active_plan: &Rc<dyn ActivePlan>) {
        let is_empty = {
            let mut active_plans = self.active_plans.borrow_mut();
            if let Some(index) = active_plans.iter().position(|plan| same_plan(plan, active_plan)) {
                active_plans.remove(index);
            }
            active_plans.is_empty()
        };
        if is_empty {
            self.dispose_observer();
        }
    }

    /// Whether the queue has a first notification (`Queue.Count > 0`).
    fn has_head(&self) -> bool {
        !self.queue.borrow().is_empty()
    }

    /// Whether the first notification of the queue is the completion (`Queue.Peek().Kind == NotificationKind.OnCompleted`).
    fn head_is_completed(&self) -> bool {
        matches!(self.queue.borrow().front(), Some(Notification::OnCompleted))
    }

    /// The value of the first notification of the queue (`Queue.Peek().Value`): a copy, since a
    /// plan may name the sequence more than once.
    fn head_value(&self) -> T
    where
        T: Clone,
    {
        self.queue.borrow().front().and_then(Notification::value).cloned().expect("the first notification of the queue is a value")
    }

    /// `Dispose(bool)`: the observer stops, and its subscription is disposed.
    ///
    /// The plans and the error handler are released as well: after the disposal nothing reaches
    /// them through the observer, and they hold the observer in turn.
    fn dispose_observer(&self) {
        self.is_stopped.set(true);
        if !self.is_disposed.replace(true) {
            self.subscription.dispose();
            let active_plans = std::mem::take(&mut *self.active_plans.borrow_mut());
            drop(active_plans);
            let on_error = self.on_error.borrow_mut().take();
            drop(on_error);
        }
    }
}

impl<T: 'static> IObserver<Notification<T>> for JoinObserver<T> {
    // `ObserverBase<Notification<T>>.OnNext` and `OnNextCore`.
    fn on_next(&self, notification: Notification<T>) {
        if self.is_stopped.get() || self.is_disposed.get() {
            return;
        }
        if let Notification::OnError(error) = notification {
            let on_error = self.on_error.borrow().clone();
            if let Some(on_error) = on_error {
                on_error(error);
            }
            return;
        }
        self.queue.borrow_mut().push_back(notification);
        // Working on a copy, since the plans may change while they match.
        let active_plans = self.active_plans.borrow().clone();
        for active_plan in active_plans {
            active_plan.match_();
        }
    }

    // `OnErrorCore` and `OnCompletedCore` do nothing: the materialized sequence ends after its
    // last notification, which `on_next` has handled.
    fn on_error(&self, _error: ObservableError) {
        self.is_stopped.set(true);
    }

    fn on_completed(&self) {
        self.is_stopped.set(true);
    }
}

impl<T: 'static> IJoinObserver for JoinObserver<T> {
    fn subscribe(&self) {
        let Some(this) = self.this.upgrade() else {
            return;
        };
        let observer: Rc<dyn IObserver<Notification<T>>> = this;
        self.subscription.set_disposable(Some(self.source.materialize().subscribe_safe(observer)));
    }

    fn dequeue(&self) {
        self.queue.borrow_mut().pop_front();
    }
}

impl<T: 'static> IDisposable for JoinObserver<T> {
    fn dispose(&self) {
        self.dispose_observer();
    }
}

/// The join observers of a subscription, one per sequence, in the order they were made
/// (`Dictionary<object, IJoinObserver> externalSubscriptions`). A sequence is known by the address
/// of its handle, as upstream knows it by reference.
struct ExternalSubscriptions {
    entries: RefCell<Vec<(*const (), Rc<dyn Any>, Rc<dyn IJoinObserver>)>>,
}

impl ExternalSubscriptions {
    fn new() -> Self {
        Self { entries: RefCell::new(Vec::new()) }
    }

    /// The join observer of `source`, made the first time (`Plan<TResult>.CreateObserver`).
    fn create_observer<T: 'static>(&self, source: &Rc<dyn IObservable<T>>, on_error: &OnError) -> Rc<JoinObserver<T>> {
        let key = Rc::as_ptr(source) as *const ();
        let existing = self
            .entries
            .borrow()
            .iter()
            .filter(|(entry, _, _)| *entry == key)
            .find_map(|(_, observer, _)| observer.clone().downcast::<JoinObserver<T>>().ok());
        if let Some(observer) = existing {
            return observer;
        }
        let observer = JoinObserver::new(source.clone(), on_error.clone());
        let any: Rc<dyn Any> = observer.clone();
        let join: Rc<dyn IJoinObserver> = observer.clone();
        self.entries.borrow_mut().push((key, any, join));
        observer
    }

    /// The join observers (`externalSubscriptions.Values`).
    fn observers(&self) -> Vec<Rc<dyn IJoinObserver>> {
        self.entries.borrow().iter().map(|(_, _, observer)| observer.clone()).collect()
    }
}

/// Adds a join observer to the distinct join observers of a plan (`ActivePlan.AddJoinObserver`).
fn add_join_observer(join_observers: &mut Vec<Rc<dyn IJoinObserver>>, join_observer: Rc<dyn IJoinObserver>) {
    if !join_observers.iter().any(|observer| std::ptr::addr_eq(Rc::as_ptr(observer), Rc::as_ptr(&join_observer))) {
        join_observers.push(join_observer);
    }
}

/// What a plan makes for a subscription (`Plan<TResult>.Activate`).
trait PlanCore<R> {
    fn activate(&self, external_subscriptions: &ExternalSubscriptions, observer: &Rc<dyn IObserver<R>>, deactivate: &Deactivate) -> Rc<dyn ActivePlan>;
}

/// A plan of [`Observable::when`]: a pattern with the projection of its values (`Plan<TResult>`),
/// made by the `then` of a pattern.
pub struct Plan<R> {
    core: Rc<dyn PlanCore<R>>,
}

impl<R> Clone for Plan<R> {
    fn clone(&self) -> Self {
        Self { core: self.core.clone() }
    }
}

/// The types of the patterns and plans over `$count` sequences (`Pattern<TSource1, ...>`,
/// `Plan<T1, ..., TResult>`, `ActivePlan<T1, ...>`).
macro_rules! join_arity {
    ($pattern:ident, $plan:ident, $active:ident, $count:literal; $($t:ident $field:ident $value:ident),+) => {
        #[doc = concat!(
            "A join pattern over ", stringify!($count), " sequences (`Pattern<TSource1, ...>` of ",
            stringify!($count), "): it matches when all of them have a value."
        )]
        pub struct $pattern<$($t),+> {
            $($field: Rc<dyn IObservable<$t>>,)+
        }

        impl<$($t),+> Clone for $pattern<$($t),+> {
            fn clone(&self) -> Self {
                Self { $($field: self.$field.clone(),)+ }
            }
        }

        impl<$($t: Clone + 'static),+> $pattern<$($t),+> {
            /// The plan that matches when all the sequences of the pattern have a value and projects
            /// the values with `selector` (`Then<TResult>(Func<TSource1, ..., TResult>)`).
            pub fn then<R: 'static>(&self, selector: impl Fn($($t),+) -> R + 'static) -> Plan<R> {
                self.try_then(move |$($value),+| Ok(selector($($value),+)))
            }

            /// [`then`](Self::then) with a selector that may fail; a failure ends the sequence of
            /// [`Observable::when`] with its error (`Then` with a selector that throws).
            pub fn try_then<R: 'static>(&self, selector: impl Fn($($t),+) -> Result<R, ObservableError> + 'static) -> Plan<R> {
                Plan { core: Rc::new($plan { $($field: self.$field.clone(),)+ selector: Rc::new(selector) }) }
            }
        }

        /// `Plan<T1, ..., TResult>`.
        struct $plan<$($t,)+ R> {
            $($field: Rc<dyn IObservable<$t>>,)+
            selector: Rc<dyn Fn($($t),+) -> Result<R, ObservableError>>,
        }

        impl<$($t: Clone + 'static,)+ R: 'static> PlanCore<R> for $plan<$($t,)+ R> {
            fn activate(
                &self,
                external_subscriptions: &ExternalSubscriptions,
                observer: &Rc<dyn IObserver<R>>,
                deactivate: &Deactivate,
            ) -> Rc<dyn ActivePlan> {
                let on_error: OnError = {
                    let observer = observer.clone();
                    Rc::new(move |error| observer.on_error(error))
                };
                $(let $field = external_subscriptions.create_observer(&self.$field, &on_error);)+

                let selector = self.selector.clone();
                let target = observer.clone();
                let on_next: Box<dyn Fn($($t),+)> = Box::new(move |$($value),+| match selector($($value),+) {
                    Ok(result) => target.on_next(result),
                    Err(error) => target.on_error(error),
                });

                let mut join_observers: Vec<Rc<dyn IJoinObserver>> = Vec::new();
                $(add_join_observer(&mut join_observers, $field.clone());)+

                let deactivate = deactivate.clone();
                let active_plan: Rc<dyn ActivePlan> = Rc::new_cyclic(|this: &Weak<$active<$($t),+>>| $active {
                    this: this.clone(),
                    $($field: $field.clone(),)+
                    join_observers,
                    on_next,
                    deactivate,
                });
                $($field.add_active_plan(active_plan.clone());)+
                active_plan
            }
        }

        /// `ActivePlan<T1, ...>`.
        struct $active<$($t),+> {
            this: Weak<$active<$($t),+>>,
            $($field: Rc<JoinObserver<$t>>,)+
            join_observers: Vec<Rc<dyn IJoinObserver>>,
            on_next: Box<dyn Fn($($t),+)>,
            deactivate: Deactivate,
        }

        impl<$($t: Clone + 'static),+> $active<$($t),+> {
            /// The end of the plan (the `onCompleted` of `Plan<T1, ..., TResult>.Activate`): it leaves
            /// its join observers and is deactivated.
            fn complete(&self) {
                let Some(this) = self.this.upgrade() else {
                    return;
                };
                let active_plan: Rc<dyn ActivePlan> = this;
                $(self.$field.remove_active_plan(&active_plan);)+
                (self.deactivate)(&active_plan);
            }
        }

        impl<$($t: Clone + 'static),+> ActivePlan for $active<$($t),+> {
            fn match_(&self) {
                let has_heads = [$(self.$field.has_head()),+];
                if has_heads.iter().all(|has_head| *has_head) {
                    let completed = [$(self.$field.head_is_completed()),+];
                    if completed.iter().any(|completed| *completed) {
                        self.complete();
                    } else {
                        $(let $value = self.$field.head_value();)+
                        for join_observer in &self.join_observers {
                            join_observer.dequeue();
                        }
                        (self.on_next)($($value),+);
                    }
                }
            }
        }
    };
}

/// The `And` of a pattern, which adds a sequence to it (`And<TSourceN>(IObservable<TSourceN>)`).
macro_rules! join_and {
    ($pattern:ident => $next:ident<$new:ident> $new_field:ident; $($t:ident $field:ident),+) => {
        impl<$($t: 'static),+> $pattern<$($t),+> {
            /// The pattern that also needs a value of `other`
            /// (`And<TSourceN>(IObservable<TSourceN>)`).
            pub fn and<$new: 'static>(&self, other: Rc<dyn IObservable<$new>>) -> $next<$($t,)+ $new> {
                $next { $($field: self.$field.clone(),)+ $new_field: other }
            }
        }
    };
}

join_arity!(Pattern1, Plan1, ActivePlan1, 1; T1 first v1);
join_arity!(Pattern2, Plan2, ActivePlan2, 2; T1 first v1, T2 second v2);
join_arity!(Pattern3, Plan3, ActivePlan3, 3; T1 first v1, T2 second v2, T3 third v3);
join_arity!(Pattern4, Plan4, ActivePlan4, 4; T1 first v1, T2 second v2, T3 third v3, T4 fourth v4);
join_arity!(Pattern5, Plan5, ActivePlan5, 5; T1 first v1, T2 second v2, T3 third v3, T4 fourth v4, T5 fifth v5);
join_arity!(Pattern6, Plan6, ActivePlan6, 6; T1 first v1, T2 second v2, T3 third v3, T4 fourth v4, T5 fifth v5, T6 sixth v6);
join_arity!(Pattern7, Plan7, ActivePlan7, 7; T1 first v1, T2 second v2, T3 third v3, T4 fourth v4, T5 fifth v5, T6 sixth v6, T7 seventh v7);
join_arity!(Pattern8, Plan8, ActivePlan8, 8; T1 first v1, T2 second v2, T3 third v3, T4 fourth v4, T5 fifth v5, T6 sixth v6, T7 seventh v7, T8 eighth v8);
join_arity!(Pattern9, Plan9, ActivePlan9, 9;
    T1 first v1, T2 second v2, T3 third v3, T4 fourth v4, T5 fifth v5, T6 sixth v6, T7 seventh v7, T8 eighth v8, T9 ninth v9);
join_arity!(Pattern10, Plan10, ActivePlan10, 10;
    T1 first v1, T2 second v2, T3 third v3, T4 fourth v4, T5 fifth v5, T6 sixth v6, T7 seventh v7, T8 eighth v8, T9 ninth v9,
    T10 tenth v10);
join_arity!(Pattern11, Plan11, ActivePlan11, 11;
    T1 first v1, T2 second v2, T3 third v3, T4 fourth v4, T5 fifth v5, T6 sixth v6, T7 seventh v7, T8 eighth v8, T9 ninth v9,
    T10 tenth v10, T11 eleventh v11);
join_arity!(Pattern12, Plan12, ActivePlan12, 12;
    T1 first v1, T2 second v2, T3 third v3, T4 fourth v4, T5 fifth v5, T6 sixth v6, T7 seventh v7, T8 eighth v8, T9 ninth v9,
    T10 tenth v10, T11 eleventh v11, T12 twelfth v12);
join_arity!(Pattern13, Plan13, ActivePlan13, 13;
    T1 first v1, T2 second v2, T3 third v3, T4 fourth v4, T5 fifth v5, T6 sixth v6, T7 seventh v7, T8 eighth v8, T9 ninth v9,
    T10 tenth v10, T11 eleventh v11, T12 twelfth v12, T13 thirteenth v13);
join_arity!(Pattern14, Plan14, ActivePlan14, 14;
    T1 first v1, T2 second v2, T3 third v3, T4 fourth v4, T5 fifth v5, T6 sixth v6, T7 seventh v7, T8 eighth v8, T9 ninth v9,
    T10 tenth v10, T11 eleventh v11, T12 twelfth v12, T13 thirteenth v13, T14 fourteenth v14);
join_arity!(Pattern15, Plan15, ActivePlan15, 15;
    T1 first v1, T2 second v2, T3 third v3, T4 fourth v4, T5 fifth v5, T6 sixth v6, T7 seventh v7, T8 eighth v8, T9 ninth v9,
    T10 tenth v10, T11 eleventh v11, T12 twelfth v12, T13 thirteenth v13, T14 fourteenth v14, T15 fifteenth v15);
join_arity!(Pattern16, Plan16, ActivePlan16, 16;
    T1 first v1, T2 second v2, T3 third v3, T4 fourth v4, T5 fifth v5, T6 sixth v6, T7 seventh v7, T8 eighth v8, T9 ninth v9,
    T10 tenth v10, T11 eleventh v11, T12 twelfth v12, T13 thirteenth v13, T14 fourteenth v14, T15 fifteenth v15,
    T16 sixteenth v16);

join_and!(Pattern2 => Pattern3<T3> third; T1 first, T2 second);
join_and!(Pattern3 => Pattern4<T4> fourth; T1 first, T2 second, T3 third);
join_and!(Pattern4 => Pattern5<T5> fifth; T1 first, T2 second, T3 third, T4 fourth);
join_and!(Pattern5 => Pattern6<T6> sixth; T1 first, T2 second, T3 third, T4 fourth, T5 fifth);
join_and!(Pattern6 => Pattern7<T7> seventh; T1 first, T2 second, T3 third, T4 fourth, T5 fifth, T6 sixth);
join_and!(Pattern7 => Pattern8<T8> eighth; T1 first, T2 second, T3 third, T4 fourth, T5 fifth, T6 sixth, T7 seventh);
join_and!(Pattern8 => Pattern9<T9> ninth; T1 first, T2 second, T3 third, T4 fourth, T5 fifth, T6 sixth, T7 seventh, T8 eighth);
join_and!(Pattern9 => Pattern10<T10> tenth;
    T1 first, T2 second, T3 third, T4 fourth, T5 fifth, T6 sixth, T7 seventh, T8 eighth, T9 ninth);
join_and!(Pattern10 => Pattern11<T11> eleventh;
    T1 first, T2 second, T3 third, T4 fourth, T5 fifth, T6 sixth, T7 seventh, T8 eighth, T9 ninth, T10 tenth);
join_and!(Pattern11 => Pattern12<T12> twelfth;
    T1 first, T2 second, T3 third, T4 fourth, T5 fifth, T6 sixth, T7 seventh, T8 eighth, T9 ninth, T10 tenth, T11 eleventh);
join_and!(Pattern12 => Pattern13<T13> thirteenth;
    T1 first, T2 second, T3 third, T4 fourth, T5 fifth, T6 sixth, T7 seventh, T8 eighth, T9 ninth, T10 tenth, T11 eleventh,
    T12 twelfth);
join_and!(Pattern13 => Pattern14<T14> fourteenth;
    T1 first, T2 second, T3 third, T4 fourth, T5 fifth, T6 sixth, T7 seventh, T8 eighth, T9 ninth, T10 tenth, T11 eleventh,
    T12 twelfth, T13 thirteenth);
join_and!(Pattern14 => Pattern15<T15> fifteenth;
    T1 first, T2 second, T3 third, T4 fourth, T5 fifth, T6 sixth, T7 seventh, T8 eighth, T9 ninth, T10 tenth, T11 eleventh,
    T12 twelfth, T13 thirteenth, T14 fourteenth);
join_and!(Pattern15 => Pattern16<T16> sixteenth;
    T1 first, T2 second, T3 third, T4 fourth, T5 fifth, T6 sixth, T7 seventh, T8 eighth, T9 ninth, T10 tenth, T11 eleventh,
    T12 twelfth, T13 thirteenth, T14 fourteenth, T15 fifteenth);

/// The pattern of two sequences (`And<TLeft, TRight>(IObservable<TLeft>, IObservable<TRight>)`).
pub(crate) fn and<T1: 'static, T2: 'static>(left: Rc<dyn IObservable<T1>>, right: Rc<dyn IObservable<T2>>) -> Pattern2<T1, T2> {
    Pattern2 { first: left, second: right }
}

/// The pattern of one sequence (`new Pattern<TSource>(source)` of `Then`).
pub(crate) fn pattern<T1: 'static>(source: Rc<dyn IObservable<T1>>) -> Pattern1<T1> {
    Pattern1 { first: source }
}

/// The plans of `When`, enumerated for each subscription; an error is an enumeration that throws.
type PlanIterator<R> = Box<dyn Iterator<Item = Result<Plan<R>, ObservableError>>>;
type Plans<R> = Rc<dyn Fn() -> Result<PlanIterator<R>, ObservableError>>;

/// `WhenObservable<TResult>` (an `ObservableBase<TResult>`).
struct WhenObservable<R> {
    plans: Plans<R>,
}

impl<R: 'static> IObservable<R> for WhenObservable<R> {
    fn subscribe(&self, observer: Rc<dyn IObserver<R>>) -> Rc<dyn IDisposable> {
        let plans = self.plans.clone();
        subscribe_auto_detach(observer, move |observer| subscribe_when(&plans, observer))
    }
}

/// `WhenObservable<TResult>.SubscribeCore`: activates the plans, then subscribes to their
/// sequences, in the order the plans first named them.
fn subscribe_when<R: 'static>(plans: &Plans<R>, observer: Rc<dyn IObserver<R>>) -> Rc<dyn IDisposable> {
    let external_subscriptions = Rc::new(ExternalSubscriptions::new());
    let active_plans: Rc<RefCell<Vec<Weak<dyn ActivePlan>>>> = Rc::new(RefCell::new(Vec::new()));
    let out_observer: Rc<dyn IObserver<R>> = Rc::new(OutObserver { observer: observer.clone(), external_subscriptions: external_subscriptions.clone() });

    let on_deactivate: Deactivate = {
        let (active_plans, out_observer) = (active_plans.clone(), out_observer.clone());
        Rc::new(move |active_plan: &Rc<dyn ActivePlan>| {
            let is_empty = {
                let mut active_plans = active_plans.borrow_mut();
                if let Some(index) = active_plans.iter().position(|plan| std::ptr::addr_eq(plan.as_ptr(), Rc::as_ptr(active_plan))) {
                    active_plans.remove(index);
                }
                active_plans.is_empty()
            };
            if is_empty {
                out_observer.on_completed();
            }
        })
    };

    let activated = (|| -> Result<(), ObservableError> {
        for plan in plans()? {
            let active_plan = plan?.core.activate(&external_subscriptions, &out_observer, &on_deactivate);
            active_plans.borrow_mut().push(Rc::downgrade(&active_plan));
        }
        Ok(())
    })();
    if let Err(error) = activated {
        observer.on_error(error);
        return Disposable::empty();
    }

    let join_observers = external_subscriptions.observers();
    let group = CompositeDisposable::with_capacity(join_observers.len());
    for join_observer in join_observers {
        join_observer.subscribe();
        group.add(join_observer);
    }
    group
}

/// `WhenObservable<TResult>.OutObserver`: an error disposes every join observer first.
struct OutObserver<R> {
    observer: Rc<dyn IObserver<R>>,
    external_subscriptions: Rc<ExternalSubscriptions>,
}

impl<R: 'static> IObserver<R> for OutObserver<R> {
    fn on_next(&self, value: R) {
        self.observer.on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        for join_observer in self.external_subscriptions.observers() {
            join_observer.dispose();
        }
        self.observer.on_error(error);
    }

    fn on_completed(&self) {
        self.observer.on_completed();
    }
}

impl Observable {
    /// The projections of `plans`, notified as the values of the sequences of each plan match up
    /// (`When<TResult>(params Plan<TResult>[])`, `When<TResult>(IEnumerable<Plan<TResult>>)`). The
    /// sequence completes when every plan has ended (when a sequence of a plan has completed and
    /// its values are used up), and ends with the first error of a sequence. Each subscription
    /// enumerates `plans` anew.
    pub fn when<R: 'static, I>(plans: I) -> Rc<dyn IObservable<R>>
    where
        I: IntoIterator<Item = Plan<R>> + Clone + 'static,
        I::IntoIter: 'static,
    {
        let plans: Plans<R> = Rc::new(move || Ok(Box::new(plans.clone().into_iter().map(Ok::<Plan<R>, ObservableError>)) as PlanIterator<R>));
        Rc::new(WhenObservable { plans })
    }

    /// [`when`](Self::when) over plans whose enumeration may fail: `plans` starts an enumeration
    /// (`GetEnumerator`, which fails with an `Err`), and each item is the next plan or the error of
    /// the step (`MoveNext` that throws). A failure ends the sequence with its error, before any
    /// subscription.
    pub fn try_when<R: 'static, I>(plans: impl Fn() -> Result<I, ObservableError> + 'static) -> Rc<dyn IObservable<R>>
    where
        I: IntoIterator<Item = Result<Plan<R>, ObservableError>>,
        I::IntoIter: 'static,
    {
        let plans: Plans<R> = Rc::new(move || plans().map(|plans| Box::new(plans.into_iter()) as PlanIterator<R>));
        Rc::new(WhenObservable { plans })
    }
}

/// The methods of `And` and `Then` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! joins_methods {
    () => {
        /// The pattern that matches when the sequence and `right` both have a value
        /// (`And<TLeft, TRight>(IObservable<TLeft>, IObservable<TRight>)`).
        fn and<U: 'static>(&self, right: std::rc::Rc<dyn ferroui_system::IObservable<U>>) -> crate::linq::joins::Pattern2<T, U> {
            crate::linq::joins::and(self.observable(), right)
        }

        /// The plan that matches each value of the sequence and projects it with `selector`
        /// (`Then<TSource, TResult>(IObservable<TSource>, Func<TSource, TResult>)`).
        fn then<R: 'static>(&self, selector: impl Fn(T) -> R + 'static) -> crate::linq::joins::Plan<R>
        where
            T: Clone,
        {
            crate::linq::joins::pattern(self.observable()).then(selector)
        }

        /// [`then`](Self::then) with a selector that may fail; a failure ends the sequence of
        /// [`Observable::when`](crate::linq::Observable::when) with its error (`Then` with a
        /// selector that throws).
        fn try_then<R: 'static>(
            &self,
            selector: impl Fn(T) -> Result<R, ferroui_system::ObservableError> + 'static,
        ) -> crate::linq::joins::Plan<R>
        where
            T: Clone,
        {
            crate::linq::joins::pattern(self.observable()).try_then(selector)
        }
    };
}
pub(crate) use joins_methods;
