use super::{IObservable, IObserver, LightweightSubject};

/// An observer that is an observable as well.
pub trait IFerroSubject<T>: IObserver<T> + IObservable<T> {}

impl<T: Clone + 'static> IFerroSubject<T> for LightweightSubject<T> {}
