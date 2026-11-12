use std::cell::Cell;
use std::fmt;

/// A field that holds an optional handle (an `Rc`, an `Arc`, or a value made
/// of them) and is read by cloning it.
///
/// It is what `RefCell<Option<T>>` is used for when every read clones the value
/// and every write replaces it, without the borrow flag: the cell is as large
/// as `Option<T>`, eight bytes less than the `RefCell`, which is what an object
/// with several such fields (a composition visual has eight) is smaller by for
/// each of them.
///
/// A read takes the value out, clones it and puts it back, so the clone of `T`
/// must not read the cell it is in. A clone that increments reference counts
/// does not. A write puts the new value in before the old one is dropped, so
/// what the old value does when it is dropped can read the cell and sees the
/// new value.
pub(crate) struct OptionCell<T> {
    value: Cell<Option<T>>,
}

impl<T> OptionCell<T> {
    /// A cell that holds nothing.
    #[inline]
    pub(crate) const fn new() -> Self {
        Self { value: Cell::new(None) }
    }

    /// Replaces the value. The old value is dropped after the cell holds the
    /// new one.
    #[inline]
    pub(crate) fn set(&self, value: Option<T>) {
        drop(self.value.replace(value));
    }

    /// Replaces the value and returns the one the cell held.
    #[inline]
    pub(crate) fn replace(&self, value: Option<T>) -> Option<T> {
        self.value.replace(value)
    }

    /// Takes the value out: the cell holds nothing afterwards.
    #[inline]
    pub(crate) fn take(&self) -> Option<T> {
        self.value.take()
    }

    /// Calls `read` with the value, without cloning it. `read` must not read the cell.
    #[inline]
    pub(crate) fn with<R>(&self, read: impl FnOnce(Option<&T>) -> R) -> R {
        let value = self.value.take();
        let result = read(value.as_ref());
        self.value.set(value);
        result
    }

    /// Whether the cell holds a value, without cloning it.
    #[inline]
    pub(crate) fn is_some(&self) -> bool {
        let value = self.value.take();
        let result = value.is_some();
        self.value.set(value);
        result
    }
}

impl<T: Clone> OptionCell<T> {
    /// A clone of the value.
    #[inline]
    pub(crate) fn get(&self) -> Option<T> {
        let value = self.value.take();
        let result = value.clone();
        self.value.set(value);
        result
    }
}

impl<T> Default for OptionCell<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + fmt::Debug> fmt::Debug for OptionCell<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::OptionCell;
    use std::cell::RefCell;
    use std::mem::size_of;
    use std::rc::Rc;

    trait Named {
        fn name(&self) -> &str;
    }

    struct Item(&'static str);

    impl Named for Item {
        fn name(&self) -> &str {
            self.0
        }
    }

    #[test]
    fn a_new_cell_holds_nothing() {
        let cell: OptionCell<Rc<dyn Named>> = OptionCell::new();
        assert!(cell.get().is_none());
    }

    #[test]
    fn a_read_clones_the_value_and_leaves_it_in_the_cell() {
        let item: Rc<dyn Named> = Rc::new(Item("first"));
        let cell = OptionCell::new();
        cell.set(Some(item.clone()));

        let read = cell.get().expect("the value that was set");
        assert!(Rc::ptr_eq(&read, &item));
        assert_eq!(Rc::strong_count(&item), 3);

        drop(read);
        assert_eq!(cell.get().map(|value| value.name().to_string()).as_deref(), Some("first"));
        assert_eq!(Rc::strong_count(&item), 2);
    }

    #[test]
    fn a_write_drops_the_value_it_replaces() {
        let first: Rc<dyn Named> = Rc::new(Item("first"));
        let second: Rc<dyn Named> = Rc::new(Item("second"));
        let cell = OptionCell::new();

        cell.set(Some(first.clone()));
        cell.set(Some(second.clone()));
        assert_eq!(Rc::strong_count(&first), 1);
        assert_eq!(Rc::strong_count(&second), 2);

        cell.set(None);
        assert!(cell.get().is_none());
        assert_eq!(Rc::strong_count(&second), 1);
    }

    #[test]
    fn the_value_that_is_replaced_is_dropped_after_the_cell_holds_the_new_one() {
        struct ReadsTheCellWhenDropped {
            cell: Rc<OptionCell<Rc<Holder>>>,
            seen: Rc<RefCell<Vec<&'static str>>>,
        }

        struct Holder {
            name: &'static str,
            on_drop: Option<ReadsTheCellWhenDropped>,
        }

        impl Drop for Holder {
            fn drop(&mut self) {
                if let Some(reader) = &self.on_drop {
                    let current = reader.cell.get().map(|holder| holder.name).unwrap_or("nothing");
                    reader.seen.borrow_mut().push(current);
                }
            }
        }

        let cell = Rc::new(OptionCell::new());
        let seen = Rc::new(RefCell::new(Vec::new()));
        cell.set(Some(Rc::new(Holder {
            name: "old",
            on_drop: Some(ReadsTheCellWhenDropped { cell: cell.clone(), seen: seen.clone() }),
        })));

        cell.set(Some(Rc::new(Holder { name: "new", on_drop: None })));
        assert_eq!(*seen.borrow(), ["new"]);

        cell.set(None);
    }

    #[test]
    fn a_value_is_taken_or_replaced_without_a_clone() {
        let first: Rc<dyn Named> = Rc::new(Item("first"));
        let second: Rc<dyn Named> = Rc::new(Item("second"));
        let cell = OptionCell::new();
        assert!(!cell.is_some());

        cell.set(Some(first.clone()));
        assert!(cell.is_some());
        assert_eq!(Rc::strong_count(&first), 2);

        let old = cell.replace(Some(second.clone())).expect("the value that was set");
        assert!(Rc::ptr_eq(&old, &first));
        assert_eq!(Rc::strong_count(&first), 2);

        let taken = cell.take().expect("the value that replaced it");
        assert!(Rc::ptr_eq(&taken, &second));
        assert!(!cell.is_some());
        assert!(cell.get().is_none());
    }

    #[test]
    fn the_cell_is_as_large_as_the_value() {
        assert_eq!(size_of::<OptionCell<Rc<dyn Named>>>(), size_of::<Option<Rc<dyn Named>>>());
        assert_eq!(size_of::<OptionCell<Rc<dyn Named>>>(), 2 * size_of::<usize>());
        assert!(size_of::<RefCell<Option<Rc<dyn Named>>>>() > size_of::<OptionCell<Rc<dyn Named>>>());
    }
}
