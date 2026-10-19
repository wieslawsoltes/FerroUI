//! `ToAsync` (`Linq/QueryLanguage.Async.cs`, region `ToAsync`): a function turned into an
//! asynchronous one, whose calls run the function on a scheduler and give a sequence of its result.
//!
//! The 68 overloads of upstream (functions and actions of 0 to 16 arguments, with or without a
//! scheduler) are four methods of [`Observable`]: [`to_async`](Observable::to_async),
//! [`to_async_on`](Observable::to_async_on) and their `try_` forms for a function that may fail.
//! The arities are implementations of [`ToAsyncFunction`] and [`TryToAsyncFunction`] for the
//! closures of 0 to 16 arguments, which a macro generates as upstream's T4 template does. An
//! `Action` is a function returning `()`, and its sequence is one of units, as upstream's.
//!
//! Each call runs the function on the scheduler and gives the sequence of an `AsyncSubject`, which
//! keeps the result for every observer: this is [`Observable::try_start_on`] with the arguments of
//! the call, as `Start` is `ToAsync(function)()` upstream.

use crate::concurrency::{IScheduler, SchedulerDefaults};
use crate::linq::Observable;
use ferroui_system::{IObservable, ObservableError};
use std::rc::Rc;

/// A closure of `Args` (a tuple of its argument types) returning `R`, which
/// [`Observable::to_async`] turns into an asynchronous function (`ToAsync(Func<.., TResult>)`,
/// `ToAsync(Action<..>)` for `R = ()`).
pub trait ToAsyncFunction<Args, R> {
    /// The asynchronous function: `Rc<dyn Fn(A1, .., An) -> Rc<dyn IObservable<R>>>`.
    type Async;

    /// The asynchronous function whose calls run this one on `scheduler` (`ToAsync(.., IScheduler)`).
    fn to_async_on(self, scheduler: Rc<dyn IScheduler>) -> Self::Async;
}

/// A closure of `Args` (a tuple of its argument types) returning `Result<R, ObservableError>`, which
/// [`Observable::try_to_async`] turns into an asynchronous function whose sequence ends with the
/// error of a failed call (`ToAsync` of a function that throws).
pub trait TryToAsyncFunction<Args, R> {
    /// The asynchronous function: `Rc<dyn Fn(A1, .., An) -> Rc<dyn IObservable<R>>>`.
    type Async;

    /// The asynchronous function whose calls run this one on `scheduler` (`ToAsync(.., IScheduler)`).
    fn try_to_async_on(self, scheduler: Rc<dyn IScheduler>) -> Self::Async;
}

/// The implementations of [`ToAsyncFunction`] and [`TryToAsyncFunction`] for the closures of one
/// arity (the overloads of one arity in the T4 template of upstream).
macro_rules! to_async_arity {
    ($($arg:ident: $ty:ident),*) => {
        impl<F, R, $($ty: 'static),*> ToAsyncFunction<($($ty,)*), R> for F
        where
            F: Fn($($ty),*) -> R + 'static,
            R: Clone + 'static,
        {
            type Async = Rc<dyn Fn($($ty),*) -> Rc<dyn IObservable<R>>>;

            fn to_async_on(self, scheduler: Rc<dyn IScheduler>) -> Self::Async {
                let function = Rc::new(self);
                Rc::new(move |$($arg: $ty),*| {
                    let function = function.clone();
                    Observable::try_start_on(move || Ok(function($($arg),*)), scheduler.clone())
                })
            }
        }

        impl<F, R, $($ty: 'static),*> TryToAsyncFunction<($($ty,)*), R> for F
        where
            F: Fn($($ty),*) -> Result<R, ObservableError> + 'static,
            R: Clone + 'static,
        {
            type Async = Rc<dyn Fn($($ty),*) -> Rc<dyn IObservable<R>>>;

            fn try_to_async_on(self, scheduler: Rc<dyn IScheduler>) -> Self::Async {
                let function = Rc::new(self);
                Rc::new(move |$($arg: $ty),*| {
                    let function = function.clone();
                    Observable::try_start_on(move || function($($arg),*), scheduler.clone())
                })
            }
        }
    };
}

to_async_arity!();
to_async_arity!(a1: A1);
to_async_arity!(a1: A1, a2: A2);
to_async_arity!(a1: A1, a2: A2, a3: A3);
to_async_arity!(a1: A1, a2: A2, a3: A3, a4: A4);
to_async_arity!(a1: A1, a2: A2, a3: A3, a4: A4, a5: A5);
to_async_arity!(a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6);
to_async_arity!(a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6, a7: A7);
to_async_arity!(a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6, a7: A7, a8: A8);
to_async_arity!(a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6, a7: A7, a8: A8, a9: A9);
to_async_arity!(a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6, a7: A7, a8: A8, a9: A9, a10: A10);
to_async_arity!(a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6, a7: A7, a8: A8, a9: A9, a10: A10, a11: A11);
to_async_arity!(a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6, a7: A7, a8: A8, a9: A9, a10: A10, a11: A11, a12: A12);
to_async_arity!(a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6, a7: A7, a8: A8, a9: A9, a10: A10, a11: A11, a12: A12, a13: A13);
to_async_arity!(a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6, a7: A7, a8: A8, a9: A9, a10: A10, a11: A11, a12: A12, a13: A13, a14: A14);
to_async_arity!(a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6, a7: A7, a8: A8, a9: A9, a10: A10, a11: A11, a12: A12, a13: A13, a14: A14, a15: A15);
to_async_arity!(
    a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6, a7: A7, a8: A8, a9: A9, a10: A10, a11: A11, a12: A12, a13: A13, a14: A14, a15: A15, a16: A16
);

impl Observable {
    /// The asynchronous form of `function` (a closure of 0 to 16 arguments): each call runs it on
    /// the scheduler of asynchronous conversions and gives a sequence of its result, kept for every
    /// observer (`ToAsync(Func<.., TResult>)`, `ToAsync(Action<..>)` for a closure returning `()`).
    ///
    /// The argument types of the closure must be written out (`|a: i32, b: i32| a + b`), since they
    /// choose the arity.
    pub fn to_async<Args, R, F: ToAsyncFunction<Args, R>>(function: F) -> F::Async {
        function.to_async_on(SchedulerDefaults::async_conversions())
    }

    /// [`to_async`](Self::to_async) running the function on `scheduler`
    /// (`ToAsync(Func<.., TResult>, IScheduler)`, `ToAsync(Action<..>, IScheduler)`).
    pub fn to_async_on<Args, R, F: ToAsyncFunction<Args, R>>(function: F, scheduler: Rc<dyn IScheduler>) -> F::Async {
        function.to_async_on(scheduler)
    }

    /// [`to_async`](Self::to_async) of a function that may fail: the sequence of a call that fails
    /// ends with its error (`ToAsync` of a function that throws).
    pub fn try_to_async<Args, R, F: TryToAsyncFunction<Args, R>>(function: F) -> F::Async {
        function.try_to_async_on(SchedulerDefaults::async_conversions())
    }

    /// [`try_to_async`](Self::try_to_async) running the function on `scheduler`.
    pub fn try_to_async_on<Args, R, F: TryToAsyncFunction<Args, R>>(function: F, scheduler: Rc<dyn IScheduler>) -> F::Async {
        function.try_to_async_on(scheduler)
    }
}
