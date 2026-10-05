use std::rc::Rc;

use crate::platform::{IPlatformGraphicsContext, IPlatformRenderInterfaceContext};
use crate::reactive::IDisposable;

/// A value that [`OwnedDisposable`] can dispose.
pub trait OwnedDisposableValue {
    fn dispose_owned(&self);
}

impl OwnedDisposableValue for dyn IDisposable {
    fn dispose_owned(&self) {
        self.dispose();
    }
}

impl OwnedDisposableValue for dyn IPlatformGraphicsContext {
    fn dispose_owned(&self) {
        self.dispose();
    }
}

impl OwnedDisposableValue for dyn IPlatformRenderInterfaceContext {
    fn dispose_owned(&self) {
        self.dispose();
    }
}

/// A disposable value that is only disposed with its holder when the holder
/// owns it.
pub struct OwnedDisposable<T: ?Sized + OwnedDisposableValue> {
    owns: bool,
    value: Option<Rc<T>>,
}

impl<T: ?Sized + OwnedDisposableValue> OwnedDisposable<T> {
    pub fn new(value: Rc<T>, owns: bool) -> Self {
        Self { owns, value: Some(value) }
    }

    /// The held value.
    ///
    /// # Panics
    /// Panics when the holder has been disposed.
    pub fn value(&self) -> Rc<T> {
        match &self.value {
            Some(value) => value.clone(),
            None => panic!("Cannot access a disposed object: 'OwnedDisposable'."),
        }
    }

    /// Releases the value, disposing it when it is owned.
    pub fn dispose(&mut self) {
        if let Some(value) = self.value.take() {
            if self.owns {
                value.dispose_owned();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactive::Disposable;
    use std::cell::Cell;

    fn counting() -> (Rc<Cell<i32>>, Rc<dyn IDisposable>) {
        let count = Rc::new(Cell::new(0));
        let c = count.clone();
        // Counts every call, unlike `Disposable::create`.
        struct Counting(Rc<Cell<i32>>);
        impl IDisposable for Counting {
            fn dispose(&self) {
                self.0.set(self.0.get() + 1);
            }
        }
        (count, Rc::new(Counting(c)))
    }

    #[test]
    fn disposes_an_owned_value_once() {
        let (count, value) = counting();
        let mut owned = OwnedDisposable::new(value.clone(), true);
        assert!(Rc::ptr_eq(&owned.value(), &value));
        owned.dispose();
        owned.dispose();
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn does_not_dispose_a_value_it_does_not_own() {
        let (count, value) = counting();
        let mut owned = OwnedDisposable::new(value, false);
        owned.dispose();
        assert_eq!(count.get(), 0);
    }

    #[test]
    #[should_panic(expected = "OwnedDisposable")]
    fn value_panics_after_dispose() {
        let mut owned = OwnedDisposable::new(Disposable::empty(), false);
        owned.dispose();
        owned.value();
    }
}
