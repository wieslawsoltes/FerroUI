//! `Dematerialize` (`Linq/Observable/Dematerialize.cs`).

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use crate::notification::Notification;
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::rc::Rc;

/// A value that is a notification: implemented for [`Notification<T>`], so that `dematerialize`
/// is a method of the sequences of notifications (upstream's `Dematerialize<TSource>` extends
/// `IObservable<Notification<TSource>>`).
pub trait NotificationValue: 'static {
    /// The type of the values of the notification.
    type Value: 'static;

    /// The notification.
    fn into_notification(self) -> Notification<Self::Value>;
}

impl<T: 'static> NotificationValue for Notification<T> {
    type Value = T;

    fn into_notification(self) -> Notification<T> {
        self
    }
}

/// The sink of `Dematerialize` (`Dematerialize<TSource>._`): notifies what each notification of
/// the source is.
struct DematerializeSink<N: NotificationValue> {
    sink: Sink<N::Value>,
}

impl<N: NotificationValue> OperatorSink<N, N::Value> for DematerializeSink<N> {
    fn sink(&self) -> &Sink<N::Value> {
        &self.sink
    }
}

impl_sink_dispose!(DematerializeSink<N> where N: NotificationValue);

impl<N: NotificationValue> IObserver<N> for DematerializeSink<N> {
    fn on_next(&self, value: N) {
        match value.into_notification() {
            Notification::OnNext(value) => self.sink.forward_on_next(value),
            Notification::OnError(error) => self.sink.forward_on_error(error),
            Notification::OnCompleted => self.sink.forward_on_completed(),
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_completed();
    }
}

pub(crate) fn dematerialize<N: NotificationValue>(source: Rc<dyn IObservable<N>>) -> Rc<dyn IObservable<N::Value>> {
    // Deviation (DEVIATIONS.md, Reactive): upstream returns `source.AsObservable()` for the
    // sequence of `Materialize` of `source`; the handle of a sequence in the port does not reveal
    // the type of the sequence, so the notifications always go through the operator (the result
    // is the same).
    unary(source, |observer| Rc::new(DematerializeSink::<N> { sink: Sink::new(observer) }))
}

/// The methods of `Dematerialize` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! dematerialize_methods {
    () => {
        /// The notifications of a sequence of [`Notification`](crate::Notification)s as the
        /// notifications of the result: its values, then its error or its completion, which ends
        /// the result (`Dematerialize()`).
        fn dematerialize(
            &self,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<<T as crate::linq::dematerialize::NotificationValue>::Value>>
        where
            T: crate::linq::dematerialize::NotificationValue,
        {
            crate::linq::dematerialize::dematerialize(self.observable())
        }
    };
}
pub(crate) use dematerialize_methods;
