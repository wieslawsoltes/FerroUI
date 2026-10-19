//! `InternalEx` (`Kernel/InternalEx.cs`): retries with a back-off, recurring actions on a
//! scheduler, and the internal helpers of the operators.
//!
//! The internal `Swap` is `std::mem::swap`, and `OnNext(ISubject<Unit>)` is `on_next(())`.

use ferroui_reactive::concurrency::{IScheduler, SchedulerExt};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::rc::Rc;
use std::time::Duration;

type BackOffStrategy<E> = Rc<dyn Fn(&E, usize) -> Option<Duration>>;

fn retry<T: 'static, E: std::error::Error + 'static>(source: Rc<dyn IObservable<T>>, back_off_strategy: BackOffStrategy<E>, failure_count: usize) -> Rc<dyn IObservable<T>> {
    let retried = source.clone();
    source.catch_of::<E>(move |error: ObservableError| {
        let typed = error.downcast_ref::<E>().expect("catch_of hands over the errors of its type");
        match back_off_strategy(typed, failure_count) {
            None => Observable::throw::<T>(error),
            Some(delay) => {
                let next = retry(retried.clone(), back_off_strategy.clone(), failure_count + 1);
                Observable::timer(delay).select_many(move |_| next.clone())
            }
        }
    })
}

/// The sequence, subscribed again after each error of type `E` once the delay that
/// `back_off_strategy` gives for the error and the number of failures so far has passed; a `None`
/// delay ends the sequence with the error (`RetryWithBackOff<TSource, TException>`).
pub fn retry_with_back_off<T: 'static, E: std::error::Error + 'static>(
    source: Rc<dyn IObservable<T>>,
    back_off_strategy: impl Fn(&E, usize) -> Option<Duration> + 'static,
) -> Rc<dyn IObservable<T>> {
    retry(source, Rc::new(back_off_strategy), 0)
}

/// Runs `action` every `interval` on `scheduler` until the result is disposed
/// (`ScheduleRecurringAction(IScheduler, TimeSpan, Action)`).
pub fn schedule_recurring_action(scheduler: &Rc<dyn IScheduler>, interval: Duration, action: impl Fn() + 'static) -> Rc<dyn IDisposable> {
    scheduler.schedule_recursive_after(interval, move |schedule_next: &dyn Fn(Duration)| {
        action();
        schedule_next(interval);
    })
}

/// Runs `action` after each interval that `interval` gives, on `scheduler`, until the result is
/// disposed (`ScheduleRecurringAction(IScheduler, Func<TimeSpan>, Action)`).
pub fn schedule_recurring_action_with(scheduler: &Rc<dyn IScheduler>, interval: impl Fn() -> Duration + 'static, action: impl Fn() + 'static) -> Rc<dyn IDisposable> {
    let first = interval();
    scheduler.schedule_recursive_after(first, move |schedule_next: &dyn Fn(Duration)| {
        action();
        schedule_next(interval());
    })
}

/// The sequence with each value replaced by `()` (`ToUnit`).
pub fn to_unit<T: 'static>(source: &Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<()>> {
    source.select(|_| ())
}

/// A sequence of the one value that `source` makes on each subscription, then the end
/// (`InternalEx.Return(Func<T>)`).
pub(crate) fn return_with<T: 'static>(source: impl Fn() -> T + 'static) -> Rc<dyn IObservable<T>> {
    Observable::create(move |observer: Rc<dyn IObserver<T>>| {
        observer.on_next(source());
        observer.on_completed();
        Disposable::empty()
    })
}

/// The index of `value` in `list`, or the bitwise complement of where it belongs, by the
/// algorithm of .NET's `List<T>.BinarySearch` (which equal item it finds matters to the
/// operators that insert at the result): `Err(index)` for the latter.
pub(crate) fn dotnet_binary_search<I, S>(list: &[I], value: &S, comparer: impl Fn(&I, &S) -> std::cmp::Ordering) -> Result<usize, usize> {
    let (mut lower, mut upper) = (0isize, list.len() as isize - 1);
    while lower <= upper {
        let middle = lower + ((upper - lower) >> 1);
        match comparer(&list[middle as usize], value) {
            std::cmp::Ordering::Equal => return Ok(middle as usize),
            std::cmp::Ordering::Less => lower = middle + 1,
            std::cmp::Ordering::Greater => upper = middle - 1,
        }
    }
    Err(lower as usize)
}
