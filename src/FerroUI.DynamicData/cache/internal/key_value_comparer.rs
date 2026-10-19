//! `KeyValueComparer<TObject, TKey>` (`Cache/Internal/KeyValueComparer.cs`).

use std::any::Any;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::marker::PhantomData;
use std::rc::Rc;

/// A comparer of items (`IComparer<TObject>`).
pub(crate) type Comparer<T> = Rc<dyn Fn(&T, &T) -> Ordering>;

/// Compares the pairs of a key and an item of a sorted cache (`KeyValueComparer<TObject, TKey>`):
/// by the items with the comparer when there is one, then by the keys, so that each pair has a
/// unique position.
pub struct KeyValueComparer<T, K> {
    comparer: Option<Comparer<T>>,
    _key: PhantomData<fn(&K)>,
}

impl<T, K> Clone for KeyValueComparer<T, K> {
    fn clone(&self) -> Self {
        Self { comparer: self.comparer.clone(), _key: PhantomData }
    }
}

impl<T, K> Default for KeyValueComparer<T, K> {
    /// A comparer of the keys only (`new KeyValueComparer<TObject, TKey>()`).
    fn default() -> Self {
        Self::new(None)
    }
}

impl<T, K> fmt::Debug for KeyValueComparer<T, K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyValueComparer").field("has_comparer", &self.comparer.is_some()).finish()
    }
}

impl<T, K> KeyValueComparer<T, K> {
    /// A comparer of pairs by `comparer`, then by the keys (`KeyValueComparer(IComparer<TObject>?)`).
    pub fn new(comparer: Option<Rc<dyn Fn(&T, &T) -> Ordering>>) -> Self {
        Self { comparer, _key: PhantomData }
    }

    /// The comparer of the items.
    pub fn item_comparer(&self) -> Option<&Rc<dyn Fn(&T, &T) -> Ordering>> {
        self.comparer.as_ref()
    }
}

impl<T: 'static, K: Hash + 'static> KeyValueComparer<T, K> {
    /// Compares two pairs (`Compare`).
    pub fn compare(&self, x: &(K, T), y: &(K, T)) -> Ordering {
        if let Some(comparer) = &self.comparer {
            let result = comparer(&x.1, &y.1);
            if result != Ordering::Equal {
                return result;
            }
        }
        compare_keys(&x.0, &y.0)
    }

    /// The comparer as a function.
    pub fn to_fn(&self) -> Rc<dyn Fn(&(K, T), &(K, T)) -> Ordering> {
        let comparer = self.clone();
        Rc::new(move |x: &(K, T), y: &(K, T)| comparer.compare(x, y))
    }
}

/// Compares two keys as upstream does when the items compare as equal: by `IComparable<TKey>`
/// when the key has it, else by the hash codes.
///
/// Deviation (DEVIATIONS.md, DynamicData): the keys need no `Ord`, as upstream's need no
/// `IComparable`. The standard types that have an order (the strings, the integers, `char` and
/// `bool`) are compared by it, found at run time as upstream finds `IComparable<TKey>`; strings
/// compare by their code points (an ordinal comparison, where `string.CompareTo` uses the
/// culture). Other keys compare by the hash of a `DefaultHasher`, which differs from .NET's hash
/// codes; this only decides the order of items that the comparer finds equal.
pub(crate) fn compare_keys<K: Hash + 'static>(x: &K, y: &K) -> Ordering {
    macro_rules! compare_as {
        ($($ty:ty),* $(,)?) => {
            $(
                if let (Some(x), Some(y)) = ((x as &dyn Any).downcast_ref::<$ty>(), (y as &dyn Any).downcast_ref::<$ty>()) {
                    return x.cmp(y);
                }
            )*
        };
    }
    compare_as!(String, &'static str, Rc<str>, i32, i64, u32, u64, usize, isize, i8, i16, i128, u8, u16, u128, char, bool);
    hash_code(x).cmp(&hash_code(y))
}

fn hash_code<K: Hash>(key: &K) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}
