use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::rc::{Rc, Weak};

/// A list of weakly referenced items, compared by identity.
///
/// Up to [`DEFAULT_ARRAY_SIZE`](Self::DEFAULT_ARRAY_SIZE) items are kept in
/// an array; beyond that the list switches to a dictionary that counts how
/// many times each item was added.
pub struct WeakHashList<T: ?Sized + 'static> {
    dic: Option<OrderedDic<T>>,
    arr: Option<Vec<Option<Weak<T>>>>,
    arr_count: usize,
    need_compact: Cell<bool>,
}

/// An entry of the dictionary storage: a weak reference for stored entries,
/// a strong one for lookups.
struct Key<T: ?Sized> {
    weak: Option<Weak<T>>,
    strong: Option<Rc<T>>,
    hash_code: usize,
}

impl<T: ?Sized> Key<T> {
    fn make_strong(r: &Rc<T>) -> Self {
        Self { hash_code: address(Rc::as_ptr(r)), weak: None, strong: Some(r.clone()) }
    }

    fn make_weak(r: &Rc<T>) -> Self {
        Self { hash_code: address(Rc::as_ptr(r)), weak: Some(Rc::downgrade(r)), strong: None }
    }
}

impl<T: ?Sized> Clone for Key<T> {
    fn clone(&self) -> Self {
        Self { weak: self.weak.clone(), strong: self.strong.clone(), hash_code: self.hash_code }
    }
}

/// The dictionary storage: a hash index over entry slots, so that
/// enumeration follows insertion order, and a removed slot is reused by the
/// next addition (most recently freed first), as the managed dictionary does.
struct OrderedDic<T: ?Sized> {
    entries: Vec<Option<(Key<T>, i32)>>,
    free: Vec<usize>,
    map: HashMap<Key<T>, usize>,
}

impl<T: ?Sized> OrderedDic<T> {
    fn new() -> Self {
        Self { entries: Vec::new(), free: Vec::new(), map: HashMap::new() }
    }

    fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    fn get(&self, key: &Key<T>) -> Option<i32> {
        self.map.get(key).and_then(|&slot| self.entries[slot].as_ref()).map(|(_, count)| *count)
    }

    /// Sets the count of `key`, adding the entry if it is not present. An
    /// existing entry keeps its key.
    fn set(&mut self, key: Key<T>, count: i32) {
        if let Some(&slot) = self.map.get(&key) {
            if let Some(entry) = &mut self.entries[slot] {
                entry.1 = count;
            }
            return;
        }
        let slot = match self.free.pop() {
            Some(slot) => {
                self.entries[slot] = Some((key.clone(), count));
                slot
            }
            None => {
                self.entries.push(Some((key.clone(), count)));
                self.entries.len() - 1
            }
        };
        self.map.insert(key, slot);
    }

    fn remove(&mut self, key: &Key<T>) {
        if let Some(slot) = self.map.remove(key) {
            self.entries[slot] = None;
            self.free.push(slot);
        }
    }

    /// The keys, in enumeration order.
    fn keys(&self) -> impl Iterator<Item = &Key<T>> {
        self.entries.iter().flatten().map(|(key, _)| key)
    }
}

impl<T: ?Sized> Hash for Key<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash_code.hash(state);
    }
}

/// The key comparer of the dictionary storage.
impl<T: ?Sized> PartialEq for Key<T> {
    fn eq(&self, y: &Self) -> bool {
        let x = self;
        if x.hash_code != y.hash_code {
            return false;
        }
        if let Some(x_strong) = &x.strong {
            if let Some(y_strong) = &y.strong {
                return Rc::ptr_eq(x_strong, y_strong);
            }
            let Some(y_weak) = &y.weak else { return false };
            return y_weak.upgrade().is_some_and(|weak_target| Rc::ptr_eq(&weak_target, x_strong));
        } else if let Some(y_strong) = &y.strong {
            let Some(x_weak) = &x.weak else { return false };
            return x_weak.upgrade().is_some_and(|weak_target| Rc::ptr_eq(&weak_target, y_strong));
        }
        match x.weak.as_ref().and_then(Weak::upgrade) {
            None => y.weak.as_ref().and_then(Weak::upgrade).is_none(),
            Some(x_target) => y.weak.as_ref().and_then(Weak::upgrade).is_some_and(|y_target| Rc::ptr_eq(&x_target, &y_target)),
        }
    }
}

impl<T: ?Sized> Eq for Key<T> {}

fn address<T: ?Sized>(ptr: *const T) -> usize {
    ptr as *const () as usize
}

thread_local! {
    /// The shared pools of lists returned by `get_alive`, one per item type.
    static LIST_POOLS: RefCell<HashMap<TypeId, Box<dyn Any>>> = RefCell::new(HashMap::new());
}

impl<T: ?Sized + 'static> Default for WeakHashList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: ?Sized + 'static> WeakHashList<T> {
    pub const DEFAULT_ARRAY_SIZE: usize = 8;

    pub fn new() -> Self {
        Self { dic: None, arr: None, arr_count: 0, need_compact: Cell::new(false) }
    }

    pub fn is_empty(&self) -> bool {
        match &self.dic {
            Some(dic) => dic.is_empty(),
            None => self.arr_count == 0,
        }
    }

    pub fn need_compact(&self) -> bool {
        self.need_compact.get()
    }

    pub fn add(&mut self, item: &Rc<T>) {
        if let Some(dic) = &mut self.dic {
            let strong_key = Key::make_strong(item);
            if let Some(cnt) = dic.get(&strong_key) {
                dic.set(strong_key, cnt + 1);
            } else {
                dic.set(Key::make_weak(item), 1);
            }
            return;
        }

        let arr = self.arr.get_or_insert_with(|| vec![None; Self::DEFAULT_ARRAY_SIZE]);

        if self.arr_count < arr.len() {
            arr[self.arr_count] = Some(Rc::downgrade(item));
            self.arr_count += 1;
            return;
        }

        // Check if something is dead
        for c in 0..self.arr_count {
            if arr[c].as_ref().and_then(Weak::upgrade).is_none() {
                arr[c] = Some(Rc::downgrade(item));
                return;
            }
        }

        let existing: Vec<Rc<T>> = arr.iter().filter_map(|r| r.as_ref().and_then(Weak::upgrade)).collect();
        self.dic = Some(OrderedDic::new());
        for target in &existing {
            self.add(target);
        }

        self.add(item);

        self.arr = None;
        self.arr_count = 0;
    }

    pub fn remove(&mut self, item: &Rc<T>) {
        if let Some(arr) = &mut self.arr {
            for c in 0..self.arr_count {
                if arr[c].as_ref().and_then(Weak::upgrade).is_some_and(|target| Rc::ptr_eq(&target, item)) {
                    arr[c] = None;
                    self.arr_compact();
                    return;
                }
            }
        } else if let Some(dic) = &mut self.dic {
            let strong_key = Key::make_strong(item);

            if let Some(cnt) = dic.get(&strong_key) {
                if cnt > 1 {
                    dic.set(strong_key, cnt - 1);
                    return;
                }
            }

            dic.remove(&strong_key);
        }
    }

    fn arr_compact(&mut self) {
        if let Some(arr) = &mut self.arr {
            let mut empty: Option<usize> = None;
            for c in 0..self.arr_count {
                let r = arr[c].clone();
                // Mark current index as first empty
                if r.is_none() && empty.is_none() {
                    empty = Some(c);
                }
                // If current element isn't null and we have an empty one
                if let (Some(_), Some(e)) = (&r, empty) {
                    arr[c] = None;
                    arr[e] = r;
                    empty = Some(e + 1);
                }
            }

            if let Some(empty) = empty {
                self.arr_count = empty;
            }
        }
    }

    pub fn compact(&mut self) {
        if let Some(dic) = &mut self.dic {
            let to_remove: Vec<Key<T>> = dic
                .keys()
                .filter(|key| key.weak.as_ref().and_then(Weak::upgrade).is_none())
                .map(|key| Key { weak: key.weak.clone(), strong: None, hash_code: key.hash_code })
                .collect();

            for k in &to_remove {
                dic.remove(k);
            }
        }
    }

    /// Clears `list` and keeps it for a later [`get_alive`](Self::get_alive).
    pub fn return_to_shared_pool(mut list: Vec<Rc<T>>) {
        list.clear();
        LIST_POOLS.with(|pools| {
            pools
                .borrow_mut()
                .entry(TypeId::of::<T>())
                .or_insert_with(|| Box::new(Vec::<Vec<Rc<T>>>::new()))
                .downcast_mut::<Vec<Vec<Rc<T>>>>()
                .expect("the pool of an item type holds lists of that type")
                .push(list);
        });
    }

    fn rent_from_shared_pool() -> Vec<Rc<T>> {
        LIST_POOLS
            .with(|pools| {
                pools
                    .borrow_mut()
                    .get_mut(&TypeId::of::<T>())
                    .and_then(|pool| pool.downcast_mut::<Vec<Vec<Rc<T>>>>())
                    .and_then(Vec::pop)
            })
            .unwrap_or_default()
    }

    /// The items that are still alive, or `None` if there are none. The list
    /// comes from `factory` when given, else from the shared pool.
    pub fn get_alive(&mut self, factory: Option<&dyn Fn() -> Vec<Rc<T>>>) -> Option<Vec<Rc<T>>> {
        let mut pooled: Option<Vec<Rc<T>>> = None;
        let mut new_list = || match factory {
            Some(factory) => factory(),
            None => Self::rent_from_shared_pool(),
        };
        if let Some(arr) = &mut self.arr {
            let mut need_compact = false;
            for c in 0..self.arr_count {
                match arr[c].as_ref().and_then(Weak::upgrade) {
                    Some(target) => pooled.get_or_insert_with(&mut new_list).push(target),
                    None => {
                        arr[c] = None;
                        need_compact = true;
                    }
                }
            }
            if need_compact {
                self.arr_compact();
            }
            return pooled;
        }
        if let Some(dic) = &self.dic {
            for key in dic.keys() {
                match key.weak.as_ref().and_then(Weak::upgrade) {
                    Some(target) => pooled.get_or_insert_with(&mut new_list).push(target),
                    None => self.need_compact.set(true),
                }
            }
        }

        pooled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The managed tests add and remove strings by value; their small number
    // strings are cached instances, so the same object is added and removed.
    // The handles below play that part.
    fn strings(count: usize) -> Vec<Rc<str>> {
        (0..count).map(|i| Rc::from(i.to_string())).collect()
    }

    #[test]
    fn is_empty_works() {
        let mut target = WeakHashList::<str>::new();
        let one: Rc<str> = Rc::from("1");

        assert!(target.is_empty());

        target.add(&one);

        assert!(!target.is_empty());

        target.remove(&one);

        assert!(target.is_empty());

        // Fill array storage.
        let arr_max_size = WeakHashList::<str>::DEFAULT_ARRAY_SIZE;
        let items = strings(arr_max_size + 1);

        for item in items.iter().take(arr_max_size) {
            target.add(item);
        }

        assert!(!target.is_empty());

        // This goes above array storage and upgrades to a dictionary.
        target.add(&items[arr_max_size]);

        assert!(!target.is_empty());

        // Remove everything, this should still keep an empty dictionary.
        for item in items.iter().take(arr_max_size + 1) {
            target.remove(item);
        }

        assert!(target.is_empty());
    }

    #[test]
    fn array_compact_after_remove_works() {
        let mut target = WeakHashList::<str>::new();

        // Use all slots in array storage.
        let arr_max_size = WeakHashList::<str>::DEFAULT_ARRAY_SIZE;
        let items = strings(arr_max_size);

        for item in &items {
            target.add(item);
        }

        // This should compact the array.
        target.remove(&items[3]);

        // And new value should fill empty space.
        let forty_two: Rc<str> = Rc::from("42");
        target.add(&forty_two);
    }

    /// Not from upstream: the dictionary storage enumerates in insertion
    /// order and reuses the most recently freed slot, as the managed
    /// dictionary does.
    #[test]
    fn dictionary_storage_keeps_insertion_order() {
        let mut target = WeakHashList::<str>::new();
        let items = strings(WeakHashList::<str>::DEFAULT_ARRAY_SIZE + 4);
        for item in &items {
            target.add(item);
        }

        target.remove(&items[2]);
        let added: Rc<str> = Rc::from("added");
        target.add(&added);

        let mut expected = items.clone();
        expected[2] = added.clone();
        let alive = target.get_alive(None).unwrap();
        assert_eq!(alive.len(), expected.len());
        assert!(alive.iter().zip(&expected).all(|(a, e)| Rc::ptr_eq(a, e)));
    }
}
