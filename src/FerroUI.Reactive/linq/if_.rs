//! `If` (`Linq/Observable/If.cs`), and its overloads of `QueryLanguage.Imperative.cs`.

use crate::concurrency::IScheduler;
use crate::internal::subscribe_sink;
use crate::linq::creation::IdentitySink;
use crate::linq::Observable;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::rc::Rc;

type Condition = Rc<dyn Fn() -> Result<bool, ObservableError>>;

/// `If<TResult>`: each subscription evaluates the condition and subscribes to the sequence it
/// selects (`Eval`); a condition that fails ends the sequence with its error.
struct IfObservable<T> {
    condition: Condition,
    then_source: Rc<dyn IObservable<T>>,
    else_source: Rc<dyn IObservable<T>>,
}

impl<T: 'static> IObservable<T> for IfObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let condition = self.condition.clone();
        let (then_source, else_source) = (self.then_source.clone(), self.else_source.clone());
        subscribe_sink(observer, IdentitySink::new, move |sink: &Rc<IdentitySink<T>>| {
            // `If<TResult>._.Run`.
            let result = match condition() {
                Ok(true) => then_source,
                Ok(false) => else_source,
                Err(error) => {
                    sink.sink.forward_on_error(error);
                    return;
                }
            };
            sink.run(result);
        })
    }
}

pub(crate) fn if_observable<T: 'static>(
    condition: Condition,
    then_source: Rc<dyn IObservable<T>>,
    else_source: Rc<dyn IObservable<T>>,
) -> Rc<dyn IObservable<T>> {
    Rc::new(IfObservable { condition, then_source, else_source })
}

impl Observable {
    /// The values of `then_source` when `condition` holds at the subscription, else those of
    /// `else_source` (`If<TResult>(Func<bool>, IObservable<TResult>, IObservable<TResult>)`).
    pub fn if_<T: 'static>(
        condition: impl Fn() -> bool + 'static,
        then_source: Rc<dyn IObservable<T>>,
        else_source: Rc<dyn IObservable<T>>,
    ) -> Rc<dyn IObservable<T>> {
        if_observable(Rc::new(move || Ok(condition())), then_source, else_source)
    }

    /// The values of `then_source` when `condition` holds at the subscription, else an empty
    /// sequence (`If<TResult>(Func<bool>, IObservable<TResult>)`).
    pub fn if_then<T: 'static>(condition: impl Fn() -> bool + 'static, then_source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<T>> {
        Self::if_(condition, then_source, Observable::empty())
    }

    /// [`if_then`](Self::if_then), whose empty sequence ends on `scheduler`
    /// (`If<TResult>(Func<bool>, IObservable<TResult>, IScheduler)`).
    pub fn if_then_on<T: 'static>(
        condition: impl Fn() -> bool + 'static,
        then_source: Rc<dyn IObservable<T>>,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<T>> {
        Self::if_(condition, then_source, Observable::empty_on(scheduler))
    }

    /// [`if_`](Self::if_) with a condition that may fail; a failure ends the sequence with its error
    /// (`If` with a condition that throws).
    pub fn try_if<T: 'static>(
        condition: impl Fn() -> Result<bool, ObservableError> + 'static,
        then_source: Rc<dyn IObservable<T>>,
        else_source: Rc<dyn IObservable<T>>,
    ) -> Rc<dyn IObservable<T>> {
        if_observable(Rc::new(condition), then_source, else_source)
    }

    /// [`if_then`](Self::if_then) with a condition that may fail.
    pub fn try_if_then<T: 'static>(
        condition: impl Fn() -> Result<bool, ObservableError> + 'static,
        then_source: Rc<dyn IObservable<T>>,
    ) -> Rc<dyn IObservable<T>> {
        Self::try_if(condition, then_source, Observable::empty())
    }

    /// [`if_then_on`](Self::if_then_on) with a condition that may fail.
    pub fn try_if_then_on<T: 'static>(
        condition: impl Fn() -> Result<bool, ObservableError> + 'static,
        then_source: Rc<dyn IObservable<T>>,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<T>> {
        Self::try_if(condition, then_source, Observable::empty_on(scheduler))
    }
}
