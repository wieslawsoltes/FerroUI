//! How much storage an emptied buffer keeps for its next use.
//!
//! A buffer that is filled again and again (the streams of a composition
//! batch, a queue of a pass) is emptied and kept, so that filling it does not
//! allocate. A buffer keeps the size of the largest content it ever had,
//! though, and the first frame of a large tree fills each of them once with
//! what no later frame comes near: three hundred megabytes of a tree of
//! 350 000 elements stayed allocated for as long as the window lived
//! (desktop-performance.md, section 9.39).
//!
//! So an emptied buffer keeps at most [`RETAINED_BUFFER_BYTES`] and gives the
//! rest back. Content of that size is far above what a frame of an
//! application writes, and a buffer that is filled beyond it pays for growing
//! again, a few nanoseconds for an item, which is little beside what was done
//! to produce the item.
//!
//! Not from upstream, whose pooled buffers are of one fixed size and whose
//! queues are collected with the heap.

use std::collections::{HashMap, HashSet, VecDeque};
use std::mem::size_of;

/// What an emptied buffer keeps for its next use, in bytes.
pub const RETAINED_BUFFER_BYTES: usize = 4 << 20;

/// How many items of `T` a buffer keeps.
const fn kept_items<T>(extra_bytes_per_item: usize) -> usize {
    let item = size_of::<T>() + extra_bytes_per_item;
    if item == 0 { usize::MAX } else { RETAINED_BUFFER_BYTES / item }
}

/// A buffer that gives back the storage it has beyond
/// [`RETAINED_BUFFER_BYTES`].
pub trait ReleaseExcess {
    /// Gives back the storage beyond [`RETAINED_BUFFER_BYTES`]. The content
    /// is kept, whatever its size; a buffer that has no more storage than
    /// that is not touched.
    fn release_excess(&mut self);
}

impl<T> ReleaseExcess for Vec<T> {
    #[inline]
    fn release_excess(&mut self) {
        let keep = kept_items::<T>(0);
        if self.capacity() > keep {
            self.shrink_to(keep);
        }
    }
}

impl<T> ReleaseExcess for VecDeque<T> {
    #[inline]
    fn release_excess(&mut self) {
        let keep = kept_items::<T>(0);
        if self.capacity() > keep {
            self.shrink_to(keep);
        }
    }
}

// A hash table keeps a byte of its own for each item.

impl<T, S> ReleaseExcess for HashSet<T, S>
where
    T: Eq + std::hash::Hash,
    S: std::hash::BuildHasher,
{
    #[inline]
    fn release_excess(&mut self) {
        let keep = kept_items::<T>(1);
        if self.capacity() > keep {
            self.shrink_to(keep);
        }
    }
}

impl<K, V, S> ReleaseExcess for HashMap<K, V, S>
where
    K: Eq + std::hash::Hash,
    S: std::hash::BuildHasher,
{
    #[inline]
    fn release_excess(&mut self) {
        let keep = kept_items::<(K, V)>(1);
        if self.capacity() > keep {
            self.shrink_to(keep);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ReleaseExcess, RETAINED_BUFFER_BYTES};
    use std::collections::{HashMap, HashSet, VecDeque};

    const KEPT_WORDS: usize = RETAINED_BUFFER_BYTES / 8;

    #[test]
    fn a_buffer_within_the_limit_keeps_its_storage() {
        let mut buffer: Vec<u64> = Vec::with_capacity(KEPT_WORDS);
        let capacity = buffer.capacity();
        let storage = buffer.as_ptr();
        buffer.release_excess();
        assert_eq!(buffer.capacity(), capacity);
        assert_eq!(buffer.as_ptr(), storage);

        let mut queue: VecDeque<u64> = VecDeque::with_capacity(1000);
        let capacity = queue.capacity();
        queue.release_excess();
        assert_eq!(queue.capacity(), capacity);
    }

    #[test]
    fn an_emptied_buffer_beyond_the_limit_gives_the_rest_back() {
        let mut buffer: Vec<u64> = Vec::with_capacity(4 * KEPT_WORDS);
        buffer.release_excess();
        assert!(buffer.capacity() >= KEPT_WORDS);
        assert!(buffer.capacity() < 2 * KEPT_WORDS);

        let mut queue: VecDeque<u64> = VecDeque::with_capacity(4 * KEPT_WORDS);
        queue.release_excess();
        assert!(queue.capacity() >= KEPT_WORDS);
        assert!(queue.capacity() < 2 * KEPT_WORDS);

        let mut bytes: Vec<u8> = Vec::with_capacity(3 * RETAINED_BUFFER_BYTES);
        bytes.release_excess();
        assert_eq!(bytes.capacity(), RETAINED_BUFFER_BYTES);
    }

    #[test]
    fn an_emptied_table_beyond_the_limit_gives_the_rest_back() {
        let mut set: HashSet<u64> = HashSet::with_capacity(4 * KEPT_WORDS);
        set.release_excess();
        assert!(set.capacity() < 2 * KEPT_WORDS);

        let mut map: HashMap<u64, u64> = HashMap::with_capacity(4 * KEPT_WORDS);
        map.release_excess();
        assert!(map.capacity() < KEPT_WORDS);
    }

    #[test]
    fn the_content_is_kept_whatever_its_size() {
        let mut buffer: Vec<u64> = (0..2 * KEPT_WORDS as u64).collect();
        buffer.reserve(4 * KEPT_WORDS);
        buffer.release_excess();
        assert_eq!(buffer.len(), 2 * KEPT_WORDS);
        assert!(buffer.capacity() < 4 * KEPT_WORDS);
        assert!(buffer.iter().copied().eq(0..2 * KEPT_WORDS as u64));

        let mut set: HashSet<u64> = (0..1000).collect();
        set.release_excess();
        assert_eq!(set.len(), 1000);
    }

    #[test]
    fn a_buffer_of_items_without_a_size_is_not_touched() {
        let mut buffer: Vec<()> = vec![(); 1000];
        buffer.release_excess();
        assert_eq!(buffer.len(), 1000);
    }
}
