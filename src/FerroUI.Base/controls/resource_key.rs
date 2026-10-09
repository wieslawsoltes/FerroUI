use crate::TypeInfo;
use std::any::Any;
use std::fmt;
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::rc::Rc;

/// A value usable as a resource key: anything with value equality and a hash.
///
/// Implemented for every `Eq + Hash + 'static` type.
pub trait ResourceKeyObject: Any {
    fn as_any(&self) -> &dyn Any;
    fn dyn_eq(&self, other: &dyn ResourceKeyObject) -> bool;
    fn dyn_hash(&self, state: &mut dyn Hasher);
    fn key_type_name(&self) -> &'static str;
}

impl<T: Eq + Hash + 'static> ResourceKeyObject for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn dyn_eq(&self, other: &dyn ResourceKeyObject) -> bool {
        other.as_any().downcast_ref::<T>().is_some_and(|other| self == other)
    }

    fn dyn_hash(&self, mut state: &mut dyn Hasher) {
        self.hash(&mut state)
    }

    fn key_type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}

/// The key of a resource in a resource dictionary.
///
/// The object model this library mirrors uses arbitrary objects with value
/// equality as keys. The keys used in practice are strings and types (a
/// control theme is keyed by the type it targets), which get their own cheap
/// variants; any other hashable value can be used through
/// [`ResourceKey::object`].
#[derive(Clone)]
pub enum ResourceKey {
    /// A string key.
    String(Rc<str>),
    /// A type key.
    Type(&'static TypeInfo),
    /// Any other key.
    Object(Rc<dyn ResourceKeyObject>),
}

impl ResourceKey {
    /// Creates a key from any hashable value.
    pub fn object<T: Eq + Hash + 'static>(value: T) -> Self {
        ResourceKey::Object(Rc::new(value))
    }

    /// The key as a string, if it is one.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ResourceKey::String(s) => Some(s),
            _ => None,
        }
    }
}

impl PartialEq for ResourceKey {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ResourceKey::String(a), ResourceKey::String(b)) => a == b,
            (ResourceKey::Type(a), ResourceKey::Type(b)) => std::ptr::eq(*a, *b),
            (ResourceKey::Object(a), ResourceKey::Object(b)) => (**a).dyn_eq(&**b),
            _ => false,
        }
    }
}

impl Eq for ResourceKey {}

impl Hash for ResourceKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            ResourceKey::String(s) => {
                state.write_u8(0);
                s.hash(state)
            }
            ResourceKey::Type(t) => {
                state.write_u8(1);
                t.hash(state)
            }
            ResourceKey::Object(o) => {
                state.write_u8(2);
                (**o).dyn_hash(state)
            }
        }
    }
}

impl fmt::Debug for ResourceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for ResourceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceKey::String(s) => f.write_str(s),
            ResourceKey::Type(t) => f.write_str(t.name()),
            ResourceKey::Object(o) => f.write_str((**o).key_type_name()),
        }
    }
}

impl From<&str> for ResourceKey {
    fn from(value: &str) -> Self {
        ResourceKey::String(value.into())
    }
}

impl From<String> for ResourceKey {
    fn from(value: String) -> Self {
        ResourceKey::String(value.into())
    }
}

impl From<&'static TypeInfo> for ResourceKey {
    fn from(value: &'static TypeInfo) -> Self {
        ResourceKey::Type(value)
    }
}

impl From<&ResourceKey> for ResourceKey {
    fn from(value: &ResourceKey) -> Self {
        value.clone()
    }
}

/// The hasher of the tables keyed by [`ResourceKey`].
///
/// A resource lookup probes the table of every dictionary on the way to the
/// root, and the keys are short: a type, or the name of a resource. The
/// default hasher of the standard library is built to withstand keys chosen
/// by an attacker, which the keys of a resource dictionary are not (they are
/// written in the markup and the code of the application), and costs more
/// per key than the probe itself. This one takes a word at a time: it
/// multiplies, and folds the upper half of the product into the lower one,
/// so that every bit of a word reaches both the bits the table takes its
/// bucket from and the ones it tags an entry with. The same keys are equal
/// and are found as before: only the placement in the table differs.
#[derive(Clone, Copy, Default)]
pub(crate) struct ResourceKeyHasher {
    hash: u64,
}

/// Builds the [`ResourceKeyHasher`] of a table.
pub(crate) type ResourceKeyBuildHasher = BuildHasherDefault<ResourceKeyHasher>;

impl ResourceKeyHasher {
    const MULTIPLIER: u64 = 0x51_7c_c1_b7_27_22_0a_95;

    #[inline]
    fn add(&mut self, word: u64) {
        let product = (self.hash ^ word).wrapping_mul(Self::MULTIPLIER);
        self.hash = product ^ (product >> 32);
    }
}

impl Hasher for ResourceKeyHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(8);
        for chunk in &mut chunks {
            let mut word = [0u8; 8];
            word.copy_from_slice(chunk);
            self.add(u64::from_le_bytes(word));
        }
        let rest = chunks.remainder();
        if !rest.is_empty() {
            // At most seven bytes are left: the last byte of the word takes
            // their count, so that a zero byte at the end is not lost.
            let mut word = [0u8; 8];
            word[..rest.len()].copy_from_slice(rest);
            word[7] = rest.len() as u8;
            self.add(u64::from_le_bytes(word));
        }
    }

    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.add(u64::from(i));
    }

    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.add(u64::from(i));
    }

    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.add(u64::from(i));
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.add(i);
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.add(i as u64);
    }
}
