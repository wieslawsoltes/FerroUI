//! `Cast` (`Linq/Observable/Cast.cs`).
//!
//! Upstream casts a sequence of `object`; the port casts a sequence of `Rc<dyn Any>` (the values of
//! any type, shared as upstream's references are) to the values of one type, `Rc<U>`.

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::any::Any;
use std::fmt;
use std::marker::PhantomData;
use std::rc::Rc;

/// A value that can be cast to a value of a type (`object` of upstream's `Cast` and `OfType`):
/// implemented for `Rc<dyn Any>`.
///
/// A value casts to the type it was created with; Rust has no class inheritance, so a value does
/// not cast to the type of a base class as upstream's do.
pub trait AnyObject: Sized + 'static {
    /// The value as a value of `U`, or the value itself when it is not one (`(U)value`,
    /// `value is U`).
    fn downcast_object<U: 'static>(self) -> Result<Rc<U>, Self>;
}

impl AnyObject for Rc<dyn Any> {
    fn downcast_object<U: 'static>(self) -> Result<Rc<U>, Self> {
        self.downcast::<U>()
    }
}

/// The error of [`cast`](super::ObservableExt::cast) for a value that is not of the type of the
/// cast (`InvalidCastException`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidCastException {
    target_type: &'static str,
}

impl InvalidCastException {
    /// The error of a value that does not cast to `U`.
    pub fn new<U: ?Sized>() -> Self {
        Self { target_type: std::any::type_name::<U>() }
    }

    /// The name of the type of the cast.
    pub fn target_type(&self) -> &'static str {
        self.target_type
    }
}

impl fmt::Display for InvalidCastException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unable to cast the value to type '{}'.", self.target_type)
    }
}

impl std::error::Error for InvalidCastException {}

/// The sink of `Cast` (`Cast<TSource, TResult>._`): a value that is not a `U` ends the sequence
/// with an [`InvalidCastException`].
struct CastSink<T, U> {
    sink: Sink<Rc<U>>,
    _source: PhantomData<fn(T)>,
}

impl<T: AnyObject, U: 'static> OperatorSink<T, Rc<U>> for CastSink<T, U> {
    fn sink(&self) -> &Sink<Rc<U>> {
        &self.sink
    }
}

impl_sink_dispose!(CastSink<T, U> where T: AnyObject);

impl<T: AnyObject, U: 'static> IObserver<T> for CastSink<T, U> {
    fn on_next(&self, value: T) {
        match value.downcast_object::<U>() {
            Ok(result) => self.sink.forward_on_next(result),
            Err(_) => self.sink.forward_on_error(Rc::new(InvalidCastException::new::<U>())),
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_completed();
    }
}

pub(crate) fn cast<T: AnyObject, U: 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<Rc<U>>> {
    unary(source, |observer| Rc::new(CastSink::<T, U> { sink: Sink::new(observer), _source: PhantomData }))
}

/// The methods of `Cast` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! cast_methods {
    () => {
        /// Casts each value of a sequence of `Rc<dyn Any>` to `U`; a value that is not a `U` ends
        /// the sequence with an `InvalidCastException` (`Cast<TResult>()`).
        fn cast<U: 'static>(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<U>>>
        where
            T: crate::linq::cast::AnyObject,
        {
            crate::linq::cast::cast::<T, U>(self.observable())
        }
    };
}
pub(crate) use cast_methods;
