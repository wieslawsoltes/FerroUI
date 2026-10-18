//! `PushToPullAdapter<TSource, TResult>` and `PushToPullSink<TSource, TResult>`
//! (`Linq/Observable/PushToPullAdapter.cs`): a sequence read by its consumer, one step at a time,
//! from the notifications the source has pushed so far (`Collect`, `Chunkify`, `MostRecent`).
//!
//! These never wait for the source: a step answers with what has arrived, so they work on one
//! thread, unlike the blocking operators (`Latest`, `Next`, `ToEnumerable`).

use crate::disposables::SingleAssignmentDisposableValue;
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::rc::Rc;

/// The pull side of the sink of an adapter (`PushToPullSink<TSource, TResult>`).
pub(crate) trait PullSink<R> {
    /// The next value; `Ok(None)` ends the enumeration (`TryMoveNext`); an error is what upstream
    /// throws.
    fn try_move_next(&self) -> Result<Option<R>, ObservableError>;

    /// The subscription to the source (`_upstream`).
    fn upstream(&self) -> &SingleAssignmentDisposableValue;
}

/// A sequence of values read by its consumer (the `IEnumerable<TResult>` of an adapter): each
/// enumerator subscribes to the source (`GetEnumerator`).
pub struct PullEnumerable<R> {
    get_enumerator: Rc<dyn Fn() -> Result<PullEnumerator<R>, ObservableError>>,
}

impl<R> Clone for PullEnumerable<R> {
    fn clone(&self) -> Self {
        Self { get_enumerator: self.get_enumerator.clone() }
    }
}

impl<R: 'static> PullEnumerable<R> {
    /// Subscribes a sink to `source` for each enumerator (`PushToPullAdapter.GetEnumerator`);
    /// `run` makes the sink, and fails where upstream's `Run` throws.
    pub(crate) fn new<T: 'static, S: PullSink<R> + IObserver<T> + 'static>(
        source: Rc<dyn IObservable<T>>,
        run: impl Fn() -> Result<Rc<S>, ObservableError> + 'static,
    ) -> Self {
        Self {
            get_enumerator: Rc::new(move || {
                let sink = run()?;
                let subscription = source.subscribe(sink.clone());
                sink.upstream().set_disposable(Some(subscription));
                Ok(PullEnumerator { sink, done: false })
            }),
        }
    }

    /// An enumerator subscribed to the source (`GetEnumerator`); an error is what upstream
    /// throws, before any subscription.
    pub fn get_enumerator(&self) -> Result<PullEnumerator<R>, ObservableError> {
        (self.get_enumerator)()
    }
}

/// The enumerator of a [`PullEnumerable`] (`PushToPullSink<TSource, TResult>` as an
/// `IEnumerator<TResult>`). Each step gives `Ok` with a value or `Err` with an error of the
/// source or of a selector; dropping it, or [`dispose`](Self::dispose), unsubscribes from the
/// source.
///
/// Deviation (DEVIATIONS.md, Reactive): upstream's `MoveNext` throws the error again at each later
/// call; here the error is the last item and the enumeration ends after it, as a fused iterator.
pub struct PullEnumerator<R> {
    sink: Rc<dyn PullSink<R>>,
    done: bool,
}

impl<R> PullEnumerator<R> {
    /// Unsubscribes from the source (`Dispose`).
    pub fn dispose(&self) {
        self.sink.upstream().dispose();
    }
}

impl<R> Iterator for PullEnumerator<R> {
    type Item = Result<R, ObservableError>;

    /// `MoveNext` and `Current`.
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        match self.sink.try_move_next() {
            Ok(Some(value)) => Some(Ok(value)),
            Ok(None) => {
                self.done = true;
                self.dispose();
                None
            }
            Err(error) => {
                self.done = true;
                Some(Err(error))
            }
        }
    }
}

impl<R> std::iter::FusedIterator for PullEnumerator<R> {}

impl<R> Drop for PullEnumerator<R> {
    fn drop(&mut self) {
        self.dispose();
    }
}

/// Enumerates with an enumerator whose subscription failed giving that error as its one item.
impl<R: 'static> IntoIterator for &PullEnumerable<R> {
    type Item = Result<R, ObservableError>;
    type IntoIter = Box<dyn Iterator<Item = Result<R, ObservableError>>>;

    fn into_iter(self) -> Self::IntoIter {
        match self.get_enumerator() {
            Ok(enumerator) => Box::new(enumerator),
            Err(error) => Box::new(std::iter::once(Err(error))),
        }
    }
}
