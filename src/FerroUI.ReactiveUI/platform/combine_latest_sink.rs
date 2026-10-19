//! `CombineLatestSink<TFirst, TSecond>` (`Shared/Platform/CombineLatestSink.cs`).

use ferroui_reactive::Observer;
use ferroui_system::{IDisposable, ObservableError, IObservable};
use std::cell::RefCell;
use std::rc::Rc;

/// The latest values of two sources, combined (`CombineLatestSink<TFirst, TSecond>`): once both
/// have produced a value, `on_next` receives the latest pair at each change; an error of either
/// goes to `on_error`, and their ends are ignored.
pub struct CombineLatestSink {
    first_subscription: Rc<dyn IDisposable>,
    second_subscription: Rc<dyn IDisposable>,
}

/// The latest values of the two sources.
struct Latest<A, B> {
    first: Option<A>,
    second: Option<B>,
}

impl CombineLatestSink {
    /// Subscribes `first` and `second` (`CombineLatestSink(IObservable<TFirst>, IObservable<TSecond>,
    /// Action<TFirst, TSecond>, Action<Exception>?)`).
    pub fn new<A: Clone + 'static, B: Clone + 'static>(
        first: Rc<dyn IObservable<A>>,
        second: Rc<dyn IObservable<B>>,
        on_next: impl Fn(A, B) + 'static,
        on_error: Option<Rc<dyn Fn(ObservableError)>>,
    ) -> Self {
        let latest = Rc::new(RefCell::new(Latest::<A, B> { first: None, second: None }));
        let on_next: Rc<dyn Fn(A, B)> = Rc::new(on_next);

        // `Emit`: the pair, once both sources have produced a value.
        let emit = {
            let latest = latest.clone();
            let on_next = on_next.clone();
            Rc::new(move || {
                let pair = {
                    let latest = latest.borrow();
                    match (&latest.first, &latest.second) {
                        (Some(first), Some(second)) => Some((first.clone(), second.clone())),
                        _ => None,
                    }
                };
                if let Some((first, second)) = pair {
                    on_next(first, second);
                }
            })
        };

        let first_subscription = {
            let latest = latest.clone();
            let emit = emit.clone();
            let on_error = on_error.clone();
            first.subscribe(Observer::create_all(
                move |value: A| {
                    latest.borrow_mut().first = Some(value);
                    emit();
                },
                move |error| {
                    if let Some(on_error) = &on_error {
                        on_error(error);
                    }
                },
                || {},
            ))
        };
        let second_subscription = {
            let emit = emit.clone();
            second.subscribe(Observer::create_all(
                move |value: B| {
                    latest.borrow_mut().second = Some(value);
                    emit();
                },
                move |error| {
                    if let Some(on_error) = &on_error {
                        on_error(error);
                    }
                },
                || {},
            ))
        };

        Self { first_subscription, second_subscription }
    }
}

impl IDisposable for CombineLatestSink {
    fn dispose(&self) {
        self.first_subscription.dispose();
        self.second_subscription.dispose();
    }
}
