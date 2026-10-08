use std::rc::Rc;

/// A list of shared items that holds one item in place and more in a list.
///
/// Items are compared by reference, as in the original, whose items are
/// reference types. The original rents the array of the list from the shared
/// array pool; here the list owns a `Vec`.
pub struct PooledInlineList<T: ?Sized> {
    item: ItemState<T>,
}

/// The original's `object? _item`: nothing, the one item or the list.
enum ItemState<T: ?Sized> {
    Null,
    Single(Rc<T>),
    List(SimplePooledList<T>),
}

/// The state of a [`PooledInlineList`] that has given up its ownership (see
/// [`PooledInlineList::transfer_raw_state`]).
pub struct PooledInlineListRawState<T: ?Sized>(ItemState<T>);

impl<T: ?Sized> Default for PooledInlineList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: ?Sized> PooledInlineList<T> {
    pub fn new() -> Self {
        Self { item: ItemState::Null }
    }

    /// For compositor serialization purposes only, takes the ownership of previously transferred state
    pub fn from_raw_state(raw_state: PooledInlineListRawState<T>) -> Self {
        Self { item: raw_state.0 }
    }

    pub fn add(&mut self, item: Rc<T>) {
        if matches!(self.item, ItemState::Null) {
            self.item = ItemState::Single(item);
        } else {
            self.convert_to_list();
            if let ItemState::List(list) = &mut self.item {
                list.add(item);
            }
        }
    }

    /// Removes the item when it is the one the list holds in place.
    ///
    /// As in the original, which tests the argument rather than its own
    /// state for being a list, an item of a list of several is not removed.
    pub fn remove(&mut self, item: &Rc<T>) -> bool {
        let same = matches!(&self.item, ItemState::Single(single) if Rc::ptr_eq(single, item));
        if same {
            self.item = ItemState::Null;
            return true;
        }

        false
    }

    fn convert_to_list(&mut self) {
        if matches!(self.item, ItemState::List(_)) {
            return;
        }
        let mut list = SimplePooledList::new();
        if let ItemState::Single(item) = std::mem::replace(&mut self.item, ItemState::Null) {
            list.add(item);
        }
        self.item = ItemState::List(list);
    }

    pub fn ensure_capacity(&mut self, count: i32) {
        if count < 2 {
            return;
        }
        self.convert_to_list();
        if let ItemState::List(list) = &mut self.item {
            list.ensure_capacity(count as usize);
        }
    }

    pub fn dispose(&mut self) {
        if let ItemState::List(list) = &mut self.item {
            list.dispose();
        }
        self.item = ItemState::Null;
    }

    pub fn count(&self) -> i32 {
        match &self.item {
            ItemState::Null => 0,
            ItemState::List(list) => list.count() as i32,
            ItemState::Single(_) => 1,
        }
    }

    pub fn get_enumerator(&self) -> PooledInlineListEnumerator<'_, T> {
        PooledInlineListEnumerator::new(&self.item)
    }

    /// For compositor serialization purposes only, gives up the ownership of the internal state and returns it
    pub fn transfer_raw_state(&mut self) -> PooledInlineListRawState<T> {
        PooledInlineListRawState(std::mem::replace(&mut self.item, ItemState::Null))
    }
}

impl<'a, T: ?Sized> IntoIterator for &'a PooledInlineList<T> {
    type Item = &'a Rc<T>;
    type IntoIter = PooledInlineListEnumerator<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.get_enumerator()
    }
}

struct SimplePooledList<T: ?Sized> {
    items: Option<Vec<Rc<T>>>,
}

impl<T: ?Sized> SimplePooledList<T> {
    fn new() -> Self {
        Self { items: None }
    }

    fn count(&self) -> usize {
        self.items.as_ref().map_or(0, Vec::len)
    }

    fn add(&mut self, item: Rc<T>) {
        let items = self.items.get_or_insert_with(|| Vec::with_capacity(4));
        if items.len() == items.capacity() {
            let count = items.len();
            items.reserve_exact(count);
        }

        items.push(item);
    }

    fn ensure_capacity(&mut self, count: usize) {
        if self.items.is_none() {
            self.items = Some(Vec::with_capacity(count));
            return;
        }
        if let Some(items) = &mut self.items {
            if items.capacity() < count {
                items.reserve_exact(count - items.len());
            }
        }
    }

    fn dispose(&mut self) {
        self.items = None;
    }
}

/// The enumerator of a [`PooledInlineList`] (the original's nested
/// `Enumerator`). It is an `Iterator` as well.
pub struct PooledInlineListEnumerator<'a, T: ?Sized> {
    single_item: Option<&'a Rc<T>>,
    index: i32,
    list: Option<&'a SimplePooledList<T>>,
}

impl<'a, T: ?Sized> PooledInlineListEnumerator<'a, T> {
    fn new(item: &'a ItemState<T>) -> Self {
        let mut result = Self { single_item: None, index: -1, list: None };
        match item {
            ItemState::List(list) => result.list = Some(list),
            ItemState::Single(single) => result.single_item = Some(single),
            ItemState::Null => {}
        }
        result
    }

    pub fn move_next(&mut self) -> bool {
        if self.single_item.is_some() {
            if self.index >= 0 {
                return false;
            }
            self.index = 0;
            return true;
        }

        if let Some(list) = self.list {
            if self.index >= list.count() as i32 - 1 {
                return false;
            }
            self.index += 1;
            return true;
        }

        false
    }

    /// Not supported: panics.
    pub fn reset(&mut self) {
        panic!("Specified method is not supported.");
    }

    /// The current item.
    ///
    /// Panics when the enumerator is not positioned on an item.
    pub fn current(&self) -> &'a Rc<T> {
        if let Some(list) = self.list {
            return match &list.items {
                Some(items) => &items[self.index as usize],
                None => panic!("Object reference not set to an instance of an object."),
            };
        }
        match self.single_item {
            Some(single_item) => single_item,
            None => panic!("Object reference not set to an instance of an object."),
        }
    }

    pub fn dispose(&mut self) {}
}

impl<'a, T: ?Sized> Iterator for PooledInlineListEnumerator<'a, T> {
    type Item = &'a Rc<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.move_next() {
            Some(self.current())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the original has no tests of this type.
    use super::*;

    fn values(list: &PooledInlineList<i32>) -> Vec<i32> {
        list.get_enumerator().map(|item| **item).collect()
    }

    #[test]
    fn holds_one_item_in_place_and_more_in_a_list() {
        let mut target = PooledInlineList::<i32>::new();
        assert_eq!(0, target.count());
        assert!(values(&target).is_empty());

        let first = Rc::new(1);
        target.add(first.clone());
        assert_eq!(1, target.count());
        assert_eq!(vec![1], values(&target));

        for value in 2..=9 {
            target.add(Rc::new(value));
        }
        assert_eq!(9, target.count());
        assert_eq!((1..=9).collect::<Vec<_>>(), values(&target));

        // An item of a list is not removed (see `remove`).
        assert!(!target.remove(&first));
        assert_eq!(9, target.count());

        target.dispose();
        assert_eq!(0, target.count());
    }

    #[test]
    fn removes_the_item_held_in_place() {
        let mut target = PooledInlineList::<i32>::new();
        let item = Rc::new(1);
        target.add(item.clone());

        assert!(!target.remove(&Rc::new(1)));
        assert!(target.remove(&item));
        assert_eq!(0, target.count());
        assert!(!target.remove(&item));
    }

    #[test]
    fn ensure_capacity_converts_to_a_list_for_two_or_more_items() {
        let mut target = PooledInlineList::<i32>::new();
        target.ensure_capacity(1);
        target.add(Rc::new(1));
        assert_eq!(1, target.count());

        target.ensure_capacity(8);
        assert_eq!(1, target.count());
        assert_eq!(vec![1], values(&target));
    }

    #[test]
    fn transfers_its_state() {
        let mut target = PooledInlineList::<i32>::new();
        target.add(Rc::new(1));
        target.add(Rc::new(2));

        let state = target.transfer_raw_state();
        assert_eq!(0, target.count());

        let restored = PooledInlineList::from_raw_state(state);
        assert_eq!(vec![1, 2], values(&restored));
    }
}
