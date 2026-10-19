//! `NoOpComparer<T>`, `NoOpEqualityComparer<T>`, `InvertedComparer<T>` and `ComparerExtensions`
//! (`Utilities/ComparerExtensions.cs`).

use std::cmp::Ordering;
use std::rc::Rc;

/// A comparer (`IComparer<T>`).
pub type Comparer<T> = Rc<dyn Fn(&T, &T) -> Ordering>;

/// A comparer that must not be called: it panics (`NoOpComparer<T>`, whose `Compare` throws
/// `NotImplementedException`).
pub fn no_op_comparer<T: 'static>() -> Comparer<T> {
    Rc::new(|_: &T, _: &T| panic!("The method or operation is not implemented."))
}

/// An equality comparer that must not be called: it panics (`NoOpEqualityComparer<T>`).
pub fn no_op_equality_comparer<T: 'static>() -> Rc<dyn Fn(&T, &T) -> bool> {
    Rc::new(|_: &T, _: &T| panic!("The method or operation is not implemented."))
}

/// The comparer of the opposite order (`InvertedComparer<T>`).
pub fn inverted_comparer<T: 'static>(original: Comparer<T>) -> Comparer<T> {
    Rc::new(move |x: &T, y: &T| original(x, y).reverse())
}

/// `ComparerExtensions`.
pub trait ComparerExtensions<T> {
    /// The comparer of the opposite order (`Invert`).
    fn invert(&self) -> Comparer<T>;
}

impl<T: 'static> ComparerExtensions<T> for Comparer<T> {
    fn invert(&self) -> Comparer<T> {
        inverted_comparer(self.clone())
    }
}
